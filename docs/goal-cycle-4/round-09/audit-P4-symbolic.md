# Audit P4 — Symbolic-Correctness Attacker (adversarial falsification)

**Persona:** P4, symbolic correctness (factoring, printer, complex closed forms, complex log).
**Target:** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
**Provenance:** size 5 673 984 B, mtime 2026-09-11 12:41:36, SHA256 `B9C2D132204ED55C5CEE0A2949E00E56FF39614E89C79542D51D53D41C642894`, envelope `revision: 82`, `mathIrVersion: 2`.
**Oracle:** SymPy 1.14.0 (`C:\Users\ricar\dev\.lovelace-tools\python\python3.exe`), reported by `import sympy; print(sympy.__version__)` → `1.14.0`.

## How every command below was run

* **EVAL** — script piped to stdin:
  `'<script>' | & "C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe" --stdin --omit-functions`
  Output is the raw JSON envelope. `--stdin` was used to batch probes; byte-identical envelopes to the
  task's `--file <script.ls> --omit-functions` form were verified on the headline reproduction pair
  (`x = symbol("x"); -1^x` and `x = symbol("x"); (-1)^x`), shown in **F1**.
* **SYMPY** — program piped to stdin of the oracle interpreter:
  `@'...'@ | & "C:\Users\ricar\dev\.lovelace-tools\python\python3.exe" -`
* Equality of two symbolic results is judged by the envelope's `result.structured.canonical` string
  (the machine form the API exposes). Numeric spot checks use
  `evalf(subs(subs(subs(E, x, ..), y, ..), a, ..), N)`.

## Verdict summary

| verdict | count |
|---|---|
| HELD | 45 |
| FINDING | 12 |
| INCONCLUSIVE | 2 |
| **rows total** | **59** |

Twelve FINDING rows, covering **nine distinct defects (F1–F9)**; F3 has a sub-case (F3b).
**F1 is a P0 printer defect: a rendering that parses to a different value.** **F4 is a P0 solve
over-claim: `Solved` + `complete: true` with a root that does not satisfy the equation.**

---

## Probe table

### Property 1 — `expand(factor(p)) == p` for hostile polynomials

Reference = canonical of `expand(p)`; test = canonical of `expand(factor(p))`. Rows marked
“45 random” compare the two canonical strings for 45 pseudo-random polynomials (deg 2–7, coefficients
drawn from `{1,-1,2,-2,3,-3,1/2,-1/2,2/3,-5/7,5,-4,7/3}`).

| probe | script (`x = symbol("x");` prefix) | observed | expected | verdict |
|---|---|---|---|---|
| P1.1 negative leading coefficient | `expand(factor(-x^3 + x))` vs `expand(-x^3 + x)`; also `-2*x^2 + 2` | `(add (sym x) (mul (rat -1 1) (pow (sym x) (rat 3 1))))` both sides | identical | HELD |
| P1.2 negative leading + content | `expand(factor(-2*x^2 + 2))` | `(add (rat 2 1) (mul (rat -2 1) (pow (sym x) (rat 2 1))))` both sides | identical | HELD |
| P1.3 repeated roots | `expand(factor(x^3 - 3*x + 2))` (canonical of `factor` = `(mul (pow (add (rat -1 1) (sym x)) (rat 2 1)) (add (rat 2 1) (sym x)))`, pretty `(x - 1)^2*(x + 2)`); `x^4 - 2*x^3 + 2*x^2 - 2*x + 1` | both sides identical to `expand(p)` | identical | HELD |
| P1.4 rational coefficients | `x^2/2 - 1/3`; `(2/3)*x^2 - (5/7)*x + 1/11`; `(1/2)*x^4 - 1/2` | identical canonical both sides | identical | HELD |
| P1.5 high degree | `x^8 - 1`; `x^6 - 1`; `x^10 - x`; `-x^12 + x^10`; `(x^2 - 1)^4*(x^2 + 1)^2` (deg 12) | identical canonical both sides | identical | HELD |
| P1.6 constants and zero | `expand(factor(5))`, `expand(factor(-7))`, `expand(factor(0))` | `(rat 5 1)`, `(rat -7 1)`, `(rat 0 1)` equal to `expand(p)` | identical | HELD |
| P1.7 already irreducible | `x^2 + 1` (factor returns it unchanged), `x^4 + x + 1` | identical canonical both sides | identical | HELD |
| P1.8 45 random rational polynomials | see method above | `tested=45 mismatches=0 errs=0` | 0 mismatches | HELD |
| P1.9 structured factored inputs | `(x-1)^2*(x+2)^3`, `(x^2+1)^3`, `-3*(x-1)^4`, `(2*x-3)^2*(3*x+1)`, `(x^2-2)*(x^2-3)`, `(x-1)^3*(x+1)^3`, `x^3*(x-1)^2`, `-(x^2-1)^3`, `(x^2+x+1)^2*(x-1)`, `-(2*x+1)^2*(x^2+2)^2` | all HELD | identical | HELD |

