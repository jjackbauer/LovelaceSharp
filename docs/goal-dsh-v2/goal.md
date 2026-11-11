# Goal — DSH v2.0 reliability, autonomy and engineering-efficiency requirements

## Verbatim request

"<user pasted the DSH Reliability, Autonomy & Engineering-Efficiency Requirements v2.0 (2026-11-11) and invoked /stretch-goal-harness>"

## Restated outcome (one sentence)

A DSH checkout accepts a goal or task as complete only when revision-bound, independently reproducible acceptance evidence for its frozen contract passes, and every proposed requirement is either verified as already shipped, implemented and proven by a named acceptance fixture, or recorded as an approved exception.

## Definition of done (falsifiable)

1. Every requirement ID in the pasted v2.0 document (GOV-001..007, DEL-001..009, VER-001..011, MET-001..006, EFF-001..010, SAF-001..005, EVAL-001..005) has a recorded disposition: shipped-verified (with file:line), implemented-this-cycle (with a passing fixture), or approved-exception (owner, reason, expiry).
2. Each of the twelve acceptance tests in section 11 runs as an automated fixture in the DSH checkout and passes: missing artifact rejected at the integration barrier; parent complete while a child runs leaves the root open; invariant violation rejected before merge; 212/214 corpus green while held-out cases fail keeps semantic parity red; a 100%-mapped-to-one-test manifest cannot claim equivalence; a missing mutation anchor fails closed; a scorecard that disagrees with the ledger fails; a 2x host drift withholds the speedup; three identical failing commands force diagnosis; a nested child cannot exceed inherited permissions; replay after crash preserves goal truth; single-agent vs gated multi-agent comparison reports time-to-accepted.
3. The scorecard reports `not-run` / `failed` / `passed` / `waived-by-human` / `unknown` distinctly, with the denominator named for every headline number.
4. Gate G1-evidence, G2-falsification, G3-coverage, G4-reproduction and G5-honesty pass for the cycle.

## Phase-0 finding that changes the plan (verified in this checkout)

The v2.0 document's central P0 claim - that `Goal.complete()` "uses the normal revision/phase transition and has no required external acceptance evidence" - is FALSIFIED by the current checkout:

- `packages/goal/goal/src/index.ts:512-529` - `complete()` resolves the persisted acceptance contract, requires an intent trace when anchors exist, calls `requireCompletionGates`, and only then commits.
- `packages/goal/goal/src/index.ts:693-729` - `requireCompletionGates` marks a mandatory gate with no live registration `unresolved` (`GOAL_ACCEPTANCE_GATE_MISSING`), evaluates every registered check once, and throws `GoalCompletionBlockedError` with the blocked list without committing anything.

Therefore acceptance fixture GATE-01 already holds at this revision, and M1's "authoritative completion" is at least partially shipped. The remaining work must be re-derived against this code, not against commit e8f7b93 as the document assumed.

## Scope decision required (blocking)

This goal lives in a different repository (`C:\Users\ricar\deepseek-harness`) and is a different objective from the active LovelaceSharp parity goal (manifest anchor at 92.12 %, 371 rows deferred, scorecard 9/9 green). The two cannot both be the active goal. The user must choose:

- **A. Switch** to the DSH v2.0 goal (Lovelace paused at its current green state, resumable).
- **B. Apply** the v2.0 document as the acceptance standard for the LovelaceSharp run (red-team the harness on it, do not modify DSH).
- **C. Sequence**: finish the LovelaceSharp manifest anchor first, then start the DSH goal.

## Constraints (provenance)

- Do not modify LovelaceSharp to make a DSH test pass (document section 0.5: "This is a test of DSH, not a requirement to modify LovelaceSharp itself"). [source: user document]
- Preserve shipped DSH services; extend by policy and evidence, never replace the goal service, Team board, subagent manager or session projections. [source: user document]
- No requirement may be marked implemented because an extension point exists. [source: user document]
- Every claim about current DSH behaviour cites file:line in the checkout at its pinned SHA. [source: harness axiom]

## Stop criteria

Definition of done satisfied with all twelve fixtures green and every requirement dispositioned, or the user approves a reduced scope in writing. Round cap: 12 rounds.

## DEC-001 (owner: user, status: approved) — the v2.0 document governs THIS migration, not a separate DSH repository goal

**User's verbatim answer:** "it isnt a different repo. it is about the rust migration..."

**Decision:** The DSH v2.0 requirements are the acceptance standard for the LovelaceSharp Rust migration that is already in flight. No work is to be done in the deepseek-harness checkout; the requirement IDs are applied to this workspace, its parity harness and its completion claims.

**Consequences (recorded as binding):**
1. The migration may not be declared complete on mapped coverage alone. Each row's claim must name its evidence class: `mapped` (a Rust test covers the same case), `executed` (the test ran), `passed`, and `equivalent` (the assertion reproduces the C# assertion, not merely a similar one). The final report must state all four separately (VER-002, VER-008).
2. Completion is gate-bound: the scorecard (D1..D10) plus the manifest anchor must both be green, and the gate must fail closed on a missing or stale artifact (GOV-003, VER-006).
3. Every headline number names its denominator and whether it is measured, inferred or self-reported (MET-002, MET-006).
4. Exceptions require an owner, a reason and an expiry; the 118 delegated exclusions already carry `owner = "user"` and are listed in coverage.md (GOV-005).
5. The twelve acceptance tests in the document's section 11 are reinterpreted as this repo's fixtures: missing artifact → manifest row whose test does not exist must fail the gate; a 100%-mapped-to-one-test manifest → the gate must reject a trivially-shared mapping; a missing mutation anchor → GATE-011/008 already fail closed; a stale scorecard → scorecard.json must be regenerated before any completion claim.

**Open obligation:** the 371 deferred rows must become mapped or approved; until then the migration is `partially-accepted`, never `accepted`.
