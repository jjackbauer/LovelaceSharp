<#
.SYNOPSIS
  Round-11 GUI gesture driver for the DEF-001 Backspace-join defect and the DEF-002 Up-arrow
  focus defect.

.DESCRIPTION
  Uses the same primitives as tools/gui-harness.ps1: launch the shipped exe with its stderr
  redirected, restore and focus the native window, drive UI Automation, send real SendKeys,
  capture the window as PNG and dump the UIA tree. The harness `scenario` action types one
  script line and then runs its fixed run/cancel gesture; this driver replays the two-line
  edit the defect report names, so the reported gesture can be observed before the fix and
  proven after it.

  The driver never modifies the product. Every editor accessible name it samples is logged,
  together with the editor's UIA keyboard-focus state, the buffer line/byte label, whether the
  process is still alive, a PNG and a UIA tree. The final JSON is what the evidence document
  quotes.

.EXAMPLE
  powershell -ExecutionPolicy Bypass -File docs/goal-desktop-v4/round-11/gui-gesture.ps1 -Gesture backspace-join -OutDir docs/goal-desktop-v4/round-11/pre
  powershell -ExecutionPolicy Bypass -File docs/goal-desktop-v4/round-11/gui-gesture.ps1 -Gesture arrow-up -OutDir docs/goal-desktop-v4/round-11/pre-arrow
#>
[CmdletBinding()]
param(
    [ValidateSet('backspace-join', 'backspace-in-line1', 'backspace-line1-start', 'arrow-up')]
    [string]$Gesture = 'backspace-join',
    [string]$Exe = 'C:\Users\ricar\dev\LovelaceSharp\rust\target-desktop\debug\lovelace-desktop.exe',
    [string]$OutDir = 'C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-11\pre',
    [int]$TimeoutSec = 30
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct GG_Rect { public int Left; public int Top; public int Right; public int Bottom; }
public static class GG_Native {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out GG_Rect rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr processId);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
}
"@

$script:appPid = 0
trap {
    if ($script:appPid -ne 0) { Stop-Process -Id $script:appPid -Force -ErrorAction SilentlyContinue }
    throw
}

function Log([string]$line) {
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    $text = "$stamp $line"
    Write-Host $text
    Add-Content -LiteralPath (Join-Path $OutDir 'gesture.log') -Value $text -Encoding UTF8
}

function Focus-Window([IntPtr]$h) {
    for ($attempt = 1; $attempt -le 4; $attempt++) {
        if ([GG_Native]::IsIconic($h)) {
            [GG_Native]::ShowWindow($h, 9) | Out-Null
            Start-Sleep -Milliseconds 150
        }
        if ([GG_Native]::GetForegroundWindow() -eq $h) { return $true }
        [GG_Native]::ShowWindow($h, 9) | Out-Null
        [GG_Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0003) | Out-Null
        [GG_Native]::BringWindowToTop($h) | Out-Null
        [GG_Native]::SetForegroundWindow($h) | Out-Null
        Start-Sleep -Milliseconds 200
        if ([GG_Native]::GetForegroundWindow() -eq $h) { return $true }

        $fg = [GG_Native]::GetForegroundWindow()
        $fgThread = if ($fg -ne [IntPtr]::Zero) { [GG_Native]::GetWindowThreadProcessId($fg, [IntPtr]::Zero) } else { 0 }
        $myThread = [GG_Native]::GetCurrentThreadId()
        $attached = $false
        if ($fgThread -ne 0 -and $fgThread -ne $myThread) {
            $attached = [GG_Native]::AttachThreadInput($myThread, $fgThread, $true)
        }
        [GG_Native]::BringWindowToTop($h) | Out-Null
        [GG_Native]::SetForegroundWindow($h) | Out-Null
        if ($attached) { [GG_Native]::AttachThreadInput($myThread, $fgThread, $false) | Out-Null }
        Start-Sleep -Milliseconds 250
        $now = [GG_Native]::GetForegroundWindow()
        if ($now -eq $h) { return $true }
        Log "focus attempt=$attempt hwnd=$h fg=$now fgThread=$fgThread attached=$attached"
        Start-Sleep -Milliseconds 250
    }
    return $false
}

