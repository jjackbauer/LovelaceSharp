# A+ Cycle 2 — status and handoff

> Written at the end of goal round 4/5. Companion to `a-plus-cycle-2-alignment.md` (the frozen
> plan) and `a-plus-convergence-alignment-plan.md` (the Cycle-1 contracts). Everything below is
> uncommitted work on top of `f37f1ce`.

## Verified state (last full run)

| Check | Result |
|---|---|
| `dotnet build LovelaceSharp.slnx -c Release` | 0 errors |
| Full test matrix | **1935 passed / 0 failed** (Real non-Heavy included) |
| Native AOT publish | succeeded (`out/aot/Lovelace.Run.exe`) |
| Published-runner smoke | pass — solve/partial, stdout purity, jacobian shape, error envelope, `elapsedTime`/`timings`, and cancellation (`code=Cancelled, category=BudgetExceeded`, partial output + variables) |

## Closed (with the evidence that proves it)

- **N1** unary-minus precedence: `-2^2` is −4, `subs(-x^2,x,3)` is −9, `-1..2` still a range, `1..10^2` still `(1..10)^2` — `Lovelace.Suite.Tests/SignPrecedenceTests.cs`.
- **Typed catches**: zero `catch (Exception)` left in `Lovelace.Symbolics` (grep); `ExpectedFailureBoundaryTests` proves unexpected exceptions propagate.
- **Condition leaves**: `UnsatisfiableConditions` / `FiniteCondition` / `PredicateCondition` / `DomainCondition` / `IntervalCondition`; no string or raw C# record reaches a payload. §21 unsatisfiable propagation into transforms.
- **Argument validation (N2/N4)**: `diff(): argument 2 must be a symbolic variable; got Natural.`; `evalir_batch()` rejects row-width mismatch instead of silently re-flowing.
- **Shared serializer**: `Lovelace.Suite/StructuredProjection.cs`, consumed by both `Lovelace.Run` and Studio.
- **Studio**: structured payload on evaluate/inspect/variables/timings; UI renders records, matrices from `shape`+`elements`, provenance from `TransformResult.steps`, MathIR, conditions/solutions; the display-text regex/re-evaluation path is deleted; `PUT /api/format` + a UI toggle drive `engine.UnicodeOutput`. *Caveat: verified by syntax check + server-side tests, never in a browser.*
- **Cancellation**: token through `EvaluateAsync`, the builtin contract and the kernels (Gröbner, simplify, rewrite walk, expand, diff, root isolation, integration, MathIR batch, statement loop); `--cancel-after <ms>` returns a structured `Cancelled` with the partial result.
- **Reentrancy**: `ReentrancyNotSupportedException` instead of a deadlocked gate; diagnostics/timings reset moved inside the gate.
- **Assumptions**: `WithAssumptions` is `AsyncLocal`-scoped.
- **P1**: 8 (`inspect` uniform record incl. canonical/pretty/shape/rank/element domain/assumptions), 9 (`type()` = ValueKind name, `Domain` for domains), 10 (MathIR parameter domains/result domain/policies + validated batch), 11 (LimitResult conditions/exactness/one-sided conditions; IntegrationResult method/verification_method; OptimizationResult transformations/policy), 12 (`elapsedTime` + per-statement `timings`), 14 (rich argument/contradiction messages), 16 (REPL `TextReader`/`TextWriter` seam; `Lovelace.Console.Tests` 15 tests), 17 (`PrintBudget` + `--print-budget`, structured truncation diagnostic).
- **18 (partial)**: `symbench` in the solution/Makefile/CI; baseline at `benchmarks/symbench-baseline.md` + `benchmarks/raw-baseline.txt`; BDN reports moved to `benchmarks/bdn-reports/`.

## Adversarial audit (§143) — what has actually been run

Method: the **compiled** artefacts, not source reading — the published Native AOT runner
(`out/aot/Lovelace.Run.exe`) for the runner personas, and the running Studio host
(`dotnet run --project Lovelace.Studio --urls http://127.0.0.1:5203`) for the Studio persona.

