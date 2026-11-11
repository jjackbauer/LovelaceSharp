<#
.SYNOPSIS
  Gate: every requirement ID in the v2.0 contract has exactly one honest ledger row.
.DESCRIPTION
  Fails closed. Contract drift, a duplicate row, a "met" row with no evidence, or an exclusion
  without a user-approved owner is a failure. Exit 0 only on a complete, honest ledger.
#>
$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
node (Join-Path $root 'parity\tools\gate-requirements.mjs')
exit $LASTEXITCODE
