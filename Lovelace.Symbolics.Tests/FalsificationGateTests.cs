using Lovelace.Symbolics.Rewriting;
using Xunit;
using Xunit.Abstractions;
using Cplx = Lovelace.Complex.Complex;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>One real sample point, tagged with the region it probes and WHY it is near that
/// region's feature. The rationale is data, not prose: a test asserts every probe carries one.</summary>
internal readonly record struct RealProbe(string Region, Rat Value, string Label, string Why);

/// <summary>One complex sample point. Re/Im are exact terminating decimals parsed by
/// <see cref="Rl.Parse(string, object?)"/>; no epsilon enters the point itself.</summary>
internal readonly record struct ComplexProbe(string Region, string Re, string Im, string Label, string Why);

/// <summary>
/// Round 27 sample regions. Regions are the falsification gate's coverage contract:
/// <list type="bullet">
/// <item>legacy-interior - the 13 points the gate had before round 27 (kept verbatim);</item>
/// <item>near-pole - within a stated epsilon of the pole x = 0 of x.x^-1 and 0.x^-1;</item>
/// <item>near-branch-cut - straddling the branch point x = 0 / negative real axis of log and sqrt;</item>
/// <item>near-discontinuity - either side of the jump at x = 0 of |x| (derivative) and x.x^-1;</item>
/// <item>assumption-boundary - both sides of the x &gt;= 0 (NonNegative) boundary the gate itself sets;</item>
/// <item>complex - a sweep over 4 quadrants, axes and small moduli, not one token point.</item>
/// </list>
/// </summary>
internal static class FalsificationRegions
{
    // ---- legacy: unchanged, in the original order (superset guarantee) ----
    public static readonly RealProbe[] Legacy =
    {
        new("legacy-interior", Rat.From(-100, 1), "-100", "large negative magnitude (ordering/overflow boundary)"),
        new("legacy-interior", Rat.From(-5, 1), "-5", "interior negative"),
        new("legacy-interior", Rat.From(-3, 2), "-3/2", "interior negative fractional"),
        new("legacy-interior", Rat.From(-1, 1), "-1", "unit negative, sign boundary"),
        new("legacy-interior", Rat.From(-1, 3), "-1/3", "small negative fraction"),
        new("legacy-interior", Rat.From(-1, 1000), "-1/1000", "near-zero negative"),
        new("legacy-interior", Rat.Zero, "0", "exact zero: definedness boundary of x^-1"),
        new("legacy-interior", Rat.From(1, 1000), "1/1000", "near-zero positive"),
        new("legacy-interior", Rat.From(1, 3), "1/3", "small positive fraction"),
        new("legacy-interior", Rat.From(1, 1), "1", "unit positive"),
        new("legacy-interior", Rat.From(3, 2), "3/2", "interior positive fractional"),
        new("legacy-interior", Rat.From(5, 1), "5", "interior positive"),
        new("legacy-interior", Rat.From(100, 1), "100", "large positive magnitude"),
    };

    // ---- near-pole: |x| <= 1e-6 of the pole at 0 ----
    // The registry's only poles are at x = 0: x.x^-1 and 0.x^-1 are undefined there
    // (Simplify.cs round-27 registry), and log is singular at 0. Points are exact rationals,
    // so 'near' costs nothing in precision: epsilon = 1e-6 and 1e-9.
    public static readonly RealProbe[] NearPole =
    {
        new("near-pole", Rat.From(1, 1000000), "1/10^6", "1e-6 above the pole at 0 (exact rational, epsilon = 1e-6)"),
        new("near-pole", Rat.From(-1, 1000000), "-1/10^6", "1e-6 below the pole at 0 (exact rational, epsilon = 1e-6)"),
        new("near-pole", Rat.From(1, 1000000000), "1/10^9", "1e-9 above the pole at 0 (exact rational, epsilon = 1e-9)"),
        new("near-pole", Rat.From(-1, 1000000000), "-1/10^9", "1e-9 below the pole at 0 (exact rational, epsilon = 1e-9)"),
    };

