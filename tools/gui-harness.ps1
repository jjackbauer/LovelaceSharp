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
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateSet('launch', 'wait', 'capture', 'uia', 'click', 'keys', 'close', 'state', 'focus', 'typeinto', 'setvalue')]
    [string]$Action,
    [string]$Exe = '',
    [string[]]$Arguments = @(),
    [int]$ProcessId = 0,
    [string]$Title = '',
    [string]$Name = '',
    [string]$Keys = '',
    [string]$OutDir = '',
    [string]$Log = '',
    [int]$TimeoutSec = 30
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

function Focus-Window([IntPtr]$h) {
    # SetForegroundWindow can silently fail when the caller is not foreground. Promote the window
    # and verify: capturing or typing without owning the foreground produces evidence about the
    # wrong application, which is worse than no evidence.
    [GuiNative]::ShowWindow($h, 9) | Out-Null                      # SW_RESTORE
    [GuiNative]::SetWindowPos($h, $HWND_TOPMOST, 0, 0, 0, 0, $SWP_NOSIZE_MOVE) | Out-Null
    [GuiNative]::BringWindowToTop($h) | Out-Null
    [GuiNative]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 300
    $fg = [GuiNative]::GetForegroundWindow()
    return ($fg -eq $h)
}

function Assert-Foreground([IntPtr]$h, [string]$action) {
    $fg = [GuiNative]::GetForegroundWindow()
    if ($fg -ne $h) {
        $title = (Get-Process | Where-Object { $_.MainWindowHandle -eq $fg } | Select-Object -First 1).MainWindowTitle
        throw "$action refused: target hwnd $h does not own the foreground (foreground=$fg '$title')"
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
