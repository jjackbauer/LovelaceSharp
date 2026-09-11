$ErrorActionPreference='Continue'; Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$out='docs\goal-cycle-3\round-5'
& dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo *>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
& dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo *>&1 | Select-String 'Passed!|Failed!' | ForEach-Object { $_.Line.Trim() }
$e=Get-Item 'out\aot\Lovelace.Run.exe'
$src=Get-ChildItem -Recurse -Include *.cs -File | Where-Object { $_.FullName -notmatch '\\(obj|bin|out)\\' } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Write-Output ('aot bytes={0} STALE={1}' -f $e.Length, ($e.LastWriteTime -lt $src.LastWriteTime))
$enc = New-Object System.Text.UTF8Encoding($false)
$d = Join-Path $out 'vprobes'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$cases = @(
 @{N='ns_identity'; S='x = symbol("x"); solve_full((x^2-1)/(x^2-1) == 0, x)'},
 @{N='ns_real';     S='x = symbol("x"); solve_full(x^2 + 1 == 0, x, real)'},
 @{N='ns_xoverx';   S='x = symbol("x"); solve_full(x/x == 0, x)'},
 @{N='partial';     S='x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)'},
 @{N='solved';      S='x = symbol("x"); solve_full(x^2 - 4 == 0, x)'},
 @{N='unevaluated'; S='x = symbol("x"); solve_full(x^4 + 1 == 0, x)'},
 @{N='arity';       S='x = symbol("x"); compile_full(x^2 + 1)'},
 @{N='arity_ok';    S='x = symbol("x"); compile_full(x^2 + 1, [x]).result_type'}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $raw = [string]::Join([char]10, (& 'out\aot\Lovelace.Run.exe' --file $f --omit-functions))
  $code = $LASTEXITCODE
  $note=''
  try { $o = $raw | ConvertFrom-Json
    if ($o.ok) { $s=$o.result.structured; $f2=@{}; foreach ($x in $s.fields) { $f2[$x.name]=$x.value }
      $st=$f2['status']; if ($st) { $note = 'status=' + $st.value + ' complete=' + $f2['complete'].value + ' completeness=' + $f2['completeness'].value } else { $note = $s.kind + ' ' + $s.value } }
    else { $note = 'code=' + $o.code + ' category=' + $o.category + ' recoverable=' + $o.recoverable + ' msg=' + $o.message } } catch { $note='PARSE-FAIL' }
  Write-Output ('{0,-12} exit={1} {2}' -f $c.N, $code, $note) }