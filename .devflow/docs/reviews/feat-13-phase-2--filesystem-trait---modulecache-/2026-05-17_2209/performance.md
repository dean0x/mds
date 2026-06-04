# Performance Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**NativeFs::normalize does two canonicalize() syscalls on every call, including cache hits** - `crates/mds-core/src/fs.rs:267`
**Confidence**: 85%
- Problem: The old `resolve()` method called `canonicalize_and_check()` before the cache lookup, paying two `canonicalize()` syscalls even on cache hits. The new code moves canonicalization into `NativeFs::normalize()`, which is called from `resolve_path()` (line 114 in resolver.rs) and `resolve_import_from()` (line 211) — both of which run **before** the cache lookup in `resolve_by_key()`. This means the expensive double-canonicalize pattern (parent + full path at fs.rs:184-191) still executes on every resolution attempt, even when the module is already cached. For projects with many imports of the same module, this is a regression in the same class as the prior code — the trait boundary did not change the call order, but it also did not improve it. The `check_symlink` method calls `parent.canonicalize()` and `full_path.canonicalize()` — two filesystem syscalls (~100us each on HDD) per call.
- Fix: Consider a two-phase design: a cheap key normalization that skips canonicalization for paths that are already canonical (absolute and no symlink components), reserving full canonicalization for the read path. Alternatively, add a `normalize_cache: HashMap<String, String>` inside `NativeFs` (behind a `RefCell` or similar) so repeat normalizations of the same path hit an in-memory lookup instead of the OS.

```rust
// Sketch: Add a normalization cache inside NativeFs
pub struct NativeFs {
    root_dir: OnceLock<PathBuf>,
    norm_cache: RefCell<HashMap<String, String>>,  // relative -> canonical
}

fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
    // ... null-byte check, path construction ...
    let path_str = path.display().to_string();
    if let Some(cached) = self.norm_cache.borrow().get(&path_str) {
        return Ok(cached.clone());
    }
    let canonical = Self::check_symlink(&path)?;
    // ... root init, traversal check ...
    let key = canonical.display().to_string();
    self.norm_cache.borrow_mut().insert(path_str, key.clone());
    Ok(key)
}
```

**NativeFs::read performs redundant metadata + read double I/O** - `crates/mds-core/src/fs.rs:284-301`
**Confidence**: 82%
- Problem: `NativeFs::read()` calls `std::fs::metadata()` (line 284) to pre-check the file size, then calls `std::fs::read()` (line 293) which reads the entire file, and then re-checks the byte count (line 295). The metadata pre-check is described as an optimization to "avoid allocating memory for files that will be rejected", but in the normal case (files under 10 MB), this means two syscalls per read instead of one. The old code in `read_validated_file` used a single `std::fs::read()` plus a post-read size check — one fewer syscall per file. The metadata call introduces an additional TOCTOU window (file could change between metadata and read), so the defense-in-depth rationale is weakened.
- Fix: Remove the metadata pre-check and rely solely on the post-read size check, matching the old pattern. The memory allocation for a 10 MB file is bounded and acceptable.

```rust
fn read(&self, normalized: &str) -> Result<String, MdsError> {
    let path = Path::new(normalized);
    let bytes = std::fs::read(path)
        .map_err(|e| MdsError::io(format!("cannot read {normalized}: {e}")))?;
    if bytes.len() as u64 > MAX_FILE_SIZE {
        return Err(MdsError::resource_limit(format!(
            "file too large ({} bytes, max {} bytes): {normalized}",
            bytes.len(),
            MAX_FILE_SIZE,
        )));
    }
    String::from_utf8(bytes)
        .map_err(|e| MdsError::io(format!("invalid UTF-8 in {normalized}: {e}")))
}
```

### MEDIUM

**VirtualFs::read clones entire file content on every cache miss** - `crates/mds-core/src/fs.rs:142`
**Confidence**: 85%
- Problem: `VirtualFs::read()` returns `Ok(content.clone())` which clones the entire file content string. When the same module is imported from multiple files, the `resolve_by_key` cache prevents re-reads, but on the first miss the full content is cloned from the HashMap. Since VirtualFs stores `HashMap<String, String>`, the clone is an O(n) operation proportional to file size. For large virtual module sets (e.g., WASM environments with many modules), this is an unnecessary allocation.
- Fix: Consider storing content as `Arc<str>` or `Arc<String>` in VirtualFs so reads return a cheap Arc clone. Alternatively, accept this as a reasonable trade-off for simplicity since each module is only read once (the ModuleCache caches at the resolved module level, not the raw content level).

**compile_virtual takes HashMap by value, forcing caller allocation** - `crates/mds-core/src/lib.rs:441`
**Confidence**: 80%
- Problem: `compile_virtual()` and `compile_virtual_collecting_warnings()` take `modules: HashMap<String, String>` by value. This means the caller must move or clone the entire HashMap. If the caller needs to compile multiple entry points from the same module set, they must clone the entire HashMap each time. This is an API design choice that can cause unnecessary allocation.
- Fix: This is acceptable for v1 of the API since most callers will have a single entry point. If multi-entry-point usage becomes common, consider adding an overload that takes `&HashMap<String, String>` or changing `VirtualFs` to use `Arc<HashMap<String, String>>`.

## Issues in Code You Touched (Should Fix)

_(none)_

## Pre-existing Issues (Not Blocking)

_(none)_

## Suggestions (Lower Confidence)

- **Box<dyn FileSystem> dynamic dispatch overhead** - `crates/mds-core/src/resolver.rs:47` (Confidence: 60%) — The `fs` field uses `Box<dyn FileSystem>`, introducing vtable indirection on every `normalize()`, `read()`, and `is_markdown()` call. In hot paths with many imports this adds one indirect branch per call. However, the overhead is negligible compared to actual filesystem I/O and string processing. Consider using an enum-based dispatch (`enum Fs { Native(NativeFs), Virtual(VirtualFs) }`) only if profiling shows this matters.

- **PathBuf-to-String key conversion on every resolution** - `crates/mds-core/src/resolver.rs:113,154,192` (Confidence: 65%) — The new code converts between `Path::display().to_string()` (resolver.rs:113), `key.to_string()` for insertion into the resolving set (line 154), and `key.to_string()` for cache insertion (line 192). The old code used `PathBuf` natively. While individual allocations are small, they add up in deep import trees. The old code had similar cloning costs with PathBuf, so this is a lateral move rather than a regression.

- **find_project_root scans up to 256 directories on first resolve** - `crates/mds-core/src/fs.rs:206-218` (Confidence: 65%) — `find_project_root` walks up the directory tree checking for `.git` or `.mdsroot` markers, calling `dir.join(marker).exists()` up to 512 times (256 iterations x 2 markers). This only runs once per `NativeFs` instance (guarded by `OnceLock`), so the amortized cost is negligible. Pre-existing pattern from main.

## Summary
| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Performance Score**: 7/10
**Recommendation**: APPROVED_WITH_CONDITIONS

The FileSystem trait abstraction is well-designed for its purpose (enabling VirtualFs for WASM/testing). The key performance consideration is the `NativeFs::normalize` canonicalization cost on cache hits and the redundant metadata pre-check in `NativeFs::read`. The canonicalization-before-cache-check pattern is inherited from the old code, so it is not a regression, but the trait boundary creates a natural opportunity to fix it. The metadata double-read is new and should be simplified. Overall, the performance characteristics are acceptable for a module compiler that processes tens to low hundreds of files.
