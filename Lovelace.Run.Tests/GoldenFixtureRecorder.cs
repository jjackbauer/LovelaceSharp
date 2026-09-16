using System.Text.Json;
using System.Text.Json.Nodes;
using Xunit;
using Xunit.Abstractions;

namespace Lovelace.Run.Tests;

/// <summary>
/// Re-records the golden envelope fixtures from the current runner.
/// <para>
/// The goldens pin the byte shape of the agent-facing wire, so they are SUPPOSED to fail when the
/// contract legitimately changes — and then somebody has to re-record them, and the way that was done
/// was by hand, which is how a golden ends up holding a value the runner no longer produces. This puts
/// the recorder next to the comparison, using the SAME <see cref="TestSupport.Normalise"/> the comparison
/// uses, so the two cannot disagree about what is volatile.
/// </para>
/// <para>
/// It is inert unless <c>LOVELACE_RECORD_GOLDENS=1</c> is set, and it is a <c>Fact</c> rather than a
/// skipped theory so that setting the variable is the whole interface:
/// <code>
///   $env:LOVELACE_RECORD_GOLDENS=1; dotnet test --filter RecordGoldenFixtures
/// </code>
/// A re-record is a REVIEWABLE diff: the test prints every file it rewrote, and the reviewer reads the
/// diff rather than trusting the recorder. Adding a fixture is still caught by
/// <c>GoldenEnvelopeTests.EveryFixtureScriptHasACorpusRow</c>.
/// </para>
/// </summary>
public class GoldenFixtureRecorder
{
    private readonly ITestOutputHelper _output;

    public GoldenFixtureRecorder(ITestOutputHelper output) => _output = output;

    [Fact]
    public async Task RecordGoldenFixtures()
    {
        if (Environment.GetEnvironmentVariable("LOVELACE_RECORD_GOLDENS") != "1")
        {
            _output.WriteLine(
                "inert: set LOVELACE_RECORD_GOLDENS=1 to re-record the golden envelope fixtures");
            return;
        }

        foreach (var row in GoldenEnvelopeTests.Corpus)
        {
            string name = (string)row[0]!;
            var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync(name);
            Assert.True(stdout.Length > 0, $"'{name}' produced no envelope. stderr: {stderr.Trim()}");

            JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{name}'");
            JsonNode normalised = TestSupport.Normalise(envelope)!;
            // UnsafeRelaxedJsonEscaping, because the goldens are READ BY PEOPLE in diffs: the default
            // encoder writes the volatile placeholder as \u003Cvolatile\u003E, which is the same JSON but
            // an unreadable golden. The fixtures on disk use the literal characters, so the recorder must
            // too — otherwise a re-record rewrites every existing golden for no reason at all.
            string json = normalised.ToJsonString(new JsonSerializerOptions
            {
                WriteIndented = true,
                Encoder = System.Text.Encodings.Web.JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
            });

            File.WriteAllText(SourceGoldenPath(name), json + Environment.NewLine);
            _output.WriteLine($"{name}.json rewritten (exit {exitCode})");
        }
    }

    /// <summary>
    /// The golden in the SOURCE tree, not the copy the build staged next to the test assembly. The
    /// comparison reads the staged copy (that is what runs), but a re-record has to reach the file a
    /// reviewer will see in the diff — which is the first reason this helper exists rather than a bare
    /// <c>TestSupport.GoldenPath</c>.
    /// </summary>
    private static string SourceGoldenPath(string name)
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory is not null && !File.Exists(Path.Combine(directory.FullName, "Lovelace.Run.Tests.csproj")))
            directory = directory.Parent;

        Assert.True(directory is not null,
            $"could not locate Lovelace.Run.Tests.csproj above {AppContext.BaseDirectory}");

        return Path.Combine(directory!.FullName, "fixtures", name + ".json");
    }
}
