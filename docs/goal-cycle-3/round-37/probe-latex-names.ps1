# Round 37 (Cycle 3, item B): JIT-runner probe — latex() of awkward symbol NAMES.
# One invocation per name: the runner's envelope reports the LAST statement's result, so a
# multi-statement probe would show only one rendering.
$names = @('x_1', 'p%q', 'a&b', 'h#1', 'c{d}', 'o}e', 't~u', 'e^f', 'm$n')
foreach ($n in $names) {
    $src = 's = symbol("' + $n + '"); latex(s)'
    $json = $src | dotnet run --project Lovelace.Run -c Release --no-build -- --stdin --omit-functions
    $obj = $json | ConvertFrom-Json
    '{0,-6} -> {1}' -f $n, $obj.result.display
}
# a compound expression: the escape must not disturb the surrounding structure
$src = 's = symbol("x_1"); latex((s + 1)/(s - 1))'
$json = $src | dotnet run --project Lovelace.Run -c Release --no-build -- --stdin --omit-functions
'compound (x_1 + 1)/(x_1 - 1) -> ' + ($json | ConvertFrom-Json).result.display
# a backslash in a name, through the language (the .ls string escape is doubled)
$src = 's = symbol("b' + '\\' + 's"); latex(s)'
'backslash-name source: ' + $src
$json = $src | dotnet run --project Lovelace.Run -c Release --no-build -- --stdin --omit-functions
'backslash-name -> ' + ($json | ConvertFrom-Json).result.display
