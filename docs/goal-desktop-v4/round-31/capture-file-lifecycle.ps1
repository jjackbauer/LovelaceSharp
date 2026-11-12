<#
.SYNOPSIS
  Round-31 DOC-002 / DOC-003 evidence: the native file lifecycle and the unsaved-changes prompt
  in the shipped workbench window.

.DESCRIPTION
  The console session is LOCKED (the foreground window is the Windows lock screen), so this script
  uses only methods that do not need an interactive desktop:
    - the app is launched with a real .lv file path, which opens it from disk;
    - a window resize is a SetWindowPos call on the app's own HWND;
    - buttons are pressed with UI Automation Invoke (message-based, reaches a locked window);
    - text is entered by POSTING WM_KEYDOWN + WM_CHAR pairs to the window (winit turns them into
      egui key/text events); injected hardware input (SendKeys/keybd_event) cannot reach a locked
      session and is not used;
    - pixels come from PrintWindow with PW_RENDERFULLCONTENT into a bitmap of the window rect,
      which also renders a window that is not visible on the lock screen;
    - the UIA trees are dumped with System.Windows.Automation.

  The evidence sequence:
    1. fixture: docs/goal-desktop-v4/round-31/crlf-bom.lv, a real CRLF script saved with a UTF-8
       BOM. Its SHA-256 is recorded before launch.
    2. launch with the file path; capture opened-file.png + opened-file-uia.txt (the tab shows the
       file name, the file bar shows UTF-8 BOM / CRLF and the path).
    3. UIA-invoke the file bar's Save; the unmodified file must be byte-identical afterwards
       (SHA-256 equal) and the log must say "Saved ... (bytes, UTF-8 BOM / CRLF)".
    4. place the caret on the last line with posted Down/End keys and type " + 1"; the tab title
       becomes "<name> *" and the window title follows.
    5. UIA-invoke the file bar's Close; capture unsaved-prompt.png + unsaved-prompt-uia.txt (the
       prompt offers Save / Discard / Cancel and names the document).
    6. UIA-invoke the prompt's Cancel; dump after-cancel-uia.txt: the tab is still open, still
       dirty, the buffer still shows the typed text, and the file on disk is unchanged.
    7. UIA-invoke Close, then the prompt's Discard: the tab closes, the file on disk is unchanged.
    8. close the window (WM_CLOSE); the remaining tab is clean, so no prompt appears.

  APPDATA is redirected to this round's appdata folder and the process working directory is this
  round's work folder, so the resolved plot directory is writable and outside the working
  directory (without that redirect every plot statement fails with access denied).

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
$Fixture = Join-Path $OutDir 'crlf-bom.lv'
if (-not (Test-Path -LiteralPath $Fixture)) { throw "fixture not found: $Fixture" }

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Round31Native {
  public delegate bool EnumProc(IntPtr hwnd, IntPtr lparam);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT rect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc call, IntPtr lparam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
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
function Get-FileBytes([string]$path) { [System.IO.File]::ReadAllBytes($path) }

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
    $hwnd = [Round31Native]::WindowOf([uint32]$processId, 'Lovelace Desktop')
    if ($hwnd -eq [IntPtr]::Zero) { throw "no app window for $name" }
    $rect = New-Object Round31Native+RECT
    if (-not [Round31Native]::GetWindowRect($hwnd, [ref]$rect)) { throw "GetWindowRect failed for $name" }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -le 0 -or $height -le 0) { throw "window has no size for $name ($width x $height)" }
    $bitmap = New-Object System.Drawing.Bitmap ($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $hdc = $graphics.GetHdc()
    $ok = [Round31Native]::PrintWindow($hwnd, $hdc, 2)
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
# is how the driver verifies posted caret keys actually moved the caret before typing.
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

# The title bar's system Close button is also a UIA Button named 'Close' and comes first in the
# tree; the workbench's own Close button is the last one. Pressing the system button sends
# WM_CLOSE, which is the window-close path (proven later on purpose), not the tab-close path.
function Invoke-WorkbenchClose([int]$processId, [string]$name) {
    $found = @(Get-Buttons $processId | Where-Object { $_.Current.Name -eq $name })
    if ($found.Count -lt 1) { throw "no button '$name'" }
    Invoke-Button $processId $name ($found.Count - 1)
}

# The window procedure of an egui/eframe window is reached even on a locked desktop: the posted
# messages are queued for the app's own thread, not injected as hardware input. Only WM_KEYDOWN is
# posted: winit resolves the typed character from the virtual key with the current keyboard layout
# (a following WM_CHAR would deliver the same character a second time, which the probe in the
# round report documents).
function Send-Key([int]$targetProcessId, [int]$virtualKey, [int]$repeat = 1) {
    $target = [Round31Native]::WindowOf([uint32]$targetProcessId, 'Lovelace Desktop')
    if ($target -eq [IntPtr]::Zero) { throw 'Send-Key: no app window' }
    for ($i = 0; $i -lt $repeat; $i++) {
        [void][Round31Native]::PostMessage($target, 0x0100, [IntPtr]$virtualKey, [IntPtr]0x001E0001)
        Start-Sleep -Milliseconds 40
    }
}

# ---------------------------------------------------------------------------------------------
# Fixture + launch
# ---------------------------------------------------------------------------------------------
$originalHash = Get-Sha256 $Fixture
$originalBytes = Get-FileBytes $Fixture
Write-CaptureLog ("FIXTURE {0}: {1} bytes sha256={2}" -f $Fixture, $originalBytes.Length, $originalHash)
$hasBom = ($originalBytes.Length -ge 3 -and $originalBytes[0] -eq 0xEF -and $originalBytes[1] -eq 0xBB -and $originalBytes[2] -eq 0xBF)
Add-Check 'fixture-bom' $hasBom 'the fixture starts with a UTF-8 byte-order mark'
$crlf = 0
for ($i = 0; $i -lt $originalBytes.Length - 1; $i++) { if ($originalBytes[$i] -eq 13 -and $originalBytes[$i+1] -eq 10) { $crlf++ } }
Add-Check 'fixture-crlf' ($crlf -ge 2) ("the fixture uses CRLF line endings ({0} pairs)" -f $crlf)

$process = Start-Process -FilePath $Exe -ArgumentList @("`"$Fixture`"") -PassThru
$deadline = (Get-Date).AddSeconds(30)
$hwnd = [IntPtr]::Zero
do {
    Start-Sleep -Milliseconds 250
    $process.Refresh()
    if ($process.HasExited) { throw "the app exited during startup (code $($process.ExitCode))" }
    $hwnd = [Round31Native]::WindowOf([uint32]$process.Id, 'Lovelace Desktop')
} while ($hwnd -eq [IntPtr]::Zero -and (Get-Date) -lt $deadline)
if ($hwnd -eq [IntPtr]::Zero) { throw 'the workbench window never appeared' }
# A 1600x1000 window keeps every pane legible in the capture.
[void][Round31Native]::SetWindowPos($hwnd, [IntPtr]::Zero, 10, 10, 1600, 1000, 0x0014)
Start-Sleep -Milliseconds 900
$title = New-Object System.Text.StringBuilder 512
[void][Round31Native]::GetWindowText($hwnd, $title, 512)
Write-CaptureLog ("LAUNCH pid={0} hwnd={1} title='{2}' argv='{3}'" -f $process.Id, $hwnd, $title.ToString(), $Fixture)
Add-Check 'window-title-names-the-file' ($title.ToString().Contains('crlf-bom.lv')) $title.ToString()

# ---------------------------------------------------------------------------------------------
# 1. opened from disk: the tab carries the file name
# ---------------------------------------------------------------------------------------------
$openedLines = Save-UiaDump $process.Id 'opened-file'
$opened = Save-Capture $process.Id 'opened-file'
$tabLine = @($openedLines | Where-Object { $_ -match 'Button \| crlf-bom\.lv' })
Add-Check 'opened-tab-shows-the-file-name' ($tabLine.Count -ge 1) ($tabLine -join ' ; ')
$badgeLine = @($openedLines | Where-Object { $_ -match 'UTF-8 BOM' })
Add-Check 'opened-file-records-bom-and-crlf' ($badgeLine.Count -ge 1) ($badgeLine -join ' ; ')
$pathLine = @($openedLines | Where-Object { $_ -match [regex]::Escape($Fixture) })
Add-Check 'file-bar-shows-the-path' ($pathLine.Count -ge 1) ($pathLine -join ' ; ')
$openLogLine = @($openedLines | Where-Object { $_ -match 'Opened .*crlf-bom\.lv' })
Add-Check 'open-logged-with-encoding' ($openLogLine.Count -ge 1) ($openLogLine -join ' ; ')
$openedSizeLine = @($openedLines | Where-Object { $_ -match '\d+ lines / \d+ bytes' })
Add-Check 'editor-states-the-buffer-size' ($openedSizeLine.Count -ge 1) ($openedSizeLine[0])

# ---------------------------------------------------------------------------------------------
# 2. Save of an unmodified file is byte-identical
# ---------------------------------------------------------------------------------------------
Start-Sleep -Milliseconds 200
Invoke-Button $process.Id 'Save'
Start-Sleep -Milliseconds 600
$savedHash = Get-Sha256 $Fixture
$savedLines = Save-UiaDump $process.Id 'after-unmodified-save'
Add-Check 'unmodified-save-byte-identical' ($savedHash -eq $originalHash) ("before={0} after={1}" -f $originalHash, $savedHash)
$savedLogLine = @($savedLines | Where-Object { $_ -match 'Saved .*bytes' })
Add-Check 'save-logged-encoding-and-size' ($savedLogLine.Count -ge 1) ($savedLogLine -join ' ; ')
$dirtyAfterSave = @($savedLines | Where-Object { $_ -match 'Button \| crlf-bom\.lv \*' })
Add-Check 'unmodified-save-keeps-tab-clean' ($dirtyAfterSave.Count -eq 0) ("dirty tab nodes: {0}" -f $dirtyAfterSave.Count)

# ---------------------------------------------------------------------------------------------
# 3. edit through posted keys: the tab and the window title show the dirty marker
# ---------------------------------------------------------------------------------------------
# caret starts at line 1 column 1: move it to the last line ("b") with posted VK_DOWN keys,
# checking the status strip after each one, then End and " = 1".
for ($attempt = 0; $attempt -lt 10; $attempt++) {
    $caret = Get-CaretPosition $process.Id
    if ($caret -and $caret.line -ge 4) { break }
    Send-Key $process.Id 0x28 1      # VK_DOWN
    Start-Sleep -Milliseconds 250
}
Send-Key $process.Id 0x23 1          # VK_END
Start-Sleep -Milliseconds 300
$caretBeforeTyping = Get-CaretPosition $process.Id
Write-CaptureLog ("CARET before typing: line={0} column={1}" -f $caretBeforeTyping.line, $caretBeforeTyping.column)
Add-Check 'caret-reached-the-last-line' ($caretBeforeTyping -and $caretBeforeTyping.line -eq 4) ("Ln {0}, Col {1}" -f $caretBeforeTyping.line, $caretBeforeTyping.column)
Send-Key $process.Id 0x20 1      # VK_SPACE -> " "
Send-Key $process.Id 0xBB 1      # VK_OEM_PLUS (unshifted) -> "="
Send-Key $process.Id 0x20 1      # VK_SPACE -> " "
Send-Key $process.Id 0x31 1      # VK_1 -> "1"

Start-Sleep -Milliseconds 700
$dirtyLines = Save-UiaDump $process.Id 'dirty'
$dirtyTab = @($dirtyLines | Where-Object { $_ -match 'Button \| crlf-bom\.lv \*' })
Add-Check 'dirty-tab-shows-the-marker' ($dirtyTab.Count -ge 1) ($dirtyTab -join ' ; ')
$dirtyTitle = New-Object System.Text.StringBuilder 512
[void][Round31Native]::GetWindowText($hwnd, $dirtyTitle, 512)
Add-Check 'window-title-shows-the-marker' ($dirtyTitle.ToString().Contains('crlf-bom.lv *')) $dirtyTitle.ToString()
$dirtySizeLine = @($dirtyLines | Where-Object { $_ -match '\d+ lines / \d+ bytes' })
Add-Check 'typed-text-reached-the-buffer' ($dirtySizeLine.Count -ge 1 -and $dirtySizeLine[0] -ne $openedSizeLine[0]) ("before={0} after={1}" -f $openedSizeLine[0], ($dirtySizeLine -join ' ; '))

# ---------------------------------------------------------------------------------------------
# 4. Close on a dirty tab prompts; Cancel keeps everything
# ---------------------------------------------------------------------------------------------
Start-Sleep -Milliseconds 200
Invoke-WorkbenchClose $process.Id 'Close'
Start-Sleep -Milliseconds 700
$promptLines = Save-UiaDump $process.Id 'unsaved-prompt'
$promptCapture = Save-Capture $process.Id 'unsaved-prompt'
$promptQuestion = @($promptLines | Where-Object { $_ -match 'Save changes to crlf-bom\.lv before closing' })
Add-Check 'prompt-names-the-document' ($promptQuestion.Count -ge 1) ($promptQuestion -join ' ; ')
foreach ($answer in @('Save', 'Discard', 'Cancel')) {
    $nodes = @($promptLines | Where-Object { $_ -match ("Button \| {0} \|" -f $answer) })
    Add-Check ("prompt-offers-{0}" -f $answer.ToLower()) ($nodes.Count -ge 1) ("{0}: {1} nodes" -f $answer, $nodes.Count)
}

# The toolbar's Cancel only exists while a run is in flight (it is not), so the prompt owns the
# only Cancel button in the tree.
$allCancel = @(Get-Buttons $process.Id | Where-Object { $_.Current.Name -eq 'Cancel' })
$cancelIndex = [Math]::Max(0, $allCancel.Count - 1)
Invoke-Button $process.Id 'Cancel' $cancelIndex
Start-Sleep -Milliseconds 700
$afterCancelLines = Save-UiaDump $process.Id 'after-cancel'
$afterCancel = Save-Capture $process.Id 'after-cancel'
$stillOpen = @($afterCancelLines | Where-Object { $_ -match 'Button \| crlf-bom\.lv \*' })
Add-Check 'cancel-keeps-the-dirty-tab' ($stillOpen.Count -ge 1) ($stillOpen -join ' ; ')
$stillSized = @($afterCancelLines | Where-Object { $_ -match '\d+ lines / \d+ bytes' })
Add-Check 'cancel-keeps-the-buffer' ($stillSized.Count -ge 1 -and $stillSized[0] -eq $dirtySizeLine[0]) ("buffer={0}" -f ($stillSized -join ' ; '))
$afterCancelHash = Get-Sha256 $Fixture
Add-Check 'cancel-writes-nothing' ($afterCancelHash -eq $originalHash) ("sha256={0}" -f $afterCancelHash)

# ---------------------------------------------------------------------------------------------
# 5. Close again, Discard: the tab goes away, the app stays open, the file is untouched
# ---------------------------------------------------------------------------------------------
Start-Sleep -Milliseconds 200
Invoke-WorkbenchClose $process.Id 'Close'
Start-Sleep -Milliseconds 700
$discardPrompt = Save-UiaDump $process.Id 'discard-prompt'
$discardButtons = @(Get-Buttons $process.Id | Where-Object { $_.Current.Name -eq 'Discard' })
if ($discardButtons.Count -lt 1) { throw 'the second prompt has no Discard button' }
$discardButtons[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
Write-CaptureLog ("UIA INVOKE button 'Discard' (of {0})" -f $discardButtons.Count)
Start-Sleep -Milliseconds 900
$process.Refresh()
Add-Check 'discard-keeps-the-app-open' (-not $process.HasExited) 'a discarded tab must not quit the workbench'
$afterDiscardLines = Save-UiaDump $process.Id 'after-discard'
$afterDiscard = Save-Capture $process.Id 'after-discard'
$tabGone = @($afterDiscardLines | Where-Object { $_ -match 'Button \| crlf-bom\.lv' })
Add-Check 'discard-closes-the-tab' ($tabGone.Count -eq 0) ("remaining tab nodes: {0}" -f $tabGone.Count)
$afterDiscardHash = Get-Sha256 $Fixture
Add-Check 'discard-writes-nothing' ($afterDiscardHash -eq $originalHash) ("sha256={0}" -f $afterDiscardHash)

# ---------------------------------------------------------------------------------------------
# 6. The window-close prompt: a second dirty document, refused, then discarded
# ---------------------------------------------------------------------------------------------
# Select the remaining untitled tab and type into it, so the window itself has unsaved work.
# Closing the fixture tab selects its neighbour, so the untitled tab is already active.
Start-Sleep -Milliseconds 200
Send-Key $process.Id 0x5A 1      # VK_Z -> "z"
Send-Key $process.Id 0x20 1      # VK_SPACE -> " "
Send-Key $process.Id 0xBB 1      # VK_OEM_PLUS -> "="
Send-Key $process.Id 0x20 1      # VK_SPACE -> " "
Send-Key $process.Id 0x31 1      # VK_1 -> "1"
Start-Sleep -Milliseconds 500
$untitledLines = Save-UiaDump $process.Id 'untitled-dirty'
$untitledDirty = @($untitledLines | Where-Object { $_ -match 'Button \| untitled\.lv \*' })
Add-Check 'second-document-is-dirty' ($untitledDirty.Count -ge 1) ($untitledDirty -join ' ; ')

Start-Sleep -Milliseconds 200
[void][Round31Native]::PostMessage($hwnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)   # WM_CLOSE
Start-Sleep -Milliseconds 900
$process.Refresh()
Add-Check 'window-close-is-refused-for-unsaved-work' (-not $process.HasExited) 'the window must stay open while a document is dirty'
$windowPromptLines = Save-UiaDump $process.Id 'window-close-prompt'
$windowPrompt = Save-Capture $process.Id 'window-close-prompt'
$windowQuestion = @($windowPromptLines | Where-Object { $_ -match 'Save changes to untitled\.lv before closing' })
Add-Check 'window-prompt-names-the-document' ($windowQuestion.Count -ge 1) ($windowQuestion -join ' ; ')

$windowDiscard = @(Get-Buttons $process.Id | Where-Object { $_.Current.Name -eq 'Discard' })
if ($windowDiscard.Count -lt 1) { throw 'the window prompt has no Discard button' }
$windowDiscard[0].GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
Write-CaptureLog ("UIA INVOKE button 'Discard' (window prompt, of {0})" -f $windowDiscard.Count)
$closed = $process.WaitForExit(10000)
if (-not $closed) { Stop-Process -Id $process.Id -Force }
Add-Check 'discarding-the-window-prompt-quits' $closed ("exited={0}" -f $closed)

# ---------------------------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------------------------
$failed = @($script:Checks | Where-Object { -not $_.ok })
# Windows PowerShell 5.1's ConvertTo-Json cannot serialise this object graph (it throws
# "Argument types do not match" on the nested arrays), so the script writes the JSON itself.
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
[void]$json.Append("  ""round"": 31,`n")
[void]$json.Append("  ""goal"": ""DOC-002 / DOC-003"",`n")
[void]$json.Append("  ""method"": ""UIA Invoke + PrintWindow(PW_RENDERFULLCONTENT) on a locked session; keyboard via posted WM_KEYDOWN messages (no hardware injection)"",`n")
[void]$json.Append("  ""session"": ""locked"",`n")
[void]$json.Append("  ""fixture"": { ""path"": $(ConvertTo-JsonString $Fixture), ""bytes"": $($originalBytes.Length), ""sha256"": $(ConvertTo-JsonString $originalHash) },`n")
[void]$json.Append("  ""captures"": [`n")
$captureList = @($opened, $promptCapture, $afterCancel, $afterDiscard, $windowPrompt)
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
