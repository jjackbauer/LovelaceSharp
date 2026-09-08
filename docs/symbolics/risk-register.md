# Lovelace.Symbolics - Risk Register

> **Status:** Technical and architectural risks with mitigations and explicit scope cuts.
> Companions: [architecture.md](architecture.md), [implementation-plan.md](implementation-plan.md),
> [testing-and-validation.md](testing-and-validation.md), [dsh-execution-plan.md](dsh-execution-plan.md).

---

## 1. Risk table

Likelihood/Impact: Low / Medium / High. Each risk names its monitoring signal (the thing that
tells us the risk is materializing) and its mitigation owner.

| # | Risk | L | I | Mitigation | Monitoring signal |
|---|---|---|---|---|---|
| R1 | **Expression explosion** (expand, simplify, polynomial conversion, rewriting blow up node counts) | High | High | Budgets are typed results (INV-12): MaxNodes/MaxDepth/MaxExpansionTerms/MaxSteps on every unbounded op; expansion only on demand (expand is explicit); sparse polynomial storage; canonical forms bounded by term count checks in constructors; EffortLevel presets cap aggressive paths | Node-count deltas in property tests; budget-exhaustion counters in benchmark runs; OOM-free stress fixtures in CI |
| R2 | **Rewrite unsoundness** (a rule fires where it is not valid: branch cuts, domains, 0^0-style corners) | Medium | High | Rules are conditional by construction (preconditions over the tri-state engine); Unknown never licenses a rewrite; integrator self-verifies via differentiation; solver results carry conditions; falsification suite attacks every rule outside its declared domain (testing doc section 5) | Falsification report status per rule; oracle disagreement rate; regression corpus growth rate |
| R3 | **Branch cuts / domain bugs** (log/sqrt/pow/as-inverse families under complex vs real semantics) | High | Medium-High | Per-function documented branch conventions in the registry; canonicalization is assumption-free so domain-dependent transforms live only in auditable rules; complex-aware evaluation (SYM-13b) used by the falsification engine for cut-adjacent samples | Boundary-biased sample disagreements; complex-sample failures in the MC suite |
| R4 | **Hash-consing memory retention** (long-lived contexts accumulate every distinct subexpression) | Medium | Medium | Per-context pools (sessions are already short-lived in Studio/Run); context disposal; budgeted pool sweeps as a later hardening step; documented guidance that long-running hosts recreate contexts | Memory benchmarks in optbench; pool-size diagnostics |
| R5 | **Native AOT incompatibility** (reflection, dynamic codegen, or trimming gaps sneak in) | Medium | High | Standing repo rule: IsAotCompatible + source-generated JSON contexts + no reflection/Assembly.Load/MakeGenericType-over-open-generics; AOT publish smoke on every merge (copilot-instructions rule 6); compile-time-linked plugins only | `dotnet publish -p:PublishAot=true` gate in the merge checklist; grep gates for reflection APIs |
| R6 | **Polynomial coefficient blow-up** (GCD/Groebner/factor intermediates explode) | Medium | High | Sparse representation; subresultant PRS for GCD (not naive Euclid); monomial-order choice per algorithm; factorization degree caps per EffortLevel; Groebner pair-selection strategies from day one | Intermediate-size counters in poly benchmarks; cyclic-n termination budgets |
| R7 | **E-graph saturation explosion** (Phase 6b) | High | High | Hard caps (MaxEgraphClasses/Iterations/NodeCount) mandatory from the first line; rule set deliberately small; extraction bounded; e-graph isolated behind IOptimizer so the deterministic optimizer ships regardless | Saturation benchmarks on k3-k6; cap-trigger rates |
| R8 | **Solver incompleteness / wrong conditions** (dropped denominator constraints, missed branches) | Medium | Medium-High | Structure dispatch with explicit Unevaluated fallback; conditions attached to SolutionSet and to each Solution; extraneous roots filtered by back-substitution; oracle corpus for the solver families | Solve-failure taxonomy counts; condition round-trip tests |
| R9 | **Integration rule loops / wrong antiderivatives** | Medium | High | Tiered engine with per-tier budgets; mandatory diff-verification of every result; loops caught by step budgets and surfaced with rule IDs; unevaluated Integral is the safe default | Tier-hit distribution; self-check failure counts (should be zero in CI) |
| R10 | **Plugin non-determinism** (registration order, ambient state, culture) | Low | High | Freeze-before-use registries; explicit registration order; canonical term order (never hash order); culture-invariant printing/parsing; determinism replay tests (INV-15) in every package | Bit-identical replay failures in CI |
| R11 | **Serialization instability** (canonical text or MathIR formats drift under refactors) | Medium | Medium | Version headers rejected loudly (INV-14); round-trip property tests; MathIR FormatVersion monotonic; oracle corpus recorded as canonical text (regenerable, diffable) | Round-trip test failures; diff noise in recorded corpora |
| R12 | **Parallel-agent architectural drift** (teams interpret frozen contracts differently; duplicate implementations) | High | High | One architecture owner for the constitution; contract-first development (declare + review public surface before implementation); file ownership map in dsh-execution-plan.md; grep gates like the repo's DSP rewire precedent; single merge queue with the Phase gates | Contract-change count after freeze; duplicate-symbol lint; review backlog |
| R13 | **Real equality hazard leaks into the kernel** (string-based Real equality used for identity/keys) | Medium | High | RealLiteral canonical identity (digits+exponent); Real is banned from hash keys/canonical comparison (INV-08); code-review rule + targeted tests | Grep gate for Real.Equals/GetHashCode usage inside Lovelace.Symbolics |
| R14 | **Backward-compatibility regression in Suite** (ValueKind/operator changes break existing scripts) | Medium | High | Symbolic is an appended non-widening domain kind (Complex precedent); numeric behavior untouched; full existing Suite.Tests + doctests must stay green per merge; additive-only contracts | Existing-suite greenness per merge; doctest drift |
| R15 | **Lean/CI toolchain risk** (Lake not available in CI; manifest drift) | Low | Low-Medium | Proofs stay out of the default fast CI; a separate job installs Lean (or the staleness check runs hash-only, Lean-free); proofs gated manually if the runner is hostile | CI job status; manifest staleness flag |
| R16 | **Scope creep / breadth-over-depth** (the plan drifts toward CAS-breadth) | Medium | High | Explicit non-goals list (architecture section 24); package boundaries fixed; new features require their own SYM package + phase decision | Package count; unplanned surface additions in reviews |

