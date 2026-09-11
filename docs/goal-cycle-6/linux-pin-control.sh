set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
rm -rf "$HOME/ls-control"
cp -r "$HOME/ls" "$HOME/ls-control"
cd "$HOME/ls-control"
git checkout 9e761ba -- Lovelace.Symbolics/SymbolicsPlugin.cs
echo "--- product summary in control ---"
grep -n "Solves an equation for x" Lovelace.Symbolics/SymbolicsPlugin.cs | head -2
echo "--- (a) OLD test on OLD product (expect pass) ---"
git checkout 7f69f54 -- Lovelace.Console.Tests/ReplOutputTests.cs
dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj --configuration Release --nologo 2>&1 | grep -E "Passed!|Failed!|error CS" | head -3
echo "--- (b) NEW test on OLD product (expect fail at the new pin) ---"
cp /mnt/c/Users/ricar/dev/LovelaceSharp/Lovelace.Console.Tests/ReplOutputTests.cs Lovelace.Console.Tests/ReplOutputTests.cs
dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj --configuration Release --nologo 2>&1 | grep -E "Passed!|Failed!|FAIL\]|expected line|ReplOutputTests.cs:line" | head -8
echo CONTROL_DONE
