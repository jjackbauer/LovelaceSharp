using System.Text.Json;

namespace Lovelace.Knowledge;

/// <summary>The canonical observation sigma of one executed sample.</summary>
public sealed record Observation(
    bool Success,
    string Sigma,
    string? Kind,
    string? Typed,
    string? ErrorMessage);

/// <summary>
/// Canonicalization of the Lovelace.Run JSON envelope into sigma (req 6).
/// Success -> "ok|kind|typed"; failure -> "err|message". Noise (revision,
/// elapsed, plot SVG, variables, functions) is discarded.
/// </summary>
public static class CanonicalObservation
{
    /// <summary>Builds sigma from the raw JSON stdout of Lovelace.Run.</summary>
    public static Observation FromRunnerOutput(string stdout)
    {
        using var doc = JsonDocument.Parse(stdout);
        var root = doc.RootElement;

        bool ok = root.TryGetProperty("ok", out var okProp) && okProp.ValueKind == JsonValueKind.True;
        if (ok)
        {
            if (root.TryGetProperty("result", out var res) && res.ValueKind == JsonValueKind.Object)
            {
                string kind = res.TryGetProperty("kind", out var k) ? (k.GetString() ?? "Unknown") : "Unknown";
                string typed = res.TryGetProperty("typed", out var t) ? (t.GetString() ?? "") : "";
                return new Observation(true, "ok|" + kind + "|" + typed, kind, typed, null);
            }
            return new Observation(true, "ok|Void|", "Void", "", null);
        }

        string message = root.TryGetProperty("message", out var m) ? (m.GetString() ?? "(no message)") : "(no message)";
        return new Observation(false, "err|" + message, null, null, message);
    }

    /// <summary>Whether sigma represents an error class.</summary>
    public static bool IsError(string sigma) => sigma.StartsWith("err|", StringComparison.Ordinal);

    /// <summary>
    /// The purpose-relative behavior class (plane key) for clustering (§4.2, §15.3):
    /// the result <c>kind</c> (plus a True/False tag for Boolean), or the error class
    /// for failures. The exact typed value is retained per-sample as provenance.
    /// </summary>
    public static string PlaneSigma(Observation o)
    {
        if (!o.Success)
            return "err|" + (o.ErrorMessage ?? "(no message)");
        return o.Kind switch
        {
            "Natural" => "Natural",
            "Integer" => "Integer",
            "Real" => "Real",
            "Boolean" => "Boolean:" + BooleanLabel(o),
            "Text" => "Text",
            "Vector" => "Vector",
            "Array" => "Array",
            "Function" => "Function",
            "Void" => "Void",
            _ => o.Kind ?? "Void",
        };
    }

    private static string BooleanLabel(Observation o)
    {
        var typed = o.Typed ?? "";
        return typed.StartsWith("True", StringComparison.Ordinal) ? "True" : "False";
    }
}
