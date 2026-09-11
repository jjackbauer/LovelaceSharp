param([string]$Script, [string]$Dest)
$exe = "C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe"
$out = & $exe --file $Script --omit-functions
[System.IO.File]::WriteAllText($Dest, ($out -join "`n"), (New-Object System.Text.UTF8Encoding($false)))
