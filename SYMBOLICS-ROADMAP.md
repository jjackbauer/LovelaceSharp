# Lovelace.Symbolics — Roadmap and Product Statement

> **Status:** SHIPPED AND HARDENED. The kernel described here is implemented in
> `Lovelace.Symbolics` + `Lovelace.MathIR` (commits `f36dd06`, `13c61a0`, `1af7272`,
> `50ac339`) and shipped through Suite/Studio. The five detailed planning documents in
> [docs/symbolics/](docs/symbolics/) carry post-cycle status annotations; the binding
> change log is [docs/symbolics/hardening-alignment-plan.md](docs/symbolics/hardening-alignment-plan.md).
> The DX + semantic-surface convergence cycle (`358ce89`,
> [docs/symbolics/dx-convergence-alignment-plan.md](docs/symbolics/dx-convergence-alignment-plan.md))
> then shipped structured `*_full` results with language property access, the safe simplify
> contract, explicit solver domains, the record/help/metadata surface, the pretty printer,
> matrix shape preservation, and the structured DSH envelope.
> This document remains the *why*; the delivered reality is described in
> [Lovelace.Symbolics/README.md](Lovelace.Symbolics/README.md).

---

## 1. Where LovelaceSharp is today

LovelaceSharp is an **exact, arbitrary-precision numerical platform**:

- **Numerics.** `Natural` → `Integer` → `Real` (each built on the one below; `Real` stores exact
  periodic fractions like `0.(3)`) plus `Complex` over a pair of `Real`s, all on .NET 10 with
  `System.Numerics` generic-math interfaces.
- **Language.** `Lovelace.Suite` — a full scripting language (tokenizer → parser → interpreter)
  with functions, control flow, vectors and N-dimensional arrays, and built-in linear algebra
  (`det`, `inv`, `matmul`, `trace`) — the single engine behind the REPL, the Studio web IDE, and
  the DSH `lovelace` tool.
- **Arrays.** `NdArray<T>` + `IField<T>` in `Lovelace.Array`, a generic element-arithmetic seam
  that keeps all array algorithms abstract over the element type.
- **Extensibility.** The Modus plugin model (`IModusPlugin`/`IModusContext`): compile-time-linked
  extensions register builtins and kernels without touching the interpreter's internals.
  `Lovelace.Dsp` and `Lovelace.Statistics` are the first consumers.
- **Verification.** `Lovelace.Knowledge` (MGIR) discovers *behavior planes and boundaries* of the
  arithmetic lattice by seeded Monte Carlo sampling against the real engine, with a confidence
  ladder (`Hypothesized → Observed → Repeated → Bounded → Conformant → Proven`).
  `Lovelace.Proofs` formally proves the digit-wise arithmetic in Lean 4 (core-only).
- **Deployment.** Everything is **Native AOT**-shipped: `IsAotCompatible` libraries,
  source-generated JSON serialization, no reflection, no runtime codegen.

What the platform does **not** have today is a symbolic layer: it can *compute* `sqrt(2)` to any
precision, but it cannot *carry* `sqrt(x)`, differentiate it, solve equations containing it, or
compile an optimized evaluator for it.

---

## 2. The change: from numerical library to symbolic-numeric mathematical compiler

**Lovelace.Symbolics** adds the missing middle of the pipeline. Today Lovelace goes:

```text
mathematical source  →  parse / interpret numerically
```

With Lovelace.Symbolics it goes:

```text
mathematical source
    → Suite AST / source representation
    → symbolic expression DAG (hash-consed, immutable)
    → assumption-aware canonicalization
    → algebra / calculus / solving
    → equivalence-preserving optimization
    → MathIR (typed computational IR)
    → scalar / array / native / future GPU / future distributed execution
```

The product identity this creates:

> **Lovelace is an open symbolic-numeric mathematical compiler.**

"Symbolic-numeric" means the two worlds stay *honest with each other*: exact symbolic operations
never silently become approximations, and approximate evaluation is always a deliberate, visible
lowering step. "Compiler" means the symbolic layer is not a dead-end pretty-printer — it feeds an
optimized, executable, machine-inspectable intermediate representation (MathIR) that runs on the
existing arbitrary-precision numerics under the existing Native AOT story.

## 3. What Lovelace.Symbolics 1.0 delivers

