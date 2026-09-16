# Lovelace Agentic Harnessability Benchmark

A controlled benchmark for measuring how efficiently an autonomous coding agent repairs Lovelace Script as the runtime exposes progressively richer machine feedback.

This is not a "which model memorized Lovelace?" benchmark. The primary experiment holds model, task, runner and budget fixed while changing only the observation surface.

## Observation modes

| Mode | Agent-visible feedback |
|---|---|
| `L0` | success + human result; failures collapse to `failed` |
| `L1` | L0 + human diagnostic messages |
| `L2` | structured values/variables + error category/recoverability + source locations |
| `L3` | L2 + execution timings/cancellation telemetry + **on-demand** function registry lookup |

L3 registry metadata is queried with `bench.py inspect`; it is not dumped into every evaluation result. This keeps semantic introspection available without forcing a large context tax on every turn.

## Corpus

The v1 corpus contains 20 repair tasks across syntax, name resolution, arity, arrays/shapes, indexing, precedence, exact numerics, recursion, matrix APIs and symbolic operations.

Each task exposes one visible example but is scored against additional hidden functional probes. Current corpus: **48 hidden probes**. This prevents a model from replacing a broken program with the visible constant answer.

Goldens and probes belong to the evaluator and must not be placed in the agent workspace.

Recommended layout:

```text
/bench-controller/                  # this package; private evaluator data lives here
/work/LovelaceSharp/                # agent cwd; no private/ directory
```

## Validate against a real runner

```bash
python bench.py validate \
  --runner /work/LovelaceSharp/Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run
```

Validation checks:

- every golden visible example executes;
- every hidden probe executes against its golden;
- no initial broken seed passes all hidden probes;
- all 80 task × observation-mode projections serialize;
- L3 semantic registry lookup works.

## Prepare hidden oracle fingerprints

```bash
python bench.py prepare --runner /path/to/Lovelace.Run
```

The oracle fingerprints only the **final structured result value**. It deliberately ignores intermediary variable names, helper functions, pretty/display strings, timings and revision numbers, so semantically equivalent implementations can pass.

## Agent-facing evaluation

```bash
python bench.py observe --mode L2 --task shape-matmul \
  --candidate candidate.ls --runner /path/to/Lovelace.Run
```

L3 additionally exposes targeted semantic introspection:

```bash
python bench.py inspect --query det --runner /path/to/Lovelace.Run
```

The DSH wrapper should expose only the observation surface assigned to that experiment. Do not also load the normal richer `lovelace` tool or the ablation is invalid.

## Trajectory format

One JSON object per run in JSONL form:

```json
{
  "run_id": "ds-v4pro-L2-001",
  "task_id": "shape-matmul",
  "mode": "L2",
  "model": "deepseek-v4-pro",
  "replicate": 1,
  "events": [
    {"type":"candidate","script":"...","input_tokens":1200,"output_tokens":180},
    {"type":"observation","observation":{"ok":false}}
  ],
  "final_script": "...",
  "agent_claimed_done": true
}
```

No chain-of-thought is required or recorded.

## Score

```bash
python bench.py score --runs runs.jsonl --runner /path/to/Lovelace.Run --budget 10
```

Metrics:

- success rate and Success@1/3/5/10;
- median evaluations-to-green;
- median tokens-to-green when token accounting is supplied;
- repeated-diagnostic rate;
- false-convergence rate;
- bounded harnessability `H_B`.

For an evaluation budget `B`, let `S(k)` be the fraction of runs solved by evaluation `k`:

```text
H_B = (1/B) * sum(k=1..B) S(k)
```

`H_B = 1` means every run converged on its first evaluation. Late convergence contributes less; censored runs contribute zero after the budget.

## First empirical matrix

```text
20 tasks × 4 modes × 5 replicates = 400 trajectories
```

Hold constant model/version, DSH preset, sampling parameters, reference condition, max evaluations, LovelaceSharp commit and environment. Randomize task order across replicates.

Run the matrix separately under:

1. zero-shot Lovelace;
2. compact language reference supplied;
3. reference + permitted docs/help retrieval.

Do not pool these conditions. They separate model prior from language learnability and harness feedback.

## Interpretation

The causal comparison is L0 vs L1 vs L2 vs L3. A later Lovelace-vs-Python/Julia/C# study is useful only after this within-language ablation is stable.

The question is deliberately narrow:

> How much agent search/compute does a machine-oriented Lovelace runtime surface save?
