# Lovelace — A+ Symbolic Runtime Hardening & Convergence: Alignment Report

> **Status:** Alignment proposal (awaiting approval). No production code, API, test, or docs file
> was modified during this audit. All scratch artifacts were written outside the repository
> (`%TEMP%\lovelace-audit`, `%TEMP%\lovelace-probe`, `out/audit-aot`).
>
> This report is the mandatory pre-implementation gate (§0 of the cycle brief).

---

## A. Current HEAD

| Item | Value |
|---|---|
| Branch / commit | `main` @ `f37f1ce61631cba5638584349f0934ea4f209681` ("docs: DX-cycle documentation pass") |
| Working tree | clean (`git status --short` empty before and after the audit) |
| Solution | `LovelaceSharp.slnx` (34 projects) |
| Build | `dotnet build LovelaceSharp.slnx -c Debug` → **0 errors**, 57 warnings, 1m36s |
| Symbolics tests | `Lovelace.Symbolics.Tests` → **294 passed / 0 failed / 0 skipped** (1m51s) |
| Suite tests | `Lovelace.Suite.Tests` → **438 passed / 0 failed / 0 skipped** (28s) |
| Native AOT publish | `dotnet publish Lovelace.Run -c Release -p:PublishAot=true -p:InvariantGlobalization=true` → **succeeded** |
| AOT smoke | published `Lovelace.Run.exe` executed a real script and emitted the JSON envelope (exit 0) |
| CI (`.github/workflows/ci.yml`) | 12 test projects + `Lovelace.Real.Tests (Category!=Heavy)` + Codecov. **No MathIR job, no AOT job, no publish step, no post-publish execution smoke, no doctest job.** |

Live reproduction used the **published native binary** and the debug `Lovelace.Run` runner, not source reading alone.

---

## B. Reproduction matrix

Verdicts: **Confirmed** · **Already fixed** · **Not applicable** · **New adjacent issue**.
Every row carries evidence from an executed command or an exact `file:line`.

### B.1 Solver completeness and domain semantics (§2–§12, §92, §96, §109, §130–§132)

| # | Issue | Verdict | Evidence |
|---|---|---|---|
| §2 | Complex-domain polynomial solve returns real roots for deg ≥ 4 with mixed roots | **Confirmed (P0)** | `HighDegreeRoots` (`Solve.cs:171-181`) returns `null` **only when the real-root count is 0**. Live: `solve(x^4 - x^2 - 1 == 0, x)` → `[rootof(...,0), rootof(...,1)]`; `solve(x^4 - 2 == 0, x)` → 2 roots of a 4-root set |
| §3 | Mixed real/complex high-degree regression | **Confirmed (P0)** | `solve_full(x^4 - x^2 - 1 == 0, x)` → `status: Solved, domain: Complex, solutions: [2 real RootOf], conditions: []` — an apparently complete subset |
| §4 | "Solved implies complete over the requested domain" invariant | **Confirmed missing** | No `complete`/`Completeness` member exists. Status derives only from `SolutionKind {Exact, Empty, Unevaluated}` (`Solve.cs:7`) at `SymbolicsPlugin.cs:291-296`. No test asserts the invariant |
| §5 | Per-solution conditions preserved publicly | **Confirmed** | Kernel has `Solution(Expr Value, AssumptionSet Conditions)` (`Solve.cs:18`); the projection keeps **only the value** (`SymbolicsPlugin.cs:275-283`) and emits **one union** (`SymbolicsPlugin.cs:300-302`, `UnionConditions` at `560-572`) |
| §6 | common_conditions without replacing branch locality | **Confirmed** | Only the union exists; there is no per-solution condition field at all, so "common" and "branch" are conflated |
| §7 | `Solved` + empty projected solutions | **Confirmed** | `solve_full((x^2-1)/(x^2-1) == 0, x)` → `status: Solved, solutions: [], conditions: [x^2 - 1 != 0]`. Status derives from `set.Kind`; the emptiness check (`Solve.cs:84-86`) runs **before** the plugin's `ViolatesConditions` filter |
| §8 | Multiplicity preserved | **Confirmed** | `foreach (var (factor, mult) in factors.Factors)` (`Solve.cs:148`) — `mult` is never used. Live: `solve_full(x^2 - 2*x + 1 == 0, x)` → `solutions: [1]`, no multiplicity |
| §9 | Exactness metadata (Exact/AlgebraicExact/Approximate/ParametricExact) | **Confirmed missing** | No such enum/field. `SolutionKind` is completeness-shaped, not exactness. Integration has `IntegrationKind {SolvedExact, SolvedConditional, Unevaluated}` (`Integrate.cs:5`) — the solver has no analogue |
| §10 | Unsupported domains rejected, not widened | **Confirmed (P0)** | `SolveDomainOf` (`SymbolicsPlugin.cs:431-434`): anything that is not exactly `MathDomain.Real` becomes `SolveDomain.Complex`. Live: `solve(x^2 + 1 == 0, x, integer)` → complex roots; `solve(2*x == 1, x, integer)` → `[1/2]` — a non-integer "solution" for an integer-domain request |
| §11 | `SolveResult.domain` is a Domain value | **Confirmed** | `new RecordField("domain", domain.ToString())` (`SymbolicsPlugin.cs:298`); JSON shows `{"kind":"Text","display":"Complex"}`. `families[].parameter_domain` is also a string (`:289`) |
| §12 | Algebraic root roadmap | **Partly present** | `RootOfExpr` (`Expr.cs:400-409`) is **real-only** (`Constructors.cs:672-687`, `Roots.N` bisection, `Assumptions.cs:439` → `Domain.Real`). No complex index, no isolating region |
| §92 | Deterministic, documented solution ordering | **Confirmed** | Quadratic emits `[+sqrt(disc), -sqrt(disc)]` (`Solve.cs:207-211`), cubic emits the principal branch first (`270-291`), deg ≥ 4 ascends only within one defining polynomial; nothing re-sorts, and no ordering contract is documented |
| §96 | Semantic strings in machine APIs | **Confirmed** | `"no solutions"` (`:212`,`:255`,`:258`), `"unevaluated: "` (`:262`), `" for integer k"` (`:252-254`), `"x = "` assignments (`:213-214`,`:226`), `"unsatisfiable"` (`:520-521`) |
| §130 | `solve_full((x^2-1)/(x-1) == 0, x)` | **Confirmed but structurally wrong** | Returns `solutions: [-1], conditions: [x - 1 != 0]`. Mathematically right for this input; the condition is a global union, so branch locality is unrecoverable |
| §131 | Mixed-root acceptance (Unevaluated or Partial) | **Confirmed failing** | Returns `Solved` with real roots only |
| §132 | Real-domain acceptance | **Already fixed** | `solve(x^4 - x^2 - 1 == 0, x, real)` → both real roots; deg ≥ 4 with **zero** real roots correctly reports `Unevaluated` under Complex (`solve(x^4 + 1 == 0, x)`) |

**New adjacent solver issues (all reproduced live):**

