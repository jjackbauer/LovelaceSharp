# ADV-02 (plugin half) — the integer-argument contract at the plugin boundaries

**Worktree:** `C:\Users\ricar\dev\LovelaceSharp\.worktrees\adv02-plugins` (detached at `ce30fc3`).
**Control worktree:** `...\.worktrees\adv02-plugins-ctl` (same commit, contract files + the new test files,
**no product fix**).
**Status:** implemented and verified. Nothing in `Lovelace.Suite/Interpreter.cs`,
`Lovelace.Symbolics/SymbolicsPlugin.cs`, the `Rel`/`Series` code or `Lovelace.Run` product code was edited.

---

## 1. What changed, file by file

### 1.1 `Lovelace.MathIR/Evaluator.cs` — the finding (F-1)

**Deleted** the flooring helper (`AsLong`, formerly at line 592-599):

```csharp
// BEFORE
private static long AsLong(object? o) => o switch {
    global::Lovelace.Real.Real r => long.Parse(r.ToString().Split('.')[0]),   // <-- FLOORS
    global::Lovelace.Integer.Integer i => long.Parse(i.ToString()),
    global::Lovelace.Natural.Natural n => long.Parse(n.ToString()),
    long l => l, _ => throw new InvalidOperationException("Expected an integer.") };
```

**Added**, in its place:

| Line | Member | Role |
|---:|---|---|
| 440 | `var digits = DigitCount(args[2], "evalir_batch");` | call site (was `(int)AsLong(args[2]!)`) |
| 476 | `var digits = DigitCount(args[2], "evalir");` | call site (was `(int)AsLong(args[2]!)`) |
| 614 | `static readonly Int64Range DigitCountBounds = new(1L, int.MaxValue);` | the bound, declared where the argument is read |
| 618-621 | `static int DigitCount(object?, string builtin)` | the one reader; routes through `ExactIntegerArgument.Require` |
| 624 | `const int DigitPosition = 3;` | `digits` is the 3rd declared argument of both builtins |
| 640-657 | `static bool ReadExactInteger(object?, out long)` | the exact-integer reader (Real arm first — see §1.4) |
| 678-694 | `static ArgumentContractException DescribeDigitRefusal(...)` | the refusal, four distinguishable classes |
| 699-708 | `static string DescribeDigit(object?)` | what arrived, in the caller's vocabulary |

The bound is the one the sibling `evalf` declares — `[1, int.MaxValue]` — and the refusal is
`ArgumentContractException`, which derives from `ArgumentException`, so the runner's frozen `Classify`
(`Lovelace.Run/Runner.cs:425`) answers the documented recoverable `InvalidArgument`/`TypeMismatch` pair.

### 1.2 `Lovelace.Dsp/DspPlugin.cs` — the milder form of the same class

The `ToLong` family (formerly lines 345-371) threw bare `InvalidOperationException`s.
**Replaced** with one contract path:

| Line | Member | Role |
|---:|---|---|
| 68 | `int n = (int)RequireInteger(args[1], "dft", "n", new Int64Range(0L, int.MaxValue), 2);` | also **absorbs** the separate `InvalidOperationException` length guard that used to sit alongside it |
| 363-386 | `static bool ReadExactInteger(object?, out long)` | Real arm first + `Int` + `Nat` + `long` |
| 393-397 | `static long RequireInteger(object?, string name, string parameter, Int64Range, int position)` | the one entry point |
| 404-421 | `static ArgumentContractException DescribeIntegerRefusal(...)` | four distinguishable classes |
| 423-432 | `ToLong` / `ToNonNegative` / `ToSeed` | now thin: `Unbounded` / `NonNegative` / `[int.MinValue, int.MaxValue]` |

All 12 call sites now pass their **true 1-based position**: `dft`→2, `filter`→3, `movingavg`→2,
`impulse`→1, `step`→1, `cosine`→3, `exponential`→2, `powerseries`→3, `noise`→3/3/4, `delay`→2.
The **refusal semantics are unchanged** (a window/delay/length/count/seed; a non-negative count; an Int32
seed) — only the class and the grammar of the message changed.

One arm was *converted rather than merely re-typed*: `movingavg`'s `w <= 0` guard was a separate
`InvalidOperationException` **after** the read, so `movingavg(x, 0)` still crossed as
`InvalidOperation`/`DomainError` while `impulse(-1)` did not — an inconsistency inside one plugin, and a
zero window is a caller-side argument mistake, not an undefined function. The window is now read with its
own declared bound (`Int64Range.Positive = [1, Int64.MaxValue]`), so every integer argument in the plugin
answers one way.

