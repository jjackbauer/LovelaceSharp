# Lovelace agentic-harnessability benchmark — execution record

This directory is the working copy of the `lovelace-harnessability-public-branch`
package, made runnable and then actually run on this machine.

Nothing in the agent-visible half was modified: `bench.py`, `tasks.json`,
`run.schema.json` and the shipped reports are byte-identical to the zip.  What
was added is the evaluator-owned half the public branch deliberately omits, plus
the DSH integration that `DSH-INTEGRATION.md` specifies and that the branch's
`RUN-REPORT.md` says was never executed.

## 1. What the package actually measures

The zip is **not** a Lovelace-vs-Python comparison.  `README.md` is explicit: it
is a *within-language* ablation that holds model, task, runner and budget fixed
and varies only the **observation surface** the harness exposes.

| Mode | Agent-visible feedback |
|---|---|
| `L0` | success + human result; failures collapse to `failed` |
| `L1` | L0 + human diagnostic messages |
| `L2` | structured values/variables + error category/recoverability + source locations |
| `L3` | L2 + execution timings + **on-demand** function registry lookup |

The Lovelace-vs-Python/Julia/C# study is named in `README.md` as a *later*
study, gated on this ablation being stable.  The primary metric is bounded
harnessability `H_B = (1/B) * sum(k=1..B) S(k)`: 1.0 means every run converged on
its first evaluation.

## 2. What was missing, and what was reconstructed

The public branch ships the controller and the 20 broken seeds, but `private/`
(goldens, hidden probes, oracle fingerprints) is gitignored and absent.  Without
it `bench.py validate`, `prepare` and `score` cannot run at all.

`lovelace-harnessability-public-branch/private/build_corpus.py` reconstructs it
deterministically: 20 goldens and 48 hidden functional probes (12 tasks x 2 +
8 tasks x 3).  Every probe must discriminate its shipped broken seed, so a
constant "visible answer" cannot satisfy the oracle while an equivalent
implementation still passes.

Reconstruction fidelity, against the branch's own published
`validation-summary.json`:

| Quantity | Published | Reconstructed |
|---|---:|---:|
| tasks | 20 | 20 |
| goldens verified | 20 | 20 |
| hidden probes | 48 | 48 |
| broken seeds satisfying the oracle | 0 | 0 |
| seed executions ok / error | 12 / 8 | 12 / 8 |
| observation projections | 80 | 80 |
| L3 registry smoke | true | true |
| mean L0 / L1 / L2 bytes | 55.2 / 94.0 / 643.5 | 55.2 / 94.0 / 643.5 |

```
bench.py validate --runner runner/Lovelace.Run.exe
-> ok: true, goldens 20/20, hidden probes 48/48, accidental seed matches [],
   80 projections, L3 registry smoke true, seed_execution 12 ok / 8 error
```

The branch's verifier invariants also hold (`checks/verify_invariants.py`,
6/6): a hard-coded factorial or determinant answer is rejected, an equivalent
determinant (`det(transpose(m))`) and an equivalent exact-third (`n * (1/3)`)
are accepted, and a correct `fib` with no visible sample call passes.

Four shipped seeds no longer match `validation-report.json` (16/20 match
exactly).  `checks/check_seed_fidelity.py` lists them; each difference is a
corpus edit, not a runtime difference:

* `range-precedence` — the report's visible call produced `1..100`; the shipped
  seed calls `squares(5)` and yields `1..25`;
* `symbolic-diff` — the report says `Unknown function 'derivative'`; the shipped
  seed calls `derive`;
* `solve-equation` — the report's seed solved `x^2-4=0` exactly; the shipped seed
  subtracts 1 first and correctly yields `+/-sqrt(5)`;
* `reshape-numel` — the report's seed raised a shape error; the shipped seed now
  succeeds while returning the transposed shape.

## 3. What had to be provisioned

Nothing about the benchmark could run as-is on this machine:

* **No Python** — only the Microsoft Store alias stub, which fails.  A CPython
  3.12.10 embeddable runtime is provisioned into `tools/python`.
* **The published runner predated `--omit-functions`** and rejected the flag
  `bench.py` depends on.  `Lovelace.Run` is republished as a Native AOT
  `win-x64` binary from this checkout, which sits at `f264eed` — the exact
  commit the branch's `RUN-REPORT.md` names.
* **No agent runtime** — `RUN-REPORT.md` records that the model trajectories
  "have not been run because this environment does not expose a DSH/DeepSeek
  agent runtime or model credentials".  This machine does.

## 4. The DSH integration that closes the loop

