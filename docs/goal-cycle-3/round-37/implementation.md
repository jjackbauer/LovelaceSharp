# Round 37 — implementation

Repo: `C:\Users\ricar\dev\LovelaceSharp` · shell: Windows PowerShell 5.1 · **no git commit made**
(branch left dirty exactly as found, plus the edits below).

Written FIRST as a skeleton (baseline + two placeholders) before any source edit, then filled in
with the observed output of every command below.

Two independent items, disjoint files:

| Item | Files | State |
|---|---|---|
| A — clear the 20 residual analyzer warnings (test projects only) | 6 test files in 4 projects | **done: forced full rebuild is 0 warnings, 0 errors** |
| B — LaTeX symbol-name escaping | `Lovelace.Symbolics/Printing.cs` + 1 new test file | **done: 49/49 new assertions pass, JIT runner verified** |

Nothing outside SCOPE was edited. The 51 previously-fixed warnings (34 CS0109, 8 CS8767, 7 CS8600,
2 CS8602) were not touched: no `new` keyword came back and no numeric operator signature changed.
No `NoWarn`, no project-wide analyzer disable, no pragma anywhere in this round.

---

## 0. Baseline — the forced full rebuild BEFORE any change

```
> dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental
```

20 warning lines, 10 unique diagnostics, all in test projects (MSBuild prints each diagnostic once
in context and once in the recap block after `Build succeeded.`, hence 20 lines for 10 warnings):

```
Lovelace.Representation.Tests/DigitStoreSnapshotDigitsTests.cs(37,9): warning xUnit2013: Do not use Assert.Equal() to check for collection size. Use Assert.Single instead.
Lovelace.Real.Tests/RealAsyncLocalTests.cs(52,49): warning xUnit1031: Test methods should not use blocking task operations, as they can cause deadlocks. Use an async test method and await instead.
Lovelace.Real.Tests/RealAsyncLocalTests.cs(81,68): warning xUnit1031: Test methods should not use blocking task operations, as they can cause deadlocks. Use an async test method and await instead.
Lovelace.Real.Tests/RealAsyncLocalTests.cs(143,18): warning xUnit1031: Test methods should not use blocking task operations, as they can cause deadlocks. Use an async test method and await instead.
Lovelace.Real.Tests/RealPiTests.cs(130,65): warning xUnit1031: Test methods should not use blocking task operations, as they can cause deadlocks. Use an async test method and await instead.
Lovelace.Suite.Tests/EmptyReductionTests.cs(47,9): warning xUnit2013: Do not use Assert.Equal() to check for collection size. Use Assert.Empty instead.
Lovelace.Symbolics.Tests/DifferentialOracleTests.cs(119,66): warning CS8600: Converting null literal or possible null value to non-nullable type.
Lovelace.Symbolics.Tests/DifferentialOracleTests.cs(286,47): warning CS8600: Converting null literal or possible null value to non-nullable type.
Lovelace.Symbolics.Tests/SympyOracle.cs(308,56): warning CS8600: Converting null literal or possible null value to non-nullable type.
Lovelace.Symbolics.Tests/DifferentialOracleTests.cs(239,21): warning xUnit2020: Do not use Assert.True(false, message) to fail a test. Use Assert.Fail(message) instead.
    10 Warning(s)
    0 Error(s)
```

Per code: xUnit1031 ×4 sites, xUnit2013 ×2 sites, CS8600 ×3 sites, xUnit2020 ×1 site (each logged
twice = the 20 lines the brief names: xUnit1031 ×8, xUnit2013 ×4, CS8600 ×6, xUnit2020 ×2).

---

## ITEM A — the residual analyzer warnings

The Step-A failing output for item A **is** the analyzer diagnostics above: the "assertion" is the
analyzer rule, the observed failure is the build log. There is no separate test to write for a
warning; the after-state is the same command with `0 Warning(s)`.

### A.1 What changed, site by site

