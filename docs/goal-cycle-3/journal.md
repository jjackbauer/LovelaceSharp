# Goal Journal — cycle-3 (LovelaceSharp symbolic runtime convergence)

> Append-only. See the stretch-goal-harness evidence protocol for entry format.

---

## Round 0 — Phase 0 contract

### DEC-001: Goal contracted from the Cycle-3 brief

- **Decision**: The goal is the brief's §2–§4 item list, executed in the brief's mandated order:
  baseline first, reproduce every item against compiled binaries, write the alignment addendum,
  **stop for approval**, then implement P0 → P1 → P2.
- **Rationale**: The brief's §0 protocol is explicit and its method-requirements section records
  costs already paid by Cycles 1–2 (stale AOT binary, speculative restructuring, probe error).
- **Alternatives considered**: Starting implementation immediately (rejected: the brief mandates a
  stop-for-approval gate after the addendum); re-deriving the item list from the report instead of
  the brief (rejected: the brief is the user's stated scope).
- **Related**: OBS-001

---

<!-- New entries go below this line -->

## Round 1 — Baseline + reproduction matrix

### OBS-001: Cycle-2 handoff claims a 1971-test total that its own per-suite list contradicts

- **Source**: `docs/symbolics/a-plus-cycle-2-report.md:158` and the per-suite list at `:120-124`
- **Fact**: The listed per-suite counts sum to 1976, not 1971.
- **Implications**: The cycle-start figure quoted in the Cycle-3 brief is 5 low; the measured baseline is 1976.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-002, N9

### OBS-002: No golden fixture and no runner test project exist anywhere

- **Source**: `glob **/*golden*` (0 matches), `glob **/*Lovelace.Run.Tests*` (0 matches), `.github/workflows/ci.yml:150-250`
- **Fact**: The only envelope assertions are inline `python3` heredocs in the CI AOT job.
- **Implications**: Item 5's second half is confirmed; the envelope is not unit-testable.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-010

### OBS-003: asinh/acosh/atanh are kernel-registered functions but not builtins

- **Source**: `Lovelace.Symbolics/Functions.cs:93/95/97`, `Evaluation.cs:341-345`, `Assumptions.cs:723-725`; absent from `SymbolicsPlugin.cs:25-26` and `CoreBuiltinMetadata.cs:22-82`
- **Fact**: Evaluators and a domain rule exist; only the builtin descriptor is missing.
- **Implications**: Item 6's "not registered functions at all" is imprecise — this is an exposure gap, not missing mathematics.
- **Confidence**: High
- **Agent**: Observer B (Q2), verified by orchestrator
- **Related**: EVD-028, N10

### OBS-004: ParameterDomainOf discards its argument

- **Source**: `Lovelace.Symbolics/SymbolicsPlugin.cs:612`
- **Fact**: `private static MathDomain ParameterDomainOf(ParameterDomain p) => MathDomain.Integer;`
- **Implications**: `ParameterDomain.NonNegativeIntegers` (`Solve.cs:72`) can never reach the wire.
- **Confidence**: High
- **Agent**: Observer A (B-rows), verified by orchestrator
- **Related**: EVD-029, N5

### HYP-001: Every reachable NoSolutions result pairs with completeness Unknown / complete false

- **Claim**: No input produces the latent `NoSolutions + Complete` pairing that the source at `SymbolicsPlugin.cs:384-406` permits.
- **Supporting OBS**: OBS-004
- **Why it matters**: If true, item 4 is a consistency fix; if false, it is a semantic contradiction already on the wire.
- **Falsification strategy**: Construct the `set.Status == Solved && accepted.Count == 0` state; try real-domain rejection, identities, poles, and transcendental no-solution equations.
- **Status**: Under review
- **Confidence**: Medium

### VAL-001: Observer A's emitted-completeness claim is Falsified by execution

- **Target**: Observer A Q4 ("emitted status NoSolutions + completeness Complete")
- **Method**: Executed 5 independent `solve_full` inputs against the freshly published AOT binary and read the envelope.
- **Evidence examined**: `docs/goal-cycle-3/round-1/probes/ns_*.json` — all five emit `completeness: Text("Unknown")`, `complete: Boolean(false)`
- **Result**: **Falsified**
- **Conclusion**: The source-level reading identifies a real latent branch, but it is not observable. Recorded as RISK-003, not as a confirmed defect. Execution outranks source reading.
- **Related**: EVD-027, RISK-003

### VAL-002: Observer B's metamorphic-set claim is Supported; the brief's premise is Falsified

- **Target**: "The metamorphic set (§116) is not implemented" (brief item 10)
- **Method**: Opened each cited test body and read the assertions.
- **Evidence examined**: `PropertyTests.cs:146, 177, 236`; `SmokeTests.cs:59, 157`
- **Result**: **Falsified** (the brief's premise) / **Supported** (Observer B)
- **Conclusion**: 5 of 6 §116 properties exist; `A·A⁻¹ = I` is numeric-only and `cancel preserves value off poles` is absent.
- **Related**: EVD-015

### VAL-003: `--print-budget` is honoured (my own probe was the defect)

- **Target**: my probe showing byte-identical output under `--print-budget 6`
- **Method**: Re-probed with `expand((x+1)^12)` (nodeCount 58) against the same binary.
- **Evidence examined**: `probes/r3_budget_expand.json` — `truncated=true, truncationReason=node-budget, budget=6, nodeCount=58`
- **Result**: **Falsified** (the defect claim)
- **Conclusion**: My first probe used an expression whose node count was below the budget. Recorded as the third instance of the "check your probe first" lesson.
- **Related**: EVD-024

### DEC-002: Item 10 is scoped down to its two real gaps

- **Decision**: Treat item 10 as "strengthen the matrix-inverse property to the symbolic/conditions form and add the cancel-off-poles property", not "implement the metamorphic set".
- **Rationale**: VAL-002 — five of six properties already exist as real property tests.
- **Alternatives considered**: Re-implementing the whole set (rejected: would duplicate passing tests and risk weakening them).
- **Related**: VAL-002, EVD-015

### RISK-003: Latent NoSolutions/Completeness inconsistency in the projection

- **Risk**: `SymbolicsPlugin.cs:384-385` rewrites a local status while `set.Status` stays `Solved`, so `:406` would emit `completeness Complete` beside `status NoSolutions`.
- **Likelihood**: Low (unreachable in 5 attempts)
- **Impact**: High (a three-way contradiction on the wire)
- **Evidence**: VAL-001, `SymbolicsPlugin.cs:384-406`
- **Mitigation**: Single-source the derivation in the item-4 fix and test the state directly rather than via an input.
- **Gate**: —

### OQ-001: Why does `simplify_full((x+1)^40)` report `changed: true` with zero steps?

- **Question**: Is `TransformResult.changed` comparing canonical form while `original`/`expression` pretty-print identically, or is it wrong?
- **Needed evidence**: A targeted probe over inputs that are and are not structurally changed, reading `changed` against canonical equality.
- **Priority**: P2 curiosity
- **Raised by**: Round 1, orchestrator probe `heavy_cancel`
- **Related**: N6

### OQ-002: Did any run in this cycle actually exercise cancellation?

- **Question**: `--cancel-after 1` on solves with measured elapsed 1.8-2.1 ms returned a normal result rather than `Cancelled`. Is the token polled inside long kernel loops, or only at statement boundaries?
- **Needed evidence**: A single statement whose runtime is orders of magnitude above the budget (e.g. a large `expand`), run with `--cancel-after 1`.
- **Priority**: P1 important
- **Raised by**: Round 1, orchestrator probes `r3_cancel_*`
- **Related**: EVD-026

### OQ-003: What regions were the 13 falsification sample points intended to cover?

- **Question**: Observer B could not determine the design intent of `FalsificationTests.cs:17-22`; the points are unnamed rationals reused by four subsets.
- **Needed evidence**: The Cycle-1 plan's §117 wording, or the commit that introduced the set.
- **Priority**: P2 curiosity
- **Raised by**: Round 1, Observer B "could not determine"
- **Related**: EVD-016

### OQ-004: Does a Symbolic `abs` payload ever reach the plugin, or is it rejected earlier?

- **Question**: Observer B could not trace dispatch order; the runtime message comes from the plugin, but the payload-kind gate may sit in the interpreter.
- **Needed evidence**: Read the dispatch path from `Interpreter.cs:1224` to the plugin's `abs` handler.
- **Priority**: P2 curiosity
- **Raised by**: Round 1, Observer B "could not determine"
- **Related**: EVD-012

### TODO-001: Close the two remaining §116 metamorphic gaps

- **Task**: Symbolic `A·A⁻¹ = I` under the emitted `det ≠ 0` condition, and `cancel` preserves value off poles.
- **Priority**: P1
- **Depends on**: Phase B
- **Status**: Open
- **Related**: VAL-002

### TODO-002: Wire `Lovelace.Console.Tests` into the slnx and the CI suite list

- **Task**: Add the project to `LovelaceSharp.slnx` and to the `suites=(...)` array in `.github/workflows/ci.yml`.
- **Priority**: P0 (acceptance gate: a test project CI does not run)
- **Depends on**: —
- **Status**: Open
- **Related**: EVD-021
## Round 2 — runner test project + golden fixtures (A5)

### DEC-003: Phase A5 lands before the P0 wire changes

- **Decision**: Build the runner test project and the golden-fixture corpus first, then change the wire (A1–A4), updating the fixtures deliberately in the same rounds.
- **Rationale**: RISK-001 — the enum/structured changes are a wire revision touching the projection, Studio, the CI smoke and every fixture at once. Fixtures first turn that into a red-to-green signal instead of a silent protocol drift.
- **Alternatives considered**: changing the wire first and adding fixtures afterwards (rejected: it leaves the contract unpinned exactly when it is most volatile).
- **Related**: EVD-030

### VAL-004: The runner extraction preserves the envelope exactly

- **Target**: "extracting Runner.RunAsync from Program.cs changes no emitted value"
- **Method**: captured the envelope for `solve_full(x^2 - 4 == 0, x)` before the edit and after it, normalised the four volatile keys, and compared the documents myself.
- **Evidence examined**: EVD-030 — both 3456 bytes; normalised forms identical at 3276 chars.
- **Result**: **Supported**
- **Conclusion**: the seam needed for in-process envelope testing is behaviour-preserving. The test project can call the real code path rather than reimplementing it.
- **Related**: EVD-030
### VAL-005: The runner test project is a real delivery, not scaffolding

- **Target**: the Round-2 Implementer delivery (18 tests plus 14 golden fixtures)
- **Method**: read every test body; mutated a golden value and re-ran; searched for skip attributes; re-ran the suites and the AOT publish myself.
- **Evidence examined**: EVD-031 to EVD-036; `GoldenEnvelopeTests.cs:112-128` guards that every fixture script has a corpus row; the only `Skip` site is conditional on the runner assembly being present and the run reports 0 skipped.
- **Result**: **Supported**
- **Conclusion**: the corpus is genuine. An added, removed or changed envelope value fails with a JSON path, and key-order changes still pass. No assertion is metadata-only.
- **Related**: EVD-031, EVD-032

### OBS-005: A timestamp-preserving restore defeats PreserveNewest fixture copying

- **Source**: observed while falsifying (my own Copy-Item restore preserved the pre-mutation mtime)
- **Fact**: the tests read fixtures from AppContext.BaseDirectory; when the source file mtime is older than the copied output, MSBuild does not re-copy, so a stale golden is compared.
- **Implications**: a developer editing a fixture normally gets a copy (newer mtime); the hazard needs a timestamp-preserving restore. CI is unaffected (clean output). Recorded so a later falsification round does not misread it as a product defect.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: EVD-032

### OBS-006: Two orchestrator script bugs, both mine, both caught by their observable

- **Source**: my own verification scripts this round
- **Fact**: (a) naming a PowerShell function parameter after the automatic arguments variable made every dotnet call return -2147450751 instead of running; (b) the code runtime's string parser rejects a dollar sign inside a template literal.
- **Implications**: the third and fourth "the probe is the defect" instances this cycle. Worth recording because the first failure code masqueraded as an environment problem and cost a build-server shutdown detour.
- **Confidence**: High
- **Agent**: orchestrator
- **Related**: VAL-003
### OBS-007: The published-runner smoke gate in CI could never pass

- **Source**: `.github/workflows/ci.yml` (pre-change) versus EVD-042
- **Fact**: the smoke asserted `fields['complete']['value'] is True`, but a Boolean structured value crosses as the string "true". The assertion always raised, so the aot-smoke job failed at its first scenario — before reaching the later scenarios at all.
- **Implications**: an acceptance gate of this cycle is "a CI failure". Whether the workflow actually ran on `35e2609` is not verifiable from this machine (no runner, no python3), but the job as written cannot succeed. Repaired in Round 4a and re-checked by a node proxy over the published binary.
- **Confidence**: High (the assertion and the value form are both verified; only "did GitHub run it" is unknown)
- **Agent**: Implementer (found) + orchestrator (verified)
- **Related**: EVD-042, EVD-043, N11

### VAL-006: Round 4a delivered the Enum kind without moving any value

- **Target**: the Implementer's claim that exactly 6 goldens changed, all Text-to-Enum, with no value or display byte moved
- **Method**: read the golden diff; re-ran six suites and the AOT publish myself; read the raw JSON of the published binary.
- **Evidence examined**: EVD-041..EVD-045, `docs/goal-cycle-3/round-4a/golden.diff`
- **Result**: **Supported**
- **Conclusion**: the Enum kind is additive as designed — a field gains a kind and a type name, its value is unchanged, and the human display is unchanged.
- **Related**: EVD-041, EVD-045
### VAL-007: Round 4b closed the structured-diagnostics half of item 3

- **Target**: diagnostics as a structured array on all seven rich records, with no free text left
- **Method**: re-ran five suites and the AOT publish myself; read the raw JSON of the freshly published binary; counted Text-valued diagnostics on the wire.
- **Evidence examined**: EVD-046..EVD-050, `docs/goal-cycle-3/round-4b/golden.diff`
- **Result**: **Supported**
- **Conclusion**: item 3 is closed. A diagnostic carries a stable code, an ErrorCategory enum, a message, a recoverability flag, a location and nested details; the human note survives only inside the message and in the display projection.
- **Related**: EVD-046, EVD-048

### DEC-004: The kernel's free-text members stay; only the wire was cleaned

- **Decision**: `SolutionSet.Note`, `LimitResult.FailureReason` and `IntegrationResult.Note` remain in the kernel; the wire no longer exposes them as fields.
- **Rationale**: decision D1 of the addendum said "drop the free-text Note/FailureReason from the emitted record". The Implementer flagged that deleting the kernel members would touch three files outside its SCOPE and would break the human solve projections. Emitting them nowhere while keeping them as the human projection's source satisfies the bar ("no semantic strings in machine APIs") without a wider change.
- **Alternatives considered**: deleting the kernel members (rejected this round: wider blast radius, no additional wire benefit); keeping them as emitted fields (rejected: that is the defect).
- **Related**: EVD-049, EVD-051

### OQ-005 (resolved): the unreachable ParameterDomain arm

- **Question**: whether `ParameterDomain.NonNegativeIntegers` can be reached from the language.
- **Resolution**: Round 3 confirmed it cannot — all five SolutionFamily sites pass `Integers`. The arm is pinned by a direct internal call and the mapping is now a total switch with a throwing default, so a future arm cannot be silently mis-mapped. Closed.
- **Related**: EVD-037

### RISK-004: Two matrix records still carry free text on the wire

- **Risk**: `MatrixInverseResult` and `MatrixSolveResult` emit `diagnostics` as text and a text `status`, and neither is in the schema registry — so the drift test cannot see them.
- **Likelihood**: High (it is the current state)
- **Impact**: Medium (they are outside the brief's seven, but "no semantic strings in machine APIs" is a general acceptance bar)
- **Evidence**: EVD-051
- **Mitigation**: Round 6 closes both together — register the schemas, extend the drift corpus to call `inv_full` and `linsolve_full`, and give both records the same Diagnostic and status vocabulary.
- **Gate**: —
### VAL-008: Round 5 closed item 4 and N1

- **Target**: NoSolutions now pairs with a complete answer; wrong-arity calls are typed errors
- **Method**: re-ran two suites myself; ran seven probes against the freshly published binary and read the envelope fields.
- **Evidence examined**: EVD-052..EVD-056
- **Result**: **Supported**
- **Conclusion**: RISK-003's three-way contradiction is removed by construction (one mapping function feeds both fields), and the user surface no longer shows a framework index failure. Item 4 and N1 are closed.
- **Related**: EVD-052, EVD-054

### RISK-003 (closed): the latent three-way contradiction

- **Resolution**: SolveCompletenessMapping.Of is now the single derivation for both complete and completeness, called by both solve_full and the system-solve result, so the two fields cannot disagree. Closed at Round 5.
- **Related**: EVD-052
### VAL-009: Round 6 closed N12 and item 14

- **Target**: the two matrix records join the solve vocabulary, and the registry plus drift corpus can see them
- **Method**: re-ran Suite and Run suites; ran the published binary on both new fixture scripts and read every field.
- **Evidence examined**: EVD-057, EVD-058
- **Result**: **Supported**
- **Conclusion**: N12 and item 14 are closed. The completeness mapping was reused unchanged (not duplicated), and the drift corpus now calls inv_full and linsolve_full so the registry can no longer drift unnoticed for these types.
- **Related**: EVD-051, EVD-057
### VAL-010: The six rounds compose without breaking anything

- **Target**: the integrated working tree after rounds 2-6 (wire revisions, a new test project, an AOT-affecting refactor)
- **Method**: ran every suite in the repository myself and summed the results.
- **Evidence examined**: EVD-059
- **Result**: **Supported**
- **Conclusion**: 2074 passing, 0 failed, 0 skipped. No round left a latent failure for another to hide.
- **Related**: EVD-059

### OQ-006: Can the SymPy oracle extension be verified anywhere from this machine?

- **Question**: the oracle still cannot execute locally and still reports pass while comparing nothing. Can any part of the extension be verified here, or is CI the only place?
- **Needed evidence**: a local run with a real interpreter, or the CI job's observed output.
- **Priority**: P1 important (it is the next round's whole subject)
- **Raised by**: Round 7, orchestrator context limit
- **Related**: EVD-017
### VAL-011: No test project can hide from CI

- **Target**: the acceptance gate 'a test project that CI does not run'
- **Method**: enumerated every *.Tests directory on disk and matched each name against both the CI suite list and the solution file.
- **Evidence examined**: EVD-060
- **Result**: **Supported**
- **Conclusion**: 15/15 wired. Round 2 fixed Console.Tests and added Run.Tests; this confirms nothing else was hiding, which a spot-check of the two known projects would not have shown.
- **Related**: EVD-021, EVD-033, EVD-060
### VAL-012: The no-swallowing claim holds after nine rounds of edits

- **Target**: the section 5 do-not-regress claim 'zero broad catch (Exception) in Lovelace.Symbolics'
- **Method**: grepped the project; for the one broad catch found elsewhere in the stack, read its body instead of assuming.
- **Evidence examined**: EVD-061, EVD-062, EVD-063
- **Result**: **Supported**
- **Conclusion**: zero broad catches in the symbolic reasoning, and the single broad catch in SuiteEngine records a diagnostic and rethrows. The claim was asserted in Cycles 1-2 and inherited unverified by this cycle; it is now re-proved on the current tree.
- **Related**: EVD-061, EVD-062
### RISK-005: Nine rounds of verified work exist only in the working tree

- **Risk**: every Cycle-3 change (wire revisions, a new test project, an AOT-affecting refactor, a CI gate repair, the matrix-record vocabulary) is uncommitted on top of 35e2609. A single accident destroys it, and none of it is attributable in git history.
- **Likelihood**: Low per unit time, but the exposure grows with every round
- **Impact**: High (the largest single risk to this cycle, and Cycle 2 named the same thing about its own tail)
- **Evidence**: git status --porcelain reports 28 changed or untracked paths; HEAD is still 35e2609
- **Mitigation**: the tree is green and reproducible (EVD-059: 2074 passed / 0 failed across 15 projects), so an intermediate commit is safe to make the moment the human wants one. This round deliberately does NOT commit: the cycle is not finished, and committing is a decision for the maintainer, not for the orchestrator.
- **Gate**: -

### OQ-007: Should the cycle be committed in stages?

- **Question**: the brief ends with a commit ('a commit' was listed as remaining work in the Cycle-2 handoff). Should Cycle 3 land as one commit at the end, or as staged commits per phase (P0 / P1 / P2) so the contract changes are individually revertible?
- **Needed evidence**: a maintainer's preference. The P0 wire revisions are the sort of change a reviewer would want to see in isolation.
- **Priority**: P1 important (it affects how the remaining rounds are executed)
- **Raised by**: Round 10, orchestrator, on finding its remaining context insufficient for another implementation round
- **Related**: RISK-005
### OQ-008: Is the item-12 delivery real, or only reported?

- **Question**: the item-12 subagent was still running when this session exhausted its context. Did it actually make the unavailable-oracle case loud and conditional (skipped, not passed) and add a CI job that installs sympy, or does the silent-pass path remain?
- **Needed evidence**: `docs/goal-cycle-3/round-11/implementation.md`, plus my own two runs of the oracle class (no env var -> SKIPPED; LOVELACE_REQUIRE_SYMPY=1 -> FAILED).
- **Priority**: P0 blocking for the next round - nothing from that round enters evidence.md until this is answered.
- **Raised by**: Round 11, orchestrator context exhaustion
- **Related**: RISK-005, EVD-017
### VAL-013: Round 11 verified - and it found a kernel bug

- **Target**: item 12 (the differential oracle) plus the Implementer's F1 finding
- **Method**: ran the oracle class twice myself with and without the requirement variable; then probed the alleged Factoring defect through the published binary rather than trusting the report.
- **Evidence examined**: EVD-064, EVD-065
- **Result**: **Supported** (item 12's local half) and **Supported** (F1 is real, and worse than reported)
- **Conclusion**: the oracle now reports SKIPPED when it cannot run and FAILS when required, so it can no longer be green without comparing anything. Its corpus work immediately surfaced a genuine correctness defect.
- **Related**: EVD-064, EVD-065, N13

### N13 (new, P0-class correctness): factor() drops the sign for a negative leading coefficient

- **Fact**: factor(-x^2 + 1) returns (x - 1)*(x + 1); expanding it gives x^2 - 1, not -x^2 + 1. factor(-x^3 + x) returns x*(x - 1)*(x + 1), which expands to x^3 - x.
- **Why it matters**: factor() silently returns a different polynomial from its input. It breaks the section 116 metamorphic property expand(factor(p)) = p, and it is exactly the class of defect the oracle exists to catch. The control case shows it is invisible unless the result is expanded.
- **Evidence**: EVD-065
- **Priority**: P0 - a wrong mathematical answer, not a presentation issue
- **Mitigation**: fix Factoring.Factor to carry the sign (leading-coefficient normalisation), add the metamorphic property test expand(factor(p)) = p over negative-leading-coefficient inputs, and REINSTATE the oracle case the Implementer excluded at OracleCorpus.cs:129 as a live comparison.
- **Related**: EVD-065

### DEC-005: An oracle case may not be excluded because the kernel is wrong

- **Decision**: the excluded factorization case at OracleCorpus.cs:129 must be reinstated as a live comparison once N13 is fixed. Until then it stays visible as a documented known failure, not as a silent omission.
- **Rationale**: the Implementer excluded the case and documented it, which is honest but leaves the oracle blind exactly where it found a defect. Documenting a wrong answer is not the same as fixing it, and an oracle that skips the failing inputs cannot catch their regression.
- **Alternatives considered**: leaving the exclusion with a comment (rejected: the oracle would never re-test the case after the fix).
- **Related**: EVD-065, N13
### OQ-009: Is N13 actually fixed, or only tested for?

- **Question**: the N13 fix subagent was still running at context exhaustion and had not yet landed a kernel change. Does factor() carry the sign now, or does the tree still return the negation for a negative leading coefficient?
- **Needed evidence**: `docs/goal-cycle-3/round-13/implementation.md`, plus my own four published-binary probes (see the ROUND 13 section in state.md).
- **Priority**: P0 blocking - N13 is a wrong mathematical answer and the whole cycle's A+ claim rests on it.
- **Raised by**: Round 13, orchestrator context exhaustion
- **Related**: EVD-065, RISK-005
### VAL-014: Round 13 FAILED - and it left the tree worse than it found it

- **Target**: the N13 fix (OQ-009), plus the state the interrupted subagent left behind
- **Method**: interrupted the subagent after ~45 minutes with no report; rebuilt the solution; probed factor() through the JIT runner; ran the Symbolics suite three times.
- **Evidence examined**: EVD-066, EVD-067
- **Result**: **Falsified** (the fix) and **Supported** (a new, worse problem)
- **Conclusion**: two findings. (1) N13 is unfixed: the tree still returns the negation, and no kernel change was made. (2) The Symbolics test suite now times out at 420 s where it took 14-21 s before, while the code still builds cleanly - so the interrupted work is not merely incomplete, it is blocking. This is the first round of the cycle that consumed budget and left the tree in a worse state than it found it.
- **Related**: EVD-066, EVD-067, RISK-006

### RISK-006: An interrupted implementer left a non-terminating suite

- **Risk**: the Symbolics suite hangs (three timeouts at 420 s, empty output). Everything downstream - the published-runner smoke, the AOT gate, every later round - is blocked until this is triaged.
- **Likelihood**: Certain (it is the current state)
- **Impact**: High (it blocks the cycle, and the hang is unexplained)
- **Evidence**: EVD-067. The project compiles with 0 errors, so the cause is in the new test file FactorMetamorphicTests.cs or in the oracle files, not in the kernel.
- **Mitigation**: the next round opens by triaging that file: read it, then bisect by filtering to the new tests, and identify the non-terminating case. Do NOT delete the file to restore green - a hanging case is itself a finding, and deleting it destroys the evidence. If the hang is inside the kernel, that is a second defect and a more serious one than N13.
- **Gate**: G4
### N14 (new, P0-critical): Factoring.Factor does not terminate for a negative-leading polynomial with a repeated root

- **Fact**: `factor(-x^3 - x^2 + x + 1)` and `factor(-x^2 - 2*x - 1)` never return (killed at 15 s). Eight sibling corpus entries return immediately, including other negative-leading cases and the positive-leading repeated-root control.
- **Pattern**: the two hanging inputs are exactly the negative-leading polynomials whose factorisation has a repeated root: `-(x+1)^2` and `-(x+1)^2*(x-1)`.
- **Why it matters**: a hang is strictly worse than N13's wrong sign. It is reachable from the published binary with one expression, so it is a user-facing non-termination, and it blocks the Symbolics suite and therefore every downstream gate.
- **Evidence**: EVD-068, EVD-069, EVD-070
- **Relationship to N13**: same code path and same trigger class (negative leading coefficient). They should be fixed together, and the fix for one should be tested against the other.
- **Priority**: P0-critical, above N13
- **Mitigation**: fix the negative-leading-coefficient path in `Lovelace.Symbolics/Factoring.cs`; acceptance is (a) both hanging inputs return, (b) `expand(factor(p)) == p` over the whole 18-entry corpus including the negative cases, (c) the positive controls keep their existing output.
- **Related**: EVD-066, EVD-068

### RISK-006 (resolved to N14)

- The suite timeout was not caused by the interrupted agent's test file; that file merely *exposed* a pre-existing kernel hang. The test file is a legitimate, well-formed failing test and must be kept.
- **Related**: EVD-068, EVD-070
### RISK-007: Two dispatches on the same fix produced nothing

- **Risk**: the N13/N14 fix has now failed twice. The first agent burned ~45 minutes and landed only a test file; the second failed outright, leaving a skeleton report whose root cause is 'TBD'. Re-dispatching the same objective with the same prompt is the anti-pattern the harness names explicitly ('you will get the same answer').
- **Likelihood**: High if the next attempt repeats the same shape
- **Impact**: High - N14 is a blocking kernel hang
- **Evidence**: EVD-072, EVD-071
- **Mitigation**: change the approach, not the wording. The next dispatch must (a) name the exact target found here - `Lovelace.Symbolics/Algebra/Factor.cs:14` for the entry point and `Polynomial.cs:353-355` for the 'make monic' normalisation that is the prime suspect for the dropped sign - and (b) ask for a MINIMAL diagnostic first: instrument or unit-test the two hanging inputs directly against `Factoring.Factor` in-process, find the non-terminating loop, and only then fix it. Do not ask for the full acceptance matrix again in the first pass.
- **Gate**: G4

### VAL-015: The round-14 dispatch failed without producing a claim to falsify

- **Target**: the second N13/N14 implementer dispatch
- **Method**: read its report skeleton and checked git status for kernel changes.
- **Evidence examined**: EVD-072
- **Result**: **Falsified** (no delivery)
- **Conclusion**: no claim was made, so nothing entered evidence as a result of that round's work. The round still produced one verified fact - the exact location of the code that must change (EVD-071) - which is more than the failed agent produced in 45 minutes.
- **Related**: EVD-071, EVD-072, RISK-007
### HYP-002: The dropped sign is a constant residual factor lost in the rebuild

- **Claim**: `FactorPoly` extracts the correct linear factors, but the leftover constant factor (for `-x^2+1` that is `-1`) does not survive into the rebuilt expression. The same leftover, in the repeated-root case, is what makes the division loop non-terminating.
- **Supporting OBS**: EVD-073, EVD-074
- **Why it matters**: it turns both defects into one root cause with one fix point, and it explains why positive-leading inputs are unaffected (their constant residual is `+1`, which `Exprs.Multiply` drops harmlessly).
- **Falsification strategy**: dump the `factors` list of `FactorPoly` for `-x^2+1` and for `x^2-1` and compare. If `-x^2+1` yields the same two linear factors with an additional `(-1, 1)` entry that the published output does not show, the hypothesis holds and the loss is in `Polynomial.ToExpr()` or `Exprs.Multiply`. If the list is genuinely missing the constant, the loss is at `Factor.cs:62` instead. For the hang, instrument `FactorPoly(-x^2-2*x-1)` and count iterations of the `foreach (var r in roots)` loop at :52 - if `remaining` stops shrinking while `rem.IsZero` stays true, the loop at :52-61 is the non-terminating site.
- **Status**: Under review
- **Confidence**: Medium

### DEC-006: The next dispatch asks for a dump, not a fix

- **Decision**: the next implementer round must first PRINT the `factors` list produced by `FactorPoly` for the hanging and sign-dropping inputs, and locate the non-terminating loop, before changing any code. Only then attempt the fix.
- **Rationale**: two dispatches that went straight to a fix produced nothing (EVD-072). The diagnosis is now narrow enough that a dump settles HYP-002 in one step.
- **Alternatives considered**: another fix-first dispatch (rejected: it is the documented anti-pattern and has failed twice).
- **Related**: HYP-002, RISK-007, EVD-071
### VAL-016: HYP-002 resolved - the root cause is a monic normalisation inside the square-free decomposition

- **Target**: HYP-002 (the dropped sign is a lost constant residual)
- **Method**: read `Polynomial.IsOne` (rules out the constant being skipped), then read the monic normalisation and its call site.
- **Evidence examined**: EVD-075, EVD-076, EVD-077
- **Result**: **Supported**, with a more precise mechanism than the hypothesis proposed
- **Conclusion**: the constant is not lost in the rebuild. `SquareFreeUnivariate` RETURNS a monic polynomial (`Polynomial.cs:373` via `NormalizeUnivariate` at `:351-361`), discarding the leading-coefficient multiplier, and `FactorPoly` never restores it. The discarded multiplier is `-1` exactly when the leading coefficient is negative. This is a confirmed root cause, not an inference.
- **Related**: EVD-075, EVD-076, EVD-077

### DEC-007: The fix point is now surgical

- **Decision**: fix the sign by making the leading-coefficient multiplier survive the square-free decomposition - either by having `SquareFreeUnivariate` report the divisor it applied, or by having `FactorPoly` capture the leading coefficient of its input before decomposition and fold it into the returned `Content`. Do NOT change `NormalizeUnivariate` itself: `GcdUnivariate` (`Polynomial.cs:338-349`) depends on monic normalisation for its Euclid loop, so changing it in place would break the gcd.
- **Rationale**: the defect is a discarded multiplier, not a wrong normalisation. Restoring the multiplier at the factorization boundary is the minimal, local fix.
- **Alternatives considered**: removing the monic normalisation (rejected: it is load-bearing for `GcdUnivariate`); patching the display (rejected: it would hide a wrong value).
- **Status of the hang (N14)**: `GcdUnivariate` drives Yun's algorithm and relies on `NormalizeUnivariate`, so the same function is the prime suspect for the non-termination. That remains an INFERENCE, not a confirmed root cause - it is marked as such and must be reproduced with the dump before being claimed.
- **Related**: EVD-075, N14
### VAL-017: N13 closed - and the lesson about how it was closed

- **Target**: the N13 sign defect
- **Method**: dispatched a fix with the root cause SUPPLIED (found by orchestrator source reading the round before), then re-ran the probes myself on a rebuilt runner.
- **Evidence examined**: EVD-078, EVD-079, EVD-080
- **Result**: **Supported**
- **Conclusion**: N13 is closed. The fix captures the primitive part leading coefficient before SquareFreeUnivariate and folds it into Content, leaving NormalizeUnivariate untouched as DEC-007 required. 16 of 18 corpus entries satisfy expand(factor(p)) == p with 0 wrong answers.
- **Process lesson**: three dispatches on this defect. The two that failed were given the OBJECTIVE and told to search; the one that succeeded was given the ROOT CAUSE and told to apply it. For a defect whose location is unknown, dispatch the diagnosis or read the source in-house; for one whose location is known, dispatch the fix. Conflating the two cost roughly 90 minutes of agent time and two dead rounds.
- **Related**: EVD-078, RISK-007, DEC-007

### RISK-008: The Symbolics suite still cannot complete

- **Risk**: N14's two hanging inputs are in the metamorphic corpus, so the Symbolics suite still blocks. Every downstream gate waits on it.
- **Likelihood**: Certain
- **Impact**: High
- **Evidence**: EVD-080
- **Mitigation**: fix N14 next using the method that worked for N13 - locate the non-terminating loop by SOURCE READING first, then dispatch a fix with the located loop quoted. Prime suspect: GcdUnivariate (Polynomial.cs:338-349), whose Euclid loop normalises every step, driven by Yun's algorithm in SquareFreeUnivariate.
- **Gate**: G4
### VAL-018: N14 root cause CONFIRMED by source reading - the loop's terminal test is too narrow

- **Target**: N14, the non-terminating factor() on negative-leading repeated-root inputs
- **Method**: read Yun's algorithm and the gcd it depends on; derived the invariant that makes the body a no-op; checked the prediction against the observed hang set.
- **Evidence examined**: EVD-081, EVD-082
- **Result**: **Supported**
- **Conclusion**: line 380 tests 'while (!b.IsOne && !b.IsZero)'. A polynomial whose Yun state reaches the constant -1 satisfies both and re-enters a body that provably changes nothing: gcd(constant, d) is the monic 1, so b is unchanged, c is unchanged, and d loses only a constant's derivative, which is zero. The loop is invariant. This is a confirmed root cause, not an inference, and it predicts the exact hang set.
- **Related**: EVD-081, EVD-082

### DEC-008: The minimal fix is a degree test, not a value test

- **Decision**: terminate the loop on DEGREE rather than on the value 1 - i.e. 'while (b.TotalDegree > 0)' (TotalDegree already exists at Polynomial.cs:124 and returns -1 for zero). The terminal constant must be reported rather than silently dropped: it is the residual content/sign, and FactorPoly now folds leading coefficients into Content, so whichever route is chosen must not double-count or lose it.
- **Rationale**: the defect is that the terminal test excludes a legal terminal state (-1) while the body has no way to make progress from it. A degree test admits every constant as terminal, which is the mathematically correct stopping condition for Yun's algorithm.
- **Alternatives considered**: special-casing -1 (rejected: any nonzero constant other than 1 can reach the same state, for example -2 or 1/3 - the value test is the bug, not the set of values it misses); changing GcdUnivariate to stop normalising (rejected: DEC-007 - it is load-bearing for the Euclid loop).
- **Related**: EVD-081, DEC-007
### VAL-019: N14 closed - the kernel factoring path is now correct and terminating

- **Target**: N14, the non-terminating factor() on negative-leading repeated-root inputs
- **Method**: dispatched a fix with the root cause SUPPLIED (found by orchestrator source reading the round before), then re-ran the full probe set myself on a rebuilt runner.
- **Evidence examined**: EVD-083, EVD-084, EVD-085
- **Result**: **Supported**
- **Conclusion**: the guard is now 'while (b.TotalDegree > 0)'. All eight probes return and round-trip, including the two that previously hung. The Symbolics suite completes again (the Implementer reports 435 passed / 6 pre-existing oracle skips / 0 failed). Factor.cs needed no change, which confirms the diagnosis: the terminal constant was already folded into Content by the N13 fix.
- **Process confirmation**: this is the second defect in a row fixed on the first dispatch once the root cause was located in-source first, after two earlier dispatches that were told to search and produced nothing. Two diagnoses, two one-dispatch fixes.
- **Related**: EVD-083, VAL-018, VAL-017

### RISK-008 (closed): the Symbolics suite completes again

- **Resolution**: the two hanging corpus entries no longer hang, so the suite runs to completion. Closed at Round 20.
- **Related**: EVD-083, EVD-080
### VAL-020: Item 6 partially closed - the exposure gap and negative exponents are fixed

- **Target**: P1 item 6, real-only limitations surfaced as errors rather than a capability statement
- **Method**: dispatched a bounded fix for the two concrete halves, then re-ran eight probes myself on a rebuilt runner.
- **Evidence examined**: EVD-086..EVD-089
- **Result**: **Supported** for the two halves attempted
- **Conclusion**: the three hyperbolic functions are exposed (117 to 120 builtins) even though the mathematics already existed in the kernel, and negative integer exponents now evaluate exactly as rationals. 0^-1 keeps a typed recoverable error but now names the base rather than the exponent, closing N2.
- **Still open within item 6**: sqrt(-1) and (-1)^(1/2) remain explicit unsupported operations, by design (decision D5 scoped complex roots out, following the section D root-representation decision). The machine-readable capabilities() statement recommended in D5 is still NOT implemented, so an agent still discovers these limits by calling and failing. Item 6 therefore stays OPEN, not closed.
- **Related**: EVD-086, EVD-089, N2
### VAL-021: Item 6 CLOSED

- **Target**: P1 item 6, real-only limitations surfaced as a capability statement rather than only as per-call errors
- **Method**: dispatched the capabilities() builtin, then read its structure off a runner rebuilt from source and compared every advertised code and category against the live errors I had measured myself the round before.
- **Evidence examined**: EVD-090, EVD-091, EVD-092
- **Result**: **Supported**
- **Conclusion**: both halves of item 6 are now done. The exposure gap and negative exponents were closed in Round 21 (EVD-086..EVD-089); the structured statement is closed here. An agent can now read what is unsupported, with the exact code and category the failure will carry, instead of tripping over each case.
- **Design note worth keeping**: the brief's suggested snake_case codes do not exist in the runtime. Rather than advertise them and be wrong, the implementation carries the descriptive names as operation_class and publishes the real observed codes. That is the honest resolution and it is the reason the code-match claim could be verified rather than taken on trust.
- **Related**: EVD-090, EVD-089, EVD-086
### VAL-022: Item 7 CLOSED - a value the engine creates is expressible back into it

- **Target**: P1 item 7, abs(x) cannot be written although the rewriter produces it
- **Method**: dispatched the fix, then ran the probes myself on a runner rebuilt from source, comparing canonical forms rather than printed text.
- **Evidence examined**: EVD-093..EVD-096
- **Result**: **Supported**
- **Conclusion**: abs(x) carries the identical canonical form to what simplify_full(sqrt(x^2)) produces under x real, so the round-trip is closed by structural equality, not by a string match. diff(abs(x),x) returns a piecewise that is honest at 0 instead of a wrong closed form, and numeric abs plus substitution are unchanged.
- **Correction to my own brief**: I told the implementer the rejection lived in the plugin's symbolic path. It does not - abs is a CORE builtin (Interpreter.cs:1264-1277) and SymbolicsPlugin.cs never registers it. The implementer found this and argued against shadowing it from the plugin (it would clobber numeric abs, cannot rebuild AbsArray results, and would change precision scope). That is a better-scoped fix than the one I specified, and it is recorded rather than smoothed over.
- **Related**: EVD-093, EVD-094
### N15 (new, P0-class display correctness): the printer mis-rendered nested subtraction

- **Fact**: before this round the pretty printer rendered `x - (y - a)` as `x - y - a` and `a - (x + y)` as `a - x + y`. Those strings PARSE TO DIFFERENT EXPRESSIONS than the value they describe.
- **Why it matters**: this is worse than the cosmetic doubling that item 8 named. Item 8 was filed as a readability defect; the round-trip property revealed that the same precedence logic also dropped parentheses that are semantically required. A display string that re-parses to a different value is a correctness defect, not cosmetics.
- **Evidence**: EVD-098
- **Status**: FIXED in the same round (Printing.cs:688, right operand of minus, precedence 1 to 2).
- **Related**: EVD-098, EVD-097

### VAL-023: Item 8 CLOSED - and it was worth more than the item claimed

- **Target**: P1 item 8, redundant parentheses in the pretty printer
- **Method**: dispatched with a round-trip acceptance property (parse(pretty(e)) must canonicalise identically) rather than a string expectation, then verified the renderings myself on a rebuilt runner.
- **Evidence examined**: EVD-097..EVD-100
- **Result**: **Supported**, and it surfaced N15
- **Conclusion**: all three named renderings are now minimal and correct, and the printer additionally stopped emitting text that parses to the wrong value. The decision to make the acceptance a PROPERTY rather than a list of nicer strings is what found N15 - a string-comparison test would have passed on the fixed output while leaving nested subtraction broken.
- **Related**: EVD-097, EVD-098, N15
### VAL-024: Item 9 CLOSED by measurement, not by guessing

- **Target**: P2 item 9, AssumptionSet.Add quadratic in atoms
- **Method**: required a before measurement first; the measurement confirmed an exponent of about 2.1, which justified the fix path; then I re-ran the two 324-matrices by name on the changed tree.
- **Evidence examined**: EVD-101..EVD-104
- **Result**: **Supported**
- **Conclusion**: the quadratic term is gone - the rendering is computed once per atom and insertion is a binary-search splice instead of a whole-array re-sort. 800 domain atoms went from 424 ms to 7.9 ms (53x) and 800 relation atoms from 512 ms to 34 ms (15x). Both 324-matrices pass by name. The residual exponent of about 1.8 is the O(n) Contains scan plus the immutable array copy per insertion, which the Implementer documented as unimproved rather than claiming it away.
- **Why this is the right shape**: the brief permitted either the fix or a pinned bound. Cycle 2 declined to restructure because it had measured and judged real workloads small; this round re-measured and found the exponent worse than the doubled-from-cycle-2 figure, so the fix was justified on evidence rather than on tidiness. The differential test against the replicated OLD algorithm across 50 permutations is what makes the ordering claim checkable instead of asserted.
- **Related**: EVD-101, EVD-102, EVD-103
### VAL-025: Item 10 CLOSED - the two genuine metamorphic gaps are covered

- **Target**: P1 item 10, the two section 116 properties that were missing or numeric-only
- **Method**: dispatched with an explicit prohibition on numeric substitution standing in for a symbolic claim, then ran the new tests and read their names myself.
- **Evidence examined**: EVD-105..EVD-108
- **Result**: **Supported**
- **Conclusion**: both gaps are closed by real tests. The matrix identity is decided by exact polynomial arithmetic under the kernel's own emitted condition, and the cancellation property is asserted away from poles with the exclusion represented structurally.
- **Why this is not a tautology**: the implementer proved falsifiability with a negative control - an inverse that is numerically correct at one point but symbolically wrong passes the OLD test and fails the new one (EVD-106). That is the standard the harness sets for a test that must fail against a stub, and it was met without being asked for explicitly.
- **Honest labelling**: both tests PASSED IMMEDIATELY because the capabilities already existed; this round added coverage, not failing-first fixes. The implementer labelled that plainly rather than manufacturing a failure.
- **Related**: EVD-105, EVD-106, N16

### N16 (new, P2): two cancel() limitations found while closing item 10

- **Fact**: (1) the bare cancel builtin attaches no condition, so a caller is not told the cancellation is only valid off the pole; (2) cancel((x-1)^2/(x-1)) does not reduce, because Polynomial.TryFromExpr rejects a power of a sum, while the expanded spelling does reduce.
- **Evidence**: EVD-108, characterised by Cancel_UnexpandedPowerOfASum_IsNotReduced_KnownGap
- **Priority**: P2 - neither is a wrong answer; the first is a missing condition on a convenience path, the second is an incomplete reduction that the *_full path reports honestly
- **Mitigation**: decide in a later round whether the bare builtin should carry the condition, and whether TryFromExpr should accept a power of a sum. Both are now pinned by characterization tests so a fix will be visible.
- **Related**: EVD-108
### VAL-026: Item 11 CLOSED - the falsification gate is wider and registry-driven

- **Target**: P1 item 11, falsification breadth limited to interior / near-zero / large-magnitude with hard-coded rule ids
- **Method**: dispatched with an explicit requirement that the rule set come from the registry and that the strictness property survive; then ran the gate tests and read their names myself.
- **Evidence examined**: EVD-109..EVD-112
- **Result**: **Supported**
- **Conclusion**: five new regions with documented near-feature points, a registry-driven sweep asserted at 9/9/9, and both negative controls green - the gate still fails on a throwing rule and on a wrong rule. The legacy points are a proven subset, so the widening is a superset rather than a replacement.
- **Honest gaps recorded by the implementer, not hidden**: the log/exp sweeps exercise reals only, and the complex sweep reaches 5 of the 9 rules. Both are stated in the report rather than implied complete.
- **Cost**: the Symbolics suite went from about 13 s to 15 s, well inside the 60 s ceiling I set, so no point count was reduced.
- **Related**: EVD-109, EVD-110, EVD-111
### VAL-027: Item 13 CLOSED - LaTeX shipped without becoming a second printer

- **Target**: P2 item 13, the LaTeX printer deferred twice
- **Method**: required implementation as a PrintMode arm of the existing printer with an explicit ban on a duplicated precedence table; then ran the renderings and the golden suite myself.
- **Evidence examined**: EVD-113..EVD-115
- **Result**: **Supported**
- **Conclusion**: LaTeX is a fourth arm of one printer, with no duplicated precedence rule, and Canonical/Pretty output is byte-identical (Run.Tests goldens 39/39, PrettyParenthesesTests green). The item's own condition - never a second printer that can disagree - is met structurally rather than by convention.
- **Why the parity test is real**: it does not grep for command names. It denoises both renderings into structural trees with two independent readers and asserts the nesting is identical over a 34-row precedence-sensitive corpus, and a second test removes each delimiter pair and fails if the parse does not change - a test that demonstrably failed four times during development, which is the only evidence that a test can fail at all.
- **Stated gaps, not hidden**: symbol-name escaping is not done, and the parity corpus excludes the piecewise/logic/calculus arms.
- **Related**: EVD-113, EVD-114, EVD-115
### VAL-028: Item 17 CLOSED for the 51 named warnings

### VAL-036: Both remaining actionable below-A+ items are closed

- **Target**: the 20 residual analyzer warnings, and LaTeX symbol-name escaping
- **Method**: dispatched both (disjoint files), then ran a forced full rebuild and the escaping probes myself.
- **Evidence examined**: EVD-146, EVD-147
- **Result**: **Supported**
- **Conclusion**: the build is now 0 warnings / 0 errors on a forced full rebuild, and every LaTeX special is escaped. The underscore decision - escaped literal rather than subscript - is justified on the grounds that a symbol name is an opaque atom and a subscript would be undefined for _x, x_, a__b and x_1_2.
- **Related**: EVD-146, EVD-147

### DEC-010: The goal is blocked on resources outside this environment

- **Decision**: mark the goal blocked after 37 rounds, rather than continuing to spend rounds on work that cannot advance.
- **Blocking condition**, unchanged for the last four rounds: all three remaining below-A+ categories require something this agent does not have. (1) The cycle is uncommitted on 35e2609 - a maintainer decision, with OQ-007 asking whether the P0 wire revisions should land as their own revertible commit. (2) The SymPy oracle CI job has never run - it needs a GitHub runner. (3) The Studio UI has never been loaded in a browser from this environment.
- **Why this is not merely difficulty**: there is no remaining piece of the work I can advance. All 19 brief items are closed, all audit defects are fixed, N16 is closed, the build is clean, and the below-A+ list has been reduced to exactly the three items above.
- **Related**: RISK-005, OQ-007, EVD-146, EVD-147