function Assert-Foreground([IntPtr]$h, [string]$what) {
    if ([GG_Native]::GetForegroundWindow() -ne $h) {
        throw "$what refused: window $h does not own the foreground"
    }
}

function Send-Checked([IntPtr]$h, [string]$sequence, [string]$label) {
    if (-not (Focus-Window $h)) { throw "cannot focus the window before $label" }
    Assert-Foreground $h $label
    [System.Windows.Forms.SendKeys]::SendWait($sequence)
    Log "send label=$label keys='$sequence'"
    Start-Sleep -Milliseconds 250
}

function Click-At([int]$x, [int]$y) {
    [GG_Native]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 80
    [GG_Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 50
    [GG_Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 200
}

function Get-EditorElement($rootEl) {
    $edits = $rootEl.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        (New-Object System.Windows.Automation.PropertyCondition(
            [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
            [System.Windows.Automation.ControlType]::Edit)))
    foreach ($e in $edits) { if ($e.Current.Name -like 'Script editor,*') { return $e } }
    return $null
}

function Get-EditorName([IntPtr]$h) {
    try {
        $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $e = Get-EditorElement $rootEl
        if ($e) { return $e.Current.Name }
    } catch { }
    return ''
}

function Get-EditorKeyboardFocus([IntPtr]$h) {
    try {
        $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $e = Get-EditorElement $rootEl
        if ($e) { return [bool]$e.Current.HasKeyboardFocus }
    } catch { }
    return $false
}

function Get-BufferLabel([IntPtr]$h) {
    try {
        $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $texts = $rootEl.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            (New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Text)))
        foreach ($t in $texts) {
            $n = $t.Current.Name
            if ($n -match '^\d+ lines / \d+ bytes$') { return $n }
        }
    } catch { }
    return ''
}

function Wait-EditorName([IntPtr]$h, [string]$pattern, [string]$label, [int]$tries = 25) {
    $last = ''
    for ($i = 0; $i -lt $tries; $i++) {
        $last = Get-EditorName $h
        if ($last -and $last -match $pattern) { return $last }
        Start-Sleep -Milliseconds 100
    }
    Log "wait label=$label pattern='$pattern' last='$last'"
    return $last
}

function Save-Shot([IntPtr]$h, [string]$path, [string]$label) {
    if (-not (Focus-Window $h)) { throw "cannot focus the window for $label capture" }
    Assert-Foreground $h "$label capture"
    Start-Sleep -Milliseconds 250
    $r = New-Object GG_Rect
    [void][GG_Native]::GetWindowRect($h, [ref]$r)
    $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
    if ($w -lt 400 -or $ht -lt 300) { throw "refusing $label capture with rect ${w}x${ht}" }
    $bmp = New-Object System.Drawing.Bitmap($w, $ht)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $ht)))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    $bytes = (Get-Item -LiteralPath $path).Length
    Log "capture label=$label rect=${w}x${ht} file=$path bytes=$bytes"
    return $bytes
}

function Dump-Uia([IntPtr]$h, [string]$path, [string]$label) {
    $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    $lines = New-Object System.Collections.Generic.List[string]
    $script:ggNodes = 0
    $lines.Add("UIA $label hwnd=$h")
    $lines.Add("root | $($rootEl.Current.Name) | $($rootEl.Current.ControlType.ProgrammaticName)")
    function GG-Walk($el, [int]$depth) {
        if ($depth -gt 10) { return }
        try { $c = $walker.GetFirstChild($el) } catch { return }
        while ($c) {
            try {
                $n = $c.Current
                $type = $n.ControlType.ProgrammaticName.Replace('ControlType.', '')
                $lines.Add(('  ' * $depth) + "$type | $($n.Name) | focusable=$($n.IsKeyboardFocusable) | enabled=$($n.IsEnabled)")
                $script:ggNodes++
                GG-Walk $c ($depth + 1)
                $c = $walker.GetNextSibling($c)
            } catch { break }
        }
    }
    GG-Walk $rootEl 0
    $lines | Set-Content -LiteralPath $path -Encoding UTF8
    Log "uia label=$label nodes=$script:ggNodes file=$path"
    return $script:ggNodes
}

