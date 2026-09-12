# Round 11 — the two remaining FLAG leaks on the evalf path

**Objective.** A truncated or inexact value must never be labelled exact anywhere on the wire.
Two leaks: (a) a truncated constant published by `evalf` as an exact rational, (b) two
`ComplexMath` entry points answering an inexact argument with an exact result.

**Trees.** Nothing was committed, staged or pushed.

| Tree | Path | What it is |
| --- | --- | --- |
| Scratch (the work) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-flag` | `git worktree add .worktrees/c6-flag HEAD` — detached at `e3aab61` |
| Control (requirement 5) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-flag-ctl` | `git worktree add .worktrees/c6-flag-ctl HEAD` — pristine `e3aab61`, only the two new test files copied in |

**Files changed (scratch tree).** `Lovelace.Symbolics/SymbolicsPlugin.cs` (+46/−14) and
`Lovelace.Complex/ComplexMath.cs` (+29/−6) — `git diff --numstat`; new tests
`Lovelace.Symbolics.Tests/EvalfTruncatedConstantExactnessTests.cs` and
`Lovelace.Complex.Tests/ComplexMathExpSqrtInexactArgumentTests.cs`. No other file was touched.

---

## 1. Pre-fix behaviour (observed, before any edit)

Runner: `dotnet <c6-flag>/Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval "<expr>" --json --omit-functions`.

### 1a. A truncated constant labelled exact with a rational form

```
CASE  evalf(pi(30), 40)
  ->  {"kind":"Real","value":"3.141592653589793238462643383279","exact":true,"numerator":"3141592653589793238462643383279","denominator":"1000000000000000000000000000000"}
CASE  evalf(pi(1), 40)
  ->  {"kind":"Real","value":"3.1","exact":true,"numerator":"31","denominator":"10"}
CASE  evalf(pi(1), 5)
  ->  {"kind":"Real","value":"3.1","exact":true,"numerator":"31","denominator":"10"}
CASE  evalf(e(30), 40)
  ->  {"kind":"Real","value":"2.718281828459045235360287471352","exact":true,"numerator":"339785228557380654420035933919","denominator":"125000000000000000000000000000"}
CASE  evalf(e(1), 40)
  ->  {"kind":"Real","value":"2.7","exact":true,"numerator":"27","denominator":"10"}
CASE  evalf(pi(30), 100)
  ->  {"kind":"Real","value":"3.141592653589793238462643383279","exact":true,"numerator":"3141592653589793238462643383279","denominator":"1000000000000000000000000000000"}
CASE  pi(30)
  ->  {"kind":"Real","value":"3.141592653589793238462643383279","exact":false}
CASE  e(30)
  ->  {"kind":"Real","value":"2.718281828459045235360287471352","exact":false}
```

The ambient constant is `exact:false` and the `evalf` of it claims `exact:true` **with a rational
form**. The claim appears exactly when the rendered width is NARROWER than the requested count
(`evalf(pi(30), 29)` → `3.14159265358979323846264338327` `exact:false`; `evalf(pi(1), 1)` →
`3.1` `exact:false`) — i.e. the flag was being read off the width of the rendering, not the route.

Two neighbours of the same class, also measured on the defective tree:

```
CASE  evalf(pi(30)-pi(30), 40)          ->  {"kind":"Integer","value":"0","exact":true}
CASE  evalf(exp(pi(30)-pi(30)), 40)     ->  {"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}
CASE  sin(pi(30))                       ->  {"kind":"Symbolic","pretty":"0.000000000000000000000000000000","exact":false}
```

### 1b. `ComplexMath.Exp` and the complex `Sqrt` (library level)

Both were reproduced by running the new test file on the pristine control tree (the exact pre-fix
code) — the failures below are the observation, and they are also the failing-first transcript:

```
Failed ComplexMathExpSqrtInexactArgumentTests.ExpOfAnInexactZero_IsNotExact
  Error Message: exp(inexact zero) = 1 claims exactness
Failed ComplexMathExpSqrtInexactArgumentTests.ComplexExpOfAnInexactPart_IsNotExact(realInexact: True, imaginaryInexact: False)
  Error Message: exp(0, 0) = (1, 0): the real part claims exactness
Failed ComplexMathExpSqrtInexactArgumentTests.SqrtOfAnInexactArgument_IsNotExact
  Error Message: sqrt(inexact 0 + 0i) = (0, 0): the real part claims exactness
Failed ComplexMathExpSqrtInexactArgumentTests.SqrtOfAnInexactImaginaryPart_IsNotExact
  Error Message: sqrt(9 + inexact 0i) = (3, 0): the imaginary part claims exactness
Failed!  - Failed: 4, Passed: 3, Skipped: 0, Total: 7 — Lovelace.Complex.Tests.dll
```

