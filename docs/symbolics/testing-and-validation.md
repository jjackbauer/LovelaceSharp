# Lovelace.Symbolics — Testing and Validation

> **Status:** Planning document — executed; see the [POST-CYCLE] annotations below.
> The correctness program shipped inside `Lovelace.Symbolics.Tests` (`PropertyTests.cs`,
> `FalsificationTests.cs`, `DifferentialOracleTests.cs`, `KernelHardeningTests.cs`,
> `ConcurrencyTests.cs`, `MathIrTypingTests.cs`, `LinsolveCheckTests.cs`, `SystemSolveTests.cs`);
> the binding change log is [hardening-alignment-plan.md](hardening-alignment-plan.md).
> Defines the correctness program, falsification strategy, regression policy, and benchmark
> plan for the symbolic-numeric compiler kernel. Companion documents:
> [architecture.md](architecture.md), [implementation-plan.md](implementation-plan.md),
> [risk-register.md](risk-register.md), [dsh-execution-plan.md](dsh-execution-plan.md).

---

## 1. Philosophy

The kernel's identity is **exactness with honesty about limits**. The testing program therefore
inverts the usual priority: every algebraic transformation is assumed wrong until it has
survived a layered attack surface. The layers, from strongest to weakest:

```text
1. Formal proof (Lean)                 — selected foundational rules
2. Algebraic derivation by the kernel  — canonical-form construction, trusted algorithms
3. Property-based testing              — structural identities over random expression families
4. Differential oracle comparison      — SymPy / AngouriMath as dev-time oracles
5. Monte Carlo / boundary falsification — the MGIR machinery repurposed against rewrite rules
6. Deterministic regression corpus     — every discovered counterexample, permanently
```

Layers 1–3 are *demonstrations*; layers 4–5 are *attacks*; layer 6 is the memory that makes
attacks count. This mirrors the repo's existing culture: `Lovelace.Proofs` proves the digit
arithmetic in Lean while `Lovelace.Knowledge` (MGIR) probes the engine's behavior planes by
seeded Monte Carlo sampling against the real `Lovelace.Run` executable.

A rule is only promoted across the confidence ladder when its evidence supports it. The repo
already owns a confidence model — `Lovelace.Knowledge.Confidence` with levels
`Hypothesized → Observed → Repeated → Bounded → Conformant → Proven` (`Lovelace.Knowledge/Model.cs`).
Symbolics adopts and extends this ladder instead of inventing a parallel one:

| Symbolics evidence level | Map to | Meaning |
|---|---|---|
| `Proven` | `Confidence.Proven` | Lean theorem artifact linked by rule ID |
| `TrustedKernel` | `Confidence.Conformant` | Derived by canonical construction or a trusted algorithm (e.g. polynomial GCD) |
| `PropertyTested` | `Confidence.Repeated` | Structural property suite green over many random families |
| `OracleChecked` | `Confidence.Bounded` | Agrees with an independent CAS on a structured corpus |
| `StressTested` | `Confidence.Observed` | Survived Monte Carlo + boundary falsification |
| `Heuristic` | `Confidence.Hypothesized` | Untested heuristic (never ships silently; logged in provenance) |

---

## 2. Unit-test strategy per subsystem

xUnit only (repo convention: `MethodName_GivenScenario_ExpectedResult`). Each work package
(see [implementation-plan.md](implementation-plan.md)) ships with its unit tests in the same
PR; the table below names the invariant classes each package must cover.

