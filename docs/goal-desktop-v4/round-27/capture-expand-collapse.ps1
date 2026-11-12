<#
.SYNOPSIS
  Round-27 RES-004 evidence: the same solve result captured twice - every nested field collapsed,
  then one specific field (solutions) expanded - with the accessibility names and the tree
  contents visible in each frame.

.DESCRIPTION
  The console session is unlocked, so this round uses the normal harness for launch, UI
  Automation invocation, UIA dumps and capture:
    - create work/solve-record.lv:
        x = symbol("x")
        solve_full(x^2 - 4 == 0, x)
      The last value is a SolveResult record with 12 fields; `solutions` is a vector of two
      nested Solution records, each with a symbolic `value` field;
    - launch the shipped lovelace-desktop.exe on that document through tools/gui-harness.ps1;
    - restore/size the window to 2500x1380 so the whole RESULT tree fits the 2560x1440 screen;
    - invoke Run through UI Automation (no typing);
    - wait for the status strip to read "run Finished";
    - collapse every direct field header with its UIA InvokePattern (12 fields);
    - assert the UIA names now state the path and the collapsed state, and that the nested
      Solution rows and the pretty/canonical renderings are gone from the tree;
    - PrintWindow(PW_RENDERFULLCONTENT) frame 1: all fields collapsed;
    - expand exactly one field, `solutions`, by keyboard: UI Automation SetFocus on its header,
      then a Space key sent to the foreground window (the product's keyboard toggle);
    - assert the name flipped to "expanded" and that the nested content is back;
    - PrintWindow frame 2: the specific field expanded;
    - probe both frames for the pane palette (accent, text, dim);
    - close the application.

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

$Document = Join-Path $Work 'solve-record.lv'
Set-Content -LiteralPath $Document -Value @(
    'x = symbol("x")',
    'solve_full(x^2 - 4 == 0, x)'
) -Encoding ASCII
Write-Host "document=$Document exe=$Exe out=$OutDir"

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
public struct R27Rect { public int Left, Top, Right, Bottom; }
public static class R27Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R27Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@

# --- launch ---------------------------------------------------------------------------------
$launch = Invoke-Harness @('-Action', 'launch', '-Exe', $Exe, '-Arguments', $Document, '-Log', $DriverLog)
$processId = 0
if ($launch -match 'PROCESS_ID=(\d+)') { $processId = [int]$matches[1] }
if ($processId -eq 0) { throw "launch did not report a process id: $launch" }
Write-CaptureLog "launch pid=$processId"

$wait = Invoke-Harness @('-Action', 'wait', '-ProcessId', "$processId", '-Log', $DriverLog)
if ($wait -notmatch 'HWND=(\d+)') { throw "wait did not report a window handle: $wait" }
$h = [IntPtr][long]$matches[1]
Write-CaptureLog "window hwnd=$h"

if ([R27Native]::IsIconic($h)) { [void][R27Native]::ShowWindow($h, 9) }
[void][R27Native]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
Start-Sleep -Milliseconds 900
$rect = New-Object R27Rect
[void][R27Native]::GetWindowRect($h, [ref]$rect)
Write-CaptureLog "resized=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"

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
Write-CaptureLog "status='$status'"
Add-Content -LiteralPath $DriverLog -Value "status='$status'" -Encoding UTF8
if ($status -notmatch '\|\s*run Finished\s*$') { throw "the run never reached Finished: '$status'" }
Start-Sleep -Milliseconds 600

# --- UI Automation helpers over the live result tree ----------------------------------------
function Get-Buttons {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button)
    return $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Get-ButtonNames {
    $names = @()
    foreach ($button in (Get-Buttons)) { $names += $button.Current.Name }
    return $names
}

function Find-ResultButton([string]$Path, [string]$State) {
    foreach ($button in (Get-Buttons)) {
        if ($button.Current.Name.Contains("[$Path] $State")) { return $button }
    }
    return $null
}

function Invoke-ResultButton($button) {
    $pattern = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
}

# The direct field headers are the buttons whose accessible path is `result.<name>`; the root
# record is `result` and the nested rows/cells add brackets, so the regex selects 12 fields.
function Get-DirectFieldPaths {
    $paths = @()
    foreach ($button in (Get-Buttons)) {
        if ($button.Current.Name -match '\[(result\.[A-Za-z_]+)\]\s+expanded$') {
            $paths += $matches[1]
        }
    }
    return $paths
}

# --- all fields collapsed -------------------------------------------------------------------
$fieldPaths = @()
for ($attempt = 0; $attempt -lt 20; $attempt++) {
    $fieldPaths = @(Get-DirectFieldPaths)
    if ($fieldPaths.Count -ge 12) { break }
    Start-Sleep -Milliseconds 250
}
$fieldPaths = @($fieldPaths | Sort-Object -Unique)
Write-CaptureLog "field paths=$($fieldPaths.Count): $($fieldPaths -join ', ')"
if ($fieldPaths.Count -ne 12) { throw "expected 12 direct field headers, found $($fieldPaths.Count)" }
if (-not ($fieldPaths -contains 'result.solutions')) { throw 'the solutions field is not a direct header' }

$initialNames = @(Get-ButtonNames)
$expandedSolutions = @($initialNames | Where-Object { $_.Contains('[result.solutions] expanded') })
if ($expandedSolutions.Count -ne 1) {
    throw "the initial tree does not expose one expanded solutions header: $($expandedSolutions -join ' | ')"
}
Write-CaptureLog "initial solutions name='$($expandedSolutions[0])'"

foreach ($path in $fieldPaths) {
    $button = Find-ResultButton $path 'expanded'
    if (-not $button) { throw "no expanded header for $path just before collapsing it" }
    Invoke-ResultButton $button
    Start-Sleep -Milliseconds 220
    $deadline = (Get-Date).AddSeconds(5)
    while ((Get-Date) -lt $deadline) {
        if (Find-ResultButton $path 'collapsed') { break }
        Start-Sleep -Milliseconds 100
    }
    if (-not (Find-ResultButton $path 'collapsed')) { throw "header $path did not collapse" }
}
Write-CaptureLog 'all 12 direct field headers collapsed'

function Save-PrintWindow([IntPtr]$Hwnd, [string]$Path) {
    [void][R27Native]::RedrawWindow($Hwnd, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
    Start-Sleep -Milliseconds 300
    [void][R27Native]::SetWindowPos($Hwnd, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
    Start-Sleep -Milliseconds 700
    $windowRect = New-Object R27Rect
    [void][R27Native]::GetWindowRect($Hwnd, [ref]$windowRect)
    $w = [int]($windowRect.Right - $windowRect.Left)
    $ht = [int]($windowRect.Bottom - $windowRect.Top)
    $bmp = New-Object System.Drawing.Bitmap($w, $ht)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $dc = $g.GetHdc()
    $ok = [R27Native]::PrintWindow($Hwnd, $dc, 2)
    $g.ReleaseHdc($dc)
    $g.Dispose()
    $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    $bytes = (Get-Item -LiteralPath $Path).Length
    if ($bytes -lt 20480) { throw "PrintWindow frame $Path is smaller than 20 KB ($bytes); refusing it as empty" }
    Write-CaptureLog "printwindow=$Path bytes=$bytes ok=$ok rect=${w}x${ht}"
    Add-Content -LiteralPath $DriverLog -Value "printwindow file=$Path bytes=$bytes ok=$ok rect=${w}x${ht}" -Encoding UTF8
    return $bytes
}

function Save-UiaDump([string]$Path) {
    $uia = Invoke-Harness @('-Action', 'uia', '-ProcessId', "$processId", '-OutDir', $Work, '-Log', $DriverLog)
    $produced = Join-Path $Work 'uia-tree.txt'
    if (-not (Test-Path -LiteralPath $produced)) { throw "uia action produced no tree: $uia" }
    Copy-Item -LiteralPath $produced -Destination $Path -Force
    $bytes = (Get-Item -LiteralPath $Path).Length
    Write-CaptureLog "uia=$Path bytes=$bytes"
    return (Get-Content -LiteralPath $Path -Raw)
}

# --- capture 1: all collapsed ---------------------------------------------------------------
$CollapsedPng = Join-Path $OutDir 'expand-collapse-collapsed.png'
$collapsedBytes = Save-PrintWindow $h $CollapsedPng
$CollapsedUia = Join-Path $OutDir 'expand-collapse-collapsed-uia.txt'
$collapsedText = Save-UiaDump $CollapsedUia

foreach ($path in $fieldPaths) {
    if (-not $collapsedText.Contains("[$path] collapsed")) {
        throw "the collapsed UIA tree does not list '$path' as collapsed"
    }
}
foreach ($gone in @('Solution - ', '[result.solutions[0]]', 'pretty     ', 'canonical  ')) {
    if ($collapsedText.Contains($gone)) {
        throw "the collapsed tree still exposes '$gone'"
    }
}
Write-CaptureLog 'uia collapsed: 12 field paths state collapsed; nested rows and renderings absent'

# --- expand exactly one field by keyboard ---------------------------------------------------
$solutions = Find-ResultButton 'result.solutions' 'collapsed'
if (-not $solutions) { throw 'no collapsed solutions header to expand' }
$solutions.SetFocus()
Start-Sleep -Milliseconds 250
$keys = Invoke-Harness @('-Action', 'keys', '-ProcessId', "$processId", '-Keys', ' ', '-Log', $DriverLog)
if ($keys -notmatch 'SENT=') { throw "the harness did not send Space: $keys" }
$method = 'keyboard Space on the UIA-focused header'
$deadline = (Get-Date).AddSeconds(8)
$expanded = $false
while ((Get-Date) -lt $deadline) {
    if (Find-ResultButton 'result.solutions' 'expanded') { $expanded = $true; break }
    Start-Sleep -Milliseconds 150
}
if (-not $expanded) {
    # Recorded fallback: the same header toggled by its UIA InvokePattern (the click route).
    $method = 'uia invoke fallback (Space did not toggle the focused header)'
    $button = Find-ResultButton 'result.solutions' 'collapsed'
    Invoke-ResultButton $button
    Start-Sleep -Milliseconds 400
    if (-not (Find-ResultButton 'result.solutions' 'expanded')) {
        throw 'the solutions header could not be expanded by Space or by InvokePattern'
    }
}
Write-CaptureLog "expanded result.solutions method=$method"
Add-Content -LiteralPath $DriverLog -Value "expanded result.solutions method=$method" -Encoding UTF8

# --- capture 2: the specific field expanded -------------------------------------------------
$ExpandedPng = Join-Path $OutDir 'expand-collapse-expanded.png'
$expandedBytes = Save-PrintWindow $h $ExpandedPng
$ExpandedUia = Join-Path $OutDir 'expand-collapse-expanded-uia.txt'
$expandedText = Save-UiaDump $ExpandedUia

if (-not $expandedText.Contains('[result.solutions] expanded')) {
    throw 'the expanded UIA tree does not list result.solutions as expanded'
}
foreach ($required in @(
    '[result.solutions[0]] expanded',
    '[result.solutions[1]] expanded',
    '[result.solutions[0].value] expanded',
    '[result.status] collapsed',
    'pretty',
    'canonical'
)) {
    if (-not $expandedText.Contains($required)) {
        throw "the expanded UIA tree does not expose '$required'"
    }
}
Write-CaptureLog 'uia expanded: solutions open with nested rows and renderings; status still collapsed'

# --- palette probe of both frames -----------------------------------------------------------
function Get-Palette($Path) {
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
$collapsedPalette = @(Get-Palette $CollapsedPng)
$expandedPalette = @(Get-Palette $ExpandedPng)
$collapsedHash = (Get-FileHash -LiteralPath $CollapsedPng -Algorithm SHA256).Hash
$expandedHash = (Get-FileHash -LiteralPath $ExpandedPng -Algorithm SHA256).Hash
Write-CaptureLog "palette collapsed accent=$($collapsedPalette[0]) text=$($collapsedPalette[1]) dim=$($collapsedPalette[2])"
Write-CaptureLog "palette expanded accent=$($expandedPalette[0]) text=$($expandedPalette[1]) dim=$($expandedPalette[2])"
Write-CaptureLog "sha256 collapsed=$collapsedHash expanded=$expandedHash"
foreach ($palette in @($collapsedPalette, $expandedPalette)) {
    # The collapsed frame hides most of the TEXT-coloured values on purpose, so the exact-colour
    # text count is low there; the check only has to prove the pane palette is present at all.
    if ($palette[0] -lt 50 -or $palette[1] -lt 20 -or $palette[2] -lt 50) {
        throw "a frame does not carry the pane palette (accent=$($palette[0]) text=$($palette[1]) dim=$($palette[2]))"
    }
}
if ($collapsedHash -eq $expandedHash) { throw 'the two frames are identical; the collapse state did not change the tree' }

# --- close ----------------------------------------------------------------------------------
[void](Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog))
Write-CaptureLog "done collapsed=$collapsedBytes expanded=$expandedBytes method=$method"