Causes read off the source before the fix: `Lovelace.Complex/ComplexMath.cs:32`
(`if (Rl.IsZero(x)) return Rl.One;` — the exact-one shortcut never consults `x.IsExact`),
`ComplexMath.cs:223` (`return TruncateComplex(new Complex(e * Cos(z.Im), e * Sin(z.Im)), digits);`
— no provenance wrapper, unlike the trig overloads fixed by `78c19d8`) and `ComplexMath.cs:290`
(`return TruncateComplex(...)` for `Sqrt` — no provenance wrapper at all; `sqrt(0) = 0` is exact by
construction in `Lovelace.Real/Real.cs:1280-1281`, so a truncated argument got an exact zero back).

---

## 2. The fix

`fix.diff` beside this file is the complete `git diff` of the scratch tree. The two mechanisms:

**(a)** `SymbolicsPlugin.NumericAtRequestedPrecision` (was `:1517-1531`, now `:1542-1562`) used to
project EVERY already-numeric payload onto the exact rational tier:
`Rl r => Exprs.Rational(RationalReal.FromReal(r))`. A `NumRat` is exact by construction, so the
payload's route was gone by the time `NumToPayload` rebuilt a Real from its digits. The projection
now takes the requested `digits` and splits:

```csharp
private static Expr RealAtRequestedPrecision(Rl value, int digits) =>
    value.IsExact
        ? Exprs.Rational(RationalReal.FromReal(value))
        : Exprs.Real(RealLiteral.FromRealExact(Rl.AsInexact(
            RationalReal.ToReal(RationalReal.FromReal(value), Math.Min(digits, 1000)))));
```

An **exact** value takes the old path unchanged. An **inexact** one is truncated through the very
same `RationalReal.ToReal(…, Math.Min(digits, 1000))` that `NumToPayload` (`:1561`) applied to the
rational before — so the DIGITS are identical by construction — and crosses as an **inexact
`RealLiteral`**, the one carrier that keeps `IsExact` across the conversion.
`SymbolicsPlugin.cs:481` passes `digits` in.

**(b)** `ComplexMath` now routes both entry points through the helper `78c19d8` introduced
(`ComplexMath.cs:363`, `WithArgumentProvenance`): `Exp(Rl)` returns
`WithArgumentProvenance(Rl.One, x)` at the `IsZero` shortcut (`:38`), `Exp(Complex)` wraps its
result (`:235`) and `Sqrt(Complex)` wraps its result (`:312`). No value expression was edited in
either overload.

---

## 3. Before / after (same command, same tree, after rebuild)

```
CASE  evalf(pi(30), 40)                        pre: {"kind":"Real","value":"3.141592653589793238462643383279","exact":true,"numerator":"3141592653589793238462643383279","denominator":"1000000000000000000000000000000"}
                                               post:{"kind":"Real","value":"3.141592653589793238462643383279","exact":false}
```

| Expression | pre-fix | post-fix | value moved? |
| --- | --- | --- | --- |
| `evalf(pi(30), 40)` | Real `3.141592653589793238462643383279` **exact:true**, num `3141592653589793238462643383279`, den `10^30` | Real `3.141592653589793238462643383279` **exact:false**, no num/den | no |
| `evalf(pi(1), 40)` | Real `3.1` **exact:true**, num `31`, den `10` | Real `3.1` **exact:false** | no |
| `evalf(pi(1), 5)` | Real `3.1` **exact:true**, num `31`, den `10` | Real `3.1` **exact:false** | no |
| `evalf(pi(30), 5)` | Real `3.14159` exact:false | Real `3.14159` exact:false | no |
| `evalf(pi(30), 29)` | Real `3.14159265358979323846264338327` exact:false | same | no |
| `evalf(pi(30), 30)` | Real `…279` exact:false | same | no |
| `evalf(pi(30), 100)` | Real `…279` **exact:true**, num/den as above | Real `…279` **exact:false** | no |
| `evalf(e(30), 40)` | Real `2.718281828459045235360287471352` **exact:true**, num `339785228557380654420035933919`, den `125·10^27` | Real `…352` **exact:false** | no |
| `evalf(e(1), 40)` | Real `2.7` **exact:true**, num `27`, den `10` | Real `2.7` **exact:false** | no |
| `evalf(e(30), 7)` | Real `2.7182818` exact:false | same | no |
| `evalf(e(30), 100)` | Real `…352` **exact:true** | Real `…352` **exact:false** | no |
| `evalf(pi(30)-pi(30), 40)` | Integer `0` **exact:true** | Real `0` **exact:false** (kind follows the honest tier: an inexact value has no Integer carrier) | no |
| `evalf(exp(pi(30)-pi(30)), 40)` | Real `1` **exact:true**, num `1`, den `1` | Real `1` **exact:false** | no |
| `ComplexMath.Exp(inexact zero)` | `1` **exact:true** | `1` **exact:false** | no |
| `ComplexMath.Exp(inexact 0 + 0i)` | `(1, 0)` both **exact:true** | `(1, 0)` both **exact:false** | no |
| `ComplexMath.Sqrt(inexact 0 + 0i)` | `(0, 0)` both **exact:true** | `(0, 0)` both **exact:false** | no |
| `ComplexMath.Sqrt(inexact 4 + 0i)` | `(2, 0)` im **exact:true** | `(2, 0)` both **exact:false** | no |
| `ComplexMath.Sqrt(inexact 2 + 0i)` | re inexact, im **exact:true** | `(√2, 0)` both **exact:false** | no |
| `ComplexMath.Sqrt(9 + inexact 0i)` | `(3, 0)` im **exact:true** | `(3, 0)` both **exact:false** | no |

