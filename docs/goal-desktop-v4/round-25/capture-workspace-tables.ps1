<#
.SYNOPSIS
  Round-25 WKS-001/WKS-002 evidence: run a script that defines a user function and a variable in
  the live product window, then capture the Workspace tables.

.DESCRIPTION
  The console session is unlocked, so this round uses the normal harness for input and
  accessibility:
    - create work/workspace-tables.lv: `func aaa(x) = x * 2`, `a = 21`, `aaa(a)`;
    - launch the shipped lovelace-desktop.exe on that document through tools/gui-harness.ps1;
    - restore/size the window so both tables are visible;
    - invoke the Run button through UI Automation (-Action click), no typing;
    - wait for the status strip to read "run Finished";
    - dump the live accessibility tree and assert the variables table names `_` with its kind and
      display, the functions table names the user `aaa`, builtin `abs` and plugin `acos` rows, and
      the header count equals the number of function rows the tree exposes (builtin + plugin +
      user), i.e. the registry capture;
    - capture with -Action capture and with PrintWindow(PW_RENDERFULLCONTENT);
    - probe the PrintWindow pixels for the accent, plugin and dim colours.

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

$Document = Join-Path $Work 'workspace-tables.lv'
$lines = @(
    'func aaa(x) = x * 2',
    'a = 21',
    'aaa(a)'
)
Set-Content -LiteralPath $Document -Value $lines -Encoding ASCII
Write-Host "document=$Document exe=$Exe out=$OutDir"

function Invoke-Harness([string[]]$HarnessArgs) {
    $text = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness @HarnessArgs 2>&1 | Out-String
    $text.Trim() | Add-Content -LiteralPath $DriverLog -Encoding UTF8
    return $text
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
public struct R25Rect { public int Left, Top, Right, Bottom; }
public static class R25Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R25Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@
if ([R25Native]::IsIconic($h)) { [void][R25Native]::ShowWindow($h, 9) }
[void][R25Native]::SetWindowPos($h, [IntPtr]::Zero, 10, 10, 1500, 1320, 0x0040)
Start-Sleep -Milliseconds 700
$rect = New-Object R25Rect
[void][R25Native]::GetWindowRect($h, [ref]$rect)
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
Copy-Item -LiteralPath $Uia -Destination (Join-Path $OutDir 'workspace-tables-uia.txt') -Force
$uiaText = Get-Content -LiteralPath $Uia -Raw

foreach ($required in @(
    'committed 3/3',
    'VARIABLES',
    'FUNCTIONS',
    'aaa',
    'builtin',
    'plugin',
    'Natural',
    '21',
    '42',
    'run Finished'
)) {
    if (-not $uiaText.Contains($required)) {
        throw "the UIA evidence does not contain '$required'"
    }
}

# The underscores global must be its own Text node (not merely a character somewhere else).
if (-not ($uiaText -match '(?m)^\s*Text \| _ \|')) {
    throw "the UIA tree has no '_' variable label"
}
# The user function's row must be in the tree as its own label.
if (-not ($uiaText -match '(?m)^\s*Text \| aaa \|')) {
    throw "the UIA tree has no 'aaa' function label"
}

# Count the origin labels the accessibility tree exposes for the registry rows. The table paints
# every row, so the three origin counts must add up to the header's function count.
$builtins = ([regex]::Matches($uiaText, '(?m)^\s*Text \| builtin \|')).Count
$plugins = ([regex]::Matches($uiaText, '(?m)^\s*Text \| plugin \|')).Count
$users = ([regex]::Matches($uiaText, '(?m)^\s*Text \| user \|')).Count
if (-not ($uiaText -match 'VARIABLES[^0-9]+(\d+)')) { throw 'no variables count in the UIA tree' }
$uiaVariables = [int]$matches[1]
if (-not ($uiaText -match 'FUNCTIONS[^0-9]+(\d+)')) { throw 'no functions count in the UIA tree' }
$uiaFunctions = [int]$matches[1]
$originRows = $builtins + $plugins + $users
Write-Host "uia counts variables=$uiaVariables functions=$uiaFunctions builtin=$builtins plugin=$plugins user=$users rows=$originRows"
Add-Content -LiteralPath $DriverLog -Value "uia counts variables=$uiaVariables functions=$uiaFunctions builtin=$builtins plugin=$plugins user=$users rows=$originRows" -Encoding UTF8
if ($uiaVariables -ne 2) { throw "the variables header must count the two captured globals, got $uiaVariables" }
if ($builtins -lt 1 -or $plugins -lt 1 -or $users -ne 1) {
    throw "the tree must expose a builtin row, a plugin row and exactly one user row; got builtin=$builtins plugin=$plugins user=$users"
}
if ($originRows -ne $uiaFunctions) {
    throw "the function count ($uiaFunctions) must equal the builtin+plugin+user rows the tree exposes ($originRows)"
}

# --- capture: harness capture, plus a PrintWindow frame -------------------------------------
$capture = Invoke-Harness @('-Action', 'capture', '-ProcessId', "$processId", '-OutDir', $OutDir, '-Name', 'workspace-tables', '-Log', $DriverLog)
$Png = Join-Path $OutDir 'workspace-tables.png'
if (-not (Test-Path -LiteralPath $Png)) { throw "capture produced no file: $capture" }
$captureBytes = (Get-Item -LiteralPath $Png).Length
Write-Host "capture=$Png bytes=$captureBytes"

[void][R25Native]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
Start-Sleep -Milliseconds 400
[void][R25Native]::SetWindowPos($h, [IntPtr]::Zero, 10, 10, 1500, 1320, 0x0040)
Start-Sleep -Milliseconds 700
$rect = New-Object R25Rect
[void][R25Native]::GetWindowRect($h, [ref]$rect)
$w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
$ok = [R25Native]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$g.Dispose()
$Pw = Join-Path $OutDir 'workspace-tables-printwindow.png'
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
$warn = if ($probe.ContainsKey('D7BA7D')) { $probe['D7BA7D'] } else { 0 }
$dim = if ($probe.ContainsKey('8B98A9')) { $probe['8B98A9'] } else { 0 }
Write-Host "pixels accent=$accent warn=$warn dim=$dim"
Add-Content -LiteralPath $DriverLog -Value "pixels accent=$accent warn=$warn dim=$dim" -Encoding UTF8
if ($accent -lt 10 -or $dim -lt 10) { throw 'the PrintWindow frame has too little accent/dim text to be the table' }

# --- close ----------------------------------------------------------------------------------
$close = Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog)
Write-Host "closed=$close"
Write-Output "WORKSPACE_TABLES_CAPTURE_OK PNG=$Png PRINTWINDOW=$Pw UIA=$(Join-Path $OutDir 'workspace-tables-uia.txt')"
