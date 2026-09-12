# F1-C (P0) — a --plot-dir the process cannot create no longer aborts the runner

**Round 15 / cycle 6 — implementation record. Tree used: a scratch worktree, `.worktrees/c6-p0` (detached HEAD `9a671c0`), created with `git worktree add .worktrees/c6-p0 HEAD`. Nothing was staged, committed or pushed. The main checkout `C:\Users\ricar\dev\LovelaceSharp` received no source edit — the fix lives UNCOMMITTED in that worktree (`git status --short` there: ` M Lovelace.Run/Runner.cs`, `?? Lovelace.Run.Tests/PlotDirectoryErrorTests.cs`). Only this document and the logs next to it were written into the main tree.**

Finding: `docs/goal-cycle-6/round-13/audit-C-hostile.md:19-78` (F1, P0, NEW) and `docs/goal-cycle-6/evidence.md:54` (EVD-282).

## 0. What was wrong

`Directory.CreateDirectory(engine.PlotOutputDirectory)` sat at `Lovelace.Run/Runner.cs:168`, **outside** the `try` that started at `:177`, and `Program.cs:24` caught nothing. A caller-supplied `--plot-dir` the OS refuses to create therefore escaped as an unhandled exception: the process died and stdout stayed empty, so neither a code nor a category ever reached the caller — the exact opposite of the protocol document's own contract (`docs/symbolics/dsh-protocol.md:9-10`, `:207-209`; repeated at `Lovelace.Run/Program.cs:18` and `Lovelace.Run/Runner.cs:27`).

## 1. Pre-fix observation (re-observed by me, not copied)

Binary: the **published** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` (Native AOT, 5 776 896 B, 2026-09-12 15:06:37). Harness: `%TEMP%\p0probe2.ps1`, a `.NET` `ProcessStartInfo` runner so the **exit code** and the raw **stdout/stderr byte counts** are measured off the pipes; PowerShell 5.1 silently drops an empty argument to a native command, so the `""` row is passed through the process API (the same caveat audit C recorded at `audit-C-hostile.md:51-52`). Full transcript: `docs/goal-cycle-6/round-15/pre-probes.txt`.

| probe (`--eval 1 --json --omit-functions --omit-variables` + the value) | exit | stdout | stderr | stderr headline |
|---|---|---|---|---|
| control, no `--plot-dir` | 0 | **451 B** | 0 B | — (envelope) |
| `--plot-dir C:\Windows\notepad.exe` | **-1073740791** (0xC0000409) | **0 B** | 527 B | `Unhandled exception. System.IO.IOException: Cannot create 'C:\Windows\notepad.exe' …` |
| `--plot-dir ""` | **-1073740791** | **0 B** | 497 B | `Unhandled exception. System.ArgumentException: The value cannot be an empty string. (Parameter 'path')` |
| `--plot-dir …\bad<dir` | **-1073740791** | **0 B** | 581 B | `Unhandled exception. System.IO.IOException: The filename, directory name, or volume label syntax is incorrect. …` |
| `--plot-dir <fresh temp dir>` with `--eval plot([1,2,3])` | 0 | 17 186 B | 0 B | — (envelope; `plot.svg: 14 636 B` on disk) |

Every crashing stack names the same frame:

~~~
   at System.IO.FileSystem.CreateDirectory(String, Byte[]) + 0x448
   at System.IO.Directory.CreateDirectory(String) + 0x2a
   at Lovelace.Run.Runner.<RunAsync>d__2.MoveNext() + 0x70d
~~~

## 2. The failing test, written FIRST

New file `Lovelace.Run.Tests/PlotDirectoryErrorTests.cs` (in the worktree). It spawns the **real runner** the way the sibling `StdoutPurityTests` does (`dotnet exec <baseDir>/Lovelace.Run.dll …`) because only a process can show an abort and an empty stdout; it reads `StandardOutput.BaseStream` so the byte count is the finding's own measurement. Four tests:

* `PlotDirectoryNamingAnExistingFile_IsAnErrorEnvelopeWithExitOne`
* `EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne`
* `PlotDirectoryWithInvalidCharacters_IsAnErrorEnvelopeWithExitOne`
* `ValidPlotDirectory_StillReceivesThePlotFile` (the success-path regression guard)

Each refusal asserts, structurally: stdout bytes **> 0**; stderr free of `Unhandled exception`; exit **1**; exactly one JSON document; `protocolVersion == 1`; `ok == false`; `code == "PlotDirectoryError"`; `category == "TypeMismatch"` **and** a member of the one taxonomy (`Lovelace.Abstractions/Diagnostic.cs:12-21`); `recoverable == true`; the offending value named in `message`; and `diagnostics`/`elapsed`/`elapsedTime{value,unit}`/`timings` present, because invariant 4 is not scoped to success (`dsh-protocol.md:202-205`).

### Observed failure BEFORE the fix

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0
dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~PlotDirectoryErrorTests"
```

