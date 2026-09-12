# P0-B — the leading significant digit at the ambient-precision boundary (cycle 6, round 17)

**Audit**: `docs/goal-cycle-6/round-13/audit-B-lattice.md` L2 (**P0**) — "at the ambient-precision boundary
the *leading* significant digit of a below-resolution value is wrong".

**Verdict**: reproduced, fixed, and pinned by a test that failed first. One product file changed
(`Lovelace.Complex/ComplexMath.cs`), one test file added.

**Trees** (neither is committed, staged or pushed; both are scratch worktrees of the repo):

| tree | path | HEAD | contents |
|---|---|---|---|
| scratch (the change) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0b` | `54b1753` | `Lovelace.Complex/ComplexMath.cs` modified, `Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs` new |
| control (pristine) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0bctl` | `54b1753` | ONLY the new test file copied in |

Both were created with `git worktree add .worktrees/<name> HEAD`. `git status --porcelain` in the
scratch tree lists exactly ` M Lovelace.Complex/ComplexMath.cs` and
`?? Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs` (plus an untracked `.p0b-scratch/`
holding the oracle script and the diff).

**Binaries**:

| | path | bytes | written |
|---|---|---|---|
| pre-fix (published, audit B's own) | `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` | 5 776 896 | 2026-09-12 15:06:37 |
| post-fix | `.worktrees\c6-p0b\out\aot-fix\Lovelace.Run.exe` | 5 797 376 | 2026-09-12 16:35:47 |

The post-fix binary was published from the scratch tree with the repo's own command, exit 0:
`dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot-fix`.

**Oracle**: mpmath 1.3.0 at 2400 dps, after
`$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH` and
`$env:LOVELACE_REQUIRE_SYMPY='1'`. Script: `.worktrees/c6-p0b/.p0b-scratch/oracle-p0b.py`.

**The wire path that reaches the defect**: `sin` of a real argument is
`ComplexMath.Sin` — `Lovelace.Symbolics/Evaluation.cs:335-337`
(`Tier(a) <= 2 ? FromReal(ComplexMath.Sin(ToReal(a))) : …`), so the audit's `evalf(sin(pi(30)), 40)`
lands in the file the audit located.

---

## 1. Pre-fix observation (re-observed, four probes, published binary)

```
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval "<script>" --json --omit-functions --omit-variables
```

**Binary**: `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`

| script | `result.structured.value` (what the API publishes) | `exact` | statement time |
|---|---|---|---|
| `setprecision(30); evalf(sin(pi(30)), 40)` | `0` | false | 965.7 µs |
| `setprecision(31); evalf(sin(pi(30)), 40)` | `0.0000000000000000000000000000004` | false | 1.72 ms |
| `setprecision(32); evalf(sin(pi(30)), 40)` | `0.00000000000000000000000000000049` | false | 1.29 ms |
| `setprecision(60); evalf(sin(pi(30)), 60)` | `0.000000000000000000000000000000502884197169399375105820974943` | false | 1.65 ms |

Verbatim envelope for the deciding probe (`setprecision(31); evalf(sin(pi(30)), 40)`):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Real","display":"0.0000000000000000000000000000004","typed":"0.0000000000000000000000000000004 (Real)","structured":{"kind":"Real","value":"0.0000000000000000000000000000004","exact":false}},"output":[],"variables":[],"functions":[],"elapsed":"1.72 ms","elapsedTime":{"value":1.72,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":32,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false},{"position":18,"elapsed":{"value":1.59,"unit":"ms"},"resultKind":"Real","hasOutput":false}]}
```


```
setprecision(31); print(evalf(sin(pi(30)), 40)); evalf(sin(pi(30)), 40)
```
returned (trimmed to the deciding fields, verbatim):

```json
{"ok":true,"revision":81,"result":{"kind":"Real","display":"0.0000000000000000000000000000004",
 "structured":{"kind":"Real","value":"0.0000000000000000000000000000004","exact":false}},
 "output":["0.0000000000000000000000000000004"],"elapsed":"2.07 ms"}
