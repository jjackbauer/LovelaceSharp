using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// "The elimination found no solutions" and "the system is PROVABLY inconsistent" are different
/// claims, and only the second may be published as an empty solution set. These tests pin the
/// disposition on both sides of that line.
///
/// <para>Before this round the absence of solutions, with no note, was published as
/// <c>NoSolutions</c> with <c>complete: true</c>. The two nonlinear triangular shapes below (a
/// parabola meeting a line, and a circle meeting a line) each have TWO solutions — SymPy 1.14.0
/// returns both — and both were reported as a complete empty set. They now enumerate, so
/// <c>complete: true</c> is earned; the shapes the boundary still refuses report
/// <c>Unevaluated</c> with a stable diagnostic code instead of an empty set.</para>
/// </summary>
public class SystemSolveProofTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static Expr Eq(Expr left, Expr right) => Exprs.Relation(RelOp.Eq, left, right);

    private static readonly string Tolerance = "0." + new string('0', 24) + "1";

    /// <summary>The new proof API, read by NAME so that this file still COMPILES on a tree that
    /// predates it: the absence of the proof gate must fail on the assertion below, not on a
    /// missing member — the same construction-time pattern DiagnosticContractTests uses for
    /// ErrorCategory.</summary>
    private static bool ProvedEmpty(SystemSolveResult result)
    {
        var property = typeof(SystemSolveResult).GetProperty("ProvedEmpty");
        Assert.True(property is not null,
            "SystemSolveResult must publish ProvedEmpty: an empty result may be reported as NoSolutions only when the emptiness is PROVED");
        return (bool)property!.GetValue(result)!;
    }

    /// <summary>Every assignment of every solution satisfies every equation at 50 digits.</summary>
    private static void VerifyAssignments(
        SystemSolveResult result, IReadOnlyList<Expr> equations, Symbol[] vars, ExprContext ctx)
    {
        Assert.NotEmpty(result.Solutions);
        using var scope = Rl.WithPrecision(50, 20);
        Num tolerance = NumOps.FromReal(Rl.Parse(Tolerance, null));
        foreach (var sol in result.Solutions)
        {
            var bindings = new Dictionary<Symbol, Num>();
            foreach (var v in vars)
                bindings[v] = Evaluation.EvaluateToNum(sol.Assignment[v], ctx, new Dictionary<Symbol, Num>());
            foreach (var eq in equations)
            {
                var f = eq is RelationExpr r ? Exprs.Subtract(r.Left, r.Right) : eq;
                Num residual = Evaluation.EvaluateToNum(f, ctx, bindings);
                Assert.True(NumOps.Compare(NumOps.Abs(residual, ctx), tolerance) < 0,
                    $"assignment {string.Join(", ", vars.Select(v => v.Name + "=" + Printing.PrettyPrint(sol.Assignment[v])))} does not satisfy {Printing.PrettyPrint(f)}");
            }
        }
    }

    /// <summary>Compares one kernel value with the decimal SymPy produced for it.</summary>
    private static bool CloseTo(Expr value, string expected, ExprContext ctx)
    {
        using var scope = Rl.WithPrecision(60, 30);
        if (!Rl.TryParse(expected, null, out Rl? theirs))
            return false;
        Num mine = Evaluation.EvaluateToNum(value, ctx, new Dictionary<Symbol, Num>());
        Num difference = NumOps.Abs(NumOps.Subtract(NumOps.Re(mine, ctx), NumOps.FromReal(theirs!)), ctx);
        return NumOps.Compare(difference, NumOps.FromReal(Rl.Parse(Tolerance, null))) < 0;
    }

    // ------------------------------------------------------------------
    // The two reproduced shapes: they have solutions, so they must enumerate them
    // ------------------------------------------------------------------

    /// <summary>sympy.solve([x**2 + y - 1, x - y], [x, y], dict=True) returns x = y = -1/2 + sqrt(5)/2
    /// and x = y = -sqrt(5)/2 - 1/2.</summary>
    [Fact]
    public void NonlinearElimination_FindsBothSolutionsOfParabolaAndLine()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var eqs = new Expr[]
        {
            Eq(Exprs.Add(Exprs.Power(x, 2), y), Exprs.One),
            Eq(Exprs.Subtract(x, y), Exprs.Zero),
        };

        var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);

        Assert.Equal(2, result.Solutions.Count);
        Assert.False(ProvedEmpty(result));
        Assert.Equal(SolveStatus.Solved, result.Status);
        Assert.Equal(Completeness.Complete, result.Complete);
        Assert.Null(result.Note);
        VerifyAssignments(result, eqs, new[] { x, y }, ctx);   // includes x - y = 0
        var values = result.Solutions.Select(s => s.Assignment[x]).ToArray();
        Assert.Contains(values, v => CloseTo(v, "0.61803398874989484820458683436563811772", ctx));
        Assert.Contains(values, v => CloseTo(v, "-1.6180339887498948482045868343656381177", ctx));
    }

    /// <summary>sympy.solve([x**2 + y**2 - 1, y - x], [x, y], dict=True) returns
    /// x = y = +/-sqrt(2)/2.</summary>
    [Fact]
    public void NonlinearElimination_FindsBothSolutionsOfCircleAndLine()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var eqs = new Expr[]
        {
            Eq(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2)), Exprs.One),
            Eq(Exprs.Subtract(y, x), Exprs.Zero),
        };

        var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);

        Assert.Equal(2, result.Solutions.Count);
        Assert.False(ProvedEmpty(result));
        Assert.Equal(SolveStatus.Solved, result.Status);
        Assert.Null(result.Note);
        VerifyAssignments(result, eqs, new[] { x, y }, ctx);
        var values = result.Solutions.Select(s => s.Assignment[x]).ToArray();
        Assert.Contains(values, v => CloseTo(v, "0.70710678118654752440084436210484903928", ctx));
        Assert.Contains(values, v => CloseTo(v, "-0.7071067811865475244008443621048490393", ctx));
    }

    // ------------------------------------------------------------------
    // An absence of solutions is not a proof: the boundary must say so
    // ------------------------------------------------------------------

    /// <summary>Every system here has solutions the v1 elimination cannot enumerate (a free
    /// variable, an equation that constrains nothing, complex algebraic roots). Each one used to
    /// return an EMPTY solution set published as NoSolutions/complete.</summary>
    [Fact]
    public void Kernel_AbsenceOfSolutions_IsUnevaluatedWithAStableNote()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var cases = new (string Label, Expr[] Eqs, Symbol[] Vars)[]
        {
            ("a single equation in two variables leaves y free",
                new[] { Eq(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2)), Exprs.One) }, new[] { x, y }),
            ("an equation that does not mention y at all",
                new[] { Eq(Exprs.Power(x, 2), Exprs.One) }, new[] { x, y }),
            ("an equation that constrains nothing",
                new[] { Eq(Exprs.Subtract(x, x), Exprs.Zero) }, new[] { x }),
            ("complex algebraic roots of a quartic (RootOf is real-only in v1)",
                new[] { Eq(Exprs.Add(Exprs.Power(x, 4), Exprs.One), Exprs.Zero), Eq(Exprs.Subtract(y, x), Exprs.Zero) },
                new[] { x, y }),
        };

        foreach (var (label, eqs, vars) in cases)
        {
            var result = SystemSolvers.Solve(eqs, vars, ctx);

            Assert.Empty(result.Solutions);
            Assert.False(ProvedEmpty(result), $"{label}: nothing was proved about emptiness");
            Assert.Equal(SolveStatus.Unevaluated, result.Status);
            Assert.NotNull(result.Note);
            Assert.StartsWith("system-solve.", result.Note);
        }
    }

    // ------------------------------------------------------------------
    // A derived contradiction IS a proof: NoSolutions stays a complete answer
    // ------------------------------------------------------------------

    /// <summary>sympy.solve agrees that none of these has a solution. The univariate pair is
    /// included for the other proof shape: every root of x - 1 = 0 is refuted by x - 2 = 0.</summary>
    [Fact]
    public void Kernel_DerivedContradiction_IsNoSolutionsWithoutANote()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var cases = new (string Label, Expr[] Eqs, Symbol[] Vars)[]
        {
            ("two parallel lines",
                new[] { Eq(Exprs.Add(x, y), Exprs.One), Eq(Exprs.Add(x, y), Exprs.Integer(2)) }, new[] { x, y }),
            ("a hyperbola and its asymptote",
                new[] { Eq(Exprs.Multiply(x, y), Exprs.One), Eq(x, Exprs.Zero) }, new[] { x, y }),
            ("two incompatible quadratics",
                new[] { Eq(Exprs.Subtract(Exprs.Power(x, 2), Exprs.One), Exprs.Zero),
                        Eq(Exprs.Subtract(Exprs.Power(x, 2), Exprs.Integer(4)), Exprs.Zero) }, new[] { x, y }),
            ("two incompatible univariate equations",
                new[] { Eq(Exprs.Subtract(x, Exprs.One), Exprs.Zero),
                        Eq(Exprs.Subtract(x, Exprs.Integer(2)), Exprs.Zero) }, new[] { x }),
        };

        foreach (var (label, eqs, vars) in cases)
        {
            var result = SystemSolvers.Solve(eqs, vars, ctx);

            Assert.Empty(result.Solutions);
            Assert.True(ProvedEmpty(result), $"{label}: the elimination DERIVED a contradiction");
            Assert.Equal(SolveStatus.NoSolutions, result.Status);
            Assert.Null(result.Note);   // a proof needs no excuse: the diagnostics array stays empty
        }
    }

    /// <summary><c>solve_system_full([], [x])</c> used to escape as a CLR index exception. An
    /// empty system is the OPPOSITE of an empty solution set — every assignment is a solution — so
    /// it is refused by name.</summary>
    [Fact]
    public void Kernel_EmptyEquationList_IsATypedRefusal()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");

        var result = SystemSolvers.Solve(Array.Empty<Expr>(), new[] { x }, ctx);

        Assert.Empty(result.Solutions);
        Assert.False(ProvedEmpty(result));
        Assert.Equal(SolveStatus.Unevaluated, result.Status);
        Assert.NotNull(result.Note);
        Assert.StartsWith("system-solve.no-equations", result.Note);
    }

    // ------------------------------------------------------------------
    // The published record: the same distinction crosses the wire
    // ------------------------------------------------------------------

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value Field(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    /// <summary>An unproved empty result is Unevaluated/complete:false with a diagnostic whose code
    /// is stable and whose category is NOT an internal invariant failure.</summary>
    [Theory]
    [InlineData("solve_system_full([x^2 + y^2 == 1], [x, y])")]
    [InlineData("solve_system_full([x^2 == 1], [x, y])")]
    [InlineData("solve_system_full([x^4 + 1 == 0, y == x], [x, y])")]
    public void Wire_AbsenceOfSolutions_IsUnevaluatedWithADiagnostic(string script)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");

        var r = engine.Evaluate(script).AsRecord();

        Assert.Equal("SystemSolveResult", r.TypeName);
        Assert.Equal("Unevaluated", Field(r, "status").AsEnum().Name);
        Assert.False(Field(r, "complete").AsBoolean(), $"{script}: an unproved empty set is not a complete answer");
        Assert.Empty(Field(r, "solutions").AsVector());

        var diagnostics = Field(r, "diagnostics").AsVector();
        Assert.True(diagnostics.Count >= 1, $"{script}: Unevaluated must say WHY in structure");
        var diagnostic = diagnostics[0].AsRecord();
        Assert.Equal("Diagnostic", diagnostic.TypeName);
        Assert.Equal("system-solve.unevaluated", Field(diagnostic, "code").AsText());
        Assert.NotEqual("InternalInvariantFailure", Field(diagnostic, "category").AsEnum().Name);
        Assert.StartsWith("system-solve.", Field(diagnostic, "message").AsText());
    }

    /// <summary>A system whose emptiness is DERIVED keeps the frozen pairing: NoSolutions is a
    /// complete answer, and the diagnostics array is empty because nothing needs excusing.</summary>
    [Theory]
    [InlineData("solve_system_full([x + y == 1, x + y == 2], [x, y])")]
    [InlineData("solve_system_full([x*y == 1, x == 0], [x, y])")]
    [InlineData("solve_system_full([x^2 - 1 == 0, x^2 - 4 == 0], [x, y])")]
    public void Wire_DerivedContradiction_IsCompleteNoSolutions(string script)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");

        var r = engine.Evaluate(script).AsRecord();

        Assert.Equal("NoSolutions", Field(r, "status").AsEnum().Name);
        Assert.True(Field(r, "complete").AsBoolean(), $"{script}: a provably empty set is a complete answer");
        Assert.Empty(Field(r, "solutions").AsVector());
        Assert.Empty(Field(r, "diagnostics").AsVector());
    }

    /// <summary>The two nonlinear shapes that used to be published as a complete empty set now
    /// carry their two solutions across the wire.</summary>
    [Theory]
    [InlineData("solve_system_full([x^2 + y == 1, x - y == 0], [x, y])")]
    [InlineData("solve_system_full([x^2 + y^2 == 1, y == x], [x, y])")]
    public void Wire_NonlinearShapes_PublishTheirTwoSolutions(string script)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\"); y = symbol(\"y\")");

        var r = engine.Evaluate(script).AsRecord();

        Assert.Equal("Solved", Field(r, "status").AsEnum().Name);
        Assert.True(Field(r, "complete").AsBoolean());
        Assert.Equal(2, Field(r, "solutions").AsVector().Count);
        Assert.Empty(Field(r, "diagnostics").AsVector());
    }

    /// <summary>An empty equation list crosses the wire as a typed refusal record.</summary>
    [Fact]
    public void Wire_EmptyEquationList_IsARecord_NotAClrException()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        // before this round the evaluation did not return at all: an IndexOutOfRangeException
        // escaped the plugin as a TypeMismatch error envelope
        var r = engine.Evaluate("solve_system_full([], [x])").AsRecord();

        Assert.Equal("SystemSolveResult", r.TypeName);
        Assert.Equal("Unevaluated", Field(r, "status").AsEnum().Name);
        Assert.False(Field(r, "complete").AsBoolean());
        var diagnostics = Field(r, "diagnostics").AsVector();
        Assert.Single(diagnostics);
        Assert.Equal("system-solve.unevaluated", Field(diagnostics[0].AsRecord(), "code").AsText());
        Assert.StartsWith("system-solve.no-equations", Field(diagnostics[0].AsRecord(), "message").AsText());
    }

    // ------------------------------------------------------------------
    // The oracle: SymPy 1.14.0 is the ground truth for both directions
    // ------------------------------------------------------------------

    /// <summary>The enumerated solutions of both reproduced shapes are compared with
    /// <c>sympy.solve</c> itself — the count AND every coordinate at 30 digits — and SymPy is asked
    /// to confirm that the shapes on the other side of the boundary really have no solution.</summary>
    [RequiresSympyFact]
    public void NonlinearElimination_AndTheBoundary_AgreeWithSympySolve()
    {
        SympySession sympy = SympyOracle.Require();
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");

        var solvable = new (string Label, string SympySystem, Expr[] Eqs)[]
        {
            ("x**2 + y - 1, x - y", "[x**2 + y - 1, x - y]", new Expr[]
            {
                Eq(Exprs.Add(Exprs.Power(x, 2), y), Exprs.One),
                Eq(Exprs.Subtract(x, y), Exprs.Zero),
            }),
            ("x**2 + y**2 - 1, y - x", "[x**2 + y**2 - 1, y - x]", new Expr[]
            {
                Eq(Exprs.Add(Exprs.Power(x, 2), Exprs.Power(y, 2)), Exprs.One),
                Eq(Exprs.Subtract(y, x), Exprs.Zero),
            }),
        };

        foreach (var (label, sympySystem, eqs) in solvable)
        {
            var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);
            IReadOnlyList<string> theirs = sympy.EvalLines(
                SympyScript.Symbols(new[] { "x", "y" }) + "\n" +
                $"sols = sympy.solve({sympySystem}, [x, y], dict=True)\n" +
                "print(len(sols))\n" +
                "for s in sorted(sols, key=lambda s: (sympy.re(s[x]), sympy.re(s[y]))):\n" +
                "    print(sympy.sstr(sympy.N(sympy.re(s[x]), 30)))\n" +
                "    print(sympy.sstr(sympy.N(sympy.re(s[y]), 30)))");

            int theirCount = int.Parse(theirs[0]);
            Assert.True(theirCount == 2, $"{label}: the SymPy ground truth has {theirCount} solution(s), the corpus expects 2");
            Assert.Equal(theirCount, result.Solutions.Count);
            for (int i = 0; i < theirCount; i++)
            {
                string theirX = theirs[1 + (i * 2)];
                string theirY = theirs[2 + (i * 2)];
                Assert.True(
                    result.Solutions.Any(s => CloseTo(s.Assignment[x], theirX, ctx) && CloseTo(s.Assignment[y], theirY, ctx)),
                    $"{label}: sympy's solution (x = {theirX}, y = {theirY}) is missing from the kernel's {result.Solutions.Count} solution(s)");
            }
        }

        var inconsistent = new (string Label, string SympySystem, Expr[] Eqs)[]
        {
            ("x + y - 1, x + y - 2", "[x + y - 1, x + y - 2]", new Expr[]
            {
                Eq(Exprs.Add(x, y), Exprs.One),
                Eq(Exprs.Add(x, y), Exprs.Integer(2)),
            }),
            ("x*y - 1, x", "[x*y - 1, x]", new Expr[]
            {
                Eq(Exprs.Multiply(x, y), Exprs.One),
                Eq(x, Exprs.Zero),
            }),
        };

        foreach (var (label, sympySystem, eqs) in inconsistent)
        {
            IReadOnlyList<string> theirs = sympy.EvalLines(
                SympyScript.Symbols(new[] { "x", "y" }) + "\n" +
                $"print(len(sympy.solve({sympySystem}, [x, y], dict=True)))");
            Assert.True(theirs[0] == "0", $"{label}: the SymPy ground truth must have no solution, it returned {theirs[0]}");

            var result = SystemSolvers.Solve(eqs, new[] { x, y }, ctx);
            Assert.True(ProvedEmpty(result), $"{label}: the kernel must PROVE the emptiness SymPy reports");
            Assert.Equal(SolveStatus.NoSolutions, result.Status);
        }
    }
}