| # | Site | Rule | Change | Why it does not weaken the test |
|---|---|---|---|---|
| 1 | `DigitStoreSnapshotDigitsTests.cs:37` | xUnit2013 | `Assert.Equal(1, snapshot.Length)` → `Assert.Single(snapshot)` | Same claim (exactly one element); the next line still asserts the element is 5 |
| 2 | `EmptyReductionTests.cs:47` | xUnit2013 | `Assert.Equal(0, result.AsVector().Count)` → `Assert.Empty(result.AsVector())` | Same claim (the vector is empty) |
| 3 | `RealAsyncLocalTests.cs:21,52` | xUnit1031 | `public void` → `public async Task`; `Task.WhenAll(taskA, taskB).GetAwaiter().GetResult()` → `await Task.WhenAll(taskA, taskB)` | The assertion is still "after both tasks have finished, each saw only its own precision"; awaiting is the same join without the deadlock risk |
| 4 | `RealAsyncLocalTests.cs:64,81` | xUnit1031 | same shape: `async Task` + `long[] results = await Task.WhenAll(childA, childB)` | Same results array, same two assertions |
| 5 | `RealAsyncLocalTests.cs:123,143` | xUnit1031 | `async Task` + `await Task.WhenAll(tasks)` inside the existing `try`, `finally` still restores `DisplayDecimalPlaces` | Same join; the "no torn reads" assertion (`Assert.Empty(invalidObservations)`) is unchanged |
| 6 | `RealPiTests.cs:114,130` | xUnit1031 | `async Task` + `string[] results = await Task.WhenAll(tasks)` | Same join inside the same `Real.WithPrecision(10, 10)` scope; `Assert.All(results, r => Assert.Equal(expected, r))` unchanged |
| 7 | `DifferentialOracleTests.cs:119` | CS8600 | `Assert.True(Rl.TryParse(theirs[i + 1], null, out Rl theirValue), "<msg>")` → `if (!Rl.TryParse(theirs[i + 1], null, out Rl? theirValue)) Assert.Fail("<msg>")` | The failing case fails the test with the **identical message**; the successful case gives the compiler the `[MaybeNullWhen(false)]` guarantee instead of a claim it cannot check |
| 8 | `DifferentialOracleTests.cs:286` | CS8600 | `out Rl theirs` → `out Rl? theirs` (the existing `if (!TryParse(...)) return ...` guard already handles the null) | No behaviour change: the guard was and is the only way past the line |
| 9 | `SympyOracle.cs:308` | CS8600 | `out Rl theirs` → `out Rl? theirs` (same existing guard) | No behaviour change |
| 10 | `DifferentialOracleTests.cs:239` | xUnit2020 | `Assert.True(false, "<msg>")` → `Assert.Fail("<msg>")` (the `break;` stays; see A.3) | The default arm of a switch over matrix operations fails the test with the identical message |

CS8600 note: `Real.TryParse` is declared `TryParse([NotNullWhen(true)] string?, [MaybeNullWhen(false)]
out Real result)` (`Lovelace.Real/Real.cs:1685`) — `Real` is a reference type, so a non-nullable
`out Rl` local is a claim the method does not make. The fix is to accept the nullability the
signature declares and let the existing guard (or a real `Assert.Fail`) handle the null. No
suppression operator (`!`) was added anywhere.

### A.2 Decisions and the one dead end (recorded honestly)

* **Blocking waits → real awaits, not `Task.Run` wrappers or analyzer suppressions.** All four
  xUnit1031 sites were joins whose only purpose is "wait for these tasks"; making the test method
  `async Task` and awaiting is the fix the rule asks for, and xUnit 2.9.3 supports it. No test's
  assertions, tolerances, scopes or messages changed.
* **`Assert.Single`/`Assert.Empty` keep the exact claim.** In `DigitStoreSnapshotDigitsTests` the
  single-element claim now comes from `Assert.Single` and the value claim from the untouched next
  line, so nothing is lost.
