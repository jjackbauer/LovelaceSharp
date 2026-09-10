# A+ Cycle 2 — final report

> Written at the end of the cycle. Companion documents: **a-plus-convergence-alignment-plan.md**
> (the Cycle-1 contracts, §A–§J, treated as law), **a-plus-cycle-2-alignment.md** (this cycle's
> frozen plan and reproduction matrix), **a-plus-cycle-2-status.md** (the working handoff), and
> **dsh-protocol.md** (the machine protocol).

## Exact commit

**5632575** — "A+ symbolic runtime convergence: Cycles 1-2 (solver, protocol, typed failures,
hosts)", on branch main, parent f37f1ce. 92 paths, 3,833 insertions, 596 deletions. Working tree
clean. The commit contains Cycle 1's previously-uncommitted work and all of Cycle 2: they are
interleaved in the same files, so a file-level split would have misattributed them.

## Deliverables

| Area | State |
|---|---|
| P0 blockers (§2 items 1–6) | closed |
| P1 items (§3 items 7–17) | closed |
| P2 items (§4) | 18 closed, 19 measured + deliberately not restructured, 20 closed, 21 closed, 22 closed, 23 closed, 24 deferred by the plan itself, 25 decided, 26 measured |
| §5 do-not-regress behaviours | re-verified on the compiled binary at the end of the cycle |
| §82 deep structural equality | closed |
| Gates | suites 1971/0 · AOT published · smoke 10/10 · baseline + deltas complete · audit all five personas |

## Semantic bugs fixed (each reproduced before the fix, each now tested)

1. **Unary minus bound tighter than exponentiation**: minus 2 squared evaluated to 4, and
   subs(-x^2, x, 3) gave 9 instead of -9. Every CAS parses that as -(x squared). Found by direct
   probe, not by the plan.
2. **An unsatisfiable assumption set could be accepted**: symbol(x, integer) with x > 1/2 and
   x < 1 has no model. Integer-interval emptiness is now decided in AssumptionSet.Add.
3. **assume() was shape-sensitive**: 1/2 < x was rejected while x > 1/2 was accepted — the same
   fact. Mirrored spellings are now normalised.
4. **Piecewise guards ignored the assumption lattice** (§21): a symbolic guard was decided
   numerically, so it was always "undecidable" regardless of what was assumed.
5. **A framework ArgumentException leaked out of numeric log/pow** where the kernel's typed
   EvaluationException is required — which meant the narrowed P0 catches could be bypassed by a
   domain edge. Found by the falsification gate, and it undermined the P0 work itself.
6. **An unterminated string literal was silently accepted as data** (a typo became a value).
   Found by the fuzz persona.
7. **1/3 - 1/3 reported exact: false** — zero carried the subtraction's negative exponent.
   Found by a preservation test.
8. **Budget exhaustion and division by zero surfaced as unrecoverable internal invariant
   failures**; they are now BudgetExceeded and DivisionByZero, both recoverable.
9. **StructuredResultBenchmarks never ran** — its setup script used newline-separated statements,
   which this language does not accept, so BenchmarkDotNet aborted the class silently.
10. **That same class measured a duplicate projection** whose justifying comment had gone stale
    when the projection moved into Lovelace.Suite — the shipped path is now the measured one.

## Contract and protocol changes

- Structured condition leaves replace strings: UnsatisfiableConditions, FiniteCondition,
  PredicateCondition, DomainCondition, IntervalCondition. No raw C# record syntax reaches a payload.
- One shared StructuredProjection (Lovelace.Suite) consumed by both hosts — no second implementation.
- Rich results completed: LimitResult conditions/exactness/one-sided conditions;
  IntegrationResult method/verification_method; OptimizationResult transformations/policy;
  CompilationResult parameter domains, result domain and policies.
- Inspection is a uniform record (type, domain as a Domain value, exact, free_symbols, node_count,
  canonical, pretty, shape, rank, element_domain, assumptions, members) with Null where inapplicable.
