# Round 2 — the agent-facing JSON envelope is now pinned by versioned golden fixtures

Goal: Lovelace.Run's wire contract cannot change unnoticed. The envelope construction was
extracted into a testable entry point **without changing a single emitted value**, and the runner
now has its own test project (Lovelace.Run.Tests) whose golden fixtures pin the protocol.

## 1. What changed (scope only)

| Path | Change |
| --- | --- |
| `Lovelace.Run/Runner.cs` (new) | `public static class Runner` with `RunAsync(string[] args, TextWriter stdout, TextWriter stderr, TextReader? stdin = null)`. The whole CLI moved here verbatim; every write that was `Console.Out`/`Console.Error` now targets the supplied writers, and `--stdin` reads the supplied reader (default `Console.In`). |
| `Lovelace.Run/RunProtocol.cs` (new) | The envelope DTOs and the source-generated `RunJsonContext`/`RunJsonPrettyContext` moved verbatim (they lived in Program.cs, not in their own file). No DTO or JSON field was renamed. |
| `Lovelace.Run/Program.cs` | Now only the top-level entry point: `return await Lovelace.Run.Runner.RunAsync(args, Console.Out, Console.Error);` plus the protocol header comment. |
| `Lovelace.Run.Tests/**` (new) | xUnit project mirroring `Lovelace.Suite.Tests`' package versions; 18 tests; 14-row fixture corpus (28 fixture files). |
| `LovelaceSharp.slnx` | `+` `Lovelace.Console.Tests/Lovelace.Console.Tests.csproj`, `+` `Lovelace.Run.Tests/Lovelace.Run.Tests.csproj`. |
| `.github/workflows/ci.yml` | Both project paths added to the `suites=(` array of the `fast-tests` job (existing comments untouched; one comment added for the runner suite). |

AOT safety: `IsAotCompatible` is still `true`, the JSON source-generation contexts are kept (the
reflection serializer is still never used), and no reflection was introduced. Extra check beyond
STEP 6: a full Native AOT publish plus a run of the published binary (see §7).

## 2. STEP 1 — the pre-change wire (captured before editing anything)

```
dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release
dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file docs/goal-cycle-3/round-2/regression.ls --omit-functions
```

`docs/goal-cycle-3/round-2/regression.ls` is the one-line script
`x = symbol("x"); solve_full(x^2 - 4 == 0, x)` (JIT build, not AOT).
The command exited `0` and wrote **3458 bytes** of envelope to
`docs/goal-cycle-3/round-2/pre-refactor-envelope.json`; stderr was empty.
`pre-refactor-envelope.json` sha256 = `B6FD2192163EEBC4726D37038E456779FD06352C59F961A4BCD52232AD4ECF22`.

## 3. STEP 3 — regression proof

The extraction was rebuilt and **the identical command** re-run into the post-change file:

```
dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo
dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file docs/goal-cycle-3/round-2/regression.ls --omit-functions
node docs/goal-cycle-3/round-2/normalise-compare.js docs/goal-cycle-3/round-2/pre-refactor-envelope.json docs/goal-cycle-3/round-2/post-refactor-envelope.json
```

`normalise-compare.js` parses both documents, replaces the value of every volatile key
(`revision`, `elapsed`, `elapsedTime`, `timings`) with the literal string `"<volatile>"` at any
depth, and deep-compares the trees (key order irrelevant, any added/removed/changed value is a
difference). Observed output of the compare:

```
IDENTICAL — normalised documents are deep-equal (3276 chars normalised)
compare exit=0
```

The two raw files are the same size and differ **only** in the volatile values, which is why the
raw hashes differ:

```
pre  sha256=B6FD2192163EEBC4726D37038E456779FD06352C59F961A4BCD52232AD4ECF22
post sha256=E636CE87C4D22DEF25CA982028986A8FD2F987151FCFEB06E42F2431FEBFD649
volatile values observed:
pre  {"revision":76,"elapsed":"47.89 ms","elapsedTime":{"value":47.89,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":4.52,"unit":"ms"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":37.14,"unit":"ms"},"resultKind":"Record","hasOutput":false}]}
post {"revision":76,"elapsed":"52.46 ms","elapsedTime":{"value":52.46,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":5.19,"unit":"ms"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":40.63,"unit":"ms"},"resultKind":"Record","hasOutput":false}]}
```

The comparison was re-run once more against the final source tree (nothing else touching
Lovelace.Run) and again printed `IDENTICAL — normalised documents are deep-equal`.

## 4. STEP 4 — test-first, observed failing, then passing

