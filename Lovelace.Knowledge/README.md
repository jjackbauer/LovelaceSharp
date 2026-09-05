# Lovelace.Knowledge — observation-driven behavioral graph discovery

This library is the C# core of the **MGIR knowledge-compilation** tooling specified in
[`.github/requirements/MGIR-Knowledge-Compilation.md`](../../.github/requirements/MGIR-Knowledge-Compilation.md).
It models the LovelaceSharp engine as an **observation map** over an input space, samples it with
Monte Carlo, canonicalizes the outputs, clusters them into **behavior planes**, detects the
**boundaries** between planes, fits guards, and grows a **graph model** until it converges with the
real system.

The graph is built **purely from observations** (executions of the real `Lovelace.Run` binary).
No source, no Lean proofs, no hand-seeded nodes/edges/guards. The only inputs the agent sets are the
**sampling domain**, the **proposal distribution**, and the **convergence thresholds** (P3).

## Projects

| Project | What |
|---|---|
| `Lovelace.Knowledge` | This library: sampler, canonicalizer, reducer, convergence, graph, persistence. |
| `Lovelace.Knowledge.Run` | The JSON-over-stdio CLI (Native AOT). See its [README](../Lovelace.Knowledge.Run/README.md). |
| `Lovelace.Knowledge.Tests` | xUnit tests for the pure logic. |
| `harness/knowledge.host.js` | The thin DSH bridge (transport only — no logic). |

## §15 resolutions (proposed defaults — documented, not blocking)

**Ω (input domain).** Pairwise `a op b` over naturals + integers + reals. Defaults:

- `NaturalValues` = 0..12 (13 values)
- `IntegerValues` = -6..6 (13 values)
- `RealValues` = `-1.5, -0.5, 0.25, 0.5, 0.(3), 0.1(6), 1.5, 2.5` (negatives and fractions, including periodic)
- `Operations` = `+ - * / % ^ == != > < >= <=`
- `SweepOperations` = `- / % > <` (1-D sweeps that localize the interesting boundaries)

The first run targets the **natural-subtraction underflow** boundary (`a - b` widens Natural → Integer
at `b > a`) and the **division-by-zero** boundary (`a / b` errors at `b == 0`). Real arithmetic is
sampled randomly for breadth; it is not swept, because its interesting transitions (terminating vs
periodic reals) are number-theoretic, not contiguous intervals.

**σ (canonical observation).** Two levels:

- **σ_exact** = `ok|kind|typed` on success, `err|message` on failure (per §6). Kept per-sample as provenance.
- **σ_plane** (the behavior class used for clustering and boundaries, §4.2 purpose-relative) = the
  result **kind** (plus a `True`/`False` tag for Boolean), or the error class. So `0.5 (Real)` and
  `0.(3) (Real)` are one plane `Real`; the exact typed value is retained per-sample so the
  terminating-vs-periodic distinction is still reported.

**Thresholds (C1–C4).**

| Metric | Meaning | Default threshold |
|---|---|---|
| C1 | new-plane discovery rate over last K=3 batches | ≤ 0.01 (saturates to 0) |
| C2 | all boundaries localized (no unresolved) + stable | 100% |
| C3 | held-out near-boundary prediction agreement | 100% |
| C4 | boundary-adjacent planes have min support | ≥ 2 samples |

**Budget.** Default `MaxSamples = 700`, `BatchSize = 64`, `MinRandomSamples = 100`. The demo run
converged at **314 samples** (156 sweep + 100 random + 13 bisection + 45 validation). Each sample is one
`Lovelace.Run` process spawn (~10–20 ms), so the whole run is seconds.

## The loop (§9)

`config → sample → execute → canonicalize → reduce (cluster / detect / fit / mark frontiers) → merge
(idempotent) → measure C1–C4 → bias toward frontiers (bisection + held-out validation) → repeat`.
Every step is a deterministic function of the sample set (P5): same config + seed ⇒ same graph
(byte-identical, verified).

## Confidence levels (§5.1)

`hypothesized < observed < repeated < bounded < conformant < proven`. A plane is `observed` at 1
sample, `repeated` at ≥ 2; a boundary is `bounded` once localized (counterexamples both sides),
`conformant` once the same guard is reproduced across two step sizes (`h=2`, `h=3`). `proven` is
never assigned (Lean proofs are out of scope).

## Visuals

The discovered planes and boundaries are rendered as Mermaid diagrams in
[`BEHAVIOR-GRAPH.md`](./BEHAVIOR-GRAPH.md); the numeric evidence is in
[`CONVERGENCE-RESULTS.md`](./CONVERGENCE-RESULTS.md).

## Guard kinds

- `threshold` — uniform regions on both sides, e.g. `right > left` (subtraction underflow).
- `equality` — a single error point, e.g. `right == 0` (division/modulo by zero).
- `composite` — no simple uniform predicate is supported by the data (e.g. `Natural ↔ Real` for
  division, which is divisibility, not an interval).
