using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Real;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// A value that came from a TRUNCATING EVALUATION stays inexact when it crosses into a symbolic
/// literal and back.
///
/// <para>THE ROUTE IS PART OF THE ANSWER. <see cref="Rl.IsExact"/> is cleared where a digit is lost
/// (π/e cut to N places, the square root of a non-square, the series the transcendentals run on)
/// and is propagated by every operation that consumes a truncated operand. The symbolic carrier is
/// <see cref="RealLiteral"/>; reading it back is <see cref="RealLiteral.ToReal"/>.</para>
///
/// <para>THE DEFECT THIS FILE PINS. The literal stored digits + exponent and no route, and
/// <c>ToReal()</c> re-derived its exactness from a WIDTH heuristic: <see cref="Rl.TryParse"/> marks a
/// literal inexact only when its fractional width reaches the ambient digit budget
/// (<c>min(MaxComputationDecimalPlaces, 1000)</c>). The same truncation therefore came back exact
/// whenever its rendering happened to be NARROWER than the digit count that was requested.
/// <c>evalf(sin(pi(30)/6), 40)</c> published 0.4999…9 (30 nines) as <c>exact: true</c> with a
/// numerator and a denominator, while <c>evalf(sin(pi(30)/6), 30)</c> — the identical expression
/// — published <c>exact: false</c>, and <c>evalf(sin(pi(45)/6), 40)</c> published
/// <c>exact: true</c> again. The answer was being read off the length of the decimal instead of off
/// the route that produced it. The same laundering ran through the additive and multiplicative
/// constant folds, so <c>subs(x + 1, x, pi(30))</c> reported <c>inspect(...).exact = false</c> and
/// then <c>evalf(...).exact = true</c> for the SAME value — the symbolic tree and the numeric
/// read-back disagreed with each other.</para>
///
/// <para>NOTHING HERE IS SATISFIED BY MAKING THE FLAG FALSE EVERYWHERE. Every negative assertion is
/// paired with its control: a genuinely exact value (1/2, sqrt(4), 1/17, sin(0), 2i,
/// dft([1,2,3])[0]) must still report exact, and the literal round trip must preserve the DIGITS as
/// well as the flag — a fix that marked every literal inexact would fail this file too.</para>
/// </summary>
/// <para>
/// <c>Category=Costly</c>: the cases here evaluate elementary functions of truncated constants at
/// tens of digits, and the COVERAGE COLLECTOR turns that into minutes (measured: the filtered run
/// under the collector did not finish in 35 minutes before the corpus was bounded and still took
/// 404 s after it, against about a minute without the collector). CI therefore excludes the category
/// from its instrumented loop and runs it, uninstrumented, in its own step - excluded, never
/// skipped.
/// </para>
[Trait("Category", "Costly")]
public class TruncatingEvaluationStaysInexactTests
{
    /// <summary>The digit count the acceptance transcript requests.</summary>
    private const int Digits = 40;

    /// <summary>An exact integer Real. <see cref="Rl"/> has no <c>FromLong</c>; its integer
    /// constructor takes the kernel's arbitrary-precision <see cref="Int"/>.</summary>
    private static Rl N(long value) => new(new Int(value));

