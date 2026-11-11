<#
.SYNOPSIS
  Gate: the CLI surface (RUN-001..RUN-014) behaves like the C# oracle.
#>
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
node (Join-Path $root 'parity\tools\gate-cli.mjs')
exit $LASTEXITCODE
