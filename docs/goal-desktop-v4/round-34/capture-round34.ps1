<#
.SYNOPSIS
  Round-34 window proof for journeys J-03, J-10, J-11 and J-12.

.DESCRIPTION
  The console session is LOCKED (the foreground window is the Windows lock screen: LogonUI owns
  it). The script therefore uses only input paths that reach a locked window:

    - the shipped lovelace-desktop.exe is launched on the journey's script file;
    - buttons (Run, Clear Variables, Save, Quit) are pressed with UI Automation Invoke;
    - the precision spinner is driven through its UIA RangeValue pattern;
    - J-11's typing and J-12's F8 are delivered as window messages (WM_KEYDOWN/WM_CHAR/WM_KEYUP)
      because SendKeys/keybd_event go to the secure desktop and cannot reach a locked window;
    - pixels come from PrintWindow(PW_RENDERFULLCONTENT) into a 1600x1000 32bpp bitmap;
    - the UIA tree is dumped flat for every checked state.

  tools/gui-harness.ps1 is the project's window harness; its interactive paths (keys/capture) call
  Focus-Window/Assert-Foreground and CopyFromScreen, which a locked session cannot satisfy. This
  driver uses the same UIA Invoke + PrintWindow protocol as tools/gui-harness.ps1's locked path,
  and the same real product window, document and run/ExecutionHost path.

  APPDATA is redirected to this round's per-journey appdata folder and the app starts in the
  round's work folder, so the app-scoped plot directory is writable and is not under the working
  directory.

  Windows PowerShell 5.1 compatible. ASCII-only source.
#>
[CmdletBinding()]
param(
    [string]$Repo = 'C:\Users\ricar\dev\LovelaceSharp',
    [string]$OutDir = ''
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes

if (-not $OutDir) { $OutDir = $PSScriptRoot }
$Exe = Join-Path $Repo 'rust\target-desktop\debug\lovelace-desktop.exe'
if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Round34Native {
  public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint mapType);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc call, IntPtr lparam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr processId);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
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

