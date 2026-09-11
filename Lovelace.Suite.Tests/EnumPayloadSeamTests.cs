using Lovelace.Abstractions;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The Modus seam must carry the Enum kind in BOTH directions: an argument is unwrapped to an
/// <see cref="EnumValue"/> payload for the plugin and the echoed payload is re-wrapped as a
/// <see cref="Value"/>. The echo builtin records the payload it received, so a seam that
/// silently degraded the enum to a string is visible instead of merely "the value still prints".
/// </summary>
public class EnumPayloadSeamTests
{
    private sealed class EnumEchoPlugin : IModusPlugin
    {
        public string Name => "EnumEcho";

        /// <summary>The payload the builtin actually received across the boundary.</summary>
        public object? LastPayload { get; private set; }

        public void Register(IModusContext context) =>
            context.RegisterBuiltin("enum_echo", new[] { "v" }, args =>
            {
                LastPayload = args[0];
                return args[0];
            });
    }

    private static (SuiteEngine Engine, EnumEchoPlugin Echo) NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new SymbolicsPlugin());
        var echo = new EnumEchoPlugin();
        engine.LoadPlugin(echo);
        engine.Evaluate("x = symbol(\"x\")");
        return (engine, echo);
    }

    [Fact]
    public void EnumValue_RoundTripsThroughTheModusSeam()
    {
        var (engine, echo) = NewEngine();

        // the argument crosses as an EnumValue PAYLOAD (not the string "Solved"), and the echoed
        // payload is re-wrapped on the way back
        Value returned = engine.Evaluate("enum_echo(solve_full(x^2 - 4 == 0, x).status)");

        // Wrap then Unwrap preserves BOTH components: the payload that crossed back is the enum
        // itself, not a string that happens to print the same
        var payload = Assert.IsType<EnumValue>(echo.LastPayload);
        Assert.Equal("SolveStatus", payload.TypeName);
        Assert.Equal("Solved", payload.Name);

        Assert.Equal(ValueKind.Enum, returned.Kind);
        var dto = StructuredProjection.ToStructured(returned);
        Assert.Equal("Enum", dto.Kind);
        Assert.Equal("SolveStatus", dto.Type);
        Assert.Equal("Solved", dto.Value);
        Assert.Equal("Solved", ValueFormatter.Format(returned));
        Assert.Equal("Solved", ValueFormatter.FormatTyped(returned));
    }

    /// <summary>The Value API itself: the kind exists, the accessor round-trips both components,
    /// and the new member was APPENDED so the widening lattice's numeric values are untouched.</summary>
    [Fact]
    public void Value_API_CarriesTheEnumKind()
    {
        var value = new Value(new EnumValue("SolveStatus", "Partial"));

        Assert.Equal(ValueKind.Enum, value.Kind);
        Assert.Equal("SolveStatus", value.AsEnum().TypeName);
        Assert.Equal("Partial", value.AsEnum().Name);

        // append-only: Natural=0, Integer=1, Real=2 are load-bearing for WidenPair, and Enum is
        // the LAST member
        Assert.Equal(0, (int)ValueKind.Natural);
        Assert.Equal(1, (int)ValueKind.Integer);
        Assert.Equal(2, (int)ValueKind.Real);
        Assert.Equal(ValueKind.Enum, Enum.GetValues<ValueKind>().Last());
    }

    [Fact]
    public void EnumValues_AreEqualExactlyWhenTypeNameAndNameMatch()
    {
        var (engine, _) = NewEngine();

        // two independently produced values of the same enum type and member are equal
        var solved = engine.Evaluate("solve_full(x^2 - 4 == 0, x).status");
        var solvedAgain = engine.Evaluate("solve_full(x^2 - 4 == 0, x).status");
        Assert.Null(StructuralEquality.FirstDifference(solved, solvedAgain));

        // the SAME member name in two different enum types is NOT equal: SolveStatus.Partial is
        // not Completeness.Partial, which is exactly the confusion the Enum kind exists to stop
        var status = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).status");
        var completeness = engine.Evaluate("solve_full(x^4 - x^2 - 1 == 0, x).completeness");
        string? difference = StructuralEquality.FirstDifference(status, completeness);
        Assert.NotNull(difference);
        Assert.Contains("SolveStatus", difference);
        Assert.Contains("Completeness", difference);
    }
}
