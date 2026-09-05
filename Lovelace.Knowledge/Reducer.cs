using System.Text;

namespace Lovelace.Knowledge;

/// <summary>The result of reducing a sample set into graph structure.</summary>
public sealed record Reduction(
    List<Plane> Planes,
    List<BoundaryEdge> Boundaries,
    List<Frontier> Frontiers);

/// <summary>
/// Deterministic reducer: clusters observations into behavior planes, detects
/// boundaries by finite differences along sweep axes, fits guards, and marks
/// frontiers (P5). It never executes samples — bisection is driven by the loop.
/// </summary>
public static class Reducer
{
    public static Reduction Reduce(IReadOnlyList<SampleRecord> samples, KnowledgeConfig config)
    {
        var planes = ClusterPlanes(samples);
        var (boundaries, boundaryFrontiers) = DetectBoundaries(samples, config);
        var frontiers = boundaryFrontiers
            .Concat(LowSupportFrontiers(planes, config))
            .Concat(WeakDimensionFrontiers(samples, config))
            .ToList();
        return new Reduction(planes, boundaries, frontiers);
    }

    // ---------------------------------------------------------------------
    // 1. Plane clustering
    // ---------------------------------------------------------------------

    private static List<Plane> ClusterPlanes(IReadOnlyList<SampleRecord> samples)
    {
        var groups = new SortedDictionary<string, List<SampleRecord>>(StringComparer.Ordinal);
        foreach (var s in samples)
        {
            if (!groups.TryGetValue(s.Sigma, out var list))
                groups[s.Sigma] = list = new List<SampleRecord>();
            list.Add(s);
        }

        var planes = new List<Plane>(groups.Count);
        foreach (var (sigma, list) in groups)
        {
            var first = list[0];
            var confidence = list.Count >= 2 ? Confidence.Repeated : Confidence.Observed;
            planes.Add(new Plane(
                sigma,
                first.Kind,
                first.ErrorMessage,
                list.Count,
                confidence,
                list.Select(s => s.Index).OrderBy(i => i).ToList()));
        }
        return planes;
    }

    // ---------------------------------------------------------------------
    // 2. Boundary detection (finite differences) + guard fitting
    // ---------------------------------------------------------------------

    private static (List<BoundaryEdge>, List<Frontier>) DetectBoundaries(
        IReadOnlyList<SampleRecord> samples, KnowledgeConfig config)
    {
        // Fitting samples: sweep + refine (held-out Validate samples are excluded).
        var fitting = samples
            .Where(s => s.SweepId is not null && s.AxisPos is not null && s.SamplingKind != SamplingKind.Validate)
            .ToList();

        var byAxis = new SortedDictionary<string, List<SampleRecord>>(StringComparer.Ordinal);
        foreach (var s in fitting)
        {
            var axisKey = AxisKey(s);
            if (!byAxis.TryGetValue(axisKey, out var list))
                byAxis[axisKey] = list = new List<SampleRecord>();
            list.Add(s);
        }

        // Accumulate localized + pending boundaries.
        var candidates = new Dictionary<string, Candidate>(StringComparer.Ordinal);
        var frontiers = new List<Frontier>();

        foreach (var (axisKey, axisSamples) in byAxis)
        {
            var ordered = axisSamples
                .OrderBy(s => ExactNumber.Parse(s.AxisPos!))
                .ThenBy(s => s.Index)
                .ToList();
            if (ordered.Count < 2) continue;

            var anchor = ordered[0].Left.Literal;
            var sweptSide = ordered[0].SweptSide ?? "right";
            var op = ordered[0].Op;

            for (int i = 0; i + 1 < ordered.Count; i++)
            {
                var a = ordered[i];
                var b = ordered[i + 1];
                if (a.Sigma == b.Sigma) continue;

                var pa = ExactNumber.Parse(a.AxisPos!);
                var pb = ExactNumber.Parse(b.AxisPos!);
                bool localized = pa.IsInteger && pb.IsInteger && (pb.Num - pa.Num) == BigIntegerOne;

                if (!localized)
                {
                    frontiers.Add(new Frontier(
                        StableId("frontier:" + axisKey + ":" + a.Sigma + "->" + b.Sigma),
                        FrontierKinds.UnresolvedBoundary,
                        "boundary between " + a.Sigma + " and " + b.Sigma + " needs bisection",
                        op.ToString(),
                        sweptSide,
                        anchor,
                        a.AxisPos,
                        b.AxisPos));
                    continue;
                }

                var key = axisKey + "|" + a.Sigma + "|" + b.Sigma;
                if (!candidates.TryGetValue(key, out var cand))
                    candidates[key] = cand = new Candidate(op, sweptSide, anchor, a.Sigma, b.Sigma, ordered);
                cand.AddEvidence(a, b);
            }
        }

        // Build boundary edges from localized candidates.
        var boundaries = new List<BoundaryEdge>(candidates.Count);
        foreach (var (key, cand) in candidates.OrderBy(kv => kv.Key, StringComparer.Ordinal))
        {
            var guard = FitGuard(cand.Op, cand.SweptSide, cand.Anchor, cand.PlaneLo, cand.PlaneHi, cand.AxisOrdered);
            int confirmations = cand.StepSizes.Count;
            var confidence = confirmations >= config.C2MinConfirmations
                ? Confidence.Conformant
                : Confidence.Bounded;

            var evidence = cand.Evidence
                .OrderBy(e => e.Side, StringComparer.Ordinal)
                .ThenBy(e => e.SampleIndex)
                .ToList();

            boundaries.Add(new BoundaryEdge(
                StableId("boundary:" + key),
                cand.PlaneLo,
                cand.PlaneHi,
                cand.Op.ToString(),
                cand.SweptSide,
                guard,
                confidence,
                evidence));
        }

        return (boundaries, frontiers);
    }

