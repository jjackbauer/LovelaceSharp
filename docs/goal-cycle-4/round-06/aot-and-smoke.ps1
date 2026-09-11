# Cycle-4 D6 + D7: re-publish the Native AOT binary from the final tree, prove it is FRESH, and
# then run the same five smoke scenarios the CI aot-smoke job runs - locally, so the job's
# assertions are exercised even though the job itself has never executed on a runner.
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out = 'docs\goal-cycle-4\round-06'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$lines = @()
$failures = 0
function Check([string]$label, [bool]$ok, [string]$detail) {
  $script:lines += ('  [{0}] {1}{2}' -f $(if ($ok) { 'PASS' } else { 'FAIL' }), $label, $(if ($detail) { " - $detail" } else { '' }))
  if (-not $ok) { $script:failures++ }
}

$lines += "=== publish $(Get-Date -Format o) ==="
$pub = Join-Path $out 'aot-publish.log'
& dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot *> $pub
$pubExit = $LASTEXITCODE
$pubText = Get-Content $pub -Raw
$lines += "publish exit=$pubExit"
$lines += "publish warnings=" + ([regex]::Matches($pubText, ': warning ')).Count + " errors=" + ([regex]::Matches($pubText, ': error ')).Count
Check 'AOT publish succeeded' ($pubExit -eq 0) "exit $pubExit"

$exe = 'out\aot\Lovelace.Run.exe'
Check 'published binary exists' (Test-Path $exe) $exe
$exeTime = (Get-Item $exe).LastWriteTime
$lines += "exe LastWriteTime=$exeTime  size=$((Get-Item $exe).Length)"

# Freshness guard (constraint 7): the binary must be newer than the newest source file. Cycle 2
# produced two false defects from a stale binary, so this is checked, not assumed.
$sources = Get-ChildItem -Recurse -File -Include *.cs,*.csproj,*.json,*.js,*.css,*.html,*.md |
  Where-Object { $_.FullName -notmatch '\\(bin|obj|out|\.git|\.lovelace)' -and $_.FullName -notmatch 'BenchmarkDotNet\.Artifacts' }
$newest = ($sources | Sort-Object LastWriteTime -Descending | Select-Object -First 1)
$lines += "newest source: $($newest.FullName) at $($newest.LastWriteTime)"
Check 'binary is newer than the newest source file' ($exeTime -gt $newest.LastWriteTime) "$([math]::Round(($exeTime - $newest.LastWriteTime).TotalSeconds,1))s newer"