- type() uses one vocabulary: the ValueKind name, with domain values reported as Domain.
- Exact rationals cross as numerator/denominator alongside the decimal text; exactness is honest
  (a truncated approximation reports exact: false and carries no rational form).
- Durations are structural: elapsedTime {value, unit} plus per-statement timings.
- Inspection bridge (ISymbolicInspectionBridge) supplies relevant assumptions structurally.
- Record schema registry (17 schemas) with a drift test over a live corpus; rule to proof-obligation
  bridge with a registry-equality invariant.
- Error taxonomy extended: Cancelled, ReentrancyNotSupported, BudgetExceeded, DivisionByZero,
  UnsupportedOperation — all recoverable, none reported as internal.

## Studio / host / REPL changes

- Studio: structured payload on evaluate/inspect/variables/timings; UI renders records, structural
  matrices from shape+elements, provenance from TransformResult.steps, MathIR, conditions and
  solutions; the display-text regex and server re-evaluation path was deleted; PUT /api/format plus
  a UI toggle drive engine.UnicodeOutput.
- REPL: TextReader/TextWriter seam; console.ReadKey is no longer required, so the REPL is testable.
  New Lovelace.Console.Tests (15 output-text tests).
- Runner: --omit-variables, --print-budget, --cancel-after. Cancellation returns a structured
  Cancelled status with partial output and variables.
- Engine: CancellationToken through EvaluateAsync, the builtin contract and the kernels; reentrancy
  reported as a structured diagnostic instead of deadlocking; assumptions are flow-local.