### Property 2 — `parse(pretty(e))` must canonicalise identically to `e`

| probe | script | observed | expected | verdict |
|---|---|---|---|---|
| P2.1 nested subtraction | `x - (y - a)`; `x - (y + a)`; `a - (b - (x - y))`; `-(x - (y - a))` | pretty `x - (y - a)`, `x - (a + y)`, `a - (b - (x - y))`, `-(x - (y - a))`; reparse canonical identical | identical canonical | HELD |
| P2.2 nested division | `x/(y/z)`; `a/(b/(x/y))`; `x/(y*a)`; `(x - y)/a` | pretty `x/(y/z)`, `a/(b/(x/y))`, `x/(a*y)`, `(x - y)/a`; identical | identical canonical | HELD |
| P2.3 division by a product | `(x/y)/z`; `x/y/a`; `2/x/y`; `x^(-1)/y` | pretty `x/(y*z)`, `x/(a*y)`, `2/(x*y)`, `1/(x*y)`; reparse canonical `(mul x (pow (mul y z) (rat -1 1)))` ≠ `(mul x (pow y (rat -1 1)) (pow z (rat -1 1)))` | identical canonical | **FINDING (F5)** |
| P2.4 negative powers | `x^(-2)`; `x^(-1/2)`; `(x + y)^(-2)`; `1/(x*y)`; `(x*y)^-1` | pretty `x^-2`, `1/sqrt(x)`, `(x + y)^-2`, `(x*y)^-1`, `(x*y)^-1`; identical | identical canonical | HELD |
| P2.5 fractional powers, symbolic base | `x^(1/2)`; `x^(2/3)`; `x^(-y)`; `x^(y^z)`; `(x^y)^z` | `sqrt(x)`, `x^(2/3)`, `x^(-y)`, `x^(y^z)`, `(x^y)^z`; identical | identical canonical | HELD |
| P2.6 function composition | `sin(cos(x))`; `sin(-x)`; `-sin(x) + cos(-x)` | `sin(cos(x))`, `sin(-x)`, `cos(-x) - sin(x)`; identical | identical canonical | HELD |
| P2.7 unary minus in every position (16 cases) | `-x^2`, `(-x)^2`, `-(x + y)`, `-(x*y)`, `x - -y`, `-(x - y)`, `x*(-y)`, `-(-(-x))`, `-(-x)^2`, `-(-x)^(-2)`, `(-1)*x`, `x*(-1)`, `1/(-x)`, `(-x)/y`, `-2*x`, `2*(-x)` | pretty `-x^2`, `(-x)^2`, `-(x + y)`, `-x*y`, `x + y`, `-(x - y)`, `-x*y`, `-x`, `-(-x)^2`, `-(-x)^-2`, `-x`, `-x`, `(-x)^-1`, `-x/y`, `-2*x`, `-2*x`; all reparse identical | identical canonical | HELD |
| P2.8 negative **numeric** base of a power | `(-1)^x`; `(-2)^x`; `(-1/2)^x`; `(-3)^x`; `(-1)^(x+1)`; `(1-2)^x`; `(0-2)^x`; `1/((-2)^x)` | pretty `-1^x`, `-2^x`, `-1/2^x`, `-3^x`, `-1^(x + 1)`, `-1^x`, `-2^x`, `(-2^x)^-1`; reparsing yields a different canonical (`(-1)^x` → `(rat -1 1)`) and a different value | identical canonical and value | **FINDING (F1)** |
| P2.9 imaginary unit | `sqrt(-1)`; `sqrt(-4)`; `solve_full(x^4 - 1, x)` solutions | pretty `i`, `2*i`, `i`, `-i`; reparse `i` → `DomainError: Undefined variable 'i'.` | rendering must be re-readable | **FINDING (F2)** |
| P2.10 numeric base with rational exponent | `solve_full(x^3 - 2, x)` solution values `2^(1/3)`, `2^(1/3)*(-1/2*i*sqrt(3) - 1/2)` | reparse `2^(1/3)` → `UnsupportedOperation: Non-integer exponents are not yet supported.` | rendering must be re-readable | **FINDING (F3)** |
| P2.11 `rootof` rendering | `solve_full(x^4 - 2*x^2 - 3, x)` solution values `rootof(x^4 - 2*x^2 - 3, 0)` | reparse → `DomainError: Unknown function 'rootof'.` | rendering must be re-readable | **FINDING (F3)** |
| P2.12 π-containing expressions | `pi*x` | pretty is 1004 chars (`3.14159…(500 digits)*x`); reparse canonical is byte-identical to the original | identical canonical | HELD (π is never rendered as `pi`, but the decimal round-trips) |
| P2.13 35-expression fuzz | depth-3 random expressions over `{x,y,a,1,2,3,-1,1/2,2/3,4}` with `+ - * / ^` and `sqrt log sin cos exp abs` | `tested=35 held=29 canonical_mismatch=6 reparse_err=0`; the 6 decompose as 3× the F5 division-of-product class, 1× the F1 value-changing class (`((-(-1)^(2/1))^(-3/cos(x)))`), 1× real-coefficient folding (rejected as rounding, P2.14), 1× harness false positive (`(((1^x)^(2-2))^((3-1)/(4-a)))` prints `1`; EVAL of `1` yields a non-Symbolic result with no `canonical` field, so no comparison exists); 0 new defect classes | 35 held | **FINDING (same classes as F1/F5)** |
| P2.14 apparent value difference from the fuzz | `(((-1 / 1/2) / log(-1)) * (abs(y) + (4 ^ a)))` vs its rendering `-1*(abs(y) + 4^a)/(2*log(-1))` | difference at evalf 20 = `5.135e-18 i`, at 40 = `5.135e-38 i`, at 60 = `0` | scales with requested precision ⇒ rounding, not a value defect | HELD (false positive rejected) |