| Subsystem | Unit tests must cover |
|---|---|
| `Rational` | sign/GCD normalization, zero/one/minus-one recognition, exact arithmetic identities, conversion to/from `Integer`/`Real`, parsing of `p/q`, overflow-free construction, AOT serialization |
| Expr DAG / interning | structural equality transitivity/symmetry, hash-consing returns identical reference for equal structure, canonical operand order stability, node ID stability, thread-safety under concurrent interning |
| Assumptions | tri-state predicate algebra (True/False/Unknown, and **never** false-positive), inference propagation through functions, contradiction detection (`x > 0 ∧ x < 0`), scoped assumption push/pop, query caching invalidation |
| Canonicalization | every invariant of §E of architecture.md: flattening, coefficient extraction, ordering, `x+0→x`, `x*0→0`, `x^a*x^b→x^(a+b)`, integer-power folding, division representation, unevaluated preservation |
| Rewrite engine | pattern matching (structural/typed/sequence wildcards), precondition evaluation under assumptions, directionality, rule-ID provenance, loop protection, budget exhaustion behavior, deterministic iteration order |
| Polynomials | add/mul/div/rem identities against naive expansion, degree/total-degree, monomial order correctness, GCD × LCM = |a·b|, content/primitive part, square-free decomposition, coefficient preservation |
| Gröbner | Buchberger criterion, ideal membership on known examples (cyclic-4, intersection cases), S-polynomial reduction convergence, termination budget |
| Calculus | every derivative rule in the function table; chain/product/quotient; higher-order; gradient/Jacobian/Hessian shapes; memoization correctness |
| Series | Taylor coefficients for known functions to order 10; `sin(x)/x` at 0; Laurent-style cases; truncation order invariants |
| Limits | one-sided vs two-sided agreement; infinite limits; indeterminate forms resolved via series; unevaluated fallback semantics |
| Integration | each tier of §10.4 of architecture.md has a table; every claimed closed form re-differentiates to the integrand; unevaluated `Integral` returned when tier fails |
| Solvers | linear exact solutions; quadratic/cubic/quartic formula identities; `RootOf` properties (defining polynomial satisfied); rational-equation denominator restrictions; inconsistent/underdetermined detection |
| Symbolic matrices | `det`/`inv` identity `A·A⁻¹ = I` (as an expression identity, verified by expansion or evaluation), Bareiss vs generic agreement, shape errors |
| MathIR | SSA validity, typed temporaries, constant-pool dedup, lowering round-trip (IR → eval ≡ direct eval at high precision), CSE correctness, serialization versioning |
| Provenance | every `RewriteStep` carries rule ID + assumptions; trace replay determinism; serialization round-trip |

**Doctest convention.** Symbolic builtins added to the language get executable examples in
`Lovelace.Suite/docs/Language.md` (the repo's doctest harness `LanguageDocumentationTests` makes
every example an assertion). This forces the user-facing surface to stay in sync with the kernel.

---

## 3. Property-based testing

[POST-CYCLE] Shipped with seeded deterministic System.Random PRNGs (no external property
framework, per the repo policy): canonical-text round-trip and re-interning, simplify-value
equivalence under declared assumptions, symbolic derivative vs high-precision central
differences, integrate-to-diff round-trip, solve-to-substitute-to-zero, optimize-value
equivalence, MathIR-vs-tree equivalence, concurrency (parallel symbol creation/parse/
simplify/diff/compile), and precision integrity (500-digit round-trips, huge exponents).
The catalog below was the source; not every listed family has a suite yet.

The repo currently has almost no property-style testing (xUnit `Fact`s dominate; a handful of
`Theory`/`MemberData` uses in `Lovelace.Suite.Tests`). Symbolics changes that *within xUnit* — a
small deterministic generator layer over the repo's own `SplitMix64` PRNG
(`Lovelace.Knowledge/Randomness.cs`, seed-reproducible across platforms) avoids a new test
framework dependency while giving the falsification engine and the property suites one shared
source of random expressions.

### 3.1 Expression generator design (package SYM-XX, see implementation plan)

```text
ExprGenerator
  ├── config: depth bound, leaf pool (symbols x,y,z,n; small integers/rationals; Pi/E/I)
  ├── bias knobs: probability of Add/Mul/Pow/Function/Relation nodes
  └── assumption-aware: generated symbol n carries Integer assumptions when used in
      even/odd/factorial contexts; x carries Positive for sqrt(x^2) families
```

Deterministic: seed → identical expression stream across machines (SplitMix64). Each generator
produces (expr, assumption set, evaluation points) triples so property tests can check both
structure and numerics.

### 3.2 Property catalog (mandatory per rule family)

| Property | Checks |
|---|---|
| `simplify(e) ≡ e` | high-precision numeric evaluation of both sides agrees on random points in the valid domain |
| `expand(factor(p)) ≡ p` | for every polynomial p the round-trip is structurally canonical |
| `D(f+g) = D(f) + D(g)` | structural equality after canonicalization |
| `D(f·g) = D(f)·g + f·D(g)` | structural equality after canonicalization |
| `subs(e, x→a)` ≡ `eval(e at a)` | substitution followed by exact evaluation equals direct evaluation |
| `gcd·lcm = |a·b|` | coefficient-wise and for multivariate cases |
| `series(f, x₀, n) → coeffs` | each coefficient equals `D^k f(x₀)/k!` |
| `integrate(diff(f,x),x) ≡ f` (mod constant) | for every integrable table entry |
| `solve(f=0)` roots satisfy `f(root) = 0` | substitution + exact zero test, or RootOf property |
| `A · A⁻¹ ≡ I` | for random invertible symbolic matrices (checked by evaluation, not expansion) |
| `lower(opt(e)) ≡ lower(e)` | optimized IR evaluates to the same numbers as the original |

