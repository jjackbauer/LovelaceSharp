# A3 — Independent adversarial audit: symbolic kernel, rewrite paths, printer

**Auditor:** A3 (independent; did not write this product). **Date:** see file mtime. **Verdict class:** attack, not confirmation.

## 0. Subject, contract, method

* **Product under test:** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` (published Native AOT; envelope self-reports `protocolVersion 1`, `symbolicFormatVersion "#!lovelace-sym 1"`, `mathIrVersion 2`; the `revision` field varied 80–85 across runs, i.e. the binary is being republished by other agents while this audit ran — every probe below was run twice back-to-back and the two envelopes were compared).
* **Invocation (identical for every probe):** `.\out\aot\Lovelace.Run.exe --file <script>.ls --omit-functions`. The "command" column of each table is the *content of the script file*; a fresh file in `%TEMP%\a3audit` was written for each of the two runs of each probe.
* **Reproduction rule applied:** every probe ran twice from a clean temp file. Envelopes were parsed and compared field-by-field. All rows below were **stable across the two runs** (identical apart from `elapsed`/`revision`) except where a row says otherwise. `RUN2` text for findings is pasted in the FINDINGS section.
* **Contract attacked:** `docs/symbolics/dsh-protocol.md` (v1) plus the builtin metadata returned by `capabilities()`. Claims of the *script brief* (e.g. "a newline is NOT a separator") were checked separately and are noted but not scored against the product.
* **Independent ground truth:** SymPy 1.14.0 (`python3 -c "import sympy; print(sympy.__version__)"` → `1.14.0`) and mpmath at 250/1200 dps for digit-level checks. No expectation in this report was taken from what the binary printed.
* **Root verification method:** for every returned root the kernel's own `canonical` s-expression *and* the kernel's own `canonical` of the equation were translated to SymPy by an independent translator (`rat/sym/pow/add/mul/fn log|exp|sin|cos|tan|abs|sign/i/inf/rootof/integ/der/pw/ne`) and the residual `E.subs(x, root)` was evaluated at 40 dps. A root is a solution iff the residual is 0; anything with `|residual| > 1e-15` is an extraneous root.
* **Verdicts:** `HELD` = product is right; `FINDING` = it is wrong; `INCONCLUSIVE` = could not decide (see `COULD NOT DECIDE`). Severity: P0 wrong mathematical statement / wrong completeness / wrong exactness / internal invariant failure / death without envelope; P1 documented contract contradicted or a machine-visible field recoverable only by parsing display text; P2 inconsistency or rough edge with no wrong answer; P3 cosmetic.
* **Totals:** 375 probe inputs, presented as 363 table rows (one row aggregates 13 arithmetic identities, one restates the depth probes): **263 HELD / 82 FINDING / 9 evidence / 6 N-A / 2 discarded / 1 note**, which resolves to **21 distinct findings — 7 P0, 6 P1, 7 P2, 1 P3** — and 4 matters undecided (see `COULD NOT DECIDE`). Counts are restated in `14. Summary`.

### 0.1 SymPy ground truth pasted once, used by the findings

```
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$ python3 -c "import sympy; print(sympy.__version__)"
1.14.0

$ python3 <truth.py>
sympy 1.14.0

# extraneous roots
solve(Eq(sqrt(x)+2,0), x)                 = []
solveset(Eq(sqrt(x)+2,0), x, S.Reals)     = EmptySet
solve(Eq(sqrt(2*x+3),x), x)               = [3]
solveset(Eq(sqrt(2*x+3),x), x, S.Reals)   = {3}
(sqrt(2*x+3)-x).subs(x,-1)                = 2
(sqrt(x)+2).subs(x,4)                     = 4

# underdetermined systems
linsolve([x+y-3],[x,y])                   = {(3 - y, y)}
linsolve([x+y-1,2*x+2*y-2],[x,y])         = {(1 - y, y)}
linsolve([a+b+c+d-4,a-b,c-d,a+c-2],[a,b,c,d]) = {(2 - d, 2 - d, d, d)}
witness (a,b,c,d)=(1,1,1,1):
   a+b+c+d = 4  a-b = 0  c-d = 0  a+c = 2
witness (x,y)=(0,3) for x+y=3             = 3

# indeterminate powers
limit((1+1/x)**x, x, oo)                  = E = 2.7182818284590452354
limit((1+x)**(1/x), x, 0)                 = E = 2.7182818284590452354

# exact decimal product
Rational(1,17)*17                         = 1

# x/y/x canonical meaning
x/y/x == x/(x*y) ?                        = 0
```

## 1. Solve entry points — smoke (s-series, 16 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| s01 | solve_full, quadratic | `x = symbol("x"); solve_full(x^2 - 4 == 0, x)` | `status=SolveStatus:Solved, domain=complex, complete=true, completeness=Complete, solutions=[(rat -2 1),(rat 2 1)], represented_count=2` | HELD | — |
| s02 | solve short form | `x = symbol("x"); solve(x^2 - 4 == 0, x)` | `kind=Vector [(rat -2 1), (rat 2 1)]` | HELD | — |
| s03 | default domain is complex; ±i | `x = symbol("x"); solve_full(x^2 + 1 == 0, x)` | `Solved, domain=complex, Complete, [(i), (mul (rat -1 1) (i))], exactness=AlgebraicExact` | HELD | — |
| s04 | empty solution set over real | `x = symbol("x"); solve_full(x^2 + 1 == 0, x, real)` | `status=NoSolutions, complete=true, completeness=Complete, diagnostic code=solve.no-solutions` | HELD | — |
| s05 | domain constructor | `complex()` | `kind=Domain domain=complex` | HELD | — |
| s06 | inspect fields | `x = symbol("x"); inspect(x^2 + 1)` | `canonical=(add (rat 1 1) (pow (sym x) (rat 2 1))) pretty=x^2 + 1 node_count=5 exact=true` | HELD | — |
| s07 | sqrt of negative | `sqrt(-1)` | `kind=Symbolic canonical=(i) pretty=i` | HELD | — |
| s08 | division by zero | `1/0` | `RC=1 code=DivisionByZero category=DomainError recoverable=true` | HELD | — |
| s09 | 0^0 | `0^0` | `kind=Natural value=1` (SymPy: `0**0 = 1`) | HELD | — |
| s10 | log(0), symbolic path | `log(0)` | `kind=Symbolic canonical=(fn log (rat 0 1))` (unevaluated; contrast n09) | HELD | — |
| s11 | infinity constructor | `inf()` | `kind=Symbolic canonical=(inf)` | HELD | — |
| s12 | LaTeX renderer | `x = symbol("x"); latex(x^2 + 1)` | `kind=Text value=x^{2} + 1` | HELD | — |
| s13 | digits of evalf | `evalf(sqrt(2), 30)` | `kind=Real value=1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727` | HELD (all 100 digits correct vs mpmath; the `30` is ignored → n40) | — |
| s14 | substitution | `x = symbol("x"); subs(x^2, x, 3)` | `canonical=(rat 9 1)` | HELD | — |
| s15 | rational arithmetic | `(1/17)*17` | `kind=Real value=0.999…(1000 digits)…9984` | FINDING | P0 |
| s16 | declared enum type travels | `x = symbol("x"); r = solve_full(x^2 - 4 == 0, x); type(r.status)` | `kind=Text value=SolveStatus` | HELD | — |

## 2. Solve completeness, single equation (c-series, 24 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| c01 | quartic, 2 real + 2 complex roots | `solve_full(x^4 - x^2 - 1 == 0, x)` | `Partial, Partial, complete=false, solutions=[rootof(…,0), rootof(…,1)], represented_count=2, unrepresented_count=2, code=solve.unrepresented-roots` | HELD (SymPy: 2 real + 2 imaginary) | — |
| c02 | cubic with complex roots expressible in radicals | `solve_full(x^3 - 1 == 0, x)` | `Solved, Complete, 3 solutions: `1/2*(i*sqrt(3) - 1)`, `1/2*(-i*sqrt(3) - 1)`, `1` | HELD | — |
| c03 | same cubic restricted to real | `solve_full(x^3 - 1 == 0, x, real)` | `Solved/Complete, 1 solution (rat 1 1)` | HELD | — |
| c04 | cubic with three integer roots | `solve_full(x^3 - 6*x^2 + 11*x - 6 == 0, x)` | `Solved/Complete, [(rat 1 1),(rat 2 1),(rat 3 1)]` | HELD | — |
| c05 | irrational real roots | `solve_full(x^2 - 2 == 0, x, real)` | `Solved/Complete, [`-1/2*sqrt(8)`, `1/2*sqrt(8)`], AlgebraicExact` | HELD | — |
| c06 | radical equation, impossible over ℝ | `solve_full(sqrt(x) + 2 == 0, x, real)` | `Solved, complete=true, completeness=Complete, solutions=[4]` | **FINDING** | **P0** |
| c07 | principal-sqrt equation | `solve_full(sqrt(x) == -2, x)` | `Solved, complete=true, solutions=[4]` | **FINDING** | **P0** |
| c08 | radical equation, one valid root | `solve_full(sqrt(x + 1) == x - 1, x)` | `Unevaluated, completeness=Unknown, code=solve.unevaluated "No solver for this structure."` | HELD (honest gap; SymPy {3}) | — |
| c09 | reciprocal equals zero | `solve_full(1/x == 0, x)` | `NoSolutions/Complete, code=solve.no-solutions` | HELD | — |
| c10 | removable singularity | `solve_full((x^2 - 1)/(x - 1) == 0, x)` | `Solved/Complete, x=-1, conditions=[(ne (add (rat -1 1) (sym x)) (rat 0 1))]` | HELD | — |
| c11 | identity (singular) | `solve_full(x/x == 1, x)` | `Unevaluated/Unknown, code=solve.unevaluated` | HELD (gap) | — |
| c12 | tautology | `solve_full(x == x, x)` | `Unevaluated, msg "0 = 0: every value is a solution."` | HELD (gap) | — |
| c13 | literal false equation | `solve_full(1 == 2, x)` | `RC=1 InvalidOperation/DomainError "argument 1 must be a symbolic expression; got Boolean."` | HELD (no wrong answer; rough edge) | P2 |
| c14 | exponential never zero | `solve_full(exp(x) == 0, x, real)` | `NoSolutions/Complete "exp(u) = 0 has no solution."` | HELD | — |
| c15 | no real roots | `solve_full(x^2 + x + 1 == 0, x, real)` | `NoSolutions/Complete "no real solutions"` | HELD | — |
| c16 | repeated root, factored input | `solve_full((x - 1)^2 == 0, x)` | `Solved/Complete, solutions=[1 mult=1, 1 mult=1], represented_count=2` | FINDING (same equation, two different shapes: q06 gives 1 solution mult=2) | P2 |
| c17 | quartic roots on both axes | `solve_full(x^4 == 16, x)` | `Solved/Complete, [(mul -2 i),(mul 2 i),-2,2]` | HELD | — |
| c18 | |x| = negative | `solve_full(abs(x) == -1, x)` | `Unevaluated` | HELD (gap) | — |
| c19 | |x| = 2 | `solve_full(abs(x) == 2, x)` | `Unevaluated` | HELD (gap; SymPy real {−2,2}) | — |
| c20 | quintic, one real root | `solve_full(x^5 - 1 == 0, x)` | `Partial: represented 1, unrepresented_count 4, solve.unrepresented-roots` | HELD | — |
| c22 | trigonometric family | `solve_full(sin(x) == 0, x)` | `Solved/Complete, families=[template k*pi, parameter k, period pi, parameter_domain integer]` | HELD | — |
| c23 | identically-true product form | `solve_full(x*0 == 0, x)` | `Unevaluated, msg "0 = 0: every value is a solution."` | HELD (gap) | — |
| c24 | transcendental equation | `solve_full(log(x) == -1, x)` | `Solved/Complete, x=(fn exp (rat -1 1)), conditions=[(ne (rat 1 1) (rat 0 1))]` | HELD (the condition is vacuous `1 != 0`) | P3 |
| c25 | exponential equation | `solve_full(2^x == 8, x)` | `Unevaluated "No solver for this structure."` | HELD (gap; SymPy: 3) | — |

