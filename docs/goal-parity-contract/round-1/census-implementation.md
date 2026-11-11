# Round 1 — C# test census and frozen oracle: implementation log

Status: **complete**. Every command below was run in this workspace at commit
`1cc28f255eca6029849f880949fc7be1d40ab8af` (branch `rust`) on 2026-11-10, and the
output shown is the observed output, copied verbatim from the terminal.

## 0. Artifacts

| path | what it is |
|---|---|
| `parity/tools/census/census.csproj` | net10.0 console project, **no** `PackageReference` |
| `parity/tools/census/Program.cs` | reflection census generator + deterministic JSON writer |
| `parity/tools/census/SourceLocator.cs` | portable-PDB + source-scan method-to-file:line mapping |
| `parity/oracle.json` | the frozen oracle: commit, pinned timestamp, build/run commands, 36 source dirs |
| `parity/census.json` | generated census (3,998,378 bytes) |
| `parity/build-oracle.ps1` | verify + build + probe the oracle, and `-Verify` mode |
| `parity/census-notes.md` | method, totals, reconciliation, limitations |
| `parity/build/oracle-verified.json` | artifact written by the oracle script |
| this file | commands and observed output |

Environment observed:

    PS> dotnet --version
    10.0.103
    PS> dotnet --list-sdks
    8.0.419 [C:\Program Files\dotnet\sdk]
    10.0.103 [C:\Program Files\dotnet\sdk]
    PS> $PSVersionTable.PSVersion.ToString()
    5.1.26100.4202
    PS> git rev-parse HEAD
    1cc28f255eca6029849f880949fc7be1d40ab8af

## 1. Rebuild the 15 test assemblies at the frozen commit

The census reads `bin/Debug/net10.0/<project>.Tests.dll`. The pre-existing DLLs were
older than some sources (`Lovelace.Symbolics.Tests` DLL 2026-09-12 vs newest source
2026-09-16), so all 15 test projects were rebuilt first. `dotnet build` works; only the
VSTest testhost does not, and `dotnet test` was not used anywhere.

    PS> $projects = Get-ChildItem -Directory -Filter '*.Tests' | Sort-Object Name
    PS> foreach ($p in $projects) {
    >>   $proj = Join-Path $p.FullName ($p.Name + '.csproj')
    >>   $out = dotnet build $proj -c Debug --nologo 2>&1
    >>   $code = $LASTEXITCODE
    >>   $errs = ($out | Where-Object { $_ -match ' error ' } | Select-Object -First 2) -join ' // '
    >>   "$($p.Name): exit=$code errors=[$errs]"
    >> }
    Lovelace.Abstractions.Tests: exit=0 errors=[]
    Lovelace.Array.Tests: exit=0 errors=[]
    Lovelace.Complex.Tests: exit=0 errors=[]
    Lovelace.Console.Tests: exit=0 errors=[]
    Lovelace.Dsp.Tests: exit=0 errors=[]
    Lovelace.Integer.Tests: exit=0 errors=[]
    Lovelace.Knowledge.Tests: exit=0 errors=[]
    Lovelace.Natural.Tests: exit=0 errors=[]
    Lovelace.Real.Tests: exit=0 errors=[]
    Lovelace.Representation.Tests: exit=0 errors=[]
    Lovelace.Run.Tests: exit=0 errors=[]
    Lovelace.Studio.Tests: exit=0 errors=[]
    Lovelace.Suite.Tests: exit=0 errors=[]
    Lovelace.Symbolics.Tests: exit=0 errors=[]
    precbench.Tests: exit=0 errors=[]

## 2. Build the census tool

    PS> dotnet build 'parity\tools\census\census.csproj' -c Release --nologo 2>&1
      Determining projects to restore...
      All projects are up-to-date for restore.
      census -> C:\Users\ricar\dev\LovelaceSharp\parity\tools\census\bin\Release\net10.0\census.dll

    Build succeeded.
        0 Warning(s)
        0 Error(s)

    Time Elapsed 00:00:00.84

    PS> "BUILD_EXIT=$LASTEXITCODE"
    BUILD_EXIT=0

