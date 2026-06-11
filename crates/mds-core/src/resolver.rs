use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use indexmap::{IndexMap, IndexSet};

use crate::ast::{BlockNode, DefineBlock, ExportDirective, ImportDirective, Node};
use crate::error::MdsError;
use crate::evaluator::{evaluate, evaluate_messages, EvalMessage};
use crate::fs::{FileSystem, NativeFs, VirtualFs};
use crate::lexer::tokenize;
use crate::limits::{MAX_BLOCKS_PER_MODULE, MAX_FRONTMATTER_IMPORTS};
use crate::parser::is_valid_identifier;
use crate::parser::parse_with_ctx;
use crate::scope::{FunctionDef, NamespaceScope, Scope};
use crate::validator;
use crate::value::Value;

/// A resolved module with its AST, exports, and prompt body.
///
/// Fields are `pub(crate)` — all external access must go through the methods
/// (`get_export`, `get_all_exports`, `get_prompt_value`, `to_namespace`) which
/// enforce export-visibility logic. Direct field access bypasses that logic.
///
/// # Template Inheritance Fields (Phase 2)
///
/// - `effective_skeleton`: the root-ancestor body as a shared `Arc<[Node]>`. For a
///   non-extending module this is the module's own body (built once; Arc-shared across
///   all extending descendants). For an extending module it is `Arc::clone` of the
///   base's skeleton — never a deep-clone of the `Vec<Node>` (DoS guard, P1).
///
/// - `effective_blocks`: name → fully-overridden `BlockNode`. For non-extending modules
///   it is seeded from the module's own `@block` declarations. For extending modules it
///   is a clone of `base.effective_blocks` with the child's overrides applied (most-
///   derived wins, diamond-inheritance safe — NEVER mutate the cached base map).
///
/// - `frontmatter_values`: the module's parsed YAML mapping. Reserved-key splitting is
///   deferred to Phase 3 (`deep_merge_yaml` refactor); Phase 3 can refine this without
///   re-architecting the field.
///
/// - `extends_path`: the raw `@extends` path string if this was a child template.
///
/// # Cache-poisoning invariant (A1)
///
/// A file may be resolved as a skeleton base before it is also compiled as a standalone
/// entry point (or vice-versa). The cache key is the normalized file key in both cases,
/// so the SAME `ResolvedModule` entry serves both roles.  A skeleton-only entry has
/// `prompt_body = None`; a standalone compile also sets `prompt_body`. To avoid poisoning
/// a future standalone compile with a skeleton entry that has no `prompt_body`, we store
/// ALL fields on every entry. When called as a skeleton base, `process_module_skeleton`
/// returns a full `ResolvedModule` (with `prompt_body = None`). A later standalone call
/// for the same key will find a cache hit and return that entry as-is.  Because
/// `process_module` (non-skeleton path) also produces `prompt_body = None` when the body
/// evaluates to empty/whitespace-only, a `None` prompt_body is not *solely* a skeleton
/// signal — callers must not rely on `prompt_body.is_none()` to mean "skeleton cache hit".
/// The caching rule is: first resolution wins; subsequent hits return the cached entry.
/// This is correct for skeleton bases whose `effective_skeleton` / `effective_blocks` are
/// also used by extending children via Arc-sharing, and for standalone modules whose full
/// compilation is cached.
#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub(crate) functions: HashMap<String, Arc<FunctionDef>>,
    pub(crate) prompt_body: Option<String>,
    pub(crate) raw_frontmatter: Option<String>,
    pub(crate) has_explicit_exports: bool,
    pub(crate) explicit_exports: HashSet<String>,
    /// Root-ancestor body, Arc-shared across all descendants (never deep-cloned).
    /// For non-extending modules: own body. For extending: Arc::clone of base's skeleton.
    pub(crate) effective_skeleton: Arc<[Node]>,
    /// Fully-overridden block map for this subtree. Seeded from own @block declarations
    /// (non-extending) or clone(base.effective_blocks)+child overrides (extending).
    pub(crate) effective_blocks: IndexMap<String, Arc<BlockNode>>,
    /// Parsed YAML frontmatter mapping. Reserved-key splitting deferred to Phase 3.
    pub(crate) frontmatter_values: Option<serde_yaml_ng::Mapping>,
    /// The raw @extends path, if this was a child template.
    // Used by Phase 3 (reserved-key exclusion for the `extends` key) and Phase 5 (diagnostics).
    #[allow(dead_code)]
    pub(crate) extends_path: Option<String>,
}

/// Maximum import depth to prevent stack overflow from deeply chained imports.
const MAX_IMPORT_DEPTH: usize = 64;

/// Module cache to avoid re-resolving the same file or virtual key.
///
/// Supports multiple filesystem backends via the [`FileSystem`] trait.
pub struct ModuleCache {
    fs: Box<dyn FileSystem>,
    /// Stores resolved modules in first-resolution (depth-first) order.
    /// IndexMap preserves insertion order while providing O(1) get/insert/contains_key,
    /// enabling efficient dependency-graph extraction via `dependencies()`.
    modules: IndexMap<String, Arc<ResolvedModule>>,
    /// Tracks modules currently being resolved. IndexSet provides both O(1)
    /// membership test (like HashSet) and insertion-ordered iteration (like Vec),
    /// so a separate `resolving_stack` is no longer needed.
    resolving: IndexSet<String>,
}

impl std::fmt::Debug for ModuleCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleCache")
            .field("modules_count", &self.modules.len())
            .field("resolving_count", &self.resolving.len())
            .finish_non_exhaustive()
    }
}

impl ModuleCache {
    /// Create a new `ModuleCache` backed by the native OS filesystem.
    ///
    /// Equivalent to [`ModuleCache::native`].
    pub fn new() -> Self {
        Self::native()
    }

    /// Create a `ModuleCache` backed by the native OS filesystem.
    pub fn native() -> Self {
        Self {
            fs: Box::new(NativeFs::new()),
            modules: IndexMap::new(),
            resolving: IndexSet::new(),
        }
    }

    /// Create a `ModuleCache` backed by an in-memory virtual filesystem.
    ///
    /// Useful for testing and WASM environments where OS filesystem access
    /// is unavailable.
    pub fn virtual_fs(modules: HashMap<String, String>) -> Self {
        Self {
            fs: Box::new(VirtualFs::new(modules)),
            modules: IndexMap::new(),
            resolving: IndexSet::new(),
        }
    }

    /// Create a `ModuleCache` with a custom [`FileSystem`] implementation.
    pub fn with_fs(fs: Box<dyn FileSystem>) -> Self {
        Self {
            fs,
            modules: IndexMap::new(),
            resolving: IndexSet::new(),
        }
    }

