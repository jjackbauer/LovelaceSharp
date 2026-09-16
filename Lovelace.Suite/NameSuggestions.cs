namespace Lovelace.Suite;

/// <summary>
/// Name-resolution help for a call the engine cannot resolve.
/// <para>
/// The engine knows every name it registers. When a caller asks for a name it does not have, that
/// knowledge is the cheapest and most useful thing it can hand back: a caller that wrote
/// <c>determinant</c> where the engine has <c>det</c> is not making a mathematics mistake, it is
/// guessing at a vocabulary. The harnessability benchmark measured what withholding it costs — four
/// of twenty repair tasks stall in every observation mode that cannot reach the registry, and every
/// one of them stalls on exactly this.
/// </para>
/// <para>
/// The order is total (rank, then ordinal name) so two runs over the same registry produce the same
/// list, and an oracle fingerprint can never depend on dictionary iteration order.
/// </para>
/// </summary>
public static class NameSuggestions
{
    /// <summary>The most names one failure offers. A caller needs a lead, not a catalogue.</summary>
    public const int MaxSuggestions = 3;

    /// <summary>
    /// Synonyms no string metric reaches: an English word for an operation whose registered name is
    /// an abbreviation (<c>derivative</c> vs <c>diff</c>). Every target is a registered builtin; a
    /// target that is absent from the live registry is dropped rather than suggested.
    /// </summary>
    private static readonly Dictionary<string, string[]> Synonyms = new(StringComparer.OrdinalIgnoreCase)
    {
        ["determinant"] = new[] { "det" },
        ["inverse"] = new[] { "inv" },
        ["invert"] = new[] { "inv" },
        ["derivative"] = new[] { "diff" },
        ["derive"] = new[] { "diff" },
        ["differentiate"] = new[] { "diff" },
        ["average"] = new[] { "mean" },
        ["avg"] = new[] { "mean" },
        ["length"] = new[] { "len" },
        ["size"] = new[] { "len" },
        ["count"] = new[] { "len" },
        ["factorize"] = new[] { "factor" },
        ["factorise"] = new[] { "factor" },
        ["rows"] = new[] { "shape" },
        ["cols"] = new[] { "shape" },
        ["columns"] = new[] { "shape" },
        ["dimensions"] = new[] { "shape" },
        ["dims"] = new[] { "shape" },
        ["identity"] = new[] { "eye" },
        ["numelements"] = new[] { "numel" },
        ["trans"] = new[] { "transpose" },
        ["matrixmultiply"] = new[] { "matmul" },
    };

    /// <summary>
    /// Notes for names that are not missing functions at all: a language feature the caller reached
    /// for as a function, or a builtin the language deliberately does not have. These are the names
    /// no registry entry can ever answer, which is exactly why they need an answer.
    /// </summary>
    private static readonly Dictionary<string, string> SyntaxHints = new(StringComparer.OrdinalIgnoreCase)
    {
        ["range"] = "The range operator is written 1..n; no builtin is needed.",
        ["map"] = "There is no map builtin: arithmetic and powers broadcast over arrays, so (1..n)^2 squares elementwise.",
        ["help"] = "The runtime publishes its function registry (names, parameters and arity) for lookup.",
    };

    /// <summary>The registered names closest to <paramref name="queried"/>, best first.</summary>
    public static IReadOnlyList<string> Suggest(IEnumerable<string> registered, string queried)
    {
        var names = registered as IReadOnlyCollection<string> ?? registered.ToList();
        var ranked = new List<(int Rank, string Name)>();
        foreach (var name in names)
        {
            int rank = Rank(name, queried);
            if (rank >= 0)
                ranked.Add((rank, name));
        }

        if (Synonyms.TryGetValue(queried, out var synonyms))
        {
            var present = new HashSet<string>(names, StringComparer.Ordinal);
            foreach (var synonym in synonyms)
                if (present.Contains(synonym))
                    ranked.Add((0, synonym));
        }

        return ranked
            .OrderBy(candidate => candidate.Rank)
            .ThenBy(candidate => candidate.Name, StringComparer.Ordinal)
            .Select(candidate => candidate.Name)
            .Distinct(StringComparer.Ordinal)
            .Take(MaxSuggestions)
            .ToArray();
    }

    /// <summary>The syntax notes that apply to <paramref name="queried"/> (empty when none do).</summary>
    public static IReadOnlyList<string> HintsFor(string queried) =>
        SyntaxHints.TryGetValue(queried, out var hint) ? new[] { hint } : Array.Empty<string>();

    /// <summary>
    /// How close a registered name is to the one that failed: 0 for a case-only difference, 1 for a
    /// prefix of the other (<c>determinant</c>/<c>det</c>, <c>inverse</c>/<c>inv</c>), 2 for a
    /// near-miss, 3 for a long shared opening, and -1 when the two are unrelated.
    /// </summary>
    public static int Rank(string registered, string queried)
    {
        if (registered.Equals(queried, StringComparison.OrdinalIgnoreCase))
            return 0;

        // Short names are not evidence: "a" and "e" are one edit apart and mean nothing to each other.
        if (registered.Length >= 3 && queried.Length >= 3)
        {
            if (registered.StartsWith(queried, StringComparison.OrdinalIgnoreCase) ||
                queried.StartsWith(registered, StringComparison.OrdinalIgnoreCase))
                return 1;

            // Short names are dense: "fft" is two edits from both "det" and "dot", and neither is a
            // guess a caller would thank us for. A near-miss only counts when the name is long enough
            // for the distance to carry information.
            int tolerance = Math.Min(registered.Length, queried.Length) <= 4 ? 1 : 2;
            if (EditDistance(registered, queried) <= tolerance)
                return 2;
        }

        return SharedPrefix(registered, queried) >= 4 ? 3 : -1;
    }

    private static int SharedPrefix(string a, string b)
    {
        int shared = 0;
        int limit = Math.Min(a.Length, b.Length);
        while (shared < limit && char.ToLowerInvariant(a[shared]) == char.ToLowerInvariant(b[shared]))
            shared++;
        return shared;
    }

    /// <summary>Levenshtein distance, capped: a pair more than two edits apart costs the same here.</summary>
    public static int EditDistance(string a, string b)
    {
        if (Math.Abs(a.Length - b.Length) > 2)
            return int.MaxValue;

        var previous = new int[b.Length + 1];
        var current = new int[b.Length + 1];
        for (int j = 0; j <= b.Length; j++)
            previous[j] = j;

        for (int i = 1; i <= a.Length; i++)
        {
            current[0] = i;
            for (int j = 1; j <= b.Length; j++)
            {
                int substitution = previous[j - 1] +
                    (char.ToLowerInvariant(a[i - 1]) == char.ToLowerInvariant(b[j - 1]) ? 0 : 1);
                current[j] = Math.Min(Math.Min(current[j - 1] + 1, previous[j] + 1), substitution);
            }
            (previous, current) = (current, previous);
        }

        return previous[b.Length];
    }
}
