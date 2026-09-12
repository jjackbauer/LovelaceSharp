using Lovelace.Complex;
using Lovelace.Real;
using Xunit;

using Cplx = Lovelace.Complex.Complex;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Complex.Tests;

/// <summary>
/// THE ROUTE OF THE ARGUMENT IS THE ROUTE OF THE RESULT, part two (cycle 6, row 11): the two entry
/// points the trigonometric round (<c>78c19d8</c>) reported and left open.
///
/// <para>THE FIRST IS <see cref="ComplexMath.Exp(Rl)"/>. Its <c>IsZero</c> shortcut answers
/// <see cref="Rl.One"/> without consulting the argument's provenance, so an argument that only
/// APPROXIMATES zero — <c>pi(30) − pi(30)</c>, the exactly-zero reduction of a truncated π, the
/// imaginary part of a truncated complex — came back out as an exact 1. The wire shows it:
/// <c>evalf(exp(pi(30)-pi(30)), 40)</c> published <c>{"value":"1","exact":true,"numerator":"1",
/// "denominator":"1"}</c> although <c>pi(30)-pi(30)</c> is <c>{"value":"0","exact":false}</c>.
/// The complex overload inherits it: <c>e = Exp(z.Re)</c> was exact, and with an exact imaginary
/// part (the <c>cos(0)/sin(0)</c> values) both components of the result were exact too.</para>
///
/// <para>THE SECOND IS <see cref="ComplexMath.Sqrt(Cplx)"/>, which carried no provenance at all.
/// Its two components are built by <c>Rl.Sqrt</c> of an argument that is an EXACT zero whenever the
/// magnitude cancels the real part — <c>sqrt(0) = 0</c> is exact by construction — so a truncated
/// argument answered with an exact zero for a part that is a truncation's zero
/// (<c>Sqrt(inexact 0 + 0i) = (0, 0)</c> with both parts exact, <c>Sqrt(inexact 4 + 0i) = (2, 0)</c>
/// with the imaginary part exact).</para>
///
/// <para>WHAT CHANGES IS THE FLAG, NOT THE VALUE. Every case asserts the values it asserted before
/// (<c>exp(0) = 1</c>, <c>sqrt(4) = 2</c>, <c>sqrt(0) = 0</c>) and the exact controls beside them:
/// an EXACT argument keeps its exact result, so clearing the flag unconditionally does not satisfy
/// this file.</para>
/// </summary>
public class ComplexMathExpSqrtInexactArgumentTests
{
    private static void AssertComplexInexact(string label, Cplx value)
    {
        Assert.False(value.Re.IsExact, label + " = (" + value.Re + ", " + value.Im + "): the real part claims exactness");
        Assert.False(value.Im.IsExact, label + " = (" + value.Re + ", " + value.Im + "): the imaginary part claims exactness");
    }

    // ------------------------------------------------------------------
    // Exp
    // ------------------------------------------------------------------

    /// <summary>The <c>IsZero</c> shortcut of the real exponential may not launder its argument's
    /// route: an inexact zero is an approximation's zero. The value stays 1; the exact control
    /// <c>exp(0)</c> beside it stays exact.</summary>
    [Fact]
    public void ExpOfAnInexactZero_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        Rl inexactZero = Rl.AsInexact(Rl.Zero);
        Assert.False(inexactZero.IsExact, "the premise: an inexact zero is inexact");

        Rl exp = ComplexMath.Exp(inexactZero);
        Assert.Equal(Rl.One, exp);
        Assert.False(exp.IsExact, "exp(inexact zero) = " + exp + " claims exactness");

