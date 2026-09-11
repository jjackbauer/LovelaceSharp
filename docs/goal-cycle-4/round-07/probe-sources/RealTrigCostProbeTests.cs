using System.Diagnostics;
using System.Reflection;
using System.Text;
using Lovelace.Real;

using Int = Lovelace.Integer.Integer;
using Nat = Lovelace.Natural.Natural;

namespace Lovelace.Real.Tests;

/// <summary>
/// N23 cost probe: measures where <see cref="Real.Cos(Real)"/> spends its time at the project's
/// default computation precision (1000 decimal places). Heavy-category so the default suite skips
/// it; run it explicitly with
/// <c>dotnet test Lovelace.Real.Tests -c Release --filter "FullyQualifiedName~RealTrigCostProbeTests" --logger "console;verbosity=detailed"</c>.
/// </summary>
[Trait("Category", "Heavy")]
public class RealTrigCostProbeTests
{
    private const BindingFlags Priv = BindingFlags.NonPublic | BindingFlags.Static;
    private static readonly MethodInfo ReduceMi = typeof(Real).GetMethod("ReduceToTwoPi", Priv)!;
    private static readonly MethodInfo SpecialMi = typeof(Real).GetMethod("TrySpecialAngle", Priv)!;
    private static readonly MethodInfo CosTaylorMi = typeof(Real).GetMethod("CosTaylor", Priv)!;
    private static readonly MethodInfo SinTaylorMi = typeof(Real).GetMethod("SinTaylor", Priv)!;
    private static readonly MethodInfo NormalizeMi = typeof(Real).GetMethod("Normalize", Priv)!;

    private static double TicksToMs(long ticks) => ticks * 1000.0 / Stopwatch.Frequency;

    private static (double Ms, T Result) Time<T>(Func<T> work)
    {
        long t0 = Stopwatch.GetTimestamp();
        T r = work();
        return (TicksToMs(Stopwatch.GetTimestamp() - t0), r);
    }

    /// <summary>The exact rows of the private special-angle table, in source order.</summary>
    private static readonly (long Num, long Den)[] Table =
    {
        (0, 1), (1, 6), (1, 4), (1, 3), (1, 2), (2, 3), (3, 4), (5, 6),
        (1, 1), (7, 6), (5, 4), (4, 3), (3, 2), (5, 3), (7, 4), (11, 6),
    };

    [Fact]
    public void Probe_PhaseBreakdown_Cos_At1000Places()
    {
        const long p = 1000;
        const long guard = p + 10;
        var log = new StringBuilder();

        using var scope = Real.WithPrecision(p, p);

        var (piMs, pi) = Time(() => Real.Pi);
        log.AppendLine($"Pi (cached afterwards):                            {piMs,10:F1} ms");

        var (twoPiMs, twoPi) = Time(() => pi * new Real("2"));
        log.AppendLine($"twoPi = pi*2:                                      {twoPiMs,10:F1} ms");

        var (halfPiMs, _) = Time(() => pi / new Real("2"));
        log.AppendLine($"halfPi = pi/2 (period-detecting Divide):           {halfPiMs,10:F1} ms");

        var halfOfMi = typeof(Real).GetMethod("HalfOf", Priv);
        if (halfOfMi is not null)
        {
            var (halfOfMs, _) = Time(() => (Real)halfOfMi.Invoke(null, new object[] { pi })!);
            log.AppendLine($"halfPi via HalfOf (the path Sin/Cos now use):      {halfOfMs,10:F1} ms");
        }

        var (redHalfMs, _) = Time(() => (Real)ReduceMi.Invoke(null, new object[] { new Real("0.5"), pi, twoPi })!);
        log.AppendLine($"ReduceToTwoPi(0.5) (no-op branch):                 {redHalfMs,10:F1} ms");

        var (red100Ms, _) = Time(() => (Real)ReduceMi.Invoke(null, new object[] { new Real("100"), pi, twoPi })!);
        log.AppendLine($"ReduceToTwoPi(100) (real reduction, Divide path):  {red100Ms,10:F1} ms");

        Real third = new Real("1") / new Real("3");
        var (redThirdMs, _) = Time(() => (Real)ReduceMi.Invoke(null, new object[] { third, pi, twoPi })!);
        log.AppendLine($"ReduceToTwoPi(1/3) (no-op branch):                 {redThirdMs,10:F1} ms");

        var (spHalfMs, _) = Time(() => (bool)SpecialMi.Invoke(null, new object[] { new Real("0.5"), pi, null!, null! })!);
        log.AppendLine($"TrySpecialAngle(0.5) (miss -> all 16 rows):        {spHalfMs,10:F1} ms");

        Real piOver6 = pi / new Real("6");
        var (spHitMs, _) = Time(() => (bool)SpecialMi.Invoke(null, new object[] { piOver6, pi, null!, null! })!);
        log.AppendLine($"TrySpecialAngle(pi/6) (hit at row 1):              {spHitMs,10:F1} ms");

        var xHalf = new Real("0.5");
        var (cosTaylorMs, _) = Time(() => (Real)CosTaylorMi.Invoke(null, new object[] { xHalf, guard })!);
        log.AppendLine($"CosTaylor(0.5, {guard}):                            {cosTaylorMs,10:F1} ms");

        var (sinTaylorMs, _) = Time(() => (Real)SinTaylorMi.Invoke(null, new object[] { xHalf, guard })!);
        log.AppendLine($"SinTaylor(0.5, {guard}):                            {sinTaylorMs,10:F1} ms");

        var (cosMs, cosVal) = Time(() => Real.Cos(new Real("0.5")));
        log.AppendLine($"Real.Cos(0.5) TOTAL (public, one call):            {cosMs,10:F1} ms");
        log.AppendLine($"  -> stored: exp={cosVal.Exponent} digits={cosVal.ToNatural().ToString().Length} periodic={cosVal.IsPeriodic}");

        log.AppendLine();
        log.AppendLine("--- special-angle table, row by row (pi*num/den) ---");
        double tableTotal = 0.0;
        foreach (var (num, den) in Table)
        {
            var (rowMs, _) = Time<Real>(() => num == 0 ? Real.Zero : pi * new Real(new Int(num)) / new Real(new Int(den)));
            tableTotal += rowMs;
            log.AppendLine($"  num={num,2} den={den,2}: {rowMs,10:F1} ms");
        }
        log.AppendLine($"  table total: {tableTotal,10:F1} ms (sum of the 16 rows as timed here)");

        log.AppendLine();
        log.AppendLine("--- one representative comparison ---");
        Real angle16 = pi * new Real(new Int(1)) / new Real(new Int(6));
        var (cmpMs, _) = Time(() => xHalf.Equals(angle16));
        log.AppendLine($"  x.Equals(pi/6) one row:                          {cmpMs,10:F1} ms");

        Console.WriteLine(log.ToString());
        File.WriteAllText(ProbePath("phase-breakdown.txt"), log.ToString());
    }

