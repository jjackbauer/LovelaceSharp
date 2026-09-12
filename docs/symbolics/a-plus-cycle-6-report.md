# A+ Convergence — Cycle 6 report

**Repository**: `jjackbauer/LovelaceSharp` · **HEAD at writing**: `b009dfe` (pushed; `origin/main` matches)
**Commits this cycle**: `98a9049`, `70241dc`, `300f6bb`, `1b70a32`, `e8638c0`, `d4d7ccf`, `b009dfe`
**Harness memory**: `docs/goal-cycle-6/{goal,journal,evidence,state,deliverables}.md`
**Amendment**: section **P** at `docs/symbolics/a-plus-cycle-6-amendment.md` (summarised in the plan)
**A+ is not claimed.** Three defects are open and are named in §4 rather than graded around.

---

## 1. What this cycle was handed

A red CI (three jobs; the last green run the previous cycle had confirmed was #15, and twelve commits
had been pushed without re-checking), four open rows each located but unclosed, three interrupted
worktrees with no report, and two audit waves on disk. Everything below was re-measured; nothing was
inherited.

## 2. Definition of done

| ID | Dimension | Status | The command and what it printed |
|---|---|---|---|
| **D0** | CI green on a GitHub runner | **MET** | push then read the run: **#28** on `70241dc` — `Differential oracle (SymPy installed)` **success**, `Native AOT publish + runner smoke` **success**, `Fast accuracy test suites` **success**, its step 5 running the whole 14-project loop plus both Real.Tests steps in **435 s**. Run **#27** on the intermediate commit (which still carried the stale pin) is `failure`, so the two bracket the fix. EVD-237, EVD-238, EVD-248 |
| **D1** | Zero open P0/P1 from a **fresh** adversarial audit | **NOT MET** | the round-3 falsifiers (two agents, identical prompt, independent scratch trees) each broke the round's claim and produced **P-B1**; the bound re-probe produced **P-B2** and **P-B3**. All three are open. The full four-persona fresh audit was **not run** — see §6 |
| **D2** | Every Tier-0/Tier-1 defect closed by a test that fails on the pre-fix tree | **3 of 4 rows** | row 2: 9 of 11 new cases fail on a pristine control tree, and the CLI probes go from 4104/27 658/4153 ms with `ok:true` to 284/362/268 ms with `Cancelled`; rows 3+4: 50 control-tree failures; row 1: 13 of 32 cases fail on the control tree, but its **second route is open** (P-B1) |
| **D3** | Every residual bound closed with evidence or accepted in the maintainer's words | **PARTIAL** | section P written: nine rows CLOSED with evidence; three rows OPEN as defects; seven rows are scope decisions put to the maintainer **twice in writing** with no answer recorded, so none is marked ACCEPTED |
| **D4** | The final tree re-measures green | **MET except one flaky case** | forced `--no-incremental` rebuild **0 warnings / 0 errors**; full sweep with `LOVELACE_REQUIRE_SYMPY=1` **passed=5298 failed=1 skipped=0** (the one failure is a load-sensitive `Lovelace.Run.Tests` case that passes 3/3 standalone — §5); AOT publish exit 0, 0 warnings, binary **357.5 s newer** than the newest source file; the five CI smoke scenarios **SMOKE FAILURES: 0**; capability honesty **MATCH=19 MISMATCH=0**; printer round-trip through the **published AOT binary** `ok=30 bad=0 other=0`; `Lovelace.Real.Tests` unfiltered **2489/0** |
| **D5** | Every claim traces to an EVD row I reproduced | **MET** | `docs/goal-cycle-6/evidence.md` EVD-237…EVD-259; every number in this report is from a transcript cited there |

## 3. What closed

1. **The CI breakage.** The `fast-tests` job was red from run #18 to #26 because the round that made
   `solve()` return the same `SolveResult` record `solve_full()` does reworded the builtin's summary and
   its declared return kind, and updated the Suite-side help test but not
   `Lovelace.Console.Tests/ReplOutputTests.cs`, which still pinned "Solves an equation for x." and
   "Returns: Vector | Text". The pins now assert what the product declares, whole-line, and reverting
   either product string fails the test. Reproduced on Windows and on Linux (WSL Ubuntu 24.04, .NET
   10.0.401) before being fixed.
2. **Row 1, first route** (`e8638c0`): `RealLiteral` now carries the route it was built from, so a
   truncation that leaves the numeric tier and comes back through `FromRealExact`/`ToReal` stays
   inexact. The wire went from `evalf(sin(pi(30)/6), 40)` = `0.499…999` `exact:true` with a 30-digit
   rational form to `exact:false` with no numerator/denominator, with the exact controls unchanged.
3. **Row 2** (`b009dfe`): the numeric kernels observe `--cancel-after`; a cancelled statement raises
   `EvaluationCancelledException` and both envelopes carry a `cancellation` ledger reporting budget,
   elapsed, stopped, exceeded and excess.
