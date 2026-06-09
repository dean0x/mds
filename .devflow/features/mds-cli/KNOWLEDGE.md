---
feature: mds-cli
name: MDS CLI
description: "Use when adding new subcommands, changing output-path resolution logic, modifying the watch architecture (single-file or directory mode), adding new compile paths, updating mds.json config handling, debugging stdout/stderr stream separation, or investigating exit codes. Keywords: mds build, mds check, mds watch, mds init, OutputFormat, messages mode, run_build, run_watch, build.rs, watch.rs, mds.json, output_dir, resolve_output_path, compile_and_write, compile_to_content, debounce, notify, ctrlc, content-dedup, last_written, dirs_to_watch, files_of_interest, exit_code, MAX_FILE_SIZE, read_build_input, BuildArgs, WatchArgs."
category: architecture
directories: [crates/mds-cli/src, crates/mds-cli/tests]
referencedFiles:
  - crates/mds-cli/src/main.rs
  - crates/mds-cli/src/build.rs
  - crates/mds-cli/src/watch.rs
  - crates/mds-cli/tests/cli_watch.rs
  - crates/mds-cli/tests/common/mod.rs
  - crates/mds-cli/Cargo.toml
created: 2026-06-09
updated: 2026-06-09
---

# MDS CLI

## Overview

`mds-cli` is the binary crate that implements the `mds` command-line tool. It has four subcommands — `build`, `check`, `watch`, and `init` — all wired through `main.rs` using clap. The crate is split into three source files: `main.rs` (CLI surface + dispatch), `build.rs` (all shared compile helpers, output-path resolution, and config), and `watch.rs` (the file-watcher loop). This split exists so `watch.rs` can reuse build helpers without duplicating logic or bypassing resource limits.

The crate calls into `mds-core` (aliased as `mds` in Cargo.toml) for all actual compilation. The CLI layer owns: input resolution, output-path computation, project config discovery, runtime-vars merging, stream routing (stdout vs file), exit-code mapping, and the watch event loop.

## System Context

- **mds build** — compiles one `.mds` file (or stdin) to Markdown or JSON messages. Output goes to a file (default: sibling `.md`) or stdout (`-o -`).
- **mds check** — validates without rendering. Always silent on success unless warnings exist; prints `OK: <path>` to stderr on success.
- **mds watch** — long-running watcher: single-file mode tracks transitive imports; directory mode compiles each changed `.mds` independently.
- **mds init** — writes a starter `.mds` template file. Rejects `..` path components in the output filename.

All status messages (banners, warnings, "Compiled to", "Recompiled", "Stopped watching.") go to **stderr**. Compiled content goes to **stdout only when output resolves to stdout** (i.e. `-o -` or stdin input with no output flags). This is a hard invariant — pipe consumers depend on it.

## Component Architecture

### build.rs — shared compile helpers

All `pub(crate)` functions consumed by both `build` and `watch`:

| Function | Purpose |
|---|---|
| `resolve_output_path` | Six-level precedence chain: `-o -` → `-o path` → stdin-default → `--out-dir` → `mds.json` → sibling `.md` |
| `resolve_output_path_no_create` | Same chain but skips `create_dir_all` (pure path computation, used by watch deletion) |
| `load_config` | Walk-up from input file to find `mds.json`; bounded by `MAX_TRAVERSAL_DEPTH`; enforces 1 MB cap on config file |
| `build_runtime_vars` | Merge `--vars` file + `--set KEY=VALUE` overrides into a single `HashMap<String, mds::Value>` |
| `read_build_input` | Read source file or stdin, enforce `MAX_FILE_SIZE` (PF-004 compliance) |
| `compile_to_content` | Compile without writing — returns `CompileOutput { content, dependencies }` |
| `compile_and_write` | Wraps `compile_to_content` + `write_output`; returns dep list for watch resync |
| `write_output` | Write to file or stdout; `announce` flag controls the "Compiled to" banner |
| `auto_detect_mds_file` | Scan cwd for exactly one `.mds` file; errors on zero or many |
| `exit_code` | Map `miette::Error` → 0/1/2/3 (see Exit Codes section) |
| `parse_cli_value` | Coerce `--set VALUE` string to typed `mds::Value` (bool/int/float/array/string) |

