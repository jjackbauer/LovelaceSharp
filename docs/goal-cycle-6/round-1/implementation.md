# Goal cycle 6 — round 1 implementation: correct the stale `solve` help-contract pins

Role: Implementer. Scope: **exactly one file edited** — `Lovelace.Console.Tests/ReplOutputTests.cs`.
Nothing in the product (`Lovelace.Symbolics/**`, `Lovelace.Suite/**`, `Lovelace.Console/**`), CI config,
other test files or JSON fixtures was touched. Verified by `git status --porcelain` (see §4).

All commands were run from `C:\Users\ricar\dev\LovelaceSharp` in Windows PowerShell 5.1
(`$PSVersionTable.PSVersion` = 5.1.26100.4202), `dotnet --version` = 10.0.103.

---

## 1. BEFORE the edit — observed failure

Command (verbatim):

```
dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj --configuration Release --nologo
```

Observed: **exit code 1**. Failing test: `Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary`.
Raw output (tail, reproduced as emitted):

```
dotnet : [xUnit.net 00:00:00.17]     Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary [FAIL]
  Failed Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary [30 ms]
  Error Message:
   Assert.Contains() Failure: Sub-string not found
String:    "» help solve\r\nsolve(f, x [, domain])\r\n\r\nS"···
Not found: "Solves an equation for x."
  Stack Trace:
     at Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Console.Tests\ReplOutputTests.cs:line 63
--- End of stack trace from previous location ---

Failed!  - Failed:     1, Passed:    14, Skipped:     0, Total:    15, Duration: 131 ms - Lovelace.Console.Tests.dll (net10.0)
```

So: exit 1, `Failed: 1, Passed: 14, Total: 15`, failing at **`ReplOutputTests.cs:63`** — the pre-edit
`Assert.Contains("Solves an equation for x.", t)`. The second stale pin, line 65's
`Assert.Contains("Returns: Vector | Text", t)`, sits behind the line-63 failure and was not reached in that run.

---

## 2. Product sources that define the pinned strings

**Summary text and return kind** — `Lovelace.Symbolics/SymbolicsPlugin.cs:451-455` (read directly):

```csharp
451        Add("solve", new[] { "f", "x", "domain" }, args => SolveResultRecord(args),
452        new BuiltinDescriptor("solve", new[] { "f", "x", "domain" }, BuiltinCategories.Solving,
453            "Solves an equation for x and returns the SAME SolveResult record solve_full returns: status, domain (a Domain value), complete/completeness, per-solution conditions/multiplicity/exactness, parametric families and diagnostics. The default domain is Complex; pass real for real solutions only.",
454            ["solve(x^2 - 4 == 0, x)", "solve(x^2 + 1 == 0, x, real)"], "SolveResult",
455            ["solve_full", "solve_system", "linsolve"], MinArity: 2));
```

* the summary is argument 4 of `BuiltinDescriptor` (line 453);
* the **return kind is @"SolveResult"@** (line 454) — no longer `Vector | Text`;
* `Parameters = {f, x, domain}` with `MinArity: 2` (lines 452 and 455) ⇒ signature `solve(f, x [, domain])`;
* `Related = [solve_full, solve_system, linsolve]` (line 455).

**Rendering** — `Lovelace.Suite/HelpService.cs:164-181` (`HelpService.Function`):

```csharp
165        sb.AppendLine(Signature(name, descriptor));
166        sb.AppendLine();
167        sb.AppendLine(descriptor.Summary);
...
175        sb.AppendLine();
176        sb.AppendLine($"Returns: {descriptor.ReturnKind}");
177        if (descriptor.Related.Count > 0)
178            sb.AppendLine($"See also: {string.Join(", ", descriptor.Related)}");
179        if (fn.PluginName is { } plugin)
180            sb.AppendLine($"Plugin: {plugin}");
```

**Where it reaches the transcript** — `Lovelace.Console/Repl/ReplSession.cs:70-79`:
`_output.WriteLine(help.Lookup(arg.Trim()) ...)` (line 78); in the test the sink is the injected
`StringWriter` (`ReplHarness.cs:15-18`). `StringWriter.WriteLine` performs no wrapping and
`HelpService` inserts no newline inside the summary, so the summary is one unwrapped line.