The project file contains no `PackageReference` and no `ProjectReference`; it compiles
against the net10.0 framework reference pack only (`System.Reflection.Metadata` is part
of that pack, which is what reads the portable PDBs). Restore needed no package source.

## 3. Run the census

    PS> dotnet exec 'parity\tools\census\bin\Release\net10.0\census.dll' --repo 'C:\Users\ricar\dev\LovelaceSharp' 2>&1
    census: tool=LovelaceParityCensus 1.0.0
    census: repo=C:\Users\ricar\dev\LovelaceSharp
    census: oracle=C:\Users\ricar\dev\LovelaceSharp\parity\oracle.json commit=1cc28f255eca6029849f880949fc7be1d40ab8af capturedAtUtc=2026-11-10T14:39:48Z
    census: project=Lovelace.Abstractions.Tests assembly=Lovelace.Abstractions.Tests/bin/Debug/net10.0/Lovelace.Abstractions.Tests.dll methods=33 cases=53 inline=23 member=0 unavailable=0 source[pdb=33 scan=0 missing=0]
    census: project=Lovelace.Array.Tests assembly=Lovelace.Array.Tests/bin/Debug/net10.0/Lovelace.Array.Tests.dll methods=19 cases=19 inline=0 member=0 unavailable=0 source[pdb=19 scan=0 missing=0]
    census: project=Lovelace.Complex.Tests assembly=Lovelace.Complex.Tests/bin/Debug/net10.0/Lovelace.Complex.Tests.dll methods=92 cases=121 inline=36 member=3 unavailable=0 source[pdb=92 scan=0 missing=0]
    census: project=Lovelace.Console.Tests assembly=Lovelace.Console.Tests/bin/Debug/net10.0/Lovelace.Console.Tests.dll methods=15 cases=15 inline=0 member=0 unavailable=0 source[pdb=1 scan=14 missing=0]
    census: project=Lovelace.Dsp.Tests assembly=Lovelace.Dsp.Tests/bin/Debug/net10.0/Lovelace.Dsp.Tests.dll methods=61 cases=61 inline=0 member=0 unavailable=0 source[pdb=61 scan=0 missing=0]
    census: project=Lovelace.Integer.Tests assembly=Lovelace.Integer.Tests/bin/Debug/net10.0/Lovelace.Integer.Tests.dll methods=143 cases=148 inline=7 member=0 unavailable=0 source[pdb=143 scan=0 missing=0]
    census: project=Lovelace.Knowledge.Tests assembly=Lovelace.Knowledge.Tests/bin/Debug/net10.0/Lovelace.Knowledge.Tests.dll methods=18 cases=28 inline=11 member=0 unavailable=0 source[pdb=18 scan=0 missing=0]
    census: project=Lovelace.Natural.Tests assembly=Lovelace.Natural.Tests/bin/Debug/net10.0/Lovelace.Natural.Tests.dll methods=156 cases=195 inline=53 member=0 unavailable=0 source[pdb=152 scan=4 missing=0]
    census: project=Lovelace.Real.Tests assembly=Lovelace.Real.Tests/bin/Debug/net10.0/Lovelace.Real.Tests.dll methods=285 cases=2537 inline=142 member=2142 unavailable=0 source[pdb=275 scan=10 missing=0]
    census: project=Lovelace.Representation.Tests assembly=Lovelace.Representation.Tests/bin/Debug/net10.0/Lovelace.Representation.Tests.dll methods=86 cases=91 inline=7 member=0 unavailable=0 source[pdb=79 scan=7 missing=0]
    census: project=Lovelace.Run.Tests assembly=Lovelace.Run.Tests/bin/Debug/net10.0/Lovelace.Run.Tests.dll methods=226 cases=751 inline=133 member=428 unavailable=0 source[pdb=9 scan=217 missing=0]
    census: project=Lovelace.Studio.Tests assembly=Lovelace.Studio.Tests/bin/Debug/net10.0/Lovelace.Studio.Tests.dll methods=22 cases=22 inline=0 member=0 unavailable=0 source[pdb=0 scan=22 missing=0]
    census: project=Lovelace.Suite.Tests assembly=Lovelace.Suite.Tests/bin/Debug/net10.0/Lovelace.Suite.Tests.dll methods=435 cases=929 inline=101 member=418 unavailable=0 source[pdb=191 scan=244 missing=0]
    census: project=Lovelace.Symbolics.Tests assembly=Lovelace.Symbolics.Tests/bin/Debug/net10.0/Lovelace.Symbolics.Tests.dll methods=505 cases=1226 inline=190 member=587 unavailable=0 source[pdb=462 scan=43 missing=0]
    census: project=precbench.Tests assembly=precbench.Tests/bin/Debug/net10.0/precbench.Tests.dll methods=9 cases=13 inline=6 member=0 unavailable=0 source[pdb=9 scan=0 missing=0]
    census: totals projects=15 testMethods=2105 testCases=6209 factMethods=1922 theoryMethods=183 inlineDataRows=709 memberDataRows=3578 dataUnavailable=0
    census: source total[pdb=1544 scan=561 missing=0]
    census: wrote C:\Users\ricar\dev\LovelaceSharp\parity\census.json bytes=3998378 sha256=3cb60e8cc20896999341f25f2f64ecb2584d780187816245844f96309786be02

    PS> "CENSUS1_EXIT=$LASTEXITCODE"
    CENSUS1_EXIT=0