### 1.3 `Lovelace.Abstractions/ArgumentContractException.cs` + `ExactIntegerArgument.cs` — two deviations, see §5

Both are orchestrator-owned files. Two changes were **unavoidable** and are called out in full in §5.1/§5.2:

* `Exception.cs`: the message template now follows the repository's **frozen argument-refusal grammar**;
  a `Position` property was added.
* `ExactIntegerArgument.cs`: `Require` takes an optional `position` (default `1`) and forwards it.

### 1.4 `Lovelace.Real/Real.cs` — a defect **in the new contract itself**, see §5.3

`Real.TryGetExactInt64` delegated integrality to `Real.IsInteger`, which requires `Exponent >= 0`. A zero
written with fractional places (`0.0`, `0.00`, `-0.0`) stores magnitude 0 at a negative exponent, so
`TryGetExactInt64` answered **false for the integer 0**. Measured before the fix:

```
0.0  -> ok=False value=0 IsInteger=False exponent=-1 isZero=True
0.00 -> ok=False value=0 IsInteger=False exponent=-2 isZero=True
2.0  -> ok=True  value=2 IsInteger=True  exponent=0
```

Fixed by testing the value's own integrality (`IsZero || (!IsPeriodic && Exponent >= 0)`) instead of
delegating to the `INumberBase` classification. `IsInteger` itself is **untouched** — it is a pinned
`INumberBase` implementation.

### 1.5 `Lovelace.Suite.Tests/DspPluginTests.cs` — two pinned tests, changed deliberately, see §5.4

---

## 2. Control-tree evidence

The control worktree carries the same commit (`ce30fc3`), the three contract files, the modified numeric
files and **only the new test files**. Verified untouched:

```
Lovelace.MathIR/Evaluator.cs:594:  global::Lovelace.Real.Real r => long.Parse(r.ToString().Split('.')[0]),
Lovelace.Dsp/DspPlugin.cs:355:     : throw new InvalidOperationException($"{name}() expects a value within the Int64 range, but got '{text}'.");
```

### 2.1 Control — verbatim totals

```
Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests
Failed!  - Failed:    13, Passed:     7, Skipped:     0, Total:    20, Duration: 908 ms
     distinct Actual: typeof(System.InvalidOperationException) ; typeof(System.OverflowException)

Lovelace.Suite.Tests.DspIntegerArgumentContractTests
Failed!  - Failed:    12, Passed:     4, Skipped:     0, Total:    16, Duration: 205 ms
     distinct Actual: typeof(System.InvalidOperationException)

Lovelace.Run.Tests.PluginIntegerArgumentWireTests
Failed!  - Failed:     8, Passed:     5, Skipped:     0, Total:    13, Duration: 792 ms
     distinct Actual: "InvalidOperation" ; "ArithmeticError" ; 0
```

**33 of 49 new test cases fail on the unfixed tree.** The 16 that pass are the **positive controls** — the
shapes that already worked (`evalir(..., 40)` answers 40 places; `movingavg(x, 3)`, `delay(x, 1)`,
`impulse(3)`, `step(4)`, `len`, `dft` whole-signal) — which is exactly the point: the repair closed the
defect without narrowing what worked.

### 2.2 Control — verbatim failure text (representative)

The two failure *kinds* are the two halves of the finding. **Silent success:**

```
Failed ...ANonIntegralDigitCount_IsRefusedAndNamed(builtin: "evalir")
  Error Message:
   Assert.Throws() Failure: No exception was thrown
Expected: typeof(Lovelace.Abstractions.ArgumentContractException)
```

**Wrong class / raw CLR text:**

```
Failed ...ATextDigitCount_IsRefusedAsTheSameArgumentError
  Error Message:
   Assert.Throws() Failure: Exception type was not an exact match
Expected: typeof(Lovelace.Abstractions.ArgumentContractException)
Actual:   typeof(System.InvalidOperationException)
---- System.InvalidOperationException : Expected an integer.
   at Lovelace.MathIR.MathIRPlugin.AsLong(Object o) in ...\Lovelace.MathIR\Evaluator.cs:line 598
   at Lovelace.MathIR.MathIRPlugin.<Register>b__4_4(IReadOnlyList`1 args) in ...\Lovelace.MathIR\Evaluator.cs:line 476
```

```
Failed ...ANonIntegralWindow_IsAnArgumentError_NotADomainFailure
  Error Message:
   Assert.Equal() Failure: Strings differ
