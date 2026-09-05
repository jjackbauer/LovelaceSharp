namespace Lovelace.Knowledge;

/// <summary>
/// Computes the C1–C4 convergence metrics and the explicit stopping criterion
/// (white-paper §8.2). All metrics are functions of the sample set (P5).
/// </summary>
public static class Convergence
{
    public static Metrics Measure(
        Reduction reduction,
        IReadOnlyList<SampleRecord> samples,
        KnowledgeConfig config,
        IReadOnlyList<int> newPlanesPerBatch,
        IReadOnlyList<int> batchSizes)
    {
        // ---- C1: plane saturation (new-plane rate over the last K batches) ----
        int k = config.C1WindowBatches;
        int take = Math.Max(0, newPlanesPerBatch.Count - k);
        int newLastK = newPlanesPerBatch.Skip(take).Sum();
        int samplesLastK = batchSizes.Skip(Math.Max(0, batchSizes.Count - k)).Sum();
        double rate = samplesLastK == 0 ? 0.0 : (double)newLastK / samplesLastK;
        bool c1 = rate <= config.C1NewPlaneRateThreshold;

        // ---- C2: boundary localization & stability (no unresolved boundaries; all localized) ----
        bool noUnresolved = !reduction.Frontiers.Any(f => f.Kind == FrontierKinds.UnresolvedBoundary);
        int totalB = reduction.Boundaries.Count;
        int stableB = reduction.Boundaries.Count(b => b.Confidence >= Confidence.Bounded);
        bool c2 = noUnresolved && stableB == totalB;

        // ---- C3: prediction agreement on held-out near-boundary probes ----
        (int heldOut, int agreed) = MeasureC3(reduction, samples);
        double agreement = heldOut == 0 ? 0.0 : (double)agreed / heldOut;
        bool c3 = heldOut > 0 && agreement >= config.C3AgreementThreshold;

        // ---- C4: coverage (boundary-adjacent planes have min support) ----
        var support = reduction.Planes.ToDictionary(p => p.Sigma, p => p.Support);
        var boundaryPlanes = reduction.Boundaries
            .SelectMany(b => new[] { b.FromPlane, b.ToPlane })
            .Distinct()
            .ToList();
        bool c4 = boundaryPlanes.All(s => support.TryGetValue(s, out var sup) && sup >= config.C4MinSupportPerPlane);

        var coverage = Coverage(samples, config);

        bool converged = c1 && c2 && c3 && c4;
        string? stopReason = null;
        if (converged) stopReason = "C1-C4 thresholds met";
        else
        {
            var unmet = new List<string>();
            if (!c1) unmet.Add("C1 plane saturation");
            if (!c2) unmet.Add("C2 boundary stability");
            if (!c3) unmet.Add("C3 prediction agreement");
            if (!c4) unmet.Add("C4 coverage");
            stopReason = "not converged: " + string.Join(", ", unmet);
        }

        return new Metrics(
            converged, stopReason, samples.Count, reduction.Planes.Count,
            newLastK, rate, c1,
            totalB, stableB, c2,
            heldOut, agreed, agreement, c3,
            c4, coverage);
    }

    private static (int HeldOut, int Agreed) MeasureC3(Reduction reduction, IReadOnlyList<SampleRecord> samples)
    {
        var validate = samples.Where(s => s.SamplingKind == SamplingKind.Validate).ToList();
        if (validate.Count == 0) return (0, 0);

        // Map each axis to its single non-composite guard boundary.
        var byAxis = new Dictionary<string, BoundaryEdge>(StringComparer.Ordinal);
        foreach (var b in reduction.Boundaries)
        {
            if (b.Guard.Kind == "composite") continue;
            int idx = b.Evidence.Count > 0 ? b.Evidence[0].SampleIndex : -1;
            if (idx < 0 || idx >= samples.Count) continue;
            var anchor = samples[idx].Left.Literal;
            byAxis[b.Operation + "|" + anchor + "|" + b.SweptSide] = b;
        }

        int heldOut = 0, agreed = 0;
        foreach (var v in validate)
        {
            var key = v.Op + "|" + v.Left.Literal + "|" + (v.SweptSide ?? "right");
            if (!byAxis.TryGetValue(key, out var b)) continue;
            heldOut++;
            if (Predicts(b, v)) agreed++;
        }
        return (heldOut, agreed);
    }

    private static bool Predicts(BoundaryEdge b, SampleRecord v)
    {
        var p = ExactNumber.Parse(v.AxisPos!);
        if (b.Guard.Kind == "equality")
        {
            // equality guard: error iff position == threshold (0)
            bool isError = v.Sigma.StartsWith("err|", StringComparison.Ordinal);
            return isError == (p.IsZero);
        }

        // threshold guard: low plane for p <= T, high plane for p > T
        var threshold = ExactNumber.Parse(b.Guard.Threshold);
        string predicted = p > threshold ? b.ToPlane : b.FromPlane;
        return v.Sigma == predicted;
    }

    private static List<CoverageCell> Coverage(IReadOnlyList<SampleRecord> samples, KnowledgeConfig config)
    {
        var domains = new[] { NumberDomain.Natural, NumberDomain.Integer, NumberDomain.Real };
        var cells = new Dictionary<string, CoverageCell>(StringComparer.Ordinal);
        foreach (var op in config.Operations)
        {
            foreach (var ld in domains)
            {
                foreach (var rd in domains)
                {
                    var key = op + "|" + ld + "|" + rd;
                    cells[key] = new CoverageCell(key, op.ToString(), ld.ToString(), rd.ToString(), 0, 0.0);
                }
            }
        }

        foreach (var s in samples)
        {
            var key = s.Op + "|" + s.Left.Domain + "|" + s.Right.Domain;
            if (cells.TryGetValue(key, out var cell))
                cells[key] = cell with { Samples = cell.Samples + 1, Weight = cell.Weight + s.Weight };
        }

        return cells.Values
            .OrderBy(c => c.Operation, StringComparer.Ordinal)
            .ThenBy(c => c.LeftDomain, StringComparer.Ordinal)
            .ThenBy(c => c.RightDomain, StringComparer.Ordinal)
            .ToList();
    }
}