    // ---- near-branch-cut ----
    // log and sqrt branch at x = 0 with the cut along the negative real axis; the cut is
    // approached from both sides with exact rationals of the same magnitude (a straddling pair),
    // plus exact negative rationals whose roots are themselves exact (1/4, 4, 25) so the
    // kernel's exact-root rules bind instead of rounding.
    public static readonly RealProbe[] NearBranchCut =
    {
        new("near-branch-cut", Rat.From(-1, 1000000), "-1/10^6", "negative side of the cut, 1e-6 from the branch point (exact)"),
        new("near-branch-cut", Rat.From(1, 1000000), "1/10^6", "straddles the cut against -1/10^6 at the same magnitude (exact)"),
        new("near-branch-cut", Rat.From(-1, 1000000000), "-1/10^9", "negative side of the cut, 1e-9 from the branch point (exact)"),
        new("near-branch-cut", Rat.From(1, 1000000000), "1/10^9", "straddles the cut against -1/10^9 at the same magnitude (exact)"),
        new("near-branch-cut", Rat.From(-1, 4), "-1/4", "on the cut: sqrt(1/4) = 1/2 exactly, so an exact-root path is exercised, not a rounded one"),
        new("near-branch-cut", Rat.From(-4, 1), "-4", "on the cut: sqrt(4) = 2 exactly"),
        new("near-branch-cut", Rat.From(-25, 1), "-25", "further out on the cut: sqrt(25) = 5 exactly"),
    };

    // ---- near-discontinuity ----
    // |x| is continuous but its derivative jumps at 0, and x.x^-1 is undefined exactly at 0;
    // these points sit either side of that jump at scales DIFFERENT from the pole/branch epsilons
    // (odd denominators), so they are not duplicates of the regions above. 1/999 and 1/1000003
    // are exact rationals; they become approximations only when sin/cos/exp/log convert them to
    // the kernel's Real at the 40-digit working precision (Rl.WithPrecision(40, 20)) - the
    // comparison tolerance below is the stated 1e-11.
    public static readonly RealProbe[] NearDiscontinuity =
    {
        new("near-discontinuity", Rat.From(-1, 999), "-1/999", "1.001e-3 below the jump at 0 (odd denominator: exercises the rounded real path)"),
        new("near-discontinuity", Rat.From(1, 999), "1/999", "1.001e-3 above the jump at 0"),
        new("near-discontinuity", Rat.From(-1, 1000003), "-1/1000003", "1e-6 below the jump at 0 (prime denominator, distinct scale)"),
        new("near-discontinuity", Rat.From(1, 1000003), "1/1000003", "1e-6 above the jump at 0"),
    };

    // ---- assumption-boundary ----
    // The gate itself sets x >= 0 (SymbolPredicate.NonNegative) to license pow.sqrt-square-nonnegative.
    // The boundary is 0: -1/10^6 and -1/10^9 violate it (the rule must make no claim there),
    // 0 sits exactly on it (NonNegative holds), +1/10^6 and +1/10^9 satisfy it.
    public static readonly RealProbe[] AssumptionBoundary =
    {
        new("assumption-boundary", Rat.From(-1, 1000000), "-1/10^6", "outside x >= 0 by 1e-6 (a rule with that side condition must not be licensed here)"),
        new("assumption-boundary", Rat.From(-1, 1000000000), "-1/10^9", "outside x >= 0 by 1e-9 (just past the boundary)"),
        new("assumption-boundary", Rat.Zero, "0", "exactly on the boundary: NonNegative(x) holds, NonZero(x) does not"),
        new("assumption-boundary", Rat.From(1, 1000000), "1/10^6", "inside x >= 0 by 1e-6"),
        new("assumption-boundary", Rat.From(1, 1000000000), "1/10^9", "inside x >= 0 by 1e-9 (just inside the boundary)"),
    };

