using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// Round-20 audit I, I-3, on the wire: <c>--print-budget 1</c> with a 54-character symbol name
/// crossed <c>pretty</c> as 48 identifier characters + " …" — an identifier that does not exist in
/// the value — while the three truncation fields were honest (<c>truncated: true</c>,
/// <c>node-budget</c>, <c>budget: 1</c>). The printer's own promise
/// (<c>Lovelace.Symbolics/Printing.cs:340-345</c>, <c>:402-409</c>) is that an abbreviation never
/// cuts inside a token, and the projection that enforces the node budget on the way to the wire
/// (<c>Lovelace.Suite/StructuredProjection.cs:158-166</c>) makes the same promise.
///
/// <para>Both seams are pinned here, because the finding's shape reaches the first and a value whose
/// first token is longer than the PROJECTION's proportional allowance reaches the second. The
/// retained body must always end at a token boundary: the character the cut drops is never a token
/// character. The human <c>display</c> is not a rendering under a budget and is unchanged.</para>
/// </summary>
public class PrintBudgetTokenBoundaryTests
{
    /// <summary>The finding's own name. 54 characters is longer than the 48-character floor of the
    /// kernel allowance, so <c>Math.Max(48, 6 × 1)</c> falls inside the identifier.</summary>
    private const string LongName = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    private static bool IsTokenChar(char c) => char.IsLetterOrDigit(c) || c == '.';

    private static async Task<JsonNode> StructuredAsync(string script, params string[] extra)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(script, extra);
        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}. stderr: {stderr.Trim()}");
        return TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'")["result"]!["structured"]!;
    }

    /// <summary>The promise, as the property it is: the body is a strict prefix of the real
    /// rendering and the first character it drops is not a token character.</summary>
    private static void AssertEndsAtATokenBoundary(string full, string abbreviated, string what)
    {
        Assert.EndsWith(" …", abbreviated);
        string body = abbreviated[..^2];
        Assert.True(body.Length < full.Length, $"{what}: '{abbreviated}' abbreviates nothing");
        Assert.StartsWith(body, full, StringComparison.Ordinal);
        // A cut is INSIDE a token exactly when BOTH sides of it are token characters: the retained
        // text ends with one AND the dropped text begins with one. A body that ends with a COMPLETE
        // token is not a split just because its last character is a letter or a digit.
        bool retainsTokenChar = body.Length > 0 && IsTokenChar(body[^1]);
        bool dropsTokenChar = IsTokenChar(full[body.Length]);
        Assert.False(retainsTokenChar && dropsTokenChar,
            $"{what}: the cut falls between '{body[^1]}' and '{full[body.Length]}', both token " +
            $"characters, so '{abbreviated}' ends inside the token " +
            $"…'{full[Math.Max(0, body.Length - 8)..Math.Min(full.Length, body.Length + 8)]}'");
    }

    /// <summary>The finding's exact repro: the pretty body must be the whole identifier (or a
    /// boundary before it), never 48 of its 54 characters.</summary>
    [Fact]
    public async Task AnIdentifierLongerThanTheKernelAllowance_IsNeverSplit()
    {
        string script = $"{LongName} = symbol(\"{LongName}\"); {LongName} + 1";
        string[] flags = { "--omit-functions", "--omit-variables", "--print-budget", "1" };

        JsonNode structured = await StructuredAsync(script, flags);
        string pretty = structured["pretty"]!.GetValue<string>();

        // the whole identifier is what has to be kept: an abbreviation may return MORE than the
        // character allowance by one token, never less than a token
        string body = pretty.EndsWith(" …") ? pretty[..^2] : pretty;
        Assert.True(body == LongName,
            $"the pretty body is {body.Length} characters of a {LongName.Length}-character identifier: '{body}'");
        Assert.EndsWith(" …", pretty);
        Assert.True(structured["truncated"]?.GetValue<bool>() == true, "the abbreviation did not say so");
        Assert.Equal("node-budget", structured["truncationReason"]!.GetValue<string>());
        Assert.Equal(1, structured["budget"]!.GetValue<int>());

        // the same value without a budget: the body above is a boundary prefix of THIS rendering
        JsonNode full = await StructuredAsync(script, "--omit-functions", "--omit-variables");
        AssertEndsAtATokenBoundary(full["pretty"]!.GetValue<string>(), pretty, "pretty");
        AssertEndsAtATokenBoundary(full["canonical"]!.GetValue<string>(),
            structured["canonical"]!.GetValue<string>(), "canonical");
    }

    /// <summary>The projection's own cut (the kernel printer leaves a short rendering whole, so the
    /// node budget is completed here): a 40-character identifier with a proportional allowance of
    /// 14 characters came back as 14 of them + " …".</summary>
    [Fact]
    public async Task AnIdentifierLongerThanTheProjectionsAllowance_IsNeverSplit()
    {
        string name = new string('b', 40);
        // x first, so the sum is the last statement and therefore the result
        string script = $"x = symbol(\"x\"); {name} = symbol(\"{name}\"); {name} + x";

        JsonNode structured = await StructuredAsync(script, "--omit-functions", "--omit-variables", "--print-budget", "1");
        JsonNode full = await StructuredAsync(script, "--omit-functions", "--omit-variables");

        Assert.True(structured["truncated"]?.GetValue<bool>() == true, "the abbreviation did not say so");
        Assert.Equal("node-budget", structured["truncationReason"]!.GetValue<string>());
        AssertEndsAtATokenBoundary(full["pretty"]!.GetValue<string>(), structured["pretty"]!.GetValue<string>(), "pretty");
        AssertEndsAtATokenBoundary(full["canonical"]!.GetValue<string>(), structured["canonical"]!.GetValue<string>(), "canonical");
    }

    /// <summary>The budget is a bound on the RENDERING, not on the value: the human display of the
    /// same envelope still carries the real name, and no budget means the full rendering.</summary>
    [Fact]
    public async Task TheUnbudgetedRenderings_AreUnchanged()
    {
        string script = $"{LongName} = symbol(\"{LongName}\"); {LongName} + 1";
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            script, "--omit-functions", "--omit-variables", "--print-budget", "1");
        Assert.Equal(0, exitCode);
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "envelope");

        Assert.Equal(LongName + " + 1", envelope["result"]!["display"]!.GetValue<string>());
        // typed is the display plus the value kind (dsh-protocol.md), so the real name is in it
        Assert.StartsWith(LongName + " + 1 ", envelope["result"]!["typed"]!.GetValue<string>(), StringComparison.Ordinal);

        JsonNode unbudgeted = await StructuredAsync(script, "--omit-functions", "--omit-variables");
        Assert.Null(unbudgeted["truncated"]);
        Assert.Equal(LongName + " + 1", unbudgeted["pretty"]!.GetValue<string>());
    }
}
