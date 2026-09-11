# B4 — Adversarial audit: rewrite rules and solver completeness/exactness

**Auditor:** independent adversarial auditor (B4). **Date:** cycle-5 audit wave 2.
**Product under test:** `out/aot/Lovelace.Run.exe⟫ (5,715,456 bytes, 2026-09-11 15:18), Native AOT.
**Provenance:** `git log --oneline -1⟫ = `9e761ba docs(cycle-5): T0-3 closed, the wire cluster verified, and the golden that moved for a measured reason⟫.
Only `docs/goal-cycle-5/final/*.txt⟫ were dirty; **no repository file was modified except this deliverable**. No build, no `dotnet⟫, no test run.
**Contract:** `docs/symbolics/dsh-protocol.md⟫ + the builtin metadata from `capabilities()⟫.
**Independent ground truth:** SymPy 1.14.0 / mpmath 1.3.0 (`python3⟫ from `C:\Users\ricar\dev\.lovelace-tools\python⟫), exact rational substitution.
**Method:** every probe has its input and an expected observable derived from the contract or from mathematics *before* the run; every FINDING below was produced twice (from the clean temp directories `%TEMP%\b4audit⟫, `%TEMP%\b4audit-repro⟫ and `%TEMP%\b4audit-repro2⟫) and both outputs are pasted. Commands are abbreviated in the tables as
```
R.exe <id>   ==   .\out\aot\Lovelace.Run.exe --file %TEMP%\<dir>\<id>.ls --omit-functions
```
Raw envelopes are abbreviated by …; no field that decides a verdict is elided. Probe scripts are `; ⟫-separated one-liners with `x = symbol("x");⟫ (and any `assume*⟫ prelude) omitted from the "probed" column where it is the constant prefix.

**Scale:** ≈330 distinct probes (one binary invocation each) plus ≈60 re-runs.

---

## 1. Probe table

### 1.1 Assumptions — contradictions, retraction, lattice (30 probes)

| id | what was probed | command | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| A1 | `assume(x>0)⟫ then `assume(x<0)⟫ — contradiction must be reported, not absorbed | R.exe A1 | `{"ok":false,"code":"UnsatisfiableAssumptions","category":"DomainError","message":"Assumption x < 0 contradicts the existing assumptions (its negation x >= 0 is provable).","recoverable":true,…}⟫ exit 1 | HELD | — |
| A2 | same, then `assumptions()⟫ — is the bad atom absorbed before the query? | R.exe A2 | identical `UnsatisfiableAssumptions⟫ envelope; the script aborts at the contradicting `assume⟫, `assumptions()⟫ never runs | HELD | — |
| A3 | `assume(x>0)⟫ then `assume(x==0)⟫ | R.exe A3 | `…"Assumption x = 0 contradicts the existing assumptions (its negation x != 0 is provable)."⟫ | HELD | — |
| A7 | `assume(and(x>0, x<0))⟫ | R.exe A7 | `…"Assumption x > 0 contradicts … (its negation x <= 0 is provable)."⟫ | HELD | — |
| A4 | `assume_nonnegative(x)⟫ → `simplify(sqrt(x^2))⟫ | R.exe A4 | `"display":"x","structured":{"pretty":"x","canonical":"(sym x)","domain":"complex","exact":true…}⟫ | HELD | — |
| A5 | `assume_real(x)⟫ → `simplify(sqrt(x^2))⟫ | R.exe A5 | `"canonical":"(fn abs (sym x))","domain":"real"⟫ — the correct real-domain answer | HELD | — |
| A6 | no assumption → `simplify(sqrt(x^2))⟫ | R.exe A6 | `"canonical":"(pow (pow (sym x) (rat 2 1)) (rat 1 2))"⟫ (unchanged) | HELD | — |
| A9 | `assume(x>=0)⟫ → `simplify(sqrt(x^2))⟫ | R.exe A9 | `"canonical":"(sym x)"⟫ | HELD | — |
| A8 | `assume(x>0)⟫ → `simplify(abs(x))⟫ | R.exe A8 | `"canonical":"(fn abs (sym x))"⟫ (no rule claims this; conservative) | HELD | — |
| A10 | `assume(x!=0)⟫ → `simplify(x/x)⟫ | R.exe A10 | `"canonical":"(rat 1 1)"⟫ | HELD | — |
| A11 | `assume(x==0)⟫ → `simplify(x/x)⟫ (safe mode must refuse) | R.exe A11 | `"canonical":"(mul (sym x) (pow (sym x) (rat -1 1)))"⟫ (unchanged) | HELD | — |
| A12 | `assume(x==0)⟫ → `simplify_full(x/x)⟫ | R.exe A12 | `TransformResult(status: Unsatisfiable, …, conditions: [UnsatisfiableConditions()], steps: [RewriteStep(rule_id: rat.cancel-x-over-x, … required_conditions: [x != 0])], diagnostics: [Diagnostic(code: transform.unsatisfiable-conditions, category: DomainError, …, recoverable: False…)])⟫ | HELD | — |
| H01 | `assume(x>0); simplify(sqrt(x^2)); assume_clear(); simplify(sqrt(x^2))⟫ — state leak? | R.exe H01 | last value `(pow (pow (sym x) (rat 2 1)) (rat 1 2))⟫ — retraction is clean | HELD | — |
| H02/H03 | same call before/after the assumption, both orders | R.exe H02, H03 | H02 `x⟫, H03 `x⟫ (assumption is read at call time) | HELD | — |
| H04 | `assume_clear()⟫ → `assumptions()⟫ | R.exe H04 | `""⟫ | HELD | — |
| H05 | `assume(x>0); assume(x>5)⟫ → `assumptions()⟫ | R.exe H05 | `"x > 0; x > 5"⟫ (both atoms kept, canonical order) | HELD | — |
| H06/H07 | `assume(x>=0); assume(x<=0)⟫ — model x=0 must **not** be called a contradiction | R.exe H06, H07 | H06 `"x >= 0; x <= 0"⟫; H07 `simplify(sqrt(x^2))⟫ → `(sym x)⟫ (correct: x=0) | HELD | — |
| H08 | `assume(x>0)⟫ then `assume(x<=0)⟫ | R.exe H08 | `UnsatisfiableAssumptions⟫ (correct) | HELD | — |
| H09/H10 | `assume(not(x>0))⟫ | R.exe H09, H10 | H09 `assumptions()⟫ → `"x <= 0"⟫; H10 `sqrt(x^2)⟫ unchanged | HELD | — |
| H11 | `assume_integer(x)⟫ → `simplify(sqrt(x^2))⟫ | R.exe H11 | `(fn abs (sym x))⟫ (integers are real) | HELD | — |
| H12 | `assume(y>0)⟫ (different symbol) → `simplify(sqrt(x^2))⟫ | R.exe H12 | unchanged — no cross-symbol leak | HELD | — |
| H13/H14 | same *name*, second declaration | R.exe H13, H14 | see **F9** | FINDING | P2 |
| H15 | duplicate `assume(x>0)⟫ twice | R.exe H15 | `"x > 0"⟫ (idempotent) | HELD | — |
| H16 | `assume(x==0)⟫ then `assume(x!=0)⟫ | R.exe H16 | `UnsatisfiableAssumptions⟫ | HELD | — |
| H17 | `assume(x>y)⟫ with undeclared y | R.exe H17 | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'y'."}⟫ | HELD | — |
| H18 | `assume(x>0)⟫ → `simplify(x/x); simplify_full(x/x)⟫ | R.exe H18 | last value `1⟫ with condition `x != 0⟫ | HELD | — |

### 1.2 Value preservation of every rewrite — SymPy differential over 30 expressions (30 probes; 60 invocations)

Each case ran the bare expression (to obtain the kernel's own canonical *input*) and the builtin applied to it, then SymPy substituted 18 rational **and complex** points into `input − output⟫ at 40-digit precision (harness `%TEMP%\b4audit\diff.py⟫). Expected: difference exactly 0 wherever the rule's stated conditions hold.

| id | builtin(expr), assumption | in → out (canonical) | SymPy Δ at licensed points | verdict |
|---|---|---|---|---|
| G01 | `simplify_full(x/x)⟫ | `(mul (sym x) (pow (sym x) (rat -1 1)))⟫ → `(rat 1 1)⟫ | 0 at all 18 points (skip x=0) | HELD |
| G02 | `simplify_full(0/x)⟫ | `(mul (rat 0 1) (pow (sym x) (rat -1 1)))⟫ → `(rat 0 1)⟫ | 0 at all points (skip x=0) | HELD |
| G03 | `simplify_full(exp(log(x)))⟫ | `(fn exp (fn log (sym x)))⟫ → `(sym x)⟫ | 0 at all points (skip x=0) | HELD |
| G04 | `simplify_full(log(exp(x)))⟫, no assumption | unchanged | 0 (no rule fired) | HELD |
| G05 | `simplify_full(abs(x)^2)⟫, no assumption | unchanged | 0 | HELD |
| G06 | `simplify_full(sqrt(x^2))⟫, no assumption | unchanged | 0 | HELD |
| G07 | `assume_real(x); simplify_full(sqrt(x^2))⟫ | `(pow (pow (sym x) (rat 2 1)) (rat 1 2))⟫ → `(fn abs (sym x))⟫ | **0 at the 10 real points**; non-zero only at the complex points the Real(x) condition excludes (I, −I, 1+I, …) | HELD |
| G08 | `assume_real(x); simplify_full(log(exp(x)))⟫ | `(fn log (fn exp (sym x)))⟫ → `(sym x)⟫ | 0 at all points | HELD |
| G09 | `assume_real(x); simplify_full(abs(x)^2)⟫ | `(pow (fn abs (sym x)) (rat 2 1))⟫ → `(pow (sym x) (rat 2 1))⟫ | 0 at the 10 real points; non-zero only off Real(x) | HELD |
| G10 | `assume(x>0); simplify_full(sqrt(x^2))⟫ | → `(sym x)⟫ | 0 at the 6 positive points; ±(2,4,3,2,1) at the negative ones the guard excludes | HELD |
| G11 | `assume(x>=0); simplify_full(sqrt(x^2))⟫ | → `(sym x)⟫ | 0 at the 7 non-negative points | HELD |
| G12 | `assume(x<0); simplify_full(sqrt(x^2))⟫ | unchanged | 0 (conservative) | HELD |
| G13 | `simplify_full(sin(x)^2+cos(x)^2)⟫ | → `(rat 1 1)⟫ | 0 at all 18 points | HELD |
| G14/G15 | `simplify_full⟫/`simplify((x^2-1)/(x-1))⟫ | unchanged | 0 (skip x=1, undefined both sides) | HELD |
| G16 | `cancel((x^2-1)/(x-1))⟫ | → `(add (rat 1 1) (sym x))⟫ | 0 except x=1 where the input is undefined | HELD |
| G17 | `factor(x^4-5*x^2+4)⟫ | → 4 linear factors | 0 at all 18 points | HELD |
| G18 | `expand((x+1)^3)⟫ | → `(add (rat 1 1) (pow (sym x) (rat 3 1)) (mul (rat 3 1) (sym x)) (mul (rat 3 1) (pow (sym x) (rat 2 1))))⟫ | 0 | HELD |
| G19 | `apart((x^2+1)/(x-1))⟫ | → `1 + x + 2/(x-1)⟫ | 0 except x=1 (undefined) | HELD |
| G20 | `apart(1/(x^2-1))⟫ | → `-1/2/(x+1) + 1/2/(x-1)⟫ | 0 (skip ±1) | HELD |
| G21 | `simplify(exp(log(x)))⟫ safe mode, no assumption | unchanged | 0 | HELD |
| G22/G23 | `simplify_full(abs(-x))⟫ with and without `assume_real⟫ | → `(fn abs (sym x))⟫ | 0 at all points (universal) | HELD |
| G24 | `simplify_full(x*0)⟫ | in already `(rat 0 1)⟫ | 0 | HELD |
| G25 | `simplify_full(1/(1/x))⟫ | in already `(sym x)⟫ (constructor) | 0 except x=0 (input undefined) — definedness delta, no value change | HELD |
| G26 | `assume(x!=0); simplify(x/x)⟫ | → `(rat 1 1)⟫ | 0 | HELD |
| G27 | `assume_real(x); simplify_full((x^3)^(1/3))⟫ | unchanged | 0 for all 10 real points | HELD |
| G28 | `simplify_full((x^2)^3)⟫ | in already `(pow (sym x) (rat 6 1))⟫ | 0 | HELD |
| G29 | `simplify_full(x^2/x)⟫ | unchanged | 0 (skip x=0) | HELD |
| G30 | `simplify_full(sqrt(x)*sqrt(x))⟫ | → `(sym x)⟫ | 0 at all 18 points ((√z)²=z, principal branch) | HELD |

Filtered re-run (`%TEMP%\b4audit\filt.py⟫) restricted G07/G09 to real points, G10 to x>0, G11 to x≥0, G12 to x<0: **`MISMATCHES= none⟫ in every case**. No rewrite of the shipped rule set changes the value on the domain its own conditions license.

### 1.3 Rewrite behaviour, guards and gaps (58 probes)

| id | what was probed | raw output (abridged) | verdict | sev |
|---|---|---|---|---|
| C01–C12 | `simplify⟫/`simplify_full⟫/`cancel⟫/`factor⟫/`expand⟫/`apart⟫ on `x/x, 0/x, exp(log x), log(exp x), abs(x)^2, sin²+cos², (x²−1)/(x−1), x⁴−5x²+4, (x+1)³, (x²+1)/(x−1), 1/(x²−1)⟫ | as tabulated in \1.2 (C13–C16 repeat G03/G04/G07/G09) | HELD | — |
| C17/C20 | `simplify((x^2)^3)⟫→`(pow (sym x) (rat 6 1))⟫; `simplify(x*0)⟫→`(rat 0 1)⟫ | — | HELD | — |
| C18 | `simplify(sqrt(x^2)*sqrt(x^2))⟫ → `(pow (sym x) (rat 2 1))⟫ | SymPy Δ = 0 at 18 points | HELD | — |
| C19 | `simplify(1/(1/x))⟫ → `(sym x)⟫ **unconditionally**, while `x/x⟫ is kept and cancelled only under `x != 0⟫ | `"display":"x"⟫ | HELD (definedness delta, inconsistent with the `x/x⟫ policy) | P3 |
| C21/C22 | `simplify_full(x/x)⟫ conds `[(ne (sym x) (rat 0 1))]⟫; `assume_nonnegative(x); simplify_full(sqrt(x^2))⟫ conds `[(ge (sym x) (rat 0 1))]⟫ | — | HELD | — |
| P01/P03 | `x^2 + x^2⟫ is **not** collected at construction; `x^2 - x^2⟫ likewise | `(add (pow (sym x) (rat 2 1)) (pow (sym x) (rat 2 1)))⟫ | HELD (canonical form is not fully normalised; `simplify⟫ fixes it) | P3 |
| P02/P10 | `simplify(x^2 + x^2)⟫→`2*x^2⟫; `simplify(2*x^2+3*x^2)⟫→`5*x^2⟫ | — | HELD | — |
| P04/P12/P15 | `x^2*x^2⟫→`x^4⟫; `simplify(sqrt(x)*sqrt(x))⟫→`x⟫; `simplify((x^2)^1)⟫→`x^2⟫ | — | HELD | — |
| P05/P08/P13/P14 | `simplify(x^2/x^2)⟫, `x^3/x^3⟫, `x^2*x^(-2)⟫, `(x^2)*(1/(x^2))⟫ — all unchanged | `(mul (pow (sym x) (rat -2 1)) (pow (sym x) (rat 2 1)))⟫ | FINDING (F7) | P2 |
| P06 | `simplify_full(x^2/x^2)⟫ — no step, no condition | `TransformResult(status: Satisfied, …, expression: x^-2*x^2, changed: True, conditions: [], steps: [])⟫ | FINDING (F7+F5) | P2 |
| P09 | `simplify(x^2/x)⟫ unchanged (exponents +2/−1 not combined) | — | FINDING (F7 coverage) | P2 |
| P11 | `simplify(x^2 == x^2)⟫ stays a relation `(eq (pow …) (pow …))⟫ instead of folding to true | — | INCONCLUSIVE (no `simplify(x == x)⟫ control run) | — |
| Q01/Q03/Q04 | safe mode refuses compound cancellation without a provable `NonZero⟫ — correct conservatism | `simplify((x+1)/(x+1))⟫ unchanged | HELD | — |
| Q05 | `simplify(0/x^2)⟫ unchanged (pattern needs exponent −1) | `(mul (rat 0 1) (pow (sym x) (rat -2 1)))⟫ | FINDING (F7 coverage) | P2 |
| Q08/Q09 | `solve_full(x^2/x^2 == 1, x)⟫ and `… == 2⟫ → `Unevaluated⟫ | — | HELD (conservative) | — |
| Q10/Q11/Q12 | `cancel⟫, `apart⟫ return `1⟫; `factor⟫ leaves `x^-2*x^2⟫ | — | HELD | — |
| Q13/Q14 | `simplify(exp(x^2)/exp(x^2))⟫, `simplify(x^(1/2)/x^(1/2))⟫ unchanged in safe mode | — | HELD | — |
| R01–R12 | compound cancellation under `assume(x!=0)⟫: only `a/a⟫ (bare symbol) cancels | `assume(x != 0); simplify((x+1)/(x+1))⟫ → unchanged | FINDING (F7) | P2 |
| S01/S03/S07/S08 | unsafe mode: `x/x⟫, `sin(x)/sin(x)⟫, `abs(x)/abs(x)⟫, `exp(x^2)/exp(x^2)⟫ → `1⟫ with one step | `steps=[rat.cancel-x-over-x], conds=1⟫ | HELD | — |
| S02/S04/S05/S06/S09 | unsafe mode: `(x+1)/(x+1)⟫, `x^2/x^2⟫, `x^3/x^2⟫, `(x^2+1)/(x^2+1)⟫ → unchanged, `steps=[]⟫ | — | FINDING (F7) | P2 |
| S10/S11 | `0/(x+1)⟫ → `0⟫ (1 step); `0/x^2⟫ → unchanged | — | FINDING (F7 coverage) | P2 |
| X01/X02 | `abs(x+1)/abs(x+1)⟫ → `1⟫; `exp(x+1)/exp(x+1)⟫ → `1⟫ (the equality machinery *does* work for `AddExpr⟫ arguments) | `… expression: 1, conditions: [abs(x + 1) != 0], steps: [RewriteStep(rule_id: rat.cancel-x-over-x…⟫ | HELD (isolates F7 to factor order) | — |
| X03/X04/X06 | `(x+1)^2/(x+1)^2⟫, `(2*x)/(2*x)⟫, `0/x^2⟫ unchanged | — | FINDING (F7 coverage) | P2 |
| M05/M06/M07/M08 | symbolic exponents `(x^a)^b⟫, `(x^2)^a⟫, `x^(2a)⟫, `sqrt(x^(2a))⟫ with `a>0⟫ — nothing rewritten | — | HELD (conservative; `(x^a)^b⟫ correctly not folded) | — |
| M09–M12, M15, M16 | `sqrt(x^4)⟫, `sqrt(x^6)⟫, `abs(x)^4⟫, `sqrt(x^2)*sqrt(x^2)⟫ with `assume_real⟫ | unchanged except M16 → `x^2⟫ | HELD (conservative) / F5 for `changed⟫ | P2 |
| M13/M14 | `abs(x)^3⟫ unchanged with and without `assume_real⟫ (no rule) | — | HELD | — |

### 1.4 Trig identity guard (9 probes)

| id | probed | raw output (abridged) | verdict | sev |
|---|---|---|---|---|
| TR1/V5 | `simplify_full(sin(u)^2+cos(u)^2)⟫, u=x, u=exp(x), u=sin(x), u=x^2, u=x*x | `expression: 1, steps: [RewriteStep(rule_id: trig.pythagorean-sin2-cos2, classification: Universal, …)]⟫ | HELD | — |
| TR2 | u = `x+1⟫ | `original: cos(x + 1)^2 + sin(x + 1)^2, expression: cos(x + 1)^2 + sin(x + 1)^2, changed: True, steps: []⟫ | FINDING (F6) | P2 |
| TR3 | u = `2*x⟫ | unchanged, `steps: []⟫ | FINDING (F6) | P2 |
| V2 | u = `abs(x)⟫ | unchanged, `steps: []⟫ | FINDING (F6) | P2 |
| V4 | u = `x/2⟫ (canonical `1/2*x⟫) | unchanged, `steps: []⟫ | FINDING (F6) | P2 |
| TR4 | u = `sin(x)⟫ | → `1⟫ | HELD | — |

### 1.5 Solver — completeness and root exactness (56 probes)

Ground truth: `solveset(…, S.Complexes / S.Reals)⟫, `roots()⟫, `real_roots()⟫, 20-digit `N()⟫.

| id | equation (domain) | kernel answer (abridged) | SymPy ground truth | verdict |
|---|---|---|---|---|
| D01/J02 | `x^2-4==0⟫; `x^3==1⟫(real) | `{−2, 2}⟫; `{1}⟫ — Solved/Complete | `{−2,2}⟫; `{1}⟫ | HELD |
| D02/D03/D14/J01 | `x^2+1==0⟫; `x^2==−1⟫; `x^3==1⟫(ℂ) | `{i,−i}⟫; `{(−1±i√3)/2, 1}⟫ — Solved/Complete | identical | HELD |
| D04 | `x^2-2==0⟫ | `±(1/2)*√8⟫ — exact radicals | `±√2⟫ | HELD |
| L01/L02/L07 | roots of `x^3==1⟫, `x^4==16⟫, `x^6-1==0⟫ substituted back | residuals `0⟫ (integers) and `≈9.7e-21⟫ (algebraic, `evalf⟫ at 20 digits) | residual 0 | HELD |
| L03/L04 | `x^4-x^2-1⟫, `x^5-x+1⟫ RootOf values + residuals | `−1.2720196495140689642524 / +1.2720196495140689642524⟫; `−1.167303978261418684256⟫; residual `≈1.3e-22⟫ | `±1.2720196495140689643⟫; `−1.1673039782614186843⟫ | HELD |
| D05/DG1 | `exp(x)==0⟫ | `NoSolutions⟫, `complete: true⟫, diagnostic `solve.no-solutions⟫, `recoverable: true⟫, 6-field Diagnostic record with `location: Null⟫, `details: []⟫ | `EmptySet⟫ | HELD |
| D06/K06/K08/J18 | `sin(x)==0⟫; `cos(x)==1⟫; `cos(x)^2==1⟫ | `Solved/Complete⟫, `solutions: []⟫, `families: [k*pi]⟫; `[2*k*pi]⟫; `[2kπ, acos(−1)+2kπ]⟫ | `{2kπ}∪{2kπ+π}⟫; `{2kπ}⟫; `{kπ}⟫ | HELD mathematically, **F4** contract gap |
| D21/D22/D23 | `cos(x)==0⟫; `sin(x)==1⟫; `tan(x)==0⟫ | `[1/2*pi + k*pi]⟫; `[asin(1)+2kπ]⟫; `[k*pi]⟫ | `{π/2+kπ}⟫; `{π/2+2kπ}⟫; `{kπ}⟫ | HELD |
| D07/D18/J14/J15/J16 | `x^4-x^2-1⟫; `x^5-x+1⟫; `x^6-1⟫; `x^4==16⟫ | `Partial⟫ + `solve.unrepresented-roots⟫, counts 2/4/4/2; `x^4==16⟫ fully Solved | 2 real + 2 complex; 1 real + 4 complex; 2 real + 4 complex; 4 real | HELD |
| D09/J17 | `1/x==0⟫; `1/(x-1)==0⟫ | `NoSolutions/Complete⟫ | `EmptySet⟫ | HELD |
| D12 | `(x^2-4)/(x-2)==0⟫ | `{−2}⟫ with condition `x−2 != 0⟫ | `{−2}⟫ | HELD |
| D11/L10/L08 | `x^2==0⟫; `x^2-2x+1==0⟫; `x^3-4x^2+5x-2==0⟫ | `0×2⟫; `1×2⟫; `{1×2, 2×1}⟫ | `{0:2}⟫; `{1:2}⟫; `{1:2, 2:1}⟫ | HELD |
| L09 | `(x-1)^2==0⟫ | **two** Solution records `1(m=1)⟫, `1(m=1)⟫, `represented_count: 2⟫ | `{1: 2}⟫ — same set as L10, different record | FINDING (F8) |
| D13/J04 | `(x-1)^2*(x-2)==0⟫ | `Unevaluated⟫ | `{1,2}⟫ | HELD (conservative) |
| J03/J07/J13/D08/D15/D16/J10/J11 | `sqrt(x)==x-2⟫; `sin(x)==2⟫; `abs(x)==2⟫; `sqrt(x)==-1⟫; `abs(x)==-1⟫; `2^x==8⟫; `x^2==x^2⟫; `(x^2-1)/(x-1)==2⟫ | all `Unevaluated⟫ + `solve.unevaluated⟫, `complete: false⟫ | non-empty / all-x | HELD (refusals, no false claim) |
| K05/K07 | `2^x==1⟫; `sinh(x)==0⟫ | `Unevaluated⟫ | `{0}⟫; `{kπi}⟫ | HELD (conservative) |
| J08/J12 | `x^2+1==0⟫(real); `x^2==-4⟫(real) | `NoSolutions/Complete⟫ | `EmptySet⟫ over ℝ | HELD |
| D17/J06 | `log(x)==0⟫; `log(x)==1⟫ | `{1}⟫; `{e}⟫ | `{1}⟫; `{e}⟫ | HELD |
| **J05/K01/K02/K03/K04** | `exp(x)==1⟫; `==2⟫; `==−1⟫; `exp(2x)==1⟫; `exp(x)==exp(1)⟫ — default domain **complex** | `Solved / complete: true / completeness: Complete / families: [] / unrepresented_count: 0⟫ with the single root `0⟫, `log(2)⟫, `log(-1)⟫, `0⟫, `log(exp(1))⟫ | `{2kπi}⟫, `{log2+2kπi}⟫, `{i(2k+1)π}⟫, `{kπi}⟫, `{1+2kπi}⟫ — **infinite** | **FINDING (F1)** | **P0** |
| L11 (real) | `exp(x)==1⟫ over ℝ | `Solved/Complete {0}⟫ | `{0}⟫ | HELD |
| L12 | `solve_full(x^2==4, x, integer)⟫ | `InvalidOperation/DomainError⟫, documented unsupported domain | — | HELD |

### 1.6 Protocol, limits, boundaries (43 probes)

| id | probed | raw output (abridged) | verdict | sev |
|---|---|---|---|---|
| cap | `capabilities()⟫ | 16 `UnsupportedCapability⟫ records, each with operation_class/code/category/message/trigger; `exactness: BestEffort⟫ | HELD | — |
| T01–T04 | `type(r.status)⟫, `type(r.completeness)⟫, `type(r)⟫, `type(r.solutions[0].exactness)⟫ | `SolveStatus⟫, `Completeness⟫, `SolveResult⟫, `SolutionExactness⟫ | HELD | — |
| PR2 | `print("hello", x); 1+1⟫ | `"output":["hello x"]⟫, `timings[1].hasOutput=true⟫, `timings[1].resultKind=Void⟫, stdout is the envelope only | HELD | — |
| AR1 | `simplify_full()⟫ | `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"simplify_full(): expected 1 argument; got 0.","recoverable":true}⟫ exit 1 | HELD | — |
| EM1 | zero-byte script | `{"ok":true,"revision":80,"output":[],"variables":[],"timings":[]}⟫ exit 0 | HELD | — |
| NF2 | `--file <missing>.ls⟫ | `{"ok":false,"code":"FileReadError","category":"ParseError","message":"Cannot read script file …"}⟫ exit 1 | HELD | — |
| CC300 | 300 independent rewrite sites (`abs(-(x+i))⟫ chain) | `status=Satisfied budget_exceeded=false steps=300⟫ | HELD | — |
| CC405/BB405/BUD450 | 405 rewrite sites; 450-term sum | CC405: `status=BudgetExceeded budget_exceeded=true kind='rewrite_steps' steps=400 diag=[('transform.budget-exceeded','BudgetExceeded')]⟫; BB405/450: `DepthExceeded/BudgetExceeded, recoverable=true, "expression tree depth 513 exceeds the maximum supported nesting depth of 512"⟫ exit 1 — structured refusal, no death | HELD | — |
| PB3 | `expand((x+1)^12)⟫ (nodeCount 58) `--print-budget 6⟫ | `"canonical":"(add (rat 1 1) (pow (sym x) (rat 12 1)) (mul ( …","budget":6,"truncationReason":"node-budget"…⟫ | HELD | — |
| PB4/PB5/PB6 + bisect | nodeCount ≤ 6 with budget 1…nodeCount−1 | `sqrt(x^2)⟫ nodeCount 5, budget 1 → `"canonical":"(pow (pow (sym x) (rat 2 1)) (rat 1 2))"⟫, **zero `truncated⟫/`truncationReason⟫ keys**; `x+1⟫(3), `x^2+x⟫(5), `x^2+1⟫(5), `x^4+1⟫(5), `x^3+x+1⟫(6) never truncate; `2*x^2+1⟫(7), `sqrt(x^2)+1⟫(7), `x*(x^2+1)⟫(7), `(x+1)^2⟫(8) truncate at any budget | FINDING (F3) | P1 |
| U1/U2/U3, `--version⟫, `--print-budget abc⟫/`0⟫/`-5⟫, bare missing path | usage errors | **all exit 1**, stdout empty, usage text on stderr (`Error: No script provided…⟫, `--file requires a path argument⟫, `Unknown argument '--bogus'⟫, `--print-budget requires a positive node count⟫) | FINDING (F2) | P1 |
| `--help⟫/`-h⟫ | help | exit 0, help text on stdout (no envelope) | HELD | — |
| RE1–RE6 | `revision⟫ under different scripts | `1+1⟫→81, `symbol("x")⟫→82, two symbols→83, empty→80, capabilities→82 | INCONCLUSIVE (no documented semantics) | — |
| EV1–EV4 | `evalf(1/3, 100)⟫, `evalf(1/3, 1000000000)⟫, `evalf(1/3, 0)⟫ | 102 chars; **1002 chars** (silent 1000-digit cap); `0⟫ | INCONCLUSIVE (outside rewrite/solve scope; no cap in dsh-protocol.md) | — |
| I01–I08 | record field access and error shapes | `r.solutions⟫, `r.status⟫, `r.families⟫ work; `r["solutions"]⟫/`keys(r)⟫ → `InvalidOperation/DomainError⟫ | HELD | — |

---

## 2. FINDINGS

### F1 — P0 — `solve_full⟫ reports `Solved / completeness: Complete⟫ for `exp(x)==c⟫ over the complex domain while returning only one root; the solution set is the infinite family `log(c) + 2πik⟫.

**Reproduction (clean dir `%TEMP%\b4audit-repro⟫, second run; first run `J05⟫ in `%TEMP%\b4audit⟫ gave identical deciding fields):**

```
> .\out\aot\Lovelace.Run.exe --file %TEMP%\b4audit-repro\F1_exp1.ls --omit-functions
   script: x = symbol("x"); solve_full(exp(x) == 1, x)
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,
 "result":{"kind":"Record","display":"SolveResult(status: Solved, variable: x, domain: complex, complete: True,
 completeness: Complete, solutions: [Solution(value: 0, conditions: [], multiplicity: 1, exactness: Exact)],
 families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, unrepresented_reason: ,
 diagnostics: [])", … "structured":{ … "fields":[
   {"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Solved"}},
   {"name":"domain","value":{"kind":"Domain","domain":"complex"}},
   {"name":"complete","value":{"kind":"Boolean","value":"true"}},
   {"name":"completeness","value":{"kind":"Enum","type":"Completeness","value":"Complete"}},
   {"name":"solutions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[…]}},
   {"name":"families","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
   {"name":"represented_count","value":{"kind":"Integer","value":"1","exact":true}},
   {"name":"unrepresented_count","value":{"kind":"Integer","value":"0","exact":true}}, … ]}}}
```

Second reproduction, same directory:

```
> …Run.exe --file %TEMP%\b4audit-repro\F1_exp2.ls --omit-functions
   script: x = symbol("x"); solve_full(exp(x) == 2, x)
"display":"SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
 solutions: [Solution(value: log(2), conditions: [1 != 0], multiplicity: 1, exactness: AlgebraicExact)],
 families: [], common_conditions: [1 != 0], represented_count: 1, unrepresented_count: 0, …)"
```

Same shape for `exp(x) == -1⟫ (`log(-1)⟫), `exp(2*x) == 1⟫ (`0⟫), `exp(x) == exp(1)⟫ (`log(exp(1))⟫).

**Ground truth (SymPy 1.14.0):**
```
exp(x)-1  C => ImageSet(Lambda(_n, 2*_n*I*pi), Integers)
exp(x)-2  C => ImageSet(Lambda(_n, 2*_n*I*pi + log(2)), Integers)
exp(x)+1  C => ImageSet(Lambda(_n, I*(2*_n*pi + pi)), Integers)
exp(2x)-1 C => ImageSet(Lambda(_n, _n*I*pi), Integers)
exp(x)-exp(1) C => ImageSet(Lambda(_n, 2*_n*I*pi + 1), Integers)
exp(x)-1  R => {0}
```

**Why P0:** the protocol's own mapping is violated — "When the solver cannot represent the whole set, `status⟫ is `Partial⟫ and `unrepresented_count⟫/`unrepresented_reason⟫ say how much is missing" (dsh-protocol.md, \"A real solve envelope"), and the table maps `Solved⟫→`complete: true⟫→`Completeness.Complete⟫. The whole family is missing (infinitely many roots), yet every completeness field says nothing is missing, `families⟫ is empty and `unrepresented_count⟫ is 0. The same binary *does* emit families for `sin(x)==0⟫ and `cos(x)==1⟫, so the omission is specific to the exp path, not a representational limit. The real domain is handled correctly (`{0}⟫), so the wrong claim is confined to the **default** domain, complex.

### F2 — P1 — The documented usage-error exit code `2⟫ was not produced by any usage-error path probed; every one returned `1⟫ with an empty stdout.

**Reproduction (second run, clean dir `%TEMP%\b4audit-repro2⟫; first run `U1–U3⟫ + `--version⟫ + `--print-budget abc⟫ in `%TEMP%\b4audit⟫):**

```
> .\out\aot\Lovelace.Run.exe            → exit 1  stdout=""  stderr="Error: No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path. …"
> .\out\aot\Lovelace.Run.exe --file     → exit 1  stdout=""  stderr="Error: --file requires a path argument. …"
> .\out\aot\Lovelace.Run.exe --bogus    → exit 1  stdout=""  stderr="Error: Unknown argument '--bogus'. …"
> .\out\aot\Lovelace.Run.exe --print-budget abc → exit 1  stdout=""  stderr="Error: --print-budget requires a positive node count. …"
> .\out\aot\Lovelace.Run.exe --print-budget 0   → exit 1   (same)
> .\out\aot\Lovelace.Run.exe --print-budget -5  → exit 1   (same)
> .\out\aot\Lovelace.Run.exe --version  → exit 1  stderr="Error: Unknown argument '--version'. …"
> .\out\aot\Lovelace.Run.exe nope.ls    → exit 1  stdout={"ok":false,"code":"FileReadError","category":"ParseError",…}
```

dsh-protocol.md \"Error envelope" states: "Exit codes: 0 success, 1 script/diagnostic error, **2 usage error**." No probed usage error produced 2. A caller that classifies failures by exit code cannot distinguish a usage error from a script error. (Related, and noted rather than claimed: on these paths stdout carries **no envelope at all**, while Invariant 1 says "stdout carries the envelope and nothing else".)

### F3 — P1 — `--print-budget⟫ is ignored for any rendering of 6 nodes or fewer, contra the documented "beyond it, … `truncated: true⟫, `truncationReason⟫ … `budget⟫".

**Reproduction (second run `%TEMP%\b4audit-repro\F4_pb*⟫; first run `PB4/PB5/PB6⟫ plus the bisect in `%TEMP%\b4audit⟫):**

```
> …Run.exe --file %TEMP%\b4audit-repro\F4_pb.ls --omit-functions --print-budget 1
   script: x = symbol("x"); sqrt(x^2)
{"…","result":{"kind":"Symbolic","display":"sqrt(x^2)","structured":{"kind":"Symbolic","pretty":"sqrt(x^2)",
 "canonical":"(pow (pow (sym x) (rat 2 1)) (rat 1 2))","domain":"complex","exact":false,"nodeCount":5,
 "freeSymbols":["x"]}}, …}
   → zero occurrences of "truncated", zero of "truncationReason" (nodeCount 5 > budget 1)

> …Run.exe --file %TEMP%\b4audit-repro\F4_pb2.ls --omit-functions --print-budget 1
   script: x = symbol("x"); 2*x^2 + 1
"canonical":"(add (rat 1 1) (mul (rat 2 1) (pow (sym x) (rat  …","domain":"complex","exact":true,"nodeCount":7,…
   → "truncated":true, "truncationReason":"node-budget", "budget":1   (control: the mechanism works)
```

Bisect (first run, `--print-budget <nodeCount−1>⟫ and then `--print-budget 1⟫): `x+1⟫(3) no, `x^2+x⟫(5) no, `x^2+1⟫(5) no, `x^4+1⟫(5) no, `x^3+x+1⟫(6) no, `2*x^2+1⟫(7) yes, `sqrt(x^2)+1⟫(7) yes, `x*(x^2+1)⟫(7) yes, `(x+1)^2⟫(8) yes. The documented case `expand((x+1)^12)⟫ with `--print-budget 6⟫ truncates correctly (`"canonical":"(add (rat 1 1) (pow (sym x) (rat 12 1)) (mul ( …"⟫, `"budget":6⟫, `"truncationReason":"node-budget"⟫). Impact is small (only short renderings), but the promise in Invariant 4 is unconditional.

### F4 — P1 — The documented solve-decoding procedure ("how many solutions? (`solutions.shape⟫)") returns 0 for an equation with infinitely many solutions, because the record's `families⟫/`represented_count⟫/`common_conditions⟫ fields appear nowhere in dsh-protocol.md.

**Reproduction (second run `%TEMP%\b4audit-repro2\FAM⟫; first run `D06/K06/K08⟫ in `%TEMP%\b4audit⟫, plus the raw `D06raw⟫ envelope):**

```
> …Run.exe --file %TEMP%\b4audit-repro2\FAM.ls --omit-functions
   script: x = symbol("x"); solve_full(sin(x) == 0, x)
"display":"SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
 solutions: [], families: [SolutionFamily(template: k*pi, parameter: k, period: pi, parameter_domain: integer,
 conditions: [], exactness: ParametricExact)], common_conditions: [], represented_count: 0,
 unrepresented_count: 0, unrepresented_reason: , diagnostics: [])"
```

`grep -n "famil|represented_count|common_conditions" docs/symbolics/dsh-protocol.md⟫ → **no match** (only `revision⟫ at line 104 and `unrepresented_count⟫ at line 150). The doc's value-form list and prose enumerate `status, domain, complete, completeness, solutions, unrepresented_reason, diagnostics⟫ and assert "An agent can answer, from structure alone: … **how many solutions?** (`solutions.shape⟫)". `solutions.shape⟫ is `[0]⟫ here while the true count is infinite. The kernel's mathematical answer (the `families⟫ entry `k*pi⟫, verified against SymPy `{2kπ} ∪ {2kπ+π}⟫) is **correct**; the contract's decoding table is what fails. A reviewer may re-grade this to P2 as a documentation defect.

### F5 — P2 — `TransformResult.changed⟫ is `true⟫ for expressions that were not rewritten at all, whenever the expression contains a power node: `original⟫ and `expression⟫ are identical, `steps⟫ is empty and `conditions⟫ is empty.

**Reproduction (second run `%TEMP%\b4audit-repro\F2_changed⟫; first run `F03/F07/N01/N04/N05⟫ in `%TEMP%\b4audit⟫):**

```
> …Run.exe --file %TEMP%\b4audit-repro\F2_changed.ls --omit-functions
   script: x = symbol("x"); simplify_full(x^2)
"display":"TransformResult(status: Satisfied, original: x^2, expression: x^2, changed: True, conditions: [],
 steps: [], budget_exceeded: False, budget_kind: , diagnostics: [])"
```

Controls in the same probe set: `simplify_full(2*x)⟫ → `changed: False⟫; `simplify_full(sin(x))⟫ → `False⟫; `simplify_full(x+1)⟫ → `False⟫; `simplify_full((x+1)*(x-1))⟫ → `False⟫; `simplify_full(abs(x))⟫ → `False⟫; but `simplify_full(x^2+1)⟫, `sqrt(x^2)⟫, `sqrt(x^3)⟫, `x^(1/3)⟫, `sqrt(x^4)⟫, `x^(2*a)⟫, `abs(x)^4⟫ → `changed: True, steps: []⟫.

**Root cause (source read at 9e761ba):** `SymbolicsPlugin.cs⟫ computes `changed = !r.Expression.Equals(e)⟫, and `PowerExpr.Equals⟫ (`Lovelace.Symbolics/Expr.cs:309⟫) is `other is PowerExpr p && p.Base == Base && p.Exponent == Exponent⟫ — `Expr⟫ declares no `operator ==⟫ (only `RealLiteral⟫ does, Expr.cs:164), so this is **reference** comparison, unlike `AddExpr⟫/`MultiplyExpr⟫/`FunctionExpr⟫ which use `SequenceEqual⟫. Any rebuilt (not shared) child makes an otherwise equal power tree compare unequal. No wrong value follows (verified in \1.2), hence P2, but the field contradicts `steps⟫/`expression⟫ inside the same record.

### F6 — P2 — The universal identity `sin(u)^2 + cos(u)^2 → 1⟫ fires only for some argument shapes: it is missed for `u = x+1⟫, `2*x⟫, `x/2⟫, `abs(x)⟫ while firing for `u = x, x^2, x*x, exp(x), sin(x)⟫.

**Reproduction (second run `%TEMP%\b4audit-repro2\TRG⟫; shape sweep `V1–V5⟫ and `TR1–TR4⟫ in the same fresh directory):**

```
> …Run.exe --file %TEMP%\b4audit-repro2\TRG.ls --omit-functions
   script: x = symbol("x"); [simplify_full(sin(x)^2 + cos(x)^2), simplify_full(sin(x+1)^2 + cos(x+1)^2)]
"display":"[TransformResult(status: Satisfied, original: cos(x)^2 + sin(x)^2, expression: 1, changed: True,
 conditions: [], steps: [RewriteStep(rule_id: trig.pythagorean-sin2-cos2, classification: Universal,
 before: cos(x)^2 + sin(x)^2, after: 1, required_conditions: [])], budget_exceeded: False, …),
 TransformResult(status: Satisfied, original: cos(x + 1)^2 + sin(x + 1)^2,
 expression: cos(x + 1)^2 + sin(x + 1)^2, changed: True, conditions: [], steps: [], budget_exceeded: False, …)]"
```

Shape sweep (each a separate probe, `simplify_full(sin(u)^2+cos(u)^2)⟫): `u=x⟫→1; `u=exp(x)⟫→1; `u=sin(x)⟫→1; `u=x^2⟫→1; `u=x*x⟫→1 (canonical `x^2⟫); **`u=x+1⟫→unchanged**; **`u=2*x⟫→unchanged**; **`u=x/2⟫→unchanged**; **`u=abs(x)⟫→unchanged**. The canonical factor order is the one the pattern expects (`cos⟫ first) in every case, so order is not the explanation.

**Root cause (source-inferred?):** the rule's applicability lambda (`Simplify.cs:222⟫) is `(m, c) => m.Get("x") == m.Get("y")⟫; `Expr⟫ has no `operator ==⟫, so this is reference equality on the two matched subtrees, true only when the matcher hands back the *same instance*. The observable (fires for some argument shapes, not others) is certain; the instance-identity mechanism is a source-level inference I could not instrument in the frozen binary. Impact: a `Universal⟫-classified rewrite silently does not apply to most compound arguments — no wrong value, but a machine-visible false negative in `steps⟫.

### F7 — P2 — `rat.cancel-x-over-x⟫ fires for `x/x⟫, `sin(x)/sin(x)⟫, `abs(x)/abs(x)⟫, `abs(x+1)/abs(x+1)⟫, `exp(x+1)/exp(x+1)⟫ but never for `(x+1)/(x+1)⟫ or `(x^2+1)/(x^2+1)⟫, and never for any quotient whose denominator power has exponent ≠ −1 (`x^2/x^2⟫, `0/x^2⟫, `x^3/x^2⟫, `(2*x)/(2*x)⟫).

**Reproduction (second run `%TEMP%\b4audit-repro2\CAN⟫; first runs `S02/S04/S06⟫ in `%TEMP%\b4audit⟫, isolation controls `X01/X02/X03⟫ in `%TEMP%\b4audit-repro⟫):**

```
> …Run.exe --file %TEMP%\b4audit-repro2\CAN.ls --omit-functions
   script: x = symbol("x"); [simplify_full(x/x), simplify_full((x+1)/(x+1))]
"display":"[TransformResult(status: Satisfied, original: x/x, expression: 1, changed: True,
 conditions: [x != 0], steps: [RewriteStep(rule_id: rat.cancel-x-over-x, classification: Conditional,
 before: x/x, after: 1, required_conditions: [x != 0])], budget_exceeded: False, …),
 TransformResult(status: Satisfied, original: (x + 1)/(x + 1), expression: (x + 1)/(x + 1), changed: True,
 conditions: [], steps: [], budget_exceeded: False, …)]"
```

Isolation: `simplify_full(abs(x+1)/abs(x+1))⟫ → `expression: 1, conditions: [abs(x + 1) != 0], steps: [RewriteStep(rule_id: rat.cancel-x-over-x…⟫ and `simplify_full(exp(x+1)/exp(x+1))⟫ → `expression: 1, conditions: [exp(x + 1) != 0], …⟫ — so the equality machinery does work for `AddExpr⟫ arguments and the failure for `(x+1)/(x+1)⟫ is the **canonical factor order** (there the inverse-power factor sorts first: `(x + 1)^-1*(x + 1)⟫, observed verbatim in `X03⟫/`S02⟫), i.e. the rule's pattern is positional while the canonical order is not. The exponent ≠ −1 half is a rule-coverage gap consistent with the rule's name (`x-over-x⟫), reported for completeness. Consequences are visible in the solver too: `solve_full(x^2/x^2 == 1, x)⟫ is `Unevaluated⟫. No wrong value: at every point where the input is defined the output equals it.

### F8 — P2 — The algebraically identical equations `(x-1)^2 == 0⟫ and `x^2-2x+1 == 0⟫ produce different solution records: two `Solution(value: 1, multiplicity: 1)⟫ entries with `represented_count: 2⟫ versus one `Solution(value: 1, multiplicity: 2)⟫ with `represented_count: 1⟫.

**Reproduction (second run `%TEMP%\b4audit-repro\F7_sq⟫ / `F7_exp⟫; first run `L09/L10⟫ in `%TEMP%\b4audit⟫):**

```
> …Run.exe --file %TEMP%\b4audit-repro\F7_sq.ls --omit-functions     script: x = symbol("x"); solve_full((x-1)^2 == 0, x)
"SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
 solutions: [Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact),
             Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact)],
 families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, …)"

> …Run.exe --file %TEMP%\b4audit-repro\F7_exp.ls --omit-functions    script: x = symbol("x"); solve_full(x^2 - 2*x + 1 == 0, x)
"SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
 solutions: [Solution(value: 1, conditions: [], multiplicity: 2, exactness: Exact)],
 families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, …)"
```

Ground truth `sympy.roots((x-1)**2)⟫ = `{1: 2}⟫ and `sympy.roots(x**2-2*x+1)⟫ = `{1: 2}⟫. The multiplicity *sum* is right in both encodings but `represented_count⟫/`solutions.shape⟫ are not: an agent reading either field alone concludes "2 distinct roots" in one case and "1 root of multiplicity 2" in the other for the same solution set. Not a wrong value; an inconsistency in the machine-visible record.

### F9 — P2 — Re-declaring a name with a different domain does not change (or contradict) the earlier declaration: `y = symbol("x", complex); x = symbol("x", real); simplify(sqrt(y^2))⟫ answers `abs(x)⟫ using `Real(x)⟫, and `x = symbol("x"); assume(x > 0); y = symbol("x", complex); simplify(sqrt(y^2))⟫ answers `x⟫.

**Reproduction (second run `%TEMP%\b4audit-repro\F6_sym⟫; first run `M01/M02/H14⟫ in `%TEMP%\b4audit⟫):**

```
> …Run.exe --file %TEMP%\b4audit-repro\F6_sym.ls --omit-functions
   script: x = symbol("x", real); y = symbol("x", complex); simplify(sqrt(y^2))
"display":"abs(x)","typed":"abs(x) (Symbolic)","structured":{"kind":"Symbolic","pretty":"abs(x)",
 "canonical":"(fn abs (sym x))","domain":"real","exact":true,"nodeCount":2,"freeSymbols":["x"]}
 (the same call with assume(x > 0) instead returns "x")
```

`simplify_full⟫ records the step as `pow.sqrt-square-real … required_conditions: [DomainCondition(variable: x, domain: real)]⟫, i.e. the *complex* declaration of `y⟫ is not in force. `AssumptionSet.Add⟫ throws `AssumptionContradictionException⟫ on a contradicting `assume()⟫ (verified HELD, A1–A3/A7/H16) and the transform path turns a refuted condition into `TransformResult.Unsatisfiable⟫ (A12) — so the *contradiction protocol* works, while a conflicting **symbol-domain declaration** is silently absorbed. Symbols are keyed by name (`Context.cs:18⟫, `Symbol.Equals⟫ compares `Name⟫ only — source read); whether that is intended shadowing is not stated in the contract, hence "no wrong answer under the kernel's own model" → P2.

---

## 3. COULD NOT DECIDE

1. **`transform⟫ is not a builtin.** The task names `simplify_full⟫/`transform⟫/`factor⟫/`expand⟫/`apart⟫. No `transform⟫ appears in `capabilities()⟫ or the builtin registry; `simplify_full⟫ is the `TransformResult⟫ producer, and every probe above used it. A separate `transform⟫ entry point could not be attacked.
2. **`revision⟫ semantics.** The field varies with the script (empty 80, `1+1⟫ 81, one symbol 82, two symbols 83, `capabilities()⟫ 82) — it looks like a per-symbol-registry revision counter. dsh-protocol.md lists it as always present but never defines it, so I can neither confirm nor falsify it. INCONCLUSIVE, cosmetic.
3. **Mechanism of F6** (reference equality via `m.Get("x") == m.Get("y")⟫): the source text is unambiguous, but I could not instrument the frozen binary to prove that the matcher returns distinct instances for `x+1⟫; the *behavioural* half of F6 is reproduced twice and is not in doubt.
4. **Exact `--print-budget⟫ floor rule.** Observed: truncation starts at nodeCount 7 and never occurs at ≤ 6 for any budget ≥ 1. Whether the intended rule is "nodeCount > 6" or "rendered length > head + budget" could not be settled from outside; only the contradiction with the doc at ≤ 6 nodes (F3) is asserted.
5. **Whether `solutions⟫ is a set or a multiset** (F8) is not stated anywhere in the contract, so the *severity* of the duplicate-record encoding is a judgement call; the inconsistency between the two encodings is not.
6. **`solve()⟫ (non-full) returns `Text⟫ for family answers** — e.g. `solve(sin(x)==0, x)⟫ → `"k*pi for integer k"⟫ (a Text value) while `solve_full⟫ returns structure. dsh-protocol.md permits `Text⟫ as a scalar kind and `solve_full⟫ is the documented structured surface, so no verdict; noted as a place where meaning is only in display text.
7. **`evalf⟫ precision ceiling** — `evalf(1/3, 1000000000)⟫ silently returns 1000 digits. Outside the rewrite/solve scope and not covered by dsh-protocol.md; deferred to the numeric audit (A1).
8. **`simplify(x^2 == x^2)⟫** stays an unevaluated relation instead of folding to true. Plausibly the same `==⟫-class defect, but I did not run the `simplify(x == x)⟫ control that would make it a finding.
9. **Floating-point residuals of exact algebraic roots** (`evalf(subs(x^3-1, x, root))⟫ ≈ 9.7e-21 at 20 digits) are consistent with floating-point evaluation of an exact expression, not with a wrong root; I had no documented precision contract for `evalf⟫ to test against.
10. **The kernel's `1/(1/x) → x⟫ constructor fold** extends the domain at x=0 (the input has no value there). Recorded as P3; whether the constructor is allowed a definedness delta that the rewrite rules are denied is not documented.

---

## 4. Summary

**≈330 distinct probes** (one binary invocation per probe script; ≈390 invocations including every mandated re-run) were executed against `out/aot/Lovelace.Run.exe⟫, published from `9e761ba⟫, with SymPy 1.14.0 as independent ground truth. **≈261 probes HELD**, **9 findings (≈57 probes)** and **≈12 probes were INCONCLUSIVE**. The strongest positive result is the value-preservation differential: for all 30 rewrite/factor/expand/apart/cancel cases the kernel's output equals its input at every one of 18 exact rational *and* complex points, and the four conditional rewrites (`sqrt(x^2)⟫, `abs(x)^2⟫, `sqrt(x^2)⟫ under x>0 / x≥0, `log(exp(x))⟫) differ from the input *only* at points their own reported conditions exclude — the rewrite kernel does not fabricate values. The assumption protocol also held everywhere it was attacked: contradictions (relational, equality, conjunction, negation, and the transform path's refuted conditions) are reported as `UnsatisfiableAssumptions⟫/`transform.unsatisfiable-conditions⟫ rather than absorbed, `assume_clear⟫ leaves no residue, x≥0 with x≤0 is correctly *not* a contradiction, and the 400-step rewrite budget, the 512-depth refusal and the arity/type error envelopes all behaved as documented. The failures cluster in three places: **one wrong completeness claim (P0, F1: the exp family over ℂ)**, **two documented-contract breaches (P1: exit code 2 never produced, F2; `--print-budget⟫ inert below 7 nodes, F3)** plus **one contract-vs-record gap (F4: `families⟫ absent from the protocol, so the documented `solutions.shape⟫ decode under-reports infinite solution sets)**, and **five internal inconsistencies (P2: F5 `changed:true⟫ with zero steps on power nodes; F6 the `sin²+cos²⟫ rule missing compound arguments; F7 order/coverage gaps in quotient cancellation; F8 duplicate solution records for a double root; F9 a conflicting symbol-domain declaration silently absorbed)**. Root causes for F5 and F6 are the same class of defect — `==⟫ (reference equality) where `.Equals⟫ (structural) is meant, `Expr⟫ declaring no `operator ==⟫ — and every P0/P1 is reproducible from a clean temp file with the raw envelopes quoted above.
