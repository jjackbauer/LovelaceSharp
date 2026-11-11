# Round 39 — the symbolics residue behind the deferred rows

Status: complete for the rows below. This lane closed the 11 immediate residues named in
`docs/goal-parity-contract/round-38/series-kink-hardening.md` section 4, the indeterminate
exponential limit family, the integration table, and the quartic-with-real-roots solver shape.
Every production change is pinned by a Rust test in this lane; the measured before/after probes
and the census ids each fix should unblock are below.

## 0. Population and result

| Measure | Rows |
|---|---:|
| Immediate residues named by round 38 section 4 | 11 |
| `IndeterminateExponentialLimitTests` rows the oracle evaluates | 8 |
| Integration rows (`Integrate_Table_AllVerify`, `DebugIntegrate`) | 2 |
| Quartic-with-real-roots solver rows | 3 |
| **Rows this lane expects to unblock** | **24** |

Four of the eleven round-38 residues had no Rust surface at all (piecewise, derivative, floor,
RootOf numerics, square-free polynomials); they are now kernel surfaces with the reference
canonical and display text.

## 1. The probes

Both runners on the same argv: `rust/target/release/lovelace-run.exe --eval <script>
--omit-functions` and `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval
<script> --omit-functions`. "Before" is this lane first measurement on the pre-repair tree,
"after" is the same probe on the settled tree; "oracle" is the reference on the same argv.

### 1a. `expand` at the Cardano roots — the nested-Mul fold (2 rows)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `x = symbol("x"); s = solve(x^3 - 1 == 0, x); expand(subs(x^3 - 1, x, s.solutions[0].value))` | `-3/8*(i*sqrt(3))^2 + 1/8*(i*sqrt(3))^3 + 3/8*i*sqrt(3) - 9/8` | `0`; `(rat 0 1)` |
| the same with `s.solutions[1].value` | `-1/8*(i*sqrt(3))^3 - 3/8*(i*sqrt(3))^2 - 3/8*i*sqrt(3) - 9/8` | `0`; `(rat 0 1)` |

The Add coefficient scaling rebuilt a collected term as `Mul([coef, Mul([...])])`. The inner
product then read as an ATOMIC base, so the product algebra never combined `i*i` or
`sqrt(3)*sqrt(3)` and `(i*sqrt(3))^2` never folded to -3. `scale` now inserts the coefficient
INTO the product factor list, which is the flat shape the reference Multiply constructor
builds.

### 1b. The diff / discontinuity family (4 rows)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `x = symbol("x"); diff(abs(x), x)` | `0`; `(rat 0 1)` | `piecewise(sign(x) if x != 0, diff(abs(x), x))`; `(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))` |
| kernel: `diff(floor(x), x)` | `0` (floor is not a node at all) | `diff(floor(x), x)`; `(der (x) (fn floor (sym x)))` |
| kernel: `diff(piecewise([(x > 0, x^2)], -x^2), x)` | no piecewise node | branches `2*x` / otherwise `-2*x`, guard carried |
| kernel: `free_of(piecewise([(x > 0, x)], y), x)` | no piecewise node | `false` (x occurs in the branch) |

New kernel surfaces: `Expr::piecewise`, `Expr::derivative`, their accessors, `Expr::free_of`,
canonical `(pw ...)` / `(der (x) ...)` and the display `piecewise(VALUE if GUARD, OTHERWISE)` /
`diff(BODY, VAR)`. `diff` states abs as a piecewise with the `!=` condition, floor as an
unevaluated derivative (never 0), and raises the order when a derivative is differentiated.

### 1c. The compact real literal (1 row)

| Probe | Before (Rust) | After (Rust) = C# row |
|---|---|---|
| `RealLiteral::new(Int(15), 1_000_000_000).to_string()` | no carrier | `15e1000000000` |
| `RealLiteral::parse("15e1000000000")` | — | the same `(15, 1000000000)` pair |
| `to_rational()` on it | — | `None` (a billion-digit power of ten is refused, never truncated) |

