<#
.SYNOPSIS
  Real-window GUI harness for Lovelace Desktop acceptance evidence.

.DESCRIPTION
  Launches a native window, waits for it, captures a PNG of its client area, dumps its UI Automation
  tree, invokes controls by their accessible name, sends keystrokes, and closes it. Every action
  appends a line to a driver log so the artifact bundle is reproducible and can be attached to the
  gate as `gui` evidence.

  Windows PowerShell 5.1 compatible. ASCII-only source on purpose: PS 5.1 reads .ps1 as ANSI when a
  BOM is absent, and a non-ASCII literal would corrupt the parser.

.EXAMPLE
  powershell -File tools/gui-harness.ps1 -Action launch -Exe path\to\app.exe -OutDir out -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action capture -ProcessId 1234 -OutDir out -Name shot -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action uia -ProcessId 1234 -OutDir out -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action click -ProcessId 1234 -Name "Run" -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action keys -ProcessId 1234 -Keys "^a" -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action close -ProcessId 1234 -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action scenario -Exe path\to\app.exe -OutDir out -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action scenario -FastStatus -Exe path\to\app.exe -OutDir out -Log out\driver.log
  powershell -File tools/gui-harness.ps1 -Action scenario -FastStatus -TraceFile out\run-trace.log -Exe path\to\app.exe -OutDir out -Log out\driver.log
    Drives the whole run/cancel gesture in one process and writes scenario-running.png,
    scenario-final.png, scenario-running-uia.txt, scenario-final-uia.txt and scenario-summary.md.
  powershell -File tools/gui-harness.ps1 -Action scenario -Flow cancel -FastStatus -FastKeys -WakeRepaint -CaptureBurst 3 -CaptureStates "run Cancelling" -MaxAttempts 4 -Exe path\to\app.exe -OutDir out -Log out\driver.log
    Round 14 capture-on-state: samples one cached status node at a true 1 ms cadence, and on the
    first match saves run-cancelling.png (plus -frame2/-frame3), run-cancelling-uia.txt and a
    driver-log line with the trigger offsets. -FastKeys injects Ctrl+Enter/Esc without the blocking
    SendKeys wait; -WakeRepaint asks egui for one repaint with a pointer move; -CaptureBurst grabs
    that many frames. -Flow stale captures run Stale after an edit following run Finished; -Flow
    queued with -SubmitGrab preallocates three bitmaps and copies the screen immediately after
    Ctrl+Enter, because the UIA label can lag the painted Queued frame by ~20 ms.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('launch', 'wait', 'capture', 'uia', 'click', 'keys', 'close', 'state', 'focus', 'typeinto', 'setvalue', 'scenario')]
    [string]$Action,
    [string]$Exe = '',
    [string[]]$Arguments = @(),
    [int]$ProcessId = 0,
    [string]$Title = '',
    [string]$Name = '',
    [string]$Keys = '',
    [string]$OutDir = '',
    [string]$Log = '',
    [int]$TimeoutSec = 30,
    [switch]$FastStatus,
    [string]$TraceFile = '',
    # Round 14: capture-on-state. Accepts comma-separated status labels ("run Cancelling,run Queued").
    [string[]]$CaptureStates = @(),
    # Round 14 flows: cancel (default, the legacy gesture), stale (edit after Finished), queued (submit race).
    [ValidateSet('cancel', 'stale', 'queued')][string]$Flow = 'cancel',
    [int]$MaxAttempts = 1,
    # Round 14: deliver the trigger key with keybd_event instead of the blocking SendKeys.SendWait,
    # so the 1 ms sampler is running while the product handles the key.
    [switch]$FastKeys,
    [int]$CancelAfterMs = 300,
    [string]$StaleChar = 'x',
    # Round 14 pixel capture: a transient state can be entered and left without the product
    # beginning a frame (round-14 trace: 28.1 ms Cancelling, zero frame_begin marks). A benign
    # pointer move asks egui for a repaint without touching the run state; a short burst then
    # gives the compositor more than one chance to expose that frame.
    [switch]$WakeRepaint,
    [int]$CaptureBurst = 1,
    # Round 14 queued-only: the UIA label can trail the painted frame by ~20 ms. Preallocate the
    # bitmap before the submit key and copy the screen immediately after injecting it, so the
    # grab lands inside the ~8 ms Queued frame that the trace proves was painted.
    [switch]$SubmitGrab
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing, System.Windows.Forms, UIAutomationClient, UIAutomationTypes

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public struct GuiRect { public int Left; public int Top; public int Right; public int Bottom; }
public static class GuiNative {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out GuiRect rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out GuiRect rect);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hWnd, ref GuiRect point);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, IntPtr processId);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint idAttach, uint idAttachTo, bool fAttach);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint flags, UIntPtr extraInfo);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int max);
}
"@

$HWND_TOPMOST = [IntPtr](-1)
$SWP_NOSIZE_MOVE = 0x0003

function Write-Log([string]$line) {
    $stamp = (Get-Date).ToString('HH:mm:ss.fff')
    $text = "$stamp $line"
    Write-Output $text
    if ($Log) {
        $dir = Split-Path $Log -Parent
        if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
        Add-Content -LiteralPath $Log -Value $text -Encoding UTF8
    }
}

function Get-AppProcess {
    if ($ProcessId -eq 0) { throw 'no -ProcessId given' }
    $p = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $p) { throw "process $ProcessId is not running" }
    return $p
}

function Get-MainHandle {
    $p = Get-AppProcess
    for ($i = 0; $i -lt ($TimeoutSec * 10); $i++) {
        $p.Refresh()
        if ($p.MainWindowHandle -ne 0 -and $p.MainWindowTitle) { return $p.MainWindowHandle }
        Start-Sleep -Milliseconds 100
    }
    throw "no main window appeared within $TimeoutSec s (title filter '$Title')"
}

function Find-ByTitle([string]$substr) {
    $procs = Get-Process | Where-Object { $_.MainWindowHandle -ne 0 -and $_.MainWindowTitle -like "*$substr*" }
    if (-not $procs) { throw "no window whose title contains '$substr'" }
    return @($procs)[0]
}

function Get-WindowTitleText([IntPtr]$h) {
    if ($h -eq [IntPtr]::Zero) { return '' }
    $sb = New-Object System.Text.StringBuilder 256
    [void][GuiNative]::GetWindowText($h, $sb, 256)
    return $sb.ToString()
}

function Get-AppWindowHandle([int]$procId, [string]$titleFilter) {
    # Process.MainWindowHandle can briefly be a winit helper window ('Winit Thread Event Target',
    # empty title). Only trust the handle while the process reports the product title.
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if (-not $p) { return [IntPtr]::Zero }
    $p.Refresh()
    if ($p.MainWindowHandle -eq 0) { return [IntPtr]::Zero }
    if ($p.MainWindowTitle -eq '' -or $p.MainWindowTitle -notlike "*$titleFilter*") { return [IntPtr]::Zero }
    return [IntPtr]$p.MainWindowHandle
}

function ConvertTo-SendKeysLiteral([string]$text) {
    # SendKeys treats + ^ % ~ ( ) { } as syntax: parentheses are grouping, not typeable text.
    # Escape them so "matmul(eye(400), eye(400))" arrives with all 26 characters.
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in $text.ToCharArray()) {
        $s = [string]$ch
        if ('+^%~(){}'.Contains($s)) { [void]$sb.Append('{').Append($s).Append('}') }
        else { [void]$sb.Append($s) }
    }
    return $sb.ToString()
}

function Focus-Window([IntPtr]$h) {
    # SetForegroundWindow can silently fail when the caller is not foreground. The launch
    # environment also starts the target minimized (-32000,-32000 / 160x28), so restore it first.
    # If the plain call is refused, attach to the foreground thread's input queue and retry: that
    # is the only reliable path when a browser/console owns the foreground lock.
    if ([GuiNative]::IsIconic($h)) {
        [GuiNative]::ShowWindow($h, 9) | Out-Null                  # SW_RESTORE
        Start-Sleep -Milliseconds 150
    }
    if ([GuiNative]::GetForegroundWindow() -eq $h) { return $true }
    [GuiNative]::ShowWindow($h, 9) | Out-Null
    [GuiNative]::SetWindowPos($h, $HWND_TOPMOST, 0, 0, 0, 0, $SWP_NOSIZE_MOVE) | Out-Null
    [GuiNative]::BringWindowToTop($h) | Out-Null
    [GuiNative]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 200
    if ([GuiNative]::GetForegroundWindow() -eq $h) { return $true }

    $fg = [GuiNative]::GetForegroundWindow()
    $fgThread = if ($fg -ne [IntPtr]::Zero) { [GuiNative]::GetWindowThreadProcessId($fg, [IntPtr]::Zero) } else { 0 }
    $myThread = [GuiNative]::GetCurrentThreadId()
    $attached = $false
    if ($fgThread -ne 0 -and $fgThread -ne $myThread) {
        $attached = [GuiNative]::AttachThreadInput($myThread, $fgThread, $true)
    }
    [GuiNative]::BringWindowToTop($h) | Out-Null
    [GuiNative]::SetForegroundWindow($h) | Out-Null
    if ($attached) { [GuiNative]::AttachThreadInput($myThread, $fgThread, $false) | Out-Null }
    Start-Sleep -Milliseconds 250
    return ([GuiNative]::GetForegroundWindow() -eq $h)
}

function Assert-Foreground([IntPtr]$h, [string]$action) {
    $fg = [GuiNative]::GetForegroundWindow()
    if ($fg -ne $h) {
        throw "$action refused: target hwnd $h does not own the foreground (foreground=$fg '$(Get-WindowTitleText $fg)')"
    }
}