### watch.rs — file watcher

The watch loop uses `notify 8` (non-recursive for single-file, recursive for directories) + `ctrlc 3.5`. Events and Ctrl+C are both sent over a single `mpsc::Sender<Msg>` where `Msg` is either `Msg::Fs(notify::Result<Event>)` or `Msg::Interrupt`. This design lets the main loop handle both interrupt and FS events in one `rx.recv()` call.

**Single-file mode** (`run_watch_file`):
1. Load config + resolve output path once at startup.
2. Perform initial compile via `compile_and_write` (announces "Compiled to").
3. Register `notify` watchers on all **parent directories** (not file inodes — survives atomic-rename saves).
4. Record baseline content in `last_written` after watcher registration to suppress macOS synthetic FSEvents.
5. Main loop: on each `Msg::Fs` event, check `event_is_relevant` against `files_of_interest`; if relevant, drain debounce window, then call `compile_to_content`, compare with `last_written`, write only if changed.
6. After each successful rebuild, recompute `dirs_to_watch` and `files_of_interest` from **fresh** dependency output (applies ADR-016: never trust a stale dep set).

**Directory mode** (`run_watch_dir`):
1. Load config once; compile all `.mds` files under root with `collect_mds_files` (depth-bounded at 64).
2. Register a single recursive watcher on root + optional extra watcher for out-of-tree vars file.
3. On events: filter to `.mds` files under root; if vars file changed, recompile all files; otherwise compile/delete individual changed files.
4. Content-dedup (`last_written` keyed by output path) suppresses writes when content is unchanged.
5. Source deletion → `remove_file` on the matching output; fails gracefully (warning, not fatal).

### Output-path resolution precedence

The six-level chain in `resolve_output_path_impl`:

```
1. -o -            → None (stdout)
2. -o <path>       → Some(path)  [wins over mds.json config]
3. stdin + no flags → None (stdout)
4. --out-dir <dir>  → Some(<dir>/<stem>.md)
5. mds.json         → Some(<config_dir>/<output_dir>/<stem>.md)
6. default          → Some(<source_dir>/<stem>.md)
```

`mds.json` is found by walking up from the input file. Its `build.output_dir` field is rejected if it contains `..` components (path traversal guard). The `_no_create` variant does NOT call `create_dir_all` — used by watch to compute a deletion target without side effects.

## Component Interactions

**Compile pipeline boundary**: `mds-cli` never calls `mds::compile` directly with bare file contents that bypass the resource-limit checks. All compile paths flow through either:
- `mds::compile_with_deps(path, ...)` — used for Markdown mode (enforces `MAX_FILE_SIZE` internally through the resolver)
- `read_build_input(path)` → `mds::compile_messages_str_with_deps(source, base_dir, ...)` — used for Messages mode

**PF-004 compliance**: both `compile_to_content` and `read_build_input` carry explicit doc comments marking them as the PF-004 enforcement points. Adding a NEW compile path (e.g. a new subcommand or output mode) MUST route through one of these two entry points. A bare `std::fs::read_to_string` followed by a direct `mds::compile_str` call would bypass the 10 MiB cap.

**Dep tracking**: `compile_and_write` and `compile_to_content` return `dependencies: Vec<String>` (absolute paths). The watch loop uses this to update `dirs_to_watch` and `files_of_interest` on every rebuild. The dep set is NEVER carried forward from a previous rebuild — it is recomputed from scratch each time (ADR-016).

## Exit Codes

`exit_code()` in `build.rs` maps `miette::Error` to:

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Logical/syntax error (undefined var, arity mismatch, recursion, generic miette errors) |
| 2 | I/O / filesystem error (`MdsError::Io`, `FileNotFound`, `NotMdsFile`) |
| 3 | Resource limit exceeded (`MdsError::ResourceLimit`) |

Only `MdsError` values wrapped via `.map_err(miette::Error::from)` downcast correctly. Errors created via `miette::miette!()` macro always produce exit code 1.