Requested-count control (requirement 3, second half): a caller asking `pi(30)` and evaluating it to
40 or 100 places still GETS the 30 places — `3.141592653589793238462643383279` — asserted as a
VALUE IDENTITY against `pi(30)` itself (`EvalfTruncatedConstantExactnessTests`,
`RequestWiderThanTheConstantsOwnCount_CarriesExactlyTheConstantsDigits`), and pinned digit-for-digit
in the theory above. Only the claim moved.

## 4. Exact controls (requirement 3) — untouched

Post-fix wire, same commands:

```
evalf(1/2, 40)   ->  {"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}
2+3              ->  {"kind":"Natural","value":"5","exact":true}
sin(0)           ->  {"kind":"Symbolic","pretty":"0","canonical":"(rat 0 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}
evalf(sqrt(2), 40) -> {"kind":"Real","value":"1.4142135623730950488016887242096980785696","exact":false}
evalf(sqrt(2), 5)  -> {"kind":"Real","value":"1.41421","exact":false}
evalf(sqrt(4), 40) -> {"kind":"Integer","value":"2","exact":true}
evalf(1/3, 5)      -> {"kind":"Real","value":"0.33333","exact":false}
evalf(sin(pi(30)/6), 40) -> {"kind":"Real","value":"0.499999999999999999999999999999","exact":false}
```

`ComplexMath` controls: `Exp(0) = 1` exact, `Sqrt(0 + 0i) = (0, 0)` both exact,
`Sqrt(4 + 0i) = (2, 0)` both exact — asserted in
`ComplexMathExpSqrtInexactArgumentTests.ExpOfAnInexactZero_IsNotExact` and
`SqrtOfAnInexactArgument_IsNotExact`.

## 5. Failing first, and the control tree (requirements 2 and 5)

The two test files were written and RUN before any product edit. On the unfixed scratch tree:

```
dotnet test .worktrees/c6-flag/Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release \
  --filter "FullyQualifiedName~EvalfTruncatedConstantExactnessTests"
Failed!  - Failed: 15, Passed: 7, Skipped: 0, Total: 22 — Lovelace.Symbolics.Tests.dll
dotnet test .worktrees/c6-flag/Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj -c Release \
  --filter "FullyQualifiedName~ComplexMathExpSqrtInexactArgumentTests"
Failed!  - Failed: 4, Passed: 3, Skipped: 0, Total: 7 — Lovelace.Complex.Tests.dll
```

Then fixed; then the SAME two files copied (nothing else) into the pristine control tree:

```
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-flag-ctl HEAD
Copy-Item …/c6-flag/Lovelace.Symbolics.Tests/EvalfTruncatedConstantExactnessTests.cs   -> c6-flag-ctl/…
Copy-Item …/c6-flag/Lovelace.Complex.Tests/ComplexMathExpSqrtInexactArgumentTests.cs    -> c6-flag-ctl/…
git -C .worktrees/c6-flag-ctl status --porcelain
?? Lovelace.Complex.Tests/ComplexMathExpSqrtInexactArgumentTests.cs
?? Lovelace.Symbolics.Tests/EvalfTruncatedConstantExactnessTests.cs
dotnet test .worktrees/c6-flag-ctl/Lovelace.Symbolics.Tests/… --filter "…EvalfTruncatedConstantExactnessTests"
Failed!  - Failed: 15, Passed: 7, Skipped: 0, Total: 22 — Lovelace.Symbolics.Tests.dll (net10.0)   [control-symbolics.txt]
dotnet test .worktrees/c6-flag-ctl/Lovelace.Complex.Tests/… --filter "…ComplexMathExpSqrtInexactArgumentTests"
Failed!  - Failed: 4, Passed: 3, Skipped: 0, Total: 7 — Lovelace.Complex.Tests.dll (net10.0)       [control-complex.txt]
```

