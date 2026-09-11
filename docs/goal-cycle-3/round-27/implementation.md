# Round 27 - Registry-driven, widened falsification gate

STATUS: DONE (no git commit; production code untouched)

## 1. What changed (scope)

| File | Change |
| --- | --- |
| `Lovelace.Symbolics.Tests/FalsificationGateTests.cs` (new) | Region data with per-point rationale, registry-driven gate, 5 new facts |
| `Lovelace.Symbolics.Tests/FalsificationTests.cs` | The 13 points are now `internal static readonly Rat[] LegacyPoints` (same values, same order) and `AllPoints` is `FalsificationRegions.AllRealPoints` - a strict superset. All 9 existing `[Fact]`s and `FuzzIdentity` are unchanged. |

NO production file was edited. The registry accessors already existed
(`Simplify.RulesForTesting` / `Simplify.ShippedRuleIds`, Simplify.cs:199/205), so nothing had to
be added to the kernel. No rule id is listed anywhere in the sweep.

## 2. Registry-driven sampling (required 1)

`RegistryFalsificationGate.Run(rules, ctx, x)` iterates `Simplify.RulesForTesting(ctx)`. For each
rule it:
1. instantiates the rule's own `Pattern` (wildcards -> the probe symbol `x`) via `Exprs`
   constructors - no per-rule expression is written by hand;
2. binds that match into a `Match` and calls the rule's **own** `Applicability`,
   `ConditionBuilder`/`DeclaredConditions` and **own** `Replacement` - the gate exercises the
   shipped rule code, not a copy of it;
3. sweeps every real probe of every region plus the complex sweep, comparing
   `|lhs - rhs| < 1e-11` (the tolerance the old gate already used).

Observed counts (test output, `--logger "console;verbosity=detailed"`):

```
REGISTRY COUNT (RulesForTesting / ShippedRuleIds / sampled) = 9 / 9 / 9
```

Asserted: `RulesForTesting.Count == ShippedRuleIds.Count == report.Sampled.Count == distinct`,
`Uninstantiable == Unlicensed == Degenerate == Threw == Falsified == empty`, plus non-vacuity -
every registered rule must have at least one point genuinely compared (`compared > 0`), and every
declared region must have compared at least one point. A rule added tomorrow is swept
automatically; if it cannot be instantiated/licensed/compared, the corresponding assertion fires
instead of the rule escaping silently.

Observed per-rule coverage (compared point count and regions that contributed):

| rule | compared | regions |
| --- | --- | --- |
| trig.pythagorean-sin2-cos2 | 78 | all 6 |
| pow.sqrt-square-nonnegative | 28 | legacy, near-pole, branch-cut, discontinuity, assumption-boundary |
| pow.sqrt-square-real | 31 | legacy, near-pole, branch-cut, discontinuity, assumption-boundary |
| rat.cancel-x-over-x | 74 | all 6 |
| rat.cancel-zero-over-x | 74 | all 6 |
| logexp.exp-log | 40 | all 6 |
| logexp.log-exp | 32 | legacy, near-pole, branch-cut, discontinuity, assumption-boundary |
| abs.abs-square | 33 | legacy, near-pole, branch-cut, discontinuity, assumption-boundary |
| abs.abs-neg | 78 | all 6 |

## 3. Sampling regions (required 2)

Real regions (45 points total = 13 legacy + 32 new; complex adds 12). `AllPoints` in
FalsificationTests.cs is now the union, so the 9 legacy facts sweep the new points too.

| region | points | why the point is near the feature |
| --- | --- | --- |
| legacy-interior (13, unchanged) | -100, -5, -3/2, -1, -1/3, -1/1000, 0, 1/1000, 1/3, 1, 3/2, 5, 100 | the original magnitudes around the singular/ordering boundaries |
| near-pole (4) | +-1/10^6, +-1/10^9 (exact rationals) | the registry's only poles are at x = 0 (`x.x^-1`, `0.x^-1` are undefined there; log is singular there). epsilon = 1e-6 and 1e-9, exact rationals, so no approximation is introduced. |
| near-branch-cut (7) | -1/10^6, +1/10^6, -1/10^9, +1/10^9 (straddling pairs), -1/4, -4, -25 (exact) | the branch point of log/sqrt is 0 with the cut along the negative real axis; pairs of equal magnitude approach it from both sides, and -1/4, -4, -25 sit on the cut where the kernel's exact-root path binds (sqrt(1/4)=1/2, sqrt(4)=2, sqrt(25)=5 - exact, not rounded). |
| near-discontinuity (4) | -1/999, +1/999, -1/1000003, +1/1000003 | either side of the jump at 0 (derivative of `abs`; `x.x^-1` is undefined exactly at 0). Odd/prime denominators put them at scales distinct from the pole/branch epsilons, and force the rounded real path: they are exact rationals, but sin/cos/exp/log convert them at the kernel's 40-digit working precision (stated epsilon for those conversions: 1e-11 comparison tolerance). |
| assumption-boundary (5) | -1/10^6, -1/10^9 (outside), 0 (exactly on), +1/10^6, +1/10^9 (inside) | the gate itself sets x >= 0 (`SymbolPredicate.NonNegative`) to license `pow.sqrt-square-nonnegative`; 0 is the boundary, the pairs sit on both sides of it, and the per-point guard test asserts the guard flips exactly there. |
| complex (12) | 1+i, 1-i, -1+i, -1-i, i, -i, 0.5+0.5i, -0.5+1.5i, 1.5-0.5i, 2+0.5i, -2-0.5i, 2+0i | a genuine sweep, not one token point: 4 quadrants, both axes, small and moderate moduli. Every part is an exact terminating decimal, so `Rl.Parse` introduces no point-level epsilon; all |Im z| <= 2 < pi, so a rule licensed only on log's principal strip cannot be falsely falsified. |

