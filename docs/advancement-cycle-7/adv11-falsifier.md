# ADV-11 — falsifier substrate: implementation record

Worktree: `.worktrees/adv11-falsifier` at `ce30fc3` (detached, clean at the start).
Scope: **test-side only**. No product project was touched — `git diff --stat -- ':!Lovelace.Symbolics.Tests'` is empty.

## 1 · What was built, file by file

### `Lovelace.Symbolics.Tests/Falsification/ObservationClass.cs`

- `ObservationClass` — exactly five classes: `Agree`, `Disagree`, `UndefinedLeft`, `UndefinedRight`,
  `BudgetOrRefusal`.
- `SideOutcome` — `Evaluated | Undefined | Refused | Threw`. `Threw` is deliberately NOT a class: an
  unexpected exception is a defect and is reported as itself. `ClassifiedObservation.IsDefect` exposes it.
- `ComparisonTolerance` — the tolerance is a **parameter** (`Value` + the `Text` it printed from), never a
  constant inside the classifier. `TenToTheMinusEleven()` reproduces the gate's historical 1e-11;
  `OfRational(Rat)` gives an exact one.
- `ObservationClassifier.Classify` — evaluates both sides under one scope, classifies each side
  independently, then folds them with a stated precedence: **a typed resource refusal outranks
  everything; then definedness (left before right); then the comparison**. `Undefined` comes from the
  kernel's typed `EvaluationException` on that side ONLY. `Refused` is recognised **by type** from
  `Lovelace.Symbolics.BudgetExceededException`, `Lovelace.Abstractions.InputDepthExceededException`,
  `Lovelace.Suite.ArrayAllocationRefusedException`, `Lovelace.Suite.EvaluationCancelledException`.
- `Disagree` is decided on the exact squared magnitude for a complex difference (no `sqrt` in the
  verdict) and on `|delta|` for a real one — the same care the existing gate takes.
- `FalsificationText` — deterministic renderings (rationals as `n/d`, complex as `(re,im)`, environments as
  `{atom; atom}`) and the escaping used by the probe record.

### `Lovelace.Symbolics.Tests/Falsification/FalsificationProbe.cs`

- `FalsificationProbe` — the rule id, region tag, label, exact input, the observation class, **both side
  outcomes**, the LHS/RHS renderings, both values, the delta text, the precision, the tolerance text, the
  environment, the generator version and a required `Why`.
- `ToText()` / `Parse()` — a deterministic 18-field line, **every field escaped**, so a rule id containing
  `|` round-trips (the round-trip test uses exactly that). `IdentityKey` gives a probe its identity.
- `FalsificationReport` — sorted, stable text plus `Fingerprint()` (SHA-256 over that text) and a `Parse`
  for field-level comparison.

### `Lovelace.Symbolics.Tests/Falsification/BoundaryBias.cs`

