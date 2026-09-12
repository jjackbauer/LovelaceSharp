# Cycle-6 · round-19 · implementation — the published positions name the caller's place

**Findings closed**: second-wave audit E (round 18) **F1**, **F2**, **F3** — the amendment's P.2 rows
**E-P1a**, **E-P1b**, **E-P1c** (the three P1s whose subject is a SOURCE POSITION in the envelope).

**Tree used**: `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos`, created with
`git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/c6-pos HEAD` → detached HEAD
`3792829`. Nothing was staged, committed or pushed. The control tree is
`C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl` — the same HEAD, pristine, no product change.
This document was written in the worktree and copied to the main tree at the path the brief names. The
worktree root also carries the four untracked reproduction scripts the transcripts below came from —
§probe-positions.ps1§ (published binary), §probe-positions-after.ps1§, §probe-crlf-file.ps1§,
§probe-scale.ps1§ — plus this document; §git status --porcelain§ there lists nothing else.

| file | change |
|---|---|
| `Lovelace.Run/ScriptPositions.cs` | **new** — the map from the engine's text back to the caller's text, plus the caller's line/column rule |
| `Lovelace.Run/Runner.cs` | one map per run; timings, diagnostics and the overrun diagnostic all go through it |
| `Lovelace.Run/RunProtocol.cs` | doc comments on `TimingDto`/`DiagnosticDto` only — no field added, renamed or removed |
| `Lovelace.Run.Tests/SourcePositionTests.cs` | **new** — 11 methods / 12 cases; 8 cases fail on the pristine product, 4 guard values that were already right |
| `Lovelace.Run.Tests/fixtures/error_envelope.json` | regenerated from the product: `position 0 → 17`, `column 1 → 18` |

`Lovelace.Suite/**` and `Lovelace.Symbolics/**` were **not** touched: the CRLF collapse at
`Lovelace.Suite/ScriptSource.cs:27` and the leading-BOM strip at `:31-32` stay exactly as they are, and the
runner now translates on the way out. `git status --porcelain` in the worktree lists only the five files
above.

## 1. Pre-fix behaviour, re-observed on the published binary

Binary: `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`, **5 801 472 bytes, last write
2026-09-12 16:56:28 local** — the artifact audit E measured. Command shape (Windows PowerShell 5.1; the
LF/CRLF/CR scripts are built from `[char]10` / `[char]13` so each argument is exact):

~~~powershell
& C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe --eval <script> --json --omit-functions
~~~

Every case was run twice; the two runs differ only in the duration digits. Verbatim:

### F1-LF-3stmt-det(1)-offset12
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"462 \u00B5s","elapsedTime":{"value":462,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":5.3,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":6,"elapsed":{"value":2.8,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":12,"elapsed":{"value":272.2,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}
exit=1

### F2-LF-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"274.3 \u00B5s","elapsedTime":{"value":274.3,"unit":"\u00B5s"},"timings":[]}
exit=1

### F3-CRLF-3stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"6","typed":"6 (Natural)","structured":{"kind":"Natural","value":"6","exact":true}},"output":[],"variables":[{"name":"_","kind":"Natural","display":"6","structured":{"kind":"Natural","value":"6","exact":true}}],"functions":[],"elapsed":"104.1 \u00B5s","elapsedTime":{"value":104.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":15.1,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":5,"elapsed":{"value":3.2,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":10,"elapsed":{"value":500,"unit":"ns"},"resultKind":"Natural","hasOutput":false}]}
exit=0

### F3-CRLF-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"299.3 \u00B5s","elapsedTime":{"value":299.3,"unit":"\u00B5s"},"timings":[]}
exit=1

### CTRL-CR-3stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"6","typed":"6 (Natural)","structured":{"kind":"Natural","value":"6","exact":true}},"output":[],"variables":[{"name":"_","kind":"Natural","display":"6","structured":{"kind":"Natural","value":"6","exact":true}}],"functions":[],"elapsed":"123.1 \u00B5s","elapsedTime":{"value":123.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":15.9,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":5,"elapsed":{"value":3.9,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":10,"elapsed":{"value":500,"unit":"ns"},"resultKind":"Natural","hasOutput":false}]}
exit=0

### CTRL-CR-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"300.3 \u00B5s","elapsedTime":{"value":300.3,"unit":"\u00B5s"},"timings":[]}
exit=1

### CTRL-BOM-1stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"575 \u00B5s","elapsedTime":{"value":575,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":318.9,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}
exit=1

### CTRL-LF-1stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"548.6 \u00B5s","elapsedTime":{"value":548.6,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":314.4,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":false}]}
exit=1

The 13-byte CRLF file from the audit — bytes `31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29`, the offending `)` at
offset **12** — before and after the fix, same file, same command (`--file <path> --json --omit-functions`):

file bytes: 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29  length=13
### BEFORE (published AOT binary out\aot\Lovelace.Run.exe)
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":1,"column":11}],"elapsed":"154.1 \u00B5s","elapsedTime":{"value":154.1,"unit":"\u00B5s"},"timings":[]}
exit=1
### AFTER (worktree build .worktrees\c6-pos\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll)
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":12,"line":3,"column":1}],"elapsed":"17.53 ms","elapsedTime":{"value":17.53,"unit":"ms"},"timings":[]}
exit=1

Read off the first transcript:

* the failing `det(1)` at offset **12** is reported as `"position":0,"line":1,"column":1` while the SAME
  envelope's `timings` are `[0,6,12]` — the diagnostic names the wrong place, and the envelope contradicts
  itself (F1);
* the `)` that is the first character of **source line 3** is reported as `"line":1,"column":11` — a column
  past the end of line 1, which is four characters long (F2);
* the three CRLF-separated statements start at the caller's offsets **0, 6, 12** and are published as
  **0, 5, 10** — one character early per preceding CRLF (F3).

## 2. The failing test first

`Lovelace.Run.Tests/SourcePositionTests.cs` (new). Every expectation is computed by the test from the exact
text — or the exact BYTES — the test handed to the runner; none is read back out of the envelope, and each
fixture carries a self-check pinning the literal (`Assert.Equal(12, failing)`) so a drifted fixture cannot
silently move an expectation. `Locate` (`SourcePositionTests.cs:291-322`) is the test's own line/column
oracle: it walks the caller's text counting characters, with CRLF as one break and a lone CR or LF as one
each.

| case (method line) | what it pins |
|---|---|
| `RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf` (:39) | `det(1)` at 12 → position 12, line 3, column 1; timings `[0,6,12]`; diagnostic == last timing |
| `..._Crlf` (:61) | the same script with CRLF: the call is at 14 → position 14, line 3, column 1; timings `[0,7,14]` |
| `ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf` (:85) | `1+1;⏎2+2;⏎)` → position 10, line 3, column 1; `timings` empty |
| `..._CrlfFile` (:104) | the test writes the 13 CRLF **bytes**, reads them back, finds `)` at 12 → position 12, line 3, column 1 |
| `TimingsPositions_AreCallerOffsets_Crlf` (:138) | CRLF three statements → `[0,6,12]` |
| `TimingsPositions_AreCallerOffsets_LfAndCr` (:159, Theory) | LF and CR-only three statements → `[0,5,10]` (no regression) |
| `ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly` (:178) | CR-only refusal → position 10, line 3, column 1 |
| `SingleStatementFailure_KeepsPositionZeroLineOneColumnOne` (:200) | `det(1)` alone → 0/1/1 (no regression) |
| `FailureInTheSecondStatementOfOneLine_NamesThatStatement` (:214) | `1+1; det(1)` → position 5, line 1, column 6 |
| `LeadingBom_PositionsStayOffsetsIntoTheCallersText` (:233) | `\uFEFFdet(1)` → position 1, line 1, column 2 (offsets index the caller's argument, BOM included) |
| `DiagnosticEntry_KeepsItsFourFieldShape` (:249) | the diagnostics entry is still exactly `message`/`position`/`line`/`column`, and the error envelope's key SET is unchanged |

Run against the pristine product, **before any product change was made** (this is the failing-first
transcript; build lines trimmed):

~~~powershell
PS .worktrees\c6-pos> dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter 'FullyQualifiedName~SourcePositionTests'
~~~

Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.
dotnet : [xUnit.net 00:00:00.34]     
Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf [FAIL]
At line:1 char:184
+ ... ees\c6-pos; dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj  ...
+                 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([xUnit.net 00:0...ent_Crlf [FAIL]:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
[xUnit.net 00:00:00.35]     Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf [FAIL]
[xUnit.net 00:00:00.35]     Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf 
[FAIL]
[xUnit.net 00:00:00.37]     
Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile [FAIL]
[xUnit.net 00:00:00.37]     Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf 
[FAIL]
[xUnit.net 00:00:00.37]     Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement 
[FAIL]
  Failed Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf [11 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 14
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 68
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf [11 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
              ↓ (pos 1)
Expected: [0, 6, 12]
Actual:   [0, 5, 10]
              ↑ (pos 1)
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 149
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 12
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 48
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile [14 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 12
Actual:   10
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 118
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 3
Actual:   1
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 92
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 5
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 220
--- End of stack trace from previous location ---
[xUnit.net 00:00:00.39]     Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly 
[FAIL]
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 3
Actual:   1
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-pos\Lovelace.Run.Tests\SourcePositionTests.cs:line 187
--- End of stack trace from previous location ---
Failed!  - Failed:     7, Passed:     4, Skipped:     0, Total:    11, Duration: 183 ms - Lovelace.Run.Tests.dll (net10.0)


The four cases that pass pre-fix are exactly the no-regression guards (LF and CR timings, the
single-statement failure, and the DTO field shape) — the seven failures are the defects.

This first run carries 11 cases: the leading-BOM case was added afterwards, for a limb of the same map,
and it is part of the final 12-case control in §6 (where it fails on the pristine product like the other
seven).

## 3. The fix

Every published position now goes through ONE translation, host-side (no engine change):

* `Lovelace.Run/ScriptPositions.cs:36-49` holds the caller's text and the engine's text
  (`ScriptSource.ToSemicolonStatements(source)` — the same argument as before) and builds
  `_sourceIndex`, where `_sourceIndex[i]` is the offset in the CALLER's text of engine character `i`
  (`:74-86`). The map is exact because the newline→`;`/space rewrite appends exactly one character per
  input character (`Lovelace.Suite/ScriptSource.cs:39-98`): the only non-1:1 steps are the CRLF→LF collapse
  (`:27` — two characters become one, mapped at the LF) and the leading-BOM strip (`:31-32` — one
  character, skipped).
* `ScriptPositions.cs:53-60` maps an engine offset to a caller offset (clamped at both ends);
  `:65-66` + `:91-117` produce the 1-based line and column **read off the caller's text**, with CRLF, CR
  and LF each ending a line exactly once.
* `Runner.cs:164` builds the map once per run (just above the engine); `Runner.cs:203` hands the engine
  `script.EngineSource`, byte-for-byte the old argument.
* `Runner.cs:523-533` (`Timings`) publishes each timing's position through the map — success path call site
  `Runner.cs:251`, failure path `:290`.
* `Runner.cs:432-440` (`PublishDiagnostic`) publishes each engine diagnostic. A RUNTIME failure carries no
  position of its own (`SuiteEngine.ToDiagnostic` defaults to 0, `Lovelace.Suite/SuiteEngine.cs:359-370`),
  but the interpreter records the statement that was running when it threw, and the failing statement is
  always the LAST timing — its `finally` adds the entry on the way out
  (`Lovelace.Suite/Interpreter.cs:333-341`). So whenever any statement ran, that offset IS the failure's
  position; only a failure before the first statement (a tokenizer/parser refusal, which is the one
  diagnostic that carries the engine's scraped `at position N`) keeps the engine's own value.
* `Runner.cs:444-449` (`Locate`) is the single translation both the diagnostic path and the overrun path
  use; `Runner.cs:405-420` (`OverrunDiagnostic`) now reports the caller's coordinates too — it used to run
  its own LF-only line counting over the caller's text, which was wrong for a CR-only script as well.
  Runner's old private `ComputeLineColumn` is gone: one rule, not two.
* Call sites in the failure path: `Runner.cs:267` (diagnostics) and `:290` (timings).

What did **not** change: `protocolVersion` 1, every DTO name and field (`SourcePositionTests.cs:249-264`
pins the key sets), the success envelope's shape, the JSON contexts, and the message text.

## 4. Before / after — every cell observed

| case | source (true place) | before | after |
|---|---|---|---|
| F1 runtime failure, LF | `a = 1⏎b = 2⏎det(1)` — failing call at **12** (line 3 col 1) | diag `0/1/1`; timings `[0,6,12]` | diag **`12/3/1`**; timings `[0,6,12]` |
| F1 runtime failure, CRLF | same with CRLF — failing call at **14** (line 3 col 1) | diag `0/1/1`; timings `[0,7,14]` | diag **`14/3/1`**; timings `[0,7,14]` |
| F2 parse refusal, LF | `1+1;⏎2+2;⏎)` — the `)` at **10** (line 3 col 1) | `10/1/11` | **`10/3/1`** |
| F2+F3 parse refusal, CRLF file (13 B) | `1+1;␍⏎2+2;␍⏎)` — the `)` at **12** (line 3 col 1) | `10/1/11` | **`12/3/1`** |
| F3 timings, CRLF | `1+1;␍⏎2+2;␍⏎3+3` — true offsets `0,6,12` | `[0,5,10]` | **`[0,6,12]`** |
| control: timings, LF | `1+1;⏎2+2;⏎3+3` — true `0,5,10` | `[0,5,10]` | `[0,5,10]` (unchanged) |
| control: timings, CR only | `1+1;␍2+2;␍3+3` — true `0,5,10` | `[0,5,10]` | `[0,5,10]` (unchanged) |
| control: single statement | `det(1)` at 0 | `0/1/1` | `0/1/1` (unchanged) |
| control: two statements, one line | `1+1; det(1)` — failing call at **5** (line 1 col 6) | `0/1/1` | **`5/1/6`** |
| control: CR-only parse refusal | `1+1;␍2+2;␍)` — the `)` at **10** (line 3 col 1) | `10/1/11` | **`10/3/1`** |
| control: leading BOM | `\uFEFFdet(1)` — the call at **1** of the caller's argument (line 1 col 2) | `0/1/1` | **`1/1/2`** |

The after transcript (the same probe script, run against the fixed build
`.worktrees/c6-pos/Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` via `dotnet exec`):

### F1-LF-3stmt-det(1)-offset12
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":12,"line":3,"column":1}],"elapsed":"22.76 ms","elapsedTime":{"value":22.76,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":2.54,"unit":"ms"},"resultKind":"Natural","hasOutput":false},{"position":6,"elapsed":{"value":6.2,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":12,"elapsed":{"value":4.26,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}
exit=1

### F2-LF-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":3,"column":1}],"elapsed":"21.44 ms","elapsedTime":{"value":21.44,"unit":"ms"},"timings":[]}
exit=1

### F3-CRLF-3stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"6","typed":"6 (Natural)","structured":{"kind":"Natural","value":"6","exact":true}},"output":[],"variables":[{"name":"_","kind":"Natural","display":"6","structured":{"kind":"Natural","value":"6","exact":true}}],"functions":[],"elapsed":"15.27 ms","elapsedTime":{"value":15.27,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":3.69,"unit":"ms"},"resultKind":"Natural","hasOutput":false},{"position":6,"elapsed":{"value":5.1,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":12,"elapsed":{"value":1.8,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false}]}
exit=0

### F3-CRLF-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":12,"line":3,"column":1}],"elapsed":"17.7 ms","elapsedTime":{"value":17.7,"unit":"ms"},"timings":[]}
exit=1

### F1-CRLF-3stmt-det(1)-offset14
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":14,"line":3,"column":1}],"elapsed":"22.26 ms","elapsedTime":{"value":22.26,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":2.44,"unit":"ms"},"resultKind":"Natural","hasOutput":false},{"position":7,"elapsed":{"value":6.8,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":14,"elapsed":{"value":4.23,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}
exit=1

### CTRL-CR-3stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Natural","display":"6","typed":"6 (Natural)","structured":{"kind":"Natural","value":"6","exact":true}},"output":[],"variables":[{"name":"_","kind":"Natural","display":"6","structured":{"kind":"Natural","value":"6","exact":true}}],"functions":[],"elapsed":"15.17 ms","elapsedTime":{"value":15.17,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":2.89,"unit":"ms"},"resultKind":"Natural","hasOutput":false},{"position":5,"elapsed":{"value":4.2,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false},{"position":10,"elapsed":{"value":1.3,"unit":"\u00B5s"},"resultKind":"Natural","hasOutput":false}]}
exit=0

### CTRL-CR-parse-line3
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","recoverable":true,"diagnostics":[{"message":"Unexpected token \u0027)\u0027 at position 10: expected a number, string, identifier, \u0027[\u0027, or \u0027(\u0027.","position":10,"line":3,"column":1}],"elapsed":"18.82 ms","elapsedTime":{"value":18.82,"unit":"ms"},"timings":[]}
exit=1

### CTRL-BOM-1stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":1,"line":1,"column":2}],"elapsed":"22.82 ms","elapsedTime":{"value":22.82,"unit":"ms"},"timings":[{"position":1,"elapsed":{"value":6.26,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}
exit=1

### CTRL-LF-1stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":0,"line":1,"column":1}],"elapsed":"22.77 ms","elapsedTime":{"value":22.77,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":5.43,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}
exit=1

### CTRL-LF-1line-2nd-stmt
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":5,"line":1,"column":6}],"elapsed":"22.44 ms","elapsedTime":{"value":22.44,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":2.79,"unit":"ms"},"resultKind":"Natural","hasOutput":false},{"position":5,"elapsed":{"value":3.33,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}
exit=1

Scale check — 20 000 CRLF-separated statements, 297 778-byte file, last statement at true offset
297 764 (`match=True` after; `False` before):

file bytes=297778  statements=20000  expected last statement offset=297764
after: timings count=20000  first position=0  last position=297764  expected last=297764  match=True
before: timings count=20000  first position=0  last position=277765  expected last=297764  match=False

## 5. The golden fixture

`Lovelace.Run.Tests/fixtures/error_envelope.json` pinned `"position": 0, "line": 1, "column": 1` for a
failure at offset **17**. The fix publishes 17/1/18, so the pin was regenerated **from the product**:

1. a temporary test written for this run (and deleted afterwards) called the product through the same entry
   point the golden test uses (`TestSupport.RunFixtureAsync("error_envelope")`), applied the comparison's
   own volatility rule (`TestSupport.Normalise`: `revision`/`elapsed`/`elapsedTime`/`timings` →
   `"<volatile>"`) and wrote the result with two-space indentation and `UnsafeRelaxedJsonEscaping` — that
   encoder is what spells `<volatile>` literally, as the committed golden does;
2. the regenerated file was diffed against the committed golden: **every byte outside the two changed lines
   is identical** (one hunk, below), so the regeneration procedure is faithful and the only change is the
   value the fix moves;
3. the raw (un-normalised) envelope the product emitted in the same run:

{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"solve(): currently supports domains real and complex; got integer.","recoverable":true,"diagnostics":[{"message":"solve(): currently supports domains real and complex; got integer.","position":17,"line":1,"column":18}],"elapsed":"29.22 ms","elapsedTime":{"value":29.22,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":7.55,"unit":"ms"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":8.18,"unit":"ms"},"resultKind":"Void","hasOutput":false}]}

**Justification of the new values.** The fixture script is
`x = symbol("x"); solve(2*x == 1, x, integer)`. The failing call `solve(…)` begins at offset **17** — the
product's own `timings[1].position` is 17 in the raw envelope above, and
`Lovelace.Run.Tests/EnvelopeDurationTests.cs:70` already pins 17 as the second statement's offset. The old
pin named offset 0, which is the `x` of the FIRST statement, the one that succeeded. The script is one
line, so `line` stays 1 and `column` is the 1-based reading of offset 17 = **18**.

diff --git a/Lovelace.Run.Tests/fixtures/error_envelope.json b/Lovelace.Run.Tests/fixtures/error_envelope.json
index 8d14998..c15d829 100644
--- a/Lovelace.Run.Tests/fixtures/error_envelope.json
+++ b/Lovelace.Run.Tests/fixtures/error_envelope.json
@@ -10,9 +10,9 @@
   "diagnostics": [
     {
       "message": "solve(): currently supports domains real and complex; got integer.",
-      "position": 0,
+      "position": 17,
       "line": 1,
-      "column": 1
+      "column": 18
     }
   ],
   "elapsed": "<volatile>",

## 6. Control — the tests have teeth

Only the two test-side files were copied into the pristine HEAD tree; no product file differs there:

~~~powershell
PS .worktrees\c6-posctl> git status --porcelain
 M Lovelace.Run.Tests/fixtures/error_envelope.json
?? Lovelace.Run.Tests/SourcePositionTests.cs
~~~

### CONTROL (final test files) on pristine HEAD .worktrees/c6-posctl
 M Lovelace.Run.Tests/fixtures/error_envelope.json
?? Lovelace.Run.Tests/SourcePositionTests.cs
Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.
dotnet : [xUnit.net 00:00:00.32]     
Lovelace.Run.Tests.SourcePositionTests.LeadingBom_PositionsStayOffsetsIntoTheCallersText [FAIL]
At line:1 char:667
+ ... og -Append; dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj  ...
+                 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([xUnit.net 00:0...lersText [FAIL]:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
[xUnit.net 00:00:00.32]     
Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf [FAIL]
[xUnit.net 00:00:00.33]     Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf [FAIL]
[xUnit.net 00:00:00.33]     Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf 
[FAIL]
[xUnit.net 00:00:00.35]     
Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile [FAIL]
[xUnit.net 00:00:00.35]     Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf 
[FAIL]
  Failed Lovelace.Run.Tests.SourcePositionTests.LeadingBom_PositionsStayOffsetsIntoTheCallersText [9 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 1
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.LeadingBom_PositionsStayOffsetsIntoTheCallersText() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 241
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf [1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 14
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 70
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf [9 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
              ↓ (pos 1)
Expected: [0, 6, 12]
Actual:   [0, 5, 10]
              ↑ (pos 1)
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.TimingsPositions_AreCallerOffsets_Crlf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 152
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 12
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 48
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile [12 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 12
Actual:   10
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 121
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 3
Actual:   1
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 95
--- End of stack trace from previous location ---
[xUnit.net 00:00:00.37]     Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement 
[FAIL]
[xUnit.net 00:00:00.37]     Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly 
[FAIL]
  Failed Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 5
Actual:   0
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.FailureInTheSecondStatementOfOneLine_NamesThatStatement() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 223
--- End of stack trace from previous location ---
  Failed Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 3
Actual:   1
  Stack Trace:
     at Lovelace.Run.Tests.SourcePositionTests.ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\SourcePositionTests.cs:line 190
--- End of stack trace from previous location ---
Failed!  - Failed:     8, Passed:     4, Skipped:     0, Total:    12, Duration: 169 ms - Lovelace.Run.Tests.dll (net10.0)
CONTROL_SOURCEPOS_EXIT=1


### CONTROL: golden fixture pinned to the NEW positions, run against the pristine HEAD product
Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll (.NETCoreApp,Version=v10.0)
A total of 1 test files matched the specified pattern.
dotnet : [xUnit.net 00:00:00.40]     Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: 
"error_envelope", expectedExitCode: 1) [FAIL]
At line:1 char:437
+ ... og -Append; dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj  ...
+                 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    + CategoryInfo          : NotSpecified: ([xUnit.net 00:0...Code: 1) [FAIL]:String) [], RemoteException
    + FullyQualifiedErrorId : NativeCommandError
 
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "error_envelope", expectedExitCode: 1) [7 ms]
  Error Message:
   error_envelope: envelope does not match the golden fixture:
$.diagnostics[0].column: expected 18, got 1
$.diagnostics[0].position: expected 17, got 0
  Stack Trace:
     at Lovelace.Run.Tests.TestSupport.AssertSameJson(JsonNode expected, JsonNode actual, String what) in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\TestSupport.cs:line 143
   at Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(String name, Int32 expectedExitCode) in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-posctl\Lovelace.Run.Tests\GoldenEnvelopeTests.cs:line 59
--- End of stack trace from previous location ---
Failed!  - Failed:     1, Passed:    32, Skipped:     0, Total:    33, Duration: 309 ms - Lovelace.Run.Tests.dll (net10.0)
CONTROL_GOLDEN_EXIT=1


## 7. Suite totals

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` in `.worktrees/c6-pos` — **exit 0, 15
projects, 0 failed, 5 490 passed, 8 skipped**:

Passed!  - Failed:     0, Passed:    28, Skipped:     0, Total:    28, Duration: 143 ms - Lovelace.Knowledge.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    20, Skipped:     0, Total:    20, Duration: 67 ms - Lovelace.Abstractions.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    91, Skipped:     0, Total:    91, Duration: 137 ms - Lovelace.Representation.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    19, Skipped:     0, Total:    19, Duration: 104 ms - Lovelace.Array.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   148, Skipped:     0, Total:   148, Duration: 92 ms - Lovelace.Integer.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   195, Skipped:     0, Total:   195, Duration: 139 ms - Lovelace.Natural.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    13, Skipped:     0, Total:    13, Duration: 114 ms - precbench.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15, Duration: 511 ms - Lovelace.Console.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22, Duration: 462 ms - Lovelace.Studio.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   121, Skipped:     0, Total:   121, Duration: 1 s - Lovelace.Complex.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:    61, Skipped:     0, Total:    61, Duration: 4 s - Lovelace.Dsp.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:  2495, Skipped:     0, Total:  2495, Duration: 5 s - Lovelace.Real.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   848, Skipped:     0, Total:   848, Duration: 16 s - Lovelace.Suite.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:  1140, Skipped:     8, Total:  1148, Duration: 25 s - Lovelace.Symbolics.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:   274, Skipped:     0, Total:   274, Duration: 27 s - Lovelace.Run.Tests.dll (net10.0)

Timing subsets and the Symbolics Costly subset (`--no-build`, per project):

### Suite Timing  filter=Category=Timing

Passed!  - Failed:     0, Passed:     8, Skipped:     0, Total:     8, Duration: 4 s - Lovelace.Suite.Tests.dll (net10.0)


### Run Timing  filter=Category=Timing

Passed!  - Failed:     0, Passed:     5, Skipped:     0, Total:     5, Duration: 430 ms - Lovelace.Run.Tests.dll (net10.0)


### Real Timing  filter=Category=Timing

Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 2 s - Lovelace.Real.Tests.dll (net10.0)


### Symbolics Timing  filter=Category=Timing

Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 296 ms - Lovelace.Symbolics.Tests.dll 
(net10.0)


### Symbolics Costly  filter=Category=Costly

Passed!  - Failed:     0, Passed:    32, Skipped:     0, Total:    32, Duration: 12 s - Lovelace.Symbolics.Tests.dll 
(net10.0)

## 8. Could not verify / known residuals

1. **The parse-error MESSAGE still carries the engine's offset.** For the CRLF file `message` says
   `"Unexpected token ')' at position 10"` while `position` is now 12 — the message is the exception's own
   prose, published verbatim, and the findings name the numeric position fields. For an LF source the two
   agree. Rewriting prose in the host would be a new contract on the message text, so it was not done.
   Visible in the F3-CRLF-parse transcript above.
2. **The protocol document's own error example is now stale**: `docs/symbolics/dsh-protocol.md:194-199`
   shows `timings[1].position 17` next to `diagnostics[0] "position": 0, "line": 1, "column": 1`. That
   directory is outside this round's SCOPE (another round owns it) — left for its owner.
3. **The Native AOT binary was not re-published.** The after-observations are the Release build of
   `Lovelace.Run.dll` (via `dotnet exec`) and the in-process test host; publishing AOT is CI's step.
   Nothing in the change touches serialisation or reflection (the source-generated `RunJsonContext` is
   untouched), so no AOT-only path is new — but this was not measured either way.
4. **BOM routes differ by construction**: `--file` decodes with BOM detection
   (`File.ReadAllTextAsync`), so its offsets index the text after the BOM, while `--eval`/`--stdin` keep the
   caller's BOM and include it. Both are literal offsets into the text the runner received; only the
   `--eval` route is pinned by a test.
5. **`--stdin` was not probed** (it shares the `--eval` path from `source` onward; only the read differs).
6. Wall-clock-tagged tests (`Category=Timing`) are machine-sensitive and the machine was not idle; they
   passed (section 7). The Symbolics `Category=Timing` filter matched 1 test today, not the 8 that
   `docs/symbolics/a-plus-cycle-6-amendment.md:25` records — that is a property of the current test set,
   unrelated to this change.

## 9. Appendix — the complete diff

===== git diff (tracked: Runner.cs, RunProtocol.cs, fixtures/error_envelope.json) =====
diff --git a/Lovelace.Run.Tests/fixtures/error_envelope.json b/Lovelace.Run.Tests/fixtures/error_envelope.json
index 8d14998..c15d829 100644
--- a/Lovelace.Run.Tests/fixtures/error_envelope.json
+++ b/Lovelace.Run.Tests/fixtures/error_envelope.json
@@ -10,9 +10,9 @@
   "diagnostics": [
     {
       "message": "solve(): currently supports domains real and complex; got integer.",
-      "position": 0,
+      "position": 17,
       "line": 1,
-      "column": 1
+      "column": 18
     }
   ],
   "elapsed": "<volatile>",
diff --git a/Lovelace.Run/RunProtocol.cs b/Lovelace.Run/RunProtocol.cs
index 1918c53..5240466 100644
--- a/Lovelace.Run/RunProtocol.cs
+++ b/Lovelace.Run/RunProtocol.cs
@@ -24,8 +24,12 @@ internal sealed record VariableDto(string Name, string Kind, string Display, Str
 /// <summary>A duration as a unit-scaled value plus its unit, so an agent never parses a suffix.</summary>
 internal sealed record DurationDto(double Value, string Unit);
 
-/// <summary>One timed top-level statement: its zero-based source position, the elapsed time as a
-/// structured duration, the kind of the value it produced, and whether it wrote print output.</summary>
+/// <summary>One timed top-level statement: its elapsed time as a structured duration, the kind of
+/// the value it produced, and whether it wrote print output. <c>Position</c> is the ZERO-BASED OFFSET
+/// OF THE STATEMENT IN THE CALLER'S SCRIPT � the text handed to <c>--eval</c>/<c>--file</c>/<c>--stdin</c>,
+/// whatever its line endings (<c>docs/symbolics/dsh-protocol.md:153-155</c>). The engine itself
+/// indexes into the semicolon-joined text it is handed; <see cref="ScriptPositions"/> translates, so a
+/// Windows (CRLF) script reports the offsets a consumer can slice the caller's text with.</summary>
 internal sealed record TimingDto(int Position, DurationDto Elapsed, string ResultKind, bool HasOutput);
 /// <summary>One entry of the builtin registry, carrying the arity metadata the call-site
 /// validator computes the arity contract from (A2-F15): <c>parameters</c> is the declared
@@ -38,6 +42,12 @@ internal sealed record FunctionDto(string Name, string[] Parameters, int MinArit
     bool Variadic, bool Builtin, string? Plugin);
 internal sealed record PlotDto(string Path, string Title, string Svg);
 internal sealed record ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured);
+/// <summary>One entry of the error envelope's <c>diagnostics</c> array: the parser's source-position
+/// form (<c>docs/symbolics/dsh-protocol.md:225-228</c>). <c>Position</c> is the zero-based offset of the
+/// failing place in the CALLER's script, and <c>Line</c>/<c>Column</c> are its 1-based line and column
+/// there (CRLF, CR and LF each end a line once). A runtime failure names the statement that was
+/// running when it threw; a lexer/parser refusal names the token the engine refused. All four fields
+/// are unchanged in NAME and COUNT from the version-1 contract.</summary>
 internal sealed record DiagnosticDto(string Message, int Position, int Line, int Column);
 
 /// <summary>The cancellation-budget ledger, published whenever <c>--cancel-after</c> was given (and
diff --git a/Lovelace.Run/Runner.cs b/Lovelace.Run/Runner.cs
index 94de88c..ee7274c 100644
--- a/Lovelace.Run/Runner.cs
+++ b/Lovelace.Run/Runner.cs
@@ -157,6 +157,12 @@ public static class Runner
             return Usage(stderr, "No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.");
         }
 
+        // The protocol's positions index into the text the CALLER supplied; the engine indexes
+        // into the semicolon-joined text it is handed. One map per run, built from the caller's own
+        // text, so every position the envelope publishes (timings, diagnostics, the overrun
+        // diagnostic) is translated by the same rule (see ScriptPositions).
+        var script = new ScriptPositions(source);
+
         var engine = new SuiteEngine();
         engine.LoadPlugin(new DspPlugin());
         var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
@@ -194,7 +200,7 @@ public static class Runner
             }
 
             var result = await engine.EvaluateAsync(
-                ScriptSource.ToSemicolonStatements(source), output, cancellation.Token);
+                script.EngineSource, output, cancellation.Token);
 
             var snapshot = engine.CaptureState();
             // one budget for every structured rendering in the envelope: the result AND each
@@ -242,10 +248,10 @@ public static class Runner
                 plot,
                 engine.LastElapsedDisplay,
                 Duration(engine.LastElapsed),
-                Timings(engine.OperationTimings),
+                Timings(engine.OperationTimings, script),
                 deadline,
                 deadline is { Exceeded: true }
-                    ? new[] { OverrunDiagnostic(deadline, source, engine.OperationTimings, completed: true) }
+                    ? new[] { OverrunDiagnostic(deadline, script, engine.OperationTimings, completed: true) }
                     : null);
 
             if (json)
@@ -258,7 +264,7 @@ public static class Runner
         catch (Exception ex)
         {
             var diagnostics = engine.Diagnostics
-                .Select(d => new DiagnosticDto(d.Message, d.Position, d.Line, d.Column))
+                .Select(d => PublishDiagnostic(script, d, engine.OperationTimings))
                 .ToArray();
             var (code, category, recoverable) = Classify(ex);
             // a run that blew its budget before failing must not look like one that respected it:
@@ -266,7 +272,7 @@ public static class Runner
             var failedDeadline = CancellationLedger(cancelAfterMs, engine.LastElapsed, stopped: code == "Cancelled");
             if (failedDeadline is { Exceeded: true })
                 diagnostics = diagnostics
-                    .Append(OverrunDiagnostic(failedDeadline, source, engine.OperationTimings, completed: false))
+                    .Append(OverrunDiagnostic(failedDeadline, script, engine.OperationTimings, completed: false))
                     .ToArray();
             // a cancelled run is not a failed run: the host reports the structured status together
             // with everything the engine had already committed
@@ -281,7 +287,7 @@ public static class Runner
             // a failed evaluation reports the SAME durations a successful one does: the elapsed pair
             // from one unit selector and one timing entry per statement that ran before the failure
             return WriteError(stdout, stderr, json, code, category, ex.Message, recoverable, diagnostics,
-                engine.LastElapsed, Timings(engine.OperationTimings), partialOutput, partialVariables, failedDeadline);
+                engine.LastElapsed, Timings(engine.OperationTimings, script), partialOutput, partialVariables, failedDeadline);
         }
     }
 
@@ -393,15 +399,14 @@ public static class Runner
     /// The diagnostic that names an exceeded deadline � published on the success envelope's
     /// <c>diagnostics</c> array and appended to the failure envelope's, so a consumer that reads only
     /// diagnostics still cannot mistake an overrun for a normal run. The position is the last
-    /// top-level statement that ran (the statement the deadline passed inside); the line/column rule
-    /// is the one the engine uses (<c>SuiteEngine.ComputeLineColumn</c>), kept here because the
-    /// runner owns the source text it was handed.
+    /// top-level statement that ran (the statement the deadline passed inside), translated into the
+    /// caller's source by the same map every other published position uses.
     /// </summary>
-    private static DiagnosticDto OverrunDiagnostic(CancellationDto ledger, string source,
+    private static DiagnosticDto OverrunDiagnostic(CancellationDto ledger, ScriptPositions script,
         IReadOnlyList<OperationTiming> timings, bool completed)
     {
-        int position = timings.Count > 0 ? timings[timings.Count - 1].Position : 0;
-        var (line, column) = ComputeLineColumn(source, position);
+        int enginePosition = timings.Count > 0 ? timings[timings.Count - 1].Position : 0;
+        var (position, line, column) = Locate(script, enginePosition);
         string outcome = ledger.Stopped
             ? "the deadline was observed, but only after the excess had already been spent"
             : completed
@@ -414,24 +419,33 @@ public static class Runner
             position, line, column);
     }
 
-    /// <summary>1-based line and column of a source offset, matching the engine's own rule.</summary>
-    private static (int Line, int Column) ComputeLineColumn(string source, int position)
+    /// <summary>
+    /// One engine diagnostic in the caller's coordinates. A lexer/parser refusal carries the only
+    /// position the engine ever knows by itself (scraped from its "at position N" message); a RUNTIME
+    /// failure carries none (<c>SuiteEngine.ToDiagnostic</c> defaults to 0), because the exception it
+    /// is built from is not annotated. But the interpreter records the offset of the statement that
+    /// was running when it threw, and the failing statement is always the LAST timing � its
+    /// <c>finally</c> adds the entry on the way out (<c>Lovelace.Suite/Interpreter.cs:333-341</c>) � so
+    /// whenever any statement ran, the last timing IS the failure's source position. Only a failure
+    /// before the first statement (a tokenizer/parser refusal) falls back to the engine's own value.
+    /// </summary>
+    private static DiagnosticDto PublishDiagnostic(ScriptPositions script, Diagnostic diagnostic,
+        IReadOnlyList<OperationTiming> timings)
     {
-        if (position < 0 || position > source.Length)
-            return (1, position + 1);
-
-        int line = 1;
-        int lastNewline = -1;
-        for (int i = 0; i < position && i < source.Length; i++)
-        {
-            if (source[i] == '\n')
-            {
-                line++;
-                lastNewline = i;
-            }
-        }
+        int enginePosition = timings.Count > 0
+            ? timings[timings.Count - 1].Position
+            : diagnostic.Position;
+        var (position, line, column) = Locate(script, enginePosition);
+        return new DiagnosticDto(diagnostic.Message, position, line, column);
+    }
 
-        return (line, position - lastNewline);
+    /// <summary>An engine offset as a caller offset together with its 1-based line and column: the
+    /// one translation every published position goes through.</summary>
+    private static (int Position, int Line, int Column) Locate(ScriptPositions script, int enginePosition)
+    {
+        int position = script.ToSourceOffset(enginePosition);
+        var (line, column) = script.LineColumn(position);
+        return (position, line, column);
     }
 
     /// <summary>A millisecond count as an invariant-culture string: the envelope must read the same
@@ -505,10 +519,13 @@ public static class Runner
         return new DurationDto(value, unit);
     }
 
-    /// <summary>One wire timing per top-level statement that ran, in statement order.</summary>
-    private static TimingDto[] Timings(IReadOnlyList<Lovelace.Suite.OperationTiming> timings) =>
+    /// <summary>One wire timing per top-level statement that ran, in statement order, each carrying
+    /// the statement's offset in the CALLER's source (the engine indexes into the semicolon-joined
+    /// text, which is not the caller's text for a CRLF or BOM-prefixed script � see ScriptPositions).</summary>
+    private static TimingDto[] Timings(IReadOnlyList<Lovelace.Suite.OperationTiming> timings,
+        ScriptPositions script) =>
         timings.Select(t => new TimingDto(
-            t.Position,
+            script.ToSourceOffset(t.Position),
             new DurationDto(t.ElapsedScale.Value, t.ElapsedScale.Unit),
             t.Result.Kind.ToString(),
             t.Output.Length > 0))

===== new file: Lovelace.Run/ScriptPositions.cs =====
diff --git "a/C:\\Users\\ricar\\AppData\\Local\\Temp\\c6pos\\empty.txt" "b/Lovelace.Run\\ScriptPositions.cs"
index e69de29..d197f91 100644
--- "a/C:\\Users\\ricar\\AppData\\Local\\Temp\\c6pos\\empty.txt"
+++ "b/Lovelace.Run\\ScriptPositions.cs"
@@ -0,0 +1,118 @@
+using Lovelace.Suite;
+
+namespace Lovelace.Run;
+
+/// <summary>
+/// The script text a CALLER supplied, together with the map back to it from the text the ENGINE is
+/// handed (<see cref="Lovelace.Suite.ScriptSource.ToSemicolonStatements"/>).
+///
+/// Every source position this runner publishes � <c>timings[].position</c> and the error envelope's
+/// <c>diagnostics[].position</c>/<c>line</c>/<c>column</c> � is defined by the protocol against the
+/// text the CALLER supplied ("the zero-based source offset of the statement",
+/// <c>docs/symbolics/dsh-protocol.md:153-155</c>; the diagnostics array is "the parser's
+/// source-position form", <c>:225-228</c>). The engine, however, indexes into the REWRITTEN text,
+/// and that rewrite is not a bijection for two inputs:
+///
+/// <list type="bullet">
+/// <item>a CRLF pair is collapsed to one <c>\n</c> before the (length-preserving) newline-to-<c>;</c>
+/// pass (<c>Lovelace.Suite/ScriptSource.cs:27</c>), so every offset after a Windows line break is
+/// short by one per preceding break;</item>
+/// <item>a leading U+FEFF is stripped (<c>:31-32</c>), shifting the first character of the caller's
+/// text by one.</item>
+/// </list>
+///
+/// The newline-to-<c>;</c> pass itself IS length-preserving (one character in, one character out, in
+/// order � <c>ScriptSource.cs:39-98</c>), so the two cases above are the whole map: character
+/// <c>i</c> of the engine's text is character <c>_sourceIndex[i]</c> of the caller's text. Both
+/// <c>line</c>/<c>column</c> are then recomputed from the caller's text, so a failure on source
+/// line 3 reports line 3 (the engine's own computation runs on the semicolon-joined text, which has
+/// no top-level newlines at all).
+/// </summary>
+internal sealed class ScriptPositions
+{
+    /// <summary>Maps <paramref name="source"/> (the caller's text, exactly as received) and builds
+    /// the index of the engine's text once, so every position in one envelope is mapped by the same
+    /// rule.</summary>
+    internal ScriptPositions(string source)
+    {
+        Source = source;
+        EngineSource = ScriptSource.ToSemicolonStatements(source);
+        _sourceIndex = BuildSourceIndex(source, EngineSource.Length);
+    }
+
+    /// <summary>The text the caller supplied � the text every published position indexes into.</summary>
+    internal string Source { get; }
+
+    /// <summary>The text the engine is evaluated on (the semicolon-joined form).</summary>
+    internal string EngineSource { get; }
+
+    /// <summary>The offset in <see cref="Source"/> that <paramref name="engineOffset"/> (an offset in
+    /// <see cref="EngineSource"/>) names. Negative offsets clamp to the start; an offset at or past
+    /// the end of the engine's text clamps to the end of the caller's text (a lexer that ran off the
+    /// end reports the end).</summary>
+    internal int ToSourceOffset(int engineOffset)
+    {
+        if (engineOffset < 0)
+            return 0;
+        if (engineOffset < _sourceIndex.Length)
+            return _sourceIndex[engineOffset];
+        return _sourceIndex.Length > 0 ? Source.Length : 0;
+    }
+
+    /// <summary>1-based line and column of a SOURCE offset, read off the caller's own text. Line
+    /// breaks are CRLF, CR and LF, each counting once � so a Windows script and a classic-Mac script
+    /// report the same line and column as the Unix one.</summary>
+    internal (int Line, int Column) LineColumn(int sourceOffset) =>
+        ComputeLineColumn(Source, sourceOffset);
+
+    /// <summary>Character <c>i</c> of the engine's text was produced from character
+    /// <c>_sourceIndex[i]</c> of the caller's text. A CRLF pair is the only many-to-one step (the
+    /// pair maps to the single <c>\n</c> at the LF's index); the only skipped character is a leading
+    /// U+FEFF.</summary>
+    private readonly int[] _sourceIndex;
+
+    private static int[] BuildSourceIndex(string source, int engineLength)
+    {
+        var index = new int[engineLength];
+        int s = source.Length > 0 && source[0] == '\uFEFF' ? 1 : 0;
+        for (int e = 0; e < engineLength; e++)
+        {
+            if (s + 1 < source.Length && source[s] == '\r' && source[s + 1] == '\n')
+                s++;    // the CR half of the pair: the engine's one character belongs to the LF
+            index[e] = s;
+            s++;
+        }
+        return index;
+    }
+
+    /// <summary>1-based line and column of an offset, the rule the engine applies to its own
+    /// (rewritten) text and this host applies to the caller's: CR, LF and CRLF each end a line
+    /// exactly once.</summary>
+    private static (int Line, int Column) ComputeLineColumn(string source, int position)
+    {
+        if (position < 0 || position > source.Length)
+            return (1, position + 1);
+
+        int line = 1;
+        int lineStart = 0;
+        for (int i = 0; i < position && i < source.Length; i++)
+        {
+            char c = source[i];
+            if (c == '\n')
+            {
+                // the LF half of a CRLF: the break was counted when the CR was read
+                if (i > 0 && source[i - 1] == '\r')
+                    continue;
+                line++;
+                lineStart = i + 1;
+            }
+            else if (c == '\r')
+            {
+                line++;
+                lineStart = i + ((i + 1 < source.Length && source[i + 1] == '\n') ? 2 : 1);
+            }
+        }
+
+        return (line, Math.Max(1, position - lineStart + 1));
+    }
+}

===== new file: Lovelace.Run.Tests/SourcePositionTests.cs =====
diff --git "a/C:\\Users\\ricar\\AppData\\Local\\Temp\\c6pos\\empty.txt" "b/Lovelace.Run.Tests\\SourcePositionTests.cs"
index e69de29..dcf7fea 100644
--- "a/C:\\Users\\ricar\\AppData\\Local\\Temp\\c6pos\\empty.txt"
+++ "b/Lovelace.Run.Tests\\SourcePositionTests.cs"
@@ -0,0 +1,322 @@
+using System.Text;
+using System.Text.Json.Nodes;
+using Xunit;
+
+namespace Lovelace.Run.Tests;
+
+/// <summary>
+/// The machine API's source positions must name the real place in the CALLER's script.
+///
+/// docs/symbolics/dsh-protocol.md:153-155 � "`timings[].position` is the zero-based source offset of the
+/// statement" � and :225-228 � the error envelope's `diagnostics` array is the parser's
+/// source-position form (`message`/`position`/`line`/`column`). Both are defined against the text
+/// the caller supplied, whatever its line endings.
+///
+/// Three live P1s (round-18 audit E, findings F1/F2/F3; amendment P.2 rows E-P1a/b/c) sit behind
+/// these assertions:
+///
+///  * a RUNTIME failure reported `0/1/1` whatever statement failed, although the engine's own timing
+///    ledger carries the failing statement's offset;
+///  * `line`/`column` computed on the runner's semicolon-joined text (which has no top-level
+///    newlines at all), so every failure on source line 3 reported line 1;
+///  * `timings[].position` short by one character per CRLF line break, because the runner's
+///    normalization collapses CRLF to LF before the length-preserving rewrite
+///    (`Lovelace.Suite/ScriptSource.cs:27`).
+///
+/// Every expected offset below is computed by the TEST from the exact text (or the exact bytes) the
+/// test handed to the runner � never read back out of the envelope � and the self-checks pin the
+/// literals so a fixture drift cannot silently move the expectation.
+/// </summary>
+public class SourcePositionTests
+{
+    // -----------------------------------------------------------------
+    // F1 � a runtime failure in the third statement
+    // -----------------------------------------------------------------
+
+    /// <summary>The failing call is the third statement, at offset 12 of the text the caller passed.
+    /// The envelope must name THAT statement: position 12, line 3, column 1.</summary>
+    [Fact]
+    public async Task RuntimeFailureInTheThirdStatement_NamesThatStatement_Lf()
+    {
+        string source = "a = 1\nb = 2\ndet(1)";
+        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
+        Assert.Equal(12, failing);   // self-check: the fixture is the one the audit measured
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(failing, DiagnosticPosition(envelope));
+        Assert.Equal(3, DiagnosticLine(envelope));
+        Assert.Equal(1, DiagnosticColumn(envelope));
+        // all three statements ran; the LAST one is the one that failed
+        Assert.Equal(new[] { 0, 6, 12 }, TimingsPositions(envelope));
+        Assert.Equal(failing, TimingsPositions(envelope)[^1]);
+
+        // the diagnostic and the timing ledger of the SAME envelope agree on the place
+        Assert.Equal(TimingsPositions(envelope)[^1], DiagnosticPosition(envelope));
+    }
+
+    /// <summary>The same script with Windows line endings: the caller's offsets are 0, 6, 12.</summary>
+    [Fact]
+    public async Task RuntimeFailureInTheThirdStatement_NamesThatStatement_Crlf()
+    {
+        string source = "a = 1\r\nb = 2\r\ndet(1)";
+        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
+        Assert.Equal(14, failing);   // 5 + CRLF + 5 + CRLF: the CRLF source is two characters longer
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(failing, DiagnosticPosition(envelope));
+        Assert.Equal(3, DiagnosticLine(envelope));
+        Assert.Equal(1, DiagnosticColumn(envelope));
+        Assert.Equal(new[] { 0, 7, 14 }, TimingsPositions(envelope));
+        Assert.Equal(failing, TimingsPositions(envelope)[^1]);
+    }
+
+    // -----------------------------------------------------------------
+    // F2 � a parse failure on the third SOURCE LINE
+    // -----------------------------------------------------------------
+
+    /// <summary>A lexer/parser refusal is the one diagnostic that carries an engine position of its
+    /// own (scraped from "at position N"). It must be reported as a place in the caller's script: the
+    /// offending `)` is at offset 10, on line 3, column 1.</summary>
+    [Fact]
+    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_Lf()
+    {
+        string source = "1+1;\n2+2;\n)";
+        int offending = source.LastIndexOf(')');
+        Assert.Equal(10, offending);
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(offending, DiagnosticPosition(envelope));
+        Assert.Equal(3, DiagnosticLine(envelope));
+        Assert.Equal(1, DiagnosticColumn(envelope));
+        // nothing was parsed, so no statement ran
+        Assert.Empty(envelope["timings"]!.AsArray());
+    }
+
+    /// <summary>The same refusal through `--file`, with the CRLF bytes the test writes: the offending
+    /// byte is the 13th, at offset 12, on line 3, column 1.</summary>
+    [Fact]
+    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrlfFile()
+    {
+        byte[] bytes = Encoding.ASCII.GetBytes("1+1;\r\n2+2;\r\n)");
+        string path = Path.Combine(Path.GetTempPath(), "lovelace-positions-" + Guid.NewGuid().ToString("N") + ".ls");
+        await File.WriteAllBytesAsync(path, bytes);
+        try
+        {
+            byte[] onDisk = await File.ReadAllBytesAsync(path);
+            int offending = Array.IndexOf(onDisk, (byte)')');
+            string text = Encoding.ASCII.GetString(onDisk);
+            var (expectedLine, expectedColumn) = Locate(text, offending);
+            Assert.Equal(12, offending);        // 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29
+            Assert.Equal((3, 1), (expectedLine, expectedColumn));
+
+            var (exitCode, envelope, _) = await RunAsync("--file", path, "--omit-functions");
+
+            Assert.Equal(1, exitCode);
+            Assert.Equal(offending, DiagnosticPosition(envelope));
+            Assert.Equal(expectedLine, DiagnosticLine(envelope));
+            Assert.Equal(expectedColumn, DiagnosticColumn(envelope));
+            Assert.Empty(envelope["timings"]!.AsArray());
+        }
+        finally
+        {
+            File.Delete(path);
+        }
+    }
+
+    // -----------------------------------------------------------------
+    // F3 � timings[].position is a caller offset for EVERY line ending
+    // -----------------------------------------------------------------
+
+    /// <summary>Three statements with CRLF separators start at the caller's offsets 0, 6 and 12.</summary>
+    [Fact]
+    public async Task TimingsPositions_AreCallerOffsets_Crlf()
+    {
+        string source = "1+1;\r\n2+2;\r\n3+3";
+        int[] expected =
+        [
+            source.IndexOf("1+1", StringComparison.Ordinal),
+            source.IndexOf("2+2", StringComparison.Ordinal),
+            source.IndexOf("3+3", StringComparison.Ordinal),
+        ];
+        Assert.Equal(new[] { 0, 6, 12 }, expected);
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(0, exitCode);
+        Assert.Equal(expected, TimingsPositions(envelope));
+    }
+
+    /// <summary>No regression: LF and CR sources were already correct and keep their offsets.</summary>
+    [Theory]
+    [InlineData("1+1;\n2+2;\n3+3")]
+    [InlineData("1+1;\r2+2;\r3+3")]
+    public async Task TimingsPositions_AreCallerOffsets_LfAndCr(string source)
+    {
+        int[] expected =
+        [
+            source.IndexOf("1+1", StringComparison.Ordinal),
+            source.IndexOf("2+2", StringComparison.Ordinal),
+            source.IndexOf("3+3", StringComparison.Ordinal),
+        ];
+        Assert.Equal(new[] { 0, 5, 10 }, expected);
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(0, exitCode);
+        Assert.Equal(expected, TimingsPositions(envelope));
+    }
+
+    /// <summary>A CR-only source reports its parse failure at the caller's place too: offset 10,
+    /// line 3, column 1.</summary>
+    [Fact]
+    public async Task ParseFailureOnTheThirdSourceLine_ReportsThatLine_CrOnly()
+    {
+        string source = "1+1;\r2+2;\r)";
+        int offending = source.LastIndexOf(')');
+        var (expectedLine, expectedColumn) = Locate(source, offending);
+        Assert.Equal(10, offending);
+        Assert.Equal((3, 1), (expectedLine, expectedColumn));
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(offending, DiagnosticPosition(envelope));
+        Assert.Equal(expectedLine, DiagnosticLine(envelope));
+        Assert.Equal(expectedColumn, DiagnosticColumn(envelope));
+    }
+
+    // -----------------------------------------------------------------
+    // No regression: the single-statement and single-line cases
+    // -----------------------------------------------------------------
+
+    /// <summary>A ONE-statement script that fails really is at offset 0: line 1, column 1.</summary>
+    [Fact]
+    public async Task SingleStatementFailure_KeepsPositionZeroLineOneColumnOne()
+    {
+        var (exitCode, envelope, _) = await RunAsync("--eval", "det(1)");
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(0, DiagnosticPosition(envelope));
+        Assert.Equal(1, DiagnosticLine(envelope));
+        Assert.Equal(1, DiagnosticColumn(envelope));
+        Assert.Equal(new[] { 0 }, TimingsPositions(envelope));
+    }
+
+    /// <summary>A failure in the SECOND statement of a single-line script: offset 5, line 1,
+    /// column 6 � the value the old envelope reported as 0/1/1.</summary>
+    [Fact]
+    public async Task FailureInTheSecondStatementOfOneLine_NamesThatStatement()
+    {
+        string source = "1+1; det(1)";
+        int failing = source.IndexOf("det(1)", StringComparison.Ordinal);
+        Assert.Equal(5, failing);
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(failing, DiagnosticPosition(envelope));
+        Assert.Equal(1, DiagnosticLine(envelope));
+        Assert.Equal(6, DiagnosticColumn(envelope));
+    }
+
+    /// <summary>A leading byte-order mark is part of the text the caller supplied, so the positions
+    /// index into it: the engine's first statement (offset 0 of the text the engine sees, which drops
+    /// the BOM) is at offset 1 of the caller's argument, and a consumer slicing the argument gets the
+    /// statement back.</summary>
+    [Fact]
+    public async Task LeadingBom_PositionsStayOffsetsIntoTheCallersText()
+    {
+        string source = "\uFEFFdet(1)";
+        Assert.Equal(1, source.IndexOf("det(1)", StringComparison.Ordinal));
+
+        var (exitCode, envelope, _) = await RunAsync("--eval", source);
+
+        Assert.Equal(1, exitCode);
+        Assert.Equal(1, DiagnosticPosition(envelope));
+        Assert.Equal(1, DiagnosticLine(envelope));
+        Assert.Equal(2, DiagnosticColumn(envelope));
+        Assert.Equal(new[] { 1 }, TimingsPositions(envelope));
+    }
+
+    /// <summary>The diagnostics DTO shape is unchanged: four fields, no more.</summary>
+    [Fact]
+    public async Task DiagnosticEntry_KeepsItsFourFieldShape()
+    {
+        var (_, envelope, _) = await RunAsync("--eval", "a = 1\nb = 2\ndet(1)");
+
+        JsonObject diagnostic = envelope["diagnostics"]!.AsArray()[0]!.AsObject();
+        Assert.Equal(
+            new[] { "column", "line", "message", "position" },
+            diagnostic.Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
+        Assert.Equal(
+            new[]
+            {
+                "category", "code", "diagnostics", "elapsed", "elapsedTime", "mathIrVersion", "message",
+                "ok", "protocolVersion", "recoverable", "symbolicFormatVersion", "timings",
+            },
+            envelope.AsObject().Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray());
+    }
+
+    // -----------------------------------------------------------------
+    // Plumbing
+    // -----------------------------------------------------------------
+
+    private static async Task<(int ExitCode, JsonNode Envelope, string Raw)> RunAsync(params string[] arguments)
+    {
+        var stdout = new StringWriter();
+        var stderr = new StringWriter();
+        int exitCode = await Runner.RunAsync(arguments, stdout, stderr);
+        string raw = stdout.ToString();
+        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(raw, "the envelope"), raw);
+    }
+
+    private static int DiagnosticPosition(JsonNode envelope) =>
+        envelope["diagnostics"]!.AsArray()[0]!["position"]!.GetValue<int>();
+
+    private static int DiagnosticLine(JsonNode envelope) =>
+        envelope["diagnostics"]!.AsArray()[0]!["line"]!.GetValue<int>();
+
+    private static int DiagnosticColumn(JsonNode envelope) =>
+        envelope["diagnostics"]!.AsArray()[0]!["column"]!.GetValue<int>();
+
+    private static int[] TimingsPositions(JsonNode envelope) =>
+        envelope["timings"]!.AsArray().Select(t => t!["position"]!.GetValue<int>()).ToArray();
+
+    /// <summary>
+    /// The test's own oracle for "1-based line and column of an offset": walk the caller's text
+    /// counting characters, treating CRLF as ONE line break and a lone CR or LF as one each � the
+    /// rule the protocol's line/column fields are read under.
+    /// </summary>
+    private static (int Line, int Column) Locate(string source, int offset)
+    {
+        int line = 1;
+        int column = 1;
+        for (int i = 0; i < offset;)
+        {
+            if (source[i] == '\r' && i + 1 < source.Length && source[i + 1] == '\n')
+            {
+                line++;
+                column = 1;
+                i += 2;
+            }
+            else if (source[i] == '\r' || source[i] == '\n')
+            {
+                line++;
+                column = 1;
+                i++;
+            }
+            else
+            {
+                column++;
+                i++;
+            }
+        }
+        return (line, column);
+    }
+}
