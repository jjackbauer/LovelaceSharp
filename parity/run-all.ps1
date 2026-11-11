<#
.SYNOPSIS
  Runs every parity gate and writes parity/scorecard.json.
.DESCRIPTION
  Fail-closed: a dimension whose script is missing, whose command cannot start, or whose exit code is
  non-zero is FAILED. Nothing is inferred from an earlier run. The scorecard is the only summary the
  release decision may read.

  Dimensions: D1 oracle, D2 census, D3 ledgers, D4 differential, D5 fixtures, D6 CLI, D7 mutation,
  D9 documentation corpus, D10 Rust quality. (D8 is the reproducibility of this script itself: it
  always regenerates the scorecard from live commands.)
.PARAMETER Cases
  Differential cases for this run (default 200; the release gate demands 5000 for a PR slice).
.PARAMETER SkipMutate
  Skip the mutation gate (its rebuilds are expensive); the scorecard records it SKIPPED, never PASS.
#>
param([int]$Cases = 200, [switch]$SkipMutate)
$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot
$results = @()
$started = Get-Date
$nl = [string][char]10

function First-LineOrLast($output) {
  $lines = @($output -split '\r?\n' | Where-Object { $_.Trim().Length -gt 0 })
  # Prefer a verdict-looking line over trailing stderr noise.
  $verdict = @($lines | Where-Object { $_ -match '^(PASS|FAIL|rust:|oracle:|CLI matrix:|corpus:|mutations:)' })
  if ($verdict.Count -gt 0) { return $verdict[$verdict.Count - 1] }
  $errorLine = @($lines | Where-Object { $_ -match '^error(\[|:)' })
  if ($errorLine.Count -gt 0) { return $errorLine[0] }
  if ($lines.Count -gt 0) { return $lines[$lines.Count - 1] }
  return '(no output)'
}

function Invoke-NodeGate($id, $title, $script, $argList) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  if (-not (Test-Path $script)) {
    $script:results += [pscustomobject]@{ id = $id; title = $title; status = 'FAILED'; exitCode = $null; seconds = 0; command = $script; summary = 'gate script missing' }
    return
  }
  $output = & node $script @argList 2>&1 | Out-String
  $code = $LASTEXITCODE
  $sw.Stop()
  $script:results += [pscustomobject]@{ id = $id; title = $title; status = $(if ($code -eq 0) { 'PASS' } else { 'FAILED' }); exitCode = $code; seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = ('node ' + $script); summary = (First-LineOrLast $output) }
}

function Invoke-Node($id, $title, $script, $argList) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  if (-not (Test-Path $script)) {
    $script:results += [pscustomobject]@{ id = $id; title = $title; status = 'FAILED'; exitCode = $null; seconds = 0; command = $script; summary = 'script missing' }
    return
  }
  Push-Location $script:repo
  $output = & node $script @argList 2>&1 | Out-String
  $code = $LASTEXITCODE
  Pop-Location
  $sw.Stop()
  $script:results += [pscustomobject]@{ id = $id; title = $title; status = $(if ($code -eq 0) { 'PASS' } else { 'FAILED' }); exitCode = $code; seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = ('node ' + $script + ' ' + ($argList -join ' ')); summary = (First-LineOrLast $output) }
}

$parityDir = $PSScriptRoot
$toolsDir = Join-Path $parityDir 'tools'

# D1 runs a PowerShell script (it drives git and dotnet); call it with an explicit switch.
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$d1 = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $parityDir 'build-oracle.ps1') -Verify 2>&1 | Out-String
$d1code = $LASTEXITCODE
$sw.Stop()
$results += [pscustomobject]@{ id = 'D1'; title = 'Oracle frozen and reproducible'; status = $(if ($d1code -eq 0) { 'PASS' } else { 'FAILED' }); exitCode = $d1code; seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = 'build-oracle.ps1 -Verify'; summary = (First-LineOrLast $d1) }

Invoke-NodeGate 'D2' 'C# test census complete' (Join-Path $toolsDir 'gate-census.mjs') @()

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$d3ok = $true; $d3sum = @()
foreach ($g in @('gate-manifest.mjs','gate-projects.mjs','gate-requirements.mjs')) {
  $p = Join-Path $toolsDir $g
  if (-not (Test-Path $p)) { $d3ok = $false; $d3sum += ($g + ': missing'); continue }
  $out = & node $p 2>&1 | Out-String
  if ($LASTEXITCODE -ne 0) { $d3ok = $false }
  $d3sum += ($g + ': ' + (First-LineOrLast $out))
}
$sw.Stop()
$results += [pscustomobject]@{ id = 'D3'; title = 'Census manifest, project ledger, requirement ledger'; status = $(if ($d3ok) { 'PASS' } else { 'FAILED' }); exitCode = $(if ($d3ok) { 0 } else { 1 }); seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = 'gate-manifest + gate-projects + gate-requirements'; summary = ($d3sum -join ' | ') }

