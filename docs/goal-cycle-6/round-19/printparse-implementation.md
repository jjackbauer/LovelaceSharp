# Cycle-6 · round-19 · print-parse — F5 (a failing run drops its print output) and F6 (a parse failure crosses as a domain error)

**Findings closed**: cycle-6 audit E, `docs/goal-cycle-6/round-18/audit-E-cli.md:252-297` (**F5**, P1) and `:299-343` (**F6**, P1) — the two cycle-5 P1s the amendment lists as still live at `docs/symbolics/a-plus-cycle-6-amendment.md:43` (row E-P1d).

**Tree used**: `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd` — created with
`git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-pd HEAD` (HEAD = `3792829`). The control tree is
`C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pdctl`, created the same way, and it received **only** the new/changed test files (requirement 5).
Nothing was committed, staged or pushed:

~~~
 M Lovelace.Run.Tests/fixtures/error_envelope.json
 M Lovelace.Run/RunProtocol.cs
 M Lovelace.Run/Runner.cs
?? Lovelace.Run.Tests/ParseErrorClassificationTests.cs
?? Lovelace.Run.Tests/PrintOutputOnFailureTests.cs
~~~

**Scope**: the product change is `Lovelace.Run/Runner.cs` + `Lovelace.Run/RunProtocol.cs` only; the tests are the two new files plus the
one golden fixture the contract moved. `Lovelace.Suite/**`, `Lovelace.Symbolics/**` and the source-position code in `Runner.cs`
(HEAD `:395-434`) were **read, never edited** — §2 records what the Suite's refusal sites are and why the fix could stay in the runner.

**Reproduction rule**: every before/after envelope below is a real process, stdout/stderr redirected, exit code read from the process.
The pre-fix binary is the published AOT binary the audit used (`out\aot\Lovelace.Run.exe`, 5 801 472 bytes, 2026-09-12 16:56:28);
the after-state is a **Native AOT binary published from the fixed tree** (§11) and the Release in-process build the tests drive.

## 1. Pre-fix behaviour, re-observed (requirement 1)

### (1) F5 — the line the script printed is gone from the failure envelope