    private static readonly System.Numerics.BigInteger BigIntegerOne = System.Numerics.BigInteger.One;

    private static string AxisKey(SampleRecord s) =>
        s.Op + "|" + s.Left.Literal + "|" + (s.SweptSide ?? "right");

    // ---------------------------------------------------------------------
    // Guard fitting
    // ---------------------------------------------------------------------

    private static Guard FitGuard(
        Operation op, string sweptSide, string anchor, string planeLo, string planeHi,
        List<SampleRecord> axisOrdered)
    {
        var lowPositions = axisOrdered.Where(s => s.Sigma == planeLo)
            .Select(s => ExactNumber.Parse(s.AxisPos!)).OrderBy(x => x).ToList();
        var highPositions = axisOrdered.Where(s => s.Sigma == planeHi)
            .Select(s => ExactNumber.Parse(s.AxisPos!)).OrderBy(x => x).ToList();
        var pLo = lowPositions[^1];
        var pHi = highPositions[0];

        // Equality guard: an error plane at position 0 for division/modulo.
        if (planeLo.StartsWith("err|", StringComparison.Ordinal) && pLo.IsZero
            && op is Operation.Divide or Operation.Modulo)
        {
            return new Guard(sweptSide, "==", "0", sweptSide + " == 0", "equality");
        }

        // Threshold guard: the two planes are uniform on each side of pLo/pHi.
        bool uniformLow = axisOrdered.All(s =>
        {
            var p = ExactNumber.Parse(s.AxisPos!);
            return p > pLo || s.Sigma == planeLo;
        });
        bool uniformHigh = axisOrdered.All(s =>
        {
            var p = ExactNumber.Parse(s.AxisPos!);
            return p < pHi || s.Sigma == planeHi;
        });

        if (uniformLow && uniformHigh)
        {
            var anchorEx = ExactNumber.Parse(anchor);
            var anchorSide = sweptSide == "right" ? "left" : "right";
            string expr = pLo == anchorEx
                ? sweptSide + " > " + anchorSide
                : sweptSide + " > " + pLo.ToLovelaceLiteral();
            return new Guard(sweptSide, ">", pLo.ToLovelaceLiteral(), expr, "threshold");
        }

        return new Guard(sweptSide, "?", "", "non-uniform boundary", "composite");
    }

    // ---------------------------------------------------------------------
    // Frontiers
    // ---------------------------------------------------------------------

