# Harness State — cycle-5

- **Round**: 2 of the continued goal (cycle round 11) — five implementer rounds in flight, audit #2 dispatched
- **Goal**: close every Tier-0/Tier-1 defect with a pre-fix-failing test against independent ground
  truth, accept or close every residual bound in writing, and claim A+ only if a fresh adversarial audit
  cannot falsify it.

## Where the tree stands (commit `9e761ba`, pushed; CI green on #15)

| Tier-0 | State |
|---|---|
| T0-1 solver false completeness | CLOSED `bb9746f` |
| T0-2 printer spells a different value | CLOSED `34c970c` + `8502e5f` (round-trip property 30/30) |
| T0-3 wrong values marked exact | CLOSED `4ef7edf` (`2^-100000` exact; `0^(-1.0)` refuses; provenance flag) |
| T0-4 `(a/b)*b != a` | CLOSED `3ffc782` + `b908d9a` |
| T0-5 `solve_system` internal failure | CLOSED `2bd8785` (+ `a03553a` for the system solver's false NoSolutions) |
| T0-6 stack overflow on deep input | CLOSED `5382861` |

**Tier-1 closed**: `solve_system` returns the documented record with `completeness`; argument-shape
violations cross as `InvalidArgument/TypeMismatch`; error envelopes carry `elapsedTime`/`timings`;
one arity validator owns all 123 builtins; `print` keeps no CR; `solve(0==1,x)` answers with the
provably empty set.

**In flight this round**: the indeterminate-limit round; the kernel exact-flag round; two wire rounds
(`variables[]` structure, `inspect(Real).exact`, `functions[]` arity metadata, `--print-budget`,
`divrem` as a record; `evalf` digit count, structured `assumptions()`, the `solve` short form,
the `capabilities()` integer-domain claim, and two protocol-document examples).

**Audit #2 dispatched** (D1): four fresh personas against the binary published from `9e761ba` —
bulk differential testing, temporal/determinism, CLI surface + capability honesty, and rewrite/solve
falsification. Deliberately different attack strategies from audit #1 so the two overlap as little as
possible.

## Gate status

| Gate | State |
|---|---|
| G1 Evidence | PASS — EVD-201…EVD-228, each reproduced by me |
| G2 Falsification | open — audit #1's triage table still has open P1 rows; audit #2 will add its own |
| G3 Coverage | partial — every Tier-0 closed; Tier-1 mostly closed; the complex round-trip is unstarted |
| G4 Reproduction | PASS — every landed round re-run by me in the main tree; every pre-fix claim reproduced in a control worktree |
| G5 Honesty | PASS — the CI discovery, the golden that moved, the flaky tests and every open bound are recorded |

## Note for the next round

The `gh` CLI is not installed and the unauthenticated GitHub API is rate-limited, so CI status must be
read from the Actions web page or after the limit resets.
