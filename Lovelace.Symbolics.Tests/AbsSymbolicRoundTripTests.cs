using Lovelace.Suite;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 23 — the round-trip closure for <c>abs</c>. The rewriter PRODUCES <c>abs(x)</c>
/// (<see cref="Lovelace.Symbolics.Simplify"/> builds <c>FunctionExpr(abs, x)</c>), so the
/// language must be able to build that very node from source. This file goes through the
/// language surface (SuiteEngine + SymbolicsPlugin), never the kernel API, because the defect
/// is at the language/plugin boundary.
/// </summary>
public class AbsSymbolicRoundTripTests
{
    private static SuiteEngine Engine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new Lovelace.Symbolics.SymbolicsPlugin());
        return engine;
    }

    /// <summary>1. abs(x) must build the same node the rewriter builds for sqrt(x^2) under x real.</summary>
    [Fact]
    public void Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node()
    {
        var engine = Engine();
        engine.Evaluate("x = symbol(\"x\", real)");

        var applied = engine.Evaluate("abs(x)");
        Assert.Equal(ValueKind.Symbolic, applied.Kind);
        Assert.Equal("abs(x)", Printing.PrettyPrint(applied.AsSymbolic()));

        // simplify_full returns the structured TransformResult; its "expression" field is the
        // rewritten expression (the runner envelope prints exactly that field).
        var rewritten = engine.Evaluate("simplify_full(sqrt(x^2)).expression");
        Assert.Equal(ValueKind.Symbolic, rewritten.Kind);
        Assert.Equal(Printing.CanonicalPrint(rewritten.AsSymbolic()),
                     Printing.CanonicalPrint(applied.AsSymbolic()));
        // structural equality with the node the rewrites themselves construct
        var expected = Exprs.Function(Exprs.Current.Function("abs"), Exprs.Current.Symbol("x"));
        Assert.Equal(expected, applied.AsSymbolic());
    }

    /// <summary>2. numeric abs is untouched.</summary>
    [Fact]
    public void Abs_Of_Integer_Still_Returns_The_Magnitude()
    {
        var engine = Engine();
        Assert.True(engine.Evaluate("abs(-3) == 3").AsBoolean());
        Assert.True(engine.Evaluate("abs(3) == 3").AsBoolean());
    }

    /// <summary>3. diff(abs(x), x) must not throw and must not invent a closed form.</summary>
    [Fact]
    public void Diff_Of_Abs_Symbolic_Does_Not_Throw()
    {
        var engine = Engine();
        engine.Evaluate("x = symbol(\"x\")");
        var d = engine.Evaluate("diff(abs(x), x)");
        Assert.Equal(ValueKind.Symbolic, d.Kind);
        // d|x|/dx IS sign(x) for x != 0 and is undefined at 0, so the honest answer is the
        // conditional the kernel's own Diff builds (Calculus/Diff.cs:85-93): the sign branch
        // guarded by x != 0, and the UNEVALUATED derivative in the remaining branch. Never a
        // bare sign(x) (wrong at 0) and never 1 or 0 (wrong everywhere).
        Assert.Equal("piecewise(sign(x) if x != 0, diff(abs(x), x))",
                     Printing.PrettyPrint(d.AsSymbolic()));
        Assert.Equal("(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))",
                     Printing.CanonicalPrint(d.AsSymbolic()));
    }

    /// <summary>4. abs(x) must survive substitution.</summary>
    [Fact]
    public void Subs_Abs_Symbolic_At_Negative_Value_Gives_Three()
    {
        var engine = Engine();
        engine.Evaluate("x = symbol(\"x\")");
        var v = engine.Evaluate("subs(abs(x), x, -3)");
        // the substituted value is the exact rational 3 (an exact Symbolic 3, printing as "3",
        // with no free symbols left) — not abs(-3), not abs(x), not a relation
        Assert.Equal(ValueKind.Symbolic, v.Kind);
        Assert.Equal("3", Printing.PrettyPrint(v.AsSymbolic()));
        Assert.Equal("(rat 3 1)", Printing.CanonicalPrint(v.AsSymbolic()));
        Assert.Empty(Printing.FreeSymbolNames(v.AsSymbolic()));
    }

    /// <summary>5. the rewriter result itself is unchanged.</summary>
    [Fact]
    public void Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X()
    {
        var engine = Engine();
        engine.Evaluate("x = symbol(\"x\", real)");
        var rewritten = engine.Evaluate("simplify_full(sqrt(x^2))");
        Assert.Equal("TransformResult", engine.Evaluate("type(simplify_full(sqrt(x^2)))").ToString());
        Assert.Equal("abs(x)", Printing.PrettyPrint(
            engine.Evaluate("simplify_full(sqrt(x^2)).expression").AsSymbolic()));
    }
}