    public static readonly RealProbe[] AllRealProbes =
        Legacy.Concat(NearPole).Concat(NearBranchCut).Concat(NearDiscontinuity).Concat(AssumptionBoundary).ToArray();

    /// <summary>Every real sample point the gate sweeps (legacy first, then each new region).</summary>
    public static readonly Rat[] AllRealPoints = AllRealProbes.Select(p => p.Value).ToArray();

    // ---- complex ----
    // A genuine sweep, not one token point: 4 quadrants, both axes, small and moderate moduli.
    // Every part is an exact terminating decimal, so Rl.Parse introduces no point-level epsilon.
    // All |Im z| <= 2 < pi, so exp/log stay inside log's principal strip |Im z| < pi: a rule that
    // is only licensed on that strip cannot be falsely falsified by this sweep.
    public static readonly ComplexProbe[] ComplexProbes =
    {
        new("complex", "1", "1", "1+i", "first quadrant, unit imaginary part"),
        new("complex", "1", "-1", "1-i", "fourth quadrant, conjugate of 1+i"),
        new("complex", "-1", "1", "-1+i", "second quadrant (negative real part: |z|^2 != z^2 territory)"),
        new("complex", "-1", "-1", "-1-i", "third quadrant, conjugate of -1+i"),
        new("complex", "0", "1", "i", "purely imaginary: re(z) = 0, |z| = 1"),
        new("complex", "0", "-1", "-i", "purely imaginary, negative: conjugate of i"),
        new("complex", "0.5", "0.5", "0.5+0.5i", "small modulus (|z| ~ 0.707) near the branch point"),
        new("complex", "-0.5", "1.5", "-0.5+1.5i", "small negative real part, |Im| = 1.5 < pi"),
        new("complex", "1.5", "-0.5", "1.5-0.5i", "mirror of the above across the real axis"),
        new("complex", "2", "0.5", "2+0.5i", "moderate modulus with a small imaginary part"),
        new("complex", "-2", "-0.5", "-2-0.5i", "moderate negative modulus, |Im| = 0.5 < pi"),
        new("complex", "2", "0", "2+0i", "complex-typed value on the real axis: exercises the real/complex promotion path"),
    };

    public static readonly string[] RequiredRegions =
    {
        "legacy-interior", "near-pole", "near-branch-cut", "near-discontinuity", "assumption-boundary", "complex",
    };
}

/// <summary>What the registry gate saw. Failures are values here, so a negative control can
/// assert that the gate REPORTS a failure instead of passing silently.</summary>
internal sealed class GateReport
{
    public List<string> Registered { get; } = new();
    public List<string> Sampled { get; } = new();
    public List<string> Uninstantiable { get; } = new();
    public List<string> Unlicensed { get; } = new();
    public List<string> Degenerate { get; } = new();
    public List<string> Threw { get; } = new();
    public List<string> Falsified { get; } = new();
    public List<string> LicenseNotes { get; } = new();
    public List<string> UnhandledAtoms { get; } = new();
    public Dictionary<string, int> Compared { get; } = new(StringComparer.Ordinal);
    public Dictionary<string, HashSet<string>> Regions { get; } = new(StringComparer.Ordinal);

    /// <summary>Unexpected exception from rule code, or a value disagreement: both are failures.</summary>
    public bool Failed => Threw.Count > 0 || Falsified.Count > 0;

    public void Count(string ruleId, string region)
    {
        Compared[ruleId] = Compared.TryGetValue(ruleId, out var n) ? n + 1 : 1;
        if (!Regions.TryGetValue(ruleId, out var set))
            Regions[ruleId] = set = new HashSet<string>(StringComparer.Ordinal);
        set.Add(region);
    }

    public HashSet<string> AllCoveredRegions()
    {
        var all = new HashSet<string>(StringComparer.Ordinal);
        foreach (var set in Regions.Values)
            all.UnionWith(set);
        return all;
    }

