#requires -Version 5.1
<#
.SYNOPSIS
    REQ-010 reproducibility sequence: build the Rust workspace and run every gate from a checkout.

.DESCRIPTION
    One command that reproduces the state this round reports. It is the executable form of
    .github/workflows/rust-ci.yml, so an operator on a workstation runs the same steps the CI job
    runs (the workflow additionally installs the pinned toolchain and the .NET SDK).

    Steps, in order:
      1. toolchain     - cargo/rustc/rustfmt/node/dotnet versions, so the run is attributable
      2. build         - cargo build --release --locked (lovelace-run, lovelace-corpus)
      3. suite         - cargo test --workspace
      4. proof-nat     - cargo test -p lovelace-nat proof_      (PROOF-004)
      5. proof-int     - cargo test -p lovelace-int proof_      (PROOF-004)
      6. clippy        - cargo clippy --workspace --all-targets -- -D warnings
      7. fmt           - cargo fmt --all -- --check
      8. corpus        - the release corpus binary: 214/214
      9. binaries      - parity/tools/ci-binaries.mjs --locked (RUST-009/RUST-010: hashes, entry
                         point, CLI matrix 16/16, fixtures 20/20, corpus 214/214 against the
                         release binaries)
     10. ecosystem     - parity/tools/ecosystem-inventory.mjs --check (ECO-003/004 drift gate)
     11. formats       - parity/tools/format-check.mjs --check (ECO-008)
     12. citations     - parity/tools/verify-doc-citations.mjs (PROOF-002/003)
     13. workflows     - parity/tools/check-workflow-yaml.mjs

    Every step runs; a red step never stops the sequence (the report must show the whole picture) and
    never counts as a pass: the exit code is the number of failed steps.

.PARAMETER LogPath
    Where the transcript is written. Default: rust/target/tmp/reproduce.log

.PARAMETER SkipBuild
    Skip step 2 (the release binaries must already exist). The binaries step still builds through
    ci-binaries.mjs.

.EXAMPLE
    $repo = 'C:\path\to\LovelaceSharp'
    $env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
    $env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
    pwsh -NoProfile -File parity/tools/reproduce.ps1
#>
[CmdletBinding()]
param(
    [string]$LogPath = 'rust/target/tmp/reproduce.log',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$manifest = 'rust/Cargo.toml'
$newline = [Environment]::NewLine
$logFull = Join-Path $repo $LogPath
$logDir = Split-Path -Parent $logFull
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir -Force | Out-Null }
Set-Content -Path $logFull -Value '' -Encoding UTF8
$results = New-Object System.Collections.ArrayList

# Bootstrap the pinned toolchain when the checkout ships one (REQ-010: a clean checkout must
# reproduce without the caller pre-arranging PATH). Harmless when the toolchain is absent - the
# toolchain step then reports the missing tools instead of failing to start.
$localToolchain = Join-Path $repo '.toolchain'
if (Test-Path $localToolchain) {
    $env:RUSTUP_HOME = Join-Path $localToolchain 'rustup'
    $env:CARGO_HOME = Join-Path $localToolchain 'cargo'
    $env:PATH = (Join-Path $localToolchain 'cargo\bin') + ';' + $env:PATH
    $env:TMP = Join-Path $repo 'rust\target\tmp'
    $env:TEMP = $env:TMP
}

function Write-Transcript {
    param([string]$Line)
    Write-Host $Line
    Add-Content -Path $logFull -Value $Line -Encoding UTF8
}

function Invoke-Native {
    param([string]$Exe, [string[]]$Arguments)
    # Windows PowerShell 5.1 turns a native command's stderr into a terminating error under
    # $ErrorActionPreference = 'Stop'. The step result is the exit code, so stderr is captured as
    # data here and the strict preference is restored immediately afterwards.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    Push-Location $repo
    try {
        $captured = & $Exe @Arguments 2>&1
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
        $ErrorActionPreference = $previous
    }
    $text = ($captured | ForEach-Object { "$_" }) -join $newline
    return [pscustomobject]@{ ExitCode = $code; Text = $text }
}