        // the exact control, unchanged
        Assert.Equal(Rl.One, ComplexMath.Exp(Rl.Zero));
        Assert.True(ComplexMath.Exp(Rl.Zero).IsExact, "exp(0) lost its exactness");
    }

    /// <summary>
    /// The complex overload, over the three shapes an inexact zero can take: an inexact real part
    /// (the leak — <c>Exp(z.Re)</c> answered an exact 1), an inexact imaginary part (routed through
    /// the trigonometric pair fixed by <c>78c19d8</c> and already inexact), and both. The value is
    /// <c>e^0·(cos 0 + i·sin 0) = 1 + 0i</c> in every case.
    /// </summary>
    [Theory]
    [InlineData(true, false)]
    [InlineData(false, true)]
    [InlineData(true, true)]
    public void ComplexExpOfAnInexactPart_IsNotExact(bool realInexact, bool imaginaryInexact)
    {
        using var scope = Rl.WithPrecision(40, 40);
        var z = new Cplx(
            realInexact ? Rl.AsInexact(Rl.Zero) : Rl.Zero,
            imaginaryInexact ? Rl.AsInexact(Rl.Zero) : Rl.Zero);

        Cplx exp = ComplexMath.Exp(z);

        Assert.Equal(Rl.One, exp.Re);
        Assert.Equal(Rl.Zero, exp.Im);
        AssertComplexInexact("exp(" + z.Re + ", " + z.Im + ")", exp);

        // the exact control: an all-exact zero keeps both components exact
        Cplx exact = ComplexMath.Exp(new Cplx(Rl.Zero, Rl.Zero));
        Assert.Equal(Rl.One, exact.Re);
        Assert.Equal(Rl.Zero, exact.Im);
        Assert.True(exact.Re.IsExact && exact.Im.IsExact, "exp(0 + 0i) must stay exact");
    }

    /// <summary>An inexact argument that is NOT zero was already inexact; this is the guard that the
    /// repair does not reintroduce exactness anywhere on the path.</summary>
    [Fact]
    public void ComplexExpOfAnInexactNonZero_StaysInexact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        var z = new Cplx(Rl.AsInexact(Rl.One), Rl.Zero);
        Assert.False(z.Re.IsExact, "the premise: the real part is inexact");

        AssertComplexInexact("exp(inexact 1 + 0i)", ComplexMath.Exp(z));
    }

    // ------------------------------------------------------------------
    // Sqrt
    // ------------------------------------------------------------------

    /// <summary>
    /// The complex square root, over the argument shapes whose components came back exact: an
    /// inexact zero (both components are the exact <c>sqrt(0) = 0</c>), an inexact perfect square
    /// (the imaginary part is the exact zero of the cancelled difference), and an inexact
    /// non-square. The VALUES are asserted unchanged in every case.
    /// </summary>
    [Fact]
    public void SqrtOfAnInexactArgument_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);

        var inexactZero = new Cplx(Rl.AsInexact(Rl.Zero), Rl.Zero);
        Cplx rootOfInexactZero = ComplexMath.Sqrt(inexactZero);
        Assert.Equal(Rl.Zero, rootOfInexactZero.Re);
        Assert.Equal(Rl.Zero, rootOfInexactZero.Im);
        AssertComplexInexact("sqrt(inexact 0 + 0i)", rootOfInexactZero);

        var inexactFour = new Cplx(Rl.AsInexact(new Rl("4")), Rl.Zero);
        Cplx rootOfInexactFour = ComplexMath.Sqrt(inexactFour);
        Assert.Equal(new Rl("2"), rootOfInexactFour.Re);
        Assert.Equal(Rl.Zero, rootOfInexactFour.Im);
        AssertComplexInexact("sqrt(inexact 4 + 0i)", rootOfInexactFour);

        var inexactTwo = new Cplx(Rl.AsInexact(new Rl("2")), Rl.Zero);
        Cplx rootOfInexactTwo = ComplexMath.Sqrt(inexactTwo);
        Assert.Equal(Rl.Sqrt(new Rl("2")), rootOfInexactTwo.Re);
        Assert.Equal(Rl.Zero, rootOfInexactTwo.Im);
        AssertComplexInexact("sqrt(inexact 2 + 0i)", rootOfInexactTwo);

        // the exact controls, unchanged: sqrt(0) = 0 and sqrt(4) = 2, both components exact
        Cplx rootOfZero = ComplexMath.Sqrt(new Cplx(Rl.Zero, Rl.Zero));
        Assert.Equal(Rl.Zero, rootOfZero.Re);
        Assert.Equal(Rl.Zero, rootOfZero.Im);
        Assert.True(rootOfZero.Re.IsExact && rootOfZero.Im.IsExact, "sqrt(0 + 0i) must stay exact");

        Cplx rootOfFour = ComplexMath.Sqrt(new Cplx(new Rl("4"), Rl.Zero));
        Assert.Equal(new Rl("2"), rootOfFour.Re);
        Assert.Equal(Rl.Zero, rootOfFour.Im);
        Assert.True(rootOfFour.Re.IsExact && rootOfFour.Im.IsExact, "sqrt(4 + 0i) must stay exact");
    }

    /// <summary>The imaginary-part direction of the same rule: an inexact imaginary part is an
    /// inexact argument, and <c>sqrt(9 + 0i) = 3</c> is the exact control beside it.</summary>
    [Fact]
    public void SqrtOfAnInexactImaginaryPart_IsNotExact()
    {
        using var scope = Rl.WithPrecision(40, 40);
        var z = new Cplx(new Rl("9"), Rl.AsInexact(Rl.Zero));
        Assert.False(z.Im.IsExact, "the premise: the imaginary part is an inexact zero");

        Cplx root = ComplexMath.Sqrt(z);
        Assert.Equal(new Rl("3"), root.Re);
        Assert.Equal(Rl.Zero, root.Im);
        AssertComplexInexact("sqrt(9 + inexact 0i)", root);
    }
}