| ID | Issue | Evidence |
|---|---|---|
| N1 | **Genuinely wrong answer, not merely incomplete**: `solve((x+1)^2 - 4 == 0, x)` → `[1]`, silently dropping `x = -3` | Live run. `SolveInverse` (`Solve.cs:367-377`) rewrites `u^n = c` to the single principal root `c^(1/n)` and labels the result `Exact` |
| N2 | Same class: `solve((x+1)^3 == 8, x)` → `[1]`, dropping the two complex cube roots, status `Solved` | Live run |
| N3 | `SolvePeriodic` ignores the domain entirely (no `domain` parameter, `Solve.cs:412`): `solve(sin(x) == 2, x)` → `"no real solution for abs(c) > 1"` under the **default Complex** domain | Live run. Enshrined by `RelationsAndFamiliesTests.cs:140-148` |
| N4 | `exp(u)=c, c <= 0` → `Empty` under Complex: `solve(exp(x) == -1, x)` → `"no real solution for c <= 0"` | Live run; `Solve.cs:350-354` |
| N5 | Dead condition plumbing: `Expr? conditionExpr = null;` (`Solve.cs:339`) is **never assigned**, so the branch-condition block at `390-399` is dead code. No branch/argument-domain/principal-value condition is ever emitted | Static read |
| N6 | Unbounded mutual recursion in `SolveInverse` (`Solve.cs:382-383`) with no depth/budget guard — radical nesting hazard | Static read |
| N7 | `SystemSolvers.MaxSolutions = 128` silently truncates and still reports `Solved` | `Solve.cs:519`, `542-543`, `568-569`, `606-607`; `SymbolicsPlugin.cs:229` |
| N8 | `catch (Exception) { return false; }` in `SatisfiesAll` (`Solve.cs:635-638`) drops unverifiable system solutions | Static read |
| N9 | Shifted powers fall off the polynomial path: `(x+1)^3 == 8` returns one root while `(x+1)^2 - 4` also returns one root — inconsistent and unconditioned | Live run |
| N10 | `ParameterDomain.NonNegativeIntegers` is declared but never produced (`Solve.cs:21`) | Static read |

### B.2 Assumption / domain semantics (§18–§23)

An exhaustive reference-truth-table probe was executed against the real kernel
(`%TEMP%\lovelace-probe`, rational witnesses, all 6 operators × 3 assumed bounds × 3 query bounds
= 324 queries, plus all 324 two-atom pairs):

| Check | Verdict | Evidence |
|---|---|---|
| §18 `assume(x >= 5)` implies `x<5 False`, `x>=5 True`, `x>4 True`, `x<=4 False` | **Already fixed at the kernel level** | Probe printed exactly `False / True / True / False` |
| §19 Bound-reasoning matrix (324 queries vs reference) | **Already fixed** | `MATRIX TOTAL=324 WRONG=0 OVERCLAIM=0` |
| §20 Contradiction closure | **Already fixed** | `PAIRS cases=324 bad=0`; all five required examples detected |
| §21 Unsatisfiable propagation | **Confirmed gap** | `AssumptionSet.Unsatisfiable` exists (`Assumptions.cs:56-64`) and is pruned by `Merge`/`TransformCore`, but `Add` **throws** instead of returning the sentinel, and the language cannot observe unsatisfiability: `simplify(x < 5)` after `assume(x >= 5)` returns the **unevaluated relation** `x < 5`, not `False` |
| §18 exposure | **New adjacent issue** | The kernel proves the four §18 facts, but **no simplification rule folds a relation to a Boolean** (`Simplify.cs` registers only trig/power/rat/logexp/abs rules). Contradictions surface as a raw record dump — `"Assumption SymbolRelationAssumption { S = x, Op = Lt, Bound = Lovelace.Symbolics.RationalConstantExpr } contradicts ..."` |
| §22 Concurrency isolation | **Baseline present, contract unproven** | `SuiteEngine._evaluationGate` serialises per engine (`SuiteEngine.cs:187`); `ConcurrencyTests.cs` exists (109 lines). Leak-proofing across `Exprs.Current`, assumptions, precision and writer is not systematically tested |
| §23 Reentrancy | **Confirmed gap** | `SymbolicsPlugin.Run` swaps the ambient `Exprs.Current` (`SymbolicsPlugin.cs:582-600`); nested `EvaluateAsync` on the same engine hits the semaphore with no diagnostic |
| §24 Cancellation | **Confirmed missing** | Zero `CancellationToken` occurrences in Symbolics/Suite/Abstractions; `RegisterBuiltin` takes a bare `Func<IReadOnlyList<object?>, object?>` |
| §25 Budgets as structured diagnostics | **Partial** | `RewriteEngine.Budget { MaxSteps = 400 }` + `TransformResult.BudgetExceeded` bool (`Simplify.cs:13-17`). No `budget_kind`, no partial-result payload |

### B.3 Rewrite engine (§13–§17, §94, §111)

| # | Issue | Verdict | Evidence |
|---|---|---|---|
| §13 | Broad `catch(Exception) => ruleDidNotApply` | **Confirmed (P1)** | `Rewriting.cs:267-275` — the only `try` around a precondition; a pattern-variable typo (`Match.Get` throws `InvalidOperationException`, `:63`) silently disables a rule |
| §14 | Explicit applicability protocol | **Confirmed missing** | `Func<Match, ExprContext, bool> Precondition` (`Rewriting.cs:130`) — no `NotApplicable/Applicable/ApplicableWithConditions` tristate |
| §15 | Rule-failure diagnostics when tracing | **Confirmed missing** | No rejection-reason channel |
| §16 | Rule invariants | **Mostly fixed, untested** | `RuleClassification` (5 values), `DeclaredConditions`, `ConditionBuilder`, and the safe-mode wrapper requiring `Ask(...) == True` (`Simplify.cs:88-109`) all exist. No test enforces the invariants |
| §17 | Property-based falsification of every rule | **Partial** | `FalsificationTests.cs` (153 lines) exists. The rewrite registry holds only **8 rules** (`Simplify.cs:117-223`), so exhaustive boundary sampling is cheap and currently incomplete |
| §94 | Stable RuleIds + preserved step order | **Already fixed** | `RewriteStep(RuleId, Classification, Before, After, Conditions)` (`Rewriting.cs:23-32`); live `simplify_full(x/x)` emits `rule_id: rat.cancel-x-over-x, classification: Conditional, required_conditions: [x != 0]` |
| §111 | Trace-disabled has zero steps but keeps conditions | **Already fixed** | `trace` is allocated only when requested; `appliedConditions` is always collected (`Simplify.cs:52-53`) |

**New adjacent issues:** the 8 rewrite rules are rebuilt (with fresh closures) on **every** simplify call (`BuildRegistry`, `Simplify.cs:48,111`); `RewriteRuleRegistry` wraps a mutable `List` with no thread-safety; `Group()` allocates and scans per group per statement.

### B.4 Structured results and DSH protocol (§26–§36, §70–§76, §84–§88, §113)