* **`Assert.Fail` in a switch section: first attempt failed to compile — and that is in the log.**
  I first removed the `break;` on the theory that `Assert.Fail` is `[DoesNotReturn]` and therefore
  makes the section end point unreachable. The compiler disagreed:
  ```
  Lovelace.Symbolics.Tests/DifferentialOracleTests.cs(238,17): error CS8070: Control cannot fall out of switch from final case label ('default:')
      0 Warning(s)
      1 Error(s)
  ```
  Since CS8070 says the fall-out is *reachable* as far as the compiler is concerned, restoring
  `break;` cannot produce CS0162 (unreachable code) — and the rebuild confirms it (0 warnings).
  The `break;` carries a one-line comment saying it is reachable only if `Assert.Fail` returns.
* **What was NOT done:** no `NoWarn`, no `<AnalysisMode>`/`EnableNETAnalyzers` change, no
  `.editorconfig`/ruleset change, no `#pragma warning disable`, no `!` suppression, no `[Skip]`,
  no deleted or renamed test, no `[Fact]`/`[Theory]` attribute touched. The only test-visible
  changes are the 10 rows above.

### A.3 After state — the same forced full rebuild

```
> dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental
...
    0 Warning(s)
    0 Error(s)

Time Elapsed 00:00:07.46
```