- `DeterministicRng` — splitmix64, one integer of state, own file, so a run is reproducible in any process
  (`System.Random`'s stream is not a contract).
- `BiasStepLadder` — `{1, 2, 3, 5, 7}`, coprime on purpose: an even step and an odd step cannot share a grid,
  which is the idea `Lovelace.Knowledge/Proposal.cs` `BuildSweeps` uses with h=2 and h=3.
- `BoundaryBias.BroadPhase(low, high)` — points strictly between the two observations, walked down the
  ladder, each step on its own grid offset by a seeded jitter. **Every point is an exact rational**
  (`InteriorPoint` is pure integer arithmetic); a grid finer than the interval yields nothing rather than
  inventing a point.
- `BoundaryBias.NarrowPhase(low, high, lowClass, classify, out offered)` — an exact bisection driven only by
  the caller's classifier. It assumes nothing about smoothness and uses no derivative: it asks, and moves
  the bracket. It refuses to subdivide below the ladder's own fine step.

### `Lovelace.Symbolics.Tests/Falsification/FalsificationLoopTests.cs`

- `ProbePoint`, `FalsificationProbeSet` (the frozen battery first, in its own order, then generated points),
- `RuleShape` (environment + the expressions actually compared), `FalsificationLoop.ClassifyAt` / `.Run`.
- The loop **respects the rule's own declared environment**: `AssumptionHolds`/`AtomHolds` evaluate the
  interval atoms at the probe and skip a point the rule claims nothing about. Without this the substrate
  reported `pow.sqrt-square-nonnegative` as falsified at `x = -1/10` — a false counterexample manufactured
  out of a guard. The existing gate has always refused to do that; the substrate now does too.
- The loop refines **every in-order class change** with `BoundaryBias` and classifies what it offers.
- 13 tests (names below), including a planted defect, the measured hypothesis, and both determinism checks.

## 2 · The planted defect

A rule that claims `w -> w` but is wrong on a band of its input (it replaces with `w + w`). The band is
known to the test and unknown to both search arms; the replacement is a pure function of **the sample's own
value**, so neither arm can contaminate the other.

| Quantity | Value |
|---|---|
| Falsified? | **Yes** — `PLANTED DEFECT: 1 counterexample(s) of 53 samples` |
| Falsifying input | **251/200** |
| Region tag | `hypothesis` (an approach to the planted band `(5/4, 63/50)`) |
| Why that point | `an approach to the planted band (5/4, 63/50)` — it is the band midpoint `Rat.Divide((5/4 + 63/50), 2)` |
| Samples to first counterexample | 1 among the band approaches; 53 samples total in the report |
| Classes observed across the registry | `Agree`, `UndefinedLeft` |

The test asserts the report names **the region and the input**, not just that something failed.

A control pins the generator against a boundary the product documents: `-` over naturals stays `Natural`
while `b <= a` and widens to `Integer` the moment `b > a` (`Lovelace.Knowledge/BEHAVIOR-GRAPH.md` §3).
Measured through the real engine: `5 - 0 … 5 - 5` are Natural, `5 - 6 … 5 - 10` are Integer, the coarse
scan brackets **(5, 6)**, and `NarrowPhase` returns **6** — the first Integer-producing right operand, with
`0` midpoints offered because the bracket is already at the ladder's own resolution.

## 3 · The hypothesis: **FALSIFIED**

**Claim under test.** Boundary-biased search reaches the first counterexample of a planted defect in FEWER
samples than uniform sampling at the same seed and budget.

**Experiment.** The off-grid band is `(161/128, 331/256)` ≈ `(1.2578, 1.2930)`. Both arms classify the
identical identity over the identical exact-rational interval `[-2, 3]` with an identical budget of **30**
classifier calls. The uniform arm scans a fixed exact grid, step `1/12`, 60 points, in order. The biased arm
runs a deterministic coarse ladder (steps 4, 8, 16 at a seeded jitter), keeps the tightest agreeing bracket,
then asks `BoundaryBias` for interior points of that bracket.

| seed | biased | uniform | biased gap to band | uniform gap to band |
|---:|---|---|---|---|
| 1 | not found | not found | **1/128** | 323/384 |
| 2 | not found | not found | **71/896** | 323/384 |
| 3 | not found | not found | **1/128** | 323/384 |
| 5 | not found | not found | **9/1280** | 323/384 |
| 8 | not found | not found | **1/128** | 323/384 |
| 13 | not found | not found | **1/128** | 323/384 |
| **tally** | `biasedWins=0` | `uniformWins=0` | `ties=0` | `bothMissed=6` |

**Verdict: FALSIFIED.** Neither arm reached the counterexample inside the budget, so the bias bought no
discovery advantage at all. The biased arm came **closer** on every seed (gap 1/128 ≈ 0.0078 versus the
uniform grid's 323/384 ≈ 0.84), which is the honest and interesting secondary result: the bias **localizes**
better than it **discovers**, and a bisection can still step over a band narrower than the gaps it happens to
land on.

Why this is the expected shape of the result, and what it implies:

1. On a **1-D interval** with a **positive-measure** counterexample region, a uniform grid of comparable
   sample density is not systematically worse — it is a covering argument, not a search-strategy argument.
   What a bisection buys is *precision about where the change is*, not *whether it is found*.
2. The bias can only be **decisive** where the region is thin relative to the whole space (or where the
   dimension is high enough that a grid cannot cover), which is exactly where the brief's boundary-
   exploration story is aimed — not where a 1-D scan is aimed.
3. The measurement is now a **permanent regression**: the test asserts the falsified verdict
   (`biasedWins == 0`, `uniformWins == 0`, `misses == 6`) and carries the message that the pinned verdict
   must be replaced with evidence the day the bias starts winning. It cannot be tuned away silently.

**This is reported as a falsification, not tuned until it passes.** No parameter was adjusted after seeing
these numbers.

## 4 · Determinism evidence

| Check | Result |
|---|---|
| Same seed, same process, twice | identical report text and identical SHA-256: `0540C8414258F6164DC8F4CFFFAA2F09344A56DA58E089503274994D9C2F05C7` (53 samples) |
| A different seed | **different** fingerprint (the generator's sequence moved, as it must) |
| Two fresh processes | identical line: `FALSIFIER-FINGERPRINT=33391BB33998A31451862743A545525FAB05D4DF512D6A86A83166D6290E1ACF` |
| Broad-phase sequence, same seed | identical element-for-element; a different seed gives a different sequence |

The cross-process check starts the SDK's `vstest.console.dll` on the built test assembly with
`--TestCaseFilter:FullyQualifiedName~FalsificationFingerprintChild` and requires both children to print the
same fingerprint line. The child writes it with `Console.WriteLine`, which VSTest captures and the console
logger forwards on stdout.

## 5 · Test counts

| | Passed | Failed | Skipped | Total |
|---|---:|---:|---:|---:|
| Baseline `Lovelace.Symbolics.Tests` (HEAD) | 1185 | 0 | 8 | 1193 |
| After ADV-11 | **1198** | **0** | **8** | **1206** |

13 new tests, all passing, no existing test changed or removed:

```text
FrozenBattery_IsAPrefixOfWhatTheSubstrateOffers_InItsOwnOrder
Classifier_NamesEachClass_AndNeverFoldsADefectIntoOne
Tolerance_IsAParameter_TheSameValuesDecideDifferentlyUnderDifferentTolerances
ProbeRecord_RoundTripsThroughItsDeterministicTextForm
Loop_IsDeterministic_SameSeedProducesAByteIdenticalReport
Loop_IsDeterministic_AcrossProcesses
FalsificationFingerprintChild
Generator_BracketsAKnownBoundary_TheNaturalSubtractionWidening
Generator_BracketsANonIntegralBoundary_AndEveryPointIsStrictlyInside
Generator_BroadPhaseSequenceIsStableUnderTheSameSeed_AndMovesWithIt
PlantedDefect_IsFalsified_AndTheReportNamesTheRegionAndTheInput
RegistryLoop_ClassifiesEveryShippedRule_AndFindsNoDefect
Hypothesis_BoundaryBiasBeatsUniform_OnAPlantedDefect
```

The Falsification subset on its own: `Total tests: 32, Passed: 32, Failed: 0`.
Full suite command: `dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release`.

### Frozen battery, measured

```text
FROZEN BATTERY: probes=33 generated=27 total=60 distinctFrozenPairs=33 repeatedFrozenPairs=[]
```

33 = 13 legacy + 4 near-pole + 7 near-branch-cut + 4 near-discontinuity + 5 assumption-boundary. The 27
generated points are the lagged `[-1,1]` grids de-duplicated by VALUE (the coprime ladder's grids overlap on
purpose). The legacy 13 remain a **subset, in order**, of the frozen set, and the frozen set is a **prefix**
of what the substrate offers — both asserted element-for-element, including each probe's `Why`.

## 6 · Implementation findings worth passing on

1. **A falsifier must respect the rule's declared environment.** The first version compared every probe under
   a rule's identity regardless of whether its side conditions held, and reported the shipped rule
   `pow.sqrt-square-nonnegative` (`sqrt(x^2) -> x` for `x >= 0`) as FALSIFIED at `x = -1/10`. That is a
   counterexample manufactured out of a guard. `FalsificationLoop.AssumptionHolds` now skips any point where
   an atom of the rule's own environment fails, and treats an atom it cannot decide as NOT holding.
2. **A reference identity for a planted defect must be a pure function of the sample's value.** Building the
   shape under `ctx.WithAssumptions(band)` made the ambient assumption visible to the replacement's own
   guard, so the defect fired at every point instead of only inside the band. The substrate now builds the
   replacement for the point being classified.
3. **`Instantiate` had to be re-implemented in the test project**: the gate's copy is `private`. That is
   duplication of ~50 lines between `FalsificationGateTests` and the substrate, and the natural fix is to
   make the gate's copy `internal`. No product change was needed, so none was made.
4. **An unregistered name is a typed domain refusal, not a defect.** `Evaluation` reports
   `ctx.Function("no_such_function_at_all")` as an `EvaluationException`, so the classifier correctly names it
   `UndefinedLeft`. The defect path is exercised by the record shape (`SideOutcome.Threw`), because the
   kernel's expression set has no stably-throwing expression — recorded rather than faked.

## 7 · Could not verify

- **`UndefinedRight` and `BudgetOrRefusal` are not reached by the current probe set.** The registry loop
  observes `Agree` and `UndefinedLeft` only; the frozen battery is curated so every shipped rule is defined on
  it. Both classes are unit-tested on constructed shapes, but neither is exercised end-to-end by a real
  sample in this substrate yet.
- **`BoundaryBias.NarrowPhase`'s complex case.** The generator is real-rational only; the complex probe region
  exists in the frozen battery but no complex bracket descent was built or measured.
- **Only one assumption kind is decided.** `AssumptionHolds` understands `IntervalAssumption` atoms whose
  subject is the probe symbol. `SymbolPropertyAssumption`/`SymbolDomainAssumption`/
  `ExpressionPropertyAssumption` all currently count as NOT holding, which is conservative but means rules
  licensed only by those atoms are under-sampled (their `samples=` counts are 0 in the registry loop).
- **The cross-process check depends on an SDK layout.** It locates `vstest.console.dll` under `DOTNET_ROOT`
  or `%ProgramFiles%\dotnet`. It is verified on this machine only; a CI image without an SDK at either root
  would make it fail loudly rather than silently skip — which is the intended failure mode, but it is
  unverified elsewhere.
- **No AOT relevance**: test code only, no package references were added.

## 8 · Blocker report

None. No product change was required and none was made. `git status --short` in the worktree shows only the
four new files, the deliverable, and two logs.
