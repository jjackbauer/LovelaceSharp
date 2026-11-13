# Round 104 — KNOW-001..010 and CLI-001..007 re-adjudication

Date: 2026-11-13 (America/Sao_Paulo) · Repository HEAD: `e74dc64` (branch `rust`) ·
Scope: `parity/requirements.json`, `rust/lovelace-know/tests/cli_stdio_tests.rs`,
`rust/lovelace-run/src/repl.rs`, two lint-only edits in `rust/lovelace-know/src/{exact,json}.rs`,
and this report. No `.cs` file, fixture, `parity/*.toml`, desktop file or other document was touched.

## 1. Method

The rows were re-adjudicated by executing the **built binaries**, not by reading the source:

* `cargo build -p lovelace-know -p lovelace-run` reported `Finished dev profile` (both binaries
  current) with `cargo 1.99.1`.
* Every knowledge request was a JSON file passed with `--file` (the objective's
  `lovelace-know --file <request.json>` form); the bodies are listed in §2.1.
* Every REPL claim was produced by redirecting a script into `lovelace-run --repl` and
  capturing stdout, stderr and the exit code; the transcripts are in §3.
* A row is **met** only when the running binary (plus its test suite, where the requirement is
  a state machine that a pipe cannot reach) proves it. Anything less is **partial** with the
  gap named in the row note.

Paths below use `$K = rust/target/tmp/know-cli`, the scratch directory for request files and
captured graphs (build output, not tracked).

## 2. Knowledge tool (`rust/target/debug/lovelace-know.exe`)

### 2.1 Request bodies

| File | Body |
|---|---|
| `$K/config.json` | `{"command":"config"}` |
| `$K/help.json` | `{"command":"help"}` |
| `$K/sample3.json` | `{"command":"sample","batchSize":3,"runner":"<repo>/rust/target/debug/lovelace-run.exe"}` |
| `$K/sample8-a.json` / `sample8-b.json` | same with `batchSize=8` |
| `$K/sample8-seed1.json` / `-seed2.json` | same with `seed=1` / `seed=2` |
| `$K/converge8.json` | `{"command":"converge","graphPath":"$K/smoke-graph.json","runner":"...","maxSamples":8,"batchSize":8}` |
| `$K/converge-seed.json` | `{"command":"converge","graphPath":"$K/rust-graph-20240617.json","runner":"...","seed":20240617}` |
| `$K/reduce.json` | `{"command":"reduce","graphPath":"$K/smoke-graph.json"}` |
| `$K/query-*.json` | `{"command":"query","graphPath":"$K/smoke-graph.json","query":"summary\|planes\|boundaries\|frontiers\|metrics\|graph"}` |
| `$K/bad-command.json` | `{"command":"bogus"}` |
| `$K/invalid.json` | `{not json` |
| `$K/sample-bad-runner.json` | `{"command":"sample","batchSize":1,"runner":"C:/nonexistent/lovelace-run.exe"}` |

### 2.2 Commands, exit codes, observed output

| Command | Exit | Observed (stdout unless stated) |
|---|---|---|
| `--file $K/config.json` | 0 | `config resolved: seed=20240617, batch=64, maxSamples=700, ops=12, naturals=13, integers=13, reals=8` |
| `--file $K/help.json` | 0 | `Commands: config (resolve defaults), sample (...), reduce (...), converge (...), query (summary\|planes\|boundaries\|frontiers\|metrics\|graph).` |
| `--file $K/sample3.json` | 0 | `executed 3 samples: 2 ok, 1 errors, 3 distinct plane classes` |
| `--file $K/converge8.json` | 0 | `planes=2, boundaries=0, frontiers=54, samples=8, converged=False, ...; graph=...\smoke-graph.json` |
| `--file $K/reduce.json` | 0 | `planes=2, boundaries=0, frontiers=54, samples=8, converged=False, C1_newPlanes_lastK=2 (rate 0.25), C2_stable=0/0, C3_agreement=0, C4_covered=True` |
| `--file $K/query-summary.json` (also `metrics`) | 0 | same summary as `reduce` |
| `--file $K/query-planes.json` | 0 | `2 planes` + `planes` array |
| `--file $K/query-boundaries.json` | 0 | `0 boundaries` + empty array |
| `--file $K/query-frontiers.json` | 0 | `54 frontiers` + array |
| `--file $K/query-graph.json` | 0 | full `graph` document |
| `--file $K/bad-command.json` | 1 | `ok=false`, `error="unknown command 'bogus'"` |
| `--file $K/invalid.json` | 1 | `error="invalid request JSON: JSON error at byte 1: expected object key"` |
| `--file $K/missing-graph.json` | 1 | `error="no graph at '...'; run 'converge' first"` |
| `--eval '{"command":"config"}'` | 0 | byte-identical to `--file` and `--stdin` (compared with Node spawns) |
| `--bogus` | 2 | stderr `Error: Unknown argument '--bogus'.` + usage; stdout empty |
| (no arguments) | 2 | stderr `Error: No request provided. Use --eval <json>, --file <path>, or --stdin.` |
| `--help` | 0 | 12-line usage, `Commands: config \| sample \| reduce \| converge \| query` |

