# DSH integration contract

Keep `private/` outside the agent workspace. The DSH session must never see goldens, hidden probes or oracle fingerprints.

For every condition expose a candidate-evaluation tool that:

1. accepts one complete Lovelace script;
2. executes `Lovelace.Run --stdin --json --omit-functions`;
3. applies exactly the assigned `L0`/`L1`/`L2`/`L3` projection from `bench.py::project`;
4. returns only that projection.

For L3 only, expose a second semantic-inspection tool equivalent to:

```bash
python bench.py inspect --query <name-fragment> --runner <runner>
```

This gives targeted function signature/arity/plugin metadata without dumping the full registry into every tool result.

Do not expose the repository's normal `lovelace` tool in the same benchmark session: it returns a richer surface and collapses the ablation.

Trajectory logging should record observable events only: candidate source, projected tool response, token counts, wall time, tool-call count and final completion claim. Hidden chain-of-thought is neither needed nor desired.
