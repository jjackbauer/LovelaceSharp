# Audit B — the precision-and-exactness lattice (cycle 6, round 13)

**Falsifier strategy**: PRECISION-AND-EXACTNESS-LATTICE. I tried to break three invariants directly —
(1) an INEXACT/TRUNCATED value must never cross as `exact:true`, and an `exact:true` Real must never
carry a rational form that is not exactly the value; (2) a nonzero magnitude must never cross as `0`
unless the flag is honest; (3) the digits a result delivers must match mpmath at that many digits.

**Binary**: `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` (published Native AOT, final
tree, `revision:81`, `protocolVersion:1`). **Ground truth**: mpmath 1.3.0 / SymPy 1.14.0 at
`C:\Users\ricar\dev\.lovelace-tools\python`, always evaluated at mp.dps ≥ 2400 (or places+80) so the
comparison's own precision is never the limit. **Every envelope below is verbatim, trimmed to the
deciding fields.**

**Reading of "digits" (stated once, used everywhere).** I used **R1 = decimal places after the point**,
because that is the wire's own documented reading: the `evalf` descriptor says "to the given number of
decimal places" (`Lovelace.Symbolics/SymbolicsPlugin.cs:489`) and the implementation bounds it with
`Math.Min(digits, 1000)` (`SymbolicsPlugin.cs:1554-1563`). For every comparison I computed mpmath's value
truncated to N places after the point and compared the wire's Nth decimal digit against mpmath's Nth
decimal digit. Two boundary readings are reported explicitly where they matter (L3, L4). Note that a
sibling round-13 audit (`audit-A-metamorphic.md`) measured a place where R1 and R2 (significant digits)
diverge for |value| ≥ 1; none of my findings depend on that divergence — for the values I checked
(`pi`, `e`, `sqrt(2)`, `sin(1)`, `log(2)`, truncated constants) the wire's answer **is** an
N-decimal-place truncation and I verified it digit by digit.

**What was swept** (batched through `--stdin`, one process per ambient precision; every probe's digits
compared to mpmath):
`setprecision(P)` × `evalf(f, d)` over P ∈ {1,2,3,4,5,8,10,12,15,17,20,25,28…36,40,45,49,50,51,60,64,77,
80,90,99,100,101,102,110,120,128,150,200,256,300,400,500,512,600,700,777,800,900,999,1000,1001,1024,1100,
1280,1500,1800,1999,2000}, d ∈ {1,2,3,5,8,10,15,17,20,25,30,31,32,40,49,50,60,64,80,90,99,100,101,110,150,
200,500,999,1000,1001,1100}, f ∈ {pi, e, sqrt(2), sin(1), log(2), exp(1), 1/3, 2/3, pi(30), e(49),
sin(pi(30)), 10^±1000·pi, exp(±1000), sqrt(10^-1000), log(10^-1000), 1/(3·10^1000)}, plus the bare
constants, `pi(P)`/`e(P)` for P up to 2000, arithmetic mixing exact and inexact operands
(`evalf(pi,40)+1/3`, `evalf(pi,40)*1/3`, `2*evalf(pi,40)`, `evalf(pi,40)+pi(30)`, `evalf(pi,40)-evalf(pi,30)`,
`evalf(sqrt(2),40)^2`, `evalf(1/3,40)*3`), and the `print` channel of the same values. ≈1 450 envelope
evaluations; every result digit-compared. A zero result was only ever charged as a defect when its flag
was dishonest — every zero I found carries `exact:false` and is listed under "documented bounds" below.

---

## L1 (P1) — the machine-readable payload silently truncates every Real at 100 decimals, while the same value carries (and the same envelope prints) the full count

**Command (ran verbatim, exit 0, 2 016-byte envelope):**

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "setprecision(1100); print(pi(1100)); pi(1100)" --json --omit-functions --omit-variables
```

**Observed envelope (trimmed to the deciding fields; the two values are from ONE envelope):**

```json
{"ok":true,"revision":81,
 "result":{"kind":"Real","display":"3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679",
 "structured":{"kind":"Real","value":"3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679","exact":false}},
 "output":["3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679…28347913151"]}