function Get-Observed {
    param([string]$Text, [string]$Pattern)
    if (-not $Text) { return '(no summary line)' }
    $found = [regex]::Matches($Text, $Pattern, 'Multiline')
    $values = @()
    foreach ($m in $found) { $values += $m.Value.Trim() }
    if ($values.Count -eq 0) { return '(no summary line)' }
    return ($values -join ' | ')
}

function Add-Step {
    param(
        [string]$Name,
        [string]$Exe,
        [string[]]$Arguments,
        [string]$Pattern,
        [string]$Note
    )
    Write-Transcript ''
    Write-Transcript ("== " + $Name + " ==")
    Write-Transcript ("   $ " + $Exe + ' ' + ($Arguments -join ' '))
    $started = Get-Date
    $run = Invoke-Native -Exe $Exe -Arguments $Arguments
    $elapsed = [int]((Get-Date) - $started).TotalSeconds
    $run.Text -split "\n" | ForEach-Object { Write-Transcript ("   " + $_) }
    $status = 'FAIL'
    if ($run.ExitCode -eq 0) { $status = 'PASS' }
    $observed = Get-Observed -Text $run.Text -Pattern $Pattern
    [void]$results.Add([pscustomobject]@{
        step     = $Name
        command  = ($Exe + ' ' + ($Arguments -join ' '))
        status   = $status
        exitCode = $run.ExitCode
        seconds  = $elapsed
        observed = $observed
        note     = $Note
    })
    Write-Transcript ("   -> " + $status + " (exit " + $run.ExitCode + ", " + $elapsed + "s) " + $observed)
}

# --- 1. toolchain ---------------------------------------------------------------------------
Write-Transcript ("# REQ-010 reproducibility sequence - " + (Get-Date).ToString('yyyy-MM-ddTHH:mm:ssK'))
Write-Transcript ("# repo: " + $repo)
Write-Transcript ("# host: " + [System.Environment]::OSVersion.VersionString + " / " + $env:PROCESSOR_ARCHITECTURE)
$toolchainLines = @()
foreach ($tool in @('cargo', 'rustc', 'rustfmt', 'node', 'dotnet')) {
    $command = Get-Command $tool -ErrorAction SilentlyContinue
    if (-not $command) { $toolchainLines += ($tool + ': MISSING'); continue }
    $version = (Invoke-Native -Exe $tool -Arguments @('--version')).Text.Trim()
    $toolchainLines += ($tool + ': ' + (($version -split "\n")[0]))
}
foreach ($line in $toolchainLines) { Write-Transcript ("   " + $line) }
[void]$results.Add([pscustomobject]@{
    step = 'toolchain'; command = 'cargo/rustc/rustfmt/node/dotnet --version'; status = 'INFO'
    exitCode = 0; seconds = 0; observed = ($toolchainLines -join '; '); note = 'attribution'
})

# --- 2. release build ----------------------------------------------------------------------
if ($SkipBuild) {
    [void]$results.Add([pscustomobject]@{
        step = 'build'; command = 'skipped (-SkipBuild)'; status = 'SKIP'; exitCode = 0
        seconds = 0; observed = 'release binaries reused'; note = 'build'
    })
} else {
    Add-Step -Name 'build' -Exe 'cargo' -Arguments @('build', '--release', '--locked', '--manifest-path', $manifest, '-p', 'lovelace-run', '-p', 'lovelace-corpus') -Pattern 'Finished' -Note 'RUST-009 release binaries'
}

# --- 3. workspace suite ---------------------------------------------------------------------
Add-Step -Name 'suite' -Exe 'cargo' -Arguments @('test', '--workspace', '--manifest-path', $manifest) -Pattern '^test result: .*$' -Note 'workspace green'