Every response is one indented JSON object with the fixed 11-member field set (`ok`, `command`,
`summary`, `config`, `graph`, `planes`, `boundaries`, `frontiers`, `metrics`, `samples`, `error`) and null
for unused members.

## 3. REPL evidence (`rust/target/debug/lovelace-run.exe --repl`, stdin redirected)

Elapsed suffixes are volatile and are quoted as observed.

### 3.1 CLI-002 — multiline accumulation

Input `func g(x) {` / `  x + 1` / `}` / `g(2)`:

```
LovelaceSharp REPL v1.0.0
Arbitrary-precision math scripting, vector math, and plotting.
Type 'help' for a list of statements, operators, functions, and commands.

» func g(x) {
…   x + 1
… }
» g(2)
= 3 (Natural)   [164.4 µs]
» 
Bye!
```

### 3.2 CLI-005 / CLI-006 — state, `_`, precision

Input `a = 5` / `func f(x) = x^2` / `_ * 3` / `f(4)` / `vars`:

```
» a = 5
= 5 (Natural)   [258 µs]
» func f(x) = x^2
» _ * 3
= 15 (Natural)   [57.5 µs]
» f(4)
= 16 (Natural)   [100.9 µs]
» vars
  _ = 16 (Natural)
  a = 5 (Natural)
```

Input `sqrt(2)` / `set display 5` / `sqrt(2)` / `set precision 7` / `sqrt(2)`:

```
» sqrt(2)
= 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727 (Real)   [2.07 ms]
» set display 5
Display digits set to 5.
» sqrt(2)
= 1.41421 (Real)   [1.32 ms]
» set precision 7
Computation precision set to 7 decimal places.
» sqrt(2)
= 1.4142135 (Real)   [107.9 µs]
```

### 3.3 CLI-007 — errors, cancellation, recovery

`[1,2;3,4]` / `diff(x, 3)` / `2+2`:

```
» [1,2;3,4]
[1,2;3,4]
    ^
Error: Expected 'RBracket' but found ';' at position 4.   [185.1 µs]
» diff(x, 3)
Error: Undefined variable 'x'.   [59.4 µs]
» 2+2
= 4 (Natural)   [87.1 µs]
```

`--cancel-after 200` with `sum(1..1000000000)` / `1+1`:

```
» sum(1..1000000000)
Error: the evaluation was cancelled by the caller.   [202.35 ms]
» 1+1
= 2 (Natural)   [87.5 µs]
```

### 3.4 CLI-004 — commands and help

`help`, `help solve`, `funcs`, `set pretty unicode`, `inf`, `set pretty ascii`, `inf`, `set display 4`,
`pi`, `clear`, `vars`, `delete n` were driven in one session. Observed, in order: the help overview
and the `solve` page; `Pretty output: unicode.`; `= ∞ (Symbolic)`; `Pretty output: ascii.`;
`= inf (Symbolic)`; `Display digits set to 4.`; `= 3.1415 (Real)`; `All variables cleared.`;
`(no variables defined)`; `Variable 'n' is not defined.`; exit 0.

## 4. Seed 20240617 reproduction and comparison (KNOW-008 / KNOW-010)

`--file $K/converge-seed.json` (default configuration, seed 20240617) exited 0 in **10,208 ms**
and wrote `$K/rust-graph-20240617.json` (215,968 bytes). Summary:

