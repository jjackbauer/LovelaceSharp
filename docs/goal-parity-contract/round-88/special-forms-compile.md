# Round 88 — special forms and the compile result shape

**Status: fixed and pinned.** The two ported descriptor rows that were weak are now equivalent to
the pinned oracle on their published payloads: `and`/`or`/`not` canonicalise and display as the
reference's logical-operator nodes, and `compile` returns the IR source as `Text` (only
`compile_full` returns the `CompilationResult` record). Measured 2026-11-12
(America/Sao_Paulo) against `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe`.

**The 4 ids fixed:**

- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#4` (`and`)
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#25` (`compile`)
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#89` (`not`)
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#94` (`or`)

`#46` is the same `compile` cause and is fixed by the same production change; its test was
strengthened alongside the four primary rows (§4).

## 1. Probe (oracle first)

Every probe ran the documented prelude `x = symbol("x"); y = symbol("y"); t = symbol("t");`
through both binaries with `--file <probe>.ls --omit-functions`, then compared
`result` + `variables` after the differential's four volatile keys were removed at any depth.

| Row | Snippet | Oracle kind | Oracle display / typed | Oracle canonical |
|---|---|---|---|---|
| #4 | `assume(x > 0); and(x > 0, x < 1)` | Symbolic | `x < 1 and x > 0` / `x < 1 and x > 0 (Symbolic)` | `(and (lt (sym x) (rat 1 1)) (gt (sym x) (rat 0 1)))` |
| #89 | `not(x > 0)` | Symbolic | `not x > 0` / `not x > 0 (Symbolic)` | `(not (gt (sym x) (rat 0 1)))` |
| #94 | `or(x > 0, x < -1)` | Symbolic | `(x < -1 or x > 0)` / `(x < -1 or x > 0) (Symbolic)` | `(or (lt (sym x) (rat -1 1)) (gt (sym x) (rat 0 1)))` |
| #25 | `compile(x^2 + 1, [x])` | Text | the IR source below, no kind suffix | n/a (a Text value has no canonical field) |
| #46 | `k = compile(...); evalir_batch(k, [1, 2, 3], 40)` | Vector (result) / Text (`k`) | `[2, 5, 10] (Vector)` / `k` is the IR source | n/a |

The IR source both rows carry, byte for byte (no trailing newline):

```
#!mathir 2
param x
const (rat 1 1)
const (rat 2 1)
Constant 0 0
Parameter 0 0
Constant 0 1
PowInt 0 1,2
Add 0 0,3
```

The structured Text shape is exactly `{"kind":"Text","value":"<the IR above>"}`.

### Which side had the inversion

The port, not the oracle. The brief said "Rust returns Text; the reference returns a
CompilationResult record"; the probe and the reference source say the opposite: the pinned oracle
publishes `kind: "Text"` for `compile`, and `Lovelace.MathIR/Evaluator.cs:366-368` declares
`compile` with return kind `"Text"` while `compile_full` (`:385-387`) declares
`"CompilationResult"`. Rust's shared `"compile" | "compile_full"` arm published the record for
both. The fix therefore moves the port to the oracle's Text for `compile`; `compile_full` keeps
the record.

## 2. Production fixes

All four changes are in the printer/kernel surface the reference uses; no new expression variant
was invented, so ordinary functions still print `(fn name ...)`.

1. **Logical-node display** — `rust/lovelace-symbolics/src/lib.rs` (`logical_text`, :1542; used
   by the `Expr::Fun` render arm at :897). `and`/`or` render infix with their operands sorted by
   the kernel's total term order (`canonical_order`), and run through the reference's precedence
   rule: Or binds at `-1` and is parenthesised at top level, And at `0` is not, Not at `4` renders
   `not <operand>`. This is why `or`'s display now carries the outer parentheses the reference
   emits, while `and`/`not` displays are unchanged.
2. **Logical-node canonical form** — same file (:4331). Canonical tags are `(and ...)`,
   `(or ...)`, `(not ...)`; And/Or operands are emitted in `canonical_order`, which is what puts
   the `lt` before the `gt`. The generic `(fn ...)` arm is untouched for every other function.
3. **Canonical round-trip** — same file (:5061, :5161, :5259). `canonical_parse` accepts the three
   tags, `canonical_rank` gives them the reference's NodeKind ordinals (And=12, Or=13, Not=14), and
   two And/Or nodes compare their sorted operand sequences. `canonical_parse(e.canonical())` stays
   a normal form for the new tags.
4. **Compile return kind** — `rust/lovelace-lang/src/builtins.rs:1760-1764`. The arm still
   compiles, validates and builds the parameters exactly as before; `compile` now returns
   `Value::Text(ir)` (trailing newline trimmed, matching the oracle) and returns before the record
   is built. `compile_full` is byte-identical to before and to the oracle, field for field.

## 3. Per-row before / after (Rust)