### Property 3 — `solve_full` completeness claims

`SF` below is the compact projection of `result.structured.fields`
(`status / domain / complete / represented_count / solutions[value,multiplicity,exactness] / diagnostics`).

| probe | script | observed | expected (SymPy) | verdict |
|---|---|---|---|---|
| P3.1 negative discriminant, complex domain | `solve_full(x^2 + 2*x + 5, x)`; `solve_full(x^2 - 2*x + 10, x)` | Solved/complete=true, `[1/2*(-4*i - 2), 1/2*(4*i - 2)]`, `[1/2*(2 - 6*i), 1/2*(2 + 6*i)]` | `[-1 - 2*I, -1 + 2*I]`; `[1 - 3*I, 1 + 3*I]` | HELD |
| P3.2 empty over the named real domain | `solve_full(x^2 + 1, x, real)`; `solve_full(x^2 + x + 1, x, real)`; `solve_full(x^4 + 1, x, real)` | `status=NoSolutions domain=real complete=true n=0`, diag `solve.no-solutions` | `solveset(..., S.Reals)` = `EmptySet` (all three) | HELD |
| P3.3 quartic, 4 complex roots | `solve_full(x^4 - 1, x)` | Solved/complete=true, `[i, -i, -1, 1]` | `[-1, 1, -I, I]` | HELD |
| P3.4 quartic over complex it cannot express | `solve_full(x^4 + 1, x)`; `solve_full(x^4 + x^2 + 1, x)` | `status=Unevaluated complete=false unrepr=4`, diag `solve.unevaluated: complex algebraic roots not supported (RootOf is real-only in v1).` | 4 complex roots exist | HELD (no over-claim) |
| P3.5 quartic with 2 real + 2 complex roots | `solve_full(x^4 - 2*x^2 - 3, x)` | `Partial complete=false n=2 unrepr=2`, diag `solve.unrepresented-roots` | `[-sqrt(3), sqrt(3), -I, I]` | HELD (honest) |
| P3.6 same quartic over real | `solve_full(x^4 - 2*x^2 - 3, x, real)` | Solved/complete=true, 2 real RootOf values | `solveset(..., S.Reals)` = `{-sqrt(3), sqrt(3)}` | HELD |
| P3.7 degree 6 / degree 5 | `solve_full(x^6 - 1, x)`; `solve_full(x^5 - x + 1, x)` | `Partial complete=false`, `[-1, 1]` + 4 unrepresented; `Partial complete=false`, 1 `rootof` + 4 unrepresented | 6 roots; 5 roots | HELD (honest) |
| P3.8 cubic, all 3 roots | `solve_full(x^3 - 2, x)` | Solved/complete=true, `[2^(1/3), 2^(1/3)*(-1/2*i*sqrt(3) - 1/2), 2^(1/3)*(1/2*i*sqrt(3) - 1/2)]` | correct roots of 2 (rendering itself is defect **F3**) | HELD (values) |
| P3.9 spurious root | `solve_full(sqrt(x) + 2, x)` and `... , real)` and `..., complex)` | `status=Solved complete=true completeness=Complete represented_count=1 unrepresented_count=0 solutions=[Solution(value: 4, multiplicity: 1, exactness: Exact)] diagnostics=[]` | `solve(sqrt(x)+2, x)` = `[]`; `solveset(sqrt(x)+2, x, S.Complexes)` = `EmptySet`; `solveset(..., S.Reals)` = `EmptySet` | **FINDING (F4)** |
| P3.10 excluded point handled | `solve_full((x^2 - 1)/(x - 1), x)` | Solved/complete=true, `[-1]` only | `solve((x**2-1)/(x-1), x)` = `[-1]` | HELD |
| P3.11 multiplicity metadata | `solve_full((x-1)^2, x)` vs `solve_full(x^2 - 2*x + 1, x)` | `[1 (m=1), 1 (m=1)]`, n=2 — vs — `[1 (m=2)]`, n=1, for the **same polynomial**; `solve_full((x-1)^3, x)` → three m=1 entries | `roots((x-1)**2)` = `{1: 2}` | **FINDING (F6)** |
| P3.12 degenerate inputs | `solve_full(0, x)`; `solve_full(5, x)`; `solve_full(1/(x - 1), x)`; `solve_full(abs(x) - 1, x)` | `Unevaluated complete=false` “every value is a solution”; `NoSolutions complete=true`; `NoSolutions complete=true`; `Unevaluated` “No solver for this structure.” | no over-claim in any of the four | HELD |
| P3.13 domain vocabulary | `solve_full(x^2 - 2, x, rational)` / `integer` / `integers` / `natural` | `DomainError: solve(): currently supports domains real and complex; got rational.` | honest refusal | HELD |
| P3.14 exactness metadata | `solve(x^2 - 2, x)` (plain `solve`) | element `{"pretty":"-1/2*sqrt(8)","domain":"real","exact":false}` while `solve_full` reports `AlgebraicExact` for the same values | an exact radical should not be flagged `exact:false` | **FINDING (F8, minor under-claim)** |
| P3.15 quadratic over complex, negative discriminant | `solve_full(x^2 + 1, x)`; `solve_full(x^2 + 1, x, complex)` | `[i, -i]`, Solved/complete=true | `[-I, I]` | HELD |