```
planes=7, boundaries=18, frontiers=18, samples=314, converged=True,
C1_newPlanes_lastK=0 (rate 0), C2_stable=18/18, C3_agreement=1, C4_covered=True
```

Comparison against the frozen C# `knowledge-graph-demo.json`:

| Aspect | Rust | Frozen C# | Same |
|---|---|---|---|
| seed / version / config / sampling | 20240617 / 1 / identical / identical | same | yes |
| samples | 314 | 314 | yes (314/314 coordinates, provenance and weights) |
| planes | 7 | 8 | no — C# has `err\|Exponent must be positive. (Parameter 'exponent')` |
| boundaries | 18 | 18 | yes, identical in order |
| frontiers | 18 | 19 | yes for the 18 non-`low-support` frontiers; C# adds one low-support frontier |
| Integer plane support | 60 (includes sample 164) | 59 | no — follows from the C# exponent error |
| coverage cells | 108 | 108 | yes, all identical |
| metrics | converged, stopReason, totalSamples=314, c1*, c2 18/18, c3 45/45 (1.0), c4 | same | yes; only `planeCount` differs (7 vs 8) |

The five observation divergences, all runner-level expression semantics (not pipeline):

| Index | Script | Rust observation | C# observation |
|---|---|---|---|
| 164 | `-3 ^ 0` | `Integer`, `-1 (Integer)` | `err\|Exponent must be positive. (Parameter 'exponent')` |
| 177 | `-0.5 ^ 10` | `-0.0009765625 (Real)` | `0.0009765625 (Real)` |
| 207 | `0.(3) * 0.25` | `0.08(3) (Real)` | expanded terminating decimal |
| 219 | `-2 ^ 4` | `-16 (Integer)` | `16 (Integer)` |
| 231 | `10 % 0.5` | `0 (Real)` | `9 (Real)` |

The sample stream, boundaries, frontiers, coverage and all convergence metrics except
`planeCount` reproduce exactly; `seed_reproduction.rs` pins both the identities and the five
divergences, so any drift fails loudly.

## 5. Per-row adjudication

`$K = rust/target/tmp/know-cli`; `RUN=rust/target/debug/lovelace-run.exe`;
`KNOW=rust/target/debug/lovelace-know.exe`. All commands exit 0 unless shown otherwise.