```
  Failed Lovelace.Run.Tests.PlotDirectoryErrorTests.PlotDirectoryNamingAnExistingFile_IsAnErrorEnvelopeWithExitOne [2 s]
  Error Message:
   stdout must never be empty. --plot-dir 'C:\Users\ricar\AppData\Local\Temp\lovelace-plotdir-existing-file-22c6d7efd8f643c6b590b57d8de8b387.txt': exit -532462766, stdout 0 B, stderr 722 B; stderr: Unhandled exception. System.IO.IOException: Cannot create '…' because a file or directory with the same name already exists.
   at System.IO.FileSystem.CreateDirectory(String fullPath, Byte[] securityDescriptor)
   at System.IO.Directory.CreateDirectory(String path)
   at Lovelace.Run.Runner.RunA...
  Stack Trace:
     at Lovelace.Run.Tests.PlotDirectoryErrorTests.AssertPlotDirectoryRefusedAsync(String plotDirectory) in …\Lovelace.Run.Tests\PlotDirectoryErrorTests.cs:line 135
  Failed Lovelace.Run.Tests.PlotDirectoryErrorTests.EmptyPlotDirectory_IsAnErrorEnvelopeWithExitOne [2 s]
  Error Message:
   stdout must never be empty. --plot-dir '': exit -532462766, stdout 0 B, stderr 605 B; stderr: Unhandled exception. System.ArgumentException: The value cannot be an empty string. (Parameter 'path')
  Failed Lovelace.Run.Tests.PlotDirectoryErrorTests.PlotDirectoryWithInvalidCharacters_IsAnErrorEnvelopeWithExitOne [2 s]
  Error Message:
   stdout must never be empty. --plot-dir 'C:\Users\ricar\AppData\Local\Temp\lovelace-plotdir-invalid-379bc211fd544ee5a62a90e343239bb7<bad>|?*': exit -532462766, stdout 0 B, stderr 713 B; stderr: Unhandled exception. System.IO.IOException: The filename, directory name, or volume label syntax is incorrect. …

Failed!  - Failed:     3, Passed:     1, Skipped:     0, Total:     4, Duration: 7 s - Lovelace.Run.Tests.dll (net10.0)
DOTNET_EXIT=1
```

Full log: `docs/goal-cycle-6/round-15/prefix-run.log` (exit code `1`). Note the honest difference from §1: through `dotnet exec` the managed unhandled exception is mapped to `-532462766` (`0xE0434352`) instead of the AOT fail-fast `0xC0000409` — a different number, the same defect, and **0 bytes on stdout** in both hosts. The fourth test (the success path) passed before the fix, as it must: it is a regression guard, not a claim about new behaviour.

## 3. The fix

`Lovelace.Run/Runner.cs` only (37 insertions, 4 deletions). The `CreateDirectory` call moved **inside** the guarded region (now `Runner.cs:181-192`) behind a narrow filter; anything outside the filter still propagates to the general handler (`Runner.cs:256`), which answers with an envelope instead of aborting.

