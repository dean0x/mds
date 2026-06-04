# Release v0.1.0 — baseline metrics (Step 0.2)

Captured 2026-05-29 on branch `release-prep-v0.1.0` before Phase A–D work.

- **Rust tests:** `cargo test --workspace` → 590 pass, 0 fail, 0 skip
- **JS tests:** `npm test --workspaces` → 224 pass, 0 fail (default backend)
- **WASM artifact:** `mds_wasm_bg.wasm` = 469531 bytes (~458.5 KB), identical for
  `pkg/` (nodejs) and `pkg-web/` (web). `wasm-opt` currently disabled.

These are the regression floor: post-change counts must be ≥ these, wasm size not
grossly regressed.
