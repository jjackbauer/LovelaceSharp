# Cycle-6 · round-19 (rebased) · print-parse — F5 and F6 re-applied on top of the source-position round

**Answer to the rebase request: the fix did NOT re-apply as a patch (textual conflict in the same three files) and it was recreated by hand on top of the current main working tree — one integration edit was required and is reported in §1b.** The whole solution is green on the result.

**Trees.** Rebase tree `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rb`, created with `git worktree add .worktrees/c6-pd-rb HEAD` and then seeded with the main working tree's uncommitted source-position work (the five files `git status` names: `Lovelace.Run/Runner.cs`, `Lovelace.Run/RunProtocol.cs`, `Lovelace.Run.Tests/fixtures/error_envelope.json`, `Lovelace.Run/ScriptPositions.cs`, `Lovelace.Run.Tests/SourcePositionTests.cs`), so the sibling's change is intact and this round lives on top of it. Control tree `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rbctl`, the same command, untouched until the test files were copied in.

**Provenance, corrected because the tree moved under this round.** When the rebase started, the sibling's position work was **uncommitted** in the main tree (main `HEAD` was `3792829`), which is why the five files were copied across rather than a branch being rebased. While this round ran, the sibling committed it: main is now `63774da` (the fix) + `8e21f21` (its report) and the main working tree is clean. The files this round copied are byte-identical to the committed ones — `git diff --no-index` between the main tree and the rebase tree shows exactly the §1a/§1b/§5 change and **zero** lines owned by the sibling (`ScriptPositions`, `PublishDiagnostic`, `Locate`, `EngineSource` appear in no hunk). A second worktree, `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rb2`, was then created at the committed `8e21f21`, the same six files were copied in, and its `git status` lists exactly those six — so the change applies cleanly to the committed HEAD, and §6b re-ran everything there.

**Nothing was committed, staged or pushed. The change is left UNCOMMITTED in the rebase worktree:**

~~~
 M Lovelace.Run.Tests/fixtures/error_envelope.json
 M Lovelace.Run/RunProtocol.cs
 M Lovelace.Run/Runner.cs
?? Lovelace.Run.Tests/ParseErrorClassificationTests.cs
?? Lovelace.Run.Tests/PrintOutputOnFailureTests.cs
?? Lovelace.Run.Tests/SourcePositionTests.cs
?? Lovelace.Run/ScriptPositions.cs
~~~

## 1a. The change, as a diff against the main working tree

These two diffs are exactly what the rebase adds on top of the sibling's tree (`git diff --no-index <main file> <rebase file>`):

~~~diff
@@ -144,8 +144,16 @@ public static class Runner
             }
             catch (Exception ex)
             {
-                // no engine ran and no statement executed, so the durations are zero and empty
-                return WriteError(stdout, stderr, json, "FileReadError", "ParseError", $"Cannot read script file '{file}': {ex.Message}", recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
+                // no engine ran and no statement executed, so the durations are zero and empty.
+                // A path the process could not read is a property of the caller's ARGUMENT, not a
+                // lexer/parser refusal — nothing was parsed at all, so the ParseError category sent an
+                // agent to fix syntax that does not exist (audit E, F6). The code stays specific
+                // (FileReadError) and the category follows the sibling caller-level paths, which
+                // already cross as TypeMismatch (PlotDirectoryError, PlotFileError).
+                return WriteError(stdout, stderr, json, "FileReadError", "TypeMismatch",
+                    $"Cannot read script file '{file}': {ex.Message}", recoverable: true,
+                    Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
+                    Array.Empty<string>());
             }
         }
         else if (stdinMode)
@@ -196,7 +204,8 @@ public static class Runner
                 // same shape the unreadable-script-file path publishes
                 return WriteError(stdout, stderr, json, "PlotDirectoryError", "TypeMismatch",
                     $"Cannot use plot directory '{engine.PlotOutputDirectory}': {ex.Message}",
-                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
+                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
+                    Array.Empty<string>());
             }
 
             var result = await engine.EvaluateAsync(
@@ -266,7 +275,14 @@ public static class Runner
             var diagnostics = engine.Diagnostics
                 .Select(d => PublishDiagnostic(script, d, engine.OperationTimings))
                 .ToArray();
-            var (code, category, recoverable) = Classify(ex);
+            // Which LAYER the failure belongs to (audit E, F6). The engine tokenizes and parses the
+            // WHOLE source before it executes the first statement, so when the text it was handed does
+            // not parse, the failure that ended the run IS the parser's refusal. The phase is not
+            // readable from the exception TYPE — the tokenizer and the parser refuse with the same
+            // InvalidOperationException a domain failure uses (Lovelace.Suite/Tokenizer.cs:123,
+            // Parser.cs:64-71) — so the phase is measured, on the failure path, by asking the engine
+            // to parse that same text again.
+            var (code, category, recoverable) = Classify(ex, parsePhase: !SourceParses(engine, script.EngineSource));
             // a run that blew its budget before failing must not look like one that respected it:
             // the verdict travels with the failure, and an overrun adds its own diagnostic
             var failedDeadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: code == "Cancelled");
@@ -274,9 +290,15 @@ public static class Runner
                 diagnostics = diagnostics
                     .Append(OverrunDiagnostic(failedDeadline, script, engine.OperationTimings, completed: false))
                     .ToArray();