# ---- the five CI smoke scenarios, locally ----
$smoke = Join-Path $out 'smoke'
New-Item -ItemType Directory -Force -Path $smoke | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
function Run-Script([string]$name, [string]$body) {
  $f = Join-Path $smoke ($name + '.ls')
  [System.IO.File]::WriteAllText($f, $body, $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
  return $raw
}

$lines += "=== smoke 1: a complete solve reports Solved / Complete with per-solution conditions ==="
$raw = Run-Script 'solve' "x = symbol(`"x`")`nsolve_full(x^2 - 4 == 0, x)"
$env1 = $raw | ConvertFrom-Json
$fields = @{}; foreach ($f in $env1.result.structured.fields) { $fields[$f.name] = $f.value }
Check 'ok is true' ($env1.ok -eq $true)
Check 'protocolVersion == 1' ($env1.protocolVersion -eq 1)
Check 'symbolicFormatVersion starts with #!lovelace-sym' ($env1.symbolicFormatVersion -like '#!lovelace-sym*')
Check 'mathIrVersion >= 1' ($env1.mathIrVersion -ge 1)
Check 'structured is Record/SolveResult' ($env1.result.structured.kind -eq 'Record' -and $env1.result.structured.type -eq 'SolveResult')
Check 'status value Solved' ($fields['status'].value -eq 'Solved')
Check 'status kind Enum' ($fields['status'].kind -eq 'Enum')
Check 'status type SolveStatus' ($fields['status'].type -eq 'SolveStatus')
Check 'completeness kind Enum / Completeness' ($fields['completeness'].kind -eq 'Enum' -and $fields['completeness'].type -eq 'Completeness')
Check 'complete kind Boolean' ($fields['complete'].kind -eq 'Boolean')
Check 'complete value is the STRING true' ($fields['complete'].value -eq 'true')
Check 'domain complex' ($fields['domain'].kind -eq 'Domain' -and $fields['domain'].domain -eq 'complex')
Check 'solutions Array shape [2]' ($fields['solutions'].kind -eq 'Array' -and $fields['solutions'].shape[0] -eq 2)
$sf = @{}; foreach ($f in $fields['solutions'].elements[0].fields) { $sf[$f.name] = $f.value }
Check 'solution value Symbolic with rational canonical' ($sf['value'].kind -eq 'Symbolic' -and $sf['value'].canonical -like '(rat*')
Check 'solution exactness is an Enum (Exact|AlgebraicExact)' ($sf['exactness'].kind -eq 'Enum' -and $sf['exactness'].type -eq 'SolutionExactness' -and ($sf['exactness'].value -eq 'Exact' -or $sf['exactness'].value -eq 'AlgebraicExact'))

$lines += "=== smoke 2: an incomplete solve over the complex field must NOT claim completeness ==="
$raw = Run-Script 'partial' "x = symbol(`"x`")`nsolve_full(x^4 - x^2 - 1 == 0, x)"
$env2 = $raw | ConvertFrom-Json
$fields = @{}; foreach ($f in $env2.result.structured.fields) { $fields[$f.name] = $f.value }
Check 'status Partial (Enum/SolveStatus)' ($fields['status'].value -eq 'Partial' -and $fields['status'].kind -eq 'Enum' -and $fields['status'].type -eq 'SolveStatus')
Check 'completeness Partial (Enum)' ($fields['completeness'].kind -eq 'Enum' -and $fields['completeness'].value -eq 'Partial')
Check 'complete is the STRING false' ($fields['complete'].kind -eq 'Boolean' -and $fields['complete'].value -eq 'false')
Check 'unrepresented_count == 2' ($fields['unrepresented_count'].value -eq '2')

$lines += "=== smoke 3: arrays preserve shape into the protocol ==="
$raw = Run-Script 'jac' "x = symbol(`"x`")`ny = symbol(`"y`")`njacobian([x*y, x+y], [x, y])"
$env3 = $raw | ConvertFrom-Json
$s = $env3.result.structured
Check 'rank-2 Array shape [2,2]' ($s.kind -eq 'Array' -and $s.shape[0] -eq 2 -and $s.shape[1] -eq 2)
Check 'four elements' ($s.elements.Count -eq 4)
Check 'first element is Symbolic (sym y)' ($s.elements[0].kind -eq 'Symbolic' -and $s.elements[0].canonical -eq '(sym y)')

$lines += "=== smoke 4: stdout carries the envelope and nothing else ==="
$raw = Run-Script 'print' "print(`"hello from the script`")`n1 + 1"
$env4 = $raw | ConvertFrom-Json
Check 'output array holds the printed line' ($env4.output.Count -eq 1 -and $env4.output[0] -eq 'hello from the script')
Check 'result value == 2' ($env4.result.structured.value -eq '2')

$lines += "=== smoke 5: errors are structural (code + category), never message matching ==="
$f = Join-Path $smoke 'err.ls'
[System.IO.File]::WriteAllText($f, "x = symbol(`"x`")`nsolve(x^2 + 1 == 0, x, integer)", $enc)
$raw = [string]::Join([char]10, (& $exe --file $f --omit-functions 2>&1))
$exit5 = $LASTEXITCODE
$env5 = $raw | ConvertFrom-Json
Check 'non-zero exit for an unsupported domain' ($exit5 -ne 0) "exit $exit5"
Check 'ok is false' ($env5.ok -eq $false)
Check 'message names the domain' ($env5.message -like '*got integer*')
Check 'code and category are present' ($null -ne $env5.code -and $null -ne $env5.category) "code=$($env5.code) category=$($env5.category)"

$lines += ""
$lines += "SMOKE FAILURES: $failures"
[System.IO.File]::WriteAllText((Join-Path (Get-Location) (Join-Path $out 'aot-and-smoke.txt')), ($lines -join [char]10), (New-Object System.Text.UTF8Encoding($false)))
$lines | ForEach-Object { Write-Output $_ }
exit $failures
