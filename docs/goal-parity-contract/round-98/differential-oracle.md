# Round 98 — the six DifferentialOracle census rows

**Status: 6/6 closed as Rust tests. Every corpus case agrees with SymPy 1.14.0.**

The C# differential oracle (Lovelace.Symbolics.Tests/DifferentialOracleTests.cs +
OracleCorpus.cs + SympyOracle.cs) was read in full. Its six [RequiresSympyFact] rows compare the
kernel against a live SymPy session. The session's ground truth was produced on this host with
Python 3.12.10 and SymPy 1.14.0 (the exact version the C# tests cite), then embedded in six Rust
tests as literal expected values. Python is not a runtime dependency of the tests.

## 1. Corpus and comparison rule

| C# test | Corpus | Rule applied (same in the Rust test) |
|---|---|---|
| SympyOracle_Derivatives_Agree | 6 expressions × 3 rational points | canonical-print, reparse, evaluate both parts at each point; RealTolerance = 1e-13 |
| SympyOracle_Solve_Agrees | 7 equations | distinct canonical-root count vs sympy.solve; every kernel root substituted back and expanded must be exactly 0; the declared non-answer must be Unevaluated, not complete |
| SympyOracle_Roots_Agree | 8 polynomials | distinct real values vs sympy.real_roots; TightTolerance = 1e-20; closed forms substituted back exactly |
| SympyOracle_Factorization_Agrees | 9 polynomials | sorted ddd:ddd degree:multiplicity profile vs sympy.factor_list; factorization must expand back to the exact input |
| SympyOracle_Limits_Agree | 24 limits | finite values at TightTolerance = 1e-20; oo/-oo matched by status; a two-sided DoesNotExist carries both one-sided values |
| SympyOracle_MatrixOperations_Agree | 13 matrix cases | det/trace/inverse entries at the same points, both parts at 1e-13; rank compared exactly |

The generator is `rust/target/tmp/differential_oracle.py` (never committed); its output is
`rust/target/tmp/differential_oracle.json`. The C# runner itself could not execute in this
session: `dotnet test` stops at testhost startup with
`System.ComponentModel.Win32Exception (5): Access is denied` in `ProcessManager.OpenProcess`
(environment-level parent-process access, not a test failure). The SymPy programs in
`SympyOracle.cs` were reproduced verbatim, so the ground truth is the one those rows would obtain.

## 2. The six census ids

| # | Census id | Rust test |
|---:|---|---|
| 1 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_derivatives_agree` |
| 2 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Solve_Agrees#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_solve_agrees` |
| 3 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Roots_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_roots_agree` |
| 4 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_factorization_agrees` |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Limits_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_limits_agree` |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_matrixoperations_agree` |

## 3. Divergences found and fixed

The first Rust run exposed four real kernel divergences and two missing surfaces. None of the
expected values was changed to fit the kernel.

| # | Case | Before (Rust) | SymPy 1.14.0 | Fix |
|---:|---|---|---|---|
| 1 | `diff(tan(x^3), x)` | 0 at every point | `3*x^2*(tan(x^3)^2 + 1)` | `Calculus.Diff` now has the reference's tan template `cos(u)^-2` |
| 2 | `limit(tan(x)/x, x, 0)` | `Unevaluated` | 1 | the series engine now carries the tan expansion through x^7 |
| 3 | `limit(x^3 - 2*x, x, +-inf)` | `DoesNotExist` (left -inf, right +inf) for both | +oo at +oo, -oo at -oo | exact rational-degree infinity rule, with the direction's sign |
| 4 | `factor(x^3 + 1)`, `factor(x^4 - 1)` | returned the input whole (profile 003:001 / 004:001) | `(x + 1)*(x^2 - x + 1)`, `(x - 1)*(x + 1)*(x^2 + 1)` | factor over Q via content + Yun + rational roots + irreducible residual (the reference's FactorPoly), not the solver's root set |
| 5 | matrix trace | no `SymbolicMatrix::trace` API | trace of the corpus matrices | added `trace()` over the diagonal |
| 6 | library `Solvers.Solve` over the reals | complex radicals returned as real solutions; a RootOf set reported `Partial` | `x^2 + 1` -> NoSolutions; real RootOf roots -> complete Solved | `solve_polynomial_set` filters non-real radicals under `SolveDomain::Real` and treats the represented RootOf roots as the complete real set |

The tan series refuses an order larger than 8 rather than publishing a silently truncated
expansion.

## 4. Corpus tables

Values are the SymPy session output (30 significant digits) and the Rust kernel's observed value.
"Agreement" is the C# rule, not a canonical-string equality: numeric at the stated tolerance, or
the exact cardinality/status/profile.

### 4.1 Derivatives (`SympyOracle_Derivatives_Agree#0`)

Tolerance 1e-13. Rust values below are the numeric evaluator at 40 places (30 shown).

| # | Expression | SymPy at 1/3 ; -1/2 ; 7/5 | Rust at 1/3 ; -1/2 ; 7/5 | Agreement |
|---:|---|---|---|---|
| 1 | `x**7` | 0.00960219478737997256515775034294 ; 0.109375 ; 52.706752 | 0.009602194787379972565157750342 ; 0.109375 ; 52.706752000000000000000000000000 | yes |
| 2 | `sin(x**2)` | 0.662555672359261280868626202047 ; -0.968912421710644784144595449494 ; -1.06246494140683265580838260325 | 0.662555672359261280868626202046 ; -0.968912421710644784144595449494 ; -1.062464941406832655808382603252 | yes |
| 3 | `exp(-x**2)` | -0.596559544542913183054293028847 ; 0.778800783071404868245170266978 ; -0.394403578578925989214320091069 | -0.596559544542913183054293028846 ; 0.778800783071404868245170266978 ; -0.394403578578925989214320091068 | yes |
| 4 | `x*log(x)` | -0.0986122886681096913952452369225 ; 0.306852819440054690582767878542 + 3.14159265358979323846264338328i ; 1.33647223662121293050459341022 | -0.098612288668109691395245236922 ; 0.306852819440054690582767878541 + 3.141592653589793238462643383279i ; 1.336472236621212930504593410216 | yes (both parts) |
| 5 | `sin(x)/x` | -0.109881432221157204395244744381 ; 0.162537030636066568860588575655 ; -0.381375372412307584997973127307 | -0.109881432221157204395244744380 ; 0.162537030636066568860588575654 ; -0.381375372412307584997973127306 | yes |
| 6 | `tan(x**3)` | 0.333790999179746492481090015847 ; 0.761841910010786112211782624635 ; 6.91702668304069031278752914719 | 0.333790999179746492481090015847 ; 0.761841910010786112211782624634 ; 6.917026683040690312787529147187 | yes |

### 4.2 Solve (`SympyOracle_Solve_Agrees#0`)

The rule is the distinct-root count plus exact substitution; the root list is SymPy's, and the
Rust roots are represented in the kernel's own (equivalent) forms.

| # | Equation | SymPy roots (distinct count) | Rust result | Agreement |
|---:|---|---|---|---|
| 1 | `x**2 - 4*x + 4` | `[2]` (1) | Solved, 1 distinct root 2, residual 0 | yes |
| 2 | `x**2 - 5*x + 6` | `[2, 3]` (2) | Solved, 2 distinct roots, residual 0 | yes |
| 3 | `x**3 + 1` | `[1/2 - sqrt(3)*I/2, 1/2 + sqrt(3)*I/2, -1]` (3) | Solved, 3 distinct roots (`1/2 +- i*sqrt(3)/2`, -1), residual 0 | yes |
| 4 | `x**4 - 5*x**2 + 4` | `[-1, -2, 1, 2]` (4) | Solved, 4 distinct roots, residual 0 | yes |
| 5 | `x**2 + 1` | `[I, -I]` (2) | Solved, 2 distinct roots i, -i, residual 0 | yes |
| 6 | `x**2 + 4` | `[-2*I, 2*I]` (2) | Solved, 2 distinct roots 2i, -2i, residual 0 | yes |
| 7 | `sqrt(x) + 2` | `[]` (0) | Unevaluated, 0 represented, complete=false | yes (declared non-answer; SymPy's set is empty) |

### 4.3 Roots (`SympyOracle_Roots_Agree#0`)

Tolerance 1e-20; values are distinct (`(x-1)^3` is one root on both sides).

| # | Polynomial | SymPy real_roots (distinct) | Rust values | Agreement |
|---:|---|---|---|---|
| 1 | `x**4 - 5*x**2 + 4` | -2 ; -1 ; 1 ; 2 | -2 ; -1 ; 1 ; 2 | yes |
| 2 | `x**2 - 2` | -1.41421356237309504880168872421 ; 1.41421356237309504880168872421 | -1.414213562373095048801688724209 ; 1.414213562373095048801688724209 | yes |
| 3 | `x**4 - 2` | -1.18920711500272106671749997056 ; 1.18920711500272106671749997056 | -1.18920711500272106671749997056047591 ; 1.18920711500272106671749997056047591 | yes |
| 4 | `x**5 - 3*x + 1` | -1.38879198440725418280005669419 ; 0.334734141943352687075098962473 ; 1.21464804269846180398582838932 | -1.38879198440725418280005669418980841 ; 0.33473414194335268707509896247328330 ; 1.21464804269846180398582838931533196 | yes |
| 5 | `x**3 - 2` | 1.25992104989487316476721060728 | 1.259921049894873164767210607278 | yes |
| 6 | `x**3 - 3*x**2 + 3*x - 1` | 1 | 1 | yes |
| 7 | `x**5 - x - 1` | 1.16730397826141868425604589985 | 1.16730397826141868425604589985484218 | yes |
| 8 | `x**2 + 1` | (empty) | NoSolutions, 0 | yes |

### 4.4 Factorizations (`SympyOracle_Factorization_Agrees#0`)

The compared value is `sympy.factor_list`'s sorted degree:multiplicity profile; the Rust
factorization must also expand back to the exact input (all do).

| # | Polynomial | SymPy profile / factored | Rust profile / factorization | Agreement |
|---:|---|---|---|---|
| 1 | `x**2 - 5*x + 6` | `001:001;001:001` / `(x - 3)*(x - 2)` | `001:001;001:001` / `(x - 3)*(x - 2)` | yes |
| 2 | `x**3 + 1` | `001:001;002:001` / `(x + 1)*(x**2 - x + 1)` | `001:001;002:001` / `(x + 1)*(x^2 - x + 1)` | yes |
| 3 | `x**4 - 5*x**2 + 4` | `001:001;001:001;001:001;001:001` / `(x - 2)*(x - 1)*(x + 1)*(x + 2)` | same profile / `(x - 2)*(x - 1)*(x + 1)*(x + 2)` | yes |
| 4 | `x**3 - 3*x**2 + 3*x - 1` | `001:003` / `(x - 1)**3` | `001:003` / `(x - 1)^3` | yes |
| 5 | `2*x**2 - 8` | `001:001;001:001` / `2*(x - 2)*(x + 2)` | `001:001;001:001` / `2*(x - 2)*(x + 2)` | yes |
| 6 | `x**4 - 1` | `001:001;001:001;002:001` / `(x - 1)*(x + 1)*(x**2 + 1)` | `001:001;001:001;002:001` / `(x - 1)*(x + 1)*(x^2 + 1)` | yes |
| 7 | `x**2 - 2` | `002:001` / `x**2 - 2` | `002:001` / `x^2 - 2` | yes |
| 8 | `6*x**3 - 6*x` | `001:001;001:001;001:001` / `6*x*(x - 1)*(x + 1)` | `001:001;001:001;001:001` / `6*x*(x - 1)*(x + 1)` | yes |
| 9 | `x**3 + x**2 - x - 1` | `001:001;001:002` / `(x - 1)*(x + 1)**2` | `001:001;001:002` / `(x + 1)^2*(x - 1)` | yes |

### 4.5 Limits (`SympyOracle_Limits_Agree#0`)

Tolerance 1e-20 for finite values; infinity/DoesNotExist are exact status comparisons.

| # | Expression | Point, direction | SymPy value | Rust value | Agreement |
|---:|---|---|---|---|---|
| 1 | `sin(x)/x` | 0, two | 1 | Value 1 | yes |
| 2 | `(1 - cos(x))/x**2` | 0, two | 1/2 | Value 1/2 | yes |
| 3 | `(x**3 - 8)/(x - 2)` | 2, two | 12 | Value 12 | yes |
| 4 | `(x**2 - 1)/(x - 1)` | 1, two | 2 | Value 2 | yes |
| 5 | `tan(x)/x` | 0, two | 1 | Value 1 | yes |
| 6 | `1/x**2` | 0, two | oo | PlusInfinity | yes |
| 7 | `1/(x - 1)**2` | 1, two | oo | PlusInfinity | yes |
| 8 | `1/x` | 0, right | oo | PlusInfinity | yes |
| 9 | `1/x` | 0, left | -oo | MinusInfinity | yes |
| 10 | `1/x` | 0, two | left -oo / right oo | DoesNotExist (left -inf, right inf) | yes |
| 11 | `1/(x - 1)` | 1, two | left -oo / right oo | DoesNotExist (left -inf, right inf) | yes |
| 12 | `(3*x**2 + 5*x)/(2*x**2 - x)` | +oo, two | 3/2 | Value 3/2 | yes |
| 13 | `(3*x**2 + 5*x)/(2*x**2 - x)` | -oo, two | 3/2 | Value 3/2 | yes |
| 14 | `x/(x**2 + 1)` | +oo, two | 0 | Value 0 | yes |
| 15 | `(2*x**3 - x)/(x**3 + 1)` | +oo, two | 2 | Value 2 | yes |
| 16 | `x**3 - 2*x` | +oo, two | oo | PlusInfinity | yes |
| 17 | `x**3 - 2*x` | -oo, two | -oo | MinusInfinity | yes |
| 18 | `(1 + 1/x)**x` | +oo, two | 2.71828182845904523536028747135 | Value e (same to 30 digits) | yes |
| 19 | `(1 + 1/x)**x` | -oo, two | 2.71828182845904523536028747135 | Value e | yes |
| 20 | `(1 + x)**(1/x)` | 0, two | 2.71828182845904523536028747135 | Value e | yes |
| 21 | `(1 + x)**(1/x)` | 0, left | 2.71828182845904523536028747135 | Value e | yes |
| 22 | `(1 + 2/x)**x` | +oo, two | 7.38905609893065022723042746058 | Value e^2 | yes |
| 23 | `(1 + 1/x)**(3*x)` | +oo, two | 20.0855369231876677409285296546 | Value e^3 | yes |
| 24 | `(1 + 3/x)**(x/2)` | +oo, two | 4.48168907033806482260205546012 | Value e^(3/2) | yes |

### 4.6 Matrix operations (`SympyOracle_MatrixOperations_Agree#0`)

Tolerance 1e-13. The two parameter points are `(x, y) = (1/3, 7/5)` and `(2, -3/4)`.

| # | Operation | Matrix | SymPy value | Rust value | Agreement |
|---:|---|---|---|---|---|
| 1 | det | `[[x, 1], [y, x]]` | -1.28888888888888888888888888889 ; 4.75 | -58/45 ; 19/4 | yes |
| 2 | det | `[[x, 1, 0], [0, x, 1], [1, 0, x]]` | 1.03703703703703703703703703704 ; 9 | 28/27 ; 9 | yes |
| 3 | det | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | 18 ; 18 | 18 ; 18 | yes |
| 4 | trace | `[[x, 1], [y, x]]` | 0.666666666666666666666666666667 ; 4 | 2/3 ; 4 | yes |
| 5 | trace | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | 9 ; 9 | 9 ; 9 | yes |
| 6 | rank | `[[1, 2], [2, 4]]` | 1 | 1 | yes |
| 7 | rank | `[[1, 2, 3], [4, 5, 6], [7, 8, 9]]` | 2 | 2 | yes |
| 8 | rank | `[[2, 0, 1], [1, 3, 2], [1, 1, 3]]` | 3 | 3 | yes |
| 9 | rank | `[[x, 1], [0, y]]` | 2 (generic) | 2 | yes |
| 10 | rank | `[[x, x], [x, x]]` | 1 (generic) | 1 | yes |
| 11 | inverse | `[[x, 1], [y, x]]` | -0.258620689655172413793103448276 ; 0.775862068965517241379310344828 ; 1.08620689655172413793103448276 ; -0.258620689655172413793103448276 (point 1), 0.421052631578947368421052631579 ; -0.210526315789473684210526315789 ; 0.157894736842105263157894736842 ; 0.421052631578947368421052631579 (point 2) | -15/58 ; 45/58 ; 63/58 ; -15/58 and 8/19 ; -4/19 ; 3/19 ; 8/19 | yes |
| 12 | inverse | `[[x, 2], [3, y]]` | -0.253012048192771084337349397590 ; 0.361445783132530120481927710843 ; 0.542168674698795180722891566265 ; -0.0602409638554216867469879518072 and 0.1 ; 0.266666666666666666666666666667 ; 0.4 ; -0.266666666666666666666666666667 | -21/83 ; 30/83 ; 45/83 ; -5/83 and 1/10 ; 4/15 ; 2/5 ; -4/15 | yes |
| 13 | inverse | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | 5/9 ; 1/18 ; -1/6 ; -1/9 ; 7/18 ; -1/6 ; -1/9 ; -1/9 ; 1/3 (both points) | 5/9 ; 1/18 ; -1/6 ; -1/9 ; 7/18 ; -1/6 ; -1/9 ; -1/9 ; 1/3 | yes |

## 5. Tests added

`rust/lovelace-lang/src/round98_differential_oracle.rs` — six tests, one per census row.
Each embeds the SymPy values as literals and applies the rule above; no test invokes Python. The
test names and ids are in section 2.

## 6. Gate outputs (observed)

```text
cargo test --workspace
  ... 1719 passed in lovelace-lang (baseline 1713 + the six new tests), 196 in lovelace-symbolics,
  every other package green
  totals: 5,664 passed / 1 ignored / 0 failed; workspace exit 0

cargo test -p lovelace-lang --lib round98_differential_oracle
  6 passed; 0 failed; 0 ignored

cargo run --release -p lovelace-corpus
  corpus: 214 passed, 0 failed, 214 total

node parity/tools/gate-fixtures.mjs
  oracle: 20/20
  rust: 20/20
  PASS: oracle 20/20, rust 20/20

node parity/tools/gate-cli.mjs
  CLI matrix: 16/16
  PASS: every RUN-001..RUN-014 row behaves like the oracle

node parity/diff/run.mjs --cases 200 --seed 1
  GENERATED 200 match 200 mismatch 0 error 0
  REPLAYED 1758 match 1758 mismatch 0 error 0
  harness exit: 0

cargo fmt -p lovelace-lang -p lovelace-symbolics --check   exit 0
cargo clippy -p lovelace-lang -p lovelace-symbolics --all-targets -- -D warnings   exit 0
```

The workspace count is 5,664 rather than the task's 5,622 baseline because a concurrent lane has
added 36 tests in `lovelace-array` since that baseline; this round's contribution is the six
tests above (lovelace-lang 1,713 -> 1,719). All packages are green.

## 7. Files touched

* `rust/lovelace-lang/src/round98_differential_oracle.rs` (new: the six tests)
* `rust/lovelace-lang/src/lib.rs` (module registration)
* `rust/lovelace-lang/src/builtins.rs` (rational-infinity limit, factor over Q)
* `rust/lovelace-symbolics/src/lib.rs` (tan derivative template, tan series)
* `rust/lovelace-symbolics/src/solution_set.rs` (real-domain filtering / complete RootOf sets)
* `rust/lovelace-symbolics/src/symbolic_matrix.rs` (trace)
* this report.

Not touched: any `.cs` file, fixture, `parity/` mapping or gate script, `rust/lovelace-desktop`, or other
documentation. The working tree also contains uncommitted desktop/journey/tool changes from a
concurrent lane; they were not edited by this round.