    /// Returns normalized keys of all modules resolved during compilation,
    /// in first-resolution order (depth-first traversal).
    ///
    /// This includes the entry module itself. Use this after a successful
    /// `resolve_path` / `resolve_key` / `resolve_source` call to obtain the
    /// dependency graph. Callers that want to exclude the entry point should
    /// filter it out themselves (see `compile_virtual_with_deps`).
    pub fn dependencies(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Guard against excessively deep import chains.
    fn check_import_depth(&self) -> Result<(), MdsError> {
        if self.resolving.len() >= MAX_IMPORT_DEPTH {
            return Err(MdsError::import_error(format!(
                "import depth exceeds maximum of {MAX_IMPORT_DEPTH} (possible deep chain)"
            )));
        }
        Ok(())
    }

    /// Resolve a module from a filesystem path string.
    ///
    /// `path` is a UTF-8 string representation of the OS path (callers convert
    /// `&Path` to `&str` at the public API boundary via `path_to_str`).
    /// Normalizes `path` to a canonical key via the underlying [`FileSystem`],
    /// then resolves through the module cache with cycle detection and depth guarding.
    pub fn resolve_path(
        &mut self,
        path: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        let key = self.fs.normalize("", path)?;
        self.resolve_by_key(&key, runtime_vars, warnings)
    }

    /// Resolve a module by its normalized key.
    ///
    /// This is the core resolution loop: cache check → depth check →
    /// cycle detection → read → validate type → process → cache insert.
    fn resolve_by_key(
        &mut self,
        key: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        // Step 1: cache hit — return immediately without reading.
        if let Some(cached) = self.modules.get(key) {
            return Ok(Arc::clone(cached));
        }

        // Step 2: cycle detection — must happen before we push to `resolving`.
        if self.resolving.contains(key) {
            let cycle = build_cycle_string(&self.resolving, key);
            return Err(MdsError::circular_import(cycle));
        }

        // Step 3: depth guard.
        self.check_import_depth()?;

        // Step 4: read the file only on a cache miss.
        let source = self.fs.read(key)?;

        // Step 5: determine if markdown (for frontmatter type-key handling).
        let is_md = self.fs.is_markdown(key);

        // Step 6: validate file type.
        validate_file_type(key, &source)?;

        // Mark as resolving before recursing into process_module.
        // IndexSet preserves insertion order, so it serves as both the set (O(1) lookup)
        // and the ordered stack (for cycle path reconstruction).
        self.resolving.insert(key.to_string());

        let ctx = ModuleCtx {
            file_str: key,
            source: &source,
            base_key: key,
            runtime_vars,
        };
        let resolved = self.process_module(&ctx, is_md, warnings);

        // Unmark regardless of success or failure. resolve/unmark is strictly LIFO
        // (we always remove the last element we inserted), so pop() is O(1).
        // Safety-critical LIFO invariant: a mismatched pop would silently corrupt
        // cycle-detection state and allow unbounded recursion.
        let popped = self.resolving.pop();
        let resolved = Self::check_lifo_pop(resolved, popped, key)?;

        // Wrap in Arc, store in cache, and return a clone of the Arc (O(1)).
        let key_owned = key.to_string();
        let arc = Arc::new(resolved);
        self.modules.insert(key_owned, Arc::clone(&arc));

        Ok(arc)
    }

    /// Resolve an import from within a module identified by `base_key`.
    ///
    /// Validates the import path, normalizes it via the filesystem, then
    /// delegates to [`ModuleCache::resolve_by_key`].
    fn resolve_import_from(
        &mut self,
        base_key: &str,
        relative: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        validate_import_path(relative)?;
        let key = self.fs.normalize(base_key, relative)?;
        self.resolve_by_key(&key, runtime_vars, warnings)
    }

    /// Resolve a module by its normalized key.
    ///
    /// This is the entry point for virtual filesystems where there is no OS path.
    /// Use this with [`ModuleCache::virtual_fs`] or a custom [`FileSystem`] backend.
    pub fn resolve_key(
        &mut self,
        key: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        self.resolve_by_key(key, runtime_vars, warnings)
    }

    /// Resolve a module from an in-memory source string.
    ///
    /// Imports within the source are resolved relative to `base_dir`.
    ///
    /// **NativeFs-only**: this method calls `canonicalize()` and `fs.set_root()`,
    /// which only make sense for OS-backed filesystems. For virtual or
    /// WASM environments use [`ModuleCache::resolve_key`] instead.
    pub fn resolve_source(
        &mut self,
        source: &str,
        base_dir: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        // Canonicalize base_dir via the FileSystem abstraction so that custom
        // or virtual backends can override this behaviour (fixes issue #21).
        let canonical_str = self.fs.canonicalize(base_dir)?;
        self.fs.set_root(&canonical_str)?;

        // The base_key must look like a file path so that normalize() can call
        // parent() on it to get the directory. Append a synthetic filename to
        // the canonical directory so imports resolve relative to that directory
        // (not its parent).
        let base_key = format!("{canonical_str}/<source>");

        // Guard against re-entrant or cyclic calls that could form a cycle
        // back through this root module. Mirrors the resolving bookkeeping in
        // resolve_by_key so that cycle detection and depth checks apply to the
        // root module as well.
        self.check_import_depth()?;
        self.resolving.insert(base_key.clone());

        let ctx = ModuleCtx {
            file_str: "<source>",
            source,
            base_key: &base_key,
            runtime_vars,
        };
        let resolved = self.process_module(&ctx, false, warnings);

        let popped = self.resolving.pop();
        Self::check_lifo_pop(resolved, popped, &base_key).map(Arc::new)
    }

    /// Resolve a module by its normalized virtual key in messages mode.
    ///
    /// Like [`resolve_key`] but runs `evaluate_messages` instead of `evaluate`,
    /// returning structured `EvalMessage` values from `@message` blocks.
    ///
    /// Mirrors `resolve_source_messages`: a single `process_module_messages` pass
    /// over the entry module (no prior text-mode evaluation).  Imported sub-modules
    /// are resolved through the normal cache (`resolve_by_key`) inside
    /// `collect_definitions_and_imports`, so they are evaluated only once.
    pub fn resolve_key_messages(
        &mut self,
        key: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<EvalMessage>, MdsError> {
        // Cycle detection: if this key is already on the resolving stack it forms
        // a circular import that must be rejected.
        if self.resolving.contains(key) {
            let cycle = build_cycle_string(&self.resolving, key);
            return Err(MdsError::circular_import(cycle));
        }

        self.check_import_depth()?;

        let source = self.fs.read(key)?;
        let is_md = self.fs.is_markdown(key);
        validate_file_type(key, &source)?;

        self.resolving.insert(key.to_string());

        let ctx = ModuleCtx {
            file_str: key,
            source: &source,
            base_key: key,
            runtime_vars,
        };
        let result = self.process_module_messages(&ctx, is_md, warnings);

        let popped = self.resolving.pop();
        Self::check_lifo_pop(result, popped, key)
    }

    /// Resolve a module from an in-memory source string in messages mode.
    ///
    /// Like [`resolve_source`] but runs `evaluate_messages` instead of `evaluate`.
    pub fn resolve_source_messages(
        &mut self,
        source: &str,
        base_dir: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<EvalMessage>, MdsError> {
        let canonical_str = self.fs.canonicalize(base_dir)?;
        self.fs.set_root(&canonical_str)?;
        let base_key = format!("{canonical_str}/<source>");
        self.check_import_depth()?;
        self.resolving.insert(base_key.clone());
        let ctx = ModuleCtx {
            file_str: "<source>",
            source,
            base_key: &base_key,
            runtime_vars,
        };
        let result = self.process_module_messages(&ctx, false, warnings);
        let popped = self.resolving.pop();
        Self::check_lifo_pop(result, popped, &base_key)
    }

    /// Common messages-mode processing: tokenize, parse, build scope, collect messages.
    ///
    /// Shares setup with `process_module` but calls `evaluate_messages` at the end.
    fn process_module_messages(
        &mut self,
        ctx: &ModuleCtx<'_>,
        is_md: bool,
        warnings: &mut Vec<String>,
    ) -> Result<Vec<EvalMessage>, MdsError> {
        let tokens = tokenize(ctx.source, ctx.file_str)?;
        let module = parse_with_ctx(&tokens, ctx.file_str, ctx.source)?;

        let (mut scope, fm_imports) =
            build_scope_from_frontmatter(module.frontmatter.as_ref(), is_md, ctx.runtime_vars)?;

        self.resolve_frontmatter_imports(&fm_imports, &mut scope, ctx, warnings)?;

        let CollectedDefs {
            functions,
            explicit_exports,
            ..
        } = self.collect_definitions_and_imports(&module.body, &mut scope, ctx, warnings)?;

        // Validate that all named exports refer to defined functions or "prompt" —
        // mirrors process_module exactly so @export <undefined> errors in messages mode
        // the same way it does in text mode (avoids PF-004: alternate path bypassing a check).
        validate_exports(&explicit_exports, &functions)?;

        // Register collected functions in scope for @define calls within @message bodies.
        for (name, func) in &functions {
            scope.set_function(name, Arc::clone(func));
        }

        validator::validate(&module.body, &mut scope, ctx.file_str, ctx.source)?;

        // Check that at least one @message block exists before evaluating.
        if !has_message_block(&module.body) {
            return Err(MdsError::syntax(
                "compile_messages requires at least one @message block, \
                 but none were found in the template",
            ));
        }

        evaluate_messages(&module.body, &mut scope, warnings)
    }

    /// Assert the LIFO pop invariant after `process_module`.
    ///
    /// On double-fault (module error + LIFO violation), prefer the module error
    /// (user-facing root cause) over the LIFO violation (internal compiler bug).
    fn check_lifo_pop<T>(
        module_result: Result<T, MdsError>,
        popped: Option<String>,
        expected: &str,
    ) -> Result<T, MdsError> {
        let lifo_result = if popped.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(MdsError::syntax(format!(
                "internal error: resolving stack LIFO invariant violated \
                 (expected {expected}, got {got}) — this is a compiler bug, please report it",
                got = popped.as_deref().unwrap_or("<empty>"),
            )))
        };
        match (module_result, lifo_result) {
            (Err(module_err), _) => Err(module_err),
            (Ok(_), Err(lifo_err)) => Err(lifo_err),
            (Ok(resolved), Ok(())) => Ok(resolved),
        }
    }

    /// Common module processing: tokenize, parse, build scope, evaluate.
    ///
    /// `ctx.file_str` is the display path for error messages (may be `"<source>"`).
    /// `ctx.base_key` is the normalized key used to resolve relative imports.
    /// `is_md` controls whether the `type` frontmatter key is treated as a file-type marker.
    fn process_module(
        &mut self,
        ctx: &ModuleCtx<'_>,
        is_md: bool,
        warnings: &mut Vec<String>,
    ) -> Result<ResolvedModule, MdsError> {
        // Tokenize and parse
        let tokens = tokenize(ctx.source, ctx.file_str)?;
        let module = parse_with_ctx(&tokens, ctx.file_str, ctx.source)?;

        // Capture raw frontmatter before build_scope_from_frontmatter borrows the module.
        let raw_frontmatter = module.frontmatter.as_ref().map(|fm| fm.raw.clone());

        // Parse frontmatter YAML once for both scope building and storage.
        let frontmatter_values = parse_frontmatter_mapping(module.frontmatter.as_ref())?;

        // Branch: child template (@extends) vs. standalone module.
        if let Some(ext) = module.extends.clone() {
            return self.process_module_extends(
                module,
                ext,
                ctx,
                is_md,
                raw_frontmatter,
                frontmatter_values,
                warnings,
            );
        }

        // ── Standalone (non-extending) path ──────────────────────────────────

        // Build scope from frontmatter + runtime vars; extract any frontmatter imports.
        let (mut scope, fm_imports) =
            build_scope_from_frontmatter(module.frontmatter.as_ref(), is_md, ctx.runtime_vars)?;

        // Resolve frontmatter imports BEFORE body imports (per spec, ADR-014).
        self.resolve_frontmatter_imports(&fm_imports, &mut scope, ctx, warnings)?;

        // Walk the AST: collect @define functions (with closure capture), process imports/exports
        let CollectedDefs {
            functions,
            has_explicit_exports,
            explicit_exports,
            block_names,
        } = self.collect_definitions_and_imports(&module.body, &mut scope, ctx, warnings)?;

        // Validate that all named exports refer to defined functions or "prompt"
        validate_exports(&explicit_exports, &functions)?;

        // Validate semantic correctness before evaluation
        validator::validate(&module.body, &mut scope, ctx.file_str, ctx.source)?;

        // Evaluate the body to get prompt text
        let prompt_body = evaluate(&module.body, &mut scope, warnings)?;
        let prompt_body = (!prompt_body.trim().is_empty()).then_some(prompt_body);

        // Build effective_skeleton from this module's own body (Arc-shared, no deep-clone, P1).
        let effective_skeleton: Arc<[Node]> = Arc::from(module.body.as_slice());

        // Build effective_blocks from this module's own @block declarations.
        let effective_blocks = block_names
            .iter()
            .filter_map(|name| {
                module.body.iter().find_map(|n| {
                    if let Node::Block(b) = n {
                        if b.name == *name {
                            return Some((name.clone(), Arc::new(b.clone())));
                        }
                    }
                    None
                })
            })
            .collect::<IndexMap<_, _>>();

        Ok(ResolvedModule {
            functions,
            prompt_body,
            raw_frontmatter,
            has_explicit_exports,
            explicit_exports,
            effective_skeleton,
            effective_blocks,
            frontmatter_values,
            extends_path: None,
        })
    }

    /// Process a child template that has an `@extends` directive.
    ///
    /// Skeleton-resolves the base, validates child body, splices the final body,
    /// then validates and evaluates against the merged scope.
    ///
    /// Decision #2: base is NEVER validated/evaluated standalone — deferred to leaf.
    /// PF-004: base is read via resolve_by_key_skeleton (FileSystem trait, never std::fs).
    #[allow(clippy::too_many_arguments)]
    fn process_module_extends(
        &mut self,
        module: crate::ast::Module,
        ext: crate::ast::ExtendsDirective,
        ctx: &ModuleCtx<'_>,
        is_md: bool,
        raw_frontmatter: Option<String>,
        frontmatter_values: Option<serde_yaml_ng::Mapping>,
        warnings: &mut Vec<String>,
    ) -> Result<ResolvedModule, MdsError> {
        // ── Step 3a: validate and resolve the base in skeleton mode ──────────
        validate_import_path(&ext.path)
            .map_err(|e| attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset))?;

        let base_key = self
            .fs
            .normalize(ctx.base_key, &ext.path)
            .map_err(|e| attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset))?;

        // PF-004: resolve through resolve_by_key_skeleton so cycle detection,
        // MAX_IMPORT_DEPTH, dependency tracking, and MAX_FILE_SIZE all apply.
        let base = self
            .resolve_by_key_skeleton(&base_key, ctx.runtime_vars, warnings)
            .map_err(|e| attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset))?;

        // ── Step 3b: child-only-blocks check ─────────────────────────────────
        // Every top-level node in module.body must be Node::Block or whitespace-only Text.
        // (Frontmatter and @extends are already split out of module.body by the parser.)
        for node in &module.body {
            match node {
                Node::Block(_) => {}
                Node::Text(t) if t.text.trim().is_empty() => {}
                other => {
                    let offset = node_offset(other);
                    let line_len = ctx.source[offset..]
                        .find('\n')
                        .unwrap_or(ctx.source[offset..].len());
                    return Err(MdsError::extends_error_at(
                        "an extending template may contain only @block overrides",
                        ctx.file_str,
                        ctx.source,
                        offset,
                        line_len,
                    ));
                }
            }
        }

        // ── Step 3c: build effective_blocks from base, applying child overrides ──
        // Clone base's map first (diamond-inheritance safe — never mutate cached base).
        let mut effective_blocks = base.effective_blocks.clone();

        for node in &module.body {
            if let Node::Block(child_block) = node {
                // Decision #6 / F4/E4: child may only override blocks declared by the root base.
                if !effective_blocks.contains_key(&child_block.name) {
                    return Err(MdsError::extends_error_at(
                        "only the root template may declare @block placeholders",
                        ctx.file_str,
                        ctx.source,
                        child_block.offset,
                        child_block.name.len(),
                    ));
                }
                // Most-derived wins.
                effective_blocks.insert(child_block.name.clone(), Arc::new(child_block.clone()));
            }
        }

        // effective_skeleton is the root ancestor's body (Arc::clone — O(1), no deep-copy, P1).
        let effective_skeleton = Arc::clone(&base.effective_skeleton);

        // ── Step 3d: build merged scope ───────────────────────────────────────
        // Child frontmatter takes precedence over base frontmatter (minimal merge).
        // TODO(phase3): replace minimal merge with deep_merge_yaml / reserved-key exclusion
        // / full runtime-last precedence refactor per decision #3 and decision #7.
        let (mut scope, fm_imports) =
            build_scope_from_frontmatter(module.frontmatter.as_ref(), is_md, ctx.runtime_vars)?;

        // Resolve child's frontmatter imports (ADR-014: frontmatter imports before body).
        self.resolve_frontmatter_imports(&fm_imports, &mut scope, ctx, warnings)?;

