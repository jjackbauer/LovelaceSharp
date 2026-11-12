<#
.SYNOPSIS
  Round-28 SYM-001 evidence: the Workspace pane's Inspector showing the engine's six symbolic
  sections for the last committed result.

.DESCRIPTION
  The console session is unlocked, so this round uses the normal harness for launch, UIA
  invocation, UIA dumps, keyboard activation and close:
    scenario 1 (symbolic-diff.png):
      x = symbol("x")
      assume(x > 0)
      diff(x^3, x)
      -> a Symbolic result: Pretty 3*x^2, Canonical (mul ...), the engine expression tree,
         the session assumption x > 0 as a typed relation, the trace's explicit "only a
         TransformResult has steps" reason, and the compiled MathIR;
    scenario 2 (simplify-trace.png):
      x = symbol("x")
      assume(x > 0)
      simplify_full(exp(log(x)))
      -> a TransformResult: the same six sections, now with the engine's rewritten
         expression as the subject and the logexp.exp-log RewriteStep with its
         classification, before/after and required FiniteCondition.

  Per scenario: launch the shipped lovelace-desktop.exe on the script through
  tools/gui-harness.ps1, resize to 2500x1380, invoke Run through UI Automation, wait for
  "run Finished", select the Workspace pane's Inspector tab by keyboard (UIA SetFocus then
  Space, because egui's selectable tab does not expose InvokePattern), capture
  PrintWindow(PW_RENDERFULLCONTENT), dump the live UIA tree, assert the section headings and
  the engine-supplied values, probe the pane palette, and close the application.

  Windows PowerShell 5.1 compatible. ASCII-only.
#>
[CmdletBinding()]
param(
    [string]$Repo = '',
    [string]$OutDir = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes

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
$CaptureLog = Join-Path $OutDir 'capture-run.log'
if (Test-Path -LiteralPath $CaptureLog) { Remove-Item -LiteralPath $CaptureLog -Force }
$env:APPDATA = Join-Path $OutDir 'appdata'
New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null

function Invoke-Harness([string[]]$HarnessArgs) {
    return (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness @HarnessArgs 2>&1 | Out-String)
}

function Write-CaptureLog([string]$line) {
    Write-Host $line
    Add-Content -LiteralPath $CaptureLog -Value $line -Encoding UTF8
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct R28Rect { public int Left, Top, Right, Bottom; }
public static class R28Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R28Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
"@

function Get-Status([IntPtr]$Hwnd) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        if ($text.Current.Name -match '\|\s*run ([A-Za-z]+)\s*$') { return $text.Current.Name }
    }
    return ''
}

function Find-ButtonByName([IntPtr]$Hwnd, [string]$Name) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.AndCondition(
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::NameProperty, $Name)),
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::IsEnabledProperty, $true)))
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Test-InspectorSelected([IntPtr]$Hwnd) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        if ($text.Current.Name -like 'INSPECTOR*') { return $true }
    }
    return $false
}

