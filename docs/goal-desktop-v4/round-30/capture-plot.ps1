<#
.SYNOPSIS
  Round-30 PLT-004 / PLT-006 evidence: plot zoom/pan/Fit in the shipped window, and SVG/PNG
  export through the native save dialog.

.DESCRIPTION
  The console session is unlocked, so this script uses the normal harness for launch, UIA
  invocation, UIA dumps, pointer input and close:
    1. an independent engine capture is written by lovelace-run for the same script;
    2. the shipped lovelace-desktop.exe is launched on the script, resized to 2500x1380,
       Run is invoked through UI Automation and the status strip is polled to "run Finished";
    3. Zoom in x2, then Fit, then one zoom step are performed through the window's buttons
       invoked by UI Automation; a PrintWindow capture and a UIA dump are saved after Fit and
       after the zoom step;
    4. Export SVG... opens the Windows common Save dialog, already in the capture's directory
       and pre-filled with the title-derived name; this script presses its Save button with a
       posted BM_CLICK (the lock screen blocks injected keyboard input). The same happens for
       Export PNG... after the in-window options step (2x, Transparent);
    5. the exported SVG is compared byte for byte with the engine's own PlotCapture.svg file
       from this run and with the independent lovelace-run capture; the PNG's IHDR dimensions,
       decodability and SHA-256 are recorded;
    6. the JSON summary and capture-run.log are written next to the captures.

  APPDATA is redirected to a writable directory under this round's own folder, and the process
  current directory is the round's work folder, so the resolved plot directory
  (<OutDir>\appdata\Lovelace\plots) is writable and is not under the working directory. Without
  that redirect the harness's read-only %APPDATA%\Lovelace\plots makes every plot statement fail
  with "Access is denied"; this script says so in its log.

  The console session was locked during the capture. UIA Invoke and PrintWindow
  (PW_RENDERFULLCONTENT) still reach the app window; injected pointer input does not, so the
  window proof does not attempt a mouse drag. Pan is proven by the headless egui pointer test
  `ui::plot::tests::the_pane_wheel_and_drag_handlers_use_the_anchored_transform` and by the
  transform tests in the same module.

  Windows PowerShell 5.1 compatible. ASCII-only source.
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
$Runner = Join-Path $Repo 'rust\target-desktop\debug\lovelace-run.exe'
if (-not (Test-Path -LiteralPath $Runner)) { throw "lovelace-run not found: $Runner" }
$Harness = Join-Path $Repo 'tools\gui-harness.ps1'
if (-not (Test-Path -LiteralPath $Harness)) { throw "harness not found: $Harness" }

$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$AppData = Join-Path $OutDir 'appdata'
if (Test-Path -LiteralPath $AppData) { Remove-Item -LiteralPath $AppData -Recurse -Force }
New-Item -ItemType Directory -Force -Path $AppData | Out-Null
$DriverLog = Join-Path $Work 'driver.log'
$CaptureLog = Join-Path $OutDir 'capture-run.log'
if (Test-Path -LiteralPath $CaptureLog) { Remove-Item -LiteralPath $CaptureLog -Force }

$env:APPDATA = $AppData
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
# The child app must start in $Work, not in the repository root: the resolved plot directory
# (<OutDir>\appdata\Lovelace\plots) is only accepted while it is not below the working directory.
# Start-Process defaults its working directory to this host's location, so set both the provider
# location and the process current directory before the harness launches the app.
Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

function Write-CaptureLog([string]$line) {
    Write-Host $line
    Add-Content -LiteralPath $CaptureLog -Value $line -Encoding UTF8
}

function Invoke-Harness([string[]]$HarnessArgs) {
    return (& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $Harness @HarnessArgs 2>&1 | Out-String)
}