```

`output[0]` has **1 100 decimals**; `result.structured.value` has **100 decimals** (102 chars). No
`truncated`, `truncationReason` or `budget` field is present — the Real element carries only
`{exact, kind, value}`.

**Second, independent reproduction** (stdin script, so `--omit-functions --omit-variables` only; same binary):

```
setprecision(2000); print(pi(2000)); print(e(2000)); pi(2000)
```

`print(pi(2000))` → 2 000 decimals, **every digit equal to mpmath's** (verified position by position at
mp.dps 2060); `print(e(2000))` → 2 000 decimals, all equal to mpmath; the payload `pi(2000)` → **100**
decimals, `exact:false`. Same for P = 101, 200, 500, 1000, 1100, 1500 (payload 100 decimals; print = P
decimals, all verified). And through `evalf`: at `setprecision(1100)` and `setprecision(2000)`,
`evalf(pi, 1000)`, `evalf(e, 1000)`, `evalf(sqrt(2), 1000)` each answer **100** decimals; at
`setprecision(101)`, `pi(101)` answers 100 decimals while `print(pi(101))` answers 101. At
`setprecision(2000)`, `evalf(10^1000 * pi, 2000)` answers 1 001 integer digits and **100** decimals.

**What the correct behaviour is, and how I know.**
(a) The value is not clamped — the digits exist. The same envelope's own `output` channel prints 1 100 /
2 000 decimals for the *same expression*, and every one of those digits equals mpmath. The product's own
unit test asserts the value carries them: `Lovelace.Suite.Tests/InterpreterBuiltinSetprecisionTests.cs:61-79`
("Lifting the cap above the 1000 default lets pi(n) compute more digits", `Assert.Equal(-1100L,
result.AsReal().Exponent)`).
(b) The machine API promises the count. `evalf`'s descriptor: "Numerically evaluates a symbolic
expression to the given number of decimal places. **The count is honoured for an already-numeric
argument too**: such an argument is truncated to the requested count instead of being passed through at
the ambient precision" (`SymbolicsPlugin.cs:489`); the code bounds it only by the 1000-place computation
cap (`SymbolicsPlugin.cs:1554-1563`, `Math.Min(digits, 1000)`); `pi`/`e`/`setprecision` descriptors at
`Lovelace.Suite/CoreBuiltinMetadata.cs:41-46`; and EVD-276 states the operative bound is the 1000-place
cap, with `setprecision(2000); evalf(1/(3*10^999), 1000)` rendering at 1000 places
(`docs/goal-cycle-6/evidence.md:48`).
(c) The truncation is silent. The DTO *has* the reporting fields — `Truncated`, `TruncationReason`,
`Budget`, commented "set only when a print budget abbreviated the rendering (never silently)"
(`Lovelace.Suite/StructuredProjection.cs:33-36`) — but the Real branch never sets them: it emits
`value.ToString()` with `Exact` (`StructuredProjection.cs:168-182`), and `Real.ToString()` caps the
fractional part at `DisplayDecimalPlaces` (`Lovelace.Real/Real.cs:2474-2475`, and the all-fractional
twin at :2460-2467), whose default is 100 (`Real.cs:43`). The payload is projected after the statement's
precision scope has been left, so the cap in force is the default 100, not the script's `setprecision`.
The human `print` channel is rendered inside the scope (`Interpreter.cs:1764-1778`,
`ValueFormatter`) and shows the full value.

**New or recorded?** **Partly new.** The *symptom* is recorded in cycle 5 —
`docs/goal-cycle-5/audit2/B1-differential.md:1885` "**F9 (P1)** — `pi(d)` and `e(d)` silently cap at 100
decimals, and `evalf` ignores `digits` for an already-numeric argument", with "No document states a
maximum for `pi(digits)` or `e(digits)`" (:2061), and cycle 5 ordered the `digits` contract fixed
(`docs/goal-cycle-5/journal.md:425-431`, DEC-008). It is **not** recorded anywhere in
`docs/goal-cycle-6/` (grep for `100 decimals`, `cap at 100`, `silently cap` over `docs/goal-cycle-6` →
0 hits) and not in `evidence.md` or `a-plus-cycle-6-amendment.md`, whose P.2 states "none outstanding".
What is **new** is the attribution and the scope: F9 charges a clamp inside `pi(d)`/`e(d)`; the clamp is
in fact in the wire's payload projection — the value is unclamped and the same envelope proves it. That
makes the defect strictly wider than F9's (every Real in `result`, including `evalf` results, vector
elements and 10^±1000 magnitudes) and it explains why `setprecision(n)` appears not to work above 100.
I did not re-run the cycle-5 probe list; this came from the lattice.

---

## L2 (P0) — at the ambient-precision boundary the *leading* significant digit of a below-resolution value is wrong

**Command (ran verbatim, exit 0, 616-byte envelope):**

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "setprecision(31); evalf(sin(pi(30)), 40)" --json --omit-functions --omit-variables
```

