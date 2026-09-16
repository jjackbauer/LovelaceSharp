# Lovelace API enhancement plan — from measured harnessability evidence

**Goal.** Raise the number of repairs an agent completes per unit of search, by
changing what the *runtime API* tells the agent — not by changing the model, the
tasks, or the budget.

**Gate.** Rebuild the runner, re-validate the corpus, re-run the same 80-cell
matrix, and compare against the frozen baseline in section 5. No workstream is
"done" until the matrix moves.

---

## Status — gate run A executed

**WS-1.1, WS-1.2 and WS-5 are implemented and measured** (runner
`sha256 2E3E365B…9548A8F`, all four affected test suites green: 897 Suite, 361 Run,
1152 Symbolics, 20 Abstractions). Against the frozen baseline:

| Mode | H_10 baseline | H_10 gate A | delta |
|---|---:|---:|---:|
| L0 | 0.675 | 0.750 | +0.075 (single-replicate noise — see below) |
| L1 | 0.715 | **0.860** | +0.145 |
| L2 | 0.805 | **0.880** | +0.075 |
| L3 | 0.980 | 0.950 | −0.030 |

Six of seven hypotheses hold, including the predicted collapse of the L3 advantage
(0.175 → 0.070): once the failure itself names the function, the separate registry
tool is worth much less. **H6 was falsified** — the control arm moved — and the
investigation (section 7 of `RESULTS.md`) shows why it matters: the blind arm's
single-replicate estimate carries about ±0.075 of noise, so **replicates are now a
prerequisite for any L0 claim**, not a refinement.

Two contract constraints were discovered by running the tests, not by reading the
plan, and are now folded in: the error taxonomy is **frozen at seven members**
(new failure classes take a new *code*), and short names need a stricter
near-miss threshold (`fft` is two edits from both `det` and `dot`).

Still open: WS-2 (arity signatures), WS-3 (`--functions` registry query — the
residual `transpose-purpose` stall is its evidence), WS-4 (partial variables, still
0% coverage), WS-6 (harness defaults) and WS-1.3 (opt-in aliases).

---

## 0. What the baseline actually says

Measured on this machine, 80 real DSH sessions, `deepseek-flash`, budget 10
(`runs/runs-matrix.jsonl`):

| Mode | solved | Success@1 | H_10 | mean evals | failures carrying a message / category / source span | partial vars |
|---|---:|---:|---:|---:|---|---:|
| L0 | 80% | 0.20 | 0.675 | 6.60 | 0% / 0% / 0% | 0% |
| L1 | 85% | 0.30 | 0.715 | 5.55 | 100% / 0% / 0% (strings only) | 0% |
| L2 | 95% | 0.25 | 0.805 | 5.15 | 97% / 97% / 97% | **0%** |
| L3 | 100% | 0.80 | 0.980 | 4.55 | 100% / 100% / 100% | **0%** |

Run `checks/measure_surface.py` to reproduce that table.

Three findings drive every workstream below.

**F1 — The bottleneck is name resolution, not reasoning.** 15 of the 126 failure
observations are `Unknown function '...'`, and the names the agent tried are
ordinary English for the operation it wanted:

| agent wrote | runtime has | occurrences |
|---|---|---:|
| `range` | the range *operator* `1..n` | 4 |
| `determinant` | `det` | 2 |
| `inverse` | `inv` | 2 |
| `derive` | `diff` | 2 |
| `push`, `zip`, `rows`, `help`, `factorize` | — / `concat` / `shape` / — / `factor` | 5 |

Every task that stayed unsolved in L0/L1/L2 is one of these: `undefined-name`
(determinant), `matrix-inverse` (inverse), `transpose-purpose` (rows), and
`symbolic-diff` (derive) needed 6-10 evaluations. In L3 the same names are found
by registry query in **one** evaluation. The runtime knows its own function list;
it simply does not say it.

**F2 — The taxonomy is wrong exactly where an agent needs it.** Name resolution
crosses as `InvalidOperation` / `DomainError` — the same pair a genuine domain
failure uses (`Lovelace.Run/Runner.cs:413-415`, because
`Interpreter.cs:814` throws a bare `InvalidOperationException`). A consumer
cannot branch on "you used a name I do not have" vs "your mathematics is
undefined". 23 of 41 categorized failures are `DomainError`, 15 of them are
really name resolution.

**F3 — Partial results never cross.** `partialVariables` is present in **0%** of
failures, because the runner only fills it for cancellation
(`Lovelace.Run/Runner.cs:317`). An agent repairing a multi-statement program
cannot see how far it got before the failure.

Source spans are *not* a gap: when the mode carries them at all (L2/L3) they are
present ~100% of the time. Do not spend effort there.

---

## 1. Design rules (so the gate means something)

* **R1 — Observable, not cosmetic.** Every change must land in the JSON envelope
  as a structured field, not only in prose. Prose helps L1; structure helps L2/L3.
