# Desktop clippy + rustfmt cleanup (round 76)

Date: 2026-11-12 (America/Sao_Paulo)
Workspace: `C:\Users\ricar\dev\LovelaceSharp`, HEAD `07af1bb`
Toolchain: rustc/cargo 1.99.1, rustfmt 1.10.0-stable
Scope: `rust/lovelace-desktop/src` only — the two clippy sites plus `cargo fmt -p lovelace-desktop`.
No source file outside `rust/lovelace-desktop` was modified by this task (only this report was added under `docs/`).

## Ownership / mtime check

| file | first read (UTC) | hashed immediately before edit (UTC) | after edit (UTC) | after fmt (UTC) |
|---|---|---|---|---|
| `src/execution.rs` | 12:45:28.9109612 | 12:45:28.9109612 | 13:06:21.2148968 | 13:06:47.8447571 |
| `src/ui/plot.rs` | 12:36:15.4141375 | 12:36:15.4141375 | 13:06:21.2522766 | 13:06:47.9058034 |

Pre-edit SHA-256 (unchanged between first read and edit):

- `execution.rs` `193D0B3A986EA1DE73E91B4631AC6C441E62B83115821FB3F41B787077749318`
- `plot.rs` `975F35B3167A8596B8103EAB5488A2D1A7ECAE3F7E5453440014F810358531B0`

Neither lint file changed under me during the edit window. The owner session already had uncommitted work in
`engine.rs`, `gui_selftest.rs`, `main.rs`, `settings.rs`, `ui/app.rs`, `ui/editor.rs`, `workbench.rs`
(and untracked `keyboard.rs`); my only content edits are the two fixes below. rustfmt reflowed those files
formatting-only when the crate was formatted.

## Fix 1 — `manual_unwrap_or_default` (`src/execution.rs`)

Clippy diagnostic:

```text
error: match can be simplified with `.unwrap_or_default()`
   --> lovelace-desktop\src\execution.rs:652:21
    = note: `-D clippy::manual-unwrap-or-default` implied by `-D warnings`
```

Before:

```rust
        let lines = match panic::catch_unwind(AssertUnwindSafe(|| engine.take_output())) {
            Ok(lines) => lines,
            Err(_) => Vec::new(),
        };
```

After (as rustfmt left it):

```rust
        let lines =
            panic::catch_unwind(AssertUnwindSafe(|| engine.take_output())).unwrap_or_default();
```

Behaviour unchanged: the panic payload is still discarded and `Vec::new()` is `Vec::default()`.

## Fix 2 — `manual_clamp` (`src/ui/plot.rs`)

Clippy diagnostic:

```text
error: clamp-like pattern without using clamp function
  --> lovelace-desktop\src\ui\plot.rs:98:15
   = note: clamp will panic if max < min, min.is_nan(), or max.is_nan()
   = note: clamp returns NaN if the input is NaN
```

Before:

```rust
    let cap = max_side.min(MAX_SIDE).max(64.0);
```

After:

```rust
    let cap = max_side.clamp(64.0, MAX_SIDE);
```

`MAX_SIDE` is `4096.0`, so `min <= max`; the only caller passes
`ctx.input(|input| input.max_texture_side).max(512) as f32`, which cannot be NaN, so the cap is
identical for every reachable input.

## Formatting

`cargo fmt -p lovelace-desktop` exit 0. It reformatted these 11 files (formatting only, verified by
before/after SHA-256 snapshots):

```text
engine.rs  execution.rs  gui_selftest.rs  main.rs  settings.rs  statements.rs
ui/app.rs  ui/editor.rs  ui/plot.rs  ui/syntax.rs  workbench.rs
```

`cargo fmt --all -- --check` now exit 0 (it previously reported diffs only in `rust/lovelace-desktop`).

## Gates

1. Clippy:

```text
$ cargo clippy --workspace --all-targets -- -D warnings
    Checking lovelace-desktop v0.1.0 (C:\Users\ricar\dev\LovelaceSharp\rust\lovelace-desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.94s
CLIPPY_EXIT=0
```

2. Formatting:

```text
$ cargo fmt --all -- --check
FMT_CHECK_EXIT=0
```

(no diff output)

3. Desktop tests:

```text
$ cargo test -p lovelace-desktop
running 105 tests
test result: ok. 105 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.61s
TEST_DESKTOP_EXIT=0
```

The brief expected 90 tests; the crate now carries 105 because the owner session added tests
(`keyboard`, expanded `app`/`workbench`/`gui_selftest`). All pass.

4. Workspace tests:

```text
$ cargo test --workspace
WORKSPACE_TEST_EXIT=0
```

Suite results (all `ok`): array 58, bench 0, corpus 18 passed / 1 ignored, desktop 105, int 125,
json 7, lang 1562, nat 264, real 2485, run 413, recursion_depth 1, symbolics 181.
Total: **5219 passed, 1 ignored, 0 failed**.

The workspace run executed the other active sessions' in-flight changes too (parity round 76,
`lovelace-lang`/`lovelace-symbolics`), which were green at run time and were not touched by this task.