# A small JSON writer for the summary. Windows PowerShell 5.1's ConvertTo-Json threw an
# OutOfMemoryException on this object graph in the capture process (it serialises the same
# fields fine in a standalone run), so the script owns the encoding instead of depending on it.
function ConvertTo-JsonString([string]$text) {
    $builder = New-Object System.Text.StringBuilder
    [void]$builder.Append('"')
    foreach ($ch in $text.ToCharArray()) {
        $code = [int]$ch
        if ($ch -eq '"') { [void]$builder.Append('\"') }
        elseif ($ch -eq '\') { [void]$builder.Append('\\') }
        elseif ($code -eq 10) { [void]$builder.Append('\n') }
        elseif ($code -eq 13) { [void]$builder.Append('\r') }
        elseif ($code -eq 9) { [void]$builder.Append('\t') }
        elseif ($code -lt 32) { [void]$builder.Append(('\u{0:x4}' -f $code)) }
        else { [void]$builder.Append($ch) }
    }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function Write-JsonValue($value, [int]$indent, [System.Text.StringBuilder]$builder) {
    if ($null -eq $value) { [void]$builder.Append('null'); return }
    if ($value -is [string]) { [void]$builder.Append((ConvertTo-JsonString $value)); return }
    if ($value -is [bool]) {
        if ($value) { [void]$builder.Append('true') } else { [void]$builder.Append('false') }
        return
    }
    if ($value -is [int] -or $value -is [long] -or $value -is [double] -or $value -is [decimal]) {
        [void]$builder.Append($value.ToString([System.Globalization.CultureInfo]::InvariantCulture))
        return
    }
    if ($value -is [System.Collections.IDictionary]) {
        $keys = @($value.Keys)
        $pad = ' ' * ($indent + 2)
        [void]$builder.Append("{`n")
        for ($index = 0; $index -lt $keys.Count; $index++) {
            [void]$builder.Append($pad)
            [void]$builder.Append((ConvertTo-JsonString ([string]$keys[$index])))
            [void]$builder.Append(': ')
            Write-JsonValue $value[$keys[$index]] ($indent + 2) $builder
            if ($index -lt $keys.Count - 1) { [void]$builder.Append(',') }
            [void]$builder.Append("`n")
        }
        [void]$builder.Append((' ' * $indent))
        [void]$builder.Append('}')
        return
    }
    if ($value -is [System.Collections.IEnumerable]) {
        $items = @($value)
        $pad = ' ' * ($indent + 2)
        [void]$builder.Append("[`n")
        for ($index = 0; $index -lt $items.Count; $index++) {
            [void]$builder.Append($pad)
            Write-JsonValue $items[$index] ($indent + 2) $builder
            if ($index -lt $items.Count - 1) { [void]$builder.Append(',') }
            [void]$builder.Append("`n")
        }
        [void]$builder.Append((' ' * $indent))
        [void]$builder.Append(']')
        return
    }
    [void]$builder.Append((ConvertTo-JsonString ([string]$value)))
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct R30Rect { public int Left, Top, Right, Bottom; }
public static class R30Native {
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out R30Rect rect);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool RedrawWindow(IntPtr hWnd, IntPtr rect, IntPtr region, uint flags);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr dialog, int id);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint message, IntPtr wparam, IntPtr lparam);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wparam, IntPtr lparam);
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

