# Implementation — goal-cycle-6 / round 3 / row 1

**Objective.** Close open row 1: RealLiteral.FromRealExact drops provenance, so a truncated value
that leaves the numeric tier and comes back through a RealLiteral claims exact:true.

| item | value |
|---|---|
| tree | C:\Users\ricar\dev\LovelaceSharp (main working tree) |
| branch / HEAD | main / 300f6bb639d70233b4fcc060aa2731979fed42f9 — unchanged, nothing committed, nothing staged |
| platform | Windows PowerShell 5.1.26100.4202, dotnet 10.0.103 |
| patch | docs/goal-cycle-5/patches/wip-r20-exact2-UNVERIFIED.diff |
| patch SHA256 | 6a915dde51dbfb69848fa6d964233d051b4f316cd8d77f22ade964205ee76fe1 (certutil -hashfile, matches the task's hash) |
| product diff | Lovelace.Real/Real.cs, Lovelace.Symbolics/Expr.cs, Lovelace.Symbolics/Constructors.cs, Lovelace.Symbolics/Evaluation.cs — 4 files, 96 insertions(+), 9 deletions(-) |
| new test file | Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs (SHA256 4937A1683C9F6C162E0859F599C6C5C4B0BF39C24714732E55BF87169D0EC978) |
| C5Probe.cs | NOT landed — excluded from the apply; Test-Path Lovelace.Symbolics.Tests\C5Probe.cs = False |

Raw logs of every run below: %TEMP%\c6-r3\ (prefix-build.log, prefix-test.log, repair-single.log,
control.log, build-solution.log, test-solution.log, final-solution.log, oracle.log).

---

## 1. Apply the patch, excluding C5Probe.cs (step 1)

    PS> git apply --verbose --exclude=Lovelace.Symbolics.Tests/C5Probe.cs docs/goal-cycle-5/patches/wip-r20-exact2-UNVERIFIED.diff 2>&1
    APPLY_EXIT=0

stderr (the skip is the point of the --exclude):

    git : Skipped patch 'Lovelace.Symbolics.Tests/C5Probe.cs'.
    Checking patch Lovelace.Real/Real.cs...
    Checking patch Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs...
    Checking patch Lovelace.Symbolics/Constructors.cs...
    Checking patch Lovelace.Symbolics/Evaluation.cs...
    Checking patch Lovelace.Symbolics/Expr.cs...
    Applied patch Lovelace.Real/Real.cs cleanly.
    Applied patch Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs cleanly.
    Applied patch Lovelace.Symbolics/Constructors.cs cleanly.
    Applied patch Lovelace.Symbolics/Evaluation.cs cleanly.
    Applied patch Lovelace.Symbolics/Expr.cs cleanly.

git status --porcelain immediately after the apply (the docs/ entries are other agents' material,
present before this run; the four product files and the one new test file are this row's change):

     M Lovelace.Real/Real.cs
     M Lovelace.Symbolics/Constructors.cs
     M Lovelace.Symbolics/Evaluation.cs
     M Lovelace.Symbolics/Expr.cs
     M docs/goal-cycle-6/evidence.md
     M docs/goal-cycle-6/round-1/falsify-A.md
     M docs/goal-cycle-6/state.md
    ?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    ?? docs/goal-cycle-5/waiter6.ps1
    ?? docs/goal-cycle-6/round-2/

git diff --stat -- Lovelace.Real Lovelace.Symbolics:

     Lovelace.Real/Real.cs              | 15 ++++++++++
     Lovelace.Symbolics/Constructors.cs | 20 +++++++++++--
     Lovelace.Symbolics/Evaluation.cs   | 11 ++++++-
     Lovelace.Symbolics/Expr.cs         | 59 ++++++++++++++++++++++++++++++++++----
     4 files changed, 96 insertions(+), 9 deletions(-)

---

## 2. FIRST observation of the failure, on the patched tree, before any repair (step 2)

    PS> dotnet build Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo
    Build succeeded.
        0 Warning(s)
        0 Error(s)
    BUILD_EXIT=0

    PS> dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --no-build --filter "FullyQualifiedName~TruncatedValueThatHappensToBeARational"
    Test run for C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics.Tests\bin\Release\net10.0\Lovelace.Symbolics.Tests.dll (.NETCoreApp,Version=v10.0)
    A total of 1 test files matched the specified pattern.
      Failed Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact [310 ms]
      Error Message:
       Assert.IsType() Failure: Value is not the exact type
    Expected: typeof(Lovelace.Symbolics.NumInt)
    Actual:   typeof(Lovelace.Symbolics.NumRat)
      Stack Trace:
         at Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics.Tests\TruncatingEvaluationStaysInexactTests.cs:line 360

    Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1, Duration: 322 ms - Lovelace.Symbolics.Tests.dll (net10.0)
    TEST_EXIT=1

This reproduces the triage's blocker (docs/goal-cycle-6/round-2/triage-r20.md:7-9, :77-78): the
patched product change is what makes lines 340-356 pass, and the test then dies on line 360, a
hard-coded concrete-type assertion about an unrelated exact value.

---

## 3. The repair (step 3) — one assertion and its immediate comment

Diff against the file exactly as the patch authored it
(.worktrees/c6-r20 carries the patch applied verbatim; git diff --no-index, exit 1 = the expected
difference; only this one hunk changed):

    PS> git diff --no-index -- .worktrees/c6-r20/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    diff --git a/.worktrees/c6-r20/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs b/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    index 71e75e1..8f75a50 100644
    --- a/.worktrees/c6-r20/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    +++ b/Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    @@ -356,8 +356,16 @@ public class TruncatingEvaluationStaysInexactTests
                 "to be a rational; the exact expression denotes 1/2 and this is not 1/2");

             // the control on the other side of the same question: the exact value 1/2 is exact, and so
    -        // is the exact decimal 0.5 when it is read as a value rather than produced by a truncation
    -        Assert.IsType<NumInt>(Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings));
    +        // is the exact decimal 0.5 when it is read as a value rather than produced by a truncation.
    +        // Exprs.Rational canonicalises 1/2 onto the exact RATIONAL tier exactly as Exprs.Integer
    +        // does — the same tier-neutral reading the control at :309-311 relies on — so the concrete
    +        // carrier is NumRat, not NumInt. Naming the class asserted an implementation detail that
    +        // contradicts the value, but the failure it guarded against is real and is kept: RatOf
    +        // throws for anything off the exact tier, so this still fails if 1/2 stops being exactly the
    +        // rational 1/2 carried exactly, and the flag is now asserted beside the value.
    +        var foldedHalf = Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings);
    +        Assert.Equal(Rat.From(1, 2), NumOps.RatOf(foldedHalf));
    +        Assert.True(foldedHalf.IsExact, "the exact rational 1/2 must fold exactly, not merely look exact");
             Assert.True(half.IsExact, "0.5 parsed from an exact decimal literal is exact");
             Assert.True(RealLiteral.FromRealExact(half).ToReal().IsExact,
                 "the exact 1/2 must not be caught by the same rule");

