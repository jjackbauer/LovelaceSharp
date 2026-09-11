# A+ Cycle 3 — final report

> Companion documents: **a-plus-cycle-3-alignment.md** (the approved plan, decisions D1-D8, section 9.1
> wire resolutions), **a-plus-convergence-alignment-plan.md** (sections C-J, treated as law),
> **dsh-protocol.md** (the machine protocol), and the harness records under `docs/goal-cycle-3/`
> (goal, journal, 140 evidence rows, per-round artifacts).

## Exact commit

**HEAD is still `35e2609`.** Every Cycle-3 change is **uncommitted working-tree state**
(28+ modified or new paths). This is recorded as RISK-005 in the journal and as OQ-007: the P0 wire
revisions are the kind of change a reviewer would want revertible on their own. Nothing in this report
is attributable in git history yet.

## Headline

All nineteen items of the Cycle-3 brief are closed. The adversarial audit then found **three
disqualifying defects**; all three are fixed and independently verified. The cycle is **not** claimed
A+, and the below-A+ list at the end says exactly why.

## Suites

**2386 passed / 0 failed / 6 skipped** across 15 projects (cycle start: 1976). The 6 skips are the
SymPy differential oracle, which now reports SKIPPED instead of a false pass.

| Suite | start | end |
|---|---|---|
| Symbolics | 389 | **740** |
| Suite | 620 | **640** |
| Run.Tests | (did not exist) | **39** |
| Console | 15 | 15, now reachable from CI |
| Abstractions 20, Array 19, Complex 83, Dsp 61, Integer 148, Knowledge 28, Natural 195, Representation 91, Studio 22, precbench 13, Real (non-Heavy) 272 | unchanged | unchanged |

## Semantic bugs fixed

1. **N13 - factor() dropped the sign** for a negative leading coefficient: `factor(-x^2+1)` returned
   `(x-1)*(x+1)`, which expands to `x^2-1`, not the input. Root cause: SquareFreeUnivariate
   returns a MONIC polynomial, discarding the leading-coefficient multiplier. Fixed at the
   factorisation boundary.
2. **N14 - factor() did not terminate** for a negative-leading polynomial with a repeated root
   (`-x^2-2*x-1`, `-x^3-x^2+x+1` never returned). Root cause: Yun's loop guard
   `while (!b.IsOne && !b.IsZero)` excluded the constant -1, from which the body is a provable
   no-op. Now `while (b.TotalDegree > 0)`.
3. **N15 - the pretty printer emitted text that parsed to a DIFFERENT expression**:
   `x - (y - a)` rendered as `x - y - a`. Found by the round-trip property added for item 8,
   not by the item's own complaint about doubled parentheses.
4. **N17 - cancellation was never observed.** `--cancel-after 2000` on `2^1000000000` was still
   running at 60,000 ms. Root cause: exactly one cancellation poll existed, per STATEMENT
   (`Interpreter.cs:231`); the numeric kernels never polled. Now 2096 ms.
5. **N18 - indeterminate forms returned definite zeros**: `inf - inf` and `0 * inf` both returned 0.
   Root cause: a definedness guard whose default arm could never match `NamedConstantExpr(Infinity)`.
6. **N19 - limit_full claimed a limit did not exist when it did**: `x*sin(1/x)` at 0 reported
   `exists: false`. The field conflated "not determined" with "proven not to exist".
7. **The CI published-runner smoke gate could never pass**: it asserted
   `fields['complete']['value'] is True`, but a Boolean crosses as the STRING "true". The aot-smoke
   job died at its first scenario. Whether GitHub ever ran it is not verifiable from this machine.
8. **A framework IndexOutOfRange on the user surface** (N1): a two-argument builtin called with one
   argument produced "Index was out of range" instead of a typed arity error.
9. **0^-1 blamed the exponent for a base error** (N2).

## Contract changes (all additive or sanctioned by section J)