**Exact count: 0 warnings, 0 errors** (grep of the full log for `(line,col): warning ` returns 0
lines; MSBuild's own summary says `0 Warning(s)`). All 36 projects in `LovelaceSharp.slnx` were
built (` -> ` lines in the log).

### A.4 Test counts for the projects item A touched (0 failed everywhere)

```
Lovelace.Real.Tests           Passed!  - Failed: 0, Passed: 285, Skipped: 0, Total: 285
Lovelace.Representation.Tests Passed!  - Failed: 0, Passed:  91, Skipped: 0, Total:  91
Lovelace.Suite.Tests          Passed!  - Failed: 0, Passed: 642, Skipped: 0, Total: 642
Lovelace.Symbolics.Tests      Passed!  - Failed: 0, Passed: 791, Skipped: 6, Total: 797
```

Unchanged by construction: the item-A edits add no test, remove no test, rename no test method and
introduce no skip — 8 lines become `async`/awaited, 2 assertion lines take the style the
analyzer asks for, 1 becomes an `Assert.Fail` guard, and 2 `out` declarations gain the `?` their
signature's `[MaybeNullWhen(false)]` already implies. Cross-round
evidence for the totals: round 36 recorded Suite 642 and Run 39, both identical today; round 29
recorded Representation 91, identical today. `Lovelace.Real.Tests` runs unfiltered here (285);
round 29's 272 was the `Category!=Heavy` subset, so it is not a like-for-like number.

### A.5 Left in item A

Nothing. Every one of the 10 unique diagnostics is fixed at the site; none was left with a
suppression, and none proved impossible without a behaviour change.

---

## ITEM B — LaTeX symbol-name escaping

### B.1 The gap

`Printing.LatexRender` rendered `case SymbolExpr s: return s.Symbol.Name;` verbatim, so any name
containing one of LaTeX's ten special characters (`# $ % & _ { } ~ ^ \`) went into the LaTeX
source as itself: a bare `%` comments out the rest of the line, a bare `}` closes the caller's
group early, a bare `\` starts a control sequence. A symbol NAME is opaque to the kernel
(identity is the name string, ordinal — `Symbol`, `Context.cs:10`), so this was a pure rendering
defect.

### B.2 Decision: an underscore is an ESCAPED LITERAL (`x\_1`), not a subscript

**Choice:** every special character, underscore included, is escaped; `x_1` renders as `x\_1`.

**Why:** a name is an atom. Rendering part of it as a subscript would assert structure the kernel
does not have — the kernel has no `x` and no `1`, it has one symbol whose name happens to be
`x_1`. It is also undefined in the cases that immediately follow: a leading underscore (`_x`),
a trailing one (`x_`), a doubled one (`a__b`), a name that is only underscores (`__`), and where
the "remainder" would itself need escaping/recursion (`x_1_2`). The literal form has one rule, is
total over all names, and is invertible (see B.5).

**What the other option would cost:** the subscript form buys nicer typography on the common
`x_1` case, and that is all it buys. Its price is a sub-grammar (split on the first underscore,
then decide what to do with the rest: escape it, recurse, or refuse), four undefined shapes
above, a rendering that claims a base/subscript relation between things the kernel considers one
atom, and a divergence from the Canonical and Pretty arms, which must keep showing the name
verbatim (§B.4 asserts they still do). It also loses invertibility: from `x_{1}` a reader cannot
tell whether the name was `x_1` or something else that was rendered the same way.

### B.3 The escape table (all ten specials)

| char | escape | why this one |
|---|---|---|
| `#` | `\#` | control symbol |
| `$` | `\$` | control symbol |
| `%` | `\%` | control symbol |
| `&` | `\&` | control symbol |
| `_` | `\_` | control symbol — the decision in B.2 |
| `{` | `\{` | control symbol |
| `}` | `\}` | control symbol |
| `~` | `\textasciitilde{}` | no control symbol exists for a literal tilde; the LaTeX2e text-symbol command, with the empty group that terminates the control word |
| `^` | `\textasciicircum{}` | same, for the caret |
| `\` | `\textbackslash{}` | same, for the backslash itself |

The three text-symbol commands are the LaTeX2e-documented escapes for the three specials that
have no control-symbol spelling; they carry `{}` so a following letter cannot be absorbed into
the control word (`b\s` → `b\textbackslash{}s`, which would otherwise read `\textbackslashs`).
Honest caveat: no TeX engine is installed in this environment (`pdflatex`, `xelatex`, `latexmk`,
`tectonic` — none present), so the escapes are argued from the LaTeX2e escape table, not compiled.
The property this item is accountable for — no bare special can reach the document — is what is
tested, and it holds for every character. A name with **no** special character is returned
unchanged (`AsSpan().IndexOfAny` early-out), so ordinary renderings stay byte-identical.

### B.4 What changed in `Printing.cs` (LaTeX arm only)

* new private helper `LatexName(string name)` (after `LatexRender`, near `LatexCallName`),
  next to it the doc comment stating the decision and why;
* `case SymbolExpr s: return LatexName(s.Symbol.Name);`
* `DerivativeExpr` / `IntegralExpr`: the variable list is a list of symbol NAMES, so it now uses
  `LatexName(v.Name)` too.

**Not changed:** `PrintExpr` (Canonical), `Pretty`, `DebugPrint`, `Prec`/`Delimit`, the
precedence table, and every other LaTeX arm. `LatexCallName` (function names inside
`\operatorname{...}`) is deliberately untouched — see B.8.

### B.5 Step-A failing output (item B), observed before the fix

New file `Lovelace.Symbolics.Tests/LatexSymbolNameEscapingTests.cs`, run against the unfixed
printer:

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~LatexSymbolNameEscapingTests"

  Failed ...LatexSymbolName_LeavesNoBareSpecialCharacter(name: "c{d}") [< 1 ms]
  Error Message:
   symbol "c{d}" renders as "c{d}" and leaves 2 bare LaTeX special character(s): '{' at offset 1, '}' at offset 3
  Failed ...LatexSymbolName_LeavesNoBareSpecialCharacter(name: "__") [< 1 ms]
  Error Message:
   symbol "__" renders as "__" and leaves 2 bare LaTeX special character(s): '_' at offset 0, '_' at offset 1
  Failed ...LatexSymbolName_InsideLargerExpressions_IsStillEscaped [47 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
            ↓ (pos 1)
Expected: "a\\%b \\cdot c\\&d"
Actual:   "a%b \\cdot c&d"
            ↑ (pos 1)
  Failed ...LatexSymbolName_UnderscoreIsAnEscapedLiteral_NotASubscript [< 1 ms]
  Error Message:
   Assert.Equal() Failure: Strings differ
            ↓ (pos 1)
Expected: "x\\_1"
Actual:   "x_1"
            ↑ (pos 1)

Failed!  - Failed:    37, Passed:    12, Skipped:     0, Total:    49, Duration: 211 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

37 of the 49 assertions failed. The 12 that passed at that state are worth naming, because they
are the reason the file needs more than one kind of assertion:

* 11 of the 12 decoder round-trips — decoding a rendering that contains no escape at all returns
  the name itself, so the round-trip alone cannot fail on an unescaped name;
* the bare-special scan for `b\s` — a bare backslash is unavoidably consumed as the start of a
  control sequence by any tokeniser, so the scan alone cannot see a missing backslash escape.

Each check covers the other's blind spot. **Part of item B was already correct**: a name with no
special character was already rendered verbatim and still is (the fix returns such names
unchanged, byte for byte).

One correction made before the printer change, recorded because it was mine: my first version of
`OnlyTheLatexArmEscapes_PrettyAndCanonicalKeepTheNameVerbatim` asserted the Canonical-mode
rendering *equals the name*, which is wrong — Canonical mode prints `(sym <name>)`. The assertion
was corrected to `"(sym " + name + ")"` (a strictly stronger check, not a weaker one); the failing
output above is from the corrected file.

### B.6 How the test tells an escaped occurrence from a bare one

Grepping for `_` cannot distinguish `x_1` (broken) from `x\_1` (correct), so the test does not
grep. It **tokenises into control sequences first**, then searches:

* a backslash followed by letters is a control WORD — consumed together with the optional empty
  group `{}` and/or one space that terminates it (TeX swallows that space);
* a backslash followed by a non-letter is a control SYMBOL, which consumes exactly that one
  character;
* every character a control sequence consumed is escaped; **every special character left over is
  bare**.

`BareSpecials(text)` returns the offsets of the leftovers, and the assertion is that the list is
empty (the failure messages above show it naming the offset and the character). Three further
layers make the claim hard to satisfy vacuously:

1. **Round-trip.** `DecodeEscapes` inverts the escaping (control symbols give their character
   back; `\textasciitilde{}`, `\textasciicircum{}`, `\textbackslash{}` give `~`, `^`, `\`);
   `Assert.Equal(name, DecodeEscapes(Latex(Symbol(name))))` says the rendering still *means* the
   same name, which a wrong-but-escaped rendering would fail.
2. **The decision is pinned as a string.** `x\_1` exactly, and `Assert.DoesNotContain("_{", ...)`
   so a future switch to subscripts fails loudly instead of passing the "no bare special" check.
3. **Every special is pinned individually** (`#`→`\#` … `\`→`\textbackslash{}`), so an escape
   table change cannot slip through the "no bare specials" check by escaping with something else.

Corpus: `x_1`, `p%q`, `a&b`, `h#1`, `c{d}`, `o}e`, `b\s`, `e^f`, `t~u`, `m$n`, `__`
and all ten at once (`x_1%&#'{}~^\`) — underscore, percent, ampersand, hash, brace, backslash and
caret are all covered, plus dollar and tilde. The bare-symbol scan is applied to whole renderings
of a symbol (where nothing else is in the string); names inside larger expressions are pinned as
exact strings instead (`a\%b \cdot c\&d`, `\operatorname{diff}(f, x\_1)`), because a whole-document
scan would also have to excuse the printer's own `^`, `{`, `}` (e.g. `x^{2}`).

### B.7 After state — the same tests, and the whole suites

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --no-build --filter "FullyQualifiedName~LatexSymbolNameEscapingTests"
Passed!  - Failed:     0, Passed:    49, Skipped:     0, Total:    49, Duration: 201 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

JIT runner (the built `Lovelace.Run` binary, `docs/goal-cycle-3/round-37/probe-latex-names.ps1`):

```
x_1    -> x\_1
p%q    -> p\%q
a&b    -> a\&b
h#1    -> h\#1
c{d}   -> c\{d\}
o}e    -> o\}e
t~u    -> t\textasciitilde{}u
e^f    -> e\textasciicircum{}f
m$n    -> m\$n
compound (x_1 + 1)/(x_1 - 1) -> \frac{x\_1 + 1}{x\_1 - 1}
backslash-name source: s = symbol("b\\s"); latex(s)
backslash-name -> b\textbackslash{}\textbackslash{}s
```

(The `.ls` string literal does not process backslash escapes — `"b\\s"` is a two-backslash name,
and both are escaped. The single-backslash case was probed separately: source
`s = symbol("b\s"); latex(s)` prints `b\textbackslash{}s`.) The compound probe is the point of the
item: the escapes are inside a `\frac` and the surrounding structure is untouched.

### B.8 Left in item B

* **Function names are NOT escaped.** `LatexCallName` still feeds `Function.Name` straight into
  `\operatorname{...}`, so a *function* name containing `}` or `%` would still break the group or
  comment out the line. Left alone because item B is about **symbol** names (`Symbol`,
  `Context.cs:10`): a function name is a different identity (`FunctionId`) on a different
  rendering path, the item did not ask for it, and no test covers it — changing it blind would
  alter output nobody asked to change. This is the honest residual of the same root cause: a
  one-line follow-up (the same escape table applies, since `\operatorname{...}`'s argument takes
  text-mode commands) plus a test that an awkward function name survives.
* **No TeX engine here**, so the three text-symbol escapes are not compile-verified (B.3).
* Nothing else: Canonical, Pretty and Debug output for awkward names is asserted unchanged by the
  new tests and by the pre-existing `LatexPrinterTests.CanonicalAndPretty_AreUntouchedByTheLatexArm`,
  which still passes.

---

## VERIFY — every command, as observed

```
> dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental
    0 Warning(s)
    0 Error(s)