**Observed envelope (verbatim, trimmed):**

```json
{"ok":true,"revision":81,"result":{"kind":"Real","display":"0.0000000000000000000000000000004",
 "structured":{"kind":"Real","value":"0.0000000000000000000000000000004","exact":false}}}
```

The wire's 31st decimal digit is **4**. At `setprecision(32)` the same call answers
`0.00000000000000000000000000000050`→ the wire gives `…000049` (digit 31 = 4, digit 32 = 9).

**Ground truth (mpmath 1.3.0, independent):**

```
from mpmath import mp, mpf, sin as MSIN
mp.dps = 2400
p30 = mpf('3.141592653589793238462643383279')
s = MSIN(p30); print(mp.nstr(s, 80))
```
```
sin(pi(30)) = 5.0288419716939937510582097494459230781640628620899862803482532092112635556795782e-31
31-decimal truncation: 0.0000000000000000000000000000005
32-decimal truncation: 0.00000000000000000000000000000050
60-decimal truncation: 0.000000000000000000000000000000502884197169399375105820974944
```

The true leading significant digit is **5**; the wire publishes **4** — a 20 % relative error in the
value's first meaningful digit, and neither the truncation nor the rounding of the true value. The
requested count was **40 decimal places**, so the digit is well inside the resolution the caller asked
for; the ambient budget was 31 digits, which is why "the last digit is unreliable" does not cover it.

**Why this is a defect and not the documented bound:** the same expression at `setprecision(30)` answers
`{"value":"0","exact":false}` — i.e. the engine's own convention for "below the active budget" is to
answer zero with an honest flag (EVD-276, `docs/goal-cycle-6/evidence.md:48`; Language.md "transcendentals
truncate at the active budget"). At 31 the value *is* representable (its first digit sits exactly at the
budget) and the engine answers a **wrong nonzero digit** instead of zero or the right digit. The flag is
`exact:false`, so the wire does not over-claim exactness — it simply publishes a wrong value.

**Reproduced twice**, in two independent runs (a 171-probe batch and a single-probe CLI run), with
identical output.

**New?** **New.** Nothing in `docs/goal-cycle-6/evidence.md` or `a-plus-cycle-6-amendment.md` records a
wrong digit for this value; grep for `50288419716939937510582097494` over `docs/` returns only EVD-276's
quoted 60-digit string. EVD-276 examined this expression at `setprecision(60)` only and concluded "no
defect"; the 31/32 window is not covered. The sibling round-13 audit `audit-A-metamorphic.md:124` discusses
the same family for a *different* direction (|value| ≥ 1, significant-vs-decimal) and does not record this.

---

## L3 (P1) — the last delivered decimal at the ambient boundary is one off, including the value EVD-276 certifies as matching mpmath

**Command (ran verbatim, exit 0, 703-byte envelope):**

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "setprecision(60); evalf(sin(pi(30)), 60)" --json --omit-functions --omit-variables
```

**Observed envelope (verbatim, trimmed):**

```json
{"ok":true,"revision":81,"result":{"kind":"Real",
 "display":"0.000000000000000000000000000000502884197169399375105820974943",
 "structured":{"kind":"Real","value":"0.000000000000000000000000000000502884197169399375105820974943","exact":false}}}
