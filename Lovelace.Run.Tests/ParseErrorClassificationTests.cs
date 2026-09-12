using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// docs/symbolics/dsh-protocol.md:207-209 is the error table: "Categories are spelled from the one
/// taxonomy, ErrorCategory: ParseError, DomainError, UnsupportedOperation, BudgetExceeded,
/// NoSolution, TypeMismatch, InternalInvariantFailure" — and :203 names "a parse error" as an error
/// "raised before any engine exists". A source the tokenizer or the parser refuses is therefore a
/// ParseError; a script file that could not be read is not a parse error at all.
///
/// Cycle-6 audit E filed the violation as F6 (P1), "ParseError is attached to the wrong layer in
/// both directions": <c>1 +</c> and <c>1+1;⏎2+2;⏎)</c> answered InvalidOperation/DomainError —
/// the tokenizer and the parser refuse with the same InvalidOperationException a domain failure
/// uses (Lovelace.Suite/Tokenizer.cs:123, Parser.cs:64-71), so the runner's type switch could not
/// tell the phases apart — while a missing script file answered FileReadError/ParseError although
/// nothing was parsed. The recorded-but-never-closed cycle-5 item is B3-surface F3.
///
/// The categories are asserted as WIRE STRINGS, exactly as the protocol document spells them.
/// </summary>
public class ParseErrorClassificationTests
{
    /// <summary>The taxonomy member a lexer/parser refusal belongs to (dsh-protocol.md:207-209).</summary>
    private const string ParseErrorCategory = "ParseError";

    private static async Task<(int ExitCode, string Raw, JsonNode Envelope)> RunArgumentsAsync(
        string[] arguments)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments, stdout, stderr);
        string raw = stdout.ToString();
        return (exitCode, raw, TestSupport.ParseExactlyOneJsonDocument(raw, "runner stdout"));
    }

    private static Task<(int ExitCode, string Raw, JsonNode Envelope)> RunScriptAsync(string script) =>
        RunArgumentsAsync(new[] { "--eval", script, "--omit-functions", "--omit-variables" });

    /// <summary>The parsed envelope's category, as a string.</summary>
    private static string Category(JsonNode envelope) => envelope["category"]!.GetValue<string>();

    /// <summary>The parsed envelope's code, as a string.</summary>
    private static string Code(JsonNode envelope) => envelope["code"]!.GetValue<string>();

    // ------------------------------------------------------------------
    // The finding: a parse failure is a parse failure
    // ------------------------------------------------------------------

    /// <summary>
    /// The audit's multi-line shape: the third line is the offending <c>)</c>. Pre-fix this answers
    /// InvalidOperation/DomainError, i.e. the agent is told to fix a domain problem when it has a
    /// syntax error.
    /// </summary>
    [Fact]
    public async Task AParseFailureInAMultiLineScript_CrossesAsParseError()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("1+1;\n2+2;\n)");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Code(envelope));
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a caller can fix the syntax and retry, so the refusal is recoverable");
        // the message still names what the parser refused
        Assert.Contains("Unexpected token ')'", envelope["message"]!.GetValue<string>());
        // nothing executed: the parse ends the run before the first statement
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>The audit's single-line shape: the refusal is the same layer and must say so.</summary>
    [Fact]
    public async Task AParseFailureInASingleLineScript_CrossesAsParseError()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("1 +");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Code(envelope));
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Contains("expected a number", envelope["message"]!.GetValue<string>());
        Assert.Empty(envelope["timings"]!.AsArray());
    }

    /// <summary>
    /// The other limb of the same layer: the tokenizer's own string refusal already carried the
    /// ParseError CATEGORY (its code is the tokenizer's own InvalidInput, which this round does not
    /// move). Pinned so a change to the parser's classification cannot silently move this one.
    /// </summary>
    [Fact]
    public async Task TheTokenizerStringRefusal_CarriesTheSameParseErrorCategory()
    {
        var (exitCode, raw, envelope) = await RunScriptAsync("\"abc");

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal(ParseErrorCategory, Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
        Assert.Contains("Unterminated string literal", envelope["message"]!.GetValue<string>());
    }

    /// <summary>
    /// The other direction of the finding: NOTHING was parsed for a file the process could not
    /// read, so ParseError is the wrong category — it sends an agent to fix syntax that does not
    /// exist. The code stays FileReadError; the category follows the caller-argument precedent the
    /// sibling plot paths already use (PlotDirectoryError/TypeMismatch, PlotFileError/TypeMismatch).
    /// </summary>
    [Fact]
    public async Task AMissingScriptFile_IsNotAParseError()
    {
        string missing = Path.Combine(Path.GetTempPath(), "c6-pd-missing-" + Guid.NewGuid().ToString("N") + ".ls");
        Assert.False(File.Exists(missing), $"the probe path must not exist: {missing}");

        var (exitCode, raw, envelope) = await RunArgumentsAsync(
            new[] { "--file", missing, "--json", "--omit-functions", "--omit-variables" });

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal("FileReadError", Code(envelope));
        Assert.NotEqual(ParseErrorCategory, Category(envelope));
        Assert.Equal("TypeMismatch", Category(envelope));
        Assert.True(envelope["recoverable"]!.GetValue<bool>(), "a caller can name a readable path instead");
        // no engine ran: no diagnostics and no statement timing, but the envelope is still complete
        Assert.Empty(envelope["diagnostics"]!.AsArray());
        Assert.Empty(envelope["timings"]!.AsArray());
        Assert.Empty(envelope["output"]!.AsArray());
    }

    // ------------------------------------------------------------------
    // Controls: the failures that are NOT parse refusals keep their classification
    // ------------------------------------------------------------------

    /// <summary>
    /// A domain failure on a script that parses perfectly must not be relabelled: this is the
    /// control that stops the fix from turning every failure into a ParseError.
    /// </summary>
    [Fact]
    public async Task ADomainFailure_OnAParsingScript_KeepsItsDomainCategory()
    {
        var (unknownExit, unknownRaw, unknown) = await RunScriptAsync("nosuchfn(1)");
        Assert.Equal(1, unknownExit);
        Assert.Equal("InvalidOperation", Code(unknown));
        Assert.Equal("DomainError", Category(unknown));

        var (shapeExit, shapeRaw, shape) = await RunScriptAsync("det(1)");
        Assert.Equal(1, shapeExit);
        Assert.Equal("InvalidArgument", Code(shape));
        Assert.Equal("TypeMismatch", Category(shape));
    }

    /// <summary>
    /// A depth refusal happens DURING parsing but is not a syntax refusal: it is a budget stop and
    /// the depth guard's own taxonomy (DepthExceeded/BudgetExceeded) must survive.
    /// </summary>
    [Fact]
    public async Task AParsePhaseDepthRefusal_IsNotReclassifiedAsAParseError()
    {
        string deep = new string('(', 3000) + "1" + new string(')', 3000) + ";";
        var (exitCode, raw, envelope) = await RunScriptAsync(deep);

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), $"ok must be false. stdout: {raw}");
        Assert.Equal("DepthExceeded", Code(envelope));
        Assert.Equal("BudgetExceeded", Category(envelope));
        Assert.Contains("nesting depth", envelope["message"]!.GetValue<string>());
    }
}
