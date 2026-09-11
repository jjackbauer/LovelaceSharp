# c5-report.md — power-base delimiters for negative numeric bases

Worktree (this report): C:/Users/ricar/dev/LovelaceSharp/.worktrees/c5-printer
Commit: 9228305 (detached HEAD, unchanged by this work)
Control (pristine pre-fix tree): C:/Users/ricar/dev/LovelaceSharp/.worktrees/c5-control-printer
  created for this task with: git -C C:/Users/ricar/dev/LovelaceSharp worktree add .worktrees/c5-control-printer HEAD
Deliverables: this file and c5-patch.diff

## 0. Verdict

The defect is real and is fixed by ONE rule shared by Pretty and LaTeX. The new tests fail on the
pre-fix tree (8 failures) and pass after the fix (279/279 in the two printer classes; the whole
Lovelace.Symbolics.Tests project: Failed 0, Passed 849, Skipped 6, Total 855).

## 1. Root cause — confirmed at the stated lines

All line numbers in this section are PRE-FIX (commit 9228305).

- Lovelace.Symbolics/Printing.cs:472 — PowPrec = 3
- Lovelace.Symbolics/Printing.cs:480-489 — Prec(Expr) has no arm for constants; they fall through
  to AtomPrec = 4.
- Lovelace.Symbolics/Printing.cs:509 — private static bool NeedsPowerBaseDelimiter(Expr b) =>
  Prec(b) <= PowPrec;  for a numeric constant this is 4 <= 3 = false, so a negative constant never
  receives parentheses.
- Lovelace.Symbolics/Printing.cs:529 — case RationalConstantExpr r: return r.Value.ToString();
  renders the base as the text "-1", i.e. a unary minus applied to the atom "1".
- Lovelace.Symbolics/Printing.cs:625-628 (Pretty) and :875-877 (LaTeX) both apply that one rule.

Reproduction through the printer (not the AOT binary — see section 9). Raw probe output, pre-fix
tree, before any implementation change:

    SOURCE (-1)^x => kind=Symbolic | canon=(pow (rat -1 1) (sym x)) | pretty=[-1^x] | latex=[-1^{x}]
    SOURCE (-2)^x => kind=Symbolic | canon=(pow (rat -2 1) (sym x)) | pretty=[-2^x] | latex=[-2^{x}]
    SOURCE (-1/2)^x => kind=Symbolic | canon=(pow (rat -1 2) (sym x)) | pretty=[-1/2^x] | latex=[-\frac{1}{2}^{x}]
    SOURCE (-1.5)^x => kind=Symbolic | canon=(pow (rat -3 2) (sym x)) | pretty=[-3/2^x] | latex=[-\frac{3}{2}^{x}]
    SOURCE -1^x => kind=Symbolic | canon=(rat -1 1) | pretty=[-1] | latex=[-1]

The canonical form of (-1)^x is exactly (pow (rat -1 1) (sym x)) as the orchestrator stated, and the
last line is the value proof: the language reads the printer's "-1^x" as -(1^x), whose canonical
form is the constant (rat -1 1) — a different value from (pow (rat -1 1) (sym x)). The printed text
means a different value; that is the defect.

## 2. The fix — one predicate, both arms

New predicate (now Printing.cs:528-529, pre-fix :509):

    private static bool NeedsPowerBaseDelimiter(Expr b, string renderedBase) =>
        Prec(b) <= PowPrec || renderedBase.StartsWith("-", StringComparison.Ordinal);

The two call sites pass the text they are about to raise and wrap it exactly once:
- Printing.cs:648 (Pretty, pre-fix :627)  — if (NeedsPowerBaseDelimiter(p.Base, b)) b = "(" + b + ")";
- Printing.cs:897 (LaTeX, pre-fix :876)   — if (NeedsPowerBaseDelimiter(p.Base, b)) b = "\left(" + b + "\right)";