`dsh/` implements the contract in `DSH-INTEGRATION.md`:

* `dsh/bench-mcp/server.mjs` — a stdio MCP server exposing
  `evaluate_candidate` (one complete script in, exactly the assigned
  L0/L1/L2/L3 projection out), `inspect_functions` (registered in L3 only) and
  `submit_final`.  The projection is a port of `bench.py::project`; the server
  never sees goldens, probes or oracle fingerprints.
* `dsh/bench.patch.yml` — a DSH profile overlay that disables every first-party
  tool plugin (shell, filesystem, search, skills, subagents, web, todos, goals,
  plan mode, code runtime) and mounts only the benchmark server.  This is what
  keeps the ablation valid.
* `dsh/run_matrix.py` — boots one headless DSH session per (task, mode,
  replicate), records the events the surface actually served, attributes token
  usage and the exposed tool list from the DSH session log by content matching
  (scripts + observation payloads), and emits `run.schema.json`-shaped JSONL for
  `bench.py score`.

Every recorded run carries the tool list DSH actually exposed.  Across all 80
runs it is exactly:

```
L0/L1/L2 : mcp__bench__evaluate_candidate, mcp__bench__submit_final
L3       : mcp__bench__evaluate_candidate, mcp__bench__inspect_functions, mcp__bench__submit_final
```

A scan of every agent-visible log (reasoning stream and final messages) finds no
reference to `goldens`, `probes`, `oracles` or the private directory.

## 5. Reproducing

```powershell
./setup.ps1                     # python, AOT runner, corpus, oracles, validate, links
./run-bench.ps1                 # 20 tasks x 4 modes x 1 replicate = 80 real DSH runs + score
python checks/verify_invariants.py      # 6/6 RUN-REPORT invariants hold
python checks/check_seed_fidelity.py   # 16/20 seeds match the published report
python checks/make_synthetic_runs.py   # 80 synthetic trajectories, H_B = 0.9 as published
python checks/validate_runs_schema.py  # every trajectory conforms to run.schema.json
python checks/summarize_matrix.py      # the tables in section 6
```

All five checks pass on the committed state, and `bench.py validate` is green.

## 6. First empirical matrix

**80 real DSH headless sessions** — 20 tasks x 4 observation modes x 1 replicate,
model `deepseek-flash` (provider `deepseek-official`), evaluation budget 10, all
four arms run concurrently, 664k model tokens on candidate turns, 4,882 s of
agent wall time in total.  Scored by the controller's own `bench.py score`
against the hidden probes.

| Mode | n | solved | Success@1 | Success@3 | median evals to green | median tokens to green | false convergence | repeated diagnostics | **H_10** |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| L0 | 20 | 80% | 0.20 | 0.65 | 2.0 | 881.5 | 0.05 | 0.47 | **0.675** |
| L1 | 20 | 85% | 0.30 | 0.70 | 2 | 670 | 0.10 | 0.01 | **0.715** |
| L2 | 20 | 95% | 0.25 | 0.80 | 2 | 941 | 0.05 | 0.00 | **0.805** |
| L3 | 20 | 100% | 0.80 | 1.00 | 1.0 | 717.0 | 0.00 | 0.00 | **0.980** |

`H_10` is monotone in observation richness: **0.675 -> 0.715 -> 0.805 -> 0.980**.
Solving the same 20 repairs costs the agent 6.6 evaluations per task under L0 and
4.6 under L3, and the L3 arm reaches 80% first-try success against L0's 20%.

Harness-side facts behind those numbers:

| Mode | mean evals | mean wall s | mean tool calls | L3 registry queries | exposed tools |
|---|---:|---:|---:|---:|---|
| L0 | 6.60 | 59.0 | 7.75 | 0 | 2 |
| L1 | 5.55 | 86.0 | 6.65 | 0 | 2 |
| L2 | 5.15 | 56.1 | 6.20 | 0 | 2 |
| L3 | 4.55 | 43.0 | 14.80 | 185 | 3 |

Per task — the first evaluation that passes every hidden probe
(`-` = used all 10 evaluations without solving):