## 3. Solve systems (y-series, 6 probes + 2 witnesses)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| y01 | determined 2×2 | `solve_system_full([x + y == 3, x - y == 1], [x, y])` | `Solved/Complete, bindings x=2, y=1` | HELD | — |
| y02 | genuinely inconsistent | `solve_system_full([x + y == 3, x + y == 4], [x, y])` | `NoSolutions/Complete, solutions shape [0]` | HELD | — |
| y03 | one equation, two unknowns | `solve_system_full([x + y == 3], [x, y])` | `NoSolutions, complete=true, completeness=Complete, solutions=[]` | **FINDING** | **P0** |
| y05 | singular (dependent rows) | `solve_system_full([x + y == 1, 2*x + 2*y == 2], [x, y])` | `NoSolutions, complete=true, solutions=[]` | **FINDING** | **P0** |
| y06 | nonlinear + linear | `solve_system_full([x^2 + y^2 == 1, x == 0], [x, y])` | `Solved/Complete, (0,-1) and (0,1)` | HELD | — |
| y07 | solve_system short form | `solve_system([x + y == 3, x - y == 1], [x, y])` | `kind=Text value="x = 2, y = 1"` | **FINDING** (bindings only recoverable by parsing display text; `solve` does better) | **P1** |
| e05 | witness for y03 — the kernel's own arithmetic | `subs(subs(x + y, x, 0), y, 3)` | `canonical=(rat 3 1)` (satisfies `x+y=3`) | evidence | — |
| e06 | witness for y05 — (x,y)=(1,0) | `subs(subs(x + y, x, 1), y, 0); subs(subs(2*x + 2*y, x, 1), y, 0)` | last result `(rat 2 1)`; first statement gave `(rat 1 1)` (both satisfied) | evidence | — |

## 4. capabilities() trigger claims (t-series, 17 probes)

`capabilities()` lists 16 `unsupported_operations` entries, each with `operation_class`, `code`, `category`, `message` and a `trigger` script. Every trigger was executed verbatim.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| t01 | `pow.non-integer-exponent` | `2^(1/2)` | `RC=1 code=UnsupportedOperation category=UnsupportedOperation "Non-integer exponents are not yet supported."` | HELD | — |
| t02 | `pow.negative-base-unrepresentable-exponent` | `(-8)^(1/3)` | same code/category/message as documented | HELD | — |
| t03 | `solve.unsupported-domain` | `solve(x^2 - 2 == 0, x, integer)` | `RC=1 InvalidOperation/DomainError "solve(): currently supports domains real and complex; got integer."` | HELD | — |
| t04 | `rootof.complex-algebraic` | `solve_full(x^4 - x^2 - 1 == 0, x)` | `Partial + Diagnostic code=solve.unrepresented-roots category=UnsupportedOperation` | HELD | — |
| t05 | `limit.unevaluated-in-record-diagnostics` | `limit_full(sin(x), x, inf())` | `LimitStatus:Unevaluated, code=limit.unevaluated, message "coefficient does not evaluate at the point"` | HELD (message matches the metadata, though it is meaningless for `sin`) | P3 |
| t06 | `integration.unevaluated-in-record-diagnostics` | `integrate_full(exp(x^2), x)` | `IntegrationStatus:Unevaluated, expression=(integ (x) (fn exp (pow (sym x) (rat 2 1)))), code=integration.unevaluated, nested detail integration.no-closed-form` | HELD | — |
| t07 | `plot.symbolic-expression` | `plot(sin(x))` | `RC=1 InvalidOperation/DomainError "plot() argument 1 must be a vector, but got 'Symbolic'."` | HELD | — |
| t08 | `plot.symbolic-element` | `plot([x, 1, 2])` | `RC=1 InvalidOperation/DomainError "Cannot convert value of kind 'Symbolic' to a number for plotting."` | HELD | — |
| t09 | `solve.non-symbolic-variable` | `solve(x^2 - 2 == 0, 1)` | `RC=1 InvalidOperation/DomainError "argument 2 must be a symbolic variable; got Natural."` | HELD | — |
| t10 | `solve.non-symbolic-expression` | `solve([x == 1], x)` | `RC=1 InvalidOperation/DomainError "argument 1 must be a symbolic expression; got Vector."` | HELD | — |
| t11 | `diff.non-symbolic-variable` | `diff(x^2, 1)` | `RC=1 domain error, message as documented | HELD | — |
| t12 | `integrate.non-symbolic-variable` | `integrate(x^2, 1)` | `RC=1 domain error, message as documented | HELD | — |
| t13 | `limit.non-symbolic-variable` | `limit(sin(x), 1, 0)` | `RC=1 domain error, message as documented | HELD | — |
| t14 | `dsp.symbolic-element` | `dft([x, 1, 2, 3])` | `RC=1 DomainError "DSP builtins expect numeric/complex elements, but got 'SymbolExpr'."` | HELD | — |
| t15 | `linsolve.non-symbolic-matrix` | `linsolve([[1,2],[2,4]], [[1],[3]])` | `RC=1 DomainError "linsolve() requires a symbolic matrix A."` | HELD | — |
| t16 | `fft.non-power-of-two-length` | `fft([1,2,3])` | `RC=1 code=InvalidArgument category=TypeMismatch "FFT length must be a power of two, but got 3."` | HELD | — |
| t17 | cubic with a repeated root, factored input | `solve_full((x-1)^2*(x-2) == 0, x)` | `Unevaluated "No solver for this structure."` (the expanded form works: q05) | FINDING (coverage gap with honest status) | P2 |

## 5. Script-language edges and accessors (q-series, 14 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| q01 | print capture | `print(inspect(x^2 + 1).canonical, inspect(x^2 + 1).pretty)` | envelope has **no** `result` key; `output=["(add (rat 1 1) (pow (sym x) (rat 2 1))) x^2 + 1"]` | HELD (protocol `resultKind: Void` anticipated; see k08) | — |
| q02 | record indexing | `r = solve_full(x^2 - 4 == 0, x); r.solutions[0].value` | `canonical=(rat -2 1)` | HELD | — |
| q03 | nested fraction canonical/purity | `inspect((x+1)/(x-1))` | `canonical=(mul (pow (add (rat -1 1) (sym x)) (rat -1 1)) (add (rat 1 1) (sym x))) pretty=(x + 1)/(x - 1)` | HELD | — |
| q04 | string concatenation | `print("a" + "b")` | `RC=1 DomainError "Operator 'Add' is not supported for type 'Text'."` | HELD | — |
| q05 | cubic with double root, expanded | `solve_full(x^3 - 4*x^2 + 5*x - 2 == 0, x)` | `Solved/Complete, [1 mult=2, 2 mult=1]` | HELD | — |
| q06 | (x−1)² expanded | `solve_full(x^2 - 2*x + 1 == 0, x)` | `Solved/Complete, [1 mult=2], represented_count=1` | HELD (contrast c16) | — |
| q07 | radical equation with one extraneous root | `solve_full(sqrt(2*x + 3) == x, x)` | `Solved/Complete, solutions=[-1, 3]` | **FINDING** | **P0** |
| q08 | kernel's own substitution at the extraneous root | `subs(sqrt(2*x + 3) - x, x, -1)` | `canonical=(rat 2 1)` (≠ 0) | evidence | — |
| q09 | kernel's own substitution at the valid root | `subs(sqrt(2*x + 3) - x, x, 3)` | `canonical=(rat 0 1)` | evidence | — |
| q10 | last statement wins | `1 + 1; x^2; 42` | `kind=Natural value=42` | HELD | — |
| q11 | nested radical | `solve_full(sqrt(x) + sqrt(x + 1) == 1, x)` | `Unevaluated` | HELD (gap) | — |
| q12 | rootof evaluates | `r = solve_full(x^4 - x^2 - 1 == 0, x, real); evalf(r.solutions[0].value, 20); evalf(r.solutions[1].value, 20)` | last result `kind=Real value=1.2720196495140689642524` (= √((1+√5)/2) ✓) | HELD | — |
| q13 | cubic root with irrational real root | `solve_full(x^3 - 2 == 0, x)` | `Solved/Complete, 3 roots, all satisfying (v13); pretty forms are `2^(1/3)*…` | HELD (mathematically); printer breaks on it (F-PRINT-2) | — |
| q14 | sixth roots of unity | `solve_full(x^6 - 1 == 0, x)` | `Partial: represented 2, unrepresented_count 4` | HELD | — |

## 6. Substitution of every returned root into the equation (v-series, 20 probes)

Method: the kernel's own `canonical` for the residual expression `E` and for each returned root was translated to SymPy and `E.subs(x, root)` evaluated numerically at 40 dps. `residual` below is that complex value; `satisfies` = |residual| < 1e-15. SymPy's own solution set is shown for completeness.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| v01 | `sqrt(x)+2=0`, default domain | `solve_full(sqrt(x) + 2 == 0, x)` | `Solved/Complete, complete=true, root[0]=(rat 4 1) residual=(4.0,0.0) satisfies=false; SymPy solve=[] solveset=EmptySet` | **FINDING** | **P0** |
| v02 | `sqrt(x)=−2` | `solve_full(sqrt(x) - (-2) == 0, x)` | `Solved/Complete, root[0]=4 residual=(4.0,0.0) satisfies=false; SymPy []` | **FINDING** | **P0** |
| v03 | `sqrt(2x+3)=x` | `solve_full(sqrt(2*x + 3) - x == 0, x)` | `Solved/Complete, root[0]=-1 residual=(2.0,0.0) satisfies=false; root[1]=3 residual=0 satisfies=true; SymPy solve=[3]` | **FINDING** | **P0** |
| v04 | control: `sqrt(x)=2` | `solve_full(sqrt(x) - 2 == 0, x)` | `Solved/Complete, root[0]=4 residual=0 satisfies=true` | HELD | — |
| v05 | `sqrt(x+1)=x−1` | `solve_full(sqrt(x + 1) - (x - 1) == 0, x)` | `Unevaluated/Unknown, no roots returned` (SymPy {3}) | HELD | — |
| v06 | `x²−4=0` | `solve_full(x^2 - 4 == 0, x)` | `Solved/Complete, roots −2, 2, both residual 0` | HELD | — |
| v07 | `x²+1=0` | `solve_full(x^2 + 1 == 0, x)` | `Solved/Complete, roots (i), −i, both residual 0` | HELD | — |
| v08 | `x²+1=0` over real | `solve_full(x^2 + 1 == 0, x, real)` | `NoSolutions/Complete, solutions []` | HELD | — |
| v09 | `x³−1=0` | `solve_full(x^3 - 1 == 0, x)` | `Solved/Complete, 3 roots, all residual 0 (against SymPy {1, −1/2 ± i√3/2})` | HELD | — |
| v10 | quartic over real: RootOf | `solve_full(x^4 - x^2 - 1 == 0, x, real)` | `Solved/Complete, 2 rootof solutions, both residual 0 (ideal −1.272…, +1.272…)` | HELD | — |
| v11 | same quartic over complex | `solve_full(x^4 - x^2 - 1 == 0, x)` | `Partial/Partial, represented 2, unrepresented 2, roots residual 0` | HELD | — |
| v12 | `x⁵−1=0` | `solve_full(x^5 - 1 == 0, x)` | `Partial, represented 1 (x=1 residual 0), unrepresented 4` | HELD | — |
| v13 | `x³−2=0` | `solve_full(x^3 - 2 == 0, x)` | `Solved/Complete, 3 roots (2^(1/3)·ωᵏ), all residual 0` | HELD | — |
| v14 | `1/x=0` | `solve_full(1/x == 0, x)` | `NoSolutions/Complete, []` | HELD | — |
| v15 | removable singularity | `solve_full((x^2 - 1)/(x - 1) == 0, x)` | `Solved/Complete, root −1 residual 0, condition `x−1≠0` holds at −1` | HELD | — |
| v16 | `log(x)+1=0` | `solve_full(log(x) + 1 == 0, x)` | `Solved/Complete, root exp(−1) residual 0 (SymPy {exp(−1)})` | HELD | — |
| v17 | `exp(x)=0` | `solve_full(exp(x) == 0, x)` | `NoSolutions/Complete` | HELD | — |
| v18 | `x²−2=0` real | `solve_full(x^2 - 2 == 0, x, real)` | `Solved/Complete, roots ±(1/2)√8 = ±1.414213…, residual 0` | HELD | — |
| v19 | cubic, double root | `solve_full(x^3 - 4*x^2 + 5*x - 2 == 0, x)` | `Solved/Complete, root 1 multiplier 2 residual 0, root 2 residual 0 (SymPy {1:2, 2:1})` | HELD | — |
| v20 | `sin(x)=0` family | `solve_full(sin(x) == 0, x)` | `Solved/Complete, solutions [], families=[k·π, k∈ℤ]` | HELD | — |

## 7. diff / integrate / limit verified numerically against SymPy (33 probes)