function Save-PrintWindow([IntPtr]$Hwnd, [string]$Path) {
    # PrintWindow can return a blank DWM surface if the window has not been raised/painted since
    # the previous scenario closed. Raise it, force a redraw and retry with a longer settle.
    $bytes = 0
    $ok = $false
    for ($attempt = 1; $attempt -le 4; $attempt++) {
        [void][R28Native]::ShowWindow($Hwnd, 9)
        [void][R28Native]::BringWindowToTop($Hwnd)
        [void][R28Native]::SetForegroundWindow($Hwnd)
        [void][R28Native]::RedrawWindow($Hwnd, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
        Start-Sleep -Milliseconds 500
        [void][R28Native]::SetWindowPos($Hwnd, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
        Start-Sleep -Milliseconds (700 + 400 * $attempt)
        $windowRect = New-Object R28Rect
        [void][R28Native]::GetWindowRect($Hwnd, [ref]$windowRect)
        $w = [int]($windowRect.Right - $windowRect.Left)
        $ht = [int]($windowRect.Bottom - $windowRect.Top)
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc()
        $ok = [R28Native]::PrintWindow($Hwnd, $dc, 2)
        $g.ReleaseHdc($dc)
        $g.Dispose()
        $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        Write-CaptureLog "printwindow attempt=$attempt file=$Path bytes=$bytes ok=$ok rect=${w}x${ht}"
        if ($bytes -ge 20480) { break }
        Start-Sleep -Milliseconds 900
    }
    if ($bytes -lt 20480) { throw "PrintWindow frame $Path is smaller than 20 KB ($bytes) after 4 attempts; refusing it as empty" }
    Add-Content -LiteralPath $DriverLog -Value "printwindow file=$Path bytes=$bytes ok=$ok rect=${w}x${ht}" -Encoding UTF8
    return $bytes
}

function Save-UiaDump([IntPtr]$Hwnd, [int]$ProcessId, [string]$Path) {
    $uia = Invoke-Harness @('-Action', 'uia', '-ProcessId', "$ProcessId", '-OutDir', $Work, '-Log', $DriverLog)
    $produced = Join-Path $Work 'uia-tree.txt'
    if (-not (Test-Path -LiteralPath $produced)) { throw "uia action produced no tree: $uia" }
    Copy-Item -LiteralPath $produced -Destination $Path -Force
    $bytes = (Get-Item -LiteralPath $Path).Length
    Write-CaptureLog "uia=$Path bytes=$bytes"
    return (Get-Content -LiteralPath $Path -Raw)
}

function Get-Palette([string]$Path) {
    $bitmap = New-Object System.Drawing.Bitmap($Path)
    $probe = @{}
    try {
        for ($y = 0; $y -lt $bitmap.Height; $y += 2) {
            for ($x = 0; $x -lt $bitmap.Width; $x += 2) {
                $c = $bitmap.GetPixel($x, $y)
                $key = '{0:X2}{1:X2}{2:X2}' -f $c.R, $c.G, $c.B
                if ($probe.ContainsKey($key)) { $probe[$key]++ } else { $probe[$key] = 1 }
            }
        }
        return @(
            $(if ($probe.ContainsKey('4EC9B0')) { $probe['4EC9B0'] } else { 0 }),
            $(if ($probe.ContainsKey('D8DEE9')) { $probe['D8DEE9'] } else { 0 }),
            $(if ($probe.ContainsKey('8B98A9')) { $probe['8B98A9'] } else { 0 }))
    } finally {
        $bitmap.Dispose()
    }
}

function Invoke-Scenario {
    param(
        [string]$Label,
        [string]$Document,
        [string]$Png,
        [string]$UiaPath,
        [string[]]$Required
    )
    Write-CaptureLog "scenario=$Label document=$Document"
    $launch = Invoke-Harness @('-Action', 'launch', '-Exe', $Exe, '-Arguments', $Document, '-Log', $DriverLog)
    $processId = 0
    if ($launch -match 'PROCESS_ID=(\d+)') { $processId = [int]$matches[1] }
    if ($processId -eq 0) { throw "launch did not report a process id: $launch" }
    Write-CaptureLog "launch pid=$processId"

    $wait = Invoke-Harness @('-Action', 'wait', '-ProcessId', "$processId", '-Log', $DriverLog)
    if ($wait -notmatch 'HWND=(\d+)') { throw "wait did not report a window handle: $wait" }
    $h = [IntPtr][long]$matches[1]
    Write-CaptureLog "window hwnd=$h"

    if ([R28Native]::IsIconic($h)) { [void][R28Native]::ShowWindow($h, 9) }
    [void][R28Native]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
    Start-Sleep -Milliseconds 900
    $rect = New-Object R28Rect
    [void][R28Native]::GetWindowRect($h, [ref]$rect)
    Write-CaptureLog "resized=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"

    $click = Invoke-Harness @('-Action', 'click', '-ProcessId', "$processId", '-Name', 'Run', '-Log', $DriverLog)
    if ($click -notmatch 'CLICKED=Run') { throw "could not invoke Run: $click" }

    $deadline = (Get-Date).AddSeconds(30)
    $status = ''
    while ((Get-Date) -lt $deadline) {
        $status = Get-Status $h
        if ($status -match '\|\s*run Finished\s*$') { break }
        Start-Sleep -Milliseconds 150
    }
    Write-CaptureLog "status='$status'"
    if ($status -notmatch '\|\s*run Finished\s*$') { throw "the run never reached Finished: '$status'" }
    Start-Sleep -Milliseconds 700

    # egui's selectable Workspace tab exposes a Button role but no InvokePattern, so the
    # established interaction is UIA focus plus Space (the same route round 27 used for a
    # result-tree header). A raw pointer click at the published bounding rectangle is recorded
    # as the fallback if the keyboard route ever stops working.
    $method = 'keyboard Space on the UIA-focused Inspector tab'
    $tab = Find-ButtonByName $h 'Inspector'
    if (-not $tab) { throw 'the Workspace pane published no Inspector tab' }
    $tab.SetFocus()
    Start-Sleep -Milliseconds 250
    $keys = Invoke-Harness @('-Action', 'keys', '-ProcessId', "$processId", '-Keys', ' ', '-Log', $DriverLog)
    if ($keys -notmatch 'SENT=') { throw "the harness did not send Space: $keys" }
    $deadline = (Get-Date).AddSeconds(8)
    $selected = $false
    while ((Get-Date) -lt $deadline) {
        if (Test-InspectorSelected $h) { $selected = $true; break }
        Start-Sleep -Milliseconds 150
    }
    if (-not $selected) {
        $method = 'raw pointer click fallback on the Inspector tab rectangle'
        $tab = Find-ButtonByName $h 'Inspector'
        $r = $tab.Current.BoundingRectangle
        $cx = [int]($r.X + ($r.Width / 2))
        $cy = [int]($r.Y + ($r.Height / 2))
        [void][R28Native]::SetCursorPos($cx, $cy)
        Start-Sleep -Milliseconds 150
        [R28Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
        [R28Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 800
        if (-not (Test-InspectorSelected $h)) { throw 'the Inspector tab could not be selected by Space or by pointer' }
    }
    Write-CaptureLog "inspector selected method=$method"

    $bytes = Save-PrintWindow $h $Png
    $uiaText = Save-UiaDump $h $processId $UiaPath
    foreach ($needle in $Required) {
        if (-not $uiaText.Contains($needle)) {
            throw "the $Label UIA tree does not expose '$needle'"
        }
    }
    Write-CaptureLog "uia assertions=$($Required.Count) satisfied for $Label"

    $palette = @(Get-Palette $Png)
    Write-CaptureLog "palette $Label accent=$($palette[0]) text=$($palette[1]) dim=$($palette[2])"
    if ($palette[0] -lt 50 -or $palette[1] -lt 20 -or $palette[2] -lt 50) {
        throw "the $Label frame does not carry the pane palette (accent=$($palette[0]) text=$($palette[1]) dim=$($palette[2]))"
    }
    $hash = (Get-FileHash -LiteralPath $Png -Algorithm SHA256).Hash
    Write-CaptureLog "sha256 $Label=$hash bytes=$bytes"
    [void](Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog))
    return @{ Bytes = $bytes; Hash = $hash; Method = $method }
}

$DiffDoc = Join-Path $Work 'symbolic-diff.lv'
Set-Content -LiteralPath $DiffDoc -Value @(
    'x = symbol("x")',
    'assume(x > 0)',
    'diff(x^3, x)'
) -Encoding ASCII

$TraceDoc = Join-Path $Work 'simplify-trace.lv'
Set-Content -LiteralPath $TraceDoc -Value @(
    'x = symbol("x")',
    'assume(x > 0)',
    'simplify_full(exp(log(x)))'
) -Encoding ASCII

$DiffPng = Join-Path $OutDir 'symbolic-diff.png'
$DiffUia = Join-Path $OutDir 'symbolic-diff-uia.txt'
$Diff = Invoke-Scenario -Label 'symbolic-diff' -Document $DiffDoc -Png $DiffPng -UiaPath $DiffUia -Required @(
    'INSPECTOR',
    'subject  Symbolic',
    'Pretty',
    '3*x^2',
    'Canonical',
    '(mul (rat 3 1) (pow (sym x) (rat 2 1)))',
    'Expression tree',
    '5 nodes',
    'Mul',
    'Pow',
    'Num',
    'Assumptions',
    'relation >',
    'x > 0',
    '(gt (sym x) (rat 0 1))',
    'Simplification trace',
    'simplify_full',
    'MathIR',
    '#!mathir 2',
    'PowInt'
)

$TracePng = Join-Path $OutDir 'simplify-trace.png'
$TraceUia = Join-Path $OutDir 'simplify-trace-uia.txt'
$Trace = Invoke-Scenario -Label 'simplify-trace' -Document $TraceDoc -Png $TracePng -UiaPath $TraceUia -Required @(
    'INSPECTOR',
    'subject  TransformResult',
    'result.expression',
    'Pretty',
    'Canonical',
    'Expression tree',
    'Assumptions',
    'relation >',
    'Simplification trace',
    'logexp.exp-log',
    'classification  Conditional',
    'before  exp(log(x))',
    'after   x',
    'FiniteCondition',
    'MathIR',
    'param x'
)

if ($Diff.Hash -eq $Trace.Hash) { throw 'the two scenario frames are identical; the inspector did not change with the result' }
Write-CaptureLog "done diff_bytes=$($Diff.Bytes) diff_method=$($Diff.Method) trace_bytes=$($Trace.Bytes) trace_method=$($Trace.Method)"
