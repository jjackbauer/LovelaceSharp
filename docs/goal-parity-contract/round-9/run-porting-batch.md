# Round 9 — a verified porting batch from `Lovelace.Run.Tests`, plus Suite and Symbolics rows

Oracle commit `1cc28f255eca6029849f880949fc7be1d40ab8af` (the census and the C# runner are the
same pinned reference the previous rounds used). This round adds **552 verified census mappings**:
400 from `Lovelace.Run.Tests`, 135 from `Lovelace.Symbolics.Tests`, and 17 from
`Lovelace.Suite.Tests`. Coverage moved from 3,074/6,209 (49.51 %) to
**3,626/6,209 (58.40 %)**.

The objective asked for 600 new mappings. 552 were delivered — 48 short — and the shortfall is
accounted for below: the remaining pools that were probed either need production behaviour the
Rust tree does not have, or assert a C# API surface with no Rust counterpart. Nothing was weakened
to bridge the gap.

All 552 are new `#[test]`s in test-only modules under `rust/lovelace-run/src`; the only change to
`main.rs` is the `#[cfg(test)] mod` declarations that mount them. No production line was changed.

## 1. Mappings per project

| C# test project | Cases | Ported before | Ported this round | Ported now | Refused / deferred now |
|---|---:|---:|---:|---:|---:|
| Lovelace.Run.Tests | 751 | 0 | **400** | 400 (53.26 %) | 351 |
| Lovelace.Suite.Tests | 929 | 435 | **17** | 452 (48.65 %) | 477 |
| Lovelace.Symbolics.Tests | 1,226 | 97 | **135** | 232 (18.92 %) | 994 |
| **total** | | **3,074** | **552** | **3,626 (58.40 %)** | **2,465** |

### 1.1 What is in the new test modules

| Rust test module | Tests | Census cases mapped | What it ports |
|---|---:|---:|---|
| `parity_batch.rs` | 32 | 260 | `IntegerArgumentBoundaryMatrixTests` — every boundary cell whose concrete outcome Rust reproduces, grouped by the inventory row it drives |
| `wire_batch.rs` | 63 | 108 | print capture (17), registry queries (6), limit records (23), DivRem (8), variable projection (4), structured Real precision (5), arity forms (19), function arity metadata (2), inspection exactness (2), committed state (5), source positions (7), builtin shape forms (8), builtin surface records (2) |
| `golden_batch.rs` | 6 | 33 | the 20 golden envelopes (byte-equal after the same volatile-key normalisation) and the 13 semantic facts |
| `pretty_batch.rs` | 4 | 80 | `PrettyParenthesesTests`: 32 round-trip rows, 46 minimality rows, 2 reported renderings |
| `enum_batch.rs` | 1 | 8 | `EnumValueEmissionTests` — every enum-valued field carries its declared type and member |
| `dx_batch.rs` | 15 | 46 | `DxStructuredResultsTests` (type vocabulary and 10 record facts), `SolveCompletenessPairingTests`, `SolveCompletenessMappingTests`, `SystemSolveRecordTests`, and 11 exactness rows |
| `suite_batch.rs` | 16 | 17 | array/function/builtin/statement facts from `Lovelace.Suite.Tests` |

The multi-mapping rows are the same kind of mapping earlier rounds used for
`lovelace-int::tests::*`: one Rust test drives a table of concrete census rows and asserts each
row's own C# contract.

## 2. How every row was verified

The C# assertion is the contract, so the starting point was the assertion itself, read from the C#
source, then replayed against the Rust runner. The Rust test states the same concrete input and the
same concrete expectation:

1. **Golden envelopes (33).** The C# test runs `--file fixtures/<name>.ls --omit-functions` and
   compares the JSON to the golden after replacing exactly `revision`, `elapsed`, `elapsedTime`
   and `timings` with `"<volatile>"`. The Rust test does the same; all 20 fixtures compare equal.
2. **Integer-argument boundary matrix (260).** The inventory and the matrix generation were
   reconstructed from `IntegerArgumentBoundaryMatrixTests.cs` (30 registry arguments and 2
   non-registry call sites). A cell is mapped only when Rust satisfies the exact C# assertion for
   that cell: `true` cells must succeed and exit 0; `false` cells must be recoverable, must not
   be internal, and must exit 1; `null` cells may do either but must be a typed outcome. The 57
   cells that fail one of those assertions are refused, not mapped.
3. **Record facts (limit, DivRem, system solve, Dx, enum).** The Rust test reads the same wire
   fields the C# test reads (`kind`, `type`, the ordered `fields` array, enum type+member,
   condition arrays, diagnostics) and compares the concrete values the C# assertion names. Where
   the C# test compares a record to its `_full` form, the Rust test compares the two structured
   documents directly.
4. **Print, registry, positions, committed state.** Same argv, same script text, same output array
   or diagnostic position; raw JSON segment assertions (`"output":["A","B"]`) are preserved.
5. **Pretty round-trip (80).** The C# test evaluates the source, renders it, re-parses the
   rendering and compares canonical forms. The Rust test evaluates the same prelude+source,
   re-evaluates the published display, and compares `structured.canonical`. The minimality test
   applies the same doubled-parenthesis scan to the published display.
