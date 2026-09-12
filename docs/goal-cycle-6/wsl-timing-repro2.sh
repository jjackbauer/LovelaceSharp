set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
cd /mnt/c/Users/ricar/dev/LovelaceSharp
git archive HEAD | tar -x -C "$HOME/ls-timing2" 2>/dev/null || (mkdir -p "$HOME/ls-timing2" && git archive HEAD | tar -x -C "$HOME/ls-timing2")
cp Lovelace.Run.Tests/CancellationBudgetTests.cs "$HOME/ls-timing2/Lovelace.Run.Tests/CancellationBudgetTests.cs"
cd "$HOME/ls-timing2"
for i in 1 2 3; do
  dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj --configuration Release --nologo --filter "Category=Timing" 2>&1 | grep -E "Passed!|Failed!|Error Message|the excess|exceeded" | head -4
  echo "--- run $i done ---"
done
echo WSL_TIMING2_DONE
