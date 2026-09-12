using System.Text.Json.Nodes;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// The cycle-5 audit findings on the Symbolics builtin surface, each pinned through the PUBLISHED
/// machine API (the Lovelace.Run envelope), because that is the surface the findings are about:
/// <list type="number">
/// <item>audit A1-N-10 / A2-F25 — <c>evalf(f, digits)</c> ignored <c>digits</c> for an argument that
/// is already numeric, so <c>evalf(sqrt(2), 5)</c> answered with a 100-digit Real.</item>
/// <item>audit A4-P1-6 — <c>assumptions()</c> returned a bare Text value, so the assumption set was
/// recoverable only by parsing a display string.</item>
/// <item>audit A2-F5 — the short form <c>solve(f, x)</c> returned a prose Text SENTENCE as its
/// result whenever it could not represent the whole solution set.</item>
/// <item>audit A2-F12 — <c>capabilities()</c> listed <c>integer</c> among the unsupported DOMAINS
/// while <c>symbol("x", integer)</c> succeeds and a solution family's parameter domain IS integer.</item>
/// <item>audit A4-F1/A4-F4 — two examples in docs/symbolics/dsh-protocol.md contradicted the binary
/// (the canonical integer atom, and an error envelope without the structural durations invariant 4
/// requires).</item>
/// </list>
/// </summary>
public class BuiltinSurfaceContractTests
{
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables" }, stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    private static JsonNode Structured(JsonNode envelope) => envelope["result"]!["structured"]!;

    private static JsonObject Fields(JsonNode structured) => TestSupport.FieldsByName(structured);

    private static string Text(JsonNode field) => field["value"]!.GetValue<string>();

    // ------------------------------------------------------------------
    // 1. evalf honours the digit count for an already-numeric argument
    // ------------------------------------------------------------------

    /// <summary>
    /// The interpreter evaluates an argument BEFORE the builtin body runs, so <c>sqrt(2)</c> reaches
    /// <c>evalf</c> as a numeric value computed at the AMBIENT precision. The documented contract
    /// ("numerically evaluates a symbolic expression to the given number of digits") has to hold for
    /// that shape too: the value is truncated to the requested count instead of being passed through
    /// at 100 digits.
    /// </summary>
    [Fact]
    public async Task Evalf_HonoursTheDigitCount_ForAnAlreadyNumericArgument()
    {
        var (exit, envelope) = await RunAsync("evalf(sqrt(2), 5)");
        Assert.Equal(0, exit);
        JsonNode structured = Structured(envelope);
        Assert.Equal("Real", structured["kind"]!.GetValue<string>());
        Assert.Equal("1.41421", Text(structured));

        // pi arrives numeric as well (the core constant is evaluated eagerly): the count holds there
        var (piExit, piEnvelope) = await RunAsync("evalf(pi, 5)");
        Assert.Equal(0, piExit);
        Assert.Equal("3.14159", Text(Structured(piEnvelope)));

        // the symbolic path already honoured it; the two must answer alike
        var (thirdExit, thirdEnvelope) = await RunAsync("evalf(1/3, 5)");
        Assert.Equal(0, thirdExit);
        Assert.Equal("0.33333", Text(Structured(thirdEnvelope)));

        // a larger request is answered with exactly that many digits, not with the ambient 100
        var (fiftyExit, fiftyEnvelope) = await RunAsync("evalf(sqrt(2), 50)");
        Assert.Equal(0, fiftyExit);
        Assert.Equal("1.41421356237309504880168872420969807856967187537694", Text(Structured(fiftyEnvelope)));

        // and the documented example that already worked is untouched by the fix
        var (readmeExit, readmeEnvelope) = await RunAsync("setprecision(20); evalf(sqrt(2), 20)");
        Assert.Equal(0, readmeExit);
        Assert.Equal("1.4142135623730950488", Text(Structured(readmeEnvelope)));
    }

