# Convergence results — one real run against the LovelaceSharp engine

> **Visual version:** [BEHAVIOR-GRAPH.md](./BEHAVIOR-GRAPH.md) renders the planes and boundaries as
> Mermaid diagrams.

This records a real `converge` run of [`Lovelace.Knowledge.Run`](../Lovelace.Knowledge.Run) against the
published `Lovelace.Run` binary. Config = defaults ([README](./README.md)), seed `20240617`. The graph
was built purely from observations — no source, no proofs, no hand-seeded nodes.

**Reproduce:**

```bash
make knowledge && make runner
Lovelace.Knowledge.Run/bin/Release/net10.0/publish/Lovelace.Knowledge.Run.exe \
  --eval '{"command":"converge","graphPath":"knowledge-graph-demo.json"}'
```

The result is deterministic: same config + seed reproduces a **byte-identical** graph (SHA-256
`d445672bafad0af23ef4d5596144201bdf01ce3e961909eeaa8f18be50ef8f76`), across both Debug and Native-AOT
builds and across process boundaries (P5).

## Summary

- **Samples**: 314 (156 coarse sweeps ×2 step sizes + 100 random + 13 bisection + 45 held-out validation).
- **Planes**: 8 distinct behavior classes, discovered purely from observations.
- **Boundaries**: 18 localized, every one with counterexamples on both sides.
- **Converged**: yes — `C1 saturated (new-plane rate 0), C2 18/18 localized+stable, C3 45/45 agreed, C4 covered`.

## Behavior planes learned

| Plane | Support | Confidence |
|---|---|---|
| `Natural` | 82 | repeated |
| `Real` | 73 | repeated |
| `Integer` | 59 | repeated |
| `Boolean:False` | 41 | repeated |
| `Boolean:True` | 37 | repeated |
| `err|Cannot divide by zero.` | 19 | repeated |
| `err|Non-integer exponents are not yet supported.` | 2 | repeated |
| `err|Exponent must be positive. (Parameter 'exponent')` | 1 | observed |

The `Real` plane holds both terminating and periodic values (`0.5 (Real)`, `0.(3) (Real)`,
`0.1(6) (Real)`, …) — the engine's exact period detection is visible in the observations even though
both share the one `Real` behavior class.

## Discovered boundaries (with bounding evidence)

**1. Natural-subtraction underflow** — guard `right > left`, localized at `a == b`.

| anchor | below (Natural) | above (Integer) |
|---|---|---|
| 5 | `5 - 5 = 0 (Natural)` | `5 - 6 = -1 (Integer)` |
| 10 | `10 - 10 = 0 (Natural)` | `10 - 11 = -1 (Integer)` |
| 2 | `2 - 2 = 0 (Natural)` | `2 - 3 = -1 (Integer)` |
| 1 | `1 - 1 = 0 (Natural)` | `1 - 2 = -1 (Integer)` |
| 0 | `0 - 0 = 0 (Natural)` | `0 - 1 = -1 (Integer)` |

**2. Division by zero** — guard `right == 0`, localized at `b == 0`.

| anchor | below (error) | above (valid) |
|---|---|---|
| 5 | `5 / 0` → `Cannot divide by zero.` | `5 / 1 = 5 (Natural)` |

(Same for `/` anchors 1, 2, 10 and `%` anchors 5, 10.)

**3. Comparison flips** — Boolean `True ↔ False` at the equality point.

| op | below | above |
|---|---|---|
| `5 > b` | `5 > 4 = True` | `5 > 5 = False` |
| `5 < b` | `5 < 5 = False` | `5 < 6 = True` |

**4. Exact-vs-fractional division** — `Natural ↔ Real` at divisors of the anchor. These are marked
`composite` (divisibility is not a contiguous interval), reported with their bounding samples rather
than a simple predicate — e.g. `5 / 1 = 5 (Natural)` vs `5 / 2 = 2.5 (Real)`.

## Remaining frontiers (open world)

- 18 `weak-dimension` cells (unsampled op × domain-pair combinations, e.g. `+ over Integer×Natural`).
- 1 `low-support` plane (`err|Exponent must be positive…`, observed once).

These are reported, not hidden (C4).
