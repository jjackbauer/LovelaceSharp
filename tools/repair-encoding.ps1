<#
.SYNOPSIS
  Repair Markdown that was written through a Windows-1252 encoder (mojibake).

.DESCRIPTION
  Subagent-written Markdown occasionally contains UTF-8 bytes that were decoded as Windows-1252 and
  re-encoded, so an em dash becomes three visible characters. This script rewrites the common
  sequences back to their intended code points, reports the replacement count per file, and touches
  only .md files under the given root. It is idempotent.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Root,
    [switch]$WhatIf
)

$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $Root)) { throw "root not found: $Root" }

$E2 = [string][char]0x00E2
$C3 = [string][char]0x00C3
$pairs = @(
    @($E2 + [char]0x20AC + [char]0x201D, [string][char]0x2014),
    @($E2 + [char]0x20AC + [char]0x201C, [string][char]0x2013),
    @($E2 + [char]0x20AC + [char]0x2122, [string][char]0x2019),
    @($E2 + [char]0x20AC + [char]0x0153, [string][char]0x201C),
    @($E2 + [char]0x20AC + [char]0x009D, [string][char]0x201D),
    @($E2 + [char]0x20AC + [char]0x00A6, [string][char]0x2026),
    @($E2 + [char]0x20AC + [char]0x00A2, [string][char]0x2022),
    @($E2 + [char]0x20AC + [char]0x2018, [string][char]0x2018),
    @($E2 + [char]0x2014 + [char]0x2013, [string][char]0x2014),
    @($E2 + [char]0x2014 + [char]0x201D, [string][char]0x2014),
    @($E2 + [char]0x2014 + [char]0x201C, [string][char]0x2013),
    @($E2 + [char]0x2014 + [char]0x2019, [string][char]0x2019),
    @($E2 + [char]0x2014 + [char]0x0153, [string][char]0x201C),
    @($E2 + [char]0x2014 + [char]0x00A6, [string][char]0x2026),
    @($E2 + [char]0x2014 + [char]0x201A, [string][char]0x2018),
    @($E2 + [char]0x2014 + [char]0x2018, [string][char]0x2018),
    @($E2 + [char]0x2014 + [char]0x00A5, [string][char]0x2192),
    @($E2 + [char]0x2014 + [char]0x00A0, [string][char]0x2014),
    @($E2 + [char]0x2014 + [char]0x00A1, [string][char]0x2022),
    @($E2 + [char]0x2014 + [char]0x00B7, [string][char]0x00B7),
    @(([string][char]0x00E2) + [char]0x2014 + [char]0x2014, [string][char]0x2014),
    @($C3 + [char]0x2014 + [char]0x00D7, [string][char]0x00D7),
    @($C3 + [char]0x2014 + [char]0x00B7, [string][char]0x00B7),
    @($C3 + [char]0x00A2 + [char]0x2014 + [char]0x00A6, [string][char]0x2014),
    @(([string][char]0x00C2) + [char]0x00B7, [string][char]0x00B7),
    @(([string][char]0x00C3) + [char]0x2014, [string][char]0x2014),
    @(([string][char]0x00C3) + [char]0x00D7, [string][char]0x00D7),
    @(([string][char]0x00E2) + [char]0x2030 + [char]0x00A5, [string][char]0x2265),
    @(([string][char]0x00E2) + [char]0x02C6 + [char]0x2019, [string][char]0x2212),
    @(([string][char]0x00E2) + [char]0x2020 + [char]0x2019, [string][char]0x2192),
    @(([string][char]0x00E2) + [char]0x2014 + [char]0x00A2, [string][char]0x2022),
    @(([string][char]0x00E2) + [char]0x2014 + [char]0x201C, [string][char]0x2013),
    @(([string][char]0x00E2) + [char]0x2014 + [char]0x2122, [string][char]0x2122),
    @(([string][char]0x00E2) + [char]0x2014 + [char]0x2020, [string][char]0x2020)
)

$files = Get-ChildItem -LiteralPath $Root -Recurse -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Extension -in @('.md', '.txt') }
$total = 0
$changed = 0
foreach ($f in $files) {
    $text = [System.IO.File]::ReadAllText($f.FullName, [System.Text.Encoding]::UTF8)
    $count = 0
    foreach ($pair in $pairs) {
        $needle = $pair[0]
        if ($needle.Length -gt 0 -and $text.Contains($needle)) {
            $count += ([regex]::Matches($text, [regex]::Escape($needle))).Count
            $text = $text.Replace($needle, $pair[1])
        }
    }
    if ($count -gt 0) {
        $total += $count
        $changed++
        if (-not $WhatIf) {
            [System.IO.File]::WriteAllText($f.FullName, $text, (New-Object System.Text.UTF8Encoding($false)))
        }
        Write-Output ("{0}: {1}" -f $f.FullName.Substring($Root.Length).TrimStart('\'), $count)
    }
}
Write-Output "total_replacements=$total changed_files=$changed scanned=$($files.Count) whatif=$($WhatIf.IsPresent)"
