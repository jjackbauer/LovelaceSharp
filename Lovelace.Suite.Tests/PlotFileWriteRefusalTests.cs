using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// F3-C (docs/goal-cycle-6/round-13/audit-C-hostile.md:136-172): a plot WRITE the process cannot perform —
/// <c>--plot-file nope/x.svg</c> under a plot directory that exists, or <c>--plot-file ""</c> — crossed as
/// <c>InternalError/InternalInvariantFailure</c> (<c>recoverable:false</c>) with the raw CLR text
/// "Could not find a part of the path …" / "Access to the path … is denied.", while the READ side of the same
/// class (<c>--file missing</c>) is the recoverable <c>FileReadError</c>. The write site is
/// <c>File.WriteAllText(full, svg)</c> in <c>Interpreter.BuiltinPlot</c>; the refusal is a property of the
/// caller-supplied output path, exactly like the plot DIRECTORY refusal the runner already types as
/// <c>PlotDirectoryError/TypeMismatch</c> (Lovelace.Run/Runner.cs:181-192).
///
/// The refusal type is named through reflection on purpose: this file must still COMPILE in a tree that has
/// not implemented the typed refusal (the control run), where it then fails at RUNTIME.
/// </summary>
public class PlotFileWriteRefusalTests
{
    private const string RefusalTypeName = "Lovelace.Suite.PlotFileWriteException";

    private static Type? RefusalType() =>
        typeof(SuiteEngine).Assembly.GetType(RefusalTypeName, throwOnError: false);

    [Fact]
    public void TheSuiteDeclaresThePlotFileWriteRefusalTheRunnerClassifies()
    {
        Assert.NotNull(RefusalType());
    }

    [Fact]
    public async Task PlotFileUnderAMissingDirectory_IsRefusedByType()
    {
        await AssertPlotRefusedAsync(Path.Combine("nope", "x.svg"));
    }

    [Fact]
    public async Task EmptyPlotFile_IsRefusedByType()
    {
        await AssertPlotRefusedAsync(string.Empty);
    }

    /// <summary>The success path is not allowed to regress: a writable plot file is still written and the
    /// returned Text still names it.</summary>
    [Fact]
    public async Task WritablePlotFile_IsStillWritten()
    {
        string root = Path.Combine(Path.GetTempPath(), "lovelace-plotfile-ok-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var engine = new SuiteEngine { PlotOutputDirectory = root, PlotFileName = "ok.svg" };
            Value result = await engine.EvaluateAsync("plot([1, 2, 3])");

            Assert.Equal(ValueKind.Text, result.Kind);
            string path = result.AsText();
            Assert.Equal(Path.GetFullPath(Path.Combine(root, "ok.svg")), path);
            Assert.True(File.Exists(path), $"the plot file must exist at {path}");
            Assert.Contains("<svg", await File.ReadAllTextAsync(path), StringComparison.OrdinalIgnoreCase);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }

    private static async Task AssertPlotRefusedAsync(string plotFileName)
    {
        string root = Path.Combine(Path.GetTempPath(), "lovelace-plotfile-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(root);
        try
        {
            var engine = new SuiteEngine { PlotOutputDirectory = root, PlotFileName = plotFileName };
            Exception? failure = await Record.ExceptionAsync(() => engine.EvaluateAsync("plot([1, 2, 3])"));

            Assert.NotNull(failure);
            Assert.Equal(RefusalTypeName, failure!.GetType().FullName);
            // the refusal names the path the caller asked for, so a consumer can act on it
            Assert.Contains(Path.Combine(root, plotFileName), failure.Message);
        }
        finally
        {
            Directory.Delete(root, recursive: true);
        }
    }
}
