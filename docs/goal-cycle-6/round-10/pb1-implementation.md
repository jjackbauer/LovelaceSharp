# P-B1 — an inexact argument must not produce an exact result

**Round 10, cycle 6. Status: CLOSED, with the value error at the same root explicitly left open (out of
this round's scope) and two adjacent flag leaks measured but not fixed (not trigonometric entry points).**

| | |
|---|---|
| Defect | P-B1 — the special-angle path hands an EXACT value to an INEXACT argument; the wire publishes a false exactness claim |
| Tree I worked in | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pb1` — `git worktree add .worktrees/c6-pb1 HEAD`, detached at `dfe42cf938d8949b710131e41fe1d9cda8afdf2f` |
| Control tree | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pb1-ctl` — `git worktree add .worktrees/c6-pb1-ctl HEAD`, same commit, product files pristine |
| Main tree | not modified by me except `docs/goal-cycle-6/round-10/**` (the deliverable and its raw logs) |
| Git | nothing added, staged, committed or pushed anywhere; the change is `pb1.patch`, which `git apply --check` accepts against the main tree (exit 0) |

---

## 1. Pre-fix behaviour, re-observed on this tree

### 1.1 The wire (runner built from HEAD source in the scratch worktree)

```
$dll = .worktrees\c6-pb1\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll
dotnet $dll --eval '<expr>' --json --omit-functions --omit-variables
```

| expression | pre-fix `structured` block (verbatim) |
|---|---|
| `evalf(cos(pi(30)), 40)` | `{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}` |
| `evalf(cos(pi(30)), 100)` | `{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}` |
| `evalf(cos(pi(30)*2), 100)` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` |
| `inspect(cos(pi(30))).exact` | `{"kind":"Boolean","value":"false"}` |
| `cos(pi(30))` | `{"kind":"Symbolic","pretty":"-1","canonical":"(real -1)","domain":"real","exact":false}` |
| `evalf(sin(pi(30)), 40)` | `{"kind":"Real","value":"0","exact":false}` |
| `evalf(tan(pi(30)), 40)` | `{"kind":"Real","value":"0","exact":false}` |

The same envelopes were reproduced on the pristine control tree (raw transcript
`wire-prefix-control.txt`), so the route predates this round.

### 1.2 The probe (standalone console app, `.lovelace-c6-pb1-probe\core`, Release)

Raw transcript `probe-core-prefix.txt`, ambient precision 30/60/100 (the argument equals the ambient π,
which is the exactly-zero reduction):

```
=== ambient 30: argument == ambient pi ===
PiTo(30) [premise]      value=3.141592653589793238462643383279   IsExact=False
Cos(PiTo(30))           value=-1                                 IsExact=True      <-- leak
Sin(PiTo(30))           value=0                                  IsExact=False
Tan(PiTo(30))           value=0                                  IsExact=True      <-- leak
Cos(Rl.Pi)              value=-1                                 IsExact=True      <-- leak
Sin(Rl.Pi)              value=0                                  IsExact=False
Tan(Rl.Pi)              value=0                                  IsExact=True      <-- leak
```

Identical rows at ambient 60 and 100. Other pre-fix leaks the sweep measured:

| probe | pre-fix result |
|---|---|
| `Cos(Rl.AsInexact(Rl.Zero))` / `Sin` / `Tan` | 1 / 0 / 0, `IsExact=True` (the `IsZero` shortcut drops the argument's route) |
| `Atan(inexact zero)` | 0, `IsExact=True` |
| `Atan2(inexact zero, 1)` | 0, `IsExact=True` |
| `AsinReal(inexact zero)` | 0, `IsExact=True` |
| `Sin(0 + i·inexact zero)` / `Cos` / `Tan` / `Sinh` / `Cosh` | both components `IsExact=True` |
| `Exp(inexact zero)` (real) | 1, `IsExact=True` — **not fixed, not trig** |
| `Sqrt(0 + i·inexact zero)` (complex) | (0, 0), both components `IsExact=True` — **not fixed, not trig** |

Cross-scale control (ambient 100, argument `pi(30)`): `Cos(PiTo(30)) =
-0.9999999999999999999999999999999999999999999999999999999999998735537421186443267632593548461102582682`,
already `IsExact=False` — the leak is specific to the scale at which the residual is exactly zero.

### 1.3 Independent ground truth (mpmath 110 dps, `C:\Users\ricar\dev\.lovelace-tools\python`)

```
cos(pi(30))        = -0.99999999999999999999999999999999999999999999999999999999999987355374211864432676325935484611025826826606539102
cos(2*pi(30))      =  0.99999999999999999999999999999999999999999999999999999999999949421496847457730705303741938444103307306426156407
sin(pi(30))        =  0.00000000000000000000000000000050288419716939937510582097494459230781640628620899862803482532092112635556795782107435607323242032531660045856
cos(pi(30)) - (-1) =  1.2644625788135567324e-61
```

So `exact:true` was attached to a number that is off `-1` from the 61st decimal.

---

## 2. Root cause (as located, re-read)

* The symbolic evaluator routes the real tier through `ComplexMath.Sin`/`Cos`/`Tan` —
  `Lovelace.Symbolics/Evaluation.cs:335-337`:
  `Tier(a) <= 2 ? FromReal(ComplexMath.Sin(ToReal(a))) : FromComplex(...)`.
* `ComplexMath.PiDigitsFor` (`Lovelace.Complex/ComplexMath.cs:467-475` pre-fix) returns the AMBIENT digit
  count whenever the argument's own fractional places do not exceed it, so at ambient `n` the reduction
  uses a π that is exactly the argument's own truncation: `xr = pi(n)`, `k = 2`, `r = xr − 2·(π/2) = 0`
  EXACTLY, and the series runs on the exact zero.
* The pair that comes back therefore holds the table's `sin = 0`, `cos = −1` — but with the provenance of
  the zero-residual series, not of the argument. `DivideTruncate` answers `0 / anything` with an exact
  `Rl.Zero` (`ComplexMath.cs:332` pre-fix), while the cosine sum starts from `Rl.One` and keeps it exact,
  which is why `Cos(pi(n))` and `Tan(pi(n))` came out exact and `Sin(pi(n))` did not.
* The result then crosses `TruncateFrac` unchanged (`ComplexMath.cs:642`: `storedFrac <= maxFrac` returns
  the value as-is), reaches `evalf` as an exact `Real`, and `SymbolicsPlugin`'s structured projection
  publishes `exact:true` with numerator/denominator.

---

## 3. The failing test, written first

New file `.worktrees/c6-pb1/Lovelace.Complex.Tests/ComplexMathInexactArgumentProvenanceTests.cs`
(12 cases). `Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj` gained one test-only
`ProjectReference` to `Lovelace.Symbolics` so the symbolic-node/numeric-value agreement can be asserted
directly (the product dependency direction is unchanged: Symbolics → Complex).

```
dotnet test Lovelace.Complex.Tests\Lovelace.Complex.Tests.csproj -c Release \
  --filter "FullyQualifiedName~ComplexMathInexactArgumentProvenanceTests"
```

**Before the product change** (raw transcript `prefix-newtests.log`, 11 cases at that point):

```
  Failed ...InexactPiAtItsOwnScale_ProducesInexactResults(scale: 30)   Error Message: cos(pi(30)) = -1 claims exactness
  Failed ...InexactPiAtItsOwnScale_ProducesInexactResults(scale: 60)   Error Message: cos(pi(60)) = -1 claims exactness
  Failed ...InexactPiAtItsOwnScale_ProducesInexactResults(scale: 100)  Error Message: cos(pi(100)) = -1 claims exactness
  Failed ...ExactlyZeroReduction_OfAnInexactAmbientPi_IsNotExact        Error Message: cos(pi) = -1 claims exactness
  Failed ...InexactZeroArgument_DoesNotClaimExactness                  Error Message: sin(inexact zero) = 0 claims exactness
  Failed ...ComplexTrigOfAnInexactZeroPart_IsNotExact                  Error Message: sin(0 + 0i) = (0, 0): the real part claims exactness
  Failed ...SymbolicNodeAndNumericResult_AgreeForTheSameExpression     Error Message: Assert.Equal() Failure: Values differ
Test Run Failed.
Total tests: 11
     Passed: 4
     Failed: 7
```

The 4 that passed pre-fix are the guards (cross-scale `pi(30)` at ambient 100, the exact-zero controls,
the sine agreement, the exact complex zero) — they exist so that "mark everything inexact" cannot satisfy
the file.

**After the product change** (raw transcript `postfix-newtests.log`, then the full 12-case file):

```
Test Run Successful.
Total tests: 12
     Passed: 12
```

---

## 4. The fix (flag only, values untouched)

Full unified diff: `pb1.patch` (25 612 bytes, LF endings, no BOM: `ComplexMath.cs`, the test csproj
and the new test file). Verified applicable: `git -C C:\Users\ricar\dev\LovelaceSharp apply --check
pb1.patch` → exit **0** (`Checking patch Lovelace.Complex/ComplexMath.cs...`, the csproj and the new file).
The product diff is `+89 / −19` across two files; the changed file is
`Lovelace.Complex/ComplexMath.cs` only.

### 4.1 The named path

```diff
@@ SinCosAtPrecision (now lines 542-589)
         if (Rl.IsZero(x))
-            return (Rl.Zero, Rl.One);
+            return x.IsExact
+                ? (Rl.Zero, Rl.One)
+                : (Rl.AsInexact(Rl.Zero), Rl.AsInexact(Rl.One));
@@ the exit (line 588)
-        return (sin, cos);
+        return x.IsExact ? (sin, cos) : (Rl.AsInexact(sin), Rl.AsInexact(cos));
```

* the `IsZero` shortcut (`ComplexMath.cs:548-551`) is now the ARGUMENT's: exact only for an exact zero;
* the exit (`ComplexMath.cs:586-588`) marks both halves inexact whenever the argument is inexact — this
  covers every path out, including the exactly-zero reduction that produced the table value.

### 4.2 The other entry points with the SAME leak, measured in §1.2 and fixed here

A shared helper pair (`ComplexMath.cs:340-358`) states the rule once:

```csharp
private static Rl WithArgumentProvenance(Rl result, Rl argument) =>
    argument.IsExact ? result : Rl.AsInexact(result);
private static Rl WithArgumentProvenance(Rl result, Rl first, Rl second) =>
    first.IsExact && second.IsExact ? result : Rl.AsInexact(result);
private static Complex WithArgumentProvenance(Complex result, Complex argument) =>
    argument.Re.IsExact && argument.Im.IsExact
        ? result
        : new Complex(Rl.AsInexact(result.Re), Rl.AsInexact(result.Im));
```

Applied at:

| entry point | line | why it needed its own guard |
|---|---|---|
| `Tan(Rl)` | `ComplexMath.cs:94` | `DivideTruncate` answers `0 / anything` with an exact `Rl.Zero`, so the quotient laundered the inexact pair |
| `Atan(Rl)` | `ComplexMath.cs:121` | `atan` of an argument that only approximates zero is an exact 0 from the exact-zero shortcut |
| `Atan2(Rl, Rl)` | `ComplexMath.cs:137,139,145` | all three branches, including the `y == 0` return of `Rl.Zero` |
| `AsinReal(Rl)` | `ComplexMath.cs:177` | the quotient reaching `atan` is an exact zero before the series sees it |
| `Sin(Complex)` / `Cos(Complex)` / `Sinh` / `Cosh` | `ComplexMath.cs:231,241,258,268` | both components depend on both parts; the leak is the `RealSinh(0) = 0` / `RealCosh(0) = 1` factor of an exact zero |
| `Tan(Complex)` | `ComplexMath.cs:250` | the division can drop the flag the two halves now carry |

`AcosReal`, `AsinhReal`, `AcoshReal`, `AtanhReal` were measured and already propagate
(`AcosReal(inexact zero)` is inexact because `π/2` is a truncation); they are unchanged.
`DivideTruncate`'s exact-zero shortcut is deliberately untouched — it is the internal series helper and
the boundary rule is enforced at the public entry points instead.

---

## 5. The wire after the fix

Raw transcripts: `wire-prefix-control.txt` (pristine tree), `wire-postfix.txt` (fixed tree).

| expression | before | after |
|---|---|---|
| `evalf(cos(pi(30)), 40)` | `{"value":"-1","exact":true,"numerator":"-1","denominator":"1"}` | `{"value":"-1","exact":false}` — **no numerator/denominator** |
| `evalf(cos(pi(30)), 100)` | same as above | `{"value":"-1","exact":false}` |
| `evalf(cos(pi(30)*2), 100)` | `{"value":"1","exact":true,"numerator":"1","denominator":"1"}` | `{"value":"1","exact":false}` |
| `inspect(cos(pi(30))).exact` | `false` | `false` (unchanged — the node and the value now AGREE) |
| `cos(pi(30))` | `(real -1)` `exact:false` | `(real -1)` `exact:false` |
| `evalf(sin(pi(30)), 40)` | `{"value":"0","exact":false}` | `{"value":"0","exact":false}` |
| `evalf(tan(pi(30)), 40)` | `{"value":"0","exact":false}` | `{"value":"0","exact":false}` |

Exact controls, unchanged after the fix:

| expression | after |
|---|---|
| `evalf(1/2, 40)` | `{"value":"0.5","exact":true,"numerator":"1","denominator":"2"}` |
| `2+3` | `{"kind":"Natural","value":"5","exact":true}` |
| `sin(0)` | `{"kind":"Symbolic","canonical":"(rat 0 1)","domain":"rational","exact":true}` |

Values did not move: the post-fix probe (`probe-core-postfix.txt`) still reports
`Cos(PiTo(30)) = -1`, `Sin(PiTo(30)) = 0`, `Tan(PiTo(30)) = 0` at ambient 30/60/100 — only
`IsExact` flipped from `True` to `False` for cos/tan — and
`ComplexMathProvenanceTests.cs:83-84` (`Sin(Rl.Pi) == 0`, `Cos(Rl.Pi) == -1`) and
`ComplexMathPiResolutionTests.SinCos_ExactMultiplesOfTheAmbientPi_KeepTheirExactValues`
(places 20/50/100) both still pass, untouched.

---

## 6. Suites, Release, `LOVELACE_REQUIRE_SYMPY=1` with the oracle on `PATH`

Command per project: `dotnet test <project>\<project>.csproj -c Release` in `.worktrees/c6-pb1`
(raw logs `suite-*.log`, 0 build warnings across the four).

| project | failed | passed | skipped | total | exit |
|---|---|---|---|---|---|
| Lovelace.Complex.Tests | 0 | 108 | 0 | 108 | 0 |
| Lovelace.Symbolics.Tests | 0 | 1105 | 0 | 1105 | 0 |
| Lovelace.Suite.Tests | 0 | 825 | 0 | 825 | 0 |
| Lovelace.Run.Tests | 0 | 206 | 0 | 206 | 0 |

Complex's 108 includes the 12 new cases (the filtered run of the new file counts exactly 12, on
both trees); `Skipped: 0` in all four, so nothing was skipped or excluded from this run. I did not
re-run these four projects on the pre-fix product in this round (the pre-fix evidence is the control
tree's filtered run in §7 and the probes in §1), so the totals above are the totals of the FIXED tree.

## 7. Control tree — the new tests against the pristine product

```
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-pb1-ctl HEAD
# copied ONLY: Lovelace.Complex.Tests/ComplexMathInexactArgumentProvenanceTests.cs (new)
#              Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj (changed)
git -C .worktrees/c6-pb1-ctl status --porcelain
 M Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj
?? Lovelace.Complex.Tests/ComplexMathInexactArgumentProvenanceTests.cs
git -C .worktrees/c6-pb1-ctl diff --stat -- Lovelace.Complex     # (empty: the product is pristine)
```

```
dotnet test Lovelace.Complex.Tests\Lovelace.Complex.Tests.csproj -c Release \
  --filter "FullyQualifiedName~ComplexMathInexactArgumentProvenanceTests"
  Failed ...InexactPiAtItsOwnScale_ProducesInexactResults(scale: 30|60|100)  cos(pi(n)) = -1 claims exactness
  Failed ...ExactlyZeroReduction_OfAnInexactAmbientPi_IsNotExact            cos(pi) = -1 claims exactness
  Failed ...InexactZeroArgument_DoesNotClaimExactness                       sin(inexact zero) = 0 claims exactness
  Failed ...ComplexTrigOfAnInexactZeroPart_IsNotExact                       sin(0 + 0i) = (0, 0): the real part claims exactness
  Failed ...InverseTrigOfAnInexactZeroArgument_IsNotExact                   atan(inexact zero) claims exactness
  Failed ...SymbolicNodeAndNumericResult_AgreeForTheSameExpression          Assert.Equal() Failure: Values differ
Test Run Failed.
Total tests: 12
     Passed: 4
     Failed: 8
```

The 4 passing cases there are the deliberate guards (exact-zero controls and the cross-scale case), not
weak assertions: every negative claim in the file fails on the pre-fix product.

---

## 8. Could not verify / deliberately not done

1. **The VALUE error at the same root is still there.** `evalf(sin(pi(30)), 40)` answers `0` where the
   value is `5.0288419716939937e-31`, and `evalf(cos(pi(30)), 40)` answers `-1` where the value is
   `-0.999…87355374`. This round moves the FLAG only, as instructed; the digits are a separate defect.
2. **Two adjacent flag leaks measured, not fixed** (not trigonometric entry points, outside this round's
   scope): `ComplexMath.Exp(inexact zero) = 1` with `IsExact=True`, and
   `ComplexMath.Sqrt(0 + i·inexact zero)` with both components `IsExact=True`. Transcript rows in
   `probe-core-postfix.txt`. (As a side effect of the trig fix, `Exp(0 + i·inexact zero)` went from
   exact to inexact — that one propagates through `Cos`/`Sin` and is now honest.)
3. **A second, distinct laundering route exists and was NOT touched**: `evalf(pi(30), 40)` publishes
   `{"value":"3.14159…279","exact":true,"numerator":"3141592653589793238462643383279","denominator":"1000…"}`
   — a bare-`Real` argument is re-materialised through `RationalReal.FromReal` in
   `Lovelace.Symbolics/SymbolicsPlugin.cs:1521-1531`, which drops the route. It is a different entry
   (`pi(n)` is a Suite builtin returning an `Rl`; `cos(pi(30))` reaches `evalf` as a symbolic
   `RealConstantExpr`), and it lives in Lovelace.Symbolics, outside this round's scope.
4. **Not run**: the whole-solution sweep, the AOT publish, the SymPy differential oracle job and CI —
   only the four required projects under Release. No commit, stage or push was performed by me.
5. **The main tree shows `M docs/goal-cycle-6/state.md`** (`+18/−5`); that edit is not mine (I only
   created `docs/goal-cycle-6/round-10/**` in the main tree). Recorded rather than assumed away.

## 9. Raw evidence index (this directory)

| file | content |
|---|---|
| `pb1.patch` | the complete change (product + test csproj + new test file) |
| `prefix-newtests.log` | failing-first transcript (7 failed / 4 passed of 11) |
| `postfix-newtests.log` | the same file green (11/11, before the 12th case was added) |
| `postfix-complex-all.log` | full Lovelace.Complex.Tests after the fix (108/0) |
| `probe-core-prefix.txt`, `probe-core-postfix.txt` | the before/after exactness probe table |
| `wire-prefix-control.txt`, `wire-postfix.txt` | the before/after wire envelopes |
| `control-newtests.log` | the new tests against the pristine product (8 failed / 4 passed of 12) |
| `suite-Lovelace.*.log` | the four Release test runs |
