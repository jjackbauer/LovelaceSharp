# Round 42 — differential mismatch classes (D4)

Status: three production classes fixed and measured against the frozen oracle. The remaining
display family is the next package; nothing in `parity/`, the C# tree, the fixtures or the
documentation corpus was edited.

## How the classes were measured

Command (the gate's own command, no minimisation):

```
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --out <tmp>
```

Runners: oracle `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`
(sha256 f292249f…), Rust `rust/target/release/lovelace-run.exe` rebuilt from this tree after
every class. Each mismatch record was read from the run's `mismatches/*.json`: the class is the
first differing field path and the two values are the record's `expected` / `actual`. The
stored-replay counts are reported on the set the harness actually replayed (reference duplicates
of generated cases are skipped by design).

Baseline on the clean binary, before any edit:

| corpus | match | mismatch | error |
|---|---|---|---|
| generated (200) | 166 | 34 | 0 |
| replayed regressions (1,758 of 1,873 discovered) | 941 | 817 | 0 |

The earlier 136/64 generated and 1,022/851 replay figures, and the 236-record
`$.result.structured.canonical` reverse-ordering class, were measured against a release binary
built while the mutation gate had GATE-006/002 applied. They are mutation artefacts, not defects:
a clean rebuild reproduces 166/34 and removes that class entirely.

## Ranked class table (baseline, all 851 replay mismatches + 34 generated)

| rank | class (first differing field) | replay | generated | sample expected → actual |
|---|---|---|---|---|
| 1 | `$.result.display` | 392 | 14 | see the display breakdown below |
| 2 | `$.exitCode` | 145 | 6 | 1 → 0 (oracle refuses; Rust accepted) |
| 3 | `$.result.structured.canonical` (print budget) | 77 | 3 | `(add (rat 5 1) … (rat …` → `(add (rat 5 1) … ( …` |
| 4 | `$.result.kind` (det) | 74 | 3 | `"Real"` → `"Natural"`; `"Natural"` → `"Integer"` |
| 5 | limit conditions | 65 | 2 | a symbolic condition node → a Text element |
| 6 | `$.category` (reshape / 0^-1) | 45 | 4 | `"TypeMismatch"` → `"DomainError"` |
| 7 | `$.diagnostics[].message` (inv singular) | 23 | 1 | `"Matrix is singular and cannot be inverted."` → `"Matrix is singular."` |
| 8 | `$.diagnostics[].column` (undefined variable) | 18 | 1 | `1` → `6` (`diff(x)`) |
| 9 | `$.result.structured.elements[].denominator` | 8 | 0 | present `"1"` → missing (inv kinds) |
| 10 | `$.result.structured.elements[].kind` | 4 | 0 | `"Integer"` → `"Natural"` (inv kinds) |

The display class is a family of independent causes, not one defect:

| display subfamily | replay | generated | root cause |
|---|---|---|---|
| records/inv-full | 78 | 3 | missing det != 0 condition; inverse elements were Div nodes (nodeCount 7, not 9) |
| records/linsolve-full | 78 | 3 | solutions nested (shape [1]) instead of flat [2]; add-term display order |
| arrays/linsolve-symbolic | 66 | 2 | add-term display order: `5 - t` printed as `-t + 5` |
| arrays/shape-norm | 64 | 2 | Real display cap: oracle 100 fractional digits, Rust printed all 1,000 |
| records/integrate-full | 52 | 1 | unevaluated integral node form and exactness |
| symbolic/factor | 28 | 1 | product display order: `(x - 1)*x` instead of `x*(x - 1)` |
| symbolic/diff-product | 26 | 2 | `x^2 * x^-1` cancelled to `x` instead of staying a product |

## Fix 1 — exit codes (rank 2, 145 replay + 6 generated)

Three refusal paths accepted what the oracle refuses.

| probe | before | after (and oracle) |
|---|---|---|
| `# leading comment\n2 + 3` | exit 0, returns 5 | exit 1, ParseError, `Unexpected character '#' at position 0.`, position 0, column 1 |
| `résultat = 2; résultat` | exit 0, returns 2 | exit 1, ParseError, `Unexpected character 'é' at position 1.`, position 1, column 2 |
| `linsolve([[1, 2], [3, 4]], [1, 2])` | exit 0, `[0, 0.5]` | exit 1, InvalidOperation/DomainError, `linsolve() requires a symbolic matrix A.` |

Production change: the tokenizer no longer has a `#` line-comment branch (the reference has no
comment syntax at all; `%` is modulo), identifiers are ASCII (`is_ascii_alphabetic` /
`is_ascii_alphanumeric`; Unicode remains legal inside strings), and `linsolve`/`linsolve_full`
refuse a matrix with no symbolic element instead of running the numeric Cramer path. One symbolic
entry still makes the matrix symbolic; the numeric right-hand side is then fine.

Result: generated 172 match / 28 mismatch; replayed 1,080 / 678; new regressions 0.

## Fix 2 — print budget (rank 3, 77 replay + 3 generated)

The reference applies the kernel printer's character allowance `max(48, 6 x budget)` first and
only then the suite projection's proportional cut (`full x budget / nodes`) for renderings short
enough that the kernel leaves them whole. The port used the proportional rule for every case, which
moved every established cut point.

Probe `x = symbol("x"); diff(x^3 + 2*x^2 + 5*x + 7, x)` with `--print-budget 5`:

| | canonical |
|---|---|
| before | `(add (rat 5 1) (mul (rat 3 1) (pow ( …` |
| after = oracle | `(add (rat 5 1) (mul (rat 3 1) (pow (sym x) (rat  …` |

The pretty form is `3*x^2 + …` on both sides; `truncated: true`, `truncationReason:
"node-budget"` and `budget: 5` are unchanged. The short-value totality rule (a value over budget
is always abbreviated and says so) is preserved by the second layer.

Result: generated 175 match / 25 mismatch; replayed 1,154 / 604; new regressions 0.

## Fix 3 — matrix record shape (ranks 7, 9, 10 and the inv-full half of rank 1)

Six defects in the matrix builtins and record projection:

- `inv` of a numeric matrix now runs the reference's Gauss-Jordan over the numeric tier, so the
  published kinds follow the numeric operations. `inv([[1, 2], [3, 7]])` before: 7 Natural, 1
  Natural; after = oracle: 7 Integer, -2 Integer, -3 Integer, 1 Natural.
  `inv([[3, 5], [4, 6]])`: before -3 Integer / 2 Integer; after = oracle all four Real with
  numerator/denominator.
- The singular numeric inverse publishes the full sentence
  `Matrix is singular and cannot be inverted.` (was `Matrix is singular.`).
- A symbolic inverse element is `Multiply(adjugate, Power(det, -1))`, which is the reference's
  `Divide`, not a Div node: `nodeCount` 9 for `x/(x^2 - 1)` (was 7) while the canonical text is
  unchanged. `0/(x*y)` is still not folded away.
- `inv_full` carries the `det != 0` condition: `conditions: []` before, `[x^2 - 1 != 0]` after
  = oracle. It also refuses a matrix with no symbolic element, like the oracle
  (`inv_full() requires a symbolic matrix.`).
- `linsolve_full.solutions` is the flat solution vector (shape [2]), not a one-element vector
  holding it (shape [1]).

Result: generated 179 match / 21 mismatch; replayed 1,263 / 495; new regressions 0.

The inv-full records now agree on record shape, condition and element structure. The
linsolve-full records still mismatch on `$.result.display` alone: their remaining blocker is the
add-term display order, a separate class (see Remaining).

## Cumulative before/after

| after | generated match | generated mismatch | replayed match | replayed mismatch | new |
|---|---|---|---|---|---|
| baseline | 166 | 34 | 941 | 817 | 0 |
| exit codes | 172 | 28 | 1,080 | 678 | 0 |
| print budget | 175 | 25 | 1,154 | 604 | 0 |
| matrix record shape | 179 | 21 | 1,263 | 495 | 0 |

Net: +13 generated matches, +322 replayed matches, 335 fewer mismatches; no new regression in any
run (the same command, seed and corpus each time).

## Gates

| gate | result |
|---|---|
| corpus | `corpus: 214 passed, 0 failed, 214 total` |
| fixtures | oracle 20/20, rust 20/20, PASS |
| CLI | `CLI matrix: 16/16`, PASS |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` exit 0 |
| fmt | `cargo fmt --all -- --check` exit 0 |
| cargo test | 4,816 passed across the workspace; one pre-existing timing tripwire fails only under full-suite parallel load (see below) |
| gate-manifest | 1 failure: `rust-tests.json is stale` (inventory regeneration is an integration step, outside this lane's write scope) |

`assumptions_round35::eight_hundred_atoms_stay_ordered_and_fast` is a 250 ms/2,000 ms debug
timing tripwire inherited from the C# `Category=Timing` row. It passes alone (1.31 s measured,
ceiling 2,000 ms) and failed in the full parallel run; the structural assertions in the same test
pass. It is not caused by these changes (the test and its workload are untouched) but it makes a
single full-suite invocation flaky under load.

`parity/tools/gate-manifest.mjs` hashes every `.rs` file to prove the test inventory is fresh.
Any source change — these fixes included — makes `parity/rust-tests.json` stale until
`parity/gen-rust-tests.ps1` is run. That script writes `parity/`, which this lane was told not to
touch; the one failure is that staleness, not a manifest row.

## Remaining classes (next package order)

| class | replay | generated | note |
|---|---|---|---|
| display / linsolve-full | 75 | 6 | add-term display order only; record shape now correct |
| display / linsolve-symbolic | 64 | 2 | add-term display order |
| display / shape-norm | 62 | 2 | Real display cap (oracle 100 fractional digits) |
| display / integrate-full | 51 | 1 | unevaluated integral form / exactness |
| det kind | 71 | 3 | oracle det uses numeric elimination; 46 publishes Real, singular 0 Natural |
| limit conditions | 63 | 2 | conditions are Text elements instead of symbolic relations |
| reshape / 0^-1 taxonomy | 41 | 4 | TypeMismatch/InvalidArgument and the exact refusal prose |
| diagnostics position | 17 | 1 | evaluation errors report the call position instead of 0/column 1 |
| display / factor mul order | 27 | 1 | product display order |
| display / diff x^2/x | 24 | 2 | multiply power merge must keep positive and negative exponents apart |

## Files changed

| file | change |
|---|---|
| `rust/lovelace-lang/src/syntax.rs` | removed the `#` comment branch; ASCII-only identifiers |
| `rust/lovelace-lang/src/builtins.rs` | linsolve symbolic-matrix guard; numeric Gauss-Jordan inv; symbolic adjugate product and singular prose; inv_full condition/refusal; flat linsolve_full solutions |
| `rust/lovelace-lang/src/structured.rs` | two-layer print-budget allowance |
| `rust/lovelace-lang/src/lib.rs` | registered the round-42 test module |
| `rust/lovelace-lang/src/round42_differential_classes.rs` | 8 oracle-derived regression tests for the classes above |
| `rust/lovelace-lang/src/printing_round17.rs` | print-budget cut-point regression test |

No test was weakened, skipped or deleted. The regression corpus and its untracked files were left
untouched.


---

# Round 43 — display package (at a115403)

Baseline on the committed clean tree: generated 179 match / 21 mismatch; replay 1,263 / 495 on the
1,758 replayed regressions. Each class was fixed, then measured with the same command, seed and
corpus; "new" is the harness's count of regressions whose id+script+args did not mismatch before.

| after | generated match | generated mismatch | replayed match | replayed mismatch | new |
|---|---|---|---|---|---|
| baseline (a115403) | 179 | 21 | 1,263 | 495 | 0 |
| linsolve term order | 184 | 16 | 1,402 | 356 | 0 |
| Real display cap (norm) | 186 | 14 | 1,464 | 294 | 0 |
| integrate (unevaluated + verification token) | 187 | 13 | 1,515 | 243 | 0 |
| factor product order | 188 | 12 | 1,542 | 216 | 0 |
| diff x^2/x power split | 190 | 10 | 1,566 | 192 | 0 |

Net: +11 generated matches, +303 replayed matches; 314 fewer mismatches; no new regression.

## 1 — linsolve term order (144: 78 linsolve-full + 66 linsolve-symbolic)

The port rendered every sum in its stored degree order. The reference's
`Printing.OrderTermsForDisplay` is: series order when an O(...) term is present; descending degree
for a single-symbol polynomial; **canonical order otherwise**. A quotient sum such as
`5 - (4*x - 5)/(x^2 - 1)` is not a monomial, so the constant stays first. A negative leading
constant is rendered last as a subtraction (x - 1, never -1 + x).

Probe `x = symbol("x"); linsolve([[x, 1], [1, x]], [5, 4])`:

| | display |
|---|---|
| before | `[(-(4*x - 5)/(x^2 - 1) + 5)/x, (4*x - 5)/(x^2 - 1)]` |
| after = oracle | `[(5 - (4*x - 5)/(x^2 - 1))/x, (4*x - 5)/(x^2 - 1)]` |

`linsolve_full` publishes the same solutions and now agrees too. The existing Rust test that
asserted `(x^2 - 1)^N + 1` was corrected to the oracle's `1 + (x^2 - 1)^N` (the sum is not a
monomial).

