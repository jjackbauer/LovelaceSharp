$ErrorActionPreference = 'Continue'
$root = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $root
$out = Join-Path $root 'docs\goal-cycle-3\round-1'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$summaryPath = Join-Path $out 'baseline-summary.txt'
"BASELINE START $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" | Out-File $summaryPath -Encoding utf8
"HEAD $(git rev-parse HEAD)" | Add-Content $summaryPath
"dotnet $(dotnet --version)" | Add-Content $summaryPath
"psversion $($PSVersionTable.PSVersion)" | Add-Content $summaryPath
$pyOut = (& { try { python --version 2>&1 | Out-String } catch { "THREW: $($_.Exception.Message)" } })
"python-probe: $($pyOut -replace '\s+',' ')" | Add-Content $summaryPath

$results = New-Object System.Collections.ArrayList
function Run-Dotnet {
  param([string]$Label, [string]$LogFile, [string[]]$DotnetArgs)
  $logPath = Join-Path $out $LogFile
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  & dotnet @DotnetArgs *>&1 | Out-File -FilePath $logPath -Encoding utf8
  $code = $LASTEXITCODE
  $sw.Stop()
  $secs = [int]$sw.Elapsed.TotalSeconds
  $null = $results.Add([pscustomobject]@{ Label = $Label; Exit = $code; Seconds = $secs; Log = $LogFile })
  $line = '{0} exit={1} {2}s {3}' -f $Label, $code, $secs, $LogFile
  Write-Host $line
  Add-Content $summaryPath $line
}

Run-Dotnet 'build-release' 'build-release.log' @('build','LovelaceSharp.slnx','-c','Release','--nologo')

$suites = @(
 'Lovelace.Abstractions.Tests/Lovelace.Abstractions.Tests.csproj',
 'Lovelace.Array.Tests/Lovelace.Array.Tests.csproj',
 'Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj',
 'Lovelace.Console.Tests/Lovelace.Console.Tests.csproj',
 'Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.csproj',
 'Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj',
 'Lovelace.Knowledge.Tests/Lovelace.Knowledge.Tests.csproj',
 'Lovelace.Natural.Tests/Lovelace.Natural.Tests.csproj',
 'Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj',
 'Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj',
 'Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj',
 'Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj',
 'precbench.Tests/precbench.Tests.csproj'
)
foreach ($s in $suites) {
  $name = ($s -split '/')[0]
  Run-Dotnet ('test-' + $name) ('test-' + $name + '.log') @('test', $s, '-c', 'Release', '--nologo')
}
Run-Dotnet 'test-Lovelace.Real.Tests-nonheavy' 'test-Lovelace.Real.Tests-nonheavy.log' @('test','Lovelace.Real.Tests/Lovelace.Real.Tests.csproj','-c','Release','--nologo','--filter','Category!=Heavy')
Run-Dotnet 'build-symbench' 'build-symbench.log' @('build','symbench/symbench.csproj','-c','Release','--nologo')
Run-Dotnet 'publish-aot' 'publish-aot.log' @('publish','Lovelace.Run/Lovelace.Run.csproj','-c','Release','-p:PublishAot=true','-p:InvariantGlobalization=true','-o','out/aot')

$exe = Join-Path $root 'out\aot\Lovelace.Run.exe'
$srcNewest = Get-ChildItem -Path $root -Recurse -Include *.cs -File |
  Where-Object { $_.FullName -notmatch '\\obj\\' -and $_.FullName -notmatch '\\bin\\' -and $_.FullName -notmatch '\\out\\' } |
  Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (Test-Path $exe) {
  $e = Get-Item $exe
  $stale = $e.LastWriteTime -lt $srcNewest.LastWriteTime
  $line = "aot-exe mtime=$($e.LastWriteTime.ToString('s')) newest-source=$($srcNewest.LastWriteTime.ToString('s')) ($($srcNewest.Name)) STALE=$stale"
} else { $line = 'aot-exe MISSING' }
Write-Host $line
Add-Content $summaryPath $line

$smoke = Join-Path $out 'smoke'
New-Item -ItemType Directory -Force -Path $smoke | Out-Null
$enc = New-Object System.Text.UTF8Encoding($false)
function Write-Ls { param([string]$N, [string]$C) [System.IO.File]::WriteAllText((Join-Path $smoke ($N + '.ls')), $C, $enc) }

Write-Ls 'solve'      'x = symbol("x"); solve_full(x^2 - 4 == 0, x)'
Write-Ls 'partial'    'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)'
Write-Ls 'jacobian'   'x = symbol("x"); y = symbol("y"); jacobian([x*y, x+y], [x, y])'
Write-Ls 'printpure'  'print("hello from the script"); 1 + 1'
Write-Ls 'errdomain'  'x = symbol("x"); solve(x^2 + 1 == 0, x, integer)'

$scenarios = @('solve','partial','jacobian','printpure','errdomain')
foreach ($n in $scenarios) {
  $raw = & $exe --file (Join-Path $smoke ($n + '.ls')) --omit-functions 2>&1 | Out-String
  $code = $LASTEXITCODE
  [System.IO.File]::WriteAllText((Join-Path $out ('smoke-' + $n + '.json')), $raw, $enc)
  $note = ''
  try {
    $o = $raw | ConvertFrom-Json
    $note = "ok=$($o.ok) pv=$($o.protocolVersion) sym=$($o.symbolicFormatVersion) mathir=$($o.mathIrVersion)"
    if ($o.ok) {
      $s = $o.result.structured
      $note += " kind=$($s.kind) type=$($s.type)"
      if ($s.fields) {
        $f = @{}; foreach ($x in $s.fields) { $f[$x.name] = $x.value }
        foreach ($k in @('status','complete','completeness','domain','unrepresented_count','diagnostics','binding','parameter','parameter_domain')) {
          if ($f.ContainsKey($k)) { $v = $f[$k]; $note += " $k=[kind=$($v.kind) type=$($v.type) value=$($v.value) dom=$($v.domain) shape=$($v.shape -join ',')]" }
        }
      }
      if ($s.kind -eq 'Array') { $note += " shape=[$($s.shape -join ',')] elems=$($s.elements.Count)" }
      if ($s.value) { $note += " value=$($s.value)" }
    } else {
      $note += " code=$($o.code) category=$($o.category) message=$($o.message)"
    }
    if ($o.output) { $note += " output=[$($o.output -join '|')]" }
  } catch { $note = "PARSE-FAILED: $($_.Exception.Message)" }
  $line = 'smoke-{0} exit={1} {2}' -f $n, $code, $note
  Write-Host $line
  Add-Content $summaryPath $line
}

foreach ($b in @('benchmarks\symbench-baseline.md','benchmarks\raw-baseline.txt')) {
  $p = Join-Path $root $b
  $exists = Test-Path $p
  $len = if ($exists) { (Get-Item $p).Length } else { 0 }
  $line = "benchmark-asset $b exists=$exists bytes=$len"
  Write-Host $line
  Add-Content $summaryPath $line
}
"BASELINE END $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" | Add-Content $summaryPath
Write-Host 'BASELINE COMPLETE'
