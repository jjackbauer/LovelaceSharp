# Harness State — cycle-3

- **Round**: 37 of 40 (goal round 36 of 40) - N16 closed, 0 warnings achieved, LaTeX escaping done. THREE below-A+ items remain and ALL need the maintainer: the commit, the CI oracle run, the browser check. Goal marked BLOCKED on those.

## ALL THREE AUDIT DEFECTS (N17, N18, N19) ARE FIXED AND VERIFIED. NEXT: THE FINAL REPORT.

- **N17 (P0-critical)**: cancellation is not observed. `--cancel-after 2000` on `2^1000000000` still running at 60,000 ms (EVD-125). The brief lists 'a cancellation request that hangs' as an acceptance-gate failure, and the section 5 do-not-regress list contains a cancellation claim that is FALSE.
- **N18 (P0-class)**: `inf - inf` = 0 and `0 * inf` = 0, both exit 0 (EVD-126). Wrong answers on the arithmetic core.
- **N19 (P1)**: `limit_full(x*sin(1/x), x, 0)` reports `exists: false`; the limit is 0 (EVD-127).

Fix these before writing the final report. N17 first: it is an explicit acceptance gate.

## Also open

N16 (two cancel() limitations, P2); 20 residual analyzer warnings in test projects; 13 audit friction rows of which F1-F4 are the substantive ones (the rest are documentation or UX observations).
- **Goal**: Close every Cycle-3 P0 contract deviation and P1/P2 gap in the LovelaceSharp symbolic runtime, verified on freshly published binaries.
- **Definition of done**: 3/9 (D1, D2, D3). **D4 (P0 items 1-5) is now closed.**
- **Stop criteria**: D5-D9 unmet. Zero Falsified rows. Cap 40.

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 56 EVD rows |
| G2 Falsification | PASS | 8 validations (VAL-001..VAL-008) |
| G3 Coverage | FAIL | D4 closed; D5-D9 None |
| G4 Reproduction | PASS | every round re-run by the orchestrator |
| G5 Honesty | PASS | RISK-003 closed; RISK-004 and OQ-002 still open and recorded |

## Objective ledger

| Round | Objective | Agents | New EVD | Gates | Outcome |
|---|---|---|---|---|---|
| 1 | Baseline + reproduction of every brief item | 3 | 29 | G3x | addendum approved |
| 2 | Runner test project + golden fixtures (A5) + CI wiring | 1 | 7 | G3x | verified |
| 3 | Structural solver emission (A1, A2, N3, N4, N5) | 1 | 4 | G3x | verified |
| 4a | Enum value kind (D2) | 1 | 5 | G3x | verified |
| 4b | Structured diagnostics on the seven rich records | 1 | 6 | G3x | verified |
| 5 | NoSolutions pairing (item 4) + arity validation (N1) | 1 | 5 | G3x | verified |
| 6 | Matrix records vocabulary + schema registry + drift corpus (N12, item 14) | 1 | 2 | G3x | verified |
| 7 | Integrated-state regression sweep (orchestrator only) | 0 | 1 | Gxxx | verified 2074/0 |
| 8 | CI-reachability audit of every test project (orchestrator only) | 0 | 1 | Gxxx | verified 15/15 |
| 9 | Re-prove the no-swallowing do-not-regress claim (orchestrator only) | 0 | 3 | Gxxx | verified 0 broad catches |
| 10 | Record RISK-005 (uncommitted work) and OQ-007 (staged commits?) | 0 | 0 | - | recorded |
| 11 | P1 item 12: loud conditional SymPy skip + widened corpus + CI job that installs sympy | 1 | 2 | G4 ok | verified; found N13 |

**All five P0 completion blockers of the brief are closed.**

## Progress against the brief

Closed: items 1, 2, 3, 4, 5, 16, plus N1 and N11.
Open: P1 items 6-12; P2 items 13, 15 (already complete), 17, 18, 19; then the final report and the section 143 audit.

## Test totals

Baseline 1976; now **2066** across 15 projects, 0 failed. Run.Tests 34 and Symbolics 436 were
re-run by the orchestrator; the rest are the Implementer's full-matrix logs in round-5/final-*.log.

## Next objective

Round 7 - P1 item 12 (the SymPy differential oracle): extend it to roots, factorization, limits and
matrix operations under explicitly matched domains, make the local skip loud and conditional on an
explicit requirement rather than a silent pass, and add the CI job that installs sympy so the
extension is verified somewhere. Then items 6-11, then P2 items 13, 17, 18, 19.

## Environment facts worth remembering

- The orchestrator's TS runtime rejects a dollar sign and a nested template literal inside a call argument.
- The read tool truncates lines at 2000 chars; chunk long single-line JSON before reading it.
- A PowerShell function parameter named after the automatic arguments variable silently breaks dotnet invocations.
- The shell is Windows PowerShell 5.1; python/python3 are Store alias stubs, so the SymPy oracle cannot run here.
## Round 11 verification outcome (do not re-do)

