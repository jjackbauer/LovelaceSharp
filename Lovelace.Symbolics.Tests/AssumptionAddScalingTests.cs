using System.Collections.Immutable;
using System.Diagnostics;
using Lovelace.Symbolics;
using Xunit;
using Xunit.Abstractions;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// P2 item 9: <see cref="AssumptionSet.Add"/> must not re-render or re-sort the whole collection per
/// insertion. Atoms are added one at a time into a FRESH <see cref="ExprContext"/> per repetition, at
/// 100/200/400/800 atoms; the fitted exponent of elapsed time against size is reported. The order
/// tests pin the canonical ordering the 324-query bound matrix and the 324-pair contradiction matrix
/// depend on (ordinal by the atom's own rendering).
///
/// MEASURED BOUND (Release, min of 3 repetitions, one atom at a time):
///   before the round-25 fix - 100/200/400/800 atoms: domain 5.19/32.85/156.13/424.00 ms,
///     relation 5.62/35.69/116.04/512.35 ms; fitted exponent 2.13 / 2.12 (quadratic: every insertion
///     re-sorted the whole array and re-rendered both operands of every comparison).
///   after the fix - 100/200/400/800 atoms: domain 0.18/0.59/2.10/8.06 ms, relation
///     0.70/2.02/7.89/33.17 ms; fitted exponent 1.83 / 1.87. The residual growth is the O(n)
///     immutable copy and the O(n) containment scan per insertion, not rendering or sorting.
/// The ceiling asserted below (250 ms for 800 atoms) is ~7x the worst measured post-fix value and
/// still fails the pre-fix behaviour (424 ms / 512 ms), so it is a tripwire, not a benchmark.
/// </summary>
public class AssumptionAddScalingTests
{
    private readonly ITestOutputHelper _out;

    public AssumptionAddScalingTests(ITestOutputHelper output) => _out = output;

    private static readonly int[] Sizes = { 100, 200, 400, 800 };

    private const int Reps = 3;

    /// <summary>Ceiling for 800 atoms; see the class comment for the measured values behind it.</summary>
    private const double EightHundredAtomCeilingMs = 250.0;

    /// <summary>Workload A: one domain atom per distinct symbol.</summary>
    private static Assumption[] DomainAtoms(ExprContext ctx, int n)
    {
        var atoms = new Assumption[n];
        for (int i = 0; i < n; i++)
            atoms[i] = new SymbolDomainAssumption(ctx.Symbol($"x{i}"), Domain.Real);
        return atoms;
    }

    /// <summary>Workload B: one relation atom per distinct symbol (exercises the contradiction probe too).</summary>
    private static Assumption[] RelationAtoms(ExprContext ctx, int n)
    {
        var atoms = new Assumption[n];
        for (int i = 0; i < n; i++)
            atoms[i] = new SymbolRelationAssumption(ctx.Symbol($"x{i}"), RelOp.Gt, Exprs.Rational(Rat.FromLong(i + 1)));
        return atoms;
    }

    private static (double Ms, AssumptionSet Set) Measure(int n, Func<ExprContext, int, Assumption[]> make)
    {
        double best = double.PositiveInfinity;
        AssumptionSet? result = null;
        for (int r = 0; r < Reps; r++)
        {
            var ctx = new ExprContext();
            var atoms = make(ctx, n);              // symbol/expression construction is NOT timed
            GC.Collect();
            GC.WaitForPendingFinalizers();
            GC.Collect();
            var sw = Stopwatch.StartNew();
            var set = AssumptionSet.Empty;
            for (int i = 0; i < atoms.Length; i++)
                set = set.Add(atoms[i]);           // one atom at a time
            sw.Stop();
            if (sw.Elapsed.TotalMilliseconds < best)
            {
                best = sw.Elapsed.TotalMilliseconds;
                result = set;
            }
        }
        return (best, result!);
    }

    /// <summary>Least-squares slope of ln(ms) against ln(n): 1.0 is linear, 2.0 is quadratic.</summary>
    private static double FitExponent(IReadOnlyList<(int N, double Ms)> pts)
    {
        double sx = 0, sy = 0, sxx = 0, sxy = 0;
        foreach (var (n, ms) in pts)
        {
            double x = Math.Log(n), y = Math.Log(ms);
            sx += x; sy += y; sxx += x * x; sxy += x * y;
        }
        int k = pts.Count;
        return (k * sxy - sx * sy) / (k * sxx - sx * sx);
    }

    [Fact]
    public void Add_Scaling_800AtomsStaysUnder250ms_AndKeepsEveryAtomInOrder()
    {
        var lines = new List<string> { "| workload | atoms | elapsed ms (min of " + Reps + ") | stored atoms |", "|---|---|---|---|" };
        foreach (var (name, make) in new (string, Func<ExprContext, int, Assumption[]>)[]
                 { ("domain atoms", DomainAtoms), ("relation atoms", RelationAtoms) })
        {
            var pts = new List<(int N, double Ms)>();
            foreach (var n in Sizes)
            {
                var (ms, set) = Measure(n, make);
                pts.Add((n, ms));
                lines.Add($"| {name} | {n} | {ms:F2} | {set.Atoms.Length} |");

                // structural part of the pin, independent of machine speed: nothing is dropped and
                // the stored order is still ordinal by the atom's rendering
                Assert.Equal(n, set.Atoms.Length);
                var keys = set.Atoms.Select(a => a.ToString()).ToArray();
                for (int i = 1; i < keys.Length; i++)
                    Assert.True(string.CompareOrdinal(keys[i - 1], keys[i]) <= 0,
                        $"{name}: order broken at {i} with {n} atoms");

                if (n == Sizes[^1])
                    Assert.True(ms < EightHundredAtomCeilingMs,
                        $"{name}: adding 800 atoms took {ms:F2} ms, over the {EightHundredAtomCeilingMs} ms " +
                        "ceiling; the pre-fix full re-sort per insertion measured 424.00 ms (domain) and " +
                        "512.35 ms (relation) at this size.");
            }
            lines.Add($"fitted exponent ({name}) = {FitExponent(pts):F3}");
        }
        foreach (var line in lines)
            _out.WriteLine(line);
    }