    /// <summary>One engine per test: a SuiteEngine + three plugin loads is the expensive fixture.</summary>
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.LoadPlugin(new DspPlugin());
        return engine;
    }

    private static ExprContext NewContext()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static readonly Dictionary<Symbol, Num> NoBindings = new();

    /// <summary>Evaluates one language expression on a fresh engine and returns its value plus the
    /// structured projection the wire publishes for it — the SAME projection Lovelace.Run emits,
    /// so "exact:false with no numerator/denominator" is asserted on the published form, not on a
    /// re-derivation of it.</summary>
    private static (Value Value, StructuredValueDto Structured) Evaluate(string expression)
    {
        var engine = NewEngine();
        // the acceptance transcript declares the symbol first; the arithmetic cases below do not
        // reference it, which is why they are also runnable without the declaration
        engine.Evaluate("x = symbol(\"x\")");
        var value = engine.Evaluate(expression);
        return (value, StructuredProjection.ToStructured(value));
    }

    // ------------------------------------------------------------------
    // 1. the acceptance transcript, read off the published structure
    // ------------------------------------------------------------------

    /// <summary>
    /// Every one of these is a function of a TRUNCATED value, so every one of them must report
    /// <c>exact: false</c> and publish NO rational form. The first four are the cases the audit
    /// listed; they already answered false on the defective tree BY COINCIDENCE — their 40-digit
    /// renderings happen to land exactly on the width heuristic's boundary. The rest are the same
    /// defect made visible: 30- and 45-place truncations of π, the same expression at a
    /// different requested digit count, and a truncation folded through an addition.
    /// </summary>
    [Theory]
    [InlineData("evalf(sin(pi(1)*1/6), 40)")]
    [InlineData("evalf(cos(pi(1)*1/6), 40)")]
    [InlineData("evalf(sin(pi(1)*1/4), 40)")]
    [InlineData("evalf(atan(1), 40)")]
    [InlineData("evalf(sin(pi(30)/6), 40)")]
    [InlineData("evalf(cos(pi(30)/6), 40)")]
    [InlineData("evalf(sin(pi(30)/4), 40)")]
    [InlineData("evalf(atan(pi(30)/4), 40)")]
    [InlineData("evalf(exp(pi(30)/10), 40)")]
    [InlineData("evalf(sin(pi(45)/6), 40)")]
    [InlineData("evalf(sin(pi(30)/6), 100)")]
    [InlineData("evalf(subs(x + 1, x, pi(30)), 40)")]
    public void FunctionOfATruncatedValue_PublishesExactFalse_AndNoRationalForm(string expression)
    {
        var (value, structured) = Evaluate(expression);

        Assert.Equal(ValueKind.Real, value.Kind);
        Assert.True(structured.Exact == false,
            expression + " is a function of a truncated value and must not claim exactness; " +
            "published " + value.AsReal() + " with exact=" + structured.Exact);
        Assert.Null(structured.Numerator);
        Assert.Null(structured.Denominator);

        // and the value really is the truncation this test is about: a truncated irrational is
        // not a periodic expansion, so it is not the exact rational the expression denotes
        Assert.False(value.AsReal().IsExact);
    }

    /// <summary>
    /// The exact controls, read the same way. These are the values that must NOT be swept up by the
    /// repair: an integer root, a periodic rational (exact AS a rational, with the numerator and
    /// denominator published), the principal square root of a negative rational, and an exact
    /// transform. Acceptance criterion 2, in the language an agent reads.
    /// </summary>
    [Fact]
    public void ExactControls_StayExact()
    {
        var (integer, integerStructure) = Evaluate("evalf(sqrt(4), 40)");
        Assert.Equal(ValueKind.Integer, integer.Kind);
        Assert.Equal("2", integer.AsInteger().ToString());
        Assert.True(integerStructure.Exact);

        var (periodic, periodicStructure) = Evaluate("1/17");
        Assert.Equal(ValueKind.Real, periodic.Kind);
        Assert.True(periodic.AsReal().IsPeriodic);
        Assert.True(periodicStructure.Exact);
        Assert.Equal("1", periodicStructure.Numerator);
        Assert.Equal("17", periodicStructure.Denominator);

        var (complexRoot, complexStructure) = Evaluate("evalf((-4)^(1/2), 40)");
        Assert.Equal(ValueKind.Complex, complexRoot.Kind);
        Assert.True(complexStructure.Exact);
        Assert.Equal("0", complexRoot.AsComplex().Re.ToString());
        Assert.Equal("2", complexRoot.AsComplex().Im.ToString());

        var (transform, transformStructure) = Evaluate("dft([1,2,3])[0]");
        Assert.Equal(ValueKind.Complex, transform.Kind);
        Assert.True(transformStructure.Exact);
        Assert.Equal("6", transform.AsComplex().Re.ToString());
        Assert.Equal("0", transform.AsComplex().Im.ToString());
    }

    // ------------------------------------------------------------------
    // 2. the property, over a family of ARGUMENTS (not a list of strings)
    // ------------------------------------------------------------------

    /// <summary>
    /// The family of arguments the property is quantified over, built by the operations that
    /// actually lose digits. Nothing here is a literal written into the source: each member is
    /// computed, and the test asserts the premise (every member IS a truncation) before it uses
    /// them, so a family that stopped being truncations would fail loudly instead of passing
    /// vacuously.
    /// </summary>
    private static (string Label, Rl Value)[] TruncatedArguments()
    {
        // Corpus, precision and cost: the family spans the three routes that matter (a truncated
        // constant, a truncated function value, and a truncation moved into the unit interval where
        // the inverse functions are defined). It is built and consumed at 40/32 rather than 60/50
        // because the series cost is superlinear in the digit count and the COVERAGE COLLECTOR
        // turned the earlier 16-value x 9-function form into tens of minutes in CI (measured: the
        // filtered run did not finish in 35 minutes under the collector, against under two minutes
        // without it). The entries this drops are the >40-place arguments, whose route is still
        // covered by FunctionOfATruncatedValue_...("evalf(sin(pi(45)/6), 40)") and
        // ("evalf(sin(pi(30)/6), 100)") below.
        using var scope = Rl.WithPrecision(40, 32);
        var pi30 = Rl.PiTo(30);
        var e30 = Rl.ETo(30);
        var root2 = Rl.Sqrt(N(2));
        return new (string, Rl)[]
        {
            ("pi to 30 places", pi30),
            ("e to 30 places", e30),
            ("sqrt(2)", root2),
            ("sqrt(3)", Rl.Sqrt(N(3))),
            ("sin(1)", Rl.Sin(Rl.One)),
            ("cos(1)", Rl.Cos(Rl.One)),
            ("exp(1)", Rl.Exp(Rl.One)),
            ("pi/10 to 30 places", pi30 / N(10)),
            ("pi/6 to 30 places", pi30 / N(6)),
            ("pi/4 to 30 places", pi30 / N(4)),
            ("e/4 to 30 places", e30 / N(4)),
        };
    }

    /// <summary>
    /// THE PROPERTY. For every argument in the truncated family, the value must survive the trip
    /// through the symbolic literal UNCHANGED IN VALUE and UNCHANGED IN ROUTE:
    /// <c>RealLiteral.FromRealExact(v).ToReal()</c> denotes the same number and is still inexact.
    /// The digit half of the assertion is what stops "mark every literal inexact" from passing.
    /// </summary>
    [Fact]
    public void LiteralRoundTrip_PreservesBothTheDigitsAndTheRoute()
    {
        NewContext();
        foreach (var (label, value) in TruncatedArguments())
        {
            Assert.False(value.IsExact, label + ": the family must consist of truncations");

            var back = RealLiteral.FromRealExact(value).ToReal();

            // the digits are the value that crossed: a Real compares on its digits, so this is an
            // exact-number assertion, and it would fail if the literal had dropped digits
            Assert.Equal(value, back);
            Assert.False(back.IsExact,
                label + " lost digits when it was evaluated and must not come back exact " +
                "through the symbolic literal (" + back + ")");
        }
    }

    /// <summary>The control direction of the same property: an EXACT value's literal round trip
    /// must stay exact. Without this the property above would be satisfied by clearing the flag
    /// unconditionally.</summary>
    [Fact]
    public void LiteralRoundTrip_KeepsGenuinelyExactValuesExact()
    {
        NewContext();
        using var scope = Rl.WithPrecision(60, 50);

        var exactValues = new (string Label, Rl Value)[]
        {
            ("0", Rl.Zero),
            ("1", Rl.One),
            ("1/2", Rl.Parse("0.5", null)),
            ("-3/2", Rl.Parse("-1.5", null)),
            ("1/8", Rl.Parse("0.125", null)),
            ("1000000", Rl.Parse("1000000", null)),
            ("1/(10^19)", Rl.Parse("0." + new string('0', 18) + "1", null)),
        };

        foreach (var (label, value) in exactValues)
        {
            Assert.True(value.IsExact, label + ": the control family must consist of exact values");

            var back = RealLiteral.FromRealExact(value).ToReal();

            Assert.Equal(value, back);
            Assert.True(back.IsExact, label + " is exact and must stay exact through the literal");
        }
    }

    /// <summary>
    /// THE PROPERTY, second half: for every (function, truncated argument) pair, evaluating the
    /// function on the value that crossed through the literal yields an INEXACT result. The result
    /// is read off <see cref="NumReal.V"/>, which is exactly what the wire publishes for a Real, so
    /// this is the acceptance flag and not a proxy for it.
    /// </summary>
    [Fact]
    public void FunctionOfATruncatedArgument_IsInexact()
    {
        var ctx = NewContext();
        using var scope = Rl.WithPrecision(40, 32);   // see TruncatedArguments() on the cost
        var functions = new[] { "sin", "cos", "tan", "exp", "atan", "sinh", "cosh", "tanh", "sqrt" };
        int pairs = 0;

        foreach (var (label, value) in TruncatedArguments())
        {
            foreach (var name in functions)
            {
                var literal = Exprs.Real(RealLiteral.FromRealExact(value));
                var node = name == "sqrt" ? Exprs.Sqrt(literal) : Exprs.Function(ctx.Function(name), literal);
                var result = Assert.IsType<NumReal>(Evaluation.EvaluateToNum(node, ctx, NoBindings)).V;

                Assert.False(result.IsExact,
                    name + "(" + label + ") = " + result + " came from a truncated argument and " +
                    "must not claim exactness");
                pairs++;
            }
        }

        Assert.Equal(TruncatedArguments().Length * functions.Length, pairs);
    }

    /// <summary>
    /// The forward direction of the flag on the same quantified shape: every elementary function of
    /// an EXACT argument whose value IS exactly representable stays exact. This is the control that
    /// keeps the property above from being satisfied by a blanket "inexact".
    /// </summary>
    [Theory]
    [InlineData("sin", 0, 0)]
    [InlineData("tan", 0, 0)]
    [InlineData("atan", 0, 0)]
    [InlineData("asin", 0, 0)]
    [InlineData("sinh", 0, 0)]
    [InlineData("tanh", 0, 0)]
    [InlineData("cos", 0, 1)]
    [InlineData("exp", 0, 1)]
    [InlineData("cosh", 0, 1)]
    [InlineData("acosh", 1, 0)]
    [InlineData("log", 1, 0)]
    public void ElementaryFunctionOfAnExactArgument_StaysExact(string name, long argument, long expected)
    {
        var ctx = NewContext();
        using var scope = Rl.WithPrecision(Digits, Digits);
        var e = Exprs.Function(ctx.Function(name), Exprs.Integer(argument));
        var value = Evaluation.EvaluateToNum(e, ctx, NoBindings);
        // Exprs.Integer canonicalises to a RationalConstant, so the fold comes back on the exact
        // rational tier; the value is what matters, not which exact tier carries it
        Assert.Equal(Rat.From(new Int(expected)), NumOps.RatOf(value));
        Assert.True(value.IsExact);
    }

    // ------------------------------------------------------------------
    // 3. the truncation that happens to BE a rational
    // ------------------------------------------------------------------

    /// <summary>
    /// π truncated at 30 places, over 6, through sin: the stored value is 0.4999…9 — a finite
    /// decimal, therefore a rational, one 10⁻³⁰ away from 1/2 — and the flag must STILL be
    /// false, because the value stored is not the 1/2 the exact expression denotes. This is the case
    /// that kills any "it looks like a rational, so call it exact" rule, and it is why the flag is a
    /// statement about the ROUTE: the same digits reached by an exact route are exact, and reached
    /// by a truncation they are not.
    /// </summary>
    [Fact]
    public void TruncatedValueThatHappensToBeARational_IsStillInexact()
    {
        var ctx = NewContext();
        using var scope = Rl.WithPrecision(Digits, Digits);
        var half = Rl.Parse("0.5", null);

        var argument = Rl.PiTo(30) / N(6);
        Assert.False(argument.IsExact, "pi truncated at 30 places over 6 is a truncation");

        // the argument survives the crossing, digits and all
        var back = RealLiteral.FromRealExact(argument).ToReal();
        Assert.Equal(argument, back);
        Assert.False(back.IsExact, "the truncated argument must come back inexact");

        var node = Exprs.Function(ctx.Function("sin"), Exprs.Real(RealLiteral.FromRealExact(argument)));
        var value = Assert.IsType<NumReal>(Evaluation.EvaluateToNum(node, ctx, NoBindings)).V;

        // the stored value IS a rational: it is a finite decimal, so it has a numerator and a
        // denominator, and it agrees with 1/2 to 30 places
        Assert.False(value.IsPeriodic);
        var stored = RationalReal.FromReal(value);
        Assert.NotEqual(Rat.From(1, 2), stored);
        Assert.True(Rl.Abs(half - value) < Rl.Parse("0." + new string('0', 28) + "1", null),
            "the truncation agrees with 1/2 to 30 places (" + value + ")");

        // ... and it must still not claim exactness
        Assert.False(value.IsExact,
            "sin(pi truncated at 30 places / 6) = " + value + " is a truncation whose digits happen " +
            "to be a rational; the exact expression denotes 1/2 and this is not 1/2");

        // the control on the other side of the same question: the exact value 1/2 is exact, and so
        // is the exact decimal 0.5 when it is read as a value rather than produced by a truncation.
        // Exprs.Rational canonicalises 1/2 onto the exact RATIONAL tier exactly as Exprs.Integer
        // does — the same tier-neutral reading the control at :309-311 relies on — so the concrete
        // carrier is NumRat, not NumInt. Naming the class asserted an implementation detail that
        // contradicts the value, but the failure it guarded against is real and is kept: RatOf
        // throws for anything off the exact tier, so this still fails if 1/2 stops being exactly the
        // rational 1/2 carried exactly, and the flag is now asserted beside the value.
        var foldedHalf = Evaluation.EvaluateToNum(Exprs.Rational(1, 2), ctx, NoBindings);
        Assert.Equal(Rat.From(1, 2), NumOps.RatOf(foldedHalf));
        Assert.True(foldedHalf.IsExact, "the exact rational 1/2 must fold exactly, not merely look exact");
        Assert.True(half.IsExact, "0.5 parsed from an exact decimal literal is exact");
        Assert.True(RealLiteral.FromRealExact(half).ToReal().IsExact,
            "the exact 1/2 must not be caught by the same rule");
    }

    // ------------------------------------------------------------------
    // 4. the two carriers agree with each other
    // ------------------------------------------------------------------

    /// <summary>
    /// The same value read two ways must get one answer. <c>inspect(...).exact</c> reads the
    /// SYMBOLIC tree's flag; <c>evalf(...)</c> reads the numeric value the tree evaluates to. On the
    /// defective tree they disagreed for the same expression — the tree said false (correctly: a
    /// Real leaf carries the approximation) while the numeric read-back said true. This is the
    /// self-consistency half of the repair, and it is the reason the additive constant fold has to
    /// keep the route too: it synthesises a NEW literal out of the operands' digits.
    /// </summary>
    [Theory]
    [InlineData("subs(x + 1, x, pi(30))")]
    [InlineData("subs(x + 1, x, sqrt(2))")]
    [InlineData("subs(x*2, x, pi(30))")]
    [InlineData("subs(x + x, x, pi(30))")]
    public void SymbolicTreeFlag_AndNumericReadBack_Agree(string expression)
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        var symbolic = engine.Evaluate("inspect(" + expression + ").exact");
        var treeExact = symbolic.AsBoolean();

        var numeric = engine.Evaluate("evalf(" + expression + ", 40)");
        var numericExact = StructuredProjection.ToStructured(numeric).Exact;

        Assert.False(treeExact, expression + " carries a truncated decimal leaf");
        Assert.True(numericExact == false,
            expression + ": the symbolic tree says exact=false but the numeric read-back says " +
            numericExact + " (" + ValueFormatter.Format(numeric) + ")");
    }
}