* **R2 — Additive protocol.** New *optional* fields only; no renames, no removals.
  `protocolVersion` stays 1. Document each field in
  `docs/symbolics/dsh-protocol.md`.
* **R3 — Structural classification.** No consumer may have to match message text.
  The existing classifier already promises this (`Runner.cs:371`); keep it true.
* **R4 — Determinism.** Suggestion order, registry order and filters must be
  total and stable, so oracle fingerprints and repeat runs stay comparable.
* **R5 — Revalidate per revision.** After any runtime change: rebuild,
  `bench.py prepare`, `bench.py validate`. The ablation is only valid inside one
  runtime revision, and every trajectory must record which binary produced it.
* **R6 — Modes are frozen.** A surface improvement is *assigned* to a mode; the
  L0/L1/L2/L3 projections in `bench.py` and `bench-mcp/server.mjs` do not change
  to accommodate it. If a change makes L3 redundant, that is a **result**, not a
  reason to redefine L3.

---

## 2. Workstreams

### WS-1 — Name resolution that answers the question (P0)

*Evidence:* F1. 15 failures, 4 unsolvable tasks, and the single largest L2→L3 gap.

**WS-1.1 Near-miss suggestions in the failure envelope.**
* `Lovelace.Suite/Interpreter.cs:813` — replace the bare throw with a typed
  `UnknownFunctionException(name, suggestions, registrySize)`.
* Suggestions from the live registry (`_functions`), ranked deterministically:
  case-insensitive equality → one name is a prefix of the other
  (`inverse`/`inv`, `determinant`/`det`) → token/prefix overlap → Levenshtein ≤ 2.
  Cap at 3; when nothing is close, say so and name the registry size.
* Message: `Unknown function 'determinant'. Did you mean 'det'?`
  (unchanged for names with no near miss, so existing message contracts hold).
* Envelope: add optional `suggestions: string[]` and `registrySize: int` to
  `RunErrorDto` (`Lovelace.Run/RunProtocol.cs:100`, `Runner.cs:425`).
* Acceptance: L1 carries the suggestion; the L1 arm solves `undefined-name` and
  `matrix-inverse` in ≤ 3 evaluations.

**WS-1.2 Language-feature hints.** `range` is not a function, it is syntax.
Ship a small, curated hint table in the runtime
(`range` → "use the range operator: `1..n`"; `help` → "query the registry"),
surfaced through the same `suggestions` field with `kind: "syntax"`.

**WS-1.3 Alias resolution (opt-in, separate measurement).** Register declared
aliases (`derivative`/`derive`/`differentiate` → `diff`) so the call *works*.
This is a language-semantics decision, not a harness one: it silently changes what
a program means. Ship it behind `--aliases` (runner) / a profile setting, and
measure it as its own arm — never folded into the default surface.

### WS-2 — Arity and shape errors that name the signature (P1)

*Evidence:* `dot(): expected 2 arguments; got 1.` — true, but the agent still has
to guess the call shape; `arity-dot` costs 2 evaluations in every mode.

* `Lovelace.Suite/Functions.cs:123` `BuiltinArityException` already receives the
  `FunctionDefinition`; `Parameters` and `DeclaredArity()` are right there.
* Message: `dot(a, b): expected 2 arguments; got 1.`
* Envelope: `parameterNames`, `expectedArity`, `actualArity` on the error object.
* Same treatment for `BuiltinShapeException` (name the parameter that was wrong).

### WS-3 — A registry you can query without evaluating (P0/P1)

*Evidence:* L3's 185 queries over 20 runs are the demand signal — `det`, `inv`,
`trace`, `deriv`, `rational`, plus ~110 misses (`map`, `zip`, `ravel`,
`average`, `identity`, `diag`, `nth`, `each`, `select`, `help`). Today the only
way to reach the registry is `--omit-functions`'s opposite: a full evaluation that
returns ~16 KB.

* New runner surface: `--functions [pattern]` — no script, no evaluation.
  Default output is names + arity only (~1-2 KB); `--function <name>` returns one
  entry with parameters, arity, plugin and documentation.
* Deterministic ordering, hard cap with an explicit `truncated` flag.
* Acceptance: `checks` measures observation bytes for the L3 tool dropping ≥ 5x
  with no loss of the answers the L3 arm used.

### WS-4 — Partial results on failure (P1)

*Evidence:* F3, 0% coverage.
* `Lovelace.Run/Runner.cs:317` — populate `partialVariables` for **every**
  recoverable failure from the interpreter's committed globals, not just
  `Cancelled`.
* Acceptance: L2 failure observations carry `partialVariables` whenever the
  failing script had committed at least one binding; visible in
  `checks/measure_surface.py` as non-zero.

### WS-5 — Taxonomy precision (P1)

*Evidence:* F2.

