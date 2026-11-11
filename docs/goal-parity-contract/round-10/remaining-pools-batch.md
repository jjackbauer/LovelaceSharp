# Round 10 — remaining mappable pools: a verified batch

Oracle commit `1cc28f255eca6029849f880949fc7be1d40ab8af` (the census and the C# runner are the
same pinned reference the previous rounds used). This round maps **135 census rows** to **111 new
Rust tests**. Net new verified coverage is **+123 rows**: 3,626/6,209 (58.40 %) → **3,749/6,209
(60.38 %)**. Twelve of the 135 rows were already ported (their map entries were replaced with
equivalent new tests), so they add no coverage; 105 rows were previously unmapped and 18 were
curated `deferred` entries that now reproduce their C# assertion.

The objective asked for 500 new verified mappings. 123 were delivered — 377 short. The shortfall is
accounted for below: the remaining pools are dominated by C# type APIs with no Rust counterpart, by
builtins the Rust tree does not have, and by fixed-width/trigonometric tiers that do not exist in
`lovelace-real`. Nothing was weakened to bridge the gap.

## 1. Mappings per project

| C# test project | Cases | Ported before | Net new this round | Ported now | Remaining deferred |
|---|---:|---:|---:|---:|---:|
| Lovelace.Suite.Tests | 929 | 452 | **112** | 564 (60.71 %) | 365 |
| Lovelace.Real.Tests | 2,537 | 2,286 | **10** | 2,296 (90.50 %) | 241 |
| Lovelace.Run.Tests | 751 | 400 | **1** | 401 (53.40 %) | 350 |
| Lovelace.Symbolics.Tests | 1,226 | 232 | **0** | 232 (18.92 %) | 994 |
| Lovelace.Complex.Tests | 121 | 0 | **0** | 0 | 121 |
| Lovelace.Representation.Tests | 91 | 0 | **0** | 0 | 91 |
| Lovelace.Dsp.Tests | 61 | 0 | **0** | 0 | 61 |
| **total of the requested pools** | **5,716** | **2,970** | **123** | **3,093** | **2,223** |
| all projects in the census | 6,209 | 3,626 | **123** | **3,749** | 2,342 |

### 1.1 Where the 135 rows live

| Rust module | New tests | Census rows mapped | Contents |
|---|---:|---:|---|
| `lovelace-lang::suite_lex_parse_batch` | 49 | 49 | `TokenizerNumberTests` 7, `TokenizerIdentifierTests` 4, `TokenizerOperatorTests` 9, `UnterminatedStringTests` 2, `TokenTests` 7, `ParserTests` 20 |
| `lovelace-lang::suite_engine_batch` | 41 | 65 | `VectorTests` 8, `SuiteEngineTests` 4, `ScriptSourceTests` 1, `PlotTests` 4, `SuiteEngineOutputPlotTests` 1, `InterpreterVariableStoreTests` 3, `InterpreterBuiltinInvTests` 1, `ExactnessPreservationTests2` 4, `DescriptorExampleTests` 18, `DspPluginTests` 3, `EnumPayloadSeamTests` 1, `ValueTests` 3, `CrossEngineIsolationTests` 2, `LRealDispatchTests` 6, `PlotFileWriteRefusalTests` 1, `InterpreterBuiltinSetprecisionTests` 4, `InterpreterFactorialTests` 1 |
| `lovelace-real::real_parity_round10` | 20 | 20 | `RealConstructorTests` 15, `RealPropertiesTests` 3, `RealToStringTests` 1, `RealDivideLeadingZeroRunTests` 1 |
| `lovelace-run::suite_batch` | 1 | 1 | `ParseErrorClassificationTests` tokenizer string refusal |

The multi-row tests are deliberate: one Rust test drives the table of the C# theory and asserts
each row's own concrete C# expectation (`ExactnessPreservationTests2` 4 rows,
`LRealDispatchTests` 6 rows, `DescriptorExampleTests` 18 rows, `ValueTests` 3 rows).

## 2. How every row was verified

The C# assertion is the contract. Every case was read from its C# source and replayed against the
Rust engine with the same concrete input:

1. **Lexer/parser (49).** The Rust `syntax` module exposes the same tokenizer and parser. Tests
   assert the same token payload and UTF-16 position, the same AST node/operator/operand shape
   (through a test-only structural descriptor, because `Expr` has no `PartialEq`), or the same
   diagnostic position. The C# `TokenKind` ordinal table (20 rows) is refused: Rust's `Tok` enum
   has a different layout, so asserting the C# ordinals would be false.
2. **Engine facts (65).** Same script text and the same observable the C# test reads: the typed
   rendering, the structured `exact` flag, the record/field values, the plot title/SVG/path,
   `Engine::variables`/`precision`, or the typed refusal code. The 18 descriptor examples are the
   C# test's own contract exactly — "this example executes on a fresh engine with the documented
   prelude" — and only examples whose prelude+snippet the Rust tree runs are mapped.
