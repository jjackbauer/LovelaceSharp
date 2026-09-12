param(
  [string]$Exe = "C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe"
)
# Cross-surface consistency: the SAME logical script text, delivered through --file, --stdin and --eval,
# must report the same code/category/position/line/column/timings.
# NOTE: --eval is built from double-quoted text WITHOUT embedded double quotes, because PS 5.1 mangles
# embedded quotes in native arguments (EVD-299). --stdin goes through cmd's "type" so the bytes arrive
# unmodified. Newlines are built with [char]13/[char]10 - never backtick escapes, which the harness's
# TypeScript template literal cannot carry.
$ErrorActionPreference = 'Continue'
$LF = [string][char]10
$CRLF = [string]([char]13 + [char]10)
$cases = @(
  @{ Name = 'LF';       Bom = $false; Nl = $LF },
  @{ Name = 'BOM+LF';   Bom = $true;  Nl = $LF },
  @{ Name = 'CRLF';     Bom = $false; Nl = $CRLF },
  @{ Name = 'BOM+CRLF'; Bom = $true;  Nl = $CRLF }
)
function Summarize([string]$raw, [int]$exit) {
  $line = ($raw -split "\n" | Where-Object { $_.Trim().StartsWith('{') } | Select-Object -First 1)
  if (-not $line) { return "exit=$exit NO-ENVELOPE" }
  try {
    $o = $line | ConvertFrom-Json
    $d = $o.diagnostics[0]
    $tp = if ($o.timings) { (@($o.timings) | ForEach-Object { $_.position }) -join ',' } else { '' }
    return ("exit={0} code={1} pos={2} line={3} col={4} timings=[{5}]" -f $exit, $o.code, $d.position, $d.line, $d.column, $tp)
  } catch { return "exit=$exit UNPARSEABLE" }
}
foreach ($c in $cases) {
  $text = 'a = 1' + $c.Nl + 'b = 2' + $c.Nl + 'det(a)' + $c.Nl
  $path = Join-Path $env:TEMP ("surf-" + [Guid]::NewGuid().ToString("N") + ".ls")
  $enc = New-Object System.Text.UTF8Encoding($c.Bom)
  [System.IO.File]::WriteAllText($path, $text, $enc)

  $rawF = & $Exe --file $path --json --omit-functions --omit-variables 2>&1 | Out-String
  $f = Summarize $rawF $LASTEXITCODE

  $cmdline = 'type "' + $path + '" | "' + $Exe + '" --stdin --json --omit-functions --omit-variables'
  $rawS = (cmd /c $cmdline) | Out-String
  $s = Summarize $rawS $LASTEXITCODE

  $bomChar = if ($c.Bom) { [char]0xFEFF } else { '' }
  $ev = $bomChar + 'a = 1' + $c.Nl + 'b = 2' + $c.Nl + 'det(a)'
  $rawE = & $Exe --eval $ev --json --omit-functions --omit-variables 2>&1 | Out-String
  $e = Summarize $rawE $LASTEXITCODE

  Remove-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue
  $agree = ($f -eq $s) -and ($s -eq $e)
  $verdict = if ($agree) { 'AGREE' } else { 'DIVERGE' }
  Write-Output ("{0,-10} {1} file[{2}]  stdin[{3}]  eval[{4}]" -f $c.Name, $verdict, $f, $s, $e)
}
