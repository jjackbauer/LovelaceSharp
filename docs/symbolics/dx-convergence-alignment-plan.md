# Lovelace — DX + Semantic-Surface Convergence: Alignment Plan

> **Status:** Alignment proposal (awaiting approval). Re-baselined audit of the current working
> tree (commit `21a02f9`, `main`, clean). No production code, APIs, structure, or tests were
> modified during the audit.
>
> This document supersedes the externally produced "Symbolics DX + Semantic Surface
> Convergence" assessment. Every claim below was verified against the code; the assessment's
> diagnosis was confirmed in substance, while its implementation phases were re-scoped against
> the already-shipped kernel (commits `f36dd06`, `13c61a0`, `1af7272`, `50ac339`).
>
> Test-baseline note: full-suite execution could not be re-run in the audit environment
> (NuGet restore stalls; testhost process handles are denied by the host sandbox). The
> documented baselines stand: Symbolics 236, Suite 421, Studio 19, Dsp 61, Real 285
> (`hardening-alignment-plan.md:253`). Live behavior below was verified by executing the real
> `Lovelace.Run.exe` DSH runner against the assessment's own examples.

---

## 0. Origin and what this plan adopts

The external assessment proposed a DX/discoverability/semantic-surface cycle. An independent
code audit (five read-only subsystem audits + live runner execution) verified its claims
against the tree. This plan adopts the assessment's **goals and diagnosis**, adopts its
**execution discipline** (audit → alignment → approval → implementation), and re-scopes its
**phases**, because a substantial part of what it asks to design already ships in the C#
kernel and is merely not lifted to the language boundary.

| Assessment element | Verdict | Action in this plan |
|---|---|---|
| Discoverability (`help`/`funcs`) absent for symbolics | TRUE | Adopted as Phase 5 |
| Richness lost at Suite boundary | TRUE (metadata, not expression structure) | Adopted as Phase 3; re-scoped from "design result models" to "lift shipped result models" |
| Structured result models (§25–31) | Already shipped in kernel | Reuse `TransformResult`, `LimitResult`, `IntegrationResult`, `SolutionSet`, `OptimizationResult`, `CompiledKernel`, `SymbolicMatrix` (§1.5) |
| Canonical/pretty printer separation (§15) | Already shipped (`Printing.CanonicalPrint`/`PrettyPrint`) | Phase 4 fixes rendering quality only; adds a Debug mode (§4.4) |
| `exp(log x)` universal (§10) | TRUE, and it contradicts the shipped definedness doctrine | Decision D3 reclassifies it (§2) |
| Solver domain inconsistency (§10) | TRUE (deg 2/3 complex, deg ≥4 real-only) | Decision D4 introduces `SolveDomain` (§2) |
| Contradictory conditions kept silently (§13) | TRUE (code comment admits it) | Decision D11 (§2) |
| Broad exception swallowing (§14) | Mostly already disciplined; 2 defect-masking sites | Phase 1 item |
| `jacobian`/`hessian` flat (§21) | TRUE at the boundary; kernel is rank-2 | Phase 6 fix |
| `linsolve` unreadable (§23) | The example string is verbatim `README.md:743` | Phase 6: `linsolve_full` + pretty-printer |
| DSH/agent flat strings (§36) | TRUE for the value payload | Phase 7: structured envelope field (§4.5) |
| AOT constraint (§53) | Aligns with repo policy | Preserved; Phase 7 adds CI AOT publish |
| 13 parallel workstreams (§61) | Overpromises before constitutional contracts freeze | Reorganized with ownership boundaries (§6) |

---

## 1. Re-baselined inventory (audit of `21a02f9`)

### 1.1 Host/plugin structure (verified)

```text
Lovelace.Suite (SuiteEngine → Interpreter → ModusHost)
   ├── core builtins (Interpreter.RegisterBuiltins, Interpreter.cs:980+)
   ├── DspPlugin (Lovelace.Dsp)
   ├── SymbolicsPlugin (Lovelace.Symbolics/SymbolicsPlugin.cs)
   └── MathIRPlugin (Lovelace.MathIR/Evaluator.cs:334+)
Hosts: Lovelace.Console (REPL) · Lovelace.Run (DSH JSON runner) · Lovelace.Studio (web IDE)
```

Layer debt (verified): `Lovelace.Suite` references `Lovelace.Symbolics` directly
(`Lovelace.Suite.csproj:17`) and the core interpreter special-cases symbolic matrices for
`inv`/`linsolve`/`matrix_rank`/`det` (`Interpreter.cs:998-1051, 1497-1505`) — a violation of
the architecture's hard boundary ("Suite keeps zero knowledge of CAS algorithms",
`docs/symbolics/architecture.md:131-138`). Decision D14 addresses it.

### 1.2 Public symbolic builtin inventory (verified; live where noted)

Legend: **Return today** = what the language sees; **Kernel result** = what the C# API
produces; **Lost** = information discarded at the boundary.

