Set-Location 'C:\Users\ricar\dev\LovelaceSharp'
& dotnet build Lovelace.Run/Lovelace.Run.csproj -c Release --nologo 2>&1 | Select-String 'error|Build succeeded' | Select-Object -First 2 | ForEach-Object { $_.Line.Trim() }
$enc = New-Object System.Text.UTF8Encoding($false)
$d='docs\goal-cycle-3\round-22\v'; New-Item -ItemType Directory -Force -Path $d | Out-Null
$dll = (Resolve-Path 'Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll').Path
$f = Join-Path $d 'cap.ls'; [System.IO.File]::WriteAllText($f, 'capabilities()', $enc)
$o = ([string]::Join([char]10, (& dotnet $dll --file $f --omit-functions))) | ConvertFrom-Json
$s = $o.result.structured
Write-Output ('type=' + $s.type)
$fld=@{}; foreach ($x in $s.fields) { $fld[$x.name]=$x.value }
Write-Output ('supported_domains = ' + (($fld['supported_domains'].elements | ForEach-Object { $_.domain }) -join ','))
Write-Output ('unsupported_domains = ' + (($fld['unsupported_domains'].elements | ForEach-Object { $_.domain }) -join ','))
Write-Output ('exactness = ' + $fld['exactness'].type + '/' + $fld['exactness'].value)
foreach ($u in $fld['unsupported_operations'].elements) { $g=@{}; foreach ($y in $u.fields) { $g[$y.name]=$y.value }; Write-Output ('  ' + $g['operation_class'].value + ' -> code=' + $g['code'].value + ' cat=' + $g['category'].value + '/' + $g['category'].type) }