## 2 — Real display cap (64: arrays/shape-norm)

The reference's `Real.DisplayDecimalPlaces` (100 by default, moved by setprecision) truncates a
non-periodic rendering; the port printed every computed digit. The cap is now a scoped display
setting in `lovelace-real`, applied by `Engine::format`/`format_typed`. It deliberately does NOT
apply to the structured `value` field: the oracle publishes the full 1,000-digit value there while
its `display` shows 100 digits (measured on `evalf(1/3, 1001)`).

| probe | before | after = oracle |
|---|---|---|
| `norm([7, 1, 1])` display | 1,001 chars | 102 chars (100 fractional digits) |
| `sqrt(51)` display | 1,001 chars | 102 chars |
| `evalf(1/3, 1001)` display / structured value | 1,002 / 1,002 | 102 / 1,002 |
| `setprecision(1000); sqrt(2)` | — | 1,002 chars (cap follows setprecision) |
| `1/3` | `0.(3)` | `0.(3)` (periodic values are never capped) |

## 3 — integrate unevaluated + verification token (52: 51 replay + 1 generated)

Two causes in one record family:

- An unevaluated integral is the reference's BINDING node `(integ (x) operand)`, counted as
  1 + operand nodes, exact, with two diagnostics: `integration.unevaluated` carrying the
  input-specific `integration.no-closed-form` in `details` AND at the top level. The port
  published `(fn integrate operand x)` (one node too many), `exactness: None` and an empty
  diagnostics array. The canonical parser now accepts `(integ (x) body)` so the canonical form
  round-trips.
