# Cycle-6 · round-13 · audit A — metamorphic / generative falsification

**Target**: the published Native AOT binary `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5 776 896 bytes, 2026-09-12 15:06; envelopes report `mathIrVersion:2`, `symbolicFormatVersion:#!lovelace-sym 1`).
**Strategy**: generate expressions with a seeded PRNG (recorded below) and assert relations that must hold
*between* computations; never compare a single value against a stored table. Independent ground truth:
SymPy 1.14.0 / mpmath 1.3.0 under `C:\Users\ricar\dev\.lovelace-tools\python`.

**Result: 1 new P1 and 1 new P2.** 1 600+ generated metamorphic checks held clean (section "Coverage that
held"), including every solver residual, every printer round trip and every rewrite-soundness check I could
generate. Three further defects I reproduced are **already recorded in cycle-5** and are listed, not counted.

## Method notes (so every number below is reproducible)

* **Generators / seeds** (mulberry32, recorded): rational-function corpus **SEED 20260913** (36 expressions);
  transcendental corpus **SEED 773311** (16 expressions); printer corpus **SEED 424242** (20 expressions);
  solver corpus **SEED 909090** (24 polynomials). Point selection for each corpus: a seeded shuffle of
  `{a/b : b in 1..7, a in -9..9}`, filtered by SymPy to points where the expression is real and finite
  (poles excluded by testing the denominator of `together(e)`).
* **Ground truth**: `sympy.sympify` of the generated expression (with `^`→`**`), substituted with
  `Rational`, evaluated through `sympy.lambdify(x, e, 'mpmath')` at `mp.dps = 200…260`. **Warning learned the
  hard way**: an earlier ground-truth script evaluated the strings with Python's `/`, i.e. in *double*
  precision (`exp(1/3)` became `exp(0.3333333333333333)`), which manufactured four false "violations" that
  vanished once `lambdify/mpmath` was used. Every number below is from the mpmath path.
* **Comparator**: leading-significant-digit agreement (sign and point stripped, digits compared from the
  first). A rewrite/precision check is called a violation only when the compared values differ inside the
  digits the API claims (see A-1).
* **Shell**: Windows PowerShell 5.1. `--eval "<script>"` **cannot** carry a script containing `"`
  (PowerShell strips the inner quotes before the native call; observed:
  `{"ok":false,"code":"InvalidOperation","message":"Unexpected token '' at position 11…"}`), so scripts
  with string literals were fed through `--stdin` from a single-quoted here-string. This is a shell
  limitation, not a product claim; the A-1 reproduction below is quote-free and was also run in the
  `--eval` form exactly as printed.

---

## A-1 — **P1** — `evalf(f, N)` publishes N decimal places but only N − k of them are correct (k = integer digits of the result) whenever the argument is a non-terminating rational

### Exact command

```powershell
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "print(evalf(sinh(34/3), 30)); print(evalf(sinh(34/3), 100)); print(evalf(sinh(11.5), 30))" --json --omit-functions --omit-variables
```

Run twice (exit 0 both times); the envelope differs only in `elapsed`/`elapsedTime`/`timings`.

### Observed envelope (verbatim, trimmed to the deciding fields)

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,
 "output":["41780.548053564065925188446077553412",
           "41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513338634",
           "49357.885500315201914739778692889335"]}
