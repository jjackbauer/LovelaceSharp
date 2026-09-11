set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
cd "$HOME/ls"
mkdir -p /tmp/ci2
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
for p in "${suites[@]}"; do
  n=$(basename "$p" .csproj)
  s=$(date +%s)
  dotnet test "$p" --configuration Release --nologo --collect:"XPlat Code Coverage" > "/tmp/ci2/$n.log" 2>&1
  code=$?
  e=$(( $(date +%s) - s ))
  line=$(grep -E "Passed!|Failed!|Aborted" "/tmp/ci2/$n.log" | head -3 | tr '\n' ' ')
  echo "RESULT $n EXIT=$code SECONDS=$e :: $line"
done
echo '=== Real.Tests correctness subset ==='
dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj --configuration Release --nologo --filter "Category!=Heavy&Category!=Timing" --collect:"XPlat Code Coverage" > /tmp/ci2/Real-subset.log 2>&1
echo "Real-subset EXIT=$? :: $(grep -E 'Passed!|Failed!' /tmp/ci2/Real-subset.log | head -2 | tr '\n' ' ')"
echo '=== Real.Tests timing uninstrumented ==='
dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj --configuration Release --nologo --filter "Category=Timing" > /tmp/ci2/Real-timing.log 2>&1
echo "Real-timing EXIT=$? :: $(grep -E 'Passed!|Failed!' /tmp/ci2/Real-timing.log | head -2 | tr '\n' ' ')"
echo LINUX_FULL_JOB_DONE