**Why the old assertion was wrong.** Exprs.Rational(1, 2) is Rational(Int, Int) ->
Rational(Rat) -> RationalConstantExpr(Rat.From(1, 2)) on the exact tier
(Lovelace.Symbolics/Constructors.cs:47-58), and EvaluateToNum's RationalConstantExpr arm
returns new NumRat(r.Value) (Lovelace.Symbolics/Evaluation.cs:429). The carrier of 1/2 is
therefore NumRat and always was; the file's own control 200 lines earlier reads the same way with
NumOps.RatOf at TruncatingEvaluationStaysInexactTests.cs:311. Asserting the concrete class was
asserting an implementation detail that contradicts the value.

**Why it is not weaker.** NumOps.RatOf throws InvalidOperationException("Value is not exact.") for
any value off the exact tier (Lovelace.Symbolics/Evaluation.cs:73-78), so the new pair fails for
every failure the old one guarded against:
- wrong value on the exact tier -> Assert.Equal fails (observed, mutation M1 below);
- right value demoted off the exact tier (NumReal) -> RatOf throws (observed, mutation M2 below);
- and the route is now asserted too: Assert.True(foldedHalf.IsExact), which the old
  Assert.IsType<NumInt> did imply but never stated.
The only input that now passes and used to fail is the correct one — exactly the rational 1/2,
carried exactly — which is the case the objective requires to be allowed.

