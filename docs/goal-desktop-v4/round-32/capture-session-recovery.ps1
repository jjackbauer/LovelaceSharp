<#
.SYNOPSIS
  Round-32 DOC-004 / DOC-005 evidence: session restore and the crash-recovery journal in the
  shipped workbench window.

.DESCRIPTION
  The console session may be LOCKED (the foreground window is the Windows lock screen), so this
  script uses only methods that do not need an interactive desktop:
    - the app is launched with a real .lv file path, which opens it from disk;
    - a window resize is a SetWindowPos call on the app's own HWND;
    - buttons are pressed with UI Automation Invoke (message-based, reaches a locked window);
    - text is entered by POSTING WM_KEYDOWN messages to the window (winit turns them into egui
      key/text events); injected hardware input cannot reach a locked session and is not used;
    - pixels come from PrintWindow with PW_RENDERFULLCONTENT into a bitmap of the window rect,
      which also renders a window that is not visible on the lock screen;
    - the UIA trees are dumped with System.Windows.Automation.

  The evidence sequence:
    1. fixture: recovery-fixture.lv, "a = 1\nb = 2\n". Its SHA-256 is recorded.
    2. launch 1 with the fixture path; capture the launched state (the tab and the file bar).
    3. place the caret at the end of line 2 and type " = 1"; the tab becomes dirty.
    4. wait past the one-second journal debounce; the recovery record appears under
       %APPDATA%/Lovelace/recovery, keyed by the document identity, with the buffer's exact
       bytes, the file path and the caret. The record is copied to record-after-crash.lvrec.
    5. simulate a crash: Stop-Process -Force, with no WM_CLOSE and no clean-exit path. The record
       must still be there afterwards.
    6. launch 2 with no arguments: the session restores the fixture's tab by reading the file
       again (the file is untouched) and the recovery prompt offers the journaled buffer.
       Capture recovery-prompt.png + recovery-prompt-uia.txt.
    7. UIA-invoke the prompt's Recover: the exact text comes back into the tab, the caret with it,
       nothing is written and nothing is run. Capture restored-buffer.png + restored-buffer-uia.txt.
       The record is copied to record-after-recovery.lvrec and its body must be identical to the
       pre-crash body.
    8. post WM_CLOSE: the recovered tab is dirty, so the unsaved-changes prompt answers first;
       UIA-invoke Discard. The clean exit must leave no recovery records behind.

  APPDATA is redirected to this round's appdata folder and the process working directory is this
  round's work folder, so the settings file, the recovery directory and the plot directory all re-
  solve below the round's own folder and never below the working directory (PLF-006).

  Windows PowerShell 5.1 compatible. ASCII-only source.