## stdout / stderr Stream Contract

This is the most important operational invariant for pipe consumers:

- **stdout**: compiled content ONLY (when `-o -` or stdin with no output flags). No status, no warnings, no error messages.
- **stderr**: everything else — banners, warnings, "Compiled to", "Recompiled", "Stopped watching.", compile errors, "OK:" for check, ANSI clear sequences.
- **`--quiet` (`-q`)**: suppresses banners, warnings, and "Compiled to"/"Recompiled" status lines. Does NOT suppress compile errors (errors always appear on stderr regardless of quiet).
- **`--clear`**: emits `\x1b[2J\x1b[3J\x1b[H` to stderr before each rebuild BUT ONLY when `std::io::stderr().is_terminal()` is true. On piped stderr (CI, scripts) it is a complete no-op.

## Debounce Architecture

Debounce is hand-rolled (notify-debouncer-full deliberately not used — see project decision). The `drain_debounce` function:
- Takes a `debounce_ms` parameter (default 100, `--debounce 0` for immediate rebuilds).
- Computes a `deadline = Instant::now() + Duration::from_millis(debounce_ms)`.
- Loops calling `rx.recv_timeout(remaining)` until deadline or disconnect.
- Returns `(BTreeSet<PathBuf>, interrupted)`.
- The outer loop is bounded by `rx.recv()` blocking semantics — there is no unbounded `while true`.

`--debounce 0` is used in integration tests for determinism (no wait for debounce window).

## mds.json Project Config

`load_config(start: &Path) → Result<Option<(MdsConfig, PathBuf)>>`:
- Walks upward from the input file's directory, checking for `mds.json` at each level.
- Bounded by `MAX_TRAVERSAL_DEPTH` (imported from `mds-core`).
- Enforces a 1 MB cap on the config file itself.
- Returns `(config, config_dir)` where `config_dir` is the directory containing `mds.json` (used to resolve relative `output_dir` values).
- `output_dir` in `mds.json` is the only currently supported field.

`mds.json` example:
```json
{ "build": { "output_dir": "dist" } }
```

This causes `mds build src/prompt.mds` to write `dist/prompt.md` relative to the `mds.json` location, regardless of where the command is invoked.

## Anti-Patterns

- **Bare `std::fs::read_to_string` + direct `mds::compile_str`** — bypasses the `MAX_FILE_SIZE` cap (PF-004). All reads must go through `read_build_input` or `mds::compile_with_deps`.

- **Trusting stale dependency sets in the watch loop** — the dep list from the PREVIOUS rebuild must never be reused as-is for the next cycle. Always recompute from `compile_to_content` output (ADR-016). Using stale deps causes phantom watches on deleted imports or missed watches on newly added imports.

- **Writing compile output to stdout during the watch loop** — only the initial compile (`compile_and_write`) is allowed to write to stdout; subsequent rebuilds compare content and only call `write_output` if changed, with `announce=false` to suppress the duplicate "Compiled to" line. Removing the content-dedup check causes duplicate writes that corrupt downstream pipe consumers.

- **Calling `watcher.watch` recursively for single-file mode** — the watcher must use `RecursiveMode::NonRecursive` for each parent directory, not recursive on the entry's root. Recursive mode on a shared project root would generate massive event noise from unrelated files.

- **Adding a new compile path that creates directories eagerly** — `resolve_output_path_no_create` exists specifically so watch can compute a deletion target without creating directories. Deletion cleanup must use `_no_create`; creation paths use `resolve_output_path`.

- **Using `--format messages` in directory watch mode** — rejected at startup. Multiple `.mds` files cannot map to a single JSON document. Always validate directory-mode constraints before entering the watch loop.

## Gotchas

- **macOS synthetic FSEvents**: on macOS, `notify` delivers synthetic file-modified events for every file in a newly-registered watch directory. Without the `last_written` content-dedup baseline, the watcher immediately recompiles all watched files on startup (producing spurious "Recompiled" lines and duplicate stdout writes). The baseline MUST be recorded after watcher registration and before the main loop processes any events. See QA-R1/R2/R3 tests.

