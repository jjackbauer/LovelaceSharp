$root='C:\Users\ricar\dev\LovelaceSharp'; Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$d = Join-Path $root 'docs\goal-cycle-3\round-3\probes'
New-Item -ItemType Directory -Force -Path $d | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$p = @(
 @{N='ns_identity'; S='x = symbol("x"); solve_full((x^2-1)/(x^2-1) == 0, x)'},
 @{N='ns_realject'; S='x = symbol("x"); solve_full(x^2 + 1 == 0, x, real)'},
 @{N='ns_xoverx';   S='x = symbol("x"); solve_full(x/x == 0, x)'},
 @{N='partial';     S='x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)'},
 @{N='mult2';       S='x = symbol("x"); solve_full(x^2 - 2*x + 1 == 0, x)'},
 @{N='family';      S='x = symbol("x"); solve_full(sin(x) == 0, x)'},
 @{N='sign';        S='-2^2'},
 @{N='signsubs';    S='x = symbol("x"); subs(-x^2, x, 3)'},
 @{N='intreject';   S='x = symbol("x"); solve(2*x == 1, x, integer)'},
 @{N='rat';         S='1/3'},
 @{N='assume';      S='x = symbol("x"); assume(x >= 5); simplify(x < 5)'}
)
foreach ($q in $p) {
  $f = Join-Path $d ($q.N + '.ls')
  [System.IO.File]::WriteAllText($f, $q.S, $enc)
  $raw = [string]::Join([char]10, (& $exe --file $f --omit-functions))
  $code = $LASTEXITCODE
  $note = ''
  try { $o = $raw | ConvertFrom-Json
    if ($o.ok) { $s=$o.result.structured
      if ($s.fields) { $f2=@{}; foreach ($x in $s.fields) { $f2[$x.name]=$x.value }
        $note = 'status=' + $f2['status'].value + ' complete=' + $f2['complete'].value + ' completeness=' + $f2['completeness'].value + ' unrep=' + $f2['unrepresented_count'].value }
      elseif ($s.kind -eq 'Symbolic') { $note = 'Symbolic ' + $s.pretty }
      elseif ($s.kind -eq 'Real') { $note = 'Real ' + $s.value + ' exact=' + $s.exact + ' num=' + $s.numerator }
      elseif ($s.kind -eq 'Boolean') { $note = 'Boolean ' + $s.value }
      elseif ($s.value) { $note = $s.kind + ' ' + $s.value } }
    else { $note = 'ERR ' + $o.code + '/' + $o.category } } catch { $note = 'PARSE-FAIL' }
  Write-Output ('{0,-10} exit={1} {2}' -f $q.N, $code, $note) }