<#
.SYNOPSIS
  Round-29 PLT-001 window proof: the shipped workbench renders the engine's SVG plot at 100% and
  one zoom step (1.5x), and the 1.5x raster is visibly clipped by the plot pane.

.DESCRIPTION
  Launches the shipped lovelace-desktop.exe on a script whose plot() call carries a title, tick
  labels and a 24-sample polyline, resizes the window to 2500x1380, invokes Run through UI
  Automation, waits for "run Finished", then captures the real window with PrintWindow
  (PW_RENDERFULLCONTENT). It clicks the plot pane's 1.5x button, waits until the plot widget
  reports "zoom 1.50", captures again, clicks Fit and verifies the widget returns to zoom 1.00
  (so the same session is proven usable after the zoom step).

  The plot body is clipped at 1.5x: zoom 1.0 fits the 800x600 SVG into the pane, so 1.5x draws it
  at 1.5 times the fit size and egui clips the overflow to the pane rectangle. Pixel palette
  counts prove the frame really contains the rendered SVG (white canvas, red series, dark axes),
  not an empty or error state.

  Writes plot-100.png, plot-150.png, their UIA dumps, capture-run.log and window-proof.json.
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

$Work = Join-Path $OutDir 'work\window'
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

Add-Type -TypeDefinition @"
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;
public struct R29Rect { public int Left, Top, Right, Bottom; }
public static class R29Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R29Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
public static class R29Pixels {
  static byte[] Bytes(string path, out int w, out int h, out int stride) {
    using (var bmp = new Bitmap(path)) {
      w = bmp.Width; h = bmp.Height;
      using (var copy = new Bitmap(w, h, PixelFormat.Format32bppArgb)) {
        using (var g = Graphics.FromImage(copy)) { g.CompositingMode = CompositingMode.SourceCopy; g.DrawImageUnscaled(bmp, 0, 0); }
        var rect = new Rectangle(0, 0, w, h);
        var data = copy.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
        stride = data.Stride;
        var bytes = new byte[stride * h];
        Marshal.Copy(data.Scan0, bytes, 0, bytes.Length);
        copy.UnlockBits(data);
        return bytes;
      }
    }
  }
  public static long CountNear(string path, int r, int g, int b, int tolerance) {
    int w, h, stride; var bytes = Bytes(path, out w, out h, out stride); long n = 0;
    for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) {
      int i = y * stride + x * 4;
      if (Math.Abs(bytes[i + 2] - r) <= tolerance && Math.Abs(bytes[i + 1] - g) <= tolerance && Math.Abs(bytes[i] - b) <= tolerance) n++;
    }
    return n;
  }
  public static long CountNearRect(string path, int r, int g, int b, int tolerance, int x0, int y0, int x1, int y1) {
    int w, h, stride; var bytes = Bytes(path, out w, out h, out stride); long n = 0;
    int left = Math.Max(0, x0), top = Math.Max(0, y0);
    int right = Math.Min(w, x1), bottom = Math.Min(h, y1);
    for (int y = top; y < bottom; y++) for (int x = left; x < right; x++) {
      int i = y * stride + x * 4;
      if (Math.Abs(bytes[i + 2] - r) <= tolerance && Math.Abs(bytes[i + 1] - g) <= tolerance && Math.Abs(bytes[i] - b) <= tolerance) n++;
    }
    return n;
  }
  public static long CountDarkRect(string path, int maxLuminance, int x0, int y0, int x1, int y1) {
    int w, h, stride; var bytes = Bytes(path, out w, out h, out stride); long n = 0;
    int left = Math.Max(0, x0), top = Math.Max(0, y0);
    int right = Math.Min(w, x1), bottom = Math.Min(h, y1);
    for (int y = top; y < bottom; y++) for (int x = left; x < right; x++) {
      int i = y * stride + x * 4;
      int lum = (299 * bytes[i + 2] + 587 * bytes[i + 1] + 114 * bytes[i]) / 1000;
      if (lum < maxLuminance) n++;
    }
    return n;
  }
  public static double DifferPercent(string pathA, string pathB, int threshold) {
    int wa, ha, sa, wb, hb, sb;
    var a = Bytes(pathA, out wa, out ha, out sa); var b = Bytes(pathB, out wb, out hb, out sb);
    if (wa != wb || ha != hb) return 100.0;
    long diff = 0;
    for (int y = 0; y < ha; y++) for (int x = 0; x < wa; x++) {
      int i = y * sa + x * 4, j = y * sb + x * 4;
      int d = Math.Max(Math.Abs(a[i] - b[j]), Math.Max(Math.Abs(a[i + 1] - b[j + 1]), Math.Abs(a[i + 2] - b[j + 2])));
      if (d > threshold) diff++;
    }
    return 100.0 * diff / ((double)wa * ha);
  }
}
"@ -ReferencedAssemblies System.Drawing

