# Goal — cycle-4 (Unblock and Converge)

> Created from `docs/symbolics/a-plus-cycle-4-brief.md`. The Cycle-3 memory under
> `docs/goal-cycle-3/` is **read-only history**; it is superseded only by explicit entries in this
> directory. Evidence numbering continues at **EVD-148** so the two runs never collide.

## Goal (one sentence)

Resolve the three Cycle-4 blockers, close or obtain written maintainer acceptance for the four
remaining below-A+ gaps, and re-establish every cycle claim on the final tree with freshly measured
evidence.

## Definition of done — falsifiable, each with a command and an expected observable

| # | Dimension | Command | Expected observable |
|---|---|---|---|
| D1 | Cycle-3 work is committed | `git status --porcelain` + `git log --oneline 35e2609..HEAD` | 5 stage commits; no Cycle-3 path left dirty |
| D2 | SymPy differential oracle has actually executed | `LOVELACE_REQUIRE_SYMPY=1 dotnet test Lovelace.Symbolics.Tests -c Release --filter "FullyQualifiedName~DifferentialOracle"` | `Passed >= 1, Skipped = 0, Failed = 0`, with the sympy version recorded |
| D3 | Studio UI loaded in a real browser | headless Chrome/Edge against a running `Lovelace.Studio` host | Screenshot + DOM dump per panel; console error list captured |
| D4 | Full suite sweep re-measured, green | all test projects in Release | `0 failed`; total recorded and compared to 2386 |
| D5 | Forced full rebuild at zero warnings | `dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental` | `0 warning`, `0 error` |
| D6 | Native AOT still publishes and runs | `dotnet publish -p:PublishAot=true` then execute the binary | Publish succeeds; smoke scenarios pass; exe newer than newest source |
| D7 | Adversarial audit re-run against D6 binaries | audit personas driven through the **re-published** exe | Every finding either fixed or recorded with a verdict |
| D8 | Section-4 gaps closed or accepted | the four items in brief §4 | Each is closed with evidence **or** carries a dated written maintainer decision |
| D9 | Final report claim→evidence traced | `docs/symbolics/a-plus-cycle-4-report.md` | Every claim cites an `EVD-nnn` row the orchestrator personally reproduced |

## Hard constraints (gates — carry over from Cycle 3, brief §5)

1. Native AOT must keep publishing and running (`-p:PublishAot=true`).
2. `docs/symbolics/a-plus-convergence-alignment-plan.md` sections C–J are **frozen law**. Amend the
   alignment document rather than reducing scope silently.
3. No semantically meaningful string in a full machine API.
4. No broad `catch (Exception)` in `Lovelace.Symbolics` (baseline 0). The single broad catch in
   `Lovelace.Suite/SuiteEngine.cs:282` records a diagnostic and rethrows; leave it.
5. `Assumptions.cs` carries a 324-query bound matrix and a 324-pair contradiction matrix; any change
   there requires the **full** suite, not just Symbolics.
6. **No test weakened, skipped or deleted to reach green.**
7. Re-publish the AOT binary before auditing it (Cycle 2 produced two false defects from a stale binary).
8. One bounded change per round, verified in the same round.

## Stop criteria

Done means: D1–D9 all met at high confidence, **zero Falsified rows**, **zero P0 open questions**.
Cycle 4 does **not** self-award A+ while knowingly leaving a category below the bar; if a category
stays open it is named in the final report with the reason. Round cap: **30**.

## Non-goals

- Not a rewrite of the symbolic kernel; the frozen alignment plan governs scope.
- Not a new protocol revision. `--omit-display` stays DECLINED (alignment addendum §13).
- Not "improving" benchmarks on a non-idle machine (brief §4 item 4).
