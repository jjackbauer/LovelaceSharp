# Round 22 · M-1 fixer — the cancelled statement's output: measurement and outcome

**Task:** P1 — "the output of a cancelled statement is lost, and the envelope denies it", on
`for i in 1..2000000 { print(i) }` under `--cancel-after 100`; objective: a cancelled run must
publish the lines printed before the cancellation in `output[]` and `partialOutput[]`, with
`timings[].hasOutput = true` for a statement that printed.

**Outcome: the loss mechanism is NOT in this tree, and the M-1 program does not print before the
deadline.** The objective is already met at `19ae61e` — verified by measurement, pinned by five new
tests, and the audit's own repro is explained by a different fact (eager range materialization) that
the new tests also pin. **No product change was made**: there was no text to recover and no wrong
flag to correct, so any "fix" would have been a change to a correct answer. The commit is
test-only.

| item | value |
|---|---|
| worktree | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\r22-m1` (branch `r22-m1`) |
| control worktree | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\r22-m1-ctl` (detached at `19ae61e`) |
| commit | **`439092606d66b33e7f90b7c3d9427a5aa167d0a8`** (parent `19ae61e`) |
| files | `Lovelace.Run.Tests/CancelledStatementOutputTests.cs` (new, 199 lines) — the only change |
| product code | unchanged |
| `Lovelace.Run.Tests` | **336 passed / 0 failed / 0 skipped** (331 pre-existing + 5 new) |
| build | 0 warnings / 0 errors |

---

## 1. The exact command, and the exact envelope (published AOT, re-measured)

Binary: `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` (5 758 464 bytes,
2026-09-12 19:17:13).

    $exe = '...\out\aot\Lovelace.Run.exe'
    & $exe --eval 'for i in 1..2000000 { print(i) }' --json --omit-functions --omit-variables --cancel-after 100

    exit=1
    code=Cancelled category=BudgetExceeded
    output.count=0 partialOutput.count=0
    timings.count=1
      pos=0 kind=Void hasOutput=False elapsed=112.85ms
    cancellation={"budgetMs":100,"elapsedMs":113.092,"stopped":true,"exceeded":true,"excessMs":13.656}

Reproduced identically through `--file` (a BOM-free, LF script) and on a fresh JIT build of
`19ae61e` (`dotnet Lovelace.Run.dll`), so this is not an AOT artifact and not a surface artifact.

The finding's premise is the sentence "the deadline lands inside iteration 1 or 2, i.e. long after
at least one print has executed" (`audit-M-programs.md:53-55`). **That premise is false, and it is
measurable.**

## 2. Where the 100 ms actually goes: the range is materialized before the first iteration

`for i in 1..N` does not iterate lazily. `Interpreter.ExecuteForAsync`
(`Lovelace.Suite/Interpreter.cs:1247-1250`) evaluates the range expression first, and
`EvaluateRangeAsync` → `BuildRange` (`Interpreter.cs:902-907`, `1073-1113`) boxes **all N
elements** into a `List<Value>` before the `foreach` starts. Direct measurement on the published
binary:

    & $exe --eval 'len(1..100000000)'  --json --omit-functions --omit-variables
      exit=0  engine=32.05 s          <- 100 000 000 elements built to answer a length
    & $exe --eval 'v = 1..100000000; v[99999999]' ...
      exit=0  engine=34.23 s

    v = 1..2000000; print("done")   (instrumented JIT build of 19ae61e)
      buildRange start t=5ms -> buildRange end count=2000000 t=826ms

With a 600 ms budget and a print inside the loop body, the instrumented product (temporary
`Console`/file tracing in a scratch copy of `Interpreter.cs`, reverted) shows the deadline landing
*inside the build*:

    stmt 0 start t=0ms prints=0
    buildRange start 1..2000000 t=4ms
    stmt 0 finally captured=0 prints=0 t=580ms     <- deadline fired here; ZERO print calls happened
    envelope: code=Cancelled out=0 hasOut=False elapsed=598.77 ms