### Property 4 — complex values against SymPy's principal value

| probe | script | observed | expected (SymPy) | verdict |
|---|---|---|---|---|
| P4.1 | `sqrt(-1)` | `i`, canonical `(i)` | `I` | HELD |
| P4.2 | `sqrt(-4)` | `2*i` | `2*I` | HELD |
| P4.3 | `(-1)^(1/2)` | `i` | `I` | HELD |
| P4.4 | `log(-1)`; `evalf(log(-1), 30)` | symbolic `log(-1)`; `3.141592653589793238462643383279i` | `I*pi`; `N(...,30)` = `3.14159265358979323846264338328*I` | HELD |
| P4.5 | `log(-1/2)`; `evalf(log(-1/2), 30)` | `-0.693147180559945309417232121458 + 3.141592653589793238462643383279i` | `-0.693147180559945309417232121458 + 3.14159265358979323846264338328*I` | HELD |
| P4.6 | `sqrt(4)` | kind `Real`, `2` | `2` (Integer) | HELD |
| P4.7 | `sqrt(2)` | 100-decimal expansion, correct digits | `N(sqrt(2),100)` = `1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573` | HELD |
| P4.8 | `log(0)` | symbolic `log(0)`; `evalf(log(0), 30)` → `DomainError: Ln(x) requires x > 0.` | `log(0)` = `zoo` | INCONCLUSIVE (design divergence: hard error instead of `zoo`; the message “requires x > 0” is contradicted by `evalf(log(-1),30)` succeeding) |
| P4.9 | `log(1)` | `0` | `0` | HELD |
| P4.10 | `sqrt(-0)` | kind `Real`, `0` | `sqrt(S(-0))` = `0` | HELD |
| P4.11 | `sqrt(x^2)` | `sqrt(x^2)`, canonical `(pow (pow (sym x) (rat 2 1)) (rat 1 2))` | `sqrt(x**2)` (unchanged for complex `x`) | HELD |
| P4.12 | `abs((-1)^(1/2))`; `re/im/conj((-1)^(1/2))` | `abs(i)` unevaluated; re/im/conj → `DomainError: DSP builtins expect numeric/complex elements, but got 'NamedConstantExpr'.` | `Abs(I)` = `1`, `re(I)`=0, `im(I)`=1 | **FINDING (F9, minor)** |
| P4.13 | `2^(1/2)`; `4^(1/2)`; `4^(3/2)`; `8^(1/3)`; `(-1)^(1/3)`; `(-8)^(1/3)` | all → `UnsupportedOperation: Non-integer exponents are not yet supported.` while `sqrt(2)`, `sqrt(4)`, `x^(1/2)`, `(-1)^(1/2)`, `(4*x)^(1/2)` all succeed | `sqrt(2)`, `2`, `8`, `2`, `(-1)**(1/3)`, `2*(-1)**(1/3)` | **FINDING (F3)** |
| P4.14 | `evalf(sqrt(2), 5)`; `evalf(pi, 5)` | 100 decimal places for both | `N(sqrt(2),5)` = `1.4142`; `N(pi,5)` = `3.1416` | **FINDING (F7)** |

### Property 5 — limits at poles, branch cuts and discontinuities

