# L1 (P1) — the structured payload carries every digit the value stores

**Round 17 / cycle 6 — implementation record. Trees used: the fix tree is a scratch worktree,
`.worktrees/c6-l1` (detached HEAD `54b1753`), created with `git worktree add .worktrees/c6-l1 HEAD`;
the control tree is `.worktrees/c6-l1ctl`, created the same way. Nothing was staged, committed or
pushed.** In the fix tree `git status --short` reads ` M Lovelace.Run/Runner.cs`,
`?? Lovelace.Run.Tests/StructuredRealPrecisionTests.cs`, `?? .l1/` (artifacts). The main checkout
`C:\Users\ricar\dev\LovelaceSharp` received **no source edit** — only this document and the logs
beside it.

Finding: `docs/goal-cycle-6/round-13/audit-B-lattice.md:40-59` (L1, P1, NEW), lattice row `:311`
(`evalf(pi, 1100)` → 100 decimals) and final-table row `:318`; the audit's "could not check #4"
(`:339-341`) asked whether the cap also reached `variables[]` — it did, at the process default
precision (§1a, last row: `evalf(pi, 1000)` cut the variable's structure to 100 while the value owned
1 000), and it did not under `setprecision`, because the variable path already rendered at the
engine's precision. §5 shows both after the fix.

## 0. What was wrong

The envelope's structured value was rendered **outside every precision scope**. At HEAD the result
payload was built at `Lovelace.Run/Runner.cs:222-225`:

```csharp
: new ResultDto(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result),
    StructuredProjection.ToStructured(result, structuredBudget));
```

