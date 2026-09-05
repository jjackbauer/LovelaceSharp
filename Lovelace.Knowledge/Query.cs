using System.Text;

namespace Lovelace.Knowledge;

/// <summary>Read-only graph traversal / reporting (white-paper §10).</summary>
public static class Query
{
    public static string Summary(Graph graph)
    {
        var sb = new StringBuilder();
        sb.Append("planes=").Append(graph.Planes.Count)
          .Append(", boundaries=").Append(graph.Boundaries.Count)
          .Append(", frontiers=").Append(graph.Frontiers.Count)
          .Append(", samples=").Append(graph.Samples.Count);
        if (graph.Metrics is { } m)
        {
            sb.Append(", converged=").Append(m.Converged)
              .Append(", C1_newPlanes_lastK=").Append(m.C1NewPlanesLastK)
              .Append(" (rate ").Append(m.C1NewPlaneRate.ToString("0.###")).Append(')')
              .Append(", C2_stable=").Append(m.C2StableBoundaries).Append('/').Append(m.C2TotalBoundaries)
              .Append(", C3_agreement=").Append(m.C3Agreement.ToString("0.###"))
              .Append(", C4_covered=").Append(m.C4Covered);
        }
        return sb.ToString();
    }

    /// <summary>Distinct typed values observed within a plane (e.g. periodic vs terminating reals).</summary>
    public static List<string> RepresentativeValues(Plane plane, Graph graph, int max = 8)
    {
        var values = plane.SampleIndices
            .Select(i => graph.Samples[i])
            .Where(s => s.Typed is not null)
            .Select(s => s.Typed!)
            .Distinct(StringComparer.Ordinal)
            .OrderBy(x => x, StringComparer.Ordinal)
            .ToList();
        var head = values.Take(max).ToList();
        if (values.Count > max) head.Add("… (+" + (values.Count - max) + " more)");
        return head;
    }

    public static bool IsPeriodic(string typedValue) =>
        typedValue.Contains('(') && typedValue.Contains(')');
}