# --no-minimize: the volume gate compares every case; minimisation is a per-mismatch debugging tool
# (exercised by the harness's own runs and selftest) and would multiply a bulk run's cost by ~40x.
Invoke-Node 'D4' ('Differential (' + $Cases + ' cases)') (Join-Path $repo 'parity\diff\run.mjs') @('--cases', "$Cases", '--seed', '1', '--no-minimize', '--concurrency', '10')
Invoke-NodeGate 'D5' '20 protocol golden fixtures' (Join-Path $toolsDir 'gate-fixtures.mjs') @()
Invoke-NodeGate 'D6' 'RUN-001..014 CLI surface' (Join-Path $toolsDir 'gate-cli.mjs') @()
if ($SkipMutate) {
  $results += [pscustomobject]@{ id = 'D7'; title = 'Gate falsification (GATE-001..012)'; status = 'SKIPPED'; exitCode = $null; seconds = 0; command = 'mutate.ps1'; summary = 'skipped by request; never counted as pass' }
} else {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $d7 = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $parityDir 'mutate.ps1') 2>&1 | Out-String
  $d7code = $LASTEXITCODE
  $sw.Stop()
  $results += [pscustomobject]@{ id = 'D7'; title = 'Gate falsification (GATE-001..012)'; status = $(if ($d7code -eq 0) { 'PASS' } else { 'FAILED' }); exitCode = $d7code; seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = 'mutate.ps1'; summary = (First-LineOrLast $d7) }
}

$env:RUSTUP_HOME = Join-Path $repo '.toolchain\rustup'
$env:CARGO_HOME = Join-Path $repo '.toolchain\cargo'
$env:PATH = (Join-Path $repo '.toolchain\cargo\bin') + ';' + $env:PATH
$env:TMP = Join-Path $repo 'rust\target\tmp'; $env:TEMP = $env:TMP
$cargoManifest = Join-Path $repo 'rust\Cargo.toml'

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$corpus = & cargo run --release --manifest-path $cargoManifest -p lovelace-corpus 2>&1 | Out-String
$corpusCode = $LASTEXITCODE
$sw.Stop()
$corpusLine = @($corpus -split '\r?\n' | Where-Object { $_ -match '^corpus:' })
$corpusSummary = '(no corpus line)'
if ($corpusLine.Count -gt 0) { $corpusSummary = $corpusLine[$corpusLine.Count - 1] }
$results += [pscustomobject]@{ id = 'D9'; title = '214-example documentation corpus'; status = $(if ($corpusCode -eq 0) { 'PASS' } else { 'FAILED' }); exitCode = $corpusCode; seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = 'cargo run --release -p lovelace-corpus'; summary = $corpusSummary }

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$fmtOut = & cargo fmt --manifest-path $cargoManifest --all -- --check 2>&1 | Out-String
$fmtCode = $LASTEXITCODE
$clippyOut = & cargo clippy --manifest-path $cargoManifest --workspace --all-targets -- -D warnings 2>&1 | Out-String
$clippyCode = $LASTEXITCODE
$sw.Stop()
# A missing component is not a code failure; it is an unverifiable check and must say so.
$toolMissing = (($fmtOut + $clippyOut) -match 'is not installed')
$qualityOk = ($fmtCode -eq 0) -and ($clippyCode -eq 0)
$qualitySummary = 'fmt exit ' + $fmtCode + ', clippy exit ' + $clippyCode
if ($toolMissing) { $qualitySummary = 'UNAVAILABLE: rustfmt and/or clippy are not installed for the pinned toolchain; the check was not performed' }
$results += [pscustomobject]@{ id = 'D10'; title = 'fmt --check and clippy -D warnings'; status = $(if ($qualityOk) { 'PASS' } else { 'FAILED' }); exitCode = $(if ($qualityOk) { 0 } else { 1 }); seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1); command = 'cargo fmt --check; cargo clippy -D warnings'; summary = $qualitySummary }

$finished = Get-Date
$passed = @($results | Where-Object { $_.status -eq 'PASS' }).Count
$failed = @($results | Where-Object { $_.status -eq 'FAILED' }).Count
$skipped = @($results | Where-Object { $_.status -eq 'SKIPPED' }).Count
$oracleCommit = (Get-Content (Join-Path $repo 'parity\oracle.json') -Raw | ConvertFrom-Json).commit
$dotnetVersion = (& dotnet --version)
$cargoVersion = (& cargo --version)
$scorecard = [pscustomobject]@{
  schemaVersion = 1
  oracle = $oracleCommit
  startedAtUtc = $started.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  finishedAtUtc = $finished.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  host = $env:COMPUTERNAME
  dotnet = $dotnetVersion
  cargo = $cargoVersion
  differentialCases = $Cases
  totals = [pscustomobject]@{ pass = $passed; fail = $failed; skipped = $skipped; dimensions = $results.Count }
  dimensions = $results
}
$outPath = Join-Path $repo 'parity\scorecard.json'
$scorecard | ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 $outPath
Write-Output ''
Write-Output '  ID   STATUS    SEC   DIMENSION'
foreach ($r in $results) {
  Write-Output ('  {0,-4} {1,-9} {2,4}   {3}' -f $r.id, $r.status, $r.seconds, $r.title)
  Write-Output ('       -- {0}' -f $r.summary)
}
Write-Output ''
Write-Output ('scorecard: ' + $outPath + '  (pass=' + $passed + ' fail=' + $failed + ' skipped=' + $skipped + ')')
if ($failed -gt 0) { exit 1 }
exit 0