`rust/lovelace-symbolics/src/real_literal.rs` is the compact carrier: identity is the
(significand, exponent) pair, so a value with a billion digits round-trips without materializing
them. The language lexer refuses `15e1000000000` on BOTH runners, so this is kernel-level, which
is what the C# row asserts.

### 1d. MathIR old version (1 row)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `evalir("#!mathir 1\nparam x\n", [1], 10)` | `InvalidArgument`, `evalir could not read that IR text.` | `InvalidInput` / `ParseError`, `Unsupported MathIR format version 1\nparam x\n (expected 2).` (quoted verbatim) |
| `evalir("#!mathir 3", [1], 10)` | the same generic refusal | the same message quoting `3` |

The version gate reads everything after the `#!mathir ` prefix as the version text and quotes it
verbatim, which is the reference own message shape.

### 1e. Yun square-free and the numeric RootOf (3 rows)

| Probe | Before (Rust) | After (Rust) = C# row |
|---|---|---|
| square-free of `x^3 - x^2` | no API | `[([-1, 1], 1), ([0, 1], 2)]` — `(x - 1)` once, `x` twice |
| `real_root_count(x*(x - 1/1000)*(x - 1))` | `1` (wrong: the Sturm remainder never reduced an equal-degree step) | `3` |
| `nth_real_root(p, 0..2)` at 40 digits | no API | `0`, `0.001`, `1` within 1e-19 |
| `evalf_rootof(rootof(p, i), 30)` | no API | the same roots through the NODE the solver publishes |
| `nth_real_root(x^2 + 1, 0)` | no API | `Err(NoRealRoots)` |

`poly_remainder` is also repaired: it now cancels the leading term while the remainder degree
is AT LEAST the divisor degree (the old `>` left an equal-degree remainder unchanged, so the
Euclid loop in `poly_gcd` swapped two same-degree polynomials forever and the Sturm counts were
wrong). The numeric evaluation is exact INTEGER bisection on a scaled interval — a rational
bisection multiplies denominators at every step and hangs, which is measurable and is why the
first implementation was rejected.

### 1f. The indeterminate exponential limits (8 rows)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `limit((1 + 1/x)^x, x, inf)` | `Unevaluated` (`coefficient does not evaluate at the point`) | `Value`, `e`, `AlgebraicExact` |
| `limit((1 + x)^(1/x), x, 0)` | `Unevaluated` | `Value`, `e`, `AlgebraicExact` |
| `limit((1 + 2/x)^x, x, inf)` | `Unevaluated` | `Value`, `e^2` |
| `limit((1 + 1/x)^(3*x), x, inf)` | `Unevaluated` | `Value`, `e^3` |
| `limit((1 + 3/x)^(x/2), x, inf)` | `Unevaluated` | `Value`, `e^(3/2)` |
| `limit((1 + x)^(a/x), x, 0)` | `Unevaluated` | `Value`, `e^a` |
| `limit((1 + 1/x)^x, x, -inf)` | `Value`, `1` (-inf was read as a finite point) | `Value`, `e` |
| `limit((2 + x)^(1/x), x, 0)` | `Unevaluated`, wrong reason | `Unevaluated`, the reference reason (an exponent has no value at the point and the base does not substitute to 1) |
| `limit((1 + 1/x)^(x^2), x, inf)` | `Unevaluated`, wrong reason | `Unevaluated`, the reference reason (the cofactor has no limit at the point) |
| `limit((2^(1/x))^x, x, 0)` | `Value`, `1` (a wrong answer) | `Unevaluated` |

What changed in `limit_result`: the limit is taken at the ACTUAL point (substituting the point,
then shifting `x -> point + x` so the series expands around it), the `(1 + u)^v` shape is
answered as `exp(lim v*u)` with the cofactor cancelled as a ratio, a power whose base has no
limit at the point is refused instead of being decided by the series, a substitution that
leaves an undefined subterm is not taken as a value at the point, and `-inf` is recognized as
the point at infinity.