4. **Row 3** (`d4d7ccf`): `evalf(f, digits)` refuses a count it cannot honour as a recoverable
   `InvalidArgument/TypeMismatch` instead of answering `0.8`/`0`-declared-exact, and `digits = 1` still
   answers.
5. **Row 4** (`d4d7ccf`): `capabilities()` no longer advertises a class whose trigger succeeds, names
   the narrowed class, and lists the three reachable refusals that were missing. Nineteen advertised
   entries, **19/19** reproduce their advertised code and category.
6. **Section-O bounds that are no longer true**: O-B7, O-B8a…i, O-B10 (second half), and the
   under-claim inside O-B14 — each re-measured on this tree, each CLOSED in section P with its probe.

## 4. What is open — and why A+ is not claimed

| # | Defect | The measurement | Class |
|---|---|---|---|
| P-B1 | The **special-angle table** hands an exact value to an inexact argument: `evalf(cos(pi(30)), 100)` crosses as `-1`, `exact:true`, `-1/1`, while the same expression's symbolic node is `exact:false` and mpmath gives −0.999…87355374… (a difference at the **61st decimal**). The route is `ComplexMath.SinCosAtPrecision`, and two of the project's own tests pin the current behaviour, so closing it is a contract change | EVD-251, EVD-259 | **P1** |
| P-B2 | **Deep user-function recursion still kills the process**: `f(436)` exits `0xC00000FD` with **0 bytes on stdout** (O-B11). Cycle 5 closed exactly this class for nested *input* | EVD-254 | **P0** |
| P-B3 | `1/(3*10^1000)` is returned as `0`, and `evalf(1/(3*10^1000), 30)` as Integer `0` **declared exact**; mpmath gives 3.33e-1001 (O-B10, first half) | EVD-255 | **P1** |

Seven further bounds (O-B1…O-B6, O-B9, O-B12) are scope decisions. The maintainer was asked twice, in
writing, with the measured behaviour quoted; **no answer was recorded**, so section P marks them OPEN
and none is claimed as accepted. If the maintainer accepts them, D3 becomes MET; if any is rejected,
each is a round of work with a located home.

## 5. The one blemish in D4

Two full sweeps report `Lovelace.Run.Tests passed=190 failed=1` while the same project run standalone
reports exit 0 three times out of three (with and without `LOVELACE_REQUIRE_SYMPY=1`). The failing case
is not yet identified; the sweep script only writes a raw log when its summary regex fails, which is a
defect in my own harness that I am recording rather than papering over. The class is the one the cycle
already carries twice (RISK-002, RISK-003: wall-clock assertions inside instrumented runs). CI is the
arbiter: run #31 on `b009dfe` decides whether it is load-sensitive or real.

## 6. Method, and where it was adapted

- **The harness ran as designed for rounds 1–3**: one narrow objective per round, recorded before
  dispatch; an Implementer plus two Falsifiers on identical prompts for the claims that gate a
  dimension; my own reproduction of every number before it entered `evidence.md`.
- **The falsification gate earned its keep.** Round 3's claim ("row 1 is closed") was **Falsified by
  both falsifiers**, independently, with the same counterexample from different routes; I reproduced it
  and quantified it against mpmath. Round 1's claim survived two full 16-command CI replicas.
- **Three agents were stopped mid-flight** (rounds 4, 4b and 7). Their work was preserved as patches
  and file copies under `docs/goal-cycle-6/round-4/preserved/` and `.worktrees/c6-ob11`, and the two
  patches that were already independently triaged (r21, r22) were applied, verified, committed and
  pushed by me (DEC-007 records why: the rule's purpose — no unverified delivery enters the record — is
  preserved, the Implementer/Falsifier separation for a mechanical patch application is what is lost).
- **The full four-persona fresh audit (D1) was not run.** The cycle did not have the round budget or a
  stable agent window left after the stops, and running it against a tree with three known open defects
  would have produced confirmation, not information — the same reasoning Cycle 5 recorded in its own
  O.3. D1 is therefore **not met**, and the report says so.
- **Two process lapses are recorded, not hidden**: round 5's objective was written to `state.md` after
  its dispatch (OBS-010), and an early patch-applicability verdict of mine was a PowerShell pipeline
  artifact that I later corrected in a superseding row (OBS-008, EVD-249).

## 7. What a reader should not conclude

- That the exactness problem is solved. One route is closed and guarded by tests that fail on the
  pre-fix tree; the wire still over-claims on the special-angle route (P-B1).
- That CI is green *by construction*. It is green on `70241dc` and on `b009dfe`'s predecessors; the
  final commit's run is cited in §2 when it completes.
- That the four rows were closed by one agent each. Two came from preserved work that had never been
  verified (cycles 5's stopped rounds), one needed a repaired assertion, and one — row 1 — turned out to
  have a second route that only falsification found.