The full transcripts (every failed case with its message) are `control-symbolics.txt` and
`control-complex.txt`. The same two files on the FIXED tree: `postfix-symbolics.txt` 22/0,
`postfix-complex.txt` 7/0. Nothing in the test files asserts a value the fix was allowed to move:
each negative assertion is paired with the digits the case published before, so "fix the claim by
changing the number" fails the file.

## 6. Whole solution (requirement 4)

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` in the scratch tree →
exit code 0. Full transcript: `suite-whole-solution.txt`.

| Project | Failed | Passed | Skipped | Total |
| --- | --- | --- | --- | --- |
| Lovelace.Abstractions.Tests | 0 | 20 | 0 | 20 |
| Lovelace.Array.Tests | 0 | 19 | 0 | 19 |
| Lovelace.Complex.Tests | 0 | 115 | 0 | 115 |
| Lovelace.Console.Tests | 0 | 15 | 0 | 15 |
| Lovelace.Dsp.Tests | 0 | 61 | 0 | 61 |
| Lovelace.Integer.Tests | 0 | 148 | 0 | 148 |
| Lovelace.Knowledge.Tests | 0 | 28 | 0 | 28 |
| Lovelace.Natural.Tests | 0 | 195 | 0 | 195 |
| Lovelace.Real.Tests | 0 | 2495 | 0 | 2495 |
| Lovelace.Representation.Tests | 0 | 91 | 0 | 91 |
| Lovelace.Run.Tests | 0 | 206 | 0 | 206 |
| Lovelace.Studio.Tests | 0 | 22 | 0 | 22 |
| Lovelace.Suite.Tests | 0 | 825 | 0 | 825 |
| Lovelace.Symbolics.Tests | 0 | 1119 | 8 | 1127 |
| precbench.Tests | 0 | 13 | 0 | 13 |
| **total** | **0** | **5372** | **8** | **5380** |

`Lovelace.Complex.Tests` 115 = the 108 the P-B1 round reported + the 7 new ones;
`Lovelace.Symbolics.Tests` 1119 = the 1097 + the 22 new ones.

Timing subsets, same configuration:

| Command | Result |
| --- | --- |
| `dotnet test Lovelace.Suite.Tests/… --filter "Category=Timing"` | Failed 0, Passed 8, Skipped 0, Total 8 (`timing-Lovelace.Suite.Tests.txt`) |
| `dotnet test Lovelace.Run.Tests/… --filter "Category=Timing"` | Failed 0, Passed 5, Skipped 0, Total 5 (`timing-Lovelace.Run.Tests.txt`) |
| `dotnet test Lovelace.Real.Tests/… --filter "Category=Timing"` | Failed 0, Passed 1, Skipped 0, Total 1 (`timing-Lovelace.Real.Tests.txt`) |

## 7. Could not verify / not in scope

1. **`Rl.Sqrt` of an inexact zero still answers an exact zero — a leak of the SAME class, in a file
   this round was not allowed to touch.** Measured post-fix, on the wire:
   ```
   CASE  sqrt(pi(30)-pi(30))          ->  {"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}
   CASE  sqrt(pi(1)-pi(1))            ->  {"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}
   CASE  evalf(sqrt(pi(30)-pi(30)), 40) -> {"kind":"Integer","value":"0","exact":true}
   ```
   The cause is `Lovelace.Real/Real.cs:1280-1281` (`if (IsZero(value)) return Zero;` — the exact
   Zero does not inherit the argument's `IsExact`). `ComplexMath.Sqrt` no longer exposes it, but
   `Real.Sqrt` does. Not fixed: the objective scopes (b) to `Lovelace.Complex/ComplexMath.cs` and
   forbids touching files outside that scope.
2. **Other `ComplexMath` entry points.** I probed the public surface with an inexact argument
   (temporary probe file, run and deleted) — `Ln`, `Atan`, `Atan2`, `AsinReal`, `AcosReal`,
   `AtanhReal`, `AsinhReal`, `AcoshReal`, `Log`, `Pow`, `Sin/Cos/Tan` of an inexact complex — all
   returned a part with `exact=false` after the fix, and I did not find a further leak in that
   sample. The probe was a sample, not a proof, and it is not part of the delivered tests.
3. **The related VALUE losses are untouched** (out of scope; changing a value to make a flag
   assertion pass is forbidden): `evalf(sin(pi(30)), 40)` still answers `{"kind":"Real","value":"0",
   "exact":false}` where mpmath gives 5.03…e-31, and the digits of a truncated constant are still
   only the constant's own (that is the pinned contract, not a defect).
4. **Nothing was run on the main tree.** The fix lives only in `.worktrees/c6-flag`; the main tree at
   `C:\Users\ricar\dev\LovelaceSharp` is untouched, and this document is the only file written
   outside the two worktrees.
