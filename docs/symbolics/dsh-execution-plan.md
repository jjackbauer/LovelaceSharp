# Lovelace.Symbolics - DSH + DeepSeek V4 Execution Plan

> **Status:** How follow-up agentic SWE sessions implement the packages in
> [implementation-plan.md](implementation-plan.md) against the contracts in
> [architecture.md](architecture.md) - with maximum throughput and no architectural drift.

---

## 1. Operating principles

1. **Contracts before code.** Every package starts by writing its public contract (C#
   signatures + referenced invariants + required test list) into a requirements doc under
   .github/requirements/ (repo convention: requirements-first). The architecture owner
   reviews the contract, then implementation starts.
2. **One owner per choke point.** The Phase-0 constitution (SYM-01..06) and every
   INV-listed invariant have exactly one architectural owner. No other agent modifies
   those files without the owner sign-off.
3. **Agents are package-scoped.** One agent works on one SYM package at a time, in one
   worktree, against frozen contracts. Agents never redesign contracts their package
   consumes - if a dependency contract is insufficient, the agent files a contract change
   request; it does not patch the dependency silently.
4. **Fail loudly, merge green.** The merge checklist (section 6) is non-negotiable; a
   package that breaks an existing suite, the doctests, or the AOT publish never merges.
5. **The codebase is the source of truth.** When this plan and the repo disagree, the repo
   wins; document the correction in the package requirements doc (as the DSP rewire plan
   did for the Modus contract).

---

## 2. Architectural ownership and roles

| Role | Scope | Rules |
|---|---|---|
| Kernel owner | SYM-01..09 + INV-01..15 + Lovelace.Symbolics core files | single agent (sequential packages); reviews every contract that touches core; sole editor of Expr.cs, Constructors.cs, TermOrder.cs, ExprContext.cs, Assumptions/, Rewriting/ |
| Suite integrator | SYM-10 (+ SYM-35) | single agent; sole editor of Lovelace.Suite/Value.cs, NumericOps.cs, Interpreter.cs, ModusHost.cs, ValueFormatter.cs, and the Language.md symbolic sections |
| Track owners | algebra (SYM-11..17), polynomials (SYM-16..24), matrices (SYM-34), calculus (SYM-25..28), solving (SYM-29..33), optimization (SYM-36..39), MathIR (SYM-40..42), validation/agents (SYM-44..46) | one owner per track coordinates its package order; track owners may delegate individual packages to parallel sub-agents once their intra-track contracts are frozen |
| Numerics owner | SYM-01/02, SYM-13b | owns Lovelace.Rational, the Gcd/Lcm extension, and the Lovelace.Complex additions |
| Release gatekeeper | merge decisions, Phase gates, CI | runs the phase exit criteria; does not implement |

The kernel owner is the single point of coherence. Track owners keep sync discipline via
the repo journal system (.github/journals/decisions.md, risks.md) - append-only, so
disagreements leave evidence.

---

## 3. Contract freeze and re-review triggers

**Frozen at the Phase 0 exit gate:** the INV-01..15 invariants, the Expr node taxonomy,
the canonical constructor surface, the AssumptionSet/Tristate semantics, the RuleId/registry
API, the canonical text format v1, ValueKind.Symbolic semantics, and the Modus symbolic
payload mapping.

**Architecture re-review is required (not optional) when any of:**

- an INV-xx invariant changes (kernel owner only);
- a new NodeKind or public contract is added to a Phase-0 file;
- the dependency direction between projects changes (e.g. Symbolics referencing Suite, or
  Suite referencing MathIR);
- the canonical text format or MathIR FormatVersion changes (new version, never silent
  mutation);
- ValueKind ordering or widening semantics change;
- a package needs a contract change in a package it does not own.

---

## 4. Worktree and file-reservation map

The repo already uses .worktrees/ (see .worktrees/binary). Convention for symbolics:

```text
.worktrees/sym-<id>-<slug>/        one worktree per active package (e.g. sym-05-canonical)
```