+            // Everything the script printed before the run ended crosses in the documented "output"
+            // array: invariant 1 (docs/symbolics/dsh-protocol.md:9-10) is not scoped to a successful
+            // run, and the captured text is already in hand (audit E, F5). The cancelled envelope
+            // ADDITIONALLY keeps the partial* pair it has always published, so no recorded shape
+            // regresses.
+            string[] committedOutput = SplitLines(output.ToString());
             // a cancelled run is not a failed run: the host reports the structured status together
             // with everything the engine had already committed
-            string[]? partialOutput = code == "Cancelled" ? SplitLines(output.ToString()) : null;
+            string[]? partialOutput = code == "Cancelled" ? committedOutput : null;
             VariableDto[]? partialVariables = code == "Cancelled"
                 ? engine.CaptureState().Variables.Values
                     .OrderBy(v => v.Name, StringComparer.Ordinal)
@@ -287,7 +309,8 @@ public static class Runner
             // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
             // from one unit selector and one timing entry per statement that ran before the failure
             return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
-                engine.LastElapsed, Timings(engine.OperationTimings, script), partialOutput, partialVariables, failedDeadline);
+                engine.LastElapsed, Timings(engine.OperationTimings, script), committedOutput,
+                partialOutput, partialVariables, failedDeadline);
         }
     }
 
@@ -309,9 +332,39 @@ public static class Runner
     private static bool IsUnusablePlotDirectory(Exception ex) =>
         ex is ArgumentException or IOException or UnauthorizedAccessException or NotSupportedException;
 
+    /// <summary>
+    /// Whether the text the engine was handed parses. <see cref="Classify"/> reads the phase off this
+    /// probe, and the probe is one call of the SAME public parse entry the engine itself runs before
+    /// it executes anything (<see cref="SuiteEngine.Parse"/>): a text that does not parse ends the
+    /// evaluation before the first statement, so the failure that ended the run IS that refusal.
+    /// Nothing else is inferred from it: the probe's own exception is discarded, and the envelope's
+    /// message, diagnostics and durations all come from the exception the evaluation really threw.
+    /// </summary>
+    private static bool SourceParses(SuiteEngine engine, string source)
+    {
+        try
+        {
+            engine.Parse(source);
+            return true;
+        }
+        catch (Exception)
+        {
+            return false;
+        }
+    }
+
     /// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
-    /// by a consumer: the code and category are structural.</summary>
-    private static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
+    /// by a consumer: the code and category are structural.
+    /// <para>
+    /// <paramref name="parsePhase"/> says the failure came from the tokenizer/parser. Those refuse a
+    /// source with an <see cref="InvalidOperationException"/> — the SAME type a domain failure uses —
+    /// so the type alone cannot place the failure on a layer (audit E, F6;
+    /// docs/symbolics/dsh-protocol.md:207-209 spells the categories from the one <c>ErrorCategory</c>
+    /// taxonomy, in which a lexer/parser refusal is <c>ParseError</c>). The flag therefore changes the
+    /// meaning of that ONE type: every other arm names a condition the phase cannot change, and a
+    /// parse-phase DEPTH refusal keeps its own <c>DepthExceeded</c>/<c>BudgetExceeded</c> arm above.
+    /// </para></summary>
+    private static (string Code, string Category, bool Recoverable) Classify(Exception ex, bool parsePhase) => ex switch
     {
         Lovelace.Suite.EvaluationCancelledException => ("Cancelled", "BudgetExceeded", true),
         Lovelace.Suite.ReentrancyNotSupportedException => ("ReentrancyNotSupported", "UnsupportedOperation", true),
@@ -339,7 +392,12 @@ public static class Runner
         Lovelace.Symbolics.AssumptionContradictionException => ("UnsatisfiableAssumptions", "DomainError", true),
         FormatException => ("InvalidInput", "ParseError", true),
         NotSupportedException => ("UnsupportedOperation", "UnsupportedOperation", true),
-        InvalidOperationException => ("InvalidOperation", "DomainError", true),
+        // The parser's own refusal: a syntax error is a ParseError, not a domain problem an agent
+        // would try to fix by changing the mathematics. The tokenizer's string refusal reaches the
+        // FormatException arm above and already carried the ParseError CATEGORY.
+        InvalidOperationException => parsePhase
+            ? ("ParseError", "ParseError", true)
+            : ("InvalidOperation", "DomainError", true),
         ArgumentException => ("InvalidArgument", "TypeMismatch", true),
         // framework types that still escape the kernel are classified honestly rather than being
         // reported as internal invariant failures: they are user-level errors and recoverable
@@ -352,7 +410,7 @@ public static class Runner
     private static int WriteError(TextWriter stdout, TextWriter stderr,
         bool json, string code, string category, string message, bool recoverable,
         DiagnosticDto[] diagnostics, TimeSpan elapsed, TimingDto[] timings,
-        string[]? output = null, VariableDto[]? variables = null,
+        string[] output, string[]? partialOutput = null, VariableDto[]? partialVariables = null,
         CancellationDto? cancellation = null)
     {
         if (json)
@@ -362,7 +420,8 @@ public static class Runner
             // forms can never disagree — exactly like the success envelope
             WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
                 false, code, category, message, recoverable, diagnostics,
-                Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output, variables, cancellation),
+                Lovelace.Suite.Timing.Format(elapsed), Duration(elapsed), timings, output,
+                partialOutput, partialVariables, cancellation),
                 RunJsonContext.Default.RunErrorDto);
         }
         else