3. **Real representation (20).** In-crate tests read the same private fields the C# assertion
   reads (`Exponent`, `PeriodStart`, `PeriodLength`, stored magnitude, `IsExact`) and assert
   the same values. Ten of these rows were already ported to older `real_parity_batch` tests;
   the new tests replace those map entries with the same concrete assertions, so they add no
   coverage.
4. **Run wire fact (1).** Same argv, same envelope: exit 1, `ok=false`, category `ParseError`,
   recoverable, message naming the unterminated string literal.

## 3. Refusals (top reasons)

Every refused row stays `deferred` with its prior reason; no row was weakened or guessed.

| Pool | Remaining | Dominant reasons |
|---|---:|---|
| Real — `LReal64Tests`/`LReal128Tests` | 84 | The fixed-width tiers do not exist in `lovelace-real`; asserting `Real` instead would test a different subject. |
| Real — trigonometric oracle/certified/fast-path/symmetry/pi-resolution | 67 | `Real` has no `Sin`/`Cos`/`Atan` API; `pi(n)` is the only transcendental, with no argument validation or segment API. |
| Real — add/subtract periodic corpus | 18 | The documented `DIGIT_BUDGET` refusals from round 7 (period longer than the arithmetic budget). |
| Real — exact-quotient properties | 9 | Rust division truncates at the explicit budget: `(n/d)·d == n` fails at budget 1 and the 2^4000 case, so the C# property does not reproduce. |
| Real — modulus / pow-zero-base / properties / to-string / sqrt-batch / async | 41 | No `Real % Real`; no Real-exponent `Pow` refusal type; global `DisplayDecimalPlaces`/`MaxComputationDecimalPlaces` knobs and mutable `Exponent`/`PeriodStart` (Rust `PeriodStart` is −1 for terminating values, C# default is 0); `IFormattable`/`TryFormat`; batch and `AsyncLocal` APIs. |
| Suite — descriptor examples | 25 | 24 need builtins the Rust tree lacks (`latex`, `e`, `acosh`/`asinh`/`atanh`, `cancel_full`, `scale`, `step`, `cosine`, `delay`, `exponential`, `filter`, `impulse`, `movingavg`, `ndims`, `noise`, `powerseries`, `assume_clear`/`assume_integer`/`assume_real`); 1 is the corpus-guard fact. |
| Suite — recursion/depth guards | 49 | Rust stack-overflows or lacks the typed `DepthExceeded` envelope; C# answers at f(85) and refuses f(86). |
| Suite — LReal periodic/dispatch | 29 | Same fixed-width tier absence (the six `LRealDispatchTests` rows that only pin behaviour through the engine precision knob were ported). |
| Suite — TokenKind ordinals | 20 | Enum layout differs; the C# ordinals are not a Rust contract. |
| Suite — NameSuggestions / HelpService / Modus / CaptureState / Value API | 60 | These assert C# helper classes, plugin registration, events, or `Value.Widen`/`CaptureState`; no Rust API exists. |
| Suite — Dsp plugin contracts | 32 | Plugin-load/unregistered-engine semantics, `filter`/`noise`/`impulse`/`delay`/`movingavg`, and integer-argument contract types. |
| Suite — sign precedence / structured exactness / typed-array promotion / slices | 44 | Rust parses `-2^2` as 4, certifies `exact=true` where C# says false, keeps narrower array element kinds, and does not parse slice syntax. |
| Suite — parser/tokenizer residuals | 12 | Unary plus and chained assignment do not parse; the two unterminated-string position assertions differ (Rust reports the content start, C# the opening quote). |
| Run — integer boundary matrix | 93 | Cells need missing builtins/arities, accept non-integral integer arguments C# refuses, or cross with the wrong taxonomy. |
| Run — depth/recursion/value-depth/cancellation | 78 | No `DepthExceeded` code, no cancellation ledger, stack overflow instead of a typed refusal. |
| Run — taxonomy/missing wire fields | 179 | C# asserts `InvalidArgument`/`TypeMismatch` where Rust crosses `InvalidOperation`/`DomainError`; no `signature`, `suggestions`, print-budget fields, or failed-statement timing. |
| Symbolics — LaTeX | 213 | `latex` is advertised but answers `Unknown function 'latex'.`; no LaTeX printer. |
| Symbolics — pretty canonical/debug modes | 118 | Rust has no canonical S-expression parser and no debug print mode, so the two remaining theory methods cannot be reproduced. |
| Symbolics — kernel/API corpora | 663 | `Expr`/`Simplify`/`Assumptions`/`Calculus`/falsification APIs, RootOf/complex constants, nonlinear `solve_system`, and exactness provenance have no Rust counterpart or differ. |
| Complex.Tests | 121 | The C# `Complex`/`LComplex64`/`LComplex128` and `ComplexMath` provenance/pi-resolution APIs do not exist; Rust `Value::Complex` is only a numeric carrier. |
| Representation.Tests | 91 | `DigitStore` is a C# representation type; Rust `Real` stores a scaled `Int` with no digit-buffer API. |
| Dsp.Tests | 61 | `DspMath`/`Signal`/`Fourier`/`FixedDsp` APIs are absent; the language exposes only `fft`/`dft`/`conv`/`re`/`im`/`conj`, which do not implement `Fft`, filters, or fixed-width tiers. |

## 4. Observed verification

```text
$ powershell -NoProfile -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
crates=11 tests=3515 suiteExit=0 passed=3515 failed=0

$ cargo test --manifest-path rust/Cargo.toml --workspace
lovelace-lang 631 passed; lovelace-real 2328 passed; lovelace-run 137 passed;
every crate result "ok", 0 failed (3515 tests total)

$ node parity/gen-manifest.mjs
{"rows":6209,"dispositions":{"excluded":118,"ported":3749,"deferred":2342},
 "coveragePercent":60.38,"rustTests":3515}

$ node parity/tools/gate-manifest.mjs
{"censusCases":6209,"manifestRows":6209,
 "dispositions":{"ported":3749,"consolidated":0,"excluded":118,"deferred":2342},
 "coveragePercent":60.38,"claimedRustTests":3749,"projectsInCensus":15,"failures":[]}
PASS (structural): 6209 test cases dispositioned; 3749 claim Rust coverage (60.38%).

$ node parity/tools/coverage-report.mjs
coverage.md written: 3749/6209 ported (60.38 %)

$ cargo run --manifest-path rust/Cargo.toml -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total   (sym/071 and sym/076, unchanged)

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust: 20/20
PASS: oracle 20/20, rust 20/20
```

## 5. Concurrency and integrity notes

The workspace was being edited concurrently by another implementer during this round. A transient
full-suite failure appeared in `lovelace-run::pretty_batch::pretty_round_trips_to_the_same_canonical_form`
(`(x^y)^2` rendered as `x^y^2`); it was present in an isolated run and was fixed by the production
agent before the final verification. The final `gen-rust-tests` run above was taken after that fix
and is green. No production line was changed by this round; the only edits outside tests are the
`#[cfg(test)] mod` declarations.

`parity/manifest-map.json` was also edited concurrently before serialization was established. The
final file has 3,957 entries: 3,749 `ported`, 118 `excluded`, 90 curated `deferred`; the gate
reports no missing ported id and all 135 rows of this batch are present. The apparent "9 missing
Real rows" against the 2,305 figure is an accounting difference: the repository's round-7 report
and round-9 coverage both record **2,286** ported Real rows; 2,305 = 2,286 + 19 `DIGIT_BUDGET`
refusals. The ten Real rows this round replaced were already ported (`RealConstructorTests`
default/string/span constructors, `RealDivideLeadingZeroRunTests` neighbour), which is why the
`real_parity_batch` mapping count fell by ten while no census id became unported. No ids were
reconstructed by guesswork.

## 6. What was not verified

- The C# test suite cannot be executed in this environment (the VSTest testhost is denied
  `OpenProcess`; `parity/README.md`). C# assertions were read from source and, where an envelope
  was needed, replayed through the pinned C# runner. No C# test run is claimed.
- The 500-row target: 123 net new mappings were delivered. The refused pools above are the
  shortfall; closing them needs production work (LaTeX dispatch, complex/fixed-width types,
  trigonometric `Real`, depth/cancellation guards, symbolic `evalf`) that this scope forbids.
- `Lovelace.Complex.Tests`, `Lovelace.Representation.Tests` and `Lovelace.Dsp.Tests` were triaged
  class-by-class but every case in them was refused; no per-case Rust probe is claimed for the
  C#-type APIs.
- The 19 round-7 `DIGIT_BUDGET` Real rows remain refused for the same arithmetic-budget reason.

## 7. Files changed

- `rust/lovelace-lang/src/suite_lex_parse_batch.rs` — 49 new tests.
- `rust/lovelace-lang/src/suite_engine_batch.rs` — 41 new tests (65 census rows).
- `rust/lovelace-real/src/real_parity_round10.rs` — 20 new tests.
- `rust/lovelace-run/src/suite_batch.rs` — 1 new test.
- `rust/lovelace-lang/src/lib.rs`, `rust/lovelace-real/src/lib.rs` — `#[cfg(test)] mod`
  declarations only.
- `parity/manifest-map.json` — 135 curated `ported` entries with C# file/line and Rust test ids.
- `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md` — regenerated.
- `docs/goal-parity-contract/round-10/remaining-pools-batch.md` — this report.
