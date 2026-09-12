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
| P-9b | **The CI budget itself** | the `fast-tests` job exceeded its 30-minute budget (runs #30, #33 cancelled; #32 and #37 failed early) because wall-clock verdicts and a collector-multiplied corpus ran inside the instrumented loop | the loop excludes `Category=Timing` and `Category=Costly`; both run uninstrumented in their own step, so every test still gates every push. Totals preserved: Suite 817+8, Run 201+5, Real 2476+13, Symbolics 1064+8 skips+32 | `aa27753`, `55c8cab`, `be89e55`, `1ba8e16`; EVD-263, EVD-264 |
| P-10 | **O-B11 — user-function recursion was unbounded and killed the process** | `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` with `f(436)`/`f(448)` exited `0xC00000FD` with **0 bytes on stdout** and ~2 MB of `Stack overflow.` on stderr; `f(432)` answered | `InputDepth.MaxEvaluationDepth = 512` is one live counter over the interpreter's evaluation walk and exceeding it raises the cycle-5 `InputDepthExceededException`, which crosses the wire as `DepthExceeded/BudgetExceeded`: `f(85)` answers, `f(86)` is refused with a well-formed envelope, and `f(432)`/`f(448)` are refused instead of killing the process. The trade is explicit: recursion deeper than ~85 call levels is refused, where it used to work up to ~432 and then crash | `c5c1437`; EVD-254, EVD-262 |

| P-11 | **P-B1 — the special-angle path handed an EXACT value to an INEXACT argument** | `evalf(cos(pi(30)), 40)` and `(…, 100)` crossed as `{"value":"-1","exact":true,"numerator":"-1","denominator":"1"}` while the same expression's symbolic node was `exact:false` and mpmath put cos(pi(30)) at −0.999…87355374… | the argument's flag now follows through `ComplexMath.SinCosAtPrecision`'s exactly-zero shortcut and its exit, and through `Tan`, `Atan`, `Atan2`, `AsinReal` and the complex `Sin/Cos/Tan/Sinh/Cosh`; the wire answers `{"value":"-1","exact":false}` with no numerator or denominator. The pinned VALUES (`Sin(Rl.Pi) == 0`, `Cos(Rl.Pi) == -1`) are deliberately untouched | `78c19d8`; EVD-269 (12/0 fixed vs 8 failed / 4 passed on a pristine control) |
| P-12 | **P-B3 — a nonzero quotient was returned as `0`** | `1/(3*10^1000)` → `{"value":"0"}` and `evalf(1/(3*10^1000), 30)` → an Integer `0` **declared exact**, where mpmath gives 3.33e-1001 | `Real.Divide` walks a leading-zero run that alone fills the digit budget and stores only significant digits with the exponent placed accordingly: the quotient is the exact periodic `0.000…0(3)` with numerator 1 and denominator 3·10^1000, and the in-budget neighbours are unchanged | `78c19d8`; EVD-269 (6/0 fixed vs 3 failed / 3 passed) |
| P-13 | **The evalf and complex flag leaks** | `evalf(pi(30), 40)`, `evalf(pi(1), 40)`, `evalf(e(30), 40)` and `evalf(pi(30), 100)` published a truncated constant as `exact:true` **with a rational form**; `ComplexMath.Exp` of an inexact zero and `Exp/Sqrt(Complex)` for inexact arguments did the same | `NumericAtRequestedPrecision` now splits the cases — an exact Real keeps the rational projection, an inexact one is carried as an inexact `RealLiteral` at the same digit count — and the complex entry points follow the argument's provenance. The digits are unchanged; only the claim is | `762aa8c`; EVD-272 (22/0 vs 15 failed / 7 passed; 7/0 vs 4 failed / 3 passed) |

## P.2 Bounds that Cycle 6 does NOT close — stated, with the defect named

| # | Bound / defect | Evidence today | Disposition |
|---|---|---|---|
| P-B4 | **A magnitude whose leading zeros exceed the requested digit count is still published as `0`.** `evalf(sin(pi(30)), 40)` → `{"value":"0"}` where mpmath gives 5.03…e-31, and `evalf(1/(3*10^1000), 30)` → `{"value":"0"}` although the Real itself is the exact periodic 3.33e-1001. The FLAG is honest in both (`exact:false`) — what is wrong is the digits, because `evalf`'s digit count is a count of DECIMAL PLACES, not of significant digits. Closing it means either significant-digit semantics for `evalf` (a contract change with a wide blast radius) or carrying the exponent through the decimal-string conversion | EVD-270, EVD-272 | **OPEN — P1 (value precision, flag honest).** Named for the audit and for the maintainer's decision on `evalf`'s digit semantics |

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

Because P-B4 is open - and because the four-persona fresh audit was never run - **Cycle 6 does not claim A+**, exactly as Cycles 4 and 5 did not.
D-1 ("a fresh adversarial audit produces no P0/P1") is **not met**: the two independent Falsifiers of
round 3 produced the flag leak that is now closed (P-11), and rounds 10 and 11 closed it and P-B3; what
graded around. The four rows the cycle was handed are closed — row 1's second route is not.
