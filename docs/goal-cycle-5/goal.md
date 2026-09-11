# Goal — cycle-5: From "Not Claimed" to A+ (or an honest refusal, again)

## The goal, in one sentence

Make the LovelaceSharp symbolic/numeric stack defensible: close every Tier-0 and Tier-1 defect with a
test that fails on the pre-fix tree and asserts the correct behaviour against independent ground truth,
close or obtain written maintainer acceptance for every residual bound, and then claim **A+ only if a
fresh adversarial audit (new agents, new probes) cannot falsify it**.

## Definition of done — falsifiable, with the command and the expected observable

| ID | Dimension | Command | Expected observable |
|---|---|---|---|
| **D1** | Zero open P0/P1 audit findings | dispatch a *fresh* adversarial audit (new agents, new probes) against the final re-published binary | zero rows the auditors classify as P0 or P1 |
| **D2** | Every Tier-0/Tier-1 item closed by a test that asserts the *correct* behaviour | `dotnet test <project> -c Release` on the final tree, plus the same test re-run in a clean worktree at `9228305` | the new test **passes** on the final tree and **fails** at `9228305` |
| **D3** | Every remaining bound accepted in writing or closed | `docs/symbolics/a-plus-cycle-5-amendment.md` (section O) + maintainer's answer for anything left open | every item in alignment §N.3 plus every bound this cycle finds is either `CLOSED` with evidence, or `ACCEPTED` with the maintainer's own words quoted |
| **D4** | The final tree re-measures green | forced `--no-incremental` rebuild; full 15-project sweep with `LOVELACE_REQUIRE_SYMPY=1`; AOT publish + freshness + the five CI smoke scenarios | 0 warnings; 0 failed; exe newer than the newest source file; all smoke assertions PASS |
| **D5** | Every claim in the final report traces to a numbered EVD row I personally reproduced | read `evidence.md` and re-run each cited command | no claim without its EVD row |

## Independent ground truth

Mathematics is checked against **SymPy 1.14.0** at `C:\Users\ricar\dev\.lovelace-tools\python`
(python 3.12.14), the same oracle the repository already ships, plus the published AOT binary
`out/aot/Lovelace.Run.exe` for every wire-level claim. No claim about behaviour is accepted from a
summary: it is accepted from an envelope the orchestrator printed.

## Hard constraints (gates — carry over from cycles 3 and 4)

1. **Native AOT keeps publishing and running**; the binary is **re-published and freshness-checked**
   (exe newer than the newest source file) before any audit.
2. Sections **C–J** of `docs/symbolics/a-plus-convergence-alignment-plan.md` are frozen law. Change
   them by **amending** the document (Cycle 4's section N is the precedent, Cycle 5 adds section O),
   never by silently reducing scope.
3. **No semantically meaningful string in a full machine API.**
4. **No broad `catch (Exception)` in `Lovelace.Symbolics`** (currently 0 — re-verified this cycle).
   The single broad catch in `Lovelace.Suite/SuiteEngine.cs` records a diagnostic and rethrows; leave it.
5. `Assumptions.cs` carries a 324-query bound matrix and a 324-pair contradiction matrix; any change
   there requires the **full** suite, not just Symbolics.
6. **No test weakened, skipped or deleted to reach green.** A golden fixture may be regenerated only
   when the contract deliberately changed, and then only from the binary with the diff reviewed.
7. One bounded change per round, verified in the same round.
8. Do not push to any remote without asking.
9. `docs/goal-cycle-4/**` is **read-only history**. Evidence numbering continues at **EVD-201**.

## Stop criteria

D1–D5 met at high confidence, **zero unremediated Falsified rows**, **zero open P0 questions**.
Round cap: **40**.

If a category stays open at the cap, the report says so and says why — Cycle 4's refusal to grade
around its P0s is the standard this cycle is held to.

## Pre-registered honest outcomes

- **A+ claimed** only if D1–D5 all hold on the final tree.
- **Below A+ with named defects**, if a Tier-0 item cannot be closed in the round budget: the report
  names it, reproduces it, and says what it would take.
- **A+ withheld because the audit found something new** is a *successful* cycle, not a failed one.
