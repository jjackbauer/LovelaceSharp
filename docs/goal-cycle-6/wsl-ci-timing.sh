set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
rm -rf "$HOME/ls-ci"
mkdir -p "$HOME/ls-ci" "$HOME/ci-logs"
cd /mnt/c/Users/ricar/dev/LovelaceSharp
git archive e8638c0 | tar -x -C "$HOME/ls-ci"
cd "$HOME/ls-ci"
suites=(
 Lovelace.Abstractions.Tests/Lovelace.Abstractions.Tests.csproj
 Lovelace.Array.Tests/Lovelace.Array.Tests.csproj
 Lovelace.Complex.Tests/Lovelace.Complex.Tests.csproj
 Lovelace.Console.Tests/Lovelace.Console.Tests.csproj
 Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.csproj
 Lovelace.Integer.Tests/Lovelace.Integer.Tests.csproj
 Lovelace.Knowledge.Tests/Lovelace.Knowledge.Tests.csproj
 Lovelace.Natural.Tests/Lovelace.Natural.Tests.csproj
 Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj
 Lovelace.Run.Tests/Lovelace.Run.Tests.csproj
 Lovelace.Studio.Tests/Lovelace.Studio.Tests.csproj
 Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj
 Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj
 precbench.Tests/precbench.Tests.csproj
)
start_all=$(date +%s)
for p in "${suites[@]}"; do
  n=$(basename "$p" .csproj)
  s=$(date +%s)
  dotnet test "$p" --configuration Release --nologo --collect:"XPlat Code Coverage" > "$HOME/ci-logs/$n.log" 2>&1
  code=$?
  e=$(( $(date +%s) - s ))
  line=$(grep -E "Passed!|Failed!|Aborted" "$HOME/ci-logs/$n.log" | head -2 | tr '\n' ' ')
  echo "RESULT $n EXIT=$code SECONDS=$e :: $line"
done
echo "TOTAL_LOOP_SECONDS=$(( $(date +%s) - start_all ))"
echo WSL_CI_TIMING_DONE
