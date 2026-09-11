using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 21: two bounded gaps in the language surface.
/// <para>
/// (A) asinh/acosh/atanh are registered KERNEL functions with evaluators and domain rules, but
/// they were never exported as builtins, so 'atanh(0.5)' reported "Unknown function 'atanh'."
/// They are now builtins with complete help metadata like every other elementary function.
/// </para>
/// <para>
/// (B) An integer base raised to a NEGATIVE integer exponent is the reciprocal power, computed
/// exactly: 2^-1 is 1/2, not an error. Zero to a negative power has no value in any field, so it
/// stays a typed, recoverable error — and the message must name the BASE, never the exponent.
/// </para>
/// </summary>
public class HyperbolicBuiltinsAndNegativeIntegerPowersTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static long BuiltinCount(SuiteEngine engine) =>
        engine.CaptureState().Functions.Values.Count(f => f.IsBuiltin);

    // ------------------------------------------------------------------
    // A. the three hyperbolic builtins
    // ------------------------------------------------------------------

    /// <summary>Every one of the three is registered, is a builtin, and evaluates at its
    /// defining point to the exact value (a Symbolic 0, exactly as sin(0) already folds).</summary>
    [Theory]
    [InlineData("asinh", "asinh(0)")]
    [InlineData("acosh", "acosh(1)")]
    [InlineData("atanh", "atanh(0)")]
    public void HyperbolicBuiltins_AreRegistered_AndFoldAtTheirDefiningPoint(string name, string call)
    {
        var engine = NewEngine();
        var state = engine.CaptureState();
        Assert.True(state.Functions.TryGetValue(name, out var fn), name + " is not a known function");
        Assert.True(fn!.IsBuiltin, name + " is not a builtin");

        var value = engine.Evaluate(call);
        Assert.Equal(ValueKind.Symbolic, value.Kind);
        Assert.Equal("0", ValueFormatter.Format(value));
    }

    /// <summary>The exported surface is the ONLY thing that changed: the symbolic form of an
    /// argument stays a function application (as it does for sinh/cosh/tanh).</summary>
    [Fact]
    public void HyperbolicBuiltins_KeepTheirSymbolicApplicationForm()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        Assert.Equal("asinh(x)", ValueFormatter.Format(engine.Evaluate("asinh(x)")));
        Assert.Equal("acosh(x)", ValueFormatter.Format(engine.Evaluate("acosh(x)")));
        Assert.Equal("atanh(x)", ValueFormatter.Format(engine.Evaluate("atanh(x)")));
    }

    /// <summary>The help contract every other builtin satisfies: a summary, a real example, a
    /// declared return kind, and cross-references (the existing completeness test only checks the
    /// first of those, so the rest is pinned here).</summary>
    [Theory]
    [InlineData("asinh", "sinh", "asinh(0)")]
    [InlineData("acosh", "cosh", "acosh(1)")]
    [InlineData("atanh", "tanh", "atanh(0)")]
    public void HyperbolicBuiltins_HaveCompleteDescriptors(string name, string inverse, string example)
    {
        var engine = NewEngine();
        var help = engine.Help.Function(name);
        Assert.NotNull(help);
        Assert.DoesNotContain("(no summary registered)", help);
        Assert.Contains(name + "(x)", help);
        Assert.Contains(example, help);
        Assert.DoesNotContain("Returns: Value", help);

        // the cross-reference must be a REAL function, and the counterpart's help must reference
        // this one back (the see-also pairs are the only discoverability route between them)
        var snapshot = engine.CaptureState();
        snapshot.Functions.TryGetValue(inverse, out var counterpart);
        Assert.True(counterpart!.IsBuiltin, inverse + " must exist for the cross-reference to resolve");
        Assert.Contains(inverse, engine.Help.Function(name)!);
        Assert.Contains(name, engine.Help.Function(inverse)!);
    }

    /// <summary>The count the metadata completeness suite walks: exactly three names were added
    /// by the hyperbolic round. The total is a tripwire, not a target: round 22 added exactly one
    /// builtin (<c>capabilities</c>) and round 28 added exactly one more (<c>latex</c>, the LaTeX
    /// print mode); round 36 added exactly one more (<c>cancel_full</c>, the structured cancel that
    /// carries the NonZero side conditions the bare value builtin cannot — N16a), which is why the
    /// pinned number here is 107.</summary>
    [Fact]
    public void BuiltinCount_IncludesTheThreeHyperbolicFunctions()
    {
        var engine = NewEngine();
        Assert.Equal(107, BuiltinCount(engine));
    }

    // ------------------------------------------------------------------
    // B. negative integer exponents
    // ------------------------------------------------------------------

    /// <summary>2^-1 is the reciprocal power. It must be the SAME value the language produces for
    /// the literal 1/2 (an exact Real), not merely "some" result: the assertion compares the
    /// value, its kind and both renderings.</summary>
    [Fact]
    public void TwoToTheMinusOne_IsTheExactValueOfOneHalf()
    {
        var engine = NewEngine();
        var half = engine.Evaluate("1/2");
        var value = engine.Evaluate("2^-1");

        Assert.Equal(half.Kind, value.Kind);
        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.True(value.AsReal() == half.AsReal(), "2^-1 must equal 1/2 exactly");
        Assert.Equal("0.5", ValueFormatter.Format(value));
        Assert.Equal("0.5 (Real)", ValueFormatter.FormatTyped(value));

        // the strongest available statement of "exact rational": the structured projection of
        // the value IS the rational 1/2, marked exact — not a decimal approximation of it
        var structured = StructuredProjection.ToStructured(value);
        Assert.True(structured.Exact == true);
        Assert.Equal("1", structured.Numerator);
        Assert.Equal("2", structured.Denominator);
        Assert.Equal(StructuredProjection.ToStructured(half).Numerator, structured.Numerator);
        Assert.Equal(StructuredProjection.ToStructured(half).Denominator, structured.Denominator);
    }

    /// <summary>3^-2 = 1/9, again value-equal to the literal.</summary>
    [Fact]
    public void ThreeToTheMinusTwo_IsTheExactValueOfOneNinth()
    {
        var engine = NewEngine();
        var ninth = engine.Evaluate("1/9");
        var value = engine.Evaluate("3^-2");

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.True(value.AsReal() == ninth.AsReal(), "3^-2 must equal 1/9 exactly");
        Assert.Equal(ValueFormatter.Format(ninth), ValueFormatter.Format(value));
        Assert.Equal("0.(1)", ValueFormatter.Format(value));

        var structured = StructuredProjection.ToStructured(value);
        Assert.True(structured.Exact == true);
        Assert.Equal("1", structured.Numerator);
        Assert.Equal("9", structured.Denominator);
    }

    /// <summary>A negative exponent on an integer base does not disturb the positive-exponent
    /// results or the integer-exact cases.</summary>
    [Fact]
    public void PositiveAndExactIntegerPowers_AreUnchanged()
    {
        var engine = NewEngine();
        Assert.Equal("8", ValueFormatter.Format(engine.Evaluate("2^3")));
        Assert.Equal("1", ValueFormatter.Format(engine.Evaluate("2^0")));
        Assert.Equal("4", ValueFormatter.Format(engine.Evaluate("4^-1 * 4^2")));
    }

    /// <summary>A negative power of a SYMBOL is left as the symbolic reciprocal power the kernel
    /// already prints: 'x^-1'. It is not silently rewritten to 1/x and does not throw.</summary>
    [Fact]
    public void SymbolToTheMinusOne_StaysTheSymbolicReciprocalPower()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        var value = engine.Evaluate("x^-1");
        Assert.Equal(ValueKind.Symbolic, value.Kind);
        Assert.Equal("x^-1", ValueFormatter.Format(value));
    }

    /// <summary>0^-1 is a domain failure, not a crash: a typed ArgumentOutOfRangeException whose
    /// message names the BASE ('base'), never the exponent, and the engine stays usable.</summary>
    [Fact]
    public void ZeroToTheMinusOne_IsATypedRecoverableErrorThatNamesTheBase()
    {
        var engine = NewEngine();
        var ex = Assert.Throws<ArgumentOutOfRangeException>(() => engine.Evaluate("0^-1"));

        Assert.Equal("base", ex.ParamName);
        Assert.Equal("Base cannot be zero. (Parameter 'base')", ex.Message);
        Assert.DoesNotContain("exponent", ex.Message);

        // the failure is REPORTED as a diagnostic of that evaluation (diagnostics are
        // per-evaluation state, so they must be read before the next call resets them)
        Assert.Contains(engine.Diagnostics, d => d.Message == ex.Message);

        // recoverable: the session survives the failure
        Assert.Equal("2", ValueFormatter.Format(engine.Evaluate("1 + 1")));
    }

    /// <summary>
    /// REQUIREMENT CHANGE (goal cycle 4, round 03). This test used to assert that
    /// <c>sqrt(-1)</c> and <c>(-1)^(1/2)</c> THROW ("this round does NOT add complex square roots
    /// or rational powers of negative bases"). The maintainer's §4.3 decision reopened that scope:
    /// the values were rejected for want of a representation, and the representation exists. They
    /// now answer the PRINCIPAL branch exactly — the branch SymPy 1.14.0 returns for
    /// <c>(-1)**Rational(1,2)</c> and <c>sqrt(-4)</c> — so the throwing half is replaced by an
    /// assertion of the new exact value, NOT deleted.
    ///
    /// <para>The control half is the part that matters for honesty: the cases that are still
    /// unsupported keep their exact typed error and message, and
    /// <c>capabilities()</c> keeps advertising them.</para>
    /// </summary>
    [Fact]
    public void NegativeSquareRootsAndHalfPowers_NowAnswerExactly_WhileOtherNonIntegerPowersStillThrow()
    {
        var engine = NewEngine();

        // SymPy: sqrt(-1) = I, sqrt(-4) = 2*I, (-1)**Rational(1,2) = I
        Assert.Equal("i", ValueFormatter.Format(engine.Evaluate("sqrt(-1)")));
        Assert.Equal("i", ValueFormatter.Format(engine.Evaluate("(-1)^(1/2)")));
        Assert.Equal("2*i", ValueFormatter.Format(engine.Evaluate("sqrt(-4)")));

        // CONTROL 1: a POSITIVE base under a non-integer exponent is a different, much larger
        // feature (a real irrational the Real type will not approximate) and is unchanged.
        var positive = Assert.Throws<NotImplementedException>(() => engine.Evaluate("2^(1/2)"));
        Assert.Equal("Non-integer exponents are not yet supported.", positive.Message);

        // CONTROL 2: a negative base under an exponent whose denominator is >= 3 has no exact
        // complex-constant value, so it keeps the same typed error.
        var third = Assert.Throws<NotImplementedException>(() => engine.Evaluate("(-8)^(1/3)"));
        Assert.Equal("Non-integer exponents are not yet supported.", third.Message);
        Assert.Equal("Non-integer exponents are not yet supported.",
            Assert.Throws<NotImplementedException>(() => engine.Evaluate("(-1)^(1/3)")).Message);
    }
}