`Lovelace.Run.Tests/Lovelace.Run.Tests.csproj` (net10.0, xUnit 2.9.3 / Microsoft.NET.Test.Sdk
17.14.1 / xunit.runner.visualstudio 3.1.4 / coverlet.collector 6.0.4 — the same versions
Lovelace.Suite.Tests uses), nullable + implicit usings on, ProjectReference to
`..\Lovelace.Run\Lovelace.Run.csproj`, fixtures copied with
`CopyToOutputDirectory="PreserveNewest"`.

Tests were written **before** any fixture existed and run:

```
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo     (EXIT=1)

  Failed Lovelace.Run.Tests.StdoutPurityTests.RealProcessStdoutIsExactlyOneJsonDocumentAndPrintStaysInTheOutputArray [62 ms]
  Error Message:
   missing fixture script: C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run.Tests\bin\Release\net10.0\fixtures\print_purity.ls
  Stack Trace:
     at Lovelace.Run.Tests.StdoutPurityTests.RealProcessStdoutIsExactlyOneJsonDocumentAndPrintStaysInTheOutputArray() in ...\Lovelace.Run.Tests\StdoutPurityTests.cs:line 23
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(name: "record", expectedExitCode: 0) [< 1 ms]
  Error Message:
   missing fixture script: ...\Lovelace.Run.Tests\bin\Release\net10.0\fixtures\record.ls
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.SolveRecordCarriesTheStructuralSolveContract [1 ms]
  Error Message:
   missing fixture script: ...\fixtures\record.ls
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.ErrorEnvelopeCarriesAStructuralCodeAndCategory [< 1 ms]
  Error Message:
   missing fixture script: ...\fixtures\error_envelope.ls
  Failed Lovelace.Run.Tests.GoldenEnvelopeTests.EveryFixtureScriptHasACorpusRow [< 1 ms]
  Error Message:
   missing fixture directory: C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run.Tests\bin\Release\net10.0\fixtures

Failed!  - Failed:    18, Passed:     0, Skipped:     0, Total:    18, Duration: 162 ms - Lovelace.Run.Tests.dll (net10.0)
```

(18 `missing fixture` errors, one per test — the failures are assertion failures at run time, not
compile errors. Full log: `docs/goal-cycle-3/round-2/test-first-failing.log`.)

The fixtures were then generated by running the **real runner** over each corpus script
(`node docs/goal-cycle-3/round-2/generate-fixtures.js`), which writes the `.ls` script and the
`.json` golden (volatile values replaced by `"<volatile>"`, pretty-printed with 2-space
indentation and a trailing newline). Re-running the generator is byte-identical, so the goldens are
reproducible:

```
symbolic          exit=0  ok=true  revision=76  elapsed="22.76 ms"
vector            exit=0  ok=true  revision=76  elapsed="12.23 ms"
array             exit=0  ok=true  revision=77  elapsed="17.47 ms"
record            exit=0  ok=true  revision=76  elapsed="46.03 ms"
nested_record     exit=0  ok=true  revision=76  elapsed="29.93 ms"
solve_result      exit=0  ok=true  revision=76  elapsed="48.58 ms"
transform_result  exit=0  ok=true  revision=76  elapsed="25.53 ms"
limit_result      exit=0  ok=true  revision=76  elapsed="31.18 ms"
system_solve      exit=0  ok=true  revision=77  elapsed="49.19 ms"
compilation       exit=0  ok=true  revision=76  elapsed="29.5 ms"
complex           exit=0  ok=true  revision=75  elapsed="41.46 ms"
absent_field      exit=0  ok=true  revision=76  elapsed="29.02 ms"
error_envelope    exit=1  ok=false revision=undefined  elapsed="21.8 ms"
print_purity      exit=0  ok=true  revision=75  elapsed="10.06 ms"
wrote 14 script + 14 golden fixtures to ...\Lovelace.Run.Tests\fixtures
fixture files: 28
regeneration identical: True
```

```
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo     (EXIT=0)

Passed!  - Failed:     0, Passed:    18, Skipped:     0, Total:    18, Duration: 780 ms - Lovelace.Run.Tests.dll (net10.0)
```

`Skipped: 0` matters: the real-process stdout-purity test actually ran (it is only skipped when
`AppContext.BaseDirectory + "Lovelace.Run.dll"` is absent). Full log:
`docs/goal-cycle-3/round-2/test-final-passing.log`.

### Fixture corpus — 14 rows, 28 files