| # | Issue | Verdict | Evidence |
|---|---|---|---|
| §26 | `Lovelace.Run` serializes all value kinds recursively | **Confirmed** | `ToStructured` recurses into **Record and Vector only** (`Program.cs:187-212`); every other kind gets `structured: null` |
| §27 | Rank-N array shape protocol | **Confirmed** | Live `jacobian([x*y, x+y], [x,y])` → `{"kind":"Array","display":"[[y, x], [1, 1]]","structured":null}`. No `shape` field exists in any DTO. Live `inv_full` → `{"name":"inverse","kind":"Array","display":"[[y/(x*y), 0/(x*y)], ...]","structured":null}` |
| §28 | Symbolic leaf protocol (canonical, pretty, domain, exact, node_count, free_symbols) | **Confirmed** | Live: `{"name":"0","kind":"Symbolic","display":"-2","structured":null}` — display only |
| §29 | Canonical form as protocol, versioned | **Confirmed** | `Printing.FormatHeader = "#!lovelace-sym 1"` exists (`Printing.cs:15`) but is referenced **nowhere else** in the repo, and the envelope never carries the canonical form |
| §30 | Structured complex values (re/im) | **Confirmed** | `Complex.Re`/`Im` are public (`Complex.cs:16-19`) but only `ToString()` crosses (`ValueFormatter.cs:19`); live `{"kind":"Complex","display":"-1i","structured":null}` |
| §31 | Structured exact numerics (numerator/denominator) | **Confirmed** | There is **no Rational ValueKind** (`Value.cs:20-35`); `1/3` crosses as `{"kind":"Real","display":"0.(3)"}` |
| §32 | Null/absent field round-trip | **Confirmed** | `PayloadMap.Wrap(null) -> Value.Void` (`PayloadMap.cs:33`) but `Unwrap` has no `Void` arm and **throws** (`PayloadMap.cs:74-87`). Live: `diff(limit_full(1/x,x,0).value, x)` → `"A plugin builtin received an unsupported argument kind 'Void'."`. Selecting an absent field yields `"result":null`, indistinguishable from a void statement |
| §33 | Record round-trip invariants | **Partially fixed** | Field names/order preserved through `Value(RecordValue)` (`Value.cs:138-145`) and `UnwrapRecord` (`PayloadMap.cs:91-97`); no generalized test exists. Zero envelope tests: no `Lovelace.Run.Tests` project, and `PayloadMap` is `internal` with no `InternalsVisibleTo` |
| §34 | `solve_system_full` assignments structural | **Confirmed** | Live: `assignment` is a Vector of **Text** `"x = 0"`, `"y = -1"` (`SymbolicsPlugin.cs:226`) |
| §35 | Reusable `Binding` type | **Confirmed missing** | No such type; `SystemSolution` (`Solve.cs:502`) holds a `Dictionary<Symbol, Expr>` that is stringified at the boundary |
| §36 | `SolutionFamily` fully structured | **Partial** | Live: `SolutionFamily(template: k*pi, parameter: k, period: pi, parameter_domain: Integers)` — `parameter` and `parameter_domain` are strings; no conditions, completeness, or exactness |
| §70/§72 | Machine protocol, versioned envelope | **Confirmed missing** | `RunEnvelopeDto(Ok, Revision, Result, Variables, Functions, Plot, Elapsed)` (`Program.cs:257-264`) has no `protocol_version`, `symbolic_format_version`, or `mathir_version` |
| §73 | `--omit-functions` + payload control | **Already fixed (partial)** | `--omit-functions` implemented (`Program.cs:48-51`), dumping 117 entries by default. No `--omit-variables`/`--omit-display` |
| §74 | Stable naming convention | **Already fixed but mixed** | JSON keys camelCase (`Program.cs:268`); record field **names** snake_case (`rule_id`, `required_conditions`, `node_count`). The same concept has three vocabularies: `"Complex"` (solve) vs `"complex"` (inspect) vs `"Integers"` (families) |
| §75 | Structured errors (code/message/diagnostics/location/category/recoverable) | **Confirmed** | Live error envelope: `{"ok":false,"message":"...","diagnostics":[...],"elapsed":"..."}` — no code/category/recoverable. Three shapes on one CLI: `RunErrorDto`, a thinner `FileReadErrorDto` (no diagnostics/elapsed), and bare stderr for usage errors |
| §76 | Internal errors must not masquerade as user errors | **Confirmed** | `Limits.cs:66-69` converts **any** exception into `LimitResult.Uneval(ex.Message)` |
| §84/§86 | MathIR metadata and batch element structure | **Partial** | `compile_full` exposes `ir, parameters, result_type, target, mathir_version(2), exact` (`Evaluator.cs:399-405`). Missing parameter domains/types, result domain, precision policy, optimization policy. `CompilationTarget` (`Compilation.cs:6`) is dead code; the target is a hardcoded string in two places |
| §85 | Row-oriented batch input + validated shape error | **Confirmed** | `evalir_batch` accepts only a **flat** row-major list and errors with `"Values must be a multiple of N (the parameter count)."` (`Evaluator.cs:426-427`). The §135 acceptance call `evalir_batch(k, [[1,2],[3,4]], 80)` is **not supported** |
| §87/§88 | MathIR round-trip + exact-equivalence-after-optimization | **Partial** | `IrProgram.FormatVersion = 2` and deserialize revalidation exist (`Ir.cs:27,56-61,87`); no serialize→deserialize→evaluate differential test against direct symbolics |
| §113 | Protocol golden fixtures | **Confirmed missing** | No golden JSON files anywhere; `harness/lovelace.host.js` consumes `result.structured` ad hoc |
| NEW | **stdout is not pure JSON when a script calls print()** | **Confirmed (P0 protocol)** | The envelope is appended after printed text (`Program.cs:214-215`); `harness/lovelace.host.js:127` does `JSON.parse(stdout)` and reports "runner did not emit valid JSON" for a **successful** run. `Lovelace.Knowledge/Observation.cs:23` has the same exposure |
| NEW | Function registry entries lose provenance | **Confirmed** | `FunctionDto(f.Name, f.Parameters, f.IsBuiltin)` (`Program.cs:135`) drops `PluginName`/`Span` that `StateFunction` carries, so an agent cannot tell a core builtin from a Symbolics/MathIR/DSP one |
| NEW | `inv_full([[1,2],[3,4]])` rejected | **Confirmed** | `IsSymbolicMatrix` requires `elements.Any(e => e is Expr)` (`SymbolicsPlugin.cs:442-443`); a numeric caller cannot obtain the `det != 0` condition |
| NEW | Envelope duplication | **Confirmed** | Every field carries `display` **and** the nested subtree; the top level adds `typed`; the same text repeats in `result.display`, `result.typed` and `variables[0].display` |

### B.5 REPL / formatting / help / Studio (§45–§64, §102–§107, §112, §136, §144)

