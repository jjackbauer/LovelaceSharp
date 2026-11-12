<#
.SYNOPSIS
  Round-26 RES-001 evidence: run a script whose last committed value is a nested record with a
  2x2 matrix, then capture the Workspace RESULT tree the desktop builds from the engine's
  structured projection.

.DESCRIPTION
  The console session is unlocked, so this round uses the normal harness for launch, UI
  Automation input and accessibility:
    - create work/structured-results.lv:
        m = [[1, 2], [3, 4]]
        x = symbol("x")
        inv_full([[x, 1], [1, x]])
      The last value is a MatrixInverseResult record whose `inverse` field is itself a 2x2
      array and whose `conditions` field is a vector of symbolic expressions;
    - launch the shipped lovelace-desktop.exe on that document through tools/gui-harness.ps1;
    - restore/size the window to 2500x1380 so the RESULT tree and the VARIABLES table are both
      visible and the whole window fits the 2560x1440 screen;
    - invoke the Run button through UI Automation (-Action click), no typing;
    - wait for the status strip to read "run Finished";
    - dump the live accessibility tree and assert the RESULT tree exposes the projection's own
      vocabulary: MatrixInverseResult - 6 fields, status:/complete:/completeness:/inverse:/
      conditions:/diagnostics:, Array 2 x 2 - 4 elements, pretty/canonical renderings, and the
      captured variable m with its 2x2 array display;
    - capture with -Action capture and with PrintWindow(PW_RENDERFULLCONTENT);
    - probe the PrintWindow pixels for the accent, text, dim and warn colours.

  Windows PowerShell 5.1 compatible. ASCII-only.
#>
[CmdletBinding()]
param(
    [string]$Repo = '',
    [string]$OutDir = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, UIAutomationClient, UIAutomationTypes

if (-not $OutDir) { $OutDir = $PSScriptRoot }
if (-not $Repo) { $Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path }
$Exe = Join-Path $Repo 'rust\target-desktop\debug\lovelace-desktop.exe'
if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
if (-not (Test-Path -LiteralPath $Harness)) { throw "harness not found: $Harness" }

$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$DriverLog = Join-Path $Work 'driver.log'
$env:APPDATA = Join-Path $OutDir 'appdata'
New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null

$Document = Join-Path $Work 'structured-results.lv'
$lines = @(
    'm = [[1, 2], [3, 4]]',
    'x = symbol("x")',
    'inv_full([[x, 1], [1, x]])'
)
Set-Content -LiteralPath $Document -Value $lines -Encoding ASCII
Write-Host "document=$Document exe=$Exe out=$OutDir"

function Invoke-Harness([string[]]$HarnessArgs) {
    # The harness appends its own lines to $DriverLog; return its console output to the caller
    # without duplicating it in the log.
    return (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness @HarnessArgs 2>&1 | Out-String)
}

# --- launch ---------------------------------------------------------------------------------
$launch = Invoke-Harness @('-Action', 'launch', '-Exe', $Exe, '-Arguments', $Document, '-Log', $DriverLog)
$processId = 0
if ($launch -match 'PROCESS_ID=(\d+)') { $processId = [int]$matches[1] }
if ($processId -eq 0) { throw "launch did not report a process id: $launch" }
Write-Host "launch pid=$processId"

$wait = Invoke-Harness @('-Action', 'wait', '-ProcessId', "$processId", '-Log', $DriverLog)
if ($wait -notmatch 'HWND=(\d+)') { throw "wait did not report a window handle: $wait" }
$h = [IntPtr][long]$matches[1]
Write-Host "window hwnd=$h"

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
public struct R26Rect { public int Left, Top, Right, Bottom; }
public static class R26Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R26Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@
if ([R26Native]::IsIconic($h)) { [void][R26Native]::ShowWindow($h, 9) }
[void][R26Native]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
Start-Sleep -Milliseconds 900
$rect = New-Object R26Rect
[void][R26Native]::GetWindowRect($h, [ref]$rect)
Write-Host "resized=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"

# --- run and wait for Finished --------------------------------------------------------------
$click = Invoke-Harness @('-Action', 'click', '-ProcessId', "$processId", '-Name', 'Run', '-Log', $DriverLog)
if ($click -notmatch 'CLICKED=Run') { throw "could not invoke Run: $click" }

function Get-Status {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        if ($text.Current.Name -match '\|\s*run ([A-Za-z]+)\s*$') { return $text.Current.Name }
    }
    return ''
}
$deadline = (Get-Date).AddSeconds(30)
$status = ''
while ((Get-Date) -lt $deadline) {
    $status = Get-Status
    if ($status -match '\|\s*run Finished\s*$') { break }
    Start-Sleep -Milliseconds 150
}
Write-Host "status='$status'"
Add-Content -LiteralPath $DriverLog -Value "status='$status'" -Encoding UTF8
if ($status -notmatch '\|\s*run Finished\s*$') { throw "the run never reached Finished: '$status'" }