| probe | script | observed | expected (SymPy) | verdict |
|---|---|---|---|---|
| P5.1 two-sided DNE with one-sided values | `limit_full(1/x, x, 0)` | `status=DoesNotExist exists=false value=∅ left=-inf right=inf` | `limit(1/x, x, 0, '-')` = `-oo`, `'+'` = `oo` ⇒ no two-sided limit | HELD |
| P5.2 pole with agreeing sides | `limit_full(1/x^2, x, 0)`; `limit_full(1/(x - 1)^2, x, 1)` | `status=PlusInfinity exists=true value=inf` | `limit(1/x**2, x, 0)` = `oo` | HELD |
| P5.3 other one-sided-disagreeing poles | `limit_full(1/sin(x), x, 0)`; `limit_full(x/(x - 1), x, 1)`; `limit_full(exp(x)/x, x, 0)`; `limit_full(1/(x^2 - 1), x, 1)` | all `DoesNotExist exists=false left=-inf right=inf` | `limit(1/sin(x), x, 0, '-')` = `-oo`, `'+'` = `oo` | HELD |
| P5.4 value limits | `limit_full(sin(x)/x, x, 0)`; `(x^2-1)/(x-1), x, 1`; `x*log(x), x, 0`; `x^x, x, 0`; `sqrt(x), x, 0`; `(1 - cos(x))/x^2, x, 0`; `abs(x), x, 0`; `log(x), x, 1`; `(x^2 - 4)/(x - 2), x, 2` | `1`; `2`; `0`; `1`; `0`; `1/2`; `0`; `0`; `4`, each `status=Value exists=true` | `1`; `2`; `0`; `1`; `0`; `1/2`; `0`; `0`; `4` | HELD |
| P5.5 “not determined” distinguished from DNE | `log(x)@0`; `tan(x)@pi/2`; `exp(1/x)@0`; `sin(1/x)@0`; `cos(1/x)@0`; `x*sin(1/x)@0`; `abs(x)/x@0`; `x/abs(x)@0`; `sqrt(x^2)/x@0`; `log(x)/x@0`; `1/abs(x)@0`; `log(1/x)@0`; `1/(1 + exp(1/x))@0`; `exp(-x)@inf`; `1/exp(x)@inf` | every one: `status=Unevaluated exists=∅ value=∅ left=∅ right=∅` with diag `limit.unevaluated` | `tan` sides `oo`/`-oo`, `exp(1/x)` sides `0`/`oo` ⇒ DNE; `log(x)@0` = `-oo`; `exp(-x)@inf` = `0` | HELD (never claims DNE or a value it cannot prove; the gap is disclosed as `Unevaluated`) |
| P5.6 one-sided entry points | `limit_left(1/x, x, 0)`; `limit_right(1/x, x, 0)` | `-inf`; `inf` | `-oo`; `oo` | HELD |
| P5.7 4-argument direction form | `limit_full(1/x, x, 0, right)` | `DomainError: Undefined variable 'right'.` | – | INCONCLUSIVE (out of the stated property; the supported route is `limit_left`/`limit_right`) |

---

## FINDINGS

### F1 — P0: the printer omits parentheses around a negative **numeric** base of a power, producing text that parses to a different value

**Reproduction (exact commands, `--file` form as specified in the task):**

```
x = symbol("x"); -1^x      →  {"display":"-1","structured":{"pretty":"-1","canonical":"(rat -1 1)"}}
x = symbol("x"); (-1)^x    →  {"display":"-1^x","structured":{"pretty":"-1^x","canonical":"(pow (rat -1 1) (sym x))"}}
```

The second expression renders as exactly the text of the first, and the first evaluates to the
constant −1. Numeric confirmation (EVAL):

```
evalf(subs((-1)^x, x, 2), 30)  →  1
evalf(subs(-1^x,   x, 2), 30)  →  -1
evalf(subs((-2)^x, x, 2), 30)  →  4
evalf(subs(-2^x,   x, 2), 30)  →  -4
```

**SymPy (SYMPY):**

```
(-2)**x → (-2)**x   | subs x=2 -> 4        -2**x → -2**x   | subs x=2 -> -4
(-1)**x → (-1)**x   | subs x=2 -> 1        -1**x → -1**x   | subs x=2 -> -1
```

So the printed text denotes the *other* of two values SymPy keeps distinct.

**Family (all EVAL, all pretty printed without parentheses, all reparsed to a different canonical):**

| script | canonical | pretty | canonical after reparsing the pretty |
|---|---|---|---|
| `(-1)^x` | `(pow (rat -1 1) (sym x))` | `-1^x` | `(rat -1 1)` |
| `(-2)^x` | `(pow (rat -2 1) (sym x))` | `-2^x` | `(mul (rat -1 1) (pow (rat 2 1) (sym x)))` |
| `(-1/2)^x` | `(pow (rat -1 2) (sym x))` | `-1/2^x` | `(mul (rat -1 1) (pow (pow (rat 2 1) (sym x)) (rat -1 1)))` |
| `(-3)^x` | `(pow (rat -3 1) (sym x))` | `-3^x` | `(mul (rat -1 1) (pow (rat 3 1) (sym x)))` |
| `(-1)^(x+1)` | `(pow (rat -1 1) (add (rat 1 1) (sym x)))` | `-1^(x + 1)` | `(rat -1 1)` |
| `1/((-2)^x)` | `(pow (pow (rat -2 1) (sym x)) (rat -1 1))` | `(-2^x)^-1` | `(pow (mul (rat -1 1) (pow (rat 2 1) (sym x))) (rat -1 1))` |
| `(1-2)^x`, `(0-2)^x` | `(pow (rat -1 1) (sym x))`, `(pow (rat -2 1) (sym x))` | `-1^x`, `-2^x` | as above |