Why the rendered text rather than a structural predicate over the constant kinds:

1. Prec classifies VALUES and cannot express the defect. Every constant is an atom by value, which
   is exactly why it falls to AtomPrec. What is not an atom is the TEXT "-1": a unary minus
   applied to the atom "1". The question the delimiter answers is textual — "does the text I am
   about to raise bind looser than ^?" — so the predicate has to ask the text.
2. No single structural predicate can be correct for both arms. The arms spell a complex constant
   differently where it matters: Pretty renders ComplexConstantExpr through ComplexToText
   (Printing.cs:1215-1222, called at :531) as "-1 + i" with no delimiters of its own, while LaTeX
   renders it at :774 as "(-1 + 1 i)", already delimited. A structural rule "constant with a
   negative leading part" would therefore have to be true for Pretty (wrap needed) and false for
   LaTeX (already wrapped) — i.e. it could not be one rule, and wrapping in LaTeX would emit a
   redundant pair, which LatexPrinterTests.Delimiters_AreLoadBearing (:131) rejects by design.
   Passing each arm its own text is the same question asked in both notations.
3. It is total: defined for every Expr and every rendering, and it covers every constant shape
   that can print a sign (integer, rational, real, complex) without enumerating kinds — no second
   table, which is what the file's own comments forbid (Printing.cs:461-465 and :706-712).
4. It cannot double a pair. The first clause short-circuits every base the old rule already
   wrapped, so those are wrapped exactly once; a base that binds tighter than a power (Prec > 3)
   is a symbol, a named constant, a function call, a relation, ... none of which renders with a
   leading "-". The minimality property (PrettyParenthesesTests.cs:120) and
   ReportedCase_PowerOfASum_HasOnePair (:162) prove the "((x + 1))^12" regression did not return.

Verified effect, post-fix raw probe output:

    SOURCE (-1)^x   pretty=[(-1)^x]    latex=[\left(-1\right)^{x}]              canon=(pow (rat -1 1) (sym x))  ROUNDTRIP=True
    SOURCE (-2)^x   pretty=[(-2)^x]    latex=[\left(-2\right)^{x}]              canon=(pow (rat -2 1) (sym x))  ROUNDTRIP=True
    SOURCE (-1/2)^x pretty=[(-1/2)^x]  latex=[\left(-\frac{1}{2}\right)^{x}]    canon=(pow (rat -1 2) (sym x))  ROUNDTRIP=True
    SOURCE (-1.5)^x pretty=[(-3/2)^x]  latex=[\left(-\frac{3}{2}\right)^{x}]    canon=(pow (rat -3 2) (sym x))  ROUNDTRIP=True
    SOURCE (-3)^(-x) pretty=[(-3)^(-x)] latex=[\left(-3\right)^{-x}]            canon=(pow (rat -3 1) (mul (rat -1 1) (sym x)))  ROUNDTRIP=True
    MODEL pow(add(x,1), int 12) => pretty=[(x + 1)^12] latex=[\left(x + 1\right)^{12}]   (unchanged, single pair)

The required shape (-1 + i)^x cannot be spelled in the language ("i" is not bound — finding F3), so
it was checked at the model level with Exprs.Power(Exprs.Complex(Rat.From(-1,1), Rat.From(1,1)), x):

    post-fix  pretty=[(-1 + i)^x]        latex=[(-1 + 1 i)^{x}]
    pre-fix   pretty=[-1 + i^x]          latex=[(-1 + 1 i)^{x}]   (raw probe, pre-fix tree)

i.e. the text predicate repairs the Pretty arm for a complex base with negative real part; the
LaTeX arm was already correct there because it wraps the complex text itself.

## 3. Step 1 — the failing test written first (raw output, unfixed tree)

Only the two test files were changed at this point (git status: M LatexPrinterTests.cs,
M PrettyParenthesesTests.cs).

Command:
    dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj --filter 'FullyQualifiedName~PrettyParenthesesTests|FullyQualifiedName~LatexPrinterTests' --nologo