| # | Row | Script (fixtures/&lt;row&gt;.ls) | Exit | What it pins |
| --- | --- | --- | --- | --- |
| 1 | symbolic | `x = symbol("x"); x^2 + 1` | 0 | a symbolic value: pretty + canonical + domain/exact/nodeCount/freeSymbols |
| 2 | vector | `x = symbol("x"); [x, x, x]` | 0 | rank-1 Array shape `[3]` with symbolic elements |
| 3 | array | `x = symbol("x"); y = symbol("y"); [[x, y], [y, x]]` | 0 | rank-2 Array shape `[2,2]`, row-major elements |
| 4 | record | `x = symbol("x"); solve_full(x^2 - 4 == 0, x)` | 0 | SolveResult record with two Solution records |
| 5 | nested_record | `x = symbol("x"); solve_full(sin(x) == 0, x)` | 0 | SolveResult with solution FAMILIES (record in record in array) |
| 6 | solve_result | `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)` | 0 | a partial solve: status Partial, complete false, unrepresented_count |
| 7 | transform_result | `x = symbol("x"); simplify_full(sin(x)^2 + cos(x)^2)` | 0 | TransformResult with rewrite steps and conditions |
| 8 | limit_result | `x = symbol("x"); limit_full(sin(x)/x, x, 0)` | 0 | LimitResult (value/one-sided fields) |
| 9 | system_solve | `x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])` | 0 | SystemSolveResult with assignments |
| 10 | compilation | `x = symbol("x"); compile_full(x^2 + 1, [x])` | 0 | CompilationResult carrying MathIR text |
| 11 | complex | `dft([1, 2, 3])` | 0 | complex wire shape (re/im elements) — see §8 |
| 12 | absent_field | `x = symbol("x"); inspect(x^2 + 1)` | 0 | an inspection record (null/absent field handling) |
| 13 | error_envelope | `x = symbol("x"); solve(2*x == 1, x, integer)` | 1 | the error envelope: ok false + code + category + diagnostics |
| 14 | print_purity | `print("hello from the script"); 1 + 1` | 0 | stdout purity: printed text only in `output` |

### The tests

* **(a)** `GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture` — one `[Theory]` case per row. Both
  sides are parsed as JSON and compared as *trees*: key ORDER is irrelevant, any
  added/removed/changed value or array length fails with a JSON path.
* **(b)** `GoldenEnvelopeTests.SolveRecordCarriesTheStructuralSolveContract` — independent of the
  goldens, re-runs the `record` script and asserts `kind == "Record"`, `type == "SolveResult"`,
  `status == "Solved"` (the string the runner emits today), `domain.domain == "complex"`,
  `solutions.kind == "Array"`, `solutions.shape == [2]`, and that both elements are Records named
  `Solution`.
* **(c)** `GoldenEnvelopeTests.ErrorEnvelopeCarriesAStructuralCodeAndCategory` — exit code 1,
  `ok == false`, non-empty string `code` and `category`, and empty stderr in JSON mode.
* **(d)** `StdoutPurityTests.RealProcessStdoutIsExactlyOneJsonDocumentAndPrintStaysInTheOutputArray`
  — locates `AppContext.BaseDirectory + "Lovelace.Run.dll"`, starts
  `dotnet exec <that dll> --file <fixtures/print_purity.ls> --omit-functions`, and asserts stdout
  parses as **exactly one** JSON document (a `Utf8JsonReader` check that also rejects trailing
  content), that `output == ["hello from the script"]`, that the structured value is `"2"`, and
  that the printed text occurs exactly once in the whole stream (i.e. only inside `output`).
* plus `EveryFixtureScriptHasACorpusRow`, which keeps the corpus and the fixture directory from
  drifting apart.