The file's other 31 cases and every other assertion are byte-identical to the patch (the diff above
is the whole repair). No test was deleted, skipped, loosened or renamed.

### 3a. Teeth of the repaired assertion, observed by mutation (temporary, reverted)

Two temporary one-line mutations of the product were used to falsify the new assertion; each was
reverted with the edit tool and the file hash was checked back to
54275D6A1836C347C81C3945F1014D82689EAC099618563D6C1EF6CF87ECAA0B afterwards.

M1 — same tier, wrong value: Evaluation.cs:429 changed to
"case RationalConstantExpr r: return new NumRat(Rat.From(1, 3));"

    Failed Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact [61 ms]
      Error Message:
       Assert.Equal() Failure: Values differ
      Expected: 1/2
      Actual:   1/3
    Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1

M2 — right value, off the exact tier: Evaluation.cs:429 changed to
"case RationalConstantExpr r: return new NumReal(Rl.AsInexact(Rl.Parse("0.5", null)));"

    Failed Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact [59 ms]
      Error Message:
       System.InvalidOperationException : Value is not exact.
      Stack Trace:
         at Lovelace.Symbolics.NumOps.RatOf(Num n) in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics\Evaluation.cs:line 77
         at Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests.TruncatedValueThatHappensToBeARational_IsStillInexact() in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Symbolics.Tests\TruncatingEvaluationStaysInexactTests.cs:line 367
    Failed!  - Failed:     1, Passed:     0, Skipped:     0, Total:     1

After the revert: (Get-FileHash Lovelace.Symbolics\Evaluation.cs -Algorithm SHA256).Hash =
54275D6A1836C347C81C3945F1014D82689EAC099618563D6C1EF6CF87ECAA0B, identical to the hash taken
before M1. The repaired test passes again on the restored tree:

    PS> dotnet test ... --filter "FullyQualifiedName~TruncatedValueThatHappensToBeARational"
    Passed!  - Failed:     0, Passed:     1, Skipped:     0, Total:     1, Duration: 80 ms - Lovelace.Symbolics.Tests.dll (net10.0)
    TEST_EXIT=0

---

## 4. The whole solution on the repaired tree (step 4)

    PS> dotnet test LovelaceSharp.slnx --configuration Release --nologo

Run twice (once before the mutation experiments, once after the revert) — the FINAL run is quoted;
both ended TEST_EXIT=0 with identical totals. Per-project totals, verbatim:

    Passed!  - Failed:     0, Passed:    20, Skipped:     0, Total:    20, Duration: 78 ms - Lovelace.Abstractions.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    28, Skipped:     0, Total:    28, Duration: 167 ms - Lovelace.Knowledge.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    91, Skipped:     0, Total:    91, Duration: 292 ms - Lovelace.Representation.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    19, Skipped:     0, Total:    19, Duration: 81 ms - Lovelace.Array.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:   195, Skipped:     0, Total:   195, Duration: 145 ms - Lovelace.Natural.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:   148, Skipped:     0, Total:   148, Duration: 69 ms - Lovelace.Integer.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    13, Skipped:     0, Total:    13, Duration: 82 ms - precbench.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    15, Skipped:     0, Total:    15, Duration: 303 ms - Lovelace.Console.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    96, Skipped:     0, Total:    96, Duration: 2 s - Lovelace.Complex.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:  2489, Skipped:     0, Total:  2489, Duration: 4 s - Lovelace.Real.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    61, Skipped:     0, Total:    61, Duration: 3 s - Lovelace.Dsp.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:   813, Skipped:     0, Total:   813, Duration: 14 s - Lovelace.Suite.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:   175, Skipped:     0, Total:   175, Duration: 24 s - Lovelace.Run.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:    22, Skipped:     0, Total:    22, Duration: 246 ms - Lovelace.Studio.Tests.dll (net10.0)
    Passed!  - Failed:     0, Passed:  1055, Skipped:     8, Total:  1063, Duration: 1 m 59 s - Lovelace.Symbolics.Tests.dll (net10.0)
    TEST_EXIT=0

