using System.Text;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

public class InterpolationPrintTests
{
    [Fact]
    public async Task Evaluate_GivenInterpolatedString_FormatsEmbeddedExpression()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("$\"x = {3 + 4}\"");

        Assert.Equal(ValueKind.Text, result.Kind);
        Assert.Equal("x = 7", result.AsText());
    }

    [Fact]
    public async Task Evaluate_GivenInterpolatedStringWithVariable_SubstitutesValue()
    {
        var engine = new SuiteEngine();
        await engine.EvaluateAsync("name = 42");

        var result = await engine.EvaluateAsync("$\"value is {name}\"");

        Assert.Equal("value is 42", result.AsText());
    }

    [Fact]
    public async Task Evaluate_GivenEscapedBraces_ProducesLiteralBraces()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("$\"set {{1, 2}}\"");

        Assert.Equal("set {1, 2}", result.AsText());
    }

    [Fact]
    public async Task Evaluate_GivenPlainString_ReturnsText()
    {
        var engine = new SuiteEngine();

        var result = await engine.EvaluateAsync("\"hello\"");

        Assert.Equal(ValueKind.Text, result.Kind);
        Assert.Equal("hello", result.AsText());
    }

    [Fact]
    public async Task Evaluate_GivenPrint_WritesRenderedValueAndReturnsVoid()
    {
        var engine = new SuiteEngine();
        var sb = new StringBuilder();
        engine.Output = new StringWriter(sb);

        var result = await engine.EvaluateAsync("print($\"hi {1..3}\")");

        Assert.Equal(ValueKind.Void, result.Kind);
        Assert.Equal("hi [1, 2, 3]", sb.ToString().Trim());
    }
}