| # | Issue | Verdict | Evidence |
|---|---|---|---|
| §45 | `set pretty unicode` actually works | **Confirmed broken** | `ReplSession.cs:144` sets `_engine.UnicodeOutput`, but the result line (`:228-229`) and `vars` (`:240-241`) call the **static** `ValueFormatter.FormatTyped(value)` (unicode defaults to false). Only `print()` honours it |
| §46 | Host-consistent formatting context | **Confirmed** | Three mechanisms coexist: engine-aware (`SuiteEngine.FormatValueTyped`, `:97-102`), static `ValueFormatter` (Console, Run, interpolation, snapshots), and ambient `Real.DisplayDecimalPlaces`/`Natural.DisplayDigits`. There is **no** `FormatOptions`/`FormatContext` type. Studio uses the engine; Console and Run do not |
| NEW | `set display n` has **no effect** in the REPL | **Confirmed** | `ReplSession.cs:193-194` sets engine fields and `Nat.DisplayDigits` but never the `Real.DisplayDecimalPlaces` scope the formatter reads (process default 100, `Real.cs:43`); the per-statement precision scope is disposed before `PrintResult` runs |
| §47 | Unicode matrix (∞ √ π ≠ ≤ ≥) across results/arrays/records/conditions/matrices/vars/print | **Confirmed broken** | Only `print()` (`Interpreter.cs:1396`) and the engine helpers honour it |
| §48 | Canonical/Pretty/Debug strictly separated | **Already fixed** | `Printing.cs:307-352`; `PrintOptions(Mode, Unicode)`. Hosts can only reach `Pretty` (`ValueFormatter.cs:20-21`), so `Canonical`/`Debug` are unreachable except in Studio's inspect panel |
| §49 | Fraction rendering `-1/(2*(x + 1))` | **Already fixed** | Rational coefficient folds into the fraction (`Printing.cs:450-471`) |
| §50 | Radical rendering + algebraic complex forms | **Mostly fixed** | `sqrt(x)`/`1/sqrt(x)` (`:477-489`); cubic complex roots render as `2^(1/3)*(1/2*i*sqrt(3) - 1/2)` rather than `(-1 + i*sqrt(3))/2`; a bare `x^-1` renders as `x^-1`, not `1/x` |
| NEW | Pretty-print parentheses defect | **Confirmed** | Live `integrate_full(exp(-x^2), x)` displays `integrate(exp(((-x))^2), x)` — redundant parens make the only machine-readable form ambiguous between `exp(-(x^2))` and `exp((-x)^2)` |
| §51 | Polynomial ordering | **Already fixed** | Descending degree (`Printing.cs:601-637`) |
| §52 | Series ordering with nonzero centre | **Already fixed** | `CompareSeriesTerms` uses the expansion offset, O-term last (`:639-649`) |
| §53 | Matrix formatting preserves rank/shape | **Already fixed (display)** | `[[a, b], [c, d]]` deterministic; DSH still flattens to text (§27) |
| §54 | Conditional result formatting | **Already fixed** | Live `TransformResult(expression: 1, conditions: [x != 0])` |
| §55 | Debug printer | **Already fixed** | `DebugPrint` emits kind-annotated nodes (`Printing.cs:326-352`) |
| §56 | LaTeX output | **Not applicable this cycle (recommend defer)** | Zero `latex` references in the tree. Not required for A+; listed as optional Phase 12 |
| §57 | Every builtin documented | **Confirmed** | Enumerated from the live registry: **44 of 117 builtins** (37.6%) render `"(no summary registered)"` — 3 core (`inv_full`, `linsolve_full`, `ndims`), **16/16 DSP**, 25 Symbolics (all 11 elementary functions, `and/or/not`, the `assume_*` family, `assumptions`, `limit_left/right`, `solve_system_full`, `optimize_full`). The `assume` descriptor even advertises `assume(x > 0 and x < 10)`, which **fails to parse** |
| §58 | Metadata completeness test | **Confirmed missing** | No test sweeps the registry; `HelpServiceTests.cs` has 8 spot checks |
| §59 | Optional/variadic rendering | **Confirmed** | `Signature` renders `solve(f, x, domain)` (`HelpService.cs:145-146`). `BuiltinDescriptor.Variadic`/`MinArity` exist (`BuiltinDescriptor.cs:25-26`) and are **never set or read** |
| §60 | `help solve` detail | **Partially fixed** | Signature, summary, examples, returns, see-also, plugin — but no parameter descriptions and no structured default-domain line |
| §61 | Category aliases | **Confirmed** | `help symbolics`/`calculus`/`linear algebra`/`compilation` work; `help symbolic`, `help matrices`, `help mathir` fail (no alias table) |
| §62 | `funcs` filtering/search | **Confirmed** | Exact category only; live `funcs sol` → `"No category 'sol'."` — no substring/prefix search |
| §63 | Completion from descriptors | **Confirmed gap** | `EngineHost.cs:204-205` builds completions from name+parameters only, ignoring descriptor metadata |
| §64 | `inspect()` A+ | **Partial** | Live `inspect(x)` → `Inspection(type, domain, exact, free_symbols, node_count)`; `domain` is **Text**, and `canonical`/`pretty` are absent |
| §65 | `type()` naming convention | **Confirmed** | Live `type(1)` → `"Natural"` but `type(complex)` → `"complex"` (lowercase) |
| §66–§69 | Studio structured panels | **Confirmed gap** | `ValueResult(Kind, Display, Typed)` (`Dtos.cs:35`) carries **no structured payload**; no conditions/solutions/branches/provenance DTO exists. Only the symbolic inspector is structured (`SymbolicInspectResponse`, `Dtos.cs:22-29`) |
| §102–§107 | Docs | **Strong base, specific holes** | Doctests are real: `UsageDocumentationTests` executes every `lovelace/result` pair in the Symbolics README; `DocsSyncTests` pins README C# snippets against `UsageExamples.cs`. Holes: descriptor `Examples` are **never executed** (hence the broken `assume` example); no DSH protocol example; the solver-completeness limitation is documented **inaccurately** (README claims deg ≥ 4 always reports Unevaluated; only the zero-real-root case does) |
| §112 | REPL tests | **Confirmed missing** | No test drives `ReplSession`; `LineEditor` uses `Console.ReadKey` (`LineEditor.cs:50`) so redirected stdin throws — the REPL is untestable as written |

**Further new adjacent host issues:** `Lovelace.Run --text` prints pretty **JSON**, not text (`Program.cs:182-183`, contradicting its own usage line); `SessionRegistry.Sweep()` has no callers (dead eviction); the runner creates the plot directory unconditionally (`Program.cs:120`); `Lovelace.Console/README.md` documents a "Plugins" help category and `set precision/display` semantics the code does not implement; `vars` is REPL-only and cannot be scripted; there is no DSH tool-schema generator despite three doc comments claiming one.

### B.6 Concurrency, determinism, AOT, benchmarks (§22–§25, §89–§91, §120–§127, §147)

| # | Requirement | Verdict | Evidence |
|---|---|---|---|
| §22 | No leakage across engines | **Baseline present, unproven** | Per-engine gate + per-session precision; `ConcurrencyTests.cs` exists. `SymbolicsPlugin._assumptions` is unsynchronised instance state mutated inside `Run` |
| §24 | Cancellation | **Confirmed missing** | Zero tokens in the whole symbolic stack |
| §89 | Precision 64/256/1024/4096 | **Already fixed** | `precbench` + `precbench.Tests` in CI; `Real.WithPrecision` is scoped per statement |
| §90 | Native AOT | **Already fixed and verified** | AOT publish + native run succeeded this session; 19 libraries carry `IsAotCompatible`; source-generated JSON contexts in Run and Studio |
| §91 | Determinism | **Mostly fixed** | Canonical print deterministic; catalog name-ordered; conditions sorted by ordinal (`Assumptions.cs:76`). Gaps: no documented solution ordering (§92); `elapsed` is a unit-scaled string; `Group()`/`All` allocate per call |
| §120 | CI runs all critical suites | **Confirmed gap** | No MathIR-tagged job, **no AOT job**, no publish, no post-publish execution smoke, no doctest job. `Makefile test` omits Symbolics/Array/Complex/Dsp/Abstractions/Real entirely |
| §121 | CI publishes runnable artifacts | **Confirmed missing** | No publish step in `ci.yml`; `Makefile runner` exists but is never invoked by CI |
| §122 | Published-runner execution smoke | **Confirmed missing** | Not automated anywhere |
| §123 | Performance discipline | **Confirmed gap** | `symbench` covers construction/calculus/polynomial/compilation/precision (5 classes) but has **no baseline file**, is **not in the solution**, and is **not built by CI or make test**. No benchmark exists for help generation, printing, record construction, or JSON structured serialization |
| §124 | No hidden O(n²) formatting | **Not established** | No perf test; `FormatArray` is unbounded string concatenation (`ValueFormatter.cs:40-57`); the pretty printer is bounded at 64 terms for ordering (`Printing.cs:603`) |
| §125 | Help metadata startup cost | **Confirmed gap** | `Catalog()` rebuilds and re-sorts the entire registry on every `help`/`funcs` call; no caching or invalidation |
| §126 | Record field lookup | **Already adequate** | Linear scan; small records; benchmark before changing |
| §127/§128 | Print budget + blow-up diagnostics | **Confirmed gap** | No `MaxDepth`/`MaxNodes`/abbreviation mode; no structured budget diagnostics |

### B.7 Summary counts