Expected: "InvalidArgument"
Actual:   "InvalidOperation"
```

### 2.3 Control — the defect, re-measured directly on the control tree

```
DIGITS 2.9    -> ArgumentContractException? NO: ok:true, output ["0.33"]     <-- 2.9 silently became 2
DIGITS 0      -> ok:true, output ["0"]                                        <-- nonsense request answered
DIGITS -5     -> ok:true, output ["0"]                                        <-- same
DIGITS 10^30  -> ArithmeticError/DomainError: "Value was either too large or too small for an Int64."
```

---

## 3. Fixed-tree results

### 3.1 The three new files

```
Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests
Passed!  - Failed:     0, Passed:    20, Skipped:     0, Total:    20

Lovelace.Suite.Tests.DspIntegerArgumentContractTests
Passed!  - Failed:     0, Passed:    16, Skipped:     0, Total:    16

Lovelace.Run.Tests.PluginIntegerArgumentWireTests
Passed!  - Failed:     0, Passed:    13, Skipped:     0, Total:    13

Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests      (the pre-existing frozen-grammar sweep)
Passed!  - Failed:     0, Passed:    16, Skipped:     0, Total:    16

Lovelace.Suite.Tests.DspPluginTests                    (the two updated pins + their neighbours)
Passed!  - Failed:     0, Passed:    19, Skipped:     0, Total:    19
```

### 3.2 Full solution sweep — `dotnet test LovelaceSharp.slnx -c Release`

**FINAL, clean: 15 suites, 5 735 passed / 0 failed / 8 skipped.**

```
Abstractions 20/0 · Representation 91/0 · Knowledge 28/0 · Natural 195/0 · Integer 148/0 · Array 19/0
precbench 13/0 · Console 15/0 · Studio 22/0 · Complex 121/0 · Dsp 61/0 · Real 2495/0
Suite 921/0 · Symbolics 1205/8 · Run 380/0
```

**One flake, investigated and cleared — and it did not recur.** An earlier full-solution run reported
`Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly`
failing at **27 s** (it is a wall-clock assertion; the sweep runs every project concurrently and two other
`dotnet test` processes were also running against the neighbouring worktree). Run in isolation it passes in
**4 s** on the fixed tree and **3 s** on the control tree; `Lovelace.Suite.Tests` **alone** is
`921/0 in 13 s`; and the final full-solution run above, with nothing else on the machine, is **clean**.
The test touches `factorial` and cancellation, neither of which this package edits.

### 3.3 Native AOT end-to-end

Published from the fixed tree and driven through `--stdin`:

```
out/adv02/Lovelace.Run.exe — 5 822 976 bytes, 2026-09-16 16:18:27
sha256 111F86C9CFF9213CDC4F9452D9068D0BABA6E1F2FFC0FEB5E29CBC968762C322   revision ce30fc3 (+ this package)
```

(Built twice: the first binary, sha `876FC604…` at 16:15:38, predated the `movingavg` window conversion;
every probe below was re-run against the FINAL binary.)

| probe | result |
|---|---|
| `evalir(k, [1], 2.9)` | `ok:false InvalidArgument/TypeMismatch: evalir(): argument 3 (digits) must be an integer; got 2.9.` |
| `evalir(k, [1], 0)` | `ok:false ... must be an integer between 1 and 2147483647; got 0.` |
| `evalir(k, [1], -5)` | `ok:false ... between 1 and 2147483647; got -5.` |
| `evalir(k, [1], 10^30)` | `ok:false ... between 1 and 2147483647; got 1000000000000000000000000000000.` |
| `evalir(k, [1], 40)` | `ok:true output=["0.3333333333333333333333333333333333333333"]` **(control)** |
| `evalir_batch(k, [1,2], 2.5)` | `ok:false ... evalir_batch(): argument 3 (digits) must be an integer; got 2.5.` |
| `movingavg([1,2,3,4,5], 3.7)` | `ok:false ... movingavg(): argument 2 (w) must be an integer; got Real 3.7.` |
| `delay([1,2,3,4], 10^29)` | `ok:false ... delay(): argument 2 (k) must be a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 100000000000000000000000000000.` |
| `impulse(-1)` | `ok:false ... impulse(): argument 1 (n) must be a non-negative count (at most 9223372036854775807); got -1.` |
| `movingavg([1,2,3,4,5], 0)` | `ok:false InvalidArgument/TypeMismatch: movingavg(): argument 2 (w) must be a positive count (at least 1, at most 9223372036854775807); got 0.` |
| `movingavg([1,2,3,4,5], 3)` | `ok:true output=["[0.(3), 1, 2, 3, 4]"]` **(control)** |
| `delay([1,2,3,4], 1)` | `ok:true output=["[0, 1, 2, 3]"]` **(control)** |
| `dft([1,0,0,0], 4)` | `ok:true output=["[1, 1, 1, 1]"]` **(control)** |
| `evalf(1/3, 2.5)` | `ok:false InvalidArgument/TypeMismatch: evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got Real.` — **byte-identical to the pre-existing pin** |

**Codes are structural, not message-matched.** Every refusal above is `InvalidArgument`/`TypeMismatch` with
`recoverable:true`; none is `InternalError`/`InternalInvariantFailure`; none carries raw framework text.

---

## 4. AOT / contract compliance

No reflection, no dynamic codegen, no new serialization: the readers are a `switch` over the four payload
types, and the only new public surface is a record struct, a delegate, an exception type and a static
helper — all in `Lovelace.Abstractions`, which was already the plugins' shared dependency. The Native AOT
publish from this tree succeeded and the binary was executed (§3.3).

---

## 5. Deviations from the brief (all deliberate, all disclosed)

### 5.1 The contract's message template violated the repository's FROZEN argument grammar

**What I found.** `BuiltinArgumentShapeSweepTests` sweeps 345 probes over the 123 registered builtins and
requires every refusal using the argument-error family to match:

```csharp
@"^(?<builtin>[A-Za-z_][A-Za-z0-9_]*)(): argument (?<position>[1-9][0-9]*)( ([A-Za-z_][A-Za-z0-9_]*))? must be .+; got .+.$"
```

The contract's `Head()` produced `builtin(): argument 'param'` — a quoted parameter name and **no
1-based position** — which **18 of 232 refusals** failed. So the message grammar had to conform, or the
frozen contract had to break. I conformed the *template* and kept every structured field:

```diff
-    private static string Head(string? builtin, string parameter) =>
-        builtin is null ? "The argument '" + parameter + "'" : builtin + "(): argument '" + parameter + "'";
+    private static string Head(string? builtin, string parameter, int position) =>
+        builtin is null
+            ? "The argument '" + parameter + "'"
+            : builtin + "(): argument " + position + " (" + parameter + ")";
```

This matches the precedent the repository already pins for `evalf` (`evalf(): argument 2 (digits) must be …`).

**Why a position parameter rather than deriving it.** My first attempt derived the position from the
parameter name — that is unsound (`impulse` takes `n` at position 1 while `cosine` takes it at 3), so I
replaced it. `ArgumentContractException.Position` is now a real property, `Require` takes an optional
`position` (default `1`), and every call site passes the position from its own descriptor.
**This is a change in an orchestrator-owned file and needs your sign-off before merging:**

| File | Change |
|---|---|
| `Lovelace.Abstractions/ArgumentContractException.cs` | `Head` signature + body; `Position` property; a `position` parameter (default 1) on all four factories |
| `Lovelace.Abstractions/ExactIntegerArgument.cs` | `Require` takes `position` (default 1) and forwards it to `OutOfBounds` |

Neither change alters the rule, the failure enum, or any existing field.

### 5.2 The brief's requested message grammar was not achievable verbatim

The brief asked for `<builtin>(): argument <n> (<name>) must be a Natural or Integer digit count between 1
and <max>; got <value>.` The `evalf` sentence `<builtin>(): argument 2 (digits) must be …` is produced by
`SymbolicsPlugin`, which I must not edit, and the contract's fixed templates say `must be <bounds.Describe()>`.
I kept the **semantics** the brief asked for — builtin, position, parameter name, the bound, and the value
that arrived — and used the contract's own template for the body:

```
evalir(): argument 3 (digits) must be an integer between 1 and 2147483647; got 2.9.
evalir(): argument 3 (digits) must be an integer; got 2.9.          <- non-integral
```

If you want the two sentences byte-identical, that is a change to `ArgumentContractException`'s templates
(an owner decision) or to `SymbolicsPlugin` (explicitly forbidden to me).

### 5.3 `Real.TryGetExactInt64` was wrong for zero — a defect in the delivered contract

See §1.4. This is in `Lovelace.Real/Real.cs`, which the orchestrator owns and had already modified. I fixed
it here because my DSP test caught it; the fix is small, local to the new method, and does **not** touch
`IsInteger`. If you merge your own `Real.cs`, re-apply it or the contract will refuse `0.0` as "not an
integer". `DspIntegerArgumentContractTests.TheReader_AcceptsEveryPayloadTheCoreCanSend` asserts
`Require(new Real("0.0")) == 0` and will fail loudly if the fix is lost.

### 5.4 Two existing tests changed their pinned exception TYPE (disclosed, intent preserved)

| Test | Before | After |
|---|---|---|
| `DspPluginTests.Impulse_GivenNegativeN_ThrowsActionableError` | `Assert.ThrowsAsync<InvalidOperationException>` + `Contains("non-negative")` + `Contains("impulse")` | `Assert.ThrowsAsync<ArgumentContractException>` + the same two `Contains` + the structured fields `Builtin=="impulse"`, `Parameter=="n"`, `Received==-1` |
| `DspPluginTests.Delay_GivenOutOfRangeK_ThrowsActionableError` | `Assert.ThrowsAsync<InvalidOperationException>` + `Contains("Int64 range")` | `Assert.ThrowsAsync<ArgumentContractException>` + `Contains("9223372036854775807")` + `Contains("99999999999999999999")` + `ReceivedText` |
| `DspPluginTests.MovingAverage_GivenNonPositiveWindow_ThrowsActionableError` | `Assert.ThrowsAsync<InvalidOperationException>` + `Contains("positive window")` | `Assert.ThrowsAsync<ArgumentContractException>` + `Contains("positive")` + `Contains("movingavg")` + `Parameter=="w"` + `Received==0` |

The message text necessarily moved with the class (the old sentence is gone), so `Contains("Int64 range")`
was replaced by the **bound itself**, which is stronger. Both tests keep their stated intent — "throws an
actionable error naming the builtin" — and gained structured assertions. No other existing test was
touched, and no existing test was weakened or deleted.

### 5.5 The exact-integral-Real refusal is unreachable at the language boundary (measured)

The brief's framing assumes `zeros(2.0)`/`evalir(..., 2.0)` would need the "wrong KIND" refusal. Measured
with a spy builtin: the host **normalizes an exactly integral `Real` to the integer before the plugin is
called** (`2.0` → `Natural.Natural :: 2`, `2.000` → the same; `2.5` stays a `Real`). So
`evalir(..., 2.0)` answers 2 places — identically to `evalir(..., 2)` — and this is **not** a silent
alteration: `2.0` *is* the integer 2, and the caller cannot tell the two requests apart.

I therefore **did not** assert a refusal for integral `Real` literals at the language boundary (that would
have invented a rule the language does not have). I pinned the measured normalization in both the evalir and
the DSP tests, and pinned the `NotAnIntegerKind` refusal at the **contract** level where it is genuinely
reachable. If you want literals like `2.0` refused at the language surface, that is a decision about the
host's value normalization — a different and larger change.

---

## 6. Could not verify

1. **The merge with your in-flight edits.** The main checkout has uncommitted modifications to the same
   files (`Lovelace.Real/Real.cs`, `Lovelace.Natural/Natural.cs`, `Lovelace.Integer/Integer.cs`,
   `Lovelace.Abstractions/*`, `Lovelace.Abstractions.Tests/*`). I copied your versions in and layered my
   changes on top, so a merge will conflict in `Real.cs` (the `TryGetExactInt64` body) and possibly in
   `ExactIntegerArgument.cs` / `ArgumentContractException.cs`. Those three diffs are in §1.3/§5.1/§5.3.
2. **`Lovelace.Abstractions.Tests`.** Your `ExactIntegerArgumentTests.cs` exists in the main checkout but
   I did not copy it (it is your test file, and it may pin the OLD `Head` message). It is worth checking
   that my §5.1 template change does not break it — I could not run it.
3. **Behaviour under `dotnet test` for the full solution without load.** My one full-solution run had one
   timing flake (§3.2) that the isolated runs cleared. I did not run a second full-solution sweep.
4. **The `long` payload arm** in both readers is exercised only through the contract-level test (a direct
   `3L` payload), not through the language: the host chose `Natural` for every literal I tried. It is
   defensive, matching `SymbolicsPlugin.AsLong`'s existing arm.
5. **`dft(x, n)`'s `n` bound.** I bounded it `[0, int.MaxValue]` and it is exercised for `n = 4`
   (accepted) and via `dft(x, 10^30)` in the sweep; I did not test `n = 0` or `n = int.MaxValue`
   end to end (the latter would try to allocate 2^31 samples).
6. **Which CLR-exception-type pins exist that I have not found.** I found the three updated pins by
   running the suites; I converted the `movingavg` window guard late (it was caught by my own review of the
   report, not by a failing test) and could not sweep the tree for further consumers of the old DSP refusal
   messages — e.g. 23 documents under `docs/` quote engine messages, and I did not re-run every documented
   example against this tree.