    /// <summary>
    /// Cycle 5, item 1 (audit-2 B1-F5): the digit count that CANNOT be honoured. The audit found
    /// five different answers to one argument problem — <c>evalf(sin(1), 0)</c> and
    /// <c>evalf(cos(1), 0)</c> crossed as <c>InternalError</c>/<c>InternalInvariantFailure</c>
    /// ("Object reference not set to an instance of an object."), <c>evalf(exp(1), 0)</c> behaved
    /// like <c>digits = 1</c>, <c>evalf(1/3, 0)</c> answered 0 declared exact, and
    /// <c>evalf(sqrt(2), 0)</c> ignored the count. The protocol forbids an internal invariant
    /// failure as an answer to an argument problem, so the contract pinned here is the recoverable
    /// argument error the rest of the digit-count surface already produces (<c>pi(0)</c>/
    /// <c>e(0)</c>): a stable code and category, a message that names the builtin, the argument and
    /// the acceptable range, and no value at all.
    /// </summary>
    [Theory]
    [InlineData("evalf(sin(1), 0)")]      // used to raise NullReferenceException
    [InlineData("evalf(cos(1), 0)")]      // used to raise NullReferenceException
    [InlineData("evalf(exp(1), 0)")]      // used to behave like digits = 1
    [InlineData("evalf(1/3, 0)")]         // used to answer 0, declared exact: true
    [InlineData("evalf(sqrt(2), 0)")]     // used to ignore the count entirely
    [InlineData("evalf(pi, 0)")]          // the constant path, same request
    [InlineData("evalf(sin(1), -1)")]
    [InlineData("evalf(sin(1), -100)")]
    [InlineData("evalf(1/3, 2147483648)")]        // the Int32 cast wrapped to a negative count
    [InlineData("evalf(sin(1), 10^30)")]          // wider than Int64: a raw OverflowException
    public async Task Evalf_RefusesADigitCountItCannotHonour_AsARecoverableArgumentError(string script)
    {
        var (exit, envelope) = await RunAsync(script);

        Assert.Equal(1, exit);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' must not answer a value");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(), "the caller can fix the count and retry");
        Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
        Assert.NotEqual("InternalError", envelope["code"]!.GetValue<string>());

        string message = envelope["message"]!.GetValue<string>();
        Assert.StartsWith("evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got ", message);
        Assert.NotEqual("Object reference not set to an instance of an object.", message);
    }

    /// <summary>
    /// The boundary the refusal must not swallow: <c>digits = 1</c> is the smallest accepted
    /// count and must keep answering, so the guard cannot be widened into "reject small counts".
    /// </summary>
    [Fact]
    public async Task Evalf_DigitsOne_IsTheSmallestAcceptedCount()
    {
        var (exit, one) = await RunAsync("evalf(sin(1), 1)");
        Assert.Equal(0, exit);
        Assert.Equal("0.8", Text(Structured(one)));

        var (zeroExit, zero) = await RunAsync("evalf(sin(1), 0)");
        Assert.Equal(1, zeroExit);
        Assert.False(zero["ok"]!.GetValue<bool>());
    }

    // ------------------------------------------------------------------
    // 2. assumptions() is structured, and keeps the human rendering
    // ------------------------------------------------------------------