**Consequence on a real computation.** Source expression
`((-(-1)^(2/1))^(-3/cos(x)))` (canonical
`(pow (rat -1 1) (mul (rat -3 1) (pow (fn cos (sym x)) (rat -1 1))))`) renders as
`-1^(-3/cos(x))`. At `x = 2`:

```
EVAL : evalf(subs(((-(-1)^(2/1))^(-(3)/cos(x))), x, 2), 30)
     = -0.792088340496101150289799255516 - 0.610406471828512419929093114278i
EVAL : evalf(subs(-1^(-3/cos(x)), x, 2), 30)
     = -1
SYMPY: N((-1)**(Rational(-3,1)/cos(2)), 30)
     = -0.792088340496101150289799255539 - 0.610406471828512419929093114249*I
```

The original is right (28 leading digits match SymPy); **its own rendering is wrong**.

**Boundary of the defect.** Symbolic negative bases *are* parenthesised correctly —
`(-x)^2` → `(-x)^2`, `(-x)^x` → `(-x)^x` (both round-trip HELD). `(-1)^(1/2)` escapes the defect
only because it is special-cased to `i`. The defect is specific to a negative **number** base whose
power stays unevaluated.

**This is a printer defect, not a parser defect.** The parser's precedence rule is unambiguous and
consistent — `-2^2` evaluates to `-4` and `-2^x` canonicalises to
`(mul (rat -1 1) (pow (rat 2 1) (sym x)))`, i.e. `^` binds tighter than unary minus (the same rule as
SymPy/Python). The printer is what removes the parentheses, so the text it emits denotes
`-(2^x)` rather than `(-2)^x`. Confirmation that the *source* expression is stored correctly while
only its rendering is wrong: `(-2)^x` has canonical `(pow (rat -2 1) (sym x))` and
`subs((-2)^x, x, 2)` = 4, whereas its own pretty `-2^x` gives `subs(-2^x, x, 2)` = -4.

**Also observed:** the envelope marks `(-1)^x` as `"exact":false` even though both operands are exact.

### F2 — P1: the printer emits `i` for the imaginary unit, and `i` is not a readable identifier

```
EVAL: sqrt(-1)          → pretty "i",           canonical "(i)"
EVAL: sqrt(-4)          → pretty "2*i",         canonical "(mul (rat 2 1) (i))"
EVAL: (-1)^(1/2)        → pretty "i"
EVAL: solve_full(x^4 - 1, x) → solutions pretty "i", "-i"
EVAL: i                → {"ok":false,"code":"InvalidOperation","category":"DomainError",
                          "message":"Undefined variable 'i'.","diagnostics":[{"position":0,"line":1,"column":1}]}
EVAL: 2*i              → same DomainError
EVAL: 1/2*(-4*i - 2)   → same DomainError            (this is solve_full(x^2+2*x+5,x)'s root rendering)
```

No complex closed form the tool produces can be fed back into the tool. `solve_full(x^3 - 2, x)`
compounds it (see F3): its roots render as `2^(1/3)*(-1/2*i*sqrt(3) - 1/2)`, which fails on both
`i` and `2^(1/3)`.

### F3 — P1: the printer emits numeric bases with rational exponents, and the parser rejects that syntax

```
EVAL: solve_full(x^3 - 2, x)
      solutions: [2^(1/3)*(-1/2*i*sqrt(3) - 1/2), 2^(1/3)*(1/2*i*sqrt(3) - 1/2), 2^(1/3)]
EVAL: 2^(1/3)  → {"ok":false,"category":"UnsupportedOperation",
                  "message":"Non-integer exponents are not yet supported."}
EVAL: 2^(1/2)  → same;  4^(1/2) → same;  4^(3/2) → same;  8^(1/3) → same;  (-1)^(1/3) → same
EVAL: sqrt(2)  → kind Real, 1.414213562373095048801688724209698078569671875375…  (works)
EVAL: sqrt(4)  → 2 (works);  x^(1/2) → sqrt(x) (works);  (-1)^(1/2) → i (works)
SYMPY: 2**(1/2) = sqrt(2);  4**(1/2) = 2;  8**(1/3) = 2
```

Three defects in one: (a) a printed form the parser cannot read; (b) `^` with a rational exponent
fails on *literal* bases while the equivalent `sqrt()` call and the symbolic case succeed; (c) the
message “Non-integer exponents are not yet supported” is false for `x^(1/2)`, `x^(2/3)`, `(-1)^(1/2)`
and `(4*x)^(1/2)`, all of which are accepted.

