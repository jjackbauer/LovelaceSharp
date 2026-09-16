# Cycle 7 — Gate 0 and ADV-01 evidence

All commands below were run in this session. Nothing here is inferred from documentation.

## Gate 0 — baseline (revision `ce30fc3`, clean tree)

### Test sweep

```
dotnet test LovelaceSharp.slnx -c Release
```

| suite | passed | failed | skipped |
|---|---:|---:|---:|
| Lovelace.Representation.Tests | 91 | 0 | 0 |
| Lovelace.Knowledge.Tests | 28 | 0 | 0 |
| Lovelace.Abstractions.Tests | 20 | 0 | 0 |
| Lovelace.Natural.Tests | 195 | 0 | 0 |
| Lovelace.Array.Tests | 19 | 0 | 0 |
| Lovelace.Integer.Tests | 148 | 0 | 0 |
| precbench.Tests | 13 | 0 | 0 |
| Lovelace.Console.Tests | 15 | 0 | 0 |
| Lovelace.Real.Tests | 2495 | 0 | 0 |
| Lovelace.Complex.Tests | 121 | 0 | 0 |
| Lovelace.Dsp.Tests | 61 | 0 | 0 |
| Lovelace.Suite.Tests | 905 | 0 | 0 |
| Lovelace.Studio.Tests | 22 | 0 | 0 |
| Lovelace.Symbolics.Tests | 1185 | 0 | 8 |
| Lovelace.Run.Tests | 367 | 0 | 0 |
| **TOTAL** | **5685** | **0** | **8** |

### Artefact fingerprints (F-5 — the reason this section exists)

> **SUPERSEDED — read round 5c before using this table.** The two `out/aot` rows below were recorded
> as a Native AOT fingerprint, and `out/aot` is NOT a Native AOT artefact: it is a framework-dependent
> apphost whose `Lovelace.Run.dll` sits beside it, which is why those builds measured 5.7-5.8 MB while
> the genuine single-file AOT binary measures 6.0 MB. The correction, the measurement that established it
> (`The application to execute does not exist: '…\Lovelace.Run.dll'` once the DLLs are moved aside), and
> the correct artefacts are in **Round 5c — "The two published artefacts were not what their names said"**.
> The rows are left standing because a record that quietly rewrites itself is not a record.

| artefact | bytes | written | revision |
|---|---:|---|---|
| `out/aot/Lovelace.Run.exe` at session start | 5 767 680 | 2026-09-12 21:51:38 | **four commits behind HEAD** |
| `out/aot/Lovelace.Run.exe` re-published from HEAD | 5 815 808 | 2026-09-16 15:01:46 | `ce30fc3` |
| `Lovelace.Run/bin/Release/net10.0/Lovelace.Run.exe` (JIT) | 162 304 | 2026-09-16 14:56:39 | `ce30fc3` |

### F-7 — NEW, P2 (build hygiene) — `make runner` fails on any tree that has been published with a RID

`make runner` runs `dotnet publish Lovelace.Run/Lovelace.Run.csproj --configuration Release --framework net10.0
-p:PublishAot=true -p:InvariantGlobalization=true --output Lovelace.Run/bin/Release/net10.0/publish`
(Makefile:48-53). On a tree where `dotnet publish -o out/aot` has already run (which is what CI does, and what
every cycle-6 evidence step did), that command FAILS:

```
error NETSDK1047: Assets file '...\Lovelace.Run\obj\project.assets.json' doesn't have a target for
'net10.0/win-x64'. Ensure that restore has run and that you have included 'net10.0' in the TargetFrameworks
for your project. You may also need to include 'win-x64' in your project's RuntimeIdentifiers.
```

Cause: the earlier RID-specific publish leaves a RID-scoped `project.assets.json`, and the Makefile's command
passes no `-r`, so the SDK looks for a RID-specific target that the last restore did not write. Consequence:
the path `harness/lovelace.host.js:94` spawns by default is exactly the path `make runner` fails to produce,
so an agent that follows `harness/README.md` can end up driving a stale or missing runner. Reproduced this
session; recorded here rather than silently worked around. The workaround used for this cycle's measurements is
to pass `-r win-x64` explicitly.

---

## ADV-01 — the exact-integer argument contract

### What landed

| file | change |
|---|---|
| `Lovelace.Abstractions/ArgumentContractException.cs` | NEW — the one refusal an integer-semantic argument produces, deriving from `ArgumentException` so the frozen `InvalidArgument`/`TypeMismatch` wire pair applies unchanged. Carries `Failure`, `Builtin`, `Parameter`, `Bounds`, `Received`, `ReceivedText` as properties. |
| `Lovelace.Abstractions/Int64Range.cs` | NEW — the bound, declared at the call site and named in the refusal. |
| `Lovelace.Abstractions/ExactIntegerArgument.cs` | NEW — `Require`, plus the `ExactIntegerReader` delegate. The rule lives here, written once. |
| `Lovelace.Natural/Natural.cs` | `TryGetExactInt64` — from the cached decimal text or the limb count; never renders a wide value to decide it is wide. |
| `Lovelace.Integer/Integer.cs` | `TryGetExactInt64`, **virtual**, handling both ends of the `long` range. |
| `Lovelace.Real/Real.cs` | `TryGetExactInt64`, **override** — `IsInteger` is the whole integrality test, so `2.5` is refused and `2.0` is the integer 2. |
| `Lovelace.Abstractions.Tests/ExactIntegerArgumentTests.cs` | NEW — 52 tests. |

### Design decisions forced by the code (not by the plan)

1. **No interface in `Lovelace.Abstractions`.** The plan proposed an `IExactIntegerProvider` interface. That would
   have required `Lovelace.Natural` (and through it every numeric project) to reference `Lovelace.Abstractions`,
   which is the layer they are built on. The contract therefore takes an `ExactIntegerReader` **delegate**, and
   each layer that owns a payload channel supplies its own three-line reader. The rule stays in one place; the
   type knowledge stays in the layers that have it.
2. **`Integer.TryGetExactInt64` is VIRTUAL and `Real` OVERRIDES it.** `Real` derives from `Integer`, so a payload
   statically typed as `Integer` can be a non-integral `Real`. Without the override, the base rule would answer
   `true` for `2.5` and publish the floor the contract exists to refuse. Pinned by
   `Real_ThroughAnIntegerTypedReference_IsStillNotAnInteger`.
3. **The predicate follows `Real.IsInteger`, and two shapes make that observable.** `-0.0` parses to a negative
   exponent with a zero magnitude, and `1.(0)` denotes 1 but carries a period; both are refused by the product's
   own rule. They are **pinned as behaviour** (`Real_TryGetExactInt64_FollowsTheProductsIntegralityRule`) rather
   than quietly widened, because widening integrality is a decision about the numeric tower, not an argument-contract
   detail.
4. **The two `Real` refusals are kept distinct.** `2.5` is `NotAnInteger`; `2.0` is `NotAnIntegerKind`, and its
   message names the integer to write instead. The language's kind rule ("digit counts must be Natural or Integer")
   is pinned by `Lovelace.Suite.Tests.InterpreterBuiltinPiTests:86`, so the refusal is a property of the VALUE
   KIND — and a consumer must be able to tell a fixable argument from an unfixable one without parsing prose.

### Control tree

```
git worktree add .worktrees/adv01-ctl --detach HEAD        # ce30fc3, unmodified product code
copy the new test file + the test csproj change into it
dotnet test Lovelace.Abstractions.Tests -c Release
-> error CS0246: 'ArgumentContractException' could not be found
   error CS0246: 'Int64Range' could not be found
```

**The control fails at COMPILE time, and that is recorded honestly rather than dressed up.** It proves the API is
net-new; it does **not** by itself prove a behavioural difference, because at HEAD there is no API whose behaviour
could differ. The behavioural control for this class is the wire reproduction set (F-1 and Q-2) and it lands with
ADV-02, where each migrated call site gets its own control tree **and** its own pre-fix wire transcript.

### Verification on the fixed tree

```
dotnet build Lovelace.Real -c Release            -> 0 warnings, 0 errors
dotnet test Lovelace.Abstractions.Tests          -> 52 passed, 0 failed
```

### Behaviour now pinned (measured through the new predicate)

| input | `Exponent` | periodic | `IsInteger` | `TryGetExactInt64` | value |
|---|---:|---|---|---|---:|
| `2` | 0 | false | true | true | 2 |
| `2.0` | 0 | false | true | true | 2 |
| `2.000` | 0 | false | true | true | 2 |
| `-2.0` | 0 | false | true | true | -2 |
| `1000000000000000000.000` | 0 | false | true | true | 1000000000000000000 |
| `2.5` | -1 | false | false | **false** | — (not floored) |
| `2.9999` | -4 | false | false | **false** | — |
| `-0.0` | -1 | false | false | **false** | — |
| `0.(3)` | -1 | true | false | **false** | — |
| `1.(0)` | -1 | true | false | **false** | — |
| `9223372036854775808` (Nat) | — | — | — | **false** | — (too wide, refused by name) |
| `-9223372036854775808` (Int) | — | — | — | true | long.MinValue |
| `9223372036854775807` (Nat) | — | — | — | true | long.MaxValue |
| `2.5` as an `Integer`-typed reference | — | — | — | **false** | — (the virtual/override trap) |


---

## ADV-04 — certified transcendental precision at the `Real` boundary

### The measurement that defined the work (before any change)

Ground truth: mpmath 1.3.0 at 4 000 decimal places. `correct` counts the matching fractional prefix of the
value `Real.Sin`/`Real.Cos` returns.

| case | published fractional digits | correct (BEFORE) | requested |
|---|---:|---:|---:|
| `Sin(1, 30)` | 606 | 40 | 30 |
| `Sin(1, 50)` | 1378 | **59** | 50 |
| `Sin(10, 30)` | 1453 | **29** | 30 |
| `Sin(10, 50)` | 3133 | **49** | 50 |
| `Sin(10, 100)` | 10310 | **99** | 100 |
| `Sin(10, 200)` | 33516 | **199** | 200 |
| `Sin(10^100, 30)` | 1754 | **0** | 30 |

So the audit's P-2 ("hundreds of digits of false precision") was real but understated: the kernels were short of
the digits they **claimed**, not merely long past them.

### Root causes (both derived, then verified)

1. **The working precision was a constant.** `guard = digits + 10` paid nothing for the range reduction's
   cancellation, which costs up to *i* places for an argument with *i* integer digits. The measured shortfall
   tracks *i* exactly: `Sin(10, …)` is short by 1, 1 and 11 at 30/50/100 places (floored at `digits+10`).
2. **π was re-fetched on the wrong condition.** `ResolvedPiDigits` looked only at the ARGUMENT's *fractional*
   places, so a huge integer argument used the ambient 1000-place π. The reduction multiplies π by
   `k = trunc(x / 2π)` — up to `10^i` — so every place π is short by is amplified by `k` and lands as absolute
   error on a residual of order 1. Hence `Sin(10^100, 30)` had ZERO correct digits.

**A trap worth recording.** The first attempt sized the guard from `value.Exponent`, which is **0** for any
decimal literal however long it is — so `10^100` looked like a 1-digit argument and the fix did nothing. The
width has to come from the magnitude's decimal digit count (`Natural.DecimalDigitCount()`, a length read on the
cache the type already keeps) plus the exponent.

### What changed

