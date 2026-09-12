using System.Diagnostics;
using System.Globalization;
using System.Text;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Real;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Int = Lovelace.Integer.Integer;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

// TEMPORARY cost probe -- not part of the suite, deleted after measurement.
public class ZzPairCostProbe
{
    private static readonly Dictionary<Symbol, Num> NoBindings = new();
    private static Rl N(long value) => new(new Int(value));

    private static (string Label, Rl Value)[] TruncatedArguments()
    {
        using var scope = Rl.WithPrecision(60, 50);
        var pi30 = Rl.PiTo(30);
        var e30 = Rl.ETo(30);
        var root2 = Rl.Sqrt(N(2));
        return new (string, Rl)[]
        {
            ("pi30", pi30),
            ("pi41", Rl.PiTo(41)),
            ("e30", e30),
            ("e49", Rl.ETo(49)),
            ("sqrt2", root2),
            ("sqrt3", Rl.Sqrt(N(3))),
            ("sin1", Rl.Sin(Rl.One)),
            ("cos1", Rl.Cos(Rl.One)),
            ("exp1", Rl.Exp(Rl.One)),
            ("exp2", Rl.Exp(N(2))),
            ("pi_10", pi30 / N(10)),
            ("pi_6", pi30 / N(6)),
            ("pi_4", pi30 / N(4)),
            ("pi_3", pi30 / N(3)),
            ("sqrt2_2", root2 / N(2)),
            ("e_4", e30 / N(4)),
        };
    }

    private static readonly string[] AllFunctions = { "sin", "cos", "tan", "exp", "atan", "sinh", "cosh", "tanh", "sqrt" };

    private static string[] Pick(string? spec, string[] all) =>
        string.IsNullOrWhiteSpace(spec) || spec == "all"
            ? all
            : spec.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);

    private static double Ms(long a, long b) => (b - a) * 1000.0 / Stopwatch.Frequency;

    [Fact]
    public void ZzPairCost()
    {
        var outDir = Environment.GetEnvironmentVariable("LOVELACE_PC_OUT") ?? Path.GetTempPath();
        Directory.CreateDirectory(outDir);
        var tag = Environment.GetEnvironmentVariable("LOVELACE_PC_TAG") ?? "run";
        var prec = (Environment.GetEnvironmentVariable("LOVELACE_PC_PREC") ?? "60,50").Split(',');
        long computation = long.Parse(prec[0], CultureInfo.InvariantCulture);
        long display = long.Parse(prec[1], CultureInfo.InvariantCulture);
        var argFilter = Pick(Environment.GetEnvironmentVariable("LOVELACE_PC_ARGS"), Array.Empty<string>());
        var fns = Pick(Environment.GetEnvironmentVariable("LOVELACE_PC_FNS"), AllFunctions);

        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var sb = new StringBuilder();
        sb.AppendLine("arg,fn,literalMs,nodeMs,evalMs,isExact,totalMs");

        var familySw = Stopwatch.StartNew();
        var family = TruncatedArguments();
        familySw.Stop();

        var args = argFilter.Length == 0 ? family : family.Where(a => argFilter.Contains(a.Label)).ToArray();
        var missing = argFilter.Where(x => !family.Any(a => a.Label == x)).ToArray();
        if (missing.Length > 0) sb.AppendLine("#MISSING_ARGS," + string.Join('|', missing));

        double total = 0;
        int inexact = 0, exact = 0;
        var exactPairs = new List<string>();

        using (Rl.WithPrecision(computation, display))
        {
            foreach (var (label, value) in args)
            {
                foreach (var name in fns)
                {
                    var t0 = Stopwatch.GetTimestamp();
                    var literal = RealLiteral.FromRealExact(value);
                    var back = literal.ToReal();
                    var t1 = Stopwatch.GetTimestamp();
                    var node = name == "sqrt"
                        ? Exprs.Sqrt(Exprs.Real(literal))
                        : Exprs.Function(ctx.Function(name), Exprs.Real(literal));
                    var t2 = Stopwatch.GetTimestamp();
                    var result = Assert.IsType<NumReal>(Evaluation.EvaluateToNum(node, ctx, NoBindings)).V;
                    var t3 = Stopwatch.GetTimestamp();
                    var ms = Ms(t2, t3);
                    total += ms;
                    if (result.IsExact) { exact++; exactPairs.Add(name + "(" + label + ")"); } else { inexact++; }
                    var text = result.ToString();
                    if (text.Length > 22) text = text.Substring(0, 22);
                    sb.AppendLine(string.Format(CultureInfo.InvariantCulture,
                        "{0},{1},{2:F2},{3:F2},{4:F1},{5},{6:F1}",
                        label, name, Ms(t0, t1), Ms(t1, t2), ms, result.IsExact, Ms(t0, t3)));
                }
            }
        }

        File.WriteAllText(Path.Combine(outDir, "paircost-" + tag + ".csv"), sb.ToString());
        var summary = string.Format(CultureInfo.InvariantCulture,
            "TAG={0} prec={1}/{2} args={3} fns={4} pairs={5} evalTotalMs={6:F0} familyBuildMs={7:F0} inexact={8} EXACT(fail)={9} [{10}]",
            tag, computation, display, args.Length, fns.Length, inexact + exact, total, familySw.Elapsed.TotalMilliseconds,
            inexact, exact, string.Join(" ", exactPairs));
        File.AppendAllText(Path.Combine(outDir, "summary.txt"), summary + Environment.NewLine);
        Console.WriteLine(summary);
    }
}
