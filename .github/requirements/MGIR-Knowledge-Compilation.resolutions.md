# MGIR Knowledge Compilation — §15 resolutions (implemented defaults)

The open questions in
[`MGIR-Knowledge-Compilation.md`](./MGIR-Knowledge-Compilation.md) §15 are resolved with the following
defaults, implemented in `Lovelace.Knowledge` and documented in
[`Lovelace.Knowledge/README.md`](../../Lovelace.Knowledge/README.md). They are configuration — the graph
itself is still built purely from observations.

1. **Ω (first run)** — pairwise `a op b` over naturals (0..12), integers (-6..6), reals (a small set of
   terminating + periodic fractions), ops `+ - * / % ^ == != > < >= <=`. Built-ins/arrays are a later
   widening.
2. **C1–C4 thresholds** — C1 new-plane rate ≤ 0.01 over the last K=3 batches; C2 100% of boundaries
   localized and stable; C3 100% held-out near-boundary agreement; C4 boundary-adjacent planes with
   support ≥ 2. Stopping = all four, plus the sample budget.
3. **σ granularity** — result-only, at two levels: exact (`ok|kind|typed` / `err|message`) kept per
   sample, and a purpose-relative plane class (kind + Boolean tag / error class) for clustering.
4. **Budget** — `MaxSamples = 700`, `BatchSize = 64`, `MinRandomSamples = 100`; each sample is one
   `Lovelace.Run` spawn. The recorded run converged at 314 samples.
