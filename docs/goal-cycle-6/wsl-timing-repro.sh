set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
rm -rf "$HOME/ls-timing"; mkdir -p "$HOME/ls-timing"
cd /mnt/c/Users/ricar/dev/LovelaceSharp
git archive 99586b6 | tar -x -C "$HOME/ls-timing"
cd "$HOME/ls-timing"
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj --configuration Release --nologo --filter "Category=Timing" 2>&1 | tail -40
echo WSL_TIMING_DONE