if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$logPath = Join-Path $OutDir 'gesture.log'
if (Test-Path -LiteralPath $logPath) { Remove-Item -LiteralPath $logPath -Force }
$stderrPath = Join-Path $OutDir 'stderr.log'
$stdoutPath = Join-Path $OutDir 'stdout.log'

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $Exe
$psi.UseShellExecute = $false
$psi.RedirectStandardError = $true
$psi.RedirectStandardOutput = $true
$p = [System.Diagnostics.Process]::Start($psi)
$script:appPid = $p.Id
$errTask = $p.StandardError.ReadToEndAsync()
$outTask = $p.StandardOutput.ReadToEndAsync()
Log "launch pid=$($p.Id) gesture=$Gesture exe=$Exe"

# Wait for the real titled window and a normal rect (the launch environment starts it minimized).
$h = [IntPtr]::Zero
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    $p.Refresh()
    if ($p.HasExited) { throw "process exited during launch with code $($p.ExitCode)" }
    if ($p.MainWindowHandle -ne 0 -and $p.MainWindowTitle -like '*Lovelace Desktop*') {
        $candidate = [IntPtr]$p.MainWindowHandle
        if ([GG_Native]::IsIconic($candidate)) { [GG_Native]::ShowWindow($candidate, 9) | Out-Null }
        $r = New-Object GG_Rect
        [void][GG_Native]::GetWindowRect($candidate, [ref]$r)
        $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
        if ($w -lt 400 -or $ht -lt 300) {
            [void][GG_Native]::SetWindowPos($candidate, [IntPtr]::Zero, 40, 40, 1296, 839, 0x0040)
            Start-Sleep -Milliseconds 200
            [void][GG_Native]::GetWindowRect($candidate, [ref]$r)
            $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
        }
        if ($w -ge 400 -and $ht -ge 300) {
            Start-Sleep -Milliseconds 400
            $p.Refresh()
            if ($p.MainWindowHandle -eq $candidate) { $h = $candidate; break }
        }
    }
    Start-Sleep -Milliseconds 100
}
if ($h -eq [IntPtr]::Zero) { throw "no titled window with a normal rect within ${TimeoutSec}s" }
Log "window hwnd=$h after=$([int]$sw.Elapsed.TotalMilliseconds)ms"

# Wait for the UIA tree, then select the empty untitled.lv tab and click into its editor.
$root = $null
for ($i = 0; $i -lt 100; $i++) {
    try {
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        if ($all.Count -ge 30) { break }
    } catch { }
    Start-Sleep -Milliseconds 200
}
Log "uia ready nodes=$($all.Count)"

$tabCond = New-Object System.Windows.Automation.PropertyCondition(
    [System.Windows.Automation.AutomationElement]::NameProperty, 'untitled.lv')
$tab = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $tabCond)
if (-not $tab) { throw 'no untitled.lv tab' }
$tr = $tab.Current.BoundingRectangle
$tabX = [int]($tr.Left + $tr.Width / 2); $tabY = [int]($tr.Top + $tr.Height / 2)
if (-not (Focus-Window $h)) { throw 'cannot focus before tab click' }
Click-At $tabX $tabY
Log "clicked tab untitled.lv at ${tabX},${tabY}"
$editorName = Wait-EditorName $h '^Script editor, 1 lines' 'empty-tab'
if ($editorName -notmatch '^Script editor, 1 lines') {
    Log "tab select did not land ('$editorName'); trying the UIA Invoke action"
    try {
        $invokeRoot = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $invokeTab = $invokeRoot.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $tabCond)
        $invoke = $invokeTab.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $invoke.Invoke()
        Start-Sleep -Milliseconds 600
        $editorName = Wait-EditorName $h '^Script editor, 1 lines' 'empty-tab-invoke'
    } catch {
        Log "UIA Invoke failed: $($_.Exception.Message)"
    }
}
if ($editorName -notmatch '^Script editor, 1 lines') {
    Log "tab select still did not land ('$editorName'); retrying the mouse click"
    Click-At $tabX $tabY
    Start-Sleep -Milliseconds 500
    $editorName = Wait-EditorName $h '^Script editor, 1 lines' 'empty-tab-retry'
}
Log "editor after tab='$editorName'"
if ($editorName -notmatch '^Script editor, 1 lines') {
    throw "the empty untitled document was not selected; editor='$editorName'"
}