So for the audit's program at 100 ms the loop **never printed**, `output` is empty because nothing
was written, and `timings[0].hasOutput=false` is *true*. Three more measurements close the loop:

* the same program with a budget the build fits in publishes everything it printed —
  `--cancel-after 2000` → `out=1198716  partial=1198716  hasOut=True  first=1 last=1198092`
  (published AOT); the instrumented run computes the same count at the same instant.
* the auditor's control is consistent with this, not with "the same loop with a failure": their
  failing twin `for i in 1..100000000 { ... if (i == 1000) { nosuchfn(1) } }` needs the 100 M-element
  build (32 s measured for the bare build) before it can fail at `i == 1000`.
* `print("head")` before the huge loop survives at every budget tried (100/200/600/700/800 ms:
  `output=["head"]`, `partialOutput=["head"]`, `timings[0].hasOutput=true`), which is the shape
  round-19's fix pins.

## 3. Where the text would be lost, and why it is not

The capture path is one write-through, and both envelope paths read the same buffer:

* `Interpreter.ExecuteAsync` (statement loop) installs a per-statement `StringWriter` as
  `Output` (`Interpreter.cs:324-326`), and its `finally` (`333-341`) restores the previous writer,
  writes the captured text to it **before** recording the timing, and records
  `new OperationTiming(position, result, output, …)` — the text the timing entry reports
  `hasOutput` from. The `finally` runs for **any** exception, cancellation included: the token is
  observed by `KernelCancellation.PollNow()` inside the loop, which throws the same
  `OperationCanceledException` any other failure would be.
* `Runner.cs:313` reads `SplitLines(output.ToString())` **once** and hands the same array to
  `output` and (for `Cancelled`) to `partialOutput` (`Runner.cs:316`), so the two documented keys
  cannot disagree; `Timings(…)` computes `t.Output.Length > 0` (`Runner.cs:584-591`).
* `SuiteEngine.EvaluateAsync` restores the host writer in a `finally` (`SuiteEngine.cs:269-278`),
  so the Runner's `StringWriter` holds the committed text when its `catch` reads it.

Measurement sweep (JIT build of `19ae61e`, in-process, `--cancel-after` 100/200 ms, ten program
shapes: `for`/`while` loops, nested blocks, user functions, kernel calls `sum`/`fft`/`prod`,
ranges): every cancelled envelope had `output == partialOutput`, every published line was the
expected next print, and `hasOutput` was true on exactly the statements that wrote. No case
produced text that was dropped or a statement that claimed not to have printed.

## 4. Control tree: the new tests, run on pristine `19ae61e`

```
CONTROL HEAD: 19ae61ec993ed42714b91c3bef344ae10b181b96
?? Lovelace.Run.Tests/CancelledStatementOutputTests.cs      <- the only change, untracked
=== CONTROL run 1 ===
Passed ...AWhileLoopInterruptedDeepInsideTheBody_PublishesItsPrefix [251 ms]
Passed ...AStatementInterruptedAfterPrinting_PublishesWhatItPrinted [472 ms]
Passed ...TheAuditReproAtItsOwnBudget_StopsInsideTheRangeBuildBeforeAnyIteration [113 ms]
Passed ...TheCancelPathAndTheFailurePath_AgreeOnTheTextTheSameStatementPrinted [288 ms]
Passed ...AMultiStatementProgramCancelledMidFlight_CarriesExactlyWhatRan [114 ms]
=== CONTROL run 2 === (identical five passes)
```

**The control does not fail — because the bug is not there.** I am reporting that plainly rather
than manufacturing a red control: to make a test fail on `19ae61e` I would have to assert something
false about the base commit. (One intermediate version of
`AWhileLoopInterruptedDeepInsideTheBody_PublishesItsPrefix` did fail once on the control with
`output.Length > 1000`; that was my own flake — a 100 ms deadline can, on a loaded machine, fire
before the loop's first print — and the shipped version pairs every cancel case with an un-budgeted
run of the same text and asserts against measured counts instead of a constant.) The five tests pass
on the control **and** on my tree, which is the right outcome for a test-first fix of behaviour that
is already correct: they are regression pins, not a proof of a change.