| Row | Before (Rust) | After (Rust) | Oracle |
|---|---|---|---|
| #4 | canonical `(fn and (gt (sym x) (rat 0 1)) (lt (sym x) (rat 1 1)))`; display already `x < 1 and x > 0` | canonical `(and (lt (sym x) (rat 1 1)) (gt (sym x) (rat 0 1)))`; typed `x < 1 and x > 0 (Symbolic)`; nodeCount 7 | matches |
| #89 | canonical `(fn not (gt (sym x) (rat 0 1)))`; display already `not x > 0` | canonical `(not (gt (sym x) (rat 0 1)))`; typed `not x > 0 (Symbolic)`; nodeCount 4 | matches |
| #94 | display/typed `x < -1 or x > 0` / `x < -1 or x > 0 (Symbolic)`; canonical `(fn or (gt (sym x) (rat 0 1)) (lt (sym x) (rat -1 1)))` | display/typed `(x < -1 or x > 0)` / `(x < -1 or x > 0) (Symbolic)`; canonical `(or (lt (sym x) (rat -1 1)) (gt (sym x) (rat 0 1)))`; nodeCount 7 | matches |
| #25 | `Record`/`CompilationResult`; typed `CompilationResult(ir: ..., parameters: ..., ...) (CompilationResult)` | `Text`; typed is the bare IR source; structured `{"kind":"Text","value":"<IR>"}` | matches |
| #46 | `k` was `Record`/`CompilationResult`; batch result `[2, 5, 10]` | `k` is `Text` (the IR source); batch result unchanged `[2, 5, 10] (Vector)` | matches |

The five normalized envelopes (`result` + `variables`) compare **MATCH** against the oracle; the
#46 result included. `compile_full(x^2 + 1, [x])` still publishes the record with fields
`ir, parameters, result_domain, result_type, precision_policy, optimization_policy, target,
mathir_version, exact, diagnostics` and the same values as the oracle.

## 4. Strengthened tests (no assertion removed or weakened)

| Row | Test | What it now asserts |
|---|---|---|
| #4 | `lovelace-lang::suite_parity_batch::descriptor_example_004_and` (`suite_parity_batch.rs:2313`) | keeps the typed display; adds canonical `(and (lt ...) (gt ...))`, nodeCount 7, and a canonical-parse round trip |
| #89 | `lovelace-lang::suite_parity_batch::descriptor_example_089_not` (`:2729`) | keeps the typed display; adds canonical `(not (gt ...))` and nodeCount 4 |
| #25 | `lovelace-lang::suite_engine_batch::descriptor_examples_execute` (`suite_engine_batch.rs:320`) | the mapped row now asserts the compile example is `Text` with the exact IR as its `value`, not merely that it runs |
| #94 | same mapped test | asserts display/typed `(x < -1 or x > 0)`, canonical `(or (lt ...) (gt ...))` and nodeCount 7, not merely execution |
| #46 | `lovelace-lang::suite_parity_batch::descriptor_example_046_evalir_batch` (`:2392`) | keeps the `[2, 5, 10]` assertion; adds that `k` is `Text` with the exact IR value |
| #26 (record shape) | `lovelace-lang::suite_parity_batch::descriptor_example_026_compile_full` (`:2177`) | keeps the full typed record; adds `kind: Record`, `type: CompilationResult`, the exact field-name order, and the exact `ir` value |

**Existing-test audit (parent request 1):** no Rust test or manifest mapping assumed `compile`
returns a Record. The only place a compilation record is asserted is
`descriptor_example_026_compile_full` (correct: that is `compile_full`); `round84_evalir_multi`
and the `evalir` tests pass `compile(...)` only as an input and never inspect its kind. There was
therefore no wrong-contract test to correct, only the weak #25 mapping to strengthen.

## 5. Gates (observed 2026-11-12)

| Step | Command | Observed | Exit |
|---|---|---|---|
| Workspace suite | `cargo test --workspace` (in `rust/`) | **5,366 passed, 0 failed, 1 ignored** over 21 binaries | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error** (115 duplicates skipped; 1,958 executions); `harness exit: 0` | 0 |
| Clippy | `cargo clippy -p lovelace-symbolics -p lovelace-lang -p lovelace-run --all-targets -- -D warnings` | clean (`Finished dev profile`) | 0 |
| Format | `cargo fmt -p lovelace-symbolics -p lovelace-lang -p lovelace-run -- --check` | clean (no output) | 0 |

The workspace count is 5,366 rather than the snapshot's 5,365 because the shared tree already
contains the concurrent round-87 plot lane's two new tests: round-86's 5,345 + 19 desktop tests +
2 round-87 tests = 5,366. This round added no new test functions.

## 6. Scope

Changed files: `rust/lovelace-symbolics/src/lib.rs` (printer/canonical/parser; approved
extension of scope), `rust/lovelace-lang/src/builtins.rs` (compile arm only),
`rust/lovelace-lang/src/suite_parity_batch.rs`, `rust/lovelace-lang/src/suite_engine_batch.rs`,
and this report. No `.cs`, fixture, corpus, `parity/` file, desktop file or other document was
edited; the concurrent round-87 plot changes already present in `builtins.rs`/`lib.rs` were left
untouched. Probe scripts and raw envelopes live under `rust/target/tmp/r88/` (git-ignored).