$root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
$edit = Get-EditorElement $root
if (-not $edit) { throw 'editor element not found' }
$er = $edit.Current.BoundingRectangle
$edX = [int]($er.Left + 40); $edY = [int]($er.Top + 40)
Click-At $edX $edY
Log "clicked editor at ${edX},${edY} rect=$er"
Send-Checked $h '{HOME}' 'home-line1'
$n = Wait-EditorName $h 'caret line 1 column 1$' 'line1-start'
Log "caret at line 1 start='$n'"

# Type the two-line document: "a" ENTER "b".
Send-Checked $h 'a' 'line1-a'
$n = Wait-EditorName $h 'caret line 1 column 2$' 'after-a'
Log "caret after a='$n'"
Send-Checked $h '{ENTER}' 'enter'
$n = Wait-EditorName $h 'caret line 2 column 1$' 'after-enter'
Log "caret after enter='$n'"
Send-Checked $h 'b' 'line2-b'
$n = Wait-EditorName $h 'caret line 2 column 2$' 'after-b'
Log "caret after b='$n'"
Send-Checked $h '{HOME}' 'home-line2'
$n = Wait-EditorName $h 'caret line 2 column 1$' 'after-home'
Log "caret after home='$n'"

# Put the caret where the control gestures need it before the common "before" record. These
# controls do not use Up to reach line 1 because pre-fix Up itself steals focus (DEF-002); the
# mouse click parks at the end of line 1 exactly as the harness scenario does.
if ($Gesture -eq 'backspace-in-line1' -or $Gesture -eq 'backspace-line1-start') {
    Click-At ([int]($er.Right - 30)) ([int]($er.Top + 5))
    $n = Wait-EditorName $h 'caret line 1 column 2$' 'park-line1-end'
    Log "caret parked at line 1 end='$n'"
    if ($Gesture -eq 'backspace-line1-start') {
        Send-Checked $h '{HOME}' 'home-line1-control'
        $n = Wait-EditorName $h 'caret line 1 column 1$' 'control-line1-start'
        Log "caret at control line 1 start='$n'"
    }
}

# Record the two-line document before the gesture (screenshot + UIA + caret and size labels).
$prePng = Join-Path $OutDir 'gesture-before.png'
$preUia = Join-Path $OutDir 'gesture-before-uia.txt'
$preBytes = 0
$preNodes = 0
try { $preBytes = Save-Shot $h $prePng 'gesture-before' } catch { Log "pre capture failed: $($_.Exception.Message)" }
try { $preNodes = Dump-Uia $h $preUia 'gesture-before' } catch { Log "pre uia failed: $($_.Exception.Message)" }
$before = Get-EditorName $h
$beforeSize = Get-BufferLabel $h
$focusBefore = Get-EditorKeyboardFocus $h
Log "before gesture editor='$before' size='$beforeSize' uia_focus=$focusBefore pre_png=$preBytes pre_uia=$preNodes"

$gestureOutcome = 'unknown'
$after = ''
$afterSize = ''
$focusAfter = $false