| Task | L0 | L1 | L2 | L3 |
|---|---|---|---|---|
| syntax-missing-brace | 1 | 1 | 1 | 1 |
| undefined-name | - | - | 8 | 1 |
| arity-dot | 2 | 2 | 2 | 1 |
| shape-matmul | 2 | 2 | 2 | 1 |
| index-rank | 2 | 2 | 2 | 2 |
| range-precedence | 8 | 10 | 2 | 2 |
| mean-wrong-op | 1 | 1 | 1 | 1 |
| matrix-inverse | - | - | - | 1 |
| factorial-off-by-one | 1 | 2 | 1 | 1 |
| fib-base-case | 1 | 1 | 1 | 1 |
| symbolic-diff | 6 | 6 | 3 | 1 |
| symbolic-expand | 2 | 2 | 2 | 2 |
| symbolic-factor | 2 | 1 | 2 | 1 |
| solve-equation | 2 | 2 | 2 | 2 |
| limit-point | - | 2 | 2 | 1 |
| reshape-numel | 2 | 1 | 2 | 1 |
| transpose-purpose | - | - | 7 | 1 |
| exact-third | 2 | 1 | 1 | 1 |
| sum-squares | 2 | 2 | 2 | 1 |
| matrix-trace | 5 | 6 | 5 | 1 |

### What the surface buys

Three distinct mechanisms, not one:

1. **Error text (L0 -> L1).**  Under L0 every failure is the same string,
   `{"ok": false, "error": "failed"}`, so the agent repeats uninformative probes
   (repeated-diagnostic rate 0.47) and retreats into brute force: L0 needs 8
   evaluations on `range-precedence` where L2 needs 2.  Messages alone collapse
   the repeat rate to 0.01.
2. **Structured values (L1 -> L2).**  L2 is the only mode in which the agent can
   see *what a program actually computed* when it succeeds, which is what silent
   wrong-answer tasks need (`symbolic-diff` 6 -> 3, `range-precedence` 10 -> 2,
   `transpose-purpose` unsolved -> 7).
3. **API discovery (L2 -> L3).**  L3's on-demand registry lookup converts the
   harness from a checker into a discoverer.  It is used heavily — 185 queries
   over 20 runs, e.g. `det`, `inverse`, `trace`, `deriv`, `rational`, `diag` —
   and every task that depends on knowing a builtin's real name is solved on the
   **first** evaluation (`undefined-name` 8 -> 1, `matrix-inverse`
   unsolved -> 1, `transpose-purpose` 7 -> 1, `symbolic-diff` 3 -> 1).  The
   price is visible too: L3 makes 14.8 tool calls per task against L2's 6.2, so
   the win is search-structure, not raw token thrift.

### False convergence

