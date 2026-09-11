using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// Every <c>variables[]</c> entry carries the SAME structured projection the result does, so a
/// variable's mathematical meaning is recoverable from structure instead of from a human display
/// string: an agent reads <c>variables[0].structured.canonical</c>, never
/// <c>variables[0].display</c> (A2-F24).
///
/// <para>The entries used to be <c>{name, kind, display}</c> only, so <c>x = symbol("x"); [x, x]</c>
/// published <c>display: "[-2, 2]"</c> for a value that is fully structural in
/// <c>result.structured</c> — the same value, machine-readable in one place and prose in the other.
/// The projection is total over the value kinds: these tests leave one variable of every kind a
/// script can put in scope — Record, Array, Vector, Complex, Real, Symbolic, Natural, Integer,
/// Boolean, Text, Domain and Enum — and require a structured form from each.</para>
/// </summary>
public class VariableStructuredProjectionTests
{
    /// <summary>One variable per kind the language can leave in scope (Function and Void are not
    /// storable: a bare <c>sin</c> is "Undefined variable 'sin'" and a Void statement defines
    /// nothing), plus the projected result itself.</summary>
    private const string EveryKind =
        "x = symbol(\"x\"); " +
        "r = 1/3; " +
        "q = sqrt(2); " +
        "z = dft([1, 2, 3])[1]; " +
        "b = is_even(4); " +
        "t = \"hi\"; " +
        "v = [1, 2]; " +
        "m = [[1, 2], [3, 4]]; " +
        "d = real; " +
        "n = 42; " +
        "k = -7; " +
        "g = solve_full(x^2 - 4 == 0, x); " +
        "s = g.status; " +
        "i = inspect(x)";

    private static async Task<(JsonNode Envelope, JsonObject Variables)> RunAsync(params string[] extra)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(EveryKind, extra);
        Assert.True(exitCode == 0, $"the every-kind script exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "every-kind envelope");
        var index = new JsonObject();
        foreach (JsonNode? variable in envelope["variables"]!.AsArray())
            index[variable!["name"]!.GetValue<string>()] = variable!.DeepClone();
        return (envelope, index);
    }

    /// <summary>The totality property: EVERY entry has a structured form whose kind names a real
    /// kind (never a missing key and never the Null placeholder for a real value).</summary>
    [Fact]
    public async Task EveryValueKindInScope_CrossesWithItsStructuredForm()
    {
        (JsonNode envelope, JsonObject variables) = await RunAsync("--omit-functions");

        string[] documented = { "Natural", "Integer", "Real", "Complex", "Boolean", "Text", "Array",
            "Symbolic", "Record", "Domain", "Enum", "Function", "Null" };
        foreach ((string name, JsonNode? variable) in variables)
        {
            JsonNode? structured = variable!["structured"];
            Assert.True(structured is not null,
                $"variable '{name}' ({variable["kind"]}) crossed without a structured form: {variable.ToJsonString()}");
            string kind = structured!["kind"]!.GetValue<string>();
            Assert.Contains(kind, documented);
            Assert.True(kind != "Null", $"variable '{name}' is a real value but projected as Null");
            Assert.NotEmpty(variable["display"]!.GetValue<string>());
        }

        // the corpus really exercised the kinds this finding names
        Assert.Equal("Record", variables["g"]!["structured"]!["kind"]!.GetValue<string>());
        Assert.Equal("Array", variables["m"]!["structured"]!["kind"]!.GetValue<string>());
        Assert.Equal("Complex", variables["z"]!["structured"]!["kind"]!.GetValue<string>());
        Assert.Equal("Real", variables["r"]!["structured"]!["kind"]!.GetValue<string>());
    }

