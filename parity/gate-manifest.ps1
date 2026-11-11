<#
.SYNOPSIS
  Gate: every C# test case has exactly one honest disposition and every claimed Rust test exists.
#>
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
node (Join-Path $root 'parity\tools\gate-manifest.mjs')
exit $LASTEXITCODE
