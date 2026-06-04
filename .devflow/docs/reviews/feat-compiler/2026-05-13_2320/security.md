# Security Review Report

**Branch**: feat/compiler -> main
**Date**: 2026-05-13

## Issues in Your Changes (BLOCKING)

### MEDIUM

**Symlink check only applies to top-level resolve path, not intermediate components** - `src/resolver.rs:71-79`
**Confidence**: 82%
- Problem: The symlink rejection check (`symlink_metadata`) tests whether the final import path itself is a symlink. However, an attacker with write access to the filesystem could place a symlink as an intermediate directory component in the path (e.g., `./imports/malicious_symlink/../../../etc/passwd`). The `canonicalize()` call on line 81 would silently resolve the intermediate symlink, and the final canonicalized path might escape the `root_dir` boundary. The `starts_with(root)` check on line 115 does catch this post-canonicalization, so the impact is mitigated by the path traversal guard. However, the symlink check itself is incomplete since it only examines the leaf path component.
- Fix: The path traversal boundary check (`starts_with(root)`) on line 115 correctly catches escapes after canonicalization, so the current defense-in-depth approach is sound. The symlink check provides an additional early rejection but is not the sole defense. Consider documenting that the `starts_with` check is the primary security boundary and the symlink check is a best-effort early exit. Alternatively, walk each path component:
```rust
// Walk ancestors to check for symlinks on any component
let mut check = canonical.as_path();
while let Some(parent) = check.parent() {
    if let Ok(meta) = std::fs::symlink_metadata(check) {
        if meta.file_type().is_symlink() {
            return Err(MdsError::import_error(format!(
                "symlinks are not allowed in import paths: {}",
                check.display()
            )));
        }
    }
    if parent == check { break; }
    check = parent;
}
```

**`is_leaf_loop` heuristic may under-count total iterations** - `src/evaluator.rs:335-349`
**Confidence**: 80%
- Problem: The `is_leaf_loop` check on line 335 only counts iterations against `total_iterations` for loops whose body contains no nested `@for` blocks. The intent is to avoid double-counting. However, this means a two-level nested loop where the outer loop has 100,000 elements and the inner loop has 1 element would count 100,000 (inner only). But if the outer loop has 1 element and the inner has 100,000 elements and there are also 999 other single-level sibling loops with 100,000 iterations each, the total accounting may miss the outer loop's contribution. More importantly, if a function called via `{func()}` inside a loop body itself contains a `@for` loop, the static `matches!(n, Node::For(_))` check on the AST does not see into function bodies, so the outer loop would be counted as a leaf when it is not. This could allow bypassing the `MAX_TOTAL_ITERATIONS` limit through function indirection.
- Fix: Count all loop iterations uniformly instead of using the leaf heuristic. The simpler approach counts total iterations across all loop entry points:
```rust
*total_iterations += 1;
if *total_iterations > MAX_TOTAL_ITERATIONS {
    return Err(MdsError::Io {
        message: format!(
            "total loop iterations exceeded maximum of {} across all loops",
            MAX_TOTAL_ITERATIONS
        ),
    });
}
```
This may overcount for statically-nested loops, but that is the conservative (safe) direction. Alternatively, keep the current heuristic for static nesting but also count iterations when the body contains function calls.

## Issues in Code You Touched (Should Fix)

### MEDIUM

**`find_project_root` loop has no iteration bound** - `src/resolver.rs:16-28`
**Confidence**: 85%
- Problem: The `find_project_root` function walks up the directory tree using `dir.pop()` in a `loop`. While `Path::pop()` returns `false` when it can no longer pop (at the root), this is the only termination condition. On well-formed systems this always terminates, but it relies on the OS filesystem abstraction behaving correctly. The function has no explicit upper bound, which violates the reliability principle of "every loop has a fixed upper bound."
- Fix: Add a maximum iteration count as a safety net:
```rust
fn find_project_root(start: &Path) -> PathBuf {
    let mut dir = start.to_path_buf();
    // Safety: filesystem depth is bounded; 256 is far beyond any reasonable path.
    for _ in 0..256 {
        for marker in [".git", ".mdsroot"] {
            if dir.join(marker).exists() {
                return dir;
            }
        }
        if !dir.pop() {
            return start.to_path_buf();
        }
    }
    start.to_path_buf()
}
```

**`mds init` writes to arbitrary user-specified path without validation** - `src/main.rs:304-334`
**Confidence**: 80%
- Problem: The `mds init` command takes a `filename` argument and writes a starter template to that path without any path validation. A user could specify an absolute path (`mds init /etc/cron.d/malicious`) or a path with directory traversal (`mds init ../../../.bashrc`). While this is a CLI tool that runs with the user's own permissions (so the user is "attacking themselves"), and the `--force` flag is required to overwrite existing files, the lack of any path sanitization is inconsistent with the import path validation applied elsewhere.
- Fix: Consider restricting `mds init` to write only to the current directory or a relative path without `..` components. At minimum, warn if the path is absolute or traverses upward:
```rust
if filename.is_absolute() || filename.components().any(|c| c == std::path::Component::ParentDir) {
    return Err(miette::miette!(
        "init filename must be a simple relative path (no '..' or absolute paths)"
    ));
}
```

## Pre-existing Issues (Not Blocking)

No critical pre-existing issues identified. The codebase is new (all code is part of this branch).

## Suggestions (Lower Confidence)

- **YAML deserialization of frontmatter may be vulnerable to entity expansion** - `src/resolver.rs:563` (Confidence: 65%) -- The `serde_yaml` crate is used to parse frontmatter. YAML has known complexity around anchors/aliases that could cause memory amplification (the "billion laughs" attack). `serde_yaml 0.9` has some protections, but the depth of YAML alias expansion is not explicitly bounded here. The `MAX_FILE_SIZE` limit (10 MB) provides an upper bound on input size, which limits the blast radius.

- **No rate limiting on error message content from user-controlled paths** - `src/resolver.rs:74-77` (Confidence: 62%) -- Import paths provided by users are interpolated directly into error messages. While this is standard practice in compilers and there is no injection risk (errors go to stderr, not to a web context), extremely long paths could produce verbose error messages. The `validate_import_path` function does not impose a length limit.

- **`resolve_source` trusts caller-provided `base_dir` for root boundary** - `src/resolver.rs:170-186` (Confidence: 70%) -- When `resolve_source` is called (the in-memory compilation path), the `base_dir` parameter sets the `root_dir` for path traversal checks. If a library consumer passes a permissive `base_dir` (e.g., `/`), the path traversal guard is effectively disabled. This is a library API design concern, not a vulnerability in the CLI tool itself.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 0 | 2 | 0 |
| Should Fix | 0 | 0 | 2 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Security Score**: 8/10

The compiler demonstrates strong security awareness with comprehensive resource exhaustion guards (7 separate `MAX_*` constants), path traversal prevention, symlink rejection, null byte rejection, circular import detection, and file size limits. The defense-in-depth approach (validate_import_path + canonicalize + starts_with root) for path traversal is well-designed. No hardcoded secrets, no `unsafe` blocks, no command injection vectors. The MEDIUM findings are edge cases in otherwise solid security controls.

**Recommendation**: APPROVED_WITH_COMMENTS

The security posture is strong. The two blocking MEDIUM issues (leaf-loop iteration under-counting via function indirection, and the symlink check scope) represent edge cases where existing guards partially mitigate the risk but do not fully close the gap. Neither is immediately exploitable in typical use, but both should be addressed before a 1.0 release. The two should-fix items (unbounded loop in `find_project_root`, `mds init` path validation) are low-risk improvements that align with the project's own security patterns.
