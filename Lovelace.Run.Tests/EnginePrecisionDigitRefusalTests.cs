using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// K-1 (round 21): a digit count the ENGINE cannot honour was refused by the KERNEL, and the refusal
/// crossed to the caller as the framework's own sentence naming only the parameter — no count, no
/// engine state, nothing a caller could act on.
///
/// <para>The engine state that makes the request unanswerable is DOCUMENTED and stays:
/// <c>setprecision(n)</c> "sets the engine computation and display precision"
/// (<c>Lovelace.Suite/CoreBuiltinMetadata.cs:45-46</c>, the builtin at
/// <c>Lovelace.Suite/Interpreter.cs:1827-1841</c>) and the REPL relies on it
/// (<c>Lovelace.Console/Repl/ReplSession.cs:214</c>). One script is one engine, and the precision a
/// statement reads is the precision the statements before it set (<c>Lovelace.Suite/Interpreter.cs:
/// 319-321</c> re-enters the precision scope per statement), so <c>setprecision(20); pi(500)</c> is
/// a genuine 500-place request against a 20-place engine.</para>
///
/// <para>What was wrong is what the caller received. <c>Real.PiTo</c>/<c>Real.ETo</c> guard the count
/// against <c>Real.MaxComputationDecimalPlaces</c> and throw an <c>ArgumentOutOfRangeException</c>
/// that names neither number (<c>Lovelace.Real/Real.cs:1440-1441</c>, <c>:1574-1575</c>) — idiomatic
/// for the LIBRARY, whose public guard semantics are not changed by this round; a defect at the
/// LANGUAGE surface, which must refuse with the taxonomy's own code and category and a sentence a
/// caller can act on. The refusal is therefore produced by the builtin, BEFORE the kernel guard is
/// reached, as the recoverable argument error the rest of the digit-count surface already produces
/// (<c>InvalidArgument</c>/<c>TypeMismatch</c>: <c>docs/symbolics/dsh-protocol.md</c> §Error
/// envelope; <c>Lovelace.Run/Runner.cs</c> Classify).</para>
/// </summary>
public class EnginePrecisionDigitRefusalTests
{
    /// <summary>The precision the script's FIRST statement sets — the engine state the second
    /// statement's request is answered against.</summary>
    private const int EnginePrecision = 20;

    /// <summary>The engine's DEFAULT computation precision (<c>Lovelace.Suite/Interpreter.cs:160</c>):
    /// the cap the scripts that never call <c>setprecision</c> are refused against.</summary>
    private const int DefaultEnginePrecision = 1000;

    /// <summary>A count that the DEFAULT engine answers and the 20-place engine cannot: the request
    /// K-1 is about.</summary>
    private const int OverCapDigits = 500;

    /// <summary>pi's fractional digits 1-60 (mpmath, <c>mp.dps = 620</c>).</summary>
    private const string PiDigits1To60 =
        "141592653589793238462643383279502884197169399375105820974944";

    /// <summary>pi's fractional digits 481-500 — the tail only a 500-place answer owns.</summary>
    private const string PiDigits481To500 = "89122793818301194912";

    /// <summary>e's fractional digits 1-60 (mpmath, <c>mp.dps = 620</c>).</summary>
    private const string EDigits1To60 =
        "718281828459045235360287471352662497757247093699959574966967";

    /// <summary>e's fractional digits 481-500 — the tail only a 500-place answer owns.</summary>
    private const string EDigits481To500 = "93163688923009879312";

    /// <summary>10^30 written out: a count wider than Int64, which the kernel never sees because the
    /// builtin refuses it as the same argument error (the pair <c>evalf(sin(1), 10^30)</c> is already
    /// pinned to, <c>BuiltinSurfaceContractTests</c>).</summary>
    private const string DigitsWiderThanInt64 = "1000000000000000000000000000000";

