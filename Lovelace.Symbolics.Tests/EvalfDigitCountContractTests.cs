using System.Text.Json.Nodes;
using Xunit;
using Xunit.Abstractions;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Cycle 5, item 1: the CONTRACT of <c>evalf(f, digits)</c> for a count that cannot be honoured.
/// <para>
/// The audit reproduced five different answers to one argument problem at <c>9e761ba</c>:
/// <c>evalf(sin(1), 0)</c> and <c>evalf(cos(1), 0)</c> crossed as
/// <c>InternalError</c>/<c>InternalInvariantFailure</c> ("Object reference not set to an instance of
/// an object."), <c>evalf(exp(1), 0)</c> behaved like <c>digits = 1</c>, <c>evalf(1/3, 0)</c>
/// answered <c>0</c> declared <c>exact: true</c>, and <c>evalf(sqrt(2), 0)</c> ignored the count
/// entirely. The protocol is explicit that an internal invariant failure is never an answer to an
/// argument problem (docs/symbolics/dsh-protocol.md §Error envelope), and a silently rounded or
/// ignored count is not an answer either.
/// </para>
/// <para>
/// The decided contract, asserted here as a PROPERTY over every shape <c>evalf</c> can receive
/// (a transcendental, an elementary inverse, an irrational radical, a constant, an exact rational,
/// and a composite of them) and over every non-positive count: <c>digits</c> is a Natural or
/// Integer in <c>[1, 2147483647]</c>, anything else is the RECOVERABLE argument error
/// (<c>InvalidArgument</c>/<c>TypeMismatch</c>, the pair <c>pi(0)</c>/<c>e(0)</c> already produce)
/// whose message names the builtin, the argument, the acceptable range and the refused value.
/// <c>digits = 1</c> is the smallest accepted count and must keep answering; the counts that
/// already worked must keep working.
/// </para>
/// </summary>
public class EvalfDigitCountContractTests
{
    private readonly ITestOutputHelper _output;

    public EvalfDigitCountContractTests(ITestOutputHelper output) => _output = output;

    /// <summary>The shapes <c>evalf</c> must answer alike: an already-numeric transcendental, an
    /// elementary inverse, an exact irrational constant, a radical, an exact rational, a constant
    /// that arrives numeric, and a composite expression.</summary>
    private static readonly string[] Functions =
        { "sin(1)", "cos(1)", "exp(1)", "log(2)", "sqrt(2)", "1/3", "pi", "e", "2*sin(1)" };

    private static readonly string[] NonPositiveCounts = { "0", "-1", "-100" };

    /// <summary>The cross product, one case per (function, count): a failure names the shape that
    /// regressed instead of hiding it inside a loop.</summary>
    public static TheoryData<string, string> NonPositiveRequests()
    {
        var data = new TheoryData<string, string>();
        foreach (string f in Functions)
            foreach (string digits in NonPositiveCounts)
                data.Add(f, digits);
        return data;
    }