### 3.3 Where properties are not enough

Structural identity tests catch only *wrong structure*. Numeric identity tests catch only
*wrong values at sampled points*. Neither proves domain correctness — that is what the
assumption engine (tri-state predicates) and the boundary-focused falsification layer are for.
Property tests therefore run **with the assumptions the rule declares**, and the falsification
layer separately attacks the rule *outside* those assumptions.

---

## 4. Differential oracles (SymPy / AngouriMath)

[POST-CYCLE] The SymPy oracle shipped exactly as constrained here: tests-only, invoked as a
child process, and auto-SKIPPED when python3/sympy is absent (DifferentialOracleTests.cs
probes for the interpreter first). AngouriMath was not used. Disagreements are adjudicated
by independent numeric evaluation, never trusted blindly.

Mature CAS systems are used **only as development-time oracles**, never as runtime
dependencies — matching the repo's zero-third-party-runtime-dependency policy.

### 4.1 Oracle adapters

```text
tools/oracles/
├── sympy_oracle.py        # stdin/stdout JSON: {expr, op} → {result, warnings, conditions}
├── angouri_adapter.csproj # optional .NET oracle via AngouriMath (dev-only project)
└── oracle_corpus/         # generated cases + recorded expected outputs (checked in)
```

The adapters speak the canonical text form (see architecture.md §serialization): SymPy's `srepr`
and Lovelace's canonical printer are both lossless, so comparisons happen on AST text, not on
human-oriented pretty-print.

### 4.2 What oracles are used for

| Area | Oracle check |
|---|---|
| Differentiation | random elementary expressions: structural agreement after canonicalization |
| Simplification | agreement on canonical forms for a generated corpus |
| Integration tables | every table entry: `integrate` then `diff` equals integrand (both sides) |
| Solving | degree ≤ 4 symbolic formulas, linear systems, rational equations |
| Series | coefficient agreement to order 10 |
| Limits | known one-sided/infinite limit corpus |
| Factorization | factor results expand back (plus irreducibility spot-checks) |

### 4.3 Constraints and policy

- Oracle-dependent tests are **not in the default CI path** (CI runs only fast suites, per
  `.github/workflows/ci.yml`); they run in a manual/periodic gate. Recorded expected outputs
  (`oracle_corpus/`) allow CI to still verify the *same cases* without the oracle installed.
- License check: SymPy (BSD) and AngouriMath (MIT) are both permissible for dev tooling; the
  kernel imports nothing from them.
- Oracle disagreement is an incident, not a judgment call: file a regression case against
  *both* systems' claims and resolve with a hand-derived proof or a high-precision numeric
  arbitration before proceeding.

---

## 5. Monte Carlo and boundary falsification (the MGIR reuse)

[POST-CYCLE] Implemented inside Lovelace.Symbolics.Tests rather than a separate
Lovelace.Symbolics.Validation project, and WITHOUT linking the Knowledge assembly — only
its pattern (seeded deterministic PRNG, boundary-biased points, bisection-style probing) was
reused. FalsificationTests.cs attacks every shipped rewrite rule over its declared domain
with points biased toward the singular/ordering boundaries (0, ±1, ±1/1000, ±1/3, ±3/2, ±100)
and labels evidence FuzzVerified; sampling is never used to upgrade an identity to a proven
theorem. The section below remains the design reference.

`Lovelace.Knowledge` already implements the exact machinery this layer needs, pointed today at
the arithmetic lattice. It is repurposed to attack **rewrite rules**:

```text
config (Ω, q, thresholds, seed)  →  sample batch z ~ q(z)
    →  execute (rule lhs/rhs evaluated at z)  →  canonicalize σ
    →  reduce (planes/boundaries)  →  merge (idempotent)  →  measure C1–C4
    →  bias toward frontiers (bisection, held-out probes)  →  repeat until converged
```

