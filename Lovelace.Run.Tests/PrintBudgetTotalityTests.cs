using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// <c>--print-budget &lt;nodes&gt;</c> is a NODE budget: whatever the size of the value, once its
/// node count is above the budget the renderings cross abbreviated and the value says so —
/// <c>truncated: true</c>, a <c>truncationReason</c> and the <c>budget</c> that stopped it
/// (dsh-protocol.md:17-20).
///
/// <para>The comparison used to be partial: the kernel printer abbreviates only when the rendering
/// is longer than the character allowance the budget implies (<c>max(48, 6 × budget)</c>), so a
/// 3-node value at budget 1 and a 5-node value at every budget up to 8 came back WHOLE with no
/// truncation fields, while a 13-node value at budget 8 did truncate — the same flag, two
/// behaviours, decided by text length (A2-F19 / A4-F1). These tests state the property over
/// GENERATED node counts and budgets rather than a list of strings: for every budget below the
/// value's node count the answer is abbreviated, for every budget at or above it the answer is the
/// full rendering, and an abbreviation is always a prefix of the real rendering terminated by
/// " …" — never a re-ordered one, never the whole thing with a flag on top.</para>
/// </summary>
public class PrintBudgetTotalityTests
{
    /// <summary>The node counts the property is proved over. The script builder below emits
    /// exactly <c>nodes</c> nodes — the sum contributes one, each of its <c>nodes − 1</c> DISTINCT
    /// symbols one — so the test proves the count it assumes rather than trusting it.</summary>
    public static TheoryData<int> NodeCounts => new() { 3, 5, 7, 9, 11, 13 };

    /// <summary>
    /// An n-ary sum of <c>nodes − 1</c> distinct symbols: exactly <c>nodes</c> nodes.
    /// <para>
    /// The FIRST TWO rows are the ones the finding is about: their renderings are short enough
    /// that the kernel printer's character allowance (<c>max(48, 6 × budget)</c>) never bites, so
    /// before this round a budget of 1 returned them whole. The larger rows are the control shape
    /// that already truncated, kept so the property is stated over both.</para>
    /// </summary>
    private static string SumScript(int nodes)
    {
        int terms = nodes - 1;
        string declarations = string.Concat(
            Enumerable.Range(0, terms).Select(i => $"s{i} = symbol(\"s{i}\"); "));
        return declarations + string.Join(" + ", Enumerable.Range(0, terms).Select(i => "s" + i));
    }