    public string Summary() =>
        $"registered={Registered.Count} sampled={Sampled.Count} uninstantiable={Uninstantiable.Count} " +
        $"unlicensed={Unlicensed.Count} degenerate={Degenerate.Count} threw={Threw.Count} falsified={Falsified.Count} " +
        $"regions={string.Join(",", AllCoveredRegions().OrderBy(r => r, StringComparer.Ordinal))}";
}

/// <summary>
/// Registry-driven falsification. Every rule in the kernel's registry is instantiated from its
/// own <see cref="Pattern"/> (wildcards -> the probe symbol x) and then APPLIED through its own
/// <see cref="RewriteRule.Replacement"/>, and the result is compared with the unrewritten
/// expression at every sample point of every region. Nothing here lists rule ids: a rule
/// registered tomorrow is swept the moment it is registered, or the count assertion fires.
/// </summary>
internal static class RegistryFalsificationGate
{
    public static GateReport Run(IReadOnlyList<RewriteRule> rules, ExprContext ctx, Symbol x)
    {
        using var scope = Rl.WithPrecision(40, 20);
        var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 11) + "1", null)); // 1e-11, as before
        var report = new GateReport();

        foreach (var rule in rules)
        {
            report.Registered.Add(rule.Id);
            var binds = new Dictionary<string, Expr>(StringComparer.Ordinal);
            var lhs = Instantiate(rule.Pattern, x, ctx, binds);
            if (lhs is null)
            {
                report.Uninstantiable.Add(rule.Id + " (pattern node " + rule.Pattern.GetType().Name + " has no expression form)");
                continue;
            }
            if (!Printing.PrettyPrint(lhs).Contains(x.Name, StringComparison.Ordinal))
                report.Degenerate.Add(rule.Id + " (pattern instantiation lost the probe: " + Printing.PrettyPrint(lhs) + ")");

            var match = new Match();
            foreach (var b in binds)
                match.Bind(b.Key, b.Value);

            var licensed = false;
            foreach (var env in Environments(rule, match, ctx, x))
            {
                if (!Licensed(rule, match, ctx, env, out var why))
                {
                    report.LicenseNotes.Add(rule.Id + " under " + Describe(env) + ": " + why);
                    continue;
                }
                licensed = true;

                Expr rhs;
                try
                {
                    using (ctx.WithAssumptions(env))
                        rhs = rule.Replacement(match, ctx);
                }
                catch (Exception ex)
                {
                    // A rule that throws is a defect, not "not applicable": only the kernel's typed
                    // EvaluationException licenses a skip. This is the strictness the gate must keep.
                    report.Threw.Add(rule.Id + " replacement: " + ex.GetType().Name + ": " + ex.Message);
                    continue;
                }

                using (ctx.WithAssumptions(env))
                {
                    foreach (var probe in FalsificationRegions.AllRealProbes)
                        ComparePoint(report, rule.Id, probe.Region, probe.Label, NumOps.FromRat(probe.Value), lhs, rhs, env, ctx, x, tol);
                    foreach (var probe in FalsificationRegions.ComplexProbes)
                        ComparePoint(report, rule.Id, probe.Region, probe.Label,
                            NumOps.FromComplex(new Cplx(Rl.Parse(probe.Re, null), Rl.Parse(probe.Im, null))),
                            lhs, rhs, env, ctx, x, tol);
                }
            }
            if (licensed)
                report.Sampled.Add(rule.Id);
            else
                report.Unlicensed.Add(rule.Id);
        }
        return report;
    }

    private static void ComparePoint(
        GateReport report, string ruleId, string region, string label, Num value,
        Expr lhs, Expr rhs, AssumptionSet env, ExprContext ctx, Symbol x, Num tol)
    {
        foreach (var atom in env.Atoms)
            if (!AtomHolds(atom, value, ctx, x, report))
                return;   // the rule makes no claim outside its own assumption boundary

        Num lv, rv;
        try
        {
            var bindings = new Dictionary<Symbol, Num> { [x] = value };
            lv = Evaluation.EvaluateToNum(lhs, ctx, bindings);
            rv = Evaluation.EvaluateToNum(rhs, ctx, bindings);
        }
        catch (EvaluationException)
        {
            return;   // outside the rule's definedness domain - not a counterexample
        }
        catch (Exception ex)
        {
            report.Threw.Add(ruleId + " at " + region + ":" + label + ": " + ex.GetType().Name + ": " + ex.Message);
            return;
        }

        report.Count(ruleId, region);
        var delta = NumOps.Abs(NumOps.Subtract(lv, rv), ctx);
        if (NumOps.Compare(delta, tol) >= 0)
            report.Falsified.Add(ruleId + " at " + region + ":" + label + ": " + Printing.PrettyPrint(lhs) +
                                 " != " + Printing.PrettyPrint(rhs));
    }

    /// <summary>Assumption ladders a rule may be licensed under: its own declared conditions,
    /// those conditions plus realness of the probe, and realness alone. A rule licensed under no
    /// environment is reported unlicensed (never silently skipped).</summary>
    private static List<AssumptionSet> Environments(RewriteRule rule, Match match, ExprContext ctx, Symbol x)
    {
        AssumptionSet self;
        try
        {
            self = rule.ConditionBuilder?.Invoke(match, ctx) ?? rule.DeclaredConditions;
        }
        catch (Exception)
        {
            self = AssumptionSet.Empty;
        }
        var envs = new List<AssumptionSet>();
        void Consider(AssumptionSet s)
        {
            if (!envs.Contains(s))
                envs.Add(s);
        }
        Consider(self);
        var real = new SymbolDomainAssumption(x, Domain.Real);
        try { Consider(self.Add(real)); } catch (AssumptionContradictionException) { }
        Consider(AssumptionSet.Empty.Add(real));
        return envs;
    }

    private static bool Licensed(RewriteRule rule, Match match, ExprContext ctx, AssumptionSet env, out string why)
    {
        why = "";
        using var scope = ctx.WithAssumptions(env);
        RuleApplicability app;
        try
        {
            app = rule.Applicability(match, ctx);
        }
        catch (Exception ex)
        {
            why = "applicability threw " + ex.GetType().Name;
            return false;
        }
        if (app is RuleApplicability.NotApplicable or RuleApplicability.Unknown)
        {
            why = app.ToString();
            return false;
        }
        AssumptionSet conds;
        try
        {
            conds = rule.ConditionBuilder?.Invoke(match, ctx) ?? rule.DeclaredConditions;
        }
        catch (Exception ex)
        {
            why = "condition builder threw " + ex.GetType().Name;
            return false;
        }
        if (conds.IsUnsatisfiable)
        {
            why = "conditions unsatisfiable";
            return false;
        }
        foreach (var atom in conds.Atoms)
        {
            if (ctx.Assumptions.Ask(atom) != Tristate.True)
            {
                why = "condition not provable: " + atom;
                return false;
            }
        }
        return true;
    }

    /// <summary>Builds the expression a pattern denotes, binding every wildcard to the probe.
    /// Returns null for a pattern node with no expression form (a new node type must be handled
    /// here rather than silently escaping the sweep).</summary>
    private static Expr? Instantiate(Pattern p, Expr probe, ExprContext ctx, Dictionary<string, Expr> binds)
    {
        switch (p)
        {
            case LiteralPat l:
                return l.E;
            case WildPat w:
                binds[w.Name] = probe;
                return probe;
            case AddPat a:
            {
                var args = new List<Expr>();
                foreach (var t in a.Terms)
                {
                    var e = Instantiate(t, probe, ctx, binds);
                    if (e is null) return null;
                    args.Add(e);
                }
                return Exprs.Add(args);
            }
            case MulPat m:
            {
                var args = new List<Expr>();
                foreach (var f in m.Factors)
                {
                    var e = Instantiate(f, probe, ctx, binds);
                    if (e is null) return null;
                    args.Add(e);
                }
                return Exprs.Multiply(args);
            }
            case PowPat pw:
            {
                var b = Instantiate(pw.B, probe, ctx, binds);
                var e = Instantiate(pw.E, probe, ctx, binds);
                return b is null || e is null ? null : Exprs.Power(b, e);
            }
            case FunPat f:
            {
                var args = new List<Expr>();
                foreach (var arg in f.Args)
                {
                    var e = Instantiate(arg, probe, ctx, binds);
                    if (e is null) return null;
                    args.Add(e);
                }
                return Exprs.Function(ctx.Function(f.Name), args.ToArray());
            }
            default:
                return null;   // SeqWildPat and any future pattern node
        }
    }

    private static string Describe(AssumptionSet env) =>
        env.Atoms.Length == 0 ? "{}" : "{" + string.Join("; ", env.Atoms.Select(a => a.ToString())) + "}";

    private static bool AtomHolds(Assumption atom, Num value, ExprContext ctx, Symbol x, GateReport report)
    {
        switch (atom)
        {
            case SymbolDomainAssumption sd:
                if (!sd.S.Equals(x))
                    return false;
                return sd.D switch
                {
                    Domain.Integer or Domain.Rational or Domain.Real => value is not NumComplex,
                    _ => true,
                };
            case SymbolPropertyAssumption sp:
                return sp.S.Equals(x) && PredicateHolds(sp.P, value);
            case ExpressionPropertyAssumption ep:
            {
                var ev = SafeEvaluate(ep.E, ctx, x, value);
                return ev is not null && PredicateHolds(ep.P, ev);
            }
            case IntervalAssumption ia:
            {
                var v = SafeEvaluate(ia.E, ctx, x, value);
                if (v is null || v is NumComplex)
                    return false;
                var vv = NumOps.FromReal(NumOps.ToReal(v));
                if (ia.Lower is { } lo)
                {
                    var l = SafeEvaluate(lo, ctx, x, value);
                    if (l is null || l is NumComplex)
                        return false;
                    var c = NumOps.Compare(vv, NumOps.FromReal(NumOps.ToReal(l)));
                    if (c < 0 || (c == 0 && ia.LowerOpen))
                        return false;
                }
                if (ia.Upper is { } up)
                {
                    var u = SafeEvaluate(up, ctx, x, value);
                    if (u is null || u is NumComplex)
                        return false;
                    var c = NumOps.Compare(vv, NumOps.FromReal(NumOps.ToReal(u)));
                    if (c > 0 || (c == 0 && ia.UpperOpen))
                        return false;
                }
                return true;
            }
            default:
                // An atom kind this guard cannot decide: surface it instead of skipping silently.
                report.UnhandledAtoms.Add(atom.GetType().Name + ": " + atom);
                return false;
        }
    }

    /// <summary>Null means "the condition is not decidable at this point", which the guard
    /// treats as "outside the assumed domain" rather than as a satisfied condition.</summary>
    private static Num? SafeEvaluate(Expr e, ExprContext ctx, Symbol x, Num value)
    {
        try
        {
            return Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num> { [x] = value });
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static bool PredicateHolds(SymbolPredicate p, Num value) => p switch
    {
        SymbolPredicate.Finite => true,        // every evaluated Num in this kernel is finite
        SymbolPredicate.NonZero => !NumOps.IsZero(value),
        SymbolPredicate.Positive => value is not NumComplex && RealSign(value) > 0,
        SymbolPredicate.NonNegative => value is not NumComplex && RealSign(value) >= 0,
        SymbolPredicate.Negative => value is not NumComplex && RealSign(value) < 0,
        SymbolPredicate.NonPositive => value is not NumComplex && RealSign(value) <= 0,
        _ => false,                            // Even/Odd are not decidable at these points
    };

    private static int RealSign(Num value) =>
        NumOps.Compare(NumOps.FromReal(NumOps.ToReal(value)), NumOps.FromLong(0L));

    /// <summary>Exposes the assumption-boundary guard so the boundary test can assert where it flips.</summary>
    internal static bool AtomHoldsForTesting(Assumption atom, Num value, ExprContext ctx, Symbol x, GateReport report) =>
        AtomHolds(atom, value, ctx, x, report);
}

