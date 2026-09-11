# Audit #1 triage — every P0 and P1 finding, and what happened to it

Audit #1 was the **mid-cycle** adversarial audit: four independent agents, against the published
binary of commit `9228305`, told to break the product and given no list of known defects.
It is *not* the D1 audit; D1 requires a fresh audit against the **final** binary.

| Audit | Probes | Held | Finding rows | P0 | P1 |
|---|---|---|---|---|---|
| A1 numeric tower | 214 | 140 | 74 | 8 clusters | 4 |
| A2 wire protocol | 367 | 222 | 141 | 5 | 12 |
| A3 symbolic + printer | 372 | — | — | 7 | 8 |
| A4 documentation | 235 | 171 | 54 | 0 | 6 |

The four reports are the source of truth and live in `docs/goal-cycle-5/audit1/`. This file only maps
them onto the work, so that nothing is quietly dropped.

## Dispositions

| Finding | One line | Disposition |
|---|---|---|
| A3-F1 | real-domain solver returns an extraneous root with `complete:true` (`sqrt(x)+2==0`) | **CLOSED** by `bb9746f`; re-probed by me: `Unevaluated`, `complete:false` |
| A3-F3 / A1-N-04 / A1-N-05 | `(1/17)*17`, `(1/3)*(1/3)`, `(1/97)*97` wrong | **CLOSED** by `3ffc782` (exact-fraction multiply); the audit's own 45 F1–F4 probe files re-run green |
| A2-F1 / A3-F2 | `solve_system_full` answers `NoSolutions/complete:true` for a solvable nonlinear system | round in flight (system-solve completeness) |
| A2-F30 / A3-F5 | `limit((1+1/x)^x, x, inf)` = 1 instead of e | round in flight (indeterminate limits) |
| A3-F4 | deep input kills the process, no envelope | round in flight (depth guard) |
| A1-N-01…N-03 | underflow to `0 exact:true` (`2^-100000`, `0^(-1.0)`, `1/(10^1001-1)`) | planned round (Real exactness + terminating-quotient exactness) |
| A1-N-06 / A1-N-06b | additive truncation to 0; fixed-width fast path certifies a truncated product exact | round in flight (fast path + additive exactness) |
| A1-N-07, A2-F3, A2-F4, A3-F6 | exactness labels are a shape heuristic: exact values called inexact, unevaluated records called exact, `IsExact` false for `sin(x)`/`sqrt(x)` | planned rounds (Real provenance; Symbolics exactness) |
| A2-F2 | `IntegrationResult.exactness` = Approximate for a verified exact integral | planned round (Symbolics exactness) |
| A2-F5, A3-F11 | `solve` / `solve_system` return prose where a record or a refusal is required | F11 in flight (solve_system record); `solve` short form in the wire round |
| A2-F6, A2-F7 | 39/123 builtins break the one documented arity-error shape; `transpose()` leaks a CLR exception | wire round |
| A2-F8, A4-P1-4 | error envelopes carry no `elapsedTime`/`timings` | wire round |
| A2-F9, A2-F10, A2-F17 / A4-P1-3 | print output lost on error; Void omits `result`; trailing CR in `output[]` | wire round |
| A2-F11 | `SystemSolveResult` has no `completeness` | in flight (solve_system contract) |
| A2-F12, A2-F13, A1-N-11 / A4-P1-? | `capabilities()` contradicts the binary (integer domain, unlisted refusal, non-integer exponents supported-vs-not) | capability round |
| A2-F14 | `solve(0 == 1, x)` is a DomainError instead of `NoSolutions` | solver/wire round |
| A2-F15 | `functions[]` omits `MinArity`/`Variadic` | wire round |
| A2-F16 / A4-P1-2 | empty `Text` where invariant 3 demands `Null`; stale doc examples | doc amendment + wire round |
| A2-F24 | `variables[]` display-only | wire round |
| A1-N-08 | `divrem` returns prose | wire round |
| A1-N-09 | `inspect(<Real>).exact` is `Null` while the wire's `exact` says true | Real exactness round |
| A1-N-10, A2-F25 / A4-P1-? | `evalf(f, digits)` ignores `digits` for an already-numeric argument | wire round |
| A3-F7, A3-F8, A3-F9 / A2-F19..F21 / A4-P1-1 | printer outputs that do not re-parse (complex unit, `x/y/x`, roots), exact algebraic printing as an inexact Real, `--print-budget` not bounding small values | printer round 2 in flight (i, `x/y/x`) + complex round + `--print-budget` in the wire round; the genuinely larger ones are stated as bounds in section O |
| A3-F12 | `solve_system_full([], [x])` leaks a CLR index exception | in flight (system-solve round) |
| A4-P1-6 | `assumptions()` is Text-only | wire round |
| A4-P1-5 | the doc's "how many solutions? (solutions.shape)" is wrong for family-valued sets | doc amendment |
| A2-F18, F20–F29, A3-F13–F21 | P2/P3 rows | recorded here; fixed only where a round touches them, otherwise listed as accepted rough edges in the final report |

## What this triage is for

D1 is not "the old findings are fixed" — it is "a *fresh* audit cannot falsify the claim". This table
exists so that the final report can say, for every one of the audit's P0/P1 findings, either the commit
that closed it (with my own re-probe) or the reason it is still open.