The blocked capability has a well-defined principal-branch answer that this binary cannot reach at
all: SYMPY gives `N((-8)**Rational(1,3), 30)` = `1.0 + 1.73205080756887729352744634151*I` and
`N(Rational(-1,8)**Rational(2,3), 30)` = `-0.125 + 0.216506350946109661690930792688*I`, whereas
EVAL of `(0-8)^(1/3)`, `(1/8 - 1/4)^(2/3)`, `(0-1)^(2/3)` and `evalf((0-8)^(1/3), 30)` all fail with
the same `UnsupportedOperation`. Only the `1/2` power on a negative numeric base is special-cased
(`(-1)^(1/2)` → `i`).

**F3b — same class:** `solve_full(x^4 - 2*x^2 - 3, x)` prints solution values
`rootof(x^4 - 2*x^2 - 3, 0)`; EVAL of that string gives
`DomainError: Unknown function 'rootof'.` (`rootof` is absent from the 123-entry builtin registry).

### F4 — P0: `solve_full` claims `Solved` + `complete: true` for an equation with an empty solution set, and reports a root that does not satisfy it

```
EVAL: solve_full(sqrt(x) + 2, x)
      SolveResult(status: Solved, domain: complex, complete: True, completeness: Complete,
                  solutions: [Solution(value: 4, conditions: [], multiplicity: 1, exactness: Exact)],
                  families: [], represented_count: 1, unrepresented_count: 0, diagnostics: [])
EVAL: solve_full(sqrt(x) + 2, x, real)
      SolveResult(status: Solved, domain: real, complete: True, …, solutions: [Solution(value: 4, …)], diagnostics: [])
EVAL: subs(sqrt(x) + 2, x, 4)   →  4          (i.e. 4 + 0 ≠ 0)
SYMPY: solve(sqrt(x)+2, x)               = []
       solveset(sqrt(x)+2, x, S.Complexes) = EmptySet
       solveset(sqrt(x)+2, x, S.Reals)     = EmptySet
```

With the principal branch the tool itself uses (`sqrt(-1)` = `i`), `sqrt(x) = -2` has no solution;
the solver squared both sides and kept the extraneous root, then certified completeness. Contrast the
correct `solve_full(sqrt(x) - 2, x)` → `[4]`. This is both a wrong value and a false completeness
claim, in the exact shape the brief calls out (“a claim that is too strong … matters as much as a
wrong value”).

### F5 — P2: the canonical form is not idempotent under print→parse for products of inverses

```
EVAL: (x/y)/z   canonical (mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))
                pretty    x/(y*z)
EVAL: x/(y*z)   canonical (mul (sym x) (pow (mul (sym y) (sym z)) (rat -1 1)))
```

Same class: `x/y/a`, `2/x/y`, `x^(-1)/y` → pretty `x/(a*y)`, `2/(x*y)`, `1/(x*y)`, each reparsing to a
`pow`-of-a-product canonical. The values agree — `subs((x/y)/a, x,6),y,3),a,2)` = `1` and
`subs(x/(a*y), …)` = `1` — but the tool’s own machinery cannot see that:

```
EVAL: simplify((x/y)/a - x/(a*y))              → display "-x/(a*y) + x/(a*y)"   (not 0)
EVAL: expand((x/y)/a - x/(a*y))                → display "-x/(a*y) + x/(a*y)"
EVAL: simplify(1/(6*a) - (6*a)^-1)             → display "0/(6*a)"              (not 0)
SYMPY: simplify(x/(a*y) - x/y/a)               = 0
```

So a printed rendering is not recognised as equal to the expression that produced it. Property 2 as
stated (“must canonicalise IDENTICALLY”) is violated even though no value changes.

### F6 — P3: multiplicity metadata contradicts itself for the same polynomial

```
EVAL: solve_full((x-1)^2, x)
      solutions: [Solution(value: 1, multiplicity: 1, exactness: Exact),
                  Solution(value: 1, multiplicity: 1, exactness: Exact)], represented_count: 2
EVAL: solve_full(x^2 - 2*x + 1, x)
      solutions: [Solution(value: 1, multiplicity: 2, exactness: Exact)], represented_count: 1
EVAL: solve_full((x-1)^3, x)
      solutions: [1 (m=1), 1 (m=1), 1 (m=1)], represented_count: 3
SYMPY: roots((x-1)**2) = {1: 2}
```

The factored and expanded spellings of the same polynomial produce opposite metadata (two simple
roots vs one double root), and the factored spellings under-report per-entry multiplicity. The total
“with multiplicity” count is right in both cases, so this is a metadata/consistency defect rather
than a wrong solution set.

### F7 — P2: `evalf(expr, digits)` ignores `digits` for irrational reals

