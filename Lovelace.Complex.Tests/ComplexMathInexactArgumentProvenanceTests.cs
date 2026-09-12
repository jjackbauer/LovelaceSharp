using Lovelace.Complex;
using Lovelace.Real;
using Lovelace.Symbolics;
using Xunit;

using Cplx = Lovelace.Complex.Complex;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// THE ROUTE OF THE ARGUMENT IS THE ROUTE OF THE RESULT: an argument whose
/// <see cref="Rl.IsExact"/> is false must not come back out of <see cref="ComplexMath"/> carrying
/// <c>IsExact = true</c>.
///
/// <para>THE DEFECT THIS FILE PINS (cycle 6, P-B1). The symbolic evaluator routes the real tier
/// through <see cref="ComplexMath.Sin"/>/<see cref="ComplexMath.Cos"/>
/// (<c>Lovelace.Symbolics/Evaluation.cs:335-337</c>), whose <c>SinCosAtPrecision</c> reduces the
/// argument modulo a π of the ARGUMENT'S OWN SCALE. <c>pi(30)</c> therefore reduces to exactly
/// zero, the series runs on the exact zero, and the reduction's table value is returned as an exact
/// <see cref="Rl"/> — while the same expression's symbolic node reports <c>exact:false</c>. On the
/// wire that publishes a false claim of exactness:
/// <c>evalf(cos(pi(30)), 40)</c> answered
/// <c>{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}</c> while
/// <c>cos(pi(30))</c> answered <c>{"pretty":"-1","exact":false}</c> and mpmath at 110 dps gives
/// <c>-0.999…87355374…</c> — the truncation is real, the exactness claim was not.</para>
///
/// <para>WHAT CHANGES IS THE FLAG, NOT THE VALUE. The exactly-zero reduction still lands on the same
/// numbers — <c>Sin(pi) == 0</c> and <c>Cos(pi) == -1</c> are pinned by
/// <c>ComplexMathProvenanceTests.cs:83-84</c> and stay pinned here — but a π that is itself a
/// truncation cannot make them exact. The <c>IsZero</c> shortcut is the same rule: <c>Sin(0) == 0</c>
/// and <c>Cos(0) == 1</c> stay EXACT for an exact zero and only for an exact zero.</para>
///
/// <para>Every negative assertion here is paired with its exact control: <c>Sin(0)</c>/<c>Cos(0)</c>/
/// <c>Tan(0)</c> at an exact zero, and <c>Sin(0 + 0i)</c>, must still report exact, so clearing the
/// flag unconditionally does not satisfy this file.</para>
/// </summary>
public class ComplexMathInexactArgumentProvenanceTests
{
    private static readonly Dictionary<Symbol, Num> NoBindings = new();

    /// <summary>The three scales the defect was measured at (30 is the scale the wire folds at).</summary>
    public static TheoryData<long> Scales => new() { 30L, 60L, 100L };

    /// <summary>
    /// The exactly-zero reduction: at ambient precision <c>n</c> the reduction subtracts the
    /// argument's own π, so the residual IS zero and the result is the table value. The argument is
    /// a truncation (<c>Rl.PiTo(n)</c>, premise asserted) and the result must say so — at every
    /// scale, for all three functions. The VALUES are asserted too, unchanged: this round moves the
    /// flag only.
    /// </summary>
    [Theory]
    [MemberData(nameof(Scales))]
    public void InexactPiAtItsOwnScale_ProducesInexactResults(long scale)
    {
        using var scope = Rl.WithPrecision(scale, scale);
        Rl pi = Rl.PiTo(scale);
        Assert.False(pi.IsExact, "Rl.PiTo(" + scale + ") is a truncation and must be inexact");

        Rl cos = ComplexMath.Cos(pi);
        Rl sin = ComplexMath.Sin(pi);
        Rl tan = ComplexMath.Tan(pi);

        // the values the exactly-zero reduction lands on: unchanged by this round
        Assert.Equal(-Rl.One, cos);
        Assert.Equal(Rl.Zero, sin);
        Assert.Equal(Rl.Zero, tan);

        Assert.False(cos.IsExact, "cos(pi(" + scale + ")) = " + cos + " claims exactness");
        Assert.False(sin.IsExact, "sin(pi(" + scale + ")) = " + sin + " claims exactness");
        Assert.False(tan.IsExact, "tan(pi(" + scale + ")) = " + tan + " claims exactness");
    }