| Verdict | Count |
|---|---|
| **Confirmed** (defect reproduced or proven) | **38** |
| **New adjacent issue** (discovered this audit) | **16** |
| **Already fixed** (verified, keep) | **22** |
| **Partially fixed** (extend, don't rebuild) | **9** |
| **Not applicable this cycle** (LaTeX) | **1** |


### B.8 Additional findings from the deep assumption/rewrite audit

| ID | Issue | Severity | Evidence |
|---|---|---|---|
| A1 | **Interval bounds are silently ignored and can produce a wrong affirmative answer**: `AskInterval` starts `bool ok = true` and only ANDs a bound when `Exprs.NumericToRational` parses it; unparseable bounds (`pi`, `2*pi`, Real, symbolic) leave `ok == true` and the method returns `True`. Live: `Im(real x) in [pi, 2*pi]` → **True** (Im = 0 is not in that interval) | **P0 correctness** | `Assumptions.cs:365-379`; `KernelHardeningTests.cs:222-232` codifies the wrong answer |
| A2 | **Contradictions missed for property/domain atoms and cross-kind inference**: `assume_positive(x); assume(x <= 0)` is **accepted**; `assume_negative(x); assume(x >= 0)` accepted; `assume_nonnegative(x); assume(x < 0)` accepted; `NonZero(x); x == 0` accepted; `symbol("x", integer); assume(x > 1/2); assume(x < 1)` accepted | **P0/P1** | `Negate` returns `null` for domain/expression/NonZero/Finite/Interval atoms (`Assumptions.cs:98-121`); `AskRelation` never consults property atoms. On such a set, `Ask(Positive(x)) = True` **and** `Ask(x > 0) = False` simultaneously |
| A3 | **Consequence of A2: unsound safe simplification.** Under `{Positive(x), x <= 0}`, `simplify(sqrt(x^2))` → `x` — "safe" mode applies a rewrite licensed by an inconsistent assumption set | **P0** | Live probe. This is precisely §21 "an unsatisfiable set must not be interpreted as empty assumptions" |
| A4 | **`Unsatisfiable.Ask(...)` returns `Unknown` for every query**, so a consumer cannot distinguish "no model" from "no information" | **P1** | Live probe; `AssumptionSet.Unsatisfiable` has empty atoms and no special case in `Ask` |
| A5 | **Conditional rules can report zero required conditions.** Live: `simplify_full(sqrt(x^2))` with `x` real → step `pow.sqrt-square-real`, `classification: Conditional`, `required_conditions: []`, while the realness guard lives only inside the `Precondition` lambda | **P1** | Violates §16 ("every Conditional rule that changes definedness exposes conditions") |
| A6 | **Off-by-one in `RefutesLower`** (`ab > qb` where the correct non-strict test is `ab >= qb`), masked only by the literal-negation fallback | **P2 latent** | `Assumptions.cs:273`; unreachable for equal bound tiers, observable across tiers |
| A7 | **Real-valued bounds silently lose all bound reasoning**: `assume(x >= 5.0real)` gives `x < 5.0` → False (containment) but `x < 5rat` → **Unknown**, `x > 4rat` → **Unknown** — `Exprs.NumericToRational` only handles Integer/Rational while `TermOrder.ToRational` handles Real | **P1** | Live probe; `Assumptions.cs:211-212,218` |
| A8 | **Trace "zero cost when trace is null" is false** — `ConditionBuilder` is materialised unconditionally (`Rewriting.cs:283`); in safe mode conditions are computed **twice** per fired rule | **P2** | `Rewriting.cs:192` doc claim vs code |
| A9 | **Budget accounting is wrong**: `budget.Steps++` precedes the structural-equality test, so it counts *passing matches*, not applied rewrites; an identity-replacement rule reports `budgetSteps = 1` with an unchanged expression. After exhaustion, all remaining groups still fully traverse and rebuild the tree | **P2** | `Rewriting.cs:279-281`, `Simplify.cs:56-62` |
| A10 | **`WithAssumptions` is not flow-local** despite its XML doc: it is a field save/restore on shared mutable `ExprContext.Assumptions` (`Context.cs:95-118`), so two concurrent scopes interleave and the last Dispose restores a stale set | **P1 concurrency** | Only a single-scope test exists (`ConcurrencyTests.cs:99-115`) |
| A11 | `ExprContext._pool` never evicts (unbounded retention for the context lifetime); `Expr._hash`/`_nodeCount`/`_isExact` are mutable non-volatile fields read by `StructuralHash` | **P2** | `Context.cs:122`, `Expr.cs:202-204` |
| A12 | Rule coverage is tiny: **9 rules in 5 groups**; `RuleClassification.DomainSpecific/Approximate/OptimizationOnly` are defined but unused; `FunctionDefinition.Rules` is dead; `RewriteRuleRegistry.Register` has no uniqueness check | **P1 for §17** | `Simplify.cs:117-223`, `Functions.cs:22-38` |
| A13 | Falsification skips its own failures: `catch (Exception) { continue; }` in `FalsificationTests.cs:46-49` means a rule that is wrong *by throwing* is never falsified | **P2** | Static read |
| A14 | Repo hygiene: four `testhost` hang dumps (~719 MB) and `Sequence_*.xml` files sit in the working tree | **P3** | `Lovelace.Symbolics.Tests/TestResults/...` |

---

## C. Final semantic contracts (freeze before fan-out — §141)

All types are **immutable records**, Native-AOT safe (no reflection), constructed by the kernel and
projected by Suite. Field names are the contract; the JSON wire form uses the same name.

`~csharp
// ---- status / completeness / exactness: one taxonomy, one spelling -------------------
public enum SolveStatus { Solved, Partial, NoSolutions, Unevaluated, BudgetExceeded }
public enum Completeness { Complete, Partial, Unknown }
public enum SolutionExactness { Exact, AlgebraicExact, Approximate, ParametricExact }

// ---- one solution, with everything the kernel knows ---------------------------------
public sealed record Solution(
    Expr Value,
    AssumptionSet Conditions,     // conditions of THIS branch only
    int Multiplicity = 1,
    SolutionExactness Exactness = SolutionExactness.Exact,
    bool Representable = true);   // false => counted but not representable (complex algebraic)

public sealed record SolutionFamily(
    Expr Template, Symbol Parameter, Expr Period,
    ParameterDomain ParameterDomain,       // typed, not a string
    AssumptionSet Conditions,
    SolutionExactness Exactness);

public sealed record SolveResult(
    SolveStatus Status,
    Symbol Variable,
    Domain Domain,                          // first-class Domain value, not text
    Completeness Complete,
    IReadOnlyList<Solution> Solutions,
    IReadOnlyList<SolutionFamily> Families,
    AssumptionSet CommonConditions,         // intersection of every branch's conditions
    int RepresentedCount,
    int UnrepresentedCount,
    string? UnrepresentedReason,
    IReadOnlyList<Diagnostic> Diagnostics);

// ---- system solving: structural bindings, never "x = ..." ---------------------------
public sealed record Binding(string Name, Expr Value);
public sealed record SystemSolution(
    IReadOnlyList<Binding> Bindings, AssumptionSet Conditions,
    SolutionExactness Exactness);
public sealed record SystemSolveResult(
    SolveStatus Status, Domain Domain, Completeness Complete,
    IReadOnlyList<SystemSolution> Solutions,
    IReadOnlyList<Diagnostic> Diagnostics);

// ---- other rich results: one field vocabulary (§41) --------------------------------
public sealed record RewriteStep(string RuleId, RuleClassification Classification,
    Expr Before, Expr After, AssumptionSet Conditions);              // already shipped

public sealed record TransformResult(Expr Original, Expr Expression, AssumptionSet Conditions,
    IReadOnlyList<RewriteStep> Steps, TransformStatus Status, string? BudgetKind,
    Expr? PartialResult, IReadOnlyList<Diagnostic> Diagnostics);

public sealed record LimitResult(LimitStatus Status, bool Exists, Expr? Value,
    Expr? Left, Expr? Right, AssumptionSet Conditions,
    SolutionExactness Exactness, IReadOnlyList<Diagnostic> Diagnostics);

public sealed record IntegrationResult(IntegrationStatus Status, Expr Expression,
    AssumptionSet Conditions, bool Verified, string? VerificationMethod, string? Method,
    SolutionExactness Exactness, IReadOnlyList<Diagnostic> Diagnostics);

public sealed record OptimizationResult(Expr Original, Expr Optimized,
    long EstimatedCostBefore, long EstimatedCostAfter, IReadOnlyList<string> Transformations,
    string Target, string Policy, IReadOnlyList<Diagnostic> Diagnostics);

public sealed record ParameterInfo(string Name, Domain Domain);
public sealed record CompilationResult(string Ir, int IrVersion, string Target,
    IReadOnlyList<ParameterInfo> Parameters, Domain ResultDomain,
    string PrecisionPolicy, string OptimizationPolicy, bool Exact,
    IReadOnlyList<Diagnostic> Diagnostics);

public sealed record Inspection(string Type, Domain Domain, bool Exact,
    IReadOnlyList<string> FreeSymbols, int NodeCount, string Canonical, string Pretty,
    IReadOnlyList<string> RelevantAssumptions);

// ---- errors: one taxonomy, no message matching (§75) -------------------------------
public enum ErrorCategory { ParseError, DomainError, UnsupportedOperation,
    BudgetExceeded, NoSolution, TypeMismatch, InternalInvariantFailure }
public sealed record Diagnostic(string Code, ErrorCategory Category, string Message,
    bool Recoverable, SourceSpan? Location, IReadOnlyList<Diagnostic> Details);
`~

`Domain` is the **kernel** enum (`Integer, Rational, Real, Complex`) promoted to a Suite payload
value; `MathDomain` and `SolveDomain` are folded into it (one lattice, one spelling). Status
constants live in exactly one place and are documented as a stable contract (§42).

---

## D. Solver completeness contract

| Status | Applies exactly when |
|---|---|
| `Solved` | Every root over the **requested domain** is represented in `Solutions`/`Families`; `Complete = Complete`; the represented count equals the exact root count (with multiplicity). A factor whose completeness cannot be established forces `Partial`/`Unevaluated` |
| `Partial` | A non-empty exact subset is returned **and** the unrepresented remainder is known and counted: `UnrepresentedCount > 0`, `UnrepresentedReason` set, `Complete = Partial`. Never spelled `Solved` |
| `NoSolutions` | The solution set over the requested domain is provably empty — **including** the case where every candidate was removed by a domain/denominator condition (status is re-derived **after** filtering) |
| `Unevaluated` | Nothing sound can be returned: no solver for the structure, `0 = 0`, unrepresentable complex algebraic roots with no partial subset, unsupported domain value |
| `BudgetExceeded` | A budget (steps, nodes, solutions, time, cancellation) stopped the search; carries `PartialResult` when one exists |

**Hard invariants (each gets a dedicated test):**

1. `Status == Solved` implies `Complete == Complete` (no silent remainder).
2. `Status == Solved` implies `Solutions.Count + Families.Count > 0`.
3. For deg ≥ 4 complex-domain requests, if any factor has a non-real root the result is **never** `Solved` (today it is).
4. The requested domain is preserved verbatim and never silently substituted.
5. Every solution carries its own conditions; `CommonConditions` is derived, never a replacement.
6. Multiplicity survives factor → root projection.
7. Solution ordering is documented and deterministic (real: ascending; complex: canonical lexicographic on the exact representation; families: principal branch first).

**Root-representation decision (§12):** implement a **real** `AlgebraicRoot` upgrade only — extend
`RootOfExpr` with an explicit domain and an isolating rational interval — and return structured
`Partial`/`Unevaluated` for non-real algebraic roots. Full complex algebraic root objects are
**out of scope** for this cycle (they would destabilise the solver core); an honest structured state
is the A+ requirement, an incomplete answer is not.

---

## E. Domain contract

| Domain | `symbol(name, d)` | `assume_*` | `solve(..., d)` | `type/inspect` |
|---|---|---|---|---|
| `integer` | assume `Domain.Integer` | — | **Reject**: `solve(): currently supports domains real and complex; got integer.` (`DomainError`) | `Integer` |
| `rational` | assume `Domain.Rational` | — | **Reject** with the same structured `DomainError` | `Rational` |
| `real` | assume `Domain.Real` | supported | supported; result `Complete = Complete` when all roots are real | `Real` |
| `complex` | assume `Domain.Complex` | supported | supported; **default**; deg ≥ 4 non-real implies `Partial`/`Unevaluated` | `Complex` |
| (omitted) | — | — | defaults to `complex`, and the result records `complex` | — |

The supported-domain set is **metadata**, not duplicated magic checks (`BuiltinDescriptor` carries
`SupportedDomains`); the rejection message is generated from it (§79). The domain lattice also gains
**conflict detection on Add**, so `symbol(x, integer)` plus a contradicting relation is caught
rather than silently accepted (finding A2).

---

## F. Structured protocol schema (representative JSON)

`~json
{
  "protocol_version": 1,
  "symbolic_format_version": "lovelace-sym 1",
  "mathir_version": 2,
  "ok": true,
  "result": { "kind": "Record", "type": "SolveResult", "display": "…", "typed": "…",
    "structured": { "kind": "SolveResult", "fields": [
      { "name": "status", "value": { "kind": "Enum", "type": "SolveStatus", "value": "Partial" } },
      { "name": "domain", "value": { "kind": "Domain", "value": "complex" } },
      { "name": "complete", "value": { "kind": "Enum", "type": "Completeness", "value": "Partial" } },
      { "name": "solutions", "value": { "kind": "Array", "shape": [2], "elements": [
        { "kind": "Record", "type": "Solution", "fields": [
          { "name": "value", "value": { "kind": "Symbolic",
              "pretty": "x^2 + 1", "canonical": "(add (pow (sym x) (int 2)) (int 1))",
              "domain": "complex", "exact": true, "node_count": 5, "free_symbols": ["x"] } },
          { "name": "conditions", "value": { "kind": "Array", "shape": [1], "elements": [
              { "kind": "Symbolic", "pretty": "x != 0", "canonical": "(ne (sym x) (int 0))" } ] } },
          { "name": "multiplicity", "value": { "kind": "Integer", "value": "1" } },
          { "name": "exactness", "value": { "kind": "Enum", "value": "AlgebraicExact" } } ] } ] } },
      { "name": "unrepresented_reason", "value": { "kind": "Text",
          "value": "complex algebraic roots not representable" } },
      { "name": "diagnostics", "value": { "kind": "Array", "shape": [0], "elements": [] } } ] } },
  "variables": [], "functions": [], "plot": null, "elapsed": "12.3 ms"
}
`~

Also specified and golden-tested: **array** (`shape` always present, rank-N row-major), **complex**
(`{kind:"Complex", re, im, exact}`), **rational** (`{numerator, denominator}`), **null**
(`{kind:"Null"}`, distinct from `Void`), **transform** (`steps[]` with
`rule_id`/`classification`/`before`/`after`/`required_conditions`), **system solve**
(`bindings:[{variable, value}]`), **compiled kernel** (`parameters:[{name, domain}]`,
`result_domain`, `ir_version`, `precision_policy`, `optimization_policy`), **error**
(`{code, category, message, recoverable, location, diagnostics}`).

Recursion invariant (§71): every structured field is one of
`scalar | symbolic | array | record | domain | null` — **never** a bare string chosen because the
serializer lacked a case.

Additional protocol rules frozen here: **stdout carries the envelope and nothing else** (script
`print()` output is captured and returned as a structured `output` array — finding NEW/P0);
`--text` emits genuine text; the envelope carries the three version fields; and `elapsed` is
exposed both as a human string and as structured `{value, unit}`.


---

## G. Rewrite failure contract

| Situation | Semantics |
|---|---|
| Pattern does not match | `RuleApplicability.NotApplicable` — a **value**, not an exception |
| Precondition provably true | `Applicable` |
| Precondition true only under conditions | `ApplicableWithConditions` (carries them; safe mode refuses unless already provable) |
| Precondition unknown | `Unknown` → safe mode does **not** fire; full mode fires **and records the conditions** |
| Rule implementation throws | **Propagates** — a defect, not "not applicable". Only a dedicated narrow `RuleEvaluationException` (for genuinely expected states such as numeric overflow in a precondition probe) is caught |
| `ConditionBuilder` throws | **Propagates** (today it is swallowed at `Rewriting.cs:283` via the precondition guard) |

`RewriteRule.Precondition` becomes `Func<Match, ExprContext, RuleApplicability>`; the runtime
precondition (the match) stays separate from the mathematical precondition. Conditions expressed
*inside* a precondition lambda must be **declared** via `DeclaredConditions`/`ConditionBuilder`,
so that a `Conditional` step never reports `required_conditions: []` (finding A5). Optional
`IRewriteDiagnostics` records `rule_id / match_state / precondition_state / rejection_reason` only
when tracing is enabled. Invariants tested: universal rules have no conditions; a conditional rule
that changes definedness exposes conditions; safe simplification never applies an unresolved
conditional rule; full transformation preserves every condition; unexpected exceptions fail the
test.

---

## H. Assumption semantics

The kernel bound lattice is **already sound for strict lower bounds and all upper bounds**
(324/324 reference agreement, 324/324 pair contradiction agreement, all five §20 examples detected).
The cycle **completes and exposes** rather than rewrites:

1. **Fix the real correctness defect (A1).** `AskInterval` must not return `True` when a bound
   cannot be evaluated; unevaluable bounds ⇒ `Unknown`. Add the missing interval-bound test (the
   current test asserts the wrong answer).
2. **Close contradiction gaps (A2).** `Add` must consult the full lattice: property ↔ relation
   inference (`Positive(x)` vs `x <= 0`), `NonZero` vs `== 0`, domain/integrality
   (`x ∈ Integer` vs `1/2 < x < 1`), and multiple domain atoms. A contradictory set must be
   **rejected or become Unsatisfiable**, never silently accepted.
3. **Make `Ask` honest about `Unsatisfiable` (A4)** — return a distinct
   `Tristate.Unsatisfiable` (or throw a structured diagnostic) rather than `Unknown`.
4. **Unify bound tiers (A7).** `NumericToRational` must accept Real constants, matching
   `TermOrder.ToRational`; fix the `RefutesLower` off-by-one (A6) as defence in depth.
5. **Expose the reasoning to the language (§18).** Add a relation-folding rule so `simplify(x < 5)`
   under `assume(x >= 5)` returns `False`; keep the relation honest (never fold to a Boolean on
   `Unknown`).
6. **Fix `DomainOfSymbol` (A2)** to merge domain atoms by the lattice (narrowest wins) instead of
   first-match over a string-sorted array.
7. **Make `WithAssumptions` genuinely flow-local (A10)** — use `AsyncLocal` or a passed context —
   and test two concurrent scopes.
8. **Propagate `Unsatisfiable`** into rewrite branches, solver branches, Piecewise guards and
   structured diagnostics; never treat it as empty (§21).
9. **Assumption isolation contract** formally tested across concurrent engines (assumptions,
   precision, output writer, plugin state, `Exprs.Current`).
10. **Reentrancy:** nested `EvaluateAsync` on the same engine raises a structured
    `ReentrancyNotSupported` diagnostic instead of blocking on the semaphore.

---

## I. Host formatting contract

One `FormatContext` (immutable record: `Unicode`, `DisplayDecimalPlaces`, `Mode`) owned by the
engine; **every** host renders through `engine.FormatValue/FormatValueTyped/FormatExpression` (REPL
result, `vars`, `print()`, interpolation, Studio, `Run --json/--text`, doctests). The static
`ValueFormatter.Format*` overloads remain for library callers but must be given an explicit
context. `set pretty unicode|ascii` and `set display n` take effect in the next statement's
rendering, including inside records, arrays, conditions and matrices. ASCII stays the deterministic
default. A matrix test drives the REPL command surface (§112) by injecting a script source into a
headless session (`ReplSession` gains a `TextReader`/`TextWriter` seam so `Console.ReadKey` is
not required).

---

## J. Backward compatibility

The project is pre-1.0; **correctness wins**. Intentional breaks, each documented in the changelog:

| Break | Why |
|---|---|
| `SolveResult.status` may now be `Partial`; deg ≥ 4 complex-domain solves stop claiming `Solved` | prevents incorrect mathematics (§4, §131) |
| `SolveResult.solutions` becomes `Solution[]` (objects, not bare exprs); a convenience `values` vector is kept for casual use | per-solution conditions/multiplicity/exactness (§5, §8) |
| `SolveResult.domain` changes from `"Complex"` text to a Domain value | §11 |
| `solve(…, integer|rational)` now **errors** instead of returning complex roots | §10 |
| `solve_system_full` `assignment` (strings) → `bindings` (records) | §34 |
| Condition arrays may contain structured sentinel records instead of `"unsatisfiable"` | §96 |
| `Unwrap(Value.Void)` no longer throws; it maps to `null` | §32 |
| `Run` captures script `print()` output into the envelope instead of interleaving stdout | NEW/P0 protocol fix |
| `--text` emits text, not pretty JSON | its own documented contract |

Preserved: all convenience APIs (`simplify`, `solve`, `limit`, `integrate`, `optimize`,
`compile`) keep working and keep their human-facing honesty; `solve()` returns `Unevaluated`
prose rather than an incomplete vector; canonical print is frozen at `lovelace-sym 1` (additive,
versioned changes only).

---

## K. Work DAG

`~text
Phase 0  FREEZE CONTRACTS (this document, sections C–J)          <- gate: approval
   |
   +-- Phase 1  Kernel status/result types + Domain promotion            [A,B]
   +-- Phase 2  Solver completeness + domain rejection + ordering        [A,B]
   |              (blocks 3, 6, 7)  incl. N1/N2/N3/N4/N7
   +-- Phase 3  Assumption correctness (A1,A2,A3,A4,A6,A7) + exposure    [C]
   +-- Phase 4  Rewrite applicability protocol + condition declarations   [D]
   +-- Phase 5  Structured serializer (all kinds, shape, canonical)      [E]
   |              (depends on 1) incl. stdout purity + envelope version
   +-- Phase 6  Record/null/payload round-trip + Binding type            [F,G]
   +-- Phase 7  System-solver structural bindings                        [G]
   +-- Phase 8  FormatContext + REPL/Unicode/precision convergence       [H]
   +-- Phase 9  Help metadata completion + aliases + search              [I]
   +-- Phase 10 Studio panels + MathIR metadata + batch DX               [J,K]
   +-- Phase 11 Tests: property/differential/falsification/concurrency/golden [L]
   +-- Phase 12 CI: AOT job + publish + execution smoke + doctests       [M]
   +-- Phase 13 Docs/doctests incl. descriptor-example execution         [N]
   +-- Phase 14 Benchmarks (symbench baseline + new rows)                [O]
   |
   +-- Phase 15 Adversarial post-implementation audit (sections 143–147)
`~

Parallel fan-out is allowed **only after** Phase 0 approval, and only for Phases 3–14; Phases 1–2 are
on the critical path. Ownership boundaries prevent two agents redefining the same contract.

---

## L. Test matrix (A+ acceptance)

| Category | Content |
|---|---|
| **Solver regression (§109)** | linear exact; quadratic real; quadratic complex; cubic real; cubic complex; quartic all-real; quartic no-real; **quartic mixed real/complex (must never be Solved)**; reducible high-degree mixed factors; `(x+1)^2 - 4` (both roots); `(x+1)^3 == 8` (all three or honest Partial); `sin(x) == 2` under Complex; `exp(x) == -1` under Complex; rational pole exclusion; **all candidates excluded ⇒ NoSolutions**; parametric trig families; unsupported domain values (integer/rational ⇒ error); multiplicity; completeness flag/status; ordering determinism; system-solver truncation reporting |
| **Assumptions (§18–§21)** | the 324-query reference matrix as a generated theory; 324 pair contradictions; the `x >= 5` cases (currently untested); missed contradictions C1–C4; interval-bound soundness (A1, replacing the test that asserts the wrong answer); Real-bound tiers (A7); flagging of unsatisfiable propagation into rewrite/solve/piecewise; relation folding to True/False through `simplify` |
| **Rewrite (§111)** | unexpected precondition exception propagates; throwing `ConditionBuilder` propagates; conditional rule does not fire in safe mode when Unknown; fires when proven; full mode reports conditions **including precondition-encoded ones**; contradictory conditions ⇒ Unsatisfiable (must actually be reachable); budget exceeded reported with correct step accounting; trace disabled ⇒ zero steps but conditions preserved; trace enabled ⇒ stable rule ids; universal rules have no conditions; rule-id uniqueness |
| **Structured results (§110)** | per-solution conditions; families' own conditions; null fields survive; nested records; records in arrays; arrays in records; `Record→Array→Record` and `Record→Vector→Record`; domains stay Domain values; canonicals survive serialization; record deep-equality helpers; deep round-trip through plugin unwrap/re-wrap |
| **Protocol golden files (§113, §114)** | versioned fixtures for Symbolic, Vector, Array, Record, nested Record, SolveResult, TransformResult, LimitResult, SystemSolveResult, CompilationResult, Complex, null optional field, error envelope, and a `print()`-plus-result run proving stdout purity |
| **Property/falsification (§17, §117, §118)** | randomized rule sampling in interior / near-zero / near-pole / near-branch-cut / near-discontinuity / large-magnitude regions, real and complex; conditional rules sampled both satisfying and violating; falsification must **fail** on a rule that throws |
| **Differential (§115)** | SymPy as a **tests-only** oracle under explicitly matched domains/branches: simplification identities, factorization, roots, differentiation, selected integration, limits, matrix ops. Oracle mismatches are triaged, never blindly trusted |
| **Metamorphic (§116)** | d/dx ∫f = f; substitute(solution) = 0; A·A⁻¹ = I under conditions; evalIR(compiled) = eval(symbolic); expand(factor(p)) = p; cancel preserves value off poles |
| **Concurrency (§22, §23)** | engine A `x>0` vs engine B `x<0` concurrently; two concurrent `WithAssumptions` scopes; no `Exprs.Current`/assumption/precision/writer/plugin leakage; nested evaluation yields a structured diagnostic, not a deadlock |
| **MathIR (§87, §88)** | symbolic→optimize→compile→serialize→deserialize→evaluate vs direct symbolic evaluation at 64/256/1024 digits, including Piecewise; row-oriented batch shape validation |
| **REPL (§112)** | help / help solve / funcs symbolics / set pretty unicode|ascii / set display n / vars / structured rendering / matrix rendering / errors — asserting **output text** |
| **Doctest (§103, §104)** | README pairs (already present) **plus** execution of every `BuiltinDescriptor.Examples` string, so the broken `assume` example is caught |
| **AOT (§90, §120)** | publish `Lovelace.Run` (and Console) as AOT in CI; run the published binary against the §122 scenarios and validate the JSON envelope |
| **Performance (§123–§127)** | thresholds vs the recorded baseline (section M) |
| **Semantic adversarial (§146)** | poles, branch cuts, domain boundaries, contradictory assumptions, incomplete algebraic roots, singular matrices, zero denominators, unsolved limits, unsupported domains, budget exhaustion |

---

## M. Benchmark plan

**Baseline to record first** (Phase 14 start, Release, warmed, 3 iterations): the existing `symbench`
classes (construction/canonicalisation, diff, Jacobian/Hessian, sparse bivariate multiply, Gröbner
Cyclic-3, MathIR scalar/batch/tree, precision 64/256/1024) — currently **no baseline file exists**
and symbench is not in the solution or CI.

**Rows to add** (§123): expression construction, canonicalization, `simplify` safe,
`simplify` full (trace off / trace on), `diff`, `factor`, `solve`, pretty print, canonical
print, `RecordValue` construction, member access, JSON structured serialization, MathIR scalar,
MathIR batch, **help catalog generation**.

**Thresholds:** no row may regress more than **5 %** in mean or **10 %** in allocations versus the
recorded baseline for the fast paths above. The newly instrumented rows establish their own baseline
in the same run and are reported as deltas, not gates, in the first cycle. Structured serialization
must remain **O(n)** in result size with no quadratic condition-union or string-concatenation
behaviour (§124). Help catalog generation must be **cached**, ≤ 1 ms warm, and must not run per
statement (§125).

---

## Summary of what the cycle will close

The audit found that the symbolic stack is **substantially stronger than a typical "MVP+"** — the
Sturm-based real root isolation, the bound-condition lattice for strict/upper bounds (324/324
verified), the canonical/pretty/debug printer split, the definedness-conditioned rewriter, the
doctest harness, the MathIR validator and Native AOT all work and were verified live. The gaps are
concentrated and architectural, not diffuse:

1. **The solver's status is not a claim about the requested domain** — three independent paths
   (`HighDegreeRoots`, principal-branch `SolveInverse`, domain-blind elementary inverses) return
   apparently complete answers that are not, and one of them (`(x+1)^2 - 4`) is simply wrong.
2. **Per-solution semantics are destroyed at the Suite boundary** — conditions unioned, multiplicity
   and exactness dropped, domains stringified, assignments encoded as prose.
3. **The structured protocol stops at Record and Vector** — arrays, symbolics, domains, complexes and
   nulls degrade to display text, the envelope is unversioned, and `print()` corrupts stdout.
4. **Two defect-masking seams remain** — `catch (Exception) ⇒ not applicable` in the rewriter and
   the same shape in the limit/integration evaluator, contradicting the project's own doctrine.
5. **The assumption engine has two real soundness holes** — interval bounds ignored (wrong `True`)
   and cross-kind contradictions missed, which can license an unsound "safe" simplification.
6. **Host formatting is incoherent** — `set pretty unicode` and `set display` are silently
   ignored by the REPL, and Console/Run bypass the engine's formatting context.
7. **37.6 % of builtins are undocumented**, descriptor arity metadata is dead, and descriptor
   examples are never executed.
8. **CI does not prove any of the guarantees** — no AOT job, no published-artifact smoke, no MathIR
   job, no doctests, and the symbolic benchmark harness is outside the solution.

---

**Alignment complete. The proposed cycle closes the remaining semantic, DX, serialization, solver, concurrency, and agent-interface gaps described above. Please approve or request changes before implementation begins.**