    private static IEnumerable<Frontier> LowSupportFrontiers(List<Plane> planes, KnowledgeConfig config)
    {
        foreach (var p in planes)
        {
            if (p.Support < config.C4MinSupportPerPlane)
            {
                yield return new Frontier(
                    StableId("frontier:lowsupport:" + p.Sigma),
                    FrontierKinds.LowSupport,
                    "plane " + p.Sigma + " has support " + p.Support + " (< " + config.C4MinSupportPerPlane + ")",
                    null, null, null, null, null);
            }
        }
    }

    private static IEnumerable<Frontier> WeakDimensionFrontiers(
        IReadOnlyList<SampleRecord> samples, KnowledgeConfig config)
    {
        var arithmetic = new HashSet<Operation>
        {
            Operation.Add, Operation.Subtract, Operation.Multiply,
            Operation.Divide, Operation.Modulo, Operation.Power,
        };
        var domains = new[] { NumberDomain.Natural, NumberDomain.Integer, NumberDomain.Real };
        var sampledCells = new HashSet<string>(StringComparer.Ordinal);
        foreach (var s in samples)
        {
            if (!arithmetic.Contains(s.Op)) continue;
            sampledCells.Add(s.Op + "|" + s.Left.Domain + "|" + s.Right.Domain);
        }

        var frontierList = new List<Frontier>();
        foreach (var op in arithmetic.OrderBy(o => o.ToString(), StringComparer.Ordinal))
        {
            foreach (var ld in domains)
            {
                foreach (var rd in domains)
                {
                    var key = op + "|" + ld + "|" + rd;
                    if (sampledCells.Contains(key)) continue;
                    frontierList.Add(new Frontier(
                        StableId("frontier:weak:" + key),
                        FrontierKinds.WeakDimension,
                        "unsampled cell " + OperationNames.ToSymbol(op) + " over " + ld + " x " + rd,
                        op.ToString(), null, null, null, null));
                }
            }
        }
        return frontierList;
    }

    // ---------------------------------------------------------------------
    // Deterministic stable id (FNV-1a 64-bit)
    // ---------------------------------------------------------------------

    public static string StableId(string input)
    {
        ulong hash = 14695981039346656037UL;
        foreach (var b in Encoding.UTF8.GetBytes(input))
        {
            hash ^= b;
            hash *= 1099511628211UL;
        }
        return hash.ToString("x16");
    }

    private sealed class Candidate
    {
        public Operation Op { get; }
        public string SweptSide { get; }
        public string Anchor { get; }
        public string PlaneLo { get; }
        public string PlaneHi { get; }
        public List<SampleRecord> AxisOrdered { get; }
        public List<BoundEvidence> Evidence { get; } = new();
        public HashSet<string> StepSizes { get; } = new(StringComparer.Ordinal);

        public Candidate(Operation op, string sweptSide, string anchor, string planeLo, string planeHi, List<SampleRecord> axisOrdered)
        {
            Op = op;
            SweptSide = sweptSide;
            Anchor = anchor;
            PlaneLo = planeLo;
            PlaneHi = planeHi;
            AxisOrdered = axisOrdered;
        }

        public void AddEvidence(SampleRecord low, SampleRecord high)
        {
            Evidence.Add(new BoundEvidence("low", low.Index, low.AxisPos!, low.Sigma));
            Evidence.Add(new BoundEvidence("high", high.Index, high.AxisPos!, high.Sigma));
            if (StepOf(low.SweepId) is { } ls) StepSizes.Add(ls);
            if (StepOf(high.SweepId) is { } hs) StepSizes.Add(hs);
        }

        /// <summary>Extracts the coarse step-size tag (":h2", ":h3") or null.</summary>
        private static string? StepOf(string? sweepId)
        {
            if (sweepId is null) return null;
            int idx = sweepId.LastIndexOf(":h", StringComparison.Ordinal);
            if (idx < 0) return null;
            string tail = sweepId[(idx + 2)..];
            if (tail.Length == 0 || !tail.All(char.IsAsciiDigit)) return null;
            return sweepId[idx..];
        }
    }
}
