set -uo pipefail
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
cd "$HOME/ls"
dotnet --version || exit 1
mkdir -p /tmp/ci
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
  dotnet test "$p" --configuration Release --nologo --collect:"XPlat Code Coverage" > "/tmp/ci/$n.log" 2>&1
  code=$?
  e=$(( $(date +%s) - s ))
  line=$(grep -E "Passed!|Failed!|error|Aborted|crash" "/tmp/ci/$n.log" | head -5 | tr '\n' ' ')
  echo "RESULT $n EXIT=$code SECONDS=$e :: $line"
  if [ $code -ne 0 ]; then echo "FIRST_FAILURE $n"; echo "----- tail -----"; tail -30 "/tmp/ci/$n.log"; break; fi
done
echo LINUX_LOOP_DONE
