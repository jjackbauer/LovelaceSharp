<#
.SYNOPSIS
  Falsifies the verification system: applies each declared fault, runs the gate that is supposed to
  catch it, and requires that gate to FAIL. A mutation the gate accepts is a defective gate.
.DESCRIPTION
  Mutations come from parity/mutations.json. Three kinds:
    source  - a single-occurrence text replacement in a Rust source file, followed by a release build
    file    - a text replacement in a ledger/data file (no rebuild)
    invoke  - a command that must exit non-zero on its own (e.g. an empty corpus)
  Every mutation is reverted. The script re-verifies the reverted tree still passes the target gate
  at the end, so a failed restore cannot masquerade as a detected mutation.
.PARAMETER Only
  Run only the named mutation id (e.g. GATE-004).
#>
param([string]$Only = '')
$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot
$spec = Get-Content (Join-Path $PSScriptRoot 'mutations.json') -Raw | ConvertFrom-Json
$env:RUSTUP_HOME = Join-Path $repo '.toolchain\rustup'
$env:CARGO_HOME = Join-Path $repo '.toolchain\cargo'
$env:PATH = (Join-Path $repo '.toolchain\cargo\bin') + ';' + $env:PATH
$env:TMP = Join-Path $repo 'rust\target\tmp'; $env:TEMP = $env:TMP
$cargoManifest = Join-Path $repo 'rust\Cargo.toml'

function Run-Gate($name) {
  # Native tools write to stderr by design (node prints its verdict there); under
  # ErrorActionPreference=Stop that would terminate the harness, so run gates with Continue and
  # trust the exit code. A gate name may carry a case filter: "fixtures:error_envelope".
  $gateName = $name
  $gateFilter = ''
  if ($name.Contains(':')) { $gateName = $name.Split(':')[0]; $gateFilter = $name.Split(':')[1] }
  $previous = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
  switch ($gateName) {
    'fixtures'   { if ($gateFilter) { & node (Join-Path $PSScriptRoot 'tools\gate-fixtures.mjs') ('--filter=' + $gateFilter) *> $null } else { & node (Join-Path $PSScriptRoot 'tools\gate-fixtures.mjs') *> $null }; return $LASTEXITCODE }
    'cli'        { if ($gateFilter) { & node (Join-Path $PSScriptRoot 'tools\gate-cli.mjs') ('--filter=' + $gateFilter) *> $null } else { & node (Join-Path $PSScriptRoot 'tools\gate-cli.mjs') *> $null }; return $LASTEXITCODE }
    'census'     { & (Join-Path $PSScriptRoot 'gate-census.ps1') *> $null; return $LASTEXITCODE }
    'manifest'   { & (Join-Path $PSScriptRoot 'gate-manifest.ps1') *> $null; return $LASTEXITCODE }
    'projects'   { & (Join-Path $PSScriptRoot 'gate-projects.ps1') *> $null; return $LASTEXITCODE }
    'corpus'     { & cargo run --release --manifest-path $cargoManifest -p lovelace-corpus *> $null; return $LASTEXITCODE }
    'diff-empty' { Push-Location $repo; & node (Join-Path $repo 'parity\diff\run.mjs') --cases 0 *> $null; $c = $LASTEXITCODE; Pop-Location; return $c }
    'cargo-test' { & cargo test --manifest-path $cargoManifest --workspace *> $null; return $LASTEXITCODE }
    default      { throw "unknown gate '$name'" }
  }
  } finally {
    $ErrorActionPreference = $previous
  }
}

function Build-Release {
  $previous = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  try {
    & cargo build --release --manifest-path $cargoManifest -p lovelace-run *> $null
    if ($LASTEXITCODE -ne 0) { throw 'release build failed after mutation' }
  } finally {
    $ErrorActionPreference = $previous
  }
}

$selected = @($spec.mutations | Where-Object { ($Only -eq '') -or ($_.id -eq $Only) })
if ($selected.Count -eq 0) { Write-Error "no mutation matched '$Only'"; exit 2 }

$results = @()
foreach ($m in $selected) {
  $row = [pscustomobject]@{ id = $m.id; title = $m.title; gate = $m.gate; detected = $false; restored = $false; detail = '' }
  Write-Output ("== " + $m.id + ': ' + $m.title)
  # An invoke mutation IS the invocation (e.g. an empty corpus), so there is no precondition.
  if ($m.kind -eq 'invoke') {
    $code = Run-Gate $m.gate
    $row.detected = ($code -ne 0)
    $row.restored = $true
    $row.detail = "$($m.gate) exit $code"
    if (-not $row.detected) { Write-Output ("   NOT DETECTED: " + $row.detail) }
    $results += $row
    continue
  }
  # Otherwise a mutation proves nothing against a gate that is already red.
  $pre = Run-Gate $m.gate
  if ($pre -ne 0) {
    $row.detail = "NOT VALIDATED: $($m.gate) was already red before the mutation"
    Write-Output ('   ' + $row.detail)
    $results += $row
    continue
  }

  $target = Join-Path $repo $m.target
  if (-not (Test-Path $target)) { $row.detail = 'target missing: ' + $m.target; $results += $row; continue }
  $original = [System.IO.File]::ReadAllText($target)
  $count = ([regex]::Matches($original, [regex]::Escape($m.find))).Count
  if ($count -ne 1) {
    $row.detail = ('anchor occurs ' + $count + ' times (need exactly 1): ' + $m.find)
    Write-Output ('   ANCHOR DEFECT: ' + $row.detail)
    $results += $row
    continue
  }
  try {
    [System.IO.File]::WriteAllText($target, $original.Replace($m.find, $m.replace))
    if ($m.kind -eq 'source') { Build-Release }
    $code = Run-Gate $m.gate
    $row.detected = ($code -ne 0)
    $row.detail = "$($m.gate) exit $code"
  } catch {
    $row.detail = 'exception: ' + $_.Exception.Message
  } finally {
    [System.IO.File]::WriteAllText($target, $original)
    if ($m.kind -eq 'source') { Build-Release }
    $restoreCode = Run-Gate $m.gate
    $row.restored = ($restoreCode -eq 0)
  }
  if (-not $row.detected) { Write-Output ('   NOT DETECTED: ' + $row.detail) }
  if (-not $row.restored) { Write-Output '   RESTORE DEFECT: the reverted tree does not pass the gate' }
  $results += $row
}

Write-Output ''
$detected = @($results | Where-Object { $_.detected }).Count
$restored = @($results | Where-Object { $_.restored }).Count
Write-Output ('mutations: ' + $detected + '/' + $results.Count + ' detected; ' + $restored + ' reverted trees pass')
foreach ($r in $results) { Write-Output ('  ' + $r.id + ' detected=' + $r.detected + ' restored=' + $r.restored + ' - ' + $r.detail) }
if (($detected -eq $results.Count) -and ($restored -eq $results.Count)) { exit 0 }
exit 1
