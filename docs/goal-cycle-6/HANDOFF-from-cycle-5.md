# Handoff from cycle 5 to cycle 6 — state at `7f69f54`

Written when the cycle-5 orchestrator stopped the three rounds that were still in flight, so the next
session starts from a **defined** tree rather than a half-edited one.

## What was stopped, and what it left

Three implementer rounds were interrupted mid-flight. None of them wrote a `c5-report.md`, so **none of
their work may be believed** — but it is preserved, unverified, rather than thrown away:

| Worktree | Scope it was given | Preserved as | Size |
|---|---|---|---|
| `.worktrees/c5-exact2` | `RealLiteral.FromRealExact` drops provenance (`Expr.cs`, `Evaluation.cs`, `Constructors.cs`, `Real.cs`) | `patches/wip-r20-exact2-UNVERIFIED.diff` | 35 792 B |
| `.worktrees/c5-cancel` | `--cancel-after` ignored inside the array/numeric kernels | `patches/wip-r21-cancel-UNVERIFIED.diff` | 59 174 B |
| `.worktrees/c5-caps` | `evalf(f, 0)` internal invariant failure + `capabilities()` honesty | `patches/wip-r22-caps-UNVERIFIED.diff` | 170 137 B |

Each patch was produced with `git add -A ; git diff --cached --output=…` from the worktree and then
unstaged; the worktrees still hold the same content. **None of it was applied to the main tree, none of
it was tested, and none of its tests were seen to pass.** Treat each as a hypothesis with a head start:
re-diagnose, re-implement if it does not hold up, and verify from scratch. The worktree bases are
`7f69f54` (the main HEAD at the time).

## The CI failure: cause still unknown — and one tempting lead that does NOT hold

The maintainer reports CI red. The last run the cycle-5 orchestrator personally confirmed green was
**#15 on `8554854`**; twelve commits were pushed afterwards without re-checking
(`9e761ba`, `a0a70a6`, `86a5fc8`, `49f5a4c`, `e764cb5`, `952c761`, `40b96d9`, `8aba7ee`,
`d8f03d4`, `0a75c3f`, `1aa7182`, `7f69f54`). The failing job and test are **not known**.

A parallel investigation left untracked files in `docs/goal-cycle-6/ci-repro/` (a suite loop and its
logs). Its `summary.txt` records `Lovelace.Complex.Tests EXIT=1` — **do not chase that**: every log in
that directory for that suite, plain and coverage, ends `Passed!  - Failed: 0, Passed: 96`, and the
cycle-5 orchestrator re-ran the same suite on the same tree and got `96/0` as well. The `EXIT=1` is a
probe artifact (most likely `$LASTEXITCODE` read after a pipeline), not a reproduced failure. Suspect
the probe first.

Read the real failure from the Actions run — the three jobs are `fast-tests`, `sympy-oracle`,
`aot-smoke` — and fix it as the next session's **first** round, before any other work.

## The four open rows (each reproduced and located)

1. `RealLiteral.FromRealExact` (`Lovelace.Symbolics/Expr.cs:82`) drops provenance, so
   `evalf(sin(pi(1)*1/6), 40)` reports `exact:true` with a rational form.
2. `--cancel-after` is ignored inside the array/numeric kernels (`sum(1..10000000)` with a 1 ms budget
   ran 4.9–6.0 s and returned `ok:true`; no field reports the overrun).
3. `evalf(f, 0)` raises `InternalError/InternalInvariantFailure`; `evalf(1/3, 0)` returns 0 declared
   exact; `evalf(sqrt(2), 0)` ignores the count.
4. `capabilities()` under-claims `(-4)^(1/2)` and omits three reachable refusals
   (`solve.unevaluated`, `system-solve.unevaluated`, `integration.no-closed-form`).

Also recorded and not fixed: `Real.Sin` of a periodic argument ignores the period; three special-angle
lines now show true residuals instead of a hard zero (deliberate, documented); `ComplexMath`'s series is
quadratic in the ambient precision.

## The tree itself is consistent

`local == origin/main == 7f69f54`; the main tree has no modified tracked file; the three in-flight
worktrees are the only places with uncommitted work. The full cycle-5 record — evidence `EVD-201…EVD-236`,
the journal (including `OBS-016`/`DEC-009`, the round where the memory fell behind), the two audit waves
and `audit1-triage.md` — is committed under `docs/goal-cycle-5/`.