- The solved cases published `differentiation+expansion` for every integral. The oracle's token
  is `differentiation+canonical` for an ATOM integrand (number, symbol, function application:
  sin(x), cos(2*x), exp(2*x)) and `differentiation+expansion` for a composite one (x^2, 3*x^2,
  1/x, x*exp(x), 2*x*cos(x^2), x^3+2*x) — measured over 15 integrands.

Probe `x = symbol("x"); integrate_full(exp(-x^2), x)`: display, expression canonical
`(integ (x) (fn exp ...))`, nodeCount 7, `exactness: Exact` and both diagnostics now equal the
oracle. `integrate_full(sin(x), x)`: `verification_method: differentiation+canonical`.

## 4 — factor product order (28: 27 replay + 1 generated)

The reference stores a product's factors in canonical order and prints that order; direct builders
(factor()) left a different stored order. The display now sorts product factors with the same
comparator canonical() uses. `factor(x^3 - x)`: before `(x - 1)*x*(x + 1)`, after = oracle
`x*(x - 1)*(x + 1)`.

## 5 — diff x^2/x power split (26: 24 replay + 2 generated)

The port combined same-base powers unconditionally, so `x^2 * x^-1` became `x` — defining the
product at 0 where x^-1 is undefined. The product now follows the reference's definedness-preserving
split: nonnegative and negative integer exponents accumulate separately, and merge only when the
combined exponent stays negative. `diff(x^2*log(x), x)`: before `x + 2*x*log(x)`, after = oracle
`2*x*log(x) + x^2/x`. `x^2*x^-3` still merges to `x^-1`; `simplify(x/x)` stays `x/x`.