| Capability | Why it matters |
|---|---|
| Immutable, hash-consed expression DAG | Deterministic equality/hashing — the substrate every algorithm (canonicalization, rewriting, derivatives, CSE) is built on |
| Exact `Rational` coefficients + ℤ/ℚ/ℝ/ℂ domains | Polynomial and rational algorithms that never round |
| Assumption engine (`x ∈ Real`, `a > 0`, `n ∈ Integer`) | Sound simplifications — `sqrt(x^2) → abs(x)` only when provable |
| Deterministic canonical algebra (`Add`/`Multiply`/`Power` invariants) | Stable contracts parallel agent teams can rely on |
| Conditional rewrite engine with stable rule IDs | Auditable, provenance-carrying transformations instead of scattered `switch`es |
| Sparse multivariate polynomials + division/GCD/resultant + Gröbner bases | `expand`, `factor`, `cancel`, `together`, elimination, polynomial-system solving |
| Calculus: `diff`/gradient/Jacobian/Hessian, Taylor series, limits, bounded symbolic integration | The core CAS workflow (`diff → simplify → solve → series → integrate`) |
| Solving: linear, polynomial (degree ≤ 4 formulas), rational, `RootOf` fallback | Exact roots with explicit conditions, no silent guessing |
| Relations + `Piecewise` | Conditional mathematics; solver conditions; a bridge to the behavior-plane model |
| Symbolic matrices over the existing array layer + fraction-free algorithms | `det`/`inv`/`solve` of symbolic matrices without Gaussian pivot blow-up |
| Optimization: CSE, Hornerization, power reduction, e-graph equality saturation | Symbolic optimization that *pays off at execution time* |
| MathIR + arbitrary-precision evaluation | Optimized executable mathematics on `Natural`/`Integer`/`Real`/`Complex`, Native AOT-safe |
| Provenance (`RewriteStep` traces) + Monte Carlo falsification via the MGIR machinery | Every rule is attackable, and every counterexample becomes a regression test |
| Structured agent APIs (`parse`/`simplify`/`diff`/`solve`/`optimize`/`lower`/`evaluate`) | A deterministic mathematical substrate for DSH + DeepSeek agentic workflows |

The first release is **deep and coherent, not broad and shallow**: no full Risch integration, no
Mathematica-scale special-function library, no quantum algebra — a small, verifiable, extensible
kernel that makes the whole pipeline real end to end.

## 4. Why this is the right next step for this codebase

LovelaceSharp is unusually well prepared for symbolics:

- **Exact numerics already exist.** The symbolic layer needs exact integer and rational
  coefficients — `Natural`/`Integer` are production quality and Lean-proven; `Real` can represent
  exact rationals; the missing piece is a dedicated normalized `Rational(n, d)` coefficient type.
- **The generic-array seam is exactly right.** `IField<T>` + `NdArray<T>` already parameterize
  `det`/`inv`/`matmul` over element types; symbolic arrays are an element type away (plus
  specialized fraction-free algorithms where generic pivoting would explode).
- **The extension seam is exactly right.** Modus plugins already register language builtins
  without touching the interpreter; the symbolic function registry can follow the same
  compile-time-linked, AOT-safe pattern.
- **The falsification machinery exists.** `Lovelace.Knowledge`'s seeded sampling, boundary
  sweeps, and confidence ladder can be pointed at rewrite rules as a falsification engine —
  turning the repo's own behavioral-discovery system into a correctness weapon.
- **The verification culture exists.** Lean proofs already sit next to C# arithmetic; rule IDs
  and theorem metadata extend that link from digit arithmetic to algebra.
- **The agent surface exists.** The DSH `lovelace`/`mgir` tools are the template for structured
  symbolic tools agents can call deterministically.

## 5. The planning documents

| Document | Contents |
|---|---|
| [docs/symbolics/architecture.md](docs/symbolics/architecture.md) | Repository findings, architectural decisions (with alternatives and rationale), data structures, public API sketches, core invariants |
| [docs/symbolics/implementation-plan.md](docs/symbolics/implementation-plan.md) | Work packages (IDs, dependencies, acceptance criteria), dependency DAG, critical path, parallelization map, phased release plan |
| [docs/symbolics/testing-and-validation.md](docs/symbolics/testing-and-validation.md) | Unit/property/differential-oracle/Monte-Carlo validation strategy, regression policy, benchmark plan, formal-proof candidates |
| [docs/symbolics/risk-register.md](docs/symbolics/risk-register.md) | Technical risks, mitigations, explicit scope cuts |
| [docs/symbolics/dsh-execution-plan.md](docs/symbolics/dsh-execution-plan.md) | How DSH + DeepSeek V4 sessions implement the plan: frozen contracts, worktree boundaries, merge order, validation gates |

## 6. Guiding principles

1. **Correctness before clever simplification** — unevaluated `sqrt(x^2)`, `Integral`, and
   `RootOf` are valid, first-class results; an invalid rewrite is never acceptable.
2. **Assumptions are semantics, not annotations** — the assumption engine is core
   infrastructure, designed in from the first node type.
3. **Canonicalization ≠ optimization** — a small set of deterministic canonical forms keeps
   kernel invariants; equality saturation explores equivalent forms under cost models.
4. **Exact and approximate stay distinguishable** — lowering to approximate numerics is an
   explicit, provenance-carrying step.
5. **The symbolic layer feeds execution** — MathIR is a first-class deliverable of v1, not a
   later dream; Native AOT is a design constraint throughout.
6. **Frozen contracts before fan-out** — equality, hashing, canonicalization, domains, and node
   semantics are written down (in the architecture document) before parallel implementation
   begins, so many agents can build on one coherent kernel.