/// <summary>
/// Round 27: the falsification gate is registry-driven and samples five new regions on top of the
/// original 13 points. Every [Fact] in FalsificationTests still runs unchanged (superset), so the
/// older gate keeps its strictness; these tests add the registry sweep and the negative controls.
/// </summary>
public class FalsificationGateTests
{
    private readonly ITestOutputHelper _out;

    public FalsificationGateTests(ITestOutputHelper output) => _out = output;

    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    [Fact]
    public void RegistryGate_EveryRegisteredRuleIsSampled_OnEveryRegion()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var registered = Simplify.RulesForTesting(ctx);
        var shippedIds = Simplify.ShippedRuleIds(ctx);
        var report = RegistryFalsificationGate.Run(registered, ctx, x);

        _out.WriteLine("REGISTRY COUNT (RulesForTesting / ShippedRuleIds / sampled) = " +
            registered.Count + " / " + shippedIds.Count + " / " + report.Sampled.Count);
        _out.WriteLine(report.Summary());
        foreach (var rule in registered)
            _out.WriteLine("  " + rule.Id + ": compared=" + report.Compared.GetValueOrDefault(rule.Id) +
                " regions=" + string.Join(",", report.Regions.TryGetValue(rule.Id, out var rs) ? rs.OrderBy(r => r, StringComparer.Ordinal) : Enumerable.Empty<string>()));
        foreach (var note in report.LicenseNotes)
            _out.WriteLine("  license-note " + note);