| Persona | State | Evidence / outcome |
|---|---|---|
| Mathematician + fuzz input generator | **done** | 12 adversarial scripts: degree-5 solve (`Partial`, honest), `integrate(exp(-x^2))` (unevaluated, correct), `factor(x^4+1)` (irreducible), 200-deep nesting, self-assignment, contradictory assumptions |
| DSH agent | **done** | envelope versions, stdout purity, `shape [2,2]` row-major arrays, structural error envelope, cancellation returning `Cancelled` + partial output/variables, `--omit-variables` |
| Studio user | **done** | live host: session → evaluate → poll recovered `SolveResult`, `Solved`, `shape [2,2]`… i.e. status/domain/solutions/exactness/timings **from structure alone** |
| C# API consumer | **partial** | main friction found: a bare `new SuiteEngine()` has no `evalf`/`solve_full` — plugins must be loaded, and `"Unknown function 'evalf'"` gives no hint that a plugin is missing |
| Concurrency tester | **partial** | two-concurrent-`WithAssumptions`-scopes isolation and nested-scope restoration now tested; no multi-engine parallel stress or cross-engine leak test |

### Defects the audit found (all fixed, with tests)

1. **Unterminated string literal silently accepted** as a Text value — a typo became data. Now
   `InvalidInput`/`ParseError`. (fuzz generator)
2. **A framework `ArgumentException` leaked** out of numeric `log`/`pow` where the kernel's typed
   `EvaluationException` is required — which meant the narrowed catches from the P0 typed-catch work
   could be bypassed by a domain edge. (falsification gate + fuzz)
3. **`1/3 - 1/3` reported `exact: false`** — zero carried the subtraction's negative exponent.
   (preservation test, not the plan)
4. **Budget exhaustion and division by zero surfaced as `InternalInvariantFailure`/`recoverable:false`**
   — now `BudgetExceeded` and `DivisionByZero`, both recoverable. (fuzz + agent persona)

### Environment caveats to carry into the final report

- The first audit run used a **stale AOT binary** and produced two false defects; the runner must be
  re-published before auditing. Recorded because the mistake is easy to repeat.
- The Studio UI (JS/CSS panels) is verified by syntax check and this API round-trip — **never in a
  browser**.
- Benchmark numbers come from ShortRun jobs on a shared workstation; allocations are trustworthy,
  means are direction-only.

## Final status (Cycle-2 rounds 54–61 — read this before the list below)

**Everything in the plan that could be completed without a human decision is done.** The list
further down is chronological and partly superseded; this block is the accurate position.

