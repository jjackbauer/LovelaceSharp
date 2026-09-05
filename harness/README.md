# LovelaceSharp DSH harness

A DeepSeek Harness (DSH) integration that exposes a **`lovelace`** model tool for
authoring and testing [Lovelace.Suite](../Lovelace.Suite/README.md) scripts without the
interactive REPL. The tool evaluates a script, then returns the result, every named
variable, the functions in scope, and any plot (path + title + inline SVG) as JSON.

There are two pieces:

| Piece | Where | What it does |
|---|---|---|
| `Lovelace.Run` | [`../Lovelace.Run`](../Lovelace.Run) | Non-interactive .NET CLI that runs a script through `SuiteEngine` and emits a JSON envelope. |
| `lovelace.host.js` | [`lovelace.host.js`](./lovelace.host.js) | A Dynamic Cordis Plugin `code.host` body that registers the `lovelace` tool and drives `Lovelace.Run` through the DSH `subprocess` service. |

The plugin is a thin bridge: the model calls `lovelace { script: … }`, the plugin spawns
the runner with the script on stdin, and it parses the JSON back. All language logic stays
in `Lovelace.Suite`.

## Prerequisites

- .NET 10 SDK (same requirement as the rest of the repo).
- A DSH session whose **working directory is this repository** — the plugin resolves the
  runner path from the session's cwd (`exec.agent.session.header.cwd`, the same source the
  harness's own filesystem/LSP tools use), so it works for anyone who opens the repo as their
  workspace with no hardcoded absolute paths.

## 1. Build the runner

```bash
make runner
# or, equivalently:
dotnet publish Lovelace.Run/Lovelace.Run.csproj --configuration Release \
  --framework net10.0 \
  -p:PublishAot=true -p:InvariantGlobalization=true \
  --output Lovelace.Run/bin/Release/net10.0/publish
```

The plugin defaults to the published native binary at
`Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe`. On non-Windows the binary has
no `.exe` suffix — pass its path via the tool's optional `runner` argument.

## 2. Load the plugin

The plugin is a *dynamic* Cordis plugin, so it is loaded per session with the DSH
`cordis_define` / `cordis_run` tools (the same tools used to author any dynamic plugin):

1. `cordis_define` — `kind: "new"`, `idPrefix: "lovel"`, and set `code.host` to the
   **body** of [`lovelace.host.js`](./lovelace.host.js) (everything from `return {` to the
   matching closing `}` — or just the whole file, the leading `//` comments are valid).
   Leave `code.client` unset; this is a Host-only tool.
2. `cordis_run` — activate the returned `pluginId`/`packageId` with mode `run`.

After activation the `lovelace` tool becomes callable on the next model step.

## 3. Use it

```text
lovelace { "script": "x = 1..10\ny = 1 / x^2\nplot(x, y, \"1/x^2\")" }
```

Optional arguments:

- `plotDir` — directory for `plot()` SVG output (default: workspace root; created on demand).
- `plotFile` — filename for the SVG (default: `plot.svg`, which is gitignored).
- `runner` — explicit path to the runner executable (overrides the default).

The tool returns:

```text
result: C:\…\plot.svg
  x = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]  (Vector)
  y = [1, 0.25, 0.(1), 0.0625, 0.04, 0.02(7), …]  (Vector)
plot: C:\…\plot.svg
  title: "1/x^2"
  svg bytes: 3470
```

The canonical (logged) value also includes the full SVG and the functions in scope, so
tests and scripts can assert on the raw JSON.

## Example scripts

- [`examples/invx2.ls`](./examples/invx2.ls) — the canonical `1/x²` plot.

## Notes / limits

- Each tool call runs a fresh `SuiteEngine` (no state carries between calls) — keep each
  script self-contained.
- The `lovelace` tool returns a JSON envelope (`ok`, `result`, `variables`, `functions`,
  `plot`, `diagnostics`) produced by `Lovelace.Run`; see
  [`../Lovelace.Run/Program.cs`](../Lovelace.Run/Program.cs) for the exact shape.
- This is a per-session dynamic plugin. To make it a permanent, always-on tool for a
  machine, publish it as an npm package and reference it from an agent preset
  (`agent.cordis.yml`) — the dynamic-plugin form here is the repo-local, zero-publish path.

---

## MGIR behavioral graph discovery — the `mgir` tool

A second, thin bridge ([knowledge.host.js](./knowledge.host.js)) exposes the **observation-driven
behavioral graph discovery** tooling from [`.github/requirements/MGIR-Knowledge-Compilation.md`](../.github/requirements/MGIR-Knowledge-Compilation.md).
It registers a `mgir` tool that marshals a JSON request and spawns the
[`Lovelace.Knowledge.Run`](../Lovelace.Knowledge.Run) CLI via the DSH `subprocess` service — transport
only, no sampling/reduction/graph logic (that all lives in C#).

Build the CLI and the runner, then load the plugin exactly like `lovelace`:

```bash
make knowledge   # publishes Lovelace.Knowledge.Run (Native AOT)
make runner      # publishes Lovelace.Run (the sample executor)
````

```text
mgir { "command": "converge", "graphPath": "knowledge-graph.json" }
mgir { "command": "query", "graphPath": "knowledge-graph.json", "query": "boundaries" }
```

Commands: `config`, `sample`, `reduce`, `converge` (the autonomous loop), `query`. The persisted
graph JSON (`knowledge-graph.json` by default) is the durable product; `converge` may be run as a DSH
background job. The tool registrations belong to the plugin fiber (reversible on stop/update).