    /// <summary>
    /// An agent must not have to parse <c>"x &gt; 5"</c> to recover the set. The record carries the
    /// SAME atom projection <c>inspect(f).assumptions</c> uses — a relation stays a symbolic value
    /// and a domain restriction becomes a record carrying a Domain value — while <c>display</c>
    /// keeps the one-line rendering a human reads.
    /// </summary>
    [Fact]
    public async Task Assumptions_ReturnsStructuredAtoms_WithTheHumanRenderingInDisplay()
    {
        var (exit, envelope) = await RunAsync("x = symbol(\"x\"); assume(x > 5); assumptions()");
        Assert.Equal(0, exit);
        JsonNode structured = Structured(envelope);
        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("AssumptionSet", structured["type"]!.GetValue<string>());

        JsonObject fields = Fields(structured);
        Assert.Equal("x > 5", Text(fields["display"]!));
        JsonArray atoms = fields["assumptions"]!["elements"]!.AsArray();
        Assert.Single(atoms);
        Assert.Equal("Symbolic", atoms[0]!["kind"]!.GetValue<string>());
        Assert.Equal("(gt (sym x) (rat 5 1))", atoms[0]!["canonical"]!.GetValue<string>());

        // a domain restriction crosses as a Domain VALUE, not as the word "Integer"
        var (domainExit, domainEnvelope) = await RunAsync("x = symbol(\"x\"); assume_integer(x); assumptions()");
        Assert.Equal(0, domainExit);
        JsonNode atom = Fields(Structured(domainEnvelope))["assumptions"]!["elements"]!.AsArray()[0]!;
        Assert.Equal("DomainCondition", atom["type"]!.GetValue<string>());
        Assert.Equal("Domain", Fields(atom)["domain"]!["kind"]!.GetValue<string>());
        Assert.Equal("integer", Fields(atom)["domain"]!["domain"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 3. solve(f, x) publishes the same record solve_full does
    // ------------------------------------------------------------------

    /// <summary>
    /// Every shape the short form can produce — complete, parametric, empty, inconsistent,
    /// unevaluated, partial — is a SolveResult record. The prose sentences are gone, and the
    /// complete case is not a Vector either: the record is the ONE shape, exactly as
    /// <c>solve_system</c> already published the SystemSolveResult <c>solve_system_full</c> does.
    /// </summary>
    [Fact]
    public async Task Solve_ShortForm_PublishesTheSameSolveResultRecord_AsTheFullForm()
    {
        string[] shapes =
        {
            "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)",            // complete, finite
            "x = symbol(\"x\"); solve(sin(x) == 0, x)",             // parametric family
            "x = symbol(\"x\"); solve(x^2 + 1 == 0, x, real)",      // provably empty
            "x = symbol(\"x\"); solve(x - x + 1 == 0, x)",          // inconsistent
            "x = symbol(\"x\"); solve(exp(x) + x == 0, x)",         // unevaluated
            "x = symbol(\"x\"); solve(x^4 - x^2 - 1 == 0, x)",      // partial
        };
        foreach (string script in shapes)
        {
            var (exit, envelope) = await RunAsync(script);
            Assert.Equal(0, exit);
            JsonNode structured = Structured(envelope);
            Assert.Equal("Record", structured["kind"]!.GetValue<string>());
            Assert.Equal("SolveResult", structured["type"]!.GetValue<string>());
        }

        // the SAME record as the _full form, field for field
        var (shortExit, shortEnvelope) = await RunAsync("x = symbol(\"x\"); solve(x^2 - 4 == 0, x)");
        var (fullExit, fullEnvelope) = await RunAsync("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)");
        Assert.Equal(0, shortExit);
        Assert.Equal(0, fullExit);
        TestSupport.AssertSameJson(
            TestSupport.Normalise(Structured(fullEnvelope)),
            TestSupport.Normalise(Structured(shortEnvelope)),
            "solve() vs solve_full() for x^2 - 4 == 0");

        // the values the old vector carried are still there, in solutions[]
        JsonObject fields = Fields(Structured(shortEnvelope));
        Assert.Equal("Solved", fields["status"]!["value"]!.GetValue<string>());
        Assert.Equal(2, fields["solutions"]!["shape"]!.AsArray()[0]!.GetValue<int>());
        Assert.Equal(new[] { "(rat -2 1)", "(rat 2 1)" },
            fields["solutions"]!["elements"]!.AsArray()
                .Select(s => Fields(s!)["value"]!["canonical"]!.GetValue<string>()).ToArray());

        // a parametric family is structure too: template, parameter and its Domain
        var (familyExit, familyEnvelope) = await RunAsync("x = symbol(\"x\"); solve(sin(x) == 0, x)");
        Assert.Equal(0, familyExit);
        JsonNode family = Fields(Structured(familyEnvelope))["families"]!["elements"]!.AsArray()[0]!;
        Assert.Equal("SolutionFamily", family["type"]!.GetValue<string>());
        Assert.Equal("(mul (sym k) (pi))", Fields(family)["template"]!["canonical"]!.GetValue<string>());
        Assert.Equal("integer", Fields(family)["parameter_domain"]!["domain"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // 4. the capability statement names the operation its domain pair constrains
    // ------------------------------------------------------------------

    /// <summary>
    /// As <c>unsupported_domains</c> the pair read as a claim about the RUNTIME. Two live
    /// counterexamples made that reading false: <c>symbol("x", integer)</c> succeeds, and a solution
    /// family's parameter domain IS integer. The fields now name the solver's domain argument, and
    /// this test walks both directions of that scope — plus the surface the name no longer speaks
    /// for — through the published envelope.
    /// </summary>
    [Fact]
    public async Task Capabilities_DomainStatement_IsScopedToSolve_AndIntegerDomainsStillWork()
    {
        var (exit, envelope) = await RunAsync("capabilities()");
        Assert.Equal(0, exit);
        JsonObject fields = Fields(Structured(envelope));

        Assert.False(fields.ContainsKey("supported_domains"), "the runtime-wide domain claim must be gone");
        Assert.False(fields.ContainsKey("unsupported_domains"), "the runtime-wide domain claim must be gone");
        Assert.Equal(new[] { "real", "complex" }, Domains(fields["solve_domains_accepted"]!));
        Assert.Equal(new[] { "integer", "rational" }, Domains(fields["solve_domains_refused"]!));

        // every REFUSED domain really refuses, naming the domain it was handed
        foreach (string domain in new[] { "integer", "rational" })
        {
            var (refusedExit, refused) = await RunAsync($"x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})");
            Assert.Equal(1, refusedExit);
            Assert.False(refused["ok"]!.GetValue<bool>(), $"the statement refuses {domain}, so the live call must refuse it");
            Assert.Contains($"got {domain}", refused["message"]!.GetValue<string>());
        }

        // every ACCEPTED domain really answers, over the requested domain
        foreach (string domain in new[] { "real", "complex" })
        {
            var (acceptedExit, accepted) = await RunAsync($"x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})");
            Assert.Equal(0, acceptedExit);
            Assert.Equal(domain, Fields(Structured(accepted))["domain"]!["domain"]!.GetValue<string>());
        }

        // and the surface the renamed fields no longer speak for accepts every domain, including
        // the two the SOLVER refuses
        foreach (string domain in new[] { "real", "complex", "integer", "rational" })
        {
            var (symbolExit, symbolEnvelope) = await RunAsync($"symbol(\"s\", {domain})");
            Assert.Equal(0, symbolExit);
            Assert.True(symbolEnvelope["ok"]!.GetValue<bool>(),
                $"symbol(name, {domain}) is supported; the solver-scoped statement must not deny it");
        }
    }

    private static string[] Domains(JsonNode array) =>
        array["elements"]!.AsArray()
            .Select(d => d!["domain"]!.GetValue<string>())
            .ToArray();

    // ------------------------------------------------------------------
    // 5. the protocol document's examples are the binary's output
    // ------------------------------------------------------------------

    /// <summary>
    /// docs/symbolics/dsh-protocol.md is the machine contract, so its examples are checked against
    /// the binary rather than the other way round. Two of them were false: the canonical form of an
    /// integer atom (the binary writes rationals — the document even contradicted itself two
    /// sections later) and the error envelope, which lacked the <c>elapsedTime</c>/<c>timings</c>
    /// pair invariant 4 requires of EVERY envelope.
    /// </summary>
    [Fact]
    public async Task ProtocolDocumentExamples_MatchTheBinary()
    {
        string doc = File.ReadAllText(ProtocolDocumentPath());

        // the canonical atom of the Symbolic value form
        var (canonicalExit, canonicalEnvelope) = await RunAsync("x = symbol(\"x\"); x^2 + 1");
        Assert.Equal(0, canonicalExit);
        string liveCanonical = Structured(canonicalEnvelope)["canonical"]!.GetValue<string>();
        Assert.Contains($"\"canonical\": \"{liveCanonical}\"", JsonFence(doc, "## Value forms"));

        // the error envelope example carries the structural durations
        var (errorExit, errorEnvelope) = await RunAsync("x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)");
        Assert.Equal(1, errorExit);
        Assert.False(errorEnvelope["ok"]!.GetValue<bool>());
        JsonNode documented = JsonNode.Parse(JsonFence(doc, "## Error envelope"))!;
        foreach (string key in new[] { "elapsed", "elapsedTime", "timings" })
            Assert.True(documented[key] is not null,
                $"the documented error envelope must show {key} (invariant 4 is not scoped to success)");
        // ... and the live envelope really has them
        Assert.True(errorEnvelope["elapsedTime"] is not null, "the live error envelope carries elapsedTime");
        Assert.True(errorEnvelope["timings"] is not null, "the live error envelope carries timings");
    }

    /// <summary>The first fenced <c>json</c> block after a heading in the protocol document.</summary>
    private static string JsonFence(string doc, string heading)
    {
        int at = doc.IndexOf(heading, StringComparison.Ordinal);
        Assert.True(at >= 0, $"'{heading}' is not in the protocol document any more");
        string tail = doc[at..];
        const string open = "\u0060\u0060\u0060json";
        int start = tail.IndexOf(open, StringComparison.Ordinal);
        Assert.True(start >= 0, $"'{heading}' has no json example");
        start += open.Length;
        int end = tail.IndexOf("\u0060\u0060\u0060", start, StringComparison.Ordinal);
        Assert.True(end > start, $"'{heading}' has an unterminated json example");
        return tail[start..end];
    }

    /// <summary>The protocol document in THIS checkout: found by walking up from the test binary,
    /// so the same test works in a worktree and in the main tree.</summary>
    private static string ProtocolDocumentPath()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            string candidate = Path.Combine(dir.FullName, "docs", "symbolics", "dsh-protocol.md");
            if (File.Exists(candidate))
                return candidate;
            dir = dir.Parent;
        }
        throw new Xunit.Sdk.XunitException(
            $"docs/symbolics/dsh-protocol.md was not found above {AppContext.BaseDirectory}");
    }
}
