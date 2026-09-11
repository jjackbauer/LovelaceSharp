set -x
export DOTNET_CLI_TELEMETRY_OPTOUT=1
export DOTNET_NOLOGO=1
curl -sSL https://dot.net/v1/dotnet-install.sh -o /tmp/dotnet-install.sh || exit 1
bash /tmp/dotnet-install.sh --channel 10.0 --install-dir "$HOME/.dotnet" || exit 1
export PATH="$HOME/.dotnet:$PATH"
export DOTNET_ROOT="$HOME/.dotnet"
dotnet --version || exit 1
python3 -m pip install --break-system-packages sympy 2>&1 | tail -2
python3 -c "import sympy; print('SYMPY', sympy.__version__)"
mkdir -p ~/ls
(cd /mnt/c/Users/ricar/dev/LovelaceSharp && tar cf - --exclude=bin --exclude=obj --exclude=.worktrees --exclude=out .) | (cd ~/ls && tar xf -) || exit 1
du -sh ~/ls
ls ~/ls | head -50
echo WSL_SETUP_DONE