```diff
diff --git a/Lovelace.Run/Runner.cs b/Lovelace.Run/Runner.cs
index 1c4e100..7c531a3 100644
--- a/Lovelace.Run/Runner.cs
+++ b/Lovelace.Run/Runner.cs
@@ -163,10 +163,6 @@ public static class Runner
         if (plotDir is not null) engine.PlotOutputDirectory = plotDir;
         if (plotFile is not null) engine.PlotFileName = plotFile;
 
-        // plot() writes into PlotOutputDirectory without creating it, so ensure a
-        // fresh --plot-dir works (a no-op when the directory already exists).
-        Directory.CreateDirectory(engine.PlotOutputDirectory);
-
         // capture print() output: the envelope must be the only thing on stdout. Declared outside the
         // try so a cancelled evaluation can still return the output it produced (the partial result).
         var output = new StringWriter();
@@ -176,6 +172,25 @@ public static class Runner
 
         try
         {
+            // plot() writes into PlotOutputDirectory without creating it, so ensure a fresh --plot-dir
+            // works (a no-op when the directory already exists). The preflight runs INSIDE the guarded
+            // region on purpose: a directory the caller named but the process cannot create is a
+            // caller-level failure, and it must cross as an error envelope with a code and a category —
+            // never as an unhandled exception with an empty stdout. This call used to sit BEFORE the
+            // try and aborted the process (F1-C: exit 0xC0000409, 0 bytes on stdout, no envelope).
+            try
+            {
+                Directory.CreateDirectory(engine.PlotOutputDirectory);
+            }
+            catch (Exception ex) when (IsUnusablePlotDirectory(ex))
+            {
+                // no engine ran and no statement executed, so the durations are zero and empty — the
+                // same shape the unreadable-script-file path publishes
+                return WriteError(stdout, stderr, json, "PlotDirectoryError", "TypeMismatch",
+                    $"Cannot use plot directory '{engine.PlotOutputDirectory}': {ex.Message}",
+                    recoverable: true, Array.Empty<DiagnosticDto>(), TimeSpan.Zero, Array.Empty<TimingDto>());
+            }
+
             var result = await engine.EvaluateAsync(
                 ScriptSource.ToSemicolonStatements(source), output, cancellation.Token);
 
@@ -268,6 +283,24 @@ public static class Runner
         }
     }
 
+    /// <summary>
+    /// The failures a caller-supplied plot directory can produce: an empty or malformed path
+    /// (<see cref="ArgumentException"/> / <see cref="NotSupportedException"/>), a path that names an
+    /// existing FILE or a volume that cannot be reached (<see cref="IOException"/>, which also covers
+    /// <see cref="PathTooLongException"/>), and a missing permission
+    /// (<see cref="UnauthorizedAccessException"/>). Each is a property of the ARGUMENT, not of the
+    /// engine, so each crosses as the caller-level <c>PlotDirectoryError</c>/<c>TypeMismatch</c>
+    /// failure instead of being reported as an internal invariant failure (F1-C).
+    /// <para>
+    /// The filter is deliberately narrow and never swallows the diagnostic: the exception object is
+    /// what the envelope's message is built from, and any exception outside this set keeps
+    /// propagating to the general handler below, which still answers with an envelope (code
+    /// <c>InternalError</c>) rather than aborting the process or leaving stdout empty.
+    /// </para>
+    /// </summary>
+    private static bool IsUnusablePlotDirectory(Exception ex) =>
+        ex is ArgumentException or IOException or UnauthorizedAccessException or NotSupportedException;
+
     /// <summary>Maps a failure onto the stable error taxonomy. Message matching is never required
     /// by a consumer: the code and category are structural.</summary>
     private static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
```

**Design decisions, stated so they can be argued.**

* **Envelope with exit 1, not usage exit 2.** The brief accepts either. The envelope is chosen because it keeps stdout non-empty (the other half of the finding), because it is what the sibling host-level failure already does (`FileReadError`, `Runner.cs:146`), and because a uniform answer for all four values is one contract instead of two. The *missing* argument still takes the usage path: exit 2, `Error: --plot-dir requires a directory argument.` (measured, §4).
* **Code `PlotDirectoryError`, category `TypeMismatch`.** `TypeMismatch` is the taxonomy member this file already uses for a caller argument that cannot be honoured (`ArgumentException => ("InvalidArgument", "TypeMismatch")`, `Runner.cs:322`), and all four values are "a directory of the right kind was required and this is not one". The code is new and stable; consumers switch on code+category, never on the message.
* **Narrow filter, diagnostic preserved.** `Directory.CreateDirectory`'s caller-caused failures are exactly `ArgumentException`/`IOException`/`UnauthorizedAccessException`/`NotSupportedException`. Nothing else is intercepted, so a genuine internal fault is still classified by `Classify` — but it, too, now crosses as an envelope, because the preflight sits inside the guarded region.

## 4. After the fix — same probes, freshly published Native AOT binary

