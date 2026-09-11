using System.Text;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// N23 digit-identity dump: prints, for sin / cos / tan at several arguments and precisions, the
/// FULL stored digit string (magnitude, exponent, period metadata and the rendered form) so a
/// before/after run can be diffed byte for byte. Heavy-category (the 1000-place cases are the very
/// thing under measurement) — run explicitly with
/// <c>$env:LOVELACE_PROBE_DIR='&lt;dir&gt;'; dotnet test Lovelace.Real.Tests -c Release --filter "FullyQualifiedName~RealTrigDigitDumpTests"</c>.
///
/// <para><see cref="Real"/> exposes no <c>Tan</c>; the tangent probe is
/// <c>DivideNonPeriodic(Sin(x), Cos(x), p + 10)</c> — the library's own fixed-point fast path, the
/// same shape any tan built on this numeric layer would take.</para>
/// </summary>
[Trait("Category", "Heavy")]
public class RealTrigDigitDumpTests
{
    private static readonly (string Name, Func<Real> Make)[] Args =
    {
        ("1/2", () => new Real("0.5")),
        ("1", () => new Real("1")),
        ("-1/3", () => new Real("-1") / new Real("3")),
        ("pi/6", () => Real.Pi / new Real("6")),
        ("100", () => new Real("100")),
    };

    private static readonly long[] Precisions = { 20, 80, 200, 1000 };

    [Fact]
    public void Dump_SinCosTan_FullStoredDigits()
    {
        var sb = new StringBuilder();
        sb.AppendLine("# N23 digit dump - full stored representation of sin/cos/tan");
        sb.AppendLine("# format: p=<computation places> arg=<name> fn=<sin|cos|tan> sign=<+|-> exp=<exponent> period=<start>/<len>");
        sb.AppendLine("#         digits=<FULL stored magnitude string>");
        sb.AppendLine("#         rendered=<ToString() under display places == computation places>");

        foreach (long p in Precisions)
        {
            using var scope = Real.WithPrecision(p, p);
            foreach (var (name, make) in Args)
            {
                Real arg = make();
                Real sin = Real.Sin(arg);
                Real cos = Real.Cos(arg);
                Real tan = Real.DivideNonPeriodic(sin, cos, p + 10);

                Dump(sb, p, name, "sin", sin);
                Dump(sb, p, name, "cos", cos);
                Dump(sb, p, name, "tan", tan);
            }
        }

        string text = sb.ToString();
        Console.WriteLine(text);
        File.WriteAllText(ProbePath("digits-dump.txt"), text);
    }

    private static void Dump(StringBuilder sb, long p, string argName, string fn, Real value)
    {
        sb.Append("p=").Append(p)
          .Append(" arg=").Append(argName)
          .Append(" fn=").Append(fn)
          .Append(" sign=").Append(Real.IsNegative(value) ? "-" : "+")
          .Append(" exp=").Append(value.Exponent)
          .Append(" period=").Append(value.PeriodStart).Append('/').Append(value.PeriodLength)
          .AppendLine();
        sb.Append("  digits=").AppendLine(value.ToNatural().ToString());
        sb.Append("  rendered=").AppendLine(value.ToString());
    }

    private static string ProbePath(string name)
    {
        string? dir = Environment.GetEnvironmentVariable("LOVELACE_PROBE_DIR");
        if (string.IsNullOrEmpty(dir)) dir = AppContext.BaseDirectory;
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, name);
    }
}
