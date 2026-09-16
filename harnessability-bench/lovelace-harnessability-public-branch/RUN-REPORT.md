# Validation run report

Runner source: self-contained Linux x64 `Lovelace.Run` artifact produced by LovelaceSharp CI run #95 on commit `f264eed390deae96f33075bb797a9507fd33cf10`.

## Real execution results

- Python controller compilation: PASS
- Golden tasks: 20/20 PASS
- Hidden functional probes: 48/48 PASS
- Broken seeds satisfying hidden oracle: 0/20
- Observation projections executed/serialized: 80/80 PASS
- L3 function-registry smoke (`det` lookup): PASS
- Verifier invariants:
  - hard-coded factorial visible answer: rejected
  - hard-coded determinant visible answer: rejected
  - equivalent determinant implementation: accepted
  - equivalent exact-third implementation: accepted
  - correct Fibonacci function without visible sample call: accepted
- Synthetic scorer integration test: 80/80 two-step trajectories scored green at step 2 in every mode; `H_B = 0.9` as expected for budget 10.

## Observation payload sizes on broken seeds

| Mode | Mean bytes | Max bytes |
|---|---:|---:|
| L0 | 55.2 | 440 |
| L1 | 94.0 | 440 |
| L2 | 643.5 | 4,424 |
| L3 | 852.9 | 4,716 |

An earlier L3 implementation dumped the full function registry on every success and averaged roughly 10 KB per observation, with a maximum around 25 KB. Execution exposed that as a confound; L3 was changed to targeted on-demand inspection.

## Not yet executed

The 400 **model trajectories** have not been run because this environment does not expose a DSH/DeepSeek agent runtime or model credentials. The language/runtime/controller/scorer pipeline itself has been executed against the real Lovelace binary.