function Save-PrintWindow([IntPtr]$Hwnd, [string]$Path) {
    $bytes = 0
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        [void][R30Native]::ShowWindow($Hwnd, 9)
        [void][R30Native]::BringWindowToTop($Hwnd)
        [void][R30Native]::SetForegroundWindow($Hwnd)
        [void][R30Native]::RedrawWindow($Hwnd, [IntPtr]::Zero, [IntPtr]::Zero, 0x0181)
        [void][R30Native]::SetWindowPos($Hwnd, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
        Start-Sleep -Milliseconds (600 + 300 * $attempt)
        $rect = New-Object R30Rect
        [void][R30Native]::GetWindowRect($Hwnd, [ref]$rect)
        $w = [int]($rect.Right - $rect.Left)
        $h = [int]($rect.Bottom - $rect.Top)
        $bitmap = New-Object System.Drawing.Bitmap($w, $h)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        $dc = $graphics.GetHdc()
        $ok = [R30Native]::PrintWindow($Hwnd, $dc, 2)
        $graphics.ReleaseHdc($dc)
        $graphics.Dispose()
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose()
        $bytes = (Get-Item -LiteralPath $Path).Length
        Write-CaptureLog "printwindow attempt=$attempt file=$Path bytes=$bytes ok=$ok rect=${w}x${h}"
        if ($bytes -ge 20480) { return $bytes }
        Start-Sleep -Milliseconds 800
    }
    throw "PrintWindow frame $Path is smaller than 20 KB ($bytes)"
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

function Get-PlotImage([IntPtr]$Hwnd) {
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($Hwnd)
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Image)
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Get-PlotImageName([IntPtr]$Hwnd) {
    $image = Get-PlotImage $Hwnd
    if (-not $image) { return '' }
    return $image.Current.Name
}

function Find-Dialog([int]$ProcessId, [int]$TimeoutSec) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $root = [System.Windows.Automation.AutomationElement]::RootElement
        $condition = New-Object System.Windows.Automation.AndCondition(
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $ProcessId)),
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ClassNameProperty, '#32770')))
        $windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $condition)
        foreach ($window in $windows) {
            return $window
        }
        Start-Sleep -Milliseconds 150
    }
    throw "the native save dialog did not appear within $TimeoutSec s"
}

function Save-DialogDefault([System.Windows.Automation.AutomationElement]$Dialog, [string]$Label) {
    # The dialog is pre-filled with the plot's suggested file name and opens in the capture's own
    # (writable) directory, so the default Save button is exactly the user gesture. Pressing it
    # with a posted BM_CLICK works without a foreground window, unlike SendKeys. The native
    # common dialog's Save button is the child control with id 1 (IDOK).
    $handle = [IntPtr]$Dialog.Current.NativeWindowHandle
    if ($handle -eq [IntPtr]::Zero) { throw "$Label dialog has no native window handle" }
    $save = [R30Native]::GetDlgItem($handle, 1)
    if ($save -eq [IntPtr]::Zero) { throw "$Label dialog has no Save button (ID 1)" }
    [void][R30Native]::PostMessage($save, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero) # BM_CLICK
    Write-CaptureLog "$Label dialog '$($Dialog.Current.Name)' hwnd=$handle pressed Save (id 1)"
}

function Wait-NewFile([string]$Directory, [string]$Extension, [string[]]$Before, [int]$TimeoutSec) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $candidate = Get-ChildItem -LiteralPath $Directory -File -Filter "*.$Extension" -ErrorAction SilentlyContinue |
            Where-Object { $Before -notcontains $_.Name } |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($candidate -and $candidate.Length -gt 0) { return $candidate }
        Start-Sleep -Milliseconds 200
    }
    throw "no new .$Extension file appeared in $Directory within $TimeoutSec s"
}

function Wait-File([string]$Path, [int]$TimeoutSec) {
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if (Test-Path -LiteralPath $Path) {
            $item = Get-Item -LiteralPath $Path
            if ($item.Length -gt 0) { return $item }
        }
        Start-Sleep -Milliseconds 150
    }
    throw "file did not appear: $Path"
}

function Click-Button([int]$ProcessId, [string]$Name) {
    # UIA Invoke by accessible name: the path the round-29 window proof used, and the one that
    # still works when the console session is locked (no injected pointer input reaches the app).
    $click = Invoke-Harness @('-Action', 'click', '-ProcessId', "$ProcessId", '-Name', $Name, '-Log', $DriverLog)
    if ($click -notmatch 'CLICKED=') { throw "could not invoke '$Name': $click" }
    Write-CaptureLog "clicked '$Name'"
    Start-Sleep -Milliseconds 500
}

