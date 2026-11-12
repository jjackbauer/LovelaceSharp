<#
.SYNOPSIS
  Round-23 RUN-002 / LOG-003 evidence: type a one-line expression into the live quick-eval
  field, press Enter, and capture the window with the echo and result lines and the updated
  status strip.

.DESCRIPTION
  WHY THIS SCRIPT EXISTS
  tools/gui-harness.ps1 -Action scenario needs the foreground and this console session is
  locked (GetForegroundWindow is the lock screen; SetForegroundWindow is refused). The product
  window still paints while occluded, so the evidence uses the posted-input method already
  established in round 22:

    1. launch the app on a real empty document through its positional argument;
    2. find the quick-eval field in the live accessibility tree and give it focus with UI
       Automation's SetFocus (no foreground needed);
    3. post real key messages to the window: WM_KEYDOWN with the character's VK, WM_CHAR, and
       WM_KEYUP for every character, then WM_KEYDOWN/WM_KEYUP for VK_RETURN. Winit consumes them
       exactly as it consumes SendKeys output, so the field really receives "12 - 5" and Enter;
    4. wait for the status strip to report run Finished (the quick-eval run is asynchronous);
    5. read the field's ValuePattern before and after submission (text accepted, then cleared)
       and the accessibility tree with the unmodified harness (reads only, no foreground);
    6. assert the log contains exactly one "[echo] >> 12 - 5" plus the run's [result],
       [progress] and [timing] lines, and that the editor still reports its original one-line
       empty buffer with the same revision;
    7. size the window so the logs pane is tall enough for every run line, then capture the
       window with PrintWindow(PW_RENDERFULLCONTENT); CopyFromScreen is black on a locked
       desktop. Probe the captured log crop for the exact painted echo and result colours.

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

public struct R23Rect { public int Left, Top, Right, Bottom; }
public struct R23Point { public int X, Y; }

public static class R23Native {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R23Rect rect);
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
  [DllImport("kernel32.dll")] public static extern IntPtr OpenThread(uint access, bool inherit, uint threadId);
  [DllImport("kernel32.dll")] public static extern uint SuspendThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern uint ResumeThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}