```
EVAL: evalf(sqrt(2), 5)   → kind Real, 100 decimals
EVAL: evalf(sqrt(2), 10/15/20/40/60/100/120/300) → 100 decimals every time
EVAL: evalf(pi, 5)        → 100 decimals
EVAL: evalf(1/3, 5)       → 0.33333                                  (5 decimals, obeys)
EVAL: evalf(log(-2), 5)   → 0.69314 + 3.14159i                       (Complex obeys)
SYMPY: N(sqrt(2),5) = 1.4142;  N(pi,5) = 3.1416;  N(Rational(1,3),5) = 0.33333
```

The returned digits are correct (the printed 100-decimal `sqrt(2)` agrees with
`N(sqrt(2),100) = 1.414213562373095048801688724209698078569671875376948073176679737990732478462107038850387534327641573`
to the printed precision, differing only in the final rounding convention), so this is a
precision-contract violation, not a wrong value. It matters for any caller that compares fixed-width
numeric renderings, and it is inconsistent with both the rational and the complex paths.

### F8 — P3 (minor): `solve` marks exact radicals `exact: false`

```
EVAL: solve(x^2 - 2, x)
      elements: [{"pretty":"-1/2*sqrt(8)","canonical":"(mul (rat -1 2) (pow (rat 8 1) (rat 1 2)))",
                  "domain":"real","exact":false}, {…"1/2*sqrt(8)"…"exact":false}]
EVAL: solve_full(x^2 - 2, x)
      same values reported with exactness: AlgebraicExact
SYMPY: solve(x**2-2, x) = [-sqrt(2), sqrt(2)]
```

An under-claim (says “not exact” about an exact algebraic number) and an internal disagreement
between `solve` and `solve_full`. Also note the unsimplified `±1/2*sqrt(8)` (value-correct: √8/2 = √2).

### F9 — P4 (minor): the imaginary unit is not treated as a numeric/complex element

```
EVAL: abs((-1)^(1/2))  → display "abs(i)"        (unevaluated)
EVAL: re((-1)^(1/2))   → DomainError: DSP builtins expect numeric/complex elements,
                         but got 'NamedConstantExpr'.
EVAL: im((-1)^(1/2))   → same;   conj((-1)^(1/2)) → same
SYMPY: Abs(I) = 1;  re(I) = 0;  im(I) = 1;  conjugate(I) = -I
```

Three issues: `Abs(I)` should be `1`; `re`/`im`/`conj` cannot consume the value their own printer
produces; and the user-facing message leaks an internal type name (`NamedConstantExpr`).

---

## What I could not test

1. **`log(0)` semantics.** SymPy gives `zoo`; the binary raises
   `DomainError: Ln(x) requires x > 0.` I do not have a rule that says which is intended, so this
   stays INCONCLUSIVE. The message itself is demonstrably inaccurate, since
   `evalf(log(-1), 30)` returns `3.141592653589793238462643383279i` from the same run.
2. **`limit_full` direction argument.** `limit_full(1/x, x, 0, right)` fails with
   `Undefined variable 'right'`; only `limit_left`/`limit_right` exist. Whether a 4-argument direction
   form is intended is an API question outside my five properties, so no verdict is given.
3. **Deeper branch-cut probing of `^`.** I confirmed the negative-base printing defect for rational
   integer bases with symbolic exponents, but `(-1)^(2/3)`, `(-1/8)^(2/3)`, `(-4)^(1/2)` cannot be
   typed at all (`UnsupportedOperation`), so the multi-valued branch behaviour of non-half rational
   exponents on negative numeric bases is unreachable and untested.
4. **Non-polynomial completeness.** `solve_full` has no solver for `abs`, and `exp`-equations were not
   reachable in a form whose completeness metadata I could check; the “complete over the named domain”
   claim was therefore only attacked on polynomial, radical and rational equations.
5. **`--print-budget` interaction** with the very long renderings in F3 (1004-character `pi*x`, the
   500-digit reals) was not exercised; the truncation path may change which renderings are re-readable.
6. **Whether the F1 defect has other trigger shapes.** I probed negative numeric bases with symbolic,
   `x+1`, and constant-folded exponents. Bases that are *sums* of negatives
   (`(-1-x)^x`), products with negative real coefficients, or negative reals such as `pi`
   (`(0-pi)^x`) were not systematically enumerated; the fuzz sample of 35 expressions found no
   additional class, but it is a small sample.
7. **One unlogged fuzz failure.** An earlier, larger fuzz run reported
   `tested=77 held=54 mismatch=22 reparse_err=1 parse1_err=3`, but its console output was truncated
   and the single `reparse_err` line was lost before I could capture it. It did not recur in the
   compact 35-expression run (`reparse_err=0`), so I cannot state which rendering failed or why.
   Only the two reparse failures I reproduced by hand (F2, F3) are reported as findings.

---

### Reproduction inventory

Every observation above is a raw envelope line or a raw SymPy print produced in this session; no
source, test, or file outside this deliverable was modified. Scratch input for the `--file`/`--stdin`
equivalence check was created under `tmp-p4r9/` and removed after use.
