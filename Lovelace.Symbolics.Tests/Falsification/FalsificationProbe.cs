using System.Globalization;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests.Falsification;

/// <summary>
/// ADV-11: one observed sample of one identity at one input, in a form that survives leaving the
/// process. A counterexample is only useful if it can be reported, hashed and replayed, so the
/// record carries its own rationale (the repository already requires a <c>Why</c> per probe - see
/// <c>RealProbe</c>) and round-trips through a deterministic text form.
/// </summary>
/// <param name="RuleId">The stable rewrite-rule id this sample attacks.</param>
/// <param name="Region">The region tag the probe came from (<c>near-pole</c>, <c>legacy-interior</c>, ...).</param>
/// <param name="Label">The probe's own short label.</param>
/// <param name="Input">The exact input, as text: a rational literal or a pair of exact reals.</param>
/// <param name="IsComplex">Whether <paramref name="Input"/> names a complex pair.</param>
/// <param name="Class">The observation class the falsifier branches on.</param>
/// <param name="LeftOutcome">What the left side did (evaluated, undefined, refused, or threw).</param>
/// <param name="RightOutcome">What the right side did.</param>
/// <param name="Left">The left-hand rendering.</param>
/// <param name="Right">The right-hand rendering.</param>
/// <param name="LeftValue">The left-hand side's value at this input, when it produced one.</param>
/// <param name="RightValue">The right-hand side's value at this input, when it produced one.</param>
/// <param name="DeltaText">The measured difference, rendered deterministically.</param>
/// <param name="Precision">The computation precision the sample was evaluated at.</param>
/// <param name="Tolerance">The comparison tolerance text the verdict used.</param>
/// <param name="Environment">The assumption environment the rule was licensed under.</param>
/// <param name="GeneratorVersion">The generator that produced the input, so a corpus is versioned.</param>
/// <param name="Why">Why this input is near the feature its region names.</param>
public sealed record FalsificationProbe(
    string RuleId,
    string Region,
    string Label,
    string Input,
    bool IsComplex,
    ObservationClass Class,
    SideOutcome LeftOutcome,
    SideOutcome RightOutcome,
    string Left,
    string Right,
    string LeftValue,
    string RightValue,
    string DeltaText,
    long Precision,
    string Tolerance,
    string Environment,
    string GeneratorVersion,
    string Why)
{
    /// <summary>The generator version stamped on every record. A change to the sample
    /// generation must bump this, so a stored corpus can never be confused with a new run.</summary>
    public const string CurrentGeneratorVersion = "adv11-falsifier/1";

    /// <summary>True when this sample is a counterexample to the identity.</summary>
    public bool IsCounterexample => Class == ObservationClass.Disagree;

    /// <summary>True when a side threw something the kernel does not expect: a defect in the
    /// kernel or in the harness, reported as itself rather than as an observation class.</summary>
    public bool IsDefect => LeftOutcome == SideOutcome.Threw || RightOutcome == SideOutcome.Threw;

    /// <summary>The record's deterministic, line-delimited text form. Fields are separated by
    /// <c>|</c>; the two free-text fields are escaped, so a <c>|</c> in a rendering cannot shift a
    /// field. Round-trips exactly (see <see cref="Parse"/>).</summary>
    public string ToText()
    {
        var fields = new[]
        {
            RuleId, Region, Label, Input, IsComplex ? "complex" : "real", Class.ToString(),
            LeftOutcome.ToString(), RightOutcome.ToString(),
            Left, Right, LeftValue, RightValue, DeltaText,
            Precision.ToString(CultureInfo.InvariantCulture),
            Tolerance, Environment, GeneratorVersion, Why,
        };
        // EVERY field is escaped and the whole record is then unescaped on the way back, so a
        // field can carry the separator (or any control character) without shifting the others.
        return string.Join("|", fields.Select(FalsificationText.Escape));
    }

    /// <summary>Parses the form <see cref="ToText"/> writes. Throws on a malformed record
    /// rather than guessing: a corpus that does not parse is a corpus that cannot be trusted.</summary>
    public static FalsificationProbe Parse(string text)
    {
        var fields = text.Split('|');
        if (fields.Length != 18)
            throw new FormatException($"a falsification probe has 18 fields; got {fields.Length}.");
        var unescaped = fields.Select(FalsificationText.Unescape).ToArray();
        return new FalsificationProbe(
            unescaped[0], unescaped[1], unescaped[2], unescaped[3],
            unescaped[4] == "complex",
            Enum.Parse<ObservationClass>(unescaped[5]),
            Enum.Parse<SideOutcome>(unescaped[6]),
            Enum.Parse<SideOutcome>(unescaped[7]),
            unescaped[8], unescaped[9], unescaped[10], unescaped[11], unescaped[12],
            long.Parse(unescaped[13], CultureInfo.InvariantCulture),
            unescaped[14], unescaped[15], unescaped[16], unescaped[17]);
    }

    /// <summary>The probe's identity for hashing and de-duplication: the rule, the input, and the
    /// environment. Two samples that agree on these are the same sample.</summary>
    public string IdentityKey => RuleId + "@" + (IsComplex ? "C:" : "R:") + Input + "#" + Environment;
}

