using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// ADV-02 (advancement cycle 7): the <c>digits</c> argument of <c>evalir</c>/<c>evalir_batch</c> is a
/// PRECISION — a claim about the answer — and it used to be FLOORED.
///
/// <para>The defect (F-1 in <c>docs/advancement-cycle-7/plan.md</c>) was
/// <c>Lovelace.MathIR/Evaluator.cs</c>'s <c>AsLong</c>:
/// <c>long.Parse(r.ToString().Split('.')[0])</c>. Measured on the runner published from <c>ce30fc3</c>:
/// <c>evalir(compile(x*1/3, [x]), [1], 2.9)</c> answered <c>0.33</c> — 2.9 became 2 — with
/// <c>ok: true</c>; <c>digits = 0</c> and <c>digits = -5</c> both answered <c>0</c>, also with
/// <c>ok: true</c>; and <c>digits = 10^30</c> escaped as the raw CLR sentence "Value was either too
/// large or too small for an Int64." under <c>ArithmeticError</c>/<c>DomainError</c>.
/// </para>
///
/// <para>The control that shows the integer path itself was never the problem:
/// <c>evalir(..., 40)</c> and <c>evalf(1/3, 40)</c> both answer 0.333… to 40 places.</para>
///
/// <para>Every refusal case below therefore asserts the CONTRACT
/// (<see cref="ArgumentContractException"/>, which derives from <see cref="ArgumentException"/> so the
/// runner classifies it as the recoverable <c>InvalidArgument</c>/<c>TypeMismatch</c> pair) and every
/// refusal case FAILS on the unmodified tree — with <c>InvalidOperationException</c> for the two
/// flooring shapes and with <c>OverflowException</c> for the too-wide one.</para>
/// </summary>
public class EvalirDigitArgumentContractTests
{
    private const string Kernel = "compile(x*1/3, [x])";

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static SuiteEngine WithSymbol()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        return engine;
    }

    /// <summary>The call shape per builtin: evalir takes one point, evalir_batch a flat list.</summary>
    private static string Call(string builtin, string digits) => builtin == "evalir"
        ? $"evalir({Kernel}, [1], {digits})"
        : $"evalir_batch({Kernel}, [1, 2], {digits})";

    /// <summary>Both builtins carry the same argument and must carry the same contract.</summary>
    public static TheoryData<string> Builtins => new() { "evalir", "evalir_batch" };

    // ------------------------------------------------------------------
    // 1. the control: an integral count is still honoured
    // ------------------------------------------------------------------

    [Theory]
    [MemberData(nameof(Builtins))]
    public void AnIntegralDigitCount_IsStillHonoured(string builtin)
    {
        var engine = WithSymbol();

        var result = engine.Evaluate(Call(builtin, "40"));
        string rendered = ValueFormatter.Format(result);

        Assert.Contains("3333333333333333333333333333333333333333", rendered);
    }

    /// <summary>0.3(3) at 2 places is 0.33 — the request is honoured literally, so the
    /// <c>digits</c> argument is observable in the answer rather than merely accepted.</summary>
    [Theory]
    [MemberData(nameof(Builtins))]
    public void TwoPlaces_AnswerTwoPlaces(string builtin)
    {
        var engine = WithSymbol();
        string rendered = ValueFormatter.Format(engine.Evaluate(Call(builtin, "2")));
        Assert.Contains("0.33", rendered);
        Assert.DoesNotContain("0.333", rendered);
    }

    // ------------------------------------------------------------------
    // 2. a NON-INTEGRAL Real is refused, never floored
    // ------------------------------------------------------------------

    /// <summary>The finding itself: 2.9 must not become 2. The refusal is the contract's
    /// <see cref="ArgumentContractFailure.NotAnInteger"/> and it names the value that arrived, so a
    /// caller reads what it wrote rather than what the implementation made of it.</summary>
    [Theory]
    [MemberData(nameof(Builtins))]
    public void ANonIntegralDigitCount_IsRefusedAndNamed(string builtin)
    {
        var engine = WithSymbol();

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate(Call(builtin, "2.9")));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, ex.Failure);
        Assert.Equal(builtin, ex.Builtin);
        Assert.Equal("digits", ex.Parameter);
        Assert.Contains("2.9", ex.Message);
        // the pair the runner classifies it under: a caller-side argument error, recoverable
        Assert.IsAssignableFrom<ArgumentException>(ex);
    }

    /// <summary>
    /// <c>2.0</c> is the integer 2, and MEASURED at this boundary the host normalizes an exactly
    /// integral <c>Real</c> to the integer before the builtin is called — the payload arrives as a
    /// <c>Natural</c>, not a <c>Real</c>. So this answers 2 places, exactly like the literal
    /// <c>2</c>, and it is NOT the silent flooring the defect was about: that was <c>2.9</c> → 2.
    /// <para>
    /// The distinction is worth pinning rather than assuming: an argument whose value the caller
    /// cannot tell apart from <c>2</c> is not a value the runtime altered, and refusing it would
    /// invent a rule the language does not have. The contract still carries the
    /// <see cref="ArgumentContractFailure.NotAnIntegerKind"/> refusal for callers the boundary does
    /// not normalize for (see the plugin tests).
    /// </para>
    /// </summary>
    [Theory]
    [MemberData(nameof(Builtins))]
    public void AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes(string builtin)
    {
        var engine = WithSymbol();

        // the two calls answer the SAME thing, which is the whole claim: 2.0 is not a different
        // request from 2. (The batch form answers two rows, so the comparison is between the two
        // forms of the SAME builtin rather than against one literal string.)
        string fromIntegralReal = ValueFormatter.Format(engine.Evaluate(Call(builtin, "2.0")));
        string fromInteger = ValueFormatter.Format(engine.Evaluate(Call(builtin, "2")));

        Assert.Equal(fromInteger, fromIntegralReal);
        Assert.Contains("0.33", fromIntegralReal);
        Assert.DoesNotContain("0.333", fromIntegralReal);
    }

    public static TheoryData<string> NonIntegralCounts => new() { "2.5", "2.9", "0.5", "1.0000001" };

    /// <summary>The refusal must never be <see cref="ArgumentContractFailure.NotAnIntegerKind"/> for
    /// a value that is not an integer: a consumer branches on <c>Failure</c>, and telling it to
    /// "write it as 2" for 2.5 would be a repair instruction that changes the request.</summary>
    [Theory]
    [MemberData(nameof(NonIntegralCounts))]
    public void ANonIntegralCount_IsNotReportedAsTheIntegralKindCase(string digits)
    {
        var engine = WithSymbol();
        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate(Call("evalir", digits)));
        Assert.Equal(ArgumentContractFailure.NotAnInteger, ex.Failure);
        // the contract's one sentence for a payload that is not an integer at all; the exact wording
        // belongs to ArgumentContractException and is asserted there, so this pins the MEANING.
        Assert.Contains("must be a Natural or Integer that denotes an exact integer", ex.Message);
        Assert.Contains(digits, ex.Message);
    }

    // ------------------------------------------------------------------
    // 3. a count outside the bound is refused, including the ones the cast used to wrap
    // ------------------------------------------------------------------

    public static TheoryData<string, string> OutOfBoundsCases => new()
    {
        { "evalir", "0" },
        { "evalir_batch", "0" },
        { "evalir", "-5" },
        { "evalir_batch", "-5" },
    };

    /// <summary>The bounds are the SAME the sibling <c>evalf</c> declares: [1, int.MaxValue]. A count
    /// outside them is answered before any evaluation happens, so <c>0</c> can no longer be a silent
    /// no-op that answers a plausible 0.</summary>
    [Theory]
    [MemberData(nameof(OutOfBoundsCases))]
    public void ACountOutsideTheBound_IsRefused(string builtin, string digits)
    {
        var engine = WithSymbol();

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate(Call(builtin, digits)));

        Assert.Equal(ArgumentContractFailure.OutOfBounds, ex.Failure);
        Assert.Equal("digits", ex.Parameter);
        Assert.Equal(new Int64Range(1, int.MaxValue), ex.Bounds);
        Assert.Equal(long.Parse(digits, System.Globalization.CultureInfo.InvariantCulture), ex.Received);
    }

    /// <summary>A count too wide for Int64 used to escape as the raw CLR sentence naming no number.
    /// It is now the contract's own <see cref="ArgumentContractFailure.TooWide"/>, and it NAMES the
    /// value — the only faithful way to report a number wider than the type that would have held
    /// it.</summary>
    [Theory]
    [MemberData(nameof(Builtins))]
    public void ACountWiderThanInt64_IsRefusedAndNamed(string builtin)
    {
        const string tooWide = "1000000000000000000000000000000";
        var engine = WithSymbol();

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate(Call(builtin, "10^30")));

        Assert.Equal(ArgumentContractFailure.TooWide, ex.Failure);
        Assert.Equal(tooWide, ex.ReceivedText);
        Assert.Contains(tooWide, ex.Message);
        // the raw framework sentence carried no number at all; the contract's does
        Assert.DoesNotContain("too large or too small", ex.Message);
    }

    // ------------------------------------------------------------------
    // 4. a non-numeric payload is a typed argument error, not a cast failure
    // ------------------------------------------------------------------

    [Fact]
    public void ATextDigitCount_IsRefusedAsTheSameArgumentError()
    {
        var engine = WithSymbol();

        var ex = Assert.Throws<ArgumentContractException>(
            () => engine.Evaluate($"evalir({Kernel}, [1], \"forty\")"));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, ex.Failure);
        Assert.Equal("digits", ex.Parameter);
    }

    // ------------------------------------------------------------------
    // 5. the sibling evalf contract is untouched by this change
    // ------------------------------------------------------------------

    /// <summary>evalf's own refusal is a pinned message contract
    /// (<c>EvalfDigitCountContractTests</c>, <c>BuiltinSurfaceContractTests</c>) and this package
    /// must not move it.</summary>
    [Fact]
    public void EvalfDigitCountRefusal_IsUnchanged()
    {
        var engine = WithSymbol();

        var ex = Assert.Throws<ArgumentException>(() => engine.Evaluate("evalf(1/3, 2.5)"));

        // "got Real." is the payload's VALUE KIND, not its text — the pinned convention in
        // EvalfDigitCountContractTests (a Real count is refused as the wrong KIND, and the message
        // names the kind; a count that is an Integer and merely out of range names the number).
        // The evalir contract added by this package names the VALUE instead, because there the
        // value is the whole point; see the class summary.
        Assert.Equal(
            $"evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and {int.MaxValue}; got Real.",
            ex.Message);
    }
}