| ID | Command | Observed | Status |
|---|---|---|---|
| CLI-001 | `RUN --repl < $K/repl/multiline.in` | banner + `»` / `…` prompts; `= 3 (Natural)   [164.4 µs]`; `Bye!`; stderr empty; 11 transcript tests pass | met |
| CLI-002 | same input | continuation lines read under `…`; block evaluated once; `continuation_end_of_input_still_prints_bye_once` passes | met |
| CLI-003 | `cargo test -p lovelace-run repl::tests` | 9 passed, including cursor editing, history/draft restore, ANSI decode, Ctrl+C, and the new byte-driven `read_interactive` test. Raw-mode console layer unverified (ConPTY children abort 0xC0000142 in this session) | **partial** |
| CLI-004 | `RUN --repl < $K/repl/commands.in` | help / help solve / funcs; `Pretty output: unicode.`; `= ∞ (Symbolic)`; `= 3.1415 (Real)`; `All variables cleared.`; `Variable 'n' is not defined.` | met |
| CLI-005 | `RUN --repl < $K/repl/state.in` | `a = 5`; `_ * 3` -> `= 15`; `f(4)` -> `= 16`; `vars` -> `_ = 16 (Natural)`, `a = 5 (Natural)` | met |
| CLI-006 | `RUN --repl < $K/repl/precision.in` | 100-digit `sqrt(2)`; `set display 5` -> `1.41421`; `set precision 7` -> `1.4142135`; `_` tracked | met |
| CLI-007 | `RUN --repl < $K/repl/errors.in`; `RUN --repl --cancel-after 200 < $K/repl/cancel.in` | caret + `Error: Expected 'RBracket' but found ';' at position 4.`; `Error: Undefined variable 'x'.`; `= 4 (Natural)` after the error; `Error: the evaluation was cancelled by the caller.   [202.35 ms]` then `= 2 (Natural)` | met |
| KNOW-001 | `KNOW --file $K/converge-seed.json`; `KNOW --file $K/sample8-a.json` | converge 314 samples / 7 planes / 18 boundaries / 18 frontiers in 10.2 s; sample `executed 8 samples: 7 ok, 1 errors, 4 distinct plane classes`; `cargo test -p lovelace-know` 56 passed | met |
| KNOW-002 | `KNOW --file $K/sample8-a.json` twice; seed 1 vs 2 | same seed -> JSON-identical samples arrays; seed 1 `-5 ^ -0.5`... vs seed 2 `2.5 >= 4`...; 7 proposal tests pass | met |
| KNOW-003 | `KNOW --file $K/sample8-a.json` | `0 % 0` -> `err\|Cannot divide by zero.`; `3 - -2` -> `Integer` / `5 (Integer)`; `0.(3) == -4` -> `Boolean:False`; `5 >= 3` -> `Boolean:True` | met |
| KNOW-004 | `KNOW --file $K/converge-seed.json`; `KNOW --file $K/reduce.json` | 18 boundaries, 18 frontiers, C2 18/18, C3 45/45, C4 true, converged; reduce re-derives `planes=2, boundaries=0, frontiers=54`; 8 + 7 reducer/convergence tests pass | met |
| KNOW-005 | converge then `query` / `reduce` on the persisted graph | `$K/rust-graph-20240617.json` written (215,968 B), re-read by query/reduce; `frozen_reference_round_trips_byte_for_byte` passes | met |
| KNOW-006 | `config`, `sample`, `reduce`, `converge`, `query`, `help` requests | all six exit 0 with their documented members/summaries; `cli_stdio_tests` 11 passed incl. the new reduce test | met |
| KNOW-007 | `--eval` / `--file` / `--stdin` config requests; bad command; invalid JSON; missing graph; `--bogus`; no args | byte-identical config responses, exit 0; exit 1 for command/execution errors; exit 2 for usage errors; fixed 11-member null-filled response | met |
| KNOW-008 | `KNOW --file $K/converge-seed.json` | exit 0, 10.2 s; 314/314 coordinates identical; 18/18 boundaries identical; 18/18 non-low-support frontiers identical; 7 vs 8 planes from the one C# error plane; `seed_reproduction` passes | met |
| KNOW-009 | `KNOW --file $K/sample-bad-runner.json`; full converge | bogus runner -> `err\|cannot spawn C:/nonexistent/lovelace-run.exe: The system cannot find the path specified. (os error 3)`; converge spawns the real `lovelace-run.exe` once per sample (314x); `sample_command_executes_the_real_runner` passes | met |
| KNOW-010 | graph/metric comparison of the two files; tests | 0 coordinate diffs, 5 observation divergences, 18/18 boundaries, 18 non-low-support frontiers, 108/108 coverage; all metrics identical except `planeCount` (7 vs 8); `convergence_tests` 7 passed | met |

**Moved to met: 16. Left partial: CLI-003** (named gap in its ledger note).

## 6. Tests added (no existing test weakened)

1. `rust/lovelace-know/tests/cli_stdio_tests.rs::reduce_command_rederives_the_persisted_structure` —
   converges a small graph through the binary, runs `reduce` on the persisted file, and checks
   that planes/boundaries/frontiers are re-derived (matching the loaded graph) and the persisted
   metrics (`totalSamples=8`, `converged=false`) are returned. `reduce` was the only KNOW-006
   command without an end-to-end CLI test.
2. `rust/lovelace-run/src/repl.rs::tests::interactive_editor_submits_edited_and_historical_lines` —
   drives `read_interactive` with real key bytes (insert at cursor, Left arrow, Up/Down history,
   draft restore, Ctrl+C), asserting the submitted lines and the ANSI redraw. This covers the
   actual interactive loop, not only its pieces.

Two lint-only edits were required to make `clippy -D warnings` clean for `lovelace-know`:
`exact.rs` now allows `clippy::mutable_key_type` on the `HashMap<Nat, usize>` (interior mutability
is a memoized cache; `Eq`/`Hash` are value-based), and `json.rs` uses
`!(-4..15).contains(&exponent)`. No behavior changed.

## 7. Gates (all commands from the repository root on this working tree)