~~~

~~~diff
@@ -107,7 +107,17 @@ internal sealed record RunErrorDto(
     DurationDto ElapsedTime,
     // one entry per top-level statement that ran (empty when none did), exactly as on success
     TimingDto[] Timings,
-    // present only for a cancelled evaluation: everything the engine had already committed
+    // The SAME top-level output array the success envelope carries, holding everything the script
+    // printed before the run ended: all of it on success, the lines committed before the failure
+    // here — and never a line whose print did not run. Invariant 1
+    // (docs/symbolics/dsh-protocol.md:9-10) is not scoped to a successful run, so a consumer reads
+    // ONE key on both paths; a failure that printed nothing carries the array empty rather than
+    // absent, and a failure raised before any engine exists (an unreadable script file, an unusable
+    // plot directory) carries it empty too. Nothing on this path can print.
+    string[] Output,
+    // present only for a cancelled evaluation: everything the engine had already committed. The
+    // cancelled envelope keeps this pair byte-for-byte as published (cycle 3/4/5 recorded it); the
+    // lines are the same ones Output carries, so the two cannot disagree.
     string[]? PartialOutput = null,
     VariableDto[]? PartialVariables = null,
     // present only when --cancel-after was given: the deadline verdict (see CancellationDto). The
~~~

## 1b. The one integration edit the sibling's tests required (no weakening)

The sibling's `Lovelace.Run.Tests/SourcePositionTests.cs:247-264` (`DiagnosticEntry_KeepsItsFourFieldShape`) pins the ERROR envelope's key set **exactly**. F5 adds the documented top-level `output` key, so that pin had to move with the contract — it now lists `output` and stays exact, so any further key still fails:

~~~diff
@@ -254,11 +254,16 @@ public class SourcePositionTests
         Assert.Equal(
             new[] { "column", "line", "message", "position" },
             diagnostic.Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
+        // The error envelope's key set, pinned exactly. It gained "output" after this file was
+        // written: audit E's F5 closed print output being dropped on a failing run, so every failure
+        // envelope now carries the top-level output array the protocol documents
+        // (docs/symbolics/dsh-protocol.md:9-10, invariant 1) — empty here, because this script
+        // prints nothing. The list stays EXACT: any further key still fails this assertion.
         Assert.Equal(
             new[]
             {
                 "category", "code", "diagnostics", "elapsed", "elapsedTime", "mathIrVersion", "message",
-                "ok", "protocolVersion", "recoverable", "symbolicFormatVersion", "timings",
+                "ok", "output", "protocolVersion", "recoverable", "symbolicFormatVersion", "timings",
             },
             envelope.AsObject().Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
     }
~~~

Everything else in the sibling's file is untouched. Without this one line the rebased tree reports `Lovelace.Run.Tests Failed: 1, Passed: 285` (that assertion, and only that one) — the rest of the suite was already green.

## 2. Failing-first on a PRISTINE tree at the same commit (test files only)

`C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rbctl` at `3792829`, cleaned, then **only** the two test files copied in (no product file, no golden):

~~~powershell
Copy-Item <rebase>\Lovelace.Run.Tests\PrintOutputOnFailureTests.cs        <ctl>\Lovelace.Run.Tests\
Copy-Item <rebase>\Lovelace.Run.Tests\ParseErrorClassificationTests.cs    <ctl>\Lovelace.Run.Tests\
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~PrintOutputOnFailureTests|FullyQualifiedName~ParseErrorClassificationTests'
~~~

~~~
Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rbctl\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.
dotnet : [xUnit.net 00:00:00.90]     
Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray [FAIL]
At line:8 char:1
+ dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configurat ...
+ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([xUnit.net 00:0...putArray [FAIL]:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError [FAIL]
Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInAMultiLineScript_CrossesAsParseError [FAIL]
Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput [FAIL]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray [525 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AMissingScriptFile_IsNotAParseError [5 ms]
  Error Message:
   Assert.NotEqual() Failure: Strings are equal
Expected: Not "ParseError"
Actual:       "ParseError"
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError [2 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
           ↓ (pos 0)
Expected: "ParseError"
Actual:   "InvalidOperation"
           ↑ (pos 0)
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInAMultiLineScript_CrossesAsParseError [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
           ↓ (pos 0)
Expected: "ParseError"
Actual:   "InvalidOperation"
           ↑ (pos 0)
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsNothingThatNeverRan [49 ms]
  Error Message:
   System.NullReferenceException : Object reference not set to an instance of an object.
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput [130 ms]
  Error Message:
   System.NullReferenceException : Object reference not set to an instance of an object.
--- End of stack trace from previous location ---
Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray [FAIL]
[FAIL]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray [< 1 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsTheLinePrintedBeforeTheFailure [< 1 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
--- End of stack trace from previous location ---
Failed!  - Failed:     8, Passed:     4, Skipped:     0, Total:    12, Duration: 742 ms - Lovelace.Run.Tests.dll (net10.0)
~~~

## 3. After envelopes, on a Native AOT binary published from the rebased tree

~~~powershell
# in .worktrees\c6-pd-rb
dotnet publish Lovelace.Run\Lovelace.Run.csproj --configuration Release -p:PublishAot=true -p:InvariantGlobalization=true -o out\aot   # exit 0
~~~

~~~


FullName      : C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd-rb\out\aot\Lovelace.Run.exe
Length        : 5805056
LastWriteTime : 9/12/2026 5:39:44 PM



### F5 print-then-det
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":16,"line":1,"column":17}],"elapsed":"487.2 \u00B5s","elapsedTime":{"value":487.2,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":57.2,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":186.3,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":["hello"]}

STDERR=
---
### F5 print-never-runs
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":16,"line":1,"column":17}],"elapsed":"293.8 \u00B5s","elapsedTime":{"value":293.8,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":33,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":114,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":["hello"]}

STDERR=
---
### F5 print-then-parsefail
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":22,"line":2,"column":1}],"elapsed":"163.7 \u00B5s","elapsedTime":{"value":163.7,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 parse third line
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":3,"column":1}],"elapsed":"152.9 \u00B5s","elapsedTime":{"value":152.9,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 parse single line
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":3,"line":1,"column":4}],"elapsed":"151.2 \u00B5s","elapsedTime":{"value":151.2,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 missing file
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"FileReadError","category":"TypeMismatch","message":"Cannot read script file \u0027C:\\definitely\\missing\\x.ls\u0027: Could not find a part of the path \u0027C:\\definitely\\missing\\x.ls\u0027.","recoverable":true,"diagnostics":[],"elapsed":"0 ns","elapsedTime":{"value":0,"unit":"ns"},"timings":[],"output":[]}

STDERR=
---
### F6 unterminated string
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidInput","category":"ParseError","message":"Unterminated string literal at position 0.","recoverable":true,"diagnostics":[{"message":"Unterminated string literal at position 0.","position":0,"line":1,"column":1}],"elapsed":"167.6 \u00B5s","elapsedTime":{"value":167.6,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### CONTROL deep parens
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"DepthExceeded","category":"BudgetExceeded","message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","recoverable":true,"diagnostics":[{"message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","position":0,"line":1,"column":1}],"elapsed":"826 \u00B5s","elapsedTime":{"value":826,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### CONTROL det(1) domain
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"277.9 \u00B5s","elapsedTime":{"value":277.9,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":128,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":[]}

STDERR=
---
### CONTROL success print
EXIT=0
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"2","typed":"2 (Natural)","structured":{"kind":"Natural","value":"2","exact":true}},"output":["hello"],"variables":[{"name":"_","kind":"Natural","display":"2","structured":{"kind":"Natural","value":"2","exact":true}}],"functions":[],"elapsed":"105.6 \u00B5s","elapsedTime":{"value":105.6,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":33.4,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":7.2,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false}]}

STDERR=
---
### CONTROL cancelled print
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[{"message":"cancellation deadline exceeded: --cancel-after 100 ms, elapsed 106.728 ms (6.728 ms over budget); the deadline was observed, but only after the excess had already been spent.","position":33,"line":1,"column":34}],"elapsed":"106.73 ms","elapsedTime":{"value":106.73,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":64.4,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":16.5,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":33,"elapsed":{"value":106.44,"unit":"ms"},"resultKind":"Void","hasOutput":false}],"output":["hello"],"partialOutput":["hello"],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x","structured":{"kind":"Symbolic","pretty":"x","canonical":"(sym x)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":["x"]}}],"cancellation":{"budgetMs":100,"elapsedMs":106.728,"stopped":true,"exceeded":true,"excessMs":6.728}}

STDERR=
---
~~~

The deciding fields, verbatim: `print("hello"); det(1)` answers exit 1 with `"output":["hello"]` (the print crosses) and the sibling's position work alongside it (`"position":16,"line":1,"column":17`); the third-line `)` answers `"code":"ParseError","category":"ParseError"` with `"position":10,"line":3,"column":1`; `1 +` answers `ParseError/ParseError`; the unreadable file answers `FileReadError/TypeMismatch`; a print that never ran is in no envelope, and a parse failure's `output` is `[]`.

## 4. Before / after (before = the published binary of HEAD; after = the rebased AOT binary)

| probe | before | after |
|---|---|---|
| F5 · `print("hello"); det(1)` | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0,16] | exit 1 · `InvalidArgument/TypeMismatch` · `output` ["hello"] · timings [0,16] |
| F5 · `print("hello"); nosuchfn(1)` | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [0,16] | _(not probed)_ |
| F5 · `print("hello"); det(1); print("never")` | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0,16] | exit 1 · `InvalidArgument/TypeMismatch` · `output` ["hello"] · timings [0,16] |
| F5/F6 · `print("UNREACHED"); 1+1;` then `)` on line 3 | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · parse failure on the THIRD line | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · parse failure on a single line (`1 +`) | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · an unreadable script file | exit 1 · `FileReadError/ParseError` · `output` **ABSENT** · timings [] | exit 1 · `FileReadError/TypeMismatch` · `output` [] · timings [] |
| F6 · unterminated string literal | exit 1 · `InvalidInput/ParseError` · `output` **ABSENT** · timings [] | exit 1 · `InvalidInput/ParseError` · `output` [] · timings [] |
| control · `det(1)` alone — a domain failure | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0] | exit 1 · `InvalidArgument/TypeMismatch` · `output` [] · timings [0] |
| control · 3000 nested parens — a parse-phase DEPTH refusal | exit 1 · `DepthExceeded/BudgetExceeded` · `output` **ABSENT** · timings [] | exit 1 · `DepthExceeded/BudgetExceeded` · `output` [] · timings [] |
| control · the success path | exit 0 · `ok:true` · `output` ["hello"] · timings [0,16] | exit 0 · `ok:true` · `output` ["hello"] · timings [0,16] |
| control · the cancelled path | exit 1 · `Cancelled/BudgetExceeded` · `output` **ABSENT** · `partialOutput` ["hello"] · timings [0,17,33] | exit 1 · `Cancelled/BudgetExceeded` · `output` ["hello"] · `partialOutput` ["hello"] · timings [0,17,33] |

