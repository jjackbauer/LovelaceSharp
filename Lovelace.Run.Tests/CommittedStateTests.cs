using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// ADV-09 — what a failure says about the state it had already COMMITTED.
/// <para>
/// A statement that returned committed its bindings and its files, and a failure in a LATER statement
/// cannot un-commit them. The envelope used to be silent about both on the failure path:
/// <c>partialVariables</c> was filled only for a CANCELLATION — the harnessability benchmark measured its
/// coverage on ordinary failures at 0 % and filed it as F3 — and an artifact written before a failure was
/// on disk with nothing machine-readable pointing at it (round-23 audit N, N-5).
/// </para>
/// </summary>
public class CommittedStateTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script, string? plotDir = null)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        var args = new List<string> { "--eval", script, "--omit-functions" };
        if (plotDir is not null)
        {
            args.Add("--plot-dir");
            args.Add(plotDir);
        }

        int exitCode = await Runner.RunAsync(args.ToArray(), stdout, stderr, new StringReader(string.Empty));
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    private static string[] Names(JsonNode? table) =>
        table is null
            ? Array.Empty<string>()
            : table.AsArray().Select(v => v!["name"]!.GetValue<string>()).ToArray();

    /// <summary>The committed bindings of an incomplete run are readable, in a stable order, each with the
    /// same structured form the success path publishes.</summary>
    [Fact]
    public async Task AFailurePublishesTheCommittedBindings()
    {
        var (exitCode, envelope) = await RunAsync("x = 42\ny = x + 1\ndet(1)");

        Assert.Equal(1, exitCode);
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal(new[] { "x", "y" }, Names(envelope["partialVariables"]));

        // the SHAPE is the one the cancelled path has always published, `structured` included, so a
        // consumer written against that path reads this one unchanged
        JsonNode x = envelope["partialVariables"]!.AsArray()[0]!;
        Assert.Equal("Natural", x["kind"]!.GetValue<string>());
        Assert.NotNull(x["structured"]);
        Assert.Equal("42", x["structured"]!["value"]!.GetValue<string>());
    }

    /// <summary>A failure at the FIRST statement has nothing committed, and says so with an empty array
    /// rather than by omitting the field or inventing a table.</summary>
    [Fact]
    public async Task AFailureBeforeAnyCommitPublishesAnEmptyTable()
    {
        var (_, envelope) = await RunAsync("det(1)");

        Assert.NotNull(envelope["partialVariables"]);
        Assert.Empty(envelope["partialVariables"]!.AsArray());
    }

    /// <summary>A parse error runs NOTHING: the whole source is tokenized and parsed before the first
    /// statement, so the committed table is empty even though the source contains an assignment.</summary>
    [Fact]
    public async Task AParseErrorCommitsNothing()
    {
        var (_, envelope) = await RunAsync("x = 42\nx = (");

        Assert.Equal("ParseError", envelope["code"]!.GetValue<string>());
        Assert.Empty(envelope["partialVariables"]!.AsArray());
    }

    /// <summary>The artifact that N-5 was about: a plot written by a statement that RETURNED, followed by a
    /// failure, leaves the file on disk — and the envelope now names it instead of leaving a consumer to
    /// know the plot-directory convention and go looking.</summary>
    [Fact]
    public async Task AnArtifactWrittenBeforeAFailureIsPublished()
    {
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-adv09-" + Guid.NewGuid().ToString("N")[..8]);
        Directory.CreateDirectory(dir);
        try
        {
            var (exitCode, envelope) = await RunAsync("plot([1, 2, 3])\ndet(1)", dir);

            Assert.Equal(1, exitCode);
            JsonArray artifacts = envelope["artifacts"]!.AsArray();
            JsonNode artifact = Assert.Single(artifacts)!;

            Assert.Equal("plot", artifact["kind"]!.GetValue<string>());
            string path = artifact["path"]!.GetValue<string>();
            Assert.True(File.Exists(path), $"the published artifact must exist on disk: {path}");
            Assert.Equal(new FileInfo(path).Length, artifact["bytes"]!.GetValue<long>());

            // the error envelope still publishes no PLOT block (that is the documented one-plot capture),
            // so `artifacts` is the only machine-readable pointer to the file — which is the point
            Assert.Null(envelope["plot"]);
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }

    [Fact]
    public async Task ARunThatWroteNothingPublishesAnEmptyArtifactArray()
    {
        var (_, envelope) = await RunAsync("1 + 1");

        Assert.True(envelope["ok"]!.GetValue<bool>());
        Assert.NotNull(envelope["artifacts"]);
        Assert.Empty(envelope["artifacts"]!.AsArray());
    }

    [Fact]
    public async Task ASuccessfulPlotPublishesTheSameArtifact()
    {
        string dir = Path.Combine(Path.GetTempPath(), "lovelace-adv09-ok-" + Guid.NewGuid().ToString("N")[..8]);
        Directory.CreateDirectory(dir);
        try
        {
            var (exitCode, envelope) = await RunAsync("plot([1, 2, 3])", dir);

            Assert.Equal(0, exitCode);
            JsonNode artifact = Assert.Single(envelope["artifacts"]!.AsArray())!;
            Assert.Equal("plot", artifact["kind"]!.GetValue<string>());
            Assert.Equal(envelope["plot"]!["path"]!.GetValue<string>(), artifact["path"]!.GetValue<string>());
        }
        finally
        {
            Directory.Delete(dir, recursive: true);
        }
    }
}