```

The 31st decimal of the true value is **5**; the published digit was **4** — a 20 % error in the first
meaningful digit, well inside the 40 places requested. The other three probes reproduce the numbers in
the audit: 0 at ambient 30, `…49` at 32 where the value is `…50`, and `…974943` at 60 where the value
is `…974944`.

## 2. Oracle — what the value is

```
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY='1'
python .worktrees/c6-p0b/.p0b-scratch/oracle-p0b.py
pi(30)            = 3.141592653589793238462643383279000000000
sin(pi(30))       = 5.0288419716939937510582097494459230781640628620899862803482532092112635556795782e-31
trunc@30         = 0.0   (sig digits: 0.0)
trunc@31         = 5.00000000000000000000000000000000e-31   (sig digits: 5.0e-31)
trunc@32         = 5.000000000000000000000000000000000e-31   (sig digits: 5.0e-31)
trunc@60         = 5.0288419716939937510582097494400000000000000000000000000000000e-31   (sig digits: 5.02884197169399375105820974944e-31)

sin(2*pi(100))    = -1.642961730265646132941876892191011644634507188162569622349005682054038770422111192892458979098607639288576219513318668922569512965e-100
trunc@30 sig      = -1.642961730265646132941876892191e-100
```

So: `sin(pi(30)) = 5.02884197169399375105820974944 5923…e-31`; its truncations at 30 / 31 / 32 / 60
decimal places are `0` / `5e-31` / `5.0e-31` / `5.02884197169399375105820974944e-31`.

## 3. Diagnosis — verified, and sharper than "the subtraction cancels"

The audit located `ComplexMath/PiDigitsFor` (pre-fix `Lovelace.Complex/ComplexMath.cs:541-549`) and
`SinCosAtPrecision` (`:565-612`). That is where the defect lives, and the mechanism is:

1. `PiDigitsFor` returned the **ambient** precision whenever the argument carried no more places than
   the ambient π (`argumentPlaces <= ambient ? ambient : …`, `:548`). At ambient 31 the argument
   `pi(30)` therefore reduced against `pi(31)`.
2. That reduction is EXACT and leaves `r = 5e-31` — a value with **one** stored digit, and the
   leading digit happens to be right.
3. `SinTaylor` (`:695`) then applies the series' own correction `−r³/6 ≈ −2.1e-92`. Addition is
   exact (`Real.Add`, `Lovelace.Real/Real.cs:2885-2924`), so the sum is a value *just below* `5e-31`,
   whose digit at place 31 is 4, not 5.
4. `TruncateFrac` (`:786`) truncates toward zero at the ambient precision, so it keeps that 4 — and
   the wire prints `…4` where the true 31-place truncation is `…5`. At ambient 60 the same borrow
   cascades into the last kept digit (`…943` where the true truncation ends `…944`).

Diagnostic transcript (a scratch xunit test in the scratch tree, now deleted; it replicated
`SinCosAtPrecision`'s steps through the public API and threw its findings):

```
ambient=31 display=31
x        = 3.141592653589793238462643383279  (Exp -30, periodic False)
piAmb    = 3.1415926535897932384626433832795  (Exp -31)
piFit(31)= 3.1415926535897932384626433832795  (Exp -31)
-- piAmb: halfPi=1.57079632679489661923132169163975 q=0.785398163397448309615660845819875
   k=1 r(pre)=1.57079632679489661923132169163925 swap=True r=0.0000000000000000000000000000005
   s=sin(r)=0.000000000000000000000000000000499999999999999999999 c=cos(r)=0.999999999999999999999999999999999999999999999999999 sin=0.999999999999999999999999999999999999999999999999999
-- pi50: halfPi=1.57079632679489661923132169163975144209858469968755 q=0.785398163397448309615660845819875721049292349843775
   k=1 r(pre)=1.57079632679489661923132169163924855790141530031245 swap=True r=0.0000000000000000000000000000005028841971693993751
   s=sin(r)=0.000000000000000000000000000000502884197169399375099 c=cos(r)=0.999999999999999999999999999999999999999999999999999 sin=0.999999999999999999999999999999999999999999999999999
-- pi81: halfPi=1.570796326794896619231321691639751442098584699687552 q=0.7853981633974483096156608458198757210492923498437764552437361480769541015715522495
   k=1 r(pre)=1.570796326794896619231321691639248557901415300312447 swap=True r=0.000000000000000000000000000000502884197169399375105820974944592307816406286208998
   s=sin(r)=0.000000000000000000000000000000502884197169399375105 c=cos(r)=0.999999999999999999999999999999999999999999999999999 sin=0.999999999999999999999999999999999999999999999999999