This is the literal pipeline of `BEHAVIOR-GRAPH.md` §1. For rewrite validation the planes become
*agreement classes* (`agree`, `disagree`, `undefined-lhs`, `undefined-rhs`) and boundaries are
*domain edges* (branch cuts, poles, sign flips) fitted as guards.

### 5.1 Reused, adapted, and new pieces

| Piece | Source | Use |
|---|---|---|
| `SplitMix64` seeded PRNG | `Lovelace.Knowledge/Randomness.cs` | **Reuse as-is** — platform-independent determinism |
| Sampler/Reducer/Convergence loop pattern | `Lovelace.Knowledge/Sampler.cs`, `Reducer.cs`, `ConvergeLoop.cs`, `Convergence.cs` | **Adapt** — same batch/reduce/converge skeleton, new σ classes |
| `Confidence` ladder + `Plane`/`BoundaryEdge`/`Guard`/`Frontier` model | `Lovelace.Knowledge/Model.cs` | **Reuse the model**, new domain predicates |
| Behavior-graph persistence (`GraphStore`, JSON context) | `Lovelace.Knowledge/GraphStore.cs` | **Reuse pattern** for rule-falsification reports |
| Runner-based execution | `Lovelace.Knowledge/ScriptRunner.cs` + `Lovelace.Run` | **Replaced** by direct in-process high-precision evaluation (no script round-trip; the symbolic evaluator is a library call) |
| Boundary sweeping (sweep/refine/validate) | `SamplingKind` model | **Adapt** — sweep one symbolic parameter across a suspected cut |

### 5.2 Domain generation (assumption-aware)

A candidate rule `lhs ↔ rhs` ships with a declared domain config:

```text
domains:      real, complex, positive-real, integer, nonzero
sample kinds: uniform-random (SplitMix64 over exact rationals),
              boundary-biased (near 0, ±1, poles of subexpressions),
              singularity-adjacent (x₀ ± ε for each subexpression singularity x₀),
              branch-biased (angles near branch cuts: negative reals for log/sqrt,
              ±1 for asin/acos, pure-imaginary axis for exp)
evaluation:   precision p, then re-check disagreements at 2p (eliminates float noise
              and precision-luck verdicts)
```

