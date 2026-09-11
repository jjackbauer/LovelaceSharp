# Cycle-4 DEC-006: rebuild stages 3-5 with Lovelace.Symbolics/Algebra/RationalFunctions.cs moved
# from stage 4 into stage 3, so that stage 3 (the P0 wire revisions) BUILDS ON ITS OWN (EVD-173).
#
# This runs entirely inside a detached rewrite worktree. The main working tree is never touched.
# The gate is a tree comparison: the rewritten stage-5 commit must have EXACTLY the same tree as
# the old 3992d80, which is what makes it safe to move the branch underneath a dirty working tree.
$ErrorActionPreference = 'Stop'
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$r = 'C:\Users\ricar\dev\.lovelace-rewrite'
Set-Location $repo
if (Test-Path $r) { git worktree remove --force $r 2>$null | Out-Null }
git worktree add --detach $r 84a6a54 | Out-Null
Set-Location $r

$stage3 = @(
  'Lovelace.Abstractions/Diagnostic.cs','Lovelace.Abstractions/EnumValue.cs',
  'Lovelace.MathIR/Evaluator.cs','Lovelace.Suite/PayloadMap.cs','Lovelace.Suite/RecordSchemas.cs',
  'Lovelace.Suite/StructuralEquality.cs','Lovelace.Suite/StructuredProjection.cs','Lovelace.Suite/Value.cs',
  'Lovelace.Suite/ValueFormatter.cs','Lovelace.Suite/Interpreter.cs','Lovelace.Studio/IncrementalRunner.cs',
  'Lovelace.Studio.Tests/StructuredPayloadTests.cs','Lovelace.Suite.Tests/EnumPayloadSeamTests.cs',
  'Lovelace.Suite.Tests/RecordSchemaTests.cs','Lovelace.Suite.Tests/CancellationObservationTests.cs',
  'Lovelace.Symbolics/Solvers/Solve.cs','Lovelace.Symbolics/Simplify.cs','Lovelace.Symbolics/SymbolicsPlugin.cs',
  'Lovelace.Symbolics/Algebra/RationalFunctions.cs',
  'Lovelace.Symbolics.Tests/DiagnosticContractTests.cs','Lovelace.Symbolics.Tests/EnumValueEmissionTests.cs',
  'Lovelace.Symbolics.Tests/SolveCompletenessMappingTests.cs','Lovelace.Symbolics.Tests/SolveCompletenessPairingTests.cs',
  'Lovelace.Symbolics.Tests/DxSemanticClosureTests.cs','Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs',
  'Lovelace.Symbolics.Tests/RewriteProtocolTests.cs','Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs',
  'docs/symbolics/dsh-protocol.md'
)
$stage4 = @(
  'Lovelace.Abstractions/BuiltinDescriptor.cs','Lovelace.Console.Tests/ReplOutputTests.cs',
  'Lovelace.Real.Tests/RealAsyncLocalTests.cs','Lovelace.Real.Tests/RealParseTests.cs','Lovelace.Real.Tests/RealPiTests.cs',
  'Lovelace.Real/Real.cs','Lovelace.Representation.Tests/DigitStoreSnapshotDigitsTests.cs','Lovelace.Representation/DigitStore.cs',
  'Lovelace.Run/Program.cs','Lovelace.Run/RunProtocol.cs','Lovelace.Run/Runner.cs',
  'Lovelace.Suite.Tests/EmptyReductionTests.cs','Lovelace.Suite.Tests/ModusArityTests.cs','Lovelace.Suite/ModusHost.cs',
  'Lovelace.Symbolics.Tests/DifferentialOracleTests.cs','Lovelace.Symbolics.Tests/FalsificationTests.cs',
  'Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj','Lovelace.Symbolics.Tests/AbsSymbolicRoundTripTests.cs',
  'Lovelace.Symbolics.Tests/AssumptionAddScalingTests.cs','Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs',
  'Lovelace.Symbolics.Tests/FalsificationGateTests.cs','Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs',
  'Lovelace.Symbolics.Tests/LatexPrinterTests.cs','Lovelace.Symbolics.Tests/LatexSymbolNameEscapingTests.cs',
  'Lovelace.Symbolics.Tests/MetamorphicSetClosureTests.cs','Lovelace.Symbolics.Tests/OracleCorpus.cs',
  'Lovelace.Symbolics.Tests/SympyOracle.cs','Lovelace.Symbolics/Assumptions.cs'
)
$enc = New-Object System.Text.UTF8Encoding($false)
$mfile = Join-Path $env:TEMP 'cycle4-commit-msg.txt'

function Commit-From([string]$oldCommit, [string[]]$paths, [hashtable]$overrides) {
  foreach ($p in $paths) { git checkout $oldCommit -- $p 2>$null }
  # A path that is NEW to this stage cannot come from this stage's own commit; it has to be taken
  # from the commit where its change actually lives (see the RationalFunctions.cs override below).
  if ($overrides) { foreach ($k in $overrides.Keys) { git checkout $overrides[$k] -- $k 2>$null } }
  foreach ($p in $paths) { git add $p 2>$null }
  if ($overrides) { foreach ($k in $overrides.Keys) { git add $k 2>$null } }
  $body = (git log -1 --format=%B $oldCommit) -join "`n"
  [System.IO.File]::WriteAllText($mfile, $body, $enc)
  git commit -q -F $mfile
  return (git rev-parse --short HEAD)
}

$new3 = Commit-From '160ba4e' $stage3 @{ 'Lovelace.Symbolics/Algebra/RationalFunctions.cs' = '3367447' }
$new4 = Commit-From '3367447' $stage4 $null
# stage 5 is everything else the old tip carried
git checkout 3992d80 -- Lovelace.Run.Tests LovelaceSharp.slnx .github/workflows/ci.yml 2>$null
git add Lovelace.Run.Tests LovelaceSharp.slnx .github/workflows/ci.yml 2>$null
$body = (git log -1 --format=%B 3992d80) -join "`n"
[System.IO.File]::WriteAllText($mfile, $body, $enc)
git commit -q -F $mfile
$new5 = git rev-parse HEAD

Set-Location $repo
$oldTree = (git rev-parse '3992d80^{tree}').Trim()
$newTree = (git rev-parse "$new5^{tree}").Trim()
"new stage3 = $new3"
"new stage4 = $new4"
"new stage5 = $new5"
"old tree   = $oldTree"
"new tree   = $newTree"
if ($oldTree -ne $newTree) { "TREE MISMATCH - REFUSING TO MOVE main"; exit 1 }
"TREES IDENTICAL - safe to move refs/heads/main"
git update-ref refs/heads/main $new5
"main is now $(git rev-parse --short main); worktree status:"
(git status --porcelain | Measure-Object).Count
git log --oneline 35e2609..main | Out-String
git worktree remove --force $r 2>$null | Out-Null
"rewrite worktree removed"