Published from the worktree with the Makefile's own flags (`dotnet publish Lovelace.Run/Lovelace.Run.csproj --configuration Release --framework net10.0 -p:PublishAot=true -p:InvariantGlobalization=true --output out\aot-fix`, `PUBLISH_EXIT=0`, log `aot-publish.log`) → `.worktrees\c6-p0\out\aot-fix\Lovelace.Run.exe` (5 794 816 B). Transcript: `after-probes.txt`.

| `--plot-dir` value | before: exit / stdout / stderr | after: exit / stdout / stderr | after: stdout content |
|---|---|---|---|
| `C:\Windows\notepad.exe` (existing file) | -1073740791 / **0 B** / 527 B unhandled | **1** / **443 B** / 0 B | `{"ok":false,"code":"PlotDirectoryError","category":"TypeMismatch","message":"Cannot use plot directory 'C:\Windows\notepad.exe': Cannot create … already exists.","recoverable":true,"diagnostics":[],"elapsed":"0 ns","elapsedTime":{"value":0,"unit":"ns"},"timings":[]}` |
| `""` (empty string) | -1073740791 / **0 B** / 497 B unhandled | **1** / **371 B** / 0 B | same shape, message `Cannot use plot directory '': The value cannot be an empty string. (Parameter 'path')` |
| `…\bad<dir` (invalid characters) | -1073740791 / **0 B** / 581 B unhandled | **1** / **576 B** / 0 B | same shape, message `… The filename, directory name, or volume label syntax is incorrect. …` |
| **success path**: fresh `…\valid-plotdir` + `plot([1,2,3])` | 0 / 17 186 B / 0 B, `plot.svg` 14 636 B | 0 / **17 182 B** / 0 B, `plot.svg` **14 636 B** | envelope `ok:true`, `result.display` = the SVG path; the file is on disk |
| control, no `--plot-dir` | 0 / 451 B / 0 B | 0 / 454 B / 0 B | unchanged success envelope |

Two boundary probes on the same fixed binary:

| probe | exit | stdout | stderr |
|---|---|---|---|
| `--eval 1 --json --plot-dir` (no value) | **2** | 0 B | 1 118 B, `Error: --plot-dir requires a directory argument.` + usage — the documented usage path, untouched |
| `--help` | 0 | 1 068 B | 0 B |

Post-fix test run (same command as §2, after the fix):

```
Passed!  - Failed:     0, Passed:     4, Skipped:     0, Total:     4, Duration: 587 ms - Lovelace.Run.Tests.dll (net10.0)
DOTNET_EXIT=0
```

Log: `postfix-run.log`. The success-path test is not satisfied by an exit code: it also requires the directory to be created **recursively**, the SVG file to exist at `<dir>\plot.svg` with `<svg` content, and `envelope.plot.path` to equal that full path — so a fix that merely swallowed the exception and skipped the plot would fail it.

## 5. CONTROL — the same tests against pristine HEAD

Pristine tree `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-p0ctl` (`git worktree add .worktrees/c6-p0ctl HEAD`, detached HEAD `9a671c0`). **Only the one new test file was copied in** — `git status --short` there shows exactly `?? Lovelace.Run.Tests/PlotDirectoryErrorTests.cs` and no source change.

```
Failed!  - Failed:     3, Passed:     1, Skipped:     0, Total:     4, Duration: 7 s - Lovelace.Run.Tests.dll (net10.0)
DOTNET_EXIT=1
```

The three refusal tests fail there with the finding's signature — `stdout must never be empty … exit -532462766, stdout 0 B` — and the success-path guard passes, exactly as it did pre-fix in my own tree (§2). Log: `control-run.log`.