| Area | State |
|---|---|
| P0 blockers (§2 items 1–6) | **closed** |
| P1 items (§3 items 7–17) | **closed** |
| P2 item 18 (benchmarks) | **closed** — baseline, 11/11 deltas, both named properties measured |
| P2 item 19 (assumption cost) | **measured, deliberately not restructured** (see 3b; item 26 says measure first) |
| P2 item 20 (fail-don't-skip + breadth) | **gate closed** — and it found the leaked-`ArgumentException` defect |
| P2 item 21 (Piecewise assumption-aware guards) | **closed** (round 58 — `EvaluateCondition` consults the lattice first; see 3c for the dead-end that preceded it) |
| P2 item 22 (record schema registry) | **closed** (round 55 — 17 schemas + a drift test over a live corpus) |
| P2 item 23 (rule→proof bridge) | **closed** (round 54 — 9 obligations + a registry-equality invariant) |
| P2 item 24 (LaTeX) | **deferred** by the alignment doc itself |
| P2 item 26 (sub-`Ask` hot paths) | **measured** — record field lookup 4.9 ns, allocation-free; no change warranted |
| §82 deep structural equality | **closed** (round 61 — `Lovelace.Suite.StructuralEquality`) |
| Gates | **all five green**: suites 1971/0, AOT published, smoke 10/10, benchmarks complete, audit all five personas |

**What is genuinely left, and why it is not mine to close:**

1. **Item 25 — decided: `--omit-variables` shipped, `--omit-display` declined.**
   The alignment doc conditions this item: implement the flags *"only if they do not fragment the
   protocol; the protocol must remain single-sourced"*. `--omit-variables` mirrors the existing
   `--omit-functions` contract exactly (same shape, default behaviour unchanged), so it ships.
   `--omit-display` would drop `display`/`typed` — fields the DSH protocol documents as part of
   *every* value and that the AOT smoke asserts on — so it is the fragmentation case the item
   excludes. **Declined on the item's own condition, not skipped.** A maintainer can override by
   saying so; the reasoning is recorded here so the choice is reviewable rather than implicit.
2. **A commit.** 70+ changed paths, all verified, none committed. The largest remaining risk in the
   cycle is an accident, not a defect.
3. **Optional polish** — three xUnit analyzer warnings in tests (`xUnit2009` ×2 in
   `PrintBudgetTests`, `xUnit2031` in `RewriteProtocolTests`), and the stale item below this
   block.

**Defects found by attacking the system rather than by working the item list** (all fixed, all
tested): the leaked `ArgumentException` that could bypass the P0 narrowed catches; the
unterminated string literal silently accepted as data; `1/3 - 1/3` reporting `exact: false`; and
budget/division-by-zero surfacing as unrecoverable internal failures. Four of the cycle's most
consequential fixes came from the audit and preservation testing, not the plan.

## Remaining (historical, chronological — partly superseded by the block above)

1. **Item 18 finish.** Record the *new* rows' numbers (simplify safe/full trace off/on, diff, factor, solve, pretty/canonical print, structured records + JSON projection, MathIR scalar/batch, help catalog generation, record field lookup) into `benchmarks/symbench-baseline.md`. §3 of that file currently says they are smoke-measured only, so the 5 % mean / 10 % allocation threshold gate has nothing to compare against. Also confirm help catalogs are cached (they were rebuilt per call at `HelpService.Catalog()`).
1b. **Item 7 — exact-rational payload: DONE, verified on both paths.** (An earlier note here
   claimed a host-vs-library inconsistency in `evalf`; that was **my test's error, not a defect** —
   the probe showed `evalf(1/3, 50) => THREW InvalidOperationException: Unknown function 'evalf'`
   because the test built a bare `new SuiteEngine()` with no plugins loaded, while the runner loads
   Dsp + Symbolics + MathIR. `ExactRationalPayloadTests2` now pins it with the plugins wired:
   `1/3` → exact, 1/3; `1/3 + 1/6` → exact, 1/2; `evalf(1/3, 50)` → not exact, no rational form.
   Lesson: when an in-process result disagrees with the runner, check the engine's plugin wiring
   first — and capture diagnostics with a per-case try/catch writing to a file, not assertion text.)

   Superseded detail kept only for the trail: the exact-rational payload is in
   (`StructuredProjection.Real`): the runner reports `1/3` as `exact=true, numerator=1,
   denominator=3` and `evalf(1/3, 50)` as `exact=false` with no rational form. But an
   **in-process** Suite test of the same two cases disagrees on the second: a fresh `SuiteEngine`
   evaluating `evalf(1/3, 50)` did **not** produce the `exact=false` + null-numerator shape the
   runner shows, so the runner path and the library path differ for `evalf`. The two passing
   expectations (`1/3` → 1/3, `1/3 + 1/6` → 1/2) were kept; the failing one was reverted rather
   than left red. Characterise the difference (default precision scope? a different `Real`
   exactness predicate on the two paths?) before re-adding the third case.
   **Update:** a follow-up probe that evaluated `1/3`, `evalf(1/3, 50)` and `evalf(1/3, 20)` in one
   test and wrote the results to a temp file **failed before it could write anything**, so one of
   those cases throws in-process even though the runner evaluates all three happily. Next attempt:
   isolate which case throws and capture the exception type and message (`Assert.Equal("SHOW",
   ex.ToString())` and read the failure text, or call the cases one at a time). Do not re-add the
   exactness test until this is understood — it may affect the `exact` flag the whole projection
   reports.

1c. **Item 13 — integer-sandwich contradiction: design ready, not implemented.**
   Symptom (reproduced): `x = symbol("x", integer); assume(x > 1/2); assume(x < 1)` is **accepted**
   even though no integer satisfies it. The shape-sensitivity half of item 13 is already fixed
   (`1/2 < x` is now normalised to `x > 1/2`).
   Where to fix: `Lovelace.Symbolics/Assumptions.cs`, `AssumptionSet.Add` (around the existing
   `Negate(a)` provability check). The negation test cannot see this case because the two halves
   are individually consistent — the emptiness comes from the *domain*.
   Suggested rule (bounded, testable): after the existing check, if the set contains
   `SymbolDomainAssumption(s, Domain.Integer)` for the symbol being constrained, collect that
   symbol's tightest relation bounds from the atoms (`Gt`/`Ge` from `x > b`/`x >= b`, `Lt`/`Le`
   from `x < b`/`x <= b`), and decide emptiness over the integers: the set is empty when no
   integer lies inside the interval — i.e. when `ceil(lower)` (plus one if the lower bound is
   open and already integral) exceeds `floor(upper)` (minus one if the upper bound is open and
   already integral). `Rational` already provides `Floor`, `IsInteger` and `ToInteger`
   (`Assumptions.cs` uses them for the lattice), so this needs no new arithmetic. On emptiness,
   throw `AssumptionContradictionException` with a `Describe(...)`-rendered message, exactly like
   the existing contradiction path, so the DSH envelope reports `UnsatisfiableAssumptions`.
   Tests to add: the integer sandwich (`1/2 < x < 1`), an integral-touching pair that IS
   satisfiable (`x > 0`, `x < 1` ⇒ x has no integer either — expect contradiction), a
   satisfiable integer interval (`x >= 1`, `x <= 2`), and the real-domain control (`x = symbol("x");`
   `assume(x > 1/2); assume(x < 1)` must stay accepted, since the reals do have solutions).
   Do not attempt this without running the full Symbolics suite: `Assumptions.cs` carries the
   324-query bound matrix and the 324-pair contradiction matrix, both of which must stay exact.

