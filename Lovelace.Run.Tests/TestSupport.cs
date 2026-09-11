using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using Xunit.Sdk;

namespace Lovelace.Run.Tests;

/// <summary>
/// Shared plumbing for the envelope contract tests: fixture paths, an in-process invocation of
/// the runner that captures both writers, a strict "exactly one JSON document" parser, the
/// volatile-key normaliser the wire contract is defined modulo, and an order-insensitive
/// structural JSON comparison.
/// </summary>
internal static class TestSupport
{
    /// <summary>
    /// The only values that legitimately differ between two runs of the same script. Everything
    /// else in the envelope is the wire contract: if it changes, the golden comparison fails.
    /// </summary>
    internal static readonly string[] VolatileKeys = { "revision", "elapsed", "elapsedTime", "timings" };

    internal static string FixturesDirectory => Path.Combine(AppContext.BaseDirectory, "fixtures");

    internal static string ScriptPath(string name) => Path.Combine(FixturesDirectory, name + ".ls");

    internal static string GoldenPath(string name) => Path.Combine(FixturesDirectory, name + ".json");

    /// <summary>
    /// Runs this repository's runner in-process over the named fixture script, capturing the
    /// envelope from the writers instead of the console: --omit-functions, exactly as the
    /// command-line contract documents for agent loops.
    /// </summary>
    internal static async Task<(int ExitCode, string Stdout, string Stderr)> RunFixtureAsync(
        string name, params string[] extraArguments)
    {
        string scriptPath = ScriptPath(name);
        Assert.True(File.Exists(scriptPath), $"missing fixture script: {scriptPath}");

        var arguments = new List<string> { "--file", scriptPath, "--omit-functions" };
        arguments.AddRange(extraArguments);

        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Runner.RunAsync(arguments.ToArray(), stdout, stderr);
        return (exitCode, stdout.ToString(), stderr.ToString());
    }

    /// <summary>
    /// Parses <paramref name="text"/> as EXACTLY one JSON document: no leading noise, no second
    /// document, and nothing but trailing whitespace. The raw text is part of every failure
    /// message so a regression that pollutes stdout is diagnosable from the test output alone.
    /// </summary>
    internal static JsonNode ParseExactlyOneJsonDocument(string text, string what)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(text);
        var reader = new Utf8JsonReader(bytes, isFinalBlock: true, state: default);
        if (!reader.Read())
            throw new XunitException($"{what} contained no JSON document at all. Raw text: {Describe(text)}");

        using JsonDocument document = ParseValue(ref reader, what, text);
        if (reader.Read())
            throw new XunitException($"{what} contained more than one JSON document (or trailing content). Raw text: {Describe(text)}");