Resulting `parity/census.json` header (observed):

    {
      "schemaVersion": 1,
      "oracle": {
        "commit": "1cc28f255eca6029849f880949fc7be1d40ab8af",
        "capturedAtUtc": "2026-11-10T14:39:48Z"
      },
      "generator": {
        "tool": "LovelaceParityCensus 1.0.0",
        "method": "reflection"
      },
      "totals": {
        "projects": 15,
        "testMethods": 2105,
        "testCases": 6209,
        "factMethods": 1922,
        "theoryMethods": 183,
        "inlineDataRows": 709,
        "memberDataRows": 3578,
        "dataUnavailable": 0
      },
      "projects": [ ... ],
      "cases": [ ... ]
    }

## 4. Determinism (two runs, same SHA-256)

`capturedAtUtc` is **pinned**: the tool copies it (and the commit) from
`parity/oracle.json`, so no wall-clock value enters the file. Everything else derives
from assembly metadata and source text.

    PS> dotnet exec $dll --repo 'C:\Users\ricar\dev\LovelaceSharp' > $null 2>&1
    PS> $h1 = (Get-FileHash 'parity\census.json' -Algorithm SHA256).Hash.ToLowerInvariant()
    PS> dotnet exec $dll --repo 'C:\Users\ricar\dev\LovelaceSharp' > $null 2>&1
    PS> $h2 = (Get-FileHash 'parity\census.json' -Algorithm SHA256).Hash.ToLowerInvariant()
    PS> "run1=$h1"; "run2=$h2"; "equal=$($h1 -eq $h2)"
    run1=3cb60e8cc20896999341f25f2f64ecb2584d780187816245844f96309786be02
    run2=3cb60e8cc20896999341f25f2f64ecb2584d780187816245844f96309786be02
    equal=True

The second run's tail was:

    census: totals projects=15 testMethods=2105 testCases=6209 factMethods=1922 theoryMethods=183 inlineDataRows=709 memberDataRows=3578 dataUnavailable=0
    census: source total[pdb=1544 scan=561 missing=0]
    census: wrote C:\Users\ricar\dev\LovelaceSharp\parity\census.json bytes=3998378 sha256=3cb60e8cc20896999341f25f2f64ecb2584d780187816245844f96309786be02
    CENSUS2_EXIT=0
    SHA_MATCH=True

## 5. parity/oracle.json

