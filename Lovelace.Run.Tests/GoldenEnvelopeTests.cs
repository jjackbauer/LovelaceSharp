using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// The agent-facing wire contract of Lovelace.Run, pinned by versioned golden fixtures under
/// fixtures/: one .ls script plus one .json golden per corpus row. Each golden is the envelope
/// with every volatile value (revision, elapsed, elapsedTime, timings) replaced by the literal
/// string "&lt;volatile&gt;", pretty-printed with two-space indentation.
/// </summary>
public class GoldenEnvelopeTests
{
    /// <summary>
    /// Corpus row -> the process exit code the runner documents for it. The script itself is
    /// fixtures/&lt;name&gt;.ls, so a golden and its script can never drift apart.
    /// </summary>
    public static TheoryData<string, int> Corpus => new()
    {
        { "symbolic", 0 },
        { "vector", 0 },
        { "array", 0 },
        { "record", 0 },
        { "nested_record", 0 },
        { "solve_result", 0 },
        { "solve_nosolutions", 0 },
        { "transform_result", 0 },
        { "limit_result", 0 },
        { "system_solve", 0 },
        { "integration_result", 0 },
        { "optimization_result", 0 },
        { "compilation", 0 },
        { "complex", 0 },
        // Round 6: the two matrix records now cross the wire with the solve vocabulary, so they
        // are pinned like every other record the runner can emit
        { "matrix_inverse_result", 0 },
        { "matrix_solve_result", 0 },
        { "absent_field", 0 },
        // Round 22: the capability statement is part of the published protocol
        { "capabilities", 0 },
        { "error_envelope", 1 },
        { "print_purity", 0 },
    };

    [Theory]
    [MemberData(nameof(Corpus))]
    public async Task EnvelopeMatchesGoldenFixture(string name, int expectedExitCode)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync(name);
        Assert.True(exitCode == expectedExitCode,
            $"'{name}' exited {exitCode}, expected {expectedExitCode}. stderr: {stderr.Trim()}");

        JsonNode actual = TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{name}'");

        string goldenPath = TestSupport.GoldenPath(name);
        Assert.True(File.Exists(goldenPath), $"missing golden fixture: {goldenPath}");
        JsonNode expected = TestSupport.ParseExactlyOneJsonDocument(
            File.ReadAllText(goldenPath), $"golden {name}.json");

