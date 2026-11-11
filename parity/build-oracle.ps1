<#
    parity/build-oracle.ps1

    Verifies and builds the frozen C# oracle described by parity/oracle.json.

      (no switch)  verify the pinned sources, build Lovelace.Run in Release, execute the runner
                   once (--eval "1/3" --json), validate the JSON envelope, and write
                   parity/build/oracle-verified.json.
      -Verify      checks only: verify the pinned sources, require the already-built Release
                   runner, validate its envelope, print the DLL SHA-256 and exit 0. No build.

    Exit codes: 0 success, 1 build/runner/artifact failure, 2 the C# sources differ from the
    pinned commit (or the pinned commit is unknown to this repository).

    Windows PowerShell 5.1 compatible: no ternary operator, no null-coalescing operator.
#>
[CmdletBinding()]
param(
    [switch]$Verify
)

$ErrorActionPreference = 'Stop'

$script:LastNativeExit = 0

function Stop-WithCode {
    param([int]$Code, [string]$Message)
    [Console]::Error.WriteLine('oracle: FAIL ' + $Message)
    exit $Code
}

# Runs a native command without letting its stderr terminate the script under
# $ErrorActionPreference = 'Stop' (Windows PowerShell 5.1 turns redirected native stderr into a
# terminating NativeCommandError). Returns the captured output and sets $script:LastNativeExit.
function Invoke-Native {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [switch]$DiscardStdErr
    )

    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        if ($DiscardStdErr) {
            $output = & $FilePath @Arguments 2>$null
        } else {
            $output = & $FilePath @Arguments 2>&1
        }
        $script:LastNativeExit = $LASTEXITCODE
        return ,@($output)
    } finally {
        $ErrorActionPreference = $previous
    }
}

$parityDir  = $PSScriptRoot
$repoRoot   = Split-Path -Parent $parityDir
$oraclePath = Join-Path $parityDir 'oracle.json'

if (-not (Test-Path -LiteralPath $oraclePath)) {
    Stop-WithCode 2 ('missing ' + $oraclePath)
}

$oracle = Get-Content -LiteralPath $oraclePath -Raw | ConvertFrom-Json

$commit = [string]$oracle.commit
if ($commit -notmatch '^[0-9a-f]{40}$') {
    Stop-WithCode 2 ('oracle.json commit is not a 40-hex SHA: ' + $commit)
}

$sources = @($oracle.sources)
if ($sources.Count -eq 0) {
    Stop-WithCode 2 'oracle.json lists no source directories'
}

$null = Invoke-Native 'git' @('-C', $repoRoot, 'cat-file', '-e', ($commit + '^{commit}'))
if ($script:LastNativeExit -ne 0) {
    Stop-WithCode 2 ('pinned commit not found in this repository: ' + $commit)
}

$headLines = Invoke-Native 'git' @('-C', $repoRoot, 'rev-parse', 'HEAD')
$head = ([string]($headLines | Select-Object -First 1)).Trim()
Write-Host ('oracle: repo=' + $repoRoot)
Write-Host ('oracle: head=' + $head + ' pinned=' + $commit + ' sources=' + $sources.Count)
if ($head -ne $commit) {
    Write-Host 'oracle: NOTE HEAD differs from the pinned commit; only the pinned paths are checked'
}

# --- 1. the pinned C# sources must be byte-identical to the commit -------------------------
$null = Invoke-Native 'git' (@('-C', $repoRoot, 'diff', '--quiet', $commit, '--') + $sources)
$diffExit = $script:LastNativeExit
if ($diffExit -ne 0) {
    Write-Host 'oracle: tracked files that differ from the pinned commit:'
    $changed = Invoke-Native 'git' (@('-C', $repoRoot, 'diff', '--name-status', $commit, '--') + $sources)
    $changed | Select-Object -First 20 | ForEach-Object { Write-Host ('  ' + [string]$_) }
    Stop-WithCode 2 ('working tree differs from ' + $commit + ' in the pinned source directories (git diff exit ' + $diffExit + ')')
}

$statusLines = @(Invoke-Native 'git' (@('-C', $repoRoot, 'status', '--porcelain', '--') + $sources) |
    Where-Object { ([string]$_).Trim().Length -gt 0 })
if ($script:LastNativeExit -ne 0) {
    Stop-WithCode 2 'git status failed'
}
if ($statusLines.Count -gt 0) {
    Write-Host 'oracle: working tree is not clean in the pinned source directories:'
    $statusLines | Select-Object -First 20 | ForEach-Object { Write-Host ('  ' + [string]$_) }
    Stop-WithCode 2 ('git status is not empty for the pinned source directories (' + $statusLines.Count + ' entries)')
}

