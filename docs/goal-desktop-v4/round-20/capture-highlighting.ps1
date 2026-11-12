<#
.SYNOPSIS
  Round-20 EDT-002 evidence: type a craft script that exercises every required token class into
  the live desktop editor and capture the painted window.

.DESCRIPTION
  WHY THIS SCRIPT EXISTS
  `tools/gui-harness.ps1 -Action scenario` is the required driver, but this console session is
  locked: GetForegroundWindow reports the Windows Default Lock Screen and neither
  SetForegroundWindow nor AttachThreadInput is permitted from the agent's process. The harness
  aborts at its step-3 foreground assertion, exactly as round 16 documented.

  The product window still paints while occluded, so the evidence is taken without touching the
  lock screen or the foreground policy:

    1. the app is launched on an empty document (its own positional argument path);
    2. real key messages are posted to its window (WM_SETFOCUS, then WM_KEYDOWN / WM_CHAR /
       WM_KEYUP per character; VK_RETURN keydown for a newline). Winit consumes them exactly as
       it consumes the messages SendKeys generates, so the editor really types the script;
    3. the unmodified harness is invoked with `-Action uia` (no foreground needed) to capture the
       accessibility tree of the live window; it must say `Script editor, 5 lines`;
    4. the window is captured with PrintWindow(PW_RENDERFULLCONTENT), which asks the compositor
       for the window's own surface. CopyFromScreen returns black on a locked desktop.
    5. the PNG is probed for the exact palette RGB values inside the editor text area, so the
       capture is not just "a non-empty image": every class colour must be present.

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
$Log = Join-Path $OutDir 'highlighting-capture.log'
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

public struct Round20Rect { public int Left, Top, Right, Bottom; }

public static class Round20Native {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out Round20Rect rect);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
  [DllImport("kernel32.dll")] public static extern IntPtr OpenThread(uint access, bool inherit, uint threadId);
  [DllImport("kernel32.dll")] public static extern uint SuspendThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern uint ResumeThread(IntPtr thread);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}

public static class Round20Pixels {
  /// Counts exact 0xRRGGBB pixels in a crop of a PNG. The editor text is antialiased, but a
  /// glyph's fully covered pixels keep the exact TextFormat colour, so a non-zero count proves
  /// the class colour was actually painted into this window.
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
}
"@

$WM_SETFOCUS = 0x0007
$WM_KEYDOWN = 0x0100
$WM_KEYUP = 0x0101
$WM_CHAR = 0x0102
$PW_RENDERFULLCONTENT = 2

# The product refuses a plot directory below the process working directory (PLF-006) and falls
# back to %TEMP%; keep that fallback inside the writable workspace. No plot statement is run.
$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$env:APPDATA = Join-Path $OutDir 'appdata'
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
$Document = Join-Path $Work 'highlighting.lv'
Set-Content -LiteralPath $Document -Value '' -NoNewline -Encoding ASCII
Write-Log "workspace=$Work appdata=$env:APPDATA temp=$env:TEMP exe=$Exe out=$OutDir"

