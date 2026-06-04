# Reliability Review Report

**Branch**: feat/13-phase-2--filesystem-trait---modulecache- -> main
**Date**: 2026-05-17

## Issues in Your Changes (BLOCKING)

### HIGH

**VirtualFs::normalize lacks a segment count bound** - `crates/mds-core/src/fs.rs:102-118`
**Confidence**: 85%
- Problem: The `for part in relative.split('/')` loop in `VirtualFs::normalize` iterates over every segment in the `relative` path string with no upper bound on the number of segments. While this loop is bounded by the input length (it cannot iterate more times than there are `/` characters), it also pushes segments onto a `Vec` without any cap. An adversarial input like `"a/".repeat(100_000)` creates a 100,000-element `Vec<&str>` and then joins them into a single key string. The `VirtualFs::read` method will reject unknown keys with `FileNotFound`, but the allocation itself is unbounded.
- Impact: Excessive memory allocation during normalization before the file-existence check runs. In a WASM environment where `VirtualFs` is the primary backend, this could be used for denial-of-service via large crafted import paths.
- Fix: Add a `MAX_DOT_SEGMENTS` bound (the project already defines `MAX_DOT_SEGMENTS=32` per FEATURE_KNOWLEDGE) to cap the number of path segments processed:
  ```rust
  const MAX_PATH_SEGMENTS: usize = 256;

  fn normalize(&self, base: &str, relative: &str) -> Result<String, MdsError> {
      // ... existing validation ...

      let mut count = 0;
      for part in relative.split('/') {
          count += 1;
          if count > MAX_PATH_SEGMENTS {
              return Err(MdsError::resource_limit(
                  "import path has too many segments".to_string(),
              ));
          }
          match part {
              // ... existing match arms ...
          }
      }
      // ...
  }
  ```

**`resolve_source` bypasses depth guard for the root module** - `crates/mds-core/src/resolver.rs:235-265`
**Confidence**: 82%
- Problem: `resolve_source` calls `process_module` directly without going through `resolve_by_key`, which means the root module processed by `resolve_source` is never pushed onto the `resolving` stack and never checked against `check_import_depth`. While imports within that module *do* go through `resolve_by_key` (which applies the depth guard), the root module itself is invisible to cycle detection. If a module imported from `resolve_source` re-imports the same source path via a different normalized key, the cycle would not be detected since the root is not in the `resolving` set. This is a pre-existing pattern carried forward, but the new trait abstraction makes it more likely to be exercised since `resolve_source` now delegates `set_root` to the FileSystem trait which may have different normalization behavior.
- Impact: Potential for undetected cycles when `resolve_source` is used as entry point with certain filesystem implementations. The stack overflow from unbounded recursion would crash the process.
- Fix: Push a synthetic key onto `resolving` before calling `process_module` in `resolve_source`, and pop it after (mirroring the pattern in `resolve_by_key`):
  ```rust
  pub fn resolve_source(&mut self, source: &str, base_dir: &Path, ...) -> ... {
      // ... existing canonicalization ...
      let base_key = format!("{canonical_str}/<source>");

      self.check_import_depth()?;
      self.resolving.insert(base_key.clone());

      let ctx = ModuleCtx { ... };
      let resolved = self.process_module(&ctx, false, warnings);

      let popped = self.resolving.pop();
      debug_assert_eq!(popped.as_deref(), Some(base_key.as_str()));

      resolved.map(Arc::new)
  }
  ```

### MEDIUM

**VirtualFs::read clones the entire content string on every read** - `crates/mds-core/src/fs.rs:130-143`
**Confidence**: 83%
- Problem: `VirtualFs::read` calls `content.clone()` to return an owned `String`. For content at or near the 10 MB limit, this allocates a full copy of the content even though the original is already in memory. While the file size limit caps the worst case at 10 MB, repeated reads of large modules (before caching kicks in at the `ModuleCache` level) could allocate significant memory. The `ModuleCache` does cache resolved modules (so `read` is called at most once per unique key), which mitigates this for the common case.
- Impact: Up to 10 MB allocation per unique module read. Bounded by `MAX_FILE_SIZE` but still a non-trivial allocation per module.
- Fix: Consider changing `VirtualFs` to store `Arc<String>` or `Arc<str>` internally so that reads are O(1) clones:
  ```rust
  pub struct VirtualFs {
      modules: HashMap<String, Arc<String>>,
  }
  // read() returns Arc::clone() then .to_string() or the trait returns Arc<str>
  ```
  This is a design consideration for a future iteration rather than a blocking concern, since the 10 MB cap provides a hard bound.

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

(none)

## Suggestions (Lower Confidence)

- **Custom FileSystem implementations have no runtime enforcement of the security contract** - `crates/mds-core/src/fs.rs:14-35` (Confidence: 70%) -- The `FileSystem` trait documents a security contract (traversal prevention, null-byte rejection, file size limits) but `with_fs()` accepts any `Box<dyn FileSystem>` without runtime validation. A wrapper that enforces these invariants regardless of implementation would provide defense-in-depth. Since this is documented as a "MUST" obligation, consider a validating wrapper or integration test harness for custom implementations.

- **`NativeFs::read` pre-check metadata size can diverge from actual read size** - `crates/mds-core/src/fs.rs:280-304` (Confidence: 65%) -- The metadata pre-check and post-read check create a TOCTOU window where the file could grow between `metadata()` and `read()`. The code already has the post-read check as defense-in-depth which is good practice, but the comment says "Pre-check size via metadata to avoid allocating memory for files that will be rejected anyway" -- however, `std::fs::read` will still allocate based on the actual file size, not the metadata size. The defense-in-depth post-read check is the real guard here; the pre-check is a best-effort optimization that works in the common case.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 2 | 1 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 0 | 0 |

**Reliability Score**: 7/10
**Recommendation**: CHANGES_REQUESTED

The code demonstrates strong reliability fundamentals: all loops in `NativeFs::find_project_root` are bounded by `MAX_TRAVERSAL_DEPTH`, the `resolve_by_key` recursion is capped by `MAX_IMPORT_DEPTH` with cycle detection, file sizes are bounded by `MAX_FILE_SIZE` with defense-in-depth (pre-check + post-check), and the LIFO invariant on the resolving stack is explicitly verified with error reporting rather than silent corruption. The `VirtualFs::read` correctly enforces the same file size limit as `NativeFs`. The two HIGH findings -- unbounded segment allocation in `VirtualFs::normalize` and the `resolve_source` depth-guard bypass -- are the primary reliability gaps. Both are addressable with small, targeted changes.