```

The same script through `--stdin` (here-string) is byte-identical in `output` on two runs, and reversing the
statement order changes nothing:

```powershell
$s = @'
print("@1", evalf(sinh(34/3), 100))
print("@2", evalf(sinh(34/3), 30))
print("@3", evalf(sinh(34/3), 30))
'@
$s | & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables
# "@1 41780.548053564065925188446077567339192675834927386225364"  (100 places)
# "@2 41780.548053564065925188446077553412"                      (30 places)
# "@3 41780.548053564065925188446077553412"
```

Ground truth (mpmath 220 dps, `mp.nstr(sinh(mpf(34)/3), 105)`):

```
41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513352561…
```

### What is wrong

* `evalf(sinh(34/3), 30)` = `41780.548053564065925188446077553412` agrees with the truth only through the
  **25th decimal**; the 26th–30th published decimals (`553412`) are wrong (truth: `567339`). The value is
  *not* the correctly rounded 30-decimal value either (that would be `…593391…`, i.e. `…4460 77 567339`).
* `evalf(sinh(34/3), 100)` = `41780.5480535640659251884460775673391926758349273862253644277306454458985775018289223615326471730513338634`
  agrees with mpmath to **95 decimals** (100 requested). So the two `evalf` calls on the **same expression**
  disagree at the 26th decimal, and the smaller request is the wrong one: a metamorphic violation.
* Control with a **terminating** argument, same magnitude, printed by the same command:
  `evalf(sinh(11.5), 30)` = `49357.885500315201914739778692889335` — **all 30 decimals correct**
  (mpmath: `49357.88550031520191473977869288933500256760609…`).
* Measured rule, one process per N (script: `print("@i", N, evalf(sinh(34/3), N))`):

  | N (requested) | decimals printed | decimals correct |
  |---|---|---|
  | 20 | 21 | 15 |
  | 25 | 25 | 20 |
  | 30 | 30 | 25 |
  | 40 | 41 | 35 |
  | 50 | 51 | 45 |
  | 60 | 60 | 55 |
  | 80 | 81 | 75 |
  | 100 | 100 | 95 |

  Correct decimals = **N − 5** at every N, and 5 is exactly the number of integer digits of the result
  (41780). Controls at the same N=30 with a terminating argument (`11.5` → all 30 correct) and with a small
  value (`evalf(sinh(1/3), 30)` = `0.3395405572561501391012606113385`, correct to its printed digits) show
  the loss tracks *non-terminating arguments at magnitude ≥ 1*, i.e. the computation behaves as if it were
  carried at ~N **significant** digits while the output claims N **decimal places**.

### How I know the correct behaviour

* The builtin's own contract, read in the source: `Lovelace.Symbolics/SymbolicsPlugin.cs:488-490` —
  "Numerically evaluates a symbolic expression to the given number of **decimal places**. The count is
  honoured for an already-numeric argument too…". The implementation's precision scope is
  `SymbolicsPlugin.cs:482`: `using (Rl.WithPrecision(digits, Math.Min(digits, 50)))`.
* mpmath 1.3.0 at 220 dps (quoted above) is the independent ground truth for the value.
* The binary's **own** 100-place answer agrees with mpmath, so the engine can compute this value correctly;
  only the N=30 path loses the digits it advertises.

### New?

**New, and it contradicts a conclusion the cycle recorded.** Nothing in `docs/goal-cycle-6/evidence.md` or
`docs/symbolics/a-plus-cycle-6-amendment.md` records this. The nearest recorded item is **P-B4 / OQ-004 /
VAL-004** (`docs/goal-cycle-6/journal.md:601-633`, `evidence.md` EVD-276), and I must be explicit about the
relationship because it is the same *family* (value precision vs the "decimal places" contract):

* P-B4's measured instances are the **tiny-magnitude** direction — `evalf(sin(pi(30)), 40)` → `0`,
  `evalf(1/(3*10^1000), 30)` → `0` — "a magnitude whose leading zeros exceed the requested digit count"
  (amendment, P.2). My instance is the opposite direction: a magnitude **above 1**, where the value is
  printed in full and the **trailing digits inside the requested count are wrong**. Different observable,
  different trigger, no overlap in the examples or the mechanism I measured.
* VAL-004 concluded "the behaviour is the **correctly rounded value at the requested and documented
  resolution**", and OQ-004 recorded that "the builtin's descriptor **and the implementation agree on
  decimal places** (with a 1000-place cap)". `evalf(sinh(34/3), 30)` falsifies both statements as general
  claims: the returned number is not the correctly rounded 30-decimal value, and the implementation does not
  deliver 30 decimal places for |value| ≥ 1. OQ-004's open question ("decimal places or significant digits?")
  is thus not only a documentation question — the implementation is already significant-digit-like for the
  result, while the descriptor promises decimal places.
* I did **not** re-run cycle 5/6 probe lists; this came out of generated expressions.

**Severity rationale (why P1 and not P0)**: the leading digits and the order of magnitude are right; the
error is confined to the digits beyond the engine's real accuracy (here ~1.4e-23 absolute at 4.2e4). This is
the grade the cycle itself gave P-B4 ("OPEN — P1 (value precision, flag honest)"), and I follow that
precedent rather than inflating it. It is nevertheless a wrong value inside the advertised precision, and it
is silent — the envelope carries no precision or accuracy field.

---

## A-2 — **P2** — substitution **at a pole** yields an undefined symbolic form where the same expression written directly is a typed error

### Exact command

```powershell
$s = @'
x = symbol("x")
print(subs(1/(x-1), x, 1))
print(subs((x^2-1)/(x-1), x, 1))
print(type(subs(1/(x-1), x, 1)))
'@
$s | & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables
```

### Observed envelope (verbatim, trimmed)

```json
{"ok":true,"revision":81,"output":["0^-1","0/0","Symbolic"], …}
```

Two controls, same shell:

```
& … --eval "print(0^-1)" --json --omit-functions --omit-variables
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Base cannot be zero. (Parameter 'base')","recoverable":true}