The probes (scripts written byte-exact to `%TEMP%\c6pd\`; the hex dump at the head of the log is the input evidence):

~~~powershell
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\f5.ls  --json --omit-functions   # print("hello"); det(1)
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\f5b.ls --json --omit-functions   # print("hello"); nosuchfn(1)
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\f5c.ls --json --omit-functions   # print("hello"); det(1); print("never")
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\ok.ls  --json --omit-functions   # print("hello"); 1+1
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\cancel.ls --json --omit-functions --cancel-after 100
~~~

~~~
=== file bytes ===
det1.ls (6 bytes): 64 65 74 28 31 29
f6ml.ls (11 bytes): 31 2B 31 3B 0A 32 2B 32 3B 0A 29
f6single.ls (3 bytes): 31 20 2B
ok.ls (19 bytes): 70 72 69 6E 74 28 22 68 65 6C 6C 6F 22 29 3B 20 31 2B 31
unterm.ls (4 bytes): 22 61 62 63
### F5 print-then-det
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\f5.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"296.7 \u00B5s","elapsedTime":{"value":296.7,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":34.7,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":108.3,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}

STDERR=
---
### F5 print-then-unknownfn
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\f5b.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unknown function \u0027nosuchfn\u0027.","recoverable":true,"diagnostics":[{"message":"Unknown function \u0027nosuchfn\u0027.","position":0,"line":1,"column":1}],"elapsed":"209.7 \u00B5s","elapsedTime":{"value":209.7,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":32.7,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":31.9,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}

STDERR=
---
### F5 print-never-runs
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\f5c.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"290.2 \u00B5s","elapsedTime":{"value":290.2,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":35.9,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":104.4,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}

STDERR=
---
### F6 parse third line
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\f6ml.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"157.5 \u00B5s","elapsedTime":{"value":157.5,"unit":"\u00B5s"},"timings":[]}

STDERR=
---
### F6 parse single line
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\f6single.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":3,"line":1,"column":4}],"elapsed":"171.4 \u00B5s","elapsedTime":{"value":171.4,"unit":"\u00B5s"},"timings":[]}

STDERR=
---
### F6 missing file
CMD: --file C:\definitely\missing\x.ls --json --omit-functions
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"FileReadError","category":"ParseError","message":"Cannot read script file \u0027C:\\definitely\\missing\\x.ls\u0027: Could not find a part of the path \u0027C:\\definitely\\missing\\x.ls\u0027.","recoverable":true,"diagnostics":[],"elapsed":"0 ns","elapsedTime":{"value":0,"unit":"ns"},"timings":[]}

STDERR=
---
### CONTROL success print
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\ok.ls --json --omit-functions
EXIT=0
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"2","typed":"2 (Natural)","structured":{"kind":"Natural","value":"2","exact":true}},"output":["hello"],"variables":[{"name":"_","kind":"Natural","display":"2","structured":{"kind":"Natural","value":"2","exact":true}}],"functions":[],"elapsed":"103.8 \u00B5s","elapsedTime":{"value":103.8,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":35.7,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":8.8,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false}]}

STDERR=
---
### CONTROL cancelled print
CMD: --file C:\Users\ricar\AppData\Local\Temp\c6pd\cancel.ls --json --omit-functions --cancel-after 100
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[{"message":"cancellation deadline exceeded: --cancel-after 100 ms, elapsed 108.723 ms (8.723 ms over budget); the deadline was observed, but only after the excess had already been spent.","position":33,"line":1,"column":34}],"elapsed":"108.72 ms","elapsedTime":{"value":108.72,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":61.2,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":16,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":33,"elapsed":{"value":108.43,"unit":"ms"},"resultKind":"Void","hasOutput":false}],"partialOutput":["hello"],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x","structured":{"kind":"Symbolic","pretty":"x","canonical":"(sym x)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":["x"]}}],"cancellation":{"budgetMs":100,"elapsedMs":108.723,"stopped":true,"exceeded":true,"excessMs":8.723}}

STDERR=
---
~~~

**Observed (the deciding fields, verbatim from the log above).** `print("hello"); det(1)` answers **exit 1** with
`"code":"InvalidArgument","category":"TypeMismatch"` and the key set
`protocolVersion, symbolicFormatVersion, mathIrVersion, ok, code, category, message, recoverable, diagnostics, elapsed, elapsedTime, timings` —
**no `output`, no `partialOutput`** — while the SAME envelope's `timings[0].hasOutput` is `true`, i.e. the capture demonstrably happened and the
host dropped it. The printed text is nowhere on stdout (the envelope is the whole stream) and stderr is 0 bytes.

The same shape with `nosuchfn(1)` (`InvalidOperation/DomainError`) and with `print("never")` appended: a print AFTER the failing statement
never runs, and — pre-fix — the one before it did not survive either.

**Controls observed in the same run**: the success path carries `"output":["hello"]` (exit 0) and the cancelled path carries
`"partialOutput":["hello"]` (exit 1, `Cancelled/BudgetExceeded`) — i.e. the output exists on every path EXCEPT the ordinary failure.

### (2) F6 — a parse failure on the third line crosses as a domain problem, and an unreadable file as a parse problem

~~~powershell
& out\aot\Lovelace.Run.exe --eval '1 +' --json --omit-functions                      # single-line syntax error
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\f6ml.ls --json --omit-functions        # 31 2B 31 3B 0A 32 2B 32 3B 0A 29  -> ')' on line 3
& out\aot\Lovelace.Run.exe --file C:\definitely\missing\x.ls --json --omit-functions # nothing was parsed at all
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\unterm.ls --json --omit-functions      # "abc  (unterminated string)
& out\aot\Lovelace.Run.exe --file %TEMP%\c6pd\deep.ls --json --omit-functions        # 3000 nested parens (depth refusal DURING parsing)
~~~

~~~
### unterminated string
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidInput","category":"ParseError","message":"Unterminated string literal at position 0.","recoverable":true,"diagnostics":[{"message":"Unterminated string literal at position 0.","position":0,"line":1,"column":1}],"elapsed":"206.1 \u00B5s","elapsedTime":{"value":206.1,"unit":"\u00B5s"},"timings":[]}

STDERR=
---
### deep parens 3000
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"DepthExceeded","category":"BudgetExceeded","message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","recoverable":true,"diagnostics":[{"message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","position":0,"line":1,"column":1}],"elapsed":"816.4 \u00B5s","elapsedTime":{"value":816.4,"unit":"\u00B5s"},"timings":[]}

STDERR=
---
### det(1) domain
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"273.7 \u00B5s","elapsedTime":{"value":273.7,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":123.5,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}

STDERR=
---
### print-then-parsefail
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":22,"line":1,"column":23}],"elapsed":"162.5 \u00B5s","elapsedTime":{"value":162.5,"unit":"\u00B5s"},"timings":[]}

STDERR=
---
~~~

**Observed.** The `)` that is the third line's only character crosses as `"code":"InvalidOperation","category":"DomainError"` with
`timings: []` (nothing executed), and `1 +` crosses the same way — so an agent branching on `category` is told to fix its mathematics when it
has a syntax error. In the other direction the missing file crosses as `"code":"FileReadError","category":"ParseError"` although nothing was
parsed. The unterminated string literal already carried `ParseError` (code `InvalidInput`, a `FormatException`), and the 3000-paren input is a
DEPTH refusal (`DepthExceeded/BudgetExceeded`) raised during the same parse phase — which is why the fix has to tell a syntax refusal from a
budget refusal, not merely "did parsing throw?".

## 2. Which layer a parse failure belongs to — the answer, and where it is written (F6)

The question the brief asks is which of three candidate sources says where a parse failure belongs. Reading all three:

| candidate | what it actually contains | does it carry the taxonomy? |
|---|---|---|
| `docs/symbolics/dsh-protocol.md` — the error section | `:207-209`: **"Categories are spelled from the one taxonomy, `ErrorCategory`: `ParseError`, `DomainError`, `UnsupportedOperation`, `BudgetExceeded`, `NoSolution`, `TypeMismatch`, `InternalInvariantFailure`"**; `:203` names *"a parse error, an unreadable script file"* as errors *"raised before any engine exists"*; `:225-228`: the error envelope's `diagnostics` is *"the parser's source-position form"* | **YES — this is the document that names the layer** |
| `Lovelace.Run/RunProtocol.cs` | the envelope DTOs; the error record carries `string Code` and `string Category` (`RunProtocol.cs:84-106`) with **no** `ErrorCategory` enum and no rule about parsing | **NO — no taxonomy to find** |
| `Lovelace.Run/Program.cs` (the runner's own header) | the protocol contract comment: *"Anything the script prints with print() is captured and returned in the envelope's output array"* (`Program.cs:10-13`) and *"Exit codes: 0 success, 1 script/diagnostic error, 2 usage error"* (`:18`); the same words head `Runner.cs:21-23` | **NO — exit codes only; it repeats invariant 1, which is what F5 violated** |

**So the parse-failure layer is written in the protocol document's error table (`dsh-protocol.md:203`, `:207-209`), and the layer that must APPLY it
is the runner**, because the envelope's `code`/`category` are assigned there: at HEAD `Runner.Classify` (`Lovelace.Run/Runner.cs:308-344`) mapped
`InvalidOperationException` to `("InvalidOperation", "DomainError")` at `:336` while the parser refuses with exactly that type.

### Is it a Suite misclassification (report-only) or a runner one (fixable in scope)?

The refusal SITES are in the Suite and they use a bare `InvalidOperationException`:

* `Lovelace.Suite/Tokenizer.cs:123-124` — `Unexpected character '<c>' at position N.`
* `Lovelace.Suite/Tokenizer.cs:155` — `Unterminated string literal at position N.` (**`FormatException`**, the limb that already carried `ParseError`)
* `Lovelace.Suite/Parser.cs:64, :70, :103, :135, :189, :204, :240, :284, :610, :648` — `Unexpected end of input` / `Unexpected token '<t>' at position N: …` / `Unterminated interpolation expression in string.`
* the depth guard inside the same phase: `Parser.cs:44`, `AstDepth.cs:38` -> `InputDepthExceededException` (`Lovelace.Abstractions/InputDepth.cs:106`)

and the Suite declares **no** parse exception type (`grep 'class \w*Exception' Lovelace.Suite` returns only `ArrayAllocationRefusedException`,
`ValueShapeException`, `ReentrancyNotSupportedException`, `PlotFileWriteException`, `EvaluationCancelledException`, `BuiltinArityException`,
`BuiltinShapeException`). The cycle-5 diagnosis proposed adding a typed `ParseException` at those sites
(`docs/goal-cycle-5/diagnosis/D-D-robustness.md:504-507`) — **that file is out of scope this round, so I did not touch it.**

**Verdict: the misclassification as filed is a runner one and it is closed in the runner.** The runner can measure the phase itself: the engine
tokenizes and parses the WHOLE source before it executes the first statement (`Lovelace.Suite/SuiteEngine.cs:284-301`) and exposes that same parse
as public API (`SuiteEngine.cs:191-196`, `Parse`). The fix therefore calls `engine.Parse(rewrittenSource)` on the FAILURE path and classifies the
one type whose meaning the phase changes; nothing in `Lovelace.Suite/**` or `Lovelace.Symbolics/**` was edited, and no message text is parsed.

Two consequences, stated so they are not discovered later as surprises:

* `FileReadError`'s category moves `ParseError` -> `TypeMismatch`. The code stays specific; the category follows the runner's own precedent for a
  caller-supplied path that cannot be used — `PlotDirectoryError/TypeMismatch` and `PlotFileError/TypeMismatch` (`Runner.cs:288-304`, `:328-331`).
* the tokenizer's string limb keeps its own code `InvalidInput` (it already carried category `ParseError`); unifying the CODES of parse failures is a
  separate, cosmetic change this round does not make, and the test pins the category, which is the taxonomy member the document names.

## 3. Failing-first (requirement 2)

Both test files were written and run against the UNFIXED product before `Runner.cs` was edited. In the scratch tree the filtered run reported:

~~~
---- scratch tree .worktrees\c6-pd, product at HEAD, BEFORE the fix ----
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~PrintOutputOnFailureTests|FullyQualifiedName~ParseErrorClassificationTests'

  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray
   Assert.NotNull() Failure: Value is null                      (PrintOutputOnFailureTests.cs:89)
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AMissingScriptFile_IsNotAParseError
   Assert.NotEqual() Failure: Expected: Not "ParseError"  Actual: "ParseError"   (ParseErrorClassificationTests.cs:121)
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError
   Assert.Equal() Failure: Expected: "ParseError"  Actual: "InvalidOperation"        (ParseErrorClassificationTests.cs:79)
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInAMultiLineScript_CrossesAsParseError
   Assert.Equal() Failure: Expected: "ParseError"  Actual: "InvalidOperation"        (ParseErrorClassificationTests.cs:61)
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsNothingThatNeverRan
   System.NullReferenceException (the envelope has no output array)                (PrintOutputOnFailureTests.cs:39)
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput
   System.NullReferenceException (the cancelled envelope has no output array)       (PrintOutputOnFailureTests.cs:39)
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray
   Assert.NotNull() Failure: Value is null
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsTheLinePrintedBeforeTheFailure
   Assert.NotNull() Failure: Value is null

Failed!  - Failed:     8, Passed:     4, Skipped:     0, Total:    12, Duration: 862 ms - Lovelace.Run.Tests.dll (net10.0)
~~~

(This is the in-session console transcript of that run; the identical 12 cases against the identical HEAD product are reproduced **file-backed**
in §9, where the same 8 fail with the same assertion lines and the same 8 test names.)

The four that passed pre-fix are deliberately the controls: the success path, the tokenizer's string limb (already `ParseError`), the domain failure
`nosuchfn(1)` (must stay `DomainError`) and the 3000-paren depth refusal (must stay `DepthExceeded/BudgetExceeded`).

## 4. The fix (what changed and why)

**F5 — the failure envelope carries the documented `output` array.** `Runner.RunAsync`'s catch block now computes
`string[] committedOutput = SplitLines(output.ToString())` — the capture the run already made — and passes it to `WriteError`, which fills a new
required member `Output` on `RunErrorDto` (`partialOutput`/`partialVariables` are unchanged). Consequences:

* every failure envelope carries `output` (the lines printed before the failure; the array is empty when nothing printed, and empty on the two
  paths where no engine exists — unreadable script file, unusable plot directory);
* a line whose `print` never ran cannot appear, because the capture is read from the writer the engine actually wrote to;
* the cancelled envelope keeps `partialOutput`/`partialVariables` **byte-for-byte as published** (cycle 3/4/5 recorded that shape) and now also
  carries `output` with the same lines — a deliberate, documented redundancy rather than removing a published key.

**F6 — the failure's LAYER is measured, not guessed.** `Classify(Exception, bool parsePhase)` takes the phase; the catch block passes
`parsePhase: !SourceParses(engine, rewrittenSource)`, where `SourceParses` is one call of the engine's public parse entry on the same text the
evaluation got. Only the plain `InvalidOperationException` arm changes meaning (parse phase -> `("ParseError", "ParseError", true)`); every other arm
names a condition the phase cannot change, and the depth guard's own arm (`InputDepthExceededException` -> `DepthExceeded/BudgetExceeded`) sits above
it, so a parse-phase budget refusal is untouched. The probe runs **only on the failure path** (the success path's cost is unchanged), and the
probe's own exception is discarded — the envelope's message, diagnostics and durations still come from the exception the evaluation threw.
`FileReadError` moves to category `TypeMismatch` for the reason given in §2.

## 5. The diff (product)

~~~diff
diff --git a/Lovelace.Run/RunProtocol.cs b/Lovelace.Run/RunProtocol.cs
index 1918c53..238b0d6 100644
--- a/Lovelace.Run/RunProtocol.cs
+++ b/Lovelace.Run/RunProtocol.cs
@@ -97,7 +97,17 @@ internal sealed record RunErrorDto(
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
diff --git a/Lovelace.Run/Runner.cs b/Lovelace.Run/Runner.cs
index 94de88c..7cade9e 100644
--- a/Lovelace.Run/Runner.cs
+++ b/Lovelace.Run/Runner.cs
@@ -145,7 +145,15 @@ public static class Runner
             catch (Exception ex)
             {
                 // no engine ran and no statement executed, so the durations are zero and empty
-                return WriteError(stdout, stderr, json, "FileReadError", "ParseError", $"Cannot read script file '{file}': {ex.Message}", recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
+                // A path the process could not read is a property of the caller's ARGUMENT, not a
+                // lexer/parser refusal — nothing was parsed at all, so the ParseError category sent an
+                // agent to fix syntax that does not exist (F6). The code stays specific
+                // (FileReadError) and the category follows the sibling caller-level paths, which
+                // already cross as TypeMismatch (PlotDirectoryError, PlotFileError).
+                return WriteError(stdout, stderr, json, "FileReadError", "TypeMismatch",
+                    $"Cannot read script file '{file}': {ex.Message}", recoverable: true,
+                    Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
+                    Array.Empty<string>());
             }
         }
         else if (stdinMode)
@@ -165,6 +173,10 @@ public static class Runner
         if (plotDir is not null) engine.PlotOutputDirectory = plotDir;
         if (plotFile is not null) engine.PlotFileName = plotFile;
 
+        // The text the engine is handed, computed ONCE: the failure path asks whether THIS text
+        // parses, to tell a parse refusal from a domain failure (see SourceParses and Classify).
+        string rewrittenSource = ScriptSource.ToSemicolonStatements(source);
+
         // capture print() output: the envelope must be the only thing on stdout. Declared outside the
         // try so a cancelled evaluation can still return the output it produced (the partial result).
         var output = new StringWriter();
@@ -190,11 +202,11 @@ public static class Runner
                 // same shape the unreadable-script-file path publishes
                 return WriteError(stdout, stderr, json, "PlotDirectoryError", "TypeMismatch",
                     $"Cannot use plot directory '{engine.PlotOutputDirectory}': {ex.Message}",
-                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
+                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>(),
+                    Array.Empty<string>());
             }
 
-            var result = await engine.EvaluateAsync(
-                ScriptSource.ToSemicolonStatements(source), output, cancellation.Token);
+            var result = await engine.EvaluateAsync(rewrittenSource, output, cancellation.Token);
 
             var snapshot = engine.CaptureState();
             // one budget for every structured rendering in the envelope: the result AND each
@@ -260,7 +272,14 @@ public static class Runner
             var diagnostics = engine.Diagnostics
                 .Select(d => new DiagnosticDto(d.Message, d.Position, d.Line, d.Column))
                 .ToArray();
-            var (code, category, recoverable) = Classify(ex);
+            // Which LAYER the failure belongs to (F6). The engine tokenizes and parses the WHOLE
+            // source before it executes the first statement, so when the text it was handed does not
+            // parse, the failure that ended the run IS the parser's refusal. The phase is not
+            // readable from the exception TYPE — the tokenizer and the parser refuse with the same
+            // InvalidOperationException a domain failure uses (Lovelace.Suite/Tokenizer.cs:123,
+            // Parser.cs:64-71) — so the phase is measured, on the failure path, by asking the engine
+            // to parse that same text again.
+            var (code, category, recoverable) = Classify(ex, parsePhase: !SourceParses(engine, rewrittenSource));
             // a run that blew its budget before failing must not look like one that respected it:
             // the verdict travels with the failure, and an overrun adds its own diagnostic
             var failedDeadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: code == "Cancelled");
@@ -268,9 +287,14 @@ public static class Runner
                 diagnostics = diagnostics
                     .Append(OverrunDiagnostic(failedDeadline, source, engine.OperationTimings, completed: false))
                     .ToArray();
+            // Everything the script printed before the run ended crosses in the documented "output"
+            // array: invariant 1 (docs/symbolics/dsh-protocol.md:9-10) is not scoped to a successful
+            // run, and the captured text is already in hand (F5). The cancelled envelope ADDITIONALLY
+            // keeps the partial* pair it has always published, so no recorded shape regresses.
+            string[] committedOutput = SplitLines(output.ToString());
             // a cancelled run is not a failed run: the host reports the structured status together
             // with everything the engine had already committed
-            string[]? partialOutput = code == "Cancelled" ? SplitLines(output.ToString()) : null;
+            string[]? partialOutput = code == "Cancelled" ? committedOutput : null;
             VariableDto[]? partialVariables = code == "Cancelled"
                 ? engine.CaptureState().Variables.Values
                     .OrderBy(v => v.Name, StringComparer.Ordinal)
@@ -281,7 +305,8 @@ public static class Runner
             // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
             // from one unit selector and one timing entry per statement that ran before the failure
             return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
-                engine.LastElapsed, Timings(engine.OperationTimings), partialOutput, partialVariables, failedDeadline);
+                engine.LastElapsed, Timings(engine.OperationTimings), committedOutput,
+                partialOutput, partialVariables, failedDeadline);
         }
     }
 
@@ -303,9 +328,39 @@ public static class Runner
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
+    /// so the type alone cannot place the failure on a layer (F6; docs/symbolics/dsh-protocol.md:207-209
+    /// spells the categories from the one <c>ErrorCategory</c> taxonomy, in which a lexer/parser
+    /// refusal is <c>ParseError</c>). The flag therefore changes the meaning of that ONE type: every
+    /// other arm names a condition the phase cannot change, and a parse-phase DEPTH refusal keeps its
+    /// own <c>DepthExceeded</c>/<c>BudgetExceeded</c> arm above.
+    /// </para></summary>
+    private static (string Code, string Category, bool Recoverable) Classify(Exception ex, bool parsePhase) => ex switch
     {
         Lovelace.Suite.EvaluationCancelledException => ("Cancelled", "BudgetExceeded", true),
         Lovelace.Suite.ReentrancyNotSupportedException => ("ReentrancyNotSupported", "UnsupportedOperation", true),
@@ -333,7 +388,12 @@ public static class Runner
         Lovelace.Symbolics.AssumptionContradictionException => ("UnsatisfiableAssumptions", "DomainError", true),
         FormatException => ("InvalidInput", "ParseError", true),
         NotSupportedException => ("UnsupportedOperation", "UnsupportedOperation", true),
-        InvalidOperationException => ("InvalidOperation", "DomainError", true),
+        // The parser's own refusal: a syntax error is a ParseError, not a domain problem an agent
+        // would try to fix by changing the mathematics. The tokenizer's string refusal reaches the
+        // FormatException arm above and already carried the ParseError CATEGORY (F6).
+        InvalidOperationException => parsePhase
+            ? ("ParseError", "ParseError", true)
+            : ("InvalidOperation", "DomainError", true),
         ArgumentException => ("InvalidArgument", "TypeMismatch", true),
         // framework types that still escape the kernel are classified honestly rather than being
         // reported as internal invariant failures: they are user-level errors and recoverable
@@ -346,7 +406,7 @@ public static class Runner
     private static int WriteError(TextWriter stdout, TextWriter stderr,
         bool json, string code, string category, string message, bool recoverable,
         DiagnosticDto[] diagnostics, TimeSpan elapsed, TimingDto[] timings,
-        string[]? output = null, VariableDto[]? variables = null,
+        string[] output, string[]? partialOutput = null, VariableDto[]? partialVariables = null,
         CancellationDto? cancellation = null)
     {
         if (json)
@@ -356,7 +416,8 @@ public static class Runner
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

## 6. The new test files

`Lovelace.Run.Tests/PrintOutputOnFailureTests.cs` — 5 cases: the objective's repro, "only what was printed before the failure" (a print after the
failing statement must appear NOWHERE in the stream), a print-free failure (empty array, not an absent key), a print before a PARSE failure (nothing
ran, so the array is empty and the marker text is absent), plus the success and cancelled controls. It is a NEW file, so its diff is its content:

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

`Lovelace.Run.Tests/ParseErrorClassificationTests.cs` — 6 cases: the multi-line and single-line parse failures assert the documented code AND
category (`ParseError`/`ParseError`), the tokenizer's string limb asserts the same CATEGORY, the unreadable file asserts it is NOT a parse error
(`FileReadError`/`TypeMismatch`), and two controls assert no over-classification (a domain failure stays `InvalidOperation/DomainError`,
`det(1)` stays `InvalidArgument/TypeMismatch`; a parse-phase depth refusal stays `DepthExceeded/BudgetExceeded`). New file, content as diff:

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

## 7. Before / after, per probe (requirements 1 and 3)

Both columns are the same probes on the same inputs. The pre-fix column is the published binary of HEAD; the after column is the Native AOT binary
published from the fixed tree (§11) — the probes were also run on the fixed tree's Release build with identical deciding fields (`postfix-probe3.log`).

| probe | pre-fix (published binary of HEAD) | after (AOT published from the fixed tree) |
|---|---|---|
| F5 · `print("hello"); det(1)` — the objective's repro | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0,16] | exit 1 · `InvalidArgument/TypeMismatch` · `output` ["hello"] · timings [0,16] |
| F5 · `print("hello"); nosuchfn(1)` — the audit's repro | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [0,16] | _(not probed)_ |
| F5 · `print("hello"); det(1); print("never")` | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0,16] | exit 1 · `InvalidArgument/TypeMismatch` · `output` ["hello"] · timings [0,16] |
| F5/F6 · `print("UNREACHED"); 1+1;` then `)` on line 3 | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · parse failure on the script's THIRD line | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · parse failure on a single line (`1 +`) | exit 1 · `InvalidOperation/DomainError` · `output` **ABSENT** · timings [] | exit 1 · `ParseError/ParseError` · `output` [] · timings [] |
| F6 · an unreadable script file | exit 1 · `FileReadError/ParseError` · `output` **ABSENT** · timings [] | exit 1 · `FileReadError/TypeMismatch` · `output` [] · timings [] |
| F6 · unterminated string literal (already `ParseError`) | exit 1 · `InvalidInput/ParseError` · `output` **ABSENT** · timings [] | exit 1 · `InvalidInput/ParseError` · `output` [] · timings [] |
| control · `det(1)` alone — a domain failure on valid syntax | exit 1 · `InvalidArgument/TypeMismatch` · `output` **ABSENT** · timings [0] | exit 1 · `InvalidArgument/TypeMismatch` · `output` [] · timings [0] |
| control · 3000 nested parens — a DEPTH refusal raised during parsing | exit 1 · `DepthExceeded/BudgetExceeded` · `output` **ABSENT** · timings [] | exit 1 · `DepthExceeded/BudgetExceeded` · `output` [] · timings [] |
| control · `print("hello"); 1+1` — the success path | exit 0 · `ok:true` · `output` ["hello"] · timings [0,16] | exit 0 · `ok:true` · `output` ["hello"] · timings [0,16] |
| control · the cancelled path (`--cancel-after 100`) | exit 1 · `Cancelled/BudgetExceeded` · `output` **ABSENT** · `partialOutput` ["hello"] · timings [0,17,33] | exit 1 · `Cancelled/BudgetExceeded` · `output` ["hello"] · `partialOutput` ["hello"] · timings [0,17,33] |

**No regression on the success path**: `print("hello"); 1+1` still answers exit 0 with `output:["hello"]` and the same key order, stdout is still
exactly one JSON document (the envelope is the whole stream; stderr 0 bytes on every probe above), and every exit code above is unchanged
(0 success, 1 failure). The cancellation ledger, `partialVariables`, the depth refusal, the domain failure and the tokenizer's `InvalidInput`
code are all untouched.

## 8. The golden fixture that moved: `Lovelace.Run.Tests/fixtures/error_envelope.json`

**Why it moved**: the fixture is the pin for the ERROR envelope, and the contract changed — every failure envelope now carries the top-level
`output` array (§4). Its script is `x = symbol("x"); solve(2*x == 1, x, integer)` and it prints nothing, so the array is `[]`.

**How it was regenerated FROM THE PRODUCT** (not hand-edited): a temporary xUnit harness was added to my worktree, which ran every corpus row
through `Runner.RunAsync` (the product), normalised the volatile keys exactly as `TestSupport.Normalise` does, serialised with the corpus' own
style (two-space indent, relaxed escaping, the host newline, one trailing newline) into `%TEMP%\c6pd-goldens3\`, and compared BYTES with the
committed fixture. Result: **17 of the 20 fixtures were reproduced byte-for-byte**, including `symbolic`, `print_purity` and `record`; the
exceptions are `transform_result.json` and `capabilities.json` — written by an earlier round with a different pretty-printer (four-space indent,
double space after the colon), which the golden comparison is deliberately insensitive to (`TestSupport.AssertSameJson` compares structure) — and
`error_envelope.json`, which gained exactly the key the contract added. The regenerated file was copied over the fixture and the temporary harness
was then deleted (`git status` in §13 shows only the intended files). The regenerated fixture's declared failure envelope is the one printed in §1's
F6/AOT logs, so the fixture and the product cannot disagree.

**The diff** (the golden's own, trimmed to the moved lines):

~~~diff
diff --git a/Lovelace.Run.Tests/fixtures/error_envelope.json b/Lovelace.Run.Tests/fixtures/error_envelope.json
index 8d14998..77ca00e 100644
--- a/Lovelace.Run.Tests/fixtures/error_envelope.json
+++ b/Lovelace.Run.Tests/fixtures/error_envelope.json
@@ -17,5 +17,6 @@
   ],
   "elapsed": "<volatile>",
   "elapsedTime": "<volatile>",
-  "timings": "<volatile>"
+  "timings": "<volatile>",
+  "output": []
 }
~~~

**Consequence for the suite**: on the pristine control tree the patched golden alone fails there (§9), which is the second, independent proof that
the fixture moved WITH the contract and not around it.

## 9. Control (requirement 5)

`git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-pdctl HEAD` -> `3792829`, pristine. Copied in: the two new test files and the
one moved golden — **only** test files, no product file:

~~~
M Lovelace.Run.Tests/fixtures/error_envelope.json
?? Lovelace.Run.Tests/ParseErrorClassificationTests.cs
?? Lovelace.Run.Tests/PrintOutputOnFailureTests.cs
~~~

~~~powershell
# in .worktrees\c6-pdctl (product at HEAD)
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~PrintOutputOnFailureTests|FullyQualifiedName~ParseErrorClassificationTests'
~~~

~~~
Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pdctl\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.
dotnet : [xUnit.net 00:00:00.86]     
Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray [FAIL]
At line:10 char:1
+ dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configurat ...
+ ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([xUnit.net 00:0...putArray [FAIL]:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError [FAIL]
Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInAMultiLineScript_CrossesAsParseError [FAIL]
Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput [FAIL]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray [514 ms]
  Error Message:
   Assert.NotNull() Failure: Value is null
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AMissingScriptFile_IsNotAParseError [5 ms]
  Error Message:
   Assert.NotEqual() Failure: Strings are equal
Expected: Not "ParseError"
Actual:       "ParseError"
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError [1 ms]
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
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput [124 ms]
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
Failed!  - Failed:     8, Passed:     4, Skipped:     0, Total:    12, Duration: 727 ms - Lovelace.Run.Tests.dll (net10.0)
~~~

**They fail, on the same assertions**: 8 failed / 4 passed / 12 total (`CONTROL EXIT=1`). The whole project in the control tree, which also
exercises the moved golden:

~~~powershell
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo
~~~

~~~
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_ThatPrintedNothing_CarriesAnEmptyOutputArray [230 ms]
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AMissingScriptFile_IsNotAParseError [16 ms]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsNothingThatNeverRan [4 ms]
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInASingleLineScript_CrossesAsParseError [7 ms]
  Failed Lovelace.Run.Tests.ParseErrorClassificationTests.AParseFailureInAMultiLineScript_CrossesAsParseError [1 ms]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput [131 ms]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_OnAParseFailure_CarriesAnEmptyOutputArray [9 ms]
  Failed Lovelace.Run.Tests.PrintOutputOnFailureTests.AFailingRun_KeepsTheLinePrintedBeforeTheFailure [1 ms]
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "error_envelope", expectedExitCode: 1) [7 ms]
Failed!  - Failed:     9, Passed:   265, Skipped:     0, Total:   274, Duration: 20 s - Lovelace.Run.Tests.dll (net10.0)
~~~

9 failures = the 8 above + `GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "error_envelope")`, whose message names the added key:
`$: keys [category, code, …] != [category, code, …, output, …]`.

## 10. Suite totals (requirement 4)

`dotnet test LovelaceSharp.slnx --configuration Release --nologo`, in `.worktrees\c6-pd`, with `LOVELACE_REQUIRE_SYMPY=1` and the oracle on PATH
(`$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH`), exit 0:

| project | passed | failed | skipped | total |
|---|---|---|---|---|
| `Lovelace.Abstractions.Tests` (net10.0) | 20 | 0 | 0 | 20 |
| `Lovelace.Array.Tests` (net10.0) | 19 | 0 | 0 | 19 |
| `Lovelace.Representation.Tests` (net10.0) | 91 | 0 | 0 | 91 |
| `Lovelace.Integer.Tests` (net10.0) | 148 | 0 | 0 | 148 |
| `Lovelace.Knowledge.Tests` (net10.0) | 28 | 0 | 0 | 28 |
| `Lovelace.Natural.Tests` (net10.0) | 195 | 0 | 0 | 195 |
| `precbench.Tests` (net10.0) | 13 | 0 | 0 | 13 |
| `Lovelace.Console.Tests` (net10.0) | 15 | 0 | 0 | 15 |
| `Lovelace.Complex.Tests` (net10.0) | 121 | 0 | 0 | 121 |
| `Lovelace.Dsp.Tests` (net10.0) | 61 | 0 | 0 | 61 |
| `Lovelace.Real.Tests` (net10.0) | 2495 | 0 | 0 | 2495 |
| `Lovelace.Suite.Tests` (net10.0) | 848 | 0 | 0 | 848 |
| `Lovelace.Run.Tests` (net10.0) | 274 | 0 | 0 | 274 |
| `Lovelace.Symbolics.Tests` (net10.0) | 1148 | 0 | 0 | 1148 |
| `Lovelace.Studio.Tests` (net10.0) | 22 | 0 | 0 | 22 |
| **15 projects** | **5498** | **0** | **0** | **5498** |

Against the cycle-6 baseline (EVD-289: 5473 passed / 0 failed on 15 projects, `Real` non-Heavy 2482): the deltas are exactly the arithmetic of
this run — Real runs all 2495 here (the 13 `Heavy` cases are included in a plain `dotnet test`), and Run is 274 = 262 + the 12 new cases.

Timing subsets, each `--filter 'Category=Timing'`, all exit 0 (baseline EVD-273: Run 5, Suite 8, Symbolics 1, Real 1):

~~~
Lovelace.Suite.Tests  ::  Passed!  - Failed:     0, Passed:     8, Skipped:     0, Total:     8, Duration: 6 s - Lovelace.Suite.Tests.dll (net10.0)
Lovelace.Run.Tests  ::  Passed!  - Failed:     0, Passed:     5, Skipped:     0, Total:     5, Duration: 509 ms - Lovelace.Run.Tests.dll (net10.0)
Lovelace.Real.Tests  ::  Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 2 s - Lovelace.Real.Tests.dll (net10.0)
Lovelace.Symbolics.Tests  ::  Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 539 ms - Lovelace.Symbolics.Tests.dll (net10.0)
~~~

`Lovelace.Symbolics.Tests --filter 'Category=Costly'`, exit 0 (baseline 32/0):

~~~
Passed!  - Failed:     0, Passed:    32, Skipped:     0, Total:    32, Duration: 11 s - Lovelace.Symbolics.Tests.dll (net10.0)
~~~

## 11. The published artifact: a Native AOT publish of the fixed tree, and the same probes on it

~~~powershell
# in .worktrees\c6-pd (the CI publish command, .github/workflows/ci.yml:257-263)
dotnet publish Lovelace.Run\Lovelace.Run.csproj --configuration Release -p:PublishAot=true -p:InvariantGlobalization=true -o out\aot
# exit 0
~~~

~~~


FullName      : C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd\out\aot\Lovelace.Run.exe
Length        : 5803520
LastWriteTime : 9/12/2026 5:33:54 PM



### F5 print-then-det
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"499.1 \u00B5s","elapsedTime":{"value":499.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":60.3,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":181.1,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":["hello"]}

STDERR=
---
### F5 print-never-runs
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"299.1 \u00B5s","elapsedTime":{"value":299.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":34.5,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":110.8,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":["hello"]}

STDERR=
---
### F5 print-then-parsefail
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 22: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":22,"line":1,"column":23}],"elapsed":"180.5 \u00B5s","elapsedTime":{"value":180.5,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 parse third line
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"158.1 \u00B5s","elapsedTime":{"value":158.1,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 parse single line
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"ParseError","category":"ParseError","message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027\u0027 at position 3: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":3,"line":1,"column":4}],"elapsed":"158 \u00B5s","elapsedTime":{"value":158,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### F6 missing file
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"FileReadError","category":"TypeMismatch","message":"Cannot read script file \u0027C:\\definitely\\missing\\x.ls\u0027: Could not find a part of the path \u0027C:\\definitely\\missing\\x.ls\u0027.","recoverable":true,"diagnostics":[],"elapsed":"0 ns","elapsedTime":{"value":0,"unit":"ns"},"timings":[],"output":[]}

STDERR=
---
### F6 unterminated string
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidInput","category":"ParseError","message":"Unterminated string literal at position 0.","recoverable":true,"diagnostics":[{"message":"Unterminated string literal at position 0.","position":0,"line":1,"column":1}],"elapsed":"166 \u00B5s","elapsedTime":{"value":166,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### CONTROL deep parens
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"DepthExceeded","category":"BudgetExceeded","message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","recoverable":true,"diagnostics":[{"message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite the input with less nesting (the engine refuses it before a recursive walk can exhaust the process stack).","position":0,"line":1,"column":1}],"elapsed":"805.4 \u00B5s","elapsedTime":{"value":805.4,"unit":"\u00B5s"},"timings":[],"output":[]}

STDERR=
---
### CONTROL det(1) domain
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"277 \u00B5s","elapsedTime":{"value":277,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":130,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}],"output":[]}

STDERR=
---
### CONTROL success print
EXIT=0
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"2","typed":"2 (Natural)","structured":{"kind":"Natural","value":"2","exact":true}},"output":["hello"],"variables":[{"name":"_","kind":"Natural","display":"2","structured":{"kind":"Natural","value":"2","exact":true}}],"functions":[],"elapsed":"103.3 \u00B5s","elapsedTime":{"value":103.3,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":35.1,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":16,"elapsed":{"value":11.4,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false}]}

STDERR=
---
### CONTROL cancelled print
EXIT=1
STDOUT={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[{"message":"cancellation deadline exceeded: --cancel-after 100 ms, elapsed 116.767 ms (16.767 ms over budget); the deadline was observed, but only after the excess had already been spent.","position":33,"line":1,"column":34}],"elapsed":"116.77 ms","elapsedTime":{"value":116.77,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":72.3,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":14.8,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true},{"position":33,"elapsed":{"value":116.51,"unit":"ms"},"resultKind":"Void","hasOutput":false}],"output":["hello"],"partialOutput":["hello"],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x","structured":{"kind":"Symbolic","pretty":"x","canonical":"(sym x)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":["x"]}}],"cancellation":{"budgetMs":100,"elapsedMs":116.767,"stopped":true,"exceeded":true,"excessMs":16.767}}

STDERR=
---
~~~

Every deciding field is the one §7 promises, now on the artifact class the audits measure: F5's `output:["hello"]`, F6's `ParseError/ParseError` for
both syntax shapes, `FileReadError/TypeMismatch` for the unreadable file, `output:[]` where nothing ran, and the four controls unchanged.

## 12. Could not verify

| item | why |
|---|---|
| The **main checkout's published binary** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` still carries the PRE-fix behaviour | it is another round's artifact and this round did not overwrite it; the fixed AOT binary was published into `.worktrees\c6-pd\out\aot`. A consumer running the main tree's binary still sees F5/F6 until it is republished |
| GitHub Actions / CI | not run; the Linux AOT job is the parent's step. Every suite in §10 ran locally on Windows only |
| Whether any consumer outside these two checkouts keys on `FileReadError`'s category being `ParseError` | no consumer exists in this repository (`grep FileReadError` -> `Runner.cs` only) nor in `C:\Users\ricar\deepseek-harness\packages` (`grep partialOutput\|Lovelace.Run` -> none), but I cannot enumerate consumers outside those trees. The `code` is unchanged, only the category moves |
| `--text` on a failure still publishes no envelope (0 bytes stdout, one line on stderr) | finding F8 of the same audit, a P2 the round was not asked to close; the committed output is therefore still invisible in `--text` mode. Unchanged by this round |
| `--omit-variables` on the cancelled path (F4, P2) | unchanged: `partialVariables` ignores the flag. Out of this round's scope; the cancelled envelope now additionally carries `output`, which does follow the capture |
| `Lovelace.Suite/AstDepth.cs:129` — the "unhandled node type" internal guard | it is a plain `InvalidOperationException` raised during the parse phase, so the phase flag now classifies it `ParseError/ParseError` where it used to be `InvalidOperation/DomainError`. It is unreachable on the current grammar (every node type is enumerated above it) and its honest class would be `InternalInvariantFailure`; the line is in `Lovelace.Suite/**`, out of scope, and is REPORTED here rather than edited |
| A script whose failure path re-parse is itself expensive (a very large failing script parses twice) | measured only for correctness, not cost: the probe is on the failure path only, and no timing assertion covers it |

## 13. Files changed in the round's tree (scope) and hashes

~~~
# git -C .worktrees\c6-pd status --short
 M Lovelace.Run.Tests/fixtures/error_envelope.json
 M Lovelace.Run/RunProtocol.cs
 M Lovelace.Run/Runner.cs
?? Lovelace.Run.Tests/ParseErrorClassificationTests.cs
?? Lovelace.Run.Tests/PrintOutputOnFailureTests.cs

# SHA256
96D9129DC412E0B7BF85CF385F9092457B4026EFB6D3E6F9A960A1D45E260952  Lovelace.Run\Runner.cs
0ADD79A4A8452B3BF02935CDE17C16CE6A84CDA330E527EA68E18095D75E9B5B  Lovelace.Run\RunProtocol.cs
FBA63DDDBD6A1E0534568D7D3991ADA8ADF913AA3B36DB80D4C264FE9F93C1C4  Lovelace.Run.Tests\fixtures\error_envelope.json
0114818DE35FE4DCA74857A8B35A7D568758C2D62ED34E506F4510104B91D971  Lovelace.Run.Tests\PrintOutputOnFailureTests.cs
4813D624B718F03188D7E687705CC76C709B381203FBF3A8A8AB9DDB0FFF5D3E  Lovelace.Run.Tests\ParseErrorClassificationTests.cs
~~~

Nothing outside `Lovelace.Run/**` and `Lovelace.Run.Tests/**` was edited; no commit, stage or push; the deliverable of this round is this document.

## Appendix — the probe scripts, verbatim

~~~powershell
$ErrorActionPreference = 'Continue'
$d = Join-Path $env:TEMP 'c6pd'
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'

'=== file bytes ==='
Get-ChildItem (Join-Path $d '*.ls') | Sort-Object Name | ForEach-Object {
  $bytes = [IO.File]::ReadAllBytes($_.FullName)
  $hex = ($bytes | ForEach-Object { $_.ToString('X2') }) -join ' '
  '{0} ({1} bytes): {2}' -f $_.Name, $bytes.Length, $hex
}

function Probe([string]$Label, [string[]]$Argv) {
  $outFile = Join-Path $d 'stdout.tmp'
  $errFile = Join-Path $d 'stderr.tmp'
  $p = Start-Process -FilePath $exe -ArgumentList $Argv -NoNewWindow -Wait -PassThru -RedirectStandardOutput $outFile -RedirectStandardError $errFile
  '### ' + $Label
  'CMD: ' + ($Argv -join ' ')
  'EXIT=' + $p.ExitCode
  'STDOUT=' + [IO.File]::ReadAllText($outFile)
  'STDERR=' + [IO.File]::ReadAllText($errFile)
  '---'
}

Probe 'F5 print-then-det'        @('--file', (Join-Path $d 'f5.ls'), '--json', '--omit-functions')
Probe 'F5 print-then-unknownfn'  @('--file', (Join-Path $d 'f5b.ls'), '--json', '--omit-functions')
Probe 'F5 print-never-runs'      @('--file', (Join-Path $d 'f5c.ls'), '--json', '--omit-functions')
Probe 'F6 parse third line'      @('--file', (Join-Path $d 'f6ml.ls'), '--json', '--omit-functions')
Probe 'F6 parse single line'     @('--file', (Join-Path $d 'f6single.ls'), '--json', '--omit-functions')
Probe 'F6 missing file'          @('--file', 'C:\definitely\missing\x.ls', '--json', '--omit-functions')
Probe 'CONTROL success print'    @('--file', (Join-Path $d 'ok.ls'), '--json', '--omit-functions')
Probe 'CONTROL cancelled print'  @('--file', (Join-Path $d 'cancel.ls'), '--json', '--omit-functions', '--cancel-after', '100')
~~~

~~~powershell
$ErrorActionPreference = 'Continue'
$d = Join-Path $env:TEMP 'c6pd'
$exe = 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe'
function Probe([string]$Label, [string[]]$Argv) {
  $outFile = Join-Path $d 'stdout.tmp'; $errFile = Join-Path $d 'stderr.tmp'
  $p = Start-Process -FilePath $exe -ArgumentList $Argv -NoNewWindow -Wait -PassThru -RedirectStandardOutput $outFile -RedirectStandardError $errFile
  '### ' + $Label; 'EXIT=' + $p.ExitCode
  'STDOUT=' + [IO.File]::ReadAllText($outFile)
  'STDERR=' + [IO.File]::ReadAllText($errFile); '---'
}
Probe 'unterminated string'  @('--file', (Join-Path $d 'unterm.ls'), '--json', '--omit-functions')
Probe 'deep parens 3000'     @('--file', (Join-Path $d 'deep.ls'), '--json', '--omit-functions')
Probe 'det(1) domain'        @('--file', (Join-Path $d 'det1.ls'), '--json', '--omit-functions')
Probe 'print-then-parsefail' @('--file', (Join-Path $d 'printparse.ls'), '--json', '--omit-functions')
~~~

~~~powershell
$ErrorActionPreference = 'Continue'
$d = Join-Path $env:TEMP 'c6pd'
$exe = 'C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pd\out\aot\Lovelace.Run.exe'
Get-Item $exe | Select-Object FullName,Length,LastWriteTime | Format-List
function Probe([string]$Label, [string[]]$Argv) {
  $outFile = Join-Path $d 'stdout.tmp'; $errFile = Join-Path $d 'stderr.tmp'
  $p = Start-Process -FilePath $exe -ArgumentList $Argv -NoNewWindow -Wait -PassThru -RedirectStandardOutput $outFile -RedirectStandardError $errFile
  '### ' + $Label; 'EXIT=' + $p.ExitCode
  'STDOUT=' + [IO.File]::ReadAllText($outFile)
  'STDERR=' + [IO.File]::ReadAllText($errFile); '---'
}
Probe 'F5 print-then-det'          @('--file', (Join-Path $d 'f5.ls'), '--json', '--omit-functions')
Probe 'F5 print-never-runs'        @('--file', (Join-Path $d 'f5c.ls'), '--json', '--omit-functions')
Probe 'F5 print-then-parsefail'    @('--file', (Join-Path $d 'printparse.ls'), '--json', '--omit-functions')
Probe 'F6 parse third line'        @('--file', (Join-Path $d 'f6ml.ls'), '--json', '--omit-functions')
Probe 'F6 parse single line'       @('--file', (Join-Path $d 'f6single.ls'), '--json', '--omit-functions')
Probe 'F6 missing file'            @('--file', 'C:\definitely\missing\x.ls', '--json', '--omit-functions')
Probe 'F6 unterminated string'     @('--file', (Join-Path $d 'unterm.ls'), '--json', '--omit-functions')
Probe 'CONTROL deep parens'        @('--file', (Join-Path $d 'deep.ls'), '--json', '--omit-functions')
Probe 'CONTROL det(1) domain'      @('--file', (Join-Path $d 'det1.ls'), '--json', '--omit-functions')
Probe 'CONTROL success print'      @('--file', (Join-Path $d 'ok.ls'), '--json', '--omit-functions')
Probe 'CONTROL cancelled print'    @('--file', (Join-Path $d 'cancel.ls'), '--json', '--omit-functions', '--cancel-after', '100')
~~~