- Documentation: DSH protocol updated; Console README documents the descriptor-example prelude;
  SuiteEngine documents plugin wiring (the main C#-consumer friction point).

## Tests

Total across 14 suites: **1971 passing, 0 failed** (cycle start: 1725). Suites: Abstractions 20,
Array 19, Complex 83, Console 15 (new), Dsp 61, Integer 148, Knowledge 28, Natural 195,
Representation 91, Studio 22, Suite 620, Symbolics 389, precbench 13, Real (non-heavy) 272.

Notable additions: sign/operator precedence; expected-failure boundary (unexpected exceptions
propagate); unsatisfiable propagation; integer-interval contradictions; domain-failure vocabulary;
assumption-scope isolation (two concurrent scopes, nested scopes); cross-engine isolation
(assumptions, writers, variables); Piecewise assumption-aware guards; exact-rational payload and
exactness preservation; print budget; unterminated strings; record schemas; structural equality;
rewrite proof bridge; descriptor examples (132 executing); REPL output text; Studio structured
payload; timing scale.

## Differential, property and fuzz results

- The falsification gate now fails when a rule throws rather than skipping it — widening it is what
  found defect 5. All shipped rules are exercised at every sample point on the current tree.
- Randomized sampling remains a fixed boundary-biased rational set in interior, near-zero and
  large-magnitude regions; near-pole, near-branch-cut, near-discontinuity and complex-region
  sampling are still NOT implemented (item 20's breadth half remains open).
- The SymPy differential oracle covers differentiation and one solve corpus only, and **cannot run
  on this machine** (no Python), so those tests skip rather than compare. Extending it is unverified
  work.

## Native AOT and published-runner smoke

dotnet publish with PublishAot=true succeeded; the published native binary was exercised across ten
scenarios: a complete solve, a partial solve, a rank-2 jacobian, a domain rejection, an
unterminated literal (parse error), division by zero, a budget stop, cancellation with partial
results, --omit-variables, and --print-budget. Every case is structurally classified and
recoverable; protocolVersion 1, symbolicFormatVersion "#!lovelace-sym 1" and mathIrVersion 2 are
present on every success.

## Benchmark baseline and deltas

Baseline at benchmarks/symbench-baseline.md with the raw console transcript. Deltas for all 11
measurable baseline rows: **allocations byte-identical on every pre-existing row**, and no row
regressed beyond the 5 percent mean / 10 percent allocation gate. Two rows improved by design and
are explained: the help catalog, now memoised (allocations 21.49 KB to 11.93 KB on Help_Overview),
and serialization verified sub-linear in result size (2 to 64 solutions: 16.2x mean, 16.7x
allocated, against 32x data). The apparent mean speed-ups elsewhere (minus 10 to minus 32 percent)
are NOT claimed as improvements: these are ShortRun means on a shared workstation, and the
recorded error bars are as wide as the means. A quotable number needs a longer job on an idle
machine.

## Intentional breaks

- Condition arrays may contain structured records where they previously carried strings.
- solve() over integer/rational domains errors rather than returning complex roots.
- SolveResult.domain, family parameter domains and Inspection.domain are Domain values, not text.
- type(complex) is Domain, not complex.
- Unwrap(Value.Void) maps to null instead of throwing.
- Runner captures script print output into the envelope instead of interleaving stdout.
- StructuredResultBenchmarks now measures the shipped projection, so its numbers are not comparable
  with the local-copy row.

## Adversarial audit (§143) — friction, reported honestly

| Persona | Outcome |
|---|---|
| New user / mathematician | degree-5 solve reports Partial with a reason; integrate(exp(-x squared)) is unevaluated (correct); factor(x^4+1) unchanged (irreducible over the rationals). Friction: abs(x) is produced by the rewriter but cannot be written in the language. |
| Fuzz input generator | found defect 6 (unterminated string). Empty scripts, 200-deep nesting, self-assignment and contradictory assumptions all behave. |
| DSH agent | envelope versions, stdout purity, shapes, structural errors, cancellation with partial results all correct. |
| Studio user | live host round-trip recovered SolveResult, Solved, solution shape, domain, exactness and timings from structure alone. |
| C# API consumer | friction: a bare SuiteEngine has no evaluator until plugins are loaded, and "Unknown function" gives no hint. Mitigated by documenting the wiring on SuiteEngine; the message itself is unchanged. |
| Concurrency tester | cross-engine assumptions, output writers and variables isolated; two concurrent assumption scopes isolated; nested scopes restore correctly. |

Method note, recorded because it is easy to repeat: the first audit run used a **stale AOT binary**
and produced two false defects. Re-publish before auditing.

## Categories still below A+ (do not read the green board as exhausted)

1. **Item 24 (LaTeX printer)** — deferred by the alignment plan itself; no LaTeX output exists.
2. **Item 20's breadth half** — sampling regions beyond interior/near-zero/large-magnitude remain
   unimplemented (no near-pole, branch-cut, discontinuity or complex sampling).
3. **The differential oracle** — SymPy covers only differentiation and one solve corpus, and cannot
   execute here at all; extended oracle coverage is unverified.
4. **Benchmark deltas** — ShortRun means on a shared machine; they establish no-regression, not
   improvement. Allocations are the reliable signal.
5. **Studio UI** — verified by syntax check, server-side tests and a live API round-trip, never in
   a browser.
6. **Item 19** — AssumptionSet.Add is O(n squared) in atoms (measured: 400 atoms costs 91 ms);
   deliberately not restructured because no real workload reaches that size.
7. **Record schema registry** — 17 schemas cover the result records the kernel emits; matrix result
   records are not registered.
8. **--omit-display** — declined on the item's own condition (it would strip a documented protocol
   field), not implemented.

## What this cycle actually demonstrated

Four of the ten defects above — including the one that undermined the P0 typed-catch work — were
found by attacking the system (fuzzing, preservation tests, the falsification gate, a live host
round-trip) rather than by working the item list. The plan-driven items were largely mechanical;
the consequential fixes came from adversarial and preservation testing. Future cycles should treat
§143 as a standing practice rather than a one-off gate.
