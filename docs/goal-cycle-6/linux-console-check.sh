set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
cp /mnt/c/Users/ricar/dev/LovelaceSharp/Lovelace.Console.Tests/ReplOutputTests.cs "$HOME/ls/Lovelace.Console.Tests/ReplOutputTests.cs" || exit 1
cd "$HOME/ls"
dotnet test Lovelace.Console.Tests/Lovelace.Console.Tests.csproj --configuration Release --nologo --collect:"XPlat Code Coverage" 2>&1 | grep -E "Passed!|Failed!|error CS" | head -5
echo "LINUX_CONSOLE_DONE"
