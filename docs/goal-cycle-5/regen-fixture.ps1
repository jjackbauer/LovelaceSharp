$ErrorActionPreference='Continue'
$enc = New-Object System.Text.UTF8Encoding($false)
$d = 'Lovelace.Run.Tests\fixtures'
$name = $env:FIXTURE
$tmp = Join-Path $env:TEMP ($name + '-regen.ls')
Copy-Item "$d\$name.ls" $tmp -Force
$raw = & dotnet run --project Lovelace.Run -c Release --no-build -- --file $tmp --omit-functions 2>&1 | Out-String
$j = $raw | ConvertFrom-Json
function Fix($o) {
  if ($null -eq $o) { return $o }
  if ($o -is [System.Management.Automation.PSCustomObject]) {
    foreach ($p in @($o.PSObject.Properties)) { if (@('revision','elapsed','elapsedTime','timings') -contains $p.Name) { $p.Value = '<volatile>' } else { Fix $p.Value | Out-Null } }
  } elseif ($o -is [System.Array]) { foreach ($i in $o) { Fix $i | Out-Null } }
  return $o
}
$j = Fix $j
[System.IO.File]::WriteAllText((Join-Path (Get-Location) "$d\$name.json"), ($j | ConvertTo-Json -Depth 100), $enc)
'regenerated ' + $name + '.json (' + (Get-Item "$d\$name.json").Length + ' bytes)'
$o = & dotnet test Lovelace.Run.Tests -c Release --nologo 2>&1 | Out-String
($o -split ([char]10)) | Select-String -Pattern 'Passed!|Failed!|\[FAIL\]' | Select-Object -First 4 | ForEach-Object { $_.Line.Trim() }