using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The ONE call-site shape guard (audit D, finding F1). The regression suite drives the shipped
/// registry end to end (Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests.cs); these cases pin the
/// MECHANISM: a builtin registered here, that the engine has never seen, gets the same attribution
/// as <c>det</c> — so closing F1 is one guard rather than 26 patched call sites — and a coercion
/// failure on an ENGINE-INTERNAL value keeps crossing as the internal failure it is instead of
/// being disguised as the caller's argument error.
/// <para>
/// The assertions read the exception the ENGINE raises and never name the guard's own type, so this
/// file also COMPILES against the pre-fix tree — where it fails on the behaviour, not on a missing
/// symbol (the same discipline as Lovelace.Run.Tests/ArityPropertyTests.cs:56-86).
/// </para>
/// </summary>
public class BuiltinArgumentShapeGuardTests
{
    private static SuiteEngine EngineWith(string name, Func<IReadOnlyList<Value>, Value> body)
    {
        var engine = new SuiteEngine();
        engine.RegisterBuiltin(name, new[] { "a", "b" }, body);
        return engine;
    }

    /// <summary>The documented grammar, produced by a builtin that exists only in this test: the
    /// body reads its SECOND argument through the shared <see cref="Value"/> accessor, so the guard
    /// names the builtin, the 1-based position and the kind that arrived — and the failure reaches
    /// the runner as an argument error (an <see cref="ArgumentException"/>), which the taxonomy
    /// classifies as the documented <c>InvalidArgument</c>/<c>TypeMismatch</c>.</summary>
    [Fact]
    public void ARegisteredBuiltinThatCoercesItsArgument_IsAttributedToTheBuiltinAndThePosition()
    {
        var engine = EngineWith("probe", args => new Value(args[1].AsArrayValue().Numel.ToString()));

        var error = Record.Exception(() => engine.Evaluate("probe(1, 2)"));

        Assert.NotNull(error);
        Assert.Equal("probe(): argument 2 must be an array or vector; got Natural.", error!.Message);
        Assert.DoesNotContain("cast", error.Message);
        Assert.IsAssignableFrom<ArgumentException>(error);
    }

    /// <summary>A builtin whose BODY casts the payload itself — the shape the 52 F1 calls had — is
    /// attributed to the argument the body was working on instead of escaping as the raw CLR cast
    /// the runner can only report as an internal invariant failure.</summary>
    [Fact]
    public void ARegisteredBuiltinThatCastsItsOwnPayload_IsAttributedToTheArgumentItReadLast()
    {
        var engine = EngineWith("probe", args => new Value((string)(object)args[1]!));

        var error = Record.Exception(() => engine.Evaluate("probe(1, 2)"));

        Assert.NotNull(error);
        Assert.Equal("probe(): argument 2 must be a value this builtin can use; got Natural.", error!.Message);
        Assert.IsAssignableFrom<ArgumentException>(error);
    }

    /// <summary>The guard converts a coercion failure only when the value that failed it IS one of
    /// the call's arguments. A value the body built itself is an engine-internal value: its
    /// coercion failure is not a caller mistake, so it keeps crossing as the internal failure it
    /// was before (an <see cref="InvalidCastException"/> subclass, which the runner reports as
    /// <c>InternalError</c>/<c>InternalInvariantFailure</c>) rather than being disguised as an
    /// argument error.</summary>
    [Fact]
    public void ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError()
    {
        var engine = EngineWith("bug", args =>
        {
            _ = args[0];   // the body DID read an argument: the rule is about the failing value
            return new Value(Value.Void.AsArrayValue().Numel.ToString());
        });

        var error = Record.Exception(() => engine.Evaluate("bug(1, 2)"));

        Assert.NotNull(error);
        Assert.IsAssignableFrom<InvalidCastException>(error!);
        Assert.DoesNotContain("argument", error!.Message);
    }
}
