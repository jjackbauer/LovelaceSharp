namespace Lovelace.Console.Tests;

/// <summary>
/// Drives a <see cref="Lovelace.Console.Repl.ReplSession"/> headlessly: commands come
/// from an in-memory reader and the whole session transcript is captured in memory, so
/// tests assert on the text a user would actually see.
/// </summary>
internal static class ReplHarness
{
    /// <summary>Runs <paramref name="script"/> (one command per line, "\n"-separated) to
    /// completion and returns everything the session wrote.</summary>
    public static async Task<string> RunAsync(string script)
    {
        var input = new StringReader(script);
        var output = new StringWriter();
        var session = new Lovelace.Console.Repl.ReplSession(input, output);
        await session.RunAsync();
        return output.ToString();
    }

    /// <summary>Splits a transcript into its non-empty, right-trimmed lines.</summary>
    public static string[] Lines(string transcript) =>
        transcript.Replace("\r\n", "\n")
                  .Split('\n')
                  .Select(l => l.TrimEnd())
                  .Where(l => l.Length > 0)
                  .ToArray();

    /// <summary>True when the transcript contains <paramref name="needle"/> as a whole line.</summary>
    public static bool HasLine(string transcript, string needle) =>
        Lines(transcript).Contains(needle);

    /// <summary>Asserts that <paramref name="expected"/> appears as a whole line, quoting the
    /// transcript on failure so a mismatch is diagnosable without re-running the test.</summary>
    public static void AssertHasLine(string transcript, string expected) =>
        Assert.True(HasLine(transcript, expected),
            $"expected line:{Environment.NewLine}  {expected}{Environment.NewLine}in transcript:{Environment.NewLine}{transcript}");

    /// <summary>Asserts that some line starts with <paramref name="prefix"/> (used for result
    /// lines, which end with a variable elapsed-time suffix).</summary>
    public static void AssertHasLineStartingWith(string transcript, string prefix) =>
        Assert.True(Lines(transcript).Any(l => l.StartsWith(prefix, StringComparison.Ordinal)),
            $"expected a line starting with:{Environment.NewLine}  {prefix}{Environment.NewLine}in transcript:{Environment.NewLine}{transcript}");
}