| Files | Reserved for | Notes |
|---|---|---|
| Lovelace.Symbolics/Expr.cs, Constructors.cs, TermOrder.cs, ExprContext.cs, Assumptions/*, Rewriting/*, NodePool.cs | Kernel owner | nobody else edits, ever |
| Lovelace.Suite/Value.cs, NumericOps.cs, Interpreter.cs, ModusHost.cs, ValueFormatter.cs | Suite integrator | changed exactly once (SYM-10), then re-frozen; SYM-35 needs Suite-integrator coordination |
| Lovelace.Suite/docs/Language.md | Suite integrator + package owners | each symbolic builtin ships its doctest in the same PR |
| Lovelace.Natural/Integer/*.cs | Numerics owner | Gcd/Lcm extension only |
| Lovelace.Complex/*.cs | Numerics owner | elementary functions only |
| LovelaceSharp.slnx, Makefile, .github/workflows/ci.yml | Release gatekeeper | additive project/target entries staged per package |
| Lovelace.Proofs/** | Proofs owner (SYM-48) | Lean toolchain untouched by others |
| harness/** | SYM-46 | mirrors existing host.js structure |

Two agents touching the same reserved file means the plan was violated; the gatekeeper
stops the merge and re-routes. Everything else under Lovelace.Symbolics/** is
namespace-per-package (each package owns its directory: Algebra/, Calculus/, Solvers/,
Matrices/, Optimization/, Functions/), which makes parallel edits structurally safe.

---

## 5. Session organization

### 5.1 The standard package session prompt

Every implementing agent receives: the package ID + name, the exact contract text (from the
requirements doc), the files it may touch, the files it may not touch, the required test
list, the benchmark list if any, and the merge checklist. Example skeleton:

```text
Implement package SYM-23 (Factorization over Q).
Contract: .github/requirements/Lovelace.Symbolics.Factor.md
May touch: Lovelace.Symbolics/Algebra/Factor.cs, Lovelace.Symbolics.Tests/FactorTests.cs
Must not touch: Lovelace.Symbolics/Constructors.cs, any Suite file.
Required tests: expand(factor(p)) == p property; fixture corpus; degree caps per EffortLevel.
Merge checklist: fast suites green; AOT publish smoke for Lovelace.Run; falsification
record for every new rule; doctests if the user surface changes.
```

### 5.2 Sequencing guidance for DSH sessions

- Phase 0: strictly sequential sessions for SYM-01..06 (one session may implement two
  adjacent packages; never interleave). SYM-07/08/09/13/13b then run as parallel sessions.
- Post-gate: one session per package; parallel tracks run simultaneously (up to ~6 tracks);
  within a track, downstream packages wait for their dependencies but may pre-write
  contract docs and tests against frozen shapes.
- Long-running work (falsification sweeps, benchmark campaigns, Lean builds) goes to
  DSH background jobs with the repo existing mgir-style tooling; sessions never
  busy-wait on them - they poll job results at package end.
- After a session resume/fork, the agent must re-read the package requirements doc and the
  current contract files before writing code - contracts may have evolved.

### 5.3 Using the repo own workflow conventions

- Follow .github/copilot-instructions.md (naming, xUnit style, end-to-end verification,
  AOT re-confirmation).
- Use the journal system for decisions/risks; use .github/requirements/ for contracts
  (the repo treats requirements as the pre-implementation gate).
- Follow the dsp-plugin-rewire-plan.md discipline for any cross-project change: state the
  problem, the contract change, the steps, and the grep-gate verification.

## 6. Merge checklist and merge ordering

**Per-PR checklist (every package):**

1. Package contract doc merged first (or same PR, with the contract clearly marked).
2. All required tests pass, including the new package suite; affected existing suites green
   (Suite.Tests for any Suite-touching package; Numerics suites for SYM-01/02/13b).
3. Doctests: any language-surface change ships its Language.md examples in the same PR.
4. AOT smoke: dotnet publish -p:PublishAot=true succeeds for Lovelace.Run (and Studio,
   when touched).
5. Grep gates (mirroring the DSP precedent): no unexpected references (Suite must not
   reference Lovelace.Symbolics.Solvers/Calculus - only the domain-kind surface);
   no Real.Equals/GetHashCode inside Symbolics; no reflection APIs.
6. Falsification record (post-SYM-44): every new or changed rewrite rule carries seed +
   sample count + status; every counterexample adds a regression test.
7. Benchmarks (for benchmark-tagged packages): new benchmark rows compiled and a smoke run
   recorded (numbers do not gate merges except in Phase 7, where the product benchmark
   break-even must be demonstrated).

**Merge order:** Phase 0 sequentially (SYM-01 to 06), then tracks merge in dependency order
per the DAG (implementation-plan.md section 11). A track may not merge ahead of a
contract it consumes; the gatekeeper enforces this against the DAG, not against dates.

---

## 7. Validation gates per phase

| Phase | Gate (all must pass on main) |
|---|---|
| 0 | Constitution suite green; Suite.Tests/Array.Tests/numerics suites unchanged-green; AOT smoke; determinism replays |
| 1 | simplify/expand/collect/diff recorded-oracle corpus green; determinism replays |
| 2 | expand(factor(p)) and gcd*lcm properties green; cyclic-4 within budget; oracle corpus green |
| 3 | series/limit/integral corpus green; every integral self-verified |
| 4 | solve fixture corpus green; RootOf numerics verified at 50 digits; conditions round-trip |
| 5 | matrix acceptance steps in all three hosts; existing array tests untouched |
| 6 | k1-k8 equivalence by evaluation; caps honored; k3 collapses to 1 |
| 7 | lowering round-trip property green; product benchmark published with break-even counts |
| 8 | falsification records complete; DSH tool acceptance scenario green; trace pane end-to-end; Lean manifest + CI job green |

---

## 8. Differential oracles and falsification in practice

- **Oracles (SymPy/AngouriMath) are dev tools:** live oracle runs happen on the developer
  machine or in a manual CI dispatch; default CI replays the recorded corpus only. An
  oracle disagreement produces a regression case against both systems and a hand-derived
  resolution before the package proceeds.
- **Falsification runs are background jobs:** symfalsify sweeps are seeded, long, and
  resumable - DSH background jobs with the report as the durable artifact; the regression
  exporter turns every finding into a test in the same session.
- **Rule evidence ladder:** rules are promoted only along the ladder in testing doc
  section 1 (Heuristic, StressTested, OracleChecked, PropertyTested, TrustedKernel,
  Proven); the registry metadata records the current level, and verify(rule) exposes it
  to agents.

---

## 9. Benchmarks: when they matter

- Benchmark-tagged packages (SYM-05, 13, 17, 20, 23, 24, 34, 36-43) add symbench/optbench
  rows in their package; a smoke run (--job short) is recorded in the PR.
- Full benchmark campaigns run as background jobs before each phase exit.
- Only the Phase 7 product benchmark (original vs optimized, break-even counts) is a hard
  gate; all earlier numbers are advisory regression tripwires (a 10x slowdown in a
  benchmark-tagged package triggers investigation, not automatic rejection).

---

## 10. Anti-duplication rules

- **Single implementations:** one polynomial GCD, one determinant, one series type, one
  canonical printer. The package table is the registry of who owns what; a second
  implementation of the same algorithm is a merge violation.
- **Grep gates catch drift:** Suite must hold zero references to CAS algorithm namespaces
  (Solvers/Calculus/Polynomials) beyond the domain-kind surface; Symbolics must hold zero
  references to Suite; MathIR is referenced only by the MathIR plugin and hosts, never by
  Symbolics.
- **Rules are data, not code:** new mathematical identities are RewriteRule instances with
  RuleIds - never new special cases inside algorithms. A PR that hard-codes a new identity
  into Simplify is returned with the rule template.

---

## 11. When to stop and re-review the architecture

Stop-the-line conditions: (1) an invariant is violated in practice and cannot be fixed
within the package; (2) two tracks need the same core change; (3) a budget keeps tripping
on realistic inputs of the acceptance scenario; (4) the falsification engine finds a
systematic (non-rule-local) unsoundness pattern; (5) AOT constraints turn out to conflict
with a shipped design. In any of these, freeze merges, re-open the architecture document,
and re-run the Phase-0 gate rather than patching around the problem.