# --- 4-5. exhaustive small-domain proofs ----------------------------------------------------
Add-Step -Name 'proof-nat' -Exe 'cargo' -Arguments @('test', '-p', 'lovelace-nat', 'proof_', '--manifest-path', $manifest) -Pattern '^test result: .*$' -Note 'PROOF-004 Nat exhaustive domains'
Add-Step -Name 'proof-int' -Exe 'cargo' -Arguments @('test', '-p', 'lovelace-int', 'proof_', '--manifest-path', $manifest) -Pattern '^test result: .*$' -Note 'PROOF-004 Int exhaustive domains'

# --- 6-7. lint and format -------------------------------------------------------------------
Add-Step -Name 'clippy' -Exe 'cargo' -Arguments @('clippy', '--workspace', '--all-targets', '--manifest-path', $manifest, '--', '-D', 'warnings') -Pattern '^error(\[|:).*$|^Finished .*$' -Note 'deny warnings'
Add-Step -Name 'fmt' -Exe 'cargo' -Arguments @('fmt', '--all', '--manifest-path', $manifest, '--', '--check') -Pattern '^Diff in .*$' -Note 'rustfmt --check'

# --- 8. corpus through the release binary ---------------------------------------------------
$corpus = Join-Path $repo 'rust/target/release/lovelace-corpus.exe'
if (-not (Test-Path $corpus)) { $corpus = Join-Path $repo 'rust/target/release/lovelace-corpus' }
Add-Step -Name 'corpus' -Exe $corpus -Arguments @() -Pattern '^corpus: .*$' -Note 'D9 corpus gate'

# --- 9. RUST-009/RUST-010: release binaries through their published entry points ------------
Add-Step -Name 'binaries' -Exe 'node' -Arguments @('parity/tools/ci-binaries.mjs', '--locked') -Pattern '^(build|binary|entry point|oracle|CLI matrix|rust|oracle|corpus|evidence|PASS): .*$' -Note 'RUST-010 published entry points'

# --- 10-13. documentation and ledger gates --------------------------------------------------
Add-Step -Name 'ecosystem' -Exe 'node' -Arguments @('parity/tools/ecosystem-inventory.mjs', '--check') -Pattern '^PASS: .*$' -Note 'ECO-003/004 drift gate'
Add-Step -Name 'formats' -Exe 'node' -Arguments @('parity/tools/format-check.mjs', '--check') -Pattern '^PASS: .*$' -Note 'ECO-008 validator'
Add-Step -Name 'citations' -Exe 'node' -Arguments @('parity/tools/verify-doc-citations.mjs') -Pattern '^PASS: .*$' -Note 'PROOF-002/003 citations'
Add-Step -Name 'workflows' -Exe 'node' -Arguments @('parity/tools/check-workflow-yaml.mjs') -Pattern '^PASS: .*$' -Note 'workflow structure'

# --- summary --------------------------------------------------------------------------------
Write-Transcript ''
Write-Transcript '== summary =='
foreach ($result in $results) {
    Write-Transcript ("   " + $result.step.PadRight(12) + " " + $result.status.PadRight(6) + " " + ([string]$result.seconds).PadLeft(5) + "s  " + $result.observed)
}
$failed = @($results | Where-Object { $_.status -eq 'FAIL' })
$outDir = Join-Path $repo 'parity/build'
if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
$record = [pscustomobject]@{
    schemaVersion = 1
    generatedBy   = 'parity/tools/reproduce.ps1'
    generatedAt   = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    host          = [pscustomobject]@{ os = [System.Environment]::OSVersion.VersionString; arch = $env:PROCESSOR_ARCHITECTURE }
    toolchain     = $toolchainLines
    steps         = $results
    failed        = $failed.Count
}
$record | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $outDir 'reproduce.json') -Encoding UTF8
Write-Transcript ''
Write-Transcript ("   transcript:    " + $LogPath)
Write-Transcript ("   evidence:      parity/build/reproduce.json")
Write-Transcript ("   failed steps:  " + $failed.Count + " / " + $results.Count)
if ($failed.Count -gt 0) {
    foreach ($result in $failed) { Write-Transcript ("   FAIL " + $result.step + ": " + $result.command) }
    exit $failed.Count
}
Write-Transcript '   PASS: every step reproduced'
exit 0
