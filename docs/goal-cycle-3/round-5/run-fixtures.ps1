$ErrorActionPreference = 'Stop'
$runner = Resolve-Path "Lovelace.Run/bin/Release/net10.0/Lovelace.Run.exe"
$fixtures = Resolve-Path "Lovelace.Run.Tests/fixtures"
$raw = "out/raw-r5"
New-Item -ItemType Directory -Force -Path $raw | Out-Null
foreach ($f in Get-ChildItem $fixtures -Filter *.ls | Sort-Object Name) {
  $name = [System.IO.Path]::GetFileNameWithoutExtension($f.Name)
  $json = & $runner --file $f.FullName --omit-functions 2>$null
  $text = [string]::Join([char]10, $json)
  [System.IO.File]::WriteAllText((Join-Path $raw ($name + ".json")), $text, (New-Object System.Text.UTF8Encoding($false)))
  Write-Output ("raw " + $name)
}