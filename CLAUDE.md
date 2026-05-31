# MDS (Markdown Script)

Composable LLM prompt template compiler. Rust core with WASM and native Node.js bindings.

## Project structure

- `crates/mds-core` — compiler library (published to crates.io as `mds`)
- `crates/mds-cli` — CLI binary (published to crates.io as `mds`)
- `crates/mds-wasm` — WASM bindings (not published to crates.io)
- `crates/mds-napi` — native Node.js bindings via napi-rs (not published to crates.io)
- `packages/mds` — universal npm package (`@mdscript/mds`), native with WASM fallback
- `packages/mds-wasm` — WASM-only npm package (`@mdscript/mds-wasm`)
- `packages/bundler-utils` — shared bundler transform utilities
- `packages/vite-plugin`, `packages/rollup-plugin`, `packages/webpack-loader` — bundler integrations
- `scripts/` — release gates and automation (`verify-versions.mjs`, `verify-napi-names.mjs`, `bump-version.mjs`)

## Build and test

```bash
# Rust
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings

# JS/WASM
npm ci
npm run build -w @mdscript/mds-wasm
npm run build --workspaces --if-present
npm test --workspaces --if-present

# Version consistency check
node scripts/verify-versions.mjs
```

MSRV: 1.88. Workspace panic strategy is `unwind` (required for catch_unwind at JS boundary).

## Release process

All packages ship as a single coordinated release at the same version.

### Automated release (recommended)

```bash
gh workflow run release.yml -f version=X.Y.Z
```

This single command handles everything:
1. **Prepare** — bumps version in workspace Cargo.toml + all 7 package.json files, updates internal `@mdscript/*` dependency ranges to `^X.Y.Z`, stamps CHANGELOG, commits to main, creates and pushes the `vX.Y.Z` tag
2. **Version gate** — asserts all versions match, no `file:` specifiers, internal deps pinned
3. **Build napi** — cross-compiles native addon for 7 targets (macOS arm64/x64, Linux gnu/musl x64/arm64, Windows x64)
4. **Stage + verify** — generates platform npm packages, runs A3 name/loader gate
5. **Publish crates.io** — `mds-core` first, wait for index, then `mds-cli`
6. **Publish npm** — platform packages, host napi, WASM, universal, bundler plugins (all with OIDC provenance)
7. **GitHub Release** — auto-generated release notes

Without the version input (`gh workflow run release.yml`), it runs a dry-run: builds all targets and runs gates but publishes nothing.

### Manual release (alternative)

```bash
node scripts/bump-version.mjs X.Y.Z   # bump all manifests + stamp CHANGELOG
git add -A && git commit -m "release: vX.Y.Z"
git tag vX.Y.Z
git push origin main vX.Y.Z           # tag push triggers release.yml
```

### Required secrets

- `CARGO_REGISTRY_TOKEN` — crates.io publish token
- `NPM_TOKEN` — npm publish token for `@mdscript/*`

### Post-release

- Verify packages on crates.io and npmjs.com (check provenance attestation)
- Smoke test: `npm i @mdscript/mds && node -e "import('@mdscript/mds').then(m=>m.init())"`
- The CHANGELOG `[Unreleased]` section is ready for the next cycle

### Cross-compilation notes

- aarch64-unknown-linux-gnu: uses apt `gcc-aarch64-linux-gnu`
- aarch64-unknown-linux-musl: uses zig as linker via `setup-zig`
- x86_64 linux targets: use napi `--use-napi-cross`
- macOS/Windows: native toolchains

## Key conventions

- All npm packages are scoped under `@mdscript/`
- Workspace Cargo.toml version is the single source of truth
- `scripts/verify-versions.mjs` (D1 gate) and `scripts/verify-napi-names.mjs` (A3 gate) are critical safety checks
- `cargo publish` steps are idempotent (tolerate "already published")
- `NPM_CONFIG_ACCESS=public` is required for scoped package provenance
