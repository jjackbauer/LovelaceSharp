<#
.SYNOPSIS
  Round-33 window proof for journeys J-02 (basic script) and J-15 (plot presentation).

.DESCRIPTION
  The console session may be LOCKED (the foreground window is the Windows lock screen). The script
  therefore uses only input paths that reach a locked window:
    - the shipped lovelace-desktop.exe is launched on the journey's script file;
    - buttons (Run, Zoom in, Quit) are pressed with UI Automation Invoke (message-based);
    - pixels come from PrintWindow(PW_RENDERFULLCONTENT) into a bitmap of the window rectangle;
    - the UIA tree is dumped flat before/after each action, which is the accessibility artifact.

  J-02: launch on J-02.ls, Run, wait for the status strip to read "run Finished", capture the
  window and dump UIA. The dump must carry the typed result line `[result] = 1024`, the status
  strip (rev 1, cursor, run Finished) and the Workspace variable rows for a and b with their
  Natural kind cells.

  J-15: launch on J-15.ls, Run, wait Finished, then wait for the plot pane to finish rasterising
  ("raster NxM @ Sx" in the header), capture and dump UIA; UIA-invoke "Zoom in" once, wait for the
  canvas widget to report zoom 1.25, capture and dump again. The pane's accessible name carries
  the canvas size and zoom, and the raster line proves the SVG was rendered, not only parsed.
  After a clean quit the engine's plot file is copied into this folder and compared with the
  headless export.

  APPDATA is redirected to this round's appdata window folder and the app starts in the round's
  work folder, so plots are writable and are not under the working directory.

  Windows PowerShell 5.1 compatible. ASCII-only source.
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

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Round33Native {
  public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc call, IntPtr lparam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public static IntPtr WindowOf(uint pid, string titlePart) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, p) => {
      uint owner; GetWindowThreadProcessId(h, out owner);
      if (owner != pid || !IsWindowVisible(h)) { return true; }
      var sb = new StringBuilder(512); GetWindowText(h, sb, 512);
      if (sb.ToString().IndexOf(titlePart, StringComparison.OrdinalIgnoreCase) >= 0) { found = h; return false; }
      return true;
    }, IntPtr.Zero);
    return found;
  }
  public static string ForegroundTitle() {
    var sb = new StringBuilder(512);
    GetWindowText(GetForegroundWindow(), sb, 512);
    return sb.ToString();
  }
}
"@

$Work = Join-Path $OutDir 'window-work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$AppData = Join-Path $OutDir 'window-appdata'
if (Test-Path -LiteralPath $AppData) { Remove-Item -LiteralPath $AppData -Recurse -Force }
New-Item -ItemType Directory -Force -Path $AppData | Out-Null
$CaptureLog = Join-Path $OutDir 'capture-journeys.log'
if (Test-Path -LiteralPath $CaptureLog) { Remove-Item -LiteralPath $CaptureLog -Force }
$env:APPDATA = $AppData
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

$J02 = Join-Path $OutDir 'J-02.ls'
$J15 = Join-Path $OutDir 'J-15.ls'
$PlotDir = Join-Path $AppData 'Lovelace\plots'
$ExpectedSvgHash = '0B23D6F74C16BDD102CEE23E9388D884A9212FC614CCA55B9F814EA66271406C'

$script:Checks = New-Object System.Collections.Generic.List[object]
function Write-CaptureLog([string]$line) {
    Write-Host $line
    Add-Content -LiteralPath $CaptureLog -Value $line -Encoding UTF8
}
function Add-Check([string]$name, [bool]$ok, [string]$detail) {
    $script:Checks.Add([pscustomobject]@{ name = $name; ok = $ok; detail = $detail }) | Out-Null
    Write-CaptureLog ("CHECK {0} {1}: {2}" -f $(if ($ok) { 'ok  ' } else { 'FAIL' }), $name, $detail)
}
function Get-Sha256([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash }
function Test-BytesEqual([byte[]]$a, [byte[]]$b) {
    if ($a.Length -ne $b.Length) { return $false }
    for ($i = 0; $i -lt $a.Length; $i++) { if ($a[$i] -ne $b[$i]) { return $false } }
    return $true
}

function Get-AppRoot([int]$processId) {
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $processId)
    return [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children, $condition)
}

