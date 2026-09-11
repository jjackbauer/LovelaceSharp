# P1 numeric-boundary attacker: probe harness (v3).
# Windows PowerShell 5.1: ProcessStartInfo has no ArgumentList, so build .Arguments.
param(
  [Parameter(Mandatory=$true)][string]$Batch,
  [Parameter(Mandatory=$true)][string]$CasesJson,
  [int]$TimeoutMs = 20000
)
$ErrorActionPreference = 'Continue'
Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
$exe  = (Resolve-Path 'out\aot\Lovelace.Run.exe').Path
$root = 'docs\goal-cycle-4\round-09'
$dir  = Join-Path $root ("probes\" + $Batch)
New-Item -ItemType Directory -Force -Path $dir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $root 'raw') | Out-Null
$enc  = New-Object System.Text.UTF8Encoding($false)
$cases = Get-Content $CasesJson -Raw | ConvertFrom-Json
$log  = Join-Path $root ("raw\" + $Batch + ".txt")
$lines = @()
foreach ($c in $cases) {
  $lines += "### $($c.name)"
  try {
    $f = Join-Path $dir ($c.name + '.ls')
    [System.IO.File]::WriteAllText($f, [string]$c.body, $enc)
    $full = (Resolve-Path $f).Path
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName  = $exe
    $psi.Arguments = '--file "' + $full + '" --omit-functions'
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow  = $true
    $proc = [System.Diagnostics.Process]::Start($psi)
    $outTask = $proc.StandardOutput.ReadToEndAsync()
    $errTask = $proc.StandardError.ReadToEndAsync()
    if (-not $proc.WaitForExit($TimeoutMs)) {
      try { $proc.Kill() } catch {}
      $proc.WaitForExit(5000) | Out-Null
      $code = 'TIMEOUT'
    } else { $code = $proc.ExitCode }
    $outTxt = $outTask.GetAwaiter().GetResult()
    $errTxt = $errTask.GetAwaiter().GetResult()
    if ($null -eq $outTxt) { $outTxt = '' }
    if ($null -eq $errTxt) { $errTxt = '' }
    $lines += "BODY: $(($c.body -replace "`r?`n", ' ; '))"
    $lines += "EXIT: $code"
    $lines += "STDOUT: $($outTxt.Trim())"
    if ($errTxt.Trim().Length -gt 0) { $lines += "STDERR: $($errTxt.Trim())" }
  } catch {
    $lines += "HARNESS-ERROR: $($_.Exception.Message)"
  }
  $lines += ""
}
[System.IO.File]::WriteAllText((Join-Path (Get-Location) $log), ($lines -join [char]10), $enc)
Write-Output ("batch $Batch : {0} probes -> {1}" -f $cases.Count, $log)