public static class R23Pixels {
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
$VK_RETURN = 0x0D
$PW_RENDERFULLCONTENT = 2

$env:APPDATA = Join-Path $OutDir 'appdata'
New-Item -ItemType Directory -Force -Path $env:APPDATA | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
$Document = Join-Path $Work 'quick-eval.lv'
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
        $probe = New-Object R23Rect
        if ([R23Native]::GetWindowRect($candidate, [ref]$probe)) {
            $pw = [int]($probe.Right - $probe.Left); $ph = [int]($probe.Bottom - $probe.Top)
            $probeTitle = New-Object System.Text.StringBuilder 256
            [void][R23Native]::GetWindowText($candidate, $probeTitle, 256)
            if ($pw -ge 800 -and $ph -ge 500 -and $probeTitle.ToString() -like '*Lovelace Desktop*') {
                $h = $candidate
                break
            }
        }
    }
    Start-Sleep -Milliseconds 100
}
if ($h -eq [IntPtr]::Zero) { Stop-Process -Id $p.Id -Force; throw 'the product window never appeared' }
$foreground = [R23Native]::GetForegroundWindow()
Write-Log "window hwnd=$h foreground=$foreground locked=$($foreground -ne $h)"
$windowPid = [uint32]0
$windowTid = [R23Native]::GetWindowThreadProcessId($h, [ref]$windowPid)
$script:UiThread = [R23Native]::OpenThread(0x0002, $false, $windowTid)
if ($script:UiThread -eq [IntPtr]::Zero) { throw "cannot open the window thread $windowTid for suspend/resume" }
if ([R23Native]::IsIconic($h)) { [void][R23Native]::ShowWindow($h, 9) }
$dpi = [R23Native]::GetDpiForWindow($h)
if ($dpi -ne 96) { throw "the DPI-unaware launch still reports dpi=$dpi; refusing a scaled capture" }
[void][R23Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 3888, 4500, 0x0040)
Write-Log "dpi=$dpi requested=3888x4500 (1296x1500 logical at 300%)"
Start-Sleep -Milliseconds 800
$p.Refresh()
if ($p.HasExited) { throw "the product exited during the window resize (exit code $($p.ExitCode))" }
$rect = New-Object R23Rect
[void][R23Native]::GetWindowRect($h, [ref]$rect)
Write-Log "resized rect=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top) alive=$(-not $p.HasExited)"
[void][R23Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500

function Save-WindowShot([string]$Path, [string]$Label) {
    # A DPI switch can reshape the window between two calls, so every attempt re-applies the
    # geometry and then proves the capture really contains the full-width log header band. A
    # stale small surface blitted into the top-left corner fails that probe and is retried.
    for ($attempt = 1; $attempt -le 40; $attempt++) {
        [void][R23Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 3888, 4500, 0x0040)
        [void][R23Native]::RedrawWindow($h, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
        [void][R23Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 700
        $rect = New-Object R23Rect
        if (-not [R23Native]::GetWindowRect($h, [ref]$rect)) { continue }
        $w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
        if ($w -ne 3888 -or $ht -ne 4500) {
            Write-Log "capture retry label=$Label attempt=$attempt rect=${w}x${ht}"
            continue
        }
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc()
        $ok = [R23Native]::PrintWindow($h, $dc, $PW_RENDERFULLCONTENT)
        $g.ReleaseHdc($dc)
        $g.Dispose()
        $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        $logTop = [R23Pixels]::LastBandTop($Path, 0x222830, 8)
        if ($ok -and $bytes -gt 20480 -and $logTop -ge 0) {
            Write-Log "capture label=$Label path=$Path rect=${w}x${ht} bytes=$bytes printwindow=$ok log_top=$logTop attempt=$attempt"
            return $bytes
        }
        Write-Log "capture retry label=$Label attempt=$attempt bytes=$bytes printwindow=$ok log_top=$logTop"
        [void][R23Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 700
    }
    throw "$Label capture failed its content probe after 40 attempts"
}

function Send-Key([int]$vk) {
    [void][R23Native]::SuspendThread($script:UiThread)
    try {
        [void][R23Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, [IntPtr]1)
        [void][R23Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, [IntPtr]0xC0000001)
    } finally {
        [void][R23Native]::ResumeThread($script:UiThread)
    }
    Start-Sleep -Milliseconds 60
}

function VkFor([char]$ch) {
    # The expression deliberately uses only keys whose unshifted VK is well defined. The
    # keydown carries the real character VK (not VK_PROCESSKEY): in this build winit only
    # forwards the following WM_CHAR to the focused egui TextEdit when the keydown is a
    # character key, which is also what a real keyboard sends.
    switch ($ch) {
        ' ' { return 0x20 }
        '-' { return 0xBD }
        '+' { return 0xBB }
        '/' { return 0xBF }
        '*' { return 0x6A }
        default { return [int][char]::ToUpper($ch) }
    }
}

function Type-Characters([string]$text) {
    foreach ($ch in $text.ToCharArray()) {
        $vk = VkFor $ch
        [void][R23Native]::SuspendThread($script:UiThread)
        try {
            [void][R23Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, [IntPtr]1)
            [void][R23Native]::PostMessage($h, $WM_CHAR, [IntPtr][int]$ch, [IntPtr]1)
            [void][R23Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, [IntPtr]0xC0000001)
        } finally {
            [void][R23Native]::ResumeThread($script:UiThread)
        }
        Start-Sleep -Milliseconds 25
    }
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

function Find-QuickEvalEdit {
    $root = Get-Root
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit)
    $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($edit in $edits) {
        if ($edit.Current.Name -eq 'Quick eval') { return $edit }
    }
    return $null
}

function Get-QuickEvalValue {
    $edit = Find-QuickEvalEdit
    if (-not $edit) { return '<missing>' }
    try {
        $pattern = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        return $pattern.Current.Value
    } catch {
        return "<no ValuePattern: $($_.Exception.Message)>"
    }
}

function Focus-QuickEval {
    $edit = Find-QuickEvalEdit
    if (-not $edit) { throw 'the accessibility tree has no Quick eval Edit node' }
    $edit.SetFocus()
    Start-Sleep -Milliseconds 500
    Write-Log "quick-eval focus set name='$($edit.Current.Name)' focusable=$($edit.Current.IsKeyboardFocusable)"
}

function Get-EditorName {
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

function Get-StatusElement {
    $root = Get-Root
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        $name = $text.Current.Name
        if ($name -match '\|\s*run ([A-Za-z]+)\s*$') { return $text }
    }
    return $null
}

function Get-Status {
    $element = Get-StatusElement
    if (-not $element) { return '' }
    return $element.Current.Name
}

function Get-StatusPhase {
    $status = Get-Status
    if ($status -match '\|\s*run ([A-Za-z]+)\s*$') { return $matches[1] }
    return ''
}

function Wait-Status([string]$wanted, [int]$timeoutMs) {
    $deadline = (Get-Date).AddMilliseconds($timeoutMs)
    while ((Get-Date) -lt $deadline) {
        if ((Get-StatusPhase) -eq $wanted) { return $true }
        Start-Sleep -Milliseconds 100
    }
    Write-Log "wait-status wanted=$wanted last='$(Get-Status)'"
    return $false
}

$Expression = '12 - 5'
[void][R23Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 300

$initialStatus = Get-Status
$initialEditor = Get-EditorName
$initialValue = Get-QuickEvalValue
$initialRevision = ''
if ($initialStatus -match '\|\s*rev (\d+)\s*\|') { $initialRevision = $matches[1] }
Write-Log "initial status='$initialStatus' editor='$initialEditor' revision='$initialRevision' quick_eval='$initialValue'"
if (-not $initialEditor) { throw 'the UIA tree has no Script editor node' }
if ($initialStatus -notmatch '\|\s*run Idle\s*$') { throw "the empty document must start Idle, not '$initialStatus'" }
if ($initialValue -ne '') { throw "the quick-eval field must start empty, not '$initialValue'" }

Focus-QuickEval
Type-Characters $Expression
Start-Sleep -Milliseconds 500
$typedValue = Get-QuickEvalValue
Write-Log "typed quick_eval='$typedValue'"
if ($typedValue -ne $Expression) { throw "the quick-eval field did not receive '$Expression': '$typedValue'" }
$afterTypingEditor = Get-EditorName
if ($afterTypingEditor -ne $initialEditor) {
    throw "typing in quick eval changed the editor name: '$afterTypingEditor' (was '$initialEditor')"
}

Send-Key $VK_RETURN
if (-not (Wait-Status 'Finished' 20000)) { throw 'the quick-eval run never reached Finished' }
Start-Sleep -Milliseconds 600
$finalStatus = Get-Status
$finalEditor = Get-EditorName
$finalValue = Get-QuickEvalValue
$finalRevision = ''
if ($finalStatus -match '\|\s*rev (\d+)\s*\|') { $finalRevision = $matches[1] }
Write-Log "after Enter status='$finalStatus' editor='$finalEditor' revision='$finalRevision' quick_eval='$finalValue'"
if ($finalRevision -ne $initialRevision) { throw "the revision moved from '$initialRevision' to '$finalRevision'" }
if ($finalEditor -ne $initialEditor) { throw "the editor buffer changed: '$finalEditor' (was '$initialEditor')" }
if ($finalValue -ne '') { throw "the accepted submission did not clear the field: '$finalValue'" }

# The unmodified harness dumps the live accessibility tree: exact painted log strings, in order,
# with no OCR and no foreground ownership.
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness -Action uia -ProcessId $p.Id -OutDir $Work -Log (Join-Path $Work 'uia-driver.log') | Out-Null
$Uia = Join-Path $Work 'uia-tree.txt'
if (-not (Test-Path -LiteralPath $Uia)) { throw 'the harness uia action produced no tree' }
Copy-Item -LiteralPath $Uia -Destination (Join-Path $OutDir 'quick-eval-uia.txt') -Force
$uiaText = Get-Content -LiteralPath $Uia -Raw
foreach ($required in @(
    '[echo] >> 12 - 5',
    '[result] = 7',
    '[timing] Run finished',
    '[progress] statement 1/1: 12 - 5',
    '[info] Quick eval queued',
    'run Finished'
)) {
    if (-not $uiaText.Contains($required)) { throw "the UIA evidence does not contain '$required'" }
}
$echoLines = ($uiaText -split "`r?`n") | Where-Object { $_ -match 'Text \| \[echo\]' }
if ($echoLines.Count -ne 1) { throw "expected exactly one echo line in the UIA tree, found $($echoLines.Count)" }
$logLines = ($uiaText -split "`r?`n") | Where-Object { $_ -match 'Text \| \[(echo|stdout|result|diagnostic|timing|progress|info)\]' }
Write-Log "uia log lines=$($logLines.Count)"
for ($i = 0; $i -lt $logLines.Count; $i++) {
    Write-Log ("uia log[{0}] {1}" -f $i, $logLines[$i].Trim())
}
$joined = $logLines -join "`n"
$order = @('[echo] >> 12 - 5', '[progress] statement 1/1: 12 - 5', '[result] = 7', '[timing] Run finished')
$last = -1
foreach ($marker in $order) {
    $pos = $joined.IndexOf($marker)
    if ($pos -le $last) { throw "log order is wrong at '$marker'" }
    $last = $pos
}

$Png = Join-Path $OutDir 'quick-eval.png'
$PngBytes = Save-WindowShot $Png 'quick-eval'

# Crop the logs pane plus the status strip for a readable detail image and probe the exact
# painted marker colours. The logs pane header is the bottom-most full-width HEADER_BG
# (#222830) band; the status strip below it is the bottom-most #1B1B1B band.
$logTop = [R23Pixels]::LastBandTop($Png, 0x222830, 8)
if ($logTop -lt 0) { throw 'the logs pane header band was not found in the capture' }
$statusTop = [R23Pixels]::LastBandTop($Png, 0x1B1B1B, 8)
if ($statusTop -le $logTop) { throw "the status strip band was not found below the log header (log_top=$logTop status_top=$statusTop)" }
$full = New-Object System.Drawing.Bitmap($Png)
$detailWidth = 2280
$detailHeight = $full.Height - $logTop
if ($detailHeight -lt 200) { throw "the logs/status crop is only $detailHeight px tall" }
$detailBmp = New-Object System.Drawing.Bitmap($detailWidth, $detailHeight)
$g = [System.Drawing.Graphics]::FromImage($detailBmp)
$destRect = New-Object System.Drawing.Rectangle -ArgumentList 0, 0, $detailWidth, $detailHeight
$sourceRect = New-Object System.Drawing.Rectangle -ArgumentList 0, $logTop, $detailWidth, $detailHeight
$g.DrawImage($full, $destRect, $sourceRect, [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose()
$full.Dispose()
$Detail = Join-Path $OutDir 'quick-eval-logs.png'
$detailBmp.Save($Detail, [System.Drawing.Imaging.ImageFormat]::Png)
$detailBmp.Dispose()
Write-Log "detail crop path=$Detail bytes=$((Get-Item -LiteralPath $Detail).Length) source=0,$logTop,${detailWidth}x${detailHeight}"

$names = @('echo', 'result', 'info', 'timing', 'progress')
[int[]]$colors = @(
    0xc8b6ff, # echo: LOG_ECHO
    0x4ec9b0, # result: ACCENT
    0x8b98a9, # info: DIM
    0xd7ba7d, # timing: WARN
    0x7fb0d8  # progress: LOG_PROGRESS
)
$probeX = 8
$probeY = $logTop + 24
$probeWidth = $detailWidth - 16
$probeHeight = $statusTop - $probeY
$counts = [R23Pixels]::Count($Png, $probeX, $probeY, $probeWidth, $probeHeight, $colors)
$missing = @()
for ($i = 0; $i -lt $names.Count; $i++) {
    Write-Log "palette $($names[$i]) #$('{0:X6}' -f $colors[$i]) pixels=$($counts[$i])"
    if ($counts[$i] -le 0) { $missing += $names[$i] }
}
if ($missing.Count -gt 0) { throw "the log crop is missing these marker colours: $($missing -join ', ')" }

[void][R23Native]::CloseHandle($script:UiThread)
$graceful = $p.CloseMainWindow()
if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; Write-Log 'close forced' }
else { Write-Log "close graceful=$graceful" }
Write-Log "SUMMARY png=$Png bytes=$PngBytes detail=$Detail log_lines=$($logLines.Count) echoes=$($echoLines.Count) expression='$Expression'"
Write-Output "QUICK_EVAL_CAPTURE_OK PNG=$Png BYTES=$PngBytes DETAIL=$Detail LOG_LINES=$($logLines.Count)"