$s = @'
x = symbol("x")
print(evalf(subs(1/(x-1), x, 1), 30))
'@
$s | & … --stdin --json --omit-functions --omit-variables
{"ok":false,"code":"EvaluationError","category":"DomainError","message":"0 raised to a non-positive power.","recoverable":true}
```

Further shapes from the same run: `subs(sin(x)/x, x, 0)` → `0/0`, `subs(log(x), x, 0)` → `log(0)`,
`subs(1/x, x, 0)` → `0^-1`, while `subs(sqrt(x), x, -1)` → `i` and `0^0` → `1`.

### What is wrong and how I know

The language's own convention for the same mathematical object is a **typed error**: `0^-1` written directly
is `InvalidArgument/TypeMismatch "Base cannot be zero"`, and `1/0` is `DivisionByZero`. `subs` instead
returns it as a **Symbolic value** (`type(...)` = `Symbolic`) that survives arithmetic
(`subs(1/(x-1), x, 1) + 0` → `0^-1`) and only detonates later as an unrelated-looking `EvaluationError`
inside `evalf`. The metamorphic relation "substituting a point into f gives the value of f at that point"
is answered with an object that is not a value and not the documented error.

No number is falsified (the engine never claims a finite value at a pole), hence **P2**, not P1: it is an
API-shape inconsistency, not a wrong claim.

### New?

**New route.** The numeric-tier behaviours around it are recorded: cycle-5 `audit2/B3-surface.md:395` probed
`log(0)` (INCONCLUSIVE) and `0^-1` is pinned as an error in `docs/symbolics/a-plus-cycle-4-report.md:204`
and `docs/goal-cycle-3/evidence.md:73`. I found no cycle-5/6 record of the **substitution** route
(`subs` at a pole returning `0^-1`/`0/0`), so I report it and flag the overlap.

---

## Coverage that held (no finding) — generative, seeded, reproducible

| Relation asserted | Generator / seed | Checks | Result |
|---|---|---|---|
| Rewrite soundness at **exact rational points**: `subs(rw(f) − f, x, p)` is exactly `0` for `rw` ∈ {expand, factor, cancel, apart, collect, simplify, simplify_full(f).expression}; every generated value also compared **exactly** against SymPy `Rational` arithmetic | SEED 20260913, 36 rational functions × 3 points | 864 | **0 violations, 0 value mismatches** |
| Printer round trip: `p = print(e)` re-fed as source, then `evalf(subs(p − e, x, a), 30)` (for `integrate`, `evalf(subs(diff(p,x) − e, x, a), 30)`) at the generator's points | SEED 424242, 20 expressions × 9 printable forms × 3 points | 540 | **0 violations, 0 re-parse failures** |
| Solver soundness: every `solve_full(f == 0, x)` root `r` substituted back, engine residual `evalf(subs(f, x, r), 30)`, cross-checked by SymPy evaluating `f` at the root; counts/`completeness` read for every call | SEED 909090, 21 polynomials (quadratics, cubics with rational + `sqrt` roots, `i`-valued roots, triple root) | 55 roots | **0 violations**; independently SymPy: `OK 55 BAD 0` |
| Evaluation agreement: `evalf(subs(f, x, a), 30)` and `(…, 100)` and `evalf(subs(diff(f,x), x, a), 30)` against `lambdify(…, 'mpmath')` at 200 dps | SEED 773311, 16 transcendental compositions × 3 points | 144 | `…,100` ≥ 95 correct sig. digits in all 48; `…,30` wrong at digit 26 in the 2 cases that are finding A-1; `diff` same |
| Cross-precision self-consistency: `evalf(f,100) − evalf(f,30)` | same corpus | 48 | 4 flagged — **all are finding A-1** (2 further cases are A-1 at the diff/digit threshold) |
| Rewrite soundness for transcendental expressions: `subs(simplify(f) − f, x, a)`, `subs(expand(f) − f, x, a)` | SEED 773311 | 96 | **0 non-zero residuals** |
| `simplify` soundness under assumptions: `sqrt(x^2)`, `sqrt(x^4)`, `log(exp(x))`, `exp(log(x))`, `asin(sin(x))`, `acos(cos(x))`, `atan(tan(x))`, `sin(asin(x))`, `log(x^2)`, `sqrt(x^-2)`, … with **no** assumption, with `assume_positive(x)`, and after `assume_clear()` | 15 hand-built traps × 3 assumption states | 45 | Sound: the unsound directions are refused (`sqrt(x^2)` stays) and `assume_positive(x); simplify(sqrt(x^2))` → `x` is correct; `assume_clear()` restores the no-assumption answer exactly |
| RootOf values (the real-only solver path): `evalf(r.solutions[k].value, 25)` and residual of `x^4-5x^2+6` | direct probe | 8 | Values are `-√3, -√2, √2, √3` in increasing order, residual ~1e-27 (root truncated at 25 places) — correct |

## Reproduced, but **already recorded** — not counted as findings

1. `solve_full((x-1)^2*(x+2) == 0, x)` → `Unevaluated / Unknown / 0 solutions` while the algebraically
   identical `solve_full(x^3-3x+2 == 0, x)` → `Solved / Complete` with `{-2, 1×2}`; trigger isolated to a
   **repeated linear factor multiplied by a further factor** (`(x-1)^2`, `(x-1)^3` alone solve;
   `(x-2)^2*(x+3)`, `(x-1)*(x-1)*(x+2)`, `2*(x-1)^2*(x+2)` all fail; `(x^2-2x+1)*(x+2)` solves).
   **Recorded in cycle 5**: `docs/goal-cycle-5/audit1/A3-symbolic.md:138` (t17, "Unevaluated … the expanded
   form works", graded "coverage gap with honest status" P2) and `docs/goal-cycle-5/audit2/B4-rewrite.md:150`
   (D13/J04, "HELD (conservative)"). Not in `docs/goal-cycle-6/`; a re-discovery of a recorded issue.
2. Multiplicity representation is not stable across algebraically identical inputs: `solve_full((x-1)^2==0,x)`
   gives two records with `multiplicity 1`, `solve_full(x^2-2x+1==0,x)` one record with `multiplicity 2`, and
   `solve_full((x-2)(x^2-4x+4)==0,x)` one record with `multiplicity 3`. **Recorded** as cycle-5 F8
   (`B4-rewrite.md:149`, graded P2) and F20 (`audit1/A2-wire.md:41`).
3. Non-integer exponents of a positive base refuse numerically while the solver publishes them as roots
   (`solve_full(x^3-2==0,x)` → `2^(1/3)(±…)`; `evalf` of that root → `UnsupportedOperation "Non-integer
   exponents are not yet supported."`). **Recorded** as section P-A1 / P-A2 and the
   `pow.non-integer-exponent-of-a-positive-base` capability entry.