Totals: 15 projects, 5240 passed, 8 skipped (pre-existing [SKIP] SymPy-oracle and proof tests),
5248 total, **0 failed**. Lovelace.Symbolics.Tests reads 1063 rather than the triage's 1064
because C5Probe.cs is not in the tree.

The new file's 32 cases alone, on the same repaired tree:

    PS> dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --no-build --filter "FullyQualifiedName~TruncatingEvaluationStaysInexactTests"
    Passed!  - Failed:     0, Passed:    32, Skipped:     0, Total:    32, Duration: 1 m 50 s - Lovelace.Symbolics.Tests.dll (net10.0)
    FILTERED_EXIT=0

### 4a. The eight skipped oracle/proof cases, actually run

The 8 skips in the table above are pre-existing [SKIP] SymPy-oracle and proof cases. With the oracle
made mandatory they execute, and they pass too (same tree, after the repair):

    PS> $env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
    PS> $env:LOVELACE_REQUIRE_SYMPY = '1'
    PS> python -c "import sympy; print('sympy', sympy.__version__)"
    sympy 1.14.0
    PS> dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --no-build
    Passed!  - Failed:     0, Passed:  1063, Skipped:     0, Total:  1063, Duration: 2 m 12 s - Lovelace.Symbolics.Tests.dll (net10.0)
    ORACLE_TEST_EXIT=0

So on the repaired tree Lovelace.Symbolics.Tests is 1063/1063 with the oracle required, and no case
in it is skipped.

---

## 5. Control tree — the repaired tests against the pristine product (step 5)

Control: C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-r20-ctl at HEAD 300f6bb, product
untouched (the earlier triage had already put the two new test files there untracked; this run
overwrote the test file with the repaired one and copied no product file).

    PS> git status --porcelain            # in the control tree
    ?? Lovelace.Symbolics.Tests/C5Probe.cs
    ?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    PS> git rev-parse HEAD
    300f6bb639d70233b4fcc060aa2731979fed42f9
    PS> (Get-FileHash Lovelace.Symbolics.Tests\TruncatingEvaluationStaysInexactTests.cs -Algorithm SHA256).Hash
    4937A1683C9F6C162E0859F599C6C5C4B0BF39C24714732E55BF87169D0EC978     # identical to the main tree's repaired file

    PS> dotnet build Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo
    Build succeeded.
        0 Warning(s)
        0 Error(s)
    BUILD_EXIT=0

    PS> dotnet test Lovelace.Symbolics.Tests\Lovelace.Symbolics.Tests.csproj --configuration Release --nologo --no-build --filter "FullyQualifiedName~TruncatingEvaluationStaysInexactTests"
    Failed!  - Failed:    14, Passed:    18, Skipped:     0, Total:    32, Duration: 3 s - Lovelace.Symbolics.Tests.dll (net10.0)
    TEST_EXIT=1

