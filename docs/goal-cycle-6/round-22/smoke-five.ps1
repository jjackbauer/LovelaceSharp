param([string]$Exe = "C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe")
# The five CI aot-smoke scenarios, run by hand against the PUBLISHED binary so the wave auditing it is
# not disturbed by a re-publish. Mirrors docs/goal-cycle-5/final-aot.ps1:36-104.
$ErrorActionPreference = 'Continue'
$nl = [string][char]10
$dir = Join-Path $env:TEMP ("smoke-" + [Guid]::NewGuid().ToString("N")); New-Item -ItemType Directory -Force -Path $dir | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$pass = 0; $fail = 0
function Check([string]$what, [bool]$ok) { if ($ok) { $script:pass++ } else { $script:fail++; Write-Output ("  FAIL: " + $what) } }
function Env([string]$name, [string]$body) {
  $f = Join-Path $dir ($name + '.ls'); [System.IO.File]::WriteAllText($f, $body, $enc)
  $raw = [string]::Join([char]10, (& $Exe --file $f --omit-functions 2>&1))
  return @{ raw = $raw; exit = $LASTEXITCODE; env = ($raw | ConvertFrom-Json) }
}
function Fields($s) { $h = @{}; foreach ($f in $s) { $h[$f.name] = $f.value }; return $h }

$r1 = Env 'solve' ('x = symbol("x")' + $nl + 'solve_full(x^2 - 4 == 0, x)')
$f1 = Fields $r1.env.result.structured.fields
Check '1 ok' ($r1.env.ok -eq $true)
Check '1 protocolVersion 1' ($r1.env.protocolVersion -eq 1)
Check '1 format version' ($r1.env.symbolicFormatVersion -like '#!lovelace-sym*')
Check '1 mathIrVersion >= 1' ($r1.env.mathIrVersion -ge 1)
Check '1 Record/SolveResult' ($r1.env.result.structured.kind -eq 'Record' -and $r1.env.result.structured.type -eq 'SolveResult')
Check '1 status Solved Enum' ($f1['status'].value -eq 'Solved' -and $f1['status'].kind -eq 'Enum' -and $f1['status'].type -eq 'SolveStatus')
Check '1 completeness Complete' ($f1['completeness'].kind -eq 'Enum' -and $f1['completeness'].value -eq 'Complete')
Check '1 complete is the STRING true' ($f1['complete'].kind -eq 'Boolean' -and $f1['complete'].value -eq 'true')
Check '1 domain complex' ($f1['domain'].kind -eq 'Domain' -and $f1['domain'].domain -eq 'complex')
Check '1 solutions shape 2' ($f1['solutions'].kind -eq 'Array' -and $f1['solutions'].shape[0] -eq 2)
$sf = Fields $f1['solutions'].elements[0].fields
Check '1 solution canonical rat' ($sf['value'].kind -eq 'Symbolic' -and $sf['value'].canonical -like '(rat*')

$r2 = Env 'partial' ('x = symbol("x")' + $nl + 'solve_full(x^4 - x^2 - 1 == 0, x)')
$f2 = Fields $r2.env.result.structured.fields
Check '2 status Partial' ($f2['status'].value -eq 'Partial' -and $f2['status'].type -eq 'SolveStatus')
Check '2 completeness Partial' ($f2['completeness'].kind -eq 'Enum' -and $f2['completeness'].value -eq 'Partial')
Check '2 complete STRING false' ($f2['complete'].kind -eq 'Boolean' -and $f2['complete'].value -eq 'false')
Check '2 unrepresented_count 2' ($f2['unrepresented_count'].value -eq '2')

$r3 = Env 'jac' ('x = symbol("x")' + $nl + 'y = symbol("y")' + $nl + 'jacobian([x*y, x+y], [x, y])')
$s3 = $r3.env.result.structured
Check '3 shape 2x2' ($s3.kind -eq 'Array' -and $s3.shape[0] -eq 2 -and $s3.shape[1] -eq 2)
Check '3 four elements' ($s3.elements.Count -eq 4)
Check '3 first is sym y' ($s3.elements[0].kind -eq 'Symbolic' -and $s3.elements[0].canonical -eq '(sym y)')

$r4 = Env 'print' ('print("hello from the script")' + $nl + '1 + 1')
Check '4 output holds the line' ($r4.env.output.Count -eq 1 -and $r4.env.output[0] -eq 'hello from the script')
Check '4 result 2' ($r4.env.result.structured.value -eq '2')

$r5 = Env 'err' ('x = symbol("x")' + $nl + 'solve(x^2 + 1 == 0, x, integer)')
Check '5 non-zero exit' ($r5.exit -ne 0)
Check '5 ok false' ($r5.env.ok -eq $false)
Check '5 message names the domain' ($r5.env.message -like '*got integer*')
Check '5 code and category present' ($null -ne $r5.env.code -and $null -ne $r5.env.category)

Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
Write-Output ("SMOKE: pass={0} fail={1} of {2} checks" -f $pass, $fail, ($pass + $fail))