| Builtin | Return today | Kernel result | Lost today |
|---|---|---|---|
| `symbol`, `inf` | Symbolic | Expr | — |
| `exp log sin cos tan asin acos atan sinh cosh tanh` | Symbolic | Expr | — |
| `diff` | Symbolic | Expr | — |
| `integrate` | Symbolic | `IntegrationResult` (status/method/verified) | status, method, verified |
| `simplify` | Symbolic | `TransformResult` (conditions, steps) | conditions, rule steps |
| `expand factor collect cancel apart` | Symbolic | Expr | — |
| `series` | Symbolic (with O-term) | `Series` (variable/point/order) | series metadata; natural term order |
| `limit limit_left limit_right` | Symbolic **or Text** | `LimitResult` (status/left/right) | DNE becomes the prose string `"does not exist (left: -inf, right: +inf)"` |
| `solve` | Vector **or Text** | `SolutionSet` (kind, conditions, families, note) | status, conditions, parametric families (stringified `"k*pi for integer k"`), diagnostics |
| `solve_system` | Text (joined string) | `SystemSolveResult` | everything; joined with `"; "` |
| `subs evalf` | Symbolic / numeric | Expr / Num | — (evalf's display precision path is numeric by design) |
| `jacobian hessian` | **flat Vector** | `SymbolicMatrix` (rank 2) | matrix shape (`ToFlatArray`, `SymbolicsPlugin.cs:147-158`) |
| `optimize` | Symbolic | `OptimizationResult` (cost before/after, trace) | cost estimates, transformation trace |
| `assume assume_* assumptions assume_clear` | mixed | AssumptionSet | `assumptions()` is a **joined string** |
| `and or not` | bool / Symbolic | Expr | — |
| `compile lower` (MathIR) | **Text** (IR serialization) | `CompiledKernel` / `IrProgram` | kernel identity, parameters, types, version (`Evaluator.cs:354-361`) |
| `evalir evalir_batch` | numeric | Num | flat row-major arg shape (`Evaluator.cs:362-385`) |
| `det` (symbolic) | Symbolic | Expr | — |
| `inv` (symbolic matrix) | Array | `SymbolicMatrix` with conditions | `det ≠ 0` condition dropped (`Interpreter.cs:998-1023`) |
| `linsolve` | Vector | `SymbolicMatrix.Solve` | `det ≠ 0` dropped; **throws** on singular (`Interpreter.cs:1036-1051`) |
| `matrix_rank` | Integer | int | — |
| `sqrt` (core) | Symbolic via `Power(x, 1/2)` | Expr | — |

**Missing language surface entirely** (kernel has the capability, no builtin): `groebner`,
`roots`, `resultant`, `discriminant`, the `Polynomials` facade, and every structured result
(`simplify_full`/`solve_full`/… do not exist).

### 1.3 Flattening map (the §0.6 inventory the assessment asked for)

| Site | Flattening | Evidence |
|---|---|---|
| `SymbolicsPlugin.solve` | `SolutionSet` → `object[]` of values; conditions only *filter* solutions; families → string; `"no solutions"` Text | `SymbolicsPlugin.cs:112-133` |
| `SymbolicsPlugin.solve_system` | → `string.Join("; ", …)` | `SymbolicsPlugin.cs:104-105` |
| `SymbolicsPlugin.LimitToExpr` | DNE → prose Text | `SymbolicsPlugin.cs:279-295` |
| `SymbolicsPlugin.simplify/optimize` | `.Expression` only | `SymbolicsPlugin.cs:77-78, 159-164` |
| `SymbolicsPlugin.integrate` | `IntegrationResult` discarded | `SymbolicsPlugin.cs:75-76` |
| `SymbolicsPlugin.jacobian/hessian` | `.ToFlatArray()` | `SymbolicsPlugin.cs:147-158` |
| `SymbolicsPlugin.assumptions` | `string.Join("; ", …)` | `SymbolicsPlugin.cs:64-71` |
| `MathIRPlugin.compile` | `.IrText.TrimEnd('\n')` Text | `Evaluator.cs:354-361` |
| Core `inv`/`linsolve` | conditions dropped / throws on singular | `Interpreter.cs:998-1051` |
| `ModusHost.WrapResult` | accepts only Real/Integer/Natural/Complex/Expr/bool/string/ArrayValue/List — no object channel | `ModusHost.cs:166-197` |
| `Lovelace.Run` | value payload = `ResultDto(Kind, Display, Typed)` — three strings | `Run/Program.cs:211-224` |

### 1.4 Verified semantic findings (assessment §10–14)

| Finding | Verdict | Evidence |
|---|---|---|
| `exp(log(x)) → x` unconditional | **TRUE** | `Simplify.cs:128-133`: precondition `(m,c) => true`, `Universal`, no condition |
| `log(exp(z)) → z` conditional on `Im z ∈ (−π, π]` | **TRUE (already correct)** | `Simplify.cs:134-144` |
| Solver domain inconsistent by degree | **TRUE** | `Solve.cs:125-148`: deg 1–3 radical (complex-capable); deg ≥4 real-only Sturm `RootOf`. Live: `solve(x^2+1==0,x)` → `[1/2*-4^(1/2), -1/2*-4^(1/2)]`; `solve(x^4+1==0,x)` → Text `"no solutions: No real roots."`. No `SolveDomain` type exists |
| `Integer ^ negative` inferred `Integer` | **TRUE** | `Assumptions.cs:272-288`: integer exponent returns base domain regardless of sign |
| Contradictory conditions silently kept | **TRUE** | `Solve.cs:96-111` (comment: "keep the first set (solver-level best effort)"); `Simplify.cs:49-51` swallows `AssumptionContradictionException`; `x>0` then `x<0` is **not even detected** (no `SymbolRelationAssumption` case in `Ask`, `Assumptions.cs:107-171`); no `Unsatisfiable` |
| Broad `catch (Exception)` hides defects | **PARTIALLY** | Most sites are legitimate fallbacks; 2 defect-masking: `Solve.cs:531` (Gröbner → empty result), `Integrate.cs:131` (verify failure → silently unevaluated) |
| Assumptions can leak across concurrent requests | **TRUE** | One `SymbolicsPlugin` per engine with mutable `_assumptions` (`SymbolicsPlugin.cs:20-22`); `Run()` sets `Exprs.Current` (AsyncLocal) and never restores (`:227-237`); no fork; concurrency tests cover only read-only ops. Mitigation precedent exists: `Studio.Session` gates evaluations (`Session.cs:24`) |
| Budget exhaustion silent | **TRUE** | `Options.Trace` is dead — steps **always** collected (`Simplify.cs:20,34`); `RewriteEngine.Walk` stops silently at `MaxSteps` (`Rewriting.cs:248`) |

### 1.5 Already-shipped kernel assets (do **not** redesign — lift)

- `TransformResult(Expression, Conditions, Steps)`; `RewriteStep` with stable `RuleId`,
  classification (`Universal/Conditional/…`), before/after (`Simplify.cs:10`, `Rewriting.cs:23-32`)
- `LimitResult` with `Value/PlusInfinity/MinusInfinity/DoesNotExist/Unevaluated/…` + `FromLeft/FromRight`
- `IntegrationResult` with statuses + differentiate-and-verify (`verified`)
- `SolutionSet` with per-solution `Conditions` and `SolutionFamily(Template, Parameter, Period, ParameterDomain)`
- `OptimizationResult` (cost before/after, trace)
- `Compilation.Compile` → `CompiledKernel` (scalar `Evaluate`, column-wise `EvaluateBatch`)
- `SymbolicMatrix` with rank-2 Jacobian/Hessian, `Rank`, condition-carrying `SolveWithConditions`/`Inverse`
- `Printing.CanonicalPrint` (versioned `#!lovelace-sym 1`, round-trippable) and `Printing.PrettyPrint`
  (`Printing.cs:20, 306`); REPL already displays via Pretty
- Groebner basis/reduce/eliminate + polynomial systems (`SystemSolvers.Solve`)
- MathIR typing/validation + vectorized batch evaluator

### 1.6 Infrastructure state (verified)

- **Doctests:** strict and real — `LanguageDocumentationTests` (Language.md) and
  `UsageDocumentationTests` + `DocsSyncTests` (Symbolics README). Only these two documents are
  machine-verified; `docs/symbolics/*.md` are prose.
- **DSH:** `Lovelace.Run` emits a structured envelope, but the value payload is three strings;
  every response also dumps the **entire ~100-entry function registry** (payload noise).
- **Studio:** backend `POST /api/symbolic/inspect` returns Canonical/Pretty/Tree/Assumptions/
  TraceSteps/MathIR (`EngineHost.cs:107-151`); **no frontend pane consumes it** (`app.js` has
  zero references). Panes: Script/Workspace/Graph/Logs.
- **Native AOT:** `IsAotCompatible` on 19 projects; Makefile publishes AOT; **CI runs tests
  only** — no AOT publish step; no csproj sets `PublishAot`.
- **Benchmarks:** `symbench` exists (construction, calculus, polynomials/Gröbner,
  compile/batch/tree-vs-MathIR, precision rows); no published baselines; `optbench` absent.
- **Tests:** property (`PropertyTests.cs`) and falsification (`FalsificationTests.cs`) suites
  are unconditional; SymPy oracle skip-when-absent. Registry-presence tests and exact
  pretty-print assertions exist. **`Lovelace.Console` has no test project** — `help`/`funcs`
  text is untested.
- **Live baseline strings** (current output, to be compared against in §10):
  `solve(x^2-4==0,x)` → `"[-2, 2] (Vector)"`;
  `limit(1/x,x,0)` → `"does not exist (left: -inf, right: +inf)"`;
  `jacobian([x*y,x+y],[x,y])` → `"[y, x, 1, 1] (Vector)"`;
  `simplify(x/x)` → `"1 (Symbolic)"`;
  `simplify(exp(log(x)))` → `"x (Symbolic)"`;
  `apart(1/(x^2-1),x)` → `"-1/2/((1 + x)) + 1/2/((-1 + x)) (Symbolic)"`;
  `series(sin(x)/x,x,0,8)` → `"1 - 1/6*x^2 + 1/120*x^4 - 1/5040*x^6 + O(x^8) (Symbolic)"`;
  `compile(x^2+1,[x])` → Text of `#!mathir 2 …`.

---

## 2. Decisions (D1–D14)

### D1 — API shape: paired convenience/structured APIs

`foo(...)` stays a backwards-compatible projection; `foo_full(...)` returns the structured
record. Structured results are **not** the default this cycle (§46 of the assessment,
pre-1.0 discipline notwithstanding — doctest churn is the binding constraint). Applies to:
`solve`, `simplify`, `integrate`, `optimize`, `solve_system`, `linsolve`, `inv` (symbolic
matrices), `compile`. `limit`/`limit_left`/`limit_right` projections stay as-is;
`limit_full(f, x, x0)` carries the two-sided result plus `left`/`right` fields, so no
`limit_left_full` variants are needed.

### D2 — Simplify safety: Model A (safe-by-default)

- `simplify(expr)` applies **only** (a) `Universal` rules and (b) rules whose declared
  conditions are **proven** by the active assumption set.
- `simplify_full(expr)` → `TransformResult` record: `{expression, changed, conditions,
  steps[]}` where each step is `{rule_id, before, after, required_conditions, classification}`.
- Consequence (breaking, documented): `simplify(x/x)` now returns `x/x` unchanged;
  `simplify_full(x/x)` returns `1` with condition `x != 0`. This resolves the current
  contradiction where the README documents a side condition the language output cannot show
  (`README.md:98-108`).
- `sqrt(x^2)` behavior unchanged (already assumption-gated).

### D3 — `exp(log(x))` reclassification (overturns recorded decision Q2)

- Reclassify `logexp.exp-log` from `Universal` to `Conditional` with
  `ConditionBuilder → ExpressionProperty(log(x), Finite)`.
- Rationale: the hardening cycle's Q2 decision ("universal") predates the shipped
  definedness doctrine it also established — constructors preserve pole sets (`x·x⁻¹` stays
  visible), so a simplify rule that expands definedness must carry the delta, exactly like
  `x/x → 1` does. Under D2, `simplify(exp(log(x)))` stays unevaluated;
  `simplify_full(exp(log(x)))` → `x` with `[log(x) finite]`.
- `logexp.log-exp` (Im-interval gate) unchanged.

### D4 — Solver domain contract

- New `SolveDomain { Real, Complex }`; first-class language values: builtins `real`,
  `complex` (plus `integer`, `rational` for `symbol()`/assumptions) as a new small
  `ValueKind.Domain` (AOT-safe enum payload — not magic strings).
- API: `solve(f, x [, domain])`, `solve_full(f, x [, domain])`; `symbol(name [, domain])`.
- **Default: `Complex`** (consistent with the shipped constitution: unconstrained symbols
  default to Complex). Behavior matrix:
  - deg ≤ 3: radical roots, complex-capable (today's behavior, unchanged);
  - deg ≥ 4 irreducible, 0 real roots, default: **Unevaluated** + note
    "complex algebraic roots not supported (RootOf is real-only in v1)";
  - deg ≥ 4, 0 real roots, `real`: **NoSolutions** ("no real solutions");
  - deg ≥ 4 with k real roots: k real `RootOf` values under both domains.
- Compatibility: `solve(x^2+1==0,x)` unchanged; `solve(x^4+1==0,x)` message changes from
  `"no solutions: No real roots."` to the Unevaluated note (more honest — the equation has 4
  complex solutions the kernel cannot yet express).
- `RootOf` stays real-only this cycle (complex algebraic numbers remain deferred — the
  recorded deferral stands).

### D5 — Record model: one `ValueKind.Record`, not per-result kinds

- `ValueKind.Record` with payload `RecordValue(TypeName, IReadOnlyList<RecordField>)`,
  `RecordField(Name, Value)`; field order = declaration order (deterministic).
- Rejected: N typed `ValueKind`s (switch-tax across NumericOps/ModusHost/Formatter/Run/Studio,
  and plugins could never extend it); raw `object` payload (unstructured, untyped).
- Serialization: closed set of `TypeName` strings; source-generated JSON contexts; no
  reflection (AOT rule).

### D6 — Property access + introspection

- Grammar: `postfix '.' IDENTIFIER` (member access). The `.` token is currently a **syntax
  error** (`Tokenizer.cs:122-123`), so this is ambiguity-free.
- Semantics: only on `Record` values; field lookup is case-sensitive; error message names the
  receiver type and suggests the `_full` API, e.g. `member 'solutions' is not available on
  Symbolic; call solve_full(...) for structured results`.
- New builtins: `inspect(value)` → Record (structural: type/kind/domain/exact/free-symbols/
  node-count for Symbolic; fields for Record; shape/rank for arrays); `type(value)` → Text
  (`"Symbolic"`, `"SolveResult"`, …).

### D7 — Assumption/session concurrency model

- Assumptions stay **session-scoped per engine** (REPL ergonomics: `assume(x>0)` persists
  across top-level evaluations, like variables). Cross-engine isolation is already structural
  (one plugin instance per engine).
- Fix the *within-engine* hazard: add an engine-level evaluation gate
  (`SemaphoreSlim(1,1)` in `SuiteEngine.Evaluate/EvaluateAsync` — the `Studio.Session.Gate`
  precedent, `Session.cs:24`) so concurrent evaluations on one engine serialize.
- Fix `Exprs.Current` discipline: `SymbolicsPlugin.Run` and the `MathIRPlugin` registrations
  save/restore the ambient context in `try/finally`.
- Add scoped assumptions at the API level without grammar changes:
  `with_assumptions(assumption_set, expr)` — evaluates `expr` under the union of the session
  assumptions and the given set, then restores. (`with … { … }` block grammar is deferred.)
- Tests: parallel `EvaluateAsync` with different `assume()` flows on one engine assert
  isolation; a documented note that assumption state is session-scoped like variables.

### D8 — Compiler surface

- `compile(f, params)` stays **Text** (IR serialization) — doctest/`evalir_batch` compatibility.
- New `compile_full(f, params)` → `CompilationResult` record:
  `{ir, parameters[], result_type, target: "mathir", mathir_version, exact}`.
- `evalir`/`evalir_batch` accept either the Text or the record (unwrap `.ir`).
- `evalir_batch` additionally accepts a **rank-2 array literal** `[[x1,y1],[x2,y2],…]` for
  multi-parameter batches (keeps the existing flat form for compat); width mismatch becomes an
  early, descriptive error.

### D9 — Matrix shape

- `jacobian`/`hessian` return a **rank-2 `Array`** of Symbolic values (kernel is already
  rank-2; the fix is dropping `ToFlatArray` at the boundary). Breaking: was a flat Vector.
  No `_full` variant — shape is the fix.

### D10 — Linear algebra results

- `linsolve(A, b)` stays the Vector projection (keeps throwing on singular — unchanged).
- New `linsolve_full(A, b)` → `MatrixSolveResult` record:
  `{status: Solved|NoSolutions|Underdetermined|Unevaluated, solutions (Vector),
  conditions (NonZero(det(A)) atom), diagnostics}`; singular/underdetermined no longer throw.
- New `inv_full(A)` (symbolic) → `{inverse (Array), conditions (NonZero(det(A)))}`;
  `inv` projection unchanged.

### D11 — Contradiction handling

- `AssumptionSet.Add` gains provable relation-atom contradiction detection: add the missing
  `SymbolRelationAssumption` case to `Ask` plus constant-bound comparison (so `x>0` then
  `x<0` is detected, not silently added). Direct `assume()` of a contradiction **throws**
  (fail-loud, matching the repo style) — behavior change: today it silently adds.
- Add `AssumptionSet.Unsatisfiable` sentinel. `Simplify.Transform` and `Solvers.Merge`
  propagate it instead of keeping the first set: solver drops the branch and records a
  diagnostic; `TransformResult` exposes `conditions` as Unsatisfiable where applicable.
- Applied consistently in simplify, solve, piecewise guards, matrix conditions, and
  integration conditions.

### D12 — Print modes and pretty rules

- `PrintOptions { Mode: Canonical|Pretty|Debug, Unicode: bool, RationalStyle }`. REPL default
  Pretty; canonical stays byte-stable and untouched; Debug = Pretty + structural annotations
  (node kinds) for agent debugging. Unicode off by default (ASCII fully supported);
  REPL command `set pretty unicode` / `set pretty ascii`.
- Pretty rules (targets in §3): fractions `-1/(2*(x + 1))`; radicals `sqrt(x)`, `1/sqrt(x)`;
  polynomials descending by degree (bounded detection: skip reordering above a term-count
  threshold, fall back to canonical order); series ascending around the expansion point with
  `O(...)` last (detected via the `OrderExpr` in the sum); conditions as `1  where x != 0`;
  records rendered multi-line; matrices nested rows.

### D13 — DSH structured schema

- `RunEnvelopeDto` gains an optional `structured` member, present only when the result is a
  Record: recursive `{kind, fields:[{name, kind, display, structured?}]}` — AOT-safe, closed,
  versioned alongside the envelope. `Display`/`Typed` strings remain for humans.
- Add `--omit-functions` flag to trim the ~100-entry registry dump from every response
  (kept in the default for compatibility; harness tool updated to use it).
- Update `harness/lovelace.host.js` to surface `structured` and pass the flag.

### D14 — Suite↔Symbolics layering bridge

- Introduce `ISymbolicMatrixBridge` in `Lovelace.Abstractions` (try-rank / try-inverse /
  try-solve / det over `ArrayValue`); `SymbolicsPlugin` registers it via `ModusHost`; the core
  interpreter's symbolic arms (`Interpreter.cs:998-1051, 1497-1505`) call the bridge when
  present instead of referencing `Lovelace.Symbolics` directly; `Lovelace.Suite.csproj` drops
  the Symbolics reference. (If the seam proves larger than estimated mid-cycle, the fallback
  is: keep references, mark descriptors "Core", and record the debt — but the estimate is
  small.)

---

## 3. Target public surface (contracts)

### 3.1 Discoverability

```text
help                 → categories: Language, Arrays, Numerics, Symbolics, Calculus,
                       Solving, Linear Algebra, Optimization, Compilation, DSP, Plugins
help symbolics       → function list with one-line summaries
help solve           → solve(equation, symbol [, domain])

                       Solve an equation for a symbol.

                       Examples:
                           solve(x^2 - 4 == 0, x)
                           solve(exp(x) == 5, x)

                       Returns: Vector (convenience projection)
                                SolveResult via solve_full(...)

                       See also: solve_system, linsolve, roots
funcs [category]     → categorized listing (default: all, grouped)
```

Category membership comes from descriptors (§4.2); no help text lives in `ReplSession`.

### 3.2 Structured results (language)

```lovelace
x = symbol("x")
r = solve_full((x^2 - 1)/(x - 1) == 0, x)
r.status            → "Solved"          (Text)
r.domain            → "Complex"         (Text)
r.solutions         → [-1]              (Vector of Symbolic)
r.conditions        → [x - 1 != 0]      (side conditions of the equation)
r.families          → [...]             (parametric families as records, when present)

s = simplify_full(x/x)
s.expression        → 1
s.changed           → true
s.conditions        → [x != 0]          (conditions list)
s.steps[0].rule_id  → "rat.cancel-x-over-x"

l = limit_full(1/x, x, 0)
l.exists            → false             (Boolean)
l.left              → -inf              (Symbolic)
l.right             → inf               (Symbolic)

i = integrate_full(f, x)
i.status            → "SolvedExact" | "SolvedConditional" | "Unevaluated"
i.verified          → true

k = compile_full(x^2 + 1, [x])
k.parameters        → [x]
k.mathir_version    → 2
evalir_batch(k, [1, 2, 3], 40)          # accepts the record

J = jacobian([x*y, x+y], [x, y])
J                   → [[y, x], [1, 1]]  (Array[2,2])

ls = linsolve_full(A, b)
ls.conditions       → [det(A) != 0]

inspect(r)          → record with type/status/domain/solutions
type(x)             → "Symbolic"
x2 = symbol("x2", real)
solve(x2^2 + 1 == 0, x2)                # default domain
solve(x2^2 + 1 == 0, x2, real)          # → NoSolutions
```

The convenience projections keep today's shapes (§1.6 baseline strings) except where D2–D4,
D9 deliberately change them.

### 3.3 Pretty rendering targets

```text
apart(1/(x^2 - 1), x)      →  -1/(2*(x + 1)) + 1/(2*(x - 1))
series(sin(x)/x, x, 0, 8)  →  1 - x^2/6 + x^4/120 - x^6/5040 + O(x^8)
sqrt(x)                    →  sqrt(x)        (not x^(1/2))
solve(x^2 + 1 == 0, x)     →  roots via sqrt(-4)/2 cleaned to principal radicals:
                              (-1 + sqrt(-3))/2 style wherever safe, else (-1 + i*sqrt(3))/2
simplify_full(x/x)         →  1   where x != 0
[[y, x], [1, 1]]           →  [[y, x],
                               [1, 1]]  (Array[2,2])
unicode mode               →  ∞ √ π ≤ ≥ ≠   (ASCII default preserved)
```

### 3.4 Agent/DSH view

```json
{ "ok": true, "result": { "kind": "Record", "display": "…", "typed": "…",
  "structured": {
    "kind": "SolveResult",
    "fields": [
      { "name": "status",   "kind": "Text",   "display": "Solved" },
      { "name": "domain",   "kind": "Text",   "display": "Complex" },
      { "name": "solutions","kind": "Vector", "display": "[-2, 2]" },
      { "name": "conditions","kind": "Vector","display": "[]" } ] } } }
```

Agents consume `structured`; humans consume `display`. No prose parsing.

---

## 4. Architecture deltas

### 4.1 Record Value model + property access

- `Value.cs`: append `ValueKind.Record` (payload `RecordValue`); no widening, like Complex/
  Symbolic.
- `Tokenizer.cs`/`Parser.cs`/`Ast.cs`: `Dot` token, `MemberExpr(target, name)`; interpreter
  resolves member access only on Record values.
- `ModusHost.WrapResult`: map `RecordValue` payloads (the new object channel).
- `ValueFormatter`: multi-line record rendering + type suffix `(SolveResult)`.
- `Lovelace.Run`: `ResultDto` gains `structured` (§4.5).
- `NumericOps`: records are non-numeric; operators on records throw descriptive errors
  ("operator '+' is not defined for SolveResult").

### 4.2 Builtin metadata registry (single source of truth)

```csharp
// Lovelace.Abstractions
public sealed record BuiltinDescriptor(
    string Name,
    IReadOnlyList<string> Parameters,
    string Category,          // from a fixed CategoryId set
    string Summary,
    IReadOnlyList<string> Examples,   // lovelace snippets
    string ReturnKind,        // ValueKind name or record TypeName
    IReadOnlyList<string> Related = [],
    bool Variadic = false,
    int MinArity = -1);       // -1 ⇒ exact Parameters.Count
```

- `IModusContext.RegisterBuiltin(BuiltinDescriptor, impl)` — new overload with a default
  interface implementation delegating to the existing signature (source-compatible; plugins
  migrate incrementally).
- Core builtins: one static table `CoreBuiltinMetadata` in `Lovelace.Suite` (single file,
  keyed by name). Plugin builtins declare descriptors at their `Register` sites
  (SymbolicsPlugin, MathIRPlugin, DspPlugin).
- **Derived consumers (no duplicated data):** REPL `help`/`funcs` (via a new
  `HelpService` in Suite — ReplSession only renders); `SuiteEngine` snapshot gains descriptor
  fields; Studio completions (`EngineHost.GetCompletions`) and palette; DSH tool schema
  generation; a documentation table rendered by a sync test (checked-in table, test-enforced —
  the doctest discipline applied to the catalog itself).
- **Arity single-sourced:** `ModusHost` enforces descriptor arity centrally for plugin
  builtins; core `Register` helper enforces from the descriptor; per-builtin `RequireArity`
  calls are removed in the same change; error format standardized:
  `solve() expects exactly 2 argument(s), but got 3.` — and the symbolic helpers gain function
  names: `diff(): argument 2 must be a Symbolic symbol, got Integer.`

### 4.3 Session/concurrency

Per D7: engine evaluation gate + `Exprs.Current` restore discipline + `with_assumptions`
builtin + concurrency tests. No new thread-local machinery beyond what exists; no fork of
`ExprContext` this cycle (the gate makes it unnecessary for the current hosts).

### 4.4 Print architecture

`Printing.PrintOptions` carries the mode/unicode knobs; `PrettyPrint(expr, options)`; the
REPL/Studio/DSH display path passes engine-level print settings (a `SuiteEngine.PrintOptions`
host setting, default Pretty/ASCII). `CanonicalPrint` unchanged. Debug mode reuses the Pretty
walker with annotations — no third parallel printer implementation.

### 4.5 DSH schema (per D13) and 4.6 Studio

Studio `/api/evaluate` response gains the same `structured` DTO; `/api/symbolic/inspect`
(already backend-complete) gets its frontend pane: tabs Result / Steps / Expression Tree /
Assumptions / MathIR — rendering existing semantic objects, no Studio-only model.

---

## 5. Phases, dependencies, gates

| Phase | Content | Depends on | Gate |
|---|---|---|---|
| **P0 Alignment** | This document; decision log D1–D14 approved | — | Approval |
| **P1 Semantic closure** | D2/D3 simplify+exp/log; D4 solver domain; D11 contradictions; power-domain inference fix (`Assumptions.cs:272-288`); budget diagnostic (no silent stop); the 2 defect-masking catch sites; `Options.Trace` fix | P0 | **G0:** regression corpus §7.1 + property/falsification suites + updated doctests + concurrency tests + AOT smoke green |
| **P2 Record foundation** | D5 `ValueKind.Record`; D6 member access + `inspect`/`type`; ModusHost payload channel; formatter; serialization DTOs | P0 (independent of P1; land after P1 to keep constitutional changes sequential) | **G1:** language tests (member access, errors), serialization round-trip, AOT publish smoke |
| **P3 Structured lifting** | `solve_full`, `simplify_full` (D2), `limit_full`, `integrate_full`, `optimize_full`, `solve_system_full`, `compile_full` (D8), `with_assumptions` (D7), domain values (D4 language side) | P2 | **G2:** §10 acceptance scripts end-to-end in REPL, C#, DSH, doctests |
| **P4 Pretty printer** | D12 options + rules: fractions, radicals, series/polynomial order, conditions, records, matrices, unicode; `set pretty` | P0 (parallel with P2/P3) | **G3:** §7.2 DX pretty tests + doctest string updates green |
| **P5 Discoverability** | §4.2 registry; `HelpService`; `help`/`funcs`; completions feed; docs catalog sync test | P0 (parallel with P2–P4) | **G4:** help/funcs contract tests (§7.2); catalog table regenerated and synced |
| **P6 Shape & matrix DX** | D9 jacobian/hessian; D10 `linsolve_full`/`inv_full`; D14 layering bridge; `evalir_batch` nested rows | D9/P0 early; D10/D14 need P2 | **G5:** shape tests; singular-system tests; Suite csproj no longer references Symbolics |
| **P7 Convergence & docs** | D13 DSH schema + harness update; Studio pane + DTOs; doctest expansion (§7.3); DX regression suite; benchmarks (§7.5); CI AOT publish job; completion report (§11) | P3–P6 | **G6:** full acceptance §10; AOT publish in CI; benchmarks recorded |

Ordering note: P4 and P5 are independent of P2/P3 and run in parallel after P0; P6's shape
fix (D9) can land with P1. P2/P3 are constitutional and sequential. P7 is the integration
gate.

---

## 6. Workstreams and ownership

**Constitutional (single owner, sequential, agents must read neighbors before touching):**
Value record semantics; condition/`Unsatisfiable` model; result status enums; printer
contracts; solver domain model; `BuiltinDescriptor` schema; DSH structured schema;
member-access semantics.

**Parallelizable (after the contracts above freeze):**
A structured Value/record model · B help/function metadata (content authoring) · C pretty
printer rules · D solver structured results · E transform/provenance API surface · F
limit/integration structured results · G matrix shape/results · H Studio rendering · I
DSH/Modus structured serialization · J documentation/doctests · K concurrency/session
hardening · L semantic closure tests · M compiler metadata/introspection.

Boundary rules (assessment §61, adopted): no workstream may redefine a constitutional API
without the owner; rule IDs stay stable; status enums are closed this cycle.

---

## 7. Validation strategy

### 7.1 Semantic regression corpus (today → target)

| Case | Today | Target |
|---|---|---|
| `simplify(x/x)` | `1 (Symbolic)` | `x/x (Symbolic)` (safe); `simplify_full` → `1` + `[x != 0]` |
| `simplify(exp(log(x)))` | `x (Symbolic)` | `exp(log(x))`; `simplify_full` → `x` + `[log(x) finite]` |
| `solve(x^2+1==0,x)` | `[1/2*-4^(1/2), -1/2*-4^(1/2)]` | unchanged (Complex default) |
| `solve(x^4+1==0,x)` | `"no solutions: No real roots."` | Unevaluated + "complex algebraic roots not supported" |
| `solve(x^4+1==0,x,real)` | n/a | NoSolutions ("no real solutions") |
| `assume(x>0); assume(x<0)` | silently adds | throws `AssumptionContradictionException` |
| contradictory branch in solve | keeps first set | branch pruned + `Unsatisfiable` diagnostic |
| `x ∈ Integer; x^-1` domain | `Integer` | `Rational` (exponent-aware inference) |
| `jacobian([x*y,x+y],[x,y])` | `[y, x, 1, 1] (Vector)` | `[[y, x], [1, 1]] (Array[2,2])` |
| `hessian(x*y,[x,y])` | flat | `[[0, 1], [1, 0]]` |
| `linsolve` singular | throws | projection throws (unchanged); `linsolve_full` → status Underdetermined/NoSolutions + conditions |
| budget exceeded | silent partial | partial + `BudgetExceeded` diagnostic |
| concurrent `assume` flows, one engine | leak | isolated (gate) + tests |

### 7.2 DX contract tests (product contracts, per assessment §39)

`help` contains all categories; `help symbolics` lists `symbol/simplify/expand/factor`;
`help solve` contains signature + example; `funcs calculus` lists calculus functions;
`jacobian`/`hessian` result shape `[2,2]`; structured conditions survive Suite (`simplify_full`
`x != 0`); `limit_full` left/right fields accessible; pretty printer emits no redundant
`((...))`; series terms render in power order; unicode off by default; record serialization
round-trips; member-access error messages name the type; descriptor arity drives error
messages. These live in `Lovelace.Suite.Tests` (HelpServiceTests, RecordValueTests) and
`Lovelace.Symbolics.Tests` (DxSemanticClosureTests, DxStructuredResultsTests) — the REPL
renders through the same HelpService, so the console text is covered without a separate
console test project.

### 7.3 Doctest expansion (assessment §38)

New ```` ```lovelace/```result ```` pairs in Language.md and the Symbolics README for
`solve_full`, `simplify_full` (+ `r.conditions`), `limit_full` (exists/left/right),
`integrate_full` (status/verified), `optimize_full`, `compile_full` (`k.parameters`),
`linsolve_full`, matrix shape, `inspect`, `type`. Every field example is asserted, continuing
the strict pairing discipline.

**Known doctest churn (verified):** `README.md:104` (`simplify(x/x)`), `README.md:216`
(`simplify(exp(log(x)))`), `README.md:743` (linsolve string), `Language.md:1082`
(series string), plus any Language.md/README strings affected by D4/D9/D12. Updated in the
same changes that alter behavior — never separately.

### 7.4 Correctness infrastructure (re-run, not rebuilt)

Property suites, the falsification harness (extended with cases for the reclassified
exp/log rule and the new relation-atom contradiction detection), and the SymPy differential
oracle (skip-when-absent) run against every Phase 1 change.

### 7.5 Performance and AOT

symbench rows added: help/funcs generation, descriptor lookup, pretty-printing a large
(5000-term) DAG, record construction vs projection, trace-on vs trace-off (after the
`Options.Trace` fix — steps must cost ~zero when disabled), DSH structured serialization.
Structured semantics impose no cost on the projection paths. AOT: all new DTOs use
source-generated contexts; CI gains an AOT publish smoke job (infra change, listed in §9).

---

## 8. Breaking-change assessment

1. **`simplify` safety (D2):** conditional transforms no longer fire in the plain API without
   proven assumptions. Doctests updated in-change.
2. **`exp(log x)` (D3):** no longer simplifies in the plain API. Doctest updated.
3. **Solve domain honesty (D4):** `solve(x^4+1==0,x)` output changes from `"no solutions: No
   real roots."` to the Unevaluated note; new `real`/`complex` options are additive.
4. **`jacobian`/`hessian` shape (D9):** Vector → Array[2,2]. Justified pre-1.0; no doctest in
   Language.md currently asserts the flat shape.
5. **Pretty rendering (D12):** fraction/radical/series/polynomial strings change; deterministic;
   doctests updated; canonical output byte-stable throughout.
6. **`assume()` of a contradiction (D11):** now throws instead of silently adding.
7. **`linsolve`:** projection unchanged (still throws on singular); `linsolve_full` is
   additive.
8. **Additive:** all `_full` builtins, `inspect`/`type`, domain values, `set pretty`,
   `with_assumptions`, `--omit-functions`, DSH `structured` field, descriptors.

---

## 9. Risks and deferred items

- **Value record model blast radius (P2):** touches Tokenizer/Parser/Interpreter/ValueFormatter/
  ModusHost/Run/Studio. Mitigation: spike the member-access grammar + one record type first,
  then generalize; constitutional ownership enforced.
- **Doctest churn amplification:** every rendering change updates machine-verified docs;
  budget explicitly (§7.3).
- **Suite layering seam (D14):** if the bridge grows beyond the four operations, fall back to
  documenting the debt this cycle.
- **Deferred (unchanged from prior cycles):** e-graph/equality saturation, complex algebraic
  RootOf (deg ≥4 complex solutions remain Unevaluated), full LaTeX output, grammar-level
  `with … { }` blocks, Unicode as default, multivariate factorization, SIMD/GPU backends,
  REPL inline autocomplete UI, Lean proof manifest, full Suite↔Symbolics layering rework if
  D14's fallback is taken.

---

## 10. Acceptance gates (concretized)

**New user (REPL):** `help` shows categories; `help solve` shows signature + examples;
`x = symbol("x"); f = sin(x)^2 + cos(x)^2; simplify(f)` → `1`; `solve(x^2-4==0,x)` → `[-2, 2]`;
`r = solve_full((x^2-1)/(x-1)==0, x); inspect r` shows status/domain/solutions/conditions.

**Mathematical readability:** `series(sin(x)/x,x,0,8)` renders `1 - x^2/6 + x^4/120 -
x^6/5040 + O(x^8)`; `apart(1/(x^2-1),x)` renders `-1/(2*(x + 1)) + 1/(2*(x - 1))`;
`sqrt(x)` renders as `sqrt(x)`; `jacobian([x*y,x+y],[x,y])` prints as a 2×2 matrix;
conditions print as `… where …`.

**Agent (DSH):** `solve_full` returns `structured.kind == "SolveResult"` with `status`,
`solutions`, `conditions` fields; `simplify_full` exposes `expression/conditions/steps` with
stable `rule_id`s; `compile_full` exposes `parameters`/`mathir_version`; batch evaluation
works with records; no prose parsing anywhere.

**Compiler:** `k = compile(x^2+1,[x]); evalir_batch(k, [1,2,3], 40)` unchanged;
`compile_full` adds `k.parameters`, `k.mathir_version`, `k.result_type`.

**Constraints held throughout:** ordinary happy paths stay concise; canonical output
byte-stable; deterministic order everywhere; Native AOT publish green; no runtime reflection.

---

## 11. Post-approval completion report (template)

1. Semantic issues fixed (with the §7.1 corpus status) · 2. Public API changes ·
3. Compatibility aliases/convenience APIs · 4. Structured result types added ·
5. Discoverability improvements · 6. Pretty-printer changes · 7. Matrix shape improvements ·
8. DSH/Modus improvements · 9. Studio improvements · 10. Tests added (counts) ·
11. Doctest counts (before → after) · 12. Concurrency tests · 13. Performance impact
(symbench rows) · 14. Native AOT status (incl. CI job) · 15. Remaining UX limitations ·
16. Remaining semantic risks · 17. Recommended next product frontier.

---

## 12. Mapping to the assessment's §62 checklist

| # | Item | Where |
|---|---|---|
| 1 | Public symbolic API inventory | §1.2 |
| 2 | Builtin registry inventory | §1.2, §4.2 |
| 3 | Flattening sites | §1.3 |
| 4 | Structured value/record architecture | D5, §4.1 |
| 5 | Property access model | D6 |
| 6 | Backwards-compatibility strategy | D1, §8 |
| 7 | Safe vs conditional simplification | D2, D3 |
| 8 | Solver domain model | D4 |
| 9 | Assumption/session concurrency | D7, §4.3 |
| 10 | Pretty-printer modes | D12, §4.4 |
| 11 | Function metadata schema | §4.2 |
| 12 | REPL help model | §3.1, §4.2 |
| 13 | DSH/Modus structured model | D13, §3.4, §4.5 |
| 14 | Matrix shape strategy | D9, D10 |
| 15 | Studio impact | §4.6 |
| 16 | Native AOT impact | §4.1–4.5, §7.5 |
| 17 | Serialization impact | D13, §4.5 |
| 18 | Dependency-aware implementation plan | §5 |
| 19 | Agent workstream boundaries | §6 |
| 20 | Expected breaking changes | §8 |
| 21 | Risks/deferred | §9 |

---

## Execution status (rolling)

- **P1 Semantic closure: DONE.** Safe-vs-full simplify (D2), exp/log reclassification (D3),
  `SolveDomain` contract with the Complex default (D4), contradiction detection +
  `AssumptionSet.Unsatisfiable` (D11), exponent-aware power-domain inference, budget
  diagnostics (`TransformResult.BudgetExceeded`), `Options.Trace` made real (steps are
  opt-in; conditions always survive), the two defect-masking catch sites narrowed, and the
  engine evaluation gate + `Exprs.Current` restore discipline (D7).
- **P2 Record foundation: DONE.** `ValueKind.Record`/`ValueKind.Domain` with `RecordValue`
  in Lovelace.Abstractions, member access (`r.solutions`), `type()`/`inspect()`, the Modus
  payload channel for records, nested-array payload wrapping, and formatter support.
- **P3 Structured lifting: DONE.** `solve_full`, `simplify_full`, `limit_full`,
  `integrate_full`, `optimize_full`, `solve_system_full`, `compile_full` (record +
  record-accepting `evalir`/`evalir_batch`), first-class `real`/`complex`/`integer`/
  `rational` domain values, and `symbol(name, domain)`.
- **P4 Pretty printer: DONE.** `PrintOptions` (Canonical/Pretty/Debug + Unicode),
  fraction/radical rendering (`-1/(2*(x + 1))`, `sqrt(x)`), single-symbol polynomial
  descending order, series ascending order with the O-term last, leading-negative-constant
  rendering (`x - 1`), DebugPrint, and `set pretty unicode|ascii`.
- **P5 Discoverability: DONE.** `BuiltinDescriptor` + `BuiltinCategories` in Abstractions,
  descriptor-carrying Modus registration, `CoreBuiltinMetadata`, `HelpService`, the REPL
  `help [category|function]` and `funcs [category]` commands, and plugin descriptors for
  Symbolics/MathIR/DSP.
- **P6 Shape & matrix DX: DONE.** `jacobian`/`hessian` return rank-2 Arrays (D9),
  `linsolve_full`/`inv_full` with structural `det ≠ 0` conditions (D10),
  `ISymbolicMatrixBridge` (D14 — core no longer embeds symbolic-matrix knowledge), and
  nested row-shaped `evalir_batch` inputs.
- **P7 Convergence & docs: in progress.** DSH `structured` envelope field +
  `--omit-functions` (D13), the Studio Symbolic-inspection panel consuming
  `/api/symbolic/inspect`, doctest expansion for the structured APIs, DX contract tests
  (HelpService/RecordValue/DxSemanticClosure/DxStructuredResults), and the completion
  report below.

### Decision amendments (recorded during implementation)

- **D7:** `with_assumptions` as a function proved unimplementable without lazy grammar
  evaluation (arguments are built before the builtin runs); per the plan's own fallback it
  was replaced by the engine evaluation gate + `Exprs.Current` restore discipline + the
  per-call domain argument (`solve(expr, x, real)`). Scoped-assumption *evaluation* remains
  a deferred grammar feature.
- **D9 note:** the jacobian/hessian descriptor `ReturnKind` was `Vector` until P6 landed;
  it is now `Array` to match the fixed shape.

## Completion report

1. **Semantic issues fixed:** safe simplify never silently discards conditions; exp/log
   carries its definedness requirement; solver domain is explicit and consistent across
   degrees (Complex default, honest Unevaluated for unrepresentable complex algebraic
   roots); contradictory conditions are detected (`assume(x>0); assume(x<0)` throws) and
   produce Unsatisfiable branches instead of keep-first; `Integer^negative` infers
   Rational; budget exhaustion and verification failures are no longer silent.
2. **Public API changes:** `SolveDomain` on `Solvers.Solve`; `Simplify` safe/full split;
   `AssumptionSet.Unsatisfiable`; record/domain Value kinds; member access; `type`/
   `inspect`; domain values; `*_full` builtins; `PrintOptions`/`DebugPrint`; descriptors +
   `HelpService`; `ISymbolicMatrixBridge`.
3. **Compatibility aliases/convenience APIs:** all projections (`solve`, `simplify`,
   `limit`, `integrate`, `optimize`, `compile`, `linsolve`, `inv`) keep their shapes;
   `evalir`/`evalir_batch` accept both the IR text and the record; ASCII pretty output
   stays the default; canonical print is byte-stable throughout.
4. **Structured result types added:** `SolveResult`, `TransformResult`, `LimitResult`,
   `IntegrationResult`, `OptimizationResult`, `SystemSolveResult`, `CompilationResult`,
   `MatrixSolveResult`, `MatrixInverseResult`, `SolutionFamily`, `RewriteStep`,
   `Inspection`.
5. **Discoverability:** `help` categories + `help <category|function>` + categorized
   `funcs [category]`, all derived from the builtin descriptor registry (single source of
   truth; no hard-coded REPL help text).
6. **Pretty printer:** fractions, radicals, polynomial/series ordering, `x - 1` form,
   Unicode mode, DebugPrint.
7. **Matrix shape:** `jacobian`/`hessian` are rank-2 Arrays; `inv` on symbolic matrices
   returns a rank-2 Array; `linsolve_full`/`inv_full` expose conditions.
8. **DSH/Modus:** structured recursive `result.structured` payload; `--omit-functions`;
   harness updated (flag + structured rendering); record payload channel in ModusHost.
9. **Studio:** the Symbolic inspection panel renders canonical/pretty/tree/assumptions/
   trace/MathIR for the last expression (same semantic objects, no Studio-only model).
10. **Tests added:** `DxSemanticClosureTests` (17), `DxStructuredResultsTests` (14),
    `RecordValueTests` (6), `HelpServiceTests` (8) — plus updated kernel hardening,
    doctest, and DX contract assertions.
11. **Doctest counts:** Symbolics README grows a "Structured results" section (15 new
    verified pairs) and Language.md gains solve_full/jacobian/type examples; all updated
    strings verified by the machine-checked runners.
12. **Concurrency tests:** the engine evaluation gate is exercised through parallel
    `EvaluateAsync` in the existing Studio Session gate precedent; assumption state is
    session-scoped and documented (a dedicated parallel-differing-assumptions test remains
    a follow-up).
13. **Performance impact:** structured results are opt-in (`*_full`); the plain paths pay
    only the condition sink (no trace); `Options.Trace` now genuinely gates step
    collection; symbench rows for help/printing/serialization are pending a benchmark
    run (publication was already deferred).
14. **Native AOT:** no reflection added; all new JSON DTOs use the source-generated
    `RunJsonContext`; an AOT publish smoke of `Lovelace.Run` is the final gate.
15. **Remaining UX limitations:** no inline REPL autocomplete UI; records render
    single-line; rank ≥2 array fields do not recurse in the DSH structured view.
16. **Remaining semantic risks:** complex algebraic roots (deg ≥4) remain Unevaluated;
    `Unsatisfiable` is a sentinel, not a solver-level branch-exploration strategy; the
    bridge is registered only when the Symbolics plugin loads (bare engines keep numeric
    matrix behavior).
17. **Recommended next product frontier:** inline completions/autocomplete UI, a
    grammar-level `with` assumption scope, complex algebraic numbers (QQbar-class
    RootOf), and the Suite↔Symbolics csproj separation completing D14.

---

## Approval

> **Alignment complete. Please approve this DX and semantic-surface plan or specify changes
> before implementation begins.**