The 14 failing cases on the pristine product (from control.log):

    Failed ...TruncatingEvaluationStaysInexactTests.SymbolicTreeFlag_AndNumericReadBack_Agree(expression: "subs(x*2, x, pi(30))")
    Failed ...SymbolicTreeFlag_AndNumericReadBack_Agree(expression: "subs(x + 1, x, pi(30))")
    Failed ...SymbolicTreeFlag_AndNumericReadBack_Agree(expression: "subs(x + x, x, pi(30))")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(subs(x + 1, x, pi(30)), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(sin(pi(30)/4), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(sin(pi(30)/6), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(atan(pi(30)/4), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(sin(pi(45)/6), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(cos(pi(30)/6), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(exp(pi(30)/10), 40)")
    Failed ...FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(sin(pi(30)/6), 100)")
    Failed ...TruncatedValueThatHappensToBeARational_IsStillInexact
    Failed ...FunctionOfATruncatedArgument_IsInexact
    Failed ...LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute

Two of these are worth quoting because they name the row's defect directly:

    FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(expression: "evalf(sin(pi(30)/6), 40)")
      Error Message:
       evalf(sin(pi(30)/6), 40) is a function of a truncated value and must not claim exactness;
       published 0.499999999999999999999999999999 with exact=True
      Stack Trace: ... line 114

    LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute
      Error Message:
       pi to 30 places lost digits when it was evaluated and must not come back exact through the symbolic literal (3.141592653589793238462643383279)
      Stack Trace: ... line 217

and the repaired case still fails on the pristine product, now at the earlier, product-level
assertion rather than the tier assertion:

    Failed ...TruncatedValueThatHappensToBeARational_IsStillInexact [36 ms]
      Error Message:
       the truncated argument must come back inexact
      Stack Trace:
         at ...TruncatedValueThatHappensToBeARational_IsStillInexact() in C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-r20-ctl\Lovelace.Symbolics.Tests\TruncatingEvaluationStaysInexactTests.cs:line 340

So the repair did not soften the file: 14 of 32 new cases fail on the unpatched product (the same
14 the triage measured) and 0 fail on the patched one.

---

## 6. Wire probes through the runner's real entry point (step 6)

Command shape (Lovelace.Run built in Release by the runs above; BEFORE = the same binary built in
the pristine control worktree .worktrees\c6-r20-ctl, AFTER = the main working tree):

    dotnet <Lovelace.Run>\bin\Release\net10.0\Lovelace.Run.dll --eval "<script>" --json --omit-functions

result.structured, verbatim, in both trees:

    BEFORE  'evalf(sin(pi(30)/6), 40)'
      {"kind":"Real","value":"0.499999999999999999999999999999","exact":true,"numerator":"499999999999999999999999999999","denominator":"1000000000000000000000000000000"}
    AFTER   'evalf(sin(pi(30)/6), 40)'
      {"kind":"Real","value":"0.499999999999999999999999999999","exact":false}

    BEFORE  'evalf(sin(pi(1)*1/6), 40)'
      {"kind":"Real","value":"0.4939846521590624789945640658702374513096","exact":false}
    AFTER   'evalf(sin(pi(1)*1/6), 40)'
      {"kind":"Real","value":"0.4939846521590624789945640658702374513096","exact":false}

    BEFORE  'evalf(sin(pi(30)/6), 100)'
      {"kind":"Real","value":"0.499999999999999999999999999999","exact":true,"numerator":"499999999999999999999999999999","denominator":"1000000000000000000000000000000"}
    AFTER   'evalf(sin(pi(30)/6), 100)'
      {"kind":"Real","value":"0.499999999999999999999999999999","exact":false}

Read-off: the row-1 expression published exact:true WITH a numerator and denominator (the
laundering) before the change and publishes exact:false with neither after it. The second probe is
unchanged, as the test file's own doc comment predicted (its 40-digit rendering already landed on
the width heuristic's boundary) — it is not the case that carries the row.

Full envelope, AFTER, for the row-1 expression (unchanged shape, same display value):

    {"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Real","display":"0.499999999999999999999999999999","typed":"0.499999999999999999999999999999 (Real)","structured":{"kind":"Real","value":"0.499999999999999999999999999999","exact":false}},"output":[],"variables":[{"name":"_","kind":"Real","display":"0.499999999999999999999999999999","structured":{"kind":"Real","value":"0.499999999999999999999999999999","exact":false}}],"functions":[],"elapsed":"63.09 ms",...}

**Genuinely exact values that must stay exact** (an over-correction would be caught here):

    BEFORE  'evalf(1/2, 40)'  {"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}
    AFTER   'evalf(1/2, 40)'  {"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}

    BEFORE  '2+3'             {"kind":"Natural","value":"5","exact":true}
    AFTER   '2+3'             {"kind":"Natural","value":"5","exact":true}

    BEFORE  'sin(0)'          {"kind":"Symbolic","pretty":"0","canonical":"(rat 0 1)","domain":"rational","exact":true,"nodeCount":1,...}
    AFTER   'sin(0)'          {"kind":"Symbolic","pretty":"0","canonical":"(rat 0 1)","domain":"rational","exact":true,"nodeCount":1,...}

(Field note: the Real structured DTO carries kind/value/exact/numerator/denominator — it has no
"canonical" key; canonical/pretty belong to the Symbolic DTO, which is why sin(0) is quoted with
canonical and 1/2 with numerator/denominator. All three controls are byte-identical before and
after, so nothing was swept into inexactness.)

---

## 7. Clean full-solution build (step 7)

    PS> dotnet build LovelaceSharp.slnx --configuration Release --nologo
      Determining projects to restore...
      All projects are up-to-date for restore.
      ... 36 project -> dll lines ...
    Build succeeded.
        0 Warning(s)
        0 Error(s)
    Time Elapsed 00:00:07.98
    BUILD_EXIT=0

No file-lock warning appeared in any build or test run of this session, so no stale testhost process
had to be killed. (Said explicitly because step 7 asks for it.)

---

## 8. Working-tree state at the end (step 8)

    PS> git status --porcelain
     M Lovelace.Real/Real.cs
     M Lovelace.Symbolics/Constructors.cs
     M Lovelace.Symbolics/Evaluation.cs
     M Lovelace.Symbolics/Expr.cs
     M docs/goal-cycle-6/evidence.md
     M docs/goal-cycle-6/journal.md
     M docs/goal-cycle-6/round-1/falsify-A.md
     M docs/goal-cycle-6/state.md
    ?? Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs
    ?? docs/goal-cycle-5/waiter6.ps1
    ?? docs/goal-cycle-6/round-2/
    PS> git diff --cached --stat        # empty output: nothing staged
    STAGED_EXIT=0
    PS> git rev-parse HEAD
    300f6bb639d70233b4fcc060aa2731979fed42f9
    PS> git rev-parse --abbrev-ref HEAD
    main

Nothing was committed, pushed or staged. The four product files and the one new test file are the
whole change; docs/goal-cycle-5/waiter6.ps1, docs/goal-cycle-6/round-2/ and the docs/*.md
modifications are other agents' material in this shared workspace (journal.md changed during this
run) and were not touched by this agent. The only file this agent wrote outside the SCOPE list is
this report.

---

## What I could not verify

1. **Why the four "coincidence" cases pass in both trees.** evalf(sin(pi(1)*1/6), 40),
   evalf(cos(pi(1)*1/6), 40), evalf(sin(pi(1)*1/4), 40) and evalf(atan(1), 40) answer false on both
   trees (probe 2 above is the observed instance). The file's doc comment attributes this to the
   width heuristic's boundary; I did not instrument Rl.TryParse to prove the mechanism, so those
   four cases remain tooth-free and I am reporting that rather than claiming them.
2. **The 18 green-on-both cases.** The control run shows 18 cases passing on the pristine product.
   Eleven are the declared positive controls (ElementaryFunctionOfAnExactArgument_StaysExact), plus
   ExactControls_StayExact, LiteralRoundTrip_KeepsGenuinelyExactValuesExact,
   SymbolicTreeFlag_AndNumericReadBack_Agree(sqrt(2)) and the four coincidence cases. Their job is
   to stay green; I did not attempt to give them teeth.
3. **Aliasing of Rl.AsInexact's copy** (triage's open question at triage-r20.md:128-130). The patch's
   new public member Real.cs:219 copies an exact Real before marking it; I did not construct a case
   where the copy is observable, so I cannot say whether it is load-bearing or defensive.
4. **Whether RealLiteral.IsExact belongs in Equals+GetHashCode** (triage-r20.md:135-138). The
   full-solution run is green with it included, which is the collateral evidence the triage lacked,
   but the design question itself is not settled by tests.
5. **The SymPy oracle outside Lovelace.Symbolics.Tests.** I re-ran that one project with
   LOVELACE_REQUIRE_SYMPY=1 (section 4a: 1063/1063, 0 skipped); I did not re-run the other 14
   projects with the oracle required, because every skip the full-solution run reported was in
   Lovelace.Symbolics.Tests.
6. **Timings.** All wall-clock numbers above are single observations on a box shared with other
   agents (other worktrees were active), so they are indicative, not a measured CI cost.