if ($Gesture -eq 'backspace-join') {
    # The reported gesture: Backspace at the start of line 2 joins it with line 1.
    Send-Checked $h '{BACKSPACE}' 'backspace-join'
    Start-Sleep -Milliseconds 1200
} elseif ($Gesture -eq 'backspace-in-line1' -or $Gesture -eq 'backspace-line1-start') {
    # Controls: same key, but the edit cannot remove a line.
    Send-Checked $h '{BACKSPACE}' $Gesture
    Start-Sleep -Milliseconds 1200
} else {
    # DEF-002: Up must move the caret and keep keyboard focus in the editor.
    Send-Checked $h '{UP}' 'arrow-up'
    $afterUp = Wait-EditorName $h 'caret line 1 column 1$' 'after-up'
    $focusAfterUp = Get-EditorKeyboardFocus $h
    Log "after up editor='$afterUp' uia_focus=$focusAfterUp"
    Send-Checked $h 'c' 'type-after-up'
    Start-Sleep -Milliseconds 600
}

$p.Refresh()
$alive = -not $p.HasExited
if (-not $alive) { [void]$p.WaitForExit() }
$exitCode = if ($p.HasExited) { $p.ExitCode } else { $null }
$hwndAfter = if ($alive) { [IntPtr]$p.MainWindowHandle } else { [IntPtr]::Zero }
$windowGone = ($hwndAfter -eq [IntPtr]::Zero)
if ($alive -and -not $windowGone) {
    $after = Get-EditorName $h
    $afterSize = Get-BufferLabel $h
    $focusAfter = Get-EditorKeyboardFocus $h
}
Log "after gesture alive=$alive exited=$($p.HasExited) exit=$exitCode window_gone=$windowGone editor='$after' size='$afterSize' uia_focus=$focusAfter"

$pngBytes = 0
$nodes = 0
if ($alive -and -not $windowGone) {
    try { $pngBytes = Save-Shot $h (Join-Path $OutDir 'gesture-final.png') 'gesture-final' } catch { Log "capture failed: $($_.Exception.Message)" }
    try { $nodes = Dump-Uia $h (Join-Path $OutDir 'gesture-final-uia.txt') 'gesture-final' } catch { Log "uia failed: $($_.Exception.Message)" }
    # Prove the window still accepts an ordinary keystroke after the gesture.
    Send-Checked $h '{END}' 'end-after-gesture'
    Start-Sleep -Milliseconds 400
    $afterEnd = Get-EditorName $h
    Log "after end editor='$afterEnd'"
    try { [void](Save-Shot $h (Join-Path $OutDir 'gesture-after-typing.png') 'gesture-after-typing') } catch { Log "typing capture failed: $($_.Exception.Message)" }
}

if ($p.HasExited) {
    $errTail = $errTask.Result
    $outTail = $outTask.Result
} else {
    $errTail = if ($errTask.IsCompleted) { $errTask.Result } else { '' }
    $outTail = if ($outTask.IsCompleted) { $outTask.Result } else { '' }
}
$errTail | Set-Content -LiteralPath $stderrPath -Encoding UTF8
$outTail | Set-Content -LiteralPath $stdoutPath -Encoding UTF8

$result = [pscustomobject]@{
    gesture = $Gesture
    exe = $Exe
    pid = $p.Id
    window = $h.ToString()
    alive_after_gesture = $alive
    window_gone = $windowGone
    exit_code = $exitCode
    editor_before = $before
    size_before = $beforeSize
    uia_keyboard_focus_before = $focusBefore
    editor_after = $after
    size_after = $afterSize
    uia_keyboard_focus_after = $focusAfter
    final_png_bytes = $pngBytes
    final_uia_nodes = $nodes
    stderr = $errTail
    stdout = $outTail
}
$result | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutDir 'gesture-result.json') -Encoding UTF8
Log "result gesture=$Gesture alive=$alive window_gone=$windowGone exit=$exitCode png=$pngBytes uia=$nodes stderr_chars=$($errTail.Length)"

if ($alive -and -not $windowGone) {
    $p.CloseMainWindow() | Out-Null
    if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force }
}
Write-Output "GESTURE_DONE gesture=$Gesture alive=$alive window_gone=$windowGone exit=$exitCode editor_after='$after' size_after='$afterSize'"