`ValueFormatter` and `StructuredProjection.ToStructured` read the *ambient* `Real` precision, and
the statement's precision scope is disposed when the evaluation returns. `Real.ToString()` truncates a
non-periodic fraction at `DisplayDecimalPlaces` — `Lovelace.Real/Real.cs:2474-2475`, documented at
`:54-58` as a DISPLAY control ("Controls how many fractional digits appear in `ToString()` for
non-periodic values. Default is 100", `Real.cs:43`) — so the machine payload was cut at the process
default of 100 decimals. The engine already had the correct seam and the *variables* used it:
`SuiteEngine.ProjectValue` (`Lovelace.Suite/SuiteEngine.cs:115-120`) renders "under this engine's
display precision, so a value's human rendering and its machine-readable form come from one set of
settings" (`:107-113`), and `CaptureState` renders each variable's `display` under the same scope
(`:340-353`). That is why the *same value in one envelope* carried 1 100 digits as
`variables[_].structured.value` and 100 as `result.structured.value`.

Nothing marked the cut: the DTO has `Truncated`/`TruncationReason`/`Budget`, commented "set only when
a print budget abbreviated the rendering (never silently)" (`Lovelace.Suite/StructuredProjection.cs:33-36`),
and the protocol document requires a bounded structured rendering to carry them
(`docs/symbolics/dsh-protocol.md:17-20`) — but the Real branch sets none of them
(`StructuredProjection.cs:168-182`): it emits `value.ToString()` and `exact`.

**Mechanism nuance, observed:** `Real.ToString`'s all-fractional branch
(`Real.cs:2460-2467`) takes `Math.Min(fracLen, fracLen + DisplayDecimalPlaces) == fracLen`, so a value
whose integer part is empty was **never** cut there. Probed pre-fix on the published binary:
`evalf(pi, 1000) - pi(100)` at default precision → `display` **1 000** decimals. The finding's "every
Real" is therefore exact for the integer-part branch and too broad for the all-fractional one; the fix
does not depend on which branch runs.

## 1. Pre-fix observation (re-observed by me, not copied)

### 1a. The published binary

`out/aot/Lovelace.Run.exe` — 5 776 896 B, 2026-09-12 15:06:37 (the same binary the P0 record used).
Raw transcript: `published-aot-prefix.txt`.

| script (`--json --omit-functions`) | exit | `result.structured.value` | `result.display` | `variables[_].structured` | `output[0]` | `truncated` |
|---|---|---|---|---|---|---|
| `setprecision(1100); pi(1100)` | 0 | **100** | **100** | 1 100 | — | absent |
| `setprecision(1100); print(pi(1100)); pi(1100)` (`--omit-variables`, the audit's own command) | 0 | **100** | **100** | — | **1 100** | absent |
| `pi(30)` | 0 | 30 | 30 | 30 | — | absent |
| `evalf(pi, 40)` | 0 | 40 | 40 | 40 | — | absent |
| `evalf(pi, 1000)` (default ambient precision) | 0 | **100** | **100** | **100** | — | absent |

`setprecision(1100); pi(1100)` therefore crossed as
`{"kind":"Real","value":"3.14159…1170679","exact":false}` — 102 characters, ending at the **100th**
decimal, with `variables[_]` in the same envelope ending `…28347913151` (the 1 100th). The audit's
`output[0]` claim is confirmed: the print channel carried the 1 100 digits it was asked for; the
machine channel did not.

### 1b. The fix tree at HEAD (same tree the fix lands in)

`dotnet build Lovelace.Run\Lovelace.Run.csproj -c Release` then `dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll …`.
The "before" rows of `probes.tsv` are identical to 1a: `structured=100 display=100 var.structured=1100
var.display=1100` for the 1 100-place script; `30/30` and `40/40` for the two controls; `100/100/100`
for `evalf(pi, 1000)` at default precision. `evalf(pi, 1000)` **stores** 1 000 digits — probed:
`evalf(pi, 1000) - pi(999)` is non-zero (its first significant digit sits at decimal place 1 000) — so
1b is the same silent cut, on a count a builtin was explicitly asked for.

## 2. The contract, from the repository's own words

**Contract (a): the structured value carries the value's OWN STORED digits, in full.** Where I read it:

* `docs/symbolics/dsh-protocol.md:3-5` — the envelope exists "so that an agent never has to parse a
  display string to recover mathematical meaning". `:28-41` lists the value forms and bounds no Real
  `value`; the document's one structured bound is `--print-budget`, and `:17-20` makes a bounded
  rendering report `truncated`/`truncationReason`/`budget` — "never silently". The document is
  **silent** on any digit bound for a Real.
* `Lovelace.Suite/StructuredProjection.cs:168-182` — the Real branch is `value.ToString()` plus
  `exact`; it documents no bound and sets no marker.
* `Lovelace.Real/Real.cs:54-58` — `DisplayDecimalPlaces` documents itself as a **display** control
  ("how many fractional digits appear in `ToString()`", the C++ `casasDecimaisExibicao`), not a
  payload bound.
* `Lovelace.Symbolics/Expr.cs:104-121` — the repository's own precedent for crossing a value into a
  machine form: `RealLiteral.FromRealExact` reads the "full-precision magnitude and exponent —
  **never the display-truncated string** (`ToString` respects the ambient display scope)", copying
  "every stored digit … none is rendered or rounded".

So the documents bound no digit count for a structured Real (and the protocol forbids a silent cut),
and the one existing bound is documented as a display bound. Per the round's rule — if the documents
are silent, take (a) — **the payload carries the stored digits**. I did not choose (b): inventing a
documented bound plus a marker would have meant keeping a cut the finding exists to remove, and the
DTO's marker fields are contractually the print budget's (`StructuredProjection.cs:33-36`).

The display side keeps its documented bound: `result.display`/`result.typed` are now rendered at the
engine's display precision (the setting `variables[].display` already used for the same value,
`SuiteEngine.cs:100-127`, `:340-353`), and the display bound stays out of the structure.

## 3. The failing test, written FIRST

New file `Lovelace.Run.Tests/StructuredRealPrecisionTests.cs` (9 cases, in the fix tree). It asserts
**digit counts**, not "longer strings", and it pins both directions of the contract: the payload may
not carry fewer digits than the value stores, and it may not pad beyond them. Constant anchors are
mpmath's (`mp.dps = 1200`, `LOVELACE_REQUIRE_SYMPY=1`): pi's first 60 decimals, its digits
981-1000 (`66111959092164201989`) and 1081-1100 (`24972177528347913151`); the 1 000th and 1 100th
decimals of pi are 9 and 1 (of e, 8; of sqrt(2), 1) — no trailing-zero drop is involved.

| test | contract it asserts |
|---|---|
| `RequestedPrecisionBeyondTheOldCap_CrossesEveryStoredDigit` | `setprecision(1100); pi(1100)` → exactly **1 100** decimals, pi's prefix and 1 081-1 100 tail, `exact:false`, no numerator, **no `truncated`**, and equal to `variables[_].structured.value` |
| `EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull` | `evalf(pi, 1000)` at DEFAULT precision → exactly **1 000** decimals, same content anchors, equal to the variable's structured value (the value's digits, not the display setting's) |
| `CountsAtOrBelowTheRequest_AreCarriedExactly` (4 cases) | `pi(30)` → 30, `evalf(pi, 40)` → 40 (the finding's controls), `setprecision(1100); sqrt(2)` → 1 100, `setprecision(1100); e(1100)` → 1 100 |
| `StoredDigitsAreNotPaddedToAnyBound` | `evalf(sqrt(2), 5)` → `"1.41421"` — no padding up to a bound |
| `NestedReals_InsideAnArray_CrossInFullToo` | `setprecision(1100); [pi(1100), e(1100)]` → 1 100 decimals per element (the room belongs to the projection, not to one call shape) |
| `TheResultsDisplay_FollowsTheScriptsPrecision` | the result's `display`/`typed` are the engine-precision renderings (1 100), agreeing with `variables[_].display` |

### Observed failure BEFORE the fix

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-l1
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~StructuredRealPrecisionTests"
```

```
  Failed …CountsAtOrBelowTheRequest_AreCarriedExactly(script: "setprecision(1100); sqrt(2)", expectedDigits: 1100) [9 ms]
Expected: 1100
Actual:   100
  Failed …CountsAtOrBelowTheRequest_AreCarriedExactly(script: "setprecision(1100); e(1100)", expectedDigits: 1100) [8 ms]
Expected: 1100
Actual:   100
  Failed …RequestedPrecisionBeyondTheOldCap_CrossesEveryStoredDigit [9 ms]      Expected: 1100  Actual: 100
  Failed …NestedReals_InsideAnArray_CrossInFullToo [22 ms]                       Expected: 1100  Actual: 100
  Failed …TheResultsDisplay_FollowsTheScriptsPrecision [9 ms]                    Expected: 1100  Actual: 100
  Failed …EvalfCountAboveTheAmbientDisplayPrecision_CrossesInFull [18 ms]        Expected: 1000  Actual: 100

Failed!  - Failed:     6, Passed:     3, Skipped:     0, Total:     9, Duration: 309 ms - Lovelace.Run.Tests.dll (net10.0)
```

Full log: `prefix-run.log` (exit 1). The three that passed are exactly the controls — `pi(30)`,
`evalf(pi, 40)` (both below the old cap) and `StoredDigitsAreNotPaddedToAnyBound` — which is what a
failing-first run must show: only the claims about the defect fail.

## 4. The fix

`Lovelace.Run/Runner.cs` only — 43 insertions, 4 deletions. The result's structured form and every
variable's go through one helper that renders the SHARED projection with room for the digits the value
stores; the result's human strings go through the engine's own precision-aware seams.

```diff
diff --git a/Lovelace.Run/Runner.cs b/Lovelace.Run/Runner.cs
index 7c531a3..bd5af70 100644
--- a/Lovelace.Run/Runner.cs
+++ b/Lovelace.Run/Runner.cs
@@ -4,6 +4,8 @@ using System.Text.Json;
 using System.Text.Json.Serialization.Metadata;
 using Lovelace.Dsp;
 using Lovelace.Suite;
+// the precision scope the structured projection is rendered under (see StructuredValue)
+using Rl = global::Lovelace.Real.Real;
 
 namespace Lovelace.Run;
 
@@ -221,8 +223,8 @@ public static class Runner
 
             ResultDto? resultPayload = result.Kind == ValueKind.Void
                 ? null
-                : new ResultDto(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result),
-                    StructuredProjection.ToStructured(result, structuredBudget));
+                : new ResultDto(result.Kind.ToString(), engine.FormatValue(result), engine.FormatValueTyped(result),
+                    StructuredValue(engine, result, structuredBudget));
 
             // the deadline verdict is derived from the SAME elapsed time the envelope publishes, so
             // budget and elapsed are directly comparable (see CancellationDto)
@@ -427,9 +429,46 @@ public static class Runner
     private static Lovelace.Symbolics.Printing.PrintBudget? PrintBudgetOrNull(int? maxNodes) =>
         maxNodes is { } nodes ? new Lovelace.Symbolics.Printing.PrintBudget(MaxNodes: nodes) : null;
 
+    /// <summary>
+    /// The fractional-digit room a STRUCTURED rendering is given, as opposed to the display bound
+    /// <see cref="Rl.DisplayDecimalPlaces"/> imposes on <c>ToString</c>. The machine payload carries
+    /// the digits the value STORES, so the room must be larger than any value can be: a Real with a
+    /// billion fractional digits is roughly a gigabyte of decimal text, and every route into one is
+    /// bounded far below that (<c>pi</c>/<c>e</c> refuse counts past the 1000-place static cap, and
+    /// <c>evalf</c> clamps its count to the same 1000 — <c>SymbolicsPlugin.EvalfPayload</c>), while
+    /// <c>setprecision(n)</c> has to materialise the n digits it names. The number is also kept
+    /// inside the range a renderer can address at all: <c>Real.ToString</c> adds it to the stored
+    /// fractional length in <see cref="int"/> arithmetic, so it must stay well below
+    /// <see cref="int.MaxValue"/> for every reachable value. It is a guard against an unreachable
+    /// value, not a policy bound: no digit a script asks for is ever cut at it.
+    /// </summary>
+    private const long StructuredDecimalDigits = 1_000_000_000L;
+
+    /// <summary>
+    /// The structured form of one value — the machine API's copy of it, and the SAME projection the
+    /// result and every variable carry. It is rendered with room for the digits the value stores
+    /// rather than for the digits the display setting shows (L1, audit B): <c>Real.ToString()</c>
+    /// truncates a non-periodic fraction at <see cref="Rl.DisplayDecimalPlaces"/>
+    /// (<c>Lovelace.Real/Real.cs:2474-2475</c>), so a payload projected under the process default
+    /// carried exactly 100 decimals for a <c>setprecision(1100); pi(1100)</c> that owns 1 100 — the
+    /// display string lost the rest in silence, with none of the DTO's
+    /// <c>truncated</c>/<c>truncationReason</c>/<c>budget</c> fields set, although the protocol
+    /// requires a bounded structured rendering to say so (<c>docs/symbolics/dsh-protocol.md:17-20</c>)
+    /// and the symbolic tier already reads values off the "full-precision magnitude and exponent —
+    /// never the display-truncated string" (<c>Lovelace.Symbolics/Expr.cs:104-112</c>). The display
+    /// bound stays a DISPLAY bound (<c>Real.cs:54-58</c>): <c>display</c>/<c>typed</c> are rendered at
+    /// the engine's precision, the structure is rendered in full.
+    /// </summary>
+    private static StructuredValueDto StructuredValue(SuiteEngine engine, Value value,
+        Lovelace.Symbolics.Printing.PrintBudget? budget)
+    {
+        using var _ = Rl.WithPrecision(engine.ComputationDecimalPlaces, StructuredDecimalDigits);
+        return StructuredProjection.ToStructured(value, budget);
+    }
+
     /// <summary>
     /// The structured form of one captured variable — the SAME projection the result carries, read
-    /// off the LIVE value under the engine's display precision so a variable's <c>display</c> string
+    /// off the LIVE value (see <see cref="StructuredValue"/>) so a variable's <c>display</c> string
     /// and its <c>structured</c> form come from one set of settings (A2-F24). The snapshot and the
     /// live dictionary are the same capture: no evaluation runs between them, so the value is always
     /// there; a lookup that somehow misses still crosses as <c>{"kind":"Null"}</c> rather than
@@ -438,7 +477,7 @@ public static class Runner
     private static StructuredValueDto ProjectVariable(SuiteEngine engine, string name,
         Lovelace.Symbolics.Printing.PrintBudget? budget) =>
         engine.TryGetVariable(name, out var value)
-            ? engine.ProjectValue(value, budget)
+            ? StructuredValue(engine, value, budget)
             : new StructuredValueDto("Null");
 
     /// <summary>
```

Why a room constant instead of the engine's display precision: the engine precision is *not* an upper
bound on what a value stores. `setprecision(n)` has no upper bound (`Lovelace.Suite/Interpreter.cs:1826-1841`
only refuses `n <= 0`), and a value can outlive the setting that made it — measured after the fix:
`setprecision(2000); x = sqrt(2); setprecision(50); x` → `x.structured` **2 000** decimals,
`x.display` **50**, `result.structured` **2 000**, `result.display` **50**. Rendering the payload
under *any* user-settable number would re-introduce a silent cut for some script.
`1_000_000_000` is a guard, not a policy bound: every route into a Real is capped far below it
(`pi`/`e` refuse counts past the static 1000-place cap — `pi(2000)` is refused both at default and
at `setprecision(1000)`; `evalf` clamps at `Math.Min(digits, 1000)`,
`Lovelace.Symbolics/SymbolicsPlugin.cs:1730-1732`), and the number stays inside the `int` range
`Real.ToString` adds it in (`Real.cs:2464`, `:2474-2475`), which is why `long.MaxValue` would be
unsafe rather than merely unnecessary. No digit a script asks for is cut at it.

## 5. After

`probes.tsv`, both rows from the same tree and the same `dotnet build -c Release`:

| probe (all `--json --omit-functions`) | `result.structured` before → after | `result.display` before → after | `variables[_].structured` before → after | `output[0]` |
|---|---|---|---|---|
| `setprecision(1100); pi(1100)` | **100 → 1 100** | **100 → 1 100** | 1 100 → 1 100 | — |
| `setprecision(1100); print(pi(1100)); pi(1100)` | **100 → 1 100** | **100 → 1 100** | 1 100 → 1 100 | 1 100 |
| `pi(30)` | 30 → 30 | 30 → 30 | 30 → 30 | — |
| `evalf(pi, 40)` | 40 → 40 | 40 → 40 | 40 → 40 | — |
| `evalf(pi, 1000)` (default precision) | **100 → 1 000** | 100 → 100 | **100 → 1 000** | — |
| `setprecision(1100); sqrt(2)` | **100 → 1 100** | **100 → 1 100** | 1 100 → 1 100 | — |
| `setprecision(1100); evalf(pi, 1000)` | **100 → 1 000** | **100 → 1 000** | 1 000 → 1 000 | — |

`evalf(pi, 1000)` at default precision is the case that separates contract (a) from "render at the
engine's display precision": the value owns 1 000 digits, the human rendering in force shows 100, and
the payload now carries the 1 000 (`envelope-excerpts.txt`). The same separation under a LOWERED
setting: `setprecision(2000); x = sqrt(2); setprecision(50); x` renders 50 decimals and carries
2 000 — the display bound is in force for the human string and is not in force for the structure.

Content, not just counts: after the fix `result.structured.value` equals `variables[_].structured.value`
and, for the print probe, `output[0]` — the channel the auditor compared digit-by-digit against
mpmath. The 1 100-digit payload starts `3.141592653589793238462643383279502884197169399375105820974944`
and ends `24972177528347913151`; `exact` stays `false` and no numerator/denominator is invented;
`truncated` stays ABSENT (nothing was cut, so nothing claims a cut).

Shape untouched, verified mechanically on `before-p1100.json` vs `after-p1100.json`: 34 JSON paths in
both, **no path added and none removed**; `protocolVersion=1`, `symbolicFormatVersion="#!lovelace-sym 1"`,
`mathIrVersion=2` before and after. Under default precision the `display`/`typed` strings are
byte-identical to what the old call site produced (`engine.UnicodeOutput` defaults to `false`, so
`engine.FormatValue` == `ValueFormatter.Format(value)` there) — which is why no golden fixture moved:
`git status` in the fix tree lists no file under `Lovelace.Run.Tests/fixtures/`.

Post-fix run of the new file: `Passed! - Failed: 0, Passed: 9, Skipped: 0` (`postfix-run.log`).
Process note: the first "after" pass ran against a stale binary — `Copy-Item` preserved the restored
file's older timestamp and MSBuild's up-to-date check skipped the compile, so the probes repeated the
pre-fix numbers (`after-test.log`). The file was re-timed, the DLL timestamp moved, and every number
in this table comes from the rebuilt binary.

## 6. Control

`git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-l1ctl HEAD`, then ONLY the new
test file copied in (`git status --short` there: `?? Lovelace.Run.Tests/StructuredRealPrecisionTests.cs`;
no product file differs from HEAD). Same command, same filter:

```
Failed!  - Failed:     6, Passed:     3, Skipped:     0, Total:     9, Duration: 1 s - Lovelace.Run.Tests.dll (net10.0)
```

exit 1, the same six cases, the same `Expected: 1100/1000, Actual: 100` (`control-run.log`). The tests
have teeth on a pristine tree.

## 7. Suite

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-l1
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH ; $env:LOVELACE_REQUIRE_SYMPY='1'
dotnet test LovelaceSharp.slnx --configuration Release --nologo
```

`SOLUTION_EXIT=0`, **0 failed in all 15 test projects**, 0 skipped (`solution-test-oracle.log`):

| project | failed | passed | total |
|---|---|---|---|
| Lovelace.Abstractions.Tests | 0 | 20 | 20 |
| Lovelace.Natural.Tests | 0 | 195 | 195 |
| Lovelace.Representation.Tests | 0 | 91 | 91 |
| Lovelace.Integer.Tests | 0 | 148 | 148 |
| Lovelace.Array.Tests | 0 | 19 | 19 |
| Lovelace.Knowledge.Tests | 0 | 28 | 28 |
| precbench.Tests | 0 | 13 | 13 |
| Lovelace.Console.Tests | 0 | 15 | 15 |
| Lovelace.Complex.Tests | 0 | 115 | 115 |
| Lovelace.Real.Tests | 0 | 2 495 | 2 495 |
| Lovelace.Dsp.Tests | 0 | 61 | 61 |
| Lovelace.Suite.Tests | 0 | 828 | 828 |
| Lovelace.Run.Tests | 0 | 256 | 256 |
| Lovelace.Symbolics.Tests | 0 | 1 148 | 1 148 |
| Lovelace.Studio.Tests | 0 | 22 | 22 |
| **total** | **0** | **5 454** | **5 454** |

The same solution run WITHOUT the oracle environment also exited 0 with 0 failed (5 446 passed,
8 skipped in `Lovelace.Symbolics.Tests`; `solution-test.log`) — with `LOVELACE_REQUIRE_SYMPY=1` those 8
ran and passed, so the oracle run is the one quoted above.

Timing subsets (`--filter "Category=Timing"`), all exit 0 (`timing-subset.log`):
Suite **8/8**, Run **5/5**, Real **1/1**, Symbolics **1/1** — 15 passed, 0 failed.
Symbolics `Category=Costly` subset: **32/32**, exit 0 (`costly-subset.log`).

## 8. Could not verify / residuals

1. **No AOT republish.** The fix is verified on the Release JIT build and through the in-process
   tests; I did not re-publish the Native AOT binary (the pre-fix AOT binary and the pre-fix JIT build
   agree on every measured field, but the post-fix numbers are JIT numbers).
2. **`result.display`/`typed` changed too** (100 → 1 100 for the 1 100-place script). This is a
   deliberate part of the same call-site fix (the value's human rendering at the script's precision,
   `SuiteEngine.cs:100-127`); it is byte-identical under default precision and moves no fixture, but
   it is a visible change beyond `structured`, so it is recorded rather than buried.
3. **The room constant is an unreachable guard, not a proof.** A Real with more than ~1.14e9
   fractional digits would overflow `ToString`'s `int` arithmetic before reaching it; such a value
   is ~1 GB of decimal text and no route in the language produces one.
4. **Periodic Reals were never capped** (`Real.cs:2480-2506` ignores `DisplayDecimalPlaces`) and are
   unchanged; exact Reals already crossed their exact rational (`StructuredProjection.cs:174-181`) and
   still do.
5. **Other hosts unchanged.** `Lovelace.Studio` and the Suite tests call
   `StructuredProjection.ToStructured` directly and still render under whatever precision scope is
   ambient there — out of this round's scope (`Lovelace.Suite/**`), and not measured.
6. **`--print-budget` semantics unchanged**: the Real branch still never consults the budget and the
   DTO's marker fields still fire only on the symbolic path (`PrintBudgetTotalityTests` passes).
7. **Audit B's other precisions findings remain open** — L3 (last delivered decimal truncated rather
   than rounded), L4 (`evalf` over-delivery) and L5 (`pi(30)` at `setprecision(20)` leaking a raw
   .NET message) are different defects and were not touched.
8. Shell: Windows PowerShell 5.1; the solution/test commands above are the exact ones run.
