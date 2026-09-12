# G-2 (P1) — one text for every surface: positions index the caller's text, BOM included

Round 20 fixer landing. Fix worktree `.worktrees/r20-bom` (branch `fix/r20-bom`), control tree
`.worktrees/r20-bom-ctl` (detached at HEAD `d87e910`), both created from HEAD and nothing else.

Commit: **`81d2908`** (`81d29081b471022e154dbfd5f80b29400fd21a28`, branch `fix/r20-bom`, not pushed,
not merged). 4 files, +324/-8.

The parent orchestrator's contract finding agreed with this reading before the fix was written;
the probe it supplied is run below on both the HEAD binary and the fixed one.

## 1. The contract I found (quotes)

* `docs/symbolics/dsh-protocol.md:152-155` (HEAD) — "`timings[].position` is the zero-based
  **source offset of the statement**" — and `:225-228` — the error envelope's `diagnostics` array is
  "the **parser's source-position form** (`message`/`position`/`line`/`column`)". The document
  never said WHICH text "the source" is: it had no positions section at all, so it was **silent** on
  the BOM question.
* `--help` (`Lovelace.Run/Runner.cs:636-660`) lists `--eval`/`--file`/`--stdin` and says nothing
  about positions or encodings — **silent** too.
* Round 19's landing (`git show 63774da`, `Lovelace.Run/ScriptPositions.cs`) is not silent. Its class
  comment (`:5-13`): "The script text a CALLER supplied, together with the map back to it … Every
  source position this runner publishes — `timings[].position` and the error envelope's
  `diagnostics[].position`/`line`/`column` — is defined by the protocol against **the text the
  CALLER supplied**"; `:33-34`: "Maps `source` (the caller's text, **exactly as received**)";
  `:68-71`: "**the only skipped character is a leading U+FEFF**"; `:77`:
  `int s = source.Length > 0 && source[0] == '\uFEFF' ? 1 : 0;`. The commit message says the same:
  "the CRLF pair and a leading BOM are the only non-1:1 steps".
  A map that SKIPS the BOM while indexing the caller's text only makes sense if the caller's text
  still CONTAINS it.
* The existing test pins that reading in words and in numbers —
  `Lovelace.Run.Tests/SourcePositionTests.cs:228-245`:
  "A leading byte-order mark is part of the text the caller supplied, so the positions index into
  it", asserting `position 1` / `column 2` / `timings [1]` for `\uFEFFdet(1)` through `--eval`.
  It is unchanged by this landing and still passes.

**CHOICE: (i) — a position indexes the caller's text exactly as handed over, a leading BOM
included.** Reasons, in order: round 19's own documentation already defines it that way; the
existing `LeadingBom_PositionsStayOffsetsIntoTheCallersText` test pins it and must not be weakened;
and it is the reading a consumer can actually use — the offset slices the text the consumer handed
over. The documents are therefore not silent, so the fallback rule ("prefer (i) when silent") does
not even have to be invoked.

Consequence: **`--file` was the broken surface.** `Runner.cs:143` (HEAD) read the script with
`File.ReadAllTextAsync`, which detects the UTF-8 byte-order mark **and consumes it**, so that route
handed the rest of the runner a text one character shorter than the identical bytes on
`--eval`/`--stdin`, moving every published position by one. Nothing else about the three routes
differed.

## 2. Reproduction on the published Native AOT binary from HEAD

The audit's own file is in the repo: `out/bom.lv` = `EF BB BF` + `a = 1\nb = 2\ndet(a)\n` (22 bytes).

```powershell
& out\aot\Lovelace.Run.exe --file out\bom.lv --omit-functions --omit-variables --json
```

```
{"...","ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"det(): argument 1 must be a square matrix; got Natural.","recoverable":true,"diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":12,"line":3,"column":1}],"elapsed":"327.3 µs",...,"timings":[{"position":0,...},{"position":6,...},{"position":12,...}]}
EXIT=1
```

The parent's probe (`docs/goal-cycle-6/round-20/surface-consistency.ps1`), verbatim on that binary:

```
LF         AGREE file[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]  stdin[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]  eval[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]
BOM+LF     DIVERGE file[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]  stdin[exit=1 code=InvalidArgument pos=13 line=3 col=1 timings=[1,7,13]]  eval[exit=1 code=InvalidArgument pos=13 line=3 col=1 timings=[1,7,13]]
CRLF       AGREE file[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]  stdin[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]  eval[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]
BOM+CRLF   DIVERGE file[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]  stdin[exit=1 code=InvalidArgument pos=15 line=3 col=1 timings=[1,8,15]]  eval[exit=1 code=InvalidArgument pos=15 line=3 col=1 timings=[1,8,15]]
```

The consumer-visible harm, on the same text: `text.Substring(12)` of `"\uFEFFa = 1\nb = 2\ndet(a)"`
starts with `"\ndet(a)"`, not with the failing statement — the `--file` position names the wrong
character of the file the caller handed over.

## 3. The fix — one reader, one definition

New **`Lovelace.Run/ScriptText.cs`** is the single entry point every surface's text comes through,
and the single place the text's definition lives:

| surface | entry point | the text |
|---|---|---|
| `--eval` | `ScriptText.FromArgument(eval)` | the argument, character for character |
| `--stdin` | `ScriptText.FromStandardInputAsync(stdin)` | the characters the reader produced |
| `--file` | `ScriptText.FromFileAsync(file)` | the file's decoded characters, **a leading byte-order mark kept as U+FEFF** |

`Lovelace.Run/Runner.cs` no longer reads any surface itself: the three branches above are the only
way `source` is produced, so a fourth surface cannot arrive at a fourth decoding by accident.
The engine still drops the leading U+FEFF for tokenizing
(`Lovelace.Suite/ScriptSource.cs:31-32`), which `ScriptPositions` already maps back across
(`ScriptPositions.cs:77`), so evaluation is unchanged and only the `--file` text is repaired.

**Why not `detectEncodingFromByteOrderMarks: false`** (the mechanism the parent suggested): that
keeps the BOM, but it would also stop decoding UTF-16 files, which `File.ReadAllText` does detect —
a silent behaviour loss outside this finding. `ScriptText.Decode` therefore keeps the detection
(UTF-8 / UTF-16 LE / UTF-16 BE / UTF-32 preambles, UTF-8 when there is none) and re-attaches the
consumed mark as the first character: `reader.ReadToEnd()` then
`encoding.GetPreamble()` matched against the file's leading bytes. A file without a mark, or an
encoding without one, is returned untouched — measured below for an ASCII CRLF file (unchanged
positions) and a UTF-16LE file (BOM kept).

**CRLF handling is untouched**: `ScriptPositions.ToSourceOffset`/`LineColumn` and the
`ScriptSource` rewrite are not modified, and every round-19 CRLF/CR test still passes.

**No golden fixture was regenerated.** No fixture carries a BOM
(`Get-ChildItem Lovelace.Run.Tests/fixtures -File` → all 40 files `bom=False`), so every pinned
golden position is byte-identical after the fix; `git status` in the fix tree shows no fixture
change. One documentation line WAS stale and is corrected (see §7).

## 4. Test-first evidence

The new test is `Lovelace.Run.Tests/SurfacePositionAgreementTests.cs` (5 cases):
`EverySurface_ReportsTheSamePositions_ForTheSameText` over the four byte-level spellings
(LF / BOM+LF / CRLF / BOM+CRLF — the same script text handed to `--eval`, a real temp `--file` with
those bytes, and `--stdin`), plus `FileSurface_KeepsTheByteOrderMark_ForAUtf16File`. Every
expectation is measured by the test in the text it handed over (with the literals pinned by
self-checks), and the final assertion is the consumer property:
`source.Substring(position, 6) == "det(1)"`.

### Control tree = HEAD + the new test file ONLY → FAILS

```powershell
git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/r20-bom-ctl HEAD
cd .worktrees/r20-bom-ctl
dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj --filter "FullyQualifiedName~SurfacePositionAgreementTests" -v n
```

```
  Failed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "﻿a = 1\r\nb = 2\r\ndet(1)", first: 1, second: 8, failing: 15) [126 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
                                   ↓ (pos 1)
Expected: ["--eval 15 [1, 8, 15]", "--file 15 [1, 8, 15]", "--stdin 15 [1, 8, 15]"]
Actual:   ["--eval 15 [1, 8, 15]", "--file 14 [0, 7, 14]", "--stdin 15 [1, 8, 15]"]
                                   ↑ (pos 1)
  Stack Trace:
     at Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(String source, Int32 first, Int32 second, Int32 failing) in ...\SurfacePositionAgreementTests.cs:line 76
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "a = 1\r\nb = 2\r\ndet(1)", first: 0, second: 7, failing: 14) [6 ms]
  Failed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "﻿a = 1\nb = 2\ndet(1)", first: 1, second: 7, failing: 13) [5 ms]
  Error Message:
   Assert.Equal() Failure: Collections differ
                                   ↓ (pos 1)
Expected: ["--eval 13 [1, 7, 13]", "--file 13 [1, 7, 13]", "--stdin 13 [1, 7, 13]"]
Actual:   ["--eval 13 [1, 7, 13]", "--file 12 [0, 6, 12]", "--stdin 13 [1, 7, 13]"]
                                   ↑ (pos 1)
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "a = 1\nb = 2\ndet(1)", first: 0, second: 6, failing: 12) [7 ms]
  Failed Lovelace.Run.Tests.SurfacePositionAgreementTests.FileSurface_KeepsTheByteOrderMark_ForAUtf16File [4 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 13
Actual:   12
  Stack Trace:
     at ...SurfacePositionAgreementTests.FileSurface_KeepsTheByteOrderMark_ForAUtf16File() in ...\SurfacePositionAgreementTests.cs:line 136

Test Run Failed.
Total tests: 5
     Passed: 2
     Failed: 3
```

The two non-BOM rows PASS on HEAD and the two BOM rows plus the UTF-16 row FAIL, which is exactly
the finding: the surfaces disagree only when a byte-order mark is present, and only on `--file`.

Full suite in the same control tree (`dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -v n`),
so the delta on HEAD is attributable to the new file and nothing else:

```
Total tests: 291
     Passed: 288
     Failed: 3
```

### Fix tree → PASSES

```powershell
cd .worktrees/r20-bom
dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -v n
```

The five new cases, verbatim from that run, followed by the whole project:

```
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "﻿a = 1\r\nb = 2\r\ndet(1)", first: 1, second: 8, failing: 15) [480 ms]
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "a = 1\r\nb = 2\r\ndet(1)", first: 0, second: 7, failing: 14) [70 ms]
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "﻿a = 1\nb = 2\ndet(1)", first: 1, second: 7, failing: 13) [87 ms]
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.EverySurface_ReportsTheSamePositions_ForTheSameText(source: "a = 1\nb = 2\ndet(1)", first: 0, second: 6, failing: 12) [65 ms]
  Passed Lovelace.Run.Tests.SurfacePositionAgreementTests.FileSurface_KeepsTheByteOrderMark_ForAUtf16File [51 ms]
Total tests: 291
     Passed: 291
```

## 5. Published Native AOT verification (the fix, measured, not inferred)

```powershell
cd .worktrees/r20-bom
dotnet publish Lovelace.Run\Lovelace.Run.csproj --configuration Release -p:PublishAot=true -p:InvariantGlobalization=true -o out\aot-fix
& C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-20\surface-consistency.ps1 -Exe .\out\aot-fix\Lovelace.Run.exe
```

```
LF         AGREE file[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]  stdin[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]  eval[exit=1 code=InvalidArgument pos=12 line=3 col=1 timings=[0,6,12]]
BOM+LF     AGREE file[exit=1 code=InvalidArgument pos=13 line=3 col=1 timings=[1,7,13]]  stdin[exit=1 code=InvalidArgument pos=13 line=3 col=1 timings=[1,7,13]]  eval[exit=1 code=InvalidArgument pos=13 line=3 col=1 timings=[1,7,13]]
CRLF       AGREE file[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]  stdin[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]  eval[exit=1 code=InvalidArgument pos=14 line=3 col=1 timings=[0,7,14]]
BOM+CRLF   AGREE file[exit=1 code=InvalidArgument pos=15 line=3 col=1 timings=[1,8,15]]  stdin[exit=1 code=InvalidArgument pos=15 line=3 col=1 timings=[1,8,15]]  eval[exit=1 code=InvalidArgument pos=15 line=3 col=1 timings=[1,8,15]]
```

All four rows AGREE; LF/CRLF keep their old coordinates exactly (12/[0,6,12] and 14/[0,7,14]) and
the BOM rows move to 13/[1,7,13] and 15/[1,8,15]. The audit's own file through the fixed binary:

```powershell
& C:\Users\ricar\dev\LovelaceSharp\.worktrees\r20-bom\out\aot-fix\Lovelace.Run.exe `
  --file C:\Users\ricar\dev\LovelaceSharp\out\bom.lv --omit-functions --omit-variables --json
```

```
{"...","diagnostics":[{"message":"det(): argument 1 must be a square matrix; got Natural.","position":13,"line":3,"column":1}],...,"timings":[{"position":1,...},{"position":7,...},{"position":13,...}]}
EXIT=1
```

## 6. Full-suite totals for the projects touched

* `Lovelace.Run.Tests` (the only test project touched): **291 passed / 0 failed / 0 skipped** on the
  frozen fix tree (HEAD's own suite is 286 tests; the 5 new ones are the delta). Control tree with
  the new tests added: 288 passed / **3 failed** — the three BOM cases.
* No test was weakened, skipped or re-pinned; every pre-existing position test passes unchanged,
  including `LeadingBom_PositionsStayOffsetsIntoTheCallersText` and all four CRLF/CR cases.
* `Lovelace.Suite` (the engine and `ScriptSource`) and `Lovelace.Symbolics` were not touched, so
  their suites were not re-run.

## 7. What I could not do / open items

1. **The protocol document's error example contradicted the rule I recorded**, and was corrected in
   one line: `docs/symbolics/dsh-protocol.md:223` pinned
   `"diagnostics": [ { ..., "position": 0, "line": 1, "column": 1 } ]` beside `timings[1].position 17`
   — the very F1 shape round 19 fixed in the binary (its residual #2). It now reads
   `"position": 17, "line": 1, "column": 18`, which is what the binary and the golden
   `fixtures/error_envelope.json` answer for that one-line script. No test or golden changed.
2. **Round-19 residual #1 still stands**: a parse-error `message` carries the engine's own offset in
   prose ("Unexpected token ')' at position 10") while the numeric fields name the caller's text.
   Message text is not contract; not touched.
3. **The REPL has the same latent BOM strip**: `Lovelace.Console/Repl/ReplSession.cs:245` loads
   `:load`ed scripts with `File.ReadAllText`. It publishes no protocol positions (not a JSON
   surface) and is outside the three surfaces in this finding, so it is left as-is — but if the REPL
   ever publishes a position, it must go through `ScriptText.FromFileAsync` (or an equivalent
   shared reader) rather than its own decode.
4. **`Lovelace.Studio/IncrementalRunner.cs:178`** applies `ScriptSource.ToSemicolonStatements` to text
   the IDE already holds; nothing there reads a file, so it is unaffected.
5. The fix lives in `Lovelace.Run`, not in `Lovelace.Suite`, because `Lovelace.Run` is where the
   three protocol surfaces are defined and where the drift was; `ScriptSource`'s engine-side BOM
   strip is correct and deliberately unchanged.
6. Not pushed, not merged, per the brief; the commit is on `fix/r20-bom` only.
