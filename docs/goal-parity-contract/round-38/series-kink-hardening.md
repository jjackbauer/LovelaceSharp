# Round 38 — series at a kink and the kernel-hardening residuals

Status: complete for the rows below; sections 4 and 5 carry the per-row evidence and the measured
refusals. 41 of the 52 deferred rows in these two families now have a passing Rust test: the whole
30-row series-at-kink family and 11 of the 22 kernel-hardening rows.

## 0. Population and result

| Measure | Rows |
|---|---:|
| Deferred census rows at round start — `SeriesKinkDeterminismTests` | 30 |
| Deferred census rows at round start — `KernelHardeningTests` (residual) | 22 |
| (a) mapped to a Rust test in this round — series | 30 |
| (a) mapped to a Rust test in this round — kernel | 11 |
| (b) re-probed and refused (measured divergence) | 11 |

The round-36 document named 20 kernel rows; the census (`parity/manifest.toml`) carries 22
`KernelHardeningTests` deferred ids, and the census is the population used here.

## 1. Series at a kink (30 rows)

The published `series(f, x, x0, n)` contract is SymPy 1.14.0's RIGHT-HAND expansion. At a KINK —
a point where the function has no derivative, i.e. an `abs(g)` whose argument vanishes there — the
expansion is taken from one side, and SymPy resolves the kink BEFORE differentiating:
`Abs._eval_nseries` multiplies the argument's own series by `sign(arg.leadterm(x)[0])`, the sign of
the argument's leading COEFFICIENT. Round 36 measured the old tree publishing the source function
(`abs(x)`) and a canonical form with no `order` node. Both are repaired.

Probes below are the oracle and the Rust runner on the SAME argv
(`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval '<script>' --omit-functions`).
"Before" is this lane's first measurement on the pre-repair tree.