## 5. What the new tests pin (`Lovelace.Run.Tests/CancelledStatementOutputTests.cs`)

1. `AStatementInterruptedAfterPrinting_PublishesWhatItPrinted` — `t = 0; for i in 1..60000 { print(i); t = t + i * i }`
   at `--cancel-after 50`: `output` non-empty, equal to `partialOutput`, equal to the un-budgeted
   run's prefix, `timings[^1].hasOutput = true` on the interrupted statement.
2. `AWhileLoopInterruptedDeepInsideTheBody_PublishesItsPrefix` — the same statement shape without a
   range; tens of thousands of lines, ordered, `hasOutput = true`.
3. `TheCancelPathAndTheFailurePath_AgreeOnTheTextTheSameStatementPrinted` — the failure twin prints
   all 60 000 lines then fails; the cancelled run's lines are that sequence's prefix. This is the
   finding's own requirement ("the failure path and the cancel path cannot disagree about the same
   text") stated as an executable assertion.
4. `TheAuditReproAtItsOwnBudget_StopsInsideTheRangeBuildBeforeAnyIteration` — the audit's exact
   program at its own budget: `output == partialOutput == ["head"]`, the loop entry honestly
   reports `hasOutput=false`, the ledger reports `stopped=true`.
5. `AMultiStatementProgramCancelledMidFlight_CarriesExactlyWhatRan` — a print before the interrupted
   statement and a print from inside it both cross; the statement after it never runs and its text
   appears nowhere.

**No existing test was weakened, skipped, deleted or re-pinned.** Nothing in the suite pins the
buggy behaviour because the buggy behaviour is not present; the closest existing pins are
`PrintOutputOnFailureTests.TheCancelledPath_KeepsPartialOutput_AndCarriesTheDocumentedOutput`
(a completed statement before an interrupted one) and `EnvelopeDurationTests`' `hasOutput`
presence check — both still pass.

## 6. Full-project totals

    dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release
    Passed!  - Failed: 0, Passed: 336, Skipped: 0, Total: 336, Duration: 23 s

    dotnet build Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release
    0 Warning(s) / 0 Error(s)

Counts include the five new tests; the pre-existing total on the control is 331/0.

## 7. Could not do / open items

* **I could not produce a program in which a cancelled statement's printed text is lost.** That is
  the finding, and the honest report is that it did not reproduce in ~10 program shapes, 3 surfaces
  (`--eval`, `--file`, in-process), 2 runtimes (AOT, JIT), or with budgets from 10 ms to 3 s.
* **The audit's premise about the repro could not be verified from its own data** and is contradicted
  by measurement: `1..2000000` alone costs 0.53-0.83 s, so at 100 ms no iteration can have run.
  If the orchestrator wants M-1 graded from the published evidence alone, the deciding question is
  whether any run ever observed a *printed* line missing — and the only such run documented is the
  one whose loop body provably never started.
* **A real, adjacent defect was found and is left for its own round:** the eager materialization of
  `for` ranges is charged to the statement's budget before the loop can iterate, so
  `for i in 1..2000000 { print(i) }` under a 100 ms budget answers "the statement wrote nothing"
  for a program whose first action would have been a print. The envelope does not lie, but a consumer
  cannot tell "the loop printed nothing yet" from "the loop never got to iterate". That is a
  `BuildRange`/budget-shape issue, not the drop M-1 describes; changing it would change what a
  correct run publishes (it would make the same envelopes non-empty for the same budgets), which the
  task forbids without its own evidence.
* **`--text` still drops committed output on both the failure and the cancel path** (measured:
  `--text` cancel writes 0 bytes to stdout and only the error to stderr; `--json` carries the
  lines). This is the recorded round-18 F8 / round-20 G-5 P2, explicitly out of scope for M-1 and
  untouched here.