### 1g. The integration table (2 rows)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `integrate(3*x^2 + 2*x + 1, x)` | `x^3 + x` (the `2*x` term integrated to 0) | `x^3 + x^2 + x` |
| `integrate_full(3*x^2 + 2*x + 1, x)` | `expression: x^3 + x`, `verified: False` | `expression: x^3 + x^2 + x`, `verified: True` |
| `integrate(sin(x), x)` | unevaluated | `-cos(x)` |
| `integrate(exp(3*x), x)` | unevaluated | `1/3*exp(3*x)` |
| `integrate(x*exp(x), x)` | unevaluated | `exp(x)*(x - 1)` |
| `integrate(exp(-(x^2)), x)` | unevaluated | unevaluated (a non-linear argument stays out of the table) |

The `x` row of the table returned `x^2*0` — every term linear in the variable vanished. The
table also gains the linear-argument rule for sin/cos/exp (scaled by `1/a`) and integration by
parts for `x*exp(a*x + b)`.

### 1h. The quartic with real roots (3 rows)

| Script | Before (Rust) | After (Rust) = oracle |
|---|---|---|
| `solve(x^4 - 5*x^2 == 0, x)` | `Partial`, three `rootof(x^4 - 5*x^2, i)`, `unrepresented_count: 1` | `Solved`, `Complete`, `-1/2*sqrt(20)`, `1/2*sqrt(20)`, `0`, `unrepresented_count: 0` |

The rational-root theorem cannot see a zero constant term (its `p/q` enumeration runs over the
divisors of `a0 = 0`), so the zero root was never extracted and the degree-4 RootOf path took
the polynomial. `find_rational_root` now answers 0 directly when the constant term is zero, and
the remaining quadratic reaches the exact radical branch. The solution list is also ordered by
CANONICAL print (the reference order: the radicals, then the exact zero).

## 2. What changed

| File | Change |
|---|---|
| `rust/lovelace-symbolics/src/lib.rs` | `scale` flattens a coefficient into a product; `Expr::piecewise` / `Expr::derivative` / accessors / `free_of`; canonical `(pw ...)` / `(der ...)`; display `piecewise(VALUE if GUARD, OTHERWISE)` / `diff(BODY, VAR)`; `diff` rules for abs/floor/piecewise/derivative; `poly_remainder` equal-degree step; Yun `square_free_univariate`; `RootOfError`, `nth_real_root`, `evalf_rootof` with exact integer bisection; zero-root extraction in `find_rational_root`; canonical solution ordering |
| `rust/lovelace-symbolics/src/real_literal.rs` (new) | the compact `significand e exponent` carrier |
| `rust/lovelace-lang/src/builtins.rs` | `limit_result` takes the point and shifts the series to it; `limit_at`, `exponential_family_limit`, `cancel_var_reciprocal`, `has_undefined_subterm`, `substitution_is_defined`, `vanishes_at`, `is_negated_infinity`; `-inf` point; the exponential family; the base-has-no-limit refusal; `integrate_basic` x-term fix, linear-argument rule, by-parts rule; `linear_parts`; `ir_version_error` |

## 3. Tests

| Module | Tests | Rows |
|---|---:|---:|
| `lovelace-symbolics::round39_residue` | 10 | 11 residues + the numeric RootOf surface |
| `lovelace-lang::round39_residue` | 6 | 13 rows (limits, integration, MathIR, solver) |

## 4. Ids expected to move

