<#
.SYNOPSIS
  Round-22 LOG-002 evidence: type a successful and then a deliberately failing script into the
  live desktop window, run both, and capture the log pane with its kind markers.

.DESCRIPTION
  WHY THIS SCRIPT EXISTS
  tools/gui-harness.ps1 -Action scenario needs the foreground and this console session is
  locked (GetForegroundWindow is the lock screen; SetForegroundWindow is refused). The product
  window still paints while occluded, so the evidence uses the round-20 method:

    1. launch the app on a real empty document through its positional argument;
    2. post real key messages to the window (WM_SETFOCUS, then WM_KEYDOWN / WM_CHAR /
       WM_KEYUP per character; VK_RETURN for newline). Winit consumes them exactly as it
       consumes SendKeys output, so the editor really types both scripts;
    3. invoke the Run button through UI Automation's InvokePattern (no foreground needed);
    4. restore keyboard focus to the editor through UI Automation's SetFocus (the Run invoke
       moves focus to the button), append a failing script to the same buffer, and run again;
    5. size the window so the logs pane is tall enough that every run line is visible at once.
       The window is 1296x1500 logical; when the locked session's compositor reports a 300%
       scale for the DPI-unaware product, the physical surface is 3888x4500 and PrintWindow
       renders the whole surface even though the desktop is smaller. Both states give a
       readable capture; the content probe below accepts either physical size.
    6. read the live accessibility tree with the unmodified harness (reads only, no foreground)
       to verify the exact log text and order;
    7. capture the window with PrintWindow(PW_RENDERFULLCONTENT); CopyFromScreen is black on a
       locked desktop. Probe the log crop for the six painted marker colours.

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
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$Log = Join-Path $Work 'capture-log.txt'
function Write-Log([string]$line) {
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    $text = "$stamp $line"
    Write-Host $text
    Add-Content -LiteralPath $Log -Value $text -Encoding UTF8
}

Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @"
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
using System.Text;

public struct R22Rect { public int Left, Top, Right, Bottom; }
public struct R22Point { public int X, Y; }

public static class R22Native {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R22Rect rect);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
  [DllImport("user32.dll")] public static extern bool ScreenToClient(IntPtr hWnd, ref R22Point point);
  [DllImport("kernel32.dll")] public static extern IntPtr OpenThread(uint access, bool inherit, uint threadId);
  [DllImport("kernel32.dll")] public static extern uint SuspendThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern uint ResumeThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}