ComplexMath.Sin(x) = 0.0000000000000000000000000000004
```

The first row is the defect: the ambient π leaves the exactly-one-digit residual `5e-31`, and the
series' correction borrows it into `4.99999999999999999999e-31` (51 places = the 31+20 working
precision) — which truncates to `4e-31`. The `pi81` row is the fix's shape: the residual then carries
the true tail `5.02884197169399375105820974944…e-31`, whose digits *below* the requested window are
what the borrow lands in.

The parent's diagnosis is confirmed with one refinement: the missing thing is not extra digits for the
*subtraction* (the ambient subtraction is exact) but **guard places below the window the caller asked
for**. Cycle 5's formula `argumentPlaces + digits + GuardDigits` produces exactly those, and the
borrow then lands inside the guard instead of inside the answer.

## 4. Failing first

New test: `Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs`
(`:48-75` the four ambient precisions, `:83-98` the exact-multiple control, `:105-120` the cycle-5
residual control). Written and run BEFORE the fix, in the scratch tree at HEAD:

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0b
dotnet test Lovelace.Complex.Tests\Lovelace.Complex.Tests.csproj -c Release --filter "FullyQualifiedName~ComplexMathAmbientBoundaryTests"

  Failed Lovelace.Complex.Tests.ComplexMathAmbientBoundaryTests.Sin_OfPiAt30Places_PrintsTheMpmathDigitsAtTheAmbientPrecision [24 ms]
  Error Message:
   sin(pi(30)) at 31 places = 0.0000000000000000000000000000004, expected 0.0000000000000000000000000000005 (mpmath)
Failed!  - Failed:     1, Passed:     5, Skipped:     0, Total:     6, Duration: 57 ms - Lovelace.Complex.Tests.dll (net10.0)
```

The five passing tests are the controls (the ambient-π exact cases at 30/31/32/60 and
`sin(2·pi(100))`): the new test isolates the P0 and does not disturb them.

## 5. The fix

`Lovelace.Complex/ComplexMath.cs`, three parts (post-fix line numbers):

* `PiDigitsFor` (`:557-561`) now resolves π at `argumentPlaces + digits + GuardDigits`, floored at the
  ambient precision, for **every** argument — the cycle-5 formula, no longer short-circuited by
  `argumentPlaces <= ambient`.
* `ReducesToZeroAgainstTheAmbientPi` (`:580-586`) + `Fold` (`:594-609`): the exactness test is the
  ambient **reduction** itself, not a digit count. `SinCosAtPrecision` (`:624-…`, gate at `:644`)
  keeps the ambient π (and therefore the exact 0 / ±1 table values) when the ambient reduction lands
  on exactly zero, and uses the finer π otherwise. A digit count cannot make that call: a multiple of
  the ambient π whose trailing zero the representation dropped carries fewer places than the ambient π
  does, and a count would misread it as a coarser argument.
* The reduction is factored into `Fold` so the test and the main path run the same arithmetic.

The diff:

```diff
diff --git a/Lovelace.Complex/ComplexMath.cs b/Lovelace.Complex/ComplexMath.cs
index a950155..cc9ae05 100644
--- a/Lovelace.Complex/ComplexMath.cs
+++ b/Lovelace.Complex/ComplexMath.cs
@@ -530,22 +530,80 @@ public static class ComplexMath
     /// <para>
     /// The places π must carry for the reduction to measure the angle instead of measuring π: the
     /// argument's own fractional places, plus the precision the result is asked for, plus the guard
-    /// — or the ambient precision when the ambient π already reaches as far down as the argument
-    /// does.  Reducing an argument that carries 100 places against a 30-place π measures
-    /// <c>argument − k·π_30</c> (an artifact of order 1e-30) instead of the argument's own distance
-    /// to a multiple of π (order 1e-100), which is how <c>sin(pi(100)·2)</c> came back as
-    /// <c>+1e-30</c> instead of <c>−1.64e-100</c>.  A coarser argument is already resolved by the
-    /// ambient π, and an exact multiple of the ambient π must keep reducing exactly to zero.
+    /// — never less than the ambient precision.  Reducing an argument that carries 100 places
+    /// against a 30-place π measures <c>argument − k·π_30</c> (an artifact of order 1e-30) instead
+    /// of the argument's own distance to a multiple of π (order 1e-100), which is how
+    /// <c>sin(pi(100)·2)</c> came back as <c>+1e-30</c> instead of <c>−1.64e-100</c>.
+    /// </para>
+    /// <para>
+    /// The SAME cancellation meets the ambient precision from above.  At 31 places
+    /// <c>sin(pi(30))</c> reduces <c>pi(30) − pi(31)</c> out of operands that carry 31 places; the
+    /// residual that leaves is the exactly-one-digit <c>5e-31</c>, the series' own correction
+    /// (<c>r³/6 ≈ 2.1e-92</c>) borrows that residual's last stored digit, and the truncation to 31
+    /// places prints <c>4e-31</c> where the value is <c>5.0288…e-31</c> — a 20% error in the FIRST
+    /// meaningful digit (cycle 6, audit B L2/P0).  The guard places are what those truncations
+    /// borrow from, so the window the caller asked for comes out of the sum untouched.
+    /// </para>
+    /// <para>
+    /// The one reduction that must NOT move is the exact one: only the ambient π reduces an exact
+    /// multiple of the ambient π (<c>Rl.Pi</c>, <c>π/2</c>, <c>2π</c>, …) to exactly zero, and the
+    /// zero and the ±1 that reduction lands on are the values the library promises
+    /// (<c>Sin(Rl.Pi) == 0</c>, <c>Cos(Rl.Pi) == −1</c>).  Which arguments those are is decided by
+    /// the reduction itself (<see cref="ReducesToZeroAgainstTheAmbientPi"/>), not by a digit count:
+    /// a multiple of the ambient π whose trailing zero the representation dropped carries FEWER
+    /// places than the ambient π does, and a count would misread it as a coarser argument.
     /// </para>
     /// </summary>
     private static long PiDigitsFor(Rl x, long digits)
     {
-        long argumentPlaces = x.IsPeriodic ? 0L : -x.Exponent;
-        if (argumentPlaces < 0L)
-            argumentPlaces = 0L;
-
+        long resolved = ArgumentPlaces(x) + digits + GuardDigits;
         long ambient = AmbientDigits();
-        return argumentPlaces <= ambient ? ambient : argumentPlaces + digits + GuardDigits;
+        return resolved < ambient ? ambient : resolved;
+    }
+
+    /// <summary>The places <paramref name="x"/> carries a digit in: its own fractional digits, or 0
+    /// for a periodic value (whose digits are not a finite truncation) and for an integer.</summary>
+    private static long ArgumentPlaces(Rl x)
+    {
+        long places = x.IsPeriodic ? 0L : -x.Exponent;
+        return places < 0L ? 0L : places;
+    }
+
+    /// <summary>
+    /// Whether <paramref name="x"/> is an exact multiple of π/2 at the AMBIENT precision — the
+    /// question a digit count cannot answer, because an exact multiple of the ambient π whose
+    /// trailing zero the representation dropped carries fewer places than the ambient π does, and
+    /// because the ambient π is the only constant that reduces such a multiple to exactly zero.
+    /// The test is that reduction itself: at the working precision the difference of two values
+    /// carrying at most the ambient places is exact, so a zero here is an exact zero.
+    /// </summary>
+    private static bool ReducesToZeroAgainstTheAmbientPi(Rl x, long work)
+    {
+        Rl pi;
+        using (Rl.WithPrecision(AmbientDigits(), work))
+            pi = Rl.Pi;
+
+        using var scope = Rl.WithPrecision(work, work);
+        (_, Rl r, _) = Fold(x, pi);
+        return Rl.IsZero(r);
+    }
+
+    /// <summary>Reduces <paramref name="x"/> modulo π/2 against <paramref name="pi"/> and folds it
+    /// into <c>[0, π/4]</c>, reporting the quadrant, the folded residual and whether the sine and
+    /// cosine are swapped.</summary>
+    private static (long k, Rl r, bool swap) Fold(Rl x, Rl pi)
+    {
+        Rl twoPi = pi * new Rl("2");
+        Rl halfPi = pi / new Rl("2");
+        Rl quarterPi = pi / new Rl("4");
+
+        Rl xr = ReduceToTwoPi(x, pi, twoPi);
+        long k = TruncToLong(xr / halfPi);
+        Rl r = xr - FromLong(k) * halfPi;      // r in [0, π/2)
+
+        bool swap = false;
+        if (r > quarterPi) { r = halfPi - r; swap = true; }   // fold to [0, π/4]
+        return (k, r, swap);
     }
 
     /// <summary>
@@ -555,8 +613,9 @@ public static class ComplexMath
     /// approximation and its provenance says so.
     /// <para>
     /// THE PAIR CARRIES THE ARGUMENT'S PROVENANCE.  The reduction can land EXACTLY on a table value
-    /// from an argument that is itself a truncation — at the argument's own scale the residual IS
-    /// zero <c>(PiDigitsFor)</c>, so <c>cos(pi(30))</c> is the table's <c>−1</c> — and neither half
+    /// from an argument that is itself a truncation — an exact multiple of the ambient π reduces to
+    /// exactly zero against it (<see cref="ReducesToZeroAgainstTheAmbientPi"/>), so
+    /// <c>cos(pi(30))</c> under a 30-place ambient is the table's <c>−1</c> — and neither half
     /// may certify that as exact.  Both exits say so: the shortcut below, which may stay exact only
     /// for an EXACT zero, and the return, through
     /// <see cref="WithArgumentProvenance(Rl, Rl)"/> (cycle 6, P-B1).
@@ -574,24 +633,24 @@ public static class ComplexMath
                 : (Rl.AsInexact(Rl.Zero), Rl.AsInexact(Rl.One));
 
         long work = digits + GuardDigits;
+        long ambient = AmbientDigits();
         long piDigits = PiDigitsFor(x, digits);
 
+        // An argument that is an exact multiple of the AMBIENT π keeps reducing exactly to zero
+        // (Sin(Rl.Pi) == 0, Cos(Rl.Pi) == −1), and only the ambient π produces that zero — no finer
+        // constant may be substituted for it.  Every other argument, the coarser truncations of π
+        // the ambient π cannot resolve included, is measured against a π that reaches GuardDigits
+        // below the window the caller asked for.
+        if (ArgumentPlaces(x) <= ambient && ReducesToZeroAgainstTheAmbientPi(x, work))
+            piDigits = ambient;
+
         Rl pi;
         using (Rl.WithPrecision(piDigits, work))
-            pi = Rl.Pi;                // the ambient π unless the argument reaches further down
+            pi = Rl.Pi;                // the ambient π for an exact multiple, a finer one otherwise
 
         using var scope = Rl.WithPrecision(work, work);
 
-        Rl twoPi = pi * new Rl("2");
-        Rl halfPi = pi / new Rl("2");
-        Rl quarterPi = pi / new Rl("4");
-
-        Rl xr = ReduceToTwoPi(x, pi, twoPi);
-        long k = TruncToLong(xr / halfPi);
-        Rl r = xr - FromLong(k) * halfPi;      // r in [0, π/2)
-
-        bool swap = false;
-        if (r > quarterPi) { r = halfPi - r; swap = true; }   // fold to [0, π/4]
+        (long k, Rl r, bool swap) = Fold(x, pi);
 
         Rl s = SinTaylor(r, work);
         Rl c = CosTaylor(r, work);
```