switch ($Action) {
    'launch' {
        if ($Exe -eq '') { throw 'launch needs -Exe' }
        if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }
        $p = if ($Arguments.Count -gt 0) { Start-Process -FilePath $Exe -ArgumentList $Arguments -PassThru } else { Start-Process -FilePath $Exe -PassThru }
        Write-Log "launch pid=$($p.Id) exe=$Exe args=$($Arguments -join ' ')"
        Write-Output "PROCESS_ID=$($p.Id)"
        break
    }
    'wait' {
        $p = if ($ProcessId -ne 0) { Get-AppProcess } else { Find-ByTitle $Title }
        $h = if ($ProcessId -ne 0) { Get-MainHandle } else { [IntPtr]$p.MainWindowHandle }
        Write-Log "wait pid=$($p.Id) hwnd=$h title=$($p.MainWindowTitle)"
        Write-Output "HWND=$h"
        break
    }
    'state' {
        $p = Get-AppProcess
        $p.Refresh()
        $hung = if ($p.MainWindowHandle -ne 0) { [GuiNative]::IsHungAppWindow($p.MainWindowHandle) } else { $false }
        $responding = $p.Responding
        Write-Log "state pid=$($p.Id) hwnd=$($p.MainWindowHandle) responding=$responding hung=$hung cpu=$([math]::Round($p.TotalProcessorTime.TotalMilliseconds))ms ws=$([math]::Round($p.WorkingSet64/1MB))MB"
        Write-Output "RESPONDING=$responding HUNG=$hung WINDOW=$($p.MainWindowHandle) TITLE=$($p.MainWindowTitle)"
        break
    }
    'capture' {
        $p = Get-AppProcess
        $h = if ($Title -ne '') { [IntPtr](Find-ByTitle $Title).MainWindowHandle } else { (Get-AppProcess) | ForEach-Object { $_.Refresh(); $_.MainWindowHandle } }
        if ($Title -ne '') { $h = [IntPtr](Find-ByTitle $Title).MainWindowHandle } else { $h = [IntPtr]$h }
        if ($h -eq [IntPtr]::Zero) { throw 'no window handle' }
        if (-not (Focus-Window $h)) { throw 'capture refused: could not bring the target window to the foreground' }
        Assert-Foreground $h 'capture'
        Start-Sleep -Milliseconds 250
        $r = New-Object GuiRect
        [GuiNative]::GetWindowRect($h, [ref]$r) | Out-Null
        $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
        if ($w -le 0 -or $ht -le 0) { throw "bad window rect ${w}x${ht}" }
        $bmp = New-Object System.Drawing.Bitmap($w, $ht)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $ht)))
        if (-not $OutDir) { $OutDir = '.' }
        New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
        $name = if ($Name) { $Name } else { "window-$((Get-Date).ToString('HHmmss'))" }
        $path = Join-Path $OutDir "$name.png"
        $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        $g.Dispose(); $bmp.Dispose()
        Write-Log "capture pid=$($p.Id) hwnd=$h rect=${w}x${ht} file=$path bytes=$((Get-Item $path).Length)"
        Write-Output "CAPTURE=$path SIZE=${w}x${ht}"
        break
    }
    'uia' {
        $p = Get-AppProcess
        $p.Refresh()
        $h = [IntPtr]$p.MainWindowHandle
        if ($h -eq [IntPtr]::Zero) { throw 'no window handle' }
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
        $lines = New-Object System.Collections.Generic.List[string]
        $lines.Add("UIA pid=$($p.Id) window=$($p.MainWindowTitle)")
        $script:nodes = 0
        function Walk($el, $depth) {
            if ($depth -gt 8) { return }
            $c = $walker.GetFirstChild($el)
            while ($c) {
                $n = $c.Current
                $type = $n.ControlType.ProgrammaticName.Replace('ControlType.', '')
                $lines.Add(('  ' * $depth) + "$type | $($n.Name) | id=$($n.AutomationId) | focusable=$($n.IsKeyboardFocusable) | enabled=$($n.IsEnabled)")
                $script:nodes++
                Walk $c ($depth + 1)
                $c = $walker.GetNextSibling($c)
            }
        }
        $lines.Add("root | $($root.Current.Name) | $($root.Current.ControlType.ProgrammaticName)")
        Walk $root 0
        if (-not $OutDir) { $OutDir = '.' }
        New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
        $path = Join-Path $OutDir 'uia-tree.txt'
        $lines | Set-Content -LiteralPath $path -Encoding UTF8
        Write-Log "uia pid=$($p.Id) hwnd=$h nodes=$script:nodes file=$path"
        Write-Output "UIA=$path NODES=$script:nodes"
        break
    }
    'click' {
        if ($Name -eq '') { throw 'click needs -Name' }
        $p = Get-AppProcess
        $p.Refresh()
        $root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
        $cond = New-Object System.Windows.Automation.AndCondition(
            (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, $Name)),
            (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::IsEnabledProperty, $true))
        )
        $el = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cond)
        if (-not $el) { throw "no enabled element named '$Name'" }
        $pattern = $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        $pattern.Invoke()
        Write-Log "click pid=$($p.Id) name=$Name type=$($el.Current.ControlType.ProgrammaticName)"
        Write-Output "CLICKED=$Name"
        break
    }
    'keys' {
        if ($Keys -eq '') { throw 'keys needs -Keys' }
        $p = Get-AppProcess
        $p.Refresh()
        if (-not (Focus-Window ([IntPtr]$p.MainWindowHandle))) { throw 'keys refused: could not bring the target window to the foreground' }
        Assert-Foreground ([IntPtr]$p.MainWindowHandle) 'keys'
        Start-Sleep -Milliseconds 150
        [System.Windows.Forms.SendKeys]::SendWait($Keys)
        Write-Log "keys pid=$($p.Id) keys=$Keys"
        Write-Output "SENT=$Keys"
        break
    }
    'focus' {
        $p = Get-AppProcess
        $p.Refresh()
        $ok = Focus-Window ([IntPtr]$p.MainWindowHandle)
        Write-Log "focus pid=$($p.Id) hwnd=$($p.MainWindowHandle) ok=$ok"
        Write-Output "FOCUSED=$ok"
        if (-not $ok) { exit 1 }
        break
    }
    'scenario' {
        # One process owns the foreground for the whole gesture: launch -> restore -> focus -> type ->
        # run -> sample -> cancel -> sample -> capture -> close. Separate tool calls lose the
        # foreground, which is why the keyboard path cannot be driven across them.
        #
        # The launch environment starts the eframe window minimized, so its first rect is
        # -32000,-32000 / 160x28 and a capture taken then is empty. The driver restores and sizes the
        # window, waits for a real rect, re-acquires the UIA root from the live handle, and only then
        # types. Every SendKeys is preceded by a foreground assertion, retried at most three times.
        if ($Exe -eq '') { throw 'scenario needs -Exe' }
        if (-not (Test-Path -LiteralPath $Exe)) { throw "exe not found: $Exe" }
        if (-not $OutDir) { throw 'scenario needs -OutDir for its artifacts and summary' }
        $scriptText = if ($Keys -ne '') { $Keys } else { 'matmul(eye(400), eye(400))' }
        # Accept the documented "statement{ENTER}" spelling; the Enter is sent separately below.
        if ($scriptText -match '\{ENTER\}$') { $scriptText = $scriptText.Substring(0, $scriptText.Length - 7) }
        New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
        $runPng = Join-Path $OutDir 'scenario-running.png'
        $finalPng = Join-Path $OutDir 'scenario-final.png'
        $runUia = Join-Path $OutDir 'scenario-running-uia.txt'
        $finalUia = Join-Path $OutDir 'scenario-final-uia.txt'
        $summaryPath = Join-Path $OutDir 'scenario-summary.md'

        if ($TraceFile -ne '') {
            # The child inherits this process environment; the product writes one line per
            # run-start mark with monotonic and UTC timestamps (see src/run_trace.rs). This has
            # to happen before Start-Process: the product decides whether to trace on first use.
            $env:LOVELACE_RUN_TRACE = $TraceFile
            Write-Log "scenario run-start trace file=$TraceFile"
        }
        $p = Start-Process -FilePath $Exe -PassThru
        $launchSw = [System.Diagnostics.Stopwatch]::StartNew()
        Write-Log "scenario launch pid=$($p.Id) exe=$Exe"

        # 1. Poll for the live viewport (not winit's helper window) and a real rect. Never proceed on
        #    the minimized placeholder: -32000,-32000 / 160x28 is the state this environment starts in.
        $windowTitleFilter = if ($Title -ne '') { $Title } else { 'Lovelace Desktop' }
        $h = [IntPtr]::Zero
        $rect = ''
        $restored = $false
        $mainSeen = [IntPtr]::Zero
        $windowReadyMs = 0
        $lastNote = -1000
        while ($launchSw.Elapsed.TotalSeconds -lt $TimeoutSec) {
            $p.Refresh()
            if ($p.MainWindowHandle -ne 0) {
                $mainSeen = [IntPtr]$p.MainWindowHandle
                $candidate = Get-AppWindowHandle $p.Id $windowTitleFilter
                if ($candidate -ne [IntPtr]::Zero) {
                    if ([GuiNative]::IsIconic($candidate)) {
                        [GuiNative]::ShowWindow($candidate, 9) | Out-Null
                        $restored = $true
                    }
                    $r = New-Object GuiRect
                    [void][GuiNative]::GetWindowRect($candidate, [ref]$r)
                    $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
                    if ($w -lt 400 -or $ht -lt 300) {
                        # Force normal geometry; this environment starts the window minimized.
                        [void][GuiNative]::SetWindowPos($candidate, [IntPtr]::Zero, 40, 40, 1296, 839, 0x0040)
                        $restored = $true
                        Start-Sleep -Milliseconds 150
                        [void][GuiNative]::GetWindowRect($candidate, [ref]$r)
                        $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
                    }
                    if ($w -ge 400 -and $ht -ge 300) {
                        # The first frames can still swap native handles; require a stable handle.
                        Start-Sleep -Milliseconds 400
                        $stable = Get-AppWindowHandle $p.Id $windowTitleFilter
                        if ($stable -eq $candidate) {
                            [void][GuiNative]::GetWindowRect($candidate, [ref]$r)
                            $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
                            if ($w -ge 400 -and $ht -ge 300) {
                                $h = $candidate
                                $rect = "${w}x${ht} at $($r.Left),$($r.Top)"
                                $elapsed = [int]$launchSw.Elapsed.TotalMilliseconds
                                $windowReadyMs = $elapsed
                                Write-Log "scenario window mainhandle=$mainSeen hwnd=$h title='$(Get-WindowTitleText $h)' rect=$rect after=${elapsed}ms restored=$restored stable=True"
                                break
                            }
                        }
                        Write-Log "scenario window changed during settle: candidate=$candidate stable=$stable"
                    }
                }
                $elapsed = [int]$launchSw.Elapsed.TotalMilliseconds
                if (($elapsed - $lastNote) -ge 1000) {
                    $lastNote = $elapsed
                    $noteTitle = if ($candidate -ne [IntPtr]::Zero) { Get-WindowTitleText $candidate } else { $p.MainWindowTitle }
                    Write-Log "scenario waiting window mainhandle=$mainSeen maintitle='$($p.MainWindowTitle)' candidate=$candidate title='$noteTitle' elapsed=${elapsed}ms"
                }
            }
            Start-Sleep -Milliseconds 100
        }
        if ($h -eq [IntPtr]::Zero -or $rect -eq '') {
            throw "scenario: no '$windowTitleFilter' window with rect >= 400x300 within ${TimeoutSec}s (mainhandle=$mainSeen hwnd=$h rect='$rect')"
        }

        # 2. Re-acquire the root after the window is shown and wait for a populated tree.
        $root = $null
        $nodeCount = 0
        $uiaSw = [System.Diagnostics.Stopwatch]::StartNew()
        while ($uiaSw.Elapsed.TotalSeconds -lt 20) {
            try {
                $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
                $all = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition)
                $nodeCount = $all.Count
            } catch {
                $nodeCount = -1
                Write-Log "scenario uia poll error: $($_.Exception.Message)"
            }
            if ($nodeCount -ge 30) { break }
            Start-Sleep -Milliseconds 200
        }
        $uiaMs = [int]$uiaSw.Elapsed.TotalMilliseconds
        Write-Log "scenario uia ready hwnd=$h nodes=$nodeCount after=${uiaMs}ms"
        if ($nodeCount -lt 30) {
            throw "scenario: UIA tree not ready: $nodeCount descendants (< 30) after ${uiaMs}ms"
        }

        function Get-EditorElement($rootEl) {
            $edits = $rootEl.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                    [System.Windows.Automation.ControlType]::Edit)))
            foreach ($e in $edits) { if ($e.Current.Name -like 'Script editor,*') { return $e } }
            return $null
        }

        function Get-EditorName {
            try {
                $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
                $e = Get-EditorElement $rootEl
                if ($e) { return $e.Current.Name }
            } catch {
                Write-Log "scenario editor query error: $($_.Exception.Message)" | Out-Null
            }
            return ''
        }

        function Get-StatusElement($rootEl) {
            # The status strip is one Text node whose Name carries "| run <phase>". Returning the
            # element (not just its name) lets -FastStatus poll that one node instead of walking
            # the whole UIA tree for every sample.
            try {
                $texts = $rootEl.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                        [System.Windows.Automation.ControlType]::Text)))
                foreach ($t in $texts) {
                    $n = $t.Current.Name
                    if ($n -match '\|\s+run (Idle|Running|Cancelling|Cancelled|Finished|Failed)\b') { return $t }
                }
            } catch {
                Write-Log "scenario status element query error: $($_.Exception.Message)" | Out-Null
            }
            return $null
        }

        function Get-StatusFromRoot($rootEl) {
            $element = Get-StatusElement $rootEl
            if ($element) { return $element.Current.Name }
            return ''
        }

        function Read-Status {
            try {
                $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
                return (Get-StatusFromRoot $rootEl)
            } catch {
                Write-Log "scenario status root error: $($_.Exception.Message)" | Out-Null
                return ''
            }
        }

        # -FastStatus: sample one cached UIA element every millisecond instead of re-walking the
        # tree and sleeping 150 ms. The standard mode is unchanged so existing evidence stays
        # comparable; the fast mode is the finer sampler RUN-001 needs to resolve sub-150 ms.
        $script:statusElement = $null
        function Read-StatusFast {
            if ($script:statusElement) {
                try {
                    $name = $script:statusElement.Current.Name
                    if ($name) { return $name }
                } catch { }
                try {
                    $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
                    $script:statusElement = Get-StatusElement $rootEl
                    if ($script:statusElement) { return $script:statusElement.Current.Name }
                } catch { }
            }
            return (Read-Status)
        }

        function Read-StatusSample {
            if ($FastStatus) { return (Read-StatusFast) }
            return (Read-Status)
        }

        function Wait-EditorNameMatch([string]$pattern, [string]$label) {
            $last = ''
            for ($i = 0; $i -lt 25; $i++) {
                $last = Get-EditorName
                if ($last -and $last -match $pattern) { return $last }
                Start-Sleep -Milliseconds 100
            }
            Write-Log "scenario caret wait label=$label pattern='$pattern' last='$last'" | Out-Null
            return $last
        }

        function Click-At([int]$x, [int]$y) {
            [GuiNative]::SetCursorPos($x, $y) | Out-Null
            Start-Sleep -Milliseconds 80
            [GuiNative]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # left down
            Start-Sleep -Milliseconds 50
            [GuiNative]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # left up
            Start-Sleep -Milliseconds 150
        }

        function Assert-InputFocus([string]$label) {
            $focused = $false
            for ($attempt = 1; $attempt -le 3; $attempt++) {
                if (Focus-Window $h) {
                    if ([GuiNative]::GetForegroundWindow() -eq $h) { $focused = $true; break }
                }
                $fg = [GuiNative]::GetForegroundWindow()
                Write-Log "scenario focus attempt=$attempt label=$label ok=False fg=$fg '$(Get-WindowTitleText $fg)'"
                Start-Sleep -Milliseconds 250
            }
            if (-not $focused) {
                $fg = [GuiNative]::GetForegroundWindow()
                throw "scenario: cannot take the foreground before $label after 3 attempts (fg=$fg '$(Get-WindowTitleText $fg)')"
            }
            $fgNow = [GuiNative]::GetForegroundWindow()
            if ($fgNow -ne $h) {
                throw "scenario: foreground changed immediately before $label (expected=$h fg=$fgNow '$(Get-WindowTitleText $fgNow)')"
            }
            return $fgNow
        }

        function Send-Checked([string]$sequence, [string]$label) {
            $fgNow = Assert-InputFocus $label
            [System.Windows.Forms.SendKeys]::SendWait($sequence)
            Write-Log "scenario send label=$label keys='$sequence' fg=$fgNow"
        }

        function Send-CtrlEnterFast {
            # SendKeys.SendWait blocks for the whole key-handling window (30-50 ms measured), so the
            # sampler cannot run while the product enters Queued/Cancelling. keybd_event only injects
            # the events; the caller starts sampling immediately afterwards.
            [GuiNative]::keybd_event(0x11, 0, 0, [UIntPtr]::Zero)       # Ctrl down
            [GuiNative]::keybd_event(0x0D, 0, 0, [UIntPtr]::Zero)       # Enter down
            [GuiNative]::keybd_event(0x0D, 0, 2, [UIntPtr]::Zero)       # Enter up
            [GuiNative]::keybd_event(0x11, 0, 2, [UIntPtr]::Zero)       # Ctrl up
        }

        function Send-EscapeFast {
            [GuiNative]::keybd_event(0x1B, 0, 0, [UIntPtr]::Zero)       # Esc down
            [GuiNative]::keybd_event(0x1B, 0, 2, [UIntPtr]::Zero)       # Esc up
        }

        function Wait-PollGate([System.Diagnostics.Stopwatch]$pollSw) {
            # Start-Sleep -Milliseconds 1 measures ~15.5 ms on this host (thread timer granularity),
            # which cannot resolve a 7 ms Queued window. Spin the remainder so the cadence really is
            # one read per millisecond; the cached-element read itself costs 0.3-1.2 ms.
            while ($pollSw.Elapsed.TotalMilliseconds -lt 1.0) {
                [System.Threading.Thread]::SpinWait(64) | Out-Null
            }
        }

        function Wait-SpinMs([double]$ms) {
            $sw = [System.Diagnostics.Stopwatch]::StartNew()
            while ($sw.Elapsed.TotalMilliseconds -lt $ms) {
                [System.Threading.Thread]::SpinWait(64) | Out-Null
            }
        }

        function Wake-Repaint {
            # egui only renders on demand. During a run the UI thread can go 100+ ms without a
            # frame, so a state can be entered and left with no painted frame at all. A pointer move
            # is a real input event that requests a repaint without touching the run state.
            $r = New-Object GuiRect
            if (-not [GuiNative]::GetWindowRect($h, [ref]$r)) { return }
            $x = [int]($r.Left + 240); $y = [int]($r.Top + 170)
            [GuiNative]::SetCursorPos($x, $y) | Out-Null
            [GuiNative]::SetCursorPos(($x + 3), $y) | Out-Null
        }

        function Save-Shot([string]$path, [string]$label) {
            if ([GuiNative]::IsIconic($h)) {
                [GuiNative]::ShowWindow($h, 9) | Out-Null
                Start-Sleep -Milliseconds 250
            }
            if (-not (Focus-Window $h)) { throw "scenario: cannot focus the window for the $label capture" }
            Assert-Foreground $h "$label capture"
            Start-Sleep -Milliseconds 250
            $r = New-Object GuiRect
            if (-not [GuiNative]::GetWindowRect($h, [ref]$r)) { throw "scenario: GetWindowRect failed for the $label capture" }
            $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
            if ($w -lt 400 -or $ht -lt 300) { throw "scenario: refusing $label capture with rect ${w}x${ht}" }
            $bmp = New-Object System.Drawing.Bitmap($w, $ht)
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $ht)))
            $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
            $g.Dispose(); $bmp.Dispose()
            $bytes = (Get-Item -LiteralPath $path).Length
            if ($bytes -le 20480) { throw "scenario: $label capture is only $bytes bytes (< 20 KB): $path" }
            Write-Log "scenario capture label=$label rect=${w}x${ht} file=$path bytes=$bytes" | Out-Null
            return $bytes
        }

        function Dump-UiaTree([string]$path, [string]$label, [string[]]$ExtraHeader = @()) {
            $script:scenarioDumpNodes = 0
            $lines = New-Object System.Collections.Generic.List[string]
            $rootEl = $null
            for ($attempt = 0; $attempt -lt 10; $attempt++) {
                try { $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h); break }
                catch { Start-Sleep -Milliseconds 200 }
            }
            if (-not $rootEl) { throw "scenario: cannot acquire the UIA root for the $label dump" }
            $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
            $lines.Add("UIA $label hwnd=$h pid=$($p.Id) window='$(Get-WindowTitleText $h)'")
            foreach ($extra in $ExtraHeader) { $lines.Add($extra) }
            $lines.Add("root | $($rootEl.Current.Name) | $($rootEl.Current.ControlType.ProgrammaticName)")
            function Walk-Dump($el, [int]$depth) {
                if ($depth -gt 10) { return }
                try { $c = $walker.GetFirstChild($el) } catch { return }
                while ($c) {
                    try {
                        $n = $c.Current
                        $type = $n.ControlType.ProgrammaticName.Replace('ControlType.', '')
                        $lines.Add(('  ' * $depth) + "$type | $($n.Name) | id=$($n.AutomationId) | focusable=$($n.IsKeyboardFocusable) | enabled=$($n.IsEnabled)")
                        $script:scenarioDumpNodes++
                        Walk-Dump $c ($depth + 1)
                        $c = $walker.GetNextSibling($c)
                    } catch {
                        break
                    }
                }
            }
            Walk-Dump $rootEl 0
            $lines | Set-Content -LiteralPath $path -Encoding UTF8
            Write-Log "scenario uia-dump label=$label nodes=$script:scenarioDumpNodes file=$path" | Out-Null
            return $script:scenarioDumpNodes
        }

        # ---- Round 14: capture-on-state ----
        # The requested labels are matched against the status strip's accessible name. The first
        # sample that matches a label grabs the pixels immediately (no focus call, no sleep), then
        # walks UIA and saves the PNG. Offsets are measured from the trigger stopwatch.
        function Get-StateKey([string]$state) { return $state.Trim().ToLowerInvariant() }
        function Get-StateBare([string]$state) {
            $s = $state.Trim()
            if ($s -match '(?i)^run\s+(.+)$') { return $Matches[1].Trim() }
            return $s
        }
        function Get-StateSlug([string]$state) {
            $bare = Get-StateBare $state
            $slug = ($bare -replace '[^A-Za-z0-9]+', '-').Trim('-').ToLowerInvariant()
            return "run-$slug"
        }
        function Test-StateMatch([string]$status, [string]$requested) {
            $bare = Get-StateBare $requested
            return ($status -match ('(?i)\brun\s+' + [regex]::Escape($bare) + '\b'))
        }

        $script:requestedStates = New-Object System.Collections.Generic.List[string]
        foreach ($captureSpec in $CaptureStates) {
            foreach ($piece in ($captureSpec -split ',')) {
                $trimmed = $piece.Trim()
                if ($trimmed -and -not ($script:requestedStates -contains $trimmed)) {
                    $script:requestedStates.Add($trimmed) | Out-Null
                }
            }
        }
        $script:captureRecords = @{}
        $script:captureErrors = New-Object System.Collections.Generic.List[object]
        $script:submitGrabs = New-Object System.Collections.Generic.List[object]
        $script:stateAttempts = @{}
        foreach ($st in $script:requestedStates) { $script:stateAttempts[$st] = 0 }
        $script:currentAttempt = 0

        function Add-StateAttempts([string[]]$names) {
            foreach ($st in $script:requestedStates) {
                $bare = (Get-StateBare $st).ToLowerInvariant()
                foreach ($n in $names) {
                    if ($bare -eq $n.ToLowerInvariant()) {
                        $script:stateAttempts[$st] = [int]$script:stateAttempts[$st] + 1
                    }
                }
            }
        }

        function Get-MissingStates {
            $missing = New-Object System.Collections.Generic.List[string]
            foreach ($st in $script:requestedStates) {
                if (-not $script:captureRecords.ContainsKey((Get-StateKey $st))) { $missing.Add($st) | Out-Null }
            }
            return $missing
        }

        function Get-RunTraceTimings([string]$path) {
            # The product trace is the only sub-millisecond source for a state that starts and ends
            # between two UIA samples. It is written by the child when LOVELACE_RUN_TRACE is set.
            $runs = @{}
            if (-not $path -or -not (Test-Path -LiteralPath $path)) { return @() }
            foreach ($line in (Get-Content -LiteralPath $path)) {
                if ($line -match '^run_state_(\w+)\s+.*mono_us=(\d+).*\brun=(\d+)') {
                    $name = $Matches[1]
                    $mono = [int64]$Matches[2]
                    $runId = [int]$Matches[3]
                    if (-not $runs.ContainsKey($runId)) { $runs[$runId] = @{} }
                    if (-not $runs[$runId].ContainsKey($name)) { $runs[$runId][$name] = $mono }
                }
            }
            $out = New-Object System.Collections.Generic.List[object]
            foreach ($runId in ($runs.Keys | Sort-Object)) {
                $m = $runs[$runId]
                $queuedMs = -1.0
                if ($m.ContainsKey('queued') -and $m.ContainsKey('running')) {
                    $queuedMs = [math]::Round(($m['running'] - $m['queued']) / 1000.0, 3)
                }
                $cancellingMs = -1.0
                if ($m.ContainsKey('cancelling') -and $m.ContainsKey('cancelled')) {
                    $cancellingMs = [math]::Round(($m['cancelled'] - $m['cancelling']) / 1000.0, 3)
                }
                $out.Add([pscustomobject]@{ Run = $runId; QueuedMs = $queuedMs; CancellingMs = $cancellingMs }) | Out-Null
            }
            return $out
        }

        function Invoke-StateCapture([string]$requested, [string]$statusAtMatch, [double]$matchMs, [string]$triggerAction, [System.Diagnostics.Stopwatch]$triggerSw) {
            $key = Get-StateKey $requested
            if ($script:captureRecords.ContainsKey($key)) { return }
            $slug = Get-StateSlug $requested
            $pngPath = Join-Path $OutDir "$slug.png"
            $uiaPath = Join-Path $OutDir "$slug-uia.txt"
            $record = [pscustomobject]@{
                State = $requested; StatusAtMatch = $statusAtMatch; Trigger = $triggerAction
                Attempt = $script:currentAttempt; MatchMs = $matchMs
                GrabMs = -1.0; DumpMs = -1.0; TotalMs = -1.0
                Png = $pngPath; PngBytes = 0; Nodes = -1; Uia = $uiaPath
                Frames = ''; Wall = (Get-Date).ToString('HH:mm:ss.fff'); Error = ''
            }
            try {
                # Pixels first: no focus call and no sleep, so the grab lands within a few ms of the
                # sample. The bitmap is held in memory while the live UIA tree is walked.
                if ($WakeRepaint) { Wake-Repaint }
                $burst = [int][Math]::Max(1, $CaptureBurst)
                $frames = New-Object System.Collections.Generic.List[object]
                for ($fi = 0; $fi -lt $burst; $fi++) {
                    if ($fi -gt 0) { Wait-SpinMs 8 }
                    $r = New-Object GuiRect
                    if (-not [GuiNative]::GetWindowRect($h, [ref]$r)) { throw 'GetWindowRect failed for state capture' }
                    $w = [int]($r.Right - $r.Left); $ht = [int]($r.Bottom - $r.Top)
                    if ($w -lt 400 -or $ht -lt 300) { throw "refusing state capture with rect ${w}x${ht}" }
                    $bmp = New-Object System.Drawing.Bitmap($w, $ht)
                    $g = [System.Drawing.Graphics]::FromImage($bmp)
                    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size($w, $ht)))
                    $g.Dispose()
                    $framePath = if ($fi -eq 0) { $pngPath } else { Join-Path $OutDir ("{0}-frame{1}.png" -f $slug, ($fi + 1)) }
                    $frames.Add([pscustomobject]@{
                        Bitmap = $bmp; Path = $framePath
                        GrabMs = [math]::Round($triggerSw.Elapsed.TotalMilliseconds, 1)
                    }) | Out-Null
                }
                $record.GrabMs = $frames[0].GrabMs
                $header = @(
                    "state_capture requested='$requested' status_at_trigger='$statusAtMatch'",
                    "trigger=$triggerAction attempt=$($script:currentAttempt) match_ms=$matchMs grab_ms=$($record.GrabMs) frames=$burst",
                    'note: this tree is walked after the PNG grab; the status line may show the successor state'
                )
                $record.Nodes = Dump-UiaTree $uiaPath "state-$slug" $header
                $record.DumpMs = [math]::Round($triggerSw.Elapsed.TotalMilliseconds, 1)
                $frameInfo = New-Object System.Collections.Generic.List[string]
                $firstBytes = 0
                for ($fi = 0; $fi -lt $frames.Count; $fi++) {
                    $fr = $frames[$fi]
                    $fr.Bitmap.Save($fr.Path, [System.Drawing.Imaging.ImageFormat]::Png)
                    $fr.Bitmap.Dispose()
                    $bytes = (Get-Item -LiteralPath $fr.Path).Length
                    if ($bytes -le 20480) { throw "state capture frame $($fi + 1) is only $bytes bytes (< 20 KB): $($fr.Path)" }
                    if ($fi -eq 0) { $firstBytes = $bytes }
                    $frameInfo.Add("$($fr.Path)|$bytes|$($fr.GrabMs)") | Out-Null
                }
                $record.PngBytes = $firstBytes
                $record.Frames = ($frameInfo -join ';')
                $record.TotalMs = [math]::Round($triggerSw.Elapsed.TotalMilliseconds, 1)
                $script:captureRecords[$key] = $record
                Write-Log "scenario capture-state state='$requested' attempt=$($script:currentAttempt) trigger=$triggerAction match_ms=$matchMs grab_ms=$($record.GrabMs) dump_ms=$($record.DumpMs) total_ms=$($record.TotalMs) png=$pngPath bytes=$($record.PngBytes) nodes=$($record.Nodes) frames=$burst"
            } catch {
                $record.Error = $_.Exception.Message
                if ($bmp) { $bmp.Dispose() }
                $script:captureErrors.Add($record) | Out-Null
                Write-Log "scenario capture-state-error state='$requested' attempt=$($script:currentAttempt) error='$($record.Error)'"
            }
        }

        function Invoke-StateCaptures([string]$status, [double]$matchMs, [System.Diagnostics.Stopwatch]$triggerSw, [string]$triggerAction) {
            if ($script:requestedStates.Count -eq 0) { return }
            foreach ($st in $script:requestedStates) {
                if ($script:captureRecords.ContainsKey((Get-StateKey $st))) { continue }
                if (Test-StateMatch $status $st) {
                    Invoke-StateCapture $st $status $matchMs $triggerAction $triggerSw
                }
            }
        }

        function Restore-EditorFocus([string]$label) {
            $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
            $e = Get-EditorElement $rootEl
            if (-not $e) { throw "scenario: editor element not found for $label" }
            $er2 = $e.Current.BoundingRectangle
            if ($er2.Width -le 0 -or $er2.Height -le 0) { throw "scenario: editor has no bounding rect for $label" }
            Click-At ([int]($er2.Right - 30)) ([int]($er2.Top + 5))
            $n = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" $label
            Write-Log "scenario refocus label=$label editor='$n'"
            if ($n -notmatch "caret line 1 column $expectedColumn`$") {
                throw "scenario: refocus did not park the caret at line 1 column $expectedColumn; editor='$n'"
            }
        }

        # The application opens on the 6-line starter script. Select the empty second tab so the
        # ONE graded statement is the whole document; Ctrl+A is not implemented by this editor.
        # A first click can be consumed as the window-activation click when the launch environment
        # has just shown the window, so verify the editor and retry with a freshly read tab rect.
        try {
            for ($tabAttempt = 1; $tabAttempt -le 4; $tabAttempt++) {
                $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
                $tab = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,
                    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'untitled.lv')))
                if (-not $tab) { throw 'scenario: no untitled.lv tab in the UIA tree' }
                $tr = $tab.Current.BoundingRectangle
                if ($tr.Width -le 0 -or $tr.Height -le 0) { throw "scenario: untitled.lv tab has no bounding rect ($tr)" }
                $tabX = [int]($tr.Left + ($tr.Width / 2)); $tabY = [int]($tr.Top + ($tr.Height / 2))
                if (-not (Focus-Window $h)) { throw 'scenario: cannot focus the window before selecting the empty tab' }
                Click-At $tabX $tabY
                Write-Log "scenario clicked tab untitled.lv at ${tabX},${tabY} rect=$tr attempt=$tabAttempt"
                Start-Sleep -Milliseconds 400
                $probeName = Get-EditorName
                if ($probeName -match '^Script editor, 1 lines') { break }
                Write-Log "scenario tab click attempt=$tabAttempt did not select the empty document; editor='$probeName'"
            }
        } catch {
            throw
        }
        $editorName = Wait-EditorNameMatch '^Script editor, 1 lines' 'empty-tab'
        Write-Log "scenario editor after tab select='$editorName'"
        if ($editorName -notmatch '^Script editor, 1 lines') {
            throw "scenario: the empty document was not selected; editor='$editorName'"
        }

        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $edit = Get-EditorElement $root
        if (-not $edit) { throw 'scenario: editor element not found after selecting the empty tab' }
        $er = $edit.Current.BoundingRectangle
        if ($er.Width -le 0 -or $er.Height -le 0) { throw "scenario: editor has no bounding rect ($er)" }
        $editorX = [int]($er.Left + 40); $editorY = [int]($er.Top + 40)
        Click-At $editorX $editorY
        Write-Log "scenario clicked editor at ${editorX},${editorY} rect=$er"
        Start-Sleep -Milliseconds 300
        Write-Log "scenario caret before typing='$(Get-EditorName)'"

        # 4. Type the ONE graded statement with real keystrokes; the editor has no ValuePattern.
        $expectedColumn = $scriptText.Length + 1
        $sendScript = ConvertTo-SendKeysLiteral $scriptText
        Send-Checked $sendScript 'script-text'
        $nameAfterType = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" 'after-script'
        Write-Log "scenario caret after typing statement='$nameAfterType'"
        if ($nameAfterType -notmatch "caret line 1 column $expectedColumn`$") {
            throw "scenario: the typed statement did not land; editor='$nameAfterType'"
        }
        Send-Checked '{ENTER}' 'enter'
        $nameAfterEnter = Wait-EditorNameMatch 'caret line 2 column 1$' 'after-enter'
        Write-Log "scenario caret after enter='$nameAfterEnter'"
        if ($nameAfterEnter -notmatch 'caret line 2 column 1$') {
            throw "scenario: Enter did not reach the editor; editor='$nameAfterEnter'"
        }
        # Park the caret at the end of the statement with a mouse click instead of a row-changing
        # key: Up steals egui focus in this build and Backspace trips a product crash, while a
        # click re-focuses the editor deterministically.
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $edit = Get-EditorElement $root
        $er = $edit.Current.BoundingRectangle
        $parkX = [int]($er.Right - 30); $parkY = [int]($er.Top + 5)
        Click-At $parkX $parkY
        Write-Log "scenario clicked line 1 end at ${parkX},${parkY} to park the caret"
        $nameBeforeRun = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" 'before-run'
        Write-Log "scenario caret before run='$nameBeforeRun'"
        if ($nameBeforeRun -notmatch "caret line 1 column $expectedColumn`$") {
            throw "scenario: could not park the caret at line 1 column $expectedColumn; editor='$nameBeforeRun'"
        }

        # 5. Run, log every status transition, and capture requested transient states.
        $transientMode = ($script:requestedStates.Count -gt 0 -or $Flow -ne 'cancel')
        Write-Log "scenario mode transient=$transientMode flow=$Flow requested='$($script:requestedStates -join ',')' max_attempts=$MaxAttempts fast_keys=$FastKeys"
        $transitions = New-Object System.Collections.Generic.List[object]
        $initial = ''
        for ($i = 0; $i -lt 25; $i++) { $initial = Read-Status; if ($initial) { break }; Start-Sleep -Milliseconds 200 }
        if (-not $initial) { throw 'scenario: status strip text not found after UIA readiness' }
        Write-Log "scenario initial state='$initial'"
        if ($FastStatus) {
            $rootEl = [System.Windows.Automation.AutomationElement]::FromHandle($h)
            $script:statusElement = Get-StatusElement $rootEl
            if ($script:statusElement) {
                Write-Log "scenario fast-status cached element name='$($script:statusElement.Current.Name)'"
            } else {
                Write-Log 'scenario fast-status could not cache the status element; falling back to tree reads'
            }
        }
        $sampleDelayMs = if ($FastStatus) { 1 } else { 150 }
        $transitions.Add([pscustomobject]@{ Phase = 'pre-run'; Wall = (Get-Date).ToString('HH:mm:ss.fff'); Ms = ''; State = $initial })
        $lastState = $initial
        $script:transitions = $transitions
        $script:lastState = $initial
        # Defaults for the legacy metric block; each flow overwrites the ones it measures.
        $enterWall = ''; $enterSendMs = -1
        $escWall = ''; $escSendMs = -1
        $runningMs = -1; $runningQueryMs = -1; $runningWall = ''
        $runningBytes = 0; $runningNodes = -1; $capturedRunning = $false
        $cancelledMs = -1; $terminalQueryMs = -1; $terminalState = ''; $terminalWall = ''
        $caretAfterRun = ''; $caretRefocus = ''; $caretHome = ''; $caretEnd = ''
        $flowAttempts = 0

        if (-not $transientMode) {

        $enterWall = (Get-Date).ToString('HH:mm:ss.fff')
        $enterSw = [System.Diagnostics.Stopwatch]::StartNew()
        $sendSw = [System.Diagnostics.Stopwatch]::StartNew()
        Send-Checked '^{ENTER}' 'ctrl+enter'
        $enterSendMs = [math]::Round($sendSw.Elapsed.TotalMilliseconds, 1)
        Write-Log "scenario ctrl+enter sent wall=$enterWall send_call=${enterSendMs}ms (timed from before the send; sampling starts here)"
        $runningMs = -1
        $runningQueryMs = -1
        $runningWall = ''
        $runningBytes = 0
        $runningNodes = -1
        $capturedRunning = $false
        while ($enterSw.Elapsed.TotalSeconds -lt 30) {
            $querySw = [System.Diagnostics.Stopwatch]::StartNew()
            $s = Read-StatusSample
            $queryMs = [math]::Round($querySw.Elapsed.TotalMilliseconds, 1)
            $ms = [math]::Round($enterSw.Elapsed.TotalMilliseconds, 1)
            if ($s -and $s -ne $lastState) {
                $transitions.Add([pscustomobject]@{ Phase = 'ctrl+enter'; Wall = (Get-Date).ToString('HH:mm:ss.fff'); Ms = $ms; State = $s })
                Write-Log "scenario transition phase=ctrl+enter elapsed=${ms}ms query=${queryMs}ms state='$s'"
                $lastState = $s
            }
            if ($s -match 'run Running' -and -not $capturedRunning) {
                $runningMs = $ms
                $runningQueryMs = $queryMs
                $runningWall = (Get-Date).ToString('HH:mm:ss.fff')
                Write-Log "scenario observed Running elapsed=${ms}ms query=${queryMs}ms"
                $runningBytes = Save-Shot $runPng 'scenario-running'
                $runningNodes = Dump-UiaTree $runUia 'scenario-running'
                $capturedRunning = $true
                # Let the single kernel get going, so Esc lands inside it rather than at a boundary.
                Start-Sleep -Milliseconds 300
                break
            }
            if ($s -match 'run (Cancelled|Finished|Failed)') { break }
            Start-Sleep -Milliseconds $sampleDelayMs
        }
        if (-not $capturedRunning) {
            throw "scenario: never observed 'run Running' (terminal or 30 s before it); last='$lastState'"
        }

        $escWall = (Get-Date).ToString('HH:mm:ss.fff')
        $escSw = [System.Diagnostics.Stopwatch]::StartNew()
        $escSendSw = [System.Diagnostics.Stopwatch]::StartNew()
        Send-Checked '{ESC}' 'escape'
        $escSendMs = [math]::Round($escSendSw.Elapsed.TotalMilliseconds, 1)
        Write-Log "scenario escape sent wall=$escWall send_call=${escSendMs}ms"
        $cancelledMs = -1
        $terminalQueryMs = -1
        $terminalState = ''
        $terminalWall = ''
        while ($escSw.Elapsed.TotalSeconds -lt 8) {
            $querySw = [System.Diagnostics.Stopwatch]::StartNew()
            $s = Read-StatusSample
            $queryMs = [math]::Round($querySw.Elapsed.TotalMilliseconds, 1)
            $ms = [math]::Round($escSw.Elapsed.TotalMilliseconds, 1)
            if ($s -and $s -ne $lastState) {
                $transitions.Add([pscustomobject]@{ Phase = 'escape'; Wall = (Get-Date).ToString('HH:mm:ss.fff'); Ms = $ms; State = $s })
                Write-Log "scenario transition phase=escape elapsed=${ms}ms query=${queryMs}ms state='$s'"
                $lastState = $s
            }
            if ($s -match 'run (Cancelled|Finished|Failed)') {
                $cancelledMs = $ms
                $terminalQueryMs = $queryMs
                $terminalState = $s
                $terminalWall = (Get-Date).ToString('HH:mm:ss.fff')
                break
            }
            Start-Sleep -Milliseconds $sampleDelayMs
        }
        if ($cancelledMs -lt 0) {
            throw "scenario: no terminal state after Esc within 8 s; last='$lastState'"
        }
        Write-Log "scenario terminal elapsed=${cancelledMs}ms state='$terminalState'"
        if ($terminalState -notmatch 'run Cancelled') {
            throw "scenario: expected run Cancelled after Esc, observed '$terminalState'"
        }

        $finalBytes = Save-Shot $finalPng 'scenario-final'
        $finalNodes = Dump-UiaTree $finalUia 'scenario-final'

        # 6. Probe caret tracking: the editor's accessible name carries line/column. The run
        #    leaves egui without keyboard focus in this build, so click the editor once to restore
        #    it; the click also parks the caret at the end of the statement.
        $caretAfterRun = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" 'after-run'
        Write-Log "scenario caret after run='$caretAfterRun'"
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $edit = Get-EditorElement $root
        $er = $edit.Current.BoundingRectangle
        $focusX = [int]($er.Right - 30); $focusY = [int]($er.Top + 5)
        Click-At $focusX $focusY
        Write-Log "scenario clicked editor at ${focusX},${focusY} to restore keyboard focus after the run"
        Start-Sleep -Milliseconds 250
        $caretRefocus = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" 'after-refocus'
        Write-Log "scenario caret after refocus click='$caretRefocus'"
        Send-Checked '{HOME}' 'home'
        $caretHome = Wait-EditorNameMatch 'caret line 1 column 1$' 'after-home'
        Write-Log "scenario caret after Home='$caretHome'"
        Send-Checked '+{END}' 'shift+end'
        $caretEnd = Wait-EditorNameMatch "caret line 1 column $expectedColumn`$" 'after-shift-end'
        Write-Log "scenario caret after Shift+End='$caretEnd'"
        if ($caretAfterRun -eq $caretHome) { throw "scenario: Home did not move the caret ('$caretAfterRun')" }
        if ($caretHome -eq $caretEnd) { throw "scenario: Shift+End did not move the caret ('$caretHome')" }

        } else {
            # ---- Round 14 transient-state flows ----
            # Each flow records: the first sample offset, every status transition, the state capture
            # offsets and the per-attempt UIA transition gaps. The product trace is parsed later for
            # sub-millisecond durations (the UIA sampler is one thread and cannot see inside a frame).
            $script:attemptLog = New-Object System.Collections.Generic.List[object]
            $attemptLimit = [int][Math]::Max(1, $MaxAttempts)

            function Read-Sample([System.Diagnostics.Stopwatch]$sw) {
                $querySw = [System.Diagnostics.Stopwatch]::StartNew()
                $s = Read-StatusSample
                $queryMs = [math]::Round($querySw.Elapsed.TotalMilliseconds, 1)
                $ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1)
                return [pscustomobject]@{ Status = $s; Ms = $ms; QueryMs = $queryMs }
            }

            function Add-Transition([string]$phase, $sample) {
                if ($sample.Status -and $sample.Status -ne $script:lastState) {
                    $script:transitions.Add([pscustomobject]@{ Phase = $phase; Wall = (Get-Date).ToString('HH:mm:ss.fff'); Ms = $sample.Ms; State = $sample.Status })
                    Write-Log "scenario transition phase=$phase elapsed=$($sample.Ms)ms query=$($sample.QueryMs)ms state='$($sample.Status)'"
                    $script:lastState = $sample.Status
                }
            }

            if ($Flow -eq 'cancel') {
                $missing = @(Get-MissingStates)
                for ($attempt = 1; $attempt -le $attemptLimit; $attempt++) {
                    $script:currentAttempt = $attempt
                    $flowAttempts = $attempt
                    $missing = @(Get-MissingStates)
                    Write-Log "scenario attempt=$attempt flow=cancel remaining='$($missing -join ',')'"
                    if ($attempt -gt 1) {
                        Restore-EditorFocus "cancel-attempt-$attempt"
                        Start-Sleep -Milliseconds 100
                    }
                    Assert-InputFocus 'ctrl+enter' | Out-Null
                    Add-StateAttempts @('Queued', 'Running')
                    $enterWall = (Get-Date).ToString('HH:mm:ss.fff')
                    $enterSendSw = [System.Diagnostics.Stopwatch]::StartNew()
                    $enterSw = [System.Diagnostics.Stopwatch]::StartNew()
                    if ($FastKeys) { Send-CtrlEnterFast } else { [System.Windows.Forms.SendKeys]::SendWait('^{ENTER}') }
                    $enterSendMs = [math]::Round($enterSendSw.Elapsed.TotalMilliseconds, 1)
                    $sawRunning = $false
                    $earlyTerminal = ''
                    $queuedSampleMs = -1.0
                    $queuedGapMs = -1.0
                    while ($enterSw.Elapsed.TotalSeconds -lt 30) {
                        $pollSw = [System.Diagnostics.Stopwatch]::StartNew()
                        $sample = Read-Sample $enterSw
                        if ($sample.Status) {
                            Invoke-StateCaptures $sample.Status $sample.Ms $enterSw 'ctrl+enter'
                            if ($sample.Status -match 'run Queued' -and $queuedSampleMs -lt 0) { $queuedSampleMs = $sample.Ms }
                            if ($sample.Status -match 'run Running' -and $queuedSampleMs -ge 0 -and $queuedGapMs -lt 0) {
                                $queuedGapMs = [math]::Round($sample.Ms - $queuedSampleMs, 1)
                            }
                            Add-Transition 'ctrl+enter' $sample
                            if ($sample.Status -match 'run Running') {
                                $sawRunning = $true
                                $runningMs = $sample.Ms
                                $runningQueryMs = $sample.QueryMs
                                $runningWall = (Get-Date).ToString('HH:mm:ss.fff')
                                break
                            }
                            if ($sample.Status -match 'run (Cancelled|Finished|Failed)') { $earlyTerminal = $sample.Status; break }
                        }
                        Wait-PollGate $pollSw
                    }
                    if (-not $sawRunning) {
                        if ($earlyTerminal) { throw "scenario: run reached '$earlyTerminal' before Running on attempt $attempt" }
                        throw "scenario: never observed 'run Running' on attempt $attempt; last='$script:lastState'"
                    }
                    if ($runningBytes -eq 0) {
                        $runningBytes = Save-Shot $runPng 'scenario-running'
                        $runningNodes = Dump-UiaTree $runUia 'scenario-running'
                    }
                    Start-Sleep -Milliseconds $CancelAfterMs
                    Assert-InputFocus 'escape' | Out-Null
                    Add-StateAttempts @('Cancelling')
                    $escWall = (Get-Date).ToString('HH:mm:ss.fff')
                    $escSendSw = [System.Diagnostics.Stopwatch]::StartNew()
                    $escSw = [System.Diagnostics.Stopwatch]::StartNew()
                    if ($FastKeys) { Send-EscapeFast } else { [System.Windows.Forms.SendKeys]::SendWait('{ESC}') }
                    $escSendMs = [math]::Round($escSendSw.Elapsed.TotalMilliseconds, 1)
                    $terminal = $false
                    $terminalSleepMs = -1.0
                    $cancellingSampleMs = -1.0
                    $cancellingGapMs = -1.0
                    while ($escSw.Elapsed.TotalSeconds -lt 8) {
                        $pollSw = [System.Diagnostics.Stopwatch]::StartNew()
                        $sample = Read-Sample $escSw
                        if ($sample.Status) {
                            Invoke-StateCaptures $sample.Status $sample.Ms $escSw 'escape'
                            if ($sample.Status -match 'run Cancelling' -and $cancellingSampleMs -lt 0) { $cancellingSampleMs = $sample.Ms }
                            Add-Transition 'escape' $sample
                            if ($sample.Status -match 'run (Cancelled|Finished|Failed)') {
                                $cancelledMs = $sample.Ms
                                $terminalQueryMs = $sample.QueryMs
                                $terminalState = $sample.Status
                                $terminalWall = (Get-Date).ToString('HH:mm:ss.fff')
                                $terminal = $true
                                if ($cancellingSampleMs -ge 0) { $cancellingGapMs = [math]::Round($sample.Ms - $cancellingSampleMs, 1) }
                                break
                            }
                        }
                        Wait-PollGate $pollSw
                    }
                    if (-not $terminal) {
                        throw "scenario: no terminal state after Esc on attempt $attempt within 8 s; last='$script:lastState'"
                    }
                    if ($terminalState -notmatch 'run Cancelled') {
                        throw "scenario: expected run Cancelled after Esc, observed '$terminalState' on attempt $attempt"
                    }
                    $script:attemptLog.Add([pscustomobject]@{
                        Flow = 'cancel'; Attempt = $attempt; FirstStatus = ''; FirstMs = $enterSendMs
                        TerminalStatus = $terminalState; TerminalMs = $cancelledMs
                        QueuedObservedMs = $queuedSampleMs; QueuedGapMs = $queuedGapMs
                        CancellingObservedMs = $cancellingSampleMs; CancellingGapMs = $cancellingGapMs
                        Note = "ctrl+enter+${enterSendMs}ms esc+${escSendMs}ms"
                    }) | Out-Null
                    Write-Log "scenario attempt=$attempt flow=cancel ctrl_enter_sample_ms=$runningMs esc_send_ms=$escSendMs terminal_ms=$cancelledMs terminal='$terminalState' cancelling_sample_ms=$cancellingSampleMs cancelling_gap_ms=$cancellingGapMs"
                    $missing = @(Get-MissingStates)
                    if ($script:requestedStates.Count -gt 0 -and $missing.Count -eq 0) { break }
                    Write-Log "scenario attempt=$attempt flow=cancel missed='$($missing -join ',')'"
                    if ($attempt -lt $attemptLimit) { Start-Sleep -Milliseconds 150 }
                }
                $missing = @(Get-MissingStates)
                if ($missing.Count -gt 0) {
                    Write-Log "scenario capture-miss flow=cancel states='$($missing -join ',')' attempts=$flowAttempts"
                }
            } elseif ($Flow -eq 'queued') {
                $missing = @(Get-MissingStates)
                for ($attempt = 1; $attempt -le $attemptLimit; $attempt++) {
                    $script:currentAttempt = $attempt
                    $flowAttempts = $attempt
                    $missing = @(Get-MissingStates)
                    Write-Log "scenario attempt=$attempt flow=queued remaining='$($missing -join ',')'"
                    if ($attempt -gt 1) {
                        Restore-EditorFocus "queued-attempt-$attempt"
                        Start-Sleep -Milliseconds 100
                    }
                    Assert-InputFocus 'ctrl+enter' | Out-Null
                    Add-StateAttempts @('Queued', 'Running')
                    $submitFrames = New-Object System.Collections.Generic.List[object]
                    $sr = $null; $sw = 0; $sh = 0
                    if ($SubmitGrab) {
                        $sr = New-Object GuiRect
                        if ([GuiNative]::GetWindowRect($h, [ref]$sr)) {
                            $sw = [int]($sr.Right - $sr.Left); $sh = [int]($sr.Bottom - $sr.Top)
                            if ($sw -ge 400 -and $sh -ge 300) {
                                for ($fi = 0; $fi -lt 3; $fi++) {
                                    $sbmp = New-Object System.Drawing.Bitmap($sw, $sh)
                                    $sg = [System.Drawing.Graphics]::FromImage($sbmp)
                                    $submitFrames.Add([pscustomobject]@{ Bitmap = $sbmp; Graphics = $sg; GrabMs = -1.0 }) | Out-Null
                                }
                            }
                        }
                        if ($submitFrames.Count -eq 0) { Write-Log "scenario submit-grab attempt=$attempt skipped: no usable rect/bitmap" }
                    }
                    $enterWall = (Get-Date).ToString('HH:mm:ss.fff')
                    $enterSendSw = [System.Diagnostics.Stopwatch]::StartNew()
                    $enterSw = [System.Diagnostics.Stopwatch]::StartNew()
                    if ($FastKeys) { Send-CtrlEnterFast } else { [System.Windows.Forms.SendKeys]::SendWait('^{ENTER}') }
                    $enterSendMs = [math]::Round($enterSendSw.Elapsed.TotalMilliseconds, 1)
                    if ($submitFrames.Count -gt 0) {
                        for ($fi = 0; $fi -lt $submitFrames.Count; $fi++) {
                            $sf = $submitFrames[$fi]
                            $sf.Graphics.CopyFromScreen($sr.Left, $sr.Top, 0, 0, (New-Object System.Drawing.Size($sw, $sh)))
                            $sf.Graphics.Dispose()
                            $sf.GrabMs = [math]::Round($enterSw.Elapsed.TotalMilliseconds, 1)
                            Write-Log "scenario submit-grab attempt=$attempt frame=$($fi + 1) grabbed at $($sf.GrabMs)ms after submit"
                        }
                    }
                    $first = $null
                    $terminal = $false
                    $terminalMs = -1.0
                    $terminalState = ''
                    $queuedSampleMs = -1.0
                    $queuedGapMs = -1.0
                    while ($enterSw.Elapsed.TotalSeconds -lt 10) {
                        $pollSw = [System.Diagnostics.Stopwatch]::StartNew()
                        $sample = Read-Sample $enterSw
                        if (-not $first) {
                            $first = $sample
                            Write-Log "scenario first-sample flow=queued attempt=$attempt elapsed=$($sample.Ms)ms query=$($sample.QueryMs)ms state='$($sample.Status)'"
                        }
                        if ($sample.Status) {
                            Invoke-StateCaptures $sample.Status $sample.Ms $enterSw 'ctrl+enter'
                            if ($sample.Status -match 'run Queued' -and $queuedSampleMs -lt 0) { $queuedSampleMs = $sample.Ms }
                            if ($sample.Status -match 'run Running' -and $queuedSampleMs -ge 0 -and $queuedGapMs -lt 0) {
                                $queuedGapMs = [math]::Round($sample.Ms - $queuedSampleMs, 1)
                            }
                            Add-Transition 'ctrl+enter' $sample
                            if ($sample.Status -match 'run (Cancelled|Finished|Failed)') {
                                $terminal = $true
                                $terminalMs = $sample.Ms
                                $terminalState = $sample.Status
                                break
                            }
                        }
                        Wait-PollGate $pollSw
                    }
                    if (-not $terminal) {
                        throw "scenario: queued attempt $attempt saw no terminal state within 10 s; last='$script:lastState'"
                    }
                    if ($submitFrames.Count -gt 0) {
                        for ($fi = 0; $fi -lt $submitFrames.Count; $fi++) {
                            $sf = $submitFrames[$fi]
                            $submitPath = Join-Path $OutDir ("run-queued-submit-attempt{0}-f{1}.png" -f $attempt, ($fi + 1))
                            $sf.Bitmap.Save($submitPath, [System.Drawing.Imaging.ImageFormat]::Png)
                            $sf.Bitmap.Dispose()
                            $submitBytes = (Get-Item -LiteralPath $submitPath).Length
                            $script:submitGrabs.Add([pscustomobject]@{ Attempt = $attempt; Frame = ($fi + 1); GrabMs = $sf.GrabMs; Path = $submitPath; Bytes = $submitBytes }) | Out-Null
                            Write-Log "scenario submit-grab attempt=$attempt frame=$($fi + 1) grab_ms=$($sf.GrabMs) file=$submitPath bytes=$submitBytes"
                        }
                    }
                    $script:attemptLog.Add([pscustomobject]@{
                        Flow = 'queued'; Attempt = $attempt; FirstStatus = $first.Status; FirstMs = $first.Ms
                        TerminalStatus = $terminalState; TerminalMs = $terminalMs
                        QueuedObservedMs = $queuedSampleMs; QueuedGapMs = $queuedGapMs
                        CancellingObservedMs = -1.0; CancellingGapMs = -1.0
                        Note = "ctrl+enter_send=${enterSendMs}ms first_query=$($first.QueryMs)ms"
                    }) | Out-Null
                    Write-Log "scenario attempt=$attempt flow=queued first_ms=$($first.Ms) first='$($first.Status)' terminal_ms=$terminalMs terminal='$terminalState' queued_sample_ms=$queuedSampleMs queued_gap_ms=$queuedGapMs"
                    $missing = @(Get-MissingStates)
                    if ($script:requestedStates.Count -gt 0 -and $missing.Count -eq 0) { break }
                    if ($attempt -lt $attemptLimit) { Start-Sleep -Milliseconds 150 }
                }
                $missing = @(Get-MissingStates)
                if ($missing.Count -gt 0) {
                    Write-Log "scenario capture-miss flow=queued states='$($missing -join ',')' attempts=$flowAttempts"
                }
            } elseif ($Flow -eq 'stale') {
                $script:currentAttempt = 1
                $flowAttempts = 1
                Assert-InputFocus 'ctrl+enter' | Out-Null
                Add-StateAttempts @('Queued', 'Running')
                $enterWall = (Get-Date).ToString('HH:mm:ss.fff')
                $enterSendSw = [System.Diagnostics.Stopwatch]::StartNew()
                $enterSw = [System.Diagnostics.Stopwatch]::StartNew()
                [System.Windows.Forms.SendKeys]::SendWait('^{ENTER}')
                $enterSendMs = [math]::Round($enterSendSw.Elapsed.TotalMilliseconds, 1)
                $finished = $false
                $finishedMs = -1.0
                $first = $null
                $queuedSampleMs = -1.0
                $queuedGapMs = -1.0
                while ($enterSw.Elapsed.TotalSeconds -lt 10) {
                    $pollSw = [System.Diagnostics.Stopwatch]::StartNew()
                    $sample = Read-Sample $enterSw
                    if (-not $first) { $first = $sample }
                    if ($sample.Status) {
                        Invoke-StateCaptures $sample.Status $sample.Ms $enterSw 'ctrl+enter'
                        if ($sample.Status -match 'run Queued' -and $queuedSampleMs -lt 0) { $queuedSampleMs = $sample.Ms }
                        if ($sample.Status -match 'run Running' -and $queuedSampleMs -ge 0 -and $queuedGapMs -lt 0) {
                            $queuedGapMs = [math]::Round($sample.Ms - $queuedSampleMs, 1)
                        }
                        Add-Transition 'ctrl+enter' $sample
                        if ($sample.Status -match 'run Finished') { $finished = $true; $finishedMs = $sample.Ms; break }
                        if ($sample.Status -match 'run (Cancelled|Failed)') { throw "scenario: stale flow run ended '$($sample.Status)' instead of Finished" }
                    }
                    Wait-PollGate $pollSw
                }
                if (-not $finished) { throw "scenario: no run Finished within 10 s; last='$script:lastState'" }
                Write-Log "scenario observed Finished elapsed=${finishedMs}ms state='$script:lastState'"
                Restore-EditorFocus 'before-stale-edit'
                Start-Sleep -Milliseconds 150
                Assert-InputFocus 'stale-edit' | Out-Null
                Add-StateAttempts @('Stale')
                $editWall = (Get-Date).ToString('HH:mm:ss.fff')
                $editSendSw = [System.Diagnostics.Stopwatch]::StartNew()
                $editSw = [System.Diagnostics.Stopwatch]::StartNew()
                [System.Windows.Forms.SendKeys]::SendWait($StaleChar)
                $editSendMs = [math]::Round($editSendSw.Elapsed.TotalMilliseconds, 1)
                $stale = $false
                $staleMs = -1.0
                while ($editSw.Elapsed.TotalSeconds -lt 5) {
                    $pollSw = [System.Diagnostics.Stopwatch]::StartNew()
                    $sample = Read-Sample $editSw
                    if ($sample.Status) {
                        Invoke-StateCaptures $sample.Status $sample.Ms $editSw 'stale-edit'
                        Add-Transition 'stale-edit' $sample
                        if ($sample.Status -match 'run Stale') { $stale = $true; $staleMs = $sample.Ms; break }
                    }
                    Wait-PollGate $pollSw
                }
                $script:attemptLog.Add([pscustomobject]@{
                    Flow = 'stale'; Attempt = 1; FirstStatus = $first.Status; FirstMs = $first.Ms
                    TerminalStatus = $(if ($stale) { 'run Stale' } else { $script:lastState }); TerminalMs = $staleMs
                    QueuedObservedMs = $queuedSampleMs; QueuedGapMs = $queuedGapMs
                    CancellingObservedMs = -1.0; CancellingGapMs = -1.0
                    Note = "finished_at=${finishedMs}ms edit_send=${editSendMs}ms edit_char='$StaleChar'"
                }) | Out-Null
                Write-Log "scenario flow=stale finished_ms=$finishedMs stale_ms=$staleMs stale=$stale edit_send_ms=$editSendMs"
                if (-not $stale) { Write-Log "scenario capture-miss flow=stale states='run Stale' attempts=1" }
            }

            $finalBytes = Save-Shot $finalPng 'scenario-final'
            $finalNodes = Dump-UiaTree $finalUia 'scenario-final'
        }

        # 7. Close the window, then publish the summary with the measured numbers and byte sizes.
        $graceful = $p.CloseMainWindow()
        if (-not $p.WaitForExit(5000)) {
            Stop-Process -Id $p.Id -Force
            Write-Log "scenario close pid=$($p.Id) forced"
        } else {
            Write-Log "scenario close pid=$($p.Id) graceful=$graceful"
        }

        $traceTimings = @(Get-RunTraceTimings $TraceFile)
        if ($transientMode) {
            $sb = New-Object System.Text.StringBuilder
            [void]$sb.AppendLine('# Round 14 transient-state GUI scenario evidence')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine("Generated: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss.fff zzz'))")
            [void]$sb.AppendLine("Flow: **$Flow**; attempts: $flowAttempts of $attemptLimit")
            [void]$sb.AppendLine("Driver: tools/gui-harness.ps1 -Action scenario -Flow $Flow -FastStatus -CaptureStates '$($script:requestedStates -join ',')' -MaxAttempts $MaxAttempts -FastKeys:$FastKeys -WakeRepaint:$WakeRepaint -CaptureBurst $CaptureBurst -SubmitGrab:$SubmitGrab")
            [void]$sb.AppendLine("Product: $Exe")
            [void]$sb.AppendLine("PID: $($p.Id) (closed at the end of the gesture)")
            [void]$sb.AppendLine("Driver log: $Log")
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Window and UIA readiness')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine("Window: hwnd=$h rect=$rect after ${windowReadyMs} ms (restored_from_minimized=$restored).")
            [void]$sb.AppendLine("UIA: $nodeCount descendants from FindAll(Descendants) after ${uiaMs} ms.")
            [void]$sb.AppendLine("Script: ``$scriptText`` (typed with SendKeys, foreground asserted before every send)")
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Sampler')
            [void]$sb.AppendLine('')
            if ($FastStatus) {
                [void]$sb.AppendLine('Fine sampler: one cached status element; the loop spins to a real 1 ms cadence. `Start-Sleep -Milliseconds 1` was measured at ~15.5 ms on this host, so the spike would have missed the 7 ms Queued window; the gate uses a Stopwatch spin instead. The read itself costs 0.3-1.2 ms.')
            } else {
                [void]$sb.AppendLine('Standard sampler: full UIA tree walk every 150 ms. Transient states are not resolvable in this mode.')
            }
            if ($FastKeys) {
                [void]$sb.AppendLine('Trigger keys are injected with `keybd_event` (non-blocking). `SendKeys.SendWait` was measured blocking for 30-53 ms, which is exactly the window in which Queued/Cancelling begin, so the sampler starts immediately after injection instead.')
            } else {
                [void]$sb.AppendLine('Trigger keys use the legacy blocking `SendKeys.SendWait`; transient states that begin during the send can be missed.')
            }
            if ($WakeRepaint) {
                [void]$sb.AppendLine("Pixels: at the matching sample the driver moves the pointer 3 px inside the window (a real input event that requests an egui repaint, with no click and no run-state change), then grabs $CaptureBurst frame(s) 8 ms apart. The product trace shows whether a frame was begun while the state was current; the PNG set is the pixel evidence for that timing.")
            } else {
                [void]$sb.AppendLine('Pixels: one immediate grab at the matching sample, without waking the renderer.')
            }
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Requested states')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('| state | attempts | captured | attempt | match ms | grab ms | dump ms | PNG | bytes | UIA nodes | note |')
            [void]$sb.AppendLine('| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |')
            foreach ($st in $script:requestedStates) {
                $key = Get-StateKey $st
                $attempts = [int]$script:stateAttempts[$st]
                if ($script:captureRecords.ContainsKey($key)) {
                    $rec = $script:captureRecords[$key]
                    $note = if ($rec.Error) { $rec.Error } else { "status='$($rec.StatusAtMatch.Replace('|', '\|'))' trigger=$($rec.Trigger)" }
                    [void]$sb.AppendLine("| $st | $attempts | yes | $($rec.Attempt) | $($rec.MatchMs) | $($rec.GrabMs) | $($rec.DumpMs) | $($rec.Png) | $($rec.PngBytes) | $($rec.Nodes) | $note |")
                } else {
                    $err = $script:captureErrors | Where-Object { $_.State -eq $st } | Select-Object -First 1
                    $note = if ($err) { "capture error: $($err.Error)" } else { 'not observed in the sampled status text' }
                    [void]$sb.AppendLine("| $st | $attempts | no | - | - | - | - | - | - | - | $note |")
                }
            }
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Per-attempt sampling observations')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('| flow | attempt | first sample | terminal | Queued sampled at | Queued->Running gap | Cancelling sampled at | Cancelling->terminal gap | note |')
            [void]$sb.AppendLine('| --- | --- | --- | --- | --- | --- | --- | --- | --- |')
            foreach ($a in $script:attemptLog) {
                $firstCell = if ($a.FirstStatus) { "$($a.FirstMs) ms, $($a.FirstStatus.Replace('|', '\|'))" } else { '-' }
                $termCell = if ($a.TerminalStatus) { "$($a.TerminalMs) ms, $($a.TerminalStatus.Replace('|', '\|'))" } else { '-' }
                $qCell = if ($a.QueuedObservedMs -ge 0) { "$($a.QueuedObservedMs) ms" } else { '-' }
                $qgCell = if ($a.QueuedGapMs -ge 0) { "$($a.QueuedGapMs) ms" } else { '-' }
                $cCell = if ($a.CancellingObservedMs -ge 0) { "$($a.CancellingObservedMs) ms" } else { '-' }
                $cgCell = if ($a.CancellingGapMs -ge 0) { "$($a.CancellingGapMs) ms" } else { '-' }
                [void]$sb.AppendLine("| $($a.Flow) | $($a.Attempt) | $firstCell | $termCell | $qCell | $qgCell | $cCell | $cgCell | $($a.Note) |")
            }
            [void]$sb.AppendLine('')
            if ($script:submitGrabs.Count -gt 0) {
                [void]$sb.AppendLine('## Submit-triggered pixel grabs')
                [void]$sb.AppendLine('')
                [void]$sb.AppendLine('Preallocated bitmap copied immediately after the Ctrl+Enter injection, before the UIA label can catch up.')
                [void]$sb.AppendLine('')
                [void]$sb.AppendLine('| attempt | frame | grab ms after submit | bytes | file |')
                [void]$sb.AppendLine('| --- | --- | --- | --- | --- |')
                foreach ($g in $script:submitGrabs) {
                    [void]$sb.AppendLine("| $($g.Attempt) | $($g.Frame) | $($g.GrabMs) | $($g.Bytes) | $($g.Path) |")
                }
                [void]$sb.AppendLine('')
            }
            if ($traceTimings.Count -gt 0) {
                [void]$sb.AppendLine('## Product trace timings (sub-millisecond source)')
                [void]$sb.AppendLine('')
                [void]$sb.AppendLine('| run | Queued (ms) | Cancelling (ms) |')
                [void]$sb.AppendLine('| --- | --- | --- |')
                foreach ($t in $traceTimings) {
                    $q = if ($t.QueuedMs -ge 0) { $t.QueuedMs } else { '-' }
                    $c = if ($t.CancellingMs -ge 0) { $t.CancellingMs } else { '-' }
                    [void]$sb.AppendLine("| $($t.Run) | $q | $c |")
                }
                $queuedValues = @($traceTimings | Where-Object { $_.QueuedMs -ge 0 } | ForEach-Object { $_.QueuedMs })
                if ($queuedValues.Count -gt 0) {
                    $qMin = ($queuedValues | Measure-Object -Minimum).Minimum
                    $qMax = ($queuedValues | Measure-Object -Maximum).Maximum
                    [void]$sb.AppendLine('')
                    [void]$sb.AppendLine("Queued duration: min $qMin ms, max $qMax ms across $($queuedValues.Count) run(s).")
                }
                [void]$sb.AppendLine('')
                [void]$sb.AppendLine("Trace: $TraceFile")
            }
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Logged status transitions')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('| phase | wall clock | elapsed ms | status |')
            [void]$sb.AppendLine('| --- | --- | --- | --- |')
            foreach ($t in $transitions) {
                $stateCell = $t.State.Replace('|', '\|')
                [void]$sb.AppendLine("| $($t.Phase) | $($t.Wall) | $($t.Ms) | $stateCell |")
            }
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Captured frames')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('| state | frame | grab ms | bytes | file |')
            [void]$sb.AppendLine('| --- | --- | --- | --- | --- |')
            foreach ($rec in ($script:captureRecords.Values | Sort-Object MatchMs)) {
                $fi = 0
                foreach ($part in ($rec.Frames -split ';')) {
                    if (-not $part) { continue }
                    $cols = $part -split '\|'
                    if ($cols.Count -ge 3) {
                        $fi++
                        [void]$sb.AppendLine("| $($rec.State) | $fi | $($cols[2]) | $($cols[1]) | $($cols[0]) |")
                    }
                }
            }
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('## Artifacts')
            [void]$sb.AppendLine('')
            [void]$sb.AppendLine('| file | size / node count |')
            [void]$sb.AppendLine('| --- | --- |')
            if ($runningBytes -gt 0) { [void]$sb.AppendLine("| scenario-running.png | $runningBytes bytes |") }
            [void]$sb.AppendLine("| scenario-final.png | $finalBytes bytes |")
            foreach ($rec in ($script:captureRecords.Values | Sort-Object MatchMs)) {
                [void]$sb.AppendLine("| $($rec.Png) | $($rec.PngBytes) bytes |")
                [void]$sb.AppendLine("| $($rec.Uia) | $($rec.Nodes) UIA nodes |")
            }
            [void]$sb.AppendLine("| scenario-final-uia.txt | $finalNodes UIA nodes |")
            [void]$sb.AppendLine('| scenario-summary.md | this file |')
            $sb.ToString() | Set-Content -LiteralPath $summaryPath -Encoding UTF8
            Write-Log "scenario summary file=$summaryPath"
            Write-Log "scenario done flow=$Flow attempts=$flowAttempts captures=$($script:captureRecords.Count) requested=$($script:requestedStates.Count)"
            Write-Output "SCENARIO_OK FLOW=$Flow ATTEMPTS=$flowAttempts CAPTURED=$($script:captureRecords.Count) REQUESTED=$($script:requestedStates.Count) SUMMARY=$summaryPath"
        } else {
        $sb = New-Object System.Text.StringBuilder
        [void]$sb.AppendLine('# Round 10 GUI scenario evidence')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine("Generated: $((Get-Date).ToString('yyyy-MM-dd HH:mm:ss.fff zzz'))")
        [void]$sb.AppendLine("Driver: tools/gui-harness.ps1 -Action scenario")
        [void]$sb.AppendLine("Product: $Exe")
        [void]$sb.AppendLine("PID: $($p.Id) (closed at the end of the gesture)")
        [void]$sb.AppendLine("Driver log: $Log")
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('## Window and UIA readiness')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine("Window: hwnd=$h rect=$rect after ${windowReadyMs} ms (restored_from_minimized=$restored).")
        [void]$sb.AppendLine("UIA: $nodeCount descendants from FindAll(Descendants) after ${uiaMs} ms; the root was re-acquired from hwnd after the window was shown.")
        [void]$sb.AppendLine("Typed with SendKeys (foreground asserted before every send): ``$scriptText`` + Enter")
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('## Logged state transitions')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('| phase | wall clock | elapsed ms | status |')
        [void]$sb.AppendLine('| --- | --- | --- | --- |')
        foreach ($t in $transitions) {
            $stateCell = $t.State.Replace('|', '\|')
            [void]$sb.AppendLine("| $($t.Phase) | $($t.Wall) | $($t.Ms) | $stateCell |")
        }
        [void]$sb.AppendLine('')
        $samplerMode = if ($FastStatus) { 'fine (one cached status element, 1 ms sleep)' } else { 'standard (full UIA tree walk, 150 ms sleep)' }
        [void]$sb.AppendLine("Sampler: $samplerMode")
        if ($TraceFile -ne '') { [void]$sb.AppendLine("Run-start trace: $TraceFile (monotonic us + UTC ms per mark; see docs/goal-desktop-v4/round-12/run-start.md)") }
        [void]$sb.AppendLine("Measured Ctrl+Enter -> run Running: **$runningMs ms** ($enterWall -> $runningWall); the SendKeys call returned at ${enterSendMs} ms and the sample that observed the state took ${runningQueryMs} ms")
        [void]$sb.AppendLine("Measured Esc -> run Cancelled: **$cancelledMs ms** ($escWall -> $terminalWall); the SendKeys call returned at ${escSendMs} ms and the sample that observed the state took ${terminalQueryMs} ms")
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('## Caret accessible names')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine("- before the run: ``$nameBeforeRun``")
        [void]$sb.AppendLine("- after the run: ``$caretAfterRun``")
        [void]$sb.AppendLine("- after the refocus click: ``$caretRefocus``")
        [void]$sb.AppendLine("- after Home: ``$caretHome``")
        [void]$sb.AppendLine("- after Shift+End: ``$caretEnd``")
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('Note: the run leaves egui without keyboard focus in this build, so the driver clicks the editor once (the logged refocus click) before pressing Home and Shift+End. The names above show the restored focus still tracks the caret: column 27 -> column 1 -> column 27.')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('## Artifacts')
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('| file | size / node count |')
        [void]$sb.AppendLine('| --- | --- |')
        [void]$sb.AppendLine("| scenario-running.png | $runningBytes bytes (> 20 KB) |")
        [void]$sb.AppendLine("| scenario-final.png | $finalBytes bytes (> 20 KB) |")
        [void]$sb.AppendLine("| scenario-running-uia.txt | $runningNodes UIA nodes |")
        [void]$sb.AppendLine("| scenario-final-uia.txt | $finalNodes UIA nodes |")
        [void]$sb.AppendLine("| scenario-summary.md | this file |")
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine("Initial UIA tree after the window was shown: $nodeCount nodes.")
        $sb.ToString() | Set-Content -LiteralPath $summaryPath -Encoding UTF8
        Write-Log "scenario summary file=$summaryPath"
        Write-Log "scenario done ctrl+enter->running=${runningMs}ms esc->cancelled=${cancelledMs}ms run_png=$runningBytes final_png=$finalBytes run_uia=$runningNodes final_uia=$finalNodes"
        Write-Output "SCENARIO_OK RUNNING_MS=$runningMs CANCELLED_MS=$cancelledMs RUN_PNG=$runningBytes FINAL_PNG=$finalBytes RUN_UIA=$runningNodes FINAL_UIA=$finalNodes SUMMARY=$summaryPath"
        }
        break
    }
    'setvalue' {
        # Set a text control's value through UI Automation: no foreground ownership needed, so this
        # is reliable from a tool call that does not hold the keyboard focus.
        if ($Name -eq '') { throw 'setvalue needs -Name' }
        $p = Get-AppProcess
        $p.Refresh()
        $root = [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$p.MainWindowHandle)
        $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Edit)))
        $target = $null
        foreach ($e in $edits) { if ($e.Current.Name -like "$Name*") { $target = $e; break } }
        if (-not $target) { throw "no Edit whose name starts with '$Name'" }
        $pattern = $target.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        $pattern.SetValue($Keys)
        Write-Log "setvalue pid=$($p.Id) element='$($target.Current.Name)' chars=$($Keys.Length)"
        Write-Output "SET=$($Keys.Length)"
        break
    }
    'typeinto' {
        # Focus a named control by prefix, then type. The editor's accessible name changes with its
        # line/caret, so match the stable prefix and let the caller pass the rest of the keys.
        if ($Name -eq '') { throw 'typeinto needs -Name' }
        $p = Get-AppProcess
        $p.Refresh()
        $h = [IntPtr]$p.MainWindowHandle
        if (-not (Focus-Window $h)) { throw 'typeinto refused: could not bring the target window to the foreground' }
        Assert-Foreground $h 'typeinto'
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($h)
        $edits = $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,
            (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Edit)))
        $target = $null
        foreach ($e in $edits) { if ($e.Current.Name -like "$Name*") { $target = $e; break } }
        if (-not $target) { throw "no Edit whose name starts with '$Name'" }
        $target.SetFocus()
        Start-Sleep -Milliseconds 200
        if ($Keys -ne '') { [System.Windows.Forms.SendKeys]::SendWait($Keys) }
        Write-Log "typeinto pid=$($p.Id) element='$($target.Current.Name)' keys=$Keys"
        Write-Output "TYPED=$($target.Current.Name)"
        break
    }
    'close' {
        $p = Get-AppProcess
        $p.CloseMainWindow() | Out-Null
        $exited = $p.WaitForExit(5000)
        if (-not $exited) { Stop-Process -Id $p.Id -Force; Write-Log "close pid=$($p.Id) forced" }
        else { Write-Log "close pid=$($p.Id) graceful" }
        break
    }
}