function Get-UiaTree([int]$processId) {
    $root = Get-AppRoot $processId
    if (-not $root) { throw 'no UIA root for the app process' }
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    $lines = New-Object System.Collections.Generic.List[string]
    $lines.Add(("UIA pid={0} window={1}" -f $processId, $root.Current.Name)) | Out-Null
    foreach ($element in $all) {
        $current = $element.Current
        $lines.Add(("{0} | {1} | focusable={2} | enabled={3}" -f
            $current.ControlType.ProgrammaticName, $current.Name,
            $current.IsKeyboardFocusable, $current.IsEnabled)) | Out-Null
    }
    return $lines
}

function Save-UiaDump([int]$processId, [string]$name) {
    $lines = Get-UiaTree $processId
    $path = Join-Path $OutDir "$name-uia.txt"
    Set-Content -LiteralPath $path -Value $lines -Encoding UTF8
    Write-CaptureLog ("UIA {0}: {1} nodes -> {2}" -f $name, $lines.Count, $path)
    return $lines
}

function Save-Capture([int]$processId, [string]$name) {
    $hwnd = [Round33Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    if ($hwnd -eq [IntPtr]::Zero) { throw "no app window for $name" }
    $rect = New-Object Round33Native+RECT
    if (-not [Round33Native]::GetWindowRect($hwnd, [ref]$rect)) { throw "GetWindowRect failed for $name" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "window has no size for $name ($width x $height)" }
    $bitmap = New-Object System.Drawing.Bitmap ($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Round33Native]::PrintWindow($hwnd, $hdc, 2)
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $path = Join-Path $OutDir "$name.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $size = (Get-Item -LiteralPath $path).Length
    if (-not $ok) { throw "PrintWindow returned false for $name" }
    Write-CaptureLog ("CAPTURE {0}: PrintWindow(PW_RENDERFULLCONTENT) hwnd={1} rect={2}x{3} -> {4} ({5} bytes, sha256={6})" -f
        $name, $hwnd, $width, $height, $path, $size, (Get-Sha256 $path))
    return [pscustomobject]@{ name = $name; path = $path; width = $width; height = $height; bytes = $size; sha256 = (Get-Sha256 $path) }
}

function Get-Buttons([int]$processId) {
    $root = Get-AppRoot $processId
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button)
    return @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

function Invoke-Button([int]$processId, [string]$name, [int]$index = 0) {
    $found = @(Get-Buttons $processId | Where-Object { $_.Current.Name -eq $name })
    if ($found.Count -le $index) {
        $visible = @(Get-Buttons $processId | ForEach-Object { $_.Current.Name }) -join ', '
        throw ("no button '{0}' at index {1} (found {2}; buttons: {3})" -f $name, $index, $found.Count, $visible)
    }
    $target = $found[$index]
    $pattern = $target.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    Write-CaptureLog ("UIA INVOKE button '{0}' index {1} of {2} rect={3}" -f
        $name, $index, $found.Count, $target.Current.BoundingRectangle)
}

function Find-UiaLine($lines, [string]$pattern) {
    return @($lines | Where-Object { $_ -match $pattern })
}

function Wait-ForStatus([int]$processId, [string]$status, [int]$seconds = 60) {
    $deadline = (Get-Date).AddSeconds($seconds)
    do {
        Start-Sleep -Milliseconds 150
        try { $lines = Get-UiaTree $processId } catch { continue }
        $hit = @($lines | Where-Object { $_ -match 'ControlType\.Text' -and $_ -match [regex]::Escape($status) })
        if ($hit.Count -ge 1) { return $hit }
    } while ((Get-Date) -lt $deadline)
    throw "the status strip never reported '$status'"
}

function Wait-ForUiaPattern([int]$processId, [string]$pattern, [int]$seconds = 20) {
    $deadline = (Get-Date).AddSeconds($seconds)
    do {
        Start-Sleep -Milliseconds 200
        try { $lines = Get-UiaTree $processId } catch { continue }
        $hit = @($lines | Where-Object { $_ -match $pattern })
        if ($hit.Count -ge 1) { return $hit }
    } while ((Get-Date) -lt $deadline)
    throw "no UIA node matched '$pattern'"
}

function Wait-ForWindow([int]$processId, [int]$seconds = 30) {
    $deadline = (Get-Date).AddSeconds($seconds)
    $hwnd = [IntPtr]::Zero
    do {
        Start-Sleep -Milliseconds 250
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if (-not $process) { throw 'the app exited during startup' }
        $hwnd = [Round33Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    } while ($hwnd -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline)
    if ($hwnd -eq [IntPtr]::Zero) { throw 'the workbench window never appeared' }
    [void][Round33Native]::SetWindowPos($hwnd, [IntPtr]::Zero, 10, 10, 1600, 1000, 0x0014)
    Start-Sleep -Milliseconds 900
    return $hwnd
}

function Get-WindowTitle([IntPtr]$hwnd) {
    $title = New-Object System.Text.StringBuilder 512
    [void][Round33Native]::GetWindowText($hwnd, $title, 512)
    return $title.ToString()
}

function Stop-App([int]$processId, $process) {
    try { Invoke-Button $processId 'Quit' } catch { Write-CaptureLog ("QUIT not found: {0}" -f $_.Exception.Message) }
    if (-not $process.WaitForExit(15000)) {
        Write-CaptureLog 'QUIT timed out; killing the process'
        Stop-Process -Id $processId -Force
        Start-Sleep -Milliseconds 400
    }
}

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

# ---------------------------------------------------------------------------------------------
# 0. Session state and fixtures
# ---------------------------------------------------------------------------------------------
$foreground = [Round33Native]::ForegroundTitle()
$locked = $foreground -match 'Lock Screen|Default Lock|Windows Default'
Write-CaptureLog ("SESSION foreground='{0}' locked={1}" -f $foreground, $locked)
foreach ($fixture in @($J02, $J15)) {
    Add-Check ("fixture " + (Split-Path -Leaf $fixture)) (Test-Path -LiteralPath $fixture) (("{0} bytes" -f (Get-Item -LiteralPath $fixture).Length))
}

# ---------------------------------------------------------------------------------------------
# 1. J-02: launch on the script, Run, capture result + variables
# ---------------------------------------------------------------------------------------------
$process = Start-Process -FilePath $Exe -ArgumentList @("`"$J02`"") -PassThru
$hwnd = Wait-ForWindow $process.Id
$title = Get-WindowTitle $hwnd
Write-CaptureLog ("LAUNCH J-02 pid={0} hwnd={1} title='{2}'" -f $process.Id, $hwnd, $title)
Add-Check 'J-02 launch title names the script' ($title.Contains('J-02.ls')) $title

$idle = Wait-ForStatus $process.Id 'run Idle' 30
$idleStatus = (@($idle | Select-Object -First 1))[0]
Write-CaptureLog ("STATUS J-02 before run: {0}" -f $idleStatus)
Add-Check 'J-02 status strip is idle before Run' ($idleStatus -match 'run Idle') $idleStatus
Invoke-Button $process.Id 'Run'
$finished = Wait-ForStatus $process.Id 'run Finished' 60
$finishedStatus = (@($finished | Select-Object -First 1))[0]
Write-CaptureLog ("STATUS J-02 after run: {0}" -f $finishedStatus)
Add-Check 'J-02 status strip reports run Finished' ($finishedStatus -match 'run Finished') $finishedStatus
Add-Check 'J-02 status strip reports revision 1' ($finishedStatus -match '\|  rev 1  ') $finishedStatus
Add-Check 'J-02 status strip reports the cursor' ($finishedStatus -match 'Ln 1, Col 1') $finishedStatus

Start-Sleep -Milliseconds 500
$j02Lines = Save-UiaDump $process.Id 'J-02'
$j02Capture = Save-Capture $process.Id 'J-02'
$resultLines = Find-UiaLine $j02Lines '\| \[result\] = 1024 \|'
Add-Check 'J-02 typed result line' ($resultLines.Count -ge 1) ($resultLines -join ' ; ')
$logLines = Find-UiaLine $j02Lines '\[timing\] Run finished: 3 statements committed'
Add-Check 'J-02 run-finished log line' ($logLines.Count -ge 1) ($logLines -join ' ; ')
$progress = Find-UiaLine $j02Lines '\[progress\] statement 3/3: b'
Add-Check 'J-02 per-statement log line' ($progress.Count -ge 1) ($progress -join ' ; ')
$varHeader = Find-UiaLine $j02Lines 'VARIABLES . 3'
Add-Check 'J-02 variable table header' ($varHeader.Count -ge 1) ($varHeader -join ' ; ')
# The variable rows are the lines between the VARIABLES heading and the FUNCTIONS heading. Read
# that slice rather than the whole tree so an editor or result-tree text node cannot satisfy a
# row check by accident.
$varStart = -1
$funcStart = $j02Lines.Count
for ($index = 0; $index -lt $j02Lines.Count; $index++) {
    if ($varStart -lt 0 -and $j02Lines[$index] -match 'VARIABLES . 3') { $varStart = $index + 1 }
    elseif ($varStart -ge 0 -and $j02Lines[$index] -match 'FUNCTIONS . ') { $funcStart = $index; break }
}
if ($varStart -lt 0) { $varStart = 0 }
$varLines = @($j02Lines[$varStart..($funcStart - 1)])
Write-CaptureLog ("VARIABLE ROWS J-02: {0}" -f (($varLines | ForEach-Object { ($_ -split '\|')[1] }) -join ' | '))
foreach ($expected in @(
    @('a', 'variable row name a'),
    @('Natural', 'variable row kind Natural'),
    @('2', 'variable row display 2'),
    @('b', 'variable row name b'),
    @('1024', 'variable row display 1024'))) {
    $hits = @($varLines | Where-Object { $_ -match ('^ControlType\.Text \| ' + [regex]::Escape($expected[0]) + ' \|') })
    Add-Check ('J-02 ' + $expected[1]) ($hits.Count -ge 1) ($hits -join ' ; ')
}
Add-Check 'J-02 capture is a real window bitmap' ($j02Capture.bytes -gt 100000 -and $j02Capture.width -eq 1600 -and $j02Capture.height -eq 1000) ("{0} bytes, {1}x{2}" -f $j02Capture.bytes, $j02Capture.width, $j02Capture.height)
Stop-App $process.Id $process
Start-Sleep -Milliseconds 500

# ---------------------------------------------------------------------------------------------
# 2. J-15: launch on the plot script, Run, capture the rendered plot, then zoom
# ---------------------------------------------------------------------------------------------
$process2 = Start-Process -FilePath $Exe -ArgumentList @("`"$J15`"") -PassThru
$hwnd2 = Wait-ForWindow $process2.Id
$title2 = Get-WindowTitle $hwnd2
Write-CaptureLog ("LAUNCH J-15 pid={0} hwnd={1} title='{2}'" -f $process2.Id, $hwnd2, $title2)
Add-Check 'J-15 launch title names the script' ($title2.Contains('J-15.ls')) $title2

Invoke-Button $process2.Id 'Run'
$finished2 = Wait-ForStatus $process2.Id 'run Finished' 60
$finished2Status = (@($finished2 | Select-Object -First 1))[0]
Write-CaptureLog ("STATUS J-15 after run: {0}" -f $finished2Status)
Add-Check 'J-15 status strip reports run Finished' ($finished2Status -match 'run Finished') $finished2Status
$rastered = Wait-ForUiaPattern $process2.Id 'raster [0-9]+x[0-9]+ @' 30
Write-CaptureLog ("RASTER J-15: {0}" -f (@($rastered)[0]))
Start-Sleep -Milliseconds 500
$j15Lines = Save-UiaDump $process2.Id 'J-15'
$j15Capture = Save-Capture $process2.Id 'J-15'
$canvas = Find-UiaLine $j15Lines 'SVG plot canvas, 800 by 600 points, zoom 1.00'
Add-Check 'J-15 canvas widget names the 800x600 plot at fit' ($canvas.Count -ge 1) ($canvas -join ' ; ')
$rasterLine = Find-UiaLine $j15Lines 'raster [0-9]+x[0-9]+ @'
Add-Check 'J-15 pane rasterised the SVG (render proof)' ($rasterLine.Count -ge 1) ($rasterLine -join ' ; ')
$plotResult = Find-UiaLine $j15Lines '\[result\] = .*plot\.svg'
Add-Check 'J-15 result line names the engine plot file' ($plotResult.Count -ge 1) ($plotResult -join ' ; ')
Add-Check 'J-15 capture is a real window bitmap' ($j15Capture.bytes -gt 100000 -and $j15Capture.width -eq 1600 -and $j15Capture.height -eq 1000) ("{0} bytes, {1}x{2}" -f $j15Capture.bytes, $j15Capture.width, $j15Capture.height)

Invoke-Button $process2.Id 'Zoom in'
$zoomed = Wait-ForUiaPattern $process2.Id 'SVG plot canvas, 800 by 600 points, zoom 1\.25' 20
Write-CaptureLog ("ZOOM J-15: {0}" -f (@($zoomed)[0]))
Start-Sleep -Milliseconds 600
$j15ZoomLines = Save-UiaDump $process2.Id 'J-15-zoom'
$j15ZoomCapture = Save-Capture $process2.Id 'J-15-zoom'
$zoomCanvas = Find-UiaLine $j15ZoomLines 'SVG plot canvas, 800 by 600 points, zoom 1\.25'
Add-Check 'J-15 zoom changed the pane transform' ($zoomCanvas.Count -ge 1) ($zoomCanvas -join ' ; ')
$fitRasterText = (@($rasterLine | Select-Object -First 1))[0]
$zoomRaster = Find-UiaLine $j15ZoomLines 'raster [0-9]+x[0-9]+ @'
$zoomRasterText = (@($zoomRaster | Select-Object -First 1))[0]
$fitWidth = 0
if ($fitRasterText -match 'raster ([0-9]+)x') { $fitWidth = [int]$Matches[1] }
$zoomWidth = 0
if ($zoomRasterText -match 'raster ([0-9]+)x') { $zoomWidth = [int]$Matches[1] }
Add-Check 'J-15 zoom re-rasterised at the larger scale' ($zoomWidth -gt $fitWidth -and $fitWidth -gt 0) ("{0} -> {1}" -f $fitRasterText, $zoomRasterText)
Stop-App $process2.Id $process2
Start-Sleep -Milliseconds 800

# ---------------------------------------------------------------------------------------------
# 3. The engine plot file written by the window run
# ---------------------------------------------------------------------------------------------
$plotFile = Get-ChildItem -LiteralPath $PlotDir -Recurse -Filter 'plot.svg' -File -ErrorAction SilentlyContinue | Select-Object -First 1
$plotFileDetail = $PlotDir
if ($null -ne $plotFile) { $plotFileDetail = $plotFile.FullName }
Add-Check 'J-15 window run wrote the engine plot file' ($null -ne $plotFile) $plotFileDetail
if ($plotFile) {
    $windowPlot = Join-Path $OutDir 'J-15-window-plot.svg'
    Copy-Item -LiteralPath $plotFile.FullName -Destination $windowPlot -Force
    $windowHash = Get-Sha256 $windowPlot
    $windowBytes = (Get-Item -LiteralPath $windowPlot).Length
    Write-CaptureLog ("PLOT COPY {0}: {1} bytes sha256={2}" -f $windowPlot, $windowBytes, $windowHash)
    Add-Check 'J-15 window plot matches the reference SVG' ($windowHash -eq $ExpectedSvgHash) ("{0} bytes sha256={1}" -f $windowBytes, $windowHash)
    $headlessExport = Join-Path $OutDir 'journey-out\J-15-export.svg'
    if (Test-Path -LiteralPath $headlessExport) {
        $equal = Test-BytesEqual ([System.IO.File]::ReadAllBytes($windowPlot)) ([System.IO.File]::ReadAllBytes($headlessExport))
        Add-Check 'J-15 window plot is byte-identical to the headless export' $equal ("{0} vs {1}" -f $windowPlot, $headlessExport)
    }
}

# ---------------------------------------------------------------------------------------------
# 4. Summary
# ---------------------------------------------------------------------------------------------
$passed = @($script:Checks | Where-Object { $_.ok }).Count
$failed = @($script:Checks | Where-Object { -not $_.ok }).Count
Write-CaptureLog ("SUMMARY passed={0} failed={1}" -f $passed, $failed)

$json = New-Object System.Text.StringBuilder
[void]$json.Append("{`n")
[void]$json.Append("  ""round"": 33,`n")
[void]$json.Append("  ""journeys"": [""J-02"", ""J-15""],`n")
[void]$json.Append("  ""method"": ""UIA Invoke + PrintWindow(PW_RENDERFULLCONTENT) on a $(if ($locked) { 'locked' } else { 'possibly-unlocked' }) session; no injected keyboard or pointer input"",`n")
[void]$json.Append(("  ""session"": {0},`n" -f (ConvertTo-JsonString $(if ($locked) { 'locked' } else { 'unlocked-or-unknown' }))))
[void]$json.Append(("  ""foreground"": {0},`n" -f (ConvertTo-JsonString $foreground)))
[void]$json.Append(("  ""window"": ""1600x1000"",`n"))
[void]$json.Append(("  ""appdata"": {0},`n" -f (ConvertTo-JsonString $AppData)))
[void]$json.Append(("  ""plot_directory"": {0},`n" -f (ConvertTo-JsonString $PlotDir)))
[void]$json.Append("  ""captures"": [`n")
$captures = @($j02Capture, $j15Capture, $j15ZoomCapture)
for ($index = 0; $index -lt $captures.Count; $index++) {
    $capture = $captures[$index]
    $comma = if ($index -lt $captures.Count - 1) { ',' } else { '' }
    [void]$json.Append(("    {{ ""name"": {0}, ""file"": {1}, ""width"": {2}, ""height"": {3}, ""bytes"": {4}, ""sha256"": {5} }}{6}" -f
        (ConvertTo-JsonString $capture.name), (ConvertTo-JsonString (Split-Path -Leaf $capture.path)),
        $capture.width, $capture.height, $capture.bytes, (ConvertTo-JsonString $capture.sha256), $comma))
    [void]$json.Append("`n")
}
[void]$json.Append("  ],`n")
[void]$json.Append("  ""uia"": [""J-02-uia.txt"", ""J-15-uia.txt"", ""J-15-zoom-uia.txt""],`n")
[void]$json.Append("  ""checks"": [`n")
for ($index = 0; $index -lt $script:Checks.Count; $index++) {
    $check = $script:Checks[$index]
    $comma = if ($index -lt $script:Checks.Count - 1) { ',' } else { '' }
    [void]$json.Append(("    {{ ""name"": {0}, ""ok"": {1}, ""detail"": {2} }}{3}" -f
        (ConvertTo-JsonString $check.name), $(if ($check.ok) { 'true' } else { 'false' }),
        (ConvertTo-JsonString $check.detail), $comma))
    [void]$json.Append("`n")
}
[void]$json.Append("  ],`n")
[void]$json.Append(("  ""passed"": {0},`n" -f $passed))
[void]$json.Append(("  ""failed"": {0}`n" -f $failed))
[void]$json.Append("}`n")
Set-Content -LiteralPath (Join-Path $OutDir 'window-proof.json') -Value $json.ToString() -Encoding UTF8

if ($failed -gt 0) { exit 1 }
exit 0
