# Goal — Cycle 3: Close the Documented Gaps (LovelaceSharp symbolic runtime)

## One-sentence outcome

LovelaceSharp at `35e2609` closes every Cycle-3 P0 contract deviation and P1/P2 gap recorded in
`docs/symbolics/a-plus-cycle-2-report.md`'s "Categories still below A+" list plus the brief's
§2–§4 items, verified on freshly published binaries, with every suite green, every test project
reachable from CI, and Native AOT preserved.

## Definition of done (each falsifiable, with a command and an observable)

| # | Dimension | Command | Expected observable |
|---|---|---|---|
| D1 | Baseline recorded | `dotnet build LovelaceSharp.slnx -c Release` + every suite + AOT publish + 5-scenario smoke | exit 0; per-suite pass/fail counts; `out/aot/Lovelace.Run.exe` newer than HEAD's source; smoke all assert |
| D2 | Every §2–§4 item reproduced | per-item command against compiled binaries | each item marked Confirmed / Already fixed / Not applicable / New adjacent issue with its observed output |
| D3 | Alignment addendum approved | `docs/symbolics/a-plus-cycle-3-alignment.md` exists, reviewed by the user | explicit approval before implementation |
| D4 | P0 items 1–5 closed | runner + suite tests | `binding` records not text; `parameter` is Symbol; `diagnostics` is a record array on all 7 rich records; `NoSolutions` pairing coherent; golden fixtures + runner test project exist and CI runs them |
| D5 | P1 items 6–12 closed | runner + suite tests | capability statements structured; `abs(x)` expressible; no double parens; Assumptions.Add bounded or pinned; metamorphic set implemented; falsification breadth extended; oracle runs in CI |
| D6 | P2 items 13–19 closed or documented | code + docs | LaTeX printer from the same expression model, or an explicit below-A+ entry |
| D7 | §5 regressions | the listed probes | every listed behaviour unchanged |
| D8 | §6 acceptance gates | CI + local | no test project outside CI; no semantic string in a full API; no unregistered builtin; AOT smoke passes |
| D9 | Final report + fresh §143 audit | `docs/symbolics/a-plus-cycle-3-report.md` | exact commit, counts, deltas, honest below-A+ list, audit on the freshly published binary |

## Hard constraints (gates, not preferences)

1. Native AOT must keep publishing and running (`-p:PublishAot=true`).
2. `a-plus-convergence-alignment-plan.md` §C–§J are frozen law. Amend the alignment doc rather
   than reducing scope silently.
3. No semantically meaningful string in a full machine API.
4. No broad `catch (Exception)` in `Lovelace.Symbolics`; no internal error surfaced as
   `Unevaluated`; no test weakened/skipped/deleted to reach green.
5. `Assumptions.cs` changes require the full Symbolics suite (324-query + 324-pair matrices).
6. One bounded change per round, verified in the same round.
7. Re-publish the AOT binary before auditing it.
8. Every claim in a report is reproducible from a command or a file:line.

## Preferences (tie-breakers)

- Prefer fixing the kernel over special-casing the projection.
- Prefer one shared projection (`Lovelace.Suite/StructuredProjection.cs`) over a second renderer.
- Prefer measuring before restructuring.

## Stop criteria

- D1–D9 all at High confidence, zero Falsified rows, zero P0 open questions.
- Round cap: 40.

## Known environment limits (cannot be closed here; must be stated, not hidden)

- No Python on this machine: the SymPy oracle cannot execute locally. The deliverable is therefore
  "a CI job that installs sympy and runs it", plus a visible (not silent) local skip.
- Benchmark means are ShortRun on a shared workstation; allocations are the reliable signal.
- The Studio UI is not browser-verifiable from this harness.
