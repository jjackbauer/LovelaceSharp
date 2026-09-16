<#
.SYNOPSIS
  Provision and validate the Lovelace agentic-harnessability benchmark.

.DESCRIPTION
  Reproduces, from a clean checkout, everything the benchmark needs:
    1. a private Python runtime        -> tools/python
    2. a Native AOT Lovelace.Run       -> runner/Lovelace.Run.exe
    3. the evaluator corpus            -> private/goldens.json, private/probes.json
    4. oracle fingerprints             -> private/oracles.json
    5. the full corpus/protocol validation (bench.py validate)
    6. MCP server module links for the DSH integration

  Nothing here touches the agent-visible surface: private/ is evaluator-owned.
#>
param(
  [string]$Configuration = 'Release',
  [switch]$SkipPython,
  [switch]$SkipBuild
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$repo = Split-Path $root -Parent
$py   = Join-Path $root 'tools\python\python.exe'
$runner = Join-Path $root 'runner\Lovelace.Run.exe'

Write-Host '== 1. Python runtime ==' -ForegroundColor Cyan
if ($SkipPython) {
  Write-Host 'skipped'
} elseif (Test-Path $py) {
  Write-Host "present: $py"
} else {
  $tools = Join-Path $root 'tools'
  New-Item -ItemType Directory -Force -Path $tools | Out-Null
  $zip = Join-Path $tools 'python-embed.zip'
  $url = 'https://www.python.org/ftp/python/3.12.10/python-3.12.10-embed-amd64.zip'
  Write-Host "downloading $url"
  Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
  Expand-Archive -Path $zip -DestinationPath (Join-Path $tools 'python') -Force
  Remove-Item $zip
  Write-Host "installed: $py"
}

Write-Host '== 2. Lovelace.Run (Native AOT) ==' -ForegroundColor Cyan
if ($SkipBuild) {
  Write-Host 'skipped'
} elseif (Test-Path $runner) {
  Write-Host "present: $runner"
} else {
  & dotnet publish (Join-Path $repo 'Lovelace.Run\Lovelace.Run.csproj') `
      --configuration $Configuration --framework net10.0 `
      -p:PublishAot=true -p:InvariantGlobalization=true -r win-x64 `
      --output (Join-Path $root 'runner')
  if ($LASTEXITCODE -ne 0) { throw 'Lovelace.Run publish failed' }
}

Write-Host '== 3+4. Private corpus and oracle fingerprints ==' -ForegroundColor Cyan
& $py (Join-Path $root 'lovelace-harnessability-public-branch\private\build_corpus.py') --check --runner $runner
if ($LASTEXITCODE -ne 0) { throw 'corpus self-check failed' }
& $py (Join-Path $root 'lovelace-harnessability-public-branch\bench.py') prepare --runner $runner
if ($LASTEXITCODE -ne 0) { throw 'bench.py prepare failed' }

Write-Host '== 5. Corpus/protocol validation ==' -ForegroundColor Cyan
& $py (Join-Path $root 'lovelace-harnessability-public-branch\bench.py') validate --runner $runner
if ($LASTEXITCODE -ne 0) { throw 'bench.py validate failed' }

Write-Host '== 6. DSH integration links ==' -ForegroundColor Cyan
$mcpNm = Join-Path $root 'dsh\bench-mcp\node_modules'
$src = Join-Path $env:USERPROFILE '.dsh\profiles\node_modules'
New-Item -ItemType Directory -Force -Path (Join-Path $mcpNm '@modelcontextprotocol') | Out-Null
foreach ($pair in @(@("@modelcontextprotocol\sdk", 'sdk'), @('zod', 'zod'))) {
  $target = Join-Path $src $pair[0]
  $link = if ($pair[1] -eq 'sdk') { Join-Path $mcpNm $pair[0] } else { Join-Path $mcpNm $pair[0] }
  if (-not (Test-Path $target)) { throw "missing DSH profile package: $target" }
  if (Test-Path $link) { Remove-Item -Recurse -Force $link }
  New-Item -ItemType Junction -Path $link -Target $target | Out-Null
  Write-Host "linked $link"
}

Write-Host ''
Write-Host 'Harnessability benchmark ready.' -ForegroundColor Green
Write-Host '  corpus validation : private/validation-latest.json (re-run bench.py validate)'
Write-Host '  run the matrix    : ./run-bench.ps1 -Modes L0,L1,L2,L3'
