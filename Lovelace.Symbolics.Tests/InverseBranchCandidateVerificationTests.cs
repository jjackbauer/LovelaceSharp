using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// An inverse branch is a NECESSARY-condition generator, not an equivalence: squaring the branch
/// sqrt(x) = -2 yields the candidate x = 4, and sqrt(4) = +2, so 4 is NOT a solution of the
/// original equation. Before this file's change the candidate entered a Solved (complete) set
/// unchecked at Lovelace.Symbolics/Solvers/Solve.cs:667-686.
///
/// <para>The verification point this file pins is the inverse-branch boundary: every candidate the
/// branches produce must satisfy the ORIGINAL equation <c>h - c</c> (the expression
/// <c>SolveInverse</c> was asked to solve) before it is represented, using the same residual
/// predicate the system path already uses (<c>SatisfiesAll</c>, Solve.cs:1002-1030). A candidate
/// whose residual is not numerically decidable is KEPT (the existing conservative policy); a set
/// whose every candidate is rejected is not reported as a complete answer.</para>
/// </summary>
public class InverseBranchCandidateVerificationTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value F(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    private static ExprContext NewContext(out Symbol x)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        x = ctx.Symbol("x");
        return ctx;
    }

    private static Expr Sqrt(Expr e) => Exprs.Power(e, Exprs.Rational(1, 2));

    private static Expr[] ValuesOf(RecordValue record) =>
        F(record, "solutions").AsVector()
            .Select(v => F(v.AsRecord(), "value").AsSymbolic())
            .ToArray();

    private static bool IsNumber(Expr e, long expected) =>
        Evaluation.ConstantToNum(e) is { } n && NumOps.IsZero(NumOps.Subtract(n, NumOps.FromLong(expected)));

    /// <summary>The residual of the ORIGINAL equation at a candidate: substitute, then demand
    /// exact zero — symbolically where the kernel folds it, numerically at high precision
    /// otherwise. This is the same three-step residual the kernel's own predicate uses
    /// (Solve.cs:1002-1030), so a genuine root that only cancels numerically (exp(log 3) = 3) is
    /// not mistaken for a non-solution.</summary>
    private static void AssertSatisfies(ExprContext ctx, Symbol x, Expr equation, Expr value, string label)
    {
        Expr at = Evaluation.Substitute(equation, ctx, new Dictionary<Symbol, Expr> { [x] = value });
        string message =
            label + ": the solver represented " + Printing.PrettyPrint(value) + " as a solution, but substituting it into " +
            Printing.CanonicalPrint(equation) + " gives " + Printing.CanonicalPrint(at) + ", not 0";
        if (Evaluation.ConstantToNum(at) is { } folded)
        {
            Assert.True(NumOps.IsZero(folded), message);
            return;
        }
        Expr expanded = Algebra.Expand(at, ctx);
        if (expanded is RationalConstantExpr rc)
        {
            Assert.True(rc.Value.IsZero, message);
            return;
        }
        using var scope = Rl.WithPrecision(40, 20);
        var tolerance = NumOps.FromReal(Rl.Parse("0." + new string('0', 24) + "1", null));
        Num residual = Evaluation.EvaluateToNum(at, ctx, new Dictionary<Symbol, Num>());
        Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx), tolerance) < 0, message);
    }

    // ------------------------------------------------------------------
    // The defect: sqrt(x) + 2 == 0 has NO solution (the principal square
    // root is never -2), so no complete answer may be claimed for it.
    // ------------------------------------------------------------------

    /// <summary>The reported repro, at the language boundary the audit used: before the change this
    /// was status Solved / complete true / represented_count 1 with the value 4.</summary>
    [Fact]
    public void SolveFull_SqrtPlusTwo_DoesNotClaimACompleteSolutionSet()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate("solve_full(sqrt(x) + 2 == 0, x)").AsRecord();
        var status = F(r, "status").AsEnum().Name;

        Assert.NotEqual("Solved", status);
        Assert.False(F(r, "complete").AsBoolean(),
            "every candidate fails substitution, so no complete solution set may be claimed");
        Assert.Equal("Unknown", F(r, "completeness").AsEnum().Name);
        Assert.Equal("0", F(r, "represented_count").AsInteger().ToString());

        // the honest non-answer is machine-readable, not just a prose note
        var diagnostics = F(r, "diagnostics").AsVector();
        Assert.True(diagnostics.Count >= 1, "an Unevaluated solve carries a Diagnostic");
        var diagnostic = diagnostics[0].AsRecord();
        Assert.Equal("solve.unevaluated", F(diagnostic, "code").AsText());
        Assert.False(string.IsNullOrWhiteSpace(F(diagnostic, "message").AsText()),
            "the diagnostic message states why no solution set is claimed");
    }

    [Fact]
    public void Kernel_SqrtPlusTwo_IsNotSolvedAndRepresentsNothing()
    {
        ExprContext ctx = NewContext(out Symbol x);
        Expr equation = Exprs.Add(Sqrt(x), 2);

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);

        Assert.Equal(SolveStatus.Unevaluated, set.Status);   // the declared non-answer (see OracleCorpus.Solve)
        Assert.NotEqual(Completeness.Complete, set.Complete);
        Assert.Empty(set.Solutions);
        Assert.Empty(set.Families);
    }

    /// <summary>The candidate the branch produced (x = (-2)^2 = 4) is a NON-solution, so it must
    /// not survive anywhere in the result — and the case is only meaningful because it is one.</summary>
    [Fact]
    public void TheExtraneousCandidateFour_IsNotRepresented()
    {
        ExprContext ctx = NewContext(out Symbol x);
        Expr equation = Exprs.Add(Sqrt(x), 2);

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);
        foreach (Solution s in set.Solutions)
            Assert.False(IsNumber(s.Value, 4),
                "x = 4 is an extraneous candidate of sqrt(x) = -2 and must not be represented");

        // and the candidate really is a non-solution: sqrt(4) + 2 = 4
        Expr at = Evaluation.Substitute(equation, ctx, new Dictionary<Symbol, Expr> { [x] = Exprs.Integer(4) });
        Assert.True(Evaluation.ConstantToNum(at) is { } n && !NumOps.IsZero(n),
            "the case is only meaningful if x = 4 fails the original equation");
    }

    // ------------------------------------------------------------------
    // Precision: the gate must drop the extraneous candidate WITHOUT
    // dropping the genuine root of the sibling equation.
    // ------------------------------------------------------------------

    [Fact]
    public void SqrtMinusTwo_KeepsItsGenuineRoot()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate("solve_full(sqrt(x) - 2 == 0, x)").AsRecord();
        Assert.Equal("Solved", F(r, "status").AsEnum().Name);
        Assert.True(F(r, "complete").AsBoolean());
        Assert.Equal("1", F(r, "represented_count").AsInteger().ToString());

        Expr[] values = ValuesOf(r);
        Assert.Single(values);
        Assert.True(IsNumber(values[0], 4),
            "sqrt(x) - 2 = 0 has the genuine root x = 4; the gate must keep it, got " + Printing.PrettyPrint(values[0]));
    }

    /// <summary>The invariant, over every structure that reaches the inverse path plus the
    /// polynomial path: EVERY solution of a Solved set satisfies the original equation, and a
    /// genuine root is never dropped (the expected count pins the no-under-claim half).
    /// <para>Cycle 6: <c>exp(x) = 3</c> over the complex field is the INFINITE family
    /// <c>log(3) + 2*pi*k*i</c>, so its roots are represented by the family rather than by a finite
    /// list — the case is flagged <c>family</c> and every member is verified instead of a count.</para>
    /// </summary>
    [Theory]
    [InlineData("sqrt(x) - 2", 1, false)]
    [InlineData("cbrt(x) - 3", 1, false)]
    [InlineData("exp(x) - 3", 1, true)]
    [InlineData("log(x) - 2", 1, false)]
    [InlineData("sqrt(x^2 + 1) - 2", 2, false)]
    [InlineData("x^2 - 4", 2, false)]
    [InlineData("x^3 - 8", 3, false)]
    public void EveryRepresentedSolutionSatisfiesTheOriginalEquation(string label, int expectedCount, bool family)
    {
        ExprContext ctx = NewContext(out Symbol x);
        Expr equation = label switch
        {
            "sqrt(x) - 2" => Exprs.Add(Sqrt(x), -2),
            "cbrt(x) - 3" => Exprs.Add(Exprs.Power(x, Exprs.Rational(1, 3)), -3),
            "exp(x) - 3" => Exprs.Add(Exprs.Function(ctx.Function("exp"), x), -3),
            "log(x) - 2" => Exprs.Add(Exprs.Function(ctx.Function("log"), x), -2),
            "sqrt(x^2 + 1) - 2" => Exprs.Add(Sqrt(Exprs.Add(Exprs.Power(x, 2), 1)), -2),
            "x^2 - 4" => Exprs.Add(Exprs.Power(x, 2), -4),
            _ => Exprs.Add(Exprs.Power(x, 3), -8),
        };

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);

        Assert.True(set.Status == SolveStatus.Solved,
            label + ": a genuine root must not be dropped (status " + set.Status + ", note " + (set.Note ?? "none") + ")");
        if (family)
        {
            // the roots are the family's members, and there are infinitely many of them
            Assert.Empty(set.Solutions);
            SolutionFamily only = Assert.Single(set.Families);
            foreach (int k in new[] { -2, 0, 1, 3 })
            {
                Expr member = Evaluation.Substitute(only.Template, ctx,
                    new Dictionary<Symbol, Expr> { [only.Parameter] = Exprs.Integer(k) });
                AssertSatisfies(ctx, x, equation, member, label + " family member k = " + k);
            }
            return;
        }
        Assert.Equal(expectedCount, set.Solutions.Count);
        foreach (Solution s in set.Solutions)
            AssertSatisfies(ctx, x, equation, s.Value, label);
    }

    // ------------------------------------------------------------------
    // The other inverse branches the gate must not break: u^n = c generates
    // one candidate per root of unity, and an undecidable residual keeps
    // its candidate (the conservative policy).
    // ------------------------------------------------------------------

    /// <summary>The integer-exponent branch (u^n = c, n = 2) emits ONE candidate per root of unity.
    /// Both are genuine here, so both must survive the gate.</summary>
    [Fact]
    public void UnityRootBranches_AreKeptWhenTheyAreGenuine()
    {
        ExprContext ctx = NewContext(out Symbol x);
        // (sqrt(x) - 2)^2 = 1  =>  sqrt(x) - 2 = ±1  =>  x = 9 or x = 1
        Expr equation = Exprs.Add(Exprs.Power(Exprs.Subtract(Sqrt(x), 2), Exprs.Rational(2, 1)), -1);

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);

        Assert.True(set.Status == SolveStatus.Solved,
            "both unity-root branches are genuine solutions (status " + set.Status + ", note " + (set.Note ?? "none") + ")");
        Assert.Equal(2, set.Solutions.Count);
        foreach (Solution s in set.Solutions)
            AssertSatisfies(ctx, x, equation, s.Value, "(sqrt(x) - 2)^2 - 1");
    }

    /// <summary>The same branch where the only candidate is extraneous: (sqrt(x) + 1)^2 = 0 needs
    /// sqrt(x) = -1, which the principal root never is.</summary>
    [Fact]
    public void UnityRootBranchWithOnlyExtraneousCandidates_IsNotACompleteClaim()
    {
        ExprContext ctx = NewContext(out Symbol x);
        Expr equation = Exprs.Power(Exprs.Add(Sqrt(x), 1), Exprs.Rational(2, 1));

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);

        Assert.NotEqual(SolveStatus.Solved, set.Status);
        Assert.Empty(set.Solutions);
    }

    /// <summary>A residual that is NOT numerically decidable keeps its candidate: the gate must not
    /// turn an unverifiable candidate into a dropped one (the conservative policy Solve.cs:279
    /// documents for the denominator-pole filter).
    /// <para>Cycle 6: <c>exp(x) = y</c> over the complex field is the infinite family
    /// <c>log(y) + 2*pi*k*i</c>, so the candidate survives as the family's principal member
    /// (k = 0) instead of as a lone finite solution. The point of this test is unchanged: the
    /// residual <c>exp(log(y)) - y</c> cannot be decided numerically for a free y, and the candidate
    /// is KEPT rather than dropped.</para>
    /// </summary>
    [Fact]
    public void CandidateWithAnUndecidableResidual_IsKept()
    {
        ExprContext ctx = NewContext(out Symbol x);
        Symbol y = ctx.Symbol("y");
        Expr equation = Exprs.Subtract(Exprs.Function(ctx.Function("exp"), x), y);

        SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, equation, Exprs.Zero), x, ctx);

        Assert.True(set.Status == SolveStatus.Solved,
            "exp(x) = y is solvable for a free y; an undecidable residual must not drop it (status " +
            set.Status + ", note " + (set.Note ?? "none") + ")");
        SolutionFamily family = Assert.Single(set.Families);
        Assert.Empty(set.Solutions);
        Expr principal = Evaluation.Substitute(family.Template, ctx,
            new Dictionary<Symbol, Expr> { [family.Parameter] = Exprs.Zero });
        Assert.Equal("log(y)", Printing.PrettyPrint(principal));
        using var scope = Rl.WithPrecision(40, 20);
        Expr member = Evaluation.Substitute(family.Template, ctx, new Dictionary<Symbol, Expr>
        {
            [family.Parameter] = Exprs.Integer(2),
            [y] = Exprs.Integer(7),
        });
        Num residual = Evaluation.EvaluateToNum(
            Exprs.Subtract(Exprs.Function(ctx.Function("exp"), member), Exprs.Integer(7)),
            ctx, new Dictionary<Symbol, Num>());
        Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx),
            NumOps.FromReal(Rl.Parse("0." + new string('0', 24) + "1", null))) < 0,
            "the kept candidate's family must actually solve exp(x) = y");
    }

    /// <summary>No under-claim regression on the polynomial path.</summary>
    [Fact]
    public void PolynomialSolve_StaysSolvedAndCompleteWithTwoSolutions()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var r = engine.Evaluate("solve_full(x^2 - 4 == 0, x)").AsRecord();
        Assert.Equal("Solved", F(r, "status").AsEnum().Name);
        Assert.True(F(r, "complete").AsBoolean());
        Assert.Equal("Complete", F(r, "completeness").AsEnum().Name);
        Assert.Equal("2", F(r, "represented_count").AsInteger().ToString());
    }
}
