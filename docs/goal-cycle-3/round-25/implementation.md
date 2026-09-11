# Round 25 — P2 item 9: `AssumptionSet.Add` is quadratic in the number of atoms

STATUS: DONE. Files changed: `Lovelace.Symbolics/Assumptions.cs` (only production file touched),
`Lovelace.Symbolics.Tests/AssumptionAddScalingTests.cs` (new). No git commit.
Path taken: **STEP 2 (fix)**, justified by the STEP 1 measurement below.

## 1. STEP 1 — measurement FIRST, on the UNMODIFIED source

Method (all of it is in `AssumptionAddScalingTests`, so the measurement and the regression pin are
the same code):

* one atom at a time through the public `AssumptionSet.Add`, starting from `AssumptionSet.Empty`;
* a **fresh `ExprContext` per repetition**; the symbols and the atom array are built *before* the
  stopwatch starts, so only the `Add` loop is timed;
* 3 repetitions per size, the **minimum** reported;
* two workloads: **domain atoms** (one `SymbolDomainAssumption(sym_i, Real)` per distinct symbol) and
  **relation atoms** (one `SymbolRelationAssumption(sym_i, Gt, i+1)` per distinct symbol — this also
  drives the contradiction probe inside `Add`);
* exponent = least-squares slope of `ln(ms)` against `ln(n)` over the four sizes.

Command:

```powershell
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --no-build `
  --filter "FullyQualifiedName~AssumptionAddScalingTests" --logger "console;verbosity=detailed"