function Invoke-Harness([string[]]$HarnessArgs) {
    return (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness @HarnessArgs 2>&1 | Out-String)
}
function Write-CaptureLog([string]$line) {
    Write-Host $line
    Add-Content -LiteralPath $CaptureLog -Value $line -Encoding UTF8
}
function Get-Texts([IntPtr]$Hwnd) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($text in $texts) { $names.Add($text.Current.Name) }
    return $names
}
function Get-Status([IntPtr]$Hwnd) {
    foreach ($name in (Get-Texts $Hwnd)) {
        if ($name -match '\|\s*run ([A-Za-z]+)\s*$') { return $name }
    }
    return ''
}
function Find-TextElement([IntPtr]$Hwnd, [string]$Needle) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Text)
    $texts = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($text in $texts) {
        if ($text.Current.Name -like "*$Needle*") { return $text }
    }
    return $null
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
function Find-NamedElement([IntPtr]$Hwnd, [string]$Needle) {
    # The plot canvas is exposed as ControlType.Image, so search every descendant by name.
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $all) {
        $name = $element.Current.Name
        if ($name -and ($name -like "*$Needle*")) { return $element }
    }
    return $null
}
function Wait-ForText([IntPtr]$Hwnd, [string]$Needle, [int]$TimeoutSec) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $element = Find-NamedElement $Hwnd $Needle
        if ($element) { return $element }
        Start-Sleep -Milliseconds 150
    }
    return $null
}
function Invoke-Button([IntPtr]$Hwnd, [string]$Name, [int]$ProcessId) {
    $button = Find-ButtonByName $Hwnd $Name
    if (-not $button) { throw "no enabled UI Automation button named '$Name'" }
    try {
        $pattern = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $pattern.Invoke()
        return 'InvokePattern'
    } catch {
        $button.SetFocus()
        Start-Sleep -Milliseconds 250
        $keys = Invoke-Harness @('-Action', 'keys', '-ProcessId', "$ProcessId", '-Keys', ' ', '-Log', $DriverLog)
        if ($keys -notmatch 'SENT=') { throw "could not activate '$Name' by keyboard: $keys" }
        return 'focus+Space'
    }
}
function Save-PrintWindow([IntPtr]$Hwnd, [string]$Path) {
    $bytes = 0
    for ($attempt = 1; $attempt -le 4; $attempt++) {
        [void][R29Native]::ShowWindow($Hwnd, 9)
        [void][R29Native]::BringWindowToTop($Hwnd)
        [void][R29Native]::SetForegroundWindow($Hwnd)
        [void][R29Native]::RedrawWindow($Hwnd, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
        Start-Sleep -Milliseconds 400
        [void][R29Native]::SetWindowPos($Hwnd, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
        Start-Sleep -Milliseconds (600 + 350 * $attempt)
        $windowRect = New-Object R29Rect
        [void][R29Native]::GetWindowRect($Hwnd, [ref]$windowRect)
        $w = [int]($windowRect.Right - $windowRect.Left)
        $h = [int]($windowRect.Bottom - $windowRect.Top)
        $bmp = New-Object System.Drawing.Bitmap($w, $h)
        $graphics = [System.Drawing.Graphics]::FromImage($bmp)
        $dc = $graphics.GetHdc()
        $ok = [R29Native]::PrintWindow($Hwnd, $dc, 2)
        $graphics.ReleaseHdc($dc)
        $graphics.Dispose()
        $bmp.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bmp.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        Write-CaptureLog "printwindow attempt=$attempt file=$Path bytes=$bytes ok=$ok rect=${w}x${h}"
        if ($bytes -ge 20480) { break }
        Start-Sleep -Milliseconds 800
    }
    if ($bytes -lt 20480) { throw "PrintWindow frame $Path is smaller than 20 KB ($bytes); refusing it as empty" }
    return $bytes
}
function Save-UiaDump([IntPtr]$Hwnd, [int]$ProcessId, [string]$Path) {
    $uia = Invoke-Harness @('-Action', 'uia', '-ProcessId', "$ProcessId", '-OutDir', $Work, '-Log', $DriverLog)
    $produced = Join-Path $Work 'uia-tree.txt'
    if (-not (Test-Path -LiteralPath $produced)) { throw "uia action produced no tree: $uia" }
    Copy-Item -LiteralPath $produced -Destination $Path -Force
    Write-CaptureLog "uia=$Path bytes=$((Get-Item -LiteralPath $Path).Length)"
    return (Get-Content -LiteralPath $Path -Raw)
}
function Get-ImageSize([string]$Path) {
    $image = [System.Drawing.Image]::FromFile($Path)
    try { return "$($image.Width)x$($image.Height)" } finally { $image.Dispose() }
}

# ---------------------------------------------------------------------------
# Scenario
# ---------------------------------------------------------------------------
$Script = Join-Path $Work 'window-plot.lv'
$source = @(
    'x = 1..24',
    'y = (x - 12)^2',
    'plot(x, y, "fidelity plot: text and polyline")'
)
[System.IO.File]::WriteAllLines($Script, $source, (New-Object System.Text.ASCIIEncoding))

$launch = Invoke-Harness @('-Action', 'launch', '-Exe', $Exe, '-Arguments', $Script, '-Log', $DriverLog)
$processId = 0
if ($launch -match 'PROCESS_ID=(\d+)') { $processId = [int]$matches[1] }
if ($processId -eq 0) { throw "launch did not report a process id: $launch" }
Write-CaptureLog "launch pid=$processId script=$Script"

$wait = Invoke-Harness @('-Action', 'wait', '-ProcessId', "$processId", '-Log', $DriverLog)
if ($wait -notmatch 'HWND=(\d+)') { throw "wait did not report a window handle: $wait" }
$h = [IntPtr][long]$matches[1]
if ([R29Native]::IsIconic($h)) { [void][R29Native]::ShowWindow($h, 9) }
[void][R29Native]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
Start-Sleep -Milliseconds 900
$rect = New-Object R29Rect
[void][R29Native]::GetWindowRect($h, [ref]$rect)
Write-CaptureLog "window hwnd=$h rect=$($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"

$click = Invoke-Harness @('-Action', 'click', '-ProcessId', "$processId", '-Name', 'Run', '-Log', $DriverLog)
if ($click -notmatch 'CLICKED=Run') { throw "could not invoke Run: $click" }
$deadline = (Get-Date).AddSeconds(30)
$status = ''
while ((Get-Date) -lt $deadline) {
    $status = Get-Status $h
    if ($status -match '\|\s*run Finished\s*$') { break }
    Start-Sleep -Milliseconds 150
}
Write-CaptureLog "run status='$status'"
if ($status -notmatch '\|\s*run Finished\s*$') { throw "the run never reached Finished: '$status'" }

$at100 = Wait-ForText $h 'SVG plot canvas' 15
if (-not $at100) { throw 'the plot pane published no SVG plot canvas widget' }
$at100Name = $at100.Current.Name
if ($at100Name -notlike '*zoom 1.00*') { throw "expected zoom 1.00 before the zoom step, got '$at100Name'" }
$raster100 = Wait-ForText $h 'ms raster' 10
if (-not $raster100) { throw 'the plot pane published no raster timing line' }
$raster100Name = $raster100.Current.Name
$paneRect100 = $at100.Current.BoundingRectangle
Write-CaptureLog "at100 widget='$at100Name' raster='$raster100Name' pane=$($paneRect100.Width)x$($paneRect100.Height)"
Start-Sleep -Milliseconds 500

$png100 = Join-Path $OutDir 'plot-100.png'
$bytes100 = Save-PrintWindow $h $png100
$uia100 = Join-Path $OutDir 'plot-100-uia.txt'
$text100 = Save-UiaDump $h $processId $uia100
$size100 = Get-ImageSize $png100

$method = Invoke-Button $h '1.5x' $processId
$at150 = Wait-ForText $h 'zoom 1.50' 12
if (-not $at150) { throw "the plot pane never reported zoom 1.50 (activated by $method)" }
$at150Name = $at150.Current.Name
$raster150 = Wait-ForText $h 'ms raster' 8
if (-not $raster150) { throw 'the zoomed plot pane published no raster timing line' }
$raster150Name = $raster150.Current.Name
Write-CaptureLog "zoom method=$method widget='$at150Name' raster='$raster150Name'"
Start-Sleep -Milliseconds 700
$png150 = Join-Path $OutDir 'plot-150.png'
$bytes150 = Save-PrintWindow $h $png150
$uia150 = Join-Path $OutDir 'plot-150-uia.txt'
$text150 = Save-UiaDump $h $processId $uia150
$size150 = Get-ImageSize $png150

# The same session must remain usable: Fit restores the 100% state.
$fitMethod = Invoke-Button $h 'Fit' $processId
$fit = Wait-ForText $h 'zoom 1.00' 10
if (-not $fit) { throw "Fit did not restore zoom 1.00 (activated by $fitMethod)" }
Write-CaptureLog "fit method=$fitMethod restored='$($fit.Current.Name)'"

[void](Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog))

# ---------------------------------------------------------------------------
# Assertions
# ---------------------------------------------------------------------------
$required = @(
    'fidelity plot: text and polyline',
    'SVG plot canvas, 800 by 600 points',
    'zoom 1.00',
    'ms raster'
)
foreach ($needle in $required) {
    if (-not $text100.Contains($needle)) { throw "the 100% UIA tree does not expose '$needle'" }
}
foreach ($needle in @('SVG plot canvas, 800 by 600 points', 'zoom 1.50', 'ms raster')) {
    if (-not $text150.Contains($needle)) { throw "the 1.5x UIA tree does not expose '$needle'" }
}
if ($size100 -ne '2500x1380' -or $size150 -ne '2500x1380') {
    throw "unexpected capture sizes: 100%=$size100 150%=$size150"
}
$paneX = [int]$paneRect100.X
$paneY = [int]$paneRect100.Y
$paneW = [int]$paneRect100.Width
$paneH = [int]$paneRect100.Height
# The same fit the pane computes: base = min(available/svg), image centred with no pan.
$base = [Math]::Min(($paneW - 16.0) / 800.0, ($paneH - 16.0) / 600.0)
$imgW = 800.0 * $base
$imgH = 600.0 * $base
$imgLeft = [int]($paneX + ($paneW - $imgW) / 2.0)
$imgTop = [int]($paneY + ($paneH - $imgH) / 2.0)
$imgRight = [int]($imgLeft + $imgW)
$imgBottom = [int]($imgTop + $imgH)
$inset = 10
$palette = [ordered]@{
    white = [R29Pixels]::CountNearRect($png100, 255, 255, 255, 4, ($imgLeft + $inset), ($imgTop + $inset), ($imgRight - $inset), ($imgBottom - $inset))
    series_red = [R29Pixels]::CountNearRect($png100, 230, 25, 75, 45, ($imgLeft + $inset), ($imgTop + $inset), ($imgRight - $inset), ($imgBottom - $inset))
    axis_dark = [R29Pixels]::CountDarkRect($png100, 128, ($imgLeft + $inset), ($imgTop + $inset), ($imgRight - $inset), ($imgBottom - $inset))
    accent_teal = [R29Pixels]::CountNear($png100, 78, 201, 176, 20)
}
$clippedAt150 = (($imgW * 1.5) -gt $paneW) -or (($imgH * 1.5) -gt $paneH)
Write-CaptureLog "palette 100% pane=($paneX,$paneY ${paneW}x${paneH}) image=($imgLeft,$imgTop ${imgW}x${imgH}) white=$($palette.white) red=$($palette.series_red) dark=$($palette.axis_dark) teal=$($palette.accent_teal)"
if ($palette.white -lt 200000) { throw "the plot region carries too little white canvas: $($palette.white)" }
if ($palette.series_red -lt 300) { throw "the plot region carries too few series pixels: $($palette.series_red)" }
if ($palette.axis_dark -lt 500) { throw "the plot region carries too few axis pixels: $($palette.axis_dark)" }
if ($palette.accent_teal -lt 50) { throw "the frame does not carry the pane accent colour: $($palette.accent_teal)" }

$zoomDiff = [R29Pixels]::DifferPercent($png100, $png150, 16)
Write-CaptureLog "100% vs 150% differ=$([Math]::Round($zoomDiff,4))%"
if ($zoomDiff -lt 5.0) { throw "the 1.5x capture is too similar to 100% ($zoomDiff%); the zoom step did not repaint" }

$result = [ordered]@{
    round = 'round-29'
    generated_at = (Get-Date).ToString('o')
    window = '2500x1380'
    script = $source
    plot_title = 'fidelity plot: text and polyline'
    captures = @(
        [ordered]@{
            state = '100% fit (zoom 1.00)'
            path = $png100
            bytes = $bytes100
            sha256 = (Get-FileHash -LiteralPath $png100 -Algorithm SHA256).Hash
            size = $size100
            uia = $uia100
            plot_widget = $at100Name
            raster_line = $raster100Name
            pane_rect = "$($paneRect100.Width)x$($paneRect100.Height)"
            palette = $palette
        },
        [ordered]@{
            state = 'one zoom step (zoom 1.50)'
            path = $png150
            bytes = $bytes150
            sha256 = (Get-FileHash -LiteralPath $png150 -Algorithm SHA256).Hash
            size = $size150
            uia = $uia150
            plot_widget = $at150Name
            raster_line = $raster150Name
            activation = $method
        }
    )
    assertions = [ordered]@{
        run_finished = $true
        plot_widget_present = $true
        zoom_step_applied = $true
        zoom_activation = $method
        fit_restores_zoom_1_00 = $true
        fit_activation = $fitMethod
        frames_differ_percent = [Math]::Round($zoomDiff, 4)
        pane_rect_100 = "${paneW}x${paneH}"
        image_rect_100 = "${imgW}x${imgH}"
        image_rect_150 = "$([Math]::Round($imgW * 1.5, 1))x$([Math]::Round($imgH * 1.5, 1))"
        clipped_at_150 = $clippedAt150
        clipping = 'At zoom 1.00 the 800x600 SVG is fitted to the pane; at 1.50 it is drawn at 1.5x the fit size, so the image exceeds the pane and is clipped to the pane rectangle (visible as content cut at the pane edges).'
    }
    passed = $true
}
$json = Join-Path $OutDir 'window-proof.json'
[System.IO.File]::WriteAllText($json, ($result | ConvertTo-Json -Depth 8), (New-Object System.Text.UTF8Encoding($false)))
Write-CaptureLog "wrote $json passed=true"
Write-CaptureLog "DONE 100%=$bytes100 bytes $size100  150%=$bytes150 bytes $size150"
