using Lovelace.Abstractions;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// DX-convergence Phase 2 contract tests: the structured Record value kind, member access,
/// rendering, and the Modus payload channel.
/// </summary>
public class RecordValueTests
{
    private static SuiteEngine MakeEngine()
    {
        var engine = new SuiteEngine();
        engine.RegisterBuiltin("maker", ["n"], args =>
            new Value(new RecordValue("DemoResult",
                new RecordField("count", new Value(args[0].AsNatural())),
                new RecordField("label", new Value("hello")),
                new RecordField("nested", new Value(new RecordValue("Inner", new RecordField("flag", new Value(true))))))));
        return engine;
    }

    [Fact]
    public void Record_ReturnsThroughEngine_WithRecordKind()
    {
        var engine = MakeEngine();
        var result = engine.Evaluate("maker(3)");
        Assert.Equal(ValueKind.Record, result.Kind);
        Assert.Equal("DemoResult", result.AsRecord().TypeName);
    }

    [Fact]
    public void MemberAccess_ReadsFields()
    {
        var engine = MakeEngine();
        engine.Evaluate("r = maker(3)");
        Assert.Equal(ValueKind.Natural, engine.Evaluate("r.count").Kind);
        Assert.Equal(new global::Lovelace.Natural.Natural(3), engine.Evaluate("r.count").AsNatural());
        Assert.Equal("hello", engine.Evaluate("r.label").AsText());
        Assert.True(engine.Evaluate("r.nested.flag").AsBoolean());
    }

    [Fact]
    public void MemberAccess_UnknownField_NamesAvailableMembers()
    {
        var engine = MakeEngine();
        engine.Evaluate("r = maker(3)");
        var ex = Assert.Throws<InvalidOperationException>(() => engine.Evaluate("r.nope"));
        Assert.Contains("DemoResult", ex.Message);
        Assert.Contains("count", ex.Message);
        Assert.Contains("label", ex.Message);
    }

    [Fact]
    public void MemberAccess_OnNonRecord_ExplainsRequirement()
    {
        var engine = MakeEngine();
        engine.Evaluate("v = [1, 2, 3]");
        var ex = Assert.Throws<InvalidOperationException>(() => engine.Evaluate("v.length"));
        Assert.Contains("member access requires a record result", ex.Message);
    }

    [Fact]
    public void Record_Formatter_RendersTypeNameAndFields()
    {
        var engine = MakeEngine();
        var result = engine.Evaluate("maker(3)");
        Assert.Equal("DemoResult(count: 3, label: hello, nested: Inner(flag: True))",
            ValueFormatter.Format(result));
        Assert.Equal("DemoResult(count: 3, label: hello, nested: Inner(flag: True)) (DemoResult)",
            ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public void Record_CrossesModusPluginBoundary()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new RecordPlugin());
        var result = engine.Evaluate("plug()");
        Assert.Equal(ValueKind.Record, result.Kind);
        Assert.Equal("PluginResult", result.AsRecord().TypeName);
        Assert.Equal(new global::Lovelace.Natural.Natural(5), engine.Evaluate("plug().answer").AsNatural());
    }

    private sealed class RecordPlugin : IModusPlugin
    {
        public string Name => "RecordTestPlugin";

        public void Register(IModusContext context) =>
            context.RegisterBuiltin("plug", [], _ =>
                new RecordValue("PluginResult", new RecordField("answer", new Value(new global::Lovelace.Natural.Natural(5)))));
    }
}
