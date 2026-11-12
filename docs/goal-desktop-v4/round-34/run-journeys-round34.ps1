<#
.SYNOPSIS
  Round-34 `--journey` evidence: the DEF-004 default-output run, the suite in a writable
  app-scoped plot directory, and the default-environment control run.

.DESCRIPTION
  Three runs, each stdout saved unedited:

   1. journey-default-env.log - J-09 with the machine's default APPDATA. This is the control:
      the child process cannot write outside the workspace, so the engine cannot create
      %APPDATA%\Lovelace\plots\run-1 and J-09 fails at the plot statement, before the copy path
      the DEF-004 fix covers. The exact failing check is quoted in the log.
   2. journey-j09-default-out.log - J-09 with APPDATA redirected to this round's writable
      app-scoped directory (outside the process cwd) and NO --out. The journey must create its
      per-run output directory itself; this is the DEF-004 fix.
   3. journey-run.log - all five journeys with --out this round's journey-out. Every journey
      must pass.

  Windows PowerShell 5.1 compatible. ASCII-only source.
#>
[CmdletBinding()]
param(
    [string]$Repo = 'C:\Users\ricar\dev\LovelaceSharp',
    [string]$OutDir = ''
)
$ErrorActionPreference = 'Stop'
if (-not $OutDir) { $OutDir = $PSScriptRoot }
$Exe = Join-Path $Repo 'rust\target-desktop\debug\lovelace-desktop.exe'
if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }

$Work = Join-Path $OutDir 'work'
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$AppData = Join-Path $OutDir 'appdata-journey'
if (Test-Path -LiteralPath $AppData) { Remove-Item -LiteralPath $AppData -Recurse -Force }
New-Item -ItemType Directory -Force -Path $AppData | Out-Null
$JourneyOut = Join-Path $OutDir 'journey-out'
if (Test-Path -LiteralPath $JourneyOut) { Remove-Item -LiteralPath $JourneyOut -Recurse -Force }
New-Item -ItemType Directory -Force -Path $JourneyOut | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

$DefaultAppData = [Environment]::GetFolderPath('ApplicationData')
$results = @()

function Write-Log([string]$path, [string[]]$lines) {
    Set-Content -LiteralPath $path -Value $lines -Encoding UTF8
}

Write-Host "EXE=$Exe"
Write-Host "DefaultAppData=$DefaultAppData"
Write-Host "RedirectedAppData=$AppData"
Write-Host "Work=$Work"

# 1. Default environment: the machine APPDATA. No redirect.
$env:APPDATA = $DefaultAppData
$log1 = @()
$log1 += "# default environment: APPDATA=$DefaultAppData cwd=$Work out=<default>"
$log1 += (& $Exe --journey J-09 2>&1 | ForEach-Object { $_.ToString() })
$exit1 = $LASTEXITCODE
$log1 += "EXIT=$exit1"
Write-Log (Join-Path $OutDir 'journey-default-env.log') $log1
$results += [pscustomobject]@{ name = 'default-env-J-09'; exit = $exit1 }
Write-Host "default-env J-09 EXIT=$exit1"

# 2. Writable app-scoped plot path, default --out: DEF-004 must create the per-run directory.
$env:APPDATA = $AppData
$log2 = @()
$log2 += "# writable app-scoped APPDATA=$AppData cwd=$Work out=<default>"
$log2 += (& $Exe --journey J-09 2>&1 | ForEach-Object { $_.ToString() })
$exit2 = $LASTEXITCODE
$log2 += "EXIT=$exit2"
Write-Log (Join-Path $OutDir 'journey-j09-default-out.log') $log2
$results += [pscustomobject]@{ name = 'j09-default-out'; exit = $exit2 }
Write-Host "J-09 default-out EXIT=$exit2"

# 3. The five journeys, writable app-scoped plot path, explicit evidence directory.
$log3 = @()
$log3 += "# all journeys: APPDATA=$AppData cwd=$Work out=$JourneyOut"
$log3 += (& $Exe --journey J-02 --journey J-03 --journey J-09 --journey J-10 --journey J-15 --out $JourneyOut 2>&1 | ForEach-Object { $_.ToString() })
$exit3 = $LASTEXITCODE
$log3 += "EXIT=$exit3"
Write-Log (Join-Path $OutDir 'journey-run.log') $log3
$results += [pscustomobject]@{ name = 'five-journeys'; exit = $exit3 }
Write-Host "five journeys EXIT=$exit3"

$summary = [pscustomobject]@{
    exe = $Exe
    exe_sha256 = (Get-FileHash -LiteralPath $Exe -Algorithm SHA256).Hash
    default_appdata = $DefaultAppData
    redirected_appdata = $AppData
    cwd = $Work
    journey_out = $JourneyOut
    results = $results
}
$json = ConvertTo-Json -InputObject $summary -Depth 5
[System.IO.File]::WriteAllText((Join-Path $OutDir 'journey-summary.json'), $json, (New-Object System.Text.UTF8Encoding($false)))

if ($exit2 -ne 0 -or $exit3 -ne 0) { exit 1 }
exit 0