Write-Host 'oracle: pinned sources clean (git diff --quiet and git status --porcelain are both empty)'

# --- 2. build the runner (skipped by -Verify) ---------------------------------------------
$csproj = Join-Path $repoRoot ([string]$oracle.csproj)
if (-not $Verify) {
    if (-not (Test-Path -LiteralPath $csproj)) {
        Stop-WithCode 2 ('missing ' + $csproj)
    }

    Write-Host ('oracle: build ' + [string]$oracle.csproj + ' -c Release')
    $buildOutput = Invoke-Native 'dotnet' @('build', $csproj, '-c', 'Release', '--nologo')
    $buildExit = $script:LastNativeExit
    $buildOutput | ForEach-Object { Write-Host ('  ' + [string]$_) }
    if ($buildExit -ne 0) {
        Stop-WithCode 1 ('dotnet build failed with exit code ' + $buildExit)
    }
    Write-Host 'oracle: build OK'
}

# --- 3. the runner must answer with one JSON object ---------------------------------------
$runnerRelative = [string]$oracle.runnerPath
$runnerPath = Join-Path $repoRoot $runnerRelative.Replace('/', [System.IO.Path]::DirectorySeparatorChar)
if (-not (Test-Path -LiteralPath $runnerPath)) {
    if ($Verify) {
        Stop-WithCode 1 ('runner not built: ' + $runnerPath + ' (run this script without -Verify)')
    }
    Stop-WithCode 1 ('runner not found after build: ' + $runnerPath)
}

$runnerLines = Invoke-Native 'dotnet' @('exec', $runnerPath, '--eval', '1/3', '--json') -DiscardStdErr
$runnerExit = $script:LastNativeExit
$runnerText = (@($runnerLines) -join [Environment]::NewLine)
if ($runnerExit -ne 0) {
    Stop-WithCode 1 ('runner exited ' + $runnerExit + ' for --eval 1/3 --json')
}

$envelope = $null
try {
    $envelope = $runnerText | ConvertFrom-Json
} catch {
    Stop-WithCode 1 ('runner stdout is not valid JSON: ' + $_.Exception.Message)
}

if ($null -eq $envelope) {
    Stop-WithCode 1 'runner stdout parsed to null'
}
if (@($envelope).Count -ne 1) {
    Stop-WithCode 1 'runner stdout is not a single JSON value'
}
if ($envelope.PSObject.Properties.Name -notcontains 'protocolVersion') {
    Stop-WithCode 1 'runner envelope has no protocolVersion'
}
if ([int]$envelope.protocolVersion -ne 1) {
    Stop-WithCode 1 ('runner protocolVersion is ' + [string]$envelope.protocolVersion + ', expected 1')
}
if ($envelope.ok -ne $true) {
    Stop-WithCode 1 ('runner envelope ok is ' + [string]$envelope.ok + ', expected true')
}

Write-Host ('oracle: runner envelope ok=true protocolVersion=' + [int]$envelope.protocolVersion)

# --- 4. record the verified artifact -------------------------------------------------------
$sha = (Get-FileHash -LiteralPath $runnerPath -Algorithm SHA256).Hash.ToLowerInvariant()
$dotnetVersion = ([string]((Invoke-Native 'dotnet' @('--version') -DiscardStdErr) | Select-Object -First 1)).Trim()
$stamp = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')

if (-not $Verify) {
    $buildDir = Join-Path $parityDir 'build'
    if (-not (Test-Path -LiteralPath $buildDir)) {
        New-Item -ItemType Directory -Path $buildDir | Out-Null
    }

    $verified = [ordered]@{
        schemaVersion = 1
        commit        = $commit
        dll           = $runnerRelative
        dllSha256     = $sha
        dotnetVersion = $dotnetVersion
        verifiedAtUtc = $stamp
    }

    $verifiedPath = Join-Path $buildDir 'oracle-verified.json'
    $json = ($verified | ConvertTo-Json -Depth 4) + [Environment]::NewLine
    [System.IO.File]::WriteAllText($verifiedPath, $json, (New-Object System.Text.UTF8Encoding($false)))
    Write-Host ('oracle: wrote ' + $verifiedPath)
}

Write-Host ('oracle: OK commit=' + $commit + ' dotnet=' + $dotnetVersion + ' dll=' + $runnerRelative)
Write-Host ('oracle: dll sha256=' + $sha)
exit 0