Where an approximation is unavoidable it is stated: exact rationals everywhere for the real
regions; the only approximations come from the kernel's own transcendental conversion at 40-digit
precision, compared with the pre-existing 1e-11 tolerance.

## 4. Strictness kept: the gate still fails when a rule throws (required 3)

Both mechanisms are in force:

* the pre-existing `FuzzIdentity` catch is untouched (only `EvaluationException` licenses a skip;
  any other exception is collected and asserted empty);
* the new registry gate has the same contract at two levels - rule code (`Applicability`,
  `ConditionBuilder`, `Replacement`) and point evaluation - and reports a non-empty `Threw` list,
  which the positive test asserts empty.

Proven by a **negative control that supplies a deliberately throwing rule**
(`Gate_Fails_WhenARuleThrows`): the fake rule's `Replacement` throws
`InvalidOperationException`. Observed:

```
THROWING-RULE CONTROL: Failed=True threw=2
  test.control-throwing-rule replacement: InvalidOperationException: deliberate negative control: replacement throws
  test.control-throwing-rule replacement: InvalidOperationException: deliberate negative control: replacement throws
```

A second control (`Gate_ReportsFalsification_WhenARuleIsWrong`) supplies an unconditionally wrong
rule (`sqrt(x^2) -> x`) and the gate reports a value falsification - so the gate can fail, it is
not a test that cannot fail:

```
WRONG-RULE CONTROL: Failed=True falsified=39
  first falsification: test.control-wrong-rule at legacy-interior:-100: sqrt(x^2) != x
```

## 5. No existing sample point or assertion weakened (required 4)

Machine-checked by `Sampling_IsASuperset_OfTheOriginalPoints_AndEveryProbeIsDocumented`: every
value in `FalsificationTests.LegacyPoints` must appear in `FalsificationRegions.AllRealPoints`.
All 9 original `[Fact]`s still exist and are unmodified apart from the `AllPoints` source. No test
was deleted or skipped (the 6 skipped tests are the pre-existing `SympyOracle_*` oracle tests, none
of them mine).

## 6. Test-first

There was no production change to make first: the registry accessors already existed, so the new
tests were written against the shipped kernel and **passed immediately** (capability already
present). No failure was manufactured. Two failures during authoring were my own over-strict
assertions, corrected rather than relaxed to hide a defect (they are recorded here for honesty):
(i) the near-pole bound was written exclusive while 1e-6 is exactly on the epsilon - made
inclusive; (ii) the complex sweep had no real-axis point, so the "both axes" assertion failed -
a point `2+0i` was ADDED (sampling widened, nothing relaxed).

## 7. Verification (observed output)

```
> dotnet build LovelaceSharp.slnx -c Release --nologo
    4 Warning(s)
    0 Error(s)
Time Elapsed 00:00:02.74

> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   608, Skipped:     6, Total:   614, Duration: 15 s - Lovelace.Symbolics.Tests.dll (net10.0)
WALL_SECONDS=20.8

> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   637, Skipped:     0, Total:   637, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
WALL_SECONDS_SUITE=13.6
```

Durations: Symbolics was ~13 s before this change per the brief; it is now 15 s of test time
(20.8 s wall) - well under the ~60 s ceiling, so the point counts were NOT reduced. Region test
count: +5 facts (2 region/coverage, 1 registry sweep, 2 negative controls); total tests 609 -> 614.

## 8. What could not be covered

* `logexp.log-exp` is licensed only under `Real(x)` (its interval condition `Im(z) in (-pi, pi)`
  is proved from realness), so its sweep is real-only; the complex sweep covers 5 of 9 rules
  (trig, cancel x2, exp-log, abs-neg). Rules that are legitimately real-only
  (`pow.sqrt-square-*`, `abs.abs-square`) are excluded from complex points by their own guard -
  correctly, since `|z|^2 != z^2` over C.
* The gate's numeric assumption guard implements `SymbolDomainAssumption`,
  `SymbolPropertyAssumption`, `ExpressionPropertyAssumption` and `IntervalAssumption`; any other
  atom kind (e.g. `SymbolRelationAssumption`) is recorded in `UnhandledAtoms` and would make the
  per-rule `compared > 0` assertion fail loudly rather than silently skip. No shipped rule uses an
  unhandled atom today.
* `SymbolPredicate.Even/Odd` are not decidable at these sample points; no shipped rule uses them.
* The sweep is falsification, not proof: it is evidence over the sampled regions, not a proof over
  the domain.
* Only `Simplify`'s registry is enumerated (the kernel's single rule registry); rules registered
  in a test-local `RewriteRuleRegistry` elsewhere are out of scope by construction.