## Summary

| id | severity | new? | one-line repro |
|---|---|---|---|
| A-1 | P1 | **NEW** (same family as recorded P-B4/OQ-004; contradicts VAL-004's general conclusion) | `--eval "print(evalf(sinh(34/3), 30)); print(evalf(sinh(34/3), 100))"` → `…4460 77 553412` vs `…4460 77 567339…`; mpmath agrees with the 100-place call; correct decimals = N − 5 at every N |
| A-2 | P2 | **NEW route** (adjacent `0^-1`/`log(0)` numeric-tier behaviour is cycle-5-recorded) | `print(subs(1/(x-1), x, 1))` → `0^-1` (a `Symbolic`), while `print(0^-1)` → typed `InvalidArgument` and `evalf` of it → `EvaluationError` |

## Could not check (with the reason)

* **Non-terminating arguments to the other elementary functions at larger N.** I mapped the N → correct
  decimals curve for `sinh(34/3)` (N = 20…100) and spot-checked `exp/cosh/tanh/sin` at N = 30; I did not
  sweep the whole family × N plane. Finding A-1 is therefore stated as a measured rule for the cases cited,
  not as a proven law for every builtin.
* **The mechanism in the source.** I read `SymbolicsPlugin.cs:482` (the `WithPrecision(digits, min(digits,50))`
  scope) and the descriptor at `:488-490`, but I did not trace the loss into `Lovelace.Real`/`ComplexMath`;
  no fix is proposed (out of scope).
* **Roots printed with a non-integer power of a positive base** (`2^(1/3)`, `(sqrt(31/108)-1/2)^(1/3)` from
  `x^3+x+1`): the engine's own `evalf` refuses that shape (`UnsupportedOperation`, recorded P-A1), so those
  roots could not be verified *through the engine*; the 55 roots I did verify are all `sqrt`-expressible.
* **`rootof(p, k)` indexing** is not documented in a way I could bind to SymPy's `RootOf`; I verified the
  RootOf path by value and residual instead (4 roots), not by exhaustively matching the index convention.
* **Definite integrals**: the language exposes only the indefinite `integrate`/`integrate_full`; the
  round-trip relation was checked as `diff(integrate(f,x),x) = f`, and for the 16 transcendental integrands
  `integrate` returned an unevaluated `integrate(...)` node (documented `integration.no-closed-form`), which
  makes that particular round trip vacuous. Of a 30-integrand survey only 8 produced a closed form; those
  8 were value-checked and are correct.
* **Symbolic `jacobian`/`hessian`/`linsolve`/`series` coefficient agreement** were not exercised (time).
* **Unexplained observation, not a finding**: the envelope's `revision` field took the values 80, 81 and 83
  across identical invocations of the same script in separate processes. I did not characterise it, and I
  make no claim about it.
* **Harness limits, not product defects**: launching 24 runner processes back-to-back twice failed at the
  process layer (`0xC0000409` from the job runner; two probes returned `Thread failed to start.`,
  exit 1, and one returned the runner's `0x80000003`). Those three solver probes were re-run inside batched
  processes and are included above; I did **not** count any of them as a product crash.

## Deliverable

This file: `docs/goal-cycle-6/round-13/audit-A-metamorphic.md`. No product, test, fixture or document was
modified; the only file written is this report.
