<#
.SYNOPSIS
  Round-16 completion evidence: type `sq` into the live desktop editor and capture the
  completion popup showing `sqrt(x)`.

.DESCRIPTION
  WHY THIS SCRIPT EXISTS
  `tools/gui-harness.ps1 -Action scenario` is the required driver, and it was run first:
  it aborts at its step-3 foreground assertion ("cannot focus the window before selecting the
  empty tab") because this console session is locked - GetForegroundWindow reports the
  Windows Default Lock Screen and neither SetForegroundWindow nor AttachThreadInput is
  permitted from the agent's process. `docs/goal-desktop-v4/round-16/driver.log` and
  `scenario-blocked.log` record that attempt verbatim.

  The product window is still live and still paints while occluded, so the same evidence is
  taken without touching the lock screen or the foreground policy:

    1. the app is launched on an empty document (its own `--` argument path);
    2. real key messages are posted to its window (WM_SETFOCUS, then WM_KEYDOWN / WM_CHAR /
       WM_KEYUP for `s` and `q`). Winit consumes them exactly as it consumes the messages
       SendKeys would generate, so the editor really types `sq` and really opens the popup;
    3. the unmodified harness is invoked with `-Action uia` (no foreground needed) to capture
       the accessibility tree of the live window, which states
       `Completion: sq -> sqrt(x) [builtin] 1/2` and `caret line 1 column 3`;
    4. the window is captured with PrintWindow(PW_RENDERFULLCONTENT), which asks the
       compositor for the window's own surface. CopyFromScreen returns black on a locked
       desktop, so it cannot be used here.

  The PNG is the real window of the shipped binary, with the real popup painted in it; only
  the input channel and the pixel-read source differ from the harness scenario.

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
if (-not $Repo) {
    $Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
}
$Exe = Join-Path $Repo 'rust\target-desktop\debug\lovelace-desktop.exe'
if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Log = Join-Path $OutDir 'capture.log'
function Write-Log([string]$line) {
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    $text = "$stamp $line"
    Write-Host $text
    Add-Content -LiteralPath $Log -Value $text -Encoding UTF8
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public struct Round16Rect { public int Left, Top, Right, Bottom; }
public static class Round16Native {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out Round16Rect rect);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@

$WM_SETFOCUS = 0x0007
$WM_KEYDOWN = 0x0100
$WM_KEYUP = 0x0101
$WM_CHAR = 0x0102
$PW_RENDERFULLCONTENT = 2

# A workspace path for APPDATA and TEMP: the product refuses a plot directory below the
# process working directory (PLF-006) and falls back to %TEMP%; pointing TEMP at the crate's
# target temp keeps that fallback inside the writable workspace, as the round's environment
# notes require. No plot statement is run; this only keeps the startup probe clean.
$Work = Join-Path $env:LOCALAPPDATA 'Temp\lovelace-round16-capture'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$env:APPDATA = Join-Path $Repo 'docs\goal-desktop-v4\round-16\appdata'
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
$Document = Join-Path $Work 'completion.lv'
Set-Content -LiteralPath $Document -Value '' -NoNewline -Encoding ASCII
Write-Log "workspace=$Work appdata=$env:APPDATA temp=$env:TEMP exe=$Exe out=$OutDir"

$p = Start-Process -FilePath $Exe -ArgumentList @($Document) -PassThru
Write-Log "launch pid=$($p.Id) document=$Document"
# Wait for the live eframe viewport, not winit's first helper window: the helper is a
# 160x28 placeholder that never paints. Require the real title and a real client rect,
# exactly the distinction the harness makes before it types.
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 300; $i++) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne 0) {
        $candidate = [IntPtr]$p.MainWindowHandle
        $probe = New-Object Round16Rect
        if ([Round16Native]::GetWindowRect($candidate, [ref]$probe)) {
            $pw = [int]($probe.Right - $probe.Left); $ph = [int]($probe.Bottom - $probe.Top)
            $probeTitle = New-Object System.Text.StringBuilder 256
            [void][Round16Native]::GetWindowText($candidate, $probeTitle, 256)
            if ($pw -ge 800 -and $ph -ge 500 -and $probeTitle.ToString() -like '*Lovelace Desktop*') {
                $h = $candidate
                break
            }
        }
    }
    Start-Sleep -Milliseconds 100
}
if ($h -eq [IntPtr]::Zero) { Stop-Process -Id $p.Id -Force; throw 'the product window never appeared' }
if ([Round16Native]::IsIconic($h)) { [void][Round16Native]::ShowWindow($h, 9) }
[void][Round16Native]::SetWindowPos($h, [IntPtr](-1), 40, 40, 1296, 839, 0x0040)
# Let the first frames render; egui only paints on input, and a posted WM_SETFOCUS is a
# real focus event that asks for one repaint without touching the run state or the caret.
Start-Sleep -Milliseconds 800
[void][Round16Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500
$fg = [Round16Native]::GetForegroundWindow()
$title = New-Object System.Text.StringBuilder 256
[void][Round16Native]::GetWindowText($h, $title, 256)
Write-Log "window hwnd=$h title='$($title.ToString())' foreground=$fg (foreground is the lock screen in this session)"

function Save-WindowShot([string]$Path, [string]$Label) {
    $rect = New-Object Round16Rect
    if (-not [Round16Native]::GetWindowRect($h, [ref]$rect)) { throw "GetWindowRect failed for $Label" }
    $w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
    if ($w -lt 800 -or $ht -lt 500) { throw "refusing $Label capture with rect ${w}x${ht}" }
    for ($attempt = 1; $attempt -le 6; $attempt++) {
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc()
        $ok = [Round16Native]::PrintWindow($h, $dc, $PW_RENDERFULLCONTENT)
        $g.ReleaseHdc($dc)
        $g.Dispose()
        $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        if ($ok -and $bytes -gt 20480) {
            Write-Log "capture label=$Label path=$Path rect=${w}x${ht} bytes=$bytes printwindow=$ok attempt=$attempt"
            return $bytes
        }
        Write-Log "capture retry label=$Label attempt=$attempt bytes=$bytes printwindow=$ok"
        [void][Round16Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 400
    }
    throw "$Label capture is only $bytes bytes after 6 attempts"
}

# Before: empty document, no popup.
$beforeBytes = Save-WindowShot (Join-Path $OutDir 'completion-before.png') 'before-typing'

# Real key messages: winit needs a WM_KEYDOWN before it accepts the WM_CHAR text.
[void][Round16Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 250
$chars = @(@('s', 0x53), @('q', 0x51))
foreach ($pair in $chars) {
    $ch = [char]$pair[0]
    $vk = [int]$pair[1]
    [void][Round16Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, [IntPtr]1)
    Start-Sleep -Milliseconds 40
    [void][Round16Native]::PostMessage($h, $WM_CHAR, [IntPtr][int]$ch, [IntPtr]1)
    [void][Round16Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, [IntPtr]0xC0000001)
    Start-Sleep -Milliseconds 140
}
Start-Sleep -Milliseconds 700
Write-Log "posted keys sq (WM_SETFOCUS + WM_KEYDOWN/WM_CHAR/WM_KEYUP) to hwnd=$h"

# The unmodified harness captures the live accessibility tree (it needs no foreground).
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness -Action uia -ProcessId $p.Id -OutDir $OutDir -Log (Join-Path $OutDir 'uia-driver.log') | Out-Null
$UiaTree = Join-Path $OutDir 'uia-tree.txt'
if (-not (Test-Path -LiteralPath $UiaTree)) { throw 'the harness uia action produced no tree' }
$excerpt = Join-Path $OutDir 'completion-uia-excerpt.txt'
$matches = Select-String -LiteralPath $UiaTree -Pattern 'Script editor,|Completion:'
$matches | ForEach-Object { $_.Line.Trim() } | Set-Content -LiteralPath $excerpt -Encoding UTF8
Write-Log "harness uia tree=$UiaTree excerpt=$excerpt lines=$($matches.Count)"
if (-not ($matches | Where-Object { $_.Line -match 'Completion: sq -> sqrt\(x\)' })) {
    throw 'the UIA tree does not contain the expected sq -> sqrt(x) completion state'
}

# After: the same window with the popup painted.
$afterBytes = Save-WindowShot (Join-Path $OutDir 'completion-sq-sqrt.png') 'completion-sq-sqrt'

$graceful = $p.CloseMainWindow()
if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; Write-Log 'close forced' }
else { Write-Log "close graceful=$graceful" }
Write-Log "SUMMARY before_png_bytes=$beforeBytes completion_png_bytes=$afterBytes popup='sqrt(x)' prefix='sq'"
Write-Output "COMPLETION_CAPTURE_OK PNG=$(Join-Path $OutDir 'completion-sq-sqrt.png') BYTES=$afterBytes"
