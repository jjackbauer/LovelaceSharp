<#
.SYNOPSIS
  Gate: the C# test census is complete, consistent, and reconciled with the frozen solution.
#>
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
node (Join-Path $root 'parity\tools\gate-census.mjs')
exit $LASTEXITCODE
