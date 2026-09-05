namespace Lovelace.Knowledge;

/// <summary>A sample coordinate to execute (before execution).</summary>
public sealed record SampleSpec(
    string Script,
    Operation Op,
    Operand Left,
    Operand Right,
    string? SweepId,
    string? SweptSide,
    string? AxisPos,
    SamplingKind SamplingKind,
    double Weight);

/// <summary>
/// The explicit proposal distribution q(z): a mixture of deterministic 1-D sweeps
/// (boundary discovery + two step sizes for C2), random Monte Carlo draws (breadth),
/// and frontier-guided refinement (bisection) + held-out validation probes (C3).
/// All sampling is a deterministic function of the seed and the current graph.
/// </summary>
public sealed class Proposal
{
    private readonly List<SampleSpec> _sweeps = new();
    private readonly SplitMix64 _rng;
    private readonly HashSet<string> _validatedBoundaries = new(StringComparer.Ordinal);
    private int _sweepCursor;
    private int _randomDrawn;

    public Proposal(long seed, KnowledgeConfig config)
    {
        _rng = SplitMix64.FromLong(seed);
        BuildSweeps(config);
    }

    public int SweepCount => _sweeps.Count;

    // ---------------------------------------------------------------------
    // Next batch (deterministic phase scheduler)
    // ---------------------------------------------------------------------

    public List<SampleSpec> NextBatch(Graph? graph, KnowledgeConfig config)
    {
        var batch = new List<SampleSpec>();

        // Phase 1 — deterministic sweeps (discover planes + candidate boundaries).
        while (batch.Count < config.BatchSize && _sweepCursor < _sweeps.Count)
            Add(batch, _sweeps[_sweepCursor++]);

        // Phase 2 — random Monte Carlo breadth.
        while (batch.Count < config.BatchSize && _randomDrawn < config.MinRandomSamples)
        {
            Add(batch, RandomSpec(config));
            _randomDrawn++;
        }

        // Phases 3–4 only after breadth is exhausted: bias toward frontiers.
        bool breadthDone = _sweepCursor >= _sweeps.Count && _randomDrawn >= config.MinRandomSamples;
        if (breadthDone && graph is not null)
        {
            // Phase 3 — bisection refinement of unresolved boundary intervals.
            foreach (var f in graph.Frontiers)
            {
                if (batch.Count >= config.BatchSize) break;
                if (f.Kind != FrontierKinds.UnresolvedBoundary) continue;
                if (f.Operation is null || f.Anchor is null || f.Low is null || f.High is null) continue;
                foreach (var m in Midpoints(f.Low, f.High))
                {
                    if (batch.Count >= config.BatchSize) break;
                    Add(batch, MakeSpec(Enum.Parse<Operation>(f.Operation), Operand.Natural(f.Anchor), Operand.Natural(m),
                        "right", "refine", SamplingKind.Refine, 1.0));
                }
            }

            // Phase 4 — held-out validation probes adjacent to localized guards (C3).
            foreach (var b in graph.Boundaries)
            {
                if (batch.Count >= config.BatchSize) break;
                if (b.Guard.Kind == "composite") continue; // no clean predicate to test
                if (!_validatedBoundaries.Add(b.Id)) continue; // validate each boundary once
                foreach (var probe in ValidationProbes(b, graph))
                {
                    if (batch.Count >= config.BatchSize) break;
                    Add(batch, probe);
                }
            }
        }

        return batch;
    }

    // ---------------------------------------------------------------------
    // Random draws
    // ---------------------------------------------------------------------

    private SampleSpec RandomSpec(KnowledgeConfig config)
    {
        var op = config.Operations[_rng.NextInt(config.Operations.Count)];
        var left = RandomOperand(config);
        var right = RandomOperand(config);
        double weight = 1.0 / (config.Operations.Count * 3.0 * 3.0
            * ValueCount(left.Domain, config) * ValueCount(right.Domain, config));
        return MakeSpec(op, left, right, null, null, SamplingKind.Random, weight);
    }

    /// <summary>Standalone Monte Carlo draws (used by the 'sample' command).</summary>
    public static List<SampleSpec> RandomSamples(long seed, KnowledgeConfig config, int count)
    {
        var rng = SplitMix64.FromLong(seed);
        var list = new List<SampleSpec>(count);
        for (int i = 0; i < count; i++)
        {
            var op = config.Operations[rng.NextInt(config.Operations.Count)];
            var left = RandomOperandStatic(rng, config);
            var right = RandomOperandStatic(rng, config);
            double weight = 1.0 / (config.Operations.Count * 3.0 * 3.0
                * ValueCount(left.Domain, config) * ValueCount(right.Domain, config));
            list.Add(MakeSpec(op, left, right, null, null, SamplingKind.Random, weight));
        }
        return list;
    }

    private static Operand RandomOperandStatic(SplitMix64 rng, KnowledgeConfig config)
    {
        int d = rng.NextInt(3);
        var domain = d switch { 0 => NumberDomain.Natural, 1 => NumberDomain.Integer, _ => NumberDomain.Real };
        var values = domain switch
        {
            NumberDomain.Natural => config.NaturalValues,
            NumberDomain.Integer => config.IntegerValues,
            _ => config.RealValues,
        };
        return new Operand(domain, values[rng.NextInt(values.Count)]);
    }

    private Operand RandomOperand(KnowledgeConfig config)
    {
        int d = _rng.NextInt(3);
        var domain = d switch { 0 => NumberDomain.Natural, 1 => NumberDomain.Integer, _ => NumberDomain.Real };
        var values = domain switch
        {
            NumberDomain.Natural => config.NaturalValues,
            NumberDomain.Integer => config.IntegerValues,
            _ => config.RealValues,
        };
        return new Operand(domain, values[_rng.NextInt(values.Count)]);
    }