public static class R22Pixels {
  /// Counts exact 0xRRGGBB pixels in a crop of a PNG. Glyph interiors keep the exact text
  /// colour, so a non-zero count proves that colour was painted in this crop.
  public static long[] Count(string path, int x0, int y0, int width, int height, int[] colors) {
    long[] counts = new long[colors.Length];
    using (Bitmap bmp = new Bitmap(path)) {
      int w = Math.Min(width, bmp.Width - x0);
      int h = Math.Min(height, bmp.Height - y0);
      if (w <= 0 || h <= 0) { return counts; }
      BitmapData data = bmp.LockBits(new Rectangle(x0, y0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
      try {
        byte[] buffer = new byte[data.Stride * h];
        Marshal.Copy(data.Scan0, buffer, 0, buffer.Length);
        for (int y = 0; y < h; y++) {
          int row = y * data.Stride;
          for (int x = 0; x < w; x++) {
            int i = row + x * 4;
            int rgb = (buffer[i + 2] << 16) | (buffer[i + 1] << 8) | buffer[i];
            for (int c = 0; c < colors.Length; c++) {
              if (colors[c] == rgb) { counts[c]++; break; }
            }
          }
        }
      } finally {
        bmp.UnlockBits(data);
      }
    }
    return counts;
  }

  // Returns the top row of the bottom-most contiguous band of the given RGB rows that covers at
  // least half the image width and is at least minRows tall. The logs pane header is the last
  // such full-width band above the status strip.
  public static int LastBandTop(string path, int rgb, int minRows) {
    using (Bitmap bmp = new Bitmap(path)) {
      int w = bmp.Width; int h = bmp.Height;
      int runEnd = -1;
      BitmapData data = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
      try {
        byte[] buffer = new byte[data.Stride * h];
        Marshal.Copy(data.Scan0, buffer, 0, buffer.Length);
        for (int y = h - 1; y >= 0; y--) {
          int row = y * data.Stride; int count = 0;
          for (int x = 0; x < w; x++) {
            int i = row + x * 4;
            int pixel = (buffer[i + 2] << 16) | (buffer[i + 1] << 8) | buffer[i];
            if (pixel == rgb) { count++; }
          }
          bool band = count * 2 >= w;
          if (band) { if (runEnd < 0) { runEnd = y; } }
          else if (runEnd >= 0) {
            if (runEnd - y + 1 >= minRows) { return y + 1; }
            runEnd = -1;
          }
        }
      } finally {
        bmp.UnlockBits(data);
      }
    }
    return -1;
  }
}
"@

$WM_SETFOCUS = 0x0007
$WM_KEYDOWN = 0x0100
$WM_KEYUP = 0x0101
$WM_CHAR = 0x0102
$WM_MOUSEMOVE = 0x0200
$WM_LBUTTONDOWN = 0x0201
$WM_LBUTTONUP = 0x0202
$VK_BACK = 0x08
$VK_UP = 0x26
$VK_PROCESSKEY = 0xE5
$PW_RENDERFULLCONTENT = 2

$env:APPDATA = Join-Path $OutDir 'appdata'
New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
$Document = Join-Path $Work 'log-kinds.lv'
Set-Content -LiteralPath $Document -Value '' -NoNewline -Encoding ASCII
Write-Log "workspace=$Work appdata=$env:APPDATA temp=$env:TEMP exe=$Exe out=$OutDir"

$env:__COMPAT_LAYER = 'DPIUNAWARE'
$p = Start-Process -FilePath $Exe -ArgumentList @($Document) -PassThru
Remove-Item Env:__COMPAT_LAYER
Write-Log "launch pid=$($p.Id) document=$Document compat_layer=DPIUNAWARE"

$h = [IntPtr]::Zero
for ($i = 0; $i -lt 300; $i++) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne 0) {
        $candidate = [IntPtr]$p.MainWindowHandle
        $probe = New-Object R22Rect
        if ([R22Native]::GetWindowRect($candidate, [ref]$probe)) {
            $pw = [int]($probe.Right - $probe.Left); $ph = [int]($probe.Bottom - $probe.Top)
            $probeTitle = New-Object System.Text.StringBuilder 256
            [void][R22Native]::GetWindowText($candidate, $probeTitle, 256)
            if ($pw -ge 800 -and $ph -ge 500 -and $probeTitle.ToString() -like '*Lovelace Desktop*') {
                $h = $candidate
                break
            }
        }
    }
    Start-Sleep -Milliseconds 100
}
if ($h -eq [IntPtr]::Zero) { Stop-Process -Id $p.Id -Force; throw 'the product window never appeared' }
$windowPid = [uint32]0
$windowTid = [R22Native]::GetWindowThreadProcessId($h, [ref]$windowPid)
$script:UiThread = [R22Native]::OpenThread(0x0002, $false, $windowTid)
if ($script:UiThread -eq [IntPtr]::Zero) { throw "cannot open the window thread $windowTid for suspend/resume" }
if ([R22Native]::IsIconic($h)) { [void][R22Native]::ShowWindow($h, 9) }
$dpi = [R22Native]::GetDpiForWindow($h)
if ($dpi -ne 96) { throw "the DPI-unaware launch still reports dpi=$dpi; refusing a scaled capture" }
[void][R22Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 3888, 4500, 0x0040)
Write-Log "dpi=$dpi requested=3888x4500 (1296x1500 logical at 300%)"
Start-Sleep -Milliseconds 800
$p.Refresh()
if ($p.HasExited) { throw "the product exited during the window resize (exit code $($p.ExitCode))" }
$rect = New-Object R22Rect
[void][R22Native]::GetWindowRect($h, [ref]$rect)
Write-Log "resized rect=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top) alive=$(-not $p.HasExited)"
[void][R22Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

function Wait-ExpectedWindow([int]$width, [int]$height) {
    # The locked session's compositor can briefly report the window at 3x (the round-20 note:
    # this monitor switched between 96 and 288 dpi). Re-apply the 1x geometry and wait for it.
    for ($i = 0; $i -lt 40; $i++) {
        $rect = New-Object R22Rect
        if ([R22Native]::GetWindowRect($h, [ref]$rect)) {
            $w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
            if ($w -eq $width -and $ht -eq $height) { return }
        }
        [void][R22Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, $width, $height, 0x0040)
        Start-Sleep -Milliseconds 250
    }
    throw "the window never settled at ${width}x${height}"
}

function Save-WindowShot([string]$Path, [string]$Label) {
    # A DPI switch can reshape the window between two calls, so every attempt re-applies the
    # geometry and then proves the capture really contains the full-width log header band. A
    # 432x400 surface blitted into a 1296x1500 bitmap fails that probe and is retried.
    for ($attempt = 1; $attempt -le 40; $attempt++) {
        [void][R22Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 3888, 4500, 0x0040)
        # Force a full repaint: after a DPI switch the window can be the right size while the
        # compositor still holds the previous small surface, which PrintWindow would blit into
        # the top-left corner of the bitmap.
        [void][R22Native]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
        [void][R22Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 700
        $rect = New-Object R22Rect
        if (-not [R22Native]::GetWindowRect($h, [ref]$rect)) { continue }
        $w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
        if ($w -ne 3888 -or $ht -ne 4500) {
            Write-Log "capture retry label=$Label attempt=$attempt rect=${w}x${ht}"
            continue
        }
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc()
        $ok = [R22Native]::PrintWindow($h, $dc, $PW_RENDERFULLCONTENT)
        $g.ReleaseHdc($dc)
        $g.Dispose()
        $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        $logTop = [R22Pixels]::LastBandTop($Path, 0x222830, 8)
        if ($ok -and $bytes -gt 20480 -and $logTop -ge 0) {
            Write-Log "capture label=$Label path=$Path rect=${w}x${ht} bytes=$bytes printwindow=$ok log_top=$logTop attempt=$attempt"
            return $bytes
        }
        Write-Log "capture retry label=$Label attempt=$attempt bytes=$bytes printwindow=$ok log_top=$logTop"
        [void][R22Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 700
    }
    throw "$Label capture failed its content probe after 40 attempts"
}

function Send-Key([int]$vk) {
    [void][R22Native]::SuspendThread($script:UiThread)
    try {
        [void][R22Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, [IntPtr]1)
        [void][R22Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, [IntPtr]0xC0000001)
    } finally {
        [void][R22Native]::ResumeThread($script:UiThread)
    }
    Start-Sleep -Milliseconds 25
}

function Type-Character([char]$ch) {
    # VK_PROCESSKEY makes TranslateMessage leave the posted WM_CHAR alone; the real character's
    # VK would let TranslateMessage insert its unshifted character first.
    [void][R22Native]::SuspendThread($script:UiThread)
    try {
        [void][R22Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$VK_PROCESSKEY, [IntPtr]1)
        [void][R22Native]::PostMessage($h, $WM_CHAR, [IntPtr][int]$ch, [IntPtr]1)
        [void][R22Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$VK_PROCESSKEY, [IntPtr]0xC0000001)
    } finally {
        [void][R22Native]::ResumeThread($script:UiThread)
    }
    Start-Sleep -Milliseconds 15
}

function Type-Newline {
    Send-Key 0x0D
}

function Type-Script([string[]]$lines) {
    for ($line = 0; $line -lt $lines.Count; $line++) {
        foreach ($ch in $lines[$line].ToCharArray()) { Type-Character $ch }
        if ($line -lt $lines.Count - 1) { Type-Newline }
    }
    Start-Sleep -Milliseconds 600
}

function Get-Root {
    return [System.Windows.Automation.AutomationElement]::FromHandle($h)
}

function Find-ByName([string]$name) {
    $root = Get-Root
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::NameProperty, $name)
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Get-Status {
    $root = Get-Root
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        $name = $text.Current.Name
        if ($name -match '\|\s*run ([A-Za-z]+)\s*$') { return $matches[1] }
    }
    return ''
}

function Get-LogPaneRect {
    $pane = Find-ByName 'Logs'
    if (-not $pane) { throw 'the UIA tree has no Logs pane' }
    $r = $pane.Current.BoundingRectangle
    return @{ Left = [int]$r.Left; Top = [int]$r.Top; Width = [int]$r.Width; Height = [int]$r.Height }
}

function Focus-Editor {
    # A UIA Invoke of Run hands egui focus to the button. SetFocus on the editor's accesskit
    # node is the no-foreground way back: the next posted WM_CHAR/WM_KEYDOWN then edits.
    $root = Get-Root
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit)
    $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($edit in $edits) {
        if ($edit.Current.Name -like 'Script editor*') {
            $edit.SetFocus()
            Start-Sleep -Milliseconds 400
            return
        }
    }
    throw 'the editor element was not found for SetFocus'
}

function Wait-Status([string]$wanted, [int]$timeoutMs) {
    $deadline = (Get-Date).AddMilliseconds($timeoutMs)
    while ((Get-Date) -lt $deadline) {
        $status = Get-Status
        if ($status -eq $wanted) { return $true }
        Start-Sleep -Milliseconds 150
    }
    Write-Log "wait-status wanted=$wanted last='$(Get-Status)'"
    return $false
}

function Invoke-Run {
    $run = Find-ByName 'Run'
    if (-not $run) { throw 'the UIA tree has no Run button' }
    $pattern = $run.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    Start-Sleep -Milliseconds 200
}

function Get-EditorName {
    $editor = Find-ByName 'Script editor'
    if (-not $editor) {
        $root = Get-Root
        $condition = New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit)
        $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
        foreach ($e in $edits) {
            if ($e.Current.Name -like 'Script editor*') { return $e.Current.Name }
        }
        return ''
    }
    return $editor.Current.Name
}