Disagreements that persist at doubled precision become candidate counterexamples and enter the
**minimizer**: delta-debugging over the expression structure (shrink terms/factors toward a
smallest failing form) and over the sampled point (binary search along the offending axis,
reusing the reducer's bisection discipline).

### 5.3 Falsification is *falsification*

This layer **never proves** a rule. It exists to destroy incorrect rules and to discover missing
preconditions — `sqrt(x^2) → x` without `Positive(x)` dies here instantly. A rule that survives
N seeded rounds is promoted to `StressTested`; only a Lean artifact or a canonical-construction
argument promotes further.

### 5.4 Rule registry integration

Every rule in the rewrite registry (architecture.md §7) carries optional falsification metadata:

```csharp
// Falsification metadata attached to a rewrite rule
record FalsifyConfig(
    int Seed, int MaxSamples,
    NumberDomain[] Domains,
    string[] BoundaryAxes,
    long Precision);                       // working precision digits
```

A `symfalsify` CLI (modeled on `Lovelace.Knowledge.Run`'s JSON-over-stdio surface, see
implementation plan SYM-42) runs the suite, emits a JSON report, and every discovered
counterexample is mechanically converted into a regression test (see §7).

---

## 6. High-precision numerical validation

Where structural comparison is impossible (e.g. simplification results, optimized IR vs
original), identity is checked numerically at high precision using the existing `Real` engine
(with `Complex` where relevant).

### 6.1 Sample families (mandatory coverage per rule)

| Family | Points |
|---|---|
| Real generic | exact random rationals p/q with small p, q |
| Complex generic | a + b·i with exact rational a, b |
| Near zero | ±10⁻ᵏ for k ∈ {1, 10, 50} |
| Large magnitude | ±10ᵏ, k ∈ {6, 30, 100} |
| Negative inputs | for sqrt/log/pow rules |
| Singularity-adjacent | x₀ ± 10⁻ᵏ for each pole/singularity x₀ of the rule's subexpressions |
| Branch boundaries | negative real axis, ±1, unit circle for inverse trig/log families |
| Integer-typed symbols | for Even/Odd/Integer preconditions |

### 6.2 Comparison metric

- Absolute and relative agreement at working precision p: `|lhs − rhs| ≤ 10⁻(p−guard)` with
  guard digits (the repo's `Real` arithmetic already carries guard-digit discipline in
  `Sqrt`/`Pi`).
- Exact rational cases: substitute rational points and require the **exact** values to agree
  (no tolerance) wherever both sides evaluate inside ℚ.
- Any disagreement triggers the precision-doubling re-check of §5.2 before being treated as a
  counterexample.

---

## 7. Regression policy

**Every discovered counterexample becomes a permanent regression test.** Mechanics:

```text
tests/regressions/
├── mc/            # Monte-Carlo-discovered counterexamples (rule ID, point, expected)
├── oracle/        # oracle disagreements
└── boundary/      # domain/branch-cut cases
```

Each regression test is named `RuleId_GitIssueOrDate` and contains: the rule ID, the
counterexample expression, the assumptions in force, and the corrected expectation. A regression
test that later passes *because the rule changed* must be re-triaged, not deleted silently.

---

## 8. Formal-proof candidates and linkage

`Lovelace.Proofs` proves positional arithmetic in **core Lean 4** (no Mathlib). Symbolics extends
the same pattern incrementally — proving *the algebraic core* first, not the CAS surface:

| # | Candidate | Lean scope | Feasibility |
|---|---|---|---|
| P1 | Rational normalization: `gcd(n,d)=1` invariants, sign normalization | core Nat/Int + a small Rational development | high |
| P2 | Canonicalization soundness: each Add/Mul/Power constructor rewrite preserves value (ring/field laws over a generic field) | small field/ring library in core Lean | medium-high |
| P3 | Polynomial arithmetic correctness: add/mul/div/rem over list-of-coefficients | core Lean, mirrors the existing digit-list proofs | high |
| P4 | Euclidean GCD correctness for univariate polynomials over a field | core Lean | high |
| P5 | Selected elementary-function derivative rules (chain/product/quotient + table) stated over formal power series | needs a formal-power-series development | medium |
| P6 | Buchberger termination/correctness | needs well-founded order theory | later |

**Linkage mechanism.** A rule carries a stable `RuleId`; a generated manifest
(`Lovelace.Proofs/Symbolics/*.json`, or a checked-in markdown table) maps
`RuleId → (Lean module, theorem name, hash of the .lean source)`. The runtime never loads
Lean — the artifact is build-time documentation that CI checks for staleness (hash mismatch
fails the proof-linkage gate). This mirrors how `Lovelace.Proofs` already sits beside the C#
digit arithmetic without being linked into it.

---

## 9. Benchmarks

The repo benchmarks with **BenchmarkDotNet 0.15.8** (see `dspbench/dspbench.csproj` and its
`ManualConfig` build-timeout pattern — symbolics benchmarks must reuse that config because the
`Real` dependency chain makes BDN's cold rebuild slow). A new `symbench` project (JIT-run,
benchmarks are exempt from AOT) hosts all kernel benchmarks; the executable benchmarks for the
product-level story live in `optbench`.

### 9.1 CAS-operation benchmark families

| Family | Representative expressions | Metric |
|---|---|---|
| Construction/interning | repeated construction of `(x+y)^k` for k=1..20; concurrent interning | ops/sec, alloc |
| Canonicalization | `(x+1)^20` expanded then re-collected; term-sorted sums | time, node count |
| Simplification | trig identities; nested sqrt forms; `simplify` passes | time, rewrite steps |
| Differentiation | `diff` of random depth-8 expressions; repeated `diff` with memoization | time; memo hit rate |
| Expand/factor | `(x^2-1)^n` expansion; factorization of degree 8–16 polys | time |
| Polynomial GCD | random dense degree-20 polynomials | time |
| Gröbner | cyclic-4, cyclic-5, intersection ideals | time, S-poly count |
| Symbolic determinant | symbolic 4×4, 6×6 (Bareiss vs cofactor) | time, expression size |
| CSE | expression DAG with 10⁴ shared subexpressions | nodes before/after |
| E-graph saturation | `(x+y)^2`, trig identities, Horner candidates | saturate time, class count |
| Lowering | symbolic → MathIR for representative kernels | time |
| MathIR execution | interpreted vs lowered loop over 10⁶ points | ops/sec |

### 9.2 Product benchmark: original vs symbolically optimized execution

The benchmark that justifies the whole subsystem. For each kernel below, compare the **naïve
tree-walking evaluation** of the original expression with the **optimized MathIR** execution:

```text
k1:  a*x^5 + b*x^4 + c*x^3 + d*x^2 + e*x + f      (Hornerization)
k2:  x^8                                            (power chain: 3 multiplies)
k3:  sin(x)^2 + cos(x)^2                            (identity collapse)
k4:  exp(-a*x^2) * sin(b*x) / (1 + x^2)             (CSE + fused structure)
k5:  (x+y)^6 / (x^2 + 2*x*y + y^2)                  (algebraic cancellation)
k6:  1/(1+x) + x/(1+x^2) + x^2/(1+x^4) + ...       (Horner-style continued form)
k7:  det of a 3×3 with repeated subexpressions       (CSE)
k8:  f and its gradient together (shared subexpressions between f, ∇f, Hf)
```

Measured per kernel (following the precbench/dspbench reporting style):

| Metric | Definition |
|---|---|
| operation count | node count of the executed IR vs the original DAG |
| wall-clock runtime | BDN mean/median over N evaluation batches |
| allocation | BDN `MemoryDiagnoser` |
| peak memory | working set delta for large batches |
| precision | agreement with direct high-precision `Real` evaluation (guard digits) |
| compilation cost | time to canonicalize + optimize + lower (one-shot) |
| break-even count | number of evaluations at which compilation cost is amortized |

The break-even count is the headline number: it tells users *when* symbolic optimization pays
for itself, converting the feature from a demo into an engineering tool.

### 9.3 Realistic kernels

Beyond microbenchmarks: a fixed corpus of scientific/engineering kernels (radial basis
functions, Rosenbrock and Himmelblau functions with gradients, spherical-harmonic evaluations,
rational approximations of special functions, small linear-system solves with symbolic
coefficients) evaluated at 10⁵–10⁶ parameter points.

---

## 10. CI integration

- **Default CI** (fast suites, mirroring `.github/workflows/ci.yml`): all `Lovelace.Symbolics.*`
  unit suites, doctests (`LanguageDocumentationTests` after symbolic builtins land), MathIR
  tests, and the recorded oracle corpus replay.
- **Manual/periodic gates**: live-oracle differential runs, Monte Carlo falsification runs
  (`symfalsify`), full `Lovelace.Real.Tests`, Lean `lake build` (plus proof-manifest staleness).
- **AOT smoke**: every merge must keep `dotnet publish -p:PublishAot=true` green for
  `Lovelace.Run` (which will load the symbolics builtins) and `Lovelace.Studio` — the repo's
  standing rule from `.github/copilot-instructions.md` §6.

---

## 11. Validation matrix (features × validation layers)

| Feature | Unit | Property | Oracle | MC falsification | Boundary | Formal candidate | Benchmark |
|---|---|---|---|---|---|---|---|
| Expr DAG / interning | ✅ | ✅ | — | — | — | — | ✅ |
| Rational | ✅ | ✅ | ✅ | — | — | P1 | ✅ |
| Assumptions | ✅ | ✅ | — | — | — | — | — |
| Canonicalization | ✅ | ✅ | ✅ | ✅ | — | P2 | ✅ |
| Rewrite engine | ✅ | ✅ | ✅ | ✅ | ✅ | P2 | ✅ |
| Simplify | ✅ | ✅ | ✅ | ✅ | ✅ | — | ✅ |
| Polynomials | ✅ | ✅ | ✅ | — | — | P3, P4 | ✅ |
| Gröbner | ✅ | ✅ | ✅ | — | — | P6 (later) | ✅ |
| Differentiation | ✅ | ✅ | ✅ | ✅ | — | P5 | ✅ |
| Series | ✅ | ✅ | ✅ | — | — | — | ✅ |
| Limits | ✅ | ✅ | ✅ | ✅ | ✅ | — | — |
| Integration | ✅ | ✅ | ✅ | ✅ | — | — | ✅ |
| Solvers | ✅ | ✅ | ✅ | — | — | — | ✅ |
| Symbolic matrices | ✅ | ✅ | — | ✅ | — | P3 | ✅ |
| MathIR / execution | ✅ | ✅ | — | — | — | — | ✅ |
| Provenance | ✅ | — | — | — | — | — | — |
| Suite/agent APIs | ✅ (doctests) | — | ✅ | — | — | — | — |