- `solve_system_full`: `assignment` as strings to `bindings` as `Binding{name,value}` records; each
  `SystemSolution` gained `exactness`; `SystemSolveResult` gained `domain` and `complete`.
- `SolutionFamily.parameter` and `SolveResult.variable` are now Symbols, not name strings.
- `diagnostics` is a structured `Diagnostic[]` on all seven rich records - and was ADDED to
  `TransformResult`, `OptimizationResult` and `CompilationResult`, which never had it.
- A new `Enum` structured kind (section F's frozen form): `{"kind":"Enum","type":"SolveStatus","value":"Solved"}`.
- A provably empty solution set now reports `complete: true` / `Completeness.Complete`, derived from ONE
  expression so status and completeness cannot disagree.
- `MatrixInverseResult` and `MatrixSolveResult` joined the same vocabulary and the schema registry.
- New `capabilities()<<B> builtin returning a structured capability statement whose advertised codes are
  asserted equal to what the live calls actually produce.
- New `latex(expr)` builtin rendering through a fourth `PrintMode<<B> arm of the SAME printer.

## Protocol changes

- The `Enum` kind was added to `dsh-protocol.md`, which previously documented status as Text while
  the frozen section F showed Enum. The two documents disagreed; section F won.
- The `Diagnostic` form is documented: code, category (an Enum), message, recoverable, location, details.
- The CI smoke now asserts the Enum kind on status/completeness/exactness and compares Booleans as
  strings - which is the bug fix from semantic defect 7.
- `--omit-display` is recorded as DECLINED in the alignment addendum section 13, with the fields it would
  strip and the condition for revisiting it: an explicit protocol revision, never a convenience flag.

## Host / Studio / REPL

No Studio UI change this cycle and **the UI was not browser-verified** - it never has been. The audit
checked the API surface only and says so. The runner gained a testable seam
(`Runner.RunAsync(args, stdout, stderr)`), a new test project, and 16 golden fixtures pinned by
structural comparison.

## Differential, property, fuzz and metamorphic results

- **The SymPy oracle no longer reports a pass it did not earn.** It was an early `return;` inside a
  plain `[Fact]`, so it was green everywhere while comparing nothing. It now SKIPS with a reason naming
  what is missing, and FAILS when `LOVELACE_REQUIRE_SYMPY=1`. A CI job installs sympy so the widened
  corpus (roots, factorisation, limits, matrix ops) is exercised somewhere.
- **Cannot run in this environment**: no Python (Store alias stubs), so not one kernel-vs-SymPy
  comparison executed locally. A locally green run proves only the skip mechanism.
- **Metamorphic set (section 116)**: all six properties now exist. The matrix identity is decided by
  exact polynomial arithmetic under the kernel's emitted det != 0 condition, and a negative control
  proves the new check rejects an inverse the OLD numeric test accepted.
- **Falsification breadth**: five new regions (near-pole, near-branch-cut, near-discontinuity,
  assumption-boundary, complex), registry-driven at 9/9/9 rules, with the legacy points as a proven
  subset and two negative controls showing the gate still fails on a throwing rule and on a wrong rule.
- **Fuzz**: 22 hostile inputs (empty script, 200-deep nesting, unterminated string, self-assignment,
  contradictory assumptions, huge exponent, division by zero, long identifiers, unicode) produced no
  crash, no hang and no internal invariant failure.

## Native AOT and published-runner smoke

`dotnet publish -p:PublishAot=true` succeeded throughout; the audit **re-published before probing** and
recorded the exe 1,607.8 s newer than the newest source, which is the guard this project adopted after
Cycle 2 produced two false defects from a stale binary. The published binary was exercised across all
seven audit personas.

## Benchmark deltas

A MediumRun (15 iterations x 2 launches, 10 warmups, 99.9% CI, 11m33s) is recorded at
`benchmarks/symbench-longrun.md`; the committed baseline was NOT overwritten. First records with error
bars: `Factor_SexticRationalRoots 187.6 +- 1.65 us / 755.16 KB`; `Record_FieldLookup_ByName 4.409 +- 0.0244 ns, 0 B`;
`Help_Overview 4.532 +- 0.0742 us` with allocations 21.49 to 12.05 KB. Structured serialization confirmed
sub-linear: 32x data costs 15.96x time and 16.67x bytes.
**The machine was NOT idle** (~2.5 cores in flight), so every mean is directional and no improvement
claim is made against the ShortRun baseline, which carries no error bars. `AssumptionSet.Add` got 53x
faster at 800 atoms (424 ms to 7.9 ms), measured in-process because symbench has no assumption row.

## Intentional breaks

- Condition arrays may contain structured records where they previously carried strings (Cycle 2).
- `solve_system_full` `assignment` strings became `bindings` records.
- `SolveResult.variable` and `SolutionFamily.parameter` are Symbolic, not Text.
- `diagnostics` is an Array of Diagnostic records on every rich record; the free-text `Note` and
  `FailureReason` are no longer emitted as fields (the kernel keeps them for the human projection).
- `limit_full.exists` is now Boolean or Null instead of always Boolean.
- `status<<B>, `completeness`, `exactness` and `classification` are Enum values.
- `--omit-display` remains unimplemented and is now documented as declined.

## Categories still below A+ (do not read the green board as exhausted)

Three remain, and **all three need something this environment or this agent does not have** - none is
an unstarted piece of work:

1. **The cycle is uncommitted** (RISK-005). The largest practical exposure to 37 rounds of verified
   work. Committing is the maintainer's decision: OQ-007 asks whether the P0 wire revisions should
   land as their own revertible commit before the rest.
2. **The SymPy differential oracle has never executed anywhere.** It is no longer a silent pass - it
   reports SKIPPED locally and FAILS when `LOVELACE_REQUIRE_SYMPY=1` - and a CI job now installs sympy,
   but that job requires a GitHub runner and has not run.
3. **The Studio UI is still not browser-verified.** Its API surface is covered by tests and was checked
   in the audit; the panels have never been loaded in a browser from this environment.

Closed since the first draft of this report:

- **20 residual analyzer warnings** - a forced full rebuild now reports **0 warnings / 0 errors**. Each
  was fixed at the site (real `await`s, `Assert.Single`/`Assert.Empty`, an honest `out Rl?`, `Assert.Fail`);
  no `NoWarn`, no pragma, no suppression, no test weakened.
- **LaTeX symbol-name escaping** - an underscore renders as an escaped literal (`x_1` to `x\_1`) rather
  than a subscript, because a symbol name is an opaque atom and a subscript would assert structure the
  kernel does not have and is undefined for `_x`, `x_`, `a__b` and `x_1_2`. All ten LaTeX specials are
  escaped; the test tokenises control sequences rather than grepping, and round-trips a decode.
- **N16** - the bare `cancel` now documents what its value means and points at the new `cancel_full`, which
  returns a `CancelResult` carrying status and conditions built from the SAME atoms as the rewrite path
  (asserted atom-for-atom). `cancel((x-1)^2/(x-1))` now reduces to `x - 1`.

Also still true, and not counted as a gap because the frozen contract puts it out of scope: `sqrt(-1)`
and `(-1)^(1/2)` remain unsupported, now declared structurally through `capabilities()` rather than only
discoverable by tripping over them.

## What this cycle actually demonstrated

Four of the cycle's most consequential findings - a kernel that negates a class of polynomials, a
kernel that hangs on a quadratic, a printer that emits text parsing to a different value, and a
cancellation feature that does not cancel - were found by **attacking the system**, not by working the
item list. Fifteen rounds of closing items reported green while all four were live. The one that stings
most is cancellation: OQ-002 was raised in round 1 and stayed open for thirty rounds because every probe
I wrote used a workload that finished in about 2 ms.

The compensating pattern is now evidence-backed: **five defects fixed, five first-dispatch fixes**, every
one after the cause was located in source first. The two dispatches told to *search* burned roughly
ninety minutes and delivered nothing.