    [Fact]
    public void Probe_SeriesInternals_Cos_At1000Places()
    {
        const long p = 1000;
        const long guard = p + 10;
        var log = new StringBuilder();

        using var scope = Real.WithPrecision(p, p);

        Real x = new Real("0.5");
        var (x2Ms, x2) = Time(() => x * x);
        var (thrMs, threshold) = Time(() => new Real("0." + new string('0', (int)guard) + "1"));

        long tMul = 0, tDen = 0, tDiv = 0, tAdd = 0, tCmp = 0;
        long tNatConv = 0, tShift = 0, tNatDiv = 0, tNorm = 0;
        Real term = Real.One, sum = Real.One;
        long iters = 0;

        long t0 = Stopwatch.GetTimestamp();
        for (long k = 1; k <= 5000; k++)
        {
            long a = Stopwatch.GetTimestamp();
            Real prod = -term * x2;
            long b = Stopwatch.GetTimestamp(); tMul += b - a;

            long denom = (2 * k - 1) * (2 * k);
            var den = new Real(new Int(denom));
            long c = Stopwatch.GetTimestamp(); tDen += c - b;

            // Replicates DivideNonPeriodic's body, op by op.
            long m0 = Stopwatch.GetTimestamp();
            Nat numerator = prod.ToNatural();
            long m1 = Stopwatch.GetTimestamp(); tNatConv += m1 - m0;
            Nat scaled = numerator.ShiftLeftDecimal(guard);
            long m2 = Stopwatch.GetTimestamp(); tShift += m2 - m1;
            Nat q = Nat.DivRem(scaled, den.ToNatural(), out _);
            long m3 = Stopwatch.GetTimestamp(); tNatDiv += m3 - m2;
            var raw = new Real(q, Real.IsNegative(prod), -guard + (prod.Exponent - den.Exponent));
            Real normalized = (Real)NormalizeMi.Invoke(null, new object[] { raw })!;
            long m5 = Stopwatch.GetTimestamp(); tNorm += m5 - m3;
            term = normalized;
            long d = Stopwatch.GetTimestamp(); tDiv += d - c;

            sum = sum + term;
            long e = Stopwatch.GetTimestamp(); tAdd += e - d;

            if (Real.Abs(term) < threshold) { iters = k; break; }
            long f = Stopwatch.GetTimestamp(); tCmp += f - e;
        }
        long total = Stopwatch.GetTimestamp() - t0;

        var (strMs, str) = Time(() => sum.ToNatural().ToString());

        // The replacement termination test, probed defensively through reflection so the same file
        // runs against both the pre-fix and post-fix trees.
        string guardLine = "  IsBelowDecimalGuard (new):  n/a in this build";
        var guardMi = typeof(Real).GetMethod("IsBelowDecimalGuard", Priv);
        if (guardMi is not null)
        {
            Nat? power = null;
            long powerExponent = 0L;
            Real guarded = Real.One;
            long guardedTerms = 0L;
            long g0 = Stopwatch.GetTimestamp();
            for (long k = 1; k <= 5000; k++)
            {
                long denom = (2 * k - 1) * (2 * k);
                guarded = Real.DivideNonPeriodic(-guarded * x2, new Real(new Int(denom)), guard);
                object?[] gargs = { guarded, guard, power, powerExponent };
                bool done = (bool)guardMi.Invoke(null, gargs)!;
                power = (Nat?)gargs[2];
                powerExponent = (long)gargs[3]!;
                guardedTerms = k;
                if (done)
                    break;
            }
            guardLine = $"  IsBelowDecimalGuard (new):  {TicksToMs(Stopwatch.GetTimestamp() - g0),10:F1} ms over {guardedTerms} terms";
        }

        log.AppendLine($"x2 = x*x:                     {x2Ms,10:F1} ms");
        log.AppendLine($"threshold parse (1012 chars): {thrMs,10:F1} ms");
        log.AppendLine($"series iterations:            {iters}");
        log.AppendLine($"series TOTAL (loop):          {TicksToMs(total),10:F1} ms");
        log.AppendLine($"  -term * x2     (Multiply):  {TicksToMs(tMul),10:F1} ms");
        log.AppendLine($"  denom Real ctor:            {TicksToMs(tDen),10:F1} ms");
        log.AppendLine($"  DivideNonPeriodic:          {TicksToMs(tDiv),10:F1} ms");
        log.AppendLine($"    ToNatural():              {TicksToMs(tNatConv),10:F1} ms");
        log.AppendLine($"    ShiftLeftDecimal:         {TicksToMs(tShift),10:F1} ms");
        log.AppendLine($"    Nat.DivRem:               {TicksToMs(tNatDiv),10:F1} ms");
        log.AppendLine($"    Normalize:                {TicksToMs(tNorm),10:F1} ms");
        log.AppendLine($"  sum + term      (Add):      {TicksToMs(tAdd),10:F1} ms");
        log.AppendLine($"  Abs(term) < thr (old cmp):  {TicksToMs(tCmp),10:F1} ms");
        log.AppendLine(guardLine);
        log.AppendLine($"one Nat.ToString() of sum:    {strMs,10:F1} ms (len={str.Length})");
        log.AppendLine($"stored digits={sum.ToNatural().ToString().Length} exp={sum.Exponent} periodic={sum.IsPeriodic}");

        Console.WriteLine(log.ToString());
        File.WriteAllText(ProbePath("series-internals.txt"), log.ToString());
    }