.NOTES
  Exit code 0 only when every check in the summary passed.
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
public static class Round32Native {
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

$Work = Join-Path $OutDir 'work'
if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Work | Out-Null
$AppData = Join-Path $OutDir 'appdata'
if (Test-Path -LiteralPath $AppData) { Remove-Item -LiteralPath $AppData -Recurse -Force }
New-Item -ItemType Directory -Force -Path $AppData | Out-Null
$CaptureLog = Join-Path $OutDir 'capture-run.log'
if (Test-Path -LiteralPath $CaptureLog) { Remove-Item -LiteralPath $CaptureLog -Force }
$env:APPDATA = $AppData
$env:TMP = Join-Path $Repo 'rust\target-desktop\tmp'
$env:TEMP = $env:TMP
New-Item -ItemType Directory -Force -Path $env:TMP | Out-Null
Set-Location -LiteralPath $Work
[System.IO.Directory]::SetCurrentDirectory($Work)

$RecoveryDir = Join-Path $AppData 'Lovelace\recovery'
$SettingsPath = Join-Path $AppData 'Lovelace\desktop.json'
$Fixture = Join-Path $OutDir 'recovery-fixture.lv'
$FixtureBytes = [System.Text.Encoding]::UTF8.GetBytes("a = 1`nb = 2`n")
[System.IO.File]::WriteAllBytes($Fixture, $FixtureBytes)
$ExpectedAfterEdit = "a = 1`nb = 2 = 1`n"
$ExpectedBytes = [System.Text.Encoding]::UTF8.GetBytes($ExpectedAfterEdit)

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

# Reads one recovery record: the ASCII header becomes a map, the body stays raw bytes.
function Read-RecordFile([string]$path) {
    $bytes = [System.IO.File]::ReadAllBytes($path)
    $separator = -1
    for ($i = 0; $i -lt $bytes.Length - 1; $i++) {
        if ($bytes[$i] -eq 10 -and $bytes[$i + 1] -eq 10) { $separator = $i; break }
    }
    if ($separator -lt 0) { throw "no header separator in $path" }
    $header = @{}
    $headerText = [System.Text.Encoding]::ASCII.GetString($bytes, 0, $separator)
    foreach ($line in ($headerText -split "`n")) {
        $colon = $line.IndexOf(':')
        if ($colon -gt 0) { $header[$line.Substring(0, $colon)] = $line.Substring($colon + 1).TrimStart(' ') }
    }
    $body = New-Object byte[] ($bytes.Length - $separator - 2)
    if ($body.Length -gt 0) { [Array]::Copy($bytes, $separator + 2, $body, 0, $body.Length) }
    return [pscustomobject]@{ header = $header; body = $body; bytes = $bytes; path = $path }
}

function Get-RecordFiles([string]$directory) {
    if (-not (Test-Path -LiteralPath $directory)) { return @() }
    return @(Get-ChildItem -LiteralPath $directory -Filter '*.lvrec' -File)
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
    $hwnd = [Round32Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    if ($hwnd -eq [IntPtr]::Zero) { throw "no app window for $name" }
    $rect = New-Object Round32Native+RECT
    if (-not [Round32Native]::GetWindowRect($hwnd, [ref]$rect)) { throw "GetWindowRect failed for $name" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "window has no size for $name ($width x $height)" }
    $bitmap = New-Object System.Drawing.Bitmap ($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Round32Native]::PrintWindow($hwnd, $hdc, 2)
    $graphics.ReleaseHdc($hdc)
    $graphics.Dispose()
    $path = Join-Path $OutDir "$name.png"
    $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $size = (Get-Item -LiteralPath $path).Length
    if (-not $ok) { throw "PrintWindow returned false for $name" }
    Write-CaptureLog ("CAPTURE {0}: PrintWindow(PW_RENDERFULLCONTENT) hwnd={1} rect={2}x{3} -> {4} ({5} bytes)" -f
        $name, $hwnd, $width, $height, $path, $size)
    return [pscustomobject]@{ name = $name; path = $path; width = $width; height = $height; bytes = $size }
}

function Get-Buttons([int]$processId) {
    $root = Get-AppRoot $processId
    $condition = New-Object System.Windows.Automation.PropertyCondition(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Button)
    return @($root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $condition))
}

# The status strip reads "name | rev N | precision N | Ln R, Col C | run State"; parsing it back
# is how the driver verifies posted caret keys really moved the caret before typing.
function Get-CaretPosition([int]$processId) {
    $root = Get-AppRoot $processId
    if (-not $root) { return $null }
    $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $all) {
        if ($element.Current.Name -match 'Ln (\d+), Col (\d+)') {
            return [pscustomobject]@{ line = [int]$Matches[1]; column = [int]$Matches[2] }
        }
    }
    return $null
}

function Invoke-Button([int]$processId, [string]$name, [int]$index = 0) {
    $found = @(Get-Buttons $processId | Where-Object { $_.Current.Name -eq $name })
    if ($found.Count -le $index) {
        throw ("no button '{0}' at index {1} (found {2})" -f $name, $index, $found.Count)
    }
    $target = $found[$index]
    $pattern = $target.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
    Write-CaptureLog ("UIA INVOKE button '{0}' index {1} of {2} rect={3}" -f
        $name, $index, $found.Count, $target.Current.BoundingRectangle)
}