`diff`: kernel canonical translated to SymPy and compared with `sympy.diff` at x ∈ {1/3, 7/5, 2, 9/4} (relative tolerance 1e-12). `integrate_full`: d/dx of the returned `expression` compared with the integrand at the same points (1e-10). `limit_full`: the returned value translated and compared with `sympy.limit`.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| d01 | `d/dx x^x` | `diff(x^x, x)` | `x^x*(log(x) + x/x)` vs SymPy `x**x*(log(x)+1)` — 4/4 sample points match (x/x vs 1) | HELD | — |
| d02 | `d/dx sin(x²)` | `diff(sin(x^2), x)` | `2*x*cos(x^2)` — 4/4 match | HELD | — |
| d03 | `d/dx 1/x` | `diff(1/x, x)` | `-x^-2` — 4/4 match | HELD | — |
| d04 | `d/dx |x|` | `diff(abs(x), x)` | `pw ((ne x 0) (fn sign x)) (der (x) (fn abs x))` / `piecewise(sign(x) if x != 0, diff(abs(x), x))` — honest at x=0 | HELD | — |
| d05 | `d/dx √x` | `diff(sqrt(x), x)` | `1/2*1/sqrt(x)` = `1/(2√x)` — 4/4 match | HELD | — |
| d06 | `d/dx log(x)/x` | `diff(log(x)/x, x)` | `x^-2 - log(x)*x^-2` — 4/4 match | HELD | — |
| d07 | `d/dx tan x` | `diff(tan(x), x)` | `cos(x)^-2` — 4/4 match | HELD | — |
| d08 | `d/dx e^{−x²}` | `diff(exp(-x^2), x)` | `-2*x*exp(-x^2)` — 4/4 match | HELD | — |
| d09 | product rule | `diff(x^2*sin(x), x)` | `2*x*sin(x) + cos(x)*x^2` — 4/4 match | HELD | — |
| d10 | quotient rule | `diff(sin(x)/x, x)` | `-sin(x)*x^-2 + cos(x)/x` — 4/4 match | HELD | — |
| i01 | `∫x²dx` | `integrate_full(x^2, x)` | `status=SolvedExact verified=true method=table expression=1/3*x^3` — d/dx matches integrand | HELD | — |
| i02 | `∫dx/x` | `integrate_full(1/x, x)` | `SolvedConditional verified=true method=table expression=log(x)` — d/dx matches | HELD | — |
| i03 | `∫sin(x)/x dx` | `integrate_full(sin(x)/x, x)` | `Unevaluated, expression=(integ (x) (mul (fn sin (sym x)) (pow (sym x) (rat -1 1)))), code=integration.unevaluated` | HELD (gap) | — |
| i04 | `∫dx/(x²+1)` | `integrate_full(1/(x^2 + 1), x)` | `SolvedExact method=partial-fractions expression=atan(x)` — d/dx matches | HELD | — |
| i05 | `∫tan x dx` | `integrate_full(tan(x), x)` | `Unevaluated` (SymPy: −log(cos x)) | HELD (gap) | — |
| i06 | by parts | `integrate_full(x*exp(x), x)` | `SolvedExact method=by-parts expression=exp(x)*(x - 1)` — d/dx matches | HELD | — |
| i08 | `∫√x dx` | `integrate_full(sqrt(x), x)` | `SolvedExact method=table expression=2/3*x^(3/2)` — d/dx matches | HELD | — |
| i09 | partial fractions | `integrate_full(1/(x^2 - 1), x)` | `SolvedConditional method=partial-fractions expression=-1/2*log(x + 1) + 1/2*log(x - 1)` — d/dx matches at 4/4 points | HELD | — |
| i10 | `∫log x dx` | `integrate_full(log(x), x)` | `Unevaluated` (SymPy: x log x − x) | HELD (gap) | — |
| l01 | `lim sin(x)/x` at 0 | `limit_full(sin(x)/x, x, 0)` | `status=Value exists=true value=(rat 1 1)`; SymPy 1 | HELD | — |
| l02 | `lim 1/x` at 0 | `limit_full(1/x, x, 0)` | `status=DoesNotExist exists=false value=null` (two-sided limit does not exist; SymPy's `oo` is its one-sided default) | HELD | — |
| l03 | indeterminate power | `limit_full((1 + 1/x)^x, x, inf())` | `status=Value exists=true value=(rat 1 1)`; SymPy `E` = 2.7182818284590452354 | **FINDING** | **P0** |
| l04 | 0·∞ form | `limit_full(x*log(x), x, 0)` | `Value, value=0`; SymPy 0 | HELD | — |
| l05 | oscillating | `limit_full(sin(1/x), x, 0)` | `Unevaluated` (SymPy AccumBounds(−1,1)) | HELD (gap) | — |
| l06 | classic | `limit_full((1 - cos(x))/x^2, x, 0)` | `Value 1/2`; SymPy 1/2 | HELD | — |
| l07 | `lim √x` at 0 | `limit_full(sqrt(x), x, 0)` | `Value 0` | HELD | — |
| l08 | jump | `limit_full(abs(x)/x, x, 0)` | `Unevaluated` (no two-sided limit) | HELD (gap) | — |
| l09 | removable | `limit_full((x^2 - 4)/(x - 2), x, 2)` | `Value 4` | HELD | — |
| l10 | `lim e^{−x}` at ∞ | `limit_full(exp(-x), x, inf())` | `Unevaluated` (SymPy 0) | HELD (gap) | — |
| l11 | polynomial at ∞ | `limit_full(x^2, x, inf())` | `status=PlusInfinity value=(inf)` | HELD | — |
| l12 | squeeze | `limit_full(x*sin(1/x), x, 0)` | `Unevaluated` (SymPy 0) | HELD (gap) | — |
| l13 | indeterminate power | `limit_full((1 + x)^(1/x), x, 0)` | `status=Value exists=true value=(rat 1 1)`; SymPy `E` | **FINDING** | **P0** |
| l14 | `lim log x` at 0 | `limit_full(log(x), x, 0)` | `Unevaluated` (SymPy −∞ one-sided) | HELD (gap) | — |

## 8. Special values, exactness, numeric tower (n/m/x-series, 73 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| n01 | evalf digits | `evalf(sqrt(2), 30)` | `Real 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727` — mpmath: first 100 digits correct, |err|=3.5e-101 | HELD | — |
| n02 | π digits | `pi(30)` | `Real 3.141592653589793238462643383279` — 30 digits correct (|err|=5.0e-31) | HELD | — |
| n03 | e digits | `e(30)` | `Real 2.718281828459045235360287471352` — 30 digits correct | HELD | — |
| n04 | evalf of a Real | `evalf(1/3, 30)` | `Real 0.333333333333333333333333333333` (exactly 30 digits) | HELD | — |
| n05 | exact rational identity | `((1/17)*17) == 1` | `kind=Boolean value=false` | **FINDING** | **P0** |
| n06 | decimal sums | `(0.1 + 0.2) == 0.3` | `Boolean true` (decimal arithmetic, not IEEE binary) | HELD | — |
| n07 | precision control | `setprecision(50); pi(50)` | `Real 3.14159265358979323846264338327950288419716939937510` — 50 digits correct | HELD | — |
| n08 | 0^0 | `0^0 == 1` | `Boolean true` | HELD | — |
| n09 | evalf at a pole | `evalf(log(0), 20)` | `RC=1 code=EvaluationError category=DomainError "Ln(x) requires x > 0. (Parameter 'x')"` | FINDING (same input is accepted symbolically by s10) | P2 |
| n10 | √ of negative | `sqrt(-4)` | `Symbolic exact=true canonical=(mul (rat 2 1) (i)) pretty=2*i` | HELD | — |
| n11 | ∞−∞ | `inf() - inf()` | `Symbolic exact=false canonical=(mul (rat 0 1) (inf)) pretty=0*inf` | FINDING (indeterminate form left unevaluated, `exact=false`) | P2 |
| n12 | 0·∞ | `0*inf()` | `canonical=(mul (rat 0 1) (inf))` | FINDING (same) | P2 |
| n13 | e^∞ | `exp(inf())` | `canonical=(fn exp (inf))` (should be +∞) | FINDING (same) | P2 |
| n14 | ∞=∞ | `inf() == inf()` | `kind=Symbolic canonical=(eq (inf) (inf)) pretty=inf = inf` — `==` returns an equation node, not a Boolean | FINDING (same) | P2 |
| n15 | |−∞| | `abs(-inf())` | `canonical=(fn abs (mul (rat -1 1) (inf)))` | FINDING (same) | P2 |
| n16 | big exact integer | `2^1000` | `Natural value=10715086071862673209484250490600018105614048117055336074437503883703510511249361224931983788156958581275946729175531468251871452856923140435984577574698574803934567774824230985421074605062371141877954182153046474983581941267398767559165543946077062914571196477686542167660429831652624386837205668069376` — correct | HELD | — |
| n17 | 10^400 | `10^400` | `Natural value=1` followed by 400 zeros | HELD | — |
| n18 | sign(0) | `sign(0)` | `Integer 0` | HELD | — |
| n19 | abs(−0) | `abs(-0)` | `Integer 0` | HELD | — |
| n20 | tan at the pole | `tan(pi(30)/2)` | `Symbolic canonical=(fn tan (real 1.5707963267948966192313216916395))` | HELD (faithful; the argument is an inexact π/2) | — |
| n21 | log(0) vs −∞ | `log(0) == -inf()` | `Symbolic canonical=(eq (fn log (rat 0 1)) (mul (rat -1 1) (inf)))` | FINDING (same family as n11) | P2 |
| n22 | evalf e | `evalf(exp(1), 30)` | `Real 2.718281828459045235360287471352` — correct | HELD | — |
| n23 | nested division by zero | `1/(1/0)` | `RC=1 DivisionByZero/DomainError` | HELD | — |
| n24 | sqrt(0) | `sqrt(0)` | `kind=Real value=0` | HELD | — |
| n25 | `exact` on a derivative | `diff(x^2, x)` | `canonical=(mul (rat 2 1) (sym x)) exact=true` | HELD | — |
| n26 | `exact` on a derivative | `diff(sin(x), x)` | `canonical=(fn cos (sym x)) **exact=false**` | **FINDING** | **P0** |
| n27 | Pythagorean simplification | `simplify(sin(x)^2 + cos(x)^2)` | `canonical=(rat 1 1) exact=true` | HELD | — |
| n28 | simplify_full, no change | `simplify_full((x^2-1)/(x-1))` | `status=Satisfied, original=(x^2 - 1)/(x - 1), expression=(x^2 - 1)/(x - 1), changed=true, conditions=[]` | FINDING (`changed=true` on an unchanged expression) | P2 |
| n29 | cancel path | `cancel((x^2-1)/(x-1))` | `canonical=(add (rat 1 1) (sym x)) exact=true` | HELD (`cancel` is not claimed to be condition-preserving; the `_full` form is: x14) | — |
| n30 | expand | `expand((x + 1)^3)` | `canonical=(add (rat 1 1) (pow (sym x) (rat 3 1)) (mul (rat 3 1) (sym x)) (mul (rat 3 1) (pow (sym x) (rat 2 1))))` — correct | HELD | — |
| n31 | factor | `factor(x^2 - 1)` | `(mul (add (rat -1 1) (sym x)) (add (rat 1 1) (sym x)))` — correct | HELD | — |
| n32 | simplify √(x²) without assumptions | `simplify(sqrt(x^2))` | `canonical=(pow (pow (sym x) (rat 2 1)) (rat 1 2)) exact=false` — unchanged (correct over ℂ) | HELD | — |
| n33 | factor over ℝ | `factor(x^2 + 1)` | `canonical=(add (rat 1 1) (pow (sym x) (rat 2 1)))` — unchanged (correct over ℝ) | HELD | — |
| n34 | apart | `apart(1/(x^2 - 1), x)` | `-1/(2*(x + 1)) + 1/(2*(x - 1))` — correct | HELD | — |
| n35 | decimal division | `1/3 + 1/6` | `Real 0.5` | HELD | — |
| n36 | decimal round-trip | `3*(1/3)` | `Real 1` | HELD | — |
| n37 | radical product | `sqrt(2)*sqrt(2)` | `Real 1.999…(100 nines)` | FINDING (numeric tower; see F-1/17) | P2 |
| n38 | radical power | `sqrt(2)^2` | `Real 1.999…(100 nines)` | FINDING (same) | P2 |
| n39 | radical residual | `sqrt(2)^2 - 2` | `Real -0.000…(≈200 zeros)…2536947353619806411250171119020069889980638345900510092463050373669602008880936473413829448328397071881957966744983064473793250958186299266722210106893505692208241079145341472485557192036633345122768870256512452497239747751769031171535386737065944768432298001563818581551540315149006855705634401994594241788195280719936707318279293114976870655810128718788004158328771366445523672656979836070185702000266883571692280964022207326791116280457119431583133195091643088462480821588825517977336020840348114791984320396880830718259834123245144964850740041889377356424131415300632021414311332680055604133843976865957952344713430965945064741086290707946988084045688187674719628358300673572886528029756442949506328620675945439175388669322637357388945601539435320841638682449009455845111989513736019422386568113668527433733405389647098962319609734609051157118942706782685572032336773983501091790352539442303628786827337221625618652865012737037797069889545683638296023114698920984343307219084684726617528403105216` | FINDING (same) | P2 |
| n40 | digits argument ignored | `evalf(sqrt(2), 5)` | `Real 1.414…(the same 100-digit string as s13)` | FINDING (`digits` has no effect on a symbolic argument) | P2 |
| m01 | repeating-decimal display | `1/17` | `Real 0.(0588235294117647)` | FINDING (the display asserts an exact repeating decimal; F-1/17 shows the arithmetic does not honour it) | P0 (with n05) |
| m02 | the product of that value | `(1/17)*17` | `Real 0.999…(1000 digits)…9984` — 1002 chars; `1 - value = 1.6e-999` | **FINDING** | **P0** |
| m03 | control | `3*(1/3)` | `Real 1` | HELD | — |
| m04 | control | `(1/7)*7 == 1` | `Boolean true` | HELD | — |
| m05 | control | `(1/3)*3 == 1` | `Boolean true` | HELD | — |
| m06 | LaTeX of exact third | `latex(1/3)` | `Text \frac{1}{3}` | HELD | — |
| m07 | LaTeX of 1/17 | `latex(1/17)` | `Text \frac{1}{17}` | HELD (text is right; the value isn't) | — |
| m08 | LaTeX of a decimal sum | `latex(0.1 + 0.2)` | `Text \frac{3}{10}` | HELD | — |
| m09 | LaTeX of a Real | `latex(evalf(sqrt(2), 30))` | `Text 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727…(≈500 digits)` | FINDING (renderer shows ~500 digits while `evalf` shows 100 for the same value) | P2 |
| m10 | LaTeX of π | `latex(pi(30))` | `Text 3.141592653589793238462643383279` | HELD | — |
| m11 | exactness of a decimal literal | `1/3 == 0.333333333333333333333333333333` | `Boolean false` (correct: the literal is a different number) | HELD | — |
| m12 | decimal sum value | `0.1 + 0.2` | `Real 0.3` | HELD | — |
| m13 | exact decimal subtraction | `1 - 0.9999999999999999999999999999999999999999` | `Real 0.0000000000000000000000000000000000000001` (=10⁻⁴⁰) | HELD | — |
| x01 | `exact` flag sweep | `x` | `exact=true` | HELD | — |
| x02 | `exact` flag sweep | `x^2` | `exact=true` | HELD | — |
| x03 | `exact` flag sweep | `(1/2)*x` | `exact=true` | HELD | — |
| x04 | `exact` flag sweep | `sin(x)` | `exact=false` | **FINDING** | **P0** |
| x05 | `exact` flag sweep | `cos(x)` | `exact=false` | **FINDING** | **P0** |
| x06 | `exact` flag sweep | `sqrt(x)` | `exact=false` | **FINDING** | **P0** |
| x07 | `exact` flag sweep | `log(x)` | `exact=false` | **FINDING** | **P0** |
| x08 | `exact` flag sweep | `x + 1` | `exact=true` | HELD | — |
| x09 | `exact` flag sweep | `exp(x)` | `exact=false` | **FINDING** | **P0** |
| x10 | `exact` flag sweep | `abs(x)` | `exact=true` | HELD (the flag is not "contains a function") | — |
| x11 | numeric literal | `2` | `kind=Natural value=2 exact=true` | HELD | — |
| x12 | π as Real | `pi(30)` | `kind=Real value=3.141592653589793238462643383279` | HELD | — |
| x13 | simplify_full, no-op | `simplify_full((x^2 - 1)/(x - 1))` | `status=Satisfied, expression unchanged, changed=true` | FINDING (duplicate of n28) | P2 |
| x14 | cancel_full keeps the domain condition | `cancel_full((x^2 - 1)/(x - 1))` | `status=CancelStatus:Conditional, expression=x + 1, conditions=[x - 1 != 0]` | HELD (this is the honest form) | — |
| x15 | simplify_full of x/x | `simplify_full(x/x)` | `status=Satisfied, expression=1, conditions=[x != 0], steps=[RewriteStep rule_id=rat.cancel-x-over-x classification=Conditional required_conditions=[x != 0]]` | HELD | — |
| x16 | cancel of x/x | `cancel(x/x)` | `canonical=(rat 1 1)` | HELD | — |
| x17 | simplify, no rewrite available | `simplify((x^2 - 1)/(x - 1))` | `canonical unchanged (mul (pow (add (rat -1 1) (sym x)) (rat -1 1)) (add (rat -1 1) (pow (sym x) (rat 2 1))))` | HELD | — |
| x18 | simplify_full of equal terms | `simplify_full(1/x + 1/x)` | `status=Satisfied, expression=2/x, conditions=[]` — correct | HELD | — |
| x19 | assumptions | `assume_positive(x); simplify(sqrt(x^2))` | `canonical=(sym x)` — correct under x>0 | HELD | — |
| x20 | assumptions cleared | `assume_positive(x); assume_clear(); simplify(sqrt(x^2))` | `canonical=(pow (pow (sym x) (rat 2 1)) (rat 1 2))` | HELD | — |

## 8.1 LaTeX renderer (L-series, 30 probes)

Every rendered string was read against the value the kernel returned for the same expression (`inspect` canonical / pretty). `latex(·)` returns a `Text` value; no string below is re-parsed by the kernel, so the check is manual LaTeX semantics against the kernel's own canonical form.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| L01 | polynomial | `latex(x^2 + 1)` | `x^{2} + 1` | HELD | — |
| L02 | fraction of sums | `latex((x + 1)/(x - 1))` | `\frac{x + 1}{x - 1}` | HELD | — |
| L03 | nested fraction | `latex(1/(1 + 1/(1 + 1/x)))` | `\left(1 + \left(1 + x^{-1}\right)^{-1}\right)^{-1}` | HELD | — |
| L04 | radical | `latex(sqrt(x))` | `\sqrt{x}` | HELD | — |
| L05 | negative exponent | `latex(x^(-1))` | `x^{-1}` | HELD | — |
| L06 | unary minus vs parenthesised base | `latex(-x^2); latex((-x)^2)` | last result `\left(-x\right)^{2}` (braces protect the base — the classic failure is absent) | HELD | — |
| L07 | commutative reordering | `latex(2 - x)` | `-x + 2` | HELD | — |
| L08 | nested parentheses | `latex(x - (y - x))` | `x - \left(y - x\right)` | HELD | — |
| L09 | negated sum | `latex(-(x + y))` | `-\left(x + y\right)` | HELD | — |
| L10 | division chain | `latex(x/y/x)` | `\frac{x}{x \cdot y}` | HELD (the text means x/(xy), which is what the expression means) | — |
| L11 | complex quotient | `latex((1 + sqrt(-1))/(1 - sqrt(-1)))` | `\frac{1 + i}{1 - i}` | HELD | — |
| L12 | infinity | `latex(inf())` | `\infty` | HELD | — |
| L13 | negative infinity | `latex(-inf())` | `-\infty` | HELD | — |
| L14 | absolute value | `latex(abs(x))` | `\left|x\right|` | HELD | — |
| L15 | quotient incl. function | `latex(sin(x)/x)` | `\frac{\sin(x)}{x}` | HELD | — |
| L16 | exponential | `latex(exp(-x^2))` | `\exp(-x^{2})` | HELD | — |
| L17 | complex number | `latex(sqrt(-4))` | `2 \cdot i` | HELD | — |
| L18 | rational coefficient | `latex((1/2)*x)` | `\frac{1}{2} \cdot x` | HELD | — |
| L19 | folded power | `latex(x^2^3)` | `x^{8}` | HELD | — |
| L20 | mixed product/fraction | `latex(x*(y + 1)^2/(x + 2))` | `\frac{x \cdot \left(y + 1\right)^{2}}{x + 2}` | HELD | — |
| L21 | a Real as a fraction | `latex(1/3 + 1/6)` | `\frac{1}{2}` | HELD (0.5 is exactly 1/2) | — |
| L22 | apart result | `latex(apart(1/(x^2 - 1), x))` | `-\frac{1}{2 \cdot \left(x + 1\right)} + \frac{1}{2 \cdot \left(x - 1\right)}` | HELD | — |
| L23 | exact quadratic root | `r = solve_full(x^2 - 2 == 0, x, real); latex(r.solutions[0].value)` | `-\frac{1}{2} \cdot \sqrt{8}` | HELD (equals −√2) | — |
| L24 | RootOf | `r = solve_full(x^4 - x^2 - 1 == 0, x, real); latex(r.solutions[0].value)` | `\operatorname{rootof}(x^{4} - x^{2} - 1, 0)` | HELD (faithful; `\operatorname{rootof}` is not a standard operator but renders the same text as the pretty form) | — |
| L25 | integer | `latex(42)` | `42` | HELD | — |
| L26 | tan of a Real | `latex(tan(pi(30)/2))` | `\tan(1.5707963267948966192313216916395)` | HELD | — |
| L27 | rational exponent | `latex(sqrt(x)^3)` | `x^{\frac{3}{2}}` | HELD | — |
| L28 | logarithm | `latex(log(x + 1))` | `\log(x + 1)` | HELD | — |
| L29 | vector argument | `latex([1, x, 3])` | `RC=1 DomainError "Undefined variable 'x'."` — probe authoring error (x not declared) | discarded | — |
| L30 | indeterminate product | `latex(inf() - inf())` | `0 \cdot \infty` | HELD (faithful to the unevaluated `0*inf`) | — |

The renderer is the strongest part of the surface attacked: 28/28 non-discarded probes produced LaTeX that means what the kernel's canonical form means, including the three usual failure points (negative base under a power, nested fractions, division chains). Its only defect is F20 (digit count, not structure).

## 9. Printer round-trip: parse(pretty(e)) vs e (r-series, 40 probes)

Property under test: for an expression e built by the auditor, `inspect(e)` returns `canonical` C and `pretty` P; the audit re-parsed P in a **fresh script** and compared the resulting canonical with C.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| r01 | polynomial | `inspect(x^2 + 1)` | C=C2=`(add (rat 1 1) (pow (sym x) (rat 2 1)))`, P=`x^2 + 1` | HELD | — |
| r02 | ratio of sums | `inspect((x + 1)/(x - 1))` | C=C2=`(mul (pow (add (rat -1 1) (sym x)) (rat -1 1)) (add (rat 1 1) (sym x)))`, P=`(x + 1)/(x - 1)` | HELD | — |
| r03 | triple-nested fraction | `inspect(1/(1 + 1/(1 + 1/x)))` | P=`(1 + (1 + x^-1)^-1)^-1`, C=C2 | HELD | — |
| r04 | sqrt(2) | `inspect(sqrt(2))` | e evaluates to `kind=Real` (a number, not a symbol) — nothing to round-trip | N/A (not a symbolic value) | — |
| r05 | radical | `inspect(sqrt(x) + 2)` | P=`sqrt(x) + 2`, C=C2=`(add (rat 2 1) (pow (sym x) (rat 1 2)))` | HELD | — |
| r06 | unary minus precedence | `inspect(-x^2)` | P=`-x^2`, C=C2=`(mul (rat -1 1) (pow (sym x) (rat 2 1)))` | HELD | — |
| r07 | parenthesised negative base | `inspect((-x)^2)` | P=`(-x)^2`, C=C2=`(pow (mul (rat -1 1) (sym x)) (rat 2 1))` | HELD | — |
| r08 | subtraction reordered | `inspect(2 - x)` | P=`-x + 2`, C=C2 | HELD | — |
| r09 | nested minus | `inspect(x - (y - x))` | P=`x - (y - x)`, C=C2 (parentheses preserved) | HELD | — |
| r10 | negated sum | `inspect(-(x + y))` | P=`-(x + y)`, C=C2 | HELD | — |
| r11 | left-associative division | `inspect(x/y/x)` | C=`(mul (sym x) (pow (sym x) (rat -1 1)) (pow (sym y) (rat -1 1)))`, P=`x/(x*y)`, **C2=`(mul (sym x) (pow (mul (sym x) (sym y)) (rat -1 1)))` != C** | **FINDING** | **P1** |
| r12 | rational coefficient | `inspect((1/2)*x)` | P=`1/2*x`, C=C2 | HELD | — |
| r13 | negative exponent | `inspect(x^(-1))` | P=`x^-1`, C=C2 | HELD | — |
| r14 | reciprocal square | `inspect(1/x^2)` | P=`x^-2`, C=C2=`(pow (sym x) (rat -2 1))` | HELD | — |
| r15 | root of a square | `inspect(sqrt(x^2))` | P=`sqrt(x^2)`, C=C2 | HELD | — |
| r16 | negative base, even power | `inspect((-8)^2)` | e evaluates to `kind=Integer value=64` | N/A | — |
| r17 | negative base, odd power | `inspect((-8)^3)` | e evaluates to `kind=Integer value=-512` | N/A | — |
| r18 | negative symbolic base, odd power | `inspect((-x)^3)` | P=`(-x)^3`, C=C2 | HELD | — |
| r19 | complex unit | `inspect(sqrt(-1))` | C=`(i)`, P=`i` -> re-parse: `RC=1 InvalidOperation/DomainError "Undefined variable 'i'."` | **FINDING** | **P1** |
| r20 | complex quotient | `inspect((1 + sqrt(-1))/(1 - sqrt(-1)))` | P=`(1 + i)/(1 - i)` -> re-parse fails: `Undefined variable 'i'.` | **FINDING** | **P1** |
| r21 | infinity | `inspect(inf())` | P=`inf`, C=C2=`(inf)` | HELD | — |
| r22 | negative infinity | `inspect(-inf())` | P=`-inf`, C=C2 | HELD | — |
| r23 | absolute value | `inspect(abs(x))` | P=`abs(x)`, C=C2 | HELD | — |
| r24 | quotient of functions | `inspect(sin(x)/x)` | P=`sin(x)/x`, C=C2 | HELD | — |
| r25 | exponential of a negative square | `inspect(exp(-x^2))` | P=`exp(-x^2)`, C=C2 | HELD | — |
| r26 | log of a sum | `inspect(log(x + 1))` | P=`log(x + 1)`, C=C2 | HELD | — |
| r27 | mixed product/reciprocal | `inspect(x^2*(x + 1)/(x - 1)^2)` | P=`x^2*(x - 1)^-2*(x + 1)`, C=C2 | HELD | — |
| r28 | decimal sum | `inspect(1/3 + 1/6)` | evaluates to `kind=Real value=0.5` | N/A | — |
| r29 | numeric base, rational exponent | `2^(1/3)` | `RC=1 UnsupportedOperation "Non-integer exponents are not yet supported."` | FINDING (context for F7) | P1 |
| r30 | rational exponent via ^ | `inspect(x^(1/2))` | P=`sqrt(x)`, C=C2=`(pow (sym x) (rat 1 2))` | HELD | — |
| r31 | difference of squares | `inspect((x + y)*(x - y))` | P=`(x + y)*(x - y)`, C=C2 | HELD | — |
| r32 | sum of reciprocals | `inspect(1/(x + 1) + 1/(x - 1))` | P=`(x - 1)^-1 + (x + 1)^-1`, C=C2 | HELD | — |
| r33 | quotient of radicals | `inspect(sqrt(x + 1)/sqrt(x - 1))` | P=`1/sqrt(x - 1)*sqrt(x + 1)`, C=C2 | HELD | — |
| r34 | right-associative power | `inspect(x^2^3)` | P=`x^8`, C=C2=`(pow (sym x) (rat 8 1))` | HELD | — |
| r35 | parenthesised power | `inspect((x^2)^3)` | P=`x^6`, C=C2 | HELD | — |
| r36 | negative exponent, numeric base | `inspect(2^(-3))` | evaluates to `kind=Real value=0.125` | N/A | — |
| r37 | negative base and exponent | `inspect((-2)^(-3))` | evaluates to `kind=Real value=-0.125` | N/A | — |
| r38 | undefined variable | `inspect(x*(y + 1)^2/(z))` | `RC=1 DomainError "Undefined variable 'z'."` — probe authoring error | discarded | — |
| r39 | trig identity, un-simplified | `inspect(cos(x)^2 + sin(x)^2)` | P=`cos(x)^2 + sin(x)^2`, C=C2 | HELD | — |
| r40 | cotangent identity | `inspect(tan(x) - 1/tan(x))` | P=`tan(x) - tan(x)^-1`, C=C2 | HELD | — |

### 9.1 Round-trip of values the *solver itself* produces (s01r-s20r, 20 probes)

The pretty strings below were obtained from solver results (`inspect(r.solutions[i].value)`) and then re-parsed as fresh scripts.

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| s01r | `i` (roots of x^2+1) | `i` | `RC=1 InvalidOperation/DomainError "Undefined variable 'i'."` | **FINDING** | **P1** |
| s02r | RootOf real root | `rootof(x^4 - x^2 - 1, 0)` | `RC=1 "Unknown function 'rootof'."` | **FINDING** | **P1** |
| s03r | cube root of 2 | `2^(1/3)` | `RC=1 UnsupportedOperation "Non-integer exponents are not yet supported."` | **FINDING** | **P1** |
| s04r | complex cube root | `2^(1/3)*(-1/2*i*sqrt(3) - 1/2)` | `RC=1 UnsupportedOperation` | **FINDING** | **P1** |
| s05r | family template | `k*pi` | `RC=1 "Undefined variable 'k'."` | **FINDING** | **P1** |
| s06r | exact quadratic root | `inspect(-1/2*sqrt(8))` | the text evaluates to `kind=Real` (~1.41421356...), `pretty` field `Null`; the exact canonical `(mul (rat -1 2) (pow (rat 8 1) (rat 1 2)))` is not recovered | **FINDING** (exactness lost on round-trip) | **P1** |
| s07r | piecewise derivative | `piecewise(sign(x) if x != 0, diff(abs(x), x))` | `RC=1 "Expected 'RParen' but found 'if' at position 60."` | **FINDING** | **P1** |
| s08r | unevaluated integral | `integrate(sin(x)/x, x)` | C=C2=`(integ (x) (mul (fn sin (sym x)) (pow (sym x) (rat -1 1))))` | HELD | — |
| s09r | 0*inf | `0*inf` | C=C2=`(mul (rat 0 1) (inf))` | HELD | — |
| s10r | e^inf | `exp(inf)` | C=C2=`(fn exp (inf))` | HELD | — |
| s11r | abs(-inf) | `abs(-inf)` | C=C2=`(fn abs (mul (rat -1 1) (inf)))` | HELD | — |
| s12r | tan of a Real | `tan(1.5707963267948966192313216916395)` | C=C2=`(fn tan (real 1.5707963267948966192313216916395))` | HELD | — |
| s13r | exp(-1) | `exp(-1)` | C=C2=`(fn exp (rat -1 1))` | HELD | — |
| s14r | the pretty of r11's expression | `x/(x*y)` | C=C2=`(mul (sym x) (pow (mul (sym x) (sym y)) (rat -1 1)))` (self-consistent, but != r11's C) | HELD / see F8 | — |
| s15r | negated i | `-i` | `RC=1 "Undefined variable 'i'."` | **FINDING** | **P1** |
| s16r | 2i | `2*i` | `RC=1 "Undefined variable 'i'."` | **FINDING** | **P1** |
| s17r | negative exponent | `x^-2` | C=C2=`(pow (sym x) (rat -2 1))` | HELD | — |
| s18r | cube root of unity | `1/2*(i*sqrt(3) - 1)` | `RC=1 "Undefined variable 'i'."` | **FINDING** | **P1** |
| s19r | RootOf index 1 | `rootof(x^4 - x^2 - 1, 1)` | `RC=1 "Unknown function 'rootof'."` | **FINDING** | **P1** |
| s20r | equation text | `inf = inf` | parses as `(inf)` (the `=` is not an operator in expression position) | HELD | — |

## 10. Protocol / CLI contract (k-series and print-budget, 37 probes)

| id | what was probed | command | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| k01 | wrong arity (too few) | `compile_full(x^2)` | `RC=1 InvalidArgument/TypeMismatch "compile_full(): expected 2 arguments; got 1."` + one-line stdout + `diagnostics` with position/line/column | HELD (matches dsh-protocol Error envelope) | — |
| k02 | wrong arity, variadic builtin | `solve_full(x^2 - 4 == 0)` | `"solve_full(): expected 2 to 3 arguments; got 1."` | HELD | — |
| k03 | one past the documented max | `solve_full(x^2 - 4 == 0, x, real, 1)` | `"expected 2 to 3 arguments; got 4."` | HELD | — |
| k04 | zero args | `symbol()` | `"symbol(): expected 1 to 2 arguments; got 0."` | HELD | — |
| k05 | too many args | `symbol("x", real, 1)` | `"symbol(): expected 1 to 2 arguments; got 3."` | HELD | — |
| k06 | wrong arity | `diff(x^2)` | `"diff(): expected 2 arguments; got 1."` | HELD | — |
| k07 | statement separation | `x = symbol("x") + newline + solve(x^2 - 4 == 0, x)` (newline, no semicolon) | `RC=0, Vector [-2, 2]` — a newline **is** accepted as a separator | note (contradicts the task brief, not the product) | P3 |
| k08 | empty script | `--file` empty | `RC=0; envelope has **no result key** (top keys ... ok, revision, output, variables, functions, elapsed ...) | FINDING (no `{"kind":"Null"}`; the key vanishes) | P2 |
| k09 | whitespace-only script | `--file` " " | same as k08 | FINDING (same) | P2 |
| k10 | print capture and stdout purity | `print(1, 2, 3)` | stdout = exactly one JSON line; `output=["1 2 3"]`; no result key | HELD (invariants 1 and 4 hold) | — |
| k11 | empty diagnostics is an Array | `r.diagnostics` after a 2-root solve | `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}` | HELD (never Null, never "") | — |
| k12 | empty equation list | `solve_system_full([], [x])` | `RC=1 InvalidArgument/TypeMismatch "Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')"` | **FINDING** (leaked CLR index exception; `TypeMismatch` is false — both arguments are well-typed) | **P1** |
| k13 | empty variable list | `solve_system_full([x == 1], [])` | `RC=0 SystemSolveResult status=Unevaluated, diagnostic code system-...` (graceful) | HELD | — |
| k14 | print-budget, small expression | `--print-budget 20` on `inspect((x+1)^8)` | no `truncated`/`budget` fields; full canonical | FINDING (see F10) | P1 |
| k15 | print-budget 0 | `--print-budget 0` | `RC=2, stderr "Error: --print-budget requires a positive node count."` | HELD | — |
| k15b | print-budget 1 on a 5-node value | `--print-budget 1` on `x = symbol("x"); (x+1)^8` | envelope byte-identical to the no-flag run (after removing elapsed/revision) | **FINDING** | **P1** |
| k15c | print-budget -5 | `--print-budget -5` | `RC=2 usage error` | HELD | — |
| k15d | print-budget abc | `--print-budget abc` | `RC=2 usage error` | HELD | — |
| k16 | unknown flag | `--bogus` | `RC=2, stderr "Unknown argument '--bogus'." + usage` | HELD (exit 2 = usage) | — |
| k17 | missing file | `--file C:\nope\nope.ls` | `RC=1 code=FileReadError **category=ParseError** "Cannot read script file ..."` | FINDING (a missing file is reported as a ParseError; exit 2 is reserved for usage errors) | P2 |
| k18 | no arguments | (none) | `RC=2 "No script provided..."` | HELD | — |
| k19 | --eval | `--eval "1 + 1"` | `ok, Natural 2` | HELD | — |
| k19b | --eval with empty string | `--eval ""` | `RC=0, no result key` | FINDING (same as k08) | P2 |
| k20 | maximum precision | `setprecision(1000); pi(1000)` | `kind=Real value=3.14159...(exactly 100 digits)` | FINDING (silent cap: 1000 requested, 100 delivered, no diagnostic) | P2 |
| k21 | precision 0 | `setprecision(0); pi(10)` | `RC=1 DomainError "setprecision() expects a positive digit count, but got 0."` | HELD | — |
| k22 | negative digits | `pi(-1)` | `RC=1 InvalidArgument/TypeMismatch "Specified argument was out of the range of valid values. (Parameter 'digits')"` | FINDING (second raw CLR message; category is not a type mismatch) | P2 |
| k24 | changed flag, unchanged expression | `simplify_full(x^2 + 3*x)` | `status=Satisfied, original=x^2 + 3*x, expression=x^2 + 3*x, changed=true` | **FINDING** | P2 |
| k24b | changed flag, unchanged expression (control) | `simplify_full(sin(x))` | `original=sin(x), expression=sin(x), changed=false` | HELD (control: the flag is not simply always true) | — |
| k25 | --text mode | `--text` on `solve(x^2 - 4 == 0, x)` | 3 stdout lines: `= [-2, 2] (Vector)` / `  _ = [-2, 2]` / `  x = x` | HELD (human mode; the summary omits the solved variable name) | P3 |
| k26 | --stdin with no input | `--stdin` | `RC=0, no result key` | FINDING (same as k08) | P2 |
| pb01 | print-budget absent (baseline) | `(x+1)^8` | `pretty=(x + 1)^8 canonical=(pow (add (rat 1 1) (sym x)) (rat 8 1)) nodeCount=5` | HELD | — |
| pb02 | `--print-budget 1` | same value | truncated/budget/truncationReason absent; identical rendering | **FINDING** | **P1** |
| pb03 | `--print-budget 2` | same value | same | **FINDING** | **P1** |
| pb04 | `--print-budget 5` | same value | same (5 == nodeCount, still no truncation) | **FINDING** | **P1** |
| pb05 | `--print-budget 3` on a 98-node value | `expand((x + 1)^20)` | `truncated=true truncationReason=node-budget budget=3 pretty="x^20 + 20*x^19 + ... + 4845*x^ ..."` (a genuine prefix) | HELD | — |
| pb06 | `--print-budget 3` on a 15-node nested fraction | `1/(1 + 1/(1 + 1/(1 + 1/x)))` | `truncated=true, reason=node-budget, canonical="(pow (add (rat 1 1) (pow ( ..."` **but pretty is complete**: `(1 + (1 + (1 + x^-1)^-1)^-1)^-1` | FINDING (the documented prefix rule applies to canonical only) | P2 |
| pb07 | `--print-budget 12`, same value | same | canonical prefix longer, pretty still complete | FINDING (same) | P2 |
| pb08 | `--print-budget=3` (equals form) | `--print-budget=3` | `RC=2 "Unknown argument '--print-budget=3'."` | FINDING (only the spaced form works; equals form rejected) | P3 |

## 11. Stress, boundary and cancellation (st-series + depth, 21 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| st01 | deep nesting 3000 | `x = symbol("x"); ((((…x…))))` depth 3000 | `RC=0 kind=Symbolic pretty=x` | HELD | — |
| st02 | deep nesting 5000 | depth 5000 | `RC=3221225725 (0xC00000FD) **stdout empty**, stderr "Process is terminating due to StackOverflowException."` | **FINDING** | **P0** |
| st03 | deep nesting 8000 | depth 8000 | same | **FINDING** | **P0** |
| st04 | deep nesting 10000 | depth 10000 | same | **FINDING** | **P0** |
| st05 | deep nesting 12000 | depth 12000 | same | **FINDING** | **P0** |
| st06 | deep nesting 16000 | depth 16000 | same | **FINDING** | **P0** |
| st07 | deep nesting 20000 | depth 20000 | same | **FINDING** | **P0** |
| st08 | huge exponent, lazy | `x^100000000` | `RC=0 Symbolic pretty=x^100000000` | HELD | — |
| st09 | expand beyond the step budget | `expand((x+1)^2000)` | `RC=1 code=BudgetExceeded category=BudgetExceeded "Budget exceeded during 'expand'."` | HELD (budget is enforced and reported) | — |
| st10 | 2^10000000 | `2^10000000` | `RC=0 Natural, 3010300-digit value` | HELD | — |
| st11 | quartic the solver cannot do | `solve_full(x^4 - 3*x^3 + 2*x^2 - x + 7 == 0, x)` | `Unevaluated/Unknown` | HELD (honest) | — |
| st12 | degree-7 binomial | `solve_full(x^7 - 2 == 0, x)` | `Partial, represented 1, unrepresented 6` | HELD | — |
| st13 | rank-deficient 4×4 system | `solve_system_full([a+b+c+d==4, a-b==0, c-d==0, a+c==2],[a,b,c,d])` | `NoSolutions, complete=true, solutions=[]` — but (1,1,1,1) satisfies all four (SymPy: `{(2-d, 2-d, d, d)}`) | **FINDING** | **P0** |
| st14 | rank-deficient 3-unknown system | `solve_system_full([a+b+c==3, a+b==1],[a,b,c])` | `NoSolutions, complete=true` — (a,b,c)=(0,1,2) is a solution | **FINDING** | **P0** |
| st15 | empty system, no variables | `solve_system_full([], [])` | `Solved/Complete, solutions shape [1]` (vacuously correct) | HELD | — |
| st16 | syntax error, unterminated | `x = symbol("x"; x^2` | `RC=1 InvalidOperation/DomainError "Expected 'RParen' but found ';' at position 14."` | HELD (parse diagnostics carry position) | — |
| st17 | bad token | `x = symbol("x"); x @@ 2` | `RC=1 "Unexpected character '@' at position 19."` | HELD | — |
| cn01 | `--cancel-after 1` | `--cancel-after 1` on `expand((x+1)^4000)` | `RC=1 code=Cancelled category=BudgetExceeded "the evaluation was cancelled by the caller."; envelope carries `partialOutput:[]`, `partialVariables:[…]`` | FINDING (the CLI help promises "returning the partial result"; no `result` is returned) | P2 |
| cn02 | `--cancel-after 5` | as above | same | FINDING | P2 |
| cn03 | `--cancel-after 25` | as above | same | FINDING | P2 |
| dep01–dep07 | crash boundary | depths 3000 / 5000 / 8000 / 10000 / 12000 / 16000 / 20000 | RC=0 at 3000; RC=3221225725 with empty stdout at every depth ≥ 5000 | **FINDING** (boundary bracketed, not bisected) | **P0** |

## 12. The exact-decimal tower vs. its own output (F-1/17 evidence, 9 probes)

| id | what was probed | command (script file) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| eps01 | the value | `1/17` | `Real 0.(0588235294117647)` — a notation that asserts the *exact* repeating decimal | evidence | P0 |
| eps02 | the product | `(1/17)*17` | `Real 0.999…9984` (1002 characters); `1 - value = 1.6e-999` | evidence | P0 |
| eps03 | the identity | `((1/17)*17) == 1` | `Boolean false` (SymPy: `Rational(1,17)*17 = 1`) | **FINDING** | **P0** |
| eps04 | the residual | `(1/17)*17 - 1` | `Real -0.000…(≈1000 zeros)…` (1003 characters) | evidence | P0 |
| eps05 | which denominators break | `(1/n)*n == 1` for n∈{3,6,7,9,11,13,17,19,23,29,97,101,999}` | false for n ∈ {17, 23, 97}; true for all others tested | evidence (systematic, not a single typo) | P0 |
| eps06 | control | `(1/17 + 1/17) == 2/17` | `Boolean true` | HELD | — |
| eps07 | control | `2/17 == 2*(1/17)` | `Boolean true` | HELD | — |
| eps08 | control | `(1/17)*17 == 17/17` | `Boolean false` | evidence | P0 |
| eps09 | control | `1/17 - 1/17` | `Real 0` | HELD | — |

## FINDINGS

Each finding is one sentence with its exact reproduction. Every reproduction was executed **twice from a fresh script file**; the two envelopes are identical apart from `elapsed`/`revision`, and the `RUN2` envelope is pasted.

### F1 (P0) — Extraneous roots: the real-domain solver returns values that do not satisfy the equation, and calls the answer complete.

Reproduction: script `x = symbol("x"); solve_full(sqrt(x) + 2 == 0, x, real)`.

`RUN2` envelope (decisive fields verbatim; the `solutions` value is `(rat 4 1)`):

```
{"ok":true,"result":{"kind":"Record","display":"SolveResult(status: Solved, variable: x, domain: real, complete: True, completeness: Complete, solutions: [Solution(value: 4, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])","structured":{"kind":"Record","type":"SolveResult","fields":[
 {"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Solved"}},
 {"name":"variable","value":{"kind":"Symbolic","pretty":"x","canonical":"(sym x)"}},
 {"name":"domain","value":{"kind":"Domain","domain":"real"}},
 {"name":"complete","value":{"kind":"Boolean","value":"true"}},
 {"name":"completeness","value":{"kind":"Enum","type":"Completeness","value":"Complete"}},
 {"name":"solutions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Record","type":"Solution","fields":[
    {"name":"value","value":{"kind":"Symbolic","pretty":"4","canonical":"(rat 4 1)"}},
    {"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
    {"name":"multiplicity","value":{"kind":"Integer","value":"1"}},
    {"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Exact"}}]}]}},
 {"name":"represented_count","value":{"kind":"Integer","value":"1"}},
 {"name":"unrepresented_count","value":{"kind":"Integer","value":"0"}},
 {"name":"unrepresented_reason","value":{"kind":"Text","value":""}},
 {"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}},"elapsed":"…"}
```

Why it is false: the kernel's own arithmetic disagrees — `x = symbol("x"); subs(sqrt(x) + 2, x, 4)` returns `(rat 4 1)`, not 0; SymPy 1.14.0 gives `solveset(Eq(sqrt(x)+2,0), x, S.Reals) = EmptySet` and `solve(...) = []`. Over ℝ the square root is non-negative, so the solution set is provably empty and the returned `4` is not a solution. `complete: true` / `completeness: Complete` is therefore also false (the *documented* meaning of `Solved` + `Complete` is a complete, correct solution set).

Two further instances of the same defect, both reproduced twice:

* `x = symbol("x"); solve_full(sqrt(x) == -2, x)` → `Solved, complete=true, completeness=Complete, solutions=[Solution(value: 4, …)]`; `subs(sqrt(x) - (-2), x, 4)` = `(rat 4 1)`. (SymPy: `[]`.)
* `x = symbol("x"); solve_full(sqrt(2*x + 3) == x, x)` → `Solved, complete=true, … solutions: [Solution(value: -1, …), Solution(value: 3, …)]`; `subs(sqrt(2*x + 3) - x, x, -1)` = `(rat 2 1)` (the other root, 3, does satisfy it: `(rat 0 1)`). SymPy: `{3}`.

Root cause (inference?): the solver appears to rationalise the radical, solve the polynomial, and never substitute back — which is exactly the "extraneous root" failure mode the contract's `exactness: Exact` and `complete: true` fields rule out for a consumer.

### F2 (P0) — A consistent but under-determined linear system is reported as `NoSolutions` with `complete: true`.

Reproduction: script `x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 3], [x, y])`.

`RUN2` envelope (decisive fields verbatim; the whole result is one record):

```
{"kind":"Record","type":"SystemSolveResult","fields":[
 {"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"NoSolutions"}},
 {"name":"domain","value":{"kind":"Domain","domain":"complex"}},
 {"name":"complete","value":{"kind":"Boolean","value":"true"}},
 {"name":"solutions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
 {"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}
```

Why it is false: `(x,y) = (0,3)` satisfies the equation — the kernel's own `subs(subs(x + y, x, 0), y, 3)` returns `(rat 3 1)` — and SymPy's `linsolve([x+y-3],[x,y])` is the 1-parameter family `{(3 - y, y)}`. `NoSolutions` + `complete: true` asserts "the solution set over the requested domain is provably empty" (dsh-protocol.md), which is false.

Two further instances, both reproduced twice:

* `solve_system_full([x + y == 1, 2*x + 2*y == 2], [x, y])` → `NoSolutions, complete=true, solutions=[]`, while `(1,0)` satisfies both equations (`subs`-verified) and SymPy gives `{(1 - y, y)}`.
* `a=symbol("a"); b=symbol("b"); c=symbol("c"); d=symbol("d"); solve_system_full([a+b+c+d==4, a-b==0, c-d==0, a+c==2],[a,b,c,d])` → `NoSolutions, complete=true, solutions=[]`, while `(1,1,1,1)` satisfies all four equations and SymPy gives `{(2 - d, 2 - d, d, d)}`. (The 4th equation is twice the sum of the others, so the system has a 1-parameter solution family.)

The kernel cannot distinguish "inconsistent" from "under-determined": `solve_system_full([x + y == 3, x + y == 4], [x, y])` (genuinely empty) returns the identical envelope. A consumer that branches on `(status, complete)` gets a wrong answer for every rank-deficient system.

### F3 (P0) — `((1/17)*17) == 1` is `false`, and the printed value is not the product of the printed operands.

Reproduction: script `((1/17)*17) == 1`.

`RUN2` envelope (verbatim):

```
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Boolean","display":"False","typed":"False (Boolean)","structured":{"kind":"Boolean","value":"false"}},"output":[],"variables":[{"name":"_","kind":"Boolean","display":"False"}],"functions":[],"elapsed":"336.84 ms","elapsedTime":{"value":336.84,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":336.8,"unit":"ms"},"resultKind":"Boolean","hasOutput":false}]}
```

Supporting values from the same two runs: `1/17` returns `Real value=0.(0588235294117647)` — a display that asserts the exact repeating decimal — while `(1/17)*17` returns `Real value=0.999…(1000 digits)…9984` (1002 characters; `1 - value = 1.6e-999`) and `(1/17)*17 - 1` returns a 1003-character `Real` beginning `-0.000…`. SymPy: `Rational(1,17)*17 = 1` exactly. The defect is systematic, not a single value: `(1/n)*n == 1` is false for n ∈ {17, 23, 97} and true for n ∈ {3, 6, 7, 9, 11, 13, 19, 29, 101, 999} (controls `3*(1/3)` = `1`, `(1/7)*7 == 1` = `true`).

Note the two readings, both bad: if `Real` is an exact decimal/rational type (as `0.(…)`, `1/3 == 0.333333333333333333333333333333` → `false`, and `1 - 0.99…9` → `10⁻⁴⁰` all suggest), the product is simply wrong; if it is an approximating type, then printing `0.(0588235294117647)` for it is a false exactness claim.

### F4 (P0) — A syntactically valid, well-formed script kills the process with a stack overflow and no envelope.

Reproduction: script `x = symbol("x"); ` followed by 5000 `(`, then `x`, then 5000 `)`.

`RUN2`: `RC=3221225725 (0xC00000FD STATUS_STACK_OVERFLOW)`, **stdout empty** (no envelope, no diagnostic), stderr one line:

```
Process is terminating due to StackOverflowException.
```

Depth 3000 returns `RC=0 kind=Symbolic pretty=x`; depths 5000, 8000, 10000, 12000, 16000, 20000 all reproduce the crash (twice each). dsh-protocol.md advertises a `depth-limit` truncation reason and a `--print-budget` guard, and this parser has neither: an ordinary nesting depth kills the process before any envelope can be written. Invariant 6 ("Errors are structural") cannot be honoured on this input.

### F5 (P0) — Two indeterminate-power limits are answered `1` where the limit is `e`.

Reproductions: `x = symbol("x"); limit_full((1 + 1/x)^x, x, inf())` and `x = symbol("x"); limit_full((1 + x)^(1/x), x, 0)`.

`RUN2` envelope of the second (first differs only in the script):

```
{"ok":true,"result":{"kind":"Record","display":"LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: [])","structured":{"kind":"Record","type":"LimitResult","fields":[
 {"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"Value"}},
 {"name":"exists","value":{"kind":"Boolean","value":"true"}},
 {"name":"value","value":{"kind":"Symbolic","pretty":"1","canonical":"(rat 1 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}},
 {"name":"left","value":{"kind":"Null"}}, {"name":"right","value":{"kind":"Null"}},
 {"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
 {"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Exact"}},
 {"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}},"elapsed":"…"}
```

Why it is false: SymPy 1.14.0 gives `limit((1+1/x)**x, x, oo) = E` and `limit((1+x)**(1/x), x, 0) = E` (`2.7182818284590452354`). The kernel not only returns the wrong value, it asserts `exists: true`, `status: Value` and `exactness: Exact` — the strongest possible claim — for the `1^∞` form. The same engine gets `lim sin(x)/x = 1`, `lim (1-cos x)/x² = 1/2` and `lim x·log x = 0` right, so this is a specific hole in the power path, not a general "limits are not implemented" state.

### F6 (P0) — `exact` is `false` for exact closed forms (`sin(x)`, `cos(x)`, `sqrt(x)`, `log(x)`, `exp(x)`, `diff(sin(x),x)` = `cos(x)`) while it is `true` for `abs(x)` and `x + 1`.

Reproduction: `x = symbol("x"); diff(sin(x), x)`.

`RUN2` envelope (decisive fields verbatim):

```
{"kind":"Symbolic","display":"cos(x)","typed":"cos(x) (Symbolic)","structured":{"kind":"Symbolic","pretty":"cos(x)","canonical":"(fn cos (sym x))","domain":"complex","exact":false,"nodeCount":2,"freeSymbols":["x"]}}
```

The sweep is reproducible (`x` `x^2` `(1/2)*x` `x+1` `abs(x)` → `exact=true`; `sin(x)` `cos(x)` `sqrt(x)` `log(x)` `exp(x)` → `exact=false`), and the same `exact=false` travels with the derivative of `sin` and with `simplify(sqrt(x^2))`, while `diff(x^2,x)` = `2*x` is `exact=true`. `cos(x)` is an exact closed form; a consumer that treats `exact` as "this value is exact" is being told something false. *Judgment call:* dsh-protocol.md never defines `Symbolic.exact`, so if the field is meant as "the value may be an approximate numeric rendering" this drops to P1/P2 — but under the plain reading of the field name, and of the protocol example (`"exact": true` for `x^2+1`), `exact: false` for `cos(x)` is a wrong exactness claim.

### F7 (P0) — Five distinct printer outputs of values the kernel itself produces cannot be parsed back at all: the complex unit, RootOf, non-integer powers with a numeric base, the solution-family parameter, and the piecewise form (10 probe rows fail on them).

Reproduction (one script per case, each run twice): `i`, `rootof(x^4 - x^2 - 1, 0)`, `2^(1/3)`, `2^(1/3)*(-1/2*i*sqrt(3) - 1/2)`, `k*pi`, `piecewise(sign(x) if x != 0, diff(abs(x), x))`.

`RUN2` envelopes (verbatim, abridged only where marked):

```
$ script: i
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":true,"diagnostics":[{"message":"Undefined variable 'i'.","position":0,"line":1,"column":1}]}

$ script: rootof(x^4 - x^2 - 1, 0)
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unknown function 'rootof'.","recoverable":true,"diagnostics":[{"message":"Unknown function 'rootof'.","position":0,"line":1,"column":1}]}

$ script: 2^(1/3)
{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true,"diagnostics":[{"message":"Non-integer exponents are not yet supported.","position":0,"line":1,"column":1}]}

$ script: k*pi            (template of the sin(x)=0 family)
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'k'.","recoverable":true,"diagnostics":[{"message":"Undefined variable 'k'.","position":0,"line":1,"column":1}]}

$ script: piecewise(sign(x) if x != 0, diff(abs(x), x))     (pretty of diff(abs(x),x))
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Expected 'RParen' but found 'if' at position 60.","recoverable":true,"diagnostics":[{"message":"Expected 'RParen' but found 'if' at position 60.","position":0,"line":1,"column":1}]}
```

These are exactly the `pretty` strings the kernel emits inside `Symbolic` values: `solve_full(x^2 + 1 == 0, x)` returns `pretty: "i"`; `solve_full(x^4 - x^2 - 1 == 0, x, real)` returns `pretty: "rootof(x^4 - x^2 - 1, 0)"`; `solve_full(x^3 - 2 == 0, x)` returns `pretty: "2^(1/3)"`; `solve_full(sin(x) == 0, x)` returns the family template `pretty: "k*pi"`; `diff(abs(x), x)` returns `pretty: "piecewise(sign(x) if x != 0, diff(abs(x), x))"`. "parse(pretty(e)) canonicalises identically to e" fails for every one of them — not with a different canonical form but with a hard error, because `i`, `rootof`, `k` and `piecewise` are not part of the language surface the printer prints for.

### F8 (P1) — `x/y/x` prints as `x/(x*y)`, which re-parses to a *different canonical form*.

Reproduction: script `x = symbol("x"); y = symbol("y"); x/y/x` → `canonical=(mul (sym x) (pow (sym x) (rat -1 1)) (pow (sym y) (rat -1 1))) pretty="x/(x*y)"`; re-parsing `x/(x*y)` gives `canonical=(mul (sym x) (pow (mul (sym x) (sym y)) (rat -1 1)))`.

`RUN2` of the first script (verbatim):

```
{"ok":true,"result":{"kind":"Symbolic","display":"x/(x*y)","typed":"x/(x*y) (Symbolic)","structured":{"kind":"Symbolic","pretty":"x/(x*y)","canonical":"(mul (sym x) (pow (sym x) (rat -1 1)) (pow (sym y) (rat -1 1)))","domain":"complex","exact":true,"nodeCount":8,"freeSymbols":["x","y"]}},"elapsed":"…"}
```

The mathematics is right (`x/y/x` = `x/(x·y)` — SymPy: `simplify(x/y/x - x/(x*y)) = 0`), so no *value* is wrong; the violation is of the round-trip/canonical-identity property: two texts that mean the same thing do not canonicalise to the same s-expression, and the canonical form of the source keeps `x·x⁻¹` uncollected while the canonical form of its own printed image does not.

### F9 (P1) — The printed form of an exact algebraic solution re-parses as an inexact `Real`.

Reproduction: `x = symbol("x"); r = solve_full(x^2 - 2 == 0, x, real); inspect(r.solutions[0].value)` returns `{"kind":"Symbolic","pretty":"-1/2*sqrt(8)","canonical":"(mul (rat -1 2) (pow (rat 8 1) (rat 1 2)))","exactness":"AlgebraicExact"}` for the value inside the record, but the *same text* written by a script (`x = symbol("x"); -1/2*sqrt(8)`) evaluates to `kind=Real value=-1.4142135623730951…`.

`RUN2` of the re-parse script: `inspect(-1/2*sqrt(8))` returns `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Real"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}}, …]}` — i.e. the exactness is lost, and the `exactness: AlgebraicExact` label on the original solution cannot be reconstructed from its printed text. (The same mechanism makes `sqrt(2)` alone a `Real`: the printer and the parser disagree about whether `sqrt` of a numeric literal stays symbolic.)

### F10 (P1) — `--print-budget` is a documented no-op for expressions whose node count is modest, and truncates `canonical` without truncating `pretty`.

Reproductions: (a) `x = symbol("x"); (x + 1)^8` with `--print-budget 1` (and 2, 3, 5, 20); (b) `x = symbol("x"); 1/(1 + 1/(1 + 1/(1 + 1/x)))` with `--print-budget 3`.

(a) `RUN2` — the envelope is byte-identical to the run without the flag (only `elapsed`/`revision` differ), and no `truncated`, `truncationReason` or `budget` field is present:

```
{"kind":"Symbolic","display":"(x \u002B 1)^8","structured":{"kind":"Symbolic","pretty":"(x \u002B 1)^8","canonical":"(pow (add (rat 1 1) (sym x)) (rat 8 1))","domain":"complex","exact":true,"nodeCount":5,"freeSymbols":["x"]}}
```

nodeCount `5` exceeds the requested budget `1`, so dsh-protocol.md invariant 4 requires a ` …`-terminated prefix plus `truncated: true`/`truncationReason`/`budget`; none is produced. (b) With a 15-node value the field does appear — `truncated=true, truncationReason="node-budget", budget=3`, and `canonical` is a genuine prefix ending in `…` — but `pretty` is the **complete** `(1 + (1 + (1 + x^-1)^-1)^-1)^-1`, contradicting the same invariant's "`pretty`/`canonical` carry a ` …`-terminated prefix".

### F11 (P1) — `solve_system`'s short form returns the solution as display text.

Reproduction: `x = symbol("x"); y = symbol("y"); solve_system([x + y == 3, x - y == 1], [x, y])`.

`RUN2` envelope (verbatim, `elapsed` abridged):

```
{"ok":true,"result":{"kind":"Text","display":"x = 2, y = 1","typed":"x = 2, y = 1","structured":{"kind":"Text","value":"x = 2, y = 1"}},"output":[],"variables":[{"name":"_","kind":"Text","display":"x = 2, y = 1"},…],"elapsed":"…"}
```

The whole result is a `Text` value: recovering which variable was bound to what requires parsing "`x = 2, y = 1`", which is exactly what dsh-protocol.md's opening promise ("designed so that an agent never has to parse a display string to recover mathematical meaning") excludes. The contrast with `solve`, whose short form returns a structural `Vector` of `Symbolic` values (s02), shows this is a degradation rather than a documented convenience; the structured form exists only via `solve_system_full`.

### F12 (P1) — `solve_system_full([], [x])` fails with a leaked CLR index exception.

Reproduction: `x = symbol("x"); solve_system_full([], [x])`.

`RUN2` envelope (verbatim):

```
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')","recoverable":true,"diagnostics":[{"message":"Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')","position":0,"line":1,"column":1}],"elapsed":"333.7 \u00B5s"}
```

Nothing at the call site is a type mismatch: both arguments are well-typed (`[]` is a `Vector`, `[x]` is a `Vector` of symbols). The kernel's own empty-*variable*-list case is handled gracefully (`solve_system_full([x == 1], [])` → `status=Unevaluated` with a `system-…` diagnostic), so the empty-*equation*-list path is an unhandled index invariant surfacing as raw .NET text. A second instance of a raw CLR message is `pi(-1)` → `"Specified argument was out of the range of valid values. (Parameter 'digits')"`.

### F13 (P1) — `changed: true` is reported for a `simplify_full` that returns the input unchanged.

Reproduction: `x = symbol("x"); simplify_full(x^2 + 3*x)`.

`RUN2` (deciding fields): `status=Satisfied, original=x^2 + 3*x, expression=x^2 + 3*x, changed=true, conditions=[], steps=[]`.

`simplify_full(sin(x))` (control, same two runs) returns `original=sin(x), expression=sin(x), changed=false`, so the flag is not merely always-true — it is wrong for `x^2 + 3*x` and for `(x^2-1)/(x-1)` (`original` and `expression` are the same value in both cases), which is a machine-visible field stating something false about the transform.

## FINDINGS (continued)

### F14 (P2) — Indeterminate and infinite special values are returned as unevaluated symbolic nodes, and `==` on them returns an equation node instead of a Boolean.

Reproductions (each run twice): `inf() - inf()` and `0*inf()` → `canonical=(mul (rat 0 1) (inf)) pretty=0*inf exact=false`; `exp(inf())` → `(fn exp (inf))`; `abs(-inf())` → `(fn abs (mul (rat -1 1) (inf)))`; `inf() == inf()` → `canonical=(eq (inf) (inf)) pretty="inf = inf"`; `log(0) == -inf()` → `(eq (fn log (rat 0 1)) (mul (rat -1 1) (inf)))`.

`RUN2` of `inf() == inf()` (verbatim, decisive field): `{"kind":"Symbolic","display":"inf = inf","typed":"inf = inf (Symbolic)","structured":{"kind":"Symbolic","pretty":"inf = inf","canonical":"(eq (inf) (inf))","domain":"complex","exact":false,"nodeCount":3,"freeSymbols":[]}}`.

Nothing here is arithmetically *false* (the indeterminate forms are left alone rather than guessed), but the surface is inconsistent: `1 == 1` yields a `Boolean` while `inf() == inf()` yields a `Symbolic` equation, so a consumer switching on the result kind gets two different types for the same operator with two infinite operands. A related inconsistency: `log(0)` is accepted as a symbolic value (`(fn log (rat 0 1))`) but `evalf(log(0), 20)` fails with `EvaluationError/DomainError "Ln(x) requires x > 0. (Parameter 'x')"` — the same input is legal or illegal depending on which entry point asks.

### F15 (P2) — The same equation is described with two different solution shapes depending on how it is written.

Reproductions: `solve_full((x - 1)^2 == 0, x)` → `solutions=[Solution(value 1, multiplicity 1), Solution(value 1, multiplicity 1)], represented_count=2, unrepresented_count=0`; `solve_full(x^2 - 2*x + 1 == 0, x)` → `solutions=[Solution(value 1, multiplicity 2)], represented_count=1`. `(x-1)^2*(x-2) == 0` → `Unevaluated` while `x^3 - 4*x^2 + 5*x - 2 == 0` → `Solved, [Solution(1, mult 2), Solution(2, mult 1)]`.

The solution *sets* agree (both are {1}); the machine-visible `multiplicity` field and `represented_count` do not, and the unexpanded product form silently falls off the solver entirely. A consumer counting roots gets 2 or 1 for the same equation.

### F16 (P2) — The digit-count parameter is silently ignored or capped.

Reproductions: `evalf(sqrt(2), 5)` and `evalf(sqrt(2), 30)` return the identical 100-digit `Real` `1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727`; `setprecision(1000); pi(1000)` returns `3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679` — exactly 100 digits.

`setprecision` accepts `1000` without a diagnostic, `pi` accepts `1000` and returns 100 digits, and `evalf`'s `digits` argument changes nothing for a symbolic argument. (The digits that *are* returned are correct — mpmath agreement to the last shown digit — so this is a silent capability cap, not a wrong digit.) By contrast `pi(30)`, `e(30)` and `evalf(1/3, 30)` honour their arguments exactly, so the parameter is honoured in some paths and ignored in others.

### F17 (P2) — A script with no value produces an envelope with no `result` key at all (not `{"kind":"Null"}`).

Reproduction: `--eval ""` (also `--file` with an empty file, a whitespace-only file, `--stdin` with no input, and `print(1, 2, 3)` as the last statement) → `RC=0` and top-level keys `protocolVersion, symbolicFormatVersion, mathIrVersion, ok, revision, output, variables, functions, elapsed, elapsedTime, timings` — there is no `result` member.

dsh-protocol.md invariant 3 says "An absent field is `{"kind":"Null"}`, distinct from an empty string", and `timings[].resultKind` has a documented `Void` member for exactly this case, but the envelope gives the consumer a missing key instead of a structural null (`print` output is correctly captured in `output`).

### F18 (P2) — A missing script file is reported as a parse error with exit code 1.

Reproduction: `--file C:\nope\nope.ls` → `RC=1`, `{"ok":false,"code":"FileReadError","category":"ParseError","message":"Cannot read script file 'C:\nope\nope.ls': Could not find a part of the path 'C:\nope\nope.ls'."}` (reproduced twice).

Nothing was parsed, so `category: ParseError` (from the documented `ErrorCategory` taxonomy) misdescribes an I/O/usage condition, and dsh-protocol.md's exit-code table ("2 usage error") arguably covers it — as it stands a caller cannot distinguish "the script had a syntax error" from "the path was wrong" by category alone.

### F19 (P2) — `--cancel-after` returns an error envelope, not the partial result the flag documents.

Reproduction: `--cancel-after 5` on `x = symbol("x"); expand((x+1)^4000)`.

`RUN2` envelope (verbatim): `{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[],"elapsed":"16.62 ms","partialOutput":[],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x"}]}`.

`--help` documents the flag as "cancel the evaluation after the given time, **returning the partial result**"; the envelope is `ok:false` with no `result`, and the partial state lives in two undocumented top-level members (`partialOutput`, `partialVariables`). The `SolveStatus.BudgetExceeded` row of the documented `complete`/`completeness` mapping table is therefore still unexercised by any probe (see COULD NOT DECIDE).

### F20 (P2) — Two renderers print the same `Real` with different digit counts.

Reproduction: `latex(evalf(sqrt(2), 30))` returns a decimal with ≈500 significant digits (`1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727…`), while the `Real` value itself (and `evalf(sqrt(2), 30)`) displays 100 digits (`s13`). A consumer that compares a LaTeX rendering with a value rendering of the same number sees two different strings.

### F21 (P3) — Cosmetic rough edges.

* `--print-budget=3` (equals form) is rejected with exit 2 and `"Unknown argument '--print-budget=3'."` although the documented spelling is `--print-budget <nodes>` and every other flag in `--help` is likewise space-separated — worth noting only because the spaced form is the sole working spelling.
* `--text` prints `= [-2, 2] (Vector)` for `solve(x^2 - 4 == 0, x)` without naming the solved variable (the JSON envelope is unaffected).
* `limit_full(sin(x), x, inf())` reports `code=limit.unevaluated` with the message "coefficient does not evaluate at the point" — meaningless for `sin`, but byte-identical to the message `capabilities()` publishes for that trigger, so the contract is met and only the prose is odd.
* The task brief states "a newline is NOT a separator"; a newline-only separated script evaluates fine (`k07`). This is a brief/product mismatch in the product's favour, not a defect.

## COULD NOT DECIDE

1. **The meaning of `Symbolic.exact`.** Neither dsh-protocol.md nor `capabilities()` defines it. I could decide the *facts* (`exact=false` for `cos(x)`, `exact=true` for `abs(x)` and `x+1`, reproduced) but not whether the intended semantics makes that a false claim. If `exact` means "this value is an exact representation", F6 is P0 as stated; if it is a hint like `capabilities().exactness = BestEffort`, F6 degrades to P1/P2. **Missing evidence: a definition of the field** (contract text or a documented invariant).
2. **`SolveStatus.BudgetExceeded` and the documented `complete`/`completeness` mapping row.** dsh-protocol.md's table has a `BudgetExceeded` row ("`false` / the kernel's `Completeness` for the subset it stopped on"), but I could not produce any `solve`/`solve_system` call that returns `status=BudgetExceeded`: the step budget surfaces as a whole-envelope error (st09: `ok:false, code=BudgetExceeded`), and `--cancel-after` likewise returns an error envelope (F19). **Missing evidence: an input that makes the solver stop mid-search and report a subset**, or documentation that no such input exists in v1.
3. **`rootof` index convention beyond two real roots.** I verified that `rootof(x^4 - x^2 - 1, 0)` and `…, 1)` both satisfy the equation and evaluate (via `evalf`) to 1.2720196495140689642524 (one of the two real roots), and that SymPy's `RootOf` numbering agrees for this polynomial — but no probe produced a real-rooted polynomial with ≥3 real roots in `rootof` form, so the ordering rule of the index is unverified. **Missing evidence: a `solve_full` result whose `rootof` index must be ≥2 with all-real roots.**
4. **The exact crash threshold between depth 3000 (`RC=0`) and 5000 (`STATUS_STACK_OVERFLOW`).** Both ends are reproduced twice, but I did not bisect, so the exact maximum nesting depth is unknown. **Missing evidence: a bisection run** (cheap, ~7 executions).

Also noted, deliberately not scored: SymPy returns `oo` for `limit(1/x, x, 0)` while the kernel returns `DoesNotExist` — I judged the kernel correct (the two-sided limit does not exist) and SymPy's answer a convention of its default direction; and `evalf`/`pi` digit strings were checked against mpmath and are correct to every digit shown.

## 14. Summary

**Probe counts.** 375 probe inputs were executed across 363 table rows; each input was run twice from a clean temporary script file and the two envelopes were compared (all stable apart from `elapsed`/`revision`). Of the rows, **263 HELD** (the product is right), **82 are FINDING rows** resolving to **21 distinct findings** (**7 P0**, **6 P1**, **7 P2**, **1 P3** grouping three cosmetic items), **9 are evidence rows** recording the kernel's own substitutions used to prove a finding, **6 are N/A** (the expression evaluates to a number, so there is no pretty form to round-trip), **1 is a note**, **4 matters are undecided** (listed above in COULD NOT DECIDE), and **2 probes were discarded as authoring errors** (`r38`, `L29` — both declared an undefined variable and both got the kernel's correct "Undefined variable" error). The strongest areas are the printer *except* for values it cannot re-parse, the LaTeX renderer (28/28 correct), the rewrite/transform path (which correctly carries `conditions=[x != 0]` when it cancels `x/x` → `1`), the error taxonomy and arity validation, and the many "honest gap" answers (`Unevaluated` rather than a fabricated root) — `solve` correctly refuses `sqrt(x+1) = x-1`, `abs(x) = 2`, `2^x = 8` and `x = x`.

