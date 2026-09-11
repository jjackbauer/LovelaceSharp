Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'docs\goal-cycle-3\round-34'
$dll = 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll'
$cases = @(
  @{N='n19_undetermined'; S='x = symbol("x"); limit_full(x*sin(1/x), x, 0)'},
  @{N='n19_control_value'; S='x = symbol("x"); limit_full(sin(x)/x, x, 0)'},
  @{N='n19_dne_abs_over_x'; S='x = symbol("x"); limit_full(abs(x)/x, x, 0)'},
  @{N='n19_dne_one_over_x'; S='x = symbol("x"); limit_full(1/x, x, 0)'},
  @{N='n19_left_undetermined'; S='x = symbol("x"); limit_left(x*sin(1/x), x, 0)'},
  @{N='n19_right_undetermined'; S='x = symbol("x"); limit_right(x*sin(1/x), x, 0)'}
)
foreach ($c in $cases) {
  $f = Join-Path $d ($c.N + '.ls')
  [System.IO.File]::WriteAllText($f, $c.S, $enc)
  $errFile = Join-Path $d ($c.N + '.err.txt')
  $raw = (& dotnet $dll --file $f --omit-functions 2>$errFile | Out-String).Trim()
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $d ($c.N + '.out.txt')), $raw, $enc)
  try {
    $o = $raw | ConvertFrom-Json
    if ($o.ok) {
      $st = $o.result.structured
      if ($st.kind -eq 'Record') {
        $flds = @{}
        foreach ($fl in $st.fields) { $flds[$fl.name] = $fl.value }
        Write-Output ('{0,-24} exit={1} status={2} exists=[{3}:{4}] value=[{5}:{6}] left=[{7}:{8}] right=[{9}:{10}] lcond={11} rcond={12}' -f `
          $c.N, $code, $flds['status'].value, $flds['exists'].kind, $flds['exists'].value, $flds['value'].kind, $flds['value'].pretty, `
          $flds['left'].kind, $flds['left'].pretty, $flds['right'].kind, $flds['right'].pretty, `
          $flds['left_conditions'].elements.Count, $flds['right_conditions'].elements.Count)
      } else {
        Write-Output ('{0,-24} exit={1} kind={2} -> {3}' -f $c.N, $code, $st.kind, $st.value)
      }
    }
    else { Write-Output ('{0,-24} exit={1} -> ERR {2}' -f $c.N, $code, $o.message) }
  } catch { Write-Output ('{0,-24} exit={1} -> NOJSON: {2}' -f $c.N, $code, $raw) }
}