    private static int ValueCount(NumberDomain d, KnowledgeConfig c) => d switch
    {
        NumberDomain.Natural => c.NaturalValues.Count,
        NumberDomain.Integer => c.IntegerValues.Count,
        _ => c.RealValues.Count,
    };

    // ---------------------------------------------------------------------
    // Deterministic sweeps
    // ---------------------------------------------------------------------

    private void BuildSweeps(KnowledgeConfig config)
    {
        var naturals = SortedLiterals(config.NaturalValues);
        foreach (var op in config.SweepOperations)
        {
            foreach (var anchor in Anchors(op))
            {
                // Two coarse step sizes (for C2 stability across h). Non-multiple
                // steps (2 and 3) so the grids do not fully overlap.
                foreach (int h in new[] { 2, 3 })
                {
                    for (int i = 0; i < naturals.Count; i += h)
                    {
                        _sweeps.Add(MakeSpec(op, Operand.Natural(anchor), Operand.Natural(naturals[i]),
                            "right", SweepId(op, anchor, h), SamplingKind.Sweep, 1.0));
                    }
                }
            }
        }
    }

    private static IReadOnlyList<string> Anchors(Operation op) => op switch
    {
        Operation.Subtract => new[] { "0", "1", "2", "5", "10" },
        Operation.Divide => new[] { "1", "2", "5", "10" },
        Operation.Modulo => new[] { "5", "10" },
        Operation.Greater or Operation.Less => new[] { "5" },
        _ => Array.Empty<string>(),
    };

    private static string SweepId(Operation op, string anchor, int h) =>
        op.ToString().ToLowerInvariant() + ":" + anchor + ":right:h" + h;

    private static List<string> SortedLiterals(IReadOnlyList<string> literals) =>
        literals.Select(ExactNumber.Parse).OrderBy(x => x).Select(x => x.ToLovelaceLiteral()).ToList();

    // ---------------------------------------------------------------------
    // Script construction
    // ---------------------------------------------------------------------

    private static SampleSpec MakeSpec(
        Operation op, Operand left, Operand right, string? sweptSide, string? sweepId,
        SamplingKind kind, double weight)
    {
        var opSymbol = OperationNames.ToSymbol(op);
        var script = left.Literal + " " + opSymbol + " " + right.Literal;
        string? axisPos = sweptSide == "right" ? right.Literal : (sweptSide == "left" ? left.Literal : null);
        return new SampleSpec(script, op, left, right, sweepId, sweptSide, axisPos, kind, weight);
    }

    // ---------------------------------------------------------------------
    // Frontier-guided refinement + held-out probes
    // ---------------------------------------------------------------------

    /// <summary>Integer midpoint(s) strictly between two natural literals.</summary>
    private static List<string> Midpoints(string lowLiteral, string highLiteral)
    {
        var low = ExactNumber.Parse(lowLiteral);
        var high = ExactNumber.Parse(highLiteral);
        var result = new List<string>();
        if (!low.IsInteger || !high.IsInteger || low >= high) return result;

        var diff = high.Num - low.Num;
        var mid = low.Num + diff / 2;
        result.Add(mid.ToString());
        if (diff % 2 != 0) result.Add((mid + 1).ToString());
        // Keep only strictly-interior points.
        return result
            .Select(ExactNumber.Parse)
            .Where(x => x > low && x < high)
            .Distinct()
            .Select(x => x.ToLovelaceLiteral())
            .ToList();
    }

    /// <summary>Held-out probes one step outside each side of a localized boundary.</summary>
    private static List<SampleSpec> ValidationProbes(BoundaryEdge b, Graph graph)
    {
        var result = new List<SampleSpec>();
        var ev = b.Evidence;
        if (ev.Count == 0) return result;

        // Recover the anchor (left operand) from any bounding sample.
        int idx = ev[0].SampleIndex;
        if (idx < 0 || idx >= graph.Samples.Count) return result;
        var anchor = graph.Samples[idx].Left.Literal;

        var low = ev.Where(e => e.Side == "low").Select(e => ExactNumber.Parse(e.AxisPos)).OrderBy(x => x).ToList();
        var high = ev.Where(e => e.Side == "high").Select(e => ExactNumber.Parse(e.AxisPos)).OrderBy(x => x).ToList();
        if (low.Count == 0 || high.Count == 0) return result;

        var pLo = low[^1];
        var pHi = high[0];

        var probes = new List<ExactNumber>();
        var oneBelow = pLo.Subtract(ExactNumber.One);
        var oneAbove = pHi.Add(ExactNumber.One);
        if (!oneBelow.IsNegative) probes.Add(oneBelow);
        probes.Add(pLo);
        probes.Add(pHi);
        probes.Add(oneAbove);

        foreach (var p in probes.Distinct())
        {
            result.Add(new SampleSpec(
                anchor + " " + OperationNames.ToSymbol(Enum.Parse<Operation>(b.Operation)) + " " + p.ToLovelaceLiteral(),
                Enum.Parse<Operation>(b.Operation),
                Operand.Natural(anchor),
                Operand.Natural(p.ToLovelaceLiteral()),
                "validate",
                b.SweptSide,
                p.ToLovelaceLiteral(),
                SamplingKind.Validate,
                1.0));
        }
        return result;
    }

    private static void Add(List<SampleSpec> batch, SampleSpec spec) => batch.Add(spec);
}
