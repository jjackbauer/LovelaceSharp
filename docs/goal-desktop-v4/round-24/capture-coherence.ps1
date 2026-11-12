<#
.SYNOPSIS
  Round-24 RUN-003 evidence: run a four-statement script in the live product window and capture
  it with PrintWindow(PW_RENDERFULLCONTENT).

.DESCRIPTION
  The console session is unlocked, so this round uses the normal harness for the input and
  accessibility actions:
    - launch the app on a real .lv document through tools/gui-harness.ps1 -Action launch;
    - restore/focus and invoke the Run button through UI Automation (-Action click), no typing;
    - wait for the status strip to read "run Finished";
    - dump the live accessibility tree with -Action uia (read-only);
    - capture the window with -Action capture (foreground is available; the harness draws from
      the screen), then also take one PrintWindow(PW_RENDERFULLCONTENT) frame so the artifact is
      a true window render even if another window is momentarily on top;
    - assert the log tail names statement 4 and the workspace pane shows rev 1 / committed 4/4.

  The resize is the only action the harness does not expose; the scenario action resizes
  internally for the same reason (the default window is too short to show every run line).

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

$Document = Join-Path $Work 'coherence.lv'
# Four statements. Statement 2 is the one the capture's log line and b = 2 must agree on;
# statement 3 prints; statement 4 is the final value expression. No plot (APPDATA redirect is
# present, but this proof does not need one).
$lines = @('a = 1', 'b = a + 1', 'print("p")', 'b')
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

# --- one resize so the logs pane shows all run lines ---------------------------------------
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct R24Rect { public int Left, Top, Right, Bottom; }
public static class R24Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R24Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
"@
if ([R24Native]::IsIconic($h)) { [void][R24Native]::ShowWindow($h, 9) }
[void][R24Native]::SetWindowPos($h, [IntPtr]::Zero, 10, 10, 1500, 1320, 0x0040)
Start-Sleep -Milliseconds 600
$rect = New-Object R24Rect
[void][R24Native]::GetWindowRect($h, [ref]$rect)
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
$deadline = (Get-Date).AddSeconds(25)
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
Copy-Item -LiteralPath $Uia -Destination (Join-Path $OutDir 'coherence-uia.txt') -Force
$uiaText = Get-Content -LiteralPath $Uia -Raw
foreach ($required in @(
    '[progress] statement 4/4: b',
    '[result] = 2',
    '[timing] Run finished: 4 statements committed',
    'rev 1 - committed 4/4',
    'run Finished'
)) {
    if (-not $uiaText.Contains($required)) {
        # The middle dot in the pane line may be rendered as a different glyph in the tree.
        if ($required -eq 'rev 1 - committed 4/4' -and $uiaText.Contains('committed 4/4')) { continue }
        throw "the UIA evidence does not contain '$required'"
    }
}
$logLines = ($uiaText -split "`r?`n") | Where-Object { $_ -match 'Text \| \[(stdout|result|diagnostic|timing|progress|info|echo)\]' }
Write-Host "uia log lines=$($logLines.Count)"
for ($i = 0; $i -lt $logLines.Count; $i++) {
    Add-Content -LiteralPath $DriverLog -Value ("uia log[{0}] {1}" -f $i, $logLines[$i].Trim()) -Encoding UTF8
}
$joined = $logLines -join "`n"
$order = @('[progress] statement 4/4: b', '[result] = 2', '[timing] Run finished: 4 statements committed')
$last = -1
foreach ($marker in $order) {
    $pos = $joined.IndexOf($marker)
    if ($pos -le $last) { throw "log order is wrong at '$marker'" }
    $last = $pos
}

# --- capture: harness capture, plus a PrintWindow frame ------------------------------------
$capture = Invoke-Harness @('-Action', 'capture', '-ProcessId', "$processId", '-OutDir', $OutDir, '-Name', 'coherence', '-Log', $DriverLog)
$Png = Join-Path $OutDir 'coherence.png'
if (-not (Test-Path -LiteralPath $Png)) { throw "capture produced no file: $capture" }
$captureBytes = (Get-Item -LiteralPath $Png).Length
Write-Host "capture=$Png bytes=$captureBytes"

# A true window render independent of the screen: PrintWindow(PW_RENDERFULLCONTENT).
[void][R24Native]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
Start-Sleep -Milliseconds 400
[void][R24Native]::SetWindowPos($h, [IntPtr]::Zero, 10, 10, 1500, 1320, 0x0040)
Start-Sleep -Milliseconds 600
$rect = New-Object R24Rect
[void][R24Native]::GetWindowRect($h, [ref]$rect)
$w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$dc = $g.GetHdc()
$ok = [R24Native]::PrintWindow($h, $dc, 2)
$g.ReleaseHdc($dc)
$g.Dispose()
$Pw = Join-Path $OutDir 'coherence-printwindow.png'
$bmp.Save($Pw, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
$pwBytes = (Get-Item -LiteralPath $Pw).Length
Write-Host "printwindow=$Pw bytes=$pwBytes ok=$ok rect=${w}x${ht}"
Add-Content -LiteralPath $DriverLog -Value "capture file=$Png bytes=$captureBytes" -Encoding UTF8
Add-Content -LiteralPath $DriverLog -Value "printwindow file=$Pw bytes=$pwBytes ok=$ok rect=${w}x${ht}" -Encoding UTF8
if ($captureBytes -lt 20480 -or $pwBytes -lt 20480) { throw 'a capture is smaller than 20 KB; refusing it as empty' }

# --- close ----------------------------------------------------------------------------------
$close = Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog)
Write-Host "closed=$close"
Write-Output "COHERENCE_CAPTURE_OK PNG=$Png PRINTWINDOW=$Pw UIA=$(Join-Path $OutDir 'coherence-uia.txt')"