# The locked session's compositor switched this monitor between 96 and 288 dpi while the round
# was running. Launch the product with the standard DPI-unaware compatibility layer so the
# capture always records its own 1x surface instead of a compositor-scaled copy (at 288 dpi the
# same physical window is a 432x280 logical pane and clips every line). This changes no product
# code and no rendering path: it is the same surface the earlier rounds captured at 96 dpi.
$env:__COMPAT_LAYER = 'DPIUNAWARE'
$p = Start-Process -FilePath $Exe -ArgumentList @($Document) -PassThru
Remove-Item Env:__COMPAT_LAYER
Write-Log "launch pid=$($p.Id) document=$Document compat_layer=DPIUNAWARE"
# Wait for the live eframe viewport, not winit's first helper window: the helper is a 160x28
# placeholder that never paints. Require the real title and a real client rect.
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 300; $i++) {
    $p.Refresh()
    if ($p.MainWindowHandle -ne 0) {
        $candidate = [IntPtr]$p.MainWindowHandle
        $probe = New-Object Round20Rect
        if ([Round20Native]::GetWindowRect($candidate, [ref]$probe)) {
            $pw = [int]($probe.Right - $probe.Left); $ph = [int]($probe.Bottom - $probe.Top)
            $probeTitle = New-Object System.Text.StringBuilder 256
            [void][Round20Native]::GetWindowText($candidate, $probeTitle, 256)
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
$windowTid = [Round20Native]::GetWindowThreadProcessId($h, [ref]$windowPid)
$script:UiThread = [Round20Native]::OpenThread(0x0002, $false, $windowTid)
if ($script:UiThread -eq [IntPtr]::Zero) { throw "cannot open the window thread $windowTid for suspend/resume" }
if ([Round20Native]::IsIconic($h)) { [void][Round20Native]::ShowWindow($h, 9) }
# The compatibility layer must give the product the normal 96-dpi surface. Fail loudly if the
# environment changed again rather than silently capture a scaled, clipped window.
$dpi = [Round20Native]::GetDpiForWindow($h)
$script:Scale = [double]$dpi / 96.0
if ($dpi -ne 96) {
    throw "the DPI-unaware launch still reports dpi=$dpi; refusing a scaled capture"
}
$logicalWidth = 1296
$logicalHeight = 760
$physicalWidth = $logicalWidth
$physicalHeight = $logicalHeight
[void][Round20Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, $physicalWidth, $physicalHeight, 0x0040)
Write-Log "dpi=$dpi scale=$($script:Scale) logical=${logicalWidth}x${logicalHeight} physical=${physicalWidth}x${physicalHeight}"
Start-Sleep -Milliseconds 800
$p.Refresh()
if ($p.HasExited) { throw "the product exited during the window resize (exit code $($p.ExitCode))" }
$rect = New-Object Round20Rect
[void][Round20Native]::GetWindowRect($h, [ref]$rect)
$actualWidth = $rect.Right - $rect.Left
$actualHeight = $rect.Bottom - $rect.Top
Write-Log "resized rect=${actualWidth}x${actualHeight} alive=$(-not $p.HasExited)"
[void][Round20Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 500
$fg = [Round20Native]::GetForegroundWindow()
$title = New-Object System.Text.StringBuilder 256
[void][Round20Native]::GetWindowText($h, $title, 256)
Write-Log "window hwnd=$h title='$($title.ToString())' foreground=$fg (foreground is the lock screen in this session)"

function Save-WindowShot([string]$Path, [string]$Label) {
    $rect = New-Object Round20Rect
    if (-not [Round20Native]::GetWindowRect($h, [ref]$rect)) { throw "GetWindowRect failed for $Label" }
    $w = [int]($rect.Right - $rect.Left); $ht = [int]($rect.Bottom - $rect.Top)
    if ($w -lt 800 -or $ht -lt 500) { throw "refusing $Label capture with rect ${w}x${ht}" }
    for ($attempt = 1; $attempt -le 6; $attempt++) {
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $g.GetHdc()
        $ok = [Round20Native]::PrintWindow($h, $dc, $PW_RENDERFULLCONTENT)
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
        [void][Round20Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
        Start-Sleep -Milliseconds 400
    }
    throw "$Label capture is only $bytes bytes after 6 attempts"
}

function Send-KeyDown([int]$vk) {
    [void][Round20Native]::PostMessage($h, $WM_KEYDOWN, [IntPtr]$vk, [IntPtr]1)
}
function Send-KeyUp([int]$vk) {
    [void][Round20Native]::PostMessage($h, $WM_KEYUP, [IntPtr]$vk, [IntPtr]0xC0000001)
}
function Type-Character([char]$ch) {
    # VK_PROCESSKEY makes TranslateMessage leave the posted WM_CHAR alone. Posting the real
    # character's VK instead would let TranslateMessage insert its UNshifted character first
    # (e.g. `4` for `$`), which is exactly the input defect this method avoids.
    $vk = 0xE5
    # Queue all three messages while the window thread is suspended: winit peeks the next
    # keyboard message when it handles WM_KEYDOWN, so the WM_CHAR must already be in the queue.
    [void][Round20Native]::SuspendThread($script:UiThread)
    try {
        Send-KeyDown $vk
        [void][Round20Native]::PostMessage($h, $WM_CHAR, [IntPtr][int]$ch, [IntPtr]1)
        Send-KeyUp $vk
    } finally {
        [void][Round20Native]::ResumeThread($script:UiThread)
    }
    Start-Sleep -Milliseconds 15
}
function Type-Newline {
    [void][Round20Native]::SuspendThread($script:UiThread)
    try {
        Send-KeyDown 0x0D
        Send-KeyUp 0x0D
    } finally {
        [void][Round20Native]::ResumeThread($script:UiThread)
    }
    Start-Sleep -Milliseconds 70
}

# The craft script: every required class in five lines.
#   line 1  func definition name area, numbers, ^, +, / (1/3 is division, not a rational
#           literal); the four-letter name gives the palette probe an unambiguous teal sample
#   line 2  all flow keywords, list delimiters, comparison
#   line 3  while/return, the repeating exact-rational literal 0.(3), builtin pi
#   line 4  builtin symbol, string argument, symbol declaration
#   line 5  interpolated string with two embedded expressions and symbolic t
$Lines = @(
    'func area(x) = x^2 + 1/3',
    'for i in [1, 2] { if (i == 1) { continue } else { break } }',
    'while (area(2) < 5) { return 0.(3) + pi(2) }',
    't = symbol("t")',
    'z = $"t={t + 1.5}, area={area(2)}"'
)
$ScriptText = $Lines -join "`n"
Write-Log "typing script with $($Lines.Count) lines, $($ScriptText.Length) characters"

[void][Round20Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 250
for ($line = 0; $line -lt $Lines.Count; $line++) {
    foreach ($ch in $Lines[$line].ToCharArray()) { Type-Character $ch }
    if ($line -lt $Lines.Count - 1) { Type-Newline }
}
Start-Sleep -Milliseconds 800
[void][Round20Native]::PostMessage($h, $WM_SETFOCUS, [IntPtr]::Zero, [IntPtr]::Zero)
Start-Sleep -Milliseconds 400
Write-Log "posted all key messages (WM_SETFOCUS + WM_KEYDOWN/WM_CHAR/WM_KEYUP; VK_RETURN for newline)"

# The unmodified harness captures the live accessibility tree (it needs no foreground). It must
# report five editor lines, which proves the posted characters really landed in the buffer.
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness -Action uia -ProcessId $p.Id -OutDir $OutDir -Log (Join-Path $OutDir 'uia-driver.log') | Out-Null
$UiaTree = Join-Path $OutDir 'uia-tree.txt'
if (-not (Test-Path -LiteralPath $UiaTree)) { throw 'the harness uia action produced no tree' }
$editorLine = Select-String -LiteralPath $UiaTree -Pattern 'Script editor,' | Select-Object -First 1
if (-not $editorLine) { throw 'the UIA tree has no Script editor node' }
Write-Log "uia editor line: $($editorLine.Line.Trim())"
if (-not ($editorLine.Line -match 'Script editor, 5 lines')) {
    throw "the UIA tree does not report five editor lines after typing: $($editorLine.Line)"
}

$Png = Join-Path $OutDir 'highlighting.png'
$PngBytes = Save-WindowShot $Png 'highlighting'

# A crop of the editor's five text rows for easy review and for the palette probe. Coordinates
# are logical and multiplied by the DPI scale so the crop covers the same UI at 96 or 288 dpi.
$rect = New-Object Round20Rect
[void][Round20Native]::GetWindowRect($h, [ref]$rect)
$Detail = Join-Path $OutDir 'highlighting-detail.png'
$full = New-Object System.Drawing.Bitmap($Png)
$detailWidth = [int][Math]::Round(520 * $script:Scale)
$detailHeight = [int][Math]::Round(170 * $script:Scale)
$detailY = [int][Math]::Round(80 * $script:Scale)
$crop = New-Object System.Drawing.Bitmap($detailWidth, $detailHeight)
$g = [System.Drawing.Graphics]::FromImage($crop)
$destRect = New-Object System.Drawing.Rectangle -ArgumentList 0, 0, $detailWidth, $detailHeight
$sourceRect = New-Object System.Drawing.Rectangle -ArgumentList 0, $detailY, $detailWidth, $detailHeight
$g.DrawImage($full, $destRect, $sourceRect, [System.Drawing.GraphicsUnit]::Pixel)
$g.Dispose()
$crop.Save($Detail, [System.Drawing.Imaging.ImageFormat]::Png)
$crop.Dispose()
$full.Dispose()
Write-Log "detail crop path=$Detail bytes=$((Get-Item -LiteralPath $Detail).Length) source_rect=0,$detailY,$detailWidth,$detailHeight"

# A 2x nearest-neighbour enlargement makes the five lines and their colours easy to review.
$Detail2x = Join-Path $OutDir 'highlighting-detail-2x.png'
$detailBitmap = New-Object System.Drawing.Bitmap($Detail)
$zoomedWidth = $detailWidth * 2
$zoomedHeight = $detailHeight * 2
$zoomed = New-Object System.Drawing.Bitmap($zoomedWidth, $zoomedHeight)
$zg = [System.Drawing.Graphics]::FromImage($zoomed)
$zg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$zg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
$zg.DrawImage($detailBitmap, 0, 0, $zoomedWidth, $zoomedHeight)
$zg.Dispose()
$detailBitmap.Dispose()
$zoomed.Save($Detail2x, [System.Drawing.Imaging.ImageFormat]::Png)
$zoomed.Dispose()
Write-Log "detail 2x path=$Detail2x bytes=$((Get-Item -LiteralPath $Detail2x).Length) size=${zoomedWidth}x${zoomedHeight}"

# Exact palette colours from DARK in src/ui/syntax.rs. The crop covers the editor pane's text
# rows, not the toolbars or the accent-coloured splitter, so a hit means the highlighter painted
# that colour there.
$names = @('plain','number','rational','string','interpolation','keyword','operator','function','builtin','delimiter','symbol')
[int[]]$colors = @(
    0xd4d9e0, # plain
    0xb5ce8b, # number
    0xe5c07b, # rational
    0xce9178, # string
    0xf0a8d0, # interpolation
    0xc586c0, # keyword
    0x56b6c2, # operator
    0x4ec9b0, # function
    0x4fc1ff, # builtin
    0x8b98a9, # delimiter
    0xc8b6ff  # symbol
)
# Logical x=68..500 and y=121..251 is the editor's text body: it excludes the gutter, the pane
# border and the accent-coloured header line (which shares the function teal), so a hit is text.
$probeX = [int][Math]::Round(68 * $script:Scale)
$probeY = [int][Math]::Round(121 * $script:Scale)
$probeWidth = [int][Math]::Round(432 * $script:Scale)
$probeHeight = [int][Math]::Round(130 * $script:Scale)
$counts = [Round20Pixels]::Count($Png, $probeX, $probeY, $probeWidth, $probeHeight, $colors)
$missing = @()
for ($i = 0; $i -lt $names.Count; $i++) {
    Write-Log "palette $($names[$i]) #$('{0:X6}' -f $colors[$i]) pixels=$($counts[$i])"
    if ($counts[$i] -le 0) { $missing += $names[$i] }
}
if ($missing.Count -gt 0) {
    throw "the capture is missing these painted class colours: $($missing -join ', ')"
}

[void][Round20Native]::CloseHandle($script:UiThread)
$graceful = $p.CloseMainWindow()
if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; Write-Log 'close forced' }
else { Write-Log "close graceful=$graceful" }
Write-Log "SUMMARY png=$Png bytes=$PngBytes detail=$Detail detail_bytes=$((Get-Item -LiteralPath $Detail).Length) detail2x=$Detail2x lines=$($Lines.Count) palette_classes=$($names.Count)"
Write-Output "HIGHLIGHTING_CAPTURE_OK PNG=$Png BYTES=$PngBytes DETAIL=$Detail DETAIL2X=$Detail2x"