2. **Item 7 (exactness) — remaining half.** `Real` exactness now distinguishes truncated values, but `1/3` still crosses as `{"kind":"Real","value":"0.(3)"}`. Add `{numerator, denominator}` to the projection of exact rationals (decision **D1** in the alignment doc: keep the payload vocabulary, add fields) and prove `evalf`/`subs`/arithmetic preserve it.
3. **Item 13 (integrality).** `symbol("x", integer)` + `x > 1/2` + `x < 1` is still accepted (empty set). Also the surface is shape-sensitive: `assume(1/2 < x)` is rejected while `assume(x > 1/2)` is accepted.
3b. **Item 19 — measured, and the fix is justified but not urgent (item 26 says measure first).**
   `AssumptionSet.Add` re-renders every atom via `ToString()` and re-sorts on each insertion, so
   building an n-atom set is O(n²) string renders. Measured (fresh context, atoms added one by one):

   | atoms | time |
   |---|---|
   | 100 | 8.18 ms |
   | 200 | 26.91 ms |
   | 400 | 91.13 ms |

   Doubling the count roughly **triples** the time (exponent ≈ 1.8), confirming the audit's O(n²)
   claim rather than accepting it. Practical impact is bounded: real sessions carry a handful of
   assumptions (sub-millisecond), so this is worth fixing only alongside a reason — the fix is to
   keep a precomputed ordering key per atom instead of re-rendering, which touches
   `Assumptions.cs` and must keep the 324-query and 324-pair matrices exact. Recorded as measured;
   **not** restructured speculatively, as item 26 instructs.

3c. **Item 21 — scoped by experiment: bigger than it looks, and pruning alone is dead code.**
   The alignment doc says Piecewise guards are "decided numerically only" and that an unsatisfiable
   assumption set must prune branches. Attempted the pruning half (treat a guard that evaluates
   `Tristate.Unsatisfiable` as a pruned branch); **both tests failed and the change was reverted** —
   `Evaluation.EvaluateCondition` decides a relation guard by *numeric* evaluation of its operands,
   so a guard like `x > 0` with an unbound symbol returns `Unknown` **before any assumption query**.
   `Unsatisfiable` therefore never reaches that branch: the one-line change was unreachable dead
   code that would have read as "§21 handled".
   The real fix is in `EvaluateCondition`: consult `ctx.Assumptions` for guards whose operands are
   symbolic, e.g. build the relation atom from the guard and call `Ask` (the lattice already answers
   symbolic relations — `simplify(x < 5)` under `assume(x >= 5)` is `False`). That is a change to a
   hot, central function and needs the full suite plus the falsification gate. **Pruning alone will
   not do it.**

4. **Items 19–23, 25** as scoped in the alignment doc §4 (assumption cost/Ask memoisation, verification breadth incl. the SymPy oracle — note Python is **not** installed here so it silently skips, Piecewise pruning, record schema registry, RuleId→Lean map, omit flags).
5. **Item 15 residual**: document the descriptor-example prelude contract (`x/y/t`) in the user-facing help/docs.
6. **§143 adversarial audit** on the published binaries (new user, C# API consumer, DSH agent, Studio user, mathematician, concurrency tester, fuzz input) — not yet run.
7. **Commit.** Nothing in this cycle is committed; 50+ files changed.

## Known-honest caveats to carry into the final report

- Studio's UI is not browser-verified.
- Benchmark numbers come from a ShortRun job on a shared machine; §1 of the baseline explains why they are direction-only.
- The differential SymPy oracle cannot run on this machine (no Python), so those tests skip.
- `warning CS0162` (unreachable code at `Lovelace.Run/Program.cs:34`) and pre-existing `CS8767` warnings in `Lovelace.Integer`/CS8600 in `Lovelace.Real` remain.
