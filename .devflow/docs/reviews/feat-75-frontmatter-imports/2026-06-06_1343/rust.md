# Rust Review Report

**Branch**: feat-75-frontmatter-imports -> main
**Date**: 2026-06-06

## Issues in Your Changes (BLOCKING)

### CRITICAL

(none)

### HIGH

**Blank lines in `imports:` block prematurely terminate stripping in `strip_reserved_keys`** - `crates/mds-core/src/lib.rs:405`
**Confidence**: 85%
- Problem: The `is_top_level` check treats empty/blank lines as top-level keys (`!line.starts_with(' ') && !line.starts_with('\t')` is true for `""`). If a user writes a blank line within the `imports:` YAML block:
  ```yaml
  imports:
    - path: ./a.mds

    - path: ./b.mds
  ```
  The blank line resets `in_imports_block = false`, and the subsequent indented lines (`  - path: ./b.mds`) fall through to the output instead of being stripped. Meanwhile `build_scope_from_frontmatter` parses the YAML correctly via serde, so the import _resolves_ but the path _leaks_ into the compiled frontmatter output.
- Fix: Add an empty-line guard before the top-level check:
  ```rust
  let trimmed = line.trim();
  if trimmed.is_empty() {
      if in_imports_block {
          continue; // blank lines inside imports block are also stripped
      }
      filtered.push_str(line);
      filtered.push('\n');
      continue;
  }
  ```

### MEDIUM

**Duplicate names in selective import `names` list silently accepted** - `crates/mds-core/src/resolver.rs:1106-1118`
**Confidence**: 82%
- Problem: The `names` vector in `parse_frontmatter_imports_from_yaml` does not check for duplicate entries. `names: [greet, greet]` would be accepted, and at resolution time `scope.set_function` would be called twice for the same name -- a no-op overwrite but a signal of user error that should be caught at parse time.
- Fix: Use a `HashSet` to detect duplicates during the names loop:
  ```rust
  let mut seen = HashSet::with_capacity(names_seq.len());
  for name_val in names_seq {
      // ... existing validation ...
      if !seen.insert(name.clone()) {
          return Err(err(&format!("duplicate name '{name}' in 'names'")));
      }
      names.push(name.clone());
  }
  ```

**Non-string YAML keys in import entries silently ignored during unknown-key check** - `crates/mds-core/src/resolver.rs:1068-1071`
**Confidence**: 80%
- Problem: The unknown-key validation loop skips non-string keys with `continue`, meaning `{ path: ./lib.mds, 42: something }` passes validation. While YAML integer keys are rare in practice, strict validation would reject them as unexpected input. This is consistent with how the top-level frontmatter loop handles non-string keys (line 785), but for the tightly-scoped import entry format, rejecting non-string keys is more appropriate.
- Fix: Return an error instead of continuing:
  ```rust
  let serde_yaml_ng::Value::String(key_str) = k else {
      return Err(err("all keys must be strings"));
  };
  ```

## Issues in Code You Touched (Should Fix)

(none)

## Pre-existing Issues (Not Blocking)

### MEDIUM

**`has_type_mds_frontmatter` and `has_type_mds_frontmatter_raw` match indented `type: mds`** - `crates/mds-core/src/resolver.rs:896,912`
**Confidence**: 80%
- Problem: Both functions use `line.trim()` before checking for `type:` prefix, which means an indented `type: mds` inside a nested YAML mapping (e.g., `config:\n  type: mds`) would trigger a false-positive match, incorrectly classifying the file as MDS. The `strip_reserved_keys` function in `lib.rs` correctly only strips top-level `type: mds` lines. This pre-existing inconsistency is inherited by the new `has_type_mds_frontmatter_raw` function.
- Fix: Remove the `trim()` call and check for top-level keys only (no leading whitespace), consistent with `strip_reserved_keys`.

## Suggestions (Lower Confidence)

- **YAML comment lines within `imports:` block** - `crates/mds-core/src/lib.rs:403-427` (Confidence: 65%) -- A YAML comment line like `# divider` within the imports block would be treated as top-level (starts with `#`, not space/tab), resetting `in_imports_block`. While unlikely in practice, the stripping logic could be made more robust by also treating comment-only lines as continuations when `in_imports_block` is true.

- **`parse_frontmatter_imports` double-parses YAML** - `crates/mds-core/src/resolver.rs:1135-1147` (Confidence: 70%) -- This function parses YAML from the raw string independently of `build_scope_from_frontmatter`, which already parses the same YAML. It is called from `scan_imports` where the full scope is not available, so the double-parse is intentional, but a note in the doc comment clarifying this design choice would prevent future confusion.

- **No bound on `names` list length in selective import** - `crates/mds-core/src/resolver.rs:1106` (Confidence: 60%) -- While `MAX_FRONTMATTER_IMPORTS` bounds the number of import entries, there is no bound on how many names a single selective import can request. A pathologically large `names: [a, b, c, ...]` list (thousands of entries) would be accepted. In practice, this is bounded by the number of exports in the target module.

## Summary

| Category | CRITICAL | HIGH | MEDIUM | LOW |
|----------|----------|------|--------|-----|
| Blocking | 0 | 1 | 2 | 0 |
| Should Fix | 0 | 0 | 0 | 0 |
| Pre-existing | 0 | 0 | 1 | 0 |

**Rust Score**: 8/10
**Recommendation**: CHANGES_REQUESTED

The implementation is well-structured with thorough validation, proper error handling using `Result` types throughout (applies ADR principles from CLAUDE.md), defense-in-depth limits (`MAX_FRONTMATTER_IMPORTS`), and comprehensive test coverage (20+ new tests across unit and integration layers). The `FrontmatterImport` enum cleanly models the three import forms as a state machine, making illegal states unrepresentable. The HIGH-severity blank-line stripping bug is the primary concern -- while the YAML _parsing_ path handles this correctly, the output _stripping_ path in `strip_reserved_keys` would leak import paths into compiled output for YAML with blank lines within the imports block.
