namespace Lovelace.Symbolics;

/// <summary>How far a rewrite rule's proof obligation has got. The kernel ships no machine-checked
/// proofs yet — <see cref="Unproven"/> is the honest default, and it is the value every shipped
/// rule carries today.</summary>
public enum ProofStatus
{
    /// <summary>Stated, not proved by a machine.</summary>
    Unproven,

    /// <summary>A development exists but is not wired into CI.</summary>
    InProgress,

    /// <summary>Checked by an external prover, keyed by <see cref="ProofObligation.Theorem"/>.</summary>
    MachineChecked,
}

/// <summary>
/// A rule's proof obligation: what must be true for the rewrite to be sound, and where a
/// machine-checked development would attach.
/// </summary>
/// <param name="RuleId">The stable rewrite-rule id this obligation belongs to.</param>
/// <param name="Statement">The claim, in words, that a proof would establish.</param>
/// <param name="Theorem">The prover-side name once one exists; null while unproven.</param>
/// <param name="Status">How far the obligation has got.</param>
public sealed record ProofObligation(
    string RuleId, string Statement, string? Theorem, ProofStatus Status);

/// <summary>
/// The rule → proof-obligation bridge (alignment-plan §119). This is deliberately a *map*, not a
/// proof: it fixes the join point so a Lean (or other) development can be attached as data,
/// without the kernel pretending coverage it does not have. Every shipped rewrite rule has an
/// entry here, and a test keeps this table and the registry in step — a new rule without an
/// obligation fails the build rather than silently escaping the bridge.
/// </summary>
public static class RewriteProofs
{
    /// <summary>Obligations by rule id. Deterministic order.</summary>
    public static IReadOnlyDictionary<string, ProofObligation> Obligations { get; } =
        All().ToDictionary(o => o.RuleId, StringComparer.Ordinal);

    /// <summary>The obligation for <paramref name="ruleId"/>, or null when the rule has none.</summary>
    public static ProofObligation? For(string ruleId) =>
        Obligations.TryGetValue(ruleId, out var o) ? o : null;

    private static ProofObligation[] All() => new[]
    {
        new ProofObligation("trig.pythagorean-sin2-cos2",
            "sin(x)^2 + cos(x)^2 = 1 for every complex x", null, ProofStatus.Unproven),
        new ProofObligation("pow.sqrt-square-nonnegative",
            "sqrt(w^2) = w for every w >= 0", null, ProofStatus.Unproven),
        new ProofObligation("pow.sqrt-square-real",
            "sqrt(x^2) = |x| for every real x", null, ProofStatus.Unproven),
        new ProofObligation("logexp.exp-log",
            "exp(log(x)) = x for every x > 0", null, ProofStatus.Unproven),
        new ProofObligation("logexp.log-exp",
            "log(exp(x)) = x for every real x", null, ProofStatus.Unproven),
        new ProofObligation("abs.abs-square",
            "|x|^2 = x^2 for every real x", null, ProofStatus.Unproven),
        new ProofObligation("abs.abs-neg",
            "|−x| = |x| for every x", null, ProofStatus.Unproven),
        new ProofObligation("rat.cancel-x-over-x",
            "x / x = 1 for every x != 0", null, ProofStatus.Unproven),
        new ProofObligation("rat.cancel-zero-over-x",
            "0 / x = 0 for every x != 0", null, ProofStatus.Unproven),
    };
}