## 6. Digits, before and after (mpmath at 2400 dps)

**Binary**: `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0b\out\aot-fix\Lovelace.Run.exe`

| script | `result.structured.value` (what the API publishes) | `exact` | statement time |
|---|---|---|---|
| `setprecision(30); evalf(sin(pi(30)), 40)` | `0` | false | 925.8 µs |
| `setprecision(31); evalf(sin(pi(30)), 40)` | `0.0000000000000000000000000000005` | false | 1.84 ms |
| `setprecision(32); evalf(sin(pi(30)), 40)` | `0.0000000000000000000000000000005` | false | 1.67 ms |
| `setprecision(60); evalf(sin(pi(30)), 60)` | `0.000000000000000000000000000000502884197169399375105820974944` | false | 2.39 ms |

Verbatim envelope for the deciding probe (`setprecision(31); evalf(sin(pi(30)), 40)`):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Real","display":"0.0000000000000000000000000000005","typed":"0.0000000000000000000000000000005 (Real)","structured":{"kind":"Real","value":"0.0000000000000000000000000000005","exact":false}},"output":[],"variables":[],"functions":[],"elapsed":"1.84 ms","elapsedTime":{"value":1.84,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":24.9,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false},{"position":18,"elapsed":{"value":1.75,"unit":"ms"},"resultKind":"Real","hasOutput":false}]}
```


Digit-by-digit at the boundary — `sin(pi(30))`, mpmath truncation vs the published value
(the probe's `result.structured.value`, which is what the machine API publishes):

| ambient | mpmath (true truncation) | pre-fix | post-fix | verdict |
|---|---|---|---|---|
| 30 | `0` (every digit lies below the window) | `0` | `0` | unchanged, honest — the argument IS the ambient π |
| 31 | `0.0000000000000000000000000000005` | `0.0000000000000000000000000000004` | `0.0000000000000000000000000000005` | **fixed**, leading digit 5 |
| 32 | `0.00000000000000000000000000000050` (value `5.0e-31`) | `0.00000000000000000000000000000049` | `0.0000000000000000000000000000005` | **fixed**; the trailing zero is not printed (see §11) |
| 60 | `0.000000000000000000000000000000502884197169399375105820974944` | `…974943` | `…974944` | **fixed**, all 30 digits |

The same defect appears wherever the ambient precision is the coarser side; the fix carries the whole
family (probed on both binaries, same commands):

| probe | mpmath | pre-fix | post-fix |
|---|---|---|---|
| `setprecision(31); evalf(sin(pi(20)), 40)` | `…0026433832795` | `…0026433832794` | `…0026433832795` |
| `setprecision(60); evalf(sin(pi(40)), 80)` | `…000069399375105820974944` | `…000069399375105820974943` | `…000069399375105820974944` |
| `setprecision(100); evalf(sin(pi(50)), 120)` | `…058209749445923078164062862089986280348253421170679` | `…170678` | `…170679` |

## 7. Pinned controls — unchanged

| control | where | pre-fix | post-fix |
|---|---|---|---|
| `ComplexMath.Sin(Rl.Pi) == 0` (value) | `ComplexMathProvenanceTests.cs:83`, `ComplexMathTests.cs:73` | passes | **passes** |
| `ComplexMath.Cos(Rl.Pi) == -1` (value) | `ComplexMathProvenanceTests.cs:84` | passes | **passes** |
| `sin(pi(100)·2)` keeps `-1.64e-100` | `ComplexMathPiResolutionTests.cs:50-72` (ambient 30) | `-0.000…0164296173026564613294187689219` | identical |
| wire `setprecision(100); evalf(sin(pi(100)*2), 100)` | published binary | `0` | `0` (the argument IS an exact multiple of that ambient π) |
| wire `setprecision(31); evalf(sin(pi(31)), 40)` | published binary | `0` | `0` |

The new test repeats the two value controls at 30/31/32/60
(`ComplexMathAmbientBoundaryTests.cs:83-98`) and the cycle-5 residual at ambient 30
(`:105-120`, `|actual − expected| < 1e-105` and the printed digits start `-0.000…016429617302656461329`).

## 8. Cost

**AOT probes** (wall clock is dominated by the ~30 ms process start; the deciding figure is the
interpreter's own `elapsed` field for the statement, minimum of 7 runs):

| probe | pre-fix stmt | post-fix stmt | ratio |
|---|---|---|---|
| `setprecision(30); evalf(sin(pi(30)), 40)` | < 1 ms (µs) | < 1 ms (µs) | — |
| `setprecision(31); evalf(sin(pi(30)), 40)` | 1.22 ms | 1.62 ms | 1.33× |
| `setprecision(32); evalf(sin(pi(30)), 40)` | 1.14 ms | 1.65 ms | 1.45× |
| `setprecision(60); evalf(sin(pi(30)), 60)` | 1.48 ms | 2.23 ms | 1.51× |

Nothing moved from milliseconds to minutes; every probe stays under 3 ms of statement time and ~31 ms
of wall clock.

**In-process** (Release, warm, per call, same scratch harness run in the control tree and in the
scratch tree):

| call | pre-fix | post-fix | ratio |
|---|---|---|---|
| `sin(pi(30))` ambient 31 | 0.440 ms | 1.426 ms | 3.2× |
| `sin(1.5)` ambient 31 | 2.113 ms | 2.319 ms | 1.10× |
| `sin(pi)` ambient 31 | 0.010 ms | 0.026 ms | 2.6× (26 µs) |
| `sin(pi(30))` ambient 60 | 0.183 ms | 0.674 ms | 3.7× |
| `sin(1.5)` ambient 60 | 4.857 ms | 6.596 ms | 1.36× |
| `sin(pi)` ambient 60 | 0.017 ms | 0.034 ms | 2.0× (34 µs) |
| `sin(2·pi(100))` ambient 30 | 0.266 ms | 0.345 ms | 1.30× |
| `sin(pi(30))` ambient 200 | 2.581 ms | 4.971 ms | 1.93× |
| `sin(1.5)` ambient 200 | 180.521 ms | 200.446 ms | 1.11× |
| `sin(pi)` ambient 200 | 0.030 ms | 0.048 ms | 1.6× (48 µs) |

The cost is the second reduction the exactness test performs (at ambient π) plus, for arguments whose
residual is tiny, the finer π the reduction now needs. Ordinary arguments pay 1.1–1.4×; the previously
wrong tiny values pay up to ~3.7× of a sub-millisecond number. No probe changes order of magnitude.

## 9. Suites (Release, scratch tree, per project, 0 failed)

```
RESULT | Complex.Tests          | filter=''                  | Passed! - Failed: 0, Passed:  121, Skipped: 0, Total:  121
RESULT | Real.Tests             | filter=''                  | Passed! - Failed: 0, Passed: 2495, Skipped: 0, Total: 2495
RESULT | Suite.Tests            | filter=''                  | Passed! - Failed: 0, Passed:  828, Skipped: 0, Total:  828
RESULT | Symbolics.Tests        | filter=''                  | Passed! - Failed: 0, Passed: 1140, Skipped: 8, Total: 1148
RESULT | Dsp.Tests              | filter=''                  | Passed! - Failed: 0, Passed:   61, Skipped: 0, Total:   61
RESULT | Real.Tests Timing      | filter='Category=Timing'   | Passed! - Failed: 0, Passed:    1, Skipped: 0, Total:    1
RESULT | Suite.Tests Timing     | filter='Category=Timing'   | Passed! - Failed: 0, Passed:    8, Skipped: 0, Total:    8
RESULT | Symbolics.Tests Timing | filter='Category=Timing'   | Passed! - Failed: 0, Passed:    1, Skipped: 0, Total:    1
RESULT | Run.Tests Timing       | filter='Category=Timing'   | Passed! - Failed: 0, Passed:    5, Skipped: 0, Total:    5
RESULT | Symbolics.Tests Costly | filter='Category=Costly'   | Passed! - Failed: 0, Passed:   32, Skipped: 0, Total:   32
```

(The four `Category=Timing` subsets are the four projects that carry the trait: Real, Suite, Symbolics
and Run; `Category=Costly` exists only in Symbolics.)

## 10. Control tree — the new test fails at HEAD

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0bctl      # git worktree add .worktrees/c6-p0bctl HEAD
Copy-Item .worktrees\c6-p0b\Lovelace.Complex.Tests\ComplexMathAmbientBoundaryTests.cs .worktrees\c6-p0bctl\Lovelace.Complex.Tests\ -Force
git status --porcelain        ->  ?? Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs
dotnet test Lovelace.Complex.Tests\Lovelace.Complex.Tests.csproj -c Release --filter "FullyQualifiedName~ComplexMathAmbientBoundaryTests"

  Failed …Sin_OfPiAt30Places_PrintsTheMpmathDigitsAtTheAmbientPrecision [512 ms]
   sin(pi(30)) at 31 places = 0.0000000000000000000000000000004, expected 0.0000000000000000000000000000005 (mpmath)
Failed!  - Failed:     1, Passed:     5, Skipped:     0, Total:     6, Duration: 554 ms
```

