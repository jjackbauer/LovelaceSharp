<#
.SYNOPSIS
  Gate: every C# production project has exactly one honest Rust disposition (ECO-001/ECO-002).
#>
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
node (Join-Path $root 'parity\tools\gate-projects.mjs')
exit $LASTEXITCODE
