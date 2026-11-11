<#
.SYNOPSIS
  Gate: the 20 protocol golden fixtures pass on the C# oracle AND on the Rust runner.
.PARAMETER RustOnly
  Check only the Rust runner (used while the oracle is already known-good).
.PARAMETER OracleOnly
  Check only the C# oracle (guards against stale goldens).
.PARAMETER Verbose
  Print every difference, not just the first three per fixture.
#>
param([switch]$RustOnly, [switch]$OracleOnly, [switch]$Verbose)
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
$args = @((Join-Path $root 'parity\tools\gate-fixtures.mjs'))
if ($RustOnly) { $args += '--rust-only' }
if ($OracleOnly) { $args += '--oracle-only' }
if ($Verbose) { $args += '--verbose' }
node @args
exit $LASTEXITCODE