Time Elapsed 00:00:07.46
```
(grep of the complete log for `(line,col): warning `: **0** lines — the exact count is 0.)

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   791, Skipped:     6, Total:   797, Duration: 17 s - Lovelace.Symbolics.Tests.dll (net10.0)

> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   642, Skipped:     0, Total:   642, Duration: 12 s - Lovelace.Suite.Tests.dll (net10.0)

> dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 817 ms - Lovelace.Run.Tests.dll (net10.0)
```

Two of the touched projects as well:

```
> dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj -c Release --nologo --no-build
Passed!  - Failed:     0, Passed:   285, Skipped:     0, Total:   285, Duration: 800 ms - Lovelace.Real.Tests.dll (net10.0)

> dotnet test Lovelace.Representation.Tests/Lovelace.Representation.Tests.csproj -c Release --nologo --no-build
Passed!  - Failed:     0, Passed:    91, Skipped:     0, Total:    91, Duration: 145 ms - Lovelace.Representation.Tests.dll (net10.0)
```

All suites end `Failed: 0`. The 6 skips in `Lovelace.Symbolics.Tests` are pre-existing (the same 6
appear in the round-23/33/34/29 records); the new file contains no `Skip`.

JIT runner (item B) — observed output in B.7, produced from the freshly built
`Lovelace.Run\bin\Release\net10.0` with `--no-build`.

---

## Files written/changed

Item A (test projects only):
`Lovelace.Representation.Tests/DigitStoreSnapshotDigitsTests.cs`,
`Lovelace.Suite.Tests/EmptyReductionTests.cs`, `Lovelace.Real.Tests/RealAsyncLocalTests.cs`,
`Lovelace.Real.Tests/RealPiTests.cs`, `Lovelace.Symbolics.Tests/DifferentialOracleTests.cs`,
`Lovelace.Symbolics.Tests/SympyOracle.cs`.

Item B: `Lovelace.Symbolics/Printing.cs` (LaTeX arm only),
`Lovelace.Symbolics.Tests/LatexSymbolNameEscapingTests.cs` (new, 49 assertions).

Evidence/artifacts: this file, `docs/goal-cycle-3/round-37/probe-latex-names.ps1`.
No commit; the working tree is left exactly as found plus these files.
