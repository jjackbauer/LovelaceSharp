# Round 29 — blocker ledger and coverage-driven production fixes

Scope: rust/ production and tests. No .cs file, fixture, parity/ artifact or other document was
changed; this report is the only non-rust file. The manifest was NOT re-mapped in this round
(parity/ is out of scope), so the coverage number is unchanged; section 5 lists the exact rows the
fixes make mappable.

Oracle for every probe: the pinned runner
```
dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval '<script>' --omit-functions --omit-variables
```
(manifest oracle commit 1cc28f255eca6029849f880949fc7be1d40ab8af). Rust: the release
rust/target/release/lovelace-run.exe with the same arguments. Comparisons normalise only the four
declared volatile fields (revision, elapsed, elapsedTime, timings) and canonicalise JSON key order;
everything else is compared byte for byte.

## 0. The population and how the ledger was built

| Measure | Rows |
|---|---:|
| Census cases (parity/manifest.toml) | 6,209 |
| Ported | 4,975 (80.13 %) |
| Deferred (this ledger's population) | 1,116 |
| Approved exclusions (absent subsystems) | 118 |

Method:

1. parity/manifest.toml was parsed per row; deferred rows were grouped by (project, class, method),
   giving 834 distinct deferred class/method groups. Only 106 of the 1,116 rows carried a
   cause-specific refusal reason; the other 1,010 carried the generator placeholder ("No Rust test
   has been mapped to this capability yet."), so the placeholder is not evidence of a cause.
2. Each group was assigned to a blocker FAMILY from its C# assertion family, and every family with
   a plausible single fix was probed: the representative script (or the C# type/method contract
   where the row is not wire-reachable) was driven against the oracle and the Rust runner, and the
   exact divergence recorded.
3. Table A maps all 1,116 deferred rows to exactly one family (the counts sum to 1,116). Table B
   ranks the coherent, single-fix causes - a family such as "Symbolics kernel hardening" (45 rows)
   is NOT ranked as one cause because it is 45 different assertions.

The absence of a cause in a row's reason field is a finding in itself: 1,010 deferred rows cannot
be planned from the manifest as it stands, which is why the ledger is the round's primary artifact.

## 1. Table A — every deferred row by blocker family

| # | Blocker family | Rows | Evidence / nature | Disposition |
|---:|---|---:|---|---|
| 1 | Other Symbolics surface | 281 | Long tail of 51 classes, each a different kernel/printer/DTO behaviour (largest: DxStructuredResultsTests 17, DxSemanticClosureTests 13, FalsificationLoopTests 13, AcceptanceTests 11, ComplexClosedFormTests 11, IndeterminateFormTests 11). No single fix. | — |
| 2 | Other Suite API surface | 97 | Long tail of separate API mirrors (ValueTests 9, HelpServiceTests 8, DspPluginTests 7, ScriptSourceTests 7, SuiteEngineTests 7, Modus 6, Plot 5, ...). One surface each. | — |
| 3 | Representation crate absent (consolidated) | 91 | No Lovelace.Representation crate in the Rust workspace; parity/projects.toml dispositions the C# project consolidated. Porting it would invent a subsystem. | — |
| 4 | LReal64/LReal128 fixed-width tier absent | 84 | 58 LReal64 + 26 LReal128 rows assert ToString parity with Real and the promotion refusal; no fixed-width type existed in Rust. | FIXED 3.1 |
| 5 | Other Real API/format surface | 53 | Separate Real APIs: trig fast paths 10, modulus 8, properties 6, sqrt batch 5, ToString 4, pi segments 3, ... | — |
| 6 | Other Natural API surface | 51 | Separate Natural APIs: structure 15, increment 9, static properties 8, decimal cache 7, subtract 5, ... | — |
| 7 | Symbolics kernel hardening | 45 | 45 rows, 45 DISTINCT kernel assertions (complex powers, RootOf, limits, huge exponents, MathIR versions, simplify conditions). Not one cause. | — |
| 8 | Symbolics series at a kink/pole | 39 | series() at a kink (abs(x)) returns the input unchanged; the Laurent/pole rows need the kink expansion. | — |
| 9 | Integer surface gaps | 34 | Separate Integer APIs (increment 8, predicates 8, constructors 5, negation 3, pow 3, ToString 3, ...). | — |
| 10 | TokenKind model/ordinals absent | 20 | The C# TokenKind ordinals 0..19 have no Rust mirror; the active lexer uses syntax::Tok (9 variants, no discriminants). | — |
| 11 | Integer-argument contract (concat/dft/range/axis) | 19 | 12 boundary cells + 7 inventory templates: concat/dft/range silently floored a non-integer, reductions refused a Text axis as prose without the builtin or position. | FIXED 3.3 |
| 12 | DSP surface gaps | 18 | 18 singleton rows across the DSP plugin surface. | — |
| 13 | Real periodic add/sub truncates the exact fraction | 18 | 9 Add + 9 Subtract corpus rows at q=19/s=17: the exact fraction was expanded with a budget of max(operand fractional digits)+40, cutting a 144-digit period. | FIXED 3.2 |
| 14 | Symbolics family completeness (circular) | 18 | Circular candidates are not represented and the family is not reported incomplete. | — |
| 15 | Symbolics pretty/latex grouping | 18 | The pretty printer flattens nested subtraction and division (x/(y*z) instead of x/y/z), so the re-parsed canonical form and the LaTeX tree differ. | — |
| 16 | Symbolics help metadata aliases | 17 | CategoryAliases_Resolve: the help/registry metadata carries no category aliases. | — |
| 17 | Other Run wire surface | 16 | 16 singleton runner rows (plot file errors, stdout purity, --text surface, arity metadata, ...). | — |
| 18 | evalir digit-count contract residue | 15 | EvalirDigitArgumentContractTests rows still deferred after round 27 (named-kind refusals for wide/Real counts). | — |
| 19 | Inverse-branch candidate verification | 15 | Every represented solution must satisfy the original equation; branch verification gap. | — |
| 20 | System solve proof records | 15 | Wire records for a derived contradiction or an absent solution set. | — |
| 21 | Natural randomized cross-check (C# RNG) | 13 | Compares against BigInteger over System.Random sequences; the RNG sequence is not reproducible in Rust. | — |
| 22 | precbench crate absent | 13 | No Rust precbench crate; the project is not in the parity project ledger. | — |
| 23 | Real trig: 1e20/1e100 range reduction + no special-angle table | 12 | cos(pi/6) is one ulp off and the 1e20/1e100 turn quotient overflows i64; needs a special-angle table plus big range reduction. | — |
| 24 | inspect record schema + exactness | 11 | inspect(sqrt(2)).exact was true, complex/Text/Boolean/array values were refused, and the record always claimed type Symbolic. | FIXED 3.4 (10 of 11) |
| 25 | Symbolics exactness flags | 11 | Elementary function of an approximate argument publishes the wrong exact flag. | — |
| 26 | Symbolics truncated-evaluation exactness | 11 | Truncating-evaluation rows (symbolic tree flag vs numeric read-back). | — |
| 27 | Symbolics relations/families | 10 | SinFamily members do not verify numerically. | — |
| 28 | Record schema validation absent | 9 | No RecordSchemas.Validate mirror. | — |
| 29 | Array surface gaps | 8 | 8 singleton Array rows. | — |
| 30 | Real constructors from decimal/double | 8 | No constructor from decimal/double (digits/exponent). | — |
| 31 | Builtin shape guard/prose | 7 | Four audit reproductions still cross as prose/arity; the two whole-registry sweeps also collect violations. | — |
| 32 | System builtin argument-shape refusals | 7 | solve_system/solve_system_full/jacobian wrong shapes cross as DomainError prose, not the documented argument error. | — |
| 33 | Descriptor examples call unknown builtins | 7 | assume_clear/scale/filter/ndims are advertised but unknown to the dispatcher. | — |
| 34 | Complex surface gaps | 6 | 6 singleton Complex rows. | — |
| 35 | Usage-documentation examples | 6 | 6 README/guide examples; fixture text, outside production scope. | — |
| 36 | Real precision scopes are not async-local | 4 | The precision scope is not AsyncLocal; concurrent-task isolation rows. | — |
| 37 | solve: folded relation + unevaluated system | 4 | A relation folded to a Boolean before the kernel was refused as "solve needs an equation"; an unsupported system crossed as "No solver for this system." | FIXED 3.5 |
| 38 | limit short-form record message | 3 | limit(a/x,x,0) reports "coefficient does not evaluate at the point", not "leading coefficient is symbolic". | — |
| 39 | Recursion budget composition values | 2 | for/while compositions answer Void where C# answers the deepest call value. | — |

## 2. Table B — coherent single-fix causes, ranked by rows

"One fix?" means the rows are blocked by one production behaviour, not by N unrelated behaviours.
The decision column is the round's selection: coverage per unit of well-scoped production change.

| Rank | Cause | Rows | One fix? | Evidence | Decision |
|---:|---|---:|:--:|---|---|
| 1 | LReal64/LReal128 fixed-width tier absent | 84 | yes | no such type in the workspace; every row asserts the C# type's ToString parity with Real and its promotion refusal | **FIXED 3.1** |
| 2 | Integer-argument contract (concat/dft/range/axis) | 19 | yes | concat/dft/range accepted 2.5 by flooring; reductions refused Text as prose | **FIXED 3.3** |
| 3 | Real periodic add/sub truncates the exact fraction | 18 | yes | 1/19+1/17 has a 144-digit period, expanded with a 58-digit budget | **FIXED 3.2** |
| 4 | Symbolics pretty/latex grouping | 18 | yes (one printer area) | x/(y*z) instead of x/y/z; nested subtraction flattened | not attempted (broad printer change; would move many ported renderings) |
| 5 | Symbolics family completeness (circular) | 18 | yes (one family representation) | circular candidates not represented | not attempted (kernel feature) |
| 6 | Symbolics help metadata aliases | 17 | yes (one metadata table) | no category aliases in the registry metadata | not attempted (breadth of alias data) |
| 7 | Real trig 1e20/1e100 + special angles | 12 | no | no special-angle table; i64 turn-quotient overflow | not attempted (kernel feature) |
| 8 | Complex closed forms | 11 | no | negative-base rational powers refused | not attempted (principal-branch subsystem) |
| 9 | Indeterminate forms | 11 | unclear | infinity/indeterminate arithmetic rows (11 assertions); not probed | not attempted |
| 10 | inspect record schema + exactness | 11 | yes | exact true for sqrt(2); complex/Text refused | **FIXED 3.4 (10 of 11)** |
| 11 | System builtin argument-shape refusals | 7 | yes | solve_system/jacobian wrong shapes cross as DomainError prose | not attempted (next round's cheapest Run cause) |
| 12 | Builtin shape guard/prose | 7 | yes | matmul/symbol/assume/transpose + two registry sweeps | not attempted |
| 13 | solve: folded relation + unevaluated system | 4 | yes | folded Boolean refused as "needs an equation" | **FIXED 3.5** |

Selected: five coherent causes, **135 expected rows** (84 + 19 + 18 + 10 + 4). Causes 4-9 were
ranked above some selected causes but each is either a broad kernel/printer change whose blast
radius is the whole Symbolics surface (4, 5, 6) or needs a subsystem this round cannot honestly
claim (7, 8). Cause 11 is the next cheapest (7 rows, mechanical), and is recorded for round 30.

## 3. The fixes, with before/after probes

### 3.1 LReal64/LReal128 — the fixed-width exact-decimal tier (84 rows)

Cause: C# Lovelace.Real.LReal64 (64-bit significand, 19 significant digits) and LReal128
(128-bit, 38) had no Rust counterpart, so 58 + 26 rows that assert "the fixed-width value renders
exactly like class Real, or throws LRealPromoteException" could not be ported.

Fix: new production module rust/lovelace-real/src/lreal.rs (753 lines): a width-generic
Fixed<W: LRealWidth> with the reference's own algorithm - decimal/period parsing with the u64/u128
significand capacity, normalization, add/subtract/multiply/divide including the fixed 18/37-place
working expansion of a periodic operand, period re-detection, the promotion refusal (period longer
than 19/38, significand wider than the width), sign-correct comparison and the C# rendering
(including display truncation and the periodic form). LRealError::Promote is the
LRealPromoteException counterpart; LReal64 and LReal128 are type aliases.

Before/after: the runner never exposes LReal, so the oracle here is the C# assertion itself (the
row compares the fixed-width type's rendering with class Real's for the same expression). The
transcribed assertions were executed in Rust and are green:

| Row family (C# method) | Rows | After (Rust) |
|---|---:|---|
| LReal64Tests.Parse_GivenValue_RoundTripsLikeReal | 21 | 21/21 equal to Real's rendering |
| LReal64Tests.Add/Subtract/Multiply/Divide_GivenPair_MatchesReal | 26 | 26/26 equal |
| LReal64Tests.Add_Periodic / Add_Tenths / Compare | 7 | equal; 0.1+0.2 = "0.3" |
| LReal64Tests.Divide_LongPeriod / Add_Overflow / Multiply_Overflow / Parse_TooManyDigits | 4 | each refuses with Promote/None as the C# row requires |
| LReal128Tests.Parse_GivenValue_RoundTripsLikeReal | 10 | equal |
| LReal128Tests.Add/Multiply/Divide_GivenPair_MatchesReal | 12 | equal (incl. 28-digit 9999...9 + 1e-9, 1/29's 28-digit period) |
| LReal128Tests.Add_Periodic / Add_Tenths / Multiply_Overflow / Divide_LongPeriod | 4 | equal / refuses |

The tests live in rust/lovelace-real/src/lreal_tests.rs (20 functions, one per C# method, each
iterating the method's own rows and naming the literal in the failure message).

### 3.2 Real periodic add/sub — the exact fraction was truncated (18 rows)

Cause: every periodic add/subtract converted the operands to exact fractions and then re-expanded
the result with a digit budget of max(operand fractional digits) + 40. 1/19 + 1/17 has a 144-digit
period, so the published value was a 58-digit truncation marked inexact - a wrong exact value.

Probe (wire), before is the recorded manifest refusal + the algorithm's budget; after is measured:

| Script | Oracle | Before | After |
|---|---|---|---|
| 1/19 + 1/17 | 0.(111455...214068), 146 period digits | truncated at 58 places, inexact | **byte-identical to the oracle** (148-char display) |
| 1/17 + 1/19 | same value | same truncation | identical |
| 1/19 - 1/17 | -0.(006191...6749226) | truncated | identical |
| 1/3 - 0.333...(1000 threes) | 0.000...0(3), PeriodStart 1000, PeriodLength 1, exact false | truncated (the leading zeros were charged to the budget) | identical, PeriodStart 1000, PeriodLength 1, Exponent -1001, exact false at the 1001-place cap |
| 1/3 - 0.333...(999 / 998 threes) | PeriodStart 999 / 998, exact true | truncated | identical, exact true (the cap is the 1001st place) |

Fix: fraction_real_exact cancels the tens the numerator and denominator SHARE (a no-op on the
value), carries only the leftover denominator tens on the exponent (so a cancellation's leading
zeros do not spend the digit budget), detects the period under the reference's 1000-place
computation cap, marks the value inexact when the detected period only closes PAST that cap
(period start + period length > 1000; the reference marks 1000-threes inexact while 998/999 stay
exact), and normalizes the result. The periodic paths of add and mul use it.

Tests: rust/lovelace-real/src/round29_real_batch.rs (4 functions): the full 2,592-pair corpus of
the C# theory (12 periodic left denominators x 3 numerators x 8 right denominators x 3 numerators,
add and subtract), with the expected rational derived test-side from p, q, r, s and the actual
rational reconstructed from the value's public state (magnitude, exponent, period start/length) -
not from a production conversion helper; plus the 1/(3*10^1000) leading-zero fact and the
998/999/1000-threes exactness boundary.

Regression found and fixed during the round (recorded, not hidden): the first cut of
fraction_real_exact stripped denominator tens without cancelling the shared ones, so
(4/3)*(3/4) rendered "1.00" and 1/6+1/6 rendered "0.00(3)". Three existing tests caught it
(mul_fourThirdsTimesThreeQuarters_IsExactlyOne, r7_add_one_sixth_plus_one_sixth,
r15_periodic_quotient_times_its_divisor_is_the_dividend); the cancellation step was added and all
three pass.

### 3.3 Integer-argument contract: concat / dft / range / reduction axis (19 rows)

Cause: three call sites did not read their integer argument through the shared contract (they
floored or ignored it), and the reductions' axis refusal was prose.

Probes (oracle vs Rust), from the boundary matrix's own cells:

| Script | Oracle | Before | After |
|---|---|---|---|
| concat([1, 2], [3, 4], 2.5) | InvalidArgument/TypeMismatch "concat(): argument 3 (axis) must be a Natural or Integer that denotes an exact integer; got the Real 2.5." | accepted, [1, 2, 3, 4, 2.5] | identical to oracle |
| concat([1, 2], [3, 4], 0) | [1, 2, 3, 4] | [1, 2, 3, 4, 0] (axis appended as a value) | [1, 2, 3, 4] |
| concat([1, 2], [3, 4], 1) | "argument 3 (axis) must be the only axis (0); got 1." | accepted | identical to oracle |
| concat([1, 2], [3, 4], "x") | "... got the Text." | accepted | identical to oracle |
| dft([1, 2, 3, 4], 2.5) | "dft(): argument 2 (n) must be a Natural or Integer that denotes an exact integer; got Real 2.5." | accepted, 4-point transform | identical to oracle |
| dft([1, 2, 3, 4], 2) | [3, -1] | [10, -2 + 2i, -2, -2 - 2i] | [3, -1] |
| dft([1, 2, 3, 4], 0) / 6 | [] / 6-point | ignored | [] / 6-point |
| dft([1, 2, 3, 4], "sentinel") | "... got String." | accepted | identical to oracle |
| dft(..., -1) / 10^30 | "argument 2 (n) ... between 0 and 2147483647; got -1." / the digits | accepted | identical to oracle |
| 1..2.5 / 0.5 / 2.9999 / (5/2) | "range(): argument 'bound' ... got the Real 2.5." | floored to [1, 2] | identical to oracle |
| 1..2.0 | "argument 'bound' must be a Natural or Integer; got the Real 2, which is integral - write it as 2." | accepted | identical to oracle |
| 1.."sentinel" | "range(): argument 'bound' ... got the Text." | DomainError "Range bounds must be numeric." | identical to oracle |
| sum([[1,2],[3,4]], "sentinel") | "sum(): argument 2 (axis) must be a Natural or Integer that denotes an exact integer; got the Text." | DomainError "Axis must be an integer." | identical to oracle (prod/min/max too) |

Fix: concat reads its third argument as an axis through exact_integer_argument (any axis other than
0 is the axis-range refusal, and the axis is never appended as an element); dft reads n through the
DSP integer reader with the reference's 0..2147483647 bound, pads/truncates the samples to exactly
n and answers [] for n = 0; range reads both bounds through exact_integer_argument (scaling the
integer loop from the parsed i64); sum/prod/min/max read their axis through the same contract.
DspIntBounds gained the NonNegative32 arm with the reference's own bound text.

Tests: rust/lovelace-run/src/round29_run_batch.rs (4 functions), wire-level through
parity_batch::run_args.

### 3.4 The inspection record (10 of 11 rows)

Cause: inspect always published type Symbolic, converted the value to a symbolic expression (losing
the Real's provenance), refused every non-symbolic value, and put the math domain in
element_domain for scalars.

Probes:

| Script | Oracle | Before | After |
|---|---|---|---|
| v = sqrt(2); i = inspect(v) | type Real, exact False, all symbolic fields Null | type Symbolic, domain rational, exact True, node_count 1, canonical (rat ...) | oracle-equal |
| v = 1/1009; i = inspect(v) | type Real, exact True | type Symbolic, exact True (coincidentally) | oracle-equal |
| setprecision(18); v = 1/1009; i = inspect(v) | type Real, exact False | type Symbolic, exact True | oracle-equal |
| v = dft([1, 2, 3])[0] / [1] | type Complex, exact True / False | refused "inspect needs an expression." | oracle-equal |
| v = "hi" / is_even(4) / [1, 2] | type Text / Boolean / Vector, exact Null, shape/rank for the vector | refused | oracle-equal |
| inspect(x^2 + 1) (already-ported rows) | unchanged display | type Symbolic, domain complex, ... | unchanged (same field order and values) |

Fix: inspect now dispatches on the value kind and builds the reference's uniform 12-field record -
type from the value's own kind/record name, domain only from Symbolic/Domain, exact from the
value's provenance (Symbolic/Real/Complex/Natural/Integer; Null elsewhere), free_symbols/node_count/
canonical/pretty only for Symbolic, shape/rank only for Vector/Array, element_domain from the first
element for arrays and from DomainValueOf for scalars (Null for a Real), assumptions empty (the
engine has no bridge), members only for Record.

Remaining: Inspect_AnswersExactnessAsABoolean("(-4)^(1/2)", true) still fails at EVALUATION - the
Symbolics kernel refuses non-integer exponents - so that row stays deferred and is counted in the
Symbolics kernel family, not here.

### 3.5 solve: folded relations and unsupported systems (4 rows)

Cause: the interpreter folds 0 == 1 to a Boolean before the kernel, so solve refused it as "needs
an equation"; an unsupported but well-shaped system crossed as the DomainError "No solver for this
system." instead of the honest record.

Probes:

| Script (symbols declared) | Oracle | Before | After |
|---|---|---|---|
| solve(0 == 1, x) | SolveResult NoSolutions, complete True, diagnostic solve.no-solutions "the equation reduces to a nonzero constant." | DomainError "solve needs an equation." | oracle-equal |
| solve_full(2 == 3, x) | same record | same refusal | oracle-equal |
| solve_system([0 == 1], [x]) | SystemSolveResult NoSolutions, complete True, no diagnostics | DomainError "solve_system needs equations." | oracle-equal |
| solve_system([sin(x) == 0], [x]) | SystemSolveResult Unevaluated, complete False, diagnostic system-solve.unevaluated "...only polynomial systems are supported in v1." | DomainError "No solver for this system." | oracle-equal |
| solve(1 == 1, x) | Unevaluated, diagnostic "0 = 0: every value is a solution." | refusal | oracle-equal (control) |
| solve_system([x + y == 2, x - y == 0], [x, y]) | Solved record | Solved record | unchanged (control) |

Fix: solve and solve_system recognise a folded Boolean equation (false is the complete empty set,
true is the non-enumerated identity/contradiction), and solve_system's "this solver does not
enumerate it" branch publishes the reference's Unevaluated record with its diagnostic instead of an
error. Two small constructors (unevaluated_solve_result, unevaluated_system_result) carry the
record shapes.

Tests: 4 functions in rust/lovelace-run/src/round29_run_batch.rs.

## 4. Gates

```
$ cd rust; cargo test --workspace --no-fail-fast --offline
array 58, desktop 67, int 125, json 7, lang 1,133, nat 134, real 2,397, run 330,
symbolics 52 (+1 integration) = 4,304 tests collected, 0 failed, exit 0.
(round-28 baseline 4,267; +37 new tests: 20 LReal, 4 Real corpus, 13 Run wire)
```

```
$ rust/target/release/lovelace-corpus.exe        (from the repo root)
FAIL sym/071    (pre-existing)
FAIL sym/076    (pre-existing; C# renders -x/x/(x*y), Rust -1/(x*y))
corpus: 212 passed, 2 failed, 214 total

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-cli.mjs
CLI matrix: 16/16
PASS: every RUN-001..RUN-014 row behaves like the oracle

$ node parity/tools/gate-manifest.mjs
{ "censusCases": 6209, "manifestRows": 6209,
  "dispositions": { "ported": 4975, "consolidated": 0, "excluded": 118, "deferred": 1116 },
  "coveragePercent": 80.13, "claimedRustTests": 4975, "projectsInCensus": 15,
  "failures": ["rust-tests.json is stale: the Rust sources changed since the inventory was
                measured (run parity/gen-rust-tests.ps1)"] }
FAIL: 1 issue(s)
```

Corpus, fixtures and CLI are unchanged from the round-28 baseline (212/214, 20/20, 16/16); the
two corpus failures are the round-12 baseline and are not touched by this round.

The manifest gate's structural checks pass (6,209/6,209 dispositioned, 4,975 claimed, 80.13 %);
its single failure is the STALENESS guard: parity/rust-tests.json was measured before these
production edits, so the gate refuses to certify the inventory. Regenerating it is the owner's step
(parity/ is out of scope here) and clears the failure:
powershell -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1, then node parity/gen-manifest.mjs.
The suite that regeneration measures is green (4,304/0 above). The reported coverage number is
therefore unchanged at 4,975/6,209 (80.13 %); the 135 rows in section 5 are the rows this round
makes mappable.

Not run this round: cargo fmt/clippy (D10 was not part of this round's gate set) and the
differential runner (its case generator does not cover the changed surfaces; the targeted probes
above are the evidence). Both are recorded as not-run rather than assumed green.

## 5. Rows ready to map (not mapped: parity/ is out of scope this round)

All ids below are the deferred manifest ids whose assertion the new tests reproduce. The parent can
map them to the named Rust test; none is a weakened assertion.

| Cluster | Rows | Manifest ids | Rust test |
|---|---:|---|---|
| LReal64 | 58 | every deferred id in Lovelace.Real.Tests.LReal64Tests (Parse #0-#20, Add #0-#5, Subtract #0-#4, Multiply #0-#3, Divide #0-#10, Compare #0-#4, Add_Periodic, Add_Tenths, Divide_LongPeriod, Add_Overflow, Multiply_Overflow, Parse_TooManyDigits) | lreal64_* (12 functions) |
| LReal128 | 26 | every deferred id in Lovelace.Real.Tests.LReal128Tests (Parse #0-#9, Add #0-#3, Multiply #0-#2, Divide #0-#4, Add_Periodic, Add_Tenths, Multiply_Overflow, Divide_LongPeriod) | lreal128_* (8 functions) |
| Real periodic | 18 | RealAddPeriodicExactTests Add/Subtract at #582, #583, #584, #606, #607, #608, #630, #631, #632 | real_add_periodic_pairs_denote_the_exact_rational_sum / real_subtract_periodic_pairs_denote_the_exact_rational_difference |
| Integer contract | 19 | IntegerArgumentBoundaryMatrixTests EveryBoundaryCell #116-#119, #216-#219, #308-#311; EveryInventoryTemplate #5, #6, #7, #8, #11, #21, #30 | integer_boundary_concat_dft_and_range_refuse_a_non_integral_argument / ..._inventory_templates_reach_the_argument_they_name / ..._accepted_cells_publish_the_value_the_caller_wrote / ..._dft_named_bounds_are_typed_refusals |
| inspect | 10 | InspectionExactnessTests Inspect_AnswersExactnessAsABoolean #3, #5, #6; InspectExact_AgreesWithTheValuesOwnExactFlag #2, #4, #5; Inspect_KeepsNullWhereExactnessDoesNotApply #0, #1, #2; TruncatedQuotientAtTheBudget #0 | inspection_* (4 functions) |
| solve | 4 | SolveEmptySetTests 3 rows (Solve_OfAProvablyFalseEquation..., SolveFull_OfAProvablyFalseEquation..., SolveSystem_OfAProvablyFalseEquation...); SystemSolveRecordTests #2 | solve_of_a_folded_false_equation_is_the_empty_set / solve_full_... / solve_system_of_a_folded_false_equation_answers_the_empty_set / solve_system_of_a_non_polynomial_system_is_an_unevaluated_record |
| **total** | **135** | | |

Inspect_AnswersExactnessAsABoolean #4 ("(-4)^(1/2)") is NOT in the list: the evaluation itself is
still refused by the Symbolics kernel.

## 6. What remains, ranked

1. The 281-row Symbolics tail (51 classes; DxStructuredResults 17, DxSemanticClosure 13,
   FalsificationLoop 13, Acceptance 11, ComplexClosedForm 11, IndeterminateForm 11): one distinct
   kernel/printer/DTO behaviour each. Not a single fix; the largest coherent sub-causes are the
   pretty/latex grouping (18) and the circular-family representation (18).
2. The 97-row Suite API tail (Value 9, HelpService 8, DspPlugin 7, ScriptSource 7, SuiteEngine 7,
   Modus 6, Plot 5, ...): one missing public surface each.
3. The 91-row Representation gap: a whole absent (consolidated) crate; out of scope without
   inventing a subsystem.
4. Cheap next causes: system builtin argument-shape refusals (7), builtin shape guard/prose (7),
   limit short-form message (3).
5. Real trig (12) needs a special-angle table and a big-magnitude range reduction; complex closed
   forms (11) needs the principal-branch subsystem.

## 7. Files changed

| File | Change |
|---|---|
| rust/lovelace-real/src/lreal.rs | new: the fixed-width tier (753 lines) |
| rust/lovelace-real/src/lreal_tests.rs | new: 20 tests, 84 census assertions |
| rust/lovelace-real/src/round29_real_batch.rs | new: 4 tests, the exact-period property corpus |
| rust/lovelace-real/src/lib.rs | module registration; fraction_real_exact; DEFAULT_COMPUTATION_DECIMAL_PLACES; periodic add/mul paths |
| rust/lovelace-lang/src/builtins.rs | concat axis, dft n, range bounds, reduction axis, inspect record, solve/solve_system folded relations + Unevaluated records, DspIntBounds::NonNegative32, two result constructors |
| rust/lovelace-run/src/round29_run_batch.rs | new: 13 wire-level tests |
| rust/lovelace-run/src/main.rs | cfg(test) registration of round29_run_batch |
| docs/goal-parity-contract/round-29/blocker-ledger.md | this report |
