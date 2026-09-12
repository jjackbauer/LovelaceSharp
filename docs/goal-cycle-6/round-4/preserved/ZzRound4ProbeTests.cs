using System.Text;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>TEMPORARY round-4 probe (deleted before the round lands). Dumps the Real-level
/// behaviour of the special-angle fast path, compactly, so tests can be written against
/// observed reality. Writes docs/goal-cycle-6/round-4/probe-real.txt.</summary>
public class ZzRound4ProbeTests
{
    private static string Cut(string s, int n = 88) => s.Length <= n ? s : s[..n] + "…";

    private static string F(Real r) => (r.IsExact ? "exact  " : "inexact") + " exp=" + r.Exponent.ToString().PadLeft(5) + " " + Cut(r.ToString());

    private static Real PiAt(long places)
    {
        using var scope = Real.WithPrecision(places, places);
        return Real.PiTo(places);
    }

    [Fact]
    public void DumpRealLevelTrig()
    {
        var sb = new StringBuilder();
        foreach (long p in new long[] { 30, 40, 100 })
        {
            using var scope = Real.WithPrecision(p, p);
            sb.AppendLine($"### ambient computation/display = {p}");
            Real pi = Real.Pi;
            sb.AppendLine($"Real.Pi            {F(pi)}");
            if (p >= 30)
            {
                Real pi30 = Real.PiTo(30);
                sb.AppendLine($"Real.PiTo(30)      {F(pi30)}");
                Dump(sb, "Cos(pi30)", Real.Cos(pi30));
                Dump(sb, "Sin(pi30)", Real.Sin(pi30));
                Dump(sb, "Cos(pi30*2)", Real.Cos(pi30 * new Real("2")));
                Dump(sb, "Sin(pi30*2)", Real.Sin(pi30 * new Real("2")));
                Dump(sb, "Cos(pi30/2)", Real.Cos(pi30 / new Real("2")));
                Dump(sb, "Sin(pi30/2)", Real.Sin(pi30 / new Real("2")));
                Dump(sb, "Cos(pi30/3)", Real.Cos(pi30 / new Real("3")));
                Dump(sb, "Sin(pi30/3)", Real.Sin(pi30 / new Real("3")));
                Dump(sb, "Cos(pi30/4)", Real.Cos(pi30 / new Real("4")));
                Dump(sb, "Sin(pi30/4)", Real.Sin(pi30 / new Real("4")));
                Dump(sb, "Cos(pi30/6)", Real.Cos(pi30 / new Real("6")));
                Dump(sb, "Sin(pi30/6)", Real.Sin(pi30 / new Real("6")));
                Dump(sb, "Sin(-pi30/6)", Real.Sin(-(pi30 / new Real("6"))));
            }
            if (p >= 40) { Dump(sb, "Cos(pi40)", Real.Cos(Real.PiTo(40))); Dump(sb, "Sin(pi40)", Real.Sin(Real.PiTo(40))); }
            if (p >= 100) { Dump(sb, "Cos(pi100)", Real.Cos(Real.PiTo(100))); Dump(sb, "Sin(pi100)", Real.Sin(Real.PiTo(100))); }
            Dump(sb, "Cos(Real.Pi)", Real.Cos(pi));
            Dump(sb, "Sin(Real.Pi)", Real.Sin(pi));
            Dump(sb, "Cos(Real.Pi/2)", Real.Cos(pi / new Real("2")));
            Dump(sb, "Sin(Real.Pi/2)", Real.Sin(pi / new Real("2")));
            Dump(sb, "Cos(Real.Pi/6)", Real.Cos(pi / new Real("6")));
            Dump(sb, "Sin(Real.Pi/6)", Real.Sin(pi / new Real("6")));
            Dump(sb, "Cos(Real.Pi/3)", Real.Cos(pi / new Real("3")));
            Dump(sb, "Sin(Real.Pi/3)", Real.Sin(pi / new Real("3")));
            Dump(sb, "Cos(Real.Pi/4)", Real.Cos(pi / new Real("4")));
            Dump(sb, "Sin(Real.Pi/4)", Real.Sin(pi / new Real("4")));
            Dump(sb, "Cos(Zero)", Real.Cos(Real.Zero));
            Dump(sb, "Sin(Zero)", Real.Sin(Real.Zero));
            Dump(sb, "Cos(One)", Real.Cos(Real.One));
            Dump(sb, "Sin(One)", Real.Sin(Real.One));
            Dump(sb, "Cos(0.5)", Real.Cos(new Real("0.5")));
            Dump(sb, "Sin(0.5)", Real.Sin(new Real("0.5")));
            sb.AppendLine();
        }

        {
            using var scope = Real.WithPrecision(1000, 100);
            Real pi30 = PiAt(30);
            Dump(sb, "ambient1000 Cos(pi30, 100)", Real.Cos(pi30, 100));
            Dump(sb, "ambient1000 Cos(pi30, 40)", Real.Cos(pi30, 40));
            Dump(sb, "ambient1000 Sin(pi30, 100)", Real.Sin(pi30, 100));
            Dump(sb, "ambient1000 Cos(Real.Pi)", Real.Cos(Real.Pi));
            Dump(sb, "ambient1000 Sin(Real.Pi/6)", Real.Sin(Real.Pi / new Real("6")));
        }

        Directory.CreateDirectory(@"C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-4");
        File.WriteAllText(@"C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-4\probe-real.txt", sb.ToString());
    }

    private static void Dump(StringBuilder sb, string name, Real value) =>
        sb.AppendLine($"{name,-26} {F(value)}");
}