    /// <summary>The one sentence the language surface answers a digit count it cannot honour with:
    /// the builtin, the argument, the acceptable range — which IS the engine's computation precision,
    /// the number the kernel's own guard compares against — and the count that was refused.</summary>
    private static string ExpectedRefusal(string builtin, long enginePrecision, string got) =>
        $"{builtin}(): argument 1 (digits) must be a digit count between 1 and {enginePrecision} " +
        $"(the engine's computation precision); got {got}.";

    private static async Task<(int ExitCode, JsonNode Envelope, string Stderr)> RunAsync(string script)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            script, "--omit-functions", "--omit-variables");
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'"), stderr);
    }

    private static string RealValue(JsonNode envelope) =>
        envelope["result"]!["structured"]!["value"]!.GetValue<string>();

    /// <summary>The fractional-digit count of a decimal rendering — the unit the request is made
    /// in.</summary>
    private static int Decimals(string rendering)
    {
        int suffix = rendering.IndexOf(' ');
        if (suffix >= 0)
            rendering = rendering[..suffix];
        int dot = rendering.IndexOf('.');
        return dot < 0 ? 0 : rendering.Length - dot - 1;
    }

    // ------------------------------------------------------------------
    // 1. the request above the engine's precision is a TYPED refusal
    // ------------------------------------------------------------------

    /// <summary>
    /// ONE script, ONE engine, TWO evaluations: the first sets the engine's precision (documented
    /// engine state, unchanged), the second asks for 500 places the engine cannot compute. The
    /// second must answer the language surface's own recoverable argument error — a taxonomy code
    /// and category, a message naming BOTH numbers — and never the framework's parameter dump the
    /// kernel guard leaked (code <c>InvalidArgument</c>, category <c>TypeMismatch</c>,
    /// <c>recoverable: true</c>; the raw form was "Specified argument was out of the range of valid
    /// values. (Parameter 'digits')").
    /// </summary>
    [Theory]
    [InlineData("pi")]
    [InlineData("e")]
    public async Task ARequestAboveTheEnginesPrecision_IsRefusedAsTheDocumentedArgumentError(string builtin)
    {
        string script = $"setprecision({EnginePrecision}); {builtin}({OverCapDigits})";

        var (exit, envelope, stderr) = await RunAsync(script);

        // one engine, two statements — the timings prove the request was answered by the SAME
        // engine the setprecision statement configured, not by a fresh one
        JsonArray timings = envelope["timings"]!.AsArray();
        Assert.Equal(2, timings.Count);
        Assert.Equal(script.IndexOf($"{builtin}(", StringComparison.Ordinal),
            timings[1]!["position"]!.GetValue<int>());

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' must not answer a value: {stderr}");
        Assert.Null(envelope["result"]);

        // the documented pair for a digit count that cannot be honoured ...
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "the caller lowers the count or raises the precision and retries");
        // ... never the invariant-failure pair the protocol forbids as an answer to an argument
        // problem, and never a raw .NET type name in either structural field
        Assert.NotEqual("InternalError", envelope["code"]!.GetValue<string>());
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
        Assert.NotEqual("ArgumentOutOfRangeException", envelope["code"]!.GetValue<string>());
        Assert.NotEqual("ArgumentOutOfRangeException", envelope["category"]!.GetValue<string>());

        string message = envelope["message"]!.GetValue<string>();
        Assert.Equal(ExpectedRefusal(builtin, EnginePrecision, OverCapDigits.ToString()), message);
        // the message names BOTH numbers: what was asked for, and the precision that bounded it
        Assert.Contains(OverCapDigits.ToString(), message, StringComparison.Ordinal);
        Assert.Contains(EnginePrecision.ToString(), message, StringComparison.Ordinal);
        // and it is the language surface's own sentence, not the kernel's parameter dump
        Assert.DoesNotContain("Parameter", message, StringComparison.Ordinal);
        Assert.DoesNotContain("Specified argument", message, StringComparison.Ordinal);
        Assert.DoesNotContain("ArgumentOutOfRange", message, StringComparison.Ordinal);

        // one failure, one sentence: the diagnostic the caller also reads carries the same refusal
        string diagnostic = envelope["diagnostics"]!.AsArray()[0]!["message"]!.GetValue<string>();
        Assert.DoesNotContain("Parameter", diagnostic, StringComparison.Ordinal);
        Assert.Contains(OverCapDigits.ToString(), diagnostic, StringComparison.Ordinal);
        Assert.Contains(EnginePrecision.ToString(), diagnostic, StringComparison.Ordinal);
    }

    // ------------------------------------------------------------------
    // 2. the SAME request at the default precision is still answered, in full
    // ------------------------------------------------------------------

    /// <summary>
    /// The cap is not a refusal of the request: 500 places are inside the DEFAULT engine's precision
    /// (<c>Interpreter.cs:160</c>), so the same call must keep answering 500 real digits — the head
    /// and the tail are cross-checked against mpmath (<c>mp.dps = 620</c>) so a truncated or padded
    /// answer cannot pass as the fix.
    /// </summary>
    [Theory]
    [InlineData("pi", "3.", PiDigits1To60, PiDigits481To500)]
    [InlineData("e", "2.", EDigits1To60, EDigits481To500)]
    public async Task TheSameRequestAtTheDefaultPrecision_IsAnsweredInFull(
        string builtin, string integerPart, string head, string tail)
    {
        string script = $"{builtin}({OverCapDigits})";

        var (exit, envelope, stderr) = await RunAsync(script);

        Assert.True(exit == 0, $"'{script}' exited {exit}. stderr: {stderr}");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Real", structured["kind"]!.GetValue<string>());

        string value = structured["value"]!.GetValue<string>();
        Assert.Equal(OverCapDigits, Decimals(value));
        Assert.StartsWith($"{integerPart}{head}", value, StringComparison.Ordinal);
        Assert.EndsWith(tail, value, StringComparison.Ordinal);
        Assert.False(structured["exact"]!.GetValue<bool>());
    }

    // ------------------------------------------------------------------
    // 3. a request INSIDE the cap after a setprecision still succeeds
    // ------------------------------------------------------------------

    /// <summary>
    /// The boundary the refusal must not swallow: a count AT the engine's precision, one inside it,
    /// and the same 500-place request under a precision the script itself raised past the default —
    /// all three answer, because the cap the refusal reads is the engine's precision and not a
    /// constant.
    /// </summary>
    [Theory]
    [InlineData("setprecision(20); pi(20)", 20, "3.14159265358979323846")]
    [InlineData("setprecision(20); pi(10)", 10, "3.1415926535")]
    [InlineData("setprecision(20); e(20)", 20, "2.71828182845904523536")]
    [InlineData("setprecision(2000); pi(500)", 500, null)]
    public async Task ARequestInsideTheEnginesPrecision_StillSucceeds(string script, int expectedDigits, string? expectedValue)
    {
        var (exit, envelope, stderr) = await RunAsync(script);

        Assert.True(exit == 0, $"'{script}' exited {exit}. stderr: {stderr}");
        Assert.True(envelope["ok"]!.GetValue<bool>());

        string value = RealValue(envelope);
        Assert.Equal(expectedDigits, Decimals(value));
        if (expectedValue is not null)
            Assert.Equal(expectedValue, value);
        else
        {
            // the 500-place request under the raised precision owns its OWN tail
            Assert.StartsWith($"3.{PiDigits1To60}", value, StringComparison.Ordinal);
            Assert.EndsWith(PiDigits481To500, value, StringComparison.Ordinal);
        }
    }

    // ------------------------------------------------------------------
    // the other end of the same range, and a count wider than the type
    // ------------------------------------------------------------------

    /// <summary>
    /// A count below 1 is the same argument problem and crosses the same way — with the DEFAULT
    /// engine's precision as the upper bound, because no <c>setprecision</c> ran. This is the case
    /// the digit-count surface already advertised (<c>pi(0)</c>/<c>e(0)</c>,
    /// <c>Lovelace.Symbolics/SymbolicsPlugin.cs:1941</c>); before this round it only LOOKED advertised,
    /// because the pair the caller received came from the kernel guard.
    /// </summary>
    [Theory]
    [InlineData("pi", "0")]
    [InlineData("pi", "-3")]
    [InlineData("e", "0")]
    [InlineData("e", "-7")]
    public async Task ACountBelowOne_IsTheSameTypedRefusal(string builtin, string digits)
    {
        string script = $"{builtin}({digits})";

        var (exit, envelope, stderr) = await RunAsync(script);

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' must not answer a value: {stderr}");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Equal(ExpectedRefusal(builtin, DefaultEnginePrecision, digits),
            envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// A count wider than Int64 is an argument the builtin cannot honour either, and it must not
    /// cross as the raw CLR conversion message ("Value was either too large or too small for an
    /// Int64." as <c>ArithmeticError</c>/<c>DomainError</c>, which is what it used to be): the count
    /// the caller wrote is the value the refusal names, exactly as <c>evalf</c> already answers it.
    /// </summary>
    [Theory]
    [InlineData("pi")]
    [InlineData("e")]
    public async Task ACountWiderThanInt64_IsTheSameTypedRefusal(string builtin)
    {
        string script = $"{builtin}(10^30)";

        var (exit, envelope, stderr) = await RunAsync(script);

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' must not answer a value: {stderr}");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.Equal(ExpectedRefusal(builtin, DefaultEnginePrecision, DigitsWiderThanInt64),
            envelope["message"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // the shared-state contract, pinned where it IS deterministic
    // ------------------------------------------------------------------

    /// <summary>
    /// THE CONTRACT THE SHARED-STATE OBSERVATION IS READ AGAINST. Engine precision is per-ENGINE
    /// state: one caller's <c>setprecision</c> changes it for that engine's OTHER callers too, which
    /// is why an audit that drove two callers through one engine measured the same script refused
    /// 2/3 times in one run and 6/6 in another — the outcome depends on whose precision the request
    /// happened to meet, and no locking is added here to give that coin a guaranteed face. What IS
    /// deterministic is pinned below: for ONE caller on ONE engine the refusal is stable over
    /// repetitions (statement N reads the precision the statements before it set,
    /// <c>Interpreter.cs:319-321</c>), the code, category and message never vary, and no repetition
    /// can surface the framework's own message.
    /// </summary>
    [Fact]
    public async Task TheRefusalIsStableOverRepetitions_AndNeverAFrameworkMessage()
    {
        for (int repetition = 1; repetition <= 5; repetition++)
        {
            string script = $"setprecision({EnginePrecision}); pi({OverCapDigits})";
            var (exit, envelope, stderr) = await RunAsync(script);

            Assert.Equal(1, exit);
            Assert.False(envelope["ok"]!.GetValue<bool>(), $"repetition {repetition}: {stderr}");
            Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
            Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
            Assert.Equal(ExpectedRefusal("pi", EnginePrecision, OverCapDigits.ToString()),
                envelope["message"]!.GetValue<string>());
        }
    }

    /// <summary>
    /// The other half of the same contract, and the half the published surface already guarantees:
    /// the runner builds ONE ENGINE PER CALL (<c>Lovelace.Run/Runner.cs:180</c>), so the 20-place
    /// engine above belongs to that RUN — the identical 500-place request immediately afterwards is
    /// answered in full. Precision is shared between the callers OF AN ENGINE, never between engines,
    /// so a refusal is never a state a later run inherits.
    /// </summary>
    [Fact]
    public async Task TheEnginePrecisionOfOneRun_DoesNotLeakIntoTheNext()
    {
        var (refusedExit, refused, _) = await RunAsync($"setprecision({EnginePrecision}); pi({OverCapDigits})");
        Assert.Equal(1, refusedExit);
        Assert.False(refused["ok"]!.GetValue<bool>());

        var (answeredExit, answered, stderr) = await RunAsync($"pi({OverCapDigits})");
        Assert.True(answeredExit == 0, $"the next run exited {answeredExit}: {stderr}");
        Assert.Equal(OverCapDigits, Decimals(RealValue(answered)));
    }
}