| Census ids | Rust test |
|---|---|
| `KernelHardeningTests/CubicRoots_SatisfyPolynomial_Symbolically#0,#1` | `lovelace-symbolics::round39_residue::cubic_roots_satisfy_polynomial_symbolically` |
| `KernelHardeningTests/Diff_Abs_CarriesNonzeroCondition#0` | `..diff_abs_carries_nonzero_condition` |
| `KernelHardeningTests/Diff_Floor_IsUnevaluatedDerivative_NotZero#0` | `..diff_floor_is_unevaluated_derivative_not_zero` |
| `KernelHardeningTests/Diff_Piecewise_DifferentiatesBranches#0` | `..diff_piecewise_differentiates_branches` |
| `KernelHardeningTests/FreeOf_DoesNotTreatPiecewiseAsConstant#0` | `..free_of_does_not_treat_piecewise_as_constant` |
| `KernelHardeningTests/HugeExponent_CompactForm_RoundTrips#0` | `..huge_exponent_compact_form_round_trips` |
| `KernelHardeningTests/YunSquareFree_MultiplicitiesAreCorrect#0` | `..yun_square_free_multiplicities_are_correct` |
| `KernelHardeningTests/RootOf_CloseRoots_AllFoundAndIndexedStably#0` | `..rootof_close_roots_all_found_and_indexed_stably` |
| `KernelHardeningTests/RootOf_NoRealRoots_FailsExplicitly#0` | `..rootof_no_real_roots_fails_explicitly` |
| `IndeterminateExponentialLimitTests/Probe_LimitOfOnePlusOneOverXToTheX_IsE#0`, `Probe_LimitOfOnePlusXToTheOneOverX_IsE#0`, `Probe_LimitOfOnePlusTwoOverXToTheX_IsESquared#0`, `Family_AnswersEveryExactExponent#0`, `Probes_CrossTheWireAsExactClosedForms#0` | `lovelace-lang::round39_residue::indeterminate_exponential_limits_answer_the_exact_closed_form` |
| `IndeterminateExponentialLimitTests/IndeterminateShapes_AreRefused_NeverAnsweredWithTheValueAtThePoint#0` | `..indeterminate_exponential_shapes_stay_unevaluated_with_a_reason` |
| `IndeterminateExponentialLimitTests/DeterminedLimits_KeepExactlyTheirPreviousAnswers#0`, `SubstitutionShapedLimits_StillAnswerImmediately#0`, `UndeterminedLimit_ClaimsNoExactness_AndDeterminedOnesKeepTheirs#0` | `..determined_limits_keep_their_answers` |
| `AcceptanceTests/Integrate_Table_AllVerify#0`, `DebugIntegrate/RationalDenominator_IntegratesWithSelfVerification#0` | `..integration_table_rows_are_exact_and_verify` |
| `KernelHardeningTests/MathIR_Deserialize_RejectsOldVersion#0` | `..mathir_old_version_is_rejected_with_the_reference_message` |
| `DxSemanticClosureTests/Solve_QuarticWithRealRoots_DefaultDomain_KeepsRootOfRoots#0` | `..solve_quartic_with_real_roots_keeps_the_exact_radicals` |

Two rows carry a translation rather than a literal claim: `MathIR_Deserialize_RejectsOldVersion#0`
is asserted as the same refusal with the same code and message rather than as a CLR exception
type, and `AcceptanceTests/Integrate_Table_AllVerify#0` is asserted for the seven table entries
the C# row lists (the `method` label the record carries, `table` vs the reference `linearity`, is
not part of the row and still differs).

## 5. Gate outputs

| Gate | Command | Observed result |
|---|---|---|
| Suite | `cargo test --workspace` | **4,808 passed, 0 failed, exit 0**, no hang (the symbolics crate finishes its 110 tests in 0.97 s) |
| New tests | `cargo test -p lovelace-symbolics round39_residue`, `cargo test -p lovelace-lang round39_residue` | `10 passed` and `6 passed`, 0 failed |
| Corpus | `rust/target/release/lovelace-corpus.exe` | `corpus: 214 passed, 0 failed, 214 total` |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `PASS: oracle 20/20, rust 20/20` |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`; `PASS: every RUN-001..RUN-014 row behaves like the oracle` |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated **166/200 match, 34 mismatch** (the round-38 measurement was 162/200 and 38); replay **941/1752 match, 811 pre-existing drifts** (was 853/1752 and 899); `newRegressions: []` |

## 6. Refused / remaining

Measured, not approximated:

| Item | Measurement |
|---|---|
| `solve(x^4 - 5*x^2)` multiplicity of the zero root | the record publishes `multiplicity: 1` where the reference says 2. The outcome carrier for expression roots has no multiplicity channel; the C# row (`Solve_QuarticWithRealRoots...`) asserts the COUNT (3) and the kind (`Exact`), both of which now match. |
| `integrate_full` method label | `table` vs the reference `linearity`; no row asserts it. |
| `limit` at a nonzero point for indeterminate shapes | the series is now taken around the point, but only the substitution and the exponential family were exercised by the rows in this lane. |