---

## 2. Scope cuts (L)

Cutting order if the plan proves too large. The foundational architecture is never on the
table - cutting it would invalidate the coherence the whole plan exists to guarantee.

### Must have (the coherent kernel - not cuttable)

- Phase 0 in full: Expr DAG + interning + Rational + canonicalization + assumptions +
  rewrite engine + text form + evaluation + Suite domain kind.
- expand/collect/simplify/diff; sparse polynomial core + GCD + factorization + rational
  functions; Bareiss symbolic matrices; linear + polynomial (<=3) solving + RootOf;
- tiered integration with self-verification; series; limits (bounded scope);
- deterministic optimizer (CSE/Horner/power chains) + MathIR interpreter + batch evaluator;
- falsification engine + property suites + regression policy; agent CLI + DSH tool.

### High-value (cut only under serious schedule pressure)

- Groebner bases (SYM-24/32) - keep if any multivariate solving is needed; drop keeps
  univariate completeness;
- e-graph (SYM-38) - the deterministic optimizer covers v1 value;
- quartic formulas (behind Aggressive) - RootOf covers correctness;
- Studio trace pane (SYM-47) - agent APIs carry the same information;
- Lean proofs beyond P1/P2 (SYM-48 partial).

### Optional (nice-to-have)

- Multivariate GCD/factorization breadth; F4-style Groebner; algebraic-number arithmetic
  beyond RootOf; matrix function calculus; Piecewise->MGIR export; partial-fraction
  breadth beyond the integrator's needs.

### Later (explicit non-goals, architecture section 24)

- Full Risch; special-function breadth; ODEs; theorem proving; tensor calculus;
  noncommutative algebra; every backend (C/LLVM/WASM/GPU/distributed); runtime plugin
  loading; reflection-based anything.

---

## 3. Standing mitigations that apply to every package

1. **Budgets before features** - any package that grows structures unboundedly ships its
   Budget parameters in the same PR as the algorithm (INV-12).
2. **Self-verification where possible** - integrator diff-checks, solver back-substitutes,
   lowering round-trip tests; the kernel verifies itself structurally, not just in tests.
3. **Falsification as a merge gate** - after SYM-44 lands, no new rewrite rule merges
   without a falsification record (testing doc section 5.4).
4. **Regression memory** - every counterexample becomes a permanent test (testing doc
   section 7); bug-fix PRs must contain the regression test.
5. **Grep gates** - mirror the DSP-rewire precedent: zero Symbolics references in
   unexpected places (Suite must not reference CAS algorithm namespaces), zero reflection
   API usage, zero Real.Equals inside Symbolics internals.