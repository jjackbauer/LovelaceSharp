# Lovelace.Knowledge.Run — behavioral graph CLI

A Native AOT CLI that exposes the `Lovelace.Knowledge` tooling as **JSON-over-stdio** (the
`Lovelace.Run` pattern). Read a request, run a command, write a response.

## Build

```bash
make knowledge
# or:
dotnet publish Lovelace.Knowledge.Run/Lovelace.Knowledge.Run.csproj -c Release -f net10.0   -p:PublishAot=true -p:InvariantGlobalization=true   -o Lovelace.Knowledge.Run/bin/Release/net10.0/publish
```

## Usage

```bash
Lovelace.Knowledge.Run --eval '{"command":"converge","graphPath":"knowledge-graph.json"}'
Lovelace.Knowledge.Run --stdin < request.json
```

Request fields: `command`, `config`, `graphPath`, `runner`, `seed`, `batchSize`, `maxSamples`, `query`.

## Commands

| Command | What it does |
|---|---|
| `config` | Resolve the config (provided or defaults). |
| `sample` | Draw + execute a random Monte Carlo batch, return canonical observations. |
| `reduce` | Re-derive planes/boundaries/frontiers from a persisted graph (no execution). |
| `converge` | Autonomous loop to the C1–C4 thresholds; persists the graph. |
| `query` | Read the graph: `summary` | `planes` | `boundaries` | `frontiers` | `metrics` | `graph`. |

Exit codes: `0` success, `1` command/execution error, `2` usage error.

The CLI spawns the published `Lovelace.Run` binary for every sample; it resolves the default path
relative to its working directory (or take the `runner` field).
