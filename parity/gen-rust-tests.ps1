<#
.SYNOPSIS
  Generates parity/rust-tests.json: every test the Rust workspace actually contains, per crate.
.DESCRIPTION
  The manifest gate resolves a "ported" disposition to a test NAME that exists. Existence is not
  proof that the test passes, so this script also runs the suite and records its exit code and
  counts; a "ported" row is only accepted when the suite run recorded exit 0.
#>
$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot
$env:RUSTUP_HOME = Join-Path $repo '.toolchain\rustup'
$env:CARGO_HOME = Join-Path $repo '.toolchain\cargo'
$env:PATH = (Join-Path $repo '.toolchain\cargo\bin') + ';' + $env:PATH
$manifest = Join-Path $repo 'rust\Cargo.toml'
# The sandbox denies subdirectory deletion under %TEMP%; rustdoc creates and removes a temp dir,
# so point TMP at a repo-local directory that the workspace policy allows.
$tmp = Join-Path $repo 'rust\target\tmp'
New-Item -ItemType Directory -Force $tmp | Out-Null
$env:TMP = $tmp; $env:TEMP = $tmp; $env:TMPDIR = $tmp

$metaText = & cargo metadata --manifest-path $manifest --no-deps --format-version 1 2>$null
if ($LASTEXITCODE -ne 0) { Write-Error 'cargo metadata failed'; exit 2 }
$meta = $metaText | ConvertFrom-Json

$crates = @()
foreach ($pkg in $meta.packages) {
  $list = & cargo test --manifest-path $manifest -p $pkg.name -- --list 2>$null
  $names = @()
  foreach ($line in $list) {
    if ($line -match '^(?<n>.+): test$') { $names += $Matches['n'] }
  }
  if ($LastExitCode -ne 0) { Write-Error ("cargo test --list failed for " + $pkg.name); exit 2 }
  $crates += [pscustomobject]@{ name = $pkg.name; version = $pkg.version; tests = @($names | Sort-Object) }
}

$run = & cargo test --manifest-path $manifest --workspace 2>&1
$runExit = $LASTEXITCODE
$resultLines = @($run | Where-Object { $_ -match 'test result:' })
$passed = 0; $failed = 0
foreach ($line in $resultLines) {
  if ($line -match '(\d+) passed') { $passed += [int]$Matches[1] }
  if ($line -match '(\d+) failed') { $failed += [int]$Matches[1] }
}

$total = 0
foreach ($c in $crates) { $total += $c.tests.Count }

# Content hash of every Rust source: the manifest gate uses it to tell a real edit from a touch
# (a mutation harness writes each file back, which changes mtime but not content).
$rustRoot = Join-Path $repo 'rust'
# Match the gate's walker exactly: skip target/ and any dot-prefixed file or directory.
$sourceFiles = Get-ChildItem $rustRoot -Recurse -File -Filter *.rs | Where-Object { ($_.FullName -notmatch '\\target\\') -and ($_.Name -notlike '.*') -and ($_.FullName -notmatch '\\\.') }
$hashLines = New-Object System.Collections.Generic.List[string]
foreach ($sf in $sourceFiles) {
  $rel = $sf.FullName.Substring($rustRoot.Length + 1).Replace([char]92, [char]47)
  $hash = (Get-FileHash -LiteralPath $sf.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  $hashLines.Add($rel + ':' + $hash)
}
# Sort the COMPOSED lines ordinally, exactly as the gate does; a culture-aware sort of file paths
# would produce a different order and a different hash.
$hashLines.Sort([System.StringComparer]::Ordinal)
$joined = ($hashLines -join ([string][char]10))
$sha = [System.Security.Cryptography.SHA256]::Create()
$bytes = [System.Text.Encoding]::UTF8.GetBytes($joined)
$sourcesHash = -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') })
$cargoVersion = (& cargo --version)

$out = [pscustomobject]@{
  schemaVersion = 1
  generatedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  toolchain = $cargoVersion
  suite = [pscustomobject]@{ exitCode = $runExit; passed = $passed; failed = $failed }
  totals = [pscustomobject]@{ crates = $crates.Count; tests = $total }
  sourcesSha256 = $sourcesHash
  crates = $crates
}
$path = Join-Path $PSScriptRoot 'rust-tests.json'
$out | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 $path
Write-Output ("crates=" + $crates.Count + " tests=" + $total + " suiteExit=" + $runExit + " passed=" + $passed + " failed=" + $failed)
if ($runExit -ne 0) { exit 1 }
exit 0