        // Merge in base's frontmatter variables (child wins on collision — child already in scope).
        // This is the minimal merge: child vars were inserted above, so we skip keys already set.
        // TODO(phase3): replace with deep_merge_yaml.
        if let Some(base_fm) = &base.frontmatter_values {
            for (key, val) in base_fm {
                let serde_yaml_ng::Value::String(key_str) = key else {
                    continue;
                };
                // Skip reserved keys and keys the child already has.
                if key_str == "imports" || key_str == "type" {
                    continue;
                }
                if scope.get_var(key_str).is_none() {
                    if let Ok(value) = crate::value::Value::from_yaml(val.clone()) {
                        scope.set_var(key_str, value);
                    }
                }
            }
        }

        // Merge base functions into scope (F12: base default block calling a base @define).
        // Collision with child frontmatter-imported functions → name_collision.
        for (name, func) in &base.functions {
            if scope.get_function(name).is_some() {
                return Err(MdsError::name_collision(name.clone()));
            }
            scope.set_function(name, Arc::clone(func));
        }

        // Collect child's own definitions from its body (currently zero @define after
        // child-only-blocks check, but structurally correct).
        let CollectedDefs {
            functions: child_functions,
            has_explicit_exports,
            explicit_exports,
            block_names: _,
        } = self.collect_definitions_and_imports(&module.body, &mut scope, ctx, warnings)?;

        // Merge child-defined functions over base (child wins).
        let mut functions = base.functions.clone();
        for (name, func) in child_functions {
            functions.insert(name, func);
        }

        validate_exports(&explicit_exports, &functions)?;

        // ── Step 3e: splice final_body ────────────────────────────────────────
        // Linear O(S+B) pass over the skeleton. Each Block in the skeleton is replaced
        // by its effective body from effective_blocks (O(1) lookup). Non-Block nodes
        // pass through verbatim. Between-block spacing (Text nodes) is preserved (decision #9, F11).
        let final_body = splice_skeleton(&effective_skeleton, &effective_blocks);

        // ── Step 3f: validate + evaluate on final_body ────────────────────────
        // Operates on final_body, NOT module.body. This is what makes E12 work:
        // a base default block referencing an undefined var is caught HERE against
        // the merged leaf scope.
        validator::validate(&final_body, &mut scope, ctx.file_str, ctx.source)?;

        let prompt_body = evaluate(&final_body, &mut scope, warnings)?;
        let prompt_body = (!prompt_body.trim().is_empty()).then_some(prompt_body);

