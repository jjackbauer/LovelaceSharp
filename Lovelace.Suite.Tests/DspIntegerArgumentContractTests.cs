using Lovelace.Abstractions;
using Lovelace.Suite;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// ADV-02 (advancement cycle 7): the DSP plugin's integer arguments — a window, a delay, a transform
/// length, a sample count, a seed — used to be refused through a bare
/// <c>InvalidOperationException</c>, the exact pair (<c>InvalidOperation</c>/<c>DomainError</c>) a
/// genuine DOMAIN refusal uses. A consumer could not branch on "you gave me an argument I cannot
/// honour" versus "your mathematics is undefined".
///
/// <para>The plugin now refuses through the one argument contract
/// (<see cref="ArgumentContractException"/>, which derives from <see cref="ArgumentException"/>, so the
/// runner classifies it as the recoverable <c>InvalidArgument</c>/<c>TypeMismatch</c> pair, the shape
/// <c>dots</c>/<c>zeros</c>/<c>pi</c> already produce).</para>
///
/// <para>This file references NO type introduced by this package beyond the contract itself, so it
/// belongs with the contract rather than with the plugin: it exercises the plugin's public surface
/// through the real engine, which is the surface a host embeds.</para>
/// </summary>
public class DspIntegerArgumentContractTests
{
    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new Lovelace.Dsp.DspPlugin());
        return engine;
    }

    // ------------------------------------------------------------------
    // 1. controls: every accepted shape still answers
    // ------------------------------------------------------------------

    [Fact]
    public void TheIntegerArgumentsThatWorked_StillWork()
    {
        var engine = NewEngine();

        // movingavg is the causal, zero-padded window: the value is read off the live call
        Assert.Equal("[0.(3), 1, 2, 3, 4] (Vector)", ValueFormatter.FormatTyped(engine.Evaluate("movingavg([1, 2, 3, 4, 5], 3)")));
        Assert.Equal("[0, 1, 2, 3] (Vector)", ValueFormatter.FormatTyped(engine.Evaluate("delay([1, 2, 3, 4], 1)")));
        Assert.Equal("[1, 0] (Vector)", ValueFormatter.FormatTyped(engine.Evaluate("impulse(2)")));
        Assert.Equal("[1, 0, 0, 0] (Vector)", ValueFormatter.FormatTyped(engine.Evaluate("impulse(4)")));
        Assert.Equal("4", ValueFormatter.Format(engine.Evaluate("len(step(4))")));
        // the whole-signal form takes no integer argument and must keep answering
        Assert.Equal("[1, -1] (Vector)", ValueFormatter.FormatTyped(engine.Evaluate("dft([0, 1])")));
    }

    // ------------------------------------------------------------------
    // 2. a NON-INTEGRAL Real is refused and NAMED, never truncated
    // ------------------------------------------------------------------

    /// <summary>The measured shape: <c>movingavg(x, 3.7)</c> answered
    /// "movingavg() expects a value within the Int64 range, but got '3.7'." under
    /// <c>InvalidOperation</c>/<c>DomainError</c> — a caller-side argument error reported as a domain
    /// failure. It is now the contract's <see cref="ArgumentContractFailure.NotAnInteger"/>.</summary>
    [Theory]
    [InlineData("movingavg([1, 2, 3, 4, 5], 3.7)", "movingavg", "w")]
    [InlineData("delay([1, 2, 3, 4], 1.5)", "delay", "k")]
    [InlineData("impulse(2.5)", "impulse", "n")]
    [InlineData("step(-0.5)", "step", "n")]
    public void ANonIntegralRealCount_IsRefusedAndNamed(string expression, string builtin, string parameter)
    {
        var engine = NewEngine();

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate(expression));

        Assert.Equal(ArgumentContractFailure.NotAnInteger, ex.Failure);
        Assert.Equal(builtin, ex.Builtin);
        Assert.Equal(parameter, ex.Parameter);
        Assert.IsAssignableFrom<ArgumentException>(ex);
    }

    /// <summary>
    /// <c>3.0</c> IS the integer 3, and MEASURED at this boundary the host normalizes an exactly
    /// integral <c>Real</c> to the integer before the plugin is called, so this answers exactly what
    /// <c>3</c> answers. It is not the defect: the defect was <c>3.7</c> being truncated to 3, which
    /// is refused above.
    /// <para>
    /// The contract's wrong-KIND refusal is therefore not reachable through a language literal at
    /// this boundary. It is pinned at the contract level instead (see
    /// <see cref="TheExactlyIntegralRealRefusal_IsStillTheContractsContract"/>), because the
    /// contract must stay correct for the callers this boundary does not normalize for.
    /// </para>
    /// </summary>
    [Fact]
    public void AnExactlyIntegralRealCount_IsTheIntegerItDenotes()
    {
        var engine = NewEngine();

        string fromReal = ValueFormatter.FormatTyped(engine.Evaluate("movingavg([1, 2, 3, 4, 5], 3.0)"));
        string fromInteger = ValueFormatter.FormatTyped(engine.Evaluate("movingavg([1, 2, 3, 4, 5], 3)"));

        Assert.Equal(fromInteger, fromReal);
    }

    /// <summary>The wrong-KIND refusal, pinned where it is reachable: the contract itself, with a
    /// payload that really is an integral <c>Real</c>.</summary>
    [Fact]
    public void TheExactlyIntegralRealRefusal_IsStillTheContractsContract()
    {
        var ex = ArgumentContractException.NotAnIntegerKind(
            "movingavg", "w", Int64Range.Unbounded, "3");

        Assert.Equal(ArgumentContractFailure.NotAnIntegerKind, ex.Failure);
        Assert.Contains("write it as 3", ex.Message);
        Assert.IsAssignableFrom<ArgumentException>(ex);
    }

    // ------------------------------------------------------------------
    // 3. an out-of-range count is refused by BOUND, and names the number
    // ------------------------------------------------------------------

    /// <summary>A count that does not fit an Int64 used to say "expects a value within the Int64
    /// range" with the number in the message but under the wrong taxonomy pair. It is now the
    /// contract's own bound refusal, which names the value and the bound.</summary>
    [Fact]
    public void ACountWiderThanInt64_IsRefusedByBound()
    {
        var engine = NewEngine();
        const string tooWide = "1000000000000000000000000000000";

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate($"movingavg([1, 2, 3], {tooWide})"));

        Assert.Equal(ArgumentContractFailure.TooWide, ex.Failure);
        Assert.Equal(tooWide, ex.ReceivedText);
        Assert.Contains(tooWide, ex.Message);
    }

    /// <summary>A count that IS representable but negative is refused against the parameter's own
    /// declared bound — <c>OutOfBounds</c>, not a domain failure.</summary>
    [Fact]
    public void ANegativeSampleCount_IsRefusedAgainstTheNonNegativeBound()
    {
        var engine = NewEngine();

        var ex = Assert.Throws<ArgumentContractException>(() => engine.Evaluate("impulse(-1)"));

        Assert.Equal(ArgumentContractFailure.OutOfBounds, ex.Failure);
        Assert.Equal(-1L, ex.Received);
        Assert.Equal("n", ex.Parameter);
    }

    /// <summary>A seed that does not fit Int32 keeps its own, narrower bound.</summary>
    [Fact]
    public void ASeedOutsideTheInt32Range_IsRefusedAgainstItsOwnBound()
    {
        var engine = NewEngine();
        const long outside = 3000000000L;

        var ex = Assert.Throws<ArgumentContractException>(
            () => engine.Evaluate($"noise(1, 0, {outside}, 4)"));

        Assert.Equal(ArgumentContractFailure.OutOfBounds, ex.Failure);
        Assert.Equal(outside, ex.Received);
        // The FOUR-argument form's third position is the SEED, which is what the declaration now says
        // ("scale", "disp", "seed", "n") and what the code reads there. It used to be named "disp" in
        // the refusal while the registry named the same position "n" — one position, two names, which
        // is why the ADV-03 census could not tell what the third argument of a three-argument
        // noise(scale, disp, n) meant.
        Assert.Equal("seed", ex.Parameter);
    }

    // ------------------------------------------------------------------
    // 4. the refusals are never the domain-failure pair
    // ------------------------------------------------------------------

    /// <summary>The whole point of the change: none of these is an
    /// <see cref="InvalidOperationException"/>, which is what the runner turns into
    /// <c>InvalidOperation</c>/<c>DomainError</c> — the pair a genuine domain refusal uses
    /// (<c>Lovelace.Run/Runner.cs</c> Classify).</summary>
    [Theory]
    [InlineData("movingavg([1, 2, 3, 4, 5], 3.7)")]
    [InlineData("movingavg([1, 2, 3], 1000000000000000000000000000000)")]
    [InlineData("delay([1, 2, 3, 4], 1.5)")]
    [InlineData("impulse(-1)")]
    [InlineData("noise(1, 0, 3000000000, 4)")]
    public void AnIntegerArgumentRefusal_IsAnArgumentException_AndNeverAnInvalidOperation(string expression)
    {
        var engine = NewEngine();

        // ThrowsAny: the assertion is on the BASE type, because what the runner's classifier reads
        // is "is this an ArgumentException". Demanding the exact type would reject the very subclass
        // this package introduces.
        var ex = Assert.ThrowsAny<ArgumentException>(() => engine.Evaluate(expression));

        Assert.IsType<ArgumentContractException>(ex);
        Assert.IsNotType<InvalidOperationException>(ex);
    }

    /// <summary>A <c>long</c> payload arrives when a literal is not narrowed to Natural/Integer by
    /// the host. The reader must accept it, because a plugin call site cannot know which of the
    /// three the core chose — the same reason <c>SymbolicsPlugin.AsLong</c> has the arm.</summary>
    [Fact]
    public void TheReader_AcceptsEveryPayloadTheCoreCanSend()
    {
        // The reader the plugin supplies. It is written here INTERPRETIVELY — directly in terms of
        // the numeric types' own new, public, read-only predicate — rather than copied from the
        // plugin, so the test pins the CONTRACT and not one implementation of it.
        // Real inherits Integer, so the Real arm comes first: this ordering is load-bearing.
        static bool SampleReader(object? payload, out long value)
        {
            switch (payload)
            {
                case Lovelace.Real.Real r:
                    return r.TryGetExactInt64(out value);
                case Lovelace.Integer.Integer i:
                    return i.TryGetExactInt64(out value);
                case Lovelace.Natural.Natural n:
                    return n.TryGetExactInt64(out value);
                case long l:
                    value = l;
                    return true;
                default:
                    value = 0L;
                    return false;
            }
        }

        // the non-throwing probe half of the contract
        static bool Read(object? payload, out long value) =>
            ExactIntegerArgument.TryGetExact(payload, SampleReader, out value);

        // the refusal builder the call site owns: this sample call site accepts only Natural/Integer
        // payloads and reports anything else as the wrong KIND, exactly as the plugins do.
        static long Require(object? payload) =>
            ExactIntegerArgument.Require(
                payload, "n", Int64Range.Unbounded, SampleReader,
                p => ArgumentContractException.NotAnInteger("sample", "n", Int64Range.Unbounded, "?"),
                "sample");

        Assert.True(Read(3L, out long fromLong));
        Assert.Equal(3L, fromLong);
        Assert.True(Read(new Lovelace.Natural.Natural(7), out long fromNatural));
        Assert.Equal(7L, fromNatural);
        Assert.True(Read(new Lovelace.Integer.Integer(-7), out long fromInteger));
        Assert.Equal(-7L, fromInteger);
        Assert.True(Read(new Lovelace.Real.Real("2.0"), out long fromIntegralReal));
        Assert.Equal(2L, fromIntegralReal);

        // the exact signed boundary, in both directions
        Assert.Equal(long.MaxValue, Require(new Lovelace.Integer.Integer(long.MaxValue)));
        Assert.Equal(long.MinValue, Require(new Lovelace.Integer.Integer(long.MinValue)));

        // and the two values the flooring helper used to MANGLE: 3.7 is not an integer, and an
        // integer wider than the type is not representable — both answer false, so the contract
        // refuses them by name instead of flooring or wrapping
        Assert.False(Read(new Lovelace.Real.Real("3.7"), out _));
        Assert.False(Read(new Lovelace.Natural.Natural("1000000000000000000000000000000"), out _));
        Assert.Equal(
            0L,
            Require(new Lovelace.Real.Real("0.0")));   // 0.0 IS the integer 0 — integral, unlike 0.5
    }
}
