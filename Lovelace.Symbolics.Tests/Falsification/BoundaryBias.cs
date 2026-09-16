using System.Globalization;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests.Falsification;

/// <summary>
/// A deterministic 64-bit generator, seeded explicitly. The falsifier must be reproducible in any
/// process, so it uses a generator whose entire state is one integer and whose stream is fixed by
/// this file - not <see cref="System.Random"/>, whose implementation is not a contract.
/// </summary>
public sealed class DeterministicRng
{
    private ulong _state;

    public DeterministicRng(long seed) => _state = unchecked((ulong)seed);

    /// <summary>The next 64 bits (splitmix64).</summary>
    public ulong NextUInt64()
    {
        unchecked
        {
            _state += 0x9E3779B97F4A7C15UL;
            var z = _state;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
            return z ^ (z >> 31);
        }
    }

    /// <summary>A uniform integer in <c>[0, bound)</c> without modulo bias for the small
    /// bounds this file uses.</summary>
    public int NextInt(int bound)
    {
        if (bound <= 0)
            throw new ArgumentOutOfRangeException(nameof(bound), bound, "bound must be positive.");
        var limit = ulong.MaxValue - (ulong.MaxValue % (ulong)bound);
        ulong draw;
        do
        {
            draw = NextUInt64();
        }
        while (draw >= limit);
        return (int)(draw % (ulong)bound);
    }
}

/// <summary>
/// The deterministic step ladder a boundary search walks: element <c>k</c> is a step SIZE, and the
/// ladder is deliberately a mix of coprime sizes, so two different steps cannot share a grid and
/// the two arms of a comparison cannot accidentally sample the same points. <c>h = 2</c> and
/// <c>h = 3</c> is the idea the engine's own behavioral discovery uses
/// (<c>Lovelace.Knowledge/Proposal.cs</c> <c>BuildSweeps</c>, whose two coarse steps exist
/// precisely so their grids do not fully overlap); this file keeps the idea and its own types.
/// </summary>
public sealed record BiasStepLadder(IReadOnlyList<int> Steps)
{
    /// <summary>The ladder the falsifier uses by default: coarse-to-fine, coprime, odd steps
    /// first so an even grid and an odd grid cannot coincide.
    public static BiasStepLadder Default { get; } = new(new[] { 1, 2, 3, 5, 7 });

    /// <summary>The coarse step: what a uniform scan would have used.</summary>
    public int Coarse => Steps.Count == 0 ? 1 : Steps[0];

    /// <summary>The fine step: what the refinement walks down to.</summary>
    public int Fine => Steps.Count == 0 ? 1 : Steps[^1];

    /// <summary>Whether the ladder carries a step of the given size.</summary>
    public bool Contains(int step) => Steps.Contains(step);
}

/// <summary>Everything a biased search needs, all of it explicit so a run is reproducible from
/// the record alone.</summary>
/// <param name="Seed">The run's seed.</param>
/// <param name="Ladder">The step ladder.</param>
/// <param name="SamplesPerLevel">How many points are offered at each ladder step.</param>
/// <param name="MaxNarrowSteps">The cap on the bisection descent, so the search always terminates.</param>
public sealed record BoundaryBiasSpec(
    long Seed,
    BiasStepLadder Ladder,
    int SamplesPerLevel,
    int MaxNarrowSteps)
{
    /// <summary>The falsifier's default: 3 points per ladder step, at most 64 descents.
    public static BoundaryBiasSpec Default(long seed) =>
        new(seed, BiasStepLadder.Default, 3, 64);
}

/// <summary>
/// A DETERMINISTIC boundary-biased sample generator over an exact-rational line.
/// <para>
/// It is given two samples whose observation classes DIFFER, and produces new probe points strictly
/// between them. Every point is an exact rational: the product evaluates exact rationals exactly,
/// so no floating-point epsilon ever enters a probe. Same seed and same inputs produce a
/// byte-identical sequence, in any process.
/// </para>
/// <para>
/// It assumes nothing about smoothness and uses no derivative to decide anything: it only ever
/// asks the caller's own classifier what class a point is in, and moves the bracket towards the
/// change. There is no ranking heuristic here at all - every point offered is a candidate, and the
/// search is a plain bracket descent over the ladder.
/// </para>
/// </summary>
public sealed class BoundaryBias
{
    private readonly BoundaryBiasSpec _spec;
    private readonly DeterministicRng _rng;

