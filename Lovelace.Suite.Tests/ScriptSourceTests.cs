using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

public class ScriptSourceTests
{
    [Fact]
    public void ToSemicolonStatements_GivenNewlineSeparatedStatements_JoinsWithSemicolons()
    {
        var result = ScriptSource.ToSemicolonStatements("a = 1\nb = 2\nc = 3");

        Assert.Equal("a = 1;b = 2;c = 3", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenCrlf_NormalizesLineEndings()
    {
        var result = ScriptSource.ToSemicolonStatements("a = 1\r\nb = 2");

        Assert.Equal("a = 1;b = 2", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenBlock_PreservesNewlinesInsideBraces()
    {
        var result = ScriptSource.ToSemicolonStatements("func f(x) {\n  return x\n}");

        Assert.Equal("func f(x) {\n  return x\n}", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenNewlineAfterBlock_EmitsSemicolon()
    {
        var result = ScriptSource.ToSemicolonStatements("func f(x) {\n  return x\n}\ng()");

        Assert.Equal("func f(x) {\n  return x\n};g()", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenStringWithNewline_PreservesIt()
    {
        var result = ScriptSource.ToSemicolonStatements("s = \"a\nb\"");

        Assert.Equal("s = \"a\nb\"", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenTrailingSemicolon_SuppressesBlankSeparator()
    {
        var result = ScriptSource.ToSemicolonStatements("a = 1;\nb = 2");

        Assert.Equal("a = 1; b = 2", result);
    }

    [Fact]
    public void ToSemicolonStatements_GivenLeadingBom_StripsIt()
    {
        var result = ScriptSource.ToSemicolonStatements("\uFEFFa = 1\nb = 2");

        Assert.Equal("a = 1;b = 2", result);
    }

    [Fact]
    public async Task EvaluateAsync_GivenMultiLineScript_PlotsOneOverXSquared()
    {
        var engine = new SuiteEngine();
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-multiline-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        try
        {
            engine.PlotOutputDirectory = dir;
            engine.PlotFileName = "invx2.svg";

            const string source = "x = 1..10\ny = 1 / x^2\nplot(x, y, \"1/x^2\")";

            var result = await engine.EvaluateAsync(ScriptSource.ToSemicolonStatements(source));

            Assert.Equal(ValueKind.Text, result.Kind);
            Assert.NotNull(engine.LastPlot);
            Assert.Equal("1/x^2", engine.LastPlot!.Title);

            var y = engine.Variables["y"].AsVector();
            Assert.Equal(10, y.Count);
            Assert.Equal(1.0, PlotValue.ToDouble(y[0]), 12);
            Assert.Equal(0.25, PlotValue.ToDouble(y[1]), 12);
            Assert.Equal(0.01, PlotValue.ToDouble(y[9]), 12);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}