## 5. The golden, re-derived from the product AFTER the rebase

The same temporary regeneration harness as the first pass was re-run in the rebase tree (every corpus row through `Runner.RunAsync`, normalised, serialised in the corpus' own style, compared BYTES): **17 of 20 fixtures byte-identical again**, `transform_result.json` and `capabilities.json` differing only by an earlier round's different pretty-printer, and `error_envelope.json` differing by exactly this round's key.

**Against the sibling's file the only difference is the added key** (verified line by line, not by eye): line 19 gains its comma, line 20 is `"output": []`, line 21 is the closing brace — the sibling's `position: 17` / `column: 18` are untouched. So the golden difference that is left is the sibling's position/column change plus this round's `output`; nothing was reverted and nothing else moved.

~~~diff
@@ -17,5 +17,6 @@
   ],
   "elapsed": "<volatile>",
   "elapsedTime": "<volatile>",
-  "timings": "<volatile>"
+  "timings": "<volatile>",
+  "output": []
 }
~~~

## 6. Suite totals (rebased tree)

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` in `.worktrees\c6-pd-rb` (`LOVELACE_REQUIRE_SYMPY=1`, oracle on PATH) — **exit 0**
(`Lovelace.Run.Tests` is 286 = 262 at HEAD + 12 source-position cases + 12 of this round):

| project | passed | failed | skipped | total |
|---|---|---|---|---|
| `Lovelace.Abstractions.Tests` | 20 | 0 | 0 | 20 |
| `Lovelace.Representation.Tests` | 91 | 0 | 0 | 91 |
| `Lovelace.Knowledge.Tests` | 28 | 0 | 0 | 28 |
| `Lovelace.Array.Tests` | 19 | 0 | 0 | 19 |
| `Lovelace.Natural.Tests` | 195 | 0 | 0 | 195 |
| `Lovelace.Integer.Tests` | 148 | 0 | 0 | 148 |
| `precbench.Tests` | 13 | 0 | 0 | 13 |
| `Lovelace.Console.Tests` | 15 | 0 | 0 | 15 |
| `Lovelace.Complex.Tests` | 121 | 0 | 0 | 121 |
| `Lovelace.Dsp.Tests` | 61 | 0 | 0 | 61 |
| `Lovelace.Real.Tests` | 2495 | 0 | 0 | 2495 |
| `Lovelace.Suite.Tests` | 848 | 0 | 0 | 848 |
| `Lovelace.Run.Tests` | 286 | 0 | 0 | 286 |
| `Lovelace.Symbolics.Tests` | 1148 | 0 | 0 | 1148 |
| `Lovelace.Studio.Tests` | 22 | 0 | 0 | 22 |
| **15 projects** | **5510** | **0** | **0** | **5510** |

Timing subsets (`--filter 'Category=Timing'`), all exit 0:

~~~
Lovelace.Suite.Tests  ::  Passed!  - Failed:     0, Passed:     8, Skipped:     0, Total:     8, Duration: 4 s - Lovelace.Suite.Tests.dll (net10.0)
Lovelace.Run.Tests  ::  Passed!  - Failed:     0, Passed:     5, Skipped:     0, Total:     5, Duration: 429 ms - Lovelace.Run.Tests.dll (net10.0)
Lovelace.Real.Tests  ::  Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 1 s - Lovelace.Real.Tests.dll (net10.0)
Lovelace.Symbolics.Tests  ::  Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 392 ms - Lovelace.Symbolics.Tests.dll (net10.0)
~~~

`Lovelace.Symbolics.Tests --filter 'Category=Costly'`, exit 0:

~~~
Passed!  - Failed:     0, Passed:    32, Skipped:     0, Total:    32, Duration: 11 s - Lovelace.Symbolics.Tests.dll (net10.0)
~~~

## 6b. The same run on top of the COMMITTED HEAD (`.worktrees\c6-pd-rb2`, `8e21f21`)

The delta of the round against the committed HEAD, as git reports it (`git diff --numstat`; the two new test files are untracked and
therefore not in the list):

~~~
70	11	Lovelace.Run/Runner.cs
11	1	Lovelace.Run/RunProtocol.cs
2	1	Lovelace.Run.Tests/fixtures/error_envelope.json
6	1	Lovelace.Run.Tests/SourcePositionTests.cs
~~~

`dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~PrintOutputOnFailureTests|FullyQualifiedName~ParseErrorClassificationTests|FullyQualifiedName~GoldenEnvelopeTests'` → **45 passed / 0 failed**, exit 0.

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` → **exit 0**, 15 projects, **5510 passed / 0 failed / 0 skipped**:
Abstractions 20, Array 19, Complex 121, Console 15, Dsp 61, Integer 148, Knowledge 28, Natural 195, Representation 91, Studio 22,
precbench 13, Real 2495, Suite 848, Symbolics 1148, **Run 286**.

Timing subsets, all exit 0: Suite **8**, Run **5**, Real **1**, Symbolics **1**. `Lovelace.Symbolics.Tests --filter 'Category=Costly'`:
**32 / 0**, exit 0. (These totals equal §6's exactly: the two worktrees hold the same content.)

## 7. Could not verify

| item | why |
|---|---|
| The main working tree itself (the sibling's uncommitted state + my change) was never built or tested as one tree | the sibling holds that tree; this round tested the rebased copy in `.worktrees\c6-pd-rb`. The parent applies the diffs of §1a/§1b to the main tree |
| GitHub Actions / Linux | not run; every suite above ran locally on Windows |
| Whether any consumer outside this repository keys on `FileReadError`'s category | no consumer exists in the repo (`grep FileReadError` → `Runner.cs`) or in `C:\Users\ricar\deepseek-harness\packages`; the `code` is unchanged and only the category moves |
| `--text` on a failure (audit E F8, P2) and `--omit-variables` on the cancelled path (F4, P2) | unchanged, out of this round's scope; on `--text` the committed output is still invisible |
| `Lovelace.Suite/AstDepth.cs:129`'s unreachable "unhandled node type" guard | a plain `InvalidOperationException` raised in the parse phase now classifies `ParseError/ParseError` (was `InvalidOperation/DomainError`); unreachable on the current grammar, and its honest class would be `InternalInvariantFailure`. `Lovelace.Suite/**` is out of scope — reported, not edited |

## 8. How to take this change, hashes and status

**Merge instructions (the parent's step).** Each of the six files in `.worktrees\c6-pd-rb2` (the worktree on top of the committed HEAD `8e21f21`; `.worktrees\c6-pd-rb` holds the identical content) is the main tree's own file plus exactly the diff printed in §1a/§1b — the sibling's content is byte-preserved underneath — so the simplest route is to copy all six from the rebase worktree over the main tree:

```
Lovelace.Run/Runner.cs                               (sibling's file + §1a)
Lovelace.Run/RunProtocol.cs                          (sibling's file + §1a)
Lovelace.Run.Tests/fixtures/error_envelope.json      (sibling's file + §5)
Lovelace.Run.Tests/SourcePositionTests.cs            (sibling's file + §1b)
Lovelace.Run.Tests/PrintOutputOnFailureTests.cs      (new, this round)
Lovelace.Run.Tests/ParseErrorClassificationTests.cs  (new, this round)
```

`docs/goal-cycle-6/round-19/c6-pd.diff` in the main tree is the **pre-rebase** patch (against `3792829`) and does not
apply to the current working tree; this document supersedes it.

~~~
E60077CB528E80F7769E3A90A69C9400D49AD883A80F0994ACF64901255AA8D3  Lovelace.Run\Runner.cs
2AE9D772126973A373F943E847EAE0C4C4AE474DB8502C13FAEF23AF2AD66AAB  Lovelace.Run\RunProtocol.cs
EA7B9389D41F499695A3D45DCF9BD430075C27E87FC687E54F7201EE73CCDEDD  Lovelace.Run.Tests\fixtures\error_envelope.json
0114818DE35FE4DCA74857A8B35A7D568758C2D62ED34E506F4510104B91D971  Lovelace.Run.Tests\PrintOutputOnFailureTests.cs
4813D624B718F03188D7E687705CC76C709B381203FBF3A8A8AB9DDB0FFF5D3E  Lovelace.Run.Tests\ParseErrorClassificationTests.cs
2CFB0D18B7CE6E3A8D4CE3A2AA85BF8127FB1A88558A389480D646C416E0C6B8  Lovelace.Run.Tests\SourcePositionTests.cs
~~~

## Appendix — the two test files carried across unchanged

~~~csharp
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:9-10 — invariant 1: "stdout carries the envelope and nothing
/// else. Anything the script prints with print(...) is captured and returned in the top-level
/// output array." The sentence is not scoped to a successful run, and the runner's own header
/// states it the same way (Lovelace.Run/Runner.cs:21-23, Lovelace.Run/Program.cs:10-13).
///
/// Cycle-6 audit E filed the violation as F5 (P1): <c>print("hello"); det(1)</c> answers exit 1
/// with an envelope whose key set carries NO output array — the line the script printed is gone,
/// although the SAME envelope's <c>timings[0].hasOutput</c> is true, i.e. the capture happened.
/// The recorded-but-never-closed cycle-5 item is A2-wire F9.
///
/// The contract these tests pin: every envelope the runner emits carries the top-level
/// <c>output</c> array, holding exactly the lines the script printed before the run ended — all of
/// them on success, the ones committed before the failure on a failure, and never one that did not
/// run. The cancelled envelope additionally keeps its already-published <c>partialOutput</c> (the
/// same lines), so no recorded shape regresses.
/// </summary>
public class PrintOutputOnFailureTests
{
    private static async Task<(int ExitCode, string Raw, JsonNode Envelope)> RunAsync(
        string script, params string[] extraArguments)
    {
        var arguments = new List<string> { "--eval", script, "--omit-functions", "--omit-variables" };
        arguments.AddRange(extraArguments);

        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments.ToArray(), stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, TestSupport.ParseExactlyOneJsonDocument(raw, $"stdout of '{script}'"));
    }

    /// <summary>The top-level output array, read as the LINES the protocol says it carries.</summary>
    private static string[] Output(JsonNode envelope) =>
        envelope["output"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray();

    /// <summary>
    /// The finding itself: the line printed by the statement BEFORE the failing one must cross in
    /// the failure envelope's output array. Pre-fix the key is absent, so this fails on the NotNull
    /// assertion — the envelope is the whole stdout, so there is nowhere else for the line to be.
    /// </summary>
    [Fact]
    public async Task AFailingRun_KeepsTheLinePrintedBeforeTheFailure()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"hello\"); det(1)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"a failing run reports ok == false. stdout: {raw}");
        Assert.NotNull(envelope["output"]);
        Assert.Equal(new[] { "hello" }, Output(envelope));
        Assert.Contains("\"output\":[\"hello\"]", raw);

        // the capture the audit's deciding field points at: the printing statement DID write output
        Assert.True(envelope["timings"]![0]!["hasOutput"]!.GetValue<bool>(),
            $"the printing statement must report hasOutput. stdout: {raw}");
    }

    /// <summary>
    /// "Only what was printed BEFORE the failure": the print after the failing statement never runs,
    /// so its text must appear NOWHERE in the envelope. The marker is deliberately a word no message
    /// of this run can spell.
    /// </summary>
    [Fact]
    public async Task AFailingRun_KeepsNothingThatNeverRan()
    {
        var (exitCode, raw, envelope) = await RunAsync(
            "print(\"before\"); print(\"also before\"); det(1); print(\"UNREACHED\")");

        Assert.Equal(1, exitCode);
        Assert.Equal(new[] { "before", "also before" }, Output(envelope));
        Assert.DoesNotContain("UNREACHED", raw);
    }

    /// <summary>
    /// The key is the contract, not merely the data: a failure that printed nothing still carries
    /// the array (empty), exactly as the success envelope does — a consumer reads one key on both
    /// paths instead of testing for its existence.
    /// </summary>
    [Fact]
    public async Task AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray()
    {
        var (exitCode, raw, envelope) = await RunAsync("det(1)");

        Assert.Equal(1, exitCode);
        Assert.NotNull(envelope["output"]);
        Assert.Empty(envelope["output"]!.AsArray());
        Assert.False(envelope["timings"]![0]!["hasOutput"]!.GetValue<bool>(), raw);
    }

    /// <summary>
    /// A parse failure ends the run before the FIRST statement executes, so the print in the script
    /// produced nothing and the array must be empty — never the source text, never a stale line.
    /// </summary>
    [Fact]
    public async Task AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"UNREACHED\"); 1+1;\n)");

        Assert.Equal(1, exitCode);
        Assert.NotNull(envelope["output"]);
        Assert.Empty(envelope["output"]!.AsArray());
        Assert.DoesNotContain("UNREACHED", raw);
    }

    // ------------------------------------------------------------------
    // Controls: the paths that already carried their output must not move
    // ------------------------------------------------------------------

    /// <summary>The success path: the same key, the same order, the same array.</summary>
    [Fact]
    public async Task TheSuccessPath_IsUnchanged()
    {
        var (exitCode, raw, envelope) = await RunAsync("print(\"hello\"); 1+1");

        Assert.Equal(0, exitCode);
        Assert.True(envelope["ok"]!.GetValue<bool>(), raw);
        Assert.Equal(new[] { "hello" }, Output(envelope));
        Assert.Contains("\"output\":[\"hello\"]", raw);
    }

    /// <summary>
    /// The cancelled envelope keeps the recorded cycle-3/4/5 shape (partialOutput carrying the
    /// committed lines) AND satisfies invariant 1 in the documented key — the two are the same lines.
    /// </summary>
    [Fact]
    public async Task TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput()
    {
        var (exitCode, raw, envelope) = await RunAsync(
            "x = symbol(\"x\"); print(\"hello\"); sum(1..10000000)", "--cancel-after", "100");

        Assert.Equal(1, exitCode);
        Assert.Equal("Cancelled", envelope["code"]!.GetValue<string>());
        Assert.Equal("BudgetExceeded", envelope["category"]!.GetValue<string>());
        Assert.Equal(new[] { "hello" },
            envelope["partialOutput"]!.AsArray().Select(node => node!.GetValue<string>()).ToArray());
        Assert.Equal(new[] { "hello" }, Output(envelope));
    }
}
~~~

~~~csharp
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:207-209 is the error table: "Categories are spelled from the one
/// taxonomy, ErrorCategory: ParseError, DomainError, UnsupportedOperation, BudgetExceeded,
/// NoSolution, TypeMismatch, InternalInvariantFailure" — and :203 names "a parse error" as an error
/// "raised before any engine exists". A source the tokenizer or the parser refuses is therefore a
/// ParseError; a script file that could not be read is not a parse error at all.
///
/// Cycle-6 audit E filed the violation as F6 (P1), "ParseError is attached to the wrong layer in
/// both directions": <c>1 +</c> and <c>1+1;⏎2+2;⏎)</c> answered InvalidOperation/DomainError —
/// the tokenizer and the parser refuse with the same InvalidOperationException a domain failure
/// uses (Lovelace.Suite/Tokenizer.cs:123, Parser.cs:64-71), so the runner's type switch could not
/// tell the phases apart — while a missing script file answered FileReadError/ParseError although
/// nothing was parsed. The recorded-but-never-closed cycle-5 item is B3-surface F3.
///
/// The categories are asserted as WIRE STRINGS, exactly as the protocol document spells them.
/// </summary>
public class ParseErrorClassificationTests
{
    /// <summary>The taxonomy member a lexer/parser refusal belongs to (dsh-protocol.md:207-209).</summary>
    private const string ParseErrorCategory = "ParseError";

    private static async Task<(int ExitCode, string Raw, JsonNode Envelope)> RunArgumentsAsync(
        string[] arguments)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments, stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, TestSupport.ParseExactlyOneJsonDocument(raw, "runner stdout"));
    }

    private static Task<(int ExitCode, string Raw, JsonNode Envelope)> RunScriptAsync(string script) =>
        RunArgumentsAsync(new[] { "--eval", script, "--omit-functions", "--omit-variables" });

    /// <summary>The parsed envelope's category, as a string.</summary>
    private static string Category(JsonNode envelope) => envelope["category"]!.GetValue<string>();

    /// <summary>The parsed envelope's code, as a string.</summary>
    private static string Code(JsonNode envelope) => envelope["code"]!.GetValue<string>();

    // ------------------------------------------------------------------
    // The finding: a parse failure is a parse failure
    // ------------------------------------------------------------------

    /// <summary>
    /// The audit's multi-line shape: the third line is the offending <c>)</c>. Pre-fix this answers
    /// InvalidOperation/DomainError, i.e. the agent is told to fix a domain problem when it has a
    /// syntax error.
    /// </summary>
    [Fact]
    public async Task AParseFailureInAMultiLineScript_CrossesAsParseError()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("1+1;\n2+2;\n)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Code(envelope));
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a caller can fix the syntax and retry, so the refusal is recoverable");
        // the message still names what the parser refused
        Assert.Contains("Unexpected token ')'", envelope["message"]!.GetValue<string>());
        // nothing executed: the parse ends the run before the first statement
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>The audit's single-line shape: the refusal is the same layer and must say so.</summary>
    [Fact]
    public async Task AParseFailureInASingleLineScript_CrossesAsParseError()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("1 +");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Code(envelope));
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Contains("expected a number", envelope["message"]!.GetValue<string>());
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>
    /// The other limb of the same layer: the tokenizer's own string refusal already carried the
    /// ParseError CATEGORY (its code is the tokenizer's own InvalidInput, which this round does not
    /// move). Pinned so a change to the parser's classification cannot silently move this one.
    /// </summary>
    [Fact]
    public async Task TheTokenizerStringRefusal_CarriesTheSameParseErrorCategory()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("\"abc");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Contains("Unterminated string literal", envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// The other direction of the finding: NOTHING was parsed for a file the process could not
    /// read, so ParseError is the wrong category — it sends an agent to fix syntax that does not
    /// exist. The code stays FileReadError; the category follows the caller-argument precedent the
    /// sibling plot paths already use (PlotDirectoryError/TypeMismatch, PlotFileError/TypeMismatch).
    /// </summary>
    [Fact]
    public async Task AMissingScriptFile_IsNotAParseError()
    {
        string missing = Path.Combine(Path.GetTempPath(), "c6-pd-missing-" + Guid.NewGuid().ToString("N") + ".ls");
        Assert.False(File.Exists(missing), $"the probe path must not exist: {missing}");

        var (exitCode, raw, envelope) = await RunArgumentsAsync(
            new[] { "--file", missing, "--json", "--omit-functions", "--omit-variables" });

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal("FileReadError", Code(envelope));
        Assert.NotEqual(ParseErrorCategory, Category(envelope));
        Assert.Equal("TypeMismatch", Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>(), "a caller can name a readable path instead");
        // no engine ran: no diagnostics and no statement timing, but the envelope is still complete
        Assert.Empty(envelope["diagnostics"]!.AsArray());
        Assert.Empty(envelope["timings"]!.AsArray());
        Assert.Empty(envelope["output"]!.AsArray());
    }

    // ------------------------------------------------------------------
    // Controls: the failures that are NOT parse refusals keep their classification
    // ------------------------------------------------------------------

    /// <summary>
    /// A domain failure on a script that parses perfectly must not be relabelled: this is the
    /// control that stops the fix from turning every failure into a ParseError.
    /// </summary>
    [Fact]
    public async Task ADomainFailure_OnAParsingScript_KeepsItsDomainCategory()
    {
        var (unknownExit, unknownRaw, unknown) = await RunScriptAsync("nosuchfn(1)");
        Assert.Equal(1, unknownExit);
        Assert.Equal("InvalidOperation", Code(unknown));
        Assert.Equal("DomainError", Category(unknown));

        var (shapeExit, shapeRaw, shape) = await RunScriptAsync("det(1)");
        Assert.Equal(1, shapeExit);
        Assert.Equal("InvalidArgument", Code(shape));
        Assert.Equal("TypeMismatch", Category(shape));
    }

    /// <summary>
    /// A depth refusal happens DURING parsing but is not a syntax refusal: it is a budget stop and
    /// the depth guard's own taxonomy (DepthExceeded/BudgetExceeded) must survive.
    /// </summary>
    [Fact]
    public async Task AParsePhaseDepthRefusal_IsNotReclassifiedAsAParseError()
    {
        string deep = new string('(', 3000) + "1" + new string(')', 3000) + ";";
        var (exitCode, raw, envelope) = await RunScriptAsync(deep);

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal("DepthExceeded", Code(envelope));
        Assert.Equal("BudgetExceeded", Category(envelope));
        Assert.Contains("nesting depth", envelope["message"]!.GetValue<string>());
    }
}
~~~