    public BoundaryBias(BoundaryBiasSpec spec)
    {
        _spec = spec;
        _rng = new DeterministicRng(spec.Seed);
    }

    public BoundaryBiasSpec Spec => _spec;

    /// <summary>
    /// The broad phase: points strictly between the two observations, walked down the step ladder.
    /// Each ladder step offers <see cref="BoundaryBiasSpec.SamplesPerLevel"/> points, each on the
    /// step's own grid offset by a seeded jitter, so an even step and an odd step sample different
    /// places in the same interval.
    /// </summary>
    public IEnumerable<Rat> BroadPhase(Rat low, Rat high)
    {
        if (low >= high)
            throw new ArgumentException($"the low end {FalsificationText.Rational(low)} must be strictly below the high end {FalsificationText.Rational(high)}.");

        var steps = _spec.SamplesPerLevel;
        foreach (var step in _spec.Ladder.Steps)
        {
            // the grid divides the interval into step*samples equal parts; a grid finer than the
            // interval cannot place an interior point, so the ladder must not invent one
            if (!InteriorPoint(low, high, step * steps, 1, out _))
                continue;
            var jitter = _rng.NextInt(step);
            for (var k = 0; k < steps; k++)
            {
                // index = k*step + jitter, so an even step and an odd step offset their grids
                // differently and the two arms cannot sample the same places by accident
                var index = (k * step) + jitter;
                if (index == 0)
                    continue;   // the endpoints are the caller's own evidence, never ours
                if (!InteriorPoint(low, high, step * steps, index, out var point))
                    continue;
                yield return point;
            }
        }
    }

    /// <summary>A single point strictly between two rationals, on the grid of
    /// <paramref name="divisions"/> equal parts. Exposed so a test can assert a point really lies
    /// inside its interval. Every part is exact integer arithmetic.</summary>
    public static bool InteriorPoint(Rat low, Rat high, int divisions, int index, out Rat point)
    {
        point = Rat.Zero;
        if (index <= 0 || index >= divisions)
            return false;
        var span = Rat.Subtract(high, low);
        // (index / divisions) of the way from low to high, every part exact
        point = Rat.Add(low, Rat.Divide(Rat.Multiply(span, Rat.From(index, 1)), Rat.From(divisions, 1)));
        return point > low && point < high;
    }

    /// <summary>
    /// The narrow phase: an exact bisection that drives a bracket down to the last representable
    /// gap at the ladder's finest step. The classifier decides which half still contains the
    /// change; nothing here decides anything by itself.
    /// </summary>
    /// <param name="low">A point whose class is the low class.</param>
    /// <param name="high">A point whose class is the high class.</param>
    /// <param name="lowClass">The class at <paramref name="low"/>.</param>
    /// <param name="classify">The caller's classifier. Called once per offered point.</param>
    /// <param name="offered">Every midpoint offered, in order, including the last one.</param>
    public Rat NarrowPhase(
        Rat low,
        Rat high,
        ObservationClass lowClass,
        Func<Rat, ObservationClass> classify,
        out IReadOnlyList<Rat> offered)
    {
        var seen = new List<Rat>();
        var width = high.Numerator * low.Denominator - low.Numerator * high.Denominator;
        var gap = new Lovelace.Integer.Integer(_spec.Ladder.Fine);
        var lo = low;
        var hi = high;

        for (var step = 0; step < _spec.MaxNarrowSteps && width > gap; step++)
        {
            var numerator = hi.Numerator * lo.Denominator + lo.Numerator * hi.Denominator;
            var denominator = hi.Denominator * lo.Denominator * new Lovelace.Integer.Integer(2);
            var mid = Rat.From(numerator / denominator);
            seen.Add(mid);
            if (classify(mid) == lowClass)
                lo = mid;
            else
                hi = mid;
            width = hi.Numerator * lo.Denominator - lo.Numerator * hi.Denominator;
        }

        offered = seen;
        return hi;
    }
}
