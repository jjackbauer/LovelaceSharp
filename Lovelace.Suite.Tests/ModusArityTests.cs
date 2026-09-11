using Lovelace.Abstractions;
using Lovelace.Suite;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The declared arity of every builtin is enforced ONCE, in the host, at the three registration
/// sites that build a handler (the two descriptor-carrying overloads and the parameters-only
/// overload). The descriptor is the one source of truth: <c>Parameters.Count</c> is the upper
/// bound, <c>MinArity</c> the lower bound (<c>-1</c> meaning exactly the declared count) and
/// <c>Variadic</c> removes the upper bound so the last parameter may repeat.
/// </summary>
public class ModusArityTests
{
    private sealed class ArityPlugin : IModusPlugin
    {
        public string Name => "ArityProbe";

        public void Register(IModusContext context)
        {
            // descriptor overload, raw-object channel
            context.RegisterBuiltin(new BuiltinDescriptor("pair", new[] { "a", "b" }, BuiltinCategories.Core,
                "Exactly two arguments.", Array.Empty<string>(), "Text"), args => "pair:" + args.Count);

            // descriptor overload, optional trailing parameter
            context.RegisterBuiltin(new BuiltinDescriptor("opt", new[] { "a", "b" }, BuiltinCategories.Core,
                "An optional trailing b.", Array.Empty<string>(), "Text", MinArity: 1), args => "opt:" + args.Count);

            // descriptor overload, typed ScalarResult channel, variadic tail
            context.RegisterBuiltin(new BuiltinDescriptor("variadic_min", new[] { "a", "b" }, BuiltinCategories.Core,
                "At least one argument; the tail repeats.", Array.Empty<string>(), "Text",
                Variadic: true, MinArity: 1), args => ScalarResult.FromText("variadic_min:" + args.Count));

            // descriptor overload, variadic with the default lower bound
            context.RegisterBuiltin(new BuiltinDescriptor("variadic_two", new[] { "a", "b" }, BuiltinCategories.Core,
                "At least two arguments; the tail repeats.", Array.Empty<string>(), "Text",
                Variadic: true), args => "variadic_two:" + args.Count);

            // parameters-only overload: no descriptor, so the declared list is the exact count
            context.RegisterBuiltin("plain", new[] { "a", "b" }, args => "plain:" + args.Count);
        }
    }

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new ArityPlugin());
        return engine;
    }

    private static string Message(Action call) => Assert.ThrowsAny<Exception>(call).Message;

    [Fact]
    public void ExactCount_RequiresTheDeclaredCount()
    {
        var engine = NewEngine();
        Assert.Equal("pair:2", engine.Evaluate("pair(1, 2)").AsText());

        var tooFew = Assert.ThrowsAny<Exception>(() => engine.Evaluate("pair(1)"));
        Assert.Equal("pair(): expected 2 arguments; got 1.", tooFew.Message);
        // typed: the runner's taxonomy classifies an argument error, never an internal invariant
        Assert.IsAssignableFrom<ArgumentException>(tooFew);

        Assert.Equal("pair(): expected 2 arguments; got 3.",
            Message(() => engine.Evaluate("pair(1, 2, 3)")));
    }

    [Fact]
    public void ParametersOnlyRegistration_UsesTheSameRule()
    {
        var engine = NewEngine();
        Assert.Equal("plain:2", engine.Evaluate("plain(1, 2)").AsText());
        Assert.Equal("plain(): expected 2 arguments; got 1.",
            Message(() => engine.Evaluate("plain(1)")));
    }

    [Fact]
    public void MinArity_AllowsTheOptionalTail_AndStillRefusesLess()
    {
        var engine = NewEngine();
        Assert.Equal("opt:1", engine.Evaluate("opt(1)").AsText());
        Assert.Equal("opt:2", engine.Evaluate("opt(1, 2)").AsText());
        Assert.Equal("opt(): expected 1 to 2 arguments; got 0.",
            Message(() => engine.Evaluate("opt()")));
    }

    [Fact]
    public void Variadic_RemovesTheUpperBound_AndHonoursTheLowerOne()
    {
        var engine = NewEngine();
        Assert.Equal("variadic_min:1", engine.Evaluate("variadic_min(1)").AsText());
        Assert.Equal("variadic_min:4", engine.Evaluate("variadic_min(1, 2, 3, 4)").AsText());
        Assert.Equal("variadic_min(): expected at least 1 argument; got 0.",
            Message(() => engine.Evaluate("variadic_min()")));

        // MinArity -1 keeps the declared count as the LOWER bound
        Assert.Equal("variadic_two:2", engine.Evaluate("variadic_two(1, 2)").AsText());
        Assert.Equal("variadic_two:5", engine.Evaluate("variadic_two(1, 2, 3, 4, 5)").AsText());
        Assert.Equal("variadic_two(): expected at least 2 arguments; got 1.",
            Message(() => engine.Evaluate("variadic_two(1)")));
    }
}