        // the two registry views must agree, and every registered rule must have been swept
        Assert.Equal(registered.Count, shippedIds.Count);
        Assert.Equal(registered.Count, report.Sampled.Count);
        Assert.Equal(registered.Count, report.Sampled.Distinct().Count());
        Assert.Empty(report.Uninstantiable);
        Assert.Empty(report.Unlicensed);
        Assert.Empty(report.Degenerate);

        // the strict part: a rule that throws, or a value disagreement, fails the gate
        Assert.Empty(report.Threw);
        Assert.Empty(report.Falsified);
        Assert.False(report.Failed, report.Summary());

        // non-vacuity: every sampled rule must have had at least one point actually compared,
        // and every declared region must have compared at least one point
        foreach (var rule in registered)
            Assert.True(report.Compared.GetValueOrDefault(rule.Id) > 0,
                $"rule '{rule.Id}' was registered but no sample point was compared for it. {report.Summary()}");
        var covered = report.AllCoveredRegions();
        foreach (var region in FalsificationRegions.RequiredRegions)
            Assert.True(covered.Contains(region),
                $"region '{region}' contributed no compared point. {report.Summary()}");
    }

    [Fact]
    public void Gate_Fails_WhenARuleThrows()   // negative control for the strictness property
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var throwing = new RewriteRule(
            "test.control-throwing-rule", "test", new WildPat("t"),
            (m, c) => RuleApplicability.Applicable,
            (m, c) => throw new InvalidOperationException("deliberate negative control: replacement throws"));

        var report = RegistryFalsificationGate.Run(new[] { throwing }, ctx, x);

        Assert.True(report.Threw.Count > 0, "the gate must report failure when a rule throws; it reported none. " + report.Summary());
        Assert.True(report.Failed, "the gate must report failure when a rule throws. " + report.Summary());
        Assert.Contains(report.Threw, t => t.Contains("test.control-throwing-rule") && t.Contains("InvalidOperationException"));
        Assert.Single(report.Sampled);
        _out.WriteLine("THROWING-RULE CONTROL: Failed=" + report.Failed + " threw=" + report.Threw.Count);
        foreach (var line in report.Threw)
            _out.WriteLine("  " + line);
    }

    [Fact]
    public void Gate_ReportsFalsification_WhenARuleIsWrong()   // the gate can still fail on a value mismatch
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // sqrt(x^2) -> x claimed unconditionally: wrong at every negative point (it is |x| there)
        var wrong = new RewriteRule(
            "test.control-wrong-rule", "test",
            new PowPat(new PowPat(new WildPat("w"), new LiteralPat(Exprs.Integer(2))), new LiteralPat(Exprs.Rational(1, 2))),
            (m, c) => RuleApplicability.Applicable,
            (m, c) => m.Get("w"));

        var report = RegistryFalsificationGate.Run(new[] { wrong }, ctx, x);

        Assert.True(report.Falsified.Count > 0, "a wrong rule must be falsified. " + report.Summary());
        Assert.True(report.Failed, report.Summary());
        Assert.Contains(report.Falsified, f => f.Contains("test.control-wrong-rule"));
        _out.WriteLine("WRONG-RULE CONTROL: Failed=" + report.Failed + " falsified=" + report.Falsified.Count);
        _out.WriteLine("  first falsification: " + report.Falsified[0]);
    }

    [Fact]
    public void Sampling_IsASuperset_OfTheOriginalPoints_AndEveryProbeIsDocumented()
    {
        // (1) no original sample point was dropped or weakened
        foreach (var legacy in FalsificationTests.LegacyPoints)
            Assert.Contains(legacy, FalsificationRegions.AllRealPoints);

        // (2) every probe states why it is near its region's feature
        Assert.All(FalsificationRegions.AllRealProbes, p => Assert.False(string.IsNullOrWhiteSpace(p.Why)));
        Assert.All(FalsificationRegions.ComplexProbes, p => Assert.False(string.IsNullOrWhiteSpace(p.Why)));

        // (3) each new real region is genuinely near its named feature
        Assert.NotEmpty(FalsificationRegions.NearPole);
        Assert.All(FalsificationRegions.NearPole, p => Assert.True(
            p.Value >= Rat.From(-1, 1000000) && p.Value <= Rat.From(1, 1000000),
            "near-pole point must lie within epsilon = 1e-6 (inclusive) of the pole at 0: " + p.Label));
        Assert.NotEmpty(FalsificationRegions.NearBranchCut);
        Assert.Contains(FalsificationRegions.NearBranchCut, p => p.Value.IsNegative);
        Assert.Contains(FalsificationRegions.NearBranchCut, p => !p.Value.IsNegative);
        Assert.NotEmpty(FalsificationRegions.NearDiscontinuity);
        Assert.Contains(FalsificationRegions.NearDiscontinuity, p => p.Value.IsNegative);
        Assert.Contains(FalsificationRegions.NearDiscontinuity, p => !p.Value.IsNegative);
        Assert.Contains(FalsificationRegions.AssumptionBoundary, p => p.Value.IsZero);
        Assert.Contains(FalsificationRegions.AssumptionBoundary, p => p.Value.IsNegative);
        Assert.Contains(FalsificationRegions.AssumptionBoundary, p => !p.Value.IsNegative && !p.Value.IsZero);

        // (4) the complex region is a sweep, not a token point
        Assert.True(FalsificationRegions.ComplexProbes.Length >= 8);
        Assert.Contains(FalsificationRegions.ComplexProbes, p => p.Im != "0");
        Assert.Contains(FalsificationRegions.ComplexProbes, p => p.Im == "0");
    }

    [Fact]
    public void AssumptionBoundary_GuardFlipsExactlyAtZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var env = AssumptionSet.Empty.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
        var report = new GateReport();
        var atom = env.Atoms[0];

        Assert.True(RegistryFalsificationGate.AtomHoldsForTesting(atom, NumOps.FromRat(Rat.From(1, 1000000)), ctx, x, report));
        Assert.True(RegistryFalsificationGate.AtomHoldsForTesting(atom, NumOps.FromRat(Rat.Zero), ctx, x, report));
        Assert.False(RegistryFalsificationGate.AtomHoldsForTesting(atom, NumOps.FromRat(Rat.From(-1, 1000000)), ctx, x, report));
    }
}