```

The wire's 60th decimal is **3**. mpmath's 60-decimal truncation (command above) is
`0.0000000000000000000000000000005028841971693993751058209749**44**` — the 60th digit is **4**, and the
following digits are `45923…`, so 4 is also the correctly rounded digit (3 is neither truncation nor
rounding of the true value).

**Second and third reproductions** (same class, independent runs):
* `setprecision(2000); evalf(sin(pi(30)), 40)` → 2 000 decimals with **exactly one** wrong digit, at
  position 2000: wire `…95066012699586268897063780727**8**`, mpmath (mp.dps 2300)
  `…95066012699586268897063780727**9**`.
* `setprecision(1100); evalf(sin(pi(30)), 40)` → 1 100 decimals, one wrong digit, at position 1100.
* In the batched lattice the same last-place error appears at P = 33 (digit 33 = 1, true 2), 34 (7 vs 8),
  35 (7 vs 8), 36 (3 vs 4), 45 (8 vs 9), 50 (position 49/50).

**Correct behaviour / how I know:** the invariant is that the delivered digits are the value's digits;
the independently computed value's digits are above. The error is one unit in the last delivered place,
so it is a wrong digit rather than a wrong magnitude — but it is published with no marker, and it is
exactly the digit counted by "at that many digits".

**New?** **New, and it contradicts a recorded claim.** EVD-276 (`docs/goal-cycle-6/evidence.md:48`) quotes
this very envelope and states it "is mpmath's `5.0288419716939937510582097494448…e-31` **to the digits
printed**". The last printed digit is 3 and mpmath's is 4, so the recorded claim is false in its last
place. No other document records a wrong trailing digit at the ambient boundary.

---

## L4 (P2) — the requested digit count is not honoured in the other direction: all-fractional values come back at the ambient digit count

**Command (ran verbatim, exit 0, 6 518-byte envelope, 6.22 s):**

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "setprecision(2000); evalf(sin(pi(30)), 40)" --json --omit-functions --omit-variables
```

**Observed (deciding field, verbatim):** `"structured":{"kind":"Real","value":"0.000…(2 000 decimals)…807278","exact":false}`
— a **40**-decimal-place request answered with **2 000** decimals. Same shape at
`setprecision(1100)` (1 100 decimals for the same request); `evalf(sin(10^-1000), 50)` → 1 030 decimals;
`evalf(log(10^-1000), 20)` → 30 decimals; `evalf(sin(pi(30)), 40)` at `setprecision(100)` → 100 decimals.

**Correct behaviour:** the descriptor promises "to the given number of decimal places"
(`SymbolicsPlugin.cs:489`). The extra digits are not corrupt — at mp.dps 2300 I compared all 2 000 of them
and found the last digit only (L3) matching-boundary difference — but the count is not what was asked, and
the returned answer's length is a function of the ambient precision instead of the argument.
**Cause (read, not fixed):** `Lovelace.Real/Real.cs:2460-2467` — the all-fractional branch takes
`padded[..Math.Min((int)fracLen, (int)(fracLen + DisplayDecimalPlaces))]`, which is always `fracLen`, so
`DisplayDecimalPlaces` is never applied on that branch (contrast :2474-2475).

**New?** Not recorded in `docs/goal-cycle-6/`; not covered by the sibling audits in round 13.

---

## L5 (P2) — asking `pi(n)`/`e(n)` for more digits than the engine precision is refused with a raw framework message

**Command (ran verbatim, exit 1, 685-byte envelope):**

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "setprecision(20); pi(30)" --json --omit-functions --omit-variables
```

**Observed envelope (verbatim, trimmed):**

```json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"Specified argument was out of the range of valid values. (Parameter 'digits')",
 "recoverable":true,"diagnostics":[{"message":"Specified argument was out of the range of valid values. (Parameter 'digits')","position":0,"line":1,"column":1}]}