- **Atomic-rename saves (editor save pattern)**: editors like vim and many others save files via rename (write to temp, rename to target). An inode-level file watch is orphaned after the rename. The fix is to watch parent directories, not file inodes. `dirs_to_watch` computes the set of unique parent directories to register.

- **macOS `/tmp` → `/private/tmp` symlink**: `notify` on macOS returns canonical paths (resolving `/tmp` to `/private/tmp`). The `event_is_relevant` function handles this with a `path.canonicalize()` fallback. The `canonicalize_vars_path` helper canonicalizes the vars file path at startup for the same reason.

- **Entry file's parent directory deleted**: if the directory containing the entry file is deleted while watching, the watcher will lose the watch (documented in the `Watch` command's `known limitations` help text). This is not recoverable without restarting.

- **Directory mode stem collision**: two `.mds` files in different subdirectories with the same stem (e.g. `src/a/prompt.mds` and `src/b/prompt.mds`) compiled to a flat `--out-dir` will both map to `prompt.md` — last write wins. Documented in the Watch command's help text.

- **`--format messages` is single-file only**: `--out-dir` in messages mode is silently dropped with a warning (not an error) for `mds build`. For `mds watch`, it is a hard startup error. This asymmetry exists because `mds build` was designed before the constraint was fully specified.

- **`parse_cli_value` rejects non-finite floats**: `NaN`, `Infinity`, `-Infinity` all parse as `f64` but fail `is_finite()` and fall through to `Value::String`. This is by design — numeric operations on NaN/Inf would produce confusing template output.

- **Linux inotify limit**: on Linux, large projects may exhaust `fs.inotify.max_user_watches`. The watcher startup code includes a hint in the error message pointing users to this system parameter.

- **`--debounce 0` in tests is not zero-latency**: even with `--debounce 0`, `drain_debounce` returns an empty set immediately (not a zero-duration window). Tests still need polling loops (`wait_for_file_contains`) because the OS delivers FS events asynchronously.

- **Compile errors during watch are non-fatal**: both single-file and directory modes print the error to stderr and continue watching. The `last_written` entry for that output is NOT updated on error (so the next successful rebuild correctly detects content change).

## Key Files

- `crates/mds-cli/src/main.rs` — CLI surface: clap `Cli`/`Commands` structs, `run()` dispatch, `run_check`, `run_init`
- `crates/mds-cli/src/build.rs` — all shared compile helpers: output-path resolution, `mds.json` config, runtime vars, `compile_to_content`, `compile_and_write`, exit code mapping
- `crates/mds-cli/src/watch.rs` — watch loop: `run_watch` dispatch, `run_watch_file`, `run_watch_dir`, debounce, content-dedup, dir collection
- `crates/mds-cli/tests/cli_watch.rs` — integration tests for `mds watch` (25+ test cases covering all modes, edge cases, and QA regressions)
- `crates/mds-cli/Cargo.toml` — `notify = "8"`, `ctrlc = "3.5"`, `miette` with `fancy` feature

## Related

- **PF-004** (Active): file reads must not bypass the 10 MiB `MAX_FILE_SIZE` cap. `read_build_input` and `mds::compile_with_deps` are the two enforcement points. Any new input path added to the CLI MUST route through one of them. See the `compile_to_content` doc comment.
- **ADR-016** (Active): dynamically-resolved values must be re-validated at runtime. In the watch loop, `files_of_interest` and `dirs_to_watch` are recomputed from fresh `compile_to_content` output after every rebuild — never carried forward from the previous cycle.
- **Project decision**: `notify 8` + `ctrlc 3.5` were selected with MSRV 1.88 (30-day version cooldown). `notify-debouncer-full` was deliberately NOT used; debounce is hand-rolled in `drain_debounce`.
- **Feature: mds-compiler** — the compiler API consumed by the CLI: `mds::compile_with_deps`, `mds::compile_messages_str_with_deps`, `mds::check_collecting_warnings`, `mds::load_vars_file`. The dependency tracking that drives watch resync comes from `compile_with_deps`'s returned `dependencies` field.