**Exact literal text the product emits (observed, not inferred).** I drove the real `ReplSession`
headlessly with a throwaway .NET 10 file-based program kept **outside the repository**
(`C:\Users\ricar\AppData\Local\Temp\lovelace-observe\observe_help_solve.cs`, referencing
`Lovelace.Console.csproj`). The live console cannot be used: `Program.cs:13` builds
`new ReplSession()`, whose `LineEditor` calls `Console.ReadKey` and throws on redirected stdin
(`LineEditor.cs:119`; observed exit 1, `InvalidOperationException: Cannot read keys when either application does not have a console...`).
Observed dump (`dotnet run ...observe_help_solve.cs`, exit 0), one line per transcript line with its length:

```
[22] solve(f, x [, domain])
[0]
[291] Solves an equation for x and returns the SAME SolveResult record solve_full returns: status, domain (a Domain value), complete/completeness, per-solution conditions/multiplicity/exactness, parametric families and diagnostics. The default domain is Complex; pass real for real solutions only.
[0]
[9] Examples:
[26]     solve(x^2 - 4 == 0, x)
[32]     solve(x^2 + 1 == 0, x, real)
[0]
[20] Returns: SolveResult
[44] See also: solve_full, solve_system, linsolve
[27] Plugin: Lovelace.Symbolics
```

I also checked programmatically that this observed 291-character line occurs **verbatim** inside the
source literal on `SymbolicsPlugin.cs:453` (substring test → `true`), so the test pins the product's own
text rather than a re-typed copy.

Corroboration that this is the intended contract, not an accident: the sibling suite already pins the new
return kind — `Lovelace.Suite.Tests/HelpServiceTests.cs:59`: `Assert.Contains("Returns: SolveResult", text);`
(unchanged, out of scope). I ran that class as a cross-check:
`dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~HelpServiceTests'`
⇒ **exit 0**, `Passed! - Failed: 0, Passed: 8, Skipped: 0, Total: 8`.

---

## 3. AFTER the edit — observed result

Command (verbatim, identical to §1):

```
dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj --configuration Release --nologo
```

Observed: **exit code 0**, no `[FAIL]` line, final summary as emitted:

```
Test run for C:\Users\ricar\dev\LovelaceSharp\Lovelace.Console.Tests\bin\Release\net10.0\Lovelace.Console.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.

Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15, Duration: 153 ms - Lovelace.Console.Tests.dll (net10.0)
```

---

## 4. `git diff -- Lovelace.Console.Tests/ReplOutputTests.cs`

```diff
diff --git a/Lovelace.Console.Tests/ReplOutputTests.cs b/Lovelace.Console.Tests/ReplOutputTests.cs
index c99f2db..30294bf 100644
--- a/Lovelace.Console.Tests/ReplOutputTests.cs
+++ b/Lovelace.Console.Tests/ReplOutputTests.cs
` -60,9 +60,19 ` public class ReplOutputTests
         string t = await ReplHarness.RunAsync("help solve\nexit\n");
 
         ReplHarness.AssertHasLine(t, "solve(f, x [, domain])");
-        Assert.Contains("Solves an equation for x.", t);
+        // The summary is emitted as ONE unwrapped line: HelpService appends descriptor.Summary
+        // verbatim (Lovelace.Suite/HelpService.cs:167) and the REPL writes that block with
+        // WriteLine, so the whole sentence must appear as its own line. Pinned in full because the
+        // pre-r13 sentence ("Solves an equation for x.") is no longer the product's text.
+        ReplHarness.AssertHasLine(t,
+            "Solves an equation for x and returns the SAME SolveResult record solve_full returns: "
+            + "status, domain (a Domain value), complete/completeness, per-solution "
+            + "conditions/multiplicity/exactness, parametric families and diagnostics. The default "
+            + "domain is Complex; pass real for real solutions only.");
         Assert.Contains("Examples:", t);
-        Assert.Contains("Returns: Vector | Text", t);
+        // solve() publishes the SAME SolveResult record solve_full does (SymbolicsPlugin.cs:451-455),
+        // so the declared return kind is SolveResult, not the pre-r13 "Vector | Text".
+        ReplHarness.AssertHasLine(t, "Returns: SolveResult");
         Assert.Contains("See also: solve_full, solve_system, linsolve", t);
         Assert.Contains("Plugin: Lovelace.Symbolics", t);
     }
```