Written once with `capturedAtUtc = 2026-11-10T14:39:48Z`, `commit =
1cc28f255eca6029849f880949fc7be1d40ab8af`, `branch = rust`,
`csproj = Lovelace.Run/Lovelace.Run.csproj`,
`buildCommand = dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release`,
`runnerPath = Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`,
`dotnetVersion = 10.0.103`, and 36 `sources` directories (every project in
`LovelaceSharp.slnx`: 15 test projects, 18 production projects, and the three
benchmarking projects dspbench, precbench and symbench).

## 6. Run parity/build-oracle.ps1 (full mode)

    PS> powershell.exe -NoProfile -ExecutionPolicy Bypass -File '.\parity\build-oracle.ps1' 2>&1
    oracle: repo=C:\Users\ricar\dev\LovelaceSharp
    oracle: head=1cc28f255eca6029849f880949fc7be1d40ab8af pinned=1cc28f255eca6029849f880949fc7be1d40ab8af sources=36
    oracle: pinned sources clean (git diff --quiet and git status --porcelain are both empty)
    oracle: build Lovelace.Run/Lovelace.Run.csproj -c Release
        Determining projects to restore...
        All projects are up-to-date for restore.
        Lovelace.Abstractions -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Abstractions\bin\Release\net10.0\Lovelace.Abstractions.dll
        Lovelace.Natural -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Natural\bin\Release\net10.0\Lovelace.Natural.dll
        Lovelace.Array -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Array\bin\Release\net10.0\Lovelace.Array.dll
        Lovelace.Integer -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Integer\bin\Release\net10.0\Lovelace.Integer.dll
        Lovelace.Real -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Real\bin\Release\net10.0\Lovelace.Real.dll
        Lovelace.Rational -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Rational\bin\Release\net10.0\Lovelace.Rational.dll
        Lovelace.Complex -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Complex\bin\Release\net10.0\Lovelace.Complex.dll
        Lovelace.Dsp -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Dsp\bin\Release\net10.0\Lovelace.Dsp.dll
        Lovelace.Symbolics -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics\bin\Release\net10.0\Lovelace.Symbolics.dll
        Lovelace.Suite -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite\bin\Release\net10.0\Lovelace.Suite.dll
        Lovelace.MathIR -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.MathIR\bin\Release\net10.0\Lovelace.MathIR.dll
        Lovelace.Run -> C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll

      Build succeeded.
          0 Warning(s)
          0 Error(s)

      Time Elapsed 00:00:01.60
    oracle: build OK
    oracle: runner envelope ok=true protocolVersion=1
    oracle: wrote C:\Users\ricar\dev\LovelaceSharp\parity\build\oracle-verified.json
    oracle: OK commit=1cc28f255eca6029849f880949fc7be1d40ab8af dotnet=10.0.103 dll=Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll
    oracle: dll sha256=f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a

    PS> "ORACLE_FULL_EXIT=$LASTEXITCODE"
    ORACLE_FULL_EXIT=0

MSBuild emits the project lines in a non-deterministic order (parallel build); the block
above is one observed ordering, verbatim. The script prints the build output unchanged and
decides only on the exit code.

`parity/build/oracle-verified.json` as written:

    {
        "schemaVersion":  1,
        "commit":  "1cc28f255eca6029849f880949fc7be1d40ab8af",
        "dll":  "Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll",
        "dllSha256":  "f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a",
        "dotnetVersion":  "10.0.103",
        "verifiedAtUtc":  "2026-11-10T14:46:22Z"
    }

## 7. Run parity/build-oracle.ps1 -Verify (checks only, no build)

    PS> powershell.exe -NoProfile -ExecutionPolicy Bypass -File '.\parity\build-oracle.ps1' -Verify 2>&1
    oracle: repo=C:\Users\ricar\dev\LovelaceSharp
    oracle: head=1cc28f255eca6029849f880949fc7be1d40ab8af pinned=1cc28f255eca6029849f880949fc7be1d40ab8af sources=36
    oracle: pinned sources clean (git diff --quiet and git status --porcelain are both empty)
    oracle: runner envelope ok=true protocolVersion=1
    oracle: OK commit=1cc28f255eca6029849f880949fc7be1d40ab8af dotnet=10.0.103 dll=Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll
    oracle: dll sha256=f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a

    PS> "ORACLE_VERIFY_EXIT=$LASTEXITCODE"
    ORACLE_VERIFY_EXIT=0