# Only WM_KEYDOWN is posted: winit resolves the typed character from the virtual key with the
# current keyboard layout (a following WM_CHAR would deliver the same character twice).
function Send-Key([int]$targetProcessId, [int]$virtualKey, [int]$repeat = 1) {
    $target = [Round32Native]::WindowOf([uint32]$targetProcessId, 'Lovelace Desktop')
    if ($target -eq [IntPtr]::Zero) { throw 'Send-Key: no app window' }
    for ($i = 0; $i -lt $repeat; $i++) {
        [void][Round32Native]::PostMessage($target, 0x0100, [IntPtr]$virtualKey, [IntPtr]0x001E0001)
        Start-Sleep -Milliseconds 40
    }
}

function Wait-ForWindow([int]$processId, [int]$seconds = 30) {
    $deadline = (Get-Date).AddSeconds($seconds)
    $hwnd = [IntPtr]::Zero
    do {
        Start-Sleep -Milliseconds 250
        $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if (-not $process) { throw 'the app exited during startup' }
        $hwnd = [Round32Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    } while ($hwnd -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline)
    if ($hwnd -eq [IntPtr]::Zero) { throw 'the workbench window never appeared' }
    [void][Round32Native]::SetWindowPos($hwnd, [IntPtr]::Zero, 10, 10, 1600, 1000, 0x0014)
    Start-Sleep -Milliseconds 900
    return $hwnd
}

function Get-WindowTitle([IntPtr]$hwnd) {
    $title = New-Object System.Text.StringBuilder 512
    [void][Round32Native]::GetWindowText($hwnd, $title, 512)
    return $title.ToString()
}

# ---------------------------------------------------------------------------------------------
# 0. Session state and fixture
# ---------------------------------------------------------------------------------------------
$foreground = [Round32Native]::ForegroundTitle()
$locked = $foreground -match 'Lock Screen|Default Lock|Windows Default'
Write-CaptureLog ("SESSION foreground='{0}' locked={1}" -f $foreground, $locked)
$fixtureHash = Get-Sha256 $Fixture
Write-CaptureLog ("FIXTURE {0}: {1} bytes sha256={2}" -f $Fixture, $FixtureBytes.Length, $fixtureHash)
Add-Check 'fixture-bytes' (Test-BytesEqual ([System.IO.File]::ReadAllBytes($Fixture)) $FixtureBytes) ("{0} bytes of LF text" -f $FixtureBytes.Length)

# ---------------------------------------------------------------------------------------------
# 1. Launch 1: the fixture opens from disk
# ---------------------------------------------------------------------------------------------
$process = Start-Process -FilePath $Exe -ArgumentList @("`"$Fixture`"") -PassThru
$hwnd = Wait-ForWindow $process.Id
$title = Get-WindowTitle $hwnd
Write-CaptureLog ("LAUNCH1 pid={0} hwnd={1} title='{2}' argv='{3}'" -f $process.Id, $hwnd, $title, $Fixture)
Add-Check 'launch-title-names-the-file' ($title.Contains('recovery-fixture.lv')) $title

Start-Sleep -Milliseconds 600
$launchedLines = Save-UiaDump $process.Id 'launched'
$launched = Save-Capture $process.Id 'launched'
$tabLine = @($launchedLines | Where-Object { $_ -match 'Button \| recovery-fixture\.lv' })
Add-Check 'launched-tab-shows-the-file' ($tabLine.Count -ge 1) ($tabLine -join ' ; ')
$pathLine = @($launchedLines | Where-Object { $_ -match [regex]::Escape($Fixture) })
Add-Check 'file-bar-shows-the-path' ($pathLine.Count -ge 1) ($pathLine -join ' ; ')
$sizeLine = @($launchedLines | Where-Object { $_ -match '3 lines / 12 bytes' })
Add-Check 'launched-buffer-is-the-file' ($sizeLine.Count -ge 1) ($sizeLine -join ' ; ')
$openLog = @($launchedLines | Where-Object { $_ -match 'Opened .*recovery-fixture\.lv' })
Add-Check 'open-is-logged' ($openLog.Count -ge 1) ($openLog -join ' ; ')
$settingsWritten = Test-Path -LiteralPath $SettingsPath
Add-Check 'session-file-written-at-launch' $settingsWritten $SettingsPath
if ($settingsWritten) {
    $settingsText = Get-Content -LiteralPath $SettingsPath -Raw
    Add-Check 'session-file-names-the-open-file' ($settingsText.Contains('recovery-fixture.lv')) 'desktop.json contains the open tab'
}

# ---------------------------------------------------------------------------------------------
# 2. Type an edit through posted keys: the tab becomes dirty
# ---------------------------------------------------------------------------------------------
for ($attempt = 0; $attempt -lt 8; $attempt++) {
    $caret = Get-CaretPosition $process.Id
    if ($caret -and $caret.line -ge 2) { break }
    Send-Key $process.Id 0x28 1      # VK_DOWN
    Start-Sleep -Milliseconds 250
}
Send-Key $process.Id 0x23 1          # VK_END
Start-Sleep -Milliseconds 250
$caretBefore = Get-CaretPosition $process.Id
Write-CaptureLog ("CARET before typing: line={0} column={1}" -f $caretBefore.line, $caretBefore.column)
Add-Check 'caret-reached-line-2' ($caretBefore -and $caretBefore.line -eq 2) ("Ln {0}, Col {1}" -f $caretBefore.line, $caretBefore.column)
Send-Key $process.Id 0x20 1          # VK_SPACE -> " "
Send-Key $process.Id 0xBB 1          # VK_OEM_PLUS -> "="
Send-Key $process.Id 0x20 1          # VK_SPACE -> " "
Send-Key $process.Id 0x31 1          # VK_1 -> "1"
$lastKeystroke = Get-Date
Start-Sleep -Milliseconds 800

$dirtyLines = Save-UiaDump $process.Id 'dirty'
$dirtyTab = @($dirtyLines | Where-Object { $_ -match 'Button \| recovery-fixture\.lv \*' })
Add-Check 'dirty-tab-shows-the-marker' ($dirtyTab.Count -ge 1) ($dirtyTab -join ' ; ')
$dirtySize = @($dirtyLines | Where-Object { $_ -match '3 lines / 16 bytes' })
Add-Check 'typed-text-reached-the-buffer' ($dirtySize.Count -ge 1) ($dirtySize -join ' ; ')
$fileAfterEdit = Get-Sha256 $Fixture
Add-Check 'typing-wrote-nothing' ($fileAfterEdit -eq $fixtureHash) ("sha256={0}" -f $fileAfterEdit)

# ---------------------------------------------------------------------------------------------
# 3. The journal: the record converges on the newest revision within the one-second window
# ---------------------------------------------------------------------------------------------
# The first keystroke is journaled at once (the leading edge); the trailing write happens when the
# one-second debounce elapses, driven by the frame the window scheduled. Poll until the record
# carries the final 16-byte buffer, then measure how long the flush took.
$deadline = (Get-Date).AddSeconds(10)
$record = $null
while ((Get-Date) -lt $deadline) {
    $records = @(Get-RecordFiles $RecoveryDir)
    if ($records.Count -ge 1) {
        $candidate = Read-RecordFile $records[0].FullName
        if ($candidate.header['bytes'] -eq '16') { $record = $candidate; break }
    }
    Start-Sleep -Milliseconds 100
}
$flushMs = ((Get-Date) - $lastKeystroke).TotalMilliseconds
$records = @(Get-RecordFiles $RecoveryDir)
Add-Check 'journal-wrote-one-record' ($records.Count -eq 1 -and $null -ne $record) ("{0} record file(s) in {1}" -f $records.Count, $RecoveryDir)
Add-Check 'journal-flushed-the-final-revision' ($null -ne $record) ("newest revision on disk after {0:N0} ms (debounce 1000 ms)" -f $flushMs)
Add-Check 'journal-flush-is-bounded-by-the-window' ($flushMs -lt 3000) ("{0:N0} ms after the last keystroke" -f $flushMs)
if ($null -eq $record) { throw 'the journal never wrote the final revision' }
$recordPath = $record.path
Write-CaptureLog ("RECORD {0}: header={1}" -f $recordPath, (($record.header.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join '; '))
Add-Check 'record-is-keyed-by-the-document' ($recordPath -like ('*doc-{0}.lvrec' -f ($record.header['id']))) (Split-Path -Leaf $recordPath)
Add-Check 'record-names-the-original-path' ($record.header['path'] -eq $Fixture) ("path={0}" -f $record.header['path'])
Add-Check 'record-names-the-title' ($record.header['title'] -eq 'recovery-fixture.lv') ("title={0}" -f $record.header['title'])
Add-Check 'record-declares-the-buffer-size' ($record.header['bytes'] -eq '16') ("bytes={0}" -f $record.header['bytes'])
Add-Check 'record-body-is-byte-identical-to-the-buffer' (Test-BytesEqual $record.body $ExpectedBytes) ("body={0} bytes" -f $record.body.Length)
Add-Check 'record-carries-the-caret' ($record.header['caret'] -eq '1:9') ("caret={0}" -f $record.header['caret'])
Add-Check 'record-carries-the-mtime' ([bool]$record.header['mtime_unix_nanos']) ("mtime_unix_nanos={0}" -f $record.header['mtime_unix_nanos'])
$recordCopy = Join-Path $OutDir 'record-after-crash.lvrec'
Copy-Item -LiteralPath $recordPath -Destination $recordCopy -Force
$recordHashBeforeCrash = Get-Sha256 $recordPath

# ---------------------------------------------------------------------------------------------
# 4. Simulated crash: the process is killed, never closed
# ---------------------------------------------------------------------------------------------
Stop-Process -Id $process.Id -Force
Start-Sleep -Milliseconds 800
$stillThere = Test-Path -LiteralPath $recordPath
Add-Check 'forced-kill-leaves-the-record' $stillThere $recordPath
Add-Check 'forced-kill-leaves-it-unchanged' ((Get-Sha256 $recordPath) -eq $recordHashBeforeCrash) ("sha256={0}" -f (Get-Sha256 $recordPath))
$process.Refresh()
Add-Check 'process-was-killed-not-closed' ($process.HasExited) ("exited={0} exitcode={1}" -f $process.HasExited, $process.ExitCode)

# ---------------------------------------------------------------------------------------------
# 5. Relaunch: the session restores the tab and the recovery prompt offers the buffer
# ---------------------------------------------------------------------------------------------
$process2 = Start-Process -FilePath $Exe -PassThru
$hwnd2 = Wait-ForWindow $process2.Id
$title2 = Get-WindowTitle $hwnd2
Write-CaptureLog ("LAUNCH2 pid={0} hwnd={1} title='{2}'" -f $process2.Id, $hwnd2, $title2)
Add-Check 'relaunch-restored-the-file-tab' ($title2.Contains('recovery-fixture.lv')) $title2
Add-Check 'relaunch-restored-the-file-untouched' ((Get-Sha256 $Fixture) -eq $fixtureHash) ("sha256={0}" -f (Get-Sha256 $Fixture))

Start-Sleep -Milliseconds 800
$promptLines = Save-UiaDump $process2.Id 'recovery-prompt'
$promptCapture = Save-Capture $process2.Id 'recovery-prompt'
$promptHeading = @($promptLines | Where-Object { $_ -match 'Recover unsaved work' })
Add-Check 'prompt-has-its-heading' ($promptHeading.Count -ge 1) ($promptHeading -join ' ; ')
$promptBody = @($promptLines | Where-Object { $_ -match 'did not close cleanly' })
Add-Check 'prompt-explains-the-crash' ($promptBody.Count -ge 1) ($promptBody -join ' ; ')
$promptTitle = @($promptLines | Where-Object { $_ -match 'recovery-fixture\.lv' })
Add-Check 'prompt-lists-the-record-title' ($promptTitle.Count -ge 1) ($promptTitle -join ' ; ')
$promptPath = @($promptLines | Where-Object { $_ -match [regex]::Escape($Fixture) })
Add-Check 'prompt-reports-the-original-path' ($promptPath.Count -ge 1) ($promptPath -join ' ; ')
$promptBytes = @($promptLines | Where-Object { $_ -match '16 bytes' })
Add-Check 'prompt-reports-the-buffer-size' ($promptBytes.Count -ge 1) ($promptBytes -join ' ; ')
foreach ($answer in @('Recover', 'Discard')) {
    $nodes = @($promptLines | Where-Object { $_ -match ("Button \| {0} \|" -f $answer) })
    Add-Check ("prompt-offers-{0}" -f $answer.ToLower()) ($nodes.Count -ge 1) ("{0}: {1} nodes" -f $answer, $nodes.Count)
}

# ---------------------------------------------------------------------------------------------
# 6. Recover: the exact text comes back before any save, and the file is not written
# ---------------------------------------------------------------------------------------------
Invoke-Button $process2.Id 'Recover'
Start-Sleep -Milliseconds 1200
$restoredLines = Save-UiaDump $process2.Id 'restored-buffer'
$restoredCapture = Save-Capture $process2.Id 'restored-buffer'
$restoredTab = @($restoredLines | Where-Object { $_ -match 'Button \| recovery-fixture\.lv \*' })
Add-Check 'recovered-tab-is-dirty' ($restoredTab.Count -ge 1) ($restoredTab -join ' ; ')
$restoredSize = @($restoredLines | Where-Object { $_ -match '3 lines / 16 bytes' })
Add-Check 'recovered-buffer-has-the-exact-size' ($restoredSize.Count -ge 1) ($restoredSize -join ' ; ')
$recoveredLog = @($restoredLines | Where-Object { $_ -match 'Recovered recovery-fixture\.lv from the recovery journal' })
Add-Check 'recovery-is-logged' ($recoveredLog.Count -ge 1) ($recoveredLog -join ' ; ')
$promptGone = @($restoredLines | Where-Object { $_ -match 'Recover unsaved work' })
Add-Check 'prompt-closed-after-the-answer' ($promptGone.Count -eq 0) ("prompt nodes: {0}" -f $promptGone.Count)
$fileAfterRecover = Get-Sha256 $Fixture
Add-Check 'recovery-wrote-nothing' ($fileAfterRecover -eq $fixtureHash) ("sha256={0}" -f $fileAfterRecover)
$recordsAfterRecover = @(Get-RecordFiles $RecoveryDir)
Add-Check 'recovered-buffer-is-still-journaled' ($recordsAfterRecover.Count -eq 1) ("{0} record(s)" -f $recordsAfterRecover.Count)
if ($recordsAfterRecover.Count -eq 1) {
    $recordAfter = Read-RecordFile $recordsAfterRecover[0].FullName
    $recordCopy2 = Join-Path $OutDir 'record-after-recovery.lvrec'
    Copy-Item -LiteralPath $recordsAfterRecover[0].FullName -Destination $recordCopy2 -Force
    Add-Check 'record-body-identical-after-recovery' (Test-BytesEqual $recordAfter.body $record.body) ("body={0} bytes" -f $recordAfter.body.Length)
    Add-Check 'record-still-names-the-path' ($recordAfter.header['path'] -eq $Fixture) ("path={0}" -f $recordAfter.header['path'])
}

# ---------------------------------------------------------------------------------------------
# 7. Clean exit: the unsaved prompt answers, and the clean exit deletes the record
# ---------------------------------------------------------------------------------------------
[void][Round32Native]::PostMessage($hwnd2, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)   # WM_CLOSE
Start-Sleep -Milliseconds 900
$closePromptLines = Save-UiaDump $process2.Id 'close-prompt'
$closeQuestion = @($closePromptLines | Where-Object { $_ -match 'Save changes to recovery-fixture\.lv before closing' })
Add-Check 'clean-exit-asks-about-the-recovered-buffer' ($closeQuestion.Count -ge 1) ($closeQuestion -join ' ; ')
Invoke-Button $process2.Id 'Discard'
$closed = $process2.WaitForExit(10000)
if (-not $closed) { Stop-Process -Id $process2.Id -Force }
Add-Check 'clean-exit-quits' $closed ("exited={0}" -f $closed)
Start-Sleep -Milliseconds 500
$recordsAfterExit = @(Get-RecordFiles $RecoveryDir)
Add-Check 'clean-exit-deletes-its-records' ($recordsAfterExit.Count -eq 0) ("{0} record(s) left" -f $recordsAfterExit.Count)
$settingsAfter = Get-Content -LiteralPath $SettingsPath -Raw
Add-Check 'clean-exit-wrote-the-session' ($settingsAfter.Contains('"session"') -and $settingsAfter.Contains('recovery-fixture.lv')) 'desktop.json has a session naming the tab'

# ---------------------------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------------------------
$failed = @($script:Checks | Where-Object { -not $_.ok })
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
$json = New-Object System.Text.StringBuilder
[void]$json.Append("{`n")
[void]$json.Append("  ""round"": 32,`n")
[void]$json.Append("  ""goal"": ""DOC-004 / DOC-005"",`n")
[void]$json.Append("  ""method"": ""UIA Invoke + PrintWindow(PW_RENDERFULLCONTENT) on a $(if ($locked) { 'locked' } else { 'possibly-unlocked' }) session; keyboard via posted WM_KEYDOWN messages (no hardware injection)"",`n")
[void]$json.Append(("  ""session"": ""{0}"",`n" -f $(if ($locked) { 'locked' } else { 'unlocked-or-unknown' })))
[void]$json.Append(("  ""foreground"": {0},`n" -f (ConvertTo-JsonString $foreground)))
[void]$json.Append(("  ""fixture"": {{ ""path"": {0}, ""bytes"": {1}, ""sha256"": {2} }},`n" -f (ConvertTo-JsonString $Fixture), $FixtureBytes.Length, (ConvertTo-JsonString $fixtureHash)))
[void]$json.Append(("  ""record"": {{ ""path"": {0}, ""id"": {1}, ""bytes"": {2}, ""header"": {3} }},`n" -f (ConvertTo-JsonString $recordPath), $record.header['id'], $record.body.Length, (ConvertTo-JsonString (($record.header.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Name)=$($_.Value)" }) -join '; '))))
[void]$json.Append("  ""captures"": [`n")
$captureList = @($launched, $promptCapture, $restoredCapture)
for ($i = 0; $i -lt $captureList.Count; $i++) {
    $capture = $captureList[$i]
    $comma = if ($i -lt $captureList.Count - 1) { ',' } else { '' }
    [void]$json.Append(("    {{ ""name"": {0}, ""path"": {1}, ""width"": {2}, ""height"": {3}, ""bytes"": {4} }}{5}`n" -f
        (ConvertTo-JsonString $capture.name), (ConvertTo-JsonString $capture.path), $capture.width, $capture.height, $capture.bytes, $comma))
}
[void]$json.Append("  ],`n")
[void]$json.Append("  ""checks"": [`n")
for ($i = 0; $i -lt $script:Checks.Count; $i++) {
    $check = $script:Checks[$i]
    $comma = if ($i -lt $script:Checks.Count - 1) { ',' } else { '' }
    $okText = if ($check.ok) { 'true' } else { 'false' }
    [void]$json.Append(("    {{ ""name"": {0}, ""ok"": {1}, ""detail"": {2} }}{3}`n" -f
        (ConvertTo-JsonString $check.name), $okText, (ConvertTo-JsonString $check.detail), $comma))
}
[void]$json.Append("  ],`n")
[void]$json.Append(("  ""passed"": {0},`n" -f ($script:Checks.Count - $failed.Count)))
[void]$json.Append(("  ""failed"": {0}`n" -f $failed.Count))
[void]$json.Append("}")
$summaryPath = Join-Path $OutDir 'window-proof.json'
[System.IO.File]::WriteAllText($summaryPath, $json.ToString(), (New-Object System.Text.UTF8Encoding($false)))
Write-CaptureLog ("SUMMARY checks={0} failed={1} -> {2}" -f $script:Checks.Count, $failed.Count, $summaryPath)
if ($failed.Count -gt 0) {
    foreach ($check in $failed) { Write-CaptureLog ("FAILED {0}: {1}" -f $check.name, $check.detail) }
    exit 1
}
exit 0