        Ok(ResolvedModule {
            functions,
            prompt_body,
            raw_frontmatter,
            has_explicit_exports,
            explicit_exports,
            effective_skeleton,
            effective_blocks,
            frontmatter_values,
            extends_path: Some(ext.path),
        })
    }

    /// Resolve a module in skeleton mode: tokenize → parse → collect only (no validate/evaluate).
    ///
    /// Uses the same module cache and resolving stack as resolve_by_key, so cycle detection
    /// (mds::circular_import), MAX_IMPORT_DEPTH, dependency tracking, and the MAX_FILE_SIZE
    /// guard all apply automatically (decision #1, PF-004).
    ///
    /// Cache-poisoning invariant: both skeleton and full-compile entries are stored under the
    /// same normalized key. The first resolution wins. See ResolvedModule doc comment for details.
    fn resolve_by_key_skeleton(
        &mut self,
        key: &str,
        runtime_vars: &HashMap<String, Value>,
        warnings: &mut Vec<String>,
    ) -> Result<Arc<ResolvedModule>, MdsError> {
        // Cache hit — return immediately (full or skeleton entry, both are valid bases).
        if let Some(cached) = self.modules.get(key) {
            return Ok(Arc::clone(cached));
        }

        // Cycle detection — same resolving stack as resolve_by_key (decision #1, E5).
        if self.resolving.contains(key) {
            let cycle = build_cycle_string(&self.resolving, key);
            return Err(MdsError::circular_import(cycle));
        }

        // Depth guard (E6).
        self.check_import_depth()?;

        // PF-004: read via FileSystem trait — NEVER std::fs.
        let source = self.fs.read(key)?;
        let is_md = self.fs.is_markdown(key);
        validate_file_type(key, &source)?;

        self.resolving.insert(key.to_string());

        let ctx = ModuleCtx {
            file_str: key,
            source: &source,
            base_key: key,
            runtime_vars,
        };
        let resolved = self.process_module_skeleton(&ctx, is_md, warnings);

        let popped = self.resolving.pop();
        let resolved = Self::check_lifo_pop(resolved, popped, key)?;

        let arc = Arc::new(resolved);
        self.modules.insert(key.to_string(), Arc::clone(&arc));
        Ok(arc)
    }

    /// Tokenize → parse → collect (functions/blocks/frontmatter), NO validate/evaluate.
    ///
    /// Called when this file is a base for @extends. The resulting ResolvedModule has
    /// prompt_body = None. All fields required by extending children are populated.
    fn process_module_skeleton(
        &mut self,
        ctx: &ModuleCtx<'_>,
        is_md: bool,
        warnings: &mut Vec<String>,
    ) -> Result<ResolvedModule, MdsError> {
        let tokens = tokenize(ctx.source, ctx.file_str)?;
        let module = parse_with_ctx(&tokens, ctx.file_str, ctx.source)?;

        let raw_frontmatter = module.frontmatter.as_ref().map(|fm| fm.raw.clone());
        let frontmatter_values = parse_frontmatter_mapping(module.frontmatter.as_ref())?;

        // Build scope for @define closure capture (base functions must be available).
        let (mut scope, fm_imports) =
            build_scope_from_frontmatter(module.frontmatter.as_ref(), is_md, ctx.runtime_vars)?;
        self.resolve_frontmatter_imports(&fm_imports, &mut scope, ctx, warnings)?;

        let CollectedDefs {
            functions,
            has_explicit_exports,
            explicit_exports,
            block_names,
        } = self.collect_definitions_and_imports(&module.body, &mut scope, ctx, warnings)?;

        // Multi-level chain (A←B←C): B may itself extend A.
        // B's effective_skeleton = A's effective_skeleton (Arc::clone, O(1) fold).
        // B's effective_blocks = clone(A.effective_blocks) + B's overrides (most-derived wins, F3).
        let (effective_skeleton, effective_blocks) = if let Some(ext) = module.extends.as_ref() {
            validate_import_path(&ext.path).map_err(|e| {
                attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset)
            })?;
            let grandparent_key = self.fs.normalize(ctx.base_key, &ext.path).map_err(|e| {
                attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset)
            })?;
            let grandparent = self
                .resolve_by_key_skeleton(&grandparent_key, ctx.runtime_vars, warnings)
                .map_err(|e| {
                    attach_import_span(e, &ext.path, ctx.file_str, ctx.source, ext.offset)
                })?;

            // Child-only-blocks check for this intermediate base (3b).
            for node in &module.body {
                match node {
                    Node::Block(_) => {}
                    Node::Text(t) if t.text.trim().is_empty() => {}
                    other => {
                        let offset = node_offset(other);
                        let line_len = ctx.source[offset..]
                            .find('\n')
                            .unwrap_or(ctx.source[offset..].len());
                        return Err(MdsError::extends_error_at(
                            "an extending template may contain only @block overrides",
                            ctx.file_str,
                            ctx.source,
                            offset,
                            line_len,
                        ));
                    }
                }
            }

            let mut eff_blocks = grandparent.effective_blocks.clone();
            for node in &module.body {
                if let Node::Block(b) = node {
                    if !eff_blocks.contains_key(&b.name) {
                        return Err(MdsError::extends_error_at(
                            "only the root template may declare @block placeholders",
                            ctx.file_str,
                            ctx.source,
                            b.offset,
                            b.name.len(),
                        ));
                    }
                    eff_blocks.insert(b.name.clone(), Arc::new(b.clone()));
                }
            }

            (Arc::clone(&grandparent.effective_skeleton), eff_blocks)
        } else {
            // Root base: own body is the skeleton; blocks seeded from own @block declarations.
            let eff_skeleton: Arc<[Node]> = Arc::from(module.body.as_slice());
            let eff_blocks = block_names
                .iter()
                .filter_map(|name| {
                    module.body.iter().find_map(|n| {
                        if let Node::Block(b) = n {
                            if b.name == *name {
                                return Some((name.clone(), Arc::new(b.clone())));
                            }
                        }
                        None
                    })
                })
                .collect::<IndexMap<_, _>>();
            (eff_skeleton, eff_blocks)
        };

        Ok(ResolvedModule {
            functions,
            prompt_body: None,
            raw_frontmatter,
            has_explicit_exports,
            explicit_exports,
            effective_skeleton,
            effective_blocks,
            frontmatter_values,
            extends_path: module.extends.map(|e| e.path),
        })
    }

    /// Walk the AST body and collect `@define` functions (with closure capture),
    /// process `@import` directives, and record `@export` / `@export...from` entries.
    ///
    /// Returns a `CollectedDefs` struct with self-documenting field names.
    fn collect_definitions_and_imports(
        &mut self,
        body: &[Node],
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<CollectedDefs, MdsError> {
        let mut defs = CollectedDefs {
            functions: HashMap::new(),
            has_explicit_exports: false,
            explicit_exports: HashSet::new(),
            block_names: HashSet::new(),
        };

        let mut block_count: usize = 0;
        for node in body {
            match node {
                Node::Define(def) => collect_define(def, &mut defs, scope, ctx)?,
                Node::Import(import) => self.resolve_import(import, scope, ctx, warnings)?,
                Node::Export(export) => self.collect_export(export, &mut defs, ctx, warnings)?,
                Node::Block(block) => {
                    block_count += 1;
                    collect_block(block, &mut defs, block_count, ctx)?;
                }
                _ => {}
            }
        }

        Ok(defs)
    }

    /// Process a single `@export` directive, updating `defs` in place.
    ///
    /// Handles the three export forms: named (`@export foo`), re-export
    /// (`@export foo from "./bar"`), and wildcard (`@export * from "./bar"`).
    fn collect_export(
        &mut self,
        export: &ExportDirective,
        defs: &mut CollectedDefs,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        defs.has_explicit_exports = true;
        match export {
            ExportDirective::Named { name } => {
                defs.explicit_exports.insert(name.clone());
            }
            ExportDirective::ReExport {
                name,
                path: import_path,
            } => {
                // Resolve the source module and bring in the function for
                // re-export only. Per spec: "@export from does not bring the
                // symbol into the current file's scope".
                // Note: resolve_import_from calls validate_import_path internally,
                // so path validation errors surface with correct messages automatically.
                let source_module = self.resolve_import_from(
                    ctx.base_key,
                    import_path,
                    ctx.runtime_vars,
                    warnings,
                )?;
                let func = source_module.get_export(name).ok_or_else(|| {
                    MdsError::export_error(format!(
                        "cannot re-export '{name}': not exported from \"{import_path}\""
                    ))
                })?;
                defs.functions.insert(name.clone(), func);
                defs.explicit_exports.insert(name.clone());
            }
            ExportDirective::Wildcard { path: import_path } => {
                // Re-export all exports from the target module. These are
                // available to importers but NOT in the current file's scope.
                // Note: resolve_import_from calls validate_import_path internally,
                // so path validation errors surface with correct messages automatically.
                let source_module = self.resolve_import_from(
                    ctx.base_key,
                    import_path,
                    ctx.runtime_vars,
                    warnings,
                )?;
                for (name, func) in source_module.get_all_exports() {
                    if defs.functions.contains_key(&name) {
                        return Err(MdsError::name_collision(name));
                    }
                    defs.functions.insert(name.clone(), func);
                    defs.explicit_exports.insert(name);
                }
            }
        }
        Ok(())
    }

    fn resolve_alias_import(
        &mut self,
        path: &str,
        alias: &str,
        offset: usize,
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        if scope.get_namespace(alias).is_some() {
            return Err(MdsError::name_collision(alias.to_string()));
        }
        let resolved = self
            .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
            .map_err(|e| attach_import_span(e, path, ctx.file_str, ctx.source, offset))?;
        scope.set_namespace(alias, resolved.to_namespace());
        Ok(())
    }

    /// Resolve all frontmatter imports, populating `scope` in the same order
    /// as the declarations. Frontmatter imports run before body imports.
    fn resolve_frontmatter_imports(
        &mut self,
        imports: &[FrontmatterImport],
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        for (i, imp) in imports.iter().enumerate() {
            match imp {
                FrontmatterImport::Alias { path, alias } => {
                    if scope.get_namespace(alias).is_some() {
                        return Err(MdsError::name_collision(alias.to_string()));
                    }
                    let resolved = self
                        .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
                        .map_err(|e| attach_frontmatter_index(e, i))?;
                    scope.set_namespace(alias, resolved.to_namespace());
                }
                FrontmatterImport::Merge { path } => {
                    let resolved = self
                        .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
                        .map_err(|e| attach_frontmatter_index(e, i))?;
                    for (name, func) in resolved.get_all_exports() {
                        if scope.get_function(&name).is_some() {
                            return Err(MdsError::name_collision(name));
                        }
                        scope.set_function(&name, func);
                    }
                    if let Some(val) = resolved.get_prompt_value() {
                        scope.set_var("prompt", val);
                    }
                }
                FrontmatterImport::Selective { path, names } => {
                    let resolved = self
                        .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
                        .map_err(|e| attach_frontmatter_index(e, i))?;
                    let not_exported = |name: &str| {
                        MdsError::import_error(format!(
                            "'{name}' is not exported from '{path}' (in frontmatter imports[{i}])"
                        ))
                    };
                    for name in names {
                        if name == "prompt" {
                            scope.set_var(
                                "prompt",
                                resolved
                                    .get_prompt_value()
                                    .ok_or_else(|| not_exported(name))?,
                            );
                        } else {
                            scope.set_function(
                                name,
                                resolved
                                    .get_export(name)
                                    .ok_or_else(|| not_exported(name))?,
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve_merge_import(
        &mut self,
        path: &str,
        offset: usize,
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        let resolved = self
            .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
            .map_err(|e| attach_import_span(e, path, ctx.file_str, ctx.source, offset))?;
        // Per spec: only functions and the prompt body are imported via merge.
        // Frontmatter variables from the imported module are NOT brought into scope.
        for (name, func) in resolved.get_all_exports() {
            if scope.get_function(&name).is_some() {
                return Err(MdsError::name_collision(name));
            }
            scope.set_function(&name, func);
        }
        if let Some(val) = resolved.get_prompt_value() {
            scope.set_var("prompt", val);
        }
        Ok(())
    }

    fn resolve_selective_import(
        &mut self,
        names: &[String],
        path: &str,
        offset: usize,
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        let resolved = self
            .resolve_import_from(ctx.base_key, path, ctx.runtime_vars, warnings)
            .map_err(|e| attach_import_span(e, path, ctx.file_str, ctx.source, offset))?;
        let line_len = ctx.source[offset..]
            .find('\n')
            .unwrap_or(ctx.source[offset..].len());
        let not_exported = |name: &str| {
            MdsError::import_error_at(
                format!("'{name}' is not exported from '{path}'"),
                ctx.file_str,
                ctx.source,
                offset,
                line_len,
            )
        };
        for name in names {
            if name == "prompt" {
                scope.set_var(
                    "prompt",
                    resolved
                        .get_prompt_value()
                        .ok_or_else(|| not_exported(name))?,
                );
            } else {
                scope.set_function(
                    name,
                    resolved
                        .get_export(name)
                        .ok_or_else(|| not_exported(name))?,
                );
            }
        }
        Ok(())
    }

    fn resolve_import(
        &mut self,
        import: &ImportDirective,
        scope: &mut Scope,
        ctx: &ModuleCtx<'_>,
        warnings: &mut Vec<String>,
    ) -> Result<(), MdsError> {
        match import {
            ImportDirective::Alias {
                path,
                alias,
                offset,
            } => self.resolve_alias_import(path, alias, *offset, scope, ctx, warnings),
            ImportDirective::Merge { path, offset } => {
                self.resolve_merge_import(path, *offset, scope, ctx, warnings)
            }
            ImportDirective::Selective {
                names,
                path,
                offset,
            } => self.resolve_selective_import(names, path, *offset, scope, ctx, warnings),
        }
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolvedModule {
    /// Return `true` if `name` is an available export of this module.
    ///
    /// When no explicit `@export` list is present every name is visible.
    /// When an explicit list exists only the listed names are visible.
    fn is_exported(&self, name: &str) -> bool {
        !self.has_explicit_exports || self.explicit_exports.contains(name)
    }

    /// Get a single export by name.
    ///
    /// Returns `Arc<FunctionDef>` — cloning is O(1).
    pub fn get_export(&self, name: &str) -> Option<Arc<FunctionDef>> {
        if !self.is_exported(name) {
            return None;
        }
        self.functions.get(name).cloned()
    }

    /// Get all exported functions.
    ///
    /// Returns `Arc<FunctionDef>` values — cloning is O(1).
    pub fn get_all_exports(&self) -> Vec<(String, Arc<FunctionDef>)> {
        self.functions
            .iter()
            .filter(|(name, _)| self.is_exported(name))
            .map(|(name, func)| (name.clone(), Arc::clone(func)))
            .collect()
    }

    /// Get the prompt body as a Value, if it is an available export.
    pub fn get_prompt_value(&self) -> Option<Value> {
        if self.is_exported("prompt") {
            self.prompt_body.clone().map(Value::String)
        } else {
            None
        }
    }

    /// Convert this resolved module into a namespace scope for aliased imports.
    fn to_namespace(&self) -> NamespaceScope {
        // Build the HashMap in one pass, avoiding the intermediate Vec that
        // get_all_exports() would allocate.
        let functions = self
            .functions
            .iter()
            .filter(|(name, _)| self.is_exported(name))
            .map(|(name, func)| (name.clone(), Arc::clone(func)))
            .collect();
        // Respect export visibility: prompt_body is only included in the namespace
        // when "prompt" is an available export (same rule as get_prompt_value).
        let prompt_body = if self.is_exported("prompt") {
            self.prompt_body.clone()
        } else {
            None
        };
        NamespaceScope {
            functions,
            prompt_body,
        }
    }
}

/// Collected output of the AST definition/import walk in `collect_definitions_and_imports`.
struct CollectedDefs {
    functions: HashMap<String, Arc<FunctionDef>>,
    has_explicit_exports: bool,
    explicit_exports: HashSet<String>,
    /// Tracks declared `@block` names for duplicate-block and block-vs-function collision detection.
    ///
    /// Shared with `collect_define` so that a `@block foo:` and a `@define foo()` in the same
    /// module surface as `mds::name_collision` (same namespace — decision #10).
    block_names: HashSet<String>,
}

/// Bundle of borrowed per-module context threaded through the AST walk helpers.
struct ModuleCtx<'a> {
    /// Canonical display path of the source file (e.g. the path shown in error messages).
    file_str: &'a str,
    /// Raw file content used for source-span diagnostics (offset → line/column lookup).
    source: &'a str,
    /// Normalized key of the current module; used to resolve relative `@import` paths.
    base_key: &'a str,
    /// Variables injected at call-time (e.g. via `--set` or the public API `compile` call).
    runtime_vars: &'a HashMap<String, Value>,
}

/// Return `true` when the AST body contains at least one `@message` block
/// (possibly nested inside `@if` or `@for` — a shallow scan is enough for the
/// "no messages at all" guard; evaluation handles deeper nesting).
fn has_message_block(nodes: &[Node]) -> bool {
    nodes.iter().any(|n| match n {
        Node::Message(_) => true,
        Node::If(block) => {
            has_message_block(&block.then_body)
                || block
                    .elseif_branches
                    .iter()
                    .any(|(_, body)| has_message_block(body))
                || block
                    .else_body
                    .as_deref()
                    .map(has_message_block)
                    .unwrap_or(false)
        }
        Node::For(block) => has_message_block(&block.body),
        // A @block's default body may contain @message blocks.
        Node::Block(block) => has_message_block(&block.body),
        _ => false,
    })
}

/// Process a single `@define` directive, updating `defs` and `scope` in place.
///
/// Captures the definition-site scope for lexical closure semantics so the
/// function body can resolve alias imports, sibling functions, and frontmatter
/// variables from its defining module even when called from a different module.
fn collect_define(
    def: &DefineBlock,
    defs: &mut CollectedDefs,
    scope: &mut Scope,
    ctx: &ModuleCtx<'_>,
) -> Result<(), MdsError> {
    if defs.functions.contains_key(&def.name) || defs.block_names.contains(&def.name) {
        return Err(MdsError::name_collision_at(
            &def.name,
            ctx.file_str,
            ctx.source,
            def.offset,
            def.name.len(),
        ));
    }
    let mut func = FunctionDef::from(def);
    // Capture definition-site scope for lexical closure semantics.
    func.captured.namespaces = scope.get_all_namespaces();
    // Convert Arc<FunctionDef> → owned FunctionDef for captured.functions.
    // Owned captures break potential reference cycles (A captures B captures A).
    func.captured.functions = scope
        .get_all_functions()
        .into_iter()
        .map(|(k, v)| (k, (*v).clone()))
        .collect();
    func.captured.vars = scope.get_all_vars();
    // Wrap in Arc for cheap storage and O(1) scope insertion.
    let arc = Arc::new(func);
    defs.functions.insert(def.name.clone(), Arc::clone(&arc));
    scope.set_function(&def.name, arc);
    Ok(())
}

/// Process a single `@block` directive, updating the `block_names` set in `defs`.
///
/// Detects:
/// - Duplicate `@block` names within the same module (mds::name_collision).
/// - `@block` name colliding with a `@define` function name (mds::name_collision).
///   This is intentional: blocks and functions share the same namespace (decision #10).
/// - `MAX_BLOCKS_PER_MODULE` cap (mds::resource_limit).
///
/// Note: `count` is the running total of @block nodes seen so far (1-indexed after increment
/// at the call site), used to enforce the per-module cap.
fn collect_block(
    block: &BlockNode,
    defs: &mut CollectedDefs,
    count: usize,
    ctx: &ModuleCtx<'_>,
) -> Result<(), MdsError> {
    if count > MAX_BLOCKS_PER_MODULE {
        return Err(MdsError::resource_limit(format!(
            "module has more than {MAX_BLOCKS_PER_MODULE} @block declarations"
        )));
    }
    // Check for duplicate @block name or collision with an existing @define.
    if defs.block_names.contains(&block.name) || defs.functions.contains_key(&block.name) {
        return Err(MdsError::name_collision_at(
            &block.name,
            ctx.file_str,
            ctx.source,
            block.offset,
            block.name.len(),
        ));
    }
    defs.block_names.insert(block.name.clone());
    Ok(())
}

/// Build a scope from optional frontmatter and runtime variable overrides.
///
/// Parses frontmatter YAML (if present), populates scope with variables,
/// then applies runtime_vars to override any frontmatter keys.
/// The `type` key is skipped for `.md` files (it is a file-type marker, not a template var).
///
/// For MDS files (`.mds` or `.md` with `type: mds`), the `imports` key is extracted
/// and returned as a `Vec<FrontmatterImport>` rather than being set as a variable.
/// For plain `.md` files, `imports` is treated as a regular variable.
///
/// Returns `(scope, fm_imports)`.
fn build_scope_from_frontmatter(
    frontmatter: Option<&crate::ast::Frontmatter>,
    is_md: bool,
    runtime_vars: &HashMap<String, Value>,
) -> Result<(Scope, Vec<FrontmatterImport>), MdsError> {
    let mut scope = Scope::new();
    let mut fm_imports: Vec<FrontmatterImport> = Vec::new();

    // A .mds file is always MDS; a .md file is MDS only when its frontmatter
    // contains `type: mds`. Determine this early — it gates both frontmatter
    // parsing and the runtime_vars guard below.
    let is_mds = !is_md || frontmatter.is_some_and(|fm| has_type_mds_frontmatter_raw(&fm.raw));

    if let Some(fm) = frontmatter {
        // Parse YAML once to avoid double-parsing
        let yaml: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&fm.raw).map_err(|e| MdsError::yaml_error(e.to_string()))?;

        if let serde_yaml_ng::Value::Mapping(map) = yaml {
            for (key, val) in map {
                let serde_yaml_ng::Value::String(key_str) = key else {
                    continue;
                };
                if key_str == "type" && is_md {
                    // Skip the 'type' meta-field for .md files (it's a file-type marker)
                    continue;
                }
                if key_str == "imports" {
                    if is_mds {
                        // Parse as structured import declarations, not a scope variable
                        fm_imports = parse_frontmatter_imports_from_yaml(&val)?;
                    } else {
                        // Plain .md: treat `imports` as a regular variable
                        let value = Value::from_yaml(val)?;
                        scope.set_var(&key_str, value);
                    }
                    continue;
                }
                let value = Value::from_yaml(val)?;
                scope.set_var(&key_str, value);
            }
        }
    }

    // Apply runtime vars (override frontmatter)
    for (key, value) in runtime_vars {
        if key == "imports" && is_mds {
            // MDS files (.mds or .md with type:mds) treat `imports` as a reserved
            // key; block --set imports=... for them.
            return Err(MdsError::import_error(
                "'imports' is a reserved frontmatter key for MDS files and cannot be set \
                 via --set",
            ));
        }
        scope.set_var(key, value.clone());
    }

    Ok((scope, fm_imports))
}

/// Validate that all named exports refer to defined functions or the special `"prompt"` export.
fn validate_exports(
    explicit_exports: &HashSet<String>,
    functions: &HashMap<String, Arc<FunctionDef>>,
) -> Result<(), MdsError> {
    for name in explicit_exports {
        if name != "prompt" && !functions.contains_key(name) {
            return Err(MdsError::export_error(format!(
                "cannot export '{name}': not defined in this module"
            )));
        }
    }
    Ok(())
}

/// Validate that an import path is safe and relative.
///
/// Rejects absolute paths and paths containing components that could escape
/// the project directory (e.g., null bytes).
fn validate_import_path(path: &str) -> Result<(), MdsError> {
    if !path.starts_with("./") && !path.starts_with("../") {
        return Err(MdsError::import_error(format!(
            "import path must be relative (start with './' or '../'): \"{path}\""
        )));
    }
    // Reject null bytes which could truncate paths in some OS APIs
    if path.contains('\0') {
        return Err(MdsError::import_error("import path contains null byte"));
    }
    Ok(())
}

/// Validate that a file is a valid MDS file.
///
/// Accepts the already-read source content to avoid double-reading for `.md` files.
/// Uses the normalized key (string) rather than a Path.
fn validate_file_type(key: &str, source: &str) -> Result<(), MdsError> {
    // Extract extension from the key string (split on '/' and '\\' for portability).
    let filename = key.rsplit(['/', '\\']).next().unwrap_or(key);
    // Guard against dotfiles: a filename that starts with '.' and contains no
    // further '.' (e.g. ".mds") has no extension — reject it the same way
    // Path::extension() would return None for such files.
    let ext = if filename.starts_with('.') && !filename[1..].contains('.') {
        None
    } else {
        filename.rsplit('.').next().filter(|e| *e != filename)
    };

    if ext == Some("mds") {
        return Ok(());
    }

    // For .md files, accept when frontmatter contains `type: mds`.
    if ext == Some("md") && has_type_mds_frontmatter(source) {
        return Ok(());
    }

    Err(MdsError::not_mds_file(key.to_string()))
}

/// Return `true` if a frontmatter line declares `type: mds` at the top level.
///
/// Only non-indented lines are matched, consistent with `strip_reserved_keys`
/// which guards with `!starts_with(char::is_whitespace)`. Recognises bare,
/// single-quoted, and double-quoted YAML values.
fn is_type_mds_line(line: &str) -> bool {
    !line.starts_with(char::is_whitespace)
        && line
            .strip_prefix("type:")
            .is_some_and(|v| matches!(v.trim(), "mds" | "\"mds\"" | "'mds'"))
}

/// Return `true` if `source` has a YAML frontmatter block containing `type: mds`.
///
/// Checks without a full YAML parse by scanning frontmatter lines for the key.
fn has_type_mds_frontmatter(source: &str) -> bool {
    source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
        .and_then(|after_fence| after_fence.find("\n---").map(|end| &after_fence[..end]))
        .is_some_and(|fm| fm.lines().any(is_type_mds_line))
}

/// Return `true` if raw frontmatter content (without `---` fences) contains `type: mds`.
///
/// This is the counterpart of [`has_type_mds_frontmatter`] that works on `fm.raw`
/// (the already-extracted frontmatter body) rather than the full source.
fn has_type_mds_frontmatter_raw(raw: &str) -> bool {
    raw.lines().any(is_type_mds_line)
}

/// Format a cycle chain like "a.mds → b.mds → a.mds" from the resolving set.
///
/// `IndexSet` preserves insertion order, so we can use it as both the set
/// and the ordered stack for cycle path reconstruction.
fn build_cycle_string(resolving: &IndexSet<String>, repeated: &str) -> String {
    let start = resolving.iter().position(|k| k == repeated).unwrap_or(0);
    resolving.as_slice()[start..]
        .iter()
        .map(String::as_str)
        .chain(std::iter::once(repeated))
        .map(key_display_name)
        .collect::<Vec<_>>()
        .join(" \u{2192} ")
}

/// Return a short display name for a normalized key (filename, falling back to the key).
fn key_display_name(key: &str) -> &str {
    // Split on both '/' and '\\' for portability across OS and VirtualFs keys.
    key.rsplit(['/', '\\']).next().unwrap_or(key)
}

/// If `err` is a `FileNotFound` error with no source span, attach a span pointing
/// to the `@import` directive in the parent file. Other error variants are returned
/// unchanged so that cascading errors (e.g. circular imports inside the missing
/// file) still report their own locations.
fn attach_import_span(
    err: MdsError,
    path: &str,
    file_str: &str,
    source: &str,
    offset: usize,
) -> MdsError {
    // Compute the span length as the number of bytes from `offset` to the
    // end of the `@import` line (not including the newline character itself),
    // so the whole directive is underlined.
    let line_len = source[offset..]
        .find('\n')
        .unwrap_or(source[offset..].len());
    match err {
        MdsError::FileNotFound { span: None, .. } => {
            MdsError::file_not_found_at(path, file_str, source, offset, line_len)
        }
        MdsError::CircularImport {
            cycle, span: None, ..
        } => MdsError::circular_import_at(cycle, file_str, source, offset, line_len),
        other => other,
    }
}

/// Attach "(in frontmatter imports[i])" context to errors that have no source span.
///
/// Errors that already carry a span (e.g. cascading errors inside the imported file)
/// are returned unchanged so they continue to report their own locations.
fn attach_frontmatter_index(err: MdsError, i: usize) -> MdsError {
    match err {
        MdsError::FileNotFound {
            path, span: None, ..
        } => MdsError::import_error(format!(
            "file not found: \"{path}\" (in frontmatter imports[{i}])"
        )),
        MdsError::CircularImport {
            cycle, span: None, ..
        } => MdsError::import_error(format!(
            "circular import detected: {cycle} (in frontmatter imports[{i}])"
        )),
        MdsError::ImportError {
            message,
            span: None,
            ..
        } if !message.contains("in frontmatter") => {
            MdsError::import_error(format!("{message} (in frontmatter imports[{i}])"))
        }
        other => other,
    }
}

// ── Template inheritance helpers ──────────────────────────────────────────────

/// Parse the frontmatter YAML into a `serde_yaml_ng::Mapping` for storage.
///
/// Returns `None` when there is no frontmatter or when the YAML is not a mapping.
/// Called once per module to avoid double-parsing.
fn parse_frontmatter_mapping(
    frontmatter: Option<&crate::ast::Frontmatter>,
) -> Result<Option<serde_yaml_ng::Mapping>, MdsError> {
    let Some(fm) = frontmatter else {
        return Ok(None);
    };
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fm.raw).map_err(|e| MdsError::yaml_error(e.to_string()))?;
    if let serde_yaml_ng::Value::Mapping(map) = yaml {
        Ok(Some(map))
    } else {
        Ok(None)
    }
}

/// Return the byte offset of a node's first token.
///
/// Used to attach error spans to stray top-level nodes in a child template body.
/// Falls back to 0 for node types that don't carry an explicit offset.
fn node_offset(node: &Node) -> usize {
    match node {
        Node::Text(_) | Node::EscapedBrace => 0,
        Node::Interpolation(i) => i.offset,
        Node::If(b) => b.offset,
        Node::For(b) => b.offset,
        Node::Define(b) => b.offset,
        Node::Import(i) => match i {
            crate::ast::ImportDirective::Alias { offset, .. }
            | crate::ast::ImportDirective::Merge { offset, .. }
            | crate::ast::ImportDirective::Selective { offset, .. } => *offset,
        },
        Node::Export(_) => 0,
        Node::Include(i) => i.offset,
        Node::Message(m) => m.offset,
        Node::Block(b) => b.offset,
    }
}

/// Splice the skeleton body by replacing each `@block` placeholder with its
/// effective body (from the `effective_blocks` override map).
///
/// Linear O(S+B) pass: S = skeleton nodes, B = total block body nodes.
/// Between-block spacing (Text nodes) is preserved verbatim (decision #9, F11).
fn splice_skeleton(
    skeleton: &[Node],
    effective_blocks: &IndexMap<String, Arc<BlockNode>>,
) -> Vec<Node> {
    let mut result = Vec::with_capacity(skeleton.len());
    for node in skeleton {
        if let Node::Block(skeleton_block) = node {
            // Look up the effective block (override or base default) — O(1).
            if let Some(eff_block) = effective_blocks.get(&skeleton_block.name) {
                // Inline the effective body (edges already stripped at parse time).
                result.extend(eff_block.body.clone());
            } else {
                // Unknown block name (shouldn't happen after validation, but safe fallback).
                result.extend(skeleton_block.body.clone());
            }
        } else {
            // Non-block skeleton nodes pass through verbatim.
            result.push(node.clone());
        }
    }
    result
}

// ── Frontmatter imports ───────────────────────────────────────────────────────

/// A single import declaration from YAML frontmatter.
///
/// Three forms mirror the body `@import` directive:
/// - **Alias**: `{ path: "./lib.mds", as: lib }` — imported under a namespace alias.
/// - **Merge**: `{ path: "./lib.mds" }` — all exports merged into the current scope.
/// - **Selective**: `{ path: "./lib.mds", names: [greet, farewell] }` — named exports only.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FrontmatterImport {
    Alias { path: String, alias: String },
    Merge { path: String },
    Selective { path: String, names: Vec<String> },
}

impl FrontmatterImport {
    pub(crate) fn path(&self) -> &str {
        match self {
            Self::Alias { path, .. } | Self::Merge { path } | Self::Selective { path, .. } => path,
        }
    }
}

/// Parse the `imports` key from an already-parsed YAML value.
///
/// `imports_val` must be a YAML Sequence; each element must be a Mapping with
/// a required `path` string key and at most one of `as` (alias) or `names` (selective).
pub(crate) fn parse_frontmatter_imports_from_yaml(
    imports_val: &serde_yaml_ng::Value,
) -> Result<Vec<FrontmatterImport>, MdsError> {
    let serde_yaml_ng::Value::Sequence(seq) = imports_val else {
        return Err(MdsError::import_error(
            "imports must be a YAML sequence (in frontmatter)",
        ));
    };

    if seq.len() > MAX_FRONTMATTER_IMPORTS {
        return Err(MdsError::resource_limit(format!(
            "imports exceeds maximum of {MAX_FRONTMATTER_IMPORTS} entries (in frontmatter)"
        )));
    }

    seq.iter()
        .enumerate()
        .map(|(index, entry)| parse_single_import_entry(entry, index))
        .collect()
}

/// Parse one entry from the `imports` YAML sequence.
///
/// `index` is used solely for error messages.
fn parse_single_import_entry(
    entry: &serde_yaml_ng::Value,
    index: usize,
) -> Result<FrontmatterImport, MdsError> {
    let err =
        |msg: &str| MdsError::import_error(format!("imports[{index}]: {msg} (in frontmatter)"));

    let serde_yaml_ng::Value::Mapping(map) = entry else {
        return Err(err("each entry must be a mapping"));
    };

    // Validate all keys first: reject non-string keys and unknown field names.
    for (k, _) in map {
        let serde_yaml_ng::Value::String(key_str) = k else {
            return Err(err("keys must be strings"));
        };
        match key_str.as_str() {
            "path" | "as" | "names" => {}
            other => return Err(err(&format!("unknown key '{other}'"))),
        }
    }

    // Extract path (required)
    let path_val = map
        .get("path")
        .ok_or_else(|| err("missing required key 'path'"))?;
    let serde_yaml_ng::Value::String(path) = path_val else {
        return Err(err("'path' must be a string"));
    };
    let path = path.clone();

    // Validate path via the same rules as body @import
    validate_import_path(&path).map_err(|_| {
        err(&format!(
            "invalid path \"{path}\": must start with './' or '../'"
        ))
    })?;

    match (map.get("as"), map.get("names")) {
        (Some(_), Some(_)) => Err(err("'as' and 'names' are mutually exclusive")),
        (Some(as_v), None) => parse_alias_entry(as_v, path, &err),
        (None, Some(names_v)) => parse_selective_entry(names_v, path, &err),
        (None, None) => Ok(FrontmatterImport::Merge { path }),
    }
}

/// Parse the alias (`as`) form of a frontmatter import entry.
fn parse_alias_entry(
    as_v: &serde_yaml_ng::Value,
    path: String,
    err: &impl Fn(&str) -> MdsError,
) -> Result<FrontmatterImport, MdsError> {
    let serde_yaml_ng::Value::String(alias) = as_v else {
        return Err(err("'as' must be a string"));
    };
    if !is_valid_identifier(alias) {
        return Err(err(&format!(
            "invalid identifier '{alias}' for 'as': must start with a letter or '_' \
             and contain only alphanumeric characters or '_'"
        )));
    }
    Ok(FrontmatterImport::Alias {
        path,
        alias: alias.clone(),
    })
}

/// Parse the selective (`names`) form of a frontmatter import entry.
fn parse_selective_entry(
    names_v: &serde_yaml_ng::Value,
    path: String,
    err: &impl Fn(&str) -> MdsError,
) -> Result<FrontmatterImport, MdsError> {
    let serde_yaml_ng::Value::Sequence(names_seq) = names_v else {
        return Err(err("'names' must be a sequence"));
    };
    if names_seq.is_empty() {
        return Err(err("names cannot be empty"));
    }
    let mut names = Vec::with_capacity(names_seq.len());
    let mut seen = HashSet::with_capacity(names_seq.len());
    for name_val in names_seq {
        let serde_yaml_ng::Value::String(name) = name_val else {
            return Err(err("each name in 'names' must be a string"));
        };
        // "prompt" is a special export name — allowed without identifier validation
        if name != "prompt" && !is_valid_identifier(name) {
            return Err(err(&format!(
                "invalid identifier '{name}' in 'names': must start with a letter or \
                 '_' and contain only alphanumeric characters or '_'"
            )));
        }
        if !seen.insert(name.as_str()) {
            return Err(err(&format!("duplicate name '{name}' in 'names'")));
        }
        names.push(name.clone());
    }
    Ok(FrontmatterImport::Selective { path, names })
}

/// Parse frontmatter imports from a raw YAML string.
///
/// Returns an empty `Vec` if the `imports` key is absent. Propagates any
/// parse or validation error from [`parse_frontmatter_imports_from_yaml`].
pub(crate) fn parse_frontmatter_imports(raw: &str) -> Result<Vec<FrontmatterImport>, MdsError> {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(raw).map_err(|e| MdsError::yaml_error(e.to_string()))?;

    let serde_yaml_ng::Value::Mapping(ref map) = yaml else {
        return Ok(vec![]);
    };

    let Some(imports_val) = map.get("imports") else {
        return Ok(vec![]);
    };

    parse_frontmatter_imports_from_yaml(imports_val)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to build a YAML Value from inline YAML text
    fn yaml(s: &str) -> serde_yaml_ng::Value {
        serde_yaml_ng::from_str(s).expect("valid yaml in test")
    }

    // ── parse_frontmatter_imports_from_yaml ───────────────────────────────────

    #[test]
    fn parse_fm_import_alias() {
        let v = yaml("- path: ./lib.mds\n  as: lib\n");
        let result = parse_frontmatter_imports_from_yaml(&v).expect("should parse");
        assert_eq!(
            result,
            vec![FrontmatterImport::Alias {
                path: "./lib.mds".into(),
                alias: "lib".into(),
            }]
        );
    }

    #[test]
    fn parse_fm_import_merge() {
        let v = yaml("- path: ./lib.mds\n");
        let result = parse_frontmatter_imports_from_yaml(&v).expect("should parse");
        assert_eq!(
            result,
            vec![FrontmatterImport::Merge {
                path: "./lib.mds".into()
            }]
        );
    }

    #[test]
    fn parse_fm_import_selective() {
        let v = yaml("- path: ./lib.mds\n  names: [greet, farewell]\n");
        let result = parse_frontmatter_imports_from_yaml(&v).expect("should parse");
        assert_eq!(
            result,
            vec![FrontmatterImport::Selective {
                path: "./lib.mds".into(),
                names: vec!["greet".into(), "farewell".into()],
            }]
        );
    }

    #[test]
    fn parse_fm_import_multiple() {
        let v = yaml(
            "- path: ./a.mds\n  as: a\n\
             - path: ./b.mds\n\
             - path: ./c.mds\n  names: [f]\n",
        );
        let result = parse_frontmatter_imports_from_yaml(&v).expect("should parse");
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], FrontmatterImport::Alias { .. }));
        assert!(matches!(result[1], FrontmatterImport::Merge { .. }));
        assert!(matches!(result[2], FrontmatterImport::Selective { .. }));
    }

    #[test]
    fn parse_fm_import_empty_array() {
        let v = yaml("[]");
        let result = parse_frontmatter_imports_from_yaml(&v).expect("empty array is ok");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_fm_no_imports_key() {
        let result =
            parse_frontmatter_imports("name: Alice\ngreeting: hello\n").expect("no imports key");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_fm_err_missing_path() {
        let v = yaml("- as: lib\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("missing path should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("missing required key 'path'") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_path_not_string() {
        let v = yaml("- path: 42\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("non-string path should fail");
        let msg = err.to_string();
        assert!(msg.contains("'path' must be a string"), "got: {msg}");
    }

    #[test]
    fn parse_fm_err_invalid_as_id() {
        let v = yaml("- path: ./lib.mds\n  as: 123bad\n");
        let err =
            parse_frontmatter_imports_from_yaml(&v).expect_err("invalid identifier should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("invalid identifier") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_as_and_names() {
        let v = yaml("- path: ./lib.mds\n  as: lib\n  names: [f]\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("mutually exclusive");
        let msg = err.to_string();
        assert!(
            msg.contains("mutually exclusive") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_unknown_key() {
        let v = yaml("- path: ./lib.mds\n  foo: bar\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("unknown key should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("unknown key 'foo'") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_not_array() {
        let v = yaml("path: ./lib.mds\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("not array should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("must be a YAML sequence") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_empty_names() {
        let v = yaml("- path: ./lib.mds\n  names: []\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("empty names should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("names cannot be empty") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_absolute_path() {
        let v = yaml("- path: /absolute/path.mds\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("absolute path should fail");
        let msg = err.to_string();
        assert!(msg.contains("in frontmatter"), "got: {msg}");
    }

    #[test]
    fn parse_fm_err_exceeds_limit() {
        // Build a sequence with MAX_FRONTMATTER_IMPORTS + 1 entries
        let entry = "- path: ./lib.mds\n";
        let many = entry.repeat(MAX_FRONTMATTER_IMPORTS + 1);
        let v: serde_yaml_ng::Value = serde_yaml_ng::from_str(&many).expect("valid yaml");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("should exceed limit");
        let msg = err.to_string();
        assert!(msg.contains("exceeds maximum"), "got: {msg}");
    }

    #[test]
    fn parse_fm_prompt_name_in_selective() {
        // "prompt" is a special name — allowed without identifier validation
        let v = yaml("- path: ./lib.mds\n  names: [prompt]\n");
        let result = parse_frontmatter_imports_from_yaml(&v).expect("prompt is allowed");
        assert_eq!(
            result,
            vec![FrontmatterImport::Selective {
                path: "./lib.mds".into(),
                names: vec!["prompt".into()],
            }]
        );
    }

    #[test]
    fn parse_fm_err_duplicate_names() {
        // Duplicate names in the selective names list must be rejected.
        let v = yaml("- path: ./lib.mds\n  names: [greet, greet]\n");
        let err = parse_frontmatter_imports_from_yaml(&v).expect_err("duplicate names should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("duplicate name 'greet'") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn parse_fm_err_non_string_key() {
        // Non-string YAML keys (e.g. integer keys) must be rejected explicitly.
        // Construct a YAML mapping with an integer key via the serde_yaml_ng API
        // since inline YAML always coerces to string keys.
        let mut map = serde_yaml_ng::Mapping::new();
        map.insert(
            serde_yaml_ng::Value::String("path".into()),
            serde_yaml_ng::Value::String("./lib.mds".into()),
        );
        map.insert(
            serde_yaml_ng::Value::Number(42.into()),
            serde_yaml_ng::Value::String("something".into()),
        );
        let seq = serde_yaml_ng::Value::Sequence(vec![serde_yaml_ng::Value::Mapping(map)]);
        let err =
            parse_frontmatter_imports_from_yaml(&seq).expect_err("non-string key should fail");
        let msg = err.to_string();
        assert!(
            msg.contains("keys must be strings") && msg.contains("in frontmatter"),
            "got: {msg}"
        );
    }

    #[test]
    fn has_type_mds_frontmatter_raw_ignores_indented() {
        // Indented `type: mds` inside a nested YAML object must not be detected
        // as the file-type marker (only top-level non-indented keys should match).
        assert!(
            !has_type_mds_frontmatter_raw("config:\n  type: mds\n  key: val\n"),
            "indented type:mds should not trigger detection"
        );
        assert!(
            has_type_mds_frontmatter_raw("type: mds\nconfig:\n  type: other\n"),
            "top-level type:mds should trigger detection"
        );
    }

    #[test]
    fn has_type_mds_frontmatter_ignores_indented() {
        // Same as above but for the full-source variant.
        assert!(
            !has_type_mds_frontmatter("---\nconfig:\n  type: mds\n---\nbody\n"),
            "indented type:mds should not trigger detection in full-source variant"
        );
        assert!(
            has_type_mds_frontmatter("---\ntype: mds\nconfig:\n  type: other\n---\nbody\n"),
            "top-level type:mds should trigger detection in full-source variant"
        );
    }

    // ── Phase 1: @block collision and resource-limit tests ────────────────────

    #[test]
    fn block_duplicate_name_collision() {
        // Two @block declarations with the same name → mds::name_collision.
        let src = "@block foo:\nbody1\n@end\n@block foo:\nbody2\n@end\n";
        let result = crate::compile_str(src);
        assert!(result.is_err(), "duplicate @block name must fail");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("'foo'") || msg.contains("foo"),
            "error should mention the colliding name: {msg}"
        );
    }

    #[test]
    fn block_vs_define_name_collision() {
        // @block and @define sharing the same name → mds::name_collision.
        let src = "@define foo():\ncontent\n@end\n@block foo:\nbody\n@end\n";
        let result = crate::compile_str(src);
        assert!(result.is_err(), "@block vs @define collision must fail");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("'foo'") || msg.contains("foo"),
            "error should mention the colliding name: {msg}"
        );
    }

    #[test]
    fn define_vs_block_name_collision() {
        // @define declared after a @block with the same name → mds::name_collision.
        let src = "@block foo:\nbody\n@end\n@define foo():\ncontent\n@end\n";
        let result = crate::compile_str(src);
        assert!(result.is_err(), "@define vs @block collision must fail");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("'foo'") || msg.contains("foo"),
            "error should mention the colliding name: {msg}"
        );
    }

    #[test]
    fn block_max_per_module_cap() {
        // Declaring more than MAX_BLOCKS_PER_MODULE @blocks in one module → resource_limit.
        // Build a source with 257 @block declarations (one over the 256 cap).
        let mut src = String::new();
        for i in 0..=256usize {
            src.push_str(&format!("@block blk{i}:\nbody\n@end\n"));
        }
        let result = crate::compile_str(&src);
        assert!(
            result.is_err(),
            "exceeding MAX_BLOCKS_PER_MODULE should fail with resource_limit"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("resource limit") || msg.contains("256") || msg.contains("block"),
            "error should mention resource limit or block count: {msg}"
        );
    }

    #[test]
    fn block_exactly_at_max_allowed() {
        // Exactly MAX_BLOCKS_PER_MODULE (256) @block declarations should compile.
        let mut src = String::new();
        for i in 0..256usize {
            src.push_str(&format!("@block blk{i}:\nbody\n@end\n"));
        }
        let result = crate::compile_str(&src);
        assert!(
            result.is_ok(),
            "exactly 256 @blocks should succeed, got: {result:?}"
        );
    }

    // ── Phase 2: Template inheritance ─────────────────────────────────────────

    /// Helper: create a VirtualFs-backed ModuleCache from a &[(&str, &str)] slice.
    fn virtual_cache(files: &[(&str, &str)]) -> ModuleCache {
        ModuleCache::virtual_fs(
            files
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }

    /// Helper: compile a VirtualFs entry and return the output string.
    fn compile_virtual(files: &[(&str, &str)], entry: &str) -> Result<String, MdsError> {
        let map: std::collections::HashMap<String, String> = files
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        crate::compile_virtual(map, entry, None)
    }

    /// Helper: check (validate only, no output) a VirtualFs entry.
    fn check_virtual(files: &[(&str, &str)], entry: &str) -> Result<(), MdsError> {
        let map: std::collections::HashMap<String, String> = files
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        crate::check_virtual(map, entry, None)
    }

    // ── F1: issue worked example (headline test) ──────────────────────────────

    #[test]
    fn f1_worked_example_byte_exact() {
        // base.mds: skeleton with @block placeholders
        // child.mds: overrides instructions+tools, inherits output_format default
        // role=data analysis from child frontmatter
        let base = concat!(
            "You are a {role} assistant.\n",
            "\n",
            "@block instructions:\n",
            "Analyze data carefully.\n",
            "@end\n",
            "@block tools:\n",
            "@end\n",
            "@block output_format:\n",
            "Respond in plain text.\n",
            "@end\n",
        );
        let child = concat!(
            "---\n",
            "role: data analysis\n",
            "---\n",
            "@extends \"./base.mds\"\n",
            "@block instructions:\n",
            "Perform statistical analysis.\n",
            "@end\n",
            "@block tools:\n",
            "You have access to: Python, R\n",
            "@end\n",
        );
        let files = [("base.mds", base), ("child.mds", child)];
        let result = compile_virtual(&files, "child.mds");
        assert!(result.is_ok(), "F1 compile failed: {:?}", result.err());
        let output = result.unwrap();

        // Must contain base skeleton text with child's frontmatter variable
        assert!(
            output.contains("You are a data analysis assistant."),
            "F1: base skeleton text not rendered: {output}"
        );
        // Must contain overridden blocks from child
        assert!(
            output.contains("Perform statistical analysis."),
            "F1: child instructions block not rendered: {output}"
        );
        assert!(
            output.contains("You have access to: Python, R"),
            "F1: child tools block not rendered: {output}"
        );
        // Must contain base default for un-overridden block
        assert!(
            output.contains("Respond in plain text."),
            "F1: base default output_format block not rendered: {output}"
        );
    }

    // ── F2: standalone base compiles fine rendering its own defaults ──────────

    #[test]
    fn f2_standalone_base_compiles_with_defaults() {
        let base = concat!(
            "---\n",
            "role: general\n",
            "---\n",
            "You are a {role} assistant.\n",
            "@block instructions:\n",
            "Help the user.\n",
            "@end\n",
        );
        let child = concat!(
            "---\n",
            "role: specialist\n",
            "---\n",
            "@extends \"./base.mds\"\n",
            "@block instructions:\n",
            "Provide expert advice.\n",
            "@end\n",
        );
        let files = [("base.mds", base), ("child.mds", child)];

        // Compile base standalone — must render its own defaults
        let base_out = compile_virtual(&files, "base.mds");
        assert!(
            base_out.is_ok(),
            "F2: standalone base compile failed: {:?}",
            base_out.err()
        );
        let base_str = base_out.unwrap();
        assert!(
            base_str.contains("Help the user."),
            "F2: base default not rendered standalone: {base_str}"
        );
        assert!(
            base_str.contains("You are a general assistant."),
            "F2: base standalone role not rendered: {base_str}"
        );

        // Compile child — must use child overrides and NOT poison base standalone
        let child_out = compile_virtual(&files, "child.mds");
        assert!(
            child_out.is_ok(),
            "F2: child compile failed: {:?}",
            child_out.err()
        );
        let child_str = child_out.unwrap();
        assert!(
            child_str.contains("Provide expert advice."),
            "F2: child override not rendered: {child_str}"
        );
        assert!(
            child_str.contains("You are a specialist assistant."),
            "F2: child role not rendered: {child_str}"
        );
    }

    // ── F2 cache non-poisoning: same base file as skeleton base AND standalone ─

    #[test]
    fn f2_cache_nonpoisoning_base_then_child() {
        // Compile the base FIRST (as standalone), THEN compile child.
        // The cached entry for base must serve the child's skeleton needs.
        let base = "You are a {role} assistant.\n@block instructions:\nDefault.\n@end\n";
        let child = concat!(
            "---\nrole: expert\n---\n",
            "@extends \"./base.mds\"\n",
            "@block instructions:\nExpert advice.\n@end\n",
        );
        let files = [("base.mds", base), ("child.mds", child)];
        let mut cache = virtual_cache(&files);
        let mut warnings = vec![];

        // Compile base standalone (no role var — will fail on {role} unless runtime vars set)
        // For this test, compile the child first (skeleton base resolution caches base),
        // then assert base standalone also works from same cache.
        let child_result = cache.resolve_key("child.mds", &Default::default(), &mut warnings);
        assert!(
            child_result.is_ok(),
            "cache non-poison: child should compile: {:?}",
            child_result.err()
        );

        // Now compile base standalone — should work independently (cache returns entry).
        // Base has {role} undefined without frontmatter, so it would fail standalone unless
        // cached entry with skeleton (prompt_body=None) is returned. We use a base WITH frontmatter.
        let base_with_fm = "---\nrole: default\n---\nYou are a {role}.\n@block b:\nBody.\n@end\n";
        let child2 = concat!(
            "---\nrole: override\n---\n",
            "@extends \"./base2.mds\"\n",
            "@block b:\nOverride.\n@end\n",
        );
        let files2 = [("base2.mds", base_with_fm), ("child2.mds", child2)];
        let mut cache2 = virtual_cache(&files2);
        let mut w = vec![];

        // Both in same process/cache: resolve base standalone first
        let base_out = cache2.resolve_key("base2.mds", &Default::default(), &mut w);
        assert!(
            base_out.is_ok(),
            "cache2: standalone base should succeed: {:?}",
            base_out.err()
        );

        // Then resolve child (base is already cached)
        let child_out = cache2.resolve_key("child2.mds", &Default::default(), &mut w);
        assert!(
            child_out.is_ok(),
            "cache2: child after cached base should succeed: {:?}",
            child_out.err()
        );
        let child_mod = child_out.unwrap();
        assert!(
            child_mod
                .prompt_body
                .as_deref()
                .unwrap_or("")
                .contains("Override."),
            "cache2: child should use override block"
        );
    }

    // ── F3: multi-level chain A←B←C, most-derived wins ──────────────────────

    #[test]
    fn f3_multilevel_most_derived_wins() {
        let a = concat!(
            "@block content:\n",
            "From A.\n",
            "@end\n",
            "@block footer:\n",
            "Footer A.\n",
            "@end\n",
        );
        let b = concat!(
            "@extends \"./a.mds\"\n",
            "@block content:\n",
            "From B.\n",
            "@end\n",
        );
        let c = concat!(
            "@extends \"./b.mds\"\n",
            "@block content:\n",
            "From C.\n",
            "@end\n",
        );
        let files = [("a.mds", a), ("b.mds", b), ("c.mds", c)];

        // C overrides content → "From C." + footer default from A = "Footer A."
        let c_out = compile_virtual(&files, "c.mds").expect("F3: C should compile");
        assert!(
            c_out.contains("From C."),
            "F3: C content should be most-derived: {c_out}"
        );
        assert!(
            c_out.contains("Footer A."),
            "F3: footer should fall through to A default: {c_out}"
        );
        assert!(
            !c_out.contains("From A.") && !c_out.contains("From B."),
            "F3: C should override B which overrode A: {c_out}"
        );

        // B overrides content → "From B." + footer default from A = "Footer A."
        let b_out = compile_virtual(&files, "b.mds").expect("F3: B should compile");
        assert!(
            b_out.contains("From B."),
            "F3: B content should beat A's default: {b_out}"
        );
        assert!(
            b_out.contains("Footer A."),
            "F3: B footer should fall through to A default: {b_out}"
        );

        // A standalone → its own defaults
        let a_out = compile_virtual(&files, "a.mds").expect("F3: A should compile");
        assert!(
            a_out.contains("From A.") && a_out.contains("Footer A."),
            "F3: A standalone should render own defaults: {a_out}"
        );
    }

    // ── F5: diamond inheritance — B and C both extend A; A's cached blocks must not be polluted ─

    #[test]
    fn f5_diamond_inheritance_cache_not_polluted() {
        // A is the base. B and C both extend A.
        // B overrides `shared_block`. C does NOT override `shared_block`.
        // Compiling B then C in one process must not leak B's override into C.
        let a = "@block shared_block:\nFrom A.\n@end\n";
        let b = "@extends \"./a.mds\"\n@block shared_block:\nFrom B.\n@end\n";
        let c = "@extends \"./a.mds\"\n";

        let files = [("a.mds", a), ("b.mds", b), ("c.mds", c)];
        let mut cache = virtual_cache(&files);
        let mut warnings = vec![];

        // Compile B first
        let b_resolved = cache.resolve_key("b.mds", &Default::default(), &mut warnings);
        assert!(
            b_resolved.is_ok(),
            "F5: B should compile: {:?}",
            b_resolved.err()
        );
        let b_body = b_resolved.unwrap().prompt_body.clone().unwrap_or_default();
        assert!(
            b_body.contains("From B."),
            "F5: B should contain its override: {b_body}"
        );

        // Compile C (uses SAME cache, A already cached)
        let c_resolved = cache.resolve_key("c.mds", &Default::default(), &mut warnings);
        assert!(
            c_resolved.is_ok(),
            "F5: C should compile: {:?}",
            c_resolved.err()
        );
        let c_body = c_resolved.unwrap().prompt_body.clone().unwrap_or_default();
        assert!(
            c_body.contains("From A."),
            "F5: C should use A's default (not B's override): {c_body}"
        );
        assert!(
            !c_body.contains("From B."),
            "F5: C must NOT have B's override (cache poisoning): {c_body}"
        );
    }

    // ── F12: base default block calls a base @define → resolves ───────────────

    #[test]
    fn f12_base_define_resolves_in_child() {
        let base = concat!(
            "@define greet(name):\n",
            "Hello, {name}!\n",
            "@end\n",
            "@block content:\n",
            "{greet(\"World\")}\n",
            "@end\n",
        );
        let child = "@extends \"./base.mds\"\n";
        let files = [("base.mds", base), ("child.mds", child)];

        let result = compile_virtual(&files, "child.mds");
        assert!(
            result.is_ok(),
            "F12: child compile failed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert!(
            output.contains("Hello, World!"),
            "F12: base @define should resolve in child: {output}"
        );
    }

    // ── E3: stray child content → mds::extends ────────────────────────────────

    #[test]
    fn e3_stray_child_content_error() {
        let base = "@block b:\nDefault.\n@end\n";
        let child = concat!(
            "@extends \"./base.mds\"\n",
            "This is stray text!\n",
            "@block b:\nOverride.\n@end\n",
        );
        let files = [("base.mds", base), ("child.mds", child)];

        let err = compile_virtual(&files, "child.mds")
            .expect_err("E3: stray text should produce an error");
        let serialized = err.serialize();
        assert_eq!(
            serialized.code, "mds::extends",
            "E3: error code should be mds::extends: {serialized:?}"
        );
        assert!(
            serialized.message.contains("only @block overrides"),
            "E3: message should mention @block overrides: {}",
            serialized.message
        );

        // A5: check_virtual must produce the same error
        let check_err = check_virtual(&files, "child.mds")
            .expect_err("E3 A5: check must also reject stray text");
        assert_eq!(
            check_err.serialize().code,
            "mds::extends",
            "E3 A5: check error code should be mds::extends"
        );
    }

    // ── E4 / F4: unknown override → mds::extends ─────────────────────────────

    #[test]
    fn e4_unknown_override_error() {
        let base = "@block known:\nDefault.\n@end\n";
        let child = concat!(
            "@extends \"./base.mds\"\n",
            "@block known:\nOK.\n@end\n",
            "@block unknown_block:\nBad.\n@end\n",
        );
        let files = [("base.mds", base), ("child.mds", child)];

        let err = compile_virtual(&files, "child.mds")
            .expect_err("E4: unknown override should produce an error");
        let serialized = err.serialize();
        assert_eq!(
            serialized.code, "mds::extends",
            "E4: error code should be mds::extends: {serialized:?}"
        );
        assert!(
            serialized
                .message
                .contains("only the root template may declare"),
            "E4: message should mention root template: {}",
            serialized.message
        );

        // A5: check_virtual must produce the same error
        let check_err = check_virtual(&files, "child.mds")
            .expect_err("E4 A5: check must also reject unknown override");
        assert_eq!(
            check_err.serialize().code,
            "mds::extends",
            "E4 A5: check error code should be mds::extends"
        );
    }

    // ── E5: circular inheritance → mds::circular_import ──────────────────────

    #[test]
    fn e5_circular_inheritance_a_to_b_to_a() {
        // A extends B, B extends A → circular
        let a = "@extends \"./b.mds\"\n@block b:\nA override.\n@end\n";
        let b_content = "@block b:\nB default.\n@end\n";
        // Note: we can only test the cycle detected case; the above won't compile
        // because a.mds extends b.mds and b.mds is a root base (not extending).
        // For a true A→B→A cycle:
        let a2 = "@extends \"./b2.mds\"\n";
        let b2 = "@extends \"./a2.mds\"\n";
        let files2 = [("a2.mds", a2), ("b2.mds", b2)];

        let err = compile_virtual(&files2, "a2.mds")
            .expect_err("E5: circular @extends should produce an error");
        let serialized = err.serialize();
        assert_eq!(
            serialized.code, "mds::circular_import",
            "E5: should surface as mds::circular_import: {serialized:?}"
        );

        // Self-extension: @extends "./self.mds"
        let self_ext = "@extends \"./self.mds\"\n";
        let files_self = [("self.mds", self_ext)];
        let err_self = compile_virtual(&files_self, "self.mds")
            .expect_err("E5: self-extension should produce circular_import");
        let serialized_self = err_self.serialize();
        assert_eq!(
            serialized_self.code, "mds::circular_import",
            "E5: self-extension should surface as mds::circular_import: {serialized_self:?}"
        );

        // Unused variables — just to avoid dead_code warnings in test
        let _ = (a, b_content, files2);
    }

    // ── E5: uses valid circular detection with files that have blocks ─────────

    #[test]
    fn e5_circular_two_hop() {
        // A extends B extends A (proper 2-hop cycle)
        // A has a @block so it's a valid root base syntax-wise
        let a = "@extends \"./b.mds\"\n";
        let b = "@extends \"./a.mds\"\n@block blk:\nB.\n@end\n";
        // This won't work because a.mds has no @block — let's use a root base C that both extend
        // A extends B, B extends A — since neither has @block declarations at root,
        // the cycle is detected before block validation.
        let files = [("a.mds", a), ("b.mds", b)];
        let err = compile_virtual(&files, "a.mds").expect_err("E5: two-hop cycle should error");
        let code = err.serialize().code;
        assert_eq!(
            code, "mds::circular_import",
            "E5: two-hop cycle should be circular_import: {code}"
        );
    }

    // ── E6: 65-deep chain → import-depth error ────────────────────────────────

    #[test]
    fn e6_depth_limit_exceeded() {
        // Build a chain of 66 files: file0 extends file1 extends ... extends file65
        // file65 is the root base with @block declarations.
        let depth = 66usize; // one more than MAX_IMPORT_DEPTH (64)
        let mut files: Vec<(String, String)> = Vec::new();

        // Root base
        let root_src = "@block content:\nRoot.\n@end\n".to_string();
        files.push((format!("file{depth}.mds"), root_src));

        // Each intermediate extends the next
        for i in (0..depth).rev() {
            let src = format!("@extends \"./file{}.mds\"\n", i + 1);
            files.push((format!("file{i}.mds"), src));
        }

        let file_refs: Vec<(&str, &str)> = files
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let err = compile_virtual(&file_refs, "file0.mds")
            .expect_err("E6: depth > 64 should produce an error");
        let code = err.serialize().code;
        // Should be import error or resource_limit (depth exceeded)
        assert!(
            code == "mds::import"
                || code == "mds::resource_limit"
                || code == "mds::circular_import",
            "E6: depth-exceeded error should be import/resource_limit/circular_import: {code}"
        );
    }

    // ── E10: missing base → file-not-found with span ──────────────────────────

    #[test]
    fn e10_missing_base_file_not_found() {
        let child = "@extends \"./missing.mds\"\n";
        let files = [("child.mds", child)];
        let err = compile_virtual(&files, "child.mds")
            .expect_err("E10: missing base should produce file-not-found");
        let serialized = err.serialize();
        assert_eq!(
            serialized.code, "mds::file_not_found",
            "E10: should be file_not_found: {serialized:?}"
        );
    }

    // ── E11: parse error in base propagates with base's location ─────────────

    #[test]
    fn e11_parse_error_in_base_propagates() {
        // Base has a syntax error: @if without condition
        let base = "@block b:\n@if :\nbad\n@end\n@end\n";
        let child = "@extends \"./base.mds\"\n@block b:\nOK.\n@end\n";
        let files = [("base.mds", base), ("child.mds", child)];
        let err = compile_virtual(&files, "child.mds")
            .expect_err("E11: parse error in base should propagate");
        let code = err.serialize().code;
        assert!(
            code == "mds::syntax" || code == "mds::extends",
            "E11: parse error should be syntax or extends: {code}"
        );
    }

    // ── E12: base default block with undefined var → validation error at leaf ──

    #[test]
    fn e12_base_default_undefined_var_caught_at_leaf() {
        // Base has a default block referencing {undefined_var} which is NOT in the
        // base's frontmatter and NOT provided by the child. This should produce an
        // undefined-var error (caught against the merged scope at the leaf).
        let base = "@block content:\n{undefined_var}\n@end\n";
        let child = "@extends \"./base.mds\"\n"; // No frontmatter, no runtime vars

        let files = [("base.mds", base), ("child.mds", child)];
        let err = compile_virtual(&files, "child.mds")
            .expect_err("E12: undefined var in base default should error at leaf");
        let serialized = err.serialize();
        assert!(
            serialized.code == "mds::undefined_var" || serialized.code == "mds::syntax",
            "E12: should be undefined_var (or syntax): {serialized:?}"
        );

        // A5: check_virtual must also reject this
        let check_err = check_virtual(&files, "child.mds")
            .expect_err("E12 A5: check must also reject undefined var in base default");
        assert!(
            check_err.serialize().code == "mds::undefined_var"
                || check_err.serialize().code == "mds::syntax",
            "E12 A5: check should be undefined_var/syntax: {:?}",
            check_err.serialize()
        );
    }

    // ── A2: dependency ordering — base FIRST, before body imports ────────────

    #[test]
    fn a2_dependency_ordering_base_first() {
        let base = "@block b:\nBase.\n@end\n";
        let lib = "@define helper():\nHelper.\n@end\n";
        let child = concat!("@extends \"./base.mds\"\n", "@block b:\n@end\n",);
        // We test via compile_virtual_with_deps which returns the dependency list.
        let files: std::collections::HashMap<String, String> = [
            ("base.mds".to_string(), base.to_string()),
            ("lib.mds".to_string(), lib.to_string()),
            ("child.mds".to_string(), child.to_string()),
        ]
        .into_iter()
        .collect();

        let result = crate::compile_virtual_with_deps(files, "child.mds", None);
        assert!(result.is_ok(), "A2: should compile: {:?}", result.err());
        let output = result.unwrap();
        // base.mds must appear in dependencies (it's a dependency of child.mds)
        assert!(
            output.dependencies.contains(&"base.mds".to_string()),
            "A2: base.mds should be in dependencies: {:?}",
            output.dependencies
        );
        // base.mds must appear BEFORE any body imports (scan_imports puts extends first)
        if let Some(base_idx) = output.dependencies.iter().position(|d| d == "base.mds") {
            // If there are body imports, they must come after base
            // For this test case there are no body imports, but the order is correct.
            assert!(
                base_idx == 0,
                "A2: base.mds should be first dependency: {:?}",
                output.dependencies
            );
        }
    }

    // ── P1: effective_skeleton is Arc<[Node]>, no deep-clone ─────────────────

    #[test]
    fn p1_effective_skeleton_is_arc_shared() {
        // Verify that after resolving a child, both the base and child share the
        // same Arc<[Node]> skeleton (pointer equality).
        let base = "@block b:\nBase.\n@end\n";
        let child = "@extends \"./base.mds\"\n@block b:\nChild.\n@end\n";
        let files = [("base.mds", base), ("child.mds", child)];
        let mut cache = virtual_cache(&files);
        let mut warnings = vec![];

        // Resolve base first (as skeleton via child resolution)
        let child_resolved = cache
            .resolve_key("child.mds", &Default::default(), &mut warnings)
            .expect("P1: child should compile");
        let base_resolved = cache
            .resolve_key("base.mds", &Default::default(), &mut warnings)
            .expect("P1: base should compile");

        // Both should share the same Arc<[Node]> skeleton (Arc::ptr_eq)
        let child_skeleton = &child_resolved.effective_skeleton;
        let base_skeleton = &base_resolved.effective_skeleton;
        assert!(
            Arc::ptr_eq(child_skeleton, base_skeleton),
            "P1: child and base must share the same Arc<[Node]> skeleton (ptr_eq)"
        );
    }

    // ── MdsError::Extends serialize() wired correctly ─────────────────────────

    #[test]
    fn extends_error_serialize_code() {
        let err = MdsError::extends_error_at("test message", "child.mds", "source", 0, 5);
        let serialized = err.serialize();
        assert_eq!(
            serialized.code, "mds::extends",
            "extends error code: {serialized:?}"
        );
        assert!(
            serialized.span.is_some(),
            "extends error should have a span"
        );
    }
}