        return JsonNode.Parse(document.RootElement.GetRawText())!;
    }

    private static JsonDocument ParseValue(ref Utf8JsonReader reader, string what, string text)
    {
        try
        {
            return JsonDocument.ParseValue(ref reader);
        }
        catch (JsonException ex)
        {
            throw new XunitException($"{what} is not valid JSON ({ex.Message}). Raw text: {Describe(text)}");
        }
    }

    /// <summary>
    /// Replaces the value of every volatile key at any depth with the literal string
    /// "&lt;volatile&gt;", so two runs of the same script become comparable.
    /// </summary>
    internal static JsonNode? Normalise(JsonNode? node)
    {
        switch (node)
        {
            case JsonObject source:
            {
                var copy = new JsonObject();
                foreach (var property in source)
                {
                    copy[property.Key] = VolatileKeys.Contains(property.Key)
                        ? JsonValue.Create("<volatile>")
                        : Normalise(property.Value);
                }
                return copy;
            }
            case JsonArray source:
            {
                var copy = new JsonArray();
                foreach (var element in source)
                    copy.Add(Normalise(element));
                return copy;
            }
            default:
                return node?.DeepClone();
        }
    }

    /// <summary>
    /// Asserts two normalised JSON trees are structurally identical. Object key ORDER is
    /// irrelevant; an added, removed or changed value (or a changed array length) is not.
    /// </summary>
    internal static void AssertSameJson(JsonNode? expected, JsonNode? actual, string what)
    {
        var differences = new List<string>();
        CollectDifferences(expected, actual, "$", differences);
        if (differences.Count == 0)
            return;

        const int shown = 25;
        string detail = string.Join(Environment.NewLine, differences.Take(shown));
        if (differences.Count > shown)
            detail += $"{Environment.NewLine}... and {differences.Count - shown} more difference(s)";
        throw new XunitException($"{what}: envelope does not match the golden fixture:{Environment.NewLine}{detail}");
    }

    private static void CollectDifferences(JsonNode? expected, JsonNode? actual, string path, List<string> differences)
    {
        if (expected is null || actual is null)
        {
            if (!ReferenceEquals(expected, actual))
                differences.Add($"{path}: expected {DescribeNode(expected)}, got {DescribeNode(actual)}");
            return;
        }

        if (expected is JsonObject expectedObject && actual is JsonObject actualObject)
        {
            string[] expectedKeys = expectedObject.Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray();
            string[] actualKeys = actualObject.Select(p => p.Key).OrderBy(k => k, StringComparer.Ordinal).ToArray();
            if (!expectedKeys.SequenceEqual(actualKeys, StringComparer.Ordinal))
                differences.Add($"{path}: keys [{string.Join(", ", expectedKeys)}] != [{string.Join(", ", actualKeys)}]");
            foreach (string key in expectedKeys.Intersect(actualKeys, StringComparer.Ordinal))
                CollectDifferences(expectedObject[key], actualObject[key], $"{path}.{key}", differences);
            return;
        }

        if (expected is JsonArray expectedArray && actual is JsonArray actualArray)
        {
            if (expectedArray.Count != actualArray.Count)
                differences.Add($"{path}: array length {expectedArray.Count} != {actualArray.Count}");
            for (int i = 0; i < Math.Min(expectedArray.Count, actualArray.Count); i++)
                CollectDifferences(expectedArray[i], actualArray[i], $"{path}[{i}]", differences);
            return;
        }

        if (expected is JsonValue expectedValue && actual is JsonValue actualValue)
        {
            if (!ScalarEquals(expectedValue, actualValue))
                differences.Add($"{path}: expected {DescribeNode(expectedValue)}, got {DescribeNode(actualValue)}");
            return;
        }

        differences.Add($"{path}: expected {DescribeNode(expected)}, got {DescribeNode(actual)}");
    }

    /// <summary>Compares scalars by VALUE: 1 and 1.0 are the same number, so formatting alone
    /// can never make the comparison brittle.</summary>
    private static bool ScalarEquals(JsonValue expected, JsonValue actual)
    {
        if (expected.TryGetValue<JsonElement>(out JsonElement expectedElement) &&
            actual.TryGetValue<JsonElement>(out JsonElement actualElement))
        {
            if (expectedElement.ValueKind != actualElement.ValueKind)
                return false;
            return expectedElement.ValueKind switch
            {
                JsonValueKind.Number => expectedElement.GetDouble() == actualElement.GetDouble(),
                JsonValueKind.String => string.Equals(expectedElement.GetString(), actualElement.GetString(), StringComparison.Ordinal),
                JsonValueKind.True or JsonValueKind.False => expectedElement.GetBoolean() == actualElement.GetBoolean(),
                JsonValueKind.Null => true,
                _ => expectedElement.GetRawText() == actualElement.GetRawText(),
            };
        }
        return expected.ToJsonString() == actual.ToJsonString();
    }

    /// <summary>Indexes a structured record's fields by name, which is how the protocol
    /// documents them (an ordered array on the wire, a map to a consumer).</summary>
    internal static JsonObject FieldsByName(JsonNode structured)
    {
        var fields = new JsonObject();
        foreach (JsonNode? field in structured["fields"]!.AsArray())
            fields[field!["name"]!.GetValue<string>()] = field["value"]!.DeepClone();
        return fields;
    }

    private static string DescribeNode(JsonNode? node) => node is null ? "JSON null" : Describe(node.ToJsonString());

    private static string Describe(string text) =>
        text.Length <= 240 ? text : text.Substring(0, 240) + "...";
}