Verified by the orchestrator: no env var gives Skipped 6 / Passed 0; LOVELACE_REQUIRE_SYMPY=1 gives Failed 6 / Passed 0 with an explicit message; zero silent-pass paths remain. The widened corpora and the new CI job cannot execute on this machine and are exercised only by CI.

## Next objective

Round 13 - **N13, a P0-class correctness defect**: factor() returns a different polynomial when the leading coefficient is negative (factor(-x^2+1) expands to x^2-1). Fix Factoring.Factor to carry the sign, add the metamorphic property expand(factor(p)) = p over negative-leading-coefficient inputs, and reinstate the oracle case excluded at OracleCorpus.cs:129 as a live comparison (DEC-005).

Then P1 items 6-11, then P2 items 13, 17, 18, 19, then the final report and the section 143 audit. Item 15's proof bridge is already complete; item 12's local half is verified and its CI half needs a CI run to confirm.

## Open risks carried forward

- RISK-004: matrix records - CLOSED at Round 6.
- RISK-005: nine rounds of verified work remain uncommitted on 35e2609.
- OQ-002: is cancellation polled inside long kernel loops? Open since Round 1.
- OQ-007: should the P0 wire revisions land as their own revertible commit?
## N14 IS THE TOP OF THE BOARD

**`Factoring.Factor` does not terminate** for a negative-leading polynomial with a repeated root:

    x = symbol("x"); factor(-x^2 - 2*x - 1)       -> never returns (killed at 15 s)
    x = symbol("x"); factor(-x^3 - x^2 + x + 1)   -> never returns (killed at 15 s)

Eight sibling inputs return immediately, including other negative-leading cases and the
positive-leading repeated-root control `x^3 - 3*x^2 + 3*x - 1` -> `((x - 1))^3`.

This is reachable from the published binary with one expression: a user-facing hang, worse than N13,
and it blocks the Symbolics suite (the corpus contains both inputs) and every downstream gate.

### Fix N13 and N14 together - same code path, same trigger

Acceptance for the combined fix:
1. both hanging inputs return;
2. `expand(factor(p)) == p` over all 18 corpus entries in
   `Lovelace.Symbolics.Tests/FactorMetamorphicTests.cs`, negative cases included;
3. the positive-leading controls keep their existing output (`factor(x^2-1)` = `(x - 1)*(x + 1)`,
   `factor(x^3 - 3*x^2 + 3*x - 1)` = `((x - 1))^3`);
4. the Symbolics suite completes again (it took 14-21 s in rounds 2-11);
5. the oracle case excluded at `OracleCorpus.cs:129` is live again (DEC-005).

**Keep `FactorMetamorphicTests.cs`.** It is a well-formed failing test that exposed a real kernel hang;
declared fixes must be proven against it, and it must never be deleted to restore green.

## Standing state

Closed: items 1, 2, 3, 4, 5, 14, 16; N1, N11, N12; the CI-reachability gate; the no-swallowing
re-proof; item 12's local half (EVD-064).

Open: **N14 (P0-critical hang)**, **N13 (P0 wrong sign)**, P1 items 6-11, P2 items 13, 17, 18, 19,
the final report, and the section 143 audit.

Carried: RISK-005 (everything uncommitted on 35e2609), OQ-007, OQ-002.
## N13/N14 NEXT ATTEMPT - CHANGE THE APPROACH, NOT THE WORDING

Two dispatches failed on this fix (EVD-072). The harness names re-dispatching the same objective
with the same prompt as an anti-pattern. The next attempt must be shaped differently:

**Known targets (EVD-071):**
- `Lovelace.Symbolics/Algebra/Factor.cs:11` - `static class Factoring`; `:14` - `public static Expr Factor(Expr e, ExprContext ctx)`.
- `Lovelace.Symbolics/Algebra/Polynomial.cs:353-355` - the comment reads 'make monic (leading
  coefficient 1)' and then reads `LeadingCoefficient(MonomialOrder.Lex)`. Prime suspect for the
  dropped sign: monic normalisation that is never compensated by a leading constant.

**Prescribed first step: diagnose, do not fix.** Write a tiny in-process xUnit case that calls
`Factoring.Factor` on `-x^2 - 2*x - 1` and `-x^3 - x^2 + x + 1` with a hard timeout, and find the
loop that does not terminate. Report the file:line of that loop and the state that makes it
non-terminating. Only once that is written down should the fix be attempted. A fix proposed
without the located loop is what failed twice already.

**Then the five acceptance points** (unchanged, in state.md below): both hanging inputs return;
`expand(factor(p)) == p` over all 18 corpus entries; positive controls unchanged; the Symbolics
suite completes (14-21 s, ~437 passing); the oracle case at `OracleCorpus.cs:129` is live again.
**Keep `FactorMetamorphicTests.cs`** - never delete or skip it to reach green.