# --- live accessibility tree ----------------------------------------------------------------
$uia = Invoke-Harness @('-Action', 'uia', '-ProcessId', "$processId", '-OutDir', $Work, '-Log', $DriverLog)
$Uia = Join-Path $Work 'uia-tree.txt'
if (-not (Test-Path -LiteralPath $Uia)) { throw "uia action produced no tree: $uia" }
Copy-Item -LiteralPath $Uia -Destination (Join-Path $OutDir 'structured-results-uia.txt') -Force
$uiaText = Get-Content -LiteralPath $Uia -Raw

foreach ($required in @(
    'RESULT',
    'structured projection',
    'MatrixInverseResult - 6 fields',
    'Array 2 x 2 - 4 elements',
    'status:',
    'complete:',
    'completeness:',
    'inverse:',
    'conditions:',
    'diagnostics:',
    'pretty',
    'canonical',
    'Symbolic',
    'Array 2 x 2 - 4 elements',
    '[[1, 2], [3, 4]]',
    'run Finished'
)) {
    if (-not $uiaText.Contains($required)) {
        throw "the UIA evidence does not contain '$required'"
    }
}
$prettyCount = ([regex]::Matches($uiaText, 'pretty')).Count
$canonicalCount = ([regex]::Matches($uiaText, 'canonical')).Count
Write-Host "uia pretty=$prettyCount canonical=$canonicalCount"
Add-Content -LiteralPath $DriverLog -Value "uia pretty=$prettyCount canonical=$canonicalCount" -Encoding UTF8
if ($prettyCount -lt 2 -or $canonicalCount -lt 2) {
    throw "the matrix cells and the condition must publish both renderings (pretty=$prettyCount canonical=$canonicalCount)"
}

# --- capture: harness capture, plus a PrintWindow frame -------------------------------------
$capture = Invoke-Harness @('-Action', 'capture', '-ProcessId', "$processId", '-OutDir', $OutDir, '-Name', 'structured-results', '-Log', $DriverLog)
$Png = Join-Path $OutDir 'structured-results.png'
if (-not (Test-Path -LiteralPath $Png)) { throw "capture produced no file: $capture" }
$captureBytes = (Get-Item -LiteralPath $Png).Length
Write-Host "capture=$Png bytes=$captureBytes"

[void][R26Native]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
Start-Sleep -Milliseconds 400
[void][R26Native]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
Start-Sleep -Milliseconds 900
[void][R26Native]::GetWindowRect($h, [ref]$rect)
$w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
$ok = [R26Native]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$g.Dispose()
$Pw = Join-Path $OutDir 'structured-results-printwindow.png'
$bmp.Save($Pw, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
$pwBytes = (Get-Item -LiteralPath $Pw).Length
Write-Host "printwindow=$Pw bytes=$pwBytes ok=$ok rect=${w}x${ht}"
Add-Content -LiteralPath $DriverLog -Value "capture file=$Png bytes=$captureBytes" -Encoding UTF8
Add-Content -LiteralPath $DriverLog -Value "printwindow file=$Pw bytes=$pwBytes ok=$ok rect=${w}x${ht}" -Encoding UTF8
if ($captureBytes -lt 20480 -or $pwBytes -lt 20480) { throw 'a capture is smaller than 20 KB; refusing it as empty' }

# --- exact-colour probe of the PrintWindow frame --------------------------------------------
$probe = @{}
$bitmap = New-Object System.Drawing.Bitmap($Pw)
try {
    $stepX = 2; $stepY = 2
    for ($y = 0; $y -lt $bitmap.Height; $y += $stepY) {
        for ($x = 0; $x -lt $bitmap.Width; $x += $stepX) {
            $c = $bitmap.GetPixel($x, $y)
            $key = '{0:X2}{1:X2}{2:X2}' -f $c.R, $c.G, $c.B
            if ($probe.ContainsKey($key)) { $probe[$key]++ } else { $probe[$key] = 1 }
        }
    }
} finally {
    $bitmap.Dispose()
}
$accent = if ($probe.ContainsKey('4EC9B0')) { $probe['4EC9B0'] } else { 0 }
$text = if ($probe.ContainsKey('D8DEE9')) { $probe['D8DEE9'] } else { 0 }
$dim = if ($probe.ContainsKey('8B98A9')) { $probe['8B98A9'] } else { 0 }
$warn = if ($probe.ContainsKey('D7BA7D')) { $probe['D7BA7D'] } else { 0 }
Write-Host "probe accent=$accent text=$text dim=$dim warn=$warn"
Add-Content -LiteralPath $DriverLog -Value "probe accent=$accent text=$text dim=$dim warn=$warn" -Encoding UTF8
if ($accent -lt 50 -or $text -lt 100 -or $dim -lt 50) {
    throw "the PrintWindow frame does not carry the pane palette (accent=$accent text=$text dim=$dim)"
}

# --- close ----------------------------------------------------------------------------------
[void](Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog))
Write-Host "done capture=$captureBytes printwindow=$pwBytes"
