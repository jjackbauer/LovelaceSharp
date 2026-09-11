# Round 03 — closed-form complex values, and an exhaustive capability statement

Goal cycle 4, round 03. Two coupled changes, one commit's worth:

* **(A)** bring closed-form complex values into the kernel's supported set, so values the kernel
  currently refuses are computed **exactly**;
* **(B)** make `capabilities()`'s `unsupported_operations` list say what the runtime actually
  does, and stay consistent with (A).

Scope was widened once during the round by the maintainer's ruling (quoted in
[Requirement changes](#requirement-changes-quoted-rulings)), which authorised minimal
`Lovelace.Suite` edits and bounded the target set to **the exponent with denominator exactly 2
over an exact negative rational base**. `Lovelace.Real/**` was not touched.

---

## 1. The bounded target set, and the SymPy ground truth for it

Every branch decision in this round was taken by running SymPy 1.14.0 rather than by assuming
which branch a negative base should take.

```
& "C:\Users\ricar\dev\.lovelace-tools\python\python3.exe" -c "import sympy; print(sympy.__version__)"
1.14.0
```

```
& "...python3.exe" -c @"
import sympy
cases = [(-1,sympy.Rational(1,2)),(-4,sympy.Rational(1,2)),(-8,sympy.Rational(1,3)),(-1,sympy.Rational(1,3)),(-1,sympy.Rational(2,3)),(-8,sympy.Rational(2,3)),(-8,sympy.Rational(-1,3)),(-16,sympy.Rational(1,4)),(-64,sympy.Rational(1,6)),(sympy.Rational(-1,4),sympy.Rational(1,2)),(-3,sympy.Rational(1,2)),(-27,sympy.Rational(1,3)),(-2,sympy.Rational(1,2)),(-4,sympy.Rational(3,2))]
for b,e in cases:
    print(f'({b})^{e} = {sympy.sstr(b**e)}   N30 = {sympy.N(b**e,30)}')
print('solve x^2+2x+5:', sympy.sstr(sympy.solve(sympy.symbols('x')**2+2*sympy.symbols('x')+5, sympy.symbols('x'))))
print('solve x^2+4:', sympy.sstr(sympy.solve(sympy.symbols('x')**2+4, sympy.symbols('x'))))
"@
```

Observed output (verbatim):

```
(-1)^1/2 = I   N30 = 1.0*I
(-4)^1/2 = 2*I   N30 = 2.0*I
(-8)^1/3 = 2*(-1)**(1/3)   N30 = 1.0 + 1.73205080756887729352744634151*I
(-1)^1/3 = (-1)**(1/3)   N30 = 0.5 + 0.866025403784438646763723170753*I
(-1)^2/3 = (-1)**(2/3)   N30 = -0.5 + 0.866025403784438646763723170753*I
(-8)^2/3 = 4*(-1)**(2/3)   N30 = -2.0 + 3.46410161513775458705489268301*I
(-8)^-1/3 = -(-1)**(2/3)/2   N30 = 0.25 - 0.433012701892219323381861585376*I
(-16)^1/4 = 2*(-1)**(1/4)   N30 = 1.41421356237309504880168872421 + 1.41421356237309504880168872421*I
(-64)^1/6 = 2*(-1)**(1/6)   N30 = 1.73205080756887729352744634151 + 1.0*I
(-1/4)^1/2 = I/2   N30 = 0.5*I
(-3)^1/2 = sqrt(3)*I   N30 = 1.73205080756887729352744634151*I
(-27)^1/3 = 3*(-1)**(1/3)   N30 = 1.5 + 2.59807621135331594029116951226*I
(-2)^1/2 = sqrt(2)*I   N30 = 1.41421356237309504880168872421*I
(-4)^3/2 = -8*I   N30 = -8.0*I
solve x^2+2x+5: [-1 - 2*I, -1 + 2*I]
solve x^2+4: [-2*I, 2*I]
```

Two further values the brief asked for:

```
& "...python3.exe" -c "import sympy; x=sympy.symbols('x'); print(sympy.sstr(sympy.solve(x**2+1, x)))"
[-I, I]

& "...python3.exe" -c "import sympy; print(sympy.sstr(sympy.log(sympy.Rational(-1,2)))); print(sympy.N(sympy.log(sympy.Rational(-1,2)),30))"
-log(2) + I*pi
-0.693147180559945309417232121458 + 3.14159265358979323846264338328*I
```

**Branch decision, stated explicitly.** For an exact `a > 0`, SymPy's `(-a)**e` is the
*principal* branch: `(-a)^(m/2) = a^(m/2)·i^m`. That is the rule implemented, and it is why
`(-4)^(3/2) = -8i` (not `+8i`) and `(-4)^(-1/2) = -i/2` (not `+i/2`) — both verified above, not
assumed.

---

## 2. Test-first: the failures observed before the implementation

New file `Lovelace.Symbolics.Tests/ComplexClosedFormTests.cs` was written first and run against
the unchanged kernel. Command and result:

```
dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~ComplexClosedFormTests"
Failed!  - Failed:     8, Passed:     3, Skipped:     0, Total:    11, Duration: 85 ms
```

The three that passed were the boundary pins (things this round must **not** change). The eight
failures, with the observed value in each:

| target | assertion | observed before |
|---|---|---|
| (A)1 `sqrt(-1)` | `NodeKind.NamedConstant` | `Actual: Power` |
| (A)1 `sqrt(-4)` | `value.IsExact` | `Actual: False` |
| (A)1 `sqrt(-9)`, `sqrt(-1/4)` | `nine.IsExact` | `Actual: False` |
| (A)1 `sqrt(-2)` | `Equals(Multiply(I, sqrt(2)))` | `sqrt(-2) built (pow (rat -2 1) (rat 1 2)), expected (mul (i) (pow (rat 2 1) (rat 1 2)))` |
| (A)2 `(-1)^(1/2)` | canonical `(i)` | `Actual: (pow (rat -1 1) (rat 1 2))` |
| (A)2 `i^2 = -1` | canonical `(rat -1 1)` | `Actual: (pow (i) (rat 2 1))` |
| (A)3 `solve(x^2+1==0,x)` | roots `["(i)", "(mul (rat -1 1) (i))"]` | `Actual: ["(mul (rat -1 2) (pow (rat -4 1) (rat 1 2)))", "(mul (rat 1 2) (pow (rat -4 1) (rat 1 2)))"]` |
| (A)3 `solve(x^2+4==0,x)` | roots `["(mul (rat -2 1) (i))", "(mul (rat 2 1) (i))"]` | `Actual: ["(mul (rat -1 2) (pow (rat -16 1) (rat 1 2)))", "(mul (rat 1 2) (pow (rat -16 1) (rat 1 2)))"]` |

Before the implementation, the **envelope** level agreed with that picture exactly
(`Lovelace.Run.exe --eval <script> --json --omit-functions`):

```
sqrt(-1)      -> {"ok":false,"code":"ArithmeticError","category":"DomainError","message":"Square root is not defined for negative numbers."}
sqrt(-4)      -> same
(-1)^(1/2)    -> {"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported."}
(-1)^(1/3)    -> same
log(-1/2)     -> {"ok":true, ... "canonical":"(fn log (rat -1 2))"}   (stayed symbolic)
```

And after the implementation, the same probes:

```
sqrt(-1)   -> {"ok":true,...,"display":"i","canonical":"(i)","domain":"complex","exact":true}
sqrt(-4)   -> {"ok":true,...,"display":"2*i","canonical":"(mul (rat 2 1) (i))","exact":true}
sqrt(-9)   -> {"ok":true,...,"display":"3*i","canonical":"(mul (rat 3 1) (i))","exact":true}
sqrt(-2)   -> {"ok":true,...,"display":"i*sqrt(2)","canonical":"(mul (i) (pow (rat 2 1) (rat 1 2)))"}
sqrt(-1/3) -> {"ok":true,...,"display":"i*sqrt(1/3)","canonical":"(mul (i) (pow (rat 1 3) (rat 1 2)))"}
(-1)^(1/2) -> {"ok":true,...,"display":"i","canonical":"(i)"}
(-4)^(1/2) -> {"ok":true,...,"display":"2*i","canonical":"(mul (rat 2 1) (i))"}
x = symbol("x"); solve(x^2 + 1 == 0, x)      -> {"ok":true,...,"display":"[i, -i]"}
x = symbol("x"); solve_full(x^2 + 1 == 0, x) -> {"ok":true,... status: Solved, complete: True,
                                                unrepresented_count: 0, solutions: [i, -i],
                                                completeness: Complete, diagnostics: []}
```

`(-8)^(1/3)` and `2^(1/2)` keep their typed error, unchanged:

```
(-8)^(1/3) -> {"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported."}
2^(1/2)    -> identical envelope
```

### Exactness, stated precisely rather than loosely

* `sqrt(-1) = i`, `sqrt(-4) = 2i`, `sqrt(-9) = 3i`, `sqrt(-1/4) = i/2`, `(-4)^(3/2) = -8i`,
  `(-4)^(-1/2) = -i/2` are **exact complex constants**: the envelope reports `"exact":true`
  because their `Expr` is an exact rational multiple of the named constant `i`.
* `sqrt(-2) = i·sqrt(2)`, `sqrt(-3) = i·sqrt(3)`, `sqrt(-1/3) = i·sqrt(1/3)` are **exact radical
  expressions**, not decimals, but they report `"exact":false` — the *pre-existing* convention
  that a radical over a non-perfect-square is not an exact *rational constant*
  (`Exprs.Sqrt(2)` behaves the same way today). Nothing about that flag was changed or widened
  for this round; it is recorded here so the boundary is not mistaken for an approximation. No
  decimal is produced anywhere in this round.

---

## 3. Production changes (every file:line)

### 3.1 `Lovelace.Symbolics/Constructors.cs` — the principal branch

* **`Constructors.cs:485-497`** — in `Exprs.Power`, a new block placed **before** the
  unit-fraction exact-root test: a negative `Integer`/`Rational` constant base with a
  non-integer rational exponent of **denominator exactly 2** is answered by
  `PrincipalSquareRootPower`. Placement is load-bearing: the odd-denominator real-root shortcut
  in the unit-fraction test must not be reachable from an even denominator, and the old
  "complex-valued: leave unevaluated" exit must no longer be reached for denominator 2.
* **`Constructors.cs:499-500`** — the previous line
  `if (!eu.IsInteger && bv.IsNegative) return MakePower(b, e);   // complex-valued: leave unevaluated`
  is now reachable only for exponents the new block declined, i.e. denominators **>= 3**.
* **`Constructors.cs:581-604`** — new `private static Expr? PrincipalSquareRootPower(Rat b, Rat e)`:
  returns `null` unless `e.Denominator == 2`; otherwise computes
  `a^(m/2) = a^((m-1)/2)·sqrt(a)` with `a = |b|`, `m` odd, and multiplies by `i` when
  `m ≡ 1 (mod 4)` and by `-i` when `m ≡ 3 (mod 4)`. `a^((m-1)/2)` is an exact rational power and
  `sqrt(a)` folds to an exact rational when `a` is a perfect square and stays an exact radical
  otherwise, so no approximation enters.
* **`Constructors.cs:517-536`** — in `Exprs.Power`, `i` raised to an integer now folds on the
  4-cycle `1, i, -1, -i`. This is not cosmetic: without it `i^2` stays an opaque power, a complex
  root can never substitute back to **exactly** zero, and the differential oracle's solve corpus
  (which requires exact-zero substitution) would reject the new complex cases.

### 3.2 `Lovelace.Suite` — the two authorised fallbacks

Both are **fallbacks at the point where the real-domain operation rejects the input**. The
`Lovelace.Real` contract is untouched: `Real.Sqrt(-1)` still throws, `Real.Pow` still refuses a
non-integer exponent.

* **`Lovelace.Suite/Interpreter.cs:1451-1470`** — the `sqrt` builtin. A non-negative argument and
  a symbolic argument take the original path unchanged (`Rl.SqrtAsync` /
  `Exprs.Power(symbolic, 1/2)`). A **negative** argument — exactly the input `Real.Sqrt` refuses —
  is routed to `Exprs.Power(Exprs.Rational(RationalReal.FromReal(real)), 1/2)`, i.e. to the new
  kernel branch. `RationalReal.FromReal` is used rather than `Real.ToString` so the conversion is
  exact for a periodic value too (`sqrt(-1/3)` answers instead of raising).
* **`Lovelace.Suite/NumericOps.cs:76` and `:78-95`** — `private static readonly Rl OneHalf` and
  `PowerReal`, wired into the `(BinaryOp.Power, ValueKind.Real)` arm at **`NumericOps.cs:64`**.
  Only `negative base ^ exactly 1/2` is diverted; every other case — including `2^(1/2)` — calls
  `baseValue.Pow(exponent)` and keeps the existing typed `NotImplementedException`.

### 3.3 `Lovelace.Symbolics/Evaluation.cs`

* **`Evaluation.cs:292-317`** — `NumOps.Ln` is back to its original shape (a negative real still
  raises `EvaluationException`), with an in-code note recording that the principal complex branch
  was implemented, measured green against the oracle, and removed by the round's scope ruling. See
  [What is NOT in this round](#5-what-is-not-in-this-round).

### 3.4 `Lovelace.Symbolics/SymbolicsPlugin.cs` — the capability statement

* **`SymbolicsPlugin.cs:211-214`** — the registration comment no longer claims `sqrt(-1)` is
  undiscoverable-but-unsupported.
* **`SymbolicsPlugin.cs:700-762`** — `CapabilitiesRecord` and its doc comment, rewritten (see
  [§4](#4-the-capability-statement)).
* **`SymbolicsPlugin.cs:214`** plus the record body: the `unsupported_operations` list.

### 3.5 Documentation and fixtures

* **`Lovelace.Symbolics/README.md:587-593`** — the usage-guide example
  `solve(x^3 - 1 == 0, x)` is executed verbatim by `UsageDocumentationTests`, and its pinned
  output changed because `sqrt(-3)` is now `i*sqrt(3)`:

  ```
  before: [1/2*(-sqrt(-3) - 1), 1/2*(sqrt(-3) - 1), 1] (Vector)
  after:  [1/2*(i*sqrt(3) - 1), 1/2*(-i*sqrt(3) - 1), 1] (Vector)
  ```

  The prose under the example was updated too — it previously *asserted* that `-3^(1/2)`
  "denotes the principal complex square root, i.e. `i*sqrt(3)`" while the pinned output still
  spelled the unevaluated radical. The round made the code match the prose.
* **`Lovelace.Run.Tests/fixtures/capabilities.json`** — the published golden envelope of
  `capabilities()`, regenerated mechanically from the live runner (volatile keys normalised
  exactly as `TestSupport.Normalise` does). The diff is 10 changed lines and no structural change:
  the two `display`/`typed` strings, the first entry's `operation_class`/`code`/`category`/
  `message`/`trigger`, and the second entry's `operation_class`/`trigger`. This file is outside the
  originally declared scope but is the published pin of the record (B) changes; leaving it stale
  would have published a false protocol. No other file under `Lovelace.Run.Tests` was touched.

---

## 4. The capability statement

`capabilities()` now returns exactly this (live, from `Lovelace.Run.exe --eval capabilities()`):

```
CapabilitiesResult(
  supported_domains:   [real, complex]
  unsupported_domains: [integer, rational]
  unsupported_operations: [
    UnsupportedCapability(operation_class: pow.non-integer-exponent,
        code: UnsupportedOperation, category: UnsupportedOperation,
        message: Non-integer exponents are not yet supported., trigger: 2^(1/2)),
    UnsupportedCapability(operation_class: pow.negative-base-unrepresentable-exponent,
        code: UnsupportedOperation, category: UnsupportedOperation,
        message: Non-integer exponents are not yet supported., trigger: (-8)^(1/3)),
    UnsupportedCapability(operation_class: solve.unsupported-domain,
        code: InvalidOperation, category: DomainError,
        message: solve(): currently supports domains real and complex; got integer.,
        trigger: x = symbol("x"); solve(x^2 - 2 == 0, x, integer)),
    UnsupportedCapability(operation_class: rootof.complex-algebraic,
        code: solve.unrepresented-roots, category: UnsupportedOperation,
        message: complex algebraic roots not supported (RootOf is real-only in v1).,
        trigger: x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x))
  ],
  exactness: BestEffort)
```

### 4.1 What changed and why

* **Removed: `complex.sqrt-negative`.** (A) made it answer. Keeping it would have advertised an
  error the live call no longer produces.
* **Re-triggered: `pow.non-integer-exponent`.** Its advertised trigger was `(-1)^(1/2)`, which
  now answers. The class itself is unchanged and very much alive — `(-1)^(1/2)` was never
  *specifically* about negative bases; `2^(1/2)` produces the identical envelope. The trigger is
  now `2^(1/2)`, which is both live and a better name for the class. `CapabilitiesBuiltinTests`
  is what caught the stale trigger.
* **Added: `pow.negative-base-unrepresentable-exponent`.** The maintainer's ruling requires the
  denominator-`>= 3` case to keep its typed error *and* an accurately-worded entry. Its live
  envelope is byte-identical to the previous entry's (same code, category and message), so it is
  enumerated as a separate `operation_class` with its own trigger rather than folded in: an agent
  matching on `code` sees one class, an agent matching on `operation_class` sees both, and neither
  is misled.

### 4.2 Exhaustiveness, argued rather than asserted — and why `exactness` stays `BestEffort`

`exactness` is **unchanged**. The honest claim is narrower than "exhaustive", so the member that
says "best effort" is the truthful one. What *is* now mechanical is the honesty of every entry,
and the argument for the rest is written into the record's doc comment
(`SymbolicsPlugin.cs:700-762`):

* Every entry is **live-verified**. `CapabilitiesBuiltinTests` no longer hard-codes four probes:
  it iterates *every* advertised entry, runs its `trigger` through the published `Lovelace.Run`
  envelope, and asserts the advertised `code`, `category` and `message` equal what the live call
  produced — reading the top-level error envelope when the call fails and the first structured
  `Diagnostic` when it succeeds. Adding an entry with an untruthful trigger, or leaving one for a
  class the runtime no longer rejects, now fails there.
* Enumerated: the plugin's **message-stable** unsupported-operation classes — i.e. the ones whose
  advertised `message` is a fixed function of the class rather than of the input.
* **Not enumerated, and why** (this is the precise statement the member `BestEffort` stands for):
  1. classes whose `message` is the kernel's **input-specific reason** —
     `limit.unevaluated`, `integration.unevaluated`, `solve.unevaluated`, `solve.partial`,
     `system-solve.unevaluated`. An `UnsupportedCapability` record advertises exactly *one*
     message; for these the message varies per expression (e.g. "no closed form for …"), so a
     single entry could not state it truthfully. Enumerating them would need either a schema
     change (out of scope — the record shape is frozen) or one entry per trigger, which is not a
     class list.
  2. `transform.budget-exceeded` and `transform.unsatisfiable-conditions`: stable messages, but
     the first embeds the exhausted budget kind, and neither is an unsupported *operation* — they
     are a retryable budget stop and a contradictory-branch domain error, carried as statuses.
     They belong to the transform contract, not to this list.
  3. the exception classes the **host** classifies but this plugin never raises
     (`FormatException`, `ArgumentOutOfRangeException` from `0^-1`, the matrix/array bridges):
     reachable from the engine, not from this plugin's operation surface, so listing them here
     would advertise a boundary that is not this plugin's.
* The two **domain** arrays remain exhaustive over the closed `MathDomain` enum, as before.

---

## 5. What is NOT in this round

Stated as residuals, not as omissions.

1. **Complex `log`.** `log` of a negative real still raises `EvaluationException` from the numeric
   evaluator; it is not given the principal complex branch `log|z| + i·pi`. The maintainer's scope
   ruling excluded complex log from the bounded target set. Measured fact, for the record: with
   the principal branch added (4 lines in `NumOps.Ln`), the differential oracle's `x*log(x)`
   derivative case **compares correctly at `x = -1/2`** — kernel `log(1/2) + i·pi + 1` against
   SymPy `0.306852819440054690582767878542 + 3.14159265358979323846264338328*I` — and the whole
   project went green (`Failed: 0, Passed: 819, Skipped: 0`). It was removed on instruction, not
   because it failed. `OracleCorpus.Derivatives` records next to the case why its sample points
   are strictly positive and exactly what would have to change for a negative point to become
   comparable.
2. **Exponents with denominator `>= 3` over a negative base.** `(-8)^(1/3)`, `(-1)^(1/3)`,
   `(-16)^(1/4)` keep their typed `UnsupportedOperation` at the host boundary, as ruled.
   *Kernel-level residual worth naming:* `Exprs.Power(Rational(-8), Rational(1,3))` returns the
   **real** root `-2` (the odd-denominator shortcut in the unit-fraction test), whereas SymPy's
   principal value is `1 + 1.73205080756887729352744634151*I`. That is a pre-existing
   real-vs-principal branch convention, unchanged by this round, unreachable from the published
   envelope (which throws first), and pinned deliberately by
   `ComplexClosedFormTests.NegativeBaseThirdRoot_IsOutsideTheBoundedSet` so a later round must
   change it consciously. It is NOT claimed as correct.
3. **General non-integer exponents of positive bases.** `2^(1/2)` is a real irrational the `Real`
   type refuses to approximate; unchanged, and still advertised.
4. **Complex algebraic roots of degree `>= 4`.** `RootOf` remains real-only; `solve_full` still
   reports `Partial` with `solve.unrepresented-roots`. Unchanged, still advertised.
5. **A residual outside this round, found while running the acceptance set and NOT caused by it.**
   `Lovelace.Run.Tests.GoldenEnvelopeTests` fails 1 of 39 — the `complex` fixture (`dft([1, 2, 3])`)
   whose imaginary part moved from `0.8660254037844386467637231707525` to
   `0.866025403784438646763723170752` — and `Lovelace.Dsp.Tests.TrigTests.Sin_GivenNegativeAngle_IsNegated`
   fails. `Lovelace.Dsp.csproj` references only `Abstractions`, `Complex`, `Integer`, `Natural`
   and `Real`; it does **not** reference `Symbolics` or `Suite`, so no change in this round can
   reach either failure. Both carry the signature of the in-flight `Lovelace.Real/Real.cs`
   rounding work owned by another agent. `capabilities.json` was regenerated; `complex.json` was
   deliberately left alone so that regression is not masked.

---

## 6. The paragraph for the frozen design document

> **Complex closed-form scope (round 03).** The symbolic kernel now treats the *principal branch*
> of a negative rational base raised to an exponent with denominator exactly 2 as a first-class
> supported value: `(-a)^(m/2) = a^(m/2)·i^m`, computed exactly as a rational multiple of the
> constant `i` when `a` is a perfect square (`sqrt(-1) = i`, `sqrt(-4) = 2i`, `(-4)^(3/2) = -8i`)
> and as an exact imaginary multiple of a radical otherwise (`sqrt(-2) = i·sqrt(2)`), never as a
> decimal. The branch is the one SymPy 1.14.0 returns and was verified per case rather than
> assumed. Two consequences follow, and both are contract, not implementation detail. First, the
> kernel now folds the imaginary unit's integer powers on the 4-cycle, because without it a
> complex root cannot substitute back to exactly zero and `solve` over the complex field would
> report roots it cannot certify. Second, the *host* numeric boundary delegates rather than
> approximates: `Lovelace.Real` keeps its real-domain contract in full — `Real.Sqrt(-1)` and
> `Real.Pow` with a non-integer exponent still throw — and the single input that contract rejects
> while an exact closed form exists (`sqrt(-a)` and `(-a)^(1/2)`) is routed to the symbolic
> complex path. Everything the kernel cannot represent exactly remains a typed
> `UnsupportedOperation` rather than an approximation: general non-integer exponents of positive
> bases, exponents with denominator `>= 3` over a negative base, and complex algebraic roots of
> degree `>= 4`, each carrying an `UnsupportedCapability` entry whose advertised code, category and
> message are asserted equal to what the live call produces. `capabilities()` is therefore
> exhaustive over the kernel's *message-stable* unsupported-operation classes and explicitly
> best-effort elsewhere: classes whose message is the kernel's input-specific reason
> (`limit.unevaluated`, `integration.unevaluated`, `solve.unevaluated`, `solve.partial`,
> `system-solve.unevaluated`) cannot be described by a single message field, and enumerating them
> would require a record-schema change that this cycle did not make. Complex `log` is the largest
> deliberate omission: `log` of a negative real is still a typed domain failure, and the
> `x*log(x)` oracle case documents its positive-domain restriction together with the one change
> that would lift it.

---

## 7. Requirement changes (quoted rulings)

Two existing tests pinned behaviour that (A) exists to change. Neither was deleted or weakened;
each was replaced by an assertion of the new exact value **plus a control** asserting that the
still-unsupported neighbours keep their existing typed error.

* `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:201-234` — the
  test `ComplexRootsAndFractionalPowersOfNegativeBases_KeepTheirErrors` asserted `sqrt(-1)` throws
  `ArithmeticException` and `(-1)^(1/2)` throws `NotImplementedException`, with the doc comment
  "this round does NOT add complex square roots or rational powers of negative bases". It is now
  `NegativeSquareRootsAndHalfPowers_NowAnswerExactly_WhileOtherNonIntegerPowersStillThrow`,
  renamed and rewritten to assert `sqrt(-1) = i`, `(-1)^(1/2) = i`, `sqrt(-4) = 2*i`, and to
  control-assert that `2^(1/2)`, `(-8)^(1/3)` and `(-1)^(1/3)` still throw their exact message.

  > **Maintainer ruling, §4.3:** "`HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:202-208`
  > asserts the OLD behaviour … Replacing it is sanctioned by the §4.3 decision — but you must
  > REPLACE it with an assertion of the new exact value PLUS a control asserting a still-unsupported
  > case (`2^(1/2)`) still throws with its existing message. Never delete the test, never weaken a
  > neighbouring one, and record the change in your implementation.md as a requirement change
  > quoting this ruling."

* `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs` — strengthened, not weakened.
  `UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage` now asserts the four new
  classes **and** `DoesNotContain("complex.sqrt-negative")`; `AdvertisedCodesAndCategories_MatchTheLiveEnvelope`
  became the data-driven loop over every advertised entry described in §4.2. The one assertion
  that had to change is the four literal envelope probes, three of which would now be probing
  supported operations.

* `Lovelace.Symbolics/README.md` — a documentation example's pinned output, invalidated by (A);
  updated together with the prose that already claimed the new value.

* The maintainer also ruled, after this round's first implementation landed, to keep complex
  `log` out of scope. That reversed the brief's instruction to reinstate the excluded
  `x*log(x)` sample point; the corpus entry now documents the restriction and its lifting
  condition instead (§5.1).

---

## 8. Acceptance evidence

All commands run from `C:\Users\ricar\dev\LovelaceSharp`. The oracle runs used:

```powershell
$env:PATH = "C:\Users\ricar\dev\.lovelace-tools\python;" + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY="1"
```

```
dotnet test Lovelace.Symbolics.Tests -c Release --nologo
```

```
Passed!  - Failed:     0, Passed:   819, Skipped:     0, Total:   819 - Lovelace.Symbolics.Tests.dll (net10.0)
```

```
dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
```

```
Passed!  - Failed:     0, Passed:   819, Skipped:     0, Total:   819 - Lovelace.Symbolics.Tests.dll (net10.0)
```

The filter run reports the project total because `dotnet test --filter` still builds and reports
the assembly's summary; what matters here is `Failed 0` and `Skipped 0` **with
`LOVELACE_REQUIRE_SYMPY=1` set**, i.e. every oracle comparison actually executed — including the
new `x**2 + 1` and `x**2 + 4` solve cases.

```
dotnet test Lovelace.Suite.Tests -c Release --nologo
```

```
Passed!  - Failed:     0, Passed:   642, Skipped:     0, Total:   642 - Lovelace.Suite.Tests.dll (net10.0)
```

```
dotnet test Lovelace.Run.Tests -c Release --nologo
```

```
Failed!  - Failed:     1, Passed:    38, Skipped:     0, Total:    39 - Lovelace.Run.Tests.dll (net10.0)
```

The single remaining failure is the `complex` (`dft([1, 2, 3])`) golden described in §5.5: a
`Real`-layer rounding change owned by another agent, in a project that does not reference anything
this round touched. The `capabilities` golden — the one this round invalidated — passes.

The verification value of the oracle change is worth stating plainly: the `NumberListsAgree`
comparison now prints each SymPy value's **real and imaginary part** and compares both against the
kernel's own `Re`/`Im`, at the same tolerance as before. That is strictly stronger than the old
single-real-number comparison — a case where SymPy answers complex and the kernel real (or with a
different imaginary part) is now a reported MISMATCH rather than "sympy returned something that is
not a finite real number".