Raw output (the 8 failures; the three new corpus shapes that cannot be rows are handled in
section 6):

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-3)^(-x)") [2 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
              ↓ (pos 1)
  Expected: "(pow (rat -3 1) (mul (rat -1 1) (sym x)))"
  Actual:   "(mul (rat -1 1) (pow (rat 3 1) (mul (rat "···
              ↑ (pos 1)

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1/2)^x") [6 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
              ↓ (pos 1)
  Expected: "(pow (rat -1 2) (sym x))"
  Actual:   "(mul (rat -1 1) (pow (pow (rat 2 1) (sym "···
              ↑ (pos 1)

    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting(source: "(-1/2)^x") [< 1 ms]
    Error Message:
     "(-1/2)^x" renders with a different grouping:
    pretty: -1/2^x
            -> neg(div(A:1,pow(A:2,A:x)))
    latex : -\frac{1}{2}^{x}
            -> neg(pow(div(A:1,A:2),A:x))

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1.5)^x") [< 1 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
              ↓ (pos 1)
  Expected: "(pow (rat -3 2) (sym x))"
  Actual:   "(mul (rat -3 1) (pow (pow (rat 2 1) (sym "···
              ↑ (pos 1)

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-2)^x") [< 1 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
              ↓ (pos 1)
  Expected: "(pow (rat -2 1) (sym x))"
  Actual:   "(mul (rat -1 1) (pow (rat 2 1) (sym x)))"
              ↑ (pos 1)

    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting(source: "(-1.5)^x") [< 1 ms]
    Error Message:
     "(-1.5)^x" renders with a different grouping:
    pretty: -3/2^x
            -> neg(div(A:3,pow(A:2,A:x)))
    latex : -\frac{3}{2}^{x}
            -> neg(pow(div(A:3,A:2),A:x))

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1)^x") [< 1 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
              ↓ (pos 1)
  Expected: "(pow (rat -1 1) (sym x))"
  Actual:   "(rat -1 1)"
              ↑ (pos 1)

    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.KnownRendering_PowerOfANegativeNumericBase_HasDelimitedBase [< 1 ms]
    Error Message:
     Assert.Equal() Failure: Strings differ
             ↓ (pos 0)
  Expected: "\left(-1\right)^{x}"
  Actual:   "-1^{x}"
             ↑ (pos 0)

    Failed!  - Failed:     8, Passed:   271, Skipped:     0, Total:   279, Duration: 147 ms - Lovelace.Symbolics.Tests.dll (net10.0)

The five corpus rows fail the ROUND-TRIP property; (-1/2)^x and (-1.5)^x additionally fail the
LaTeX/Pretty grouping-parity property; the explicit LaTeX assertion fails as required. The
Minimality, CanonicalMode and DebugMode theories pass on the new rows pre-fix, as expected (they do
not depend on the base delimiter).

## 4. Step 2/3 — implementation and raw passing output

The implementation is the predicate and two call sites quoted in section 2 (Printing.cs diff:
31 changed lines, +27/-4 including the comment).

Command and raw output, filtered to the two printer classes:

    dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj --filter 'FullyQualifiedName~PrettyParenthesesTests|FullyQualifiedName~LatexPrinterTests' --nologo
    ...
    Test run for C:\Users\ricar\dev\LovelaceSharp\.worktrees\c5-printer\Lovelace.Symbolics.Tests\bin\Debug\net10.0\Lovelace.Symbolics.Tests.dll (.NETCoreApp,Version=v10.0)
    A total of 1 test files matched the specified pattern.

    Passed!  - Failed:     0, Passed:   279, Skipped:     0, Total:   279, Duration: 148 ms - Lovelace.Symbolics.Tests.dll (net10.0)

