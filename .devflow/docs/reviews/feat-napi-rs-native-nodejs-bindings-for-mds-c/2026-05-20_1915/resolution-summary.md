# Resolution Summary

**Branch**: feat/napi-rs-native-nodejs-bindings-for-mds-c -> main
**Date**: 2026-05-21
**Review**: .devflow/docs/reviews/feat-napi-rs-native-nodejs-bindings-for-mds-c/2026-05-20_1915
**Command**: /resolve

## Statistics
| Metric | Value |
|--------|-------|
| Total Issues | 21 |
| Fixed | 18 |
| False Positive | 0 |
| Deferred | 2 |
| Blocked | 0 |

## Fixed Issues

### Batch A — lib.rs FFI safety & structure
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Unchecked napi_create_string_utf8 return status | lib.rs:94-106 | 0918d38 |
| Missing // SAFETY: comments on 5 unsafe sites | lib.rs:85,112,127,150,197 | 0918d38 |
| throw_mds_error nesting depth 5 → extracted raw_create_span_obj | lib.rs:146-182 | 0918d38 |
| span.offset as u32 silent truncation → u32::try_from | lib.rs:160-161 | 0918d38 |
| debug-panics detail inconsistency → err.detail property | lib.rs:219-233 | 0918d38 |

### Batch B — Config files
| Issue | File:Line | Commit |
|-------|-----------|--------|
| MSRV 1.88 documented (napi 3.9.0 requires it) | Cargo.toml:8 | 968d029 |
| Missing workspace metadata (readme, keywords, categories) | mds-napi/Cargo.toml | 968d029 |
| Missing codegen-units = 1 in release profile | Cargo.toml:49-51 | 968d029 |
| Unnecessary extern crate in build.rs | build.rs:1 | 968d029 |

### Batch C — Tests part 1
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Vacuous assertion E-5 (help property) → unconditional | index.spec.mjs:251-262 | 99f6dae |
| Vacuous assertion E-8 (span property) → unconditional | index.spec.mjs:284-296 | 99f6dae |
| F-CF6 swallowed errors → deterministic assertion | index.spec.mjs:123-138 | 99f6dae |
| Missing test: checkFile with vars (F-K10) | index.spec.mjs | 99f6dae |
| Missing test: check result shape (F-K11) | index.spec.mjs | 99f6dae |

### Batch D — lib.rs options
| Issue | File:Line | Commit |
|-------|-----------|--------|
| parse_compile_opts 51 lines → extracted extract_base_path | lib.rs:304-354 | 52ed21f |

### Batch E — Tests part 2
| Issue | File:Line | Commit |
|-------|-----------|--------|
| Missing test: basePath non-string (V-7) | index.spec.mjs | 3f9f0f2 |
| Missing test: vars as array (V-8) | index.spec.mjs | 3f9f0f2 |
| Missing test: unknown keys check/checkFile (V-9, V-10) | index.spec.mjs | 3f9f0f2 |

## Deferred to Tech Debt
| Issue | File:Line | Risk Factor |
|-------|-----------|-------------|
| Duplicated vars-parsing logic between mds-napi and mds-wasm | lib.rs:270-395, mds-wasm | Cross-crate architectural refactor — requires shared module in mds-core |
| Options deserialization double-traversal | lib.rs:310,367 | Future performance optimization — negligible for typical small options objects |

## False Positives

(none)

## Blocked

(none)