    /// <summary>
    /// The same reduction reached through the ambient constant — <c>Rl.Pi</c> is a truncation at
    /// every precision, and <c>Cos(Rl.Pi) == -1</c> / <c>Sin(Rl.Pi) == 0</c> stay the pinned values
    /// while the flag stops claiming they are exact.
    /// </summary>
    [Fact]
    public void ExactlyZeroReduction_OfAnInexactAmbientPi_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        Assert.False(Rl.Pi.IsExact, "Rl.Pi is a truncation and must be inexact");

        Rl cos = ComplexMath.Cos(Rl.Pi);
        Rl sin = ComplexMath.Sin(Rl.Pi);
        Rl tan = ComplexMath.Tan(Rl.Pi);

        Assert.Equal(-Rl.One, cos);
        Assert.Equal(Rl.Zero, sin);
        Assert.Equal(Rl.Zero, tan);

        Assert.False(cos.IsExact, "cos(pi) = " + cos + " claims exactness");
        Assert.False(sin.IsExact, "sin(pi) = " + sin + " claims exactness");
        Assert.False(tan.IsExact, "tan(pi) = " + tan + " claims exactness");
    }

    /// <summary>
    /// An inexact argument that is NOT the ambient π still has to carry its route out: the residual
    /// here is a real one (the argument measures against a longer π), so the results were already
    /// inexact — this is the guard that the repair does not reintroduce exactness anywhere.
    /// </summary>
    [Fact]
    public void InexactPiAtAnotherScale_ProducesInexactResults()
    {
        using var scope = Rl.WithPrecision(100, 40);
        Rl pi30 = Rl.PiTo(30);
        Assert.False(pi30.IsExact, "Rl.PiTo(30) is a truncation and must be inexact");

        Assert.False(ComplexMath.Cos(pi30).IsExact, "cos(pi(30)) at ambient 100 claims exactness");
        Assert.False(ComplexMath.Sin(pi30).IsExact, "sin(pi(30)) at ambient 100 claims exactness");
        Assert.False(ComplexMath.Tan(pi30).IsExact, "tan(pi(30)) at ambient 100 claims exactness");
    }

    /// <summary>The control direction of the IsZero shortcut: an EXACT zero stays exact.</summary>
    [Fact]
    public void ExactZeroArgument_KeepsItsExactResults()
    {
        using var scope = Rl.WithPrecision(40, 40);

        Assert.Equal(Rl.Zero, ComplexMath.Sin(Rl.Zero));
        Assert.Equal(Rl.One, ComplexMath.Cos(Rl.Zero));
        Assert.Equal(Rl.Zero, ComplexMath.Tan(Rl.Zero));

        Assert.True(ComplexMath.Sin(Rl.Zero).IsExact, "sin(0) lost its exactness");
        Assert.True(ComplexMath.Cos(Rl.Zero).IsExact, "cos(0) lost its exactness");
        Assert.True(ComplexMath.Tan(Rl.Zero).IsExact, "tan(0) lost its exactness");
    }

    /// <summary>
    /// An INEXACT zero is still an inexact argument: the shortcut may not launder its route. The
    /// values stay 0/1/0 — only the claim goes.
    /// </summary>
    [Fact]
    public void InexactZeroArgument_DoesNotClaimExactness()
    {
        using var scope = Rl.WithPrecision(40, 40);
        Rl inexactZero = Rl.AsInexact(Rl.Zero);
        Assert.False(inexactZero.IsExact, "the premise: an inexact zero is inexact");

        Rl sin = ComplexMath.Sin(inexactZero);
        Rl cos = ComplexMath.Cos(inexactZero);
        Rl tan = ComplexMath.Tan(inexactZero);

        Assert.Equal(Rl.Zero, sin);
        Assert.Equal(Rl.One, cos);
        Assert.Equal(Rl.Zero, tan);

        Assert.False(sin.IsExact, "sin(inexact zero) = " + sin + " claims exactness");
        Assert.False(cos.IsExact, "cos(inexact zero) = " + cos + " claims exactness");
        Assert.False(tan.IsExact, "tan(inexact zero) = " + tan + " claims exactness");
    }

    /// <summary>
    /// THE WIRE'S OWN PAIRING: the symbolic node and the numeric value of the SAME expression must
    /// agree. <c>cos(pi(30))</c>'s symbolic node is inexact (it is a function of a truncated
    /// constant), and the numeric value <see cref="Evaluation.EvaluateToNum"/> reads back for that
    /// node — the very value <c>evalf</c> publishes — used to claim exactness instead.
    /// </summary>
    [Fact]
    public void SymbolicNodeAndNumericResult_AgreeForTheSameExpression()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;

        using var scope = Rl.WithPrecision(30, 30);
        Rl pi30 = Rl.PiTo(30);

        var literal = Exprs.Real(RealLiteral.FromRealExact(pi30));
        Assert.False(literal.IsExact, "the symbolic literal for pi(30) must be inexact");

        var node = Exprs.Function(ctx.Function("cos"), literal);
        Assert.False(node.IsExact, "the symbolic node cos(pi(30)) must be inexact");

        Rl numeric = Assert.IsType<NumReal>(Evaluation.EvaluateToNum(node, ctx, NoBindings)).V;

        Assert.Equal(ComplexMath.Cos(pi30), numeric);
        Assert.Equal(node.IsExact, numeric.IsExact);
        Assert.False(numeric.IsExact,
            "the numeric value of cos(pi(30)) = " + numeric + " claims exactness the symbolic node denies");
    }

    /// <summary>
    /// The same pairing for sine, whose value here is 0: the flag may not be read off the value's
    /// shape (an exact-looking 0 out of an inexact argument is still an approximation's zero).
    /// </summary>
    [Fact]
    public void SymbolicNodeAndNumericResult_AgreeForSine()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;

        using var scope = Rl.WithPrecision(30, 30);
        Rl pi30 = Rl.PiTo(30);

        var node = Exprs.Function(ctx.Function("sin"), Exprs.Real(RealLiteral.FromRealExact(pi30)));
        Assert.False(node.IsExact, "the symbolic node sin(pi(30)) must be inexact");

        Rl numeric = Assert.IsType<NumReal>(Evaluation.EvaluateToNum(node, ctx, NoBindings)).V;
        Assert.Equal(ComplexMath.Sin(pi30), numeric);
        Assert.Equal(node.IsExact, numeric.IsExact);
        Assert.False(numeric.IsExact, "the numeric value of sin(pi(30)) = " + numeric + " claims exactness");
    }

    /// <summary>
    /// The COMPLEX overloads: both components of sin(a+bi)/cos(a+bi) depend on both parts of the
    /// argument, so an inexact part must leave neither component claiming exactness. The leak is
    /// reachable where the hyperbolic factor is an exact zero (the <c>exp(0)</c> shortcut inside
    /// <c>RealSinh</c>/<c>RealCosh</c>), which is exactly the <c>0 + 0i</c> shape below.
    /// </summary>
    [Fact]
    public void ComplexTrigOfAnInexactZeroPart_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        var z = new Cplx(Rl.Zero, Rl.AsInexact(Rl.Zero));
        Assert.False(z.Im.IsExact, "the premise: the imaginary part is an inexact zero");

        AssertComplexInexact("sin(0 + 0i)", ComplexMath.Sin(z));
        AssertComplexInexact("cos(0 + 0i)", ComplexMath.Cos(z));
        AssertComplexInexact("tan(0 + 0i)", ComplexMath.Tan(z));
        AssertComplexInexact("sinh(0 + 0i)", ComplexMath.Sinh(z));
        AssertComplexInexact("cosh(0 + 0i)", ComplexMath.Cosh(z));
    }

    /// <summary>The control direction: an EXACT complex zero keeps its exact results.</summary>
    [Fact]
    public void ComplexTrigOfAnExactZero_KeepsItsExactness()
    {
        using var scope = Rl.WithPrecision(40, 40);
        var z = new Cplx(Rl.Zero, Rl.Zero);

        var sin = ComplexMath.Sin(z);
        Assert.Equal(Rl.Zero, sin.Re);
        Assert.Equal(Rl.Zero, sin.Im);
        Assert.True(sin.Re.IsExact && sin.Im.IsExact, "sin(0 + 0i) must stay exact");

        var cos = ComplexMath.Cos(z);
        Assert.Equal(Rl.One, cos.Re);
        Assert.Equal(Rl.Zero, cos.Im);
        Assert.True(cos.Re.IsExact && cos.Im.IsExact, "cos(0 + 0i) must stay exact");
    }

    /// <summary>
    /// THE SAME LEAK, MEASURED IN THE INVERSE ENTRY POINTS. <c>DivideTruncate</c> answers
    /// <c>0 / anything</c> with an EXACT zero, so an argument that only approximates zero reaches
    /// the series as an exact zero and atan/asin hand back an exact 0. The pre-fix measurements were
    /// <c>Atan(inexact zero) = 0 exact</c>, <c>Atan2(inexact zero, 1) = 0 exact</c> and
    /// <c>AsinReal(inexact zero) = 0 exact</c>; the exact controls beside them are untouched.
    /// </summary>
    [Fact]
    public void InverseTrigOfAnInexactZeroArgument_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        Rl inexactZero = Rl.AsInexact(Rl.Zero);

        Assert.Equal(Rl.Zero, ComplexMath.Atan(inexactZero));
        Assert.Equal(Rl.Zero, ComplexMath.Atan2(inexactZero, Rl.One));
        Assert.Equal(Rl.Zero, ComplexMath.AsinReal(inexactZero));

        Assert.False(ComplexMath.Atan(inexactZero).IsExact, "atan(inexact zero) claims exactness");
        Assert.False(ComplexMath.Atan2(inexactZero, Rl.One).IsExact, "atan2(inexact zero, 1) claims exactness");
        Assert.False(ComplexMath.AsinReal(inexactZero).IsExact, "asin(inexact zero) claims exactness");

        // the exact controls: atan(0) = 0 and asin(0) = 0 ARE exact, and stay so
        Assert.Equal(Rl.Zero, ComplexMath.Atan(Rl.Zero));
        Assert.Equal(Rl.Zero, ComplexMath.Atan2(Rl.Zero, Rl.One));
        Assert.Equal(Rl.Zero, ComplexMath.AsinReal(Rl.Zero));
        Assert.True(ComplexMath.Atan(Rl.Zero).IsExact, "atan(0) lost its exactness");
        Assert.True(ComplexMath.Atan2(Rl.Zero, Rl.One).IsExact, "atan2(0, 1) lost its exactness");
        Assert.True(ComplexMath.AsinReal(Rl.Zero).IsExact, "asin(0) lost its exactness");
    }

    private static void AssertComplexInexact(string label, Cplx value)
    {
        Assert.False(value.Re.IsExact, label + " = (" + value.Re + ", " + value.Im + "): the real part claims exactness");
        Assert.False(value.Im.IsExact, label + " = (" + value.Re + ", " + value.Im + "): the imaginary part claims exactness");
    }
}