Command and raw output, the whole project suite:

    dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj --nologo
    ...
      Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Limits_Agree [1 ms]
      Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Factorization_Agrees [1 ms]

    Passed!  - Failed:     0, Passed:   849, Skipped:     6, Total:   855, Duration: 1 m 7 s - Lovelace.Symbolics.Tests.dll (net10.0)

The 6 skips are the pre-existing SymPy-oracle-conditional skips (SympyOracle.cs:366,
Skip = "SymPy oracle unavailable: ..."); they are not related to this change and no test was
skipped, weakened, deleted or re-expected by it.

## 5. Step 4 — the same tests on the pristine pre-fix tree

CONTROL worktree C:/Users/ricar/dev/LovelaceSharp/.worktrees/c5-control-printer at HEAD 9228305.
Only the two test files were copied into it; no implementation change was made there:

    git status --porcelain
     M Lovelace.Symbolics.Tests/LatexPrinterTests.cs
     M Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs
    git rev-parse HEAD
    92283054d52f3e64538a3656d76b047bd3d37f56

Command (same filter as step 1) and raw output:

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-3)^(-x)") [3 ms]
    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1/2)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting(source: "(-1/2)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1.5)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-2)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.GroupingParity_LatexAndPrettyHaveTheSameStructuralNesting(source: "(-1.5)^x") [< 1 ms]
    Failed Lovelace.Symbolics.Tests.LatexPrinterTests.KnownRendering_PowerOfANegativeNumericBase_HasDelimitedBase [< 1 ms]

    Failed!  - Failed:     8, Passed:   271, Skipped:     0, Total:   279, Duration: 513 ms - Lovelace.Symbolics.Tests.dll (net10.0)

Identical set of 8 failures as step 1: the tests are red on the pre-fix tree and green on the fixed
tree, with no implementation change in CONTROL.

## 6. Required corpus shapes that could NOT be added (separate findings, raw evidence)

Three of the shapes the task named are not representable in this round-trip harness for reasons
that have nothing to do with the printer. Each was added to the corpus temporarily, run, then
removed (the delivered corpus keeps only rows that pass), exactly as the task instructs.

Raw evidence (fixed tree, temporary corpus rows, RoundTrip theory):

    Failed Lovelace.Symbolics.Tests.PrettyParenthesesTests.RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(source: "(-1)^2") [1 ms]
    Error Message:
     System.InvalidCastException : Unable to cast object of type 'Lovelace.Integer.Integer' to type 'Lovelace.Symbolics.Expr'.
     at Lovelace.Suite.Value.AsSymbolic() in ...\Lovelace.Suite\Value.cs:line 197
     at Lovelace.Symbolics.Tests.PrettyParenthesesTests.BuildAsync(String source) in ...\PrettyParenthesesTests.cs:line 97
     at ...RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm(String source) in ...\PrettyParenthesesTests.cs:line 119

    Failed ...(source: "(-3)^(-2)") [< 1 ms]
    Error Message:
     System.InvalidCastException : Unable to cast object of type 'Lovelace.Real.Real' to type 'Lovelace.Symbolics.Expr'.
     at Lovelace.Suite.Value.AsSymbolic() in ...\Lovelace.Suite\Value.cs:line 197
     at Lovelace.Symbolics.Tests.PrettyParenthesesTests.BuildAsync(String source) in ...\PrettyParenthesesTests.cs:line 97

    Failed ...(source: "(-2)^(1/2)") [3 ms]
    Error Message:
     System.InvalidOperationException : Undefined variable 'i'.
     at Lovelace.Suite.Interpreter.EvaluateVariable(VariableExpr var, Scope scope) in ...\Lovelace.Suite\Interpreter.cs:line 333
     at Lovelace.Symbolics.Tests.PrettyParenthesesTests.ReparseAsync(String pretty) in ...\PrettyParenthesesTests.cs:line 106

    Failed!  - Failed:     3, Passed:    37, Skipped:     0, Total:    40, Duration: 167 ms

- F1  (-1)^2  — the language EVALUATES it: the value is Integer 1 (probe: "kind=Integer raw=[Integer: 1]").
  The corpus row dies in BuildAsync before any printing, because the harness requires a symbolic
  value (PrettyParenthesesTests.cs:86-87, Value.cs:197 is an unchecked cast). It is not even a
  shape the printer can be handed: the model's smart constructor folds it too —
  Exprs.Power(Exprs.Integer(-1), Exprs.Integer(2)) canonicalises to (rat 1 1) and prints "1".
  Left out: keeping it would make the corpus permanently red for an evaluator reason, and no
  printer-side change can make it pass.
- F2  (-3)^(-2) — same class: the value is Real 0.(1) (= 1/9), so BuildAsync throws
  InvalidCastException (Real -> Expr). Left out for the same reason.
- F3  (-2)^(1/2) — BuildAsync succeeds (canonical (mul (i) (pow (rat 2 1) (rat 1 2))), pretty
  "i*sqrt(2)"), but the REPARSE throws "Undefined variable 'i'": the printer spells
  NamedConstant.I as "i" (Printing.cs:537 pretty, :780 LaTeX) while the interpreter binds no such
  name (Interpreter.cs:333). This is a genuine pretty-printing defect, but it is a different one
  (named-constant spelling / parser coverage), not the power-base rule; making it pass would mean
  widening this change to the language's constant vocabulary. Left out and reported here.

## 7. Other defects found while doing this (NOT fixed — outside the bounded objective)

- D1  POSITIVE fraction base: (1/2)^x still round-trips wrong after the fix, and is not fixed by a
  leading-minus rule by construction. Raw post-fix probe:

      SOURCE (1/2)^x => pretty=[1/2^x] latex=[\frac{1}{2}^{x}] canon=(pow (rat 1 2) (sym x)) reparse=(pow (pow (rat 2 1) (sym x)) (rat -1 1)) ROUNDTRIP=False

  The text "1/2" is likewise not an atom: the "/" binds looser than "^". The same class of defect
  as the one fixed here, but the predicate that would cover it is a different, larger rule ("is the
  rendered base text a single atom?" rather than "does it begin with a sign?"), and in LaTeX a
  bare \frac base is already grouped by the braces, so it must NOT be wrapped (that would break
  LatexPrinterTests.Delimiters_AreLoadBearing). Adjudication needed; I did not widen the change.
- D2  Pretty's ComplexConstantExpr text is not self-delimiting at all: ComplexToText
  (Printing.cs:1215-1222) returns e.g. "1 + i" with no parentheses, so the defect is not confined
  to power bases. Raw post-fix probe:

      MODEL pow(cplx(1,1),x)  => pretty=[1 + i^x]   latex=[(1 + 1 i)^{x}]   canon=(pow (cplx 1 1 1 1) (sym x))
      MODEL pow(cplx(-1,1),x) => pretty=[(-1 + i)^x] latex=[(-1 + 1 i)^{x}]  canon=(pow (cplx -1 1 1 1) (sym x))

  The negative-real-part case is repaired by the text rule; the non-negative one is not, because
  its text has no leading sign. The real repair belongs in ComplexToText (and would then have to
  remove the LaTeX arm's own parentheses at :774) — a wider change than this objective.
- D3  NamedConstant.I is printed as "i" but is not readable back by the language (see F3):
  Printing.cs:537 / :780 vs Interpreter.cs:333. Affects every pretty rendering of an expression
  containing i (for example sqrt(-4) prints "2*i").

## 8. Existing tests that encode the defect as correct

None found. No test was weakened, skipped, deleted or re-expected. The evidence: the only test
files touched are the two named above, and the FULL project suite is green after the fix — had any
existing assertion pinned the old rendering ("-1^x" for a negative-constant base), it would now be
red. Greps for "-1^" / "1^x" in Lovelace.Symbolics.Tests match only the pre-existing rows
"(x + 1)^(-1)" (LatexPrinterTests.cs:62, PrettyParenthesesTests.cs:58), which are unaffected.

## 9. Could not verify (explicit list)

- The AOT binary out/aot/Lovelace.Run.exe: my worktree has no out/ directory at all
  (Test-Path out/aot/Lovelace.Run.exe = False) and I did not publish one. Every printer claim above
  is made through Printing.PrettyPrint / Printing.PrettyPrint(PrintOptions(PrintMode.Latex)), which
  is the code that binary calls; the defect and the fix were not re-confirmed through the CLI.
- The subs() arithmetic itself: I did not run subs((-1)^x, x, 2) vs subs(-1^x, x, 2). The
  equivalent, stronger statement was verified at the language level instead: the printer's text for
  (-1)^x canonicalises to (rat -1 1), a constant, while the expression canonicalises to
  (pow (rat -1 1) (sym x)). See section 1.
- The other 14 test projects were not run (per instructions). The projects reachable from
  Lovelace.Symbolics.Tests (Lovelace.Symbolics, Lovelace.Suite, Lovelace.Run, Lovelace.Dsp,
  Lovelace.MathIR) did build and run there.
- Whether a SymPy oracle is available in this environment: the 6 skips in the full run are the
  oracle-conditional ones (SympyOracle.cs:366). I did not investigate or unskip them.
- The pre-fix FULL-project baseline: on CONTROL I ran only the filtered printer classes
  (Failed 8 / Passed 271), not the whole 855.

## 10. Files, status and the patch

git status --porcelain, run immediately BEFORE generating the patch (every path intended):

     M Lovelace.Symbolics.Tests/LatexPrinterTests.cs
     M Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs
     M Lovelace.Symbolics/Printing.cs

All scratch/probe files created during investigation (ScratchProbe.cs, ScratchProbe2.cs,
ScratchProbe3.cs) were deleted before this point, and the temporary corpus rows of section 6 were
removed; nothing else was ever added to the worktree.

Patch generated exactly as instructed:

    git add -A ; git diff --cached > c5-patch.diff

    git diff --cached --name-status
    M	Lovelace.Symbolics.Tests/LatexPrinterTests.cs
    M	Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs
    M	Lovelace.Symbolics/Printing.cs

    Lovelace.Symbolics.Tests/LatexPrinterTests.cs      | 16 +++++++++++
    Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs |  8 ++++++
    Lovelace.Symbolics/Printing.cs                     | 31 ++++++++++++++++++----
    3 files changed, 50 insertions(+), 5 deletions(-)

c5-patch.diff: 12732 bytes, 109 lines,
SHA256 01F9F4211B551F6BCD27F20EB024639B355397DE57046327527FDFA4C3372AAD

Note on scope of the patch: the git commands above were run while c5-report.md did not yet exist,
so the patch contains ONLY the implementation and the two real test files, as the housekeeping
instruction requires. c5-report.md is delivered as a file in the worktree root, not as a patch hunk.

What changed in the tests:

- Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:48-54 — five corpus rows added to the shared
  Corpus array, so the ROUND-TRIP property (RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm,
  :106) plus Minimality (:120), CanonicalMode (:188) and DebugMode (:199) all run over them:
  "(-1)^x", "(-2)^x", "(-1/2)^x", "(-1.5)^x", "(-3)^(-x)".
- Lovelace.Symbolics.Tests/LatexPrinterTests.cs:49-54 — the same four negative-numeric-base rows
  added to its Corpus, so the LaTeX grouping-parity (:108) and load-bearing-delimiter (:131)
  properties cover them too.
- Lovelace.Symbolics.Tests/LatexPrinterTests.cs:212-219 — the required explicit assertion
  KnownRendering_PowerOfANegativeNumericBase_HasDelimitedBase:
  Assert.Equal("\left(-1\right)^{x}", Latex(await BuildAsync("(-1)^x")));