6. **Suite facts (17).** Each Rust test asserts the same concrete result the C# test asserts
   (`sqrt(4)` is the Real 2, `pi(10)` is 3.1415926535, a parameter shadow leaves the global at
   10, etc.) or that the call is refused where the C# test expects an `InvalidOperationException`.

## 3. Refusals (top reasons)

These are cases that were probed and deliberately left unmapped, with the Rust behaviour that
differs. The full census keeps them `deferred` with the standard reason.

| Pool | Cases | Rust behaviour vs the C# assertion |
|---|---:|---|
| Run — matrix cells | 57 | 25 need a builtin or arity Rust does not have (`e`, `movingavg`, `delay`, `impulse`, `step`, `exponential`, `cosine`, `powerseries`, `noise`, two-argument `eye`, `mean(..., axis)`, `norm(..., axis)`); 17 ACCEPT a non-integral integer argument the C# test requires refused (concat axis, evalir/evalir_batch digits, dft n, range bound); `eye(9223372036854775808)` panics with a multiply overflow; `range(9223372036854775808)` does not return inside the cell budget |
| Run — liveness | 31 | every sentinel refusal must start `builtin()` and contain the argument anchor; Rust's messages are sentences like `Dimensions must be non-negative integers.` / `Axis must be an integer.` |
| Run — value observability | 3 | `evalir(..., digits)` publishes the same `0.(3)` for digits 1/2/3, so the "larger count ⇒ longer answer" property is false |
| Run — error taxonomy | 100+ | C# asserts `InvalidArgument`/`TypeMismatch` where Rust crosses as `InvalidOperation`/`DomainError` (committed-state failure, wide-argument refusals, plugin integer wire, arity ranges, setprecision range, missing-file `FileReadError`, ...) |
| Run — missing wire fields | 40+ | no `signature` (CallSignatureDiagnostics 4), no `suggestions`/`hints` (UnknownFunction 4), no `truncated`/`truncationReason`/`budget` (print budget 12), no cancellation ledger (Cancellation 6), no `DepthExceeded` (depth tests 31; the process stack-overflows instead), no `LReal64` |
| Run — failed statement timing | 20+ | Rust's timing ledger omits the failing statement; C# requires it (EnvelopeDuration, SourcePosition runtime-failure rows) |
| Symbolics — LaTeX | 213 | `latex` is advertised by the registry but every call answers `Unknown function 'latex'.` |
| Symbolics — complex closed forms | 11 | `sqrt(-1)`, `sqrt(-4)`, `sqrt(-2)` answer the Real `0`; `(-4)^(1/2)` is refused, while C# returns the exact imaginary constants |
| Symbolics — exactness/evalf/system | 150+ | `inspect(<Real>).exact` says true where C# says false; `evalf(pi, ...)` and `e(...)` do not exist; `evalir` supports one free symbol only; `solve_system_full` refuses the nonlinear/contradictory systems the C# tests pin |
| Suite | 477 | mostly the already-documented gaps: no LReal64, setprecision range refusals are not typed, unary plus does not parse, function/block locals leak, empty sum/prod are refused, mixed literal arrays keep narrower kinds |
| Unsupported API | — | HelpService/NameSuggestions, `CaptureState`, `ProjectValue` façade, LReal exponent, C# AST/type-shape assertions |

## 4. Observed command lines

```
powershell -File parity/gen-rust-tests.ps1
  crates=11 tests=3404 suiteExit=0 passed=3404 failed=0

node parity/gen-manifest.mjs
  rows 6209, ported 3626, deferred 2465, excluded 118, coveragePercent 58.4, rustTests 3404

node parity/tools/gate-manifest.mjs
  censusCases 6209, manifestRows 6209, ported 3626, consolidated 0, excluded 118, deferred 2465
  coveragePercent 58.4, claimedRustTests 3626, failures []
  PASS (structural): 6209 test cases dispositioned; 3626 claim Rust coverage (58.4%).

node parity/tools/coverage-report.mjs
  coverage.md written: 3626/6209 ported (58.40 %)

powershell -File parity/gate-fixtures.ps1 -RustOnly
  rust: 20/20
  PASS: rust 20/20

cargo run -p lovelace-corpus
  corpus: 212 passed, 2 failed, 214 total   (sym/071 and sym/076, unchanged)
```

## 5. What was not verified

- The C# test suite cannot be executed in this environment (the VSTest testhost is denied
  `OpenProcess`; recorded in `parity/README.md`). The C# assertions were read from source and the
  concrete values were replayed through the pinned C# runner where an envelope was needed; no C#
  test run is claimed.
- `parity/run-all.ps1` (the full scorecard) was not run; the manifest gate, fixture gate, corpus
  gate and the two requested cargo suites were.
- 48 new mappings short of the round's 600 target. The remaining probed pools need production
  changes (LaTeX dispatch, complex closed forms, exactness provenance, nonlinear system solving,
  depth/cancellation guards) that this round's scope forbids.