    private static async Task<JsonNode> StructuredAsync(string script, params string[] extra)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(script, extra);
        Assert.True(exitCode == 0, $"script '{script}' {(extra.Length == 0 ? "" : string.Join(" ", extra) + " ")}exited {exitCode}. stderr: {stderr.Trim()}");
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, $"stdout of '{script}'");
        return envelope["result"]!["structured"]!;
    }

    [Theory]
    [MemberData(nameof(NodeCounts))]
    public async Task EveryNodeCountOverBudget_IsAbbreviatedAndReportsIt(int nodes)
    {
        // The real rendering: the prefix every abbreviation must come from.
        JsonNode full = await StructuredAsync(SumScript(nodes));
        Assert.Equal(nodes, full["nodeCount"]!.GetValue<int>());
        string fullPretty = full["pretty"]!.GetValue<string>();
        string fullCanonical = full["canonical"]!.GetValue<string>();

        // The finding: budgets BELOW the node count are over budget NO MATTER how short the
        // rendering is — this is where the 3-node and 5-node values used to come back whole.
        for (int budget = 1; budget < nodes; budget++)
        {
            JsonNode structured = await StructuredAsync(SumScript(nodes), "--print-budget", budget.ToString());

            Assert.True(structured["truncated"]?.GetValue<bool>() == true,
                $"nodeCount {nodes} at budget {budget}: truncated is {structured["truncated"]?.ToJsonString() ?? "absent"}, " +
                $"so an over-budget value crossed as if it were the whole rendering");
            Assert.Equal("node-budget", structured["truncationReason"]!.GetValue<string>());
            Assert.Equal(budget, structured["budget"]!.GetValue<int>());

            AssertAbbreviationOf(fullPretty, structured["pretty"]!.GetValue<string>(), nodes, budget, "pretty");
            AssertAbbreviationOf(fullCanonical, structured["canonical"]!.GetValue<string>(), nodes, budget, "canonical");
        }

        // At and above the node count the value is NOT over budget: the full rendering, and no
        // truncation fields at all (never a flag that says "truncated" about the whole value).
        foreach (int budget in new[] { nodes, nodes + 2 })
        {
            JsonNode structured = await StructuredAsync(SumScript(nodes), "--print-budget", budget.ToString());
            Assert.Null(structured["truncated"]);
            Assert.Null(structured["truncationReason"]);
            Assert.Null(structured["budget"]);
            Assert.Equal(fullPretty, structured["pretty"]!.GetValue<string>());
            Assert.Equal(fullCanonical, structured["canonical"]!.GetValue<string>());
        }
    }

    /// <summary>An abbreviation must be a STRICT prefix of the real rendering, terminated by
    /// " …", and shorter than it — a prefix is never passed off as the whole expression, and the
    /// order of the rendering never changes.</summary>
    private static void AssertAbbreviationOf(string full, string abridged, int nodes, int budget, string form)
    {
        Assert.EndsWith(" …", abridged);
        string prefix = abridged[..^2];
        Assert.True(full.StartsWith(prefix, StringComparison.Ordinal),
            $"nodeCount {nodes} at budget {budget}: {form} '{abridged}' is not a prefix of '{full}'");
        Assert.True(prefix.Length < full.Length,
            $"nodeCount {nodes} at budget {budget}: {form} '{abridged}' abbreviates nothing ('{full}' is " +
            $"{full.Length} chars, the prefix is {prefix.Length})");
    }

    /// <summary>The smallest reported case, stated directly: a 3-node value at budget 1. The
    /// canonical form is a prefix of <c>(pow (sym x) (rat 2 1))</c> and the value says why.</summary>
    [Fact]
    public async Task SmallValueOverASmallBudget_IsAbbreviated()
    {
        JsonNode structured = await StructuredAsync("x = symbol(\"x\"); x^2", "--print-budget", "1");

        Assert.Equal(3, structured["nodeCount"]!.GetValue<int>());
        Assert.True(structured["truncated"]?.GetValue<bool>() == true,
            "x^2 (3 nodes) at budget 1 crossed with no truncation flag");
        Assert.Equal("node-budget", structured["truncationReason"]!.GetValue<string>());
        Assert.Equal(1, structured["budget"]!.GetValue<int>());
        Assert.StartsWith("(pow ", structured["canonical"]!.GetValue<string>(), StringComparison.Ordinal);
        Assert.EndsWith(" …", structured["canonical"]!.GetValue<string>());
    }

    /// <summary>The budget reaches EVERY structured rendering in the envelope, not just the
    /// result: a variable over the budget is abbreviated too, by the same policy.</summary>
    [Fact]
    public async Task TheBudgetAlsoBoundsTheStructuredVariables()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync(
            "x = symbol(\"x\"); x^2", "--omit-functions", "--print-budget", "1");
        Assert.Equal(0, exitCode);

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "envelope");
        JsonNode result = envelope["result"]!["structured"]!;
        JsonObject variables = VariableIndex(envelope);

        JsonNode xSquared = variables["_"]!["structured"]!;
        Assert.True(xSquared["truncated"]?.GetValue<bool>() == true,
            "the result was abbreviated but the variable holding it was not — two policies, one flag");
        Assert.Equal(result["canonical"]!.GetValue<string>(), xSquared["canonical"]!.GetValue<string>());

        // a variable WITHIN the budget stays whole
        JsonNode x = variables["x"]!["structured"]!;
        Assert.Null(x["truncated"]);
        Assert.Equal("(sym x)", x["canonical"]!.GetValue<string>());
    }

    private static JsonObject VariableIndex(JsonNode envelope)
    {
        var index = new JsonObject();
        foreach (JsonNode? variable in envelope["variables"]!.AsArray())
            index[variable!["name"]!.GetValue<string>()] = variable!.DeepClone();
        return index;
    }
}