    /// <summary>What the structured form is FOR: the meaning an agent needs, per kind.</summary>
    [Fact]
    public async Task StructuredVariables_CarryTheMachineryReadableMeaning()
    {
        (_, JsonObject variables) = await RunAsync("--omit-functions");

        // Symbolic: both renderings, the domain, exactness, node count and free symbols
        JsonNode x = variables["x"]!["structured"]!;
        Assert.Equal("Symbolic", x["kind"]!.GetValue<string>());
        Assert.Equal("(sym x)", x["canonical"]!.GetValue<string>());
        Assert.Equal("x", x["pretty"]!.GetValue<string>());
        Assert.Equal("complex", x["domain"]!.GetValue<string>());
        Assert.True(x["exact"]!.GetValue<bool>());
        Assert.Equal(1, x["nodeCount"]!.GetValue<int>());
        Assert.Equal(new[] { "x" }, x["freeSymbols"]!.AsArray().Select(n => n!.GetValue<string>()).ToArray());

        // Real: exactness is a BOOLEAN (never a display guess) and an exact Real is a fraction
        JsonNode third = variables["r"]!["structured"]!;
        Assert.Equal("Real", third["kind"]!.GetValue<string>());
        Assert.True(third["exact"]!.GetValue<bool>());
        Assert.Equal("1", third["numerator"]!.GetValue<string>());
        Assert.Equal("3", third["denominator"]!.GetValue<string>());
        // ... and a truncated irrational says so instead of publishing a fraction
        JsonNode sqrtTwo = variables["q"]!["structured"]!;
        Assert.False(sqrtTwo["exact"]!.GetValue<bool>());
        Assert.Null(sqrtTwo["numerator"]);

        // Complex: both parts, with the exactness flag as a boolean
        JsonNode complex = variables["z"]!["structured"]!;
        Assert.Equal("Complex", complex["kind"]!.GetValue<string>());
        Assert.NotNull(complex["re"]!.GetValue<string>());
        Assert.NotNull(complex["im"]!.GetValue<string>());
        Assert.False(complex["exact"]!.GetValue<bool>());

        // Array: shape and elements, with the array type that distinguishes a Vector from an Array
        JsonNode matrix = variables["m"]!["structured"]!;
        Assert.Equal("Array", matrix["type"]!.GetValue<string>());
        Assert.Equal(new long[] { 2, 2 }, matrix["shape"]!.AsArray().Select(n => n!.GetValue<long>()).ToArray());
        Assert.Equal(4, matrix["elements"]!.AsArray().Count);
        JsonNode vector = variables["v"]!["structured"]!;
        Assert.Equal("Vector", vector["type"]!.GetValue<string>());

        // scalars
        Assert.Equal("42", variables["n"]!["structured"]!["value"]!.GetValue<string>());
        Assert.Equal("-7", variables["k"]!["structured"]!["value"]!.GetValue<string>());
        Assert.Equal("true", variables["b"]!["structured"]!["value"]!.GetValue<string>());
        Assert.Equal("hi", variables["t"]!["structured"]!["value"]!.GetValue<string>());
        Assert.Equal("real", variables["d"]!["structured"]!["domain"]!.GetValue<string>());

        // Enum: the DECLARED enum type travels with the member name
        JsonNode status = variables["s"]!["structured"]!;
        Assert.Equal("Enum", status["kind"]!.GetValue<string>());
        Assert.Equal("SolveStatus", status["type"]!.GetValue<string>());
        Assert.Equal("Solved", status["value"]!.GetValue<string>());

        // Record: the type name and the ordered fields, not a "SolveResult(...)" string
        JsonNode record = variables["g"]!["structured"]!;
        Assert.Equal("Record", record["kind"]!.GetValue<string>());
        Assert.Equal("SolveResult", record["type"]!.GetValue<string>());
        Assert.Equal("Solved", TestSupport.FieldsByName(record)["status"]!["value"]!.GetValue<string>());
    }

    /// <summary>The projection is SHARED: the variable that holds the result carries the very same
    /// structure as <c>result.structured</c> — one implementation, so the two cannot drift.</summary>
    [Fact]
    public async Task TheResultVariable_CarriesTheSameStructureAsTheResult()
    {
        (JsonNode envelope, JsonObject variables) = await RunAsync("--omit-functions");

        JsonNode result = envelope["result"]!["structured"]!;
        Assert.Equal("Inspection", result["type"]!.GetValue<string>());
        TestSupport.AssertSameJson(TestSupport.Normalise(result),
            TestSupport.Normalise(variables["_"]!["structured"]), "variables[_].structured vs result.structured");
    }

    /// <summary>A variable has no structured form to omit when the caller asked for no variables
    /// at all: <c>--omit-variables</c> stays the payload knob it was.</summary>
    [Fact]
    public async Task OmitVariables_StillEmitsNoVariableEntries()
    {
        (JsonNode envelope, _) = await RunAsync("--omit-functions", "--omit-variables");
        Assert.Empty(envelope["variables"]!.AsArray());
    }
}