**Amended during implementation.** The first slice added `NameResolution` to
`ErrorCategory` and the Symbolics contract test
(`DiagnosticContractTests.ErrorCategory_HasExactlyTheSevenFrozenMembers`) rejected
it: the seven-member taxonomy is a frozen contract, and consumers may switch on it
exhaustively. The repo's own pattern is that a new failure class takes a new
**code** and the coarse category it belongs to (`FileReadError`/`TypeMismatch`,
`AllocationRefused`/`BudgetExceeded`, `Cancelled`/`BudgetExceeded`). An
unresolved name therefore ships as `UnknownFunction` /
`UnsupportedOperation` / recoverable — `UnsupportedOperation` being the one
member that means "this runtime does not offer that operation".

* Classify `UnknownFunctionException` as `UnknownFunction`/`UnsupportedOperation`
  (not the `InvalidOperation`/`DomainError` it shared with a genuine domain refusal).
* Audit the remaining `InvalidOperation`/`DomainError` arms that are not domain
  problems (name resolution, arity, unsupported capability) and give each its own
  code. Target: a consumer can pick the repair strategy from `code` alone.
* Acceptance: in the corpus, no name-resolution failure is categorized
  `DomainError`; the category histogram separates the classes.

### WS-6 — Stop collapsing failures by default (P2, harness side)

*Evidence:* L0's 80% ceiling and 0.47 repeated-diagnostic rate — the projection
discards everything, so the agent probes blind.
* The benchmark's L0 must stay as it is (it is the control). The **default agent
  surface** must not be L0: a default Lovelace tool should return at least L2
  (message + code + category + position + partial variables), with the richer
  registry reachable on demand.
* Acceptance: the DSH Lovelace tool's default failure payload is L2-shaped; L0
  remains reachable only as an explicit benchmark mode.

---

## 3. Sequencing

| Step | Content | Why this order |
|---|---|---|
| 1 | WS-1.1 + WS-1.2 + WS-5 | biggest measured effect, smallest diff, no semantics change |
| 2 | **Gate run A** | proves the effect before investing further |
| 3 | WS-3 + WS-2 | cheap, orthogonal, measurable in bytes and evaluations |
| 4 | **Gate run B** | tests whether L3 is still worth its extra tool |
| 5 | WS-4, WS-1.3 (opt-in), WS-6 | partial results, aliases, harness defaults |

---

## 4. The end gate

**Protocol (every gate run).**

1. `dotnet build` + `dotnet test` for `Lovelace.Suite`, `Lovelace.Run`,
   `Lovelace.Abstractions` — green, or the run does not start.
2. `dotnet publish … -p:PublishAot=true -r win-x64` → new `runner/Lovelace.Run.exe`;
   record `sha256` and the git revision in every trajectory
   (`harness.runner_sha256`).
3. `private/build_corpus.py --check`, `bench.py prepare`, `bench.py validate` —
   the corpus must reproduce 20/20 goldens, 48/48 probes, 0 accidental matches.
4. Re-run the identical matrix: 20 tasks × 4 modes × 1 replicate, same model,
   same budget 10, arms in parallel.
5. Score with `bench.py score`; compare against the frozen baseline in section 5.
6. `checks/measure_surface.py` before/after — the surface must move in the
   predicted direction.

**Hypotheses to confirm or falsify (gate run A).**

| # | Hypothesis | Threshold |
|---|---|---|
| H1 | Name-resolution failures stop being dead ends | L1 solves `undefined-name` **and** `matrix-inverse`, each in ≤ 3 evaluations |
| H2 | L1 rises | H_10(L1) ≥ 0.80 (baseline 0.715) |
| H3 | L2 rises | H_10(L2) ≥ 0.88 (baseline 0.805) |
| H4 | The L3 advantage shrinks — discovery moved into the failure itself | H_10(L3) − H_10(L2) ≤ 0.10 (baseline 0.175) |
| H5 | Fewer confident wrong answers | L1 false convergences ≤ 1 (baseline 2) |
| H6 | The control is untouched | H_10(L0) within ±0.03 of 0.675; L0's projection is unchanged by construction |
| H7 | No regression | corpus validation green; 0 accidental seed matches; `measure_surface` shows no lost span coverage |

**Falsifiers.** H4 failing (the L3 gap does not shrink) would mean the registry's
value is not name lookup but *exploration* — the agent asks about functions it has
not thought of yet — which would redirect WS-3 from "cheaper lookup" to
"better-discoverable capability index". H2/H3 failing with H1 passing would mean
the corpus is too small for the aggregate to move and the plan should report
per-task evidence instead of H_10.

**Cost.** One gate run ≈ 80 model sessions ≈ 40 minutes wall clock, ~660k tokens;
plus a ~3-minute AOT publish. Cheap enough to run after every step in section 3.

---

## 5. Frozen baseline

`runs/runs-matrix.jsonl` (80 trajectories) and `runs/score-matrix.json` are the
pre-change baseline; they are copied to `runs/baseline/` before a gate run so the
comparison cannot be overwritten. Baseline H_10: **L0 0.675, L1 0.715, L2 0.805,
L3 0.980**; 72/80 runs solved; 4 false convergences.