/// <summary>A deterministic, human- and machine-readable falsification report.</summary>
public sealed record FalsificationReport(
    string GeneratorVersion,
    int SampleCount,
    IReadOnlyList<FalsificationProbe> Probes,
    IReadOnlyList<string> Defects)
{
    /// <summary>Every counterexample the run found, in report order.</summary>
    public IReadOnlyList<FalsificationProbe> Counterexamples =>
        Probes.Where(x => x.IsCounterexample).ToArray();

    /// <summary>A defect (an unexpected exception type) is a failure of the run, not a class.</summary>
    public bool Failed => Defects.Count > 0 || Counterexamples.Count > 0;

    /// <summary>
    /// The report's deterministic text form: a header line, then one line per probe in report
    /// order. Report order is the caller's responsibility and every producer here sorts, so the
    /// same run produces byte-identical text in any process.
    /// </summary>
    public string ToText()
    {
        var sb = new System.Text.StringBuilder();
        sb.Append("falsification-report ").Append(GeneratorVersion)
          .Append(" samples=").Append(SampleCount.ToString(CultureInfo.InvariantCulture))
          .Append(" counterexamples=").Append(Counterexamples.Count.ToString(CultureInfo.InvariantCulture))
          .Append(" defects=").Append(Defects.Count.ToString(CultureInfo.InvariantCulture))
          .Append('\n');
        foreach (var probe in Probes)
            sb.Append(probe.ToText()).Append('\n');
        foreach (var defect in Defects)
            sb.Append("DEFECT|").Append(FalsificationText.Escape(defect)).Append('\n');
        return sb.ToString();
    }

    /// <summary>Parses the text <see cref="ToText"/> writes, so a stored report can be compared
    /// field for field rather than only by hash.</summary>
    public static FalsificationReport Parse(string text)
    {
        var lines = text.Split('\n', StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length == 0)
            throw new FormatException("a falsification report needs a header line.");
        var header = lines[0].Split(' ');
        if (header.Length != 5 || header[0] != "falsification-report")
            throw new FormatException("malformed falsification report header: " + lines[0]);
        var samples = int.Parse(header[2].Split('=')[1], CultureInfo.InvariantCulture);
        var probes = new List<FalsificationProbe>();
        var defects = new List<string>();
        for (var i = 1; i < lines.Length; i++)
        {
            if (lines[i].StartsWith("DEFECT|", StringComparison.Ordinal))
                defects.Add(FalsificationText.Unescape(lines[i]["DEFECT|".Length..]));
            else
                probes.Add(FalsificationProbe.Parse(lines[i]));
        }
        return new FalsificationReport(header[1], samples, probes, defects);
    }

    /// <summary>The report's stable hash, over exactly the text <see cref="ToText"/> produces.
    /// Two runs that agree on this hash agree on every probe, every class and every rendering.
    /// </summary>
    public string Fingerprint()
    {
        var bytes = System.Text.Encoding.UTF8.GetBytes(ToText());
        return Convert.ToHexString(System.Security.Cryptography.SHA256.HashData(bytes));
    }
}
