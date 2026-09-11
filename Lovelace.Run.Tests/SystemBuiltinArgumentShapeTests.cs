using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// An argument of the wrong SHAPE — a relation where the builtin requires a list of equations, a
/// bare symbol where it requires a parameter list — is a call-site mistake the caller can fix and
/// retry, exactly like a wrong argument COUNT. The frozen protocol
/// (docs/symbolics/dsh-protocol.md, "Error envelope") says such a call crosses as a recoverable
/// argument error, <c>code: InvalidArgument</c> / <c>category: TypeMismatch</c>, naming the builtin
/// and what it expected, and NEVER as an internal invariant failure.
/// <para>
/// The system-solving builtins used to hard-cast their vector positions
/// (<c>((IReadOnlyList&lt;object?&gt;)args[0]!).Select(AsExpr)</c>), so
/// <c>solve_system(x + y == 2, x - y == 0)</c> — the call form the builtin's own descriptor
/// advertises — died as <c>InternalError / InternalInvariantFailure</c> carrying the raw CLR
/// message "Specified cast is not valid." with <c>recoverable: false</c>. The vector positions of
/// <c>solve_system</c>, <c>solve_system_full</c> and <c>jacobian</c> are pinned here.
/// </para>
/// </summary>
public class SystemBuiltinArgumentShapeTests
{
    /// <summary>Runs the PUBLISHED runner (the same entry point the AOT binary uses) over an
    /// inline script and parses the one JSON envelope it emits.</summary>
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    /// <summary>The declaration prefix every probe shares; the generator bodies are driven with
    /// symbol() so the probe isolates the argument SHAPE.</summary>
    private const string Declare = "x = symbol(\"x\"); y = symbol(\"y\"); ";

    [Theory]
    // the equation-vector position of both system builtins: a relation is not a list
    [InlineData("solve_system(x + y == 2, x - y == 0)", "solve_system", "1", "a list of equations")]
    [InlineData("solve_system_full(x + y == 2, x - y == 0)", "solve_system_full", "1", "a list of equations")]
    // the function-vector position of jacobian: a bare expression is not a list
    [InlineData("jacobian(x*y, [x, y])", "jacobian", "1", "a list of expressions")]
    // the parameter-vector position: a bare symbol is not a list of parameter symbols
    [InlineData("solve_system([x + y == 2], x)", "solve_system", "2", "a list of parameter symbols")]
    [InlineData("solve_system_full([x + y == 2], x)", "solve_system_full", "2", "a list of parameter symbols")]
    [InlineData("jacobian([x*y], x)", "jacobian", "2", "a list of parameter symbols")]
    public async Task WrongShapedArgument_IsARecoverableArgumentError_NeverAnInternalFailure(
        string call, string builtin, string position, string expected)
    {
        var (exitCode, envelope) = await RunAsync(Declare + call);

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), "a rejected call reports ok == false");

        // the classification is structural: a consumer branches on code/category, never on prose
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a wrong argument shape is a user mistake the caller can fix and retry");

        // ... and the message names the builtin, the position and what it expected, plus the
        // payload kind it got, so the caller can fix the call without parsing prose
        string message = envelope["message"]!.GetValue<string>();
        Assert.StartsWith($"{builtin}(): argument {position} must be {expected}", message);
        Assert.Contains("; got ", message);
        Assert.DoesNotContain("cast", message);
    }

    /// <summary>An element that is not an expression at all (here a Record) is the same class of
    /// mistake: the vector position is not a vector of equations, and it must not cross as an
    /// internal failure either.</summary>
    [Fact]
    public async Task ElementThatIsNotAnExpression_IsAlsoAReportedArgumentShapeViolation()
    {
        var (exitCode, envelope) = await RunAsync(Declare + "solve_system([inspect(x^2)], [x, y])");

        Assert.Equal(1, exitCode);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.StartsWith("solve_system(): argument 1 must be a list of equations",
            envelope["message"]!.GetValue<string>());
    }

    /// <summary>The positive control: the declared list form is untouched by the guard, so the
    /// contract is "the wrong shape is refused", not "the builtin is now unreachable".</summary>
    [Theory]
    [InlineData("solve_system([x + y == 2, x - y == 0], [x, y])")]
    [InlineData("solve_system_full([x + y == 2, x - y == 0], [x, y])")]
    [InlineData("jacobian([x*y, x + y], [x, y])")]
    public async Task TheDeclaredListForm_StillSucceeds(string call)
    {
        var (exitCode, envelope) = await RunAsync(Declare + call);

        Assert.True(exitCode == 0, $"'{call}' exited {exitCode}: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{call}' must evaluate");
    }
}