```

Measured boundary: `pi(20)`@P=20 answers, `pi(21)`@P=20 is refused, `pi(30)`@P=29 is refused and @P=30
answers, `e(40)`@P=40 answers and `e(41)`@P=40 is refused. The refusal also swallows *valid* `evalf`
calls, because the intermediate constant is built first: at P = 1…20 `evalf(pi(30), 10)` is refused (and
P = 1…40 for `evalf(e(49), 10)`), which is why those two shapes could not be swept below P = 30/49.

**Correct behaviour / how I know:** the project's contract for an argument refusal is that the message
names the builtin, the argument, the accepted range and what arrived — asserted for every shape at
`Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs:21-27, 55-59`, and implemented for `evalf` at
`SymbolicsPlugin.cs:1735-1738`. This message names neither the builtin, nor the argument, nor the range,
nor the value; it is the raw `ArgumentOutOfRangeException` text with a parameter name the caller never
supplied. The **refusal itself** (more digits than the active engine cap) is defensible and I do **not**
charge it; only its shape is charged, plus the fact that neither `capabilities()` (19 entries) nor the
`pi` descriptor (`CoreBuiltinMetadata.cs:41-42`, "π to the requested precision") announces it.

**New?** Partly recorded: the same raw message is recorded in cycle 5 for `pi(0)`/`pi(-1)`
(`docs/goal-cycle-5/audit2/B1-differential.md:1443-1448`, `audit1/A3-symbolic.md:446`) — the *trigger*
`n > engine precision` is not recorded, and cycle 6's `capabilities()` honesty sweep (19/19, EVD-260) does
not list this refusal.

---

## Invariants that held (I tried to break them and could not)

* **No inexact value crossed as `exact:true`.** In the whole lattice — constants × P ∈ {50,60,64,80,100} ×
  d ∈ {10,30,50,100}, the truncated constants `pi(30)`/`e(49)` at P ∈ {40,50,64,100,101}, and
  `evalf(pi(30), 40)`, `evalf(pi(1), 40)`, `evalf(e(49), 100)` — every result carried `exact:false` and
  **no** `numerator`/`denominator`. P-13 (`762aa8c`) is intact on the published binary:
  `setprecision(2000); evalf(pi(30), 40)` → `{"kind":"Real","value":"3.141592653589793238462643383279","exact":false}`.
* **`exact:true` rational forms are exactly the value.** `1/3` → `{"value":"0.(3)","exact":true,
  "numerator":"1","denominator":"3"}`; `1/(3*10^1000)` → `{"value":"0.000…0(3)","exact":true,"numerator":"1",
  "denominator":"3000…000"}` (1/3·10^1000) — both rational forms are exactly the values the periodic
  notation denotes. Conversely a truncated rational does **not** claim exactness: `evalf(1/3, 5)` →
  `{"value":"0.33333","exact":false}` (no N/D).
* **Every digit delivered up to 100 decimals matched mpmath**, position by position, truncation reading:
  `pi`, `e`, `sqrt(2)` at 50/60/64/80/100 places (bare and via `evalf`), `sin(1)`/`log(2)` via `evalf`
  at 10/30/50/100, `pi(30)` at P = 40…101, `10^1000*pi`, `10^-999*pi` (1 000 places), `exp(1000)`,
  `sin(10^-1000)` (1 030 places), `log(10^-1000)`.
* **The `print` channel is digit-perfect at every precision I could reach**: `print(pi(P))` and
  `print(e(P))` for P ∈ {100,200,300,500,777,1000,1100,1500,2000} — all P digits equal to mpmath's.
* **No nonzero magnitude crossed as `0` dishonestly** (details below), and mixed exact/inexact arithmetic
  stayed honest: `evalf(pi,40)+1/3`, `evalf(pi,40)*1/3`, `2*evalf(pi,40)`, `evalf(pi,40)+pi(30)`,
  `evalf(pi,40)-pi`, `evalf(sqrt(2),40)^2` (1.999…), `evalf(1/3,40)*3` (0.999…) — all `exact:false`, all
  numerically correct to the digits shown.

## Documented bounds — measured, flag honest, **not** charged as defects

Per the brief ("where a value is below the resolution of the requested count, check the FLAG is honest"),
these zeros are the documented decimal-place semantics with its documented 1000-place cap (EVD-276,
`SymbolicsPlugin.cs:1554-1569`, `docs/goal-cycle-6/journal.md:612,625`), and every one of them carries
`exact:false`:

| probe (verbatim CLI `--eval`) | observed | true value (mpmath) | verdict |
|---|---|---|---|
| `setprecision(2000); evalf(1/(3*10^1000), 30)` | `{"value":"0","exact":false}` | 3.333…e-1001 (first digit at place 1001) | honest, documented |
| `evalf(exp(-1000), 50)` | `{"value":"0","exact":false}` | 5.075…e-435 (place 435 > 50) | honest |
| `evalf(10^-1000, 30)` / `evalf(10^-1000 * pi, 50)` / `evalf(sqrt(10^-1000), 40)` | `0`, `exact:false` | first digit at place 1000 / 1000 / 500 | honest |
| `setprecision(30); evalf(sin(pi(30)), 40)` | `{"value":"0","exact":false}` | 5.03e-31, first digit at place 31 > the 30-digit budget | honest (EVD-276) |
| `evalf(pi, 1100)` (and `pi(1101)`) | 100 decimals, not 1 100 | — | **L1**, not the cap: the 1000-place cap would give 1 000, and `print` shows the digits exist |
| `evalf(1/9, 1000)`-style trailing-zero drops (`evalf(pi,100)`@P=50 → 49 decimals, `evalf(sin(1),100)`@P=30 → 29) | value numerically equal to the N-place truncation | — | trailing-digit rendering, numerically harmless; recorded here so it is not counted |

## Final table

| id | severity | new? | one-line repro |
|---|---|---|---|
| L1 | P1 | partly — symptom recorded in cycle-5 F9, never closed, absent from cycle-6; mechanism/scope new | `setprecision(1100); print(pi(1100)); pi(1100)` → `output` 1 100 decimals, `result.structured.value` **100** decimals, no truncation marker |
| L2 | P0 | new | `setprecision(31); evalf(sin(pi(30)), 40)` → `…0004`, mpmath `…0005` (20 % error in the leading digit, inside the requested 40 places) |
| L3 | P1 | new (contradicts EVD-276's "to the digits printed") | `setprecision(60); evalf(sin(pi(30)), 60)` → `…974943`, mpmath `…974944` |
| L4 | P2 | new | `setprecision(2000); evalf(sin(pi(30)), 40)` → 2 000 decimals for a 40-place request |
| L5 | P2 | partly — message recorded for `pi(0)` in cycle 5; this trigger new | `setprecision(20); pi(30)` → `InvalidArgument/TypeMismatch` "Specified argument was out of the range of valid values. (Parameter 'digits')" |

Counts: **P0 = 1, P1 = 2, P2 = 2.**

## Could not check

1. **The `pi(30)`/`e(49)` shapes below the ambient boundary.** `evalf(pi(30), d)` and `evalf(e(49), d)`
   abort the batch for P < 30 (resp. P < 49) because the constants themselves are refused (L5); I could
   not compare their digits at those ambient precisions, only record the refusal.
2. **High-d transcendentals.** `setprecision(1000); evalf(sin(1), 1000)` did not finish in 25 s; measured
   again with a long fence it took **151 953 ms** (single call, exit 0), and `evalf(log(2), 1000)` at
   `setprecision(1000)` took 17.9 s. The d > 100 lattice was therefore carried only for `pi`, `e`,
   `sqrt(2)`, `1/3` and the small-magnitude cases. (Timing is recorded, not charged: no severity class in
   the brief covers cost, and I did not measure a baseline.)
3. **P > 2000 and d > 1100.** Not probed: `setprecision` is not documented above the 1000-place cap and
   `evalf` clamps `digits` at 1000 (`SymbolicsPlugin.cs:1554-1563`), so the region is outside the
   documented lattice.
4. **`variables[]`.** Every run used `--omit-variables` (and one run `--omit-functions`); whether the
   100-decimal payload cap of L1 also applies to variable projections (`Runner.ProjectVariable`) was not
   measured.
5. **Other hosts and modes.** Only the published AOT runner was exercised; `Lovelace.Studio`, the REPL,
   `--text`, `--file` and `--print-budget` were not probed.
6. **The `truncated`/TruncationReason` contract outside Real.** I confirmed the Real branch never sets
   them; I did not re-audit the Symbolic branch's use of the same fields.
