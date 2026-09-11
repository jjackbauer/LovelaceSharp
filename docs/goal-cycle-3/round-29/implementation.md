# Round 29 — P2 item 17: Release build warning cleanup

STATUS: **COMPLETE.** Every in-scope warning class is fixed; the Release build is warning-clean for all 51
in-scope warnings and all 15 in-solution suites are green (0 failed).

Repo: `C:\Users\ricar\dev\LovelaceSharp` · no git commit was made · nothing touched outside SCOPE.
Evidence in this directory: `build-baseline.txt`, `build-after.txt`, `testsummary.txt`, `testlog-<Project>.txt`.

---

## 1. Before / after warning counts (measured, not assumed)

Method: `dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental` for **both** measurements —
an up-to-date incremental build prints `0 Warning(s)` because nothing recompiles, so a forced full rebuild is
required to see anything. Totals are MSBuild's own summary lines (each warning also appears twice in the
`-v:n` log body, so naive line counting over-reports by 2×; the summary counts unique warnings).

| # | Baseline (before) | After | Class |
|---|---|---|---|
| 34 | CS0109 `Lovelace.Real/Real.cs` | **0** | unnecessary `new` |
| 8 | CS8767 `Integer.cs:466,468` + `Real.cs:470,472` (2 params each) | **0** | operator nullability vs `IEqualityOperators<T,T,bool>` |
| 7 | CS8600 `Real.cs:635,862,910,1056,1116,1189,1979` | **0** | `[MaybeNullWhen(false)]` out params |
| 2 | CS8602 `DigitStore.cs:613`, `RealParseTests.cs:88` | **0** | maybe-null dereference |
| **51** | **in-scope total — exactly matches the orchestrator's report (34+8+7+2)** | **0** | |
| 3 | CS8600 `Lovelace.Symbolics.Tests` (DifferentialOracleTests.cs:119,286; SympyOracle.cs:308) | 3 | out of scope |
| 4 | xUnit1031 `Lovelace.Real.Tests` (RealAsyncLocalTests.cs:52,81,143; RealPiTests.cs:130) | 4 | out of scope |
| 2 | xUnit2013 (Representation.Tests:37; Suite.Tests:47) | 2 | out of scope |
| 1 | xUnit2020 (Symbolics.Tests/DifferentialOracleTests.cs:239) | 1 | out of scope |
| **61** | **whole-solution total** | **10** | 51 fixed, 10 documented in §5 |

```
BEFORE (build-baseline.txt):
    61 Warning(s)
    0 Error(s)
Build succeeded.

AFTER (build-after.txt):
    10 Warning(s)
    0 Error(s)
Build succeeded.
```

**Reported exact count: 10 — and all 10 are outside this round's SCOPE.** The in-scope warning count is 0.
The solution build is therefore *not* yet globally zero, and I am stating that plainly rather than claiming a
clean build: §5 lists every residual warning with its file:line and why it was not touched.

---

## 2. Per-class decision

### CS0109 ×34 — `Lovelace.Real/Real.cs` — **FIXED** (removed the unnecessary `new`)

The compiler was right in all 34 cases. `Real` derives from `Integer` (`Real.cs:21 public class Real : Int`
with `Real.cs:8 using Int = Lovelace.Integer.Integer`), and a `Real`-typed member does not hide the
`Integer`-typed base member because the signature differs:

* `static bool IsZero(Real value)` vs inherited `Integer.IsZero(Integer)` — different parameter type ⇒ no hiding.
* `static Real Parse(string s)` — `Integer` has no 1-argument `Parse(string)` ⇒ no hiding.
* `static bool TryParse(string?, IFormatProvider?, out Real)` vs `Integer.TryParse(string?, IFormatProvider?, out Integer)` — different `out` type ⇒ no hiding.

**Why removing `new` cannot rebind a call** (the risk called out in the brief): `new` is purely a
warning-suppression modifier — the C# language excludes it from member lookup and overload resolution, so a
member that hides nothing binds identically with or without it. Two independent empirical checks back this:

