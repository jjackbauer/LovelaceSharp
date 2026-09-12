# Section P — amendment, Cycle 6

> Sections C–J of `a-plus-convergence-alignment-plan.md` are frozen law. Section N was Cycle 4's
> amendment and section O Cycle 5's; this is Cycle 6's. It exists so that scope changes are written down
> rather than silently reduced, and — like O — it must record what the cycle **closed**, what it did
> **not**, and what it is **asking** the maintainer to accept.
>
> Three classes of entry: **P.1 CLOSED** (with the commit and the evidence), **P.2 OPEN** (stated, with
> the defect and where it lives), **P.3 ACCEPTANCE REQUESTED** (a bound the cycle believes is a scope
> decision rather than a defect, with the maintainer's answer quoted verbatim when it arrives).

## P.1 What Cycle 6 closed

| # | Item | Was (measured) | Now | Commit / evidence |
|---|---|---|---|---|
| P-1 | **The CI never-re-checked breakage** — the `fast-tests` job was red from run #18 to #26 on every push | `Lovelace.Console.Tests/ReplOutputTests.cs` still pinned the pre-r13 help text ("Solves an equation for x.", "Returns: Vector \| Text") after `86a5fc8` reworded the `solve` descriptor and changed its return kind to `SolveResult`; the job aborted in its fourth project after 32–37 s | the two pins assert what the product declares; run **#28** (`70241dc`) is green in all three jobs (`aot-smoke`, `sympy-oracle`, `fast-tests` step 5 = 435 s) | `98a9049`, `70241dc`; EVD-237…EVD-245, EVD-248 |
| P-2 | **Row 1, first route: `RealLiteral` dropped its provenance** | a truncated value that left the numeric tier through `Exprs.Real(RealLiteral.FromRealExact(v))` and came back re-entered as exact: `evalf(sin(pi(30)/6), 40)` published `0.499…999` `exact:true` with a 30-digit rational form | `RealLiteral` carries `IsExact`; `FromRealExact` copies it, `ToReal` restores it, `Real.AsInexact` exists, and the folds in `Constructors` keep the route; the wire answers `exact:false` with no numerator/denominator | `e8638c0`; EVD-244, EVD-251, EVD-253 |
| P-3 | **Row 2: `--cancel-after` was ignored inside the numeric kernels** | no `Cancellation` reference in `Lovelace.Array/Dsp/Real/Integer/Rational/Complex/Statistics`; `sum(1..10000000)` under a 1 ms budget ran 4.9–6.0 s and answered `ok:true`; `prod(1..200000)` under 100 ms ran 47–58 s; `matmul(eye(400))` under 100 ms returned the unbudgeted envelope | a strided poll inside the kernels; a cancelled statement raises `EvaluationCancelledException` and both envelopes carry a `cancellation` ledger. Measured: `sum` 284 ms, `prod` 362 ms, `matmul` 268 ms, all `Cancelled/BudgetExceeded`; the in-budget control answers `ok:true` with `"stopped":false` | `b009dfe`; EVD-258 |
| P-4 | **Row 3: `evalf(f, digits)` with a count it cannot honour** | `evalf(sin(1), 0)` answered `0.8`; `evalf(1/3, 0)` answered `0` **declared exact**; `evalf(sqrt(2), 0)` ignored the count (and at the audit base `9e761ba` it raised `InternalError/InternalInvariantFailure`) | a Natural or Integer count outside `[1, int.MaxValue]` is a recoverable `InvalidArgument/TypeMismatch` naming the argument, the accepted range and what arrived; `digits = 1` is the smallest accepted count and still answers | `d4d7ccf`; EVD-257 |
| P-5 | **Row 4: `capabilities()` under-claimed** | it advertised `pow.non-integer-exponent` while `(-4)^(1/2)` = `2*i` succeeds, and three reachable refusals were unlisted | the class is narrowed to the shape that is really refused (`pow.non-integer-exponent-of-a-positive-base`), each entry carries a `scope` field, and `solve.unevaluated`, `system-solve.unevaluated` and `integration.no-closed-form` are listed: **19 advertised entries, 19/19 reproduce their advertised code and category** | `d4d7ccf`; EVD-257, EVD-259 (capability honesty transcript, `MATCH=19 MISMATCH=0`) |
| P-6 | **O-B7** (`IsExact` too narrow) | `sin(x)`, `sqrt(x)`, `diff(sin(x),x)` reported `exact:false` | `inspect(sin(x)).exact` = `true`, `inspect(sqrt(x)).exact` = `true` | closed by Cycle 5's `49f5a4c`; re-measured in Cycle 6 (EVD-256) |
| P-7 | **O-B8a…i** (the wire-contract cluster) | error envelopes carried no `elapsedTime`/`timings`; 39 of 123 builtins answered a wrong-arity call with `InvalidOperation/DomainError`; `print` kept a trailing CR; `variables[]` was display-only; `divrem` was prose; `--print-budget` did not bound an over-budget value; `evalf` ignored `digits` for a numeric argument; `assumptions()` was Text-only; `functions[]` omitted arity | each re-probed and now correct (246-call arity sweep: **0** wrong-arity calls returned `InvalidOperation/DomainError`; error envelopes carry `elapsedTime` and `timings`; no stray CR; `variables[]` carries `structured`; `divrem` is a `DivRemResult` record; `x^2` at `--print-budget 1` truncates with `truncationReason:node-budget`; `assumptions()` is a record; `functions[]` publishes `minArity`/`variadic`) | closed by Cycle 5's wire rounds; re-measured in Cycle 6 (`docs/goal-cycle-6/round-5/bounds-reprobe.md`) |
| P-8 | **O-B10, second half** | `setprecision(18); 1/1009` was labelled exact for a truncation | `{"value":"0.000991080277502477","exact":false}` | closed by Cycle 5's `1aa7182`; re-measured in Cycle 6 (EVD-256) |
| P-9 | **O-B13** | the CI jobs had never executed on a GitHub runner | they execute, and Cycle 6 keeps them green | closed by Cycle 5 (run #15); re-confirmed in Cycle 6 (runs #28, #29) |
| P-10 | **O-B11 — user-function recursion was unbounded and killed the process** | `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` with `f(436)`/`f(448)` exited `0xC00000FD` with **0 bytes on stdout** and ~2 MB of `Stack overflow.` on stderr; `f(432)` answered | `InputDepth.MaxEvaluationDepth = 512` is one live counter over the interpreter's evaluation walk and exceeding it raises the cycle-5 `InputDepthExceededException`, which crosses the wire as `DepthExceeded/BudgetExceeded`: `f(85)` answers, `f(86)` is refused with a well-formed envelope, and `f(432)`/`f(448)` are refused instead of killing the process. The trade is explicit: recursion deeper than ~85 call levels is refused, where it used to work up to ~432 and then crash | `c5c1437`; EVD-254, EVD-262 |

## P.2 Bounds that Cycle 6 does NOT close — stated, with the defect named

| # | Bound / defect | Evidence today | Disposition |
|---|---|---|---|
| P-B1 | **The special-angle table hands an exact value to an inexact argument on the wire.** `evalf(cos(pi(30)), 100)` crosses as `-1` with `exact:true`, `numerator -1`, `denominator 1`, while the same expression's symbolic node is `exact:false` and mpmath at 110 dps gives −0.999…87355374… (a difference at the 61st decimal). The route is `ComplexMath.SinCosAtPrecision`, which reduces the argument against a π of the argument's own scale; two tests (`ComplexMathProvenanceTests.cs:83-84`) **pin** `Sin(Real.Pi) == 0` and `Cos(Real.Pi) == -1`, so closing it is a contract change, not a bug fix | EVD-251, EVD-259; found by two independent Falsifiers in round 3 | **OPEN — P1.** Needs one round that reduces against a π reaching below the argument's own scale, updates the two pinned tests against mpmath, and proves the change on a control tree |
| P-B3 | **O-B10, first half — a nonzero quotient is returned as `0`.** `1/(3*10^1000)` → `{"value":"0","exact":false}`, and `evalf(1/(3*10^1000), 30)` → `{"kind":"Integer","value":"0","exact":true}`; mpmath gives 3.333…e-1001. The sibling inside the digit budget (`1/(3*10^100)`) is correct | EVD-255 | **OPEN — P1.** `Real.Divide` loses the scale when the quotient's leading zeros exceed the digit budget |

## P.3 Acceptance requested (no answer recorded)

The maintainer was asked twice, in writing and with the measured behaviour quoted, to accept or reject
each of these; **no answer was recorded in this session**, so none is marked ACCEPTED. They are open
scope decisions, not silent reductions:

| # | Bound | Behaviour today (re-measured 2026-09-12) |
|---|---|---|
| P-A1 | O-B1 / O-B2 — general rational exponents of a positive base, and denominator ≥ 3 over a negative base | typed `UnsupportedOperation`; advertised as `pow.non-integer-exponent-of-a-positive-base` and `pow.negative-base-unrepresentable-exponent` |
| P-A2 | O-B3 — `RootOf` stays real-only, so degree ≥ 4 complex roots are `Partial` | `solve_full(x^4 - x^2 - 1 == 0, x)` → `Partial`, `unrepresented_count 2` |
| P-A3 | O-B4 — complex `log` is the principal branch only | matches mpmath to every printed digit on the principal branch |
| P-A4 | O-B5 — `rootof(...)` prints but does not re-parse | re-feeding the printed text fails with `Undefined variable 'x'.` |
| P-A5 | O-B6 — an exact algebraic value computed numerically loses exactness (`1/2*sqrt(8)`) | a numeric truncation labelled `exact:false`; the digits match mpmath |
| P-A6 | O-B9 — `sqrt(-1)` prints `i`, and `i` does not re-parse | `Undefined variable 'i'.` |
| P-A7 | O-B12 — benchmark numbers come from a non-idle machine; no across-run spread | stated in the benchmark document's own body; the harness cannot even generate its project in this workspace (53 `symbench.csproj` copies under `.worktrees`) |

## P.4 What Cycle 6 does not claim

Because P-B1 and P-B3 are open, **Cycle 6 does not claim A+**, exactly as Cycles 4 and 5 did not.
D-1 ("a fresh adversarial audit produces no P0/P1") is **not met**: the two independent Falsifiers of
round 3 produced P-B1, and the bound re-probe produced P-B3. They are named here rather than
graded around. The four rows the cycle was handed are closed — row 1's second route is not.