Four runs claimed completion with a program that still fails the hidden probes,
all of them on tasks whose repair hinges on a builtin the agent could not name:
two hand-rolled a `pow2`-style doubling loop for `matrix_det`
(`L0`/`L1 `undefined-name`), one hand-rolled a pivoting inverse
(`L1 matrix-inverse`), one hand-rolled 2x2/3x3 determinant helpers
(`L2 matrix-inverse`).  No L3 run false-converged — when the registry is
queryable, "I could not find the function" turns into "here is the function".

### Caveats

* **n = 1 replicate per cell.**  The package's full first matrix is
  20 x 4 x 5 = 400 trajectories with randomised task order; this run is the
  80-trajectory single-replicate version, so per-task cells are anecdotes and
  only the aggregate ordering is readable.  `L1` vs `L2` on
  `undefined-name` (unsolved vs solved at 8) is exactly the kind of cell that
  needs replicates before it is a finding.
* **One model, one preset.**  `deepseek-flash` at `reasoningEffort: max`, with
  no compact language reference supplied — the zero-shot condition of the
  package's three planned conditions.  The reference-supplied and
  retrieval-allowed conditions were not run.
* **The corpus is a reconstruction.**  It reproduces every published aggregate,
  but it is not the branch's original `private/`; probe-for-probe equality with
  the original cannot be claimed.
* **L3 is not free.**  Its surface changes the tool catalog, so L3 vs L2 measures
  "more tools" as well as "more telemetry"; the timings channel of L3 is barely
  load-bearing on its own.

---

## 7. Gate run A — the first API enhancement, measured

The plan in `API-ENHANCEMENT-PLAN.md` `§2 WS-1/WS-5` was implemented against the runtime and the
same 80-cell matrix was re-run against the rebuilt binary.

**What shipped** (runner `sha256 2E3E365B…9548A8F`, recorded per trajectory as
`harness.runner_sha256`):

* an unresolved name now crosses as `code: UnknownFunction`, `category: UnsupportedOperation`,
  `recoverable: true` — previously `InvalidOperation`/`DomainError`, the same pair a genuine
  domain refusal uses;
* the envelope carries the registry's answer: `suggestions` (up to three registered names ranked
  case-insensitively, by prefix, by near-miss, then by shared opening — omitted, never `[]`, when
  nothing is close), `hints` (syntax notes: `range` is the operator `1..n`), and `registrySize`;
* the message repeats the first suggestion in prose and is byte-identical to the old sentence when
  there is nothing to offer, so no recorded message contract moved.

Two contract decisions were forced by the repository's own tests and are worth recording:

* **The error taxonomy stays frozen at seven members.** `Lovelace.Symbolics.Tests` pins
  `ErrorCategory` by name and count; a new failure class therefore takes a new **code** and the
  coarse category it belongs to, exactly as `FileReadError`/`TypeMismatch` and
  `AllocationRefused`/`BudgetExceeded` already do. `UnsupportedOperation` is the member that
  means "this runtime does not offer that operation".
* **Short names are dense.** The first ranker offered `det` and `dot` for `fft` — two edits away
  and useless — which broke a pinned message contract. Names of four characters or fewer are now
  held to a single edit.

**Results** (80 runs each; same model, tasks, projections, budget 10):

| Mode | baseline solved | gate solved | baseline H_10 | gate H_10 | delta | mean evals (base → gate) |
|---|---:|---:|---:|---:|---:|---:|
| L0 | 80% | 95% | 0.675 | 0.750 | +0.075 | 6.60 → 5.80 |
| L1 | 85% | 95% | 0.715 | **0.860** | +0.145 | 5.55 → 4.55 |
| L2 | 95% | 100% | 0.805 | **0.880** | +0.075 | 5.15 → 3.95 |
| L3 | 100% | 100% | 0.980 | 0.950 | −0.030 | 4.55 → 4.20 |

| # | Hypothesis | Verdict | Evidence |
|---|---|---|---|
| H1 | L1 solves `undefined-name` and `matrix-inverse` in ≤ 3 evaluations | **HOLDS** | both solved at evaluation 2 (baseline: unsolved in both) |
| H2 | H_10(L1) ≥ 0.80 | **HOLDS** | 0.860 |
| H3 | H_10(L2) ≥ 0.88 | **HOLDS** | 0.880 |
| H4 | L3 − L2 gap ≤ 0.10 | **HOLDS** | 0.175 → 0.070 |
| H5 | L1 false convergences ≤ 1 | **HOLDS** | 2 → 1 (L2 1 → 0) |
| H6 | L0 drift ≤ 0.03 | **FALSIFIED** | +0.075 |
| H7 | no lost solves; unknown-name failures fall | **HOLDS** | no lost solves; 15 → 8 |

**H6 is a sampling result, not a leak.** The control arm was expected to sit still because its
projection discards everything but success and `"failed"`. It moved because the blind arm's search
is a coin flip near the budget boundary:

* L0 observations in the gate run carry exactly the keys `ok`, `error`, `result` and **zero**
  occurrences of suggestion text — the surface is provably unchanged;
* the three cells that flipped (`undefined-name`, `matrix-inverse`, `limit-point`) solved at
  evaluations 10, 9 and 2 in the gate while using all 10 in the baseline — the signature of a blind
  search that happened to land inside the budget this time;
* `matrix-trace` moved the other way in the same arm (5 → 8 evaluations).

The honest conclusion is therefore about the instrument, not the change: **a single replicate of the
L0 arm carries roughly ±0.075 of noise**, which is larger than several effects the benchmark is
being asked to detect. Replicates are not a nicety here; they are required for any L0 claim.

**Mechanism.** The failures the change targets did not merely get faster, they stopped being dead
ends: unknown-name failures fell from 15 to 8, and the names the agent tried collapsed to the four
task seeds' first guesses (`determinant`, `inverse`, `derive`, `range` — twice each, once per
observation mode that still stalls on the first try). The category histogram now separates the
classes — `UnsupportedOperation: 4` is exactly the name-resolution residue, where the baseline
folded all of it into `DomainError`.

The remaining unknown-name failures are real work left on the table, and they are specific: L0/L1
still cannot solve `transpose-purpose`, because the agent reaches for `rows` (which now suggests
`shape`) when the repair needs `transpose`. A synonym table is not a substitute for a registry
query — which is what WS-3 is for.

**Cost of the gate.** ~3 min AOT publish, ~6 min corpus revalidation, ~40 min of parallel agent
wall clock, ~660k model tokens per arm set.

**Not shipped** (still open from the plan): WS-2 arity signatures, WS-3 `--functions` registry
query, WS-4 partial variables (still 0% coverage — `checks/measure_surface.py` reports it), WS-6
harness defaults, and WS-1.3 alias resolution.