$scriptA = @('6 * 7')
$scriptB = @('print("hi")', '1 / 0')
Write-Log "scriptA='$($scriptA -join '\n')' appendedB='$($scriptB -join '\n')'"

[void][R22Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 300
Type-Script $scriptA
Write-Log "typed A editor='$(Get-EditorName)'"
if ((Get-EditorName) -notmatch 'Script editor, 1 line') { throw "script A did not land: '$(Get-EditorName)'" }

Invoke-Run
if (-not (Wait-Status 'Finished' 20000)) { throw 'run A never reached Finished' }
Write-Log "run A status=Finished editor='$(Get-EditorName)'"

# The Run invoke moved egui focus to the button. SetFocus on the editor's accesskit node is
# the no-foreground way back; the caret stays at the end of the buffer, so the failing script is
# appended to the same document instead of replacing it.
Focus-Editor
Type-Newline
Type-Script $scriptB
Write-Log "appended B editor='$(Get-EditorName)'"
if ((Get-EditorName) -notmatch 'Script editor, 3 lines') { throw "script B did not land: '$(Get-EditorName)'" }

Invoke-Run
if (-not (Wait-Status 'Failed' 20000)) { throw 'run B never reached Failed' }
Write-Log "run B status=Failed"

$Png = Join-Path $OutDir 'log-kinds.png'
$PngBytes = Save-WindowShot $Png 'log-kinds'

# The unmodified harness dumps the live accessibility tree: the exact painted log strings, in
# order, with no OCR and no foreground ownership.
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness -Action uia -ProcessId $p.Id -OutDir $Work -Log (Join-Path $Work 'uia-driver.log') | Out-Null
$Uia = Join-Path $Work 'uia-tree.txt'
if (-not (Test-Path -LiteralPath $Uia)) { throw 'the harness uia action produced no tree' }
Copy-Item -LiteralPath $Uia -Destination (Join-Path $OutDir 'log-kinds-uia.txt') -Force
$uiaText = Get-Content -LiteralPath $Uia -Raw
foreach ($required in @('[stdout] hi', '[result] = 42', '[diagnostic]', '[timing] Run failed', '[progress] statement', '[info] Run queued', '[timing] Run finished')) {
    if (-not $uiaText.Contains($required)) { throw "the UIA log text does not contain '$required'" }
}
if ($uiaText.Contains('[echo]')) { throw 'the UIA log unexpectedly contains an echo line' }
$logLines = ($uiaText -split "`r?`n") | Where-Object { $_ -match 'Text \| \[(echo|stdout|result|diagnostic|timing|progress|info)\]' }
Write-Log "uia log lines=$($logLines.Count)"
for ($i = 0; $i -lt $logLines.Count; $i++) {
    Write-Log ("uia log[{0}] {1}" -f $i, $logLines[$i].Trim())
}
$joined = $logLines -join "`n"
$order = @('[result] = 42', '[timing] Run finished', '[stdout] hi', '[diagnostic]', '[timing] Run failed')
$last = -1
foreach ($marker in $order) {
    $pos = $joined.IndexOf($marker)
    if ($pos -le $last) { throw "log order is wrong at '$marker'" }
    $last = $pos
}

# Crop the log pane for a readable detail image and probe the six painted marker colours.
# The logs pane header is the bottom-most full-width HEADER_BG (#222830) band; the full-width
# status strip below it is the bottom-most #1B1B1B band. This avoids UIA rectangles, which this
# locked session scales by 3 even though the window surface is 1x.
$logTop = [R22Pixels]::LastBandTop($Png, 0x222830, 8)
if ($logTop -lt 0) { throw 'the logs pane header band was not found in the capture' }
$statusTop = [R22Pixels]::LastBandTop($Png, 0x1B1B1B, 8)
if ($statusTop -le $logTop) { throw "the status strip band was not found below the log header (log_top=$logTop status_top=$statusTop)" }
$detailWidth = 2280
$detailHeight = $statusTop - $logTop - 2
if ($detailHeight -lt 200) { throw "the logs pane crop is only $detailHeight px tall" }
$full = New-Object System.Drawing.Bitmap($Png)
$detailBmp = New-Object System.Drawing.Bitmap($detailWidth, $detailHeight)
$g = [System.Drawing.Graphics]::FromImage($detailBmp)
$destRect = New-Object System.Drawing.Rectangle -ArgumentList 0, 0, $detailWidth, $detailHeight
$sourceRect = New-Object System.Drawing.Rectangle -ArgumentList 0, $logTop, $detailWidth, $detailHeight
$g.DrawImage($full, $destRect, $sourceRect, [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose()
$full.Dispose()
$Detail = Join-Path $OutDir 'log-kinds-logs.png'
$detailBmp.Save($Detail, [System.Drawing.Imaging.ImageFormat]::Png)
$detailBmp.Dispose()
Write-Log "detail crop path=$Detail bytes=$((Get-Item -LiteralPath $Detail).Length) source=0,$logTop,${detailWidth}x${detailHeight}"

$names = @('info', 'stdout', 'result', 'diagnostic', 'timing', 'progress')
[int[]]$colors = @(
    0x8b98a9, # info: DIM
    0xd8dee9, # stdout: TEXT
    0x4ec9b0, # result: ACCENT
    0xff7b72, # diagnostic: ERROR
    0xd7ba7d, # timing: WARN
    0x7fb0d8  # progress: LOG_PROGRESS
)
$probeX = 8
$probeY = $logTop + 24
$probeWidth = $detailWidth - 16
$probeHeight = $detailHeight - 30
$counts = [R22Pixels]::Count($Png, $probeX, $probeY, $probeWidth, $probeHeight, $colors)
$missing = @()
for ($i = 0; $i -lt $names.Count; $i++) {
    Write-Log "palette $($names[$i]) #$('{0:X6}' -f $colors[$i]) pixels=$($counts[$i])"
    if ($counts[$i] -le 0) { $missing += $names[$i] }
}
if ($missing.Count -gt 0) { throw "the log crop is missing these marker colours: $($missing -join ', ')" }

[void][R22Native]::CloseHandle($script:UiThread)
$graceful = $p.CloseMainWindow()
if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; Write-Log 'close forced' }
else { Write-Log "close graceful=$graceful" }
Write-Log "SUMMARY png=$Png bytes=$PngBytes logs_detail=$Detail log_lines=$($logLines.Count) markers=$($names.Count)"
Write-Output "LOG_KINDS_CAPTURE_OK PNG=$Png BYTES=$PngBytes DETAIL=$Detail LOG_LINES=$($logLines.Count)"