| # | Script | Before (Rust) | After (Rust) = oracle |
|---:|---|---|---|
| 1 | `series(abs(x), x, 0, 3)` | `abs(x)`; `(fn abs (sym x))` | `x + O(x^3)`; `(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 2 | `series(abs(x), x, 0, 2)` | `abs(x)`; `(fn abs (sym x))` | `x + O(x^2)`; `(add (sym x) (order (sym x) (rat 0 1) (rat 2 1)))` |
| 3 | `series(x*abs(x), x, 0, 3)` | `x*abs(x)` | `x^2 + O(x^3)`; `(add (pow (sym x) (rat 2 1)) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 4 | `series(abs(sin(x)), x, 0, 3)` | `abs(sin(x))` | `x + O(x^3)`; `(add (sym x) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 5 | `series(abs(x - 1), x, 0, 3)` | `abs(x - 1)` | `1 - x + O(x^3)`; `(add (rat 1 1) (mul (rat -1 1) (sym x)) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 6 | `series(abs(x), x, 1, 3)` | `abs(x)` | `x + O((x - 1)^3)`; `(add (sym x) (order (sym x) (rat 1 1) (rat 3 1)))` |
| 7 | `series(abs(x^2 - 1), x, 0, 3)` | `abs(x^2 - 1)` | `1 - x^2 + O(x^3)`; `(add (rat 1 1) (mul (rat -1 1) (pow (sym x) (rat 2 1))) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 8 | `series(sin(x), x, 0, 5)` (smooth control) | `x - 1/6*x^3 + O(x^5)`; canonical `(fn O ...)`, `order` absent | `x - 1/6*x^3 + O(x^5)`; `(add (sym x) (mul (rat -1 6) (pow (sym x) (rat 3 1))) (order (sym x) (rat 0 1) (rat 5 1)))` |

The neighbouring shapes the same rule covers:

| Script | Before | After = oracle |
|---|---|---|
| `series(abs(x)/x, x, 0, 3)` | `abs(x)/x` | `1 + O(x^3)` |
| `series(abs(x)^2, x, 0, 3)` | `abs(x)^2` | `x^2 + O(x^3)` |
| `series(exp(abs(x)), x, 0, 3)` | `exp(abs(x))` | `1 + x + 1/2*x^2 + O(x^3)` |
| `series(abs(a*x), x, 0, 2)` | `abs(a*x)` | `a*x*sign(a) + O(x^2)`; `(add (mul (sym a) (sym x) (fn sign (sym a))) (order (sym x) (rat 0 1) (rat 2 1)))` |

The two limit rows this family also pinned:

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `limit(abs(x), x, 0)` | `Unevaluated` (`coefficient does not evaluate at the point`) | `Value`, 0, Exact |
| `limit(sqrt(x), x, 0)` | `Unevaluated` (`coefficient does not evaluate at the point`) | `Value`, 0, Exact |

`limit((1 - cos(x))/x^2, x, 0) = 1/2` and `limit(abs(x)/x, x, 0) = Unevaluated` (with
`limit.unevaluated`) are unchanged and are re-asserted by the same test.

### What changed

* `rust/lovelace-symbolics/src/series_kink.rs` (new): the kink rewrite (`resolve_kinks`: an
  `abs(g)` becomes `g`, `-g`, `0` or `sign(c)*g` from the sign of `g`'s leading coefficient), a
  polynomial-coefficient engine that carries SYMBOLIC coefficients (`|a*x|` is `a*x*sign(a)`), the
  rational engine as the smooth-shape fallback, and `publish_series`, which emits the reference's
  `O(...)` node and publishes the source function unchanged at a branch point (`series(sqrt(x),
  x, 0, 2) = sqrt(x)`, unchanged).
* `rust/lovelace-symbolics/src/lib.rs`: the `order` node is rendered `O(x^3)` / `O((x - 1)^3)`
  and canonicalised `(order var point degree)`; it sorts LAST in the canonical term order; a sum
  that carries an `O(...)` term renders in ascending powers around the expansion point
  (`Printing.OrderTermsForDisplay`); the product algebra reads a `sqrt(g)` as the half power
  `g^(1/2)` (`as_power`), so `sqrt(3)*sqrt(3)` and `(i*sqrt(3))^2` fold as the reference's Power
  constructor folds them.
* `rust/lovelace-lang/src/builtins.rs`: the published `series` builtin delegates to
  `series_kink::publish_series`; `limit_result` takes the reference's FIRST step (direct
  substitution at the point) for an exact value, so `limit(abs(x), x, 0)` and `limit(sqrt(x), x, 0)`
  are the value at 0 rather than a series coefficient. `0/0` still refuses (the numeric evaluator
  refuses the division) and the indeterminate shapes still reach the series path.

## 2. Kernel hardening (11 of 22 rows)

The family is a residue of API-level assertions; each row below is the C# assertion stated on the
language or kernel surface. "Before" is the measured pre-round behaviour on the same script/kernel
call.

| Row | Before (Rust) | After = oracle |
|---|---|---|
| `CubicRoots_SatisfyPolynomial_Numerically#0..#2` | roots already exact Cardano; no Rust residual check existed | every root's `evalf(abs(subs(poly, x, root)), 60)` is below 1e-40 |
| `RationalPower_HugeExponent_IsExact#0` | — (same kernel call) | `2^65536` is the rational `2^65536`; `2^-65536` is its reciprocal |
| `MathIR_HugeExponent_LowersAndEvaluates#0` | — | `evalir(compile(x^65536, [x]), [2], 40)` is the exact 19,729-digit integer |
| `NumericToSymbolicToNumeric_500Digits_RoundTrips#0` | — | `setprecision(600)` + 500-digit decimal publishes all 500 digits |
| `RealConstantAdd_Preserves500Digits#0` | — | `0.111…1 + 0.222…2` (500 digits) is `0.333…3` (500 threes) |
| `RealConstantMultiply_PreservesPrecision#0` | — | `3 * 0.777…7` (500 digits) is `2.333…31` — the carry survives |
| `RegisteredFunctionArity_IsEnforced#0` | refusal pair already matched | `sin(x, y)` → `InvalidArgument`/`sin(): expected 1 argument; got 2.`; `log()` → `… got 0.` |
| `Simplify_ZeroOverX_CancelsWithCondition#0` | `simplify_full(0/x)` unchanged, no step | `TransformResult(expression: 0, changed: True, conditions: [x != 0], steps: [rat.cancel-zero-over-x / Conditional])`; `simplify(0/x)` still `0/x` |
| `Solve_QuarticNoRealRoots_DefaultDomain_Unevaluated_RealDomain_Empty#0` | `status: Partial`, `unrepresented_count: 0` | `status: Unevaluated`, `unrepresented_count: 4`, reason `complex algebraic roots not supported (RootOf is real-only in v1).`; the `real` domain is `NoSolutions` |

Eight of the eleven are test-only mappings (the production already matched and the row had never
been curated). Three are production repairs: the zero-over-pole rule in `simplify_full`, the
quartic/no-real-root classification in the solver result construction, and the supporting
half-power reading of `sqrt` in the product algebra.

## 3. Ids expected to move

Rust tests live in `rust/lovelace-lang/src/round38_series_kink_hardening.rs` (module
`lovelace-lang::round38_series_kink_hardening`).

| Census ids | Rust test |
|---|---|
| `SeriesKinkDeterminismTests/KinkSeries_PublishesTheSymPyExpansion_ByteIdenticallyOnEveryRun#0..#7` | `kink_series_publishes_the_sympy_expansion_byte_identically_on_every_run` |
| `SeriesKinkDeterminismTests/KinkSeries_PublishesNoInternalSymbolAndNoUnevaluatedDerivative#0..#7` | `kink_series_publishes_no_internal_symbol_and_no_unevaluated_derivative` |
| `SeriesKinkDeterminismTests/KinkSeries_TakesTheSymPyValueAtTheSamplePoint#0..#6` (`#7` was already ported) | `kink_series_takes_the_sympy_value_at_the_sample_point` |
| `SeriesKinkDeterminismTests/KinkSeries_NeighbouringShapes_FollowTheSameRule#0..#3` | `kink_series_neighbouring_shapes_follow_the_same_rule` |
| `SeriesKinkDeterminismTests/KinkSeries_TruncationGap_IsTheDroppedTerm_NotASilentApproximation#0` | `kink_series_truncation_gap_is_the_dropped_term_not_a_silent_approximation` |
| `SeriesKinkDeterminismTests/LimitEngine_TwoSidedKinkAnswers_AreUnchanged#0,#2` (`#1,#3` were already ported) | `limit_engine_two_sided_kink_answers_are_unchanged` |
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Numerically#0..#2` | `cubic_roots_satisfy_polynomial_numerically` |
| `KernelHardeningTests/RationalPower_HugeExponent_IsExact#0` | `rational_power_huge_exponent_is_exact` |
| `KernelHardeningTests/MathIR_HugeExponent_LowersAndEvaluates#0` | `mathir_huge_exponent_lowers_and_evaluates` |
| `KernelHardeningTests/NumericToSymbolicToNumeric_500Digits_RoundTrips#0` | `numeric_to_symbolic_to_numeric_500_digits_round_trips` |
| `KernelHardeningTests/RealConstantAdd_Preserves500Digits#0` | `real_constant_add_preserves_500_digits` |
| `KernelHardeningTests/RealConstantMultiply_PreservesPrecision#0` | `real_constant_multiply_preserves_precision` |
| `KernelHardeningTests/RegisteredFunctionArity_IsEnforced#0` | `registered_function_arity_is_enforced` |
| `KernelHardeningTests/Simplify_ZeroOverX_CancelsWithCondition#0` | `simplify_zero_over_x_cancels_with_condition` |
| `KernelHardeningTests/Solve_QuarticNoRealRoots_DefaultDomain_Unevaluated_RealDomain_Empty#0` | `solve_quartic_no_real_roots_default_domain_unevaluated_real_domain_empty` |

Two translations are explicit rather than literal. `RealConstantAdd`/`RealConstantMultiply` are C#
`RealConstants` assertions; the Rust surface for "the same kernel call" is the language's 500-digit
decimal arithmetic under `setprecision(600)`, and the assertion is the exact digit string (the C#
comparison is the same digit identity through a different carrier). `RegisteredFunctionArity` claims
the refusal pair; the row's supplementary half (variadic scalar `min`/`max`, unknown names open for
extensibility) has no Rust surface — `min`/`max` are the array reductions (value, axis) and an
unknown name is an unknown-function refusal — and is not claimed.

## 4. Re-probed and refused (measured)

| Candidate | Rows | Measured divergence |
|---|---:|---|
| `CubicRoots_SatisfyPolynomial_Symbolically#0,#1` | 2 | `expand(subs(x^3 - 1, x, root))` publishes `-3/8*(i*sqrt(3))^2 + 1/8*(i*sqrt(3))^3 + …` where the reference expands to `0`: the Add coefficient scaling in the kernel rebuilds a term as a nested product, so the product algebra's combine (and the `(i*sqrt(3))^2 → -3` fold) is skipped on that path. One root (the rational `1`) does collapse. |
| `Diff_Abs_CarriesNonzeroCondition#0` | 1 | the reference answers `piecewise(sign(x) if x != 0, diff(abs(x), x))` (canonical `(pw …)` / `(der …)`); this kernel has no piecewise/derivative nodes, so `diff(abs(x), x)` answers `0`. |
| `Diff_Floor_IsUnevaluatedDerivative_NotZero#0` | 1 | `floor` is not a function on either runner (`Unknown function 'floor'.`); the C# row asserts a `DerivativeExpr` that the language cannot construct. |
| `Diff_Piecewise_DifferentiatesBranches#0` | 1 | `piecewise` construction and differentiation have no Rust surface. |
| `FreeOf_DoesNotTreatPiecewiseAsConstant#0` | 1 | `Calculus.FreeOf` over a piecewise expression has no Rust counterpart (no piecewise model). |
| `HugeExponent_CompactForm_RoundTrips#0` | 1 | the C# row is the `RealLiteral(new Int(15), 1_000_000_000)` compact form `15e1000000000`; the language lexer refuses that literal on BOTH runners (`ParseError … found 'e1000000000'`), and the Rust real tier has no such carrier. |
| `MathIR_Deserialize_RejectsOldVersion#0` | 1 | both refuse `#!mathir 1`, but the oracle's message is `Unsupported MathIR format version '1\nparam x\n' (expected 2).` and Rust's is `evalir could not read that IR text.` (different code too); the C# assertion is a `FormatException` type. |
| `RootOf_CloseRoots_AllFoundAndIndexedStably#0` | 1 | `rootof` is a printable node, but there is no `Roots.N(RootOf(p, i), 30)` numeric evaluation surface. |
| `RootOf_NoRealRoots_FailsExplicitly#0` | 1 | same missing numeric RootOf surface (`InvalidOperationException` has no Rust counterpart). |
| `YunSquareFree_MultiplicitiesAreCorrect#0` | 1 | `Polynomial.SquareFreeUnivariate` has no Rust API. |

## 5. Verification

Run on the settled tree after the last source edit; every line is the observed output.

| Gate | Command | Observed result |
|---|---|---|
| Suite | `cargo test --workspace` | 4,720 passed, 0 failed (15 are this round's new tests; the tree also carries a concurrent lane's new tests) |
| New tests | `cargo test -p lovelace-lang round38_series_kink` | `15 passed; 0 failed` |
| Corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated: 162/200 match, 38 mismatch — all 38 ids are already in the frozen `parity/diff/regressions` corpus (38/38), and 0 of their scripts exercise series/abs/sqrt/simplify/quartic; replay: 853/1752 match, 899 pre-existing drifts. None of the mismatch first-difference paths is in this round's families. |
| Probe parity | the 21-row probe above (8 family + 4 neighbouring + 4 limit + 5 kernel) | oracle and Rust agree on `ok`, `display`, `pretty` and `canonical` for 21/21 rows |

The manifest/coverage counts are unchanged in this lane because `parity/` is out of scope;
`parity/gen-rust-tests.ps1` and `node parity/gen-manifest.mjs` have not been run from this lane. The
integration lane must inventory the 15 new tests and curate the ids in section 3 (30 series + 11
kernel rows).