**The kernel cannot be trusted for solving.** Four independent P0s put wrong mathematics on the machine-visible surface: radical equations return extraneous roots with `complete: true` and `exactness: Exact` (`sqrt(x)+2 = 0` over ℝ → `x = 4`, which the kernel's own `subs` evaluates to 4, not 0); rank-deficient but consistent linear systems are reported as `NoSolutions` with `complete: true` (`x+y = 3` in two unknowns — the kernel's own substitution confirms `(0,3)` is a solution); `((1/17)*17) == 1` is `false` while `1/17` is displayed as the exact repeating decimal `0.(0588235294117647)`; and `limit_full((1+1/x)^x, x, inf())` and `limit_full((1+x)^(1/x), x, 0)` return `1` with `exists: true, exactness: Exact` where the limit is `e` = 2.7182818284590452354. **The printer cannot be trusted as a serialization of values:** its own pretty forms for `i`, `rootof(…)`, `2^(1/3)`, the family template `k*pi` and the piecewise derivative are rejected by its own parser (five distinct hard failures), `x/y/x` round-trips to a different canonical form, and the printed form of an `AlgebraicExact` root re-parses as an inexact `Real`. **The envelope itself has a hole:** 5000-deep nesting kills the process with a .NET stack overflow and an empty stdout — no envelope, no diagnostic, on syntactically valid input — and `--print-budget`, advertised in the protocol for exactly this kind of protection, is a no-op for a 5-node expression. Against that, the audit found no wrong digit in any printed constant (√2, π, e, and the 1000-digit decimal product were all checked against mpmath), no wrong LaTeX in 28 renderings, and no wrong derivative, antiderivative or limit outside the `1^∞` family.