The test project copies the runner's `runtimeconfig.json` next to the assembly: a
ProjectReference copies `Lovelace.Run.dll` but not its runtime configuration, and `dotnet exec`
refuses to start without it (observed: *"The application was run as a self-contained app because
...runtimeconfig.json was not found"*).

### Falsification of the gate (the tests are not vacuous)

```
MUTATION 1  record.json: "value": "Solved" -> "value": "SolvedX"
            → Failed!  - Failed: 1, Passed: 17, Skipped: 0, Total: 18   (exit 1)
MUTATION 2  record.json: key order reversed at EVERY level, values untouched
            → Passed!  - Failed: 0, Passed: 18, Skipped: 0, Total: 18   (exit 0)  [order-insensitive]
MUTATION 3  record.json: field name "solutions" -> "solutions_gone"
            → Error Message:
              record: envelope does not match the golden fixture:
              $.result.structured.fields[5].name: expected "solutions_gone", got "solutions"
              Failed!  - Failed: 1, Passed: 17, Skipped: 0, Total: 18    (exit 1)
```

After each mutation the golden was restored (sha256 of `record.json` back to
`0CC3E272DA3DFF70B370DBFB9D6967C7A46DF34CF53D42677849A37FDFCFB14C`).

## 5. STEP 5 — wiring

`LovelaceSharp.slnx` gained `Lovelace.Console.Tests` (next to `Lovelace.Console`) and
`Lovelace.Run.Tests` (next to `Lovelace.Run`). `.github/workflows/ci.yml` `suites=(` gained
`Lovelace.Console.Tests/Lovelace.Console.Tests.csproj` (line 65) and
`Lovelace.Run.Tests/Lovelace.Run.Tests.csproj` (line 73), with all pre-existing comments intact.
Both projects are now inside the fast-tests gate, so their tests can fail CI.

## 6. STEP 6 — full verification (observed)

```
dotnet build LovelaceSharp.slnx -c Release --nologo                     (EXIT=0)
    0 Warning(s)
    0 Error(s)
    Time Elapsed 00:00:02.50
    Lovelace.Console.Tests -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Console.Tests\bin\Release\net10.0\Lovelace.Console.Tests.dll
    Lovelace.Run.Tests -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run.Tests\bin\Release\net10.0\Lovelace.Run.Tests.dll

dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo        (EXIT=0)
Passed!  - Failed:     0, Passed:    18, Skipped:     0, Total:    18, Duration: 302 ms - Lovelace.Run.Tests.dll (net10.0)

dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo  (EXIT=0)
Passed!  - Failed:     0, Passed:   389, Skipped:     0, Total:   389, Duration: 14 s - Lovelace.Symbolics.Tests.dll (net10.0)

dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo     (EXIT=0)
Passed!  - Failed:     0, Passed:   620, Skipped:     0, Total:   620, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)

dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj -c Release --nologo (EXIT=0)
Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15, Duration: 591 ms - Lovelace.Console.Tests.dll (net10.0)
```

The three pre-existing suites still report **389 / 620 / 15 passed, 0 failed** — no regression, and
the new suite adds 18. Per-suite logs: `test-Lovelace.Run.Tests.log`,
`test-Lovelace.Symbolics.Tests.log`, `test-Lovelace.Suite.Tests.log`,
`test-Lovelace.Console.Tests.log`, `build-solution.log` in this directory.

## 7. Extra check — the runner still publishes and runs under Native AOT

```
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o %TEMP%\aot-out
    Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\win-x64\Lovelace.Run.dll
    Lovelace.Run -> C:\Users\ricar\AppData\Local\Temp\aot-out\           (exit 0, no IL2xxx/IL3xxx warnings)
    published: Lovelace.Run.exe  5611008 bytes

%TEMP%\aot-out\Lovelace.Run.exe --file docs/goal-cycle-3/round-2/regression.ls --omit-functions   (AOT EXIT=0)
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":76,"result":{"kind":"Record","display":"SolveResult(status: Solved, variable:
```

The only warnings in the publish log are the pre-existing CS8767/CS8600 nullability warnings in
`Lovelace.Integer`/`Lovelace.Real` (untouched projects). Log: `publish-aot.log`.

## 8. Anything that could not be made to pass / deviations

* **Nothing failed.** All 18 new tests and all 1,024 pre-existing tests are green.
* **The `complex` row could not use `(1 + 2*i) * (3 - i)`.** `i` is not defined — the runner
  answers `{"ok":false,"code":"InvalidOperation","message":"Undefined variable 'i'."}` — and the
  language has no complex literal at all: `Lovelace.Suite/docs/Language.md:806` states *"Complex
  has no literal in the grammar; `Complex.Parse` is library-only, and complex values enter the
  language through the DSP builtins"* (Language.md:787-807). The row therefore uses the documented
  route, `dft([1, 2, 3])` (a DSP builtin returning a complex vector: `[6, -1.5 + 0.866...i, -1.5 -
  0.866...i]`), which still pins the complex wire shape. The substitution is recorded in
  `generate-fixtures.js` next to the row.
* **No dynamic skip in xUnit v2.** `Assert.Skip` does not exist in xunit 2.9.3
  (`Xunit.Sdk.SkipException.ForSkip` is documented as "only works in v3 and later"). The
  stdout-purity test therefore uses a small `[RequiresRunnerProcessFact]` attribute derived from
  `FactAttribute` that sets `Skip` with a clear message when `Lovelace.Run.dll` is not next to
  the test assembly. The runner assembly is always present in a normal build, so the test ran
  (`Skipped: 0`); the skip path is defensive only and was not exercised.
* `docs/goal-cycle-3/round-2/*.chunked` and `chunk2.ps1` were not produced by this change; they
  belong to another writer in this workspace and were left untouched.
* No git commit was made.

## 9. Reproduce

```
dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release
dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file docs/goal-cycle-3/round-2/regression.ls --omit-functions > docs/goal-cycle-3/round-2/post-refactor-envelope.json
node docs/goal-cycle-3/round-2/normalise-compare.js docs/goal-cycle-3/round-2/pre-refactor-envelope.json docs/goal-cycle-3/round-2/post-refactor-envelope.json
node docs/goal-cycle-3/round-2/generate-fixtures.js          # only to refresh goldens deliberately
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
```