## Gates after the display package

| gate | result |
|---|---|
| corpus | `corpus: 214 passed, 0 failed, 214 total` |
| fixtures | oracle 20/20, rust 20/20, PASS |
| CLI | `CLI matrix: 16/16`, PASS |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` exit 0 |
| fmt | `cargo fmt --all -- --check` exit 0 |
| cargo test | 4,823 passed, 0 failed across the workspace (19 test binaries; +6 round-43 tests) |
| gate-manifest | 1 failure: `rust-tests.json is stale`; regeneration is the integrator's step |

## Remaining after the display package (192 replay, 10 generated)

| class | replay | generated | note |
|---|---|---|---|
| det kind | 71 | 3 | det must use numeric elimination (46 Real, singular 0 Natural) |
| limit conditions | 63 | 2 | conditions are Text elements instead of symbolic relations |
| reshape / 0^-1 taxonomy | 41 | 4 | TypeMismatch/InvalidArgument and the exact refusal prose |
| diagnostics position | 17 | 1 | evaluation errors report the call position instead of 0/column 1 |

The whole display family is therefore closed at the measured classes: every remaining replay
mismatch is in one of the four rows above.

## Round-43 files changed

| file | change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | OrderTermsForDisplay (series/polynomial/canonical + negative leading constant); canonical product order in display and signed text; integrate binding node canonical/node count; `(integ (x) body)` parser; definedness-preserving power merge; sqrt factor normalization |
| `rust/lovelace-real/src/lib.rs` | scoped `DisplayDecimalPlaces` thread-local and its application in render |
| `rust/lovelace-lang/src/lib.rs` | Engine::format/format_typed under the display-precision scope; module registration |
| `rust/lovelace-lang/src/builtins.rs` | unevaluated integral exactness + two diagnostics; verification token rule |
| `rust/lovelace-symbolics/src/printing_round16.rs` | the `(integ ...)` refusal row becomes a grammar-violation row (the tag is vocabulary now) |
| `rust/lovelace-lang/src/symbolics_round23.rs` | huge-degree sum expectation corrected to the oracle's canonical display order |
| `rust/lovelace-lang/src/round43_display_classes.rs` | 6 oracle-derived regression tests for the five classes |