    /// <summary>The one message every non-positive request produces. The RANGE is spelled with the
    /// Int32 bound because that bound is part of the contract: the count is what the builtin's
    /// precision scope takes, and a value above it used to wrap to a NEGATIVE count.</summary>
    private static string ExpectedMessage(string got) =>
        $"evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and {int.MaxValue}; got {got}.";

    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Lovelace.Run.Runner.RunAsync(
            new[] { "--eval", script, "--json", "--omit-functions" }, stdout, stderr);
        JsonNode? envelope = JsonNode.Parse(stdout.ToString());
        Assert.True(envelope is not null, $"'{script}' produced no envelope (exit {exitCode}). stderr: {stderr}");
        return (exitCode, envelope!);
    }

    private static JsonNode Structured(JsonNode envelope) => envelope["result"]!["structured"]!;

    [Theory]
    [MemberData(nameof(NonPositiveRequests))]
    public async Task NonPositiveDigits_AreARecoverableArgumentError_ForEveryShape(string f, string digits)
    {
        var (exit, envelope) = await RunAsync($"evalf({f}, {digits})");

        // the refusal is the ERROR ENVELOPE (the call cannot answer), and it is an ARGUMENT error:
        // never InternalError/InternalInvariantFailure, which the protocol forbids as an answer to
        // an argument problem, and never a value.
        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"evalf({f}, {digits}) must not answer a value");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(), "an argument error the caller can fix is recoverable");
        Assert.Equal(ExpectedMessage(digits), envelope["message"]!.GetValue<string>());

        // ... and the two fields a caller branches on are never the invariant-failure pair
        Assert.NotEqual("InternalError", envelope["code"]!.GetValue<string>());
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());

        _output.WriteLine($"evalf({f}, {digits}) -> {envelope["code"]!.GetValue<string>()}/{envelope["category"]!.GetValue<string>()}: {envelope["message"]!.GetValue<string>()}");
    }

    /// <summary>The boundary the refusal must NOT swallow: <c>digits = 1</c> is the smallest
    /// ACCEPTED count, and every shape answers with exactly one decimal place (the values are read
    /// off the live call: the count TRUNCATES, so 2*sin(1) = 1.6829 answers 1.6, not 1.7).</summary>
    public static TheoryData<string, string> DigitsOneRequests() => new()
    {
        { "sin(1)", "0.8" },
        { "cos(1)", "0.5" },
        { "exp(1)", "2.7" },
        { "log(2)", "0.6" },
        { "sqrt(2)", "1.4" },
        { "1/3", "0.3" },
        { "pi", "3.1" },
        { "e", "2.7" },
        { "2*sin(1)", "1.6" },
    };

    [Theory]
    [MemberData(nameof(DigitsOneRequests))]
    public async Task DigitsOne_IsAccepted_ForEveryShape(string f, string expected)
    {
        var (exit, envelope) = await RunAsync($"evalf({f}, 1)");

        Assert.Equal(0, exit);
        Assert.True(envelope["ok"]!.GetValue<bool>());
        JsonNode structured = Structured(envelope);
        Assert.Equal("Real", structured["kind"]!.GetValue<string>());
        Assert.Equal(expected, structured["value"]!.GetValue<string>());
    }

    /// <summary>The same contract for a count that is not a count: wider than Int32 (the
    /// <c>(int)</c> cast used to wrap — <c>2147483648</c> became a NEGATIVE count and
    /// <c>evalf(1/3, 2147483648)</c> answered <c>0</c>, declared exact), wider than Int64 (a raw
    /// CLR OverflowException crossed as <c>ArithmeticError</c>), and a Real (silently truncated to
    /// its integer part — <c>5.0</c> became 5).</summary>
    [Theory]
    [InlineData("2147483648")]
    [InlineData("4294967297")]
    [InlineData("99999999999999999999999999999999")]
    [InlineData("5.0")]
    public async Task DigitsOutsideTheAdvertisedRange_AreTheSameTypedRefusal(string digits)
    {
        var (exit, envelope) = await RunAsync($"evalf(1/3, {digits})");

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"evalf(1/3, {digits}) must not answer a value");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.StartsWith("evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got ", envelope["message"]!.GetValue<string>());
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
    }

    /// <summary>The refusal must not have narrowed what works: the counts the audit measured as
    /// HONOURED still answer exactly those digits (they are pinned here as well as in
    /// <c>Lovelace.Run.Tests.BuiltinSurfaceContractTests</c>, because the guard sits in front of
    /// the same precision scope they exercise).</summary>
    [Fact]
    public async Task CountsThatWereHonoured_KeepAnsweringTheRequestedCount()
    {
        var (sqrtExit, sqrt) = await RunAsync("evalf(sqrt(2), 5)");
        Assert.Equal(0, sqrtExit);
        Assert.Equal("1.41421", Structured(sqrt)["value"]!.GetValue<string>());

        var (fiftyExit, fifty) = await RunAsync("evalf(sqrt(2), 50)");
        Assert.Equal(0, fiftyExit);
        Assert.Equal("1.41421356237309504880168872420969807856967187537694", Structured(fifty)["value"]!.GetValue<string>());

        var (thirdExit, third) = await RunAsync("evalf(1/3, 5)");
        Assert.Equal(0, thirdExit);
        Assert.Equal("0.33333", Structured(third)["value"]!.GetValue<string>());
    }
}