        TestSupport.AssertSameJson(TestSupport.Normalise(expected), TestSupport.Normalise(actual), name);
    }

    /// <summary>
    /// Semantic assertions that survive a deliberate envelope revision: they describe what a
    /// complete solve MEANS (kind, type, status, domain, solution shape) rather than the bytes
    /// the golden happens to hold.
    /// </summary>
    [Fact]
    public async Task SolveRecordCarriesTheStructuralSolveContract()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync("record");
        Assert.True(exitCode == 0, $"record exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "record envelope");
        Assert.True(envelope["ok"]!.GetValue<bool>(), "a complete solve reports ok");

        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("SolveResult", structured["type"]!.GetValue<string>());

        JsonObject fields = TestSupport.FieldsByName(structured);
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        // D2: the KIND is the contract — an agent must not have to guess SolveStatus from the
        // spelling of "Solved". The declared enum type name rides along in "type".
        Assert.Equal("Enum", fields["status"]!["kind"]!.GetValue<string>());
        Assert.Equal("SolveStatus", fields["status"]!["type"]!.GetValue<string>());
        Assert.Equal("Enum", fields["completeness"]!["kind"]!.GetValue<string>());
        Assert.Equal("Completeness", fields["completeness"]!["type"]!.GetValue<string>());
        Assert.Equal("Complete", fields["completeness"]!["value"]!.GetValue<string>());
        Assert.Equal("complex", fields["domain"]!["domain"]!.GetValue<string>());

        JsonNode solutions = fields["solutions"]!;
        Assert.Equal("Array", solutions["kind"]!.GetValue<string>());
        JsonArray shape = solutions["shape"]!.AsArray();
        Assert.Equal(new[] { 2 }, shape.Select(n => n!.GetValue<int>()).ToArray());

        JsonArray elements = solutions["elements"]!.AsArray();
        Assert.Equal(2, elements.Count);
        foreach (JsonNode? solution in elements)
        {
            Assert.Equal("Record", solution!["kind"]!.GetValue<string>());
            Assert.Equal("Solution", solution["type"]!.GetValue<string>());
            JsonObject solutionFields = TestSupport.FieldsByName(solution);
            Assert.Equal("Enum", solutionFields["exactness"]!["kind"]!.GetValue<string>());
            Assert.Equal("SolutionExactness", solutionFields["exactness"]!["type"]!.GetValue<string>());
        }
    }

    /// <summary>
    /// A failed evaluation is structural: exit code 1, ok == false, and a non-empty code and
    /// category (message matching is never required by a consumer).
    /// </summary>
    [Fact]
    public async Task ErrorEnvelopeCarriesAStructuralCodeAndCategory()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync("error_envelope");
        Assert.True(exitCode == 1, $"error_envelope exited {exitCode}, expected 1");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "error envelope");
        Assert.False(envelope["ok"]!.GetValue<bool>(), "a failed evaluation reports ok == false");
        Assert.False(string.IsNullOrWhiteSpace(envelope["code"]!.GetValue<string>()), "code must be a non-empty string");
        Assert.False(string.IsNullOrWhiteSpace(envelope["category"]!.GetValue<string>()), "category must be a non-empty string");

        // in --json mode the envelope is the whole story: the error is not duplicated on stderr
        Assert.Equal(string.Empty, stderr);
    }

    /// <summary>
    /// The nine rich result records, each driven through its own builtin, and the fixture that
    /// emits it. Every one of them ends with a "diagnostics" field that crosses the wire as an
    /// ARRAY: a free-text diagnostics value (D1) fails here, in the published envelope itself.
    /// </summary>
    public static TheoryData<string, string> RichResultFixtures => new()
    {
        { "record", "SolveResult" },
        { "system_solve", "SystemSolveResult" },
        // the two matrix records joined this vocabulary in Round 6: their diagnostics used to be
        // the prose "matrix is singular" (and "" when solved)
        { "matrix_inverse_result", "MatrixInverseResult" },
        { "matrix_solve_result", "MatrixSolveResult" },
        { "limit_result", "LimitResult" },
        { "integration_result", "IntegrationResult" },
        { "transform_result", "TransformResult" },
        { "optimization_result", "OptimizationResult" },
        { "compilation", "CompilationResult" },
    };

    [Theory]
    [MemberData(nameof(RichResultFixtures))]
    public async Task RichResultDiagnostics_CrossTheWireAsAnArray(string fixture, string recordType)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync(fixture);
        Assert.True(exitCode == 0, $"'{fixture}' exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{fixture}'");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal(recordType, structured["type"]!.GetValue<string>());

        JsonArray ordered = structured["fields"]!.AsArray();
        Assert.Equal("diagnostics", ordered[^1]!["name"]!.GetValue<string>());

        JsonNode diagnostics = TestSupport.FieldsByName(structured)["diagnostics"]!;
        Assert.Equal("Array", diagnostics["kind"]!.GetValue<string>());
        // nothing on the wire still carries a free-text diagnostics value
        Assert.Null(diagnostics["value"]);
        Assert.NotNull(diagnostics["elements"]);
    }

    /// <summary>
    /// The Diagnostic form itself: a Record named "Diagnostic" whose six fields are exactly
    /// code, category, message, recoverable, location, details — in that order. category is an
    /// ErrorCategory ENUM (never a string), location is Null at kernel level, details is an Array.
    /// </summary>
    [Fact]
    public async Task WireDiagnostic_CarriesTheSixFrozenFieldsInOrder()
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunFixtureAsync("solve_result");
        Assert.True(exitCode == 0, $"solve_result exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "solve_result envelope");
        JsonNode structured = envelope["result"]!["structured"]!;
        Assert.Equal("Partial", TestSupport.FieldsByName(structured)["status"]!["value"]!.GetValue<string>());

        JsonNode diagnostics = TestSupport.FieldsByName(structured)["diagnostics"]!;
        Assert.Equal("Array", diagnostics["kind"]!.GetValue<string>());
        JsonArray elements = diagnostics["elements"]!.AsArray();
        Assert.True(elements.Count >= 1, "the golden partial solve carries at least one Diagnostic");

        JsonNode diagnostic = elements[0]!;
        Assert.Equal("Record", diagnostic["kind"]!.GetValue<string>());
        Assert.Equal("Diagnostic", diagnostic["type"]!.GetValue<string>());
        Assert.Equal(
            new[] { "code", "category", "message", "recoverable", "location", "details" },
            diagnostic["fields"]!.AsArray().Select(f => f!["name"]!.GetValue<string>()).ToArray());

        JsonObject fields = TestSupport.FieldsByName(diagnostic);
        Assert.False(string.IsNullOrWhiteSpace(fields["code"]!["value"]!.GetValue<string>()));
        Assert.False(string.IsNullOrWhiteSpace(fields["message"]!["value"]!.GetValue<string>()));
        Assert.Equal("Enum", fields["category"]!["kind"]!.GetValue<string>());
        Assert.Equal("ErrorCategory", fields["category"]!["type"]!.GetValue<string>());
        Assert.Equal("Boolean", fields["recoverable"]!["kind"]!.GetValue<string>());
        Assert.Equal("Null", fields["location"]!["kind"]!.GetValue<string>());
        Assert.Equal("Array", fields["details"]!["kind"]!.GetValue<string>());
        Assert.Empty(fields["details"]!["elements"]!.AsArray());
    }

    /// <summary>
    /// Guards the corpus itself: every fixture script has a row in <see cref="Corpus"/> and every
    /// row has a script, so a new fixture cannot be added without being pinned.
    /// </summary>
    [Fact]
    public void EveryFixtureScriptHasACorpusRow()
    {
        Assert.True(Directory.Exists(TestSupport.FixturesDirectory),
            $"missing fixture directory: {TestSupport.FixturesDirectory}");

        string[] onDisk = Directory.GetFiles(TestSupport.FixturesDirectory, "*.ls")
            .Select(Path.GetFileNameWithoutExtension)
            .Select(name => name!)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();
        string[] inCorpus = Corpus.Select(row => (string)row[0]!)
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToArray();

        Assert.Equal(inCorpus, onDisk);
    }
}