function Start-Invoke([int]$ProcessId, [string]$Name) {
    # The export buttons open the modal common dialog from the frame that handles the click. Start
    # the UIA Invoke in its own process: if that call waits for the dialog, the caller can still
    # reach the dialog and fill it, and the click process finishes when the dialog closes.
    $arguments = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $Harness,
        '-Action', 'click', '-ProcessId', "$ProcessId", '-Name', "`"$Name`"", '-Log', $DriverLog
    )
    $process = Start-Process -FilePath 'powershell.exe' -ArgumentList $arguments -PassThru -WindowStyle Hidden
    Write-CaptureLog "started UIA invoke of '$Name' in pid=$($process.Id)"
    return $process
}

# --- 1. The plot script and an independent engine capture --------------------------------
$Script = Join-Path $Work 'plot.lv'
Set-Content -LiteralPath $Script -Encoding ASCII -Value @(
    'x = 1..24',
    'y = (x - 12)^2',
    'plot(x, y, "round 30 plot: zoom pan export")'
)
$EngineCaptureDir = Join-Path $Work 'engine-capture'
New-Item -ItemType Directory -Force -Path $EngineCaptureDir | Out-Null
& $Runner --file $Script --plot-dir $EngineCaptureDir --plot-file engine-capture.svg | Out-Null
$runnerExit = $LASTEXITCODE
$EngineCapture = Join-Path $EngineCaptureDir 'engine-capture.svg'
if ($runnerExit -ne 0 -or -not (Test-Path -LiteralPath $EngineCapture)) {
    throw "lovelace-run did not write the engine capture (exit $runnerExit)"
}
Write-CaptureLog "engine capture $EngineCapture bytes=$((Get-Item -LiteralPath $EngineCapture).Length) sha256=$((Get-FileHash -LiteralPath $EngineCapture -Algorithm SHA256).Hash)"

# --- 2. Launch, run, wait for Finished ----------------------------------------------------
$launch = Invoke-Harness @('-Action', 'launch', '-Exe', $Exe, '-Arguments', $Script, '-Log', $DriverLog)
if ($launch -notmatch 'PROCESS_ID=(\d+)') { throw "launch did not report a process id: $launch" }
$processId = [int]$matches[1]
Write-CaptureLog "launch pid=$processId cwd=$([System.IO.Directory]::GetCurrentDirectory()) appdata=$env:APPDATA"
$wait = Invoke-Harness @('-Action', 'wait', '-ProcessId', "$processId", '-Log', $DriverLog)
if ($wait -notmatch 'HWND=(\d+)') { throw "wait did not report a window handle: $wait" }
$hwnd = [IntPtr][long]$matches[1]
Write-CaptureLog "window hwnd=$hwnd"
if ([R30Native]::IsIconic($hwnd)) { [void][R30Native]::ShowWindow($hwnd, 9) }
[void][R30Native]::SetWindowPos($hwnd, [IntPtr]::Zero, 0, 0, 2500, 1380, 0x0040)
Start-Sleep -Milliseconds 1200

$click = Invoke-Harness @('-Action', 'click', '-ProcessId', "$processId", '-Name', 'Run', '-Log', $DriverLog)
if ($click -notmatch 'CLICKED=Run') { throw "could not invoke Run: $click" }
$deadline = (Get-Date).AddSeconds(30)
$status = ''
while ((Get-Date) -lt $deadline) {
    $status = Get-Status $hwnd
    if ($status -match '\|\s*run Finished\s*$') { break }
    Start-Sleep -Milliseconds 150
}
Write-CaptureLog "status='$status'"
if ($status -notmatch '\|\s*run Finished\s*$') { throw "the run never reached Finished: '$status'" }
Start-Sleep -Milliseconds 900

$fitName = Get-PlotImageName $hwnd
Write-CaptureLog "plot image after run: '$fitName'"
if ($fitName -notmatch 'SVG plot canvas') { throw "the plot pane has no SVG canvas: '$fitName'" }

# --- 3. Zoom in twice; Fit and capture; then one zoom step and capture ---------------------
Click-Button $processId 'Zoom in'
$firstZoomName = Get-PlotImageName $hwnd
Write-CaptureLog "plot image after one zoom click: '$firstZoomName'"
if ($firstZoomName -match 'zoom 1\.00') { throw "the first Zoom in click did not reach the plot: '$firstZoomName'" }
Click-Button $processId 'Zoom in'
Start-Sleep -Milliseconds 500
$zoomedName = Get-PlotImageName $hwnd
Write-CaptureLog "plot image after two zoom steps: '$zoomedName'"

Click-Button $processId 'Fit'
Start-Sleep -Milliseconds 600
$fitBytes = Save-PrintWindow $hwnd (Join-Path $OutDir 'plot-fit.png')
$fitUia = Save-UiaDump $hwnd $processId (Join-Path $OutDir 'plot-fit-uia.txt')
$fitNameAfter = Get-PlotImageName $hwnd
Write-CaptureLog "plot image after Fit: '$fitNameAfter'"
if ($fitNameAfter -notmatch 'zoom 1\.00') { throw "Fit did not restore zoom 1.00: '$fitNameAfter'" }

Click-Button $processId 'Zoom in'
Start-Sleep -Milliseconds 700
$zoomBytes = Save-PrintWindow $hwnd (Join-Path $OutDir 'plot-zoom.png')
$zoomUia = Save-UiaDump $hwnd $processId (Join-Path $OutDir 'plot-zoom-uia.txt')
$zoomName = Get-PlotImageName $hwnd
Write-CaptureLog "plot image after one zoom step: '$zoomName'"
if ($zoomName -match 'zoom 1\.00') { throw "the zoom step did not change the plot widget: '$zoomName'" }

# --- 4. Export SVG through the native save dialog -----------------------------------------
# The dialog opens in the directory the engine wrote this run's capture into, pre-filled with
# the title-derived name; pressing Save writes there. The written file is then copied into the
# round folder under the deliverable name, and the copy's hash is recorded below.
$plotsRoot = Join-Path $AppData 'Lovelace\plots'
$runCapture = Get-ChildItem -Path $plotsRoot -Recurse -Filter 'plot.svg' -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $runCapture) { throw "no engine plot.svg was found under $plotsRoot" }
$plotRunDir = Split-Path -Parent $runCapture.FullName
Write-CaptureLog "this run's engine capture $($runCapture.FullName) bytes=$($runCapture.Length)"

$ExportSvg = Join-Path $OutDir 'plot-export.svg'
if (Test-Path -LiteralPath $ExportSvg) { Remove-Item -LiteralPath $ExportSvg -Force }
$svgBefore = @(Get-ChildItem -LiteralPath $plotRunDir -File -Filter '*.svg' | ForEach-Object { $_.Name })
$invoke = Start-Invoke $processId 'Export SVG...'
$dialog = Find-Dialog $processId 15
Save-DialogDefault $dialog 'SVG'
$writtenSvg = Wait-NewFile $plotRunDir 'svg' $svgBefore 25
if (-not $invoke.HasExited) { [void]$invoke.WaitForExit(10000) }
Copy-Item -LiteralPath $writtenSvg.FullName -Destination $ExportSvg -Force
$svgItem = Get-Item -LiteralPath $ExportSvg
Write-CaptureLog "exported SVG $($writtenSvg.FullName) -> $ExportSvg bytes=$($svgItem.Length) sha256=$((Get-FileHash -LiteralPath $ExportSvg -Algorithm SHA256).Hash)"

# --- 5. Export PNG (2x, transparent) through the in-window options and the dialog ----------
$ExportPng = Join-Path $OutDir 'plot-export.png'
if (Test-Path -LiteralPath $ExportPng) { Remove-Item -LiteralPath $ExportPng -Force }
Click-Button $processId 'Export PNG...'
Start-Sleep -Milliseconds 700
Click-Button $processId '2x'
Click-Button $processId 'Transparent'
$exportUia = Save-UiaDump $hwnd $processId (Join-Path $OutDir 'plot-export-dialog-uia.txt')
foreach ($needle in @('Export PNG', '1x', '2x', '4x', 'Opaque white', 'Transparent', 'Save PNG...', 'Cancel', 'Selected: 2x, Transparent')) {
    if (-not $exportUia.Contains($needle)) { throw "the PNG export window does not expose '$needle'" }
}
$pngBefore = @(Get-ChildItem -LiteralPath $plotRunDir -File -Filter '*.png' | ForEach-Object { $_.Name })
$invoke = Start-Invoke $processId 'Save PNG...'
$dialog = Find-Dialog $processId 15
Save-DialogDefault $dialog 'PNG'
$writtenPng = Wait-NewFile $plotRunDir 'png' $pngBefore 30
if (-not $invoke.HasExited) { [void]$invoke.WaitForExit(10000) }
Copy-Item -LiteralPath $writtenPng.FullName -Destination $ExportPng -Force
$pngItem = Get-Item -LiteralPath $ExportPng
Write-CaptureLog "exported PNG $($writtenPng.FullName) -> $ExportPng bytes=$($pngItem.Length) sha256=$((Get-FileHash -LiteralPath $ExportPng -Algorithm SHA256).Hash)"

# --- 6. Close the window and verify the artifacts -----------------------------------------
[void](Invoke-Harness @('-Action', 'close', '-ProcessId', "$processId", '-Log', $DriverLog))

$svgBytes = [System.IO.File]::ReadAllBytes($ExportSvg)
$engineBytes = [System.IO.File]::ReadAllBytes($EngineCapture)
$sameAsIndependent = ($svgBytes.Length -eq $engineBytes.Length)
if ($sameAsIndependent) {
    for ($i = 0; $i -lt $svgBytes.Length; $i++) {
        if ($svgBytes[$i] -ne $engineBytes[$i]) { $sameAsIndependent = $false; break }
    }
}
if (-not $sameAsIndependent) { throw 'the exported SVG differs from the independent engine capture' }
Write-CaptureLog "SVG byte identity: exported == independent engine capture ($($svgBytes.Length) bytes)"

$runBytes = [System.IO.File]::ReadAllBytes($runCapture.FullName)
$sameAsRunCapture = ($runBytes.Length -eq $svgBytes.Length)
if ($sameAsRunCapture) {
    for ($i = 0; $i -lt $svgBytes.Length; $i++) {
        if ($svgBytes[$i] -ne $runBytes[$i]) { $sameAsRunCapture = $false; break }
    }
}
Write-CaptureLog "GUI run capture $($runCapture.FullName) bytes=$($runBytes.Length) equal=$sameAsRunCapture"
if (-not $sameAsRunCapture) { throw 'the exported SVG differs from this run''s PlotCapture.svg file' }
$sameAsWritten = ((Get-FileHash -LiteralPath $writtenSvg.FullName -Algorithm SHA256).Hash -eq (Get-FileHash -LiteralPath $ExportSvg -Algorithm SHA256).Hash)
if (-not $sameAsWritten) { throw 'the copied SVG differs from the file the dialog wrote' }

$pngBytes = [System.IO.File]::ReadAllBytes($ExportPng)
$pngSignature = ($pngBytes[0] -eq 0x89 -and $pngBytes[1] -eq 0x50 -and $pngBytes[2] -eq 0x4e -and $pngBytes[3] -eq 0x47)
$pngWidth = [int]$pngBytes[16] * 16777216 + [int]$pngBytes[17] * 65536 + [int]$pngBytes[18] * 256 + [int]$pngBytes[19]
$pngHeight = [int]$pngBytes[20] * 16777216 + [int]$pngBytes[21] * 65536 + [int]$pngBytes[22] * 256 + [int]$pngBytes[23]
$bitmap = New-Object System.Drawing.Bitmap($ExportPng)
$decoded = "$($bitmap.Width)x$($bitmap.Height)"
$bitmap.Dispose()
Write-CaptureLog "PNG signature=$pngSignature IHDR=${pngWidth}x${pngHeight} decoded=$decoded"
if (-not $pngSignature) { throw 'the exported PNG has no PNG signature' }
if ($pngWidth -ne 1600 -or $pngHeight -ne 1200) { throw "PNG IHDR is ${pngWidth}x${pngHeight}, expected 1600x1200 (800x600 at 2x)" }
if ($decoded -ne '1600x1200') { throw "the PNG did not decode to 1600x1200 ($decoded)" }

$summary = [ordered]@{
    round = 30
    requirement = 'PLT-004, PLT-006, DEF-003'
    environment = [ordered]@{
        session = 'locked during the capture: the window proof uses UIA Invoke (AccessKit) plus PrintWindow(PW_RENDERFULLCONTENT); injected pointer input cannot be delivered while the lock screen covers the desktop'
        harness = 'tools/gui-harness.ps1 launch/wait/click/uia/close'
        appdata = $env:APPDATA
        appdata_note = 'redirected to a writable workspace directory and the app starts in the round work directory, so the plot directory is writable and is not under the working directory; without the redirect plot statements fail with Access is denied'
        current_directory = [System.IO.Directory]::GetCurrentDirectory()
        plot_directory = $plotsRoot
        window = '2500x1380'
        device_scale_factor = 1.0
    }
    plot = [ordered]@{
        script = (Get-Content -LiteralPath $Script -Raw)
        title = 'round 30 plot: zoom pan export'
        image_after_run = $fitName
        image_after_two_zoom_steps = $zoomedName
        image_after_fit = $fitNameAfter
        image_after_one_zoom = $zoomName
    }
    captures = @(
        [ordered]@{ file = 'plot-fit.png'; bytes = $fitBytes; sha256 = (Get-FileHash -LiteralPath (Join-Path $OutDir 'plot-fit.png') -Algorithm SHA256).Hash },
        [ordered]@{ file = 'plot-zoom.png'; bytes = $zoomBytes; sha256 = (Get-FileHash -LiteralPath (Join-Path $OutDir 'plot-zoom.png') -Algorithm SHA256).Hash }
    )
    export = [ordered]@{
        svg = [ordered]@{
            file = 'plot-export.svg'
            bytes = $svgItem.Length
            sha256 = (Get-FileHash -LiteralPath $ExportSvg -Algorithm SHA256).Hash
            written_by_dialog = [ordered]@{ file = $writtenSvg.FullName; bytes = $writtenSvg.Length; sha256 = (Get-FileHash -LiteralPath $writtenSvg.FullName -Algorithm SHA256).Hash }
            copy_matches_written_file = $sameAsWritten
            engine_capture_independent = [ordered]@{ file = $EngineCapture; bytes = $engineBytes.Length; sha256 = (Get-FileHash -LiteralPath $EngineCapture -Algorithm SHA256).Hash }
            engine_capture_this_run = [ordered]@{ file = $runCapture.FullName; bytes = $runCapture.Length; sha256 = (Get-FileHash -LiteralPath $runCapture.FullName -Algorithm SHA256).Hash }
            byte_identical_to_independent = $sameAsIndependent
            byte_identical_to_this_run_capture = $sameAsRunCapture
        }
        png = [ordered]@{
            file = 'plot-export.png'
            bytes = $pngItem.Length
            sha256 = (Get-FileHash -LiteralPath $ExportPng -Algorithm SHA256).Hash
            written_by_dialog = [ordered]@{ file = $writtenPng.FullName; bytes = $writtenPng.Length; sha256 = (Get-FileHash -LiteralPath $writtenPng.FullName -Algorithm SHA256).Hash }
            scale = 2
            background = 'Transparent'
            width = $pngWidth
            height = $pngHeight
            decoded = $decoded
            png_signature = $pngSignature
            note = 'the engine chart paints its own opaque white 800x600 canvas, so the transparent background choice cannot change these pixels; the transparent/opaque difference is measured on a document with transparent regions in the crate test'
        }
    }
    uia = @('plot-fit-uia.txt', 'plot-zoom-uia.txt', 'plot-export-dialog-uia.txt')
}
$summaryPath = Join-Path $OutDir 'window-proof.json'
$builder = New-Object System.Text.StringBuilder
Write-JsonValue $summary 0 $builder
[void]$builder.Append("`n")
Set-Content -LiteralPath $summaryPath -Value $builder.ToString() -Encoding UTF8
Write-CaptureLog "wrote $summaryPath"
Write-CaptureLog "done"
