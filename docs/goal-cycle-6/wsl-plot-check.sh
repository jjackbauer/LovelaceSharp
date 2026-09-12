set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"; export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1; export DOTNET_NOLOGO=1
rm -rf "$HOME/ls-plot"; mkdir -p "$HOME/ls-plot"
cd /mnt/c/Users/ricar/dev/LovelaceSharp
git archive HEAD | tar -x -C "$HOME/ls-plot"
cp Lovelace.Run.Tests/PlotDirectoryErrorTests.cs "$HOME/ls-plot/Lovelace.Run.Tests/PlotDirectoryErrorTests.cs"
cp Lovelace.Run/Runner.cs "$HOME/ls-plot/Lovelace.Run/Runner.cs"
cd "$HOME/ls-plot"
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj --configuration Release --nologo --filter "FullyQualifiedName~PlotDirectoryErrorTests" 2>&1 | tail -6
echo WSL_PLOT_DONE