The test is red on the unfixed product code and green on the fixed tree — the +20-digit claim is the
fix's, not the test's.

## 11. Could not verify / limitations

* **The 32-place rendering is 31 characters.** The value is `5.0e-31`, which equals mpmath's 32-place
  truncation, but the engine's canonical form drops the trailing zero, so the wire prints
  `0.…5` (31 places) instead of `0.…50`. No digit that IS printed is wrong; the character count is a
  rendering property, not a value, and the test pins the 32nd digit by value
  (`ComplexMathAmbientBoundaryTests.cs:57-60, 71`). `TruncateFrac` normalises through
  `* InexactOne` (`:786-817`); changing that is outside this P0's scope.
* **Arguments with MORE places than the ambient π skip the exactness test** (the gate at `:644` is
  `ArgumentPlaces(x) <= ambient`). A padded representation of an exact multiple of the ambient π
  (`pi(31)` spelled with 100 places) would therefore take the finer-π path instead of reducing to
  zero. I probed the reachable shape — `setprecision(31); evalf(sin(evalf(pi(31), 100)), 40)` — on both
  binaries: `evalf` does NOT pad (it answers `3.1415926535897932384626433832795`, 31 places) and the
  result is `0.0000000000000000000000000000000` on both. I did not find a wire-constructible value of
  that shape, and the gate is what keeps the cycle-5 path's cost unchanged.