`git status --porcelain` reports exactly one modified tracked file, and it is the in-scope file:

```
 M Lovelace.Console.Tests/ReplOutputTests.cs
```

(The `?? docs/...` entries in the same listing are the orchestrator's pre-existing untracked artifacts;
this round added only `docs/goal-cycle-6/round-1/implementation.md`.)

---

## 5. Assertions changed, untouched, and why the new pins are not weaker

**Changed — exactly 2 assertions, both inside `Help_Solve_ShowsSignatureAndSummary`:**

| Old (line 63 / 65) | New (line 67 / 75) |
|---|---|
| `Assert.Contains("Solves an equation for x.", t)` | `ReplHarness.AssertHasLine(t, <the full 291-char summary>)` |
| `Assert.Contains("Returns: Vector | Text", t)` | `ReplHarness.AssertHasLine(t, "Returns: SolveResult")` |

**Untouched:** the signature pin (line 62), `Assert.Contains("Examples:", t)` (line 72), the see-also pin
(line 76), the plugin pin (line 77), and every other test in the file — the 14 previously passing tests
still pass (§3). No assertion was deleted, skipped, loosened or given a wider tolerance.

**Why the new pins are not weaker — measured, not asserted.** Each new pin is a whole-line/strict
superstring form of the old one, and I falsified both against a product with the literals reverted, in a
scratch copy **outside the repository** (`robocopy` of this tree to
`C:\Users\ricar\AppData\Local\Temp\lovelace-falsify`, excluding `.git .worktrees out bin obj` and unrelated
large test dirs; 51.5 MB) with the **edited** `ReplOutputTests.cs` copied in and only the product literals
changed there:

* Revert the summary to the pre-r13 sentence in the scratch `SymbolicsPlugin.cs` →
  `dotnet test Lovelace.Console.Tests/... -c Release --nologo` ⇒ **exit 1**,
  `Failed: 1, Passed: 14, Total: 15`, stack trace
  `at Lovelace.Console.Tests.ReplOutputTests.Help_Solve_ShowsSignatureAndSummary() in ...\lovelace-falsify\Lovelace.Console.Tests\ReplOutputTests.cs:line 67`,
  message "expected line: Solves an equation for x and returns the SAME SolveResult record ... / in transcript: ... Solves an equation for x. The default domain is Complex. ...".
* Keep the new summary but revert the return kind to @"Vector | Text"@ ⇒ **exit 1**,
  `Failed: 1, Passed: 14, Total: 15`, stack trace `... ReplOutputTests.cs:line 75`,
  message "expected line: Returns: SolveResult / in transcript: ... Returns: Vector | Text ...".

So each new pin independently fails on the corresponding product revert — the sensitivity the old pins had
for the *old* text and lost when the product moved. Beyond parity they are strictly stronger:
`AssertHasLine` requires the whole trailing-trimmed line to match, whereas `Assert.Contains` accepted the
needle anywhere in the transcript; the summary needle grew from 26 characters to the product's complete
291-character sentence, so any further drift in the summary re-fails the test instead of silently passing;
and `Returns: SolveResult` is the *entire* returns line, which a reverted tree cannot satisfy as a fragment
(the word `SolveResult` still appears in the summary above, so a bare substring check would be the weaker
form the old assertion used).

---

## 6. Could not verify

* **Whole-CI green on GitHub's runners.** I ran `Lovelace.Console.Tests` (Windows, Release, net10.0) and the
  filtered `Lovelace.Suite.Tests` help class only. No Linux/WSL run, no full-solution run, no GitHub Actions
  execution: CI is green only insofar as the observed failing suite was the blocker; other suites were not
  exercised by me.
* **The product reverted in this working tree.** The §5 falsification reverted the literals in a scratch
  *copy* (editing the product is out of scope), so it is a byte copy plus my edited test file rather than
  this checkout; the renderer, registry and REPL path exercised there are identical, but that is a copy.
* **Live console parity.** As quoted in §2, the shipped console cannot be driven with piped stdin
  (`Console.ReadKey` throws), so transcript evidence comes from the same `ReplSession` constructed with
  `StringReader`/`StringWriter` — the seam the tests themselves use.
* **Whether other suites in the repo have unrelated failures**, and whether the scratch copy was excluded
  from any repo-level tooling (it lives under `%TEMP%`, outside the repository).