| Gate | Observed |
|---|---|
| `cargo test --workspace` | exit 0 — 5,795 passed, 0 failed, 1 ignored, 62.7 s (after the two new tests) |
| `cargo test -p lovelace-know` | exit 0 — 56 integration tests (11 cli + 7 convergence + 8 exact + 3 graph + 4 json + 7 observation + 7 proposal + 8 reducer + 1 seed) |
| `cargo test -p lovelace-run` | exit 0 — 438 unit + 1 recursion + 11 REPL = 450 passed |
| `cargo fmt -p lovelace-know -p lovelace-run -- --check` | exit 0 |
| `cargo clippy -p lovelace-know -p lovelace-run --all-targets -- -D warnings` | exit 0 |
| `rust/target/release/lovelace-corpus.exe` | exit 0 — `corpus: 214 passed, 0 failed, 214 total`. The first run was 213/214 because `%TEMP%\lovelace-plots` did not exist (plot output write); creating that directory made the gate pass. |
| `node parity/tools/gate-fixtures.mjs` | exit 0 — `oracle: 20/20`, `rust: 20/20` |
| `node parity/tools/gate-cli.mjs` | exit 0 — `CLI matrix: 16/16` |
| `node parity/tools/gate-requirements.mjs` | exit 0 — `PASS: 262 requirements accounted for; 196 met, 53 partial, 13 unmet` |
| `node parity/tools/gate-projects.mjs` | exit 0 — 19 C# projects dispositioned (4 port, 1 consolidated, 8 partial, 6 absent) |
| `node parity/tools/gate-census.mjs` | exit 0 — 2,105 methods / 6,209 cases across 15 projects |
| `node parity/tools/gate-manifest.mjs` | **exit 1, pre-existing and documented**: "rust-tests.json is stale: the Rust sources changed since the inventory was measured". `GATE-011` and `TEST-008` already record this for the concurrent desktop edits; this round's Rust edits invalidate the same hash. Remedy on a quiet tree: `parity/gen-rust-tests.ps1`. The inventory was intentionally not regenerated from this shared tree (out of scope and racy while desktop work is in flight). |

After concurrent edits landed in `rust/lovelace-int`, a final scoped re-run (`cargo test -p lovelace-know -p lovelace-run`, fmt and clippy) still exited 0; that run reported 506 passed, 0 failed across the two crates.

## 8. Ledger delta

`parity/requirements.json` was regenerated with `node parity/gen-requirements.mjs` after the row
edits (so `totals` is consistent) and passes `gate-requirements`:

| | Before | After |
|---|---|---|
| met | 180 | 196 |
| partial | 52 | 53 |
| unmet | 30 | 13 |

Only the 17 adjudicated rows (plus `totals`) differ from HEAD; every other row is byte-identical.

## 9. Remaining gap and caveats

* **CLI-003 (partial).** Editing, history and navigation are implemented and proved at the
  byte-driven editor loop, but the Windows VT / Unix `stty` raw-mode handshake that delivers real
  keystrokes could not be exercised: a piped `--repl` takes the plain path, and a ConPTY harness
  written for this round made every child (including `cmd.exe`) abort with `0xC0000142` before a
  byte was exchanged (`CreatePseudoConsole` returned S_OK; the same STARTUPINFOEX/CreateProcess
  path works with `CREATE_NEW_CONSOLE`). This is an environment limit, not a REPL failure, but the
  terminal layer remains unverified, so the row stays partial.
* **KNOW-008 / KNOW-010 note.** "Reproduction" here means the sample stream, boundaries,
  frontiers, coverage and convergence metrics reproduce exactly; the graph is not byte-identical
  because five runner-level expression-semantics divergences produce the one extra C# error
  plane. Those divergences are pinned in `seed_reproduction.rs` and are not knowledge-pipeline
  differences.
* **Corpus environment.** `lovelace-corpus` needs `%TEMP%\lovelace-plots` to exist; without it the
  plot case fails at file creation. Creating the directory (no source change) gives 214/214.
* **Shared working tree.** `rust/lovelace-desktop/src/workbench.rs`, `parity/rust-tests.json` and
  `docs/goal-parity-contract/journal.md` were already modified by concurrent work when this round
  started; the workspace suite above includes those changes.
