<#
.SYNOPSIS
  Round-33 headless evidence: rebuild lovelace-desktop and run the five mandatory journeys
  (J-02, J-03, J-09, J-10, J-15) through the shipped `--journey` mode.

.DESCRIPTION
  The journey mode drives the same Workbench/ExecutionHost path the window owns and the same Plot
  pane the window paints. It prints each observed value (result, variables with kinds, revision,
  reused count, diagnostics, plot census, exported SVG hash) and exits non-zero when the expected
  observable in docs/goal-desktop-v4/journeys.md does not match.

  APPDATA is redirected to this round's appdata folder and the process current directory is the
  round's work folder, so the resolved plot directory is writable and is not below the working
  directory. The script cleans the three evidence directories first, so the run is reproducible.

  Windows PowerShell 5.1 compatible. ASCII-only source.
#>
[CmdletBinding()]
param(
    [string]$Repo = '',
    [string]$OutDir = ''
)
$ErrorActionPreference = 'Stop'

if (-not $OutDir) { $OutDir = $PSScriptRoot }
if (-not $Repo) { $Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path }
$Cargo = Join-Path $Repo '.toolchain\cargo\bin\cargo.exe'
$Manifest = Join-Path $Repo 'rust\Cargo.toml'
$Exe = Join-Path $Repo 'rust\target-desktop\debug\lovelace-desktop.exe'
if (-not (Test-Path -LiteralPath $Cargo)) { throw "cargo not found: $Cargo" }
if (-not (Test-Path -LiteralPath $Manifest)) { throw "manifest not found: $Manifest" }

$Work = Join-Path $OutDir 'work'
$AppData = Join-Path $OutDir 'appdata'
$JourneyOut = Join-Path $OutDir 'journey-out'
foreach ($directory in @($Work, $AppData, $JourneyOut)) {
    if (Test-Path -LiteralPath $directory) { Remove-Item -LiteralPath $directory -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
}
$Log = Join-Path $OutDir 'journey-run.log'
$BuildLog = Join-Path $OutDir 'build.log'
Get-ChildItem -LiteralPath $OutDir -Filter 'J-09-run*.svg' -File -ErrorAction SilentlyContinue | Remove-Item -Force
Get-ChildItem -LiteralPath $OutDir -Filter 'J-15-export.svg' -File -ErrorAction SilentlyContinue | Remove-Item -Force

$env:RUSTUP_HOME = Join-Path $Repo '.toolchain\rustup'
$env:CARGO_HOME = Join-Path $Repo '.toolchain\cargo'
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
$env:CARGO_TARGET_DIR = Join-Path $Repo 'rust\target-desktop'
$env:APPDATA = $AppData
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null

Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

# Windows PowerShell 5.1 turns a native program's stderr into a terminating error when
# $ErrorActionPreference is Stop, and cargo writes its progress there. cmd.exe owns both
# redirections instead, so the exit code is the program's and the log is complete.
Write-Host "build: $Cargo build -p lovelace-desktop --manifest-path $Manifest"
$buildCommand = '"{0}" build -p lovelace-desktop --manifest-path "{1}" > "{2}" 2>&1' -f $Cargo, $Manifest, $BuildLog
& cmd.exe /d /c $buildCommand
$buildExit = $LASTEXITCODE
Get-Content -LiteralPath $BuildLog | ForEach-Object { Write-Host $_ }
if ($buildExit -ne 0) { throw "cargo build failed with exit code $buildExit" }
if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found after the build: $Exe" }

$journeyCommand = '"{0}" --journey J-02 --journey J-03 --journey J-09 --journey J-10 --journey J-15 --out "{1}" > "{2}" 2>&1' -f $Exe, $JourneyOut, $Log
Write-Host ("journeys: {0} (log {1})" -f $Exe, $Log)
& cmd.exe /d /c $journeyCommand
$journeyExit = $LASTEXITCODE
Get-Content -LiteralPath $Log | ForEach-Object { Write-Host $_ }
Write-Host ("journey exit code: {0}" -f $journeyExit)
if ($journeyExit -ne 0) { exit $journeyExit }
exit 0
