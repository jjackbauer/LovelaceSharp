# Desktop clippy + rustfmt cleanup, second pass (round 101)

Date: 2026-11-12 (America/Sao_Paulo)
Workspace: `C:\Users\ricar\dev\LovelaceSharp`, HEAD `9e8f874`
Toolchain: rustc/cargo 1.99.1, rustfmt 1.10.0-stable
Scope: `rust/lovelace-desktop/src` only — four lint files (six clippy diagnostics) plus
`cargo fmt -p lovelace-desktop`. No source outside `rust/lovelace-desktop` was modified; this
report is the only file added under `docs/`.

## Brief vs. actual state

The brief expected two clippy suggestions and rustfmt diffs only in `completion.rs` (5 sites).
That picture was stale: HEAD `9e8f874` ("Desktop v4: round-30 final accounting...") was committed
at 21:55:49Z, about 39 s before this task's first snapshot, and carried:

- six clippy diagnostics in four files — the bin target failed at the first two, and
  `--all-targets` then found four more in the test target;
- 150 rustfmt diff sites in 19 files (`ui/app.rs` 54, `workbench.rs` 32, ...; `completion.rs` 5).

All six diagnostics and all 150 format sites are fixed. No source files remain.

## Ownership / mtime check

A SHA-256 snapshot of all 38 files under `src/` was taken at 21:57:37Z and matched again
immediately before the first edit. No desktop source changed under me during the task:

- HEAD stayed `9e8f874` before, during and after the run.
- The last owner write to any desktop source was `journey.rs` at 21:50:32Z, before the
  21:55:49Z commit.
- The post-format snapshot was still identical after clippy, `cargo test -p lovelace-desktop`
  and `cargo test --workspace`.

The five files named below are the lint files plus `completion.rs` (the brief's only named fmt
file); all 38 hashes matched before editing.

| file | baseline mtime (UTC) | pre-edit SHA-256 | post-fmt SHA-256 |
|---|---|---|---|
| `history.rs` | 2026-11-12T16:15:15.1915490Z | `A978E08A2749A57A2BC8101EF05E6BA0CF0B5657964A7FDC782A7FF66A7220A9` | `CCA16958B2BC7B69BE15395EC4980FF6F78921CAD5B53ECD55AFC758D1C8A655` |
| `run_state.rs` | 2026-11-12T21:18:12.1221255Z | `6ACEF09233ADCCFE64B63FCEDE0623FC87D66AB8747A867FC2C09EF364958A8D` | `1AA31A200B4070136F6E15A08C0934351E6186749573B4741FB0D419255E82F3` |
| `ui/plot.rs` | 2026-11-12T21:18:12.1221255Z | `FBE18C2627517F835E1FC6B7E9542F840CD075AFA084F844E975C03D063C7AD6` | `D3D41631B67D116F456EA6C1B99E1990282FF7822D0DE9E87250F5BB6AA6EFFE` |
| `workbench.rs` | 2026-11-12T21:27:28.2914360Z | `ABEA7D24A186A8159202D3AE3D791574799FC678DEB628D9E2F98344E739F1F2` | `1BFB0CB35BEF91C858A594BAEC87024B8108C8C1210F147248EA90FA7DDCA65B` |
| `completion.rs` | 2026-11-12T21:18:12.1181283Z | `ECDD0F354E3922347379C0BB5187E08AACF3D9490407EA06AB57FE96454CE3A0` | `E76CAE2AA8A76421DD85E1BBF7AC925E2147A164442308297AB2EF6EED6CA89D` |

## Lint fixes (4 files, 6 diagnostics)

| file (post-fmt line) | diagnostic line | clippy lint | fix |
|---|---|---|---|
| `history.rs:114` | 124 | `derivable_impls` | added `Default` to the derive; deleted the manual `impl Default` |
| `run_state.rs:344` | 349 | `match_like_matches_macro` | `match (from, to) { ... }` became `matches!((from, to), ...)` |
| `ui/plot.rs:1467` | 1445 | `needless_borrows_for_generic_args` | dropped `&` before `directory.join("exported.png")` |
| `workbench.rs:2627` | 2616 | `unnecessary_map_or` | `.map_or(false, ..)` → `.is_some_and(..)` |
| `workbench.rs:4016` | 4011 | `unnecessary_map_or` | `.map_or(true, ..)` → `.is_none_or(..)` |
| `workbench.rs:5117` | 5041 | `needless_late_init` | moved `let record` to its first assignment |

Behaviour notes:

- **history.rs** — the derived `Default` produces exactly the removed manual body:
  two empty `Vec`s, `typing_open: false`, `last_typing: None`.
- **run_state.rs** — `allowed` keeps the identical arm patterns, now inside `matches!`;
  the returned `bool` is unchanged for every `(from, to)` pair.
- **ui/plot.rs** — `Pixmap::load_png` takes `impl AsRef<Path>`; the `PathBuf` is passed by
  value instead of by reference. Same path, same call.
- **workbench.rs** — the two `map_or` predicates are the direct `is_some_and` /
  `is_none_or` equivalents (`Some(f)` → `f(x)`, `None` → `false`/`true`). The late-init
  move declares `record` at its first assignment; it has no reader before that point and no
  `Drop` side effects, so the reordering is unobservable.

## Formatting

`cargo fmt -p lovelace-desktop` exit 0. It reflowed 19 files; 16 were formatting-only:

```text
completion.rs  document_io.rs  engine.rs  engine/fake.rs  engine/real.rs
execution.rs  gui_selftest.rs  keyboard.rs  main.rs  recovery.rs
results_tree.rs  settings.rs  ui/app.rs  ui/editor.rs  ui/file_dialogs.rs
win32/ffi.rs
```

The other three formatted files (`run_state.rs`, `ui/plot.rs`, `workbench.rs`) also carry lint
edits; `history.rs` was already rustfmt-clean and carries only its lint edit. Total source files
changed: **20** (16 format-only + 1 lint-only + 3 lint-and-format).

`cargo fmt --all -- --check` exit 0 (previously 150 diffs, all in `lovelace-desktop`).

## Gates

1. Clippy:

```text
$ cargo clippy --workspace --all-targets -- -D warnings
    Checking lovelace-desktop v0.1.0 (C:\Users\ricar\dev\LovelaceSharp\rust\lovelace-desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.98s

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
running 355 tests
test result: ok. 355 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.16s
TEST_DESKTOP_EXIT=0
```

4. Workspace tests:

```text
$ cargo test --workspace
...
TOTAL passed=5666 failed=0 ignored=1
TEST_WORKSPACE_EXIT=0
```

Suite breakdown (all `ok`): array 61, bench 0, corpus 18 passed / 1 ignored, desktop 355,
int 129, json 7, lang 1719, nat 264, real 2487, run 429, recursion_depth 1, symbolics 196.
Total: **5666 passed, 1 ignored, 0 failed**.

## Notes

- `rust/Cargo.lock` was already modified when the task began; this task did not write it.
- Other sessions changed `parity/*` and `rust/lovelace-real/src/lib.rs` during the window; none
  of those files were touched by this task. `lovelace-real/src/lib.rs` changed at 22:01:21Z,
  after the final gate at 22:01:07Z, and does not involve `lovelace-desktop`.
- The desktop mutation-gate hold was respected: after the final gate run, no command wrote under
  `rust/`; only this report was created.
