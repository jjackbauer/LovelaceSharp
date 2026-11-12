<#
.SYNOPSIS
  Record a requirement row's status and hash-bound evidence in the desktop v4 gate register.

.DESCRIPTION
  The gate itself never writes PASS: the orchestrator records a row only after personally observing
  the artifact. This helper computes the SHA-256 of each evidence file, stamps the current HEAD, and
  updates one row (or one journey with -Journey).

.EXAMPLE
  powershell -File tools/gate-set.ps1 -Id EDT-001 -Status PASS -Class gui `
      -Test "docs/.../round-2/gui-driver.log" -Evidence "docs/.../shot.png"

.EXAMPLE
  powershell -File tools/gate-set.ps1 -Id TST-012 -Status BLOCKED -Note "waived by DEC-006"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Id,
    [Parameter(Mandatory = $true)][ValidateSet('PASS', 'FAIL', 'BLOCKED', 'NOT_TESTED', 'APPROVED_DEVIATION')][string]$Status,
    [ValidateSet('unit', 'gui', 'perf', 'file', 'static')][string]$Class = 'gui',
    [string[]]$Evidence = @(),
    [string]$Test = '',
    [string]$Note = '',
    [switch]$Journey,
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'
if ($RepoRoot -eq '') {
    $d = $PSScriptRoot
    while ($d -and -not (Test-Path (Join-Path $d '.git'))) { $d = Split-Path $d -Parent }
    $RepoRoot = $d
}
$GateDir = Join-Path $RepoRoot 'docs\goal-desktop-v4\gate'
$RegisterPath = Join-Path $GateDir 'register.json'
$JourneysPath = Join-Path $GateDir 'journeys.json'
$head = (& git -C $RepoRoot rev-parse HEAD).Trim()

function Save-Json($obj, $path) {
    $json = $obj | ConvertTo-Json -Depth 14
    [System.IO.File]::WriteAllText($path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

$entries = @()
foreach ($e in $Evidence) {
    if ($e -eq '') { continue }
    $p = if ([System.IO.Path]::IsPathRooted($e)) { $e } else { Join-Path $RepoRoot $e }
    if (-not (Test-Path -LiteralPath $p)) { throw "evidence file does not exist: $e" }
    $rel = $e.Replace('\', '/')
    $entries += [ordered]@{
        path   = $rel
        sha256 = (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        class  = $Class
        at_head = $head
    }
}

if ($Journey) {
    $doc = Get-Content -LiteralPath $JourneysPath -Raw -Encoding utf8 | ConvertFrom-Json
    $row = @($doc.journeys | Where-Object { $_.id -eq $Id })
    if ($row.Count -ne 1) { throw "journey $Id not found" }
    $row[0].status = $Status
    $row[0].evidence = @($entries)
    if ($Note) { $row[0].notes = $Note }
    Save-Json $doc $JourneysPath
    Write-Output "$Id -> $Status ($($entries.Count) evidence, $(if ($Journey) { 'journey' } else { 'requirement' }))"
    exit 0
}

$doc = Get-Content -LiteralPath $RegisterPath -Raw -Encoding utf8 | ConvertFrom-Json
$row = @($doc.rows | Where-Object { $_.id -eq $Id })
if ($row.Count -ne 1) { throw "requirement $Id not found" }
$row[0].status = $Status
if ($entries.Count -gt 0) { $row[0].evidence = @($entries) }
if ($Test) { $row[0].test = $Test }
if ($Note) { $row[0].scope_note = $Note }
Save-Json $doc $RegisterPath
Write-Output "$Id -> $Status ($($entries.Count) evidence entries, class $Class, HEAD $head)"
