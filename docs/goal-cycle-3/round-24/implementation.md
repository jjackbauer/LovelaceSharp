# Round 24 — Minimal parentheses in the pretty printer

STATUS: DONE. Scope: `Lovelace.Symbolics/Printing.cs` (Pretty arm only),
`Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs` (new),
`Lovelace.Run.Tests/fixtures/solve_result.json` (golden regenerated),
`Lovelace.Symbolics/README.md` (two documented outputs, see §6b). No git commit.

## 1. Corpus (32 expressions, the round-trip input)

Defined in `PrettyParenthesesTests.Corpus`; each entry is infix source evaluated by a fresh
`SuiteEngine` + `SymbolicsPlugin`, so the expression under test is the one the language builds.

```
(x + 1)^12              exp(-x^2)                    integrate_full(exp(-x^2), x)
factor(-x^2 - 2*x - 1)  x - (y - a)                  x - y - a
a - (x + y)             -x^2                         (-x)^2
(-x)^3                  -(x + y)                     (-x - 1)^2
(-x)^(1/2)              (x^y)^2                      x^(y^2)
(x + y)*(x - y)         x*(y + a)                    -(x + y)*(x - y)
x/(y + 1)               (x + 1)/y                    x^(-2)
(x + 1)^(-1)            1/(2*(x + 1))                -1/(2*(x + 1))
(1/2)*x + 1/3           x/2                          exp(-(x + 1)^2)
sin(x + y) + cos(x^2)   sqrt(x + y)                  log(x + 1)/(x - 1)
diff(exp(x)*x^2, x)     sin((x + y)/(x - y))
```

Covers every shape the brief names: subtraction, unary minus, division, nested powers, powers of
negatives, products of sums, function application (incl. two-argument), negative exponents, and
rational coefficients.

The property is `CanonicalPrint(evaluate(PrettyPrint(e))) == CanonicalPrint(e)` — canonical
S-expression text on both sides, never printed infix text. `evaluate` is the real Lovelace.Suite
parser + interpreter (the same one that built `e`), so the comparison is on meaning, not spelling.

## 2. Step A — observation on the UNFIXED printer (test written first)

Command (before any edit to Printing.cs), filter `FullyQualifiedName~PrettyParenthesesTests`:

```
Failed!  - Failed:    24, Passed:    99, Skipped:     0, Total:   123, Duration: 126 ms
```

The doubling itself does NOT break the round trip: `((x + 1))^12` re-parses to
`(pow (add (rat 1 1) (sym x)) (rat 12 1))`, identical to the original. Dumping the corpus
before the fix:

```
(x + 1)^12           pretty: ((x + 1))^12                 [RT OK]
exp(-x^2)            pretty: exp((-x^2))                  [RT OK]
factor(-x^2 - 2*x - 1) pretty: -((x + 1))^2               [RT OK]
x^(y^2)              pretty: x^((y^2))                    [RT OK]
```

So the failing property for the doubling is **minimality**, exactly as the brief anticipated; I did
not fake a round-trip failure. But the round-trip test did fail on two corpus rows, and they are a
genuine, separate meaning-changing rendering defect in the same arm:

```
x - (y - a)  pretty: x - y - a   canon : (add (sym x) (mul (rat -1 1) (add (sym y) (mul (rat -1 1) (sym a)))))
                                 re-can: (add (sym x) (mul (rat -1 1) (sym a)) (mul (rat -1 1) (sym y)))  [RT DIFF]
a - (x + y)  pretty: a - x + y   canon : (add (sym a) (mul (rat -1 1) (add (sym x) (sym y))))
                                 re-can: (add (sym a) (sym y) (mul (rat -1 1) (sym x)))                  [RT DIFF]
```

The right operand of a rendered `-` was printed at precedence 1, so a nested sum lost its
parentheses and with them its meaning (`a - x + y` ≠ `a - (x + y)`). Fixed in the same bounded
edit (§3, item 6) and covered by the two corpus rows above.

Also corrected **in the test harness only** (not a source change): `2^(-1)` evaluates numerically
(Real, not symbolic) and `integrate_full` returns a `Record`; the corpus now uses
`(x + 1)^(-1)` and extracts the record's `expression` field, which is exactly the expression the
defect report renders.

## 3. Exact lines changed (Lovelace.Symbolics/Printing.cs, all numbers post-change)

Only the `Pretty` infix arm was touched. Canonical mode (`CanonicalPrint`) and Debug mode
(`DebugPrint`) are separate functions and were not edited.

