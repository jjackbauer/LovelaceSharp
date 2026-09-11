$ErrorActionPreference = 'Continue'
$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
if (-not (Test-Path $exe)) { Write-Error 'AOT exe missing'; exit 9 }
$dir = Join-Path $root 'docs\goal-cycle-3\round-1\probes'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
$idx = Join-Path $dir 'index.txt'
"PROBE RUN $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') exe=$((Get-Item $exe).LastWriteTime.ToString('s'))" | Out-File $idx -Encoding utf8

$probes = @(
  @{ N='sign_minus_pow';        S='-2^2' },
  @{ N='sign_subs';             S='x = symbol("x"); subs(-x^2, x, 3)' },
  @{ N='sign_range_neg';        S='-1..2' },
  @{ N='sign_range_pow';        S='1..3^2' },
  @{ N='solve_partial';         S='x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)' },
  @{ N='solve_shift';           S='x = symbol("x"); solve((x+1)^2 - 4 == 0, x)' },
  @{ N='solve_cube';            S='x = symbol("x"); solve((x+1)^3 == 8, x)' },
  @{ N='solve_quad';            S='x = symbol("x"); solve(x^2 - 4 == 0, x)' },
  @{ N='solve_integer';         S='x = symbol("x"); solve(2*x == 1, x, integer)' },
  @{ N='solve_unevaluated';     S='x = symbol("x"); solve(x^4 + 1 == 0, x)' },
  @{ N='solve_nosolutions';     S='x = symbol("x"); solve_full((x^2 - 1)/(x^2 - 1) == 0, x)' },
  @{ N='solve_mult2';           S='x = symbol("x"); solve_full(x^2 - 2*x + 1 == 0, x)' },
  @{ N='solve_family';          S='x = symbol("x"); solve_full(sin(x) == 0, x)' },
  @{ N='p0_systemsolve';        S='x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])' },
  @{ N='p1_sqrt_neg';           S='sqrt(-1)' },
  @{ N='p1_pow_half';           S='(-1)^(1/2)' },
  @{ N='p1_pow_third';          S='(-8)^(1/3)' },
  @{ N='p1_neg_exp';            S='2^-1' },
  @{ N='p1_zero_neg_exp';       S='0^(-1)' },
  @{ N='p1_sqrt_neg_literal';   S='sqrt(-4)' },
  @{ N='p1_atanh';              S='atanh(0.5)' },
  @{ N='p1_acosh';              S='acosh(2)' },
  @{ N='p1_asinh';              S='asinh(1)' },
  @{ N='p1_abs_input';          S='abs(-3)' },
  @{ N='p1_abs_diff';           S='x = symbol("x"); diff(abs(x), x)' },
  @{ N='p1_parens_int';         S='x = symbol("x"); integrate_full(exp(-x^2), x)' },
  @{ N='p1_metamorph_diff_int'; S='x = symbol("x"); diff(integrate(exp(x), x), x)' },
  @{ N='assume_fold';           S='x = symbol("x"); assume(x >= 5); simplify(x < 5)' },
  @{ N='assume_int_contra';     S='x = symbol("x", integer); assume(x > 1/2); assume(x < 1)' },
  @{ N='assume_real_control';   S='x = symbol("x"); assume(x > 1/2); assume(x < 1)' },
  @{ N='assume_mirrored';       S='x = symbol("x"); assume(1/2 < x)' },
  @{ N='proto_rational';        S='1/3' },
  @{ N='proto_rational_zero';   S='1/3 - 1/3' },
  @{ N='proto_approx';          S='evalf(1/3, 50)' },
  @{ N='proto_inspect';         S='x = symbol("x"); inspect(x^2 + 1)' },
  @{ N='proto_type_symbol';     S='x = symbol("x"); type(x)' },
  @{ N='proto_array';           S='x = symbol("x"); y = symbol("y"); jacobian([x*y, x+y], [x, y])' },
  @{ N='proto_print_purity';    S='print("hello from the script"); 1 + 1' }
)

foreach ($p in $probes) {
  $n = $p.N
  $f = Join-Path $dir ($n + '.ls')
  [System.IO.File]::WriteAllText($f, $p.S, $enc)
  $raw = & $exe --file $f --omit-functions 2>&1 | Out-String
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $dir ($n + '.json')), $raw, $enc)
  $line = '{0} exit={1} bytes={2}' -f $n, $code, $raw.Length
  Write-Host $line
  Add-Content $idx $line
}

# flag probe: --omit-display must be evaluated for existence/behaviour
$f = Join-Path $dir 'p2_omit_display.ls'
[System.IO.File]::WriteAllText($f, '1 + 1', $enc)
$raw = & $exe --file $f --omit-display 2>&1 | Out-String
$code = $LASTEXITCODE
[System.IO.File]::WriteAllText((Join-Path $dir 'p2_omit_display.json'), $raw, $enc)
$line = 'p2_omit_display exit={0} bytes={1}' -f $code, $raw.Length
Write-Host $line
Add-Content $idx $line

# cancellation + budget probes
$f = Join-Path $dir 'runtime_budget.ls'
[System.IO.File]::WriteAllText($f, 'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)', $enc)
$raw = & $exe --file $f --omit-functions --print-budget 8 2>&1 | Out-String
$code = $LASTEXITCODE
[System.IO.File]::WriteAllText((Join-Path $dir 'runtime_budget.json'), $raw, $enc)
Add-Content $idx ('runtime_budget exit={0}' -f $code)
$raw = & $exe --file $f --omit-functions --cancel-after 1 2>&1 | Out-String
$code = $LASTEXITCODE
[System.IO.File]::WriteAllText((Join-Path $dir 'runtime_cancel.json'), $raw, $enc)
Add-Content $idx ('runtime_cancel exit={0}' -f $code)
$raw = & $exe --file $f --omit-functions --omit-variables 2>&1 | Out-String
$code = $LASTEXITCODE
[System.IO.File]::WriteAllText((Join-Path $dir 'runtime_omitvars.json'), $raw, $enc)
Add-Content $idx ('runtime_omitvars exit={0}' -f $code)
Write-Host 'PROBES COMPLETE'