    [Fact]
    public void Probe_TimingSweep_Cos_ByPrecision()
    {
        var log = new StringBuilder();
        foreach (long p in new long[] { 20, 80, 200, 1000 })
        {
            using var scope = Real.WithPrecision(p, 40);
            Real arg = new Real("0.5");
            var (ms, value) = Time(() => Real.Cos(arg));
            log.AppendLine($"Cos(0.5) at {p,5} places: {ms,10:F1} ms   (exp={value.Exponent}, digits={value.ToNatural().ToString().Length})");
        }
        Console.WriteLine(log.ToString());
        File.WriteAllText(ProbePath("timing-sweep.txt"), log.ToString());
    }

    /// <summary>
    /// One precision per process (set <c>LOVELACE_PROBE_PRECISION</c>), timing the acceptance
    /// question: the first <c>Real.Cos(1/2)</c> of a fresh process at that precision, then the same
    /// call with the π cache warm, so the cold π cost is the difference.  Run once per precision
    /// before and after the fix.
    /// </summary>
    [Fact]
    public void Probe_ColdAndWarmSingleCall_AtEnvPrecision()
    {
        long p = long.Parse(Environment.GetEnvironmentVariable("LOVELACE_PROBE_PRECISION") ?? "1000");
        string tag = Environment.GetEnvironmentVariable("LOVELACE_PROBE_TAG") ?? "run";
        var log = new StringBuilder();

        using (Real.WithPrecision(p, 40))
        {
            var (coldMs, coldValue) = Time(() => Real.Cos(new Real("0.5")));
            var (warmMs, warmValue) = Time(() => Real.Cos(new Real("0.5")));
            var (piMs, _) = Time(() => Real.Pi);

            log.AppendLine($"[{tag}] precision {p}: first Cos(0.5) {coldMs,10:F1} ms (includes π)");
            log.AppendLine($"[{tag}] precision {p}: second Cos(0.5) {warmMs,10:F1} ms (π cached)");
            log.AppendLine($"[{tag}] precision {p}: warm Real.Pi {piMs,10:F1} ms");
            log.AppendLine($"[{tag}] precision {p}: inferred cold π {coldMs - warmMs,10:F1} ms");
            log.AppendLine($"[{tag}] precision {p}: stored exp={warmValue.Exponent} digits={warmValue.ToNatural().ToString().Length} periodic={warmValue.IsPeriodic}");
            log.AppendLine($"[{tag}] precision {p}: cold and warm results equal: {coldValue.Equals(warmValue)}");
        }

        Console.WriteLine(log.ToString());
        File.AppendAllText(ProbePath($"single-call-{tag}.txt"), log.ToString());
    }

    private static string ProbePath(string name)
    {
        string? dir = Environment.GetEnvironmentVariable("LOVELACE_PROBE_DIR");
        if (string.IsNullOrEmpty(dir)) dir = AppContext.BaseDirectory;
        Directory.CreateDirectory(dir);
        return Path.Combine(dir, name);
    }
}