$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$AppRoot = Join-Path $OutDir 'appdata'
if (Test-Path -LiteralPath $AppRoot) { Remove-Item -LiteralPath $AppRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path $AppRoot | Out-Null
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

$CaptureLog = Join-Path $OutDir 'capture-round34.log'
if (Test-Path -LiteralPath $CaptureLog) { Remove-Item -LiteralPath $CaptureLog -Force }
$script:Checks = New-Object System.Collections.Generic.List[object]

function Write-CaptureLog([string]$line) {
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    $text = "$stamp $line"
    Write-Host $text
    Add-Content -LiteralPath $CaptureLog -Value $text -Encoding UTF8
}
function Add-Check([string]$name, [bool]$ok, [string]$detail) {
    $script:Checks.Add([pscustomobject]@{ name = $name; ok = $ok; detail = $detail }) | Out-Null
    Write-CaptureLog ("CHECK {0} {1}: {2}" -f $(if ($ok) { 'ok  ' } else { 'FAIL' }), $name, $detail)
}
function Get-Sha256([string]$path) { (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash }
function Get-Sha256Text([string]$text) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($text)
    $hash = $sha.ComputeHash($bytes)
    $builder = New-Object System.Text.StringBuilder
    foreach ($byte in $hash) { [void]$builder.Append($byte.ToString('x2')) }
    $sha.Dispose()
    return $builder.ToString()
}
function Write-Utf8([string]$path, [string]$text) {
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($path, $text, $encoding)
}
function Get-UiaName([string]$line) {
    if ($line -match '^ControlType\.[A-Za-z]+ \| (.*) \| focusable=(True|False) \| enabled=(True|False)$') {
        return $Matches[1]
    }
    return $null
}
function Get-AppRoot([int]$processId) {
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ProcessIdProperty, $processId)
    $root = [System.Windows.Automation.AutomationElement]::RootElement.FindFirst(
        [System.Windows.Automation.TreeScope]::Children, $condition)
    if (-not $root) { throw "no UIA root for process $processId" }
    return $root
}
function Get-UiaLines([int]$processId) {
    $root = Get-AppRoot $processId
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
function Save-Uia([int]$processId, [string]$name) {
    $lines = Get-UiaLines $processId
    $path = Join-Path $OutDir "$name-uia.txt"
    Set-Content -LiteralPath $path -Value $lines -Encoding UTF8
    Write-CaptureLog ("UIA {0}: {1} nodes -> {2}" -f $name, $lines.Count, $path)
    return $lines
}
function Save-Capture([int]$processId, [string]$name) {
    $hwnd = [Round34Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    if ($hwnd -eq [IntPtr]::Zero) { throw "no app window for $name" }
    $rect = New-Object Round34Native+RECT
    if (-not [Round34Native]::GetWindowRect($hwnd, [ref]$rect)) { throw "GetWindowRect failed for $name" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "window has no size for $name ($width x $height)" }
    $bitmap = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Round34Native]::PrintWindow($hwnd, $hdc, 2)
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $path = Join-Path $OutDir "$name.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $size = (Get-Item -LiteralPath $path).Length
    Add-Check ("capture " + $name) ($ok -and $size -gt 100000 -and $width -eq 1600 -and $height -eq 1000) ("PrintWindow(PW_RENDERFULLCONTENT) {0}x{1}, {2} bytes, sha256={3}" -f $width, $height, $size, (Get-Sha256 $path))
    return [pscustomobject]@{ name = $name; file = (Split-Path -Leaf $path); width = $width; height = $height; bytes = $size; sha256 = (Get-Sha256 $path) }
}
function Wait-Window([int]$processId, [int]$seconds = 30) {
    $deadline = (Get-Date).AddSeconds($seconds)
    $hwnd = [IntPtr]::Zero
    do {
        Start-Sleep -Milliseconds 250
        if (-not (Get-Process -Id $processId -ErrorAction SilentlyContinue)) { throw 'the app exited during startup' }
        $hwnd = [Round34Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    } while ($hwnd -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline)
    if ($hwnd -eq [IntPtr]::Zero) { throw 'the workbench window never appeared' }
    # eframe can start minimized (the 160x28 -32000 placeholder). Restore it before sizing and
    # do not proceed until the client rect is really the capture size, or PrintWindow saves the
    # placeholder instead of the workbench.
    if ([Round34Native]::IsIconic($hwnd)) {
        [void][Round34Native]::ShowWindow($hwnd, 9)
        Start-Sleep -Milliseconds 350
    }
    $rect = New-Object Round34Native+RECT
    for ($attempt = 0; $attempt -lt 25; $attempt++) {
        [void][Round34Native]::SetWindowPos($hwnd, [IntPtr]::Zero, 10, 10, 1600, 1000, 0x0014)
        Start-Sleep -Milliseconds 200
        if ([Round34Native]::IsIconic($hwnd)) { [void][Round34Native]::ShowWindow($hwnd, 9) }
        [void][Round34Native]::GetWindowRect($hwnd, [ref]$rect)
        if (($rect.Right - $rect.Left) -eq 1600 -and ($rect.Bottom - $rect.Top) -eq 1000) { break }
    }
    [void][Round34Native]::GetWindowRect($hwnd, [ref]$rect)
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -ne 1600 -or $height -ne 1000) { throw "the workbench window never reached 1600x1000 (got ${width}x${height})" }
    Start-Sleep -Milliseconds 900
    return $hwnd
}
function Wait-Uia([int]$processId, [string]$needle, [int]$seconds = 60) {
    $deadline = (Get-Date).AddSeconds($seconds)
    do {
        Start-Sleep -Milliseconds 150
        try { $lines = Get-UiaLines $processId } catch { continue }
        $hit = @($lines | Where-Object { $_.Contains($needle) })
        if ($hit.Count -ge 1) { return $lines }
    } while ((Get-Date) -lt $deadline)
    throw "no UIA node contained '$needle'"
}
function Is-LockedScreen {
    return ([Round34Native]::ForegroundTitle() -match 'Lock Screen|Default Lock|Windows Default')
}
function Focus-App([IntPtr]$hwnd) {
    if ([Round34Native]::IsIconic($hwnd)) {
        [void][Round34Native]::ShowWindow($hwnd, 9)
        Start-Sleep -Milliseconds 150
    }
    if ([Round34Native]::GetForegroundWindow() -eq $hwnd) { return $true }
    [void][Round34Native]::ShowWindow($hwnd, 9)
    [void][Round34Native]::SetWindowPos($hwnd, [IntPtr](-1), 0, 0, 0, 0, 0x0003)
    [void][Round34Native]::BringWindowToTop($hwnd)
    [void][Round34Native]::SetForegroundWindow($hwnd)
    Start-Sleep -Milliseconds 200
    if ([Round34Native]::GetForegroundWindow() -eq $hwnd) { return $true }
    $foreground = [Round34Native]::GetForegroundWindow()
    $foregroundThread = 0
    if ($foreground -ne [IntPtr]::Zero) {
        $foregroundThread = [Round34Native]::GetWindowThreadProcessId($foreground, [IntPtr]::Zero)
    }
    $myThread = [Round34Native]::GetCurrentThreadId()
    $attached = $false
    if ($foregroundThread -ne 0 -and $foregroundThread -ne $myThread) {
        $attached = [Round34Native]::AttachThreadInput($myThread, $foregroundThread, $true)
    }
    [void][Round34Native]::BringWindowToTop($hwnd)
    [void][Round34Native]::SetForegroundWindow($hwnd)
    if ($attached) { [void][Round34Native]::AttachThreadInput($myThread, $foregroundThread, $false) }
    Start-Sleep -Milliseconds 250
    return ([Round34Native]::GetForegroundWindow() -eq $hwnd)
}
function Focus-Editor([int]$processId) {
    $editor = Get-EditorElement $processId
    if ($null -ne $editor) {
        try { $editor.SetFocus(); Start-Sleep -Milliseconds 200 } catch { }
    }
}
function Type-Into-Editor($app, [string]$text) {
    Focus-Editor $app.process.Id
    $column = 1 + $text.Length
    $mode = 'postmessage'
    if (-not (Is-LockedScreen)) {
        if (Focus-App $app.hwnd) {
            [System.Windows.Forms.SendKeys]::SendWait($text)
            $mode = 'sendkeys'
            Write-CaptureLog ("SENDKEYS '{0}' to foreground hwnd={1}" -f $text, $app.hwnd)
            Start-Sleep -Milliseconds 500
        } else {
            Write-CaptureLog 'SENDKEYS skipped: the app did not take the foreground; using window messages'
        }
    }
    $deadline = (Get-Date).AddSeconds(8)
    do {
        $lines = @(Get-UiaLines $app.process.Id)
        if (@($lines | Where-Object { $_.Contains("caret line 1 column $column") }).Count -ge 1) { return $mode }
        Start-Sleep -Milliseconds 200
    } while ((Get-Date) -lt $deadline)
    foreach ($character in $text.ToCharArray()) { Type-Letter $app.hwnd $character }
    $null = Wait-Uia $app.process.Id ("caret line 1 column $column") 10
    return 'postmessage'
}
function Press-F8($app) {
    if (-not (Is-LockedScreen)) {
        if (Focus-App $app.hwnd) {
            [System.Windows.Forms.SendKeys]::SendWait('{F8}')
            Write-CaptureLog ("SENDKEYS F8 to foreground hwnd={0}" -f $app.hwnd)
            Start-Sleep -Milliseconds 500
            return
        }
        Write-CaptureLog 'SENDKEYS F8 skipped: the app did not take the foreground; using window messages'
    }
    Post-Key $app.hwnd ([uint32]0x77) 0
}
function Get-UiaButton([int]$processId, [string]$name, [int]$index = 0) {
    $root = Get-AppRoot $processId
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button)
    $buttons = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    $found = @()
    foreach ($button in $buttons) { if ($button.Current.Name -eq $name) { $found += $button } }
    if ($found.Count -le $index) {
        $visible = @($buttons | ForEach-Object { $_.Current.Name }) -join ', '
        throw ("no button '{0}' at index {1} (found {2}; buttons: {3})" -f $name, $index, $found.Count, $visible)
    }
    return $found[$index]
}
function Invoke-UiaButton([int]$processId, [string]$name, [int]$index = 0) {
    $target = Get-UiaButton $processId $name $index
    $pattern = $target.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    Write-CaptureLog ("UIA INVOKE button '{0}' index {1}" -f $name, $index)
}
function Set-Precision([int]$processId, [int]$value) {
    $root = Get-AppRoot $processId
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Spinner)
    $spinners = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    if ($spinners.Count -lt 1) { throw 'no precision spinner' }
    $pattern = $spinners[0].GetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern)
    $pattern.SetValue($value)
    Write-CaptureLog ("UIA SET precision spinner value={0}" -f $value)
    $null = Wait-Uia $processId ("precision $value") 15
}
function Post-Key([IntPtr]$hwnd, [uint32]$virtualKey, [int]$character) {
    $scan = [Round34Native]::MapVirtualKey($virtualKey, 0)
    $down = [IntPtr]([int64](($scan -shl 16) -bor 1))
    $up = [IntPtr]([int64](($scan -shl 16) -bor 0xC0000001))
    [void][Round34Native]::PostMessage($hwnd, 0x0100, [IntPtr]$virtualKey, $down)
    if ($character -gt 0) {
        [void][Round34Native]::PostMessage($hwnd, 0x0102, [IntPtr]$character, $down)
    }
    [void][Round34Native]::PostMessage($hwnd, 0x0101, [IntPtr]$virtualKey, $up)
    Start-Sleep -Milliseconds 220
}
function Type-Letter([IntPtr]$hwnd, [char]$character) {
    $upper = [char]::ToUpperInvariant($character)
    Post-Key $hwnd ([uint32][int]$upper) ([int]$character)
}
function Start-Journey([string]$journey, [string]$fixture) {
    $appData = Join-Path $AppRoot $journey
    New-Item -ItemType Directory -Force -Path $appData | Out-Null
    $env:APPDATA = $appData
    $process = Start-Process -FilePath $Exe -ArgumentList @(('"' + $fixture + '"')) -PassThru
    $hwnd = Wait-Window $process.Id
    $process.Refresh()
    Write-CaptureLog ("LAUNCH {0} pid={1} hwnd={2} title='{3}' appdata={4}" -f $journey, $process.Id, $hwnd, $process.MainWindowTitle, $appData)
    $null = Wait-Uia $process.Id 'run Idle' 30
    return @{ process = $process; hwnd = $hwnd; journey = $journey }
}
function Stop-Journey($app) {
    $processId = $app.process.Id
    try {
        Invoke-UiaButton $processId 'Quit'
    } catch {
        Write-CaptureLog ("QUIT not found: {0}" -f $_.Exception.Message)
    }
    if (-not $app.process.WaitForExit(15000)) {
        Write-CaptureLog 'QUIT timed out; killing the process'
        Stop-Process -Id $processId -Force
        Start-Sleep -Milliseconds 400
    }
}
function Get-Slice($lines, [string]$from, [string]$to) {
    $start = -1
    $stop = @($lines).Count
    for ($index = 0; $index -lt @($lines).Count; $index++) {
        if ($start -lt 0 -and $lines[$index].Contains($from)) { $start = $index }
        elseif ($start -ge 0 -and $lines[$index].Contains($to)) { $stop = $index; break }
    }
    if ($start -lt 0) { return @() }
    return @($lines[$start..($stop - 1)])
}
function Get-Last-RunLines($lines) {
    $start = 0
    for ($index = 0; $index -lt @($lines).Count; $index++) {
        if ($lines[$index].Contains('Run started: ')) { $start = $index }
    }
    return @($lines[$start..(@($lines).Count - 1)])
}
function Get-ResultValues($lines) {
    $values = @()
    foreach ($line in $lines) {
        $name = Get-UiaName $line
        if ($name -and $name.StartsWith('[result] = ')) {
            $values += $name.Substring(11)
        }
    }
    return $values
}
function Get-FractionalDigits([string]$text) {
    $point = $text.IndexOf('.')
    if ($point -lt 0) { return 0 }
    $fraction = $text.Substring($point + 1)
    $count = 0
    foreach ($character in $fraction.ToCharArray()) {
        if ($character -ge '0' -and $character -le '9') { $count++ } else { break }
    }
    return $count
}
function Get-EditorElement([int]$processId) {
    $root = Get-AppRoot $processId
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit)
    $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition)
    foreach ($edit in $edits) {
        if ($edit.Current.Name.StartsWith('Script editor')) { return $edit }
    }
    return $null
}
function Find-ColourBox([string]$path, [int]$left, [int]$top, [int]$right, [int]$bottom, [int]$red, [int]$green, [int]$blue) {
    $bitmap = [System.Drawing.Bitmap]::FromFile($path)
    try {
        if ($left -lt 0) { $left = 0 }
        if ($top -lt 0) { $top = 0 }
        if ($right -gt $bitmap.Width) { $right = $bitmap.Width }
        if ($bottom -gt $bitmap.Height) { $bottom = $bitmap.Height }
        $width = $right - $left
        $height = $bottom - $top
        if ($width -le 0 -or $height -le 0) { throw 'empty scan rect' }
        $rect = New-Object System.Drawing.Rectangle($left, $top, $width, $height)
        $data = $bitmap.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
        $stride = $data.Stride
        $bytes = New-Object byte[] ($stride * $height)
        [System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
        $bitmap.UnlockBits($data)
        $count = 0
        $minX = [int]::MaxValue; $minY = [int]::MaxValue
        $maxX = -1; $maxY = -1
        for ($row = 0; $row -lt $height; $row++) {
            $rowBase = $row * $stride
            for ($column = 0; $column -lt $width; $column++) {
                $offset = $rowBase + $column * 4
                $b = [int]$bytes[$offset]
                $g = [int]$bytes[$offset + 1]
                $r = [int]$bytes[$offset + 2]
                if ([math]::Abs($r - $red) -le 2 -and [math]::Abs($g - $green) -le 2 -and [math]::Abs($b - $blue) -le 2) {
                    $count++
                    if ($column -lt $minX) { $minX = $column }
                    if ($column -gt $maxX) { $maxX = $column }
                    if ($row -lt $minY) { $minY = $row }
                    if ($row -gt $maxY) { $maxY = $row }
                }
            }
        }
        if ($count -eq 0) {
            return [pscustomobject]@{ count = 0; left = 0; top = 0; right = 0; bottom = 0 }
        }
        return [pscustomobject]@{
            count = $count
            left = $left + $minX
            top = $top + $minY
            right = $left + $maxX
            bottom = $top + $maxY
        }
    } finally {
        $bitmap.Dispose()
    }
}

$foreground = [Round34Native]::ForegroundTitle()
$locked = $foreground -match 'Lock Screen|Default Lock|Windows Default'
Write-CaptureLog ("SESSION foreground='{0}' locked={1}" -f $foreground, $locked)
Write-CaptureLog ("EXE={0} sha256={1}" -f $Exe, (Get-Sha256 $Exe))
Write-CaptureLog ("APPDATA root={0} cwd={1}" -f $AppRoot, (Get-Location).Path)
$head = ''
try { $head = (& git -C $Repo rev-parse HEAD).Trim() } catch { }
Write-CaptureLog ("HEAD={0}" -f $head)

$captures = New-Object System.Collections.Generic.List[object]

# ---------------------------------------------------------------------------------------------
# J-03 - precision: run at 20, change the control to 1000, run again
# ---------------------------------------------------------------------------------------------
$j03 = Join-Path $OutDir 'J-03.ls'
Write-Utf8 $j03 "setprecision(20)`nsqrt(2)`nsetprecision(1000)`nsqrt(2)`n"
Add-Check 'J-03 fixture' ((Get-Item -LiteralPath $j03).Length -gt 0) ("{0} bytes sha256={1}" -f (Get-Item -LiteralPath $j03).Length, (Get-Sha256 $j03))
$app = Start-Journey 'J-03' $j03
$app.process.Refresh()
Add-Check 'J-03 launch title names the script' ($app.process.MainWindowTitle.Contains('J-03.ls')) $app.process.MainWindowTitle
$initial = @(Get-UiaLines $app.process.Id)
$initialStatus = @($initial | Where-Object { $_.Contains('J-03.ls  |') })
Add-Check 'J-03 starts at the default 1000-digit precision' (($initialStatus -join ' ') -match 'precision 1000') ($initialStatus -join ' ; ')

Set-Precision $app.process.Id 20
$set20 = @(Get-UiaLines $app.process.Id)
$statusSet20 = @($set20 | Where-Object { $_.Contains('J-03.ls  |') -and $_.Contains('precision 20') })
$digitsSet20 = @($set20 | Where-Object { $_.Contains('20 digits') })
Add-Check 'J-03 precision control changed 1000 -> 20' ($statusSet20.Count -ge 1 -and $digitsSet20.Count -ge 1) (($statusSet20 + $digitsSet20) -join ' ; ')

Invoke-UiaButton $app.process.Id 'Run'
$null = Wait-Uia $app.process.Id 'run Finished' 90
Start-Sleep -Milliseconds 500
$uia20 = @(Save-Uia $app.process.Id 'J-03-precision-20')
$captures.Add((Save-Capture $app.process.Id 'J-03-precision-20')) | Out-Null

$status20 = @($uia20 | Where-Object { $_.Contains('J-03.ls  |') -and $_.Contains('precision 20') -and $_.Contains('run Finished') })
Add-Check 'J-03 run 1 status strip' ($status20.Count -ge 1) ($status20 -join ' ; ')
$start20 = @($uia20 | Where-Object { $_.Contains('[info] Run started: 4 statements') -and $_.Contains('precision 20') })
Add-Check 'J-03 run 1 log names the visible precision' ($start20.Count -ge 1) ($start20 -join ' ; ')
$run1 = Get-Last-RunLines $uia20
$results1 = @(Get-ResultValues $run1)
$at1000List = @($results1 | Where-Object { (Get-FractionalDigits $_) -eq 1000 })
$at1000 = if ($at1000List.Count -gt 0) { $at1000List[0] } else { '' }
Add-Check 'J-03 run 1 final result honours the script setprecision(1000)' ((Get-FractionalDigits $at1000) -eq 1000) ("digits={0} sha256={1} prefix={2}" -f (Get-FractionalDigits $at1000), (Get-Sha256Text $at1000), $(if ($at1000.Length -gt 24) { $at1000.Substring(0, 24) } else { $at1000 }))
Add-Check 'J-03 run 1 1000-digit value is the REF-digits value' ((Get-Sha256Text $at1000) -eq '92fa8c84b033aaf1a67722abccd41dab07aa693b6d64398b6baaafa47636b63c') ("sha256={0}" -f (Get-Sha256Text $at1000))
$varSlice1 = Get-Slice $uia20 'VARIABLES ' 'FUNCTIONS '
$varNames1 = @($varSlice1 | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
Add-Check 'J-03 variables table carries the 1000-digit answer' ($varNames1 -contains '_' -and $varNames1 -contains 'Real' -and @($varNames1 | Where-Object { $_.StartsWith('1.4142135') }).Count -ge 1) ("rows={0}" -f (($varNames1 | Select-Object -First 8) -join ', '))

Set-Precision $app.process.Id 1000
$set1000 = @(Get-UiaLines $app.process.Id)
$statusSet1000 = @($set1000 | Where-Object { $_.Contains('J-03.ls  |') -and $_.Contains('precision 1000') })
$digitsSet1000 = @($set1000 | Where-Object { $_.Contains('1000 digits') })
Add-Check 'J-03 precision control changed 20 -> 1000' ($statusSet1000.Count -ge 1 -and $digitsSet1000.Count -ge 1) (($statusSet1000 + $digitsSet1000) -join ' ; ')

Invoke-UiaButton $app.process.Id 'Run'
$null = Wait-Uia $app.process.Id 'run Finished' 90
Start-Sleep -Milliseconds 500
$uia1000 = @(Save-Uia $app.process.Id 'J-03')
$captures.Add((Save-Capture $app.process.Id 'J-03')) | Out-Null
$status1000 = @($uia1000 | Where-Object { $_.Contains('J-03.ls  |') -and $_.Contains('precision 1000') -and $_.Contains('run Finished') })
Add-Check 'J-03 run 2 status strip after the precision change' ($status1000.Count -ge 1) ($status1000 -join ' ; ')
$run2Queued = @($uia1000 | Where-Object { $_.Contains('[info] Run queued (revision 1, run 2).') })
Add-Check 'J-03 run 2 is a fresh run of the same document' ($run2Queued.Count -ge 1) ($run2Queued -join ' ; ')
$start1000 = @($uia1000 | Where-Object { $_.Contains('[info] Run started: 4 statements') -and $_.Contains('precision 1000') })
Add-Check 'J-03 run 2 log names precision 1000' ($start1000.Count -ge 1) ($start1000 -join ' ; ')
$run2 = Get-Last-RunLines $uia1000
$results2 = @(Get-ResultValues $run2)
$at1000b = @($results2 | Where-Object { (Get-FractionalDigits $_) -eq 1000 })
$at1000bValue = if ($at1000b.Count -gt 0) { $at1000b[0] } else { '' }
Add-Check 'J-03 run 2 re-rendered 1000 fractional digits' ((Get-FractionalDigits $at1000bValue) -eq 1000) ("digits={0} sha256={1}" -f (Get-FractionalDigits $at1000bValue), (Get-Sha256Text $at1000bValue))
Add-Check 'J-03 run 2 1000-digit value is the REF-digits value' ((Get-Sha256Text $at1000bValue) -eq '92fa8c84b033aaf1a67722abccd41dab07aa693b6d64398b6baaafa47636b63c') ("sha256={0}" -f (Get-Sha256Text $at1000bValue))
Stop-Journey $app
Start-Sleep -Milliseconds 500

# The window's Logs pane publishes only the run's final result (LogKind::Result), so the
# 20-place intermediate value inside the four-statement script is not a pane row. The first half
# of the same journey input is run as its own document to show the 19-digit display the product
# paints at precision 20; the full four-statement run above proves the 1000-digit half.
$j03b = Join-Path $OutDir 'J-03-20.ls'
Write-Utf8 $j03b "setprecision(20)`nsqrt(2)`n"
Add-Check 'J-03 precision-20 fixture' ((Get-Item -LiteralPath $j03b).Length -gt 0) ("{0} bytes sha256={1}" -f (Get-Item -LiteralPath $j03b).Length, (Get-Sha256 $j03b))
$app = Start-Journey 'J-03-20' $j03b
Set-Precision $app.process.Id 20
Invoke-UiaButton $app.process.Id 'Run'
$null = Wait-Uia $app.process.Id 'run Finished' 60
Start-Sleep -Milliseconds 500
$uia20b = @(Save-Uia $app.process.Id 'J-03-20-digits')
$captures.Add((Save-Capture $app.process.Id 'J-03-20-digits')) | Out-Null
$run20b = Get-Last-RunLines $uia20b
$results20b = @(Get-ResultValues $run20b)
$at20b = @($results20b | Where-Object { $_ -eq '1.4142135623730950488' })
$at20bValue = if ($at20b.Count -gt 0) { $at20b[0] } else { '' }
Add-Check 'J-03 sqrt(2) at precision 20 displays 19 fractional digits' ($at20b.Count -ge 1 -and (Get-FractionalDigits $at20bValue) -eq 19) ("value={0} fractional_digits={1}" -f $at20bValue, (Get-FractionalDigits $at20bValue))
$varSlice20b = Get-Slice $uia20b 'VARIABLES ' 'FUNCTIONS '
$varNames20b = @($varSlice20b | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
Add-Check 'J-03 variables table carries the 19-digit answer at precision 20' ($varNames20b -contains '_' -and @($varNames20b | Where-Object { $_.StartsWith('1.4142135623') }).Count -ge 1) ("rows={0}" -f (($varNames20b | Select-Object -First 8) -join ', '))
Stop-Journey $app
Start-Sleep -Milliseconds 500

# ---------------------------------------------------------------------------------------------
# J-10 - Clear Variables: y removed, user function f retained
# ---------------------------------------------------------------------------------------------
$j10 = Join-Path $OutDir 'J-10.ls'
Write-Utf8 $j10 "func f(x) = x + 1`ny = 41`nf(y)`n"
Add-Check 'J-10 fixture' ((Get-Item -LiteralPath $j10).Length -gt 0) ("{0} bytes sha256={1}" -f (Get-Item -LiteralPath $j10).Length, (Get-Sha256 $j10))
$app = Start-Journey 'J-10' $j10
$app.process.Refresh()
Add-Check 'J-10 launch title names the script' ($app.process.MainWindowTitle.Contains('J-10.ls')) $app.process.MainWindowTitle
Invoke-UiaButton $app.process.Id 'Run'
$null = Wait-Uia $app.process.Id 'run Finished' 60
Start-Sleep -Milliseconds 500
$uiaBefore = @(Save-Uia $app.process.Id 'J-10-before')
$captures.Add((Save-Capture $app.process.Id 'J-10-before')) | Out-Null
$varBefore = Get-Slice $uiaBefore 'VARIABLES ' 'FUNCTIONS '
$varNamesBefore = @($varBefore | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
$funcBefore = Get-Slice $uiaBefore 'FUNCTIONS ' 'RESULT'
$funcNamesBefore = @($funcBefore | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
Add-Check 'J-10 run defines y=41' ($varNamesBefore -contains 'y' -and $varNamesBefore -contains '41') ("variable rows={0}" -f (($varNamesBefore | Select-Object -First 8) -join ', '))
Add-Check 'J-10 run defines the user function f' ($funcNamesBefore -contains 'f' -and $funcNamesBefore -contains 'user') ("function rows={0}" -f (($funcNamesBefore | Select-Object -First 8) -join ', '))
$resultBefore = @($uiaBefore | Where-Object { $_.Contains('[result] = 42') })
Add-Check 'J-10 run result is 42' ($resultBefore.Count -ge 1) ($resultBefore -join ' ; ')

Invoke-UiaButton $app.process.Id 'Clear Variables'
$null = Wait-Uia $app.process.Id 'Variables cleared' 30
Start-Sleep -Milliseconds 600
$uiaAfter = @(Save-Uia $app.process.Id 'J-10')
$captures.Add((Save-Capture $app.process.Id 'J-10')) | Out-Null
$varAfter = Get-Slice $uiaAfter 'VARIABLES ' 'FUNCTIONS '
$varNamesAfter = @($varAfter | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
$varHeader = @($uiaAfter | Where-Object { $_ -match 'VARIABLES . 0' })
$funcAfter = Get-Slice $uiaAfter 'FUNCTIONS ' 'RESULT'
$funcNamesAfter = @($funcAfter | ForEach-Object { Get-UiaName $_ } | Where-Object { $_ })
$clearLog = @($uiaAfter | Where-Object { $_.Contains('Variables cleared: 2 removed;') -and $_.Contains('functions retained.') })
$funcCountBefore = -1
$funcCountAfter = -1
$beforeHeader = @($uiaBefore | Where-Object { $_ -match 'FUNCTIONS . ([0-9]+)' })
$afterHeader = @($uiaAfter | Where-Object { $_ -match 'FUNCTIONS . ([0-9]+)' })
if ($beforeHeader.Count -ge 1) { $beforeName = Get-UiaName $beforeHeader[0]; if ($beforeName -match '([0-9]+)\s*$') { $funcCountBefore = [int]$Matches[1] } }
if ($afterHeader.Count -ge 1) { $afterName = Get-UiaName $afterHeader[0]; if ($afterName -match '([0-9]+)\s*$') { $funcCountAfter = [int]$Matches[1] } }
Add-Check 'J-10 variables table is empty after Clear Variables' ($varHeader.Count -ge 1 -and -not ($varNamesAfter -contains 'y')) ("header={0} rows={1}" -f ($varHeader -join ' ; '), (($varNamesAfter | Select-Object -First 6) -join ', '))
Add-Check 'J-10 the variable y is gone' (-not ($varNamesAfter -contains 'y')) (($varNamesAfter | Select-Object -First 10) -join ', ')
Add-Check 'J-10 user function f is retained' ($funcNamesAfter -contains 'f' -and $funcNamesAfter -contains 'user') ("function rows={0}" -f (($funcNamesAfter | Select-Object -First 8) -join ', '))
Add-Check 'J-10 functions table count is unchanged' ($funcCountBefore -ge 0 -and $funcCountBefore -eq $funcCountAfter) ("before={0} after={1}" -f $funcCountBefore, $funcCountAfter)
Add-Check 'J-10 clear is logged' ($clearLog.Count -ge 1) ($clearLog -join ' ; ')
Stop-Journey $app
Start-Sleep -Milliseconds 500

# ---------------------------------------------------------------------------------------------
# J-11 - completion: type sq, capture the sqrt entry, accept it, save the inserted text
# ---------------------------------------------------------------------------------------------
$j11 = Join-Path $OutDir 'J-11.ls'
Write-Utf8 $j11 ''
Add-Check 'J-11 fixture is empty' ((Get-Item -LiteralPath $j11).Length -eq 0) ("{0} bytes" -f (Get-Item -LiteralPath $j11).Length)
$app = Start-Journey 'J-11' $j11
$app.process.Refresh()
Add-Check 'J-11 launch title names the script' ($app.process.MainWindowTitle.Contains('J-11.ls')) $app.process.MainWindowTitle
$typingMode = Type-Into-Editor $app 'sq'
Write-CaptureLog ("J-11 typing mode={0}" -f $typingMode)
$null = Wait-Uia $app.process.Id 'Completion: sq -> sqrt' 15
Start-Sleep -Milliseconds 500
$uiaPopup = @(Save-Uia $app.process.Id 'J-11-completion')
$captures.Add((Save-Capture $app.process.Id 'J-11-completion')) | Out-Null
$completion = @($uiaPopup | Where-Object { $_.Contains('Completion: sq -> sqrt(x) [builtin]') })
$editorTyped = @($uiaPopup | Where-Object { $_.Contains('Script editor, 1 lines, caret line 1 column 3') })
Add-Check 'J-11 completion popup offers sqrt with its signature' ($completion.Count -ge 1) ($completion -join ' ; ')
Add-Check 'J-11 editor caret is after the typed sq' ($editorTyped.Count -ge 1) ($editorTyped -join ' ; ')
$completionName = ''
if ($completion.Count -ge 1) { $completionName = Get-UiaName $completion[0] }
Add-Check 'J-11 popup names the selected entry and the match count' ($completionName -match '^Completion: sq -> sqrt\(x\) \[builtin\] [0-9]+/[0-9]+$') $completionName

Post-Key $app.hwnd ([uint32]0x0D) 0
$null = Wait-Uia $app.process.Id 'caret line 1 column 5' 15
Start-Sleep -Milliseconds 400
$uiaAccepted = @(Save-Uia $app.process.Id 'J-11-accepted')
$captures.Add((Save-Capture $app.process.Id 'J-11-accepted')) | Out-Null
$editorAccepted = @($uiaAccepted | Where-Object { $_.Contains('Script editor, 1 lines, caret line 1 column 5') })
$completionGone = @($uiaAccepted | Where-Object { $_.Contains('Completion:') })
Add-Check 'J-11 accepting sqrt moves the caret to column 5' ($editorAccepted.Count -ge 1) ($editorAccepted -join ' ; ')
Add-Check 'J-11 accepting the entry dismisses the popup' ($completionGone.Count -eq 0) ("completion nodes={0}" -f $completionGone.Count)
Invoke-UiaButton $app.process.Id 'Save'
Start-Sleep -Milliseconds 900
$saved = [System.IO.File]::ReadAllText($j11)
$inserted = Join-Path $OutDir 'J-11-inserted.txt'
Write-Utf8 $inserted $saved
Add-Check 'J-11 the accepted entry inserted sqrt into the document' ($saved -eq 'sqrt') ("file={0} bytes={1} sha256={2}" -f $inserted, (Get-Item -LiteralPath $inserted).Length, (Get-Sha256 $inserted))
Stop-Journey $app
Start-Sleep -Milliseconds 500

# ---------------------------------------------------------------------------------------------
# J-12 - error navigation: astral character before the failing column on line 2
# ---------------------------------------------------------------------------------------------
$emoji = [char]::ConvertFromUtf32(0x1F600)
$j12 = Join-Path $OutDir 'J-12.ls'
Write-Utf8 $j12 ("a = 1`ns = `"$emoji`"; 1 / 0`nb = 2`n")
Add-Check 'J-12 fixture' ((Get-Item -LiteralPath $j12).Length -gt 0) ("{0} bytes sha256={1}" -f (Get-Item -LiteralPath $j12).Length, (Get-Sha256 $j12))
$app = Start-Journey 'J-12' $j12
$app.process.Refresh()
Add-Check 'J-12 launch title names the script' ($app.process.MainWindowTitle.Contains('J-12.ls')) $app.process.MainWindowTitle
Invoke-UiaButton $app.process.Id 'Run'
$null = Wait-Uia $app.process.Id 'run Failed' 60
Start-Sleep -Milliseconds 600
$uiaFailed = @(Save-Uia $app.process.Id 'J-12')
$failedCapture = Save-Capture $app.process.Id 'J-12'
$captures.Add($failedCapture) | Out-Null
$diagHeader = @($uiaFailed | Where-Object { $_.Contains('Diagnostic Ln 2, Col 11:') })
$diagLog = @($uiaFailed | Where-Object { $_.Contains('[diagnostic] Diagnostic: line 2, column 11: Cannot divide by zero.') })
$statusFailed = @($uiaFailed | Where-Object { $_.Contains('J-12.ls  |') -and $_.Contains('run Failed') })
$editorFailed = @($uiaFailed | Where-Object { $_.Contains('Script editor, 4 lines, caret line 1 column 1') })
Add-Check 'J-12 failed run status' ($statusFailed.Count -ge 1) ($statusFailed -join ' ; ')
Add-Check 'J-12 diagnostic header names line 2 column 11' ($diagHeader.Count -ge 1) ($diagHeader -join ' ; ')
Add-Check 'J-12 diagnostic log line names line 2 column 11' ($diagLog.Count -ge 1) ($diagLog -join ' ; ')
Add-Check 'J-12 editor keeps the three-line document' ($editorFailed.Count -ge 1) ($editorFailed -join ' ; ')
$editorElement = Get-EditorElement $app.process.Id
if ($null -eq $editorElement) { throw 'J-12: no script editor element' }
$editorRect = $editorElement.Current.BoundingRectangle
$visibleRowsHit = @($uiaFailed | Where-Object { $_.Contains('visible rows') })
$visibleRows = 38
if ($visibleRowsHit.Count -ge 1) {
    $visibleName = Get-UiaName $visibleRowsHit[0]
    if ($visibleName -match '\/\s*([0-9]+)\s+visible rows') { $visibleRows = [int]$Matches[1] }
}
$rowHeight = $editorRect.Height / [double]$visibleRows
$underline = Find-ColourBox (Join-Path $OutDir 'J-12.png') ([int][math]::Floor($editorRect.X)) ([int][math]::Floor($editorRect.Y)) ([int][math]::Ceiling($editorRect.X + $editorRect.Width)) ([int][math]::Ceiling($editorRect.Y + $editorRect.Height)) 255 123 114
$marker = Find-ColourBox (Join-Path $OutDir 'J-12.png') 0 ([int][math]::Floor($editorRect.Y)) ([int][math]::Ceiling($editorRect.X) + 24) ([int][math]::Ceiling($editorRect.Y + $editorRect.Height)) 224 75 69
$markerCentre = 0.0
if ($marker.count -gt 0) { $markerCentre = ($marker.top + $marker.bottom) / 2.0 }
$expectedRow2 = $editorRect.Y + $rowHeight * 1.5
$rowDelta = if ($marker.count -gt 0) { [math]::Abs($markerCentre - $expectedRow2) } else { 9999.0 }
Add-Check 'J-12 diagnostic underline is painted' ($underline.count -ge 1) ("pixels={0} box={1},{2}..{3},{4}" -f $underline.count, $underline.left, $underline.top, $underline.right, $underline.bottom)
Add-Check 'J-12 diagnostic gutter marker is painted' ($marker.count -ge 1) ("pixels={0} box={1},{2}..{3},{4}" -f $marker.count, $marker.left, $marker.top, $marker.right, $marker.bottom)
Add-Check 'J-12 underline and gutter marker share the line' ($marker.count -gt 0 -and $underline.count -gt 0 -and [math]::Abs((($underline.top + $underline.bottom) / 2.0) - $markerCentre) -lt ([math]::Max(4.0, $rowHeight / 2.0))) ("underline_centre={0:F1} marker_centre={1:F1} row_h={2:F1}" -f (($underline.top + $underline.bottom) / 2.0), $markerCentre, $rowHeight)
Add-Check 'J-12 gutter marker is on the second line' ($marker.count -gt 0 -and $rowDelta -lt ($rowHeight * 0.75)) ("expected_y={0:F1} marker_centre={1:F1} delta={2:F1} row_h={3:F1}" -f $expectedRow2, $markerCentre, $rowDelta, $rowHeight)
Add-Check 'J-12 gutter marker is in the gutter, left of the underline' ($marker.count -gt 0 -and $marker.left -lt $underline.left) ("marker_left={0} underline_left={1}" -f $marker.left, $underline.left)
Add-Check 'J-12 underline starts after the gutter' ($underline.count -gt 0 -and $underline.left -gt ($editorRect.X + 20.0)) ("editor_left={0:F1} underline_left={1}" -f $editorRect.X, $underline.left)

Press-F8 $app
$null = Wait-Uia $app.process.Id 'caret line 2 column 11' 15
Start-Sleep -Milliseconds 400
$uiaJump = @(Save-Uia $app.process.Id 'J-12-jump')
$captures.Add((Save-Capture $app.process.Id 'J-12-jump')) | Out-Null
$statusJump = @($uiaJump | Where-Object { $_.Contains('J-12.ls  |') -and $_.Contains('Ln 2, Col 11') })
$editorJump = @($uiaJump | Where-Object { $_.Contains('Script editor, 4 lines, caret line 2 column 11') })
Add-Check 'J-12 F8 jumps to line 2 column 11' ($editorJump.Count -ge 1 -and $statusJump.Count -ge 1) (($editorJump + $statusJump) -join ' ; ')
Stop-Journey $app
Start-Sleep -Milliseconds 500

# ---------------------------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------------------------
$passed = @($script:Checks | Where-Object { $_.ok }).Count
$failed = @($script:Checks | Where-Object { -not $_.ok }).Count
Write-CaptureLog ("SUMMARY passed={0} failed={1}" -f $passed, $failed)

$captureArray = @()
foreach ($capture in $captures) {
    $captureArray += [pscustomobject]@{
        name = [string]$capture.name
        file = [string]$capture.file
        width = [int]$capture.width
        height = [int]$capture.height
        bytes = [int]$capture.bytes
        sha256 = [string]$capture.sha256
    }
}
$checkArray = @()
foreach ($check in $script:Checks) {
    $checkArray += [pscustomobject]@{
        name = [string]$check.name
        ok = [bool]$check.ok
        detail = [string]$check.detail
    }
}
$proof = [ordered]@{
    round = 34
    head = [string]$head
    session = $(if ($locked) { 'locked' } else { 'unlocked-or-unknown' })
    foreground = [string]$foreground
    method = 'UIA Invoke + RangeValue + window-message keys + PrintWindow(PW_RENDERFULLCONTENT); real product window and document'
    exe = [string]$Exe
    exe_sha256 = [string](Get-Sha256 $Exe)
    appdata = [string]$AppRoot
    cwd = [string](Get-Location).Path
    captures = $captureArray
    uia = @('J-03-precision-20-uia.txt', 'J-03-uia.txt', 'J-03-20-digits-uia.txt', 'J-10-before-uia.txt', 'J-10-uia.txt', 'J-11-completion-uia.txt', 'J-11-accepted-uia.txt', 'J-12-uia.txt', 'J-12-jump-uia.txt')
    checks = $checkArray
    passed = [int]$passed
    failed = [int]$failed
}
try {
    $json = ConvertTo-Json -InputObject $proof -Depth 8
    $proofPath = Join-Path $OutDir 'window-proof.json'
    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($proofPath, $json, $encoding)
    Write-CaptureLog ("window-proof.json bytes={0}" -f (Get-Item -LiteralPath $proofPath).Length)
} catch {
    Write-CaptureLog ("PROOF FAIL {0}: {1}" -f $_.Exception.GetType().FullName, $_.Exception.Message)
    Write-CaptureLog ($_.ScriptStackTrace)
    exit 1
}

if ($failed -gt 0) { exit 1 }
exit 0