    [Fact]
    public void Add_MixedAtomsInAnyInsertionOrder_YieldsTheSameCanonicalOrder()
    {
        var forward = Build(Mixed());
        var backward = Build(Mixed().Reverse().ToArray());
        var expected = Mixed().Select(a => a.ToString()).OrderBy(s => s, StringComparer.Ordinal).ToArray();

        Assert.Equal(expected, forward.Atoms.Select(a => a.ToString()));
        Assert.Equal(expected, backward.Atoms.Select(a => a.ToString()));
    }

    /// <summary>The pre-fix algorithm, replicated: append the atom and ordinal-re-sort the whole
    /// array on every insertion. The incremental set must store the SAME sequence, in every
    /// insertion order, or the ordering the matrices depend on has changed.</summary>
    [Fact]
    public void Add_OrderEqualsHistoricalAppendAndResort_ForEveryInsertionOrder()
    {
        var pool = Mixed();
        var rng = new Random(20250825);
        for (int trial = 0; trial < 50; trial++)
        {
            var order = pool.OrderBy(_ => rng.Next()).ToArray();

            var legacy = ImmutableArray.CreateBuilder<Assumption>();
            foreach (var a in order)
            {
                if (legacy.Contains(a))
                    continue;
                legacy.Add(a);
                legacy.Sort(static (x, y) => string.CompareOrdinal(x.ToString(), y.ToString()));
            }

            var incremental = Build(order);
            Assert.Equal(
                legacy.Select(a => a.ToString()).ToArray(),
                incremental.Atoms.Select(a => a.ToString()).ToArray());
        }
    }

    private static AssumptionSet Build(IReadOnlyList<Assumption> atoms)
    {
        var set = AssumptionSet.Empty;
        foreach (var a in atoms)
            set = set.Add(a);
        return set;
    }

    /// <summary>Distinct non-contradicting atoms of every shape, deliberately with names that do not
    /// sort in insertion order so an insertion never lands at the end.</summary>
    private static Assumption[] Mixed()
    {
        var ctx = new ExprContext();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var z = ctx.Symbol("z");
        return new Assumption[]
        {
            new SymbolDomainAssumption(z, Domain.Real),
            new SymbolPropertyAssumption(y, SymbolPredicate.Positive),
            new SymbolRelationAssumption(z, RelOp.Gt, Exprs.Rational(Rat.FromLong(2))),
            new SymbolDomainAssumption(x, Domain.Integer),
            new SymbolPropertyAssumption(x, SymbolPredicate.Even),
            new SymbolRelationAssumption(x, RelOp.Ge, Exprs.Rational(Rat.FromLong(3))),
            new ExpressionPropertyAssumption(Exprs.Symbol(y), SymbolPredicate.NonZero),
            new IntervalAssumption(Exprs.Symbol(z), Exprs.Rational(Rat.Zero), false, Exprs.Rational(Rat.FromLong(9)), true),
        };
    }

    [Fact]
    public void Add_SameAtomTwiceFromDifferentContexts_IsStoredOnce()
    {
        // Same symbol name from two contexts: value equality (not reference) decides, so the second
        // atom is dropped and the set keeps every distinct atom exactly once, in ordinal order.
        var ctxA = new ExprContext();
        var ctxB = new ExprContext();
        var atoms = new Assumption[]
        {
            new SymbolDomainAssumption(ctxA.Symbol("t"), Domain.Real),
            new SymbolDomainAssumption(ctxB.Symbol("t"), Domain.Real),
            new SymbolDomainAssumption(ctxA.Symbol("t"), Domain.Real),
            new SymbolPropertyAssumption(ctxA.Symbol("a"), SymbolPredicate.Positive),
        };
        var set = Build(atoms);
        var keys = set.Atoms.Select(a => a.ToString()).ToArray();
        _out.WriteLine("keys: " + string.Join(" | ", keys));
        var distinct = atoms.Distinct().ToArray();
        Assert.Equal(distinct.Length, set.Atoms.Length);
        for (int i = 1; i < keys.Length; i++)
            Assert.True(string.CompareOrdinal(keys[i - 1], keys[i]) <= 0, $"order broken at {i}: {string.Join(" | ", keys)}");
        Assert.Equal(
            distinct.Select(a => a.ToString()).OrderBy(s => s, StringComparer.Ordinal),
            keys.OrderBy(s => s, StringComparer.Ordinal));
        foreach (var a in distinct)
            Assert.Contains(a, set.Atoms);
    }
}