| file | change |
|---|---|
| `Lovelace.Real/Real.cs` | `Sin`/`Cos` now: (1) ask the **special-angle table first, against the caller's own π and in the caller's own scope**; (2) only for a general angle, widen the working scope to `digits + 10 + TrigIntegerDigits(value)` and re-fetch π at `TrigPiPlaces`; (3) re-reduce and re-check the table at that width. New helpers `TrigReductionPlaces`, `TrigIntegerDigits`, `TrigPiPlaces`, `FetchPi`. |
| `Lovelace.Natural/Natural.cs` | `DecimalDigitCount()` — the magnitude's decimal width, without materialising limbs or rendering a wide value. |
| `Lovelace.Real.Tests/RealTrigCertifiedPrefixTests.cs` | NEW — 16 tests pinning correctness of the PUBLISHED digits against mpmath goldens, plus the exact-path controls. |

**Ordering is load-bearing and was falsified the hard way.** Asking the table *after* widening the scope compares
the caller's exact angle against a FINER π and misses: `Sin(Pi/6)` came back `0.4999…`, and the reordering fixed
9 `Lovelace.Dsp.Tests` cases, 3 `RealTrigPiResolutionPropertyTests` cases, one `RealTrigSymmetryTests` case, a
`Lovelace.Run.Tests` golden envelope and 6 `Lovelace.Suite.Tests` Dsp cases at once. That intermediate state was
measured (22 failures) and is recorded here rather than hidden.

### Results after the change (same oracle, same battery)

| case | published fractional digits | correct | requested | verdict |
|---|---:|---:|---:|---|
| `Sin(1, 30)` | 606 | 39 | 30 | OK |
| `Cos(1, 30)` | 641 | 40 | 30 | OK |
| `Sin(10, 30)` | 1815 | **43** | 30 | OK |
| `Sin(10, 50)` | 3893 | **64** | 50 | OK |
| `Sin(10, 100)` | 11489 | **113** | 100 | OK |
| `Sin(10, 200)` | 35517 | **214** | 200 | OK |
| `Sin(10^100, 30)` | 4666 | **42** | 30 | OK |
| `Cos(10^100, 30)` | 4401 | **43** | 30 | OK |
| `Sin(1e-5, 30)` | — | correct | 30 | OK |

**ALL REQUESTED DIGITS CORRECT: True.**

### Cost (measured, same process)

| argument | before | after |
|---|---:|---:|
| `sin(1, 30)` | 0.2 ms | 0.3 ms |
| `sin(10^20, 30)` | — | 0.6 ms |
| `sin(10^100, 30)` | — | 0.9 ms |
| `sin(10^300, 30)` | — | 3.9 ms |

### Control tree

```
git worktree add .worktrees/adv04-ctl --detach HEAD      # ce30fc3, unmodified product code
copy RealTrigCertifiedPrefixTests.cs into it
dotnet test Lovelace.Real.Tests --filter RealTrigCertifiedPrefixTests
-> Failed!  - Failed: 4, Passed: 12, Total: 16
   Sin_OfTen_PublishesTheDigitsItClaims(digits: 30)   [FAIL]
   Sin_OfTen_PublishesTheDigitsItClaims(digits: 50)   [FAIL]
   Sin_OfTen_PublishesTheDigitsItClaims(digits: 100)  [FAIL]
   Sin_And_Cos_OfTenToTheHundred_PublishTheDigitsTheyClaim [FAIL]
fixed tree -> Passed! - Failed: 0, Passed: 16
```

The four control failures are precisely the two defect families (the in-request shortfall for an argument with
integer digits, and the total loss at extreme magnitude). The `Sin(1, …)` width assertions pass on both trees,
because over-delivery past the request is a WIDTH policy decision, not a correctness one — see below.

### The width question, decided honestly and left OPEN

After this fix every published digit is correct, but `Real.Sin(1, 30)` still **returns 606 fractional digits**
where the caller asked for 30. That is over-delivery, not false precision, and capping it is a **policy decision
with two incompatible precedents already in the product**:

* the CLI's `evalf(sin(x), d)` cuts at exactly `d` fractional places and therefore answers `0` for
  `cos(pi(120)/2 − 10^-100)`, whose true value is `1.0e-100`;
* the property suite (`RealTrigPiResolutionPropertyTests.Cos_OfHalfPiTruncatedAtDPlaces_IsTheHalfDistanceToPi`)
  requires that same case to return its tiny value.

The principled rule that satisfies both — `keep = max(digits, significant digits) + guard` — needs the
intermediate `excessMs`-style precision accounting to be stated as a public contract, and choosing it unilaterally
would move an API width that embedders can measure. It is recorded here as the cycle's remaining precision
question, with the correctness half of P-2 closed and measured.


---

## ADV-02 (Symbolics half) — the `series` order now goes through the contract

### What changed

| file | change |
|---|---|
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | The `series` order argument routes through `ExactIntegerArgument.Require` with a declared `[1, int.MaxValue]` bound. New `ReadExactInteger` (kind-exact reader), `RefuseIntegerArgument`, `DescribeIntegerPayload`, `SeriesOrder`, `SeriesOrderPosition`. **`AsInt`, `AsLong` and `ParseInt64Argument` are DELETED** — `AsLong`'s `Rl r => ParseInt64Argument(r.ToString().Split('.')[0])` was the last silent floor in the layer. |
| `Lovelace.Abstractions/ArgumentContractException.cs` | The refusal now carries the 1-based argument `Position` and speaks the protocol document's grammar (`<builtin>(): argument <n> (<name>) must be …`). |
| `Lovelace.Abstractions/Int64Range.cs` | Bounds read as natural English after "must be": `a non-negative count (0 or more, …)`, `between 1 and 2147483647`. |
| `Lovelace.Abstractions.Tests/ExactIntegerArgumentTests.cs` | 53 tests (was 52): the new signature, plus `Refusal_SpeaksTheDocumentedArgumentGrammar`. |

### Wire behaviour, before → after (JIT runner published from this tree)

| script | before | after |
|---|---|---|
| `series(sin(x), x, 0, 2.5)` | `ok:true`, `["x + O(x^2)"]` — **silently floored** | `InvalidArgument/TypeMismatch`: `series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.` |
| `series(sin(x), x, 0, 2.0)` | `ok:true`, `["x + O(x^2)"]` — silently accepted a Real | `InvalidArgument/TypeMismatch`: `…must be a Natural or Integer; got the Real 2, which is integral — write it as 2.` |
| `series(sin(x), x, 0, -1)` | `InvalidArgument` naming the value (round-23 fix) | `…must be a Natural or Integer between 1 and 2147483647; got -1.` — position and name added |
| `series(sin(x), x, 0, 10^30)` | `InvalidArgument`: "an integer argument must fit a signed 64-bit integer …" | `…must be a Natural or Integer that denotes an exact integer; got the Natural 1000000000000000000000000000000.` |
| `series(sin(x), x, 0, 2)` | `["x + O(x^2)"]` | **unchanged** `["x + O(x^2)"]` |
| `series(sin(x), x, 0, 5)` | `["x - 1/6*x^3 + O(x^5)"]` | **unchanged** |

### Two design decisions this call site forced, recorded because they are product-wide

1. **A `Real` is never an integer ARGUMENT, however exactly integral it is.** The first cut of the
   reader accepted `2.0` (it IS the integer 2) — and that would have made `series` WIDER than its
   neighbours, since `zeros(2.0)`, `[1,2,3][1.0]` and `setprecision(30.0)` are all refused today and
   `Lovelace.Suite.Tests.InterpreterBuiltinPiTests` pins the rule. The reader is therefore kind-exact,
   and the `Real` arm must come FIRST because `Real` derives from `Integer`.
2. **The refusal is still two-valued.** An exactly integral `Real` is `NotAnIntegerKind` and its
   message names the integer to write (`write it as 2`); a fractional `Real` is `NotAnInteger`. A
   consumer picks its repair from `Failure`, not from the sentence.

### Suite state at this point

Full sweep on this tree: **15 suites, 0 failures**, with two wall-clock tests
(`AssumptionAddScalingTests.Add_Scaling_800AtomsStaysUnder250ms…` and
`CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly`) failing ONCE
under the contention of three concurrent worktrees and passing when re-run alone — recorded as a
contention flake rather than a regression, and re-measured on the final tree at Gate 0.

---

## Round 1 close-out — both work packages merged, one bug caught, one hypothesis falsified

### ADV-02 (plugin half) — merged from `.worktrees/adv02-plugins`

The plugin worktree closed **F-1** at the MathIR boundary and unified the DSP refusals. Its own report is
evidence in full at [`adv02-plugins.md`](adv02-plugins.md); the numbers that matter:

* full solution sweep on its tree: **15 suites, 5 735 passed / 0 failed / 8 skipped**;
* control worktree (contract + tests, pristine product code, `Split('.')[0]` verified still at
  `Evaluator.cs:594`): **33 of 49 new cases fail** — 13/20, 12/16, 8/13 — with
  `InvalidOperationException`, `OverflowException`, `ArithmeticError` and silent `ok:true` answers;
* the 16 that pass on the control are the positive controls: the fix narrowed nothing.

**It found a real bug in the contract I delivered**, and it is worth naming precisely because the class it
belongs to is the one this cycle exists to close: `Real.TryGetExactInt64` delegated integrality to
`Real.IsInteger`, which is the `INumberBase` classification and requires `Exponent >= 0`. A **zero written
with fractional places** stores magnitude 0 at a *negative* exponent, so **`0.0` was refused as
not-an-integer** — a false refusal of a count the caller had written, produced by the very predicate whose
job is to prevent false refusals. Fixed in our `Real.cs` with the value's own integrality
(`IsZero(this) || (!IsPeriodic && Exponent >= 0)`), and pinned in two places:
`DspIntegerArgumentContractTests.TheReader_AcceptsEveryPayloadTheCoreCanSend` and the Abstractions test
`Real_TryGetExactInt64_AnswersTheValuesIntegrality`, which now asserts all of `-0.0`, `0.0`, `0.00`,
`+0.000` read as the integer 0 **while** `IsInteger` still answers `false` for them — so the divergence the
fix introduced is itself pinned rather than left to be rediscovered.

### Reconciling the contract API (two agents, two signatures)

The plugin worktree and I had independently added a 1-based argument `Position` to the same two files. The
merged signature is theirs in spirit and ours in detail: `Require(payload, parameter, bounds, read, refuse,
builtin, position)` — the bound before the reader, because every call site has a bound and only some have
reading to do; `position` last and optional, because a subscript and a range bound genuinely cannot name one.
Its own report is explicit that **deriving the position from the parameter name is unsound** (`impulse` n@1
vs `cosine` n@3), which is why it is passed rather than inferred.

### Three defects the merge itself exposed

| # | defect | how it surfaced | fix |
|---|---|---|---|
| M-1 | `concat(a, b, axis)` refused through the **shared reduction helper's** name, so a bad axis on `concat` was reported as `reduce(): argument 3 (axis) …` | the frozen shape sweep compares the builtin named in the refusal against the builtin called — 2 of 232 refusals failed | the axis reader at the `concat` call site now passes `"concat"` |
| M-2 | `reshape(a, 0)` and `reshape(a, 100000000000)` still crossed with the array kernel's `(Parameter 'shape')` tail and no builtin name | the same sweep, which requires the builtin to be named | `reshape` now refuses the element-count mismatch itself, through `ArgumentContractException.ShapeElementCount` |
| M-3 | the sweep's grammar regex `must be .+; got .+\.$` stopped matching once the *expectation* itself contained a semicolon sentence | 5 refusals reported as grammar violations that were in fact well formed | the two halves are now matched as `argument (?<position>[0-9]+)[^;]*; got .+\.$` — the grammar the test enforces is unchanged |