1. `static new` in `Real.cs` went **44 → 10**. The 10 survivors are exactly the members that *do* hide a base
   member: the six static properties (`One, Zero, Radix, AdditiveIdentity, MultiplicativeIdentity,
   NegativeOne` — lines 179–194, same names as `Integer`'s) and the four `Parse` overloads whose parameter
   lists are identical to `Integer`'s (1543, 1552, 1690, 1694). None of those 10 was ever flagged, before or
   after. They are the positive control proving the removal hit only non-hiding members.
2. After removal the build reports **zero CS0109 *and* zero CS0108**. If any of the 34 had genuinely hidden an
   inherited member, deleting `new` would have raised CS0108 ("hides inherited member; the `new` keyword is
   required"). None appeared.

Only the `new` token changed: `git diff -U0 -- Lovelace.Real/Real.cs` yields exactly **34** removed lines
matching `-.*static new `, and `git diff --numstat` reports `43 43` for the file — 34 `new` removals plus
2 operator lines plus 7 out-param lines, all in-place edits, **no line added or deleted**. No public member
changed name, signature or body.

### CS8767 ×8 — `Integer.cs:466,468` and `Real.cs:470,472` — **FIXED** (annotations matched to the interface)

`Real` and `Integer` are classes, so `IEqualityOperators<TSelf, TSelf, bool>` (reached through `INumber<T>`)
declares `operator ==(T? left, T? right)`. The implementations declared non-nullable parameters → CS8767 on
*both* parameters of both operators (4 positions × 2 params = the 8 reported).

```csharp
// Lovelace.Integer/Integer.cs:466,468
public static bool operator ==(Integer? left, Integer? right) => left!.Equals(right);
public static bool operator !=(Integer? left, Integer? right) => !left!.Equals(right);

// Lovelace.Real/Real.cs:470,472
public static bool operator ==(Real? left, Real? right) => left!.Equals(right);
public static bool operator !=(Real? left, Real? right) => !left!.Equals(right);
```

The annotation is metadata-only: the CLR signature stays `op_Equality(Real, Real)`, so **no call site changes
which overload it binds to**.

**Why the null-forgiving `!` on `left` — and why it is not an unscoped suppression.** The body is unchanged
from the original; declaring the parameter `Real?` forces the compiler to assume null, which would raise
CS8602 on the original dereference. Three candidates were weighed:

* `left is null ? right is null : left.Equals(right)` — the usual BCL pattern, **rejected because it changes
  observable behaviour**: today `Real? x = null; x == Real.Zero` and a typed `null == null` throw
  `NullReferenceException` from `left.Equals(...)`; a null-safe body would return `false` / `true`. The hard
  constraint is "NO behaviour change … if a fix would alter behaviour, STOP and document instead", so the
  existing null-operand contract is preserved verbatim.
* `[DisallowNull] Real? left` with the unchanged body — **tried and rejected**: `[DisallowNull]` is itself a
  strictness annotation, so the compiler still emits CS8767 for `left` (observed: with the attribute only the
  `right` warning cleared). It would also push a new call-site warning onto consumers passing a maybe-null
  operand. Reverted.
* `left!.Equals(right)` — **chosen.** One token, one expression; it asserts exactly the invariant the original
  code already relied on, and a null `left` still throws the same `NullReferenceException` from the same call.
  It is not a `#pragma`, and it suppresses nothing that was previously being reported: the CS8602 it covers is
  *introduced* by the annotation change that fixes CS8767.

No guard is needed for `right`: the type already treats a null right operand as `false`/`1`
(`Real.cs:395 if (other is null) return false;`, `Real.cs:409 if (other is null) return 1;`); annotating it
`Real?` alone cleared its warning.

### CS8600 ×7 — `Real.cs:635,862,910,1056,1116,1189,1979` — **FIXED** (locals declared maybe-null; null case already handled, now visible to the compiler)

These are genuine nullability defects in the *declaration*, not false positives:
`Natural.TryParse` really does `result = null; return false;` (`Natural.cs:685`), which is exactly what
`[MaybeNullWhen(false)]` advertises. The consuming code already handled that case — it just could not be seen,
because declaring the out variable non-nullable meant the maybe-null value was converted to a non-nullable type
at the call site:

```csharp
635:  if (!Nat.TryParse(allDigits, null, out Nat? mag))             mag = Nat.Zero;   // false path handled
862:  if (!TryParse(seedStr, null, out Real? seedReal) || IsZero(seedReal)) seedReal = One;
910:  if (!Nat.TryParse(truncStr, null, out Nat? truncNat))         return x;         // false path early-outs
1056: if (!Nat.TryParse(truncStr, null, out Nat? truncNat))         return x;
1116: if (!Nat.TryParse(truncStr, null, out Nat? truncNat))         return x;
1189: if (!Nat.TryParse(truncStr, null, out Nat? truncNat)) return x;
1979: if (!Nat.TryParse(newDigits, null, out Nat? newMag))          newMag = Nat.Zero;
```

The only edit is the *type declaration of the local* (`out Nat X` → `out Nat? X`, `out Real X` → `out Real? X`
at 862). Flow analysis then discharges everything: on `false` the code assigns a non-null fallback
(`Nat.Zero`, `One`) or returns; on `true` the `MaybeNullWhen(false)` contract guarantees non-null. No
suppression, no `!`, no fallback introduced, no numeric result touched — and the build shows no follow-on
CS8604 anywhere in those methods.

### CS8602 ×2 — `DigitStore.cs:613` (production) and `RealParseTests.cs:88` (test) — **FIXED**

**`DigitStore.cs:613` — genuine false positive of flow analysis, not a real defect.**
`bytesSnapshot` is assigned inside `if (!isZero) { … }` nested inside a `lock` (`DigitStore.cs:589–599`), and
the early `if (isZero) return "0";` sits after the lock. The compiler cannot correlate the local `isZero` with
that assignment across the lock/if nesting, so it keeps the "maybe null" state at line 613 — even though
`!isZero ⇒ bytesSnapshot != null` is an invariant (a rented `ArrayPool` buffer is never null, and it is
assigned before the lock is released). The invariant is stated where the state is consumed:

```csharp
// DigitStore.cs:604–611
// The compiler cannot correlate `isZero` with the assignment above, because that
// assignment happens inside the lock's nested branch, so state the invariant it
// cannot see: whenever !_isZero the rented snapshot is non-null (ArrayPool.Rent
// never returns null, and the buffer is assigned before the lock is released).
// Debug.Assert is annotated [DoesNotReturnIf(false)], which also discharges CS8602,
// and it compiles away in Release, so runtime behaviour is unchanged.
Debug.Assert(bytesSnapshot is not null);
```

Why this is the right treatment and not a dodge: (a) the assertion is provably true on every path, so it can
never fire; (b) `[Conditional("DEBUG")]` removes the call from the Release IL, so shipped behaviour is
identical — no branch, no allocation, no new exception path; (c) it does not change the declared type of the
local, so a future *real* null cannot masquerade as an empty buffer — contrast `?? Array.Empty<byte>()` or
`if (bytesSnapshot is null) return "0";`, both of which would convert a would-be crash into a silently wrong
numeric result, which is unacceptable in this library; (d) it needs one added `using System.Diagnostics;` and
no pragma. `Debug.Assert` is annotated `[DoesNotReturnIf(false)]` in .NET 5+, and the nullable walker consumes
that state even though the call is elided in Release — confirmed by CS8602 disappearing from the Release build.

**`RealParseTests.cs:88`** — `bool ok = Real.TryParse("3.14", out var r); Assert.True(ok);` leaves `r`
(a `Real?` from `[MaybeNullWhen(false)]`) maybe-null to the compiler, because xUnit's `Assert.True` carries no
`DoesNotReturnIf`. The fix asserts the thing actually being proved, using xUnit's `[NotNull]`-annotated
`Assert.NotNull`:

```csharp
86:  bool ok = Real.TryParse("3.14", out var r);
87:  Assert.True(ok);
88:  Assert.NotNull(r);          // added: [NotNull] discharges CS8602; also a strictly stronger test
89:  Assert.Equal("3.14", r.ToString());
```

This **strengthens** the test (it now fails loudly if a successful `TryParse` ever yields null) and deletes
nothing — the only test-file edit of the round, `git diff --numstat` = `1 0`.

---

## 3. Files changed (all inside SCOPE)

| File | Change | diff |
|---|---|---|
| `Lovelace.Real/Real.cs` | 34 × `new` removed; 2 operators annotated `Real?`; 7 out-param locals declared maybe-null | 43 insert / 43 delete (in-place) |
| `Lovelace.Integer/Integer.cs` | 2 operators annotated `Integer?` | 2 in-place lines |
| `Lovelace.Representation/DigitStore.cs` | `using System.Diagnostics;` + one invariant `Debug.Assert` | 9 insert / 0 delete |
| `Lovelace.Real.Tests/RealParseTests.cs` | one added `Assert.NotNull(r)` | 1 insert / 0 delete |

No `NoWarn`, no project-wide warning disabling, no `#pragma warning disable`, no csproj/warning-level change,
no test deleted, skipped or weakened, nothing outside SCOPE, nothing committed.

---

## 4. Verification — commands and observed output

### 4.1 Build
```
PS> dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental
Build succeeded.
    10 Warning(s)
    0 Error(s)
```
Unique-warning list = exactly the 10 out-of-scope entries in §5. **In-scope warnings: 0.**

### 4.2 Tests — all 15 in-solution suites, observed output (testsummary.txt)

Real.Tests was run with the brief's filter; the other 14 unfiltered (the filter is meaningless there).

| # | Suite | Result |
|---|---|---|
| 1 | Lovelace.Abstractions.Tests | `Passed! - Failed: 0, Passed: 20, Skipped: 0, Total: 20` |
| 2 | Lovelace.Array.Tests | `Passed! - Failed: 0, Passed: 19, Skipped: 0, Total: 19` |
| 3 | Lovelace.Complex.Tests | `Passed! - Failed: 0, Passed: 83, Skipped: 0, Total: 83` |
| 4 | Lovelace.Console.Tests | `Passed! - Failed: 0, Passed: 15, Skipped: 0, Total: 15` |
| 5 | Lovelace.Dsp.Tests | `Passed! - Failed: 0, Passed: 61, Skipped: 0, Total: 61` |
| 6 | **Lovelace.Integer.Tests** | `Passed! - Failed: 0, Passed: 148, Skipped: 0, Total: 148` |
| 7 | Lovelace.Knowledge.Tests | `Passed! - Failed: 0, Passed: 28, Skipped: 0, Total: 28` |
| 8 | **Lovelace.Natural.Tests** | `Passed! - Failed: 0, Passed: 195, Skipped: 0, Total: 195` |
| 9 | **Lovelace.Real.Tests** — `--filter "Category!=Heavy"` | `Passed! - Failed: 0, Passed: 272, Skipped: 0, Total: 272` (testlog-Lovelace.Real.Tests.txt) |
| 10 | **Lovelace.Representation.Tests** | `Passed! - Failed: 0, Passed: 91, Skipped: 0, Total: 91` |
| 11 | Lovelace.Run.Tests | `Passed! - Failed: 0, Passed: 39, Skipped: 0, Total: 39` |
| 12 | Lovelace.Studio.Tests | `Passed! - Failed: 0, Passed: 22, Skipped: 0, Total: 22` |
| 13 | **Lovelace.Suite.Tests** | `Passed! - Failed: 0, Passed: 638, Skipped: 0, Total: 638` |
| 14 | **Lovelace.Symbolics.Tests** | `Passed! - Failed: 0, Passed: 723, Skipped: 6, Total: 729` |
| 15 | precbench.Tests | `Passed! - Failed: 0, Passed: 13, Skipped: 0, Total: 13` |

**15/15 suites green, 0 failed, 2373 tests, 6 pre-existing SymPy-oracle skips in Symbolics.Tests.**

Pass counts do not contradict the previously recorded baseline either: round 5's implementation.md records the
same filtered `Lovelace.Real.Tests` suite at 272, and today's filtered run is also **272**.
All `*.csproj` listed in `LovelaceSharp.slnx` are covered: `Select-String LovelaceSharp.slnx -Pattern Tests`
returns exactly these 15.

On "unchanged pass counts": no test was added, removed or skipped — the only test edit is one added
assertion inside an existing test method (`RealParseTests.cs` diff = `1 0`), so test *counts* are structurally
unchanged; and the changed production code is behaviour-preserving by construction (§2). I did not have an
orchestrator-side per-suite baseline file to diff against, so the evidence here is (a) 0 failures across all
15 suites, and (b) the absence of any test-count-affecting edit.

### 4.3 Note: one apparent red suite in the raw sweep was a foreign worktree, not this repo

The sweep globbed `**/*.Tests.csproj` and therefore also ran the 7 test projects that live under
`.worktrees\binary\` — a **separate git worktree**, not in `LovelaceSharp.slnx`. Those foreign copies report different numbers than
the in-repo suites — e.g. `.worktrees\binary\Lovelace.Real.Tests` at 201, `.worktrees\binary\Lovelace.Suite.Tests`
at `Failed: 1, Passed: 261` (of 262), `.worktrees\binary\Lovelace.Studio.Tests` at 10 — because they are a
different branch with their own older sources and their own pre-built assemblies. Nothing in this round
touches them: this repo's `Lovelace.Real.Tests` is 272/272 and `Lovelace.Suite.Tests` is 638/638. Because the sweep wrote logs by project *name*, the foreign logs were overwritten by the
in-repo runs, so the retained `testlog-*.txt` files are the in-repo ones. Flagging it so the failure is not
mistaken for a regression from this round.

---

## 5. Warnings deliberately left — all OUT OF SCOPE, unchanged from baseline

None of these is part of P2 item 17's enumerated 51, and each lives in a file this round's SCOPE forbids
editing. They are reported rather than silently fixed.

| Count | Warning | Location | Why it stays / follow-up |
|---|---|---|---|
| 4 | xUnit1031 (blocking task op in test) | `Lovelace.Real.Tests/RealAsyncLocalTests.cs:52,81,143`; `RealPiTests.cs:130` | test-only; fix requires rewriting those call sites to `await` — files outside SCOPE (`RealParseTests.cs` is the only permitted test file) |
| 2 | xUnit2013 (use `Assert.Single`/`Assert.Empty`) | `Lovelace.Representation.Tests/DigitStoreSnapshotDigitsTests.cs:37`; `Lovelace.Suite.Tests/EmptyReductionTests.cs:47` | test-only, cosmetic |
| 1 | xUnit2020 (`Assert.True(false, …)` → `Assert.Fail`) | `Lovelace.Symbolics.Tests/DifferentialOracleTests.cs:239` | test-only, cosmetic |
| 3 | CS8600 | `Lovelace.Symbolics.Tests/DifferentialOracleTests.cs:119,286`; `SympyOracle.cs:308` | same class as the in-scope CS8600 (out-params of `TryXxx`), but in `Lovelace.Symbolics.Tests`, explicitly outside SCOPE; a one-line follow-up item mirroring §2's CS8600 fix |

**Bottom line:** every warning named in the defect report is gone (51 → 0), no numeric behaviour changed, no
pragma or NoWarn was introduced, and no test was weakened. The 10 residual warnings are pre-existing,
out-of-scope analyzer findings in three test projects, listed above for the next round.