* **The exact-multiple rule is self-consistent where the values coincide**, which is the one place it
  could have hurt: `π_31` and `π_32` are the SAME value (π's 32nd decimal is 0), so at ambient 32 the
  argument is the ambient π and `sin` answers 0. Observed: mpmath gives
  `sin(pi(31)) = 2.88419716939937510582097494459230781640628620899862803482534e-33` — its first digit
  is at place 33, below the 32-place window — and the wire answers `0` on BOTH binaries
  (`setprecision(32); evalf(sin(pi(31)), 40)`). Zero is therefore both the contract's answer and the
  correct truncation at that window; whenever a coarser truncation of π equals the ambient one as a
  value, the residual lies below the window for the same reason.
* **`Lovelace.Real`'s own trig was not touched.** `Real.ResolvedPiDigits`
  (`Lovelace.Real/Real.cs:1830-1847`) has the same `argumentPlaces <= ambient` shape, and `Real.Sin` /
  `Real.Cos` are used by `Lovelace.Dsp` and `Complex`, not by the symbolic `sin`. The audit's L2
  reproduced through `evalf(sin(…))`, which routes to `ComplexMath`; I did not probe `Real.Sin`
  through any wire path, and no scope in this round asked for it.
* I did not run the differential SymPy oracle suites, any coverage collector, or the audit's full
  2 000-value lattice sweep; the oracle here is mpmath at 2400 dps invoked directly (§2), and the
  boundary family was probed at 30/31/32/60 plus the three extra shapes in §6.

## 12. Files

* `Lovelace.Complex/ComplexMath.cs` (modified, scratch tree) — `PiDigitsFor`, `ArgumentPlaces`,
  `ReducesToZeroAgainstTheAmbientPi`, `Fold`, `SinCosAtPrecision`; 83 insertions, 24 deletions.
* `Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs` (new, 121 lines).
* `.p0b-scratch/oracle-p0b.py`, `.p0b-scratch/diff-complexmath.patch` (evidence, untracked).
* Nothing committed or staged by me; my writes went to the scratch worktree and to this document. The
  main tree is concurrently dirty from OTHER rounds (`Lovelace.Run/Runner.cs`,
  `Lovelace.Suite/EngineExceptions.cs`, `Lovelace.Suite/Interpreter.cs`, `docs/goal-cycle-6/state.md`,
  plus new untracked test files under `Lovelace.Run.Tests/` and `Lovelace.Suite.Tests/`), and during the
  round it also acquired copies of this round's two files — `Lovelace.Complex/ComplexMath.cs`
  (SHA-256 `B2A520D7F9BF782FF492C74600D5418C203A2AA004D2535542216F3F37CF232F`, byte-identical to the scratch
  tree's copy) and `Lovelace.Complex.Tests/ComplexMathAmbientBoundaryTests.cs` — which my session did not
  place there. The scratch worktree `.worktrees/c6-p0b` is the tree this report is about and the one I
  verified; the main-tree copies were not run by me.