```

Observed **BEFORE** (the test asserted an `n^1.5` bound and FAILED — that failure is the
falsification of "already acceptable"):

| workload | atoms | elapsed ms (min of 3) | stored atoms |
|---|---|---|---|
| domain atoms | 100 | 5.19 | 100 |
| domain atoms | 200 | 32.85 | 200 |
| domain atoms | 400 | 156.13 | 400 |
| domain atoms | 800 | 424.00 | 800 |
| relation atoms | 100 | 5.62 | 100 |
| relation atoms | 200 | 35.69 | 200 |
| relation atoms | 400 | 116.04 | 400 |
| relation atoms | 800 | 512.35 | 800 |

```
Failed Lovelace.Symbolics.Tests.AssumptionAddScalingTests.Add_Scaling_At100To800Atoms_IsNotQuadratic [4 s]
fitted exponent (domain atoms) = 2.130
fitted exponent (relation atoms) = 2.123
```

Doubling ratios (domain): 100→200 ×6.3, 200→400 ×4.8, 400→800 ×2.7 — super-linear, exponent ≈ 2.1.
Cycle-2's 8.18 / 26.91 / 91.13 ms at 100/200/400 is reproduced in shape and magnitude
(here 5.19 / 32.85 / 156.13 ms on this machine/harness).

Root cause, `Assumptions.cs:98` before the fix:

```csharp
var next = _atoms.Add(a).Sort(static (x, y) => string.CompareOrdinal(x.ToString(), y.ToString()));
```

a **whole-array re-sort on every non-duplicate insertion**, whose comparator calls `ToString()` on
both operands for every comparison — the ordering key is re-rendered O(n log n) times per insertion
and O(n² log n) times over a one-at-a-time build.

## 2. STEP 2 — the minimal fix (`Lovelace.Symbolics/Assumptions.cs`)

1. `private readonly ImmutableArray<string> _keys` — ordinal ordering keys **parallel to `_atoms`**
   (same length, same order), written when an atom enters the set. An atom's rendering is produced
   once and never again for that set's lifetime.
2. `Add` renders `key = a.ToString()` once, binary-searches it (`LowerBound`) and inserts at that
   position (`_atoms.Insert`, `_keys.Insert`). No sort, no re-render. The contradiction probe
   (`Negate`/`Ask`, then `IntegerIntervalIsEmpty`) runs on the resulting set exactly as before.
3. **Order safety net.** The old sort is *unstable*, so with two atoms that render identically the old
   tie arrangement — not the ordinal order — is the observable order. `Add` therefore falls back to
   the historical append-and-re-sort (`AppendAndSort`, the pre-fix line kept verbatim) whenever the
   incoming key already exists, and a set that has ever held tied keys (`_hasTiedKeys`, set by
   `FromAtoms` too) keeps taking that path forever. Consequently, for every set that can exist:
   * pairwise distinct keys → the ordinal order is unique → new order ≡ old order exactly;
   * tied keys → the original code path runs, so the order is bit-for-bit the old one.

`FromAtoms` keeps its own sort (unchanged behaviour) and only gains the one-time key array. Nothing
else moved: same atoms, same queries, same payload projection, same contradiction rules.

## 3. AFTER — same sizes, same method, same command

| workload | atoms | elapsed ms (min of 3) | before (ms) | speed-up |
|---|---|---|---|---|
| domain atoms | 100 | 0.18 | 5.19 | 29× |
| domain atoms | 200 | 0.67 | 32.85 | 49× |
| domain atoms | 400 | 2.09 | 156.13 | 75× |
| domain atoms | 800 | 7.93 | 424.00 | 53× |
| relation atoms | 100 | 0.72 | 5.62 | 8× |
| relation atoms | 200 | 2.03 | 35.69 | 18× |
| relation atoms | 400 | 7.99 | 116.04 | 15× |
| relation atoms | 800 | 33.58 | 512.35 | 15× |

```
| workload | atoms | elapsed ms (min of 3) | stored atoms |
|---|---|---|---|
| domain atoms | 100 | 0.18 | 100 |
| domain atoms | 200 | 0.67 | 200 |
| domain atoms | 400 | 2.09 | 400 |
| domain atoms | 800 | 7.93 | 800 |
| relation atoms | 100 | 0.72 | 100 |
| relation atoms | 200 | 2.03 | 200 |
| relation atoms | 400 | 7.99 | 400 |
| relation atoms | 800 | 33.58 | 800 |
fitted exponent (domain atoms) = 1.799
fitted exponent (relation atoms) = 1.864
```

Honest reading: the re-render/re-sort defect is gone (800 atoms 424.00 → 7.93 ms and
512.35 → 33.58 ms), but the fitted exponent only fell 2.13 → 1.80, because two other O(n)
per-insertion costs now dominate: the `_atoms.Contains(a)` duplicate scan and the
`ImmutableArray.Insert` copy (~320k pointer copies over an 800-atom build), plus the
`Ask`/`RelationsOn` scans on the relation workload. Those are inherent to the immutable-array
representation and were not the reported defect; removing them would be a storage redesign, outside
"one bounded change". At realistic sizes the cost is now single-digit milliseconds for 800 atoms.
(The after-table was measured on the final production binary; only test files changed afterwards.)

## 4. Matrices — present and passing, by name

Both live in `Lovelace.Symbolics.Tests/AssumptionSoundnessTests.cs`; both passed in the post-fix
run (observed output below). Neither was modified, and both enumerate their own combinations in
place: `Ops = {Eq, Ne, Gt, Ge, Lt, Le}` (6) × `Bounds = {4, 5, 6}` (3) × `Ops` (6) × `Bounds` (3).

| test | meaning | count |
|---|---|---|
| `AssumptionSoundnessTests.BoundReasoning_MatchesReferenceTruthTable` | every assumed relation × every queried relation checked against the reference truth table over the 9 witness rationals | 6·3·6·3 = **324 queries** |
| `AssumptionSoundnessTests.ContradictionDetection_IsNeitherMissedNorInvented` | every atom pair accepted exactly when the reference model is non-empty | 6·3·6·3 = **324 pairs** |

Observed (detail logger, filter `FullyQualifiedName~AssumptionSoundnessTests`):

```
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.ContradictionDetection_IsNeitherMissedNorInvented [6 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.BoundReasoning_MatchesReferenceTruthTable [2 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.RealValuedBounds_ParticipateInReasoning [16 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.PropertyAndRelationAtoms_Contradict [2 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.UnsatisfiableSet_AnswersUnsatisfiable_NotUnknown [< 1 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.DomainAtoms_MergeToTheNarrowest [2 ms]
  Passed Lovelace.Symbolics.Tests.AssumptionSoundnessTests.IntervalBounds_WithUnevaluableBound_AreUnknown [30 ms]
```

## 5. Build and full suite (all 15 projects, final code)

```powershell
dotnet build LovelaceSharp.slnx -c Release --nologo
# -> Build succeeded.  5 Warning(s)  0 Error(s)   (none of the warnings are in the touched files)
```

Then each project, `dotnet test <project>/<project>.csproj -c Release --no-build --nologo`
(`Lovelace.Real.Tests` additionally with `--filter "Category!=Heavy"`):

| # | project | observed result |
|---|---|---|
| 1 | Lovelace.Abstractions.Tests | Passed! - Failed: 0, Passed: 20, Skipped: 0, Total: 20 |
| 2 | Lovelace.Array.Tests | Passed! - Failed: 0, Passed: 19, Skipped: 0, Total: 19 |
| 3 | Lovelace.Complex.Tests | Passed! - Failed: 0, Passed: 83, Skipped: 0, Total: 83 |
| 4 | Lovelace.Console.Tests | Passed! - Failed: 0, Passed: 15, Skipped: 0, Total: 15 |
| 5 | Lovelace.Dsp.Tests | Passed! - Failed: 0, Passed: 61, Skipped: 0, Total: 61 |
| 6 | Lovelace.Integer.Tests | Passed! - Failed: 0, Passed: 148, Skipped: 0, Total: 148 |
| 7 | Lovelace.Knowledge.Tests | Passed! - Failed: 0, Passed: 28, Skipped: 0, Total: 28 |
| 8 | Lovelace.Natural.Tests | Passed! - Failed: 0, Passed: 195, Skipped: 0, Total: 195 |
| 9 | Lovelace.Representation.Tests | Passed! - Failed: 0, Passed: 91, Skipped: 0, Total: 91 |
| 10 | Lovelace.Studio.Tests | Passed! - Failed: 0, Passed: 22, Skipped: 0, Total: 22 |
| 11 | Lovelace.Suite.Tests | Passed! - Failed: 0, Passed: 637, Skipped: 0, Total: 637 |
| 12 | Lovelace.Symbolics.Tests | Passed! - Failed: 0, Passed: 596, Skipped: 6, Total: 602 |
| 13 | Lovelace.Run.Tests | Passed! - Failed: 0, Passed: 39, Skipped: 0, Total: 39 |
| 14 | precbench.Tests | Passed! - Failed: 0, Passed: 13, Skipped: 0, Total: 13 |
| 15 | Lovelace.Real.Tests (`Category!=Heavy`) | Passed! - Failed: 0, Passed: 272, Skipped: 0, Total: 272 |

Totals: 2245 tests, **0 failed**, 6 skipped (all pre-existing, none in the new file), every project
exit code 0. Symbolics went 598 → 602 total with this round's 4 new tests (596 passed + 6 skipped).

## 6. New tests (`Lovelace.Symbolics.Tests/AssumptionAddScalingTests.cs`)

* `Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder` — produces the tables above; the
  machine-independent half asserts, at every size, that all n atoms are stored and that the rendered
  key sequence is non-decreasing; the timing half is a tripwire at 250 ms for 800 atoms (measured
  7.93 / 33.58 ms; pre-fix 424.00 / 512.35 ms). The class comment states the measured bound, both
  exponents, and the method.
* `Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder` — replicates the pre-fix
  algorithm (append + ordinal re-sort per insertion) as a reference and asserts the incremental set
  stores the **identical sequence** for 50 random insertion orders of the 8 mixed atoms. This is the
  direct guard for the hard constraint "the ordering of the stored atoms must not change".
* `Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder` — 8 atoms of 5 shapes inserted
  forward and backward yield the identical ordinal-sorted `Atoms` sequence.
* `Add_SameAtomTwiceFromDifferentContexts_IsStoredOnce` — the same symbol name from two contexts is
  value-equal, stored once; distinct atoms are all kept and the key order stays sorted.

No test was deleted, skipped or weakened anywhere in the repo.

## 7. What I could not improve / residual risk

* Residual exponent ≈ 1.8 (§3): the duplicate scan and the immutable copy per insertion remain.
  A real O(n log n) build would need an index by symbol plus a different storage shape.
* The tied-key fallback is defensive. I could not construct two distinct atoms that render
  identically from the public API — a record's `ToString()` includes its type name and all
  properties, and equal atoms are filtered by `Contains` first — so the branch is unexercised by the
  suite. It is kept because order is behaviour here and the guard is cheap.
* `FixExponent`/timing numbers are machine-dependent; only the 250 ms tripwire is asserted, with a
  ~7× margin over the worst measured post-fix value.
* `FromAtoms` still sorts with the per-comparison comparator (one call per set, not per insertion);
  it was left alone to keep the change bounded.