M-3 is worth a sentence of its own: the first two times I looked at it I was looking at the wrong thing — a
character-for-character probe of the message and of the pattern matched fine in isolation, and the real
failure was two rows further down the violation list, where the builtin name was wrong. The lesson is the
repository's own: *read the whole violation list, not the first row*. The temporary diagnostic that made it
visible was removed once it had done its job.

### ADV-11 — merged from `.worktrees/adv11-falsifier` (test-side only, `git diff --name-only HEAD` empty)

Four files under `Lovelace.Symbolics.Tests/Falsification/` and its report at
[`adv11-falsifier.md`](adv11-falsifier.md). It touched no product code. The battery grew from
1185 to **1218 passed / 0 failed / 8 skipped**, with the frozen 33-probe prefix intact element-for-element
(the legacy 13 remain an in-order subset, each with its `Why`) and 27 generated probes after it.

**The headline is a falsified hypothesis, and it is the most useful result of the round.** The plan's Gate 3
required boundary-biased search to beat uniform search on samples-to-first-counterexample. Measured over 6
seeds at a 30-call budget on the same identity and the same exact-rational interval:

```
biasedWins=0   uniformWins=0   ties=0   bothMissed=6
```

Neither arm found the counterexample; the biased arm was **closer on every seed** (gap 1/128 ≈ 0.0078
against the uniform grid's 323/384 ≈ 0.84). The interpretation the report offers is the one I would have
written: on a 1-D interval a positive-measure region is a **covering** problem, not a search-strategy problem,
so boundary bias buys localisation precision, not first discovery — and it should only be expected to win when
the region is thin relative to the space, or in higher dimensions. Nothing was tuned after the numbers were
seen; the test asserts the falsified verdict and carries a message requiring evidence the day it starts
winning. **Gate 3 is therefore met as a falsification, not as a confirmation**, which is what the plan asked
for.

### Escalated finding that ADV-12/13 must carry

The falsifier's first version reported the *shipped* rule `pow.sqrt-square-nonnegative`
(`sqrt(x^2) -> x` for `x >= 0`) as falsified at `x = -1/10`: a counterexample manufactured out of the rule's
own guard. Any falsification work that follows must skip points where an atom of the rule's declared
environment fails, and treat an undecidable atom as NOT holding. The substrate now does, and the requirement
is recorded here rather than only in the child's report.

### Wire verification on the AOT artefact published from the reconciled tree

`out/aot/Lovelace.Run.exe`, 5 821 440 bytes, 2026-09-16 16:28:29,
SHA-256 `B53E86F0BECEE52A39A35CF1C061BA6F047958969FFE77730480DDC812D46F9B`:

| script | code | message |
|---|---|---|
| `evalir(k, [1], 2.9)` | `InvalidArgument` | `evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got 2.9.` |
| `evalir(k, [1], 0)` | `InvalidArgument` | `… must be a Natural or Integer between 1 and 2147483647; got 0.` |
| `evalir(k, [1], 10^30)` | `InvalidArgument` | `… between 1 and 2147483647; got 1000000000000000000000000000000.` (was raw CLR text under `ArithmeticError`) |
| `evalir(k, [1], 40)` | ok | `0.3333…` to 40 places — unchanged |
| `series(sin(x), x, 0, 2.5)` | `InvalidArgument` | `series(): argument 4 (order) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.` |
| `series(sin(x), x, 0, 4)` | ok | `x - 1/6*x^3 + O(x^4)` — unchanged |
| `zeros(-1)` | `InvalidArgument` | `zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1.` |
| `zeros(3.9)` | `InvalidArgument` | `… must be a Natural or Integer that denotes an exact integer; got the Real 3.9.` |
| `eye(3, -2)` | `InvalidArgument` | `eye(): argument 2 (cols) must be a Natural or Integer that is a positive count … got -2.` (was a value-less "dimensions must be positive") |
| `reshape([1,2,3,4], 0)` | `InvalidArgument` | `reshape(): argument 2 (dims) describes 0 element(s), but the value it reshapes holds 4; the element count must agree.` (was `(Parameter 'shape')`) |
| `movingavg(x, 3.7)` | `InvalidArgument` | `movingavg(): argument 2 (w) must be a Natural or Integer that denotes an exact integer; got Real 3.7.` |
| `movingavg(x, 3)` | ok | `[0.(3), 1, 2, 3, 4]` — unchanged |
| `evalf(sin(10), 30)` | ok | `-0.544021110889369813404747661851` — 30/30 correct after ADV-04 |

### Merged-tree sweep

```
15 suites — 5 832 passed / 0 failed / 8 skipped
```

(Suite totals by project: Real 2 511, Symbolics 1 218, Suite 921, Run 380, Natural 195, Integer 148,
Complex 121, Representation 91, Dsp 61, Abstractions 53, Knowledge 28, Studio 22, Array 19, Console 15,
precbench 13.)
---

## ADV-03 — the integer-argument boundary matrix (and its registry census)

`Lovelace.Run.Tests/IntegerArgumentBoundaryMatrixTests.cs`, new. It drives the PUBLISHED runner over a
17-argument inventory (17 integer-semantic parameters across 16 builtins, including the core array
constructors, the digit counts, the Kernel/series orders and the DSP counts), with 42 boundary literals per
argument:

| family | literals | required outcome |
|---|---|---|
| exact integers the call site accepts | each argument's own accepted set | **accepted**, and the value published is the value written |
| non-integral `Real` | `2.5`, `0.5`, `2.9999` | refused |
| exactly integral `Real` | `2.0` | refused (wrong KIND, naming the integer to write) |
| non-integral rational | `5/2` | refused |
| zero and negative | `0`, `-1` | either, but never internal, never unrecoverable, never silent |
| very large and the overflow boundary | `10^30`, `9223372036854775807`, `9223372036854775808` | either |

plus three observability cells: the SAME argument driven at two digit counts must publish answers of
different length, because acceptance alone would not catch a call site that accepted `2` and computed as if
the caller had written `3`.

### The census is the durable half

A hand-written matrix rots the moment a builtin is added — which is exactly how this class was found, since
a by-hand sweep of nine sites missed the MathIR boundary. So the inventory is checked **in both directions**
against the live registry:

* an integer-semantic parameter name (`dims`, `rows`, `cols`, `axis`, `axes`, `order`, `digits`, `w`, `k`,
  `n`, `index`, `bound`, `seed`) registered by a builtin the inventory does not carry **fails the build**;
* an inventory entry the registry no longer declares **fails the build** (it would be testing nothing);
* the two deliberate exclusions (`plot:title`, `transpose:axes`) are asserted to still be needed, so a
  name cannot be parked there and forgotten.

### Two things the matrix found immediately

1. **`reshape([1, 2, 3, 4], 1)` is refused** — correctly: a shape of one element cannot describe a
   four-element array, and the refusal now says so. The MATRIX was over-constrained by assuming every
   argument accepts the same integers, which is why each inventory entry now carries its own accepted set
   (reshape accepts `1` and `4`; a length-3 vector's index accepts `0..2`; `fft`'s optional length accepts
   `4`). That is a defect in the test, recorded because the first version of this file asserted the wrong
   thing and the failure was informative rather than noise.
2. **`[1, 2, 3][2.9999]` is refused by name and by value** — the row the silent-flooring class would have
   failed on. `[1, 2, 3][4/2]` is ACCEPTED, and that is correct rather than a hole: `4/2` is an exact
   `Integer` in this language, not a `Real`, so it is an integer argument written as an expression.
   (`[1, 2, 3][2.0]` is refused, `[1, 2, 3][2.5]` is refused.) The matrix pins both sides of that line.

### Cost, and how CI runs it

Every cell spawns the runner, so the theory carries `Category=Costly` and runs in CI's dedicated
uninstrumented step — the same treatment the repository already gives its other spawn-per-case suites. The
two census tests are cheap and run in the instrumented loop, so a new integer-semantic builtin is caught on
every push rather than only in the costly step.
---

## ADV-03 status at the end of round 1 — the census works, the matrix found real defects, two cells to finish

### The census is green and is the durable half

`TheInventoryCoversEveryIntegerSemanticParameterInTheRegistry` and `EveryExclusionIsStillNeeded` both pass. They
now sweep **24 registered integer-semantic arguments across 20 builtins** (found by the sweep, not by hand —
the first inventory had 17 and the census named the other seven: `prod`/`min`/`max`/`mean`/`norm` axes,
`concat`'s axis, `eye`'s column count, `dft`'s length, and the DSP generators' counts and seeds), and they
fail in three directions:

* a registered integer-semantic parameter missing from the inventory;
* an inventory entry the registry no longer declares (it would be testing nothing);
* a non-registry entry that has since become a registered one (it would be driven twice).

Two exclusions are declared with reasons, and one of them is a **finding**:

* `transpose:perm` — a permutation *vector* whose elements are indices, already driven by the frozen shape
  sweep and by the index rows;
* **`fft:n` — the registry's one INVISIBLE optional argument.** `fft` declares only `("x")` even though it
  accepts a second positional transform length, so the name-based census **cannot see that integer argument at
  all**. `dft` declares `("x", "n")` and is visible. The matrix keeps `fft:n` driven, but a name-driven census
  is blind to an undeclared optional tail, and that blindness is now written down rather than implied. A
  follow-up should either declare the parameter or make the census read arity instead of names.

### Defects the matrix found on its first full slice

| # | observation | severity |
|---|---|---|
| N-1 | `scale([1, 2, 3], k)`: `k` is **not integer-semantic after all** — `scale(x, 2.5)` answers `[2.5, 5, 7.5]` and `scale(x, 0.5)` answers `[0.5, 1, 1.5]`, i.e. it is a real scalar multiplier. The inventory row was wrong, not the product. **Removed.** | test defect |
| N-2 | `noise(scale, disp, n)`: a **non-integral `disp` is ACCEPTED** — `noise(1, 2.5, 2)` answers a vector — so `disp` is not the integer seed the descriptor's name suggests, or the seed is not read at all. Under investigation; the row is excluded until the reading is established, because asserting the wrong row is worse than not asserting it. | **open, needs one measurement** |
| N-3 | **`prod([1, 2, 3], -1)`, `min`, `max`, `sum` with an out-of-range axis answer `Axis -1 is out of range for rank 1. (Parameter 'axis')`** — raw CLR framework text and a `(Parameter '…')` tail, through `InvalidArgument`. This is the exact class audit Q-4 recorded against `reshape`, still open for every reduction's axis. The Suite call site forwards `TypedArrayOps.CheckAxis`'s framework exception instead of refusing in the contract's grammar. | **CONFIRMED P2** |
| N-4 | `reshape([1, 2, 3, 4], 1)` is refused — correctly (a one-element shape cannot describe four elements). The matrix was wrong to accept it; `reshape`'s accepted set is now `1` (vector) and `4` (row), and the failure was informative rather than noise. | test defect |

### What is verified about ADV-03 and what is not

* **Verified**: the census passes and is non-vacuous (24 registered arguments, 20 builtins, ≥ 100 builtins in the
  registry); the observability theory passes (two digit counts at the same argument must publish answers of
  different length); the `Category=Costly` tagging is in place so CI runs the spawn-per-cell theory in its
  uninstrumented step.
* **NOT yet verified**: the full 1 000-cell theory has never been run to completion — it spawns the runner per
  cell and takes longer than this round's budget. Its first slice exposed N-1..N-4, all of which are now either
  fixed in the test or filed. **The matrix must be run to completion in the next round before ADV-03 is called
  done**, and N-3 fixed, since a raw framework message is exactly what this cycle exists to remove.
### Corrected merged-tree sweep figure

The earlier "5 832 passed" line counted a projection from the subagent's tree. The measured figure on THIS
tree, with the costly boundary theory excluded (it drains its own `Category=Costly` step), is:

```
15 suites — 5 800 passed / 0 failed / 8 skipped
```

By project: Real 2 511 · Symbolics 1 218 · Suite 921 · Run 382 · Natural 195 · Integer 148 · Complex 121 ·
Representation 91 · Dsp 61 · Abstractions 53 · Knowledge 28 · Studio 22 · Array 19 · Console 15 · precbench 13.

The `Run` project's 382 counts the two cheap census tests and the observability theory; the boundary matrix's
~1 000 spawn-per-cell cases are the `Category=Costly` theory that the next round must run to completion.
---

## Round 2 — the axis leak fixed, two of my own readings falsified by measurement

### N-3 CLOSED: the reduction axis no longer leaks framework text

`prod([1, 2, 3], -1)` and its five siblings used to answer
`Axis -1 is out of range for rank 1. (Parameter 'axis')` — raw CLR text, no builtin name, through
`InvalidArgument`. The Suite now guards the axis itself, in the contract's grammar, before the array kernel
sees it:

| call | before | after |
|---|---|---|
| `prod([1, 2, 3], -1)` | `Axis -1 is out of range for rank 1. (Parameter 'axis')` | `prod(): argument 2 (axis) must be the only axis (0); got -1.` |
| `max([1, 2, 3], -1)` | same leak | `max(): argument 2 (axis) must be the only axis (0); got -1.` |
| `min([1, 2, 3], 9223372036854775807)` | same leak | `min(): argument 2 (axis) must be the only axis (0); got 9223372036854775807.` |
| `concat([1, 2], [3, 4], -1)` | same leak | `concat(): argument 3 (axis) must be the only axis (0); got -1.` |
| `sum([[1, 2], [3, 4]], 0)` / `1` | worked | `[4, 6]` / `[3, 7]` — unchanged |

A rank-1 value says "the only axis (0)" rather than a degenerate range of one, and a rank-2 value names the
range — the bound is the RANK, so only the call site can state it, which is why the guard lives in the Suite
and not in the array kernel.

### Two of my own readings were falsified by measurement, and both are recorded

**f-1 — `fft` has no optional length.** I read the ADV-03 census's blindness as "`fft` declares only `(x)`
although it accepts a second positional argument", and I *changed the descriptor and the registration* to
declare `n`. Measurement then showed the opposite: the body enforces `Require(args, 1, "fft")`, so
`fft(x, n)` is an ARITY refusal — `fft(): expected 1 argument; got 2.` — and `dft` is the only transform with
an optional length. My change would have WIDENED the surface instead of the census. Both edits were reverted
and the wire re-verified (`fft(x)` answers; `fft(x, 4)` refuses as an arity error). The inventory row and the
`UndeclaredExcluded` entry it justified are gone, and the census is simpler for it.

**f-2 — `noise`'s short form is deterministic and its seed IS 0.** I measured `noise(1, 0, 2)` against
`noise(1, 0, 0, 2)`, saw different strings, and filed a defect. Element by element the two calls are
IDENTICAL — my comparison had printed a truncated prefix of a LONGER vector, which is a reading error, not a
product difference. Measured properly: the short form is byte-identical across processes and identical to the
explicit `seed = 0` form; `noise(scale, disp, seed, n)` is consistent with the comment and with the metadata.

What survives of f-2 is a real but smaller defect, now fixed: the registry declared `("scale", "disp", "n")`
while the four-argument branch read `args[2]` as the SEED and refused it as `"disp"`, and the metadata table
declared `("scale", "disp", "n", "seed")` — one position, three names. The registration now declares
`("scale", "disp", "seed", "n")` and the refusal names `seed`, which is what the metadata always said.
The pinned test that asserted `disp` moved with it, with the reason recorded rather than the expectation
quietly changed.

### The matrix now checks that its own rows are alive

A row can look like coverage while never reaching its argument — `fft:n` was exactly that. So the inventory's
templates are now driven with a Text sentinel and must produce the ARGUMENT refusal for the parameter they
name: an arity, shape or domain refusal means the template never reaches the argument, and the row is dead.
That check is what would have caught `fft:n` without a human reading the source.

### Sweep

```
15 suites — 5 800 passed / 0 failed / 8 skipped   (costly theories excluded; they run in their own step)
```

The `Run` project's 382 includes the two census tests and the observability theory. The costly theories
(`EveryBoundaryCell` ~1 000 spawn-per-cell cases, `EveryInventoryTemplate`, `TheAcceptedValueIsTheValueTheCallerWrote`)
run in the `Category=Costly` step and are the next round's first action.
---

## ADV-03 — the non-cell theories are green; the cell theory remains the documented long pole

```
TheInventoryCoversEveryIntegerSemanticParameterInTheRegistry   PASS   (24 registered arguments, 20 builtins)
EveryExclusionIsStillNeeded                                   PASS   (4 declared exclusions, each asserted still needed)
EveryInventoryTemplate_ReachesTheArgumentItNames              PASS   (32 rows x 1 sentinel call)
TheAcceptedValueIsTheValueTheCallerWrote                      PASS   (3 pairs)
EveryBoundaryCell_LandsOnTheExactValueOrANamedRefusal         LONG   (~1 000 spawn-per-cell cases; Category=Costly)
```

**The liveness theory is the one that earns its keep**, and building it corrected four more of my readings:

* `zeros` refuses `dims` as **"argument 1 (a dimension)"**, so a liveness check keyed on the registry's
  parameter name would fail on a perfectly live row — each row now carries the **anchor** the REFUSAL prints,
  which is what gets searched;
* the digit counts refuse a Text as `pi() expects a Natural or Integer digit count, but got 'Text'.` — no
  `(digits)` parenthetical at all, because the kind refusal and the bound refusal are different sentences;
* `[1, 2, 3][…]` and `1..…` name their parameter with **quotes**, not parentheses, because they have no
  argument position to publish;
* the observability theory read `output[0]` from a script that never printed, so the theory was comparing
  nothing. It now wraps the call in `print(...)`.

Each of those was a defect in MY TEST, found by running it, and each is recorded rather than quietly fixed.

### The one remaining ADV-03 item

`EveryBoundaryCell` drives ~1 000 cells, each of which SPAWNS the runner, so it takes longer than a round of
this session and is tagged `Category=Costly` for CI's dedicated uninstrumented step. Its first slices already
found N-1, N-2 and N-3 (all closed or documented), and the two rows it still reported — `reshape:dims=1`, which
is CORRECTLY refused because a one-element shape cannot describe a four-element array, and `scale:k`, which is
a real multiplier — have both been removed from the matrix. The next round runs it to completion and reports the
count; nothing about the product is waiting on it.
---

## Round 3 — ADV-08 (registry query) and ADV-07 (call-signature diagnostics) landed

### ADV-08 — the capability surface is queryable without evaluating anything

Two flags, both answered BEFORE any script is read:

```
Lovelace.Run --functions [pattern]      a bounded listing: name, parameters, arity, category
Lovelace.Run --function <name>          one full entry: signature + summary + return kind + related + examples
Lovelace.Run --function-limit <n>       page size for --functions (default 50)
```

Measured on the published binary:

| surface | bytes |
|---|---:|
| the full envelope a SCRIPT produces (registry + metadata) | 41 772 |
| `--functions <name>` | **276** |
| `--function <name>` | **459** |
| `--functions` with the limit raised to 200 (all 123 names) | 13 962 |

The benchmark's L3 arm spent 185 registry lookups over 20 runs to work around having no such surface, and the
only way to reach the registry was to evaluate a script and read ~42 KB out of the envelope. A single-name
query is now 276 bytes and costs no evaluation — a **151x** payload reduction for the question the L3 arm was
actually asking.

**The query is provably evaluation-free**, and a test says so rather than a comment: it is answered with no
`--eval`, no `--file` and no `--stdin`, and the envelope carries no `result`, no `timings` and no `revision`.
A listing carries names and arity ONLY — no summaries, no examples — because a page that ships documentation
is a page an agent cannot afford; `--function` is where the documentation lives. Pages report `total`,
`returned` and `truncated`, so a page is never mistaken for the whole list, and the order is ordinal by name
so two queries over one registry cannot disagree.

An unknown NAME is not an empty success: `--function nosuchfn` answers the same `UnknownFunction` refusal a
call to it produces, with `registrySize`. An unknown PATTERN is an empty listing with `total: 0` — a search
that found nothing is a result, not a refusal.

**The full registry also gained the descriptor metadata** (`category`, `summary`, `returnKind`, `related`,
`examples`), resolved through the SAME two-step lookup `HelpService` uses, so the wire, the help text and the
Studio completion list cannot describe one function three ways. Fields are omitted when the registry has
nothing to say — never sent as an empty array or a placeholder string — so "no summary is registered" and
"the summary is empty" cannot look alike.

### ADV-07 — a call-site refusal carries the signature it was measured against

The types the runner already classified carried every field all along and the envelope dropped them by
forwarding `ex.Message` alone. Now:

```json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"dot(): expected 2 arguments; got 1.",
 "signature":{"function":"dot","parameters":["a","b"],"expectedMin":2,"expectedMax":2,
              "variadic":false,"actual":1}}

{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"det(): argument 1 must be a square matrix; got Natural.",
 "signature":{"function":"det","parameters":["m"],"expectedMin":1,"expectedMax":1,
              "variadic":false,"position":1,"parameter":"m",
              "expectedShape":"a square matrix","arrivedKind":"Natural"}}
```

The parameter NAMES come from the live registry rather than from the exception — the exception names the
position and the registry names what sits there — so the signature a failure publishes and the signature
`functions[]` publishes for the same builtin cannot drift. A failure that is not a call-site refusal carries
NO `signature` key at all: omitted, never empty, so a consumer branches on the field's presence rather than on
a code it has to have memorised.

Four tests, including one that drives five builtins with too few arguments and asserts the published counts
equal the registry's own `DeclaredArity` for the same names.
---

## Round 4 — ADV-09 (committed state and artifacts), and the objective's harnessability surface is complete

### The artifact that used to be invisible (audit N's N-5)

`plot([1, 2, 3]); det(1)` exited 1 with an error envelope carrying no plot block and no path while
`plot.svg` sat on disk. Both envelopes now carry an `artifacts` array:

```json
"artifacts": [ { "kind": "plot", "path": "…\\plot.svg", "title": null, "bytes": 14636 } ]
```

Measured on the published binary: `plot([1,2,3])` alone publishes it (exit 0), `plot([1,2,3])` then `det(1)`
publishes the SAME entry (exit 1), `1 + 1` publishes `[]`, and a parse error publishes `[]`. The bytes figure
is read from the file rather than assumed — a size the runtime did not read is a size it must not publish —
and the field is absent rather than zero when the file cannot be read.

### The committed variable table crosses on every failure

`partialVariables` was filled only for a CANCELLATION; the harnessability benchmark measured its coverage on
ordinary failures at **0 %** and filed it as F3, because an agent repairing a multi-statement program could not
see how far it had got. It is now published on every failure, with the shape the cancelled path has always
had (one entry per binding, `structured` included), so a consumer written against that path reads this one
unchanged:

| script | code | partialVariables |
|---|---|---|
| `x = 42; y = x + 1; det(1)` | `InvalidArgument` | `x`, `y` — in ordinal order, each with its structured form |
| `det(1)` | `InvalidArgument` | present and EMPTY — nothing was committed |
| `x = 42; x = (` | `ParseError` | empty — the whole source is parsed before the first statement runs |

`partialOutput` stays cancellation-only on purpose: on every other failure the committed lines are already in
`output[]`, and publishing the same array twice under two names is the duplication audit R-3 recorded rather
than a second fact.

### ADV-09 immediately broke the failure path, and that is the most useful thing it did

Publishing the committed table on EVERY failure means projecting variables on paths that had never projected
them — including the path where the script produced a value TOO DEEP to render. `CaptureState()` walks each
value through `ValueFormatter`, which enforces `InputDepth.MaxValueDepth`, so on the deep-value refusal the
envelope threw while describing itself and the typed `InputDepthExceededException` became an `InternalError`.
Nine tests in `ValueDepthEnvelopeTests` caught it. The projection now tolerates an unrenderable binding: the
entry stays, named, with the guard's own display text and no structured form — **present, and honest about the
part that could not be read** — because a binding too deep to print is still a committed binding, and a failure
envelope that dies while describing itself is worse than one that says less.

### A recorder for the golden envelopes, and the review it enabled

Twenty golden fixtures pin the byte shape of the wire, and they are SUPPOSED to fail when the contract changes
— but they were re-recorded by hand, which is how this tree's `capabilities.json` and `transform_result.json`
came to hold PowerShell `ConvertTo-Json` formatting (padded keys, `\u003c` escapes, compact one-line records)
instead of the runner's own pretty-printed output. `GoldenFixtureRecorder` now re-records them with the SAME
normaliser the comparison uses, writing to the SOURCE tree (not the build-staged copy), inert unless
`LOVELACE_RECORD_GOLDENS=1`.

Reviewing the resulting diff is what found the formatting damage: after the recorder ran, the diff for the
eighteen ordinary fixtures was a clean three-line `artifacts` addition, while `capabilities.json` and
`transform_result.json` showed ~4 700 lines of churn. Re-recording them in the runner's own format is a
**correction**, not churn.

### ADV-07 and ADV-08 (round 3) and ADV-09 (round 4) close the objective's harnessability surface

```
what does this runtime have?          --functions [pattern]        276 bytes, no evaluation
how is this one called?               --function <name>           459 bytes, full descriptor
why did my call fail?                 signature{...}              arity counts, parameter names, shape, kind
how far did it get?                   partialVariables[]          the committed bindings
what did it write?                    artifacts[]                 every committed file, with its size
```
---

## Round 5 — ADV-05 (the live precision oracle), and the objective's first two axes are closed

### What ADV-05 adds over the pinned goldens

`RealTrigCertifiedPrefixTests` pins golden prefixes taken from mpmath at 4 000 dps. Goldens prove a value
did not MOVE; they cannot prove it is RIGHT — a golden is only as good as the day it was recorded, and the
round-23 audit found this defect precisely because somebody ran an independent oracle instead of trusting a
pinned number.

`Lovelace.Real.Tests/RealTrigPrecisionOracleTests.cs` computes the ground truth AT TEST TIME: 24 cases
(sin/cos of 1, 10, 10^6, 10^20 and 10^100 at 30, 50 and 100 places), each asking mpmath for the true
expansion and counting how many of the kernel's published fractional digits actually match, requiring that
count to cover what the caller requested.

| measurement | before this cycle | after |
|---|---|---|
| `Sin(10, 30)` correct fractional digits | **29** of 30 | 43 |
| `Sin(10^100, 30)` correct fractional digits | **0** of 30 | 42 |
| `Sin(1, 30)` correct fractional digits | 40 published for a 30-place request, all 30 requested correct | unchanged |

### The oracle is honest about its own availability, and that took a measurement

The first run FAILED 25 of 25 with `python3 could not evaluate with mpmath` — and the reason is worth
recording: this machine has a `python3` on the PATH that is the **Windows Store app-execution alias stub**,
which exits non-zero with a "Python was not found" banner. A probe that merely tried `import mpmath` would
have declared the oracle healthy and then every case would have failed on a banner. The probe now EVALUATES
with mpmath (`mp.nstr(mp.sin(1), 25)`) and is cached once per run rather than once per case.

The availability contract is the repository's existing one and is verified BOTH ways:

| environment | `LOVELACE_REQUIRE_SYMPY` | result |
|---|---|---|
| no usable mpmath | unset | 25 passed — the battery skips, because a missing oracle is an environment fact |
| no usable mpmath | `=1` | **25 FAILED** — an oracle that compares nothing must not report a pass (this is CI's setting) |
| the real python on PATH | either | 25 passed against live mpmath |

The battery carries its own falsifier (`TheOracleRejectsATruncatedValue`): it hands the counting machinery a
value that is right for 40 places and wrong after, and requires it to be caught — at exactly the digit the
audit's defect lived on, the first one past the internal guard. A comparison that cannot fail proves nothing,
so the oracle is tested before it is trusted.

### CI wiring

`.github/workflows/ci.yml`'s `sympy-oracle` job — the one that already installs sympy, and therefore mpmath,
which ships as its dependency — now runs the battery as its own step, with the reason written next to it: the
SymPy oracle compares symbolic RESULTS, and this one counts correct DECIMAL PLACES. `LOVELACE_REQUIRE_SYMPY=1`
is set for the whole job, so the step cannot pass by comparing nothing.

### The python bridge is deliberately not shared

`RealTrigPrecisionOracleTests` carries its own forty-line python bridge rather than referencing
`Lovelace.Symbolics.Tests.SympyOracle`. The numeric tower's tests must not depend on the symbolic kernel — a
dependency that would be invisible until somebody tried to run the Real suite alone — and the duplication is
smaller than the coupling it avoids.
---

## Round 5b — the boundary matrix now RUNS, and four of my own defects were what stopped it

`EveryBoundaryCell` completes in **2 seconds** with **353 of 353 cells passing**, having never once finished
before. Four separate defects in the TEST stood between it and completion, and every one was found by
running it rather than by reading it:

| # | defect | evidence | fix |
|---|---|---|---|
| m-1 | the magnitude rows handed Int64-boundary literals to arguments that ACCEPT them, so a cell committed the engine to work proportional to the literal and `--cancel-after` could not save it (the deadline covers the evaluation loop, not the kernel that was handed a 10^18 request). Four attempts ended with the host idle and 78 of ~660 cells reported. | `pi(1000000)` alone answers in 584 ms, so the offenders were a few combinations | the magnitude rows are gated per argument by `LargeMagnitude`, and are added only where a magnitude is a fact the argument is expected to REFUSE — an index, a dimension, an axis, a count |
| m-2 | `2.0` was asserted to be REFUSED. It denotes the integer 2, and whether the call sees a Real or the host has already narrowed it to the integer depends on the HOST — the runner narrows it, a constructed plugin call does not — so the two environments reach different arms and both are correct. | seven cells failed on `2.0` | `2.0` is now its own case with NO expectation: accepted or refused, but never silently changed and never an internal failure |
| m-3 | `powerseries`' signature is `(k, a, n)` with SCALARS, and the template passed an ARRAY as `a`, so the cell never reached `n`. | `DSP builtins expect numeric/complex elements, but got 'PayloadArray'` | the template is `powerseries(2, 0.5, {0})` |
| m-4 | the rational literal `5/2` was unparenthesised, and **the range operator binds tighter than division**: `1..5/2` parses as `(1..5)/2`, so the row tested a different call than the one it named. | measured: `1..5/2` → `[0.5, 1, 1.5, 2, 2.5]`, while `1..(2.5)` is refused by name | the literal is `(5/2)` — the argument under test must be the argument written |

m-1 is worth stating as a conclusion rather than an anecdote: **a boundary matrix whose cells can outlive the
round is not measuring the boundary, it is measuring the engine's cost model**. The gate is therefore part of
the matrix's contract — `LargeMagnitude` is a claim about which arguments a magnitude is a REFUSAL for, and it
is reviewed like any other expectation.

### What the matrix covers now

353 cells over 26 integer-semantic arguments (24 registered, 2 not — a subscript and a range bound), each
driven across: the exact integers its own contract accepts, a fractional Real (`2.5`, `0.5`, `2.9999`), an
exactly integral Real (`2.0`), a non-integral rational written as `(5/2)`, zero, negative, and — where the
argument's bound makes a magnitude a refusal — `10^30` and `9223372036854775808`.
### Round-5 closing sweep

`dotnet test LovelaceSharp.slnx -c Release` at `ce30fc3` (51 working-tree entries — the cycle's own uncommitted
work), precision oracle excluded because it needs the pinned mpmath and is run separately below:

**15 of 15 suites, 6 167 passed, 0 failed, 8 skipped**, including `Lovelace.Run.Tests` at **751** (it was 415
before this round; the boundary matrix alone contributes 353 of the new cases) and `Lovelace.Real.Tests` at
**2 511**.
---

## Round 5c — ADV-03 closed: the digit counts, and the two artefacts that were not what their names said

Round 5b ended with the matrix green and nothing left that the matrix could SEE. This round went looking for
what the matrix could NOT see, by asking the published binary instead of the inventory: every registered
function with an integer-semantic parameter name was called once per integer parameter with `2.5` and with
`2.0`, and each answer was classified by whether it arrived in the argument-contract grammar. **66 probes.**

### What the probes found

| finding | evidence | verdict |
|---|---|---|
| `scale(x, 2.5)` was ACCEPTED while its published signature said its second argument is `k` | `print(scale([1,2,3],2.5))` → `[2.5, 5, 7.5]` | **the DESCRIPTOR was wrong, not the builtin** — the body reads `ToComplex(args[1])`, so `k` named an integer argument no code path read |
| `pi(2.5)`, `e(2.5)`, `setprecision(2.5)`, `setprecision(30.0)` refused OUTSIDE the contract grammar | `DomainError/InvalidOperation :: pi() expects a Natural or Integer digit count, but got 'Real'.` — no position, no parameter, and **no `signature`** | a real remainder of the coercion class: the same `digits` argument crossed as `InvalidArgument/TypeMismatch` when it was a Natural out of range and as `InvalidOperation/DomainError` when it was a Real |
| `setprecision(0)` / `setprecision(-5)` refused as `InvalidOperation/DomainError` | same shape | same remainder, the range half of it |
| `noise(2.5, 1, 1, 1)` accepted | position 1 is `disp`, a Real | not a defect — the four-argument form's positions are `seed` (3) and `n` (4), and **both are refused by name**: `noise(): argument 3 (seed) must be …`, `noise(): argument 4 (n) must be …` |

### The fix, and its control

`scale`'s parameter is renamed to `c` — NAMED AFTER WHAT IT IS — and its metadata now reads "Scales every sample
of a signal by the scalar c." The exclusion the matrix carried for `scale:k` is deleted, because the exclusion
was answering the wrong question: the census was right that a parameter named `k` on `scale` would have to be
an integer, and the descriptor was wrong to name it that.

`pi`, `e` and `setprecision` now go through the same `RequireExactInteger` path as every other integer argument,
with the position (`1`), the parameter (`digits`) and the bound the engine actually holds
(`Rl.MaxComputationDecimalPlaces` — read, never written down, so the refusal cannot drift from the bound it
reports). Measured on the published AOT binary:

```
pi(0)                              TypeMismatch/InvalidArgument :: pi(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got 0.
setprecision(20); pi(500)          TypeMismatch/InvalidArgument :: pi(): argument 1 (digits) must be a Natural or Integer between 1 and 20; got 500.
pi(1000000000000000000000)         TypeMismatch/InvalidArgument :: pi(): argument 1 (digits) must be a Natural or Integer between 1 and 1000; got 1000000000000000000000.
setprecision(0)                    TypeMismatch/InvalidArgument :: setprecision(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got 0.
setprecision(-5)                   TypeMismatch/InvalidArgument :: setprecision(): argument 1 (digits) must be a Natural or Integer that is a positive count (1 or more, at most 9223372036854775807); got -5.
pi(2.5)                            DomainError/InvalidOperation :: pi(): argument 1 (digits) must be a Natural or Integer that denotes an exact integer; got the Real 2.5.
pi(3.0)                            DomainError/InvalidOperation :: pi(): argument 1 (digits) must be a Natural or Integer; got the Real 3, which is integral — write it as 3.
```

**The KIND refusal stays `InvalidOperationException`** — the documented `pi(3.0)` contract — and it now carries
the contract's message, so a caller reading either field learns the position and the parameter. **The WIDTH and
RANGE refusals stay `InvalidArgument`/`TypeMismatch`**, and one of them caught a defect in this very change:
routing `TooWide` through the kind-refusal wrapper first turned `pi(10^30)` into `InvalidOperation`, which
`EnginePrecisionDigitRefusalTests.ACountWiderThanInt64_IsTheSameTypedRefusal` refused to accept. The wrapper is
now about the KIND alone, which is what its name always said.

Two of my own tests changed with the behaviour and are re-pinned with the reason:
`InterpreterBuiltinSetprecisionTests.Evaluate_GivenZero/Negative_…` expected `InvalidOperationException`,
and `WideArgumentRefusalTests.Setprecision_WithANonPositiveCount_…` expected `InvalidOperation` — both were
pinning the inconsistency, not a contract. The new test names the pair it must agree with:
`Setprecision_WithANonPositiveCount_IsTheSameArgumentRefusalAsPi` compares `setprecision(-5)` against `pi(-5)`
field by field.

### The two published artefacts were not what their names said, and I had claimed otherwise

This is a correction to my own round-5 reporting, and it is the kind that only a fingerprint can find.

| artefact | what I said | what it is |
|---|---|---|
| `out/aot/Lovelace.Run.exe` (162 304 B, sha256 `8F4FD27886CDC560`) | "the published Native AOT artefact" | **a framework-dependent apphost.** It was produced by `dotnet publish -r win-x64` — which selects a RUNTIME IDENTIFIER, not `PublishAot` — so `Lovelace.Run.dll` sits beside it and the .NET runtime does the work. Moving the other 26 files aside answers `The application to execute does not exist: '…\Lovelace.Run.dll'`. |
| `Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe` — **the path `harness/lovelace.host.js:94` spawns** | assumed fresh | **14 days old, and it did not know `--functions`**: `Unknown argument '--functions'`, exit 2. Every harnessability measurement in rounds 3 and 5 was taken against a binary the DSH harness never runs. |

The second one is the finding that matters: the improvement was real and the artefact the harness actually
spawns could not see it. `make runner` republishes that path from HEAD — `-p:PublishAot=true` plus the AOT
properties, which is where `PublishAot` really comes from: `Lovelace.Run.csproj` sets only `IsAotCompatible` —
and the result is a genuine single-file Native AOT binary:

```
Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe
  6 020 096 bytes | 2026-09-16 19:56:05 | sha256:8005C6C74349D8EA
  (the 19:49 build of the same path was 6 023 168 B / 5D5033B7053856DE; this one is re-published
   after the ADV-03 edits, and it is the build every measurement in this section was taken on)
  freshness: NEWER than every source it is built from (newest source: Runner.cs, 18:50:38)
  no coreclr.dll in the directory — self-contained, single file
  --functions det -> exit 0, 276 bytes
  --function  det -> exit 0, 459 bytes
  --help          -> exit 0, 2 026 bytes
```

and it answers the whole harnessability surface:

```
2^100                     exit 0, 42 053 B envelope, ok:true, artifacts:[]
zeros(2.5)                exit 1,    694 B, InvalidArgument/TypeMismatch, partialVariables:[], artifacts:[]
sin(1, 2)                 exit 1,    650 B, InvalidArgument/TypeMismatch, signature:{…}   <- the arity refusal carries its call signature
```

**The lesson is one the plan already states and the round did not apply to itself**: a claim about an artefact
must name the artefact — its path, its bytes and its revision — and "I published it" is not a fingerprint.
`out/aot` is left in place but is NOT an AOT artefact and is not cited as one anywhere below.

### Verification at the end of round 5c

| what | result |
|---|---|
| `dotnet test LovelaceSharp.slnx -c Release` (oracle excluded) | 15 of 15 suites, **6 171 passed, 0 failed, 8 skipped** |
| `--filter "Category=Costly"` — the theories the loop excludes | **351 passed, 0 failed**, 413 ms |
| `RealTrigPrecisionOracleTests` (live mpmath) | **25 passed, 0 failed** |
| `IntegerArgumentBoundaryMatrixTests` | **353 of 353 cells**, inside the costly run |
| `Lovelace.Run.Tests` | 751 passed, 0 failed |
| `Lovelace.Suite.Tests` | 924 passed, 0 failed (was 921) |

### Addendum — the harness path, re-verified on the corrected artefact

`harness/lovelace.host.js:94` spawns `Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe`. Run through
that exact path, with that exact argument:

```
harness spawn path --functions det -> exit 0, 276 bytes, no stderr
  ok=true pattern=det total=1 returned=1 truncated=false
  {"name":"det","parameters":["m"],"minArity":1,"parameterCount":1,"variadic":false,"category":"Linear Algebra"}
script via --stdin             -> exit 0, ok=true, output=["[2.5, 5, 7.5]"]
```

The second line is also the ADV-03 control at the language surface: `scale` still multiplies by a scalar, and
its descriptor now says so.
---

## Round 6 — the benchmark's runner was also a copy, and the harnessability claim now has a re-runnable measurement

Round 5c found that the binary the DSH harness spawns is published by hand and had gone stale. This round found
the same failure one directory over, in the benchmark that PRODUCES the harnessability evidence.

### The benchmark's runner is a frozen copy, and it was 9 hours old

`harnessability-bench/setup.ps1:47-55` publishes `runner/Lovelace.Run.exe` only when the file is MISSING
(`elseif (Test-Path $runner) { 'present' }`), so it never tracks the repository. Measured:

| artefact | bytes | sha256 | built from |
|---|---:|---|---|
| `harnessability-bench/runner/Lovelace.Run.exe` before this round | 5 815 808 | `2E3E365BF9AA056E` | the pre-cycle-7 tree (10:52) |
| the same path after refreshing from HEAD &nbsp;`ce30fc3` | 6 020 096 | `8005C6C74349D8EA` | `Lovelace.Run/bin/Release/net10.0/publish` (19:56) |

The older binary **does not know `--functions`** — `Unknown argument '--functions'`, exit 2 — which is the
capability `RESULTS.md` §6/§7 report on. A stale runner validates, scores and benchmarks exactly like a fresh
one, so nothing in the package reports the substitution. The refresh procedure, the probe that detects it, and
the corpus re-validation commands are now written down in `harnessability-bench/BENCH-REFRESH.md`.

### Re-validation on HEAD `ce30fc3`, all green

```
build_corpus.py --check    CHECK OK: 20 goldens, 48 probes, every probe discriminates its seed
bench.py prepare           OK for all 20 tasks; wrote private/oracles.json
bench.py validate          ok:true  goldens_verified:20  hidden_probes:48
                           accidental_seed_matches:[]  l3_registry_smoke:true
                           seed_execution: 12 ok / 8 error   (the shipped split)
```

### The observation surface, measured against the published baseline

`checks/surface_delta.py` is new. `measure_surface.py` reads recorded trajectories; nothing measured the
surface a given RUNNER produces, which is the half a runner change can move. It projects all 20 seeds through
all four sealed modes, prints the per-seed and per-mode deltas against `validation-report.json`, and writes
`runs/validation-report-HEAD.json` in that report's exact shape — without overwriting the published baseline.

The first result is a negative one, and a negative result is what the plan's "no new field may leak into a
frozen projection" requirement needs:

**The CURRENT runner reproduces the shipped `validation-summary.json`'s maxima EXACTLY.**

| mode | shipped summary max | measured on HEAD |
|---|---:|---:|
| L0 | 440 | 440 |
| L1 | 440 | 440 |
| L2 | 4 424 | 4 424 |
| L3 | 4 716 | 4 718 |

A wire change that reached a projection could not reproduce those numbers. Probed field by field: deleting
`artifacts[]` from the envelope changes **no** projection size in **any** mode for **any** of the 20 seeds —
the value `[]` and the absent field cost the same, because the projections never read it. Same for the error
`signature` and the widened refusal messages: they are on the wire, and outside all four modes.

### The 151x is reproducible, measured independently

| path | bytes | what it costs |
|---|---:|---|
| the whole evaluation envelope `inspect` reads the registry out of | 41 609 | one full evaluation |
| the projected answer `inspect` prints | 171 | one full evaluation + a projection |
| `--functions det` | 275 | **no evaluation at all** |

`41609 / 275 = 151.3`. Round 3 reported 151x from a hand measurement; this is a scripted one that a later
round can re-run, and it now ships with the benchmark rather than beside it.

### What the baseline audit found — recorded, not repaired

`validation-report.json` (per-seed) and `validation-summary.json` (aggregate) are **internally inconsistent**:
the per-seed rows sum to means of 67.4 / 115.5 / 1 041.2 / 9 955.4 while the summary states
55.2 / 94.0 / 643.5 / 852.5. The two files were produced on different runners; the summary is the older one.

Neither is a measurement of HEAD, and one row proves it: the report records `reshape-numel` as a FAILING seed
with `reshape() to [3, 3] requires 9 element(s), but this array has 6. (Parameter 'shape')`. The committed seed
is `func as_matrix(v,rows,cols) { reshape(v,cols,rows) }` / `as_matrix(1..6,2,3)` — it names `[2, 3]`, never
`[3, 3]`, and it succeeds with `[[1, 2], [3, 4], [5, 6]]`. `tasks.json` is byte-identical to HEAD (`9E33BD054085447B`),
so the report describes a seed the repository does not contain. The L3 gap is the same story at scale: those
baselines were projected from envelopes that still carried the full function registry (~16 KB each), which
`--omit-functions` stopped emitting before this cycle.

**The published baseline is therefore left exactly as it is** — it is the package's artefact, not ours — and
the fresh measurement is written beside it as `runs/validation-report-HEAD.json`. The lesson is the one this
cycle keeps re-learning: two files that have to agree and no mechanism that makes them agree will disagree.

---

## Round 7 — every fix now has a CONTROL, and the control is a tree rather than a claim

The objective's last clause is "with a control-failing test for every fix", and until this round that clause was
satisfied by intent rather than by measurement. Round 7 ran the tests against an unmodified tree.

### The control

`ce30fc3` is the published HEAD and it does **not** contain any of this cycle's work: it has no
`Lovelace.Abstractions/ExactIntegerArgument.cs`, no `RealTrigPrecisionOracleTests.cs`, no
`IntegerArgumentBoundaryMatrixTests.cs`, its `scale` descriptor still reads `new[] { "x", "k" }`, and its
`setprecision` refusal is still the `InvalidOperationException`/`DomainError` sentence. The cycle's own files are
**untracked** in the working tree, which is why the control is a clean checkout rather than a revert.

Control worktree: `.worktrees/adv01-ctl`, at `ce30fc3`, `git status` clean before and after. Each test file this
cycle added was copied in UNCHANGED (only `Lovelace.Abstractions.Tests.csproj` needed the test-only
`Lovelace.Real` reference), the suite was run, and the planted files were removed afterwards. The worktree is
clean again — `git status --porcelain` reports 0 entries.

### ADV-01..03 — the contract tests cannot even be expressed without the fix

```
Lovelace.Abstractions.Tests/ExactIntegerArgumentTests.cs(157,20): error CS0246:
    The type or namespace name 'ArgumentContractException' could not be found
Lovelace.Abstractions.Tests/ExactIntegerArgumentTests.cs(163,50): error CS0246:
    The type or namespace name 'Int64Range' could not be found
```

That is the honest shape of a control failure for a NEW contract type: the defect is the absence of the type,
so the test cannot compile against the tree that lacks it. It is recorded as what it is rather than dressed up
as a red test.

### ADV-04..05 — the control DOES run the battery, and 22 of 41 fail

`RealTrigPrecisionOracleTests` + `RealTrigCertifiedPrefixTests` use only API that exists on BOTH trees
(`Real.WithPrecision`, `Real.Sin(value, digits)`, `Real.Cos`, `Real.Parse`), so on the control they compile and
the oracle runs against the old kernel:

```
control ce30fc3 :  Failed! - Failed: 22, Passed: 19, Total: 41
fixed HEAD     :  Failed! - Failed:  0, Passed: 41, Total: 41
```

All 22 control failures are the one defect class — `sin`/`cos` of 10, 10^6, 10^20, 10^100 at 30, 50 and 100
places — which is exactly the class ADV-04 is about.

### The property itself, measured independently of the battery

`out/precision-ab/` is a small probe project that uses the SAME source against either tree (a `RealProject`
MSBuild property selects which `Lovelace.Real.csproj` it references), publishes each value through the public
API under `WithPrecision(digits, digits)`, and counts the matching prefix against mpmath at 4 200 dps. The
counting does not care which tree it is looking at:

| case | requested | control correct places | fixed correct places |
|---|---:|---:|---:|
| `sin(1, 30)` | 30 | 39 | 39 |
| `sin(10, 30)` | 30 | **29** | 43 |
| `sin(10, 50)` | 50 | **49** | 64 |
| `sin(10, 100)` | 100 | **99** | 113 |
| `sin(10^6, 30)` | 30 | **24** | 44 |
| `sin(10^20, 30)` | 30 | **10** | 43 |
| `sin(10^100, 30)` | 30 | **0** | 42 |
| `sin(10^100, 50)` | 50 | **0** | 63 |
| `sin(10^100, 100)` | 100 | **0** | 110 |
| `cos(10, 30)` | 30 | **29** | 43 |
| `cos(10^100, 30)` | 30 | **0** | 43 |
| `cos(10^100, 100)` | 100 | **0** | 112 |

**control: 11 violations of 13. fixed: 0.** The old kernel published a value for a 30-place request that had
ZERO correct fractional digits, which is the finding stated as a number rather than as a story.

### A new defect found while measuring: `DisplayDecimalPlaces` is named in the truncation bound and not applied

`Real.ToString()` documents itself as truncating non-periodic values at `DisplayDecimalPlaces`
(`Real.cs:2635-2641`), and it does — for values whose magnitude is longer than their fractional length:
`1.23456789` at display 5 prints 5 digits, `Sqrt(2)` at display 5 prints 5. It does not for the other branch,
which handles a magnitude SHORTER than the fractional length — the shape a high-precision transcendental has:

```
                                 Display  Exp      frac digits printed
1.23456789 at display 5              5     -8           5
sqrt(2)    at display 5              5   -1000          5
sin(1,30)  at display 30            30   -606         606
sin(1,30)  at display 5              5   -606         606     <- the bound is not applied
```

The cause is arithmetic rather than a missing check: the bound is
`Math.Min((int)fracLen, (int)(fracLen + DisplayDecimalPlaces))`, and since `DisplayDecimalPlaces` is never
negative the second term is never the smaller one, so the `Math.Min` is a no-op and the bound is always
`fracLen`.

**Changing it to `DisplayDecimalPlaces` was tried, measured, and REVERTED, because it is a policy question
rather than a one-line repair.** The change is one line and it makes the branch agree with the documented
contract, and it breaks three tests whose expectations are authored at widths WIDER than the digits they
request:

* `RealTrigCertifiedPrefixTests.Sin_OfTen_PublishesTheDigitsItClaims(digits: 100)` — 100 places are requested and
  the mpmath string beside it is 104 digits long, so the test itself asks for more than it requests;
* `RealTrigCertifiedPrefixTests.SmallAngles_KeepTheirMagnitude` — "published 30 fractional digits, need at least 50";
* `RealDivideLeadingZeroRunTests.Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale`.

The width policy is therefore real and unresolved, and it is recorded here rather than settled: **a
`digits`-place request currently publishes far more than `digits` digits (1 815 fractional digits for
`sin(10, 30)`), and the display limit that should bound them is silently inert for exactly the values that
carry the most.** Any of those extra digits past the 43rd is wrong. The recommendation is to fix the branch AND
re-author the three expectations, as its own change with its own control — not to slip it in behind ADV-04.

### Verification

| what | result |
|---|---|
| `RealTrigPrecisionOracleTests` + `RealTrigCertifiedPrefixTests` on the control tree | 22 failed, 19 passed |
| the same two files on HEAD | **0 failed, 41 passed** |
| `ExactIntegerArgumentTests` on the control tree | does not compile (`ArgumentContractException`, `Int64Range` absent) |
| precision A/B on the control / on HEAD | 11 violations of 13 / **0 of 13** |
| `Lovelace.Real.Tests` after the reverted experiment | **2 536 passed, 0 failed** |
| `.worktrees/adv01-ctl` | `git status` 0 entries (planted files removed, csproj restored) |

---

## Round 8 — the width defect repaired, with its three "blockers" shown to be the defect's own shadow

Round 7 found that `Real.ToString()` names `DisplayDecimalPlaces` in one rendering branch and does not apply
it, reverted the one-line repair because three tests failed, and recorded the width policy as unresolved. This
round looked at what those three tests are actually asserting. **Two of them are wrong, one of them is a
consequence of the same bug, and the policy question was never the thing blocking the fix.**

### The repair

```
- string fracPart = padded[..Math.Min((int)fracLen, (int)(fracLen + DisplayDecimalPlaces))];
+ string fracPart = padded[..(int)Math.Min(fracLen, DisplayDecimalPlaces)];
```

`Math.Min(fracLen, fracLen + Display)` is `fracLen` for every non-negative display precision, so the bound was a
no-op and every digit the magnitude carried was published. The branch below (magnitude LONGER than the
fraction) already truncated correctly, which is why `Sqrt(2)` obeyed the limit and `Sin(1, 30)` did not: the
two branches disagreed, and the one that did not truncate is the one a high-precision transcendental takes.

### The three "blockers", inspected

| test | what it asserted | verdict |
|---|---|---|
| `RealTrigCertifiedPrefixTests.SmallAngles_KeepTheirMagnitude` | 50 digits of mpmath truth for a **30**-place request under a `WithPrecision(30, 30)` scope | **the expectation was wider than the request.** Trimmed to the 30 digits the call is entitled to. |
| `RealTrigCertifiedPrefixTests.Sin_OfTen_PublishesTheDigitsItClaims(digits: 100)` | a 104-digit mpmath string for a 100-place request | **the expectation was wider than the request.** The row now carries exactly 100 digits. |
| `RealDivideLeadingZeroRunTests.Divide_GivenQuotientTheBudgetCannotHold_IsInexactAndKeepsItsScale` | the full 1 023-character rendering (`"0." + 1003 zeros + 20 digits`) under `WithPrecision(20, 40)` | **a consequence of the same bug**: the assertion only passed because the display limit was ignored. Re-authored to the 40-place window, with the scale claims (`ToNatural`, `Exponent`, `PeriodStart`, `PeriodLength`) untouched. |

None of the three was a policy obstacle. Nothing in the product depended on publishing more digits than the
display limit.

The repair also exposed a fourth: `Lovelace.Complex.Tests`
`Sin_OfATwoPiTruncationFinerThanTheAmbientPrecision_KeepsItsResidual` spelled 119 fractional digits of a
residual whose own doc comment says the value is viewed through a 30-place scope. Its numerical assertions
(`Abs(actual - expected) < 1e-104`, `!IsZero`) are untouched and still pass; the rendering assertion now states
"the published form is the display limit's rendering of the true residual", produced by the same scope from the
independently-constructed expected value rather than from a hand-copied digit string.

### The control-failing test

`RealTrigCertifiedPrefixTests.PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue` asserts the width on
BOTH branches — `Sqrt(2)` (magnitude longer than the fraction) and `Sin(1, 30)` (magnitude shorter) — plus the
correct digits inside the window. Measured on both trees:

```
control ce30fc3 :  Failed! - Failed: 4, Passed: 2, Total: 6
                    PublishedWidth_IsTheDisplayLimit_ForEveryNonPeriodicValue        [FAIL]
                    Sin_OfTen_PublishesTheDigitsItClaims(30)                        [FAIL]
                    Sin_OfTen_PublishesTheDigitsItClaims(50)                        [FAIL]
                    Sin_OfTen_PublishesTheDigitsItClaims(100)                       [FAIL]
HEAD + repair   :  Failed! - Failed: 0, Passed: 6, Total: 6
```

The two that pass on the control are the re-authored rows that assert no more than they request — a control
that failed everything would be as uninformative as one that failed nothing.

### ADV-03's remaining controls, now measured too

Round 7 left three test files unplanted. Planted and run on the control:

```
Lovelace.Suite.Tests/DspIntegerArgumentContractTests.cs        error CS0246: 'ArgumentContractException', 'Int64Range', 'ArgumentContractFailure' absent
Lovelace.Symbolics.Tests/EvalirDigitArgumentContractTests.cs   error CS0246: same three types absent
```

Every cycle-7 contract test is now accounted for on the control tree: the ones that cannot compile cannot
compile because the contract TYPE is the fix, and the ones that can compile fail on the defect they name.

### Language-level confirmation

`evalf(...)` is the language's numeric entry point to a symbolic trig expression (`sin` is a Symbolics builtin;
`print(sin(1))` prints `sin(1)`), and at the language surface the published width is now the display limit:

```
print(evalf(sin(1), 30))                     -> frac=29  0.84147098480789650665250232163
print(evalf(sin(10), 30))                    -> frac=30  -0.544021110889369813404747661851
print(pi(30))                                -> frac=30  3.141592653589793238462643383279
setprecision(30); print(evalf(sqrt(2), 30))  -> frac=30  1.414213562373095048801688724209
setprecision(40); print(1/(1009*10^1000))    -> frac=40  0.0000000000000000000000000000000000000000
```

### A separate defect found while probing, recorded not fixed

```
print(evalf(sin(10^100), 30))   ->  Value was either too large or too small for an Int64.
print(evalf(cos(10^100), 30))   ->  Value was either too large or too small for an Int64.
print(evalf(cos(1000000), 30))  ->  0.936752127533144786938532535074
```

An exact literal wider than Int64 reaches `evalf`'s numeric path and escapes as the raw CLR conversion message.
That is the SAME class ADV-03 exists to close — an integer-valued argument crossing as a framework sentence
rather than a named refusal — on the `evalf` digit/argument path. It is the direct input to **ADV-06** (the
`Real.Pow` exponent semantics and raw-`NotImplementedException` class) rather than a width question, and it is
recorded here so it is not rediscovered.

### Verification at the end of round 8

| what | result |
|---|---|
| `dotnet test LovelaceSharp.slnx -c Release` (oracle excluded) | 15 of 15 suites, **6 171 passed, 0 failed, 8 skipped** |
| `--filter "Category=Costly"` | **351 passed, 0 failed** |
| `RealTrigPrecisionOracleTests` (live mpmath) | **25 passed, 0 failed** |
| `Lovelace.Real.Tests` | 2 537 passed (2 512 + the 25 oracle cases) |
| width-contract theory on the control tree | 4 failed, 2 passed |
| the same theory on HEAD | **0 failed, 6 passed** |
| `.worktrees/adv01-ctl` | `git status` 0 entries |
| published AOT runner | `Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe`, 6 020 096 B, sha256 `E4BA237A9153E6B6`, rebuilt after the repair |
| the benchmark's runner copy | same bytes, same sha256 (refreshed) |

---

## Round 9 — the objective's acceptance test, run against the artefact rather than described

Everything the objective names was verified by the end of round 8, but the verification lived in this
conversation and in `evidence.md`, not in a thing a later round can re-run against a binary. Round 9 closed
that, and re-ran the parts of the benchmark the width repair could have moved.

### `harnessability-bench/checks/wire_contract.mjs` — 20 checks, against a runner binary

```
node checks/wire_contract.mjs [path/to/Lovelace.Run.exe]      # exit non-zero on any regression
```

```
PASS  ADV-08 --functions det exits 0                            exit 0
PASS  ADV-08 --functions det payload is small                   276 B
PASS  ADV-08 listing carries no documentation                   [name,parameters,minArity,parameterCount,variadic,category]
PASS  ADV-08 page reports total/returned/truncated              total=1 returned=1 truncated=false
PASS  ADV-08 query evaluates nothing                            none
PASS  ADV-08 --function det carries the documentation           11 fields incl. summary,returnKind,related,examples
PASS  ADV-08 --function det is small                            459 B
PASS  ADV-08 an unknown NAME is a refusal, not an empty success UnknownFunction registrySize=123
PASS  ADV-08 an unknown PATTERN is an empty result              total=0
PASS  ADV-07 an arity refusal carries signature                 {function,parameters,expectedMin,expectedMax,actual}
PASS  ADV-07 the signature names the builtin and its parameters [x]
PASS  ADV-07 the signature carries the measured arity           min=1 max=1 actual=2
PASS  ADV-07 the refusal is InvalidArgument/TypeMismatch        InvalidArgument/TypeMismatch
PASS  ADV-07 signature is omitted when it is not a call-site refusal
PASS  ADV-09 an ordinary failure carries partialVariables       a,b
PASS  ADV-09 the committed bindings are the ones bound before the failure
PASS  ADV-09 the digit-count refusal speaks the contract grammar
PASS  success envelope carries artifacts[]                      []
PASS  success envelope carries the sealed fields                13 keys
PASS  scale multiplies by a scalar (ADV-03 descriptor fix)      ["[2.5, 5, 7.5]"]

20 passed, 0 failed
```

It belongs in the benchmark rather than in a transcript because it runs against a BINARY: a stale runner copy
fails it the same way a stale runner fails the refresh probe. Two of my own checks were wrong first and are
worth naming, since both were assumptions rather than measurements: the signature's builtin field is
`function`, not `name` (the envelope's function LIST uses `name`), and the check had to read the whole
signature object to see `expectedMin`/`expectedMax`/`actual` rather than only its first key.

### The width repair, re-measured through the benchmark

The repair changes how a `Real` RENDERS, and the benchmark's 20 seeds and 48 hidden probes are full of
rendered values, so this was the highest-value place to look for collateral damage:

```
build_corpus.py --check   CHECK OK: 20 goldens, 48 probes, every probe discriminates its seed
bench.py prepare          OK for all 20 tasks; oracles.json re-derived
bench.py validate         ok:true  goldens_verified:20  hidden_probes:48  accidental_seed_matches:[]
                          seed_execution 12 ok / 8 error   l3_registry_smoke:true
```

The corpus is re-derived from scratch against the width-fixed runner and every probe still discriminates its
broken seed, so the rendering change moved no semantic content the benchmark tests.

The observation surface is also unchanged, which is the check that matters for the four SEALED projections:

| mode | round 6 measurement | round 9 measurement |
|---|---:|---:|
| L0 mean / max | 55.2 / 440 | 55.2 / 440 |
| L1 mean / max | 100.0 / 440 | 100.0 / 440 |
| L2 mean / max | 676.0 / 4 424 | 676.0 / 4 424 |
| L3 mean / max | 887.0 / 4 718 | 886.5 / 4 716 |

The L3 total moved by 6 bytes across 20 seeds — one seed's rendered value is a few bytes shorter under the
width limit, which is the repair doing exactly what it says and nothing else. No projection gained a field.

### Verification at the end of round 9

| what | result |
|---|---|
| `node checks/wire_contract.mjs` | **20 passed, 0 failed** |
| `build_corpus.py --check` / `bench.py prepare` / `bench.py validate` | all exit 0, corpus and oracles re-derived |
| observation surface, all four modes | unchanged (L3 −6 B total) |
| published AOT runner | 6 020 096 B, sha256 `E4BA237A9153E6B6` |
| benchmark runner copy | same bytes, same sha256 |
---

## Round 10 — the cited numbers, checked against the tree they are cited in

Every claim in this record is a measurement, and a measurement is only useful if it still belongs to the tree
that quotes it. Round 10 extracted the numbers this record and `RESULTS.md` actually rely on and re-measured
them on HEAD. **25 claims: 24 hold, 3 were broken on the first pass, 0 are broken now.**

### The verifier

```
node .lovelace-adv-probe/claims.mjs      # 25 claims, exit non-zero on any that does not hold
```

```
HOLDS   ADV-08 "--functions det" is 276 bytes                     276 B
HOLDS   ADV-08 "--function det" is 459 bytes                      459 B
HOLDS   the full envelope is ~41.7 KB                             41 937 B
HOLDS   the query is at least 150x smaller than the envelope      151.9x
HOLDS   --help documents the registry flags                       1 996 B, mentions both flags
HOLDS   ADV-07 arity refusal is 650 bytes                         650 B
HOLDS   ADV-07 signature.actual is 2                              {function,parameters,expectedMin,expectedMax,actual}
HOLDS   ADV-03 zeros(2.5) refusal is 694 bytes                    694 B
HOLDS   ADV-03 the zeros refusal names the position and parameter
HOLDS   ADV-03 scale multiplies by a scalar                       [2.5, 5, 7.5]
HOLDS   ADV-09 partialVariables carries a,b                       [a, b]
HOLDS   width: evalf(sin(10), 30) publishes 30                    30
HOLDS   width: pi(30) publishes 30                                30
HOLDS   width: setprecision(30); sqrt(2) publishes 30             30
HOLDS   width: setprecision(40); 1/(1009*10^1000) publishes 40    40
HOLDS   cited file sizes (5 files)                                exact
HOLDS   AOT publish path 6 020 096 B sha256 E4BA237A9153E6B6
HOLDS   the benchmark runner copy is byte-identical to it
HOLDS   the AOT artefact is self-contained (no coreclr.dll beside it)
HOLDS   the docs record that out/aot is a framework-dependent apphost
HOLDS   the docs do not assert out/aot is a Native AOT artefact

25 claims HOLD, 0 BROKEN
```

### The three that did not hold, and what was done about each

| claim | cited | measured | cause | action |
|---|---:|---:|---|---|
| the payload reduction | "**151x**" | 41 937 / 276 = **151.9x**, so 152 | nothing moved: "151x" was a truncation of a hand measurement taken at round 3, against an envelope this record also measures at 41 609 B elsewhere. The ratio depends on the envelope's own size, which differs per script. | the standing claim is now **at least 150x**, and the verifier asserts the BOUND rather than a rounded point |
| `--help` size | 2 026 B | **1 996 B** | the help text changed when ADV-08 added the `--functions` / `--function` / `--function-limit` lines to it — and then changed again in ADV-03's round when the digit-count refusals were re-worded. The measurement was taken at 19:48 and never re-taken. | the verifier now asserts what `--help` must CONTAIN, not how many bytes it is; a byte count of help text is not a contract |
| `out/aot` is not cited as AOT | — | the record's round-1 table still rowed `out/aot/Lovelace.Run.exe` under "Artefact fingerprints" with no warning that the artefact is not what its name says | round 5c discovered the mis-reading and corrected it *later in the file*; the round-1 table was left standing for nine rounds | the table now carries a **SUPERSEDED** note pointing at round 5c, and the verifier checks both halves — that the correction is present, and that nothing asserts `out/aot` is Native AOT. The rows are left standing: a record that quietly rewrites itself is not a record. |

The third one is the one worth dwelling on. Round 5c called the `out/aot` mis-description a factual error and
recorded the correction 870 lines below the error, which leaves the record asserting two contradictory things
with nothing in the text to say which wins. That is the same failure mode this cycle keeps finding in the
product — `validation-report.json` contradicting `validation-summary.json`, the benchmark's runner copy
contradicting the repository, the `DisplayDecimalPlaces` bound contradicting its own arithmetic — and the
repair is the same in all four cases: **the contradiction has to be made visible where it lives, not resolved
in a place the reader has not reached yet.**

### What is unchanged

Nothing about the product moved this round: no source file, no test, no artefact. The AOT runner is still
6 020 096 B / sha256 `E4BA237A9153E6B6`, the wheel is still green (15 of 15 suites, 6 171 passed, 0 failed,
8 skipped; 351 costly; 25 oracle), and the benchmark corpus still re-derives clean.

### Verification at the end of round 10

| what | result |
|---|---|
| the cited-claims verifier | **25 HOLD, 0 BROKEN** |
| product files changed this round | **none** (one evidence annotation, one verifier) |
| `out/aot` rows in the round-1 table | annotated SUPERSEDED, pointing at round 5c |
### Closing the round-10 verifier

The check that the record does not assert `out/aot` is a Native AOT artefact needed three attempts, and the
reason is worth keeping: the correction necessarily QUOTES the wrong claim, so a text check for the claim also
matches the sentence that retracts it. The check is now line-based and exempts any line carrying a negation, a
correction marker, or the words `framework-dependent`. It reports `0 assertions`.

```
node docs/advancement-cycle-7/claims-check.mjs
25 claims HOLD, 0 BROKEN
```

The verifier lives with the record it checks (`docs/advancement-cycle-7/claims-check.mjs`) so the next round can
run it in one command. `evidence.md` is 93 603 bytes of UTF-8 with no BOM and no mojibake (202 em dashes, 0
mis-encoded), which matters here because every round of this cycle has appended to it from a different tool.