## 8. Negative tests of the exit-code contract

(a) unknown pinned commit, run against a throwaway copy in `%TEMP%`:

    oracle: FAIL pinned commit not found in this repository: 0000000000000000000000000000000000000000
    A_UNKNOWN_COMMIT_EXIT=2

(b) a throwaway git repository with a modified tracked file listed in `sources`:

    oracle: repo=C:\Users\ricar\AppData\Local\Temp\oracle-dirty-...
    oracle: head=cb38e9352cb158ed02231120546ed18e3b585530 pinned=cb38e9352cb158ed02231120546ed18e3b585530 sources=1
    oracle: tracked files that differ from the pinned commit:
      M	src/a.txt
    oracle: FAIL working tree differs from cb38e9352cb158ed02231120546ed18e3b585530 in the pinned source directories (git diff exit 1)
    B_DIRTY_EXIT=2

Both throwaway directories were deleted afterwards. The successful path (§6, §7) exits 0.

## 9. Totals

`testMethods=2105` (`factMethods=1922` + `theoryMethods=183`),
`testCases=6209` (1,922 fact cases + 709 inline rows + 3,578 member-data rows),
`dataUnavailable=0`, 15 projects, 281 test classes (all public), 3,998,378 bytes.

Why this differs from the reference 2,087 — in short: 2,087 counts each `[Theory]` once
(1,904 [Fact] text occurrences + 183 [Theory] occurrences). The census expands theory
data rows, as required: 183 theories → 709 inline + 3,578 member rows = 4,287 cases, and
1,922 + 4,287 = 6,209. The +18 on the fact side is −1 (one `[Fact]` is prose in a doc
comment, `Lovelace.Run.Tests/FalsificationGateTests.cs:743`) +19 (methods using the
custom `FactAttribute` subclasses `[RequiresRunnerProcessFact]` ×11 and
`[RequiresSympyFact]` ×8). The full arithmetic is in `parity/census-notes.md` §3.

## 10. What could not be determined / not verified

* **Runtime pass/fail.** `dotnet test` cannot run in this environment (testhost fails
  with Win32Exception 5), so no test was executed. The census is a static/reflection
  inventory. Every case is counted regardless of `Skip`.
* **xunit's own discovery and display names.** The tool re-implements the essentials
  (FactAttribute/TheoryAttribute/DataAttribute subclasses, `DataAttribute.GetData`) but
  not xunit's exact display-name formatter, its theory pre-enumeration checks, or its
  class-visibility filters. All 281 test classes are public and none is abstract, so
  the visibility filter makes no difference here; `displayName` is our deterministic
  rendering, not xunit's string.
* **Skip semantics.** The 19 custom fact methods set `Skip` when `Lovelace.Run.dll` or
  SymPy is missing; they are still counted as cases. Whether they would be skipped at
  runtime is not observable here.
* **Stale assemblies.** The tool fails if an assembly is missing but does not detect a
  stale one; §1 documents the required rebuild.
* **Async source lines.** For async tests the PDB kickoff method has no visible
  sequence point, so `sourceLine` comes from a source scan (561 of 2,105 methods,
  including all of `Lovelace.Studio.Tests` and most of `Lovelace.Run.Tests`). Those
  lines are the declaration line; PDB-derived lines are the first sequence point
  (normally the opening brace).
* **The two MemberData refusals seen mid-implementation** (before the testhost
  environment was emulated) were real and were reported, not hidden: without
  `AppContext.BaseDirectory` pointing at the test output directory,
  `LanguageDocumentationTests.Examples` and `UsageDocumentationTests.Examples` could not
  find `Language.md`/`README.md` and were emitted with `dataUnavailable`. After the fix
  they expand to 111 and 103 rows and the final census has 0 refusals.