## 6. Suite totals (worktree `.worktrees/c6-p0`, fix applied, uncommitted)

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` → `DOTNET_EXIT=0`, **0 failed**. Per project (`solution-run.log`):

| project | failed | passed | skipped | total |
|---|---|---|---|---|
| Lovelace.Representation.Tests | 0 | 91 | 0 | 91 |
| Lovelace.Abstractions.Tests | 0 | 20 | 0 | 20 |
| Lovelace.Array.Tests | 0 | 19 | 0 | 19 |
| Lovelace.Knowledge.Tests | 0 | 28 | 0 | 28 |
| Lovelace.Natural.Tests | 0 | 195 | 0 | 195 |
| Lovelace.Integer.Tests | 0 | 148 | 0 | 148 |
| precbench.Tests | 0 | 13 | 0 | 13 |
| Lovelace.Console.Tests | 0 | 15 | 0 | 15 |
| Lovelace.Complex.Tests | 0 | 115 | 0 | 115 |
| Lovelace.Real.Tests | 0 | 2 495 | 0 | 2 495 |
| Lovelace.Dsp.Tests | 0 | 61 | 0 | 61 |
| Lovelace.Suite.Tests | 0 | 828 | 0 | 828 |
| Lovelace.Symbolics.Tests | 0 | 1 119 | 8 | 1 127 |
| **Lovelace.Run.Tests** | **0** | **247** | 0 | 247 |
| Lovelace.Studio.Tests | 0 | 22 | 0 | 22 |
| **total** | **0** | **5 416** | **8** | **5 424** |

Timing subsets (`--filter "Category=Timing"`, `timing-subsets.log`) — identical to the EVD-273 baseline: Run **5/0**, Suite **8/0**, Symbolics **1/0**, Real **1/0**, every `EXIT=0`.

## 7. Could not verify / residual

1. **The repo's published binary at `out\aot` is still the pre-fix one.** I did not republish over it (the main checkout is another round's tree and out of SCOPE). §4 therefore uses a fresh AOT publish **from the fixed worktree**; the pre-fix baseline in §1 is the repo's published binary.
2. **The fix is not in the main checkout and is not committed.** It is uncommitted in `.worktrees/c6-p0` (instruction 6). Anyone applying it must take both files: `Lovelace.Run/Runner.cs` (modified) and `Lovelace.Run.Tests/PlotDirectoryErrorTests.cs` (new).
3. **`--text` mode still leaves stdout empty on this failure.** `--eval 1 --text --plot-dir C:\Windows\notepad.exe` → exit 1, stdout 0 B, stderr 196 B `Error [PlotDirectoryError/TypeMismatch]: Cannot use plot directory …`. That is the runner's pre-existing, documented split (`Runner.cs:45-46`: stdout carries the envelope *or* the --text summary, stderr carries usage and non-JSON error text) and it is the same branch every other `--text` error already takes, `FileReadError` included — the code and category do reach the caller, on stderr. I did not change it: it is a contract change for every error path in the runner, well beyond F1-C, and no probe in the finding used `--text`. **Flagged for the parent to accept or reject.**
4. **Exception types claimed by construction, not observed:** ACL-denied and over-long (`PathTooLongException`) plot directories, and `NotSupportedException` path shapes — all covered by `IsUnusablePlotDirectory` but not reproduced (an ACL probe needs a second identity or elevation).
5. **Not re-probed after the fix:** the audit's DLL and README values (`kernel32.dll`, `README.md`); they are the same `IOException` shape as the `notepad.exe` row that was re-probed, and the test's own existing-file row covers the class.
6. **No coverage/instrumented run.** The covered subsets (Run 201/0, Suite 817/0 … from EVD-273) were not re-measured; the full uninstrumented solution run stands in for them.
7. **Not verified on Linux/macOS.** The probes are Windows-specific; the filter is portable, but the invalid-character row is a Windows behaviour.

## 8. Evidence index (all under `docs/goal-cycle-6/round-15/`)

| file | what it holds |
|---|---|
| `pre-probes.txt` | the four pre-fix values + control + success path on `out\aot\Lovelace.Run.exe`, with exit codes and stdout/stderr byte counts |
| `prefix-run.log` | the failing-first run in `.worktrees/c6-p0`: 3 failed / 1 passed, exit 1 |
| `postfix-run.log` | the same command after the fix: 4 passed / 0 failed, exit 0 |
| `after-probes.txt` | the same probes against the freshly published AOT binary from the fixed worktree |
| `aot-publish.log` | `dotnet publish … -p:PublishAot=true` → `out\aot-fix\Lovelace.Run.exe`, exit 0 |
| `control-run.log` | pristine `.worktrees/c6-p0ctl` + only the new test file: 3 failed / 1 passed |
| `solution-run.log` | `dotnet test LovelaceSharp.slnx --configuration Release --nologo`, exit 0, 15 projects, 0 failed |
| `timing-subsets.log` | `Category=Timing` for Run/Suite/Symbolics/Real: 5/8/1/1, all exit 0 |