| line | before | after | why |
|---|---|---|---|
| 474 (new) | — | `private const int ArgPrec = 1;` | one named precedence for call arguments: additive and tighter needs no parentheses inside `f(...)`, logical operators keep theirs |
| 602 | `Pretty(p.Base, 3, …)` | `Pretty(p.Base, 0, …)` | line 604-605 already wraps the base when `Prec(p.Base) <= 3`; asking for parentPrec 3 wrapped it a second time → `((x + 1))^12` |
| 609 | `Pretty(p.Exponent, 4, …)` | `Pretty(p.Exponent, 3, …)` | a `PowerExpr` exponent is parenthesised by the explicit rule at 610-613; 4 pre-wrapped and doubled it → `x^((y^2))`. Sums/products still get their parentheses here (3 > 1, 3 > 2) |
| 621 | `Pretty(a, 3, …)` | `Pretty(a, ArgPrec, …)` | arguments are delimited by the comma/closing paren → `exp((-x^2))` becomes `exp(-x^2)`, `sin((x + y))` becomes `sin(x + y)` |
| 643, 645, 647 | `Pretty(…, 4, …)` for diff/integrate/rootof operands | `ArgPrec` | same reason: `rootof((x^4 - x^2 - 1), 0)` → `rootof(x^4 - x^2 - 1, 0)`, `integrate((x + 1), x)` → `integrate(x + 1, x)` |
| 668 | `Pretty(baseExpr, 3, …)` | `Pretty(baseExpr, 0, …)` | the `O(...)` arm wraps its own base text at line 673; parentPrec 3 doubled it |
| 688 | `Pretty(inner, 1, …)` | `Pretty(inner, 2, …)` | right operand of a rendered `-`: a sum must keep its parentheses, a product/power must not (2 > 1 wraps sums only) |

### Why none of this can change a parse

Every edit either (a) removes a parenthesis pair the same arm re-adds immediately afterwards, or
(b) lowers the parent precedence of an operand that is already delimited by brackets of its own.

* (a) is idempotent by construction: lines 602+604-605, 609+610-613 and 668+673 each still wrap
  exactly once. `Prec` (line 457-466, unchanged) is the only decision input, so the
  wrap set is identical to before, minus the duplicate.
* (b) arguments of a call are separated by commas and terminated by `)`, so no precedence
  parenthesis is needed inside; the operator table itself (`Prec`) is untouched.
* The one edit that *adds* a parenthesis (line 688) adds it only where the operand's precedence is
  1 (a sum), which is exactly the case that previously misparsed.

The safety net is the round-trip property of §1: it compares canonical forms, so any change in what
the text means shows up as a diff, and it covers every shape whose parentheses were touched.

## 4. Minimality results (after the fix)

* `Minimality_NoDoubledParenthesisPair`: 32/32 pass. The detector matches parenthesis pairs
  (`((E))`: an open paren whose immediate successor opens the pair that closes exactly one
  character before its own match), so legitimate nesting such as `((x + 1)*(x - 1))^2` is not
  reported.
* Exact-string cases:
  ```
  (x + 1)^12                    -> (x + 1)^12
  exp(-x^2)                     -> exp(-x^2)
  factor(-x^2 - 2*x - 1)        -> -(x + 1)^2
  ```
* Corpus renderings after the fix (excerpt, from the post-fix dump):
  ```
  x - (y - a)   -> x - (y - a)          a - (x + y)  -> a - (x + y)
  (-x)^2        -> (-x)^2               x^(y^2)      -> x^(y^2)
  (x^y)^2       -> (x^y)^2              sqrt(-x)     for (-x)^(1/2)
  1/(2*(x + 1)) -> (2*(x + 1))^-1       log(x+1)/(x-1) -> log(x + 1)/(x - 1)
  sin(x + y) + cos(x^2) -> cos(x^2) + sin(x + y)
  (x + y)^12    (see above)             exp(-(x + 1)^2) -> exp(-(x + 1)^2)
  ```

## 5. Mode coherence

Modes touched: **Pretty only**. Canonical and Debug are separate code paths in the same file and
were not modified. Two new theories guard them over the same 32-expression corpus:
`CanonicalMode_StillRoundTripsThroughCanonicalParse` (canonical text → `CanonicalParse` →
canonical text is a fixed point, and `PrintMode.Canonical` returns exactly that text) and
`DebugMode_IsStructuralAndContainsNoPrecedenceParentheses` (`PrintMode.Debug` == `DebugPrint`,
no precedence parentheses at all). Both pass.

## 6. Golden diff and documentation

### 6a. `Lovelace.Run.Tests/fixtures/solve_result.json` — regenerated, 5 lines

Regenerated through the same pipeline the golden test compares against
(`TestSupport.RunFixtureAsync` + `Normalise`, two-space indent, LF, no BOM) and reviewed before
installing:

```diff
-    "display": "... rootof((x^4 - x^2 - 1), 0) ... rootof((x^4 - x^2 - 1), 1) ..."
+    "display": "... rootof(x^4 - x^2 - 1, 0) ... rootof(x^4 - x^2 - 1, 1) ..."
-    "typed":   "... rootof((x^4 - x^2 - 1), 0) ... rootof((x^4 - x^2 - 1), 1) ..."
+    "typed":   "... rootof(x^4 - x^2 - 1, 0) ... rootof(x^4 - x^2 - 1, 1) ..."
-                      "pretty": "rootof((x^4 - x^2 - 1), 0)",
+                      "pretty": "rootof(x^4 - x^2 - 1, 0)",
-                      "pretty": "rootof((x^4 - x^2 - 1), 1)",
+                      "pretty": "rootof(x^4 - x^2 - 1, 1)",
-      "display": "... rootof((x^4 - x^2 - 1), 0) ... rootof((x^4 - x^2 - 1), 1) ..."
+      "display": "... rootof(x^4 - x^2 - 1, 0) ... rootof(x^4 - x^2 - 1, 1) ..."
```

Every hunk is the removal of one redundant pair around the `rootof` polynomial (twice in each
`display`/`typed` string, once in each structured `pretty`). No canonical form, no status, no
diagnostic, no count changed. This is the only golden that changed; the other 18 fixtures were
already free of doubled parentheses (19 other rows).

### 6b. `Lovelace.Symbolics/README.md` — 2 lines (outside the literal scope list, flagged)

`Lovelace.Symbolics.Tests/UsageDocumentationTests` asserts every `lovelace`/`result` pair in the
README verbatim, so the two documented outputs that pinned doubled parentheses had to be brought in
line with the (verified correct) rendering. Documentation text only; no code, no behaviour.

```diff
--1/2*log((x + 1)) + 1/2*log((x - 1)) (Symbolic)
+-1/2*log(x + 1) + 1/2*log(x - 1) (Symbolic)
- integrate(exp((-x^2)), x) (Symbolic)
+ integrate(exp(-x^2), x) (Symbolic)
```

Both are removed redundant pairs; the round-trip corpus covers both shapes
(`log(x + 1)/(x - 1)`, `integrate_full(exp(-x^2), x)`).

## 7. Verification (observed output)

```
### dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:03.25

### dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   592, Skipped:     6, Total:   598, Duration: 13 s

### dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   637, Skipped:     0, Total:   637, Duration: 8 s

### dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 319 ms

### extra (sibling suites that print symbolic values)
Lovelace.Console.Tests: Passed! - Failed: 0, Passed: 15, Skipped: 0, Total: 15
Lovelace.Studio.Tests:  Passed! - Failed: 0, Passed: 22, Skipped: 0, Total: 22
```

Counts: Symbolics grew 467 → 598 (+131 = 32 corpus × 4 theories + 3 exact-string facts); the 6
skips are pre-existing. Suite and Run counts are unchanged (637 / 39).

## 8. What could not be made minimal

* `(x + 1)^12`, `(2*(x + 1))^-1`, `(x^y)^2`, `sqrt(-x)`: one pair **must** stay. A power base is
  parenthesized whenever `Prec(base) <= 3` because `x^y^2` is ambiguous; removing it would change
  the parse, so the round-trip corpus would (correctly) fail.
* `log(x + 1)`: no parentheses at all — the argument is delimited by the call's own parentheses.
  Nothing further to remove.
* `x - (y - a)`, `a - (x + y)`: the parentheses are load-bearing (they were *missing* before this
  round); `x - y*z` still prints without them.
* Not touched, deliberately: `Pretty` handles `PiecewiseExpr` guards at parentPrec 0 and the
  `MultiplyExpr` denominator join, which were already free of doubled pairs. No shape in the corpus
  is left with a redundant pair that the minimality detector can see.

## 9. Artifacts

* `Lovelace.Symbolics/Printing.cs` — the seven-line change above.
* `Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs` — corpus, round-trip, minimality, exact
  strings, mode coherence.
* `Lovelace.Run.Tests/fixtures/solve_result.json` — regenerated golden (diff in §6a).
* `Lovelace.Symbolics/README.md` — two documented outputs (§6b).
* Evidence files: `out/r24-stepA.txt` (pre-fix run), `out/r24-probe.txt` / `out/r24-probe-after.txt`
  (corpus dumps before/after), `out/r24-sym-full.txt`, `out/r24-run-full.txt`, `out/r24-verify.txt`,
  `out/r24-solve_result.json` (regenerated golden before install).
