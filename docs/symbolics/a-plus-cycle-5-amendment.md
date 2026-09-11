# Section O — amendment, Cycle 5

> Sections C–J of `a-plus-convergence-alignment-plan.md` are frozen law. Section N was Cycle 4's
> amendment; this is Cycle 5's. Like N it exists so that scope changes are written down rather than
> silently reduced — and, unlike N, it must also record what this cycle *closed* and what it did **not**.
>
> Two classes of entry below: **O.1 closed** (with the commit and the evidence), and **O.2 bounds**
> (stated, each with the honest behaviour today and a request for the maintainer's acceptance).
> Nothing here is implied away.

## O.1 What Cycle 5 closed

| # | Item | Was (measured at `9228305`) | Now | Commit |
|---|---|---|---|---|
| O-1 | The printer emitted text meaning a different value for a negative numeric base | `(-1)^x` → `-1^x`, which re-parses to `(rat -1 1)` | `(-1)^x`; the ONE power-base rule now asks the rendered text, in both arms | `34c970c`, `8502e5f` |
| O-1b | …and for **every other compound base or denominator the printer can produce** (`(1/2)^x`, `1 + i` as a base, `(x/y)/z`) | four more shapes re-parsed to a different canonical form | the round-trip property holds over a 30-shape corpus: `ok=30 bad=0` (pre-fix: `ok=15 bad=12`) | `8502e5f` |
| O-2 | The solver claimed completeness for an equation with no solution | `solve_full(sqrt(x)+2 == 0, x)` → `Solved/complete:true` with the extraneous root 4 | `Unevaluated/complete:false`; every inverse-branch candidate is verified, a set that lost one never claims completeness | `bb9746f` |
| O-2b | The **system** solver published `NoSolutions/complete:true` for solvable systems | `[x^2+y==1, x-y==0]` → "no solutions" | the two roots, or a proved-empty set, or `Unevaluated` — `NoSolutions` now requires a derived contradiction and no abandoned branch | `a03553a` |
| O-3a | An exact arithmetic identity failed | `(1/17)*17` ≠ 1; the audit's 62-probe cluster | exact: multiplication and addition go through exact fractions | `3ffc782`, `b908d9a` |
| O-3b | The limited-precision fast path certified a truncation exact | `setprecision(18); (1/17)*17` = `0.999999999999999985` `exact:true` | `1` exact; the fast path declines periodic operands | `b908d9a` |
| O-4 | A shape violation crossed as an internal invariant failure, and `solve_system` degraded to prose | `InternalError/InternalInvariantFailure`, `Text` result | `InvalidArgument/TypeMismatch` naming the builtin and position; a `SystemSolveResult` **record** including `completeness` | `2bd8785` |
| O-5 | Valid input killed the process | 3000-deep AST → exit `0xC00000FD`, **0 bytes on stdout** | `DepthExceeded/BudgetExceeded`, exit 1, well-formed envelope | `5382861` |

## O.2 Residual bounds that this amendment does NOT close

Each row is a bound the cycle **knows about** and states. The maintainer is asked to accept or reject each
in writing; none is silently reduced.

| # | Bound | Why it is not closed | Honest behaviour today | Disposition |
|---|---|---|---|---|
| O-B1 (=N.3.1) | **General rational exponents of a positive base** (`2^(1/2)`, `2^(1/3)`): a symbolic result the printer renders as `2^(1/3)` cannot be re-evaluated, so that shape does not round-trip | closing it means implementing rational powers as symbolic radicals across the whole numeric tower | `UnsupportedOperation` ("Non-integer exponents are not yet supported."), advertised as `pow.non-integer-exponent` | ❓ maintainer |
| O-B2 (=N.3.2) | **Exponents with denominator ≥ 3 over a negative base** | as N.3 | typed refusal, advertised | ❓ maintainer |
| O-B3 (=N.3.3) | **Complex algebraic roots of degree ≥ 4** — `RootOf` stays real-only per §D | as N.3 | `Partial` + `unrepresented_count` + diagnostic | ❓ maintainer |
| O-B4 (=N.3.4) | **Complex `log` beyond the principal branch** | as N.3 | principal value, which is what the oracle compares | ❓ maintainer |
| O-B5 (new) | `rootof(poly, k)` prints but does not re-parse | needs a `rootof` builtin (a feature) | display form only | ❓ maintainer |
| O-B6 (new) | An exact algebraic value computed numerically loses exactness (`1/2*sqrt(8)`) | the wire is *honest* (`exact:false` on a truncation); keeping radicals symbolic changes every numeric builtin | numeric truncation, `exact:false` | ❓ maintainer |
| O-B7 (new) | **`IsExact` is too narrow in the kernel**: `sin(x)`, `sqrt(x)`, `diff(sin(x),x)` report `exact:false` though they are exact expressions (audit A3-F6, A2-F4) | `Constructors.IsExactFunction` recognises only `abs/sign/floor/ceil/min/max`; widening it changes a flag read by the wire, the solver and the rewrite guards | `exact:false` on an exact closed form | ❓ maintainer |
| O-B8 (new) | **The wire-contract cluster** (audit A2, A4): error envelopes carry no `elapsedTime`/`timings`; 39 of 123 builtins answer a wrong-arity call with `InvalidOperation/DomainError` instead of the documented `InvalidArgument/TypeMismatch`; `print` output keeps a trailing CR; `variables[]` is display-only; `divrem` returns prose; `--print-budget` does not bound values already over budget; `evalf(f,digits)` ignores `digits` for an already-numeric argument; `assumptions()` is Text-only; `functions[]` omits `MinArity`/`Variadic` | each is small but they live in five files; the cycle ran out of round budget after closing the two P0-class items in the same area (`solve_system`, argument shape) | as listed, each reproduced twice by an independent auditor | ❓ maintainer |
| O-B9 (new) | **Complex results still do not round-trip end to end**: the printer emits `i` and the parser does not know the name (audit A3-F7) | needs a constant-vocabulary change, `abs`/`re`/`im`/`conj` on symbolic complex values, and the `re`/`im` error message that currently leaks an internal CLR type name | `sqrt(-1)` prints `i`; re-feeding it fails with `Undefined variable 'i'` | ❓ maintainer |
| O-B10 (new) | **`Real.Divide` still returns 0 for a nonzero quotient whose leading zeros exceed the digit budget** (`1/(3*10^1000)`), and the wire still labels some truncations exact (`setprecision(18); 1/1009`) | the exact-fraction paths (add/multiply) were fixed this cycle; `Divide` itself and the exactness *label* were the last round's objective, which had not returned when this cycle closed | a truncated value, printed as 0 for that one family | ❓ maintainer |
| O-B11 (new) | **Finite user-function recursion is not bounded**: `f(N)` is accepted at N=384 and dies with `0xC00000FD` at N=448 | it is program control-flow depth, not input nesting, and a cap actually safe on a 1 MB stack (≤128) would refuse recursion that works today | accepted recursion depth is stack-dependent and can still crash the process | ❓ maintainer |
| O-B12 (=N.3.5) | **Benchmark numbers come from a non-idle machine**; no across-run spread | needs an idle machine | stated in the benchmark document's own body | ❓ maintainer |
| O-B13 (=N.3.6) | ~~**The CI jobs have never executed on a GitHub runner**~~ **FALSIFIED this cycle — they have, and two of the three pass on a runner.** The maintainer asked for a push; `main` went to `origin` at `b920b8c` and CI run #14 executed all three jobs on `ubuntu-latest`: `sympy-oracle` **success**, `aot-smoke` **success**, `fast-tests` **failure** — in `Run Lovelace.Real.Tests (correctness subset)`. I reproduced the failure locally by adding the collector the CI uses (`--collect:"XPlat Code Coverage"`): `RealTrigFastPathTests.Cos_AtDefaultPrecision_StaysInteractive` holds a wall-clock budget (10 s) that coverage instrumentation inflated past the limit on a healthy 1 s cosine. Fixed by tagging it `Category=Timing` and giving it its own **uninstrumented** CI step (it still runs in CI; it is not skipped), which is what the next run exercises | Closed as a *bound*: what remains is a normal CI signal, not an unknown | CLOSED |
| O-B14 (=N.3.7) | `capabilities()`'s `exactness` is `BestEffort`; and the statement now **under-claims** in one place (`(-4)^(1/2)` works while non-integer exponents are advertised unsupported, audit A1-N-11) | proving exhaustiveness is not decidable from inside the product | 16 advertised entries, 16/16 matching the live call | ❓ maintainer |

## O.3 The one gate this cycle did not attempt

**D1 — "a fresh adversarial audit produces no P0/P1 finding" — is not met, and no fresh audit was run at
the end of the cycle.** Audit #1 (mid-cycle: 4 agents, 1188 probe inputs, new personas, no knowledge of
the work list) produced 24 P0/P1 rows; the triage table records each one's disposition. With O-B8, O-B9
and O-B10 open, a second audit would falsify the claim on its first ten probes. Running it would have
produced confirmation, not information. The cycle therefore reports **below A+ by its own gate**, exactly
as cycle 4 did, and `audit1-triage.md` is the work list a Cycle 6 starts from.
