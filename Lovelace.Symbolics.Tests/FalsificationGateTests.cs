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
        new(ComplexRegion, "1", "1", "1+i", "first quadrant, unit imaginary part"),
        new(ComplexRegion, "1", "-1", "1-i", "fourth quadrant, conjugate of 1+i"),
        new(ComplexRegion, "-1", "1", "-1+i", "second quadrant (negative real part: |z|^2 != z^2 territory)"),
        new(ComplexRegion, "-1", "-1", "-1-i", "third quadrant, conjugate of -1+i"),
        new(ComplexRegion, "0", "1", "i", "purely imaginary: re(z) = 0, |z| = 1"),
        new(ComplexRegion, "0", "-1", "-i", "purely imaginary, negative: conjugate of i"),
        new(ComplexRegion, "0.5", "0.5", "0.5+0.5i", "small modulus (|z| ~ 0.707) near the branch point"),
        new(ComplexRegion, "-0.5", "1.5", "-0.5+1.5i", "small negative real part, |Im| = 1.5 < pi"),
        new(ComplexRegion, "1.5", "-0.5", "1.5-0.5i", "mirror of the above across the real axis"),
        new(ComplexRegion, "2", "0.5", "2+0.5i", "moderate modulus with a small imaginary part"),
        new(ComplexRegion, "-2", "-0.5", "-2-0.5i", "moderate negative modulus, |Im| = 0.5 < pi"),
        new(ComplexRegion, "2", "0", "2+0i", "complex-typed value on the real axis: exercises the real/complex promotion path"),
    };

    /// <summary>The region name the complex sweep carries.</summary>
    public const string ComplexRegion = "complex";

    /// <summary>The region tag of the extra exclusion witnesses, deliberately NOT a sweep region:
    /// it never contributes a gate comparison, so it is not in <see cref="RequiredRegions"/>.</summary>
    public const string OutsideStripRegion = "complex-outside-strip";

    /// <summary>Complex points a complex-EXCLUDED rule is attacked at, deliberately OUTSIDE that
    /// rule's own assumption boundary. The gate must not compare there - that is exactly the guard
    /// this file refuses to loosen - but a negative control must show the exclusion is load-bearing
    /// by OBSERVING the identity fail.
    /// <para>The first twelve witnesses are the ordinary sweep, in sweep order, so a control cannot
    /// pick a friendlier point than the sweep itself would have reached. The last two lie outside
    /// log's principal strip (|Im z| &lt; pi). The sweep deliberately keeps |Im z| &lt;= 2 &lt; pi so a
    /// strip-licensed rule cannot be falsely falsified by it - which also means no sweep point can
    /// witness log(exp(z)) != z; those two points supply the missing witness without being added to
    /// the sweep.</para></summary>
    public static readonly ComplexProbe[] ComplexExclusionWitnesses = ComplexProbes.Concat(new[]
    {
        new ComplexProbe(OutsideStripRegion, "0", "5", "5i",
            "outside log's principal strip (Im z = 5 > pi): the only place log(exp(z)) != z can be observed"),
        new ComplexProbe(OutsideStripRegion, "0", "-5", "-5i",
            "below the strip (Im z = -5 < -pi), the conjugate-side companion of 5i"),
    }).ToArray();

    public static readonly string[] RequiredRegions =
    {
        "legacy-interior", "near-pole", "near-branch-cut", "near-discontinuity", "assumption-boundary",
        ComplexRegion,
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

    /// <summary>Every complex comparison the gate actually made: rule, environment index and probe
    /// label. The complex-treatment audit reads this to prove that a complex point was skipped for
    /// a NAMED reason and never silently.</summary>
    public List<(string RuleId, int Env, string Label)> ComplexCompared { get; } = new();

    /// <summary>The shape the gate used per rule: the environments it was licensed under, and the
    /// instantiated LHS/RHS it compared there. The complex-exclusion controls re-attack exactly
    /// these expressions, so a control can never drift from what the gate actually swept.</summary>
    public Dictionary<string, List<(AssumptionSet Env, Expr Lhs, Expr Rhs)>> LicensedShapes { get; } = new(StringComparer.Ordinal);

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
            var shapes = new List<(AssumptionSet Env, Expr Lhs, Expr Rhs)>();
            report.LicensedShapes[rule.Id] = shapes;
            var envIndex = -1;
            foreach (var env in Environments(rule, match, ctx, x))
            {
                envIndex++;
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
                shapes.Add((env, lhs, rhs));

                using (ctx.WithAssumptions(env))
                {
                    foreach (var probe in FalsificationRegions.AllRealProbes)
                        ComparePoint(report, rule.Id, envIndex, probe.Region, probe.Label, NumOps.FromRat(probe.Value), lhs, rhs, env, ctx, x, tol, complex: false);
                    foreach (var probe in FalsificationRegions.ComplexProbes)
                        ComparePoint(report, rule.Id, envIndex, probe.Region, probe.Label,
                            NumOps.FromComplex(new Cplx(Rl.Parse(probe.Re, null), Rl.Parse(probe.Im, null))),
                            lhs, rhs, env, ctx, x, tol, complex: true);
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
        GateReport report, string ruleId, int envIndex, string region, string label, Num value,
        Expr lhs, Expr rhs, AssumptionSet env, ExprContext ctx, Symbol x, Num tol, bool complex)
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

        if (complex)
            report.ComplexCompared.Add((ruleId, envIndex, label));
        report.Count(ruleId, region);
        if (ExceedsTolerance(lv, rv, tol, ctx))
            report.Falsified.Add(ruleId + " at " + region + ":" + label + ": " + Printing.PrettyPrint(lhs) +
                                 " != " + Printing.PrettyPrint(rhs));
    }

    /// <summary>Whether |lhs - rhs| reaches the tolerance. The predicate is unchanged - both sides
    /// are non-negative, so comparing squares decides the same question - but a complex difference
    /// is decided on its EXACT squared magnitude re^2 + im^2 instead of |delta| = sqrt(re^2 + im^2).
    /// That keeps the tolerance check out of the transcendental sqrt path: a numeric failure there
    /// is a magnitude-computation defect, not evidence about the rule, and must not be able to
    /// decide (or abort) a rule's verdict. Real differences keep the original |delta| vs tol form.
    /// The exact squared magnitude is also the sharper test: no rounding is introduced by a 40-digit
    /// square root before the comparison.</summary>
    private static bool ExceedsTolerance(Num lv, Num rv, Num tol, ExprContext ctx)
    {
        var delta = NumOps.Subtract(lv, rv);
        if (delta is NumComplex c)
            return NumOps.Compare(NumOps.FromReal(c.V.MagnitudeSquared), NumOps.Multiply(tol, tol)) >= 0;
        return NumOps.Compare(NumOps.Abs(delta, ctx), tol) >= 0;
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

/// <summary>How one registered rule is treated in the complex region.</summary>
internal enum ComplexDisposition
{
    /// <summary>At least one complex probe satisfied every assumption atom and was compared.</summary>
    ComplexSampled,

    /// <summary>No complex probe was compared; every complex probe is excluded by a NAMED
    /// assumption atom, and at least one complex probe shows the identity genuinely failing.</summary>
    ComplexExclusionProven,
}

/// <summary>What the complex-treatment audit observed for one rule. Everything is a value so a
/// test can assert on the observation rather than on prose.</summary>
internal sealed class ComplexTreatment
{
    public ComplexTreatment(string ruleId) => RuleId = ruleId;

    public string RuleId { get; }
    public ComplexDisposition Disposition { get; set; }

    /// <summary>Complex sweep points the gate actually compared for this rule.</summary>
    public int SampledPoints { get; set; }

    /// <summary>Licensed environments the gate used for this rule.</summary>
    public int EnvsConsidered { get; set; }

    /// <summary>(environment, witness) pairs the gate skipped because a named atom failed.</summary>
    public int SkipsAttributedToNamedAtom { get; set; }

    /// <summary>(environment, witness) pairs not compared because the identity is not DEFINED
    /// there (EvaluationException) - the kernel's own domain boundary, not a guard decision.</summary>
    public int SkipsAttributedToDomain { get; set; }

    /// <summary>Pairs where no atom blocked the guard, both sides evaluated, and yet no comparison
    /// was recorded: a skip with no stated reason. Must be zero for every rule.</summary>
    public int UnexplainedSkips { get; set; }
    public List<string> UnexplainedDetail { get; } = new();

    // ---- the exclusion witness: an OBSERVED complex counterexample outside the rule's boundary ----
    public string WitnessLabel { get; set; } = "";
    public string WitnessRegion { get; set; } = "";
    public string WitnessPoint { get; set; } = "";
    public string WitnessEnv { get; set; } = "";
    public List<string> BlockingAtoms { get; } = new();
    public string WitnessLhs { get; set; } = "";
    public string WitnessRhs { get; set; } = "";
    public string WitnessLhsValue { get; set; } = "";
    public string WitnessRhsValue { get; set; } = "";
    public Num? WitnessDeltaSquared { get; set; }
}

/// <summary>
/// The complex half of the gate, kept OUT of the gate's verdicts. The gate must not compare a
/// real-only rule at a complex point where it makes no claim, so this audit instead asks, per
/// registered rule: was the rule complex-sampled, and if not, is every complex skip explained by a
/// named failing atom AND does at least one complex point show the identity genuinely failing?
/// A "proven" exclusion therefore means: the rule is real-only because it is FALSE over complex
/// numbers at an observed point, not because the gate declined to look.
/// </summary>
internal static class ComplexTreatmentAudit
{
    public static List<ComplexTreatment> Run(IReadOnlyList<RewriteRule> rules, ExprContext ctx, Symbol x, GateReport report)
    {
        using var scope = Rl.WithPrecision(40, 20);
        var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 11) + "1", null)); // 1e-11, as in the gate
        var tolSquared = NumOps.Multiply(tol, tol);
        var treatments = new List<ComplexTreatment>();

        foreach (var rule in rules)
        {
            var t = new ComplexTreatment(rule.Id);
            treatments.Add(t);
            if (!report.LicensedShapes.TryGetValue(rule.Id, out var shape))
                continue;   // unlicensed/uninstantiable: the gate itself reports that, out loud

            t.EnvsConsidered = shape.Count;
            t.SampledPoints = report.ComplexCompared.Count(c => c.RuleId == rule.Id);

            // (a) Skip accounting over the SWEEP points themselves: each (environment, sweep point)
            // pair is compared, blocked by a named atom, or undefined there. The two extra exclusion
            // witnesses are not sweep points, so they are deliberately NOT part of this accounting.
            for (var envIndex = 0; envIndex < shape.Count; envIndex++)
            {
                var (env, lhs, rhs) = shape[envIndex];
                foreach (var probe in FalsificationRegions.ComplexProbes)
                {
                    var value = NumOps.FromComplex(new Cplx(Rl.Parse(probe.Re, null), Rl.Parse(probe.Im, null)));
                    var blocked = env.Atoms
                        .Where(a => !RegistryFalsificationGate.AtomHoldsForTesting(a, value, ctx, x, report))
                        .ToArray();
                    if (blocked.Length > 0)
                    {
                        t.SkipsAttributedToNamedAtom++;
                        continue;
                    }
                    if (report.ComplexCompared.Contains((rule.Id, envIndex, probe.Label)))
                        continue;                                  // compared: nothing was skipped
                    if (Evaluate(lhs, rhs, ctx, x, value) is null)
                    {
                        t.SkipsAttributedToDomain++;               // undefined there, so no claim to test
                        continue;
                    }
                    t.UnexplainedSkips++;                          // skipped with no stated reason
                    t.UnexplainedDetail.Add(Describe(env) + " at " + probe.Label + " (every atom held)");
                }
            }

            // (b) The exclusion witness: for a rule the sweep never compared, search the sweep points
            // first and then the strip-crossing points for a complex value where the identity is
            // blocked by a named atom AND genuinely fails. Rules the sweep did compare need no
            // witness - their coverage is the comparison itself.
            if (t.SampledPoints == 0)
            {
                for (var envIndex = 0; envIndex < shape.Count && t.WitnessLabel.Length == 0; envIndex++)
                {
                    var (env, lhs, rhs) = shape[envIndex];
                    foreach (var witness in FalsificationRegions.ComplexExclusionWitnesses)
                    {
                        var value = NumOps.FromComplex(new Cplx(Rl.Parse(witness.Re, null), Rl.Parse(witness.Im, null)));
                        var blocking = env.Atoms
                            .Where(a => !RegistryFalsificationGate.AtomHoldsForTesting(a, value, ctx, x, report))
                            .ToArray();
                        if (blocking.Length == 0)
                            continue;                                  // the guard would have compared it
                        if (Evaluate(lhs, rhs, ctx, x, value) is not var (lv, rv))
                            continue;
                        var deltaSquared = SquaredMagnitude(NumOps.Subtract(lv, rv), ctx);
                        if (NumOps.Compare(deltaSquared, tolSquared) < 0)
                            continue;                                  // the identity still holds here
                        t.WitnessLabel = witness.Label;
                        t.WitnessRegion = witness.Region;
                        t.WitnessPoint = "z = " + witness.Label + " (re=" + witness.Re + ", im=" + witness.Im + ")";
                        t.WitnessEnv = Describe(env);
                        t.BlockingAtoms.AddRange(blocking.Select(a => a.ToString()));
                        t.WitnessLhs = Printing.PrettyPrint(lhs);
                        t.WitnessRhs = Printing.PrettyPrint(rhs);
                        t.WitnessLhsValue = Text(lv);
                        t.WitnessRhsValue = Text(rv);
                        t.WitnessDeltaSquared = deltaSquared;
                        break;
                    }
                }
            }

            t.Disposition = t.SampledPoints > 0 ? ComplexDisposition.ComplexSampled : ComplexDisposition.ComplexExclusionProven;
        }
        return treatments;
    }

    /// <summary>|delta|^2, exactly where the value is exact: a complex difference is squared through
    /// its real and imaginary parts (re^2 + im^2, no square root), a real one through |delta|^2.</summary>
    private static Num SquaredMagnitude(Num delta, ExprContext ctx)
    {
        if (delta is NumComplex c)
            return NumOps.FromReal(c.V.MagnitudeSquared);
        var abs = NumOps.Abs(delta, ctx);
        return NumOps.Multiply(abs, abs);
    }

    /// <summary>A numeric value as evidence text - the Num union carries no printer of its own, so
    /// it goes through the kernel's expression printer.</summary>
    public static string Text(Num value) => Printing.PrettyPrint(Evaluation.NumToExpr(value));

    /// <summary>Both sides at one point, or null when the identity is not defined there (the
    /// kernel's typed domain failure) - an unexpected exception still surfaces as a gate defect.</summary>
    private static (Num Lv, Num Rv)? Evaluate(Expr lhs, Expr rhs, ExprContext ctx, Symbol x, Num value)
    {
        var bindings = new Dictionary<Symbol, Num> { [x] = value };
        try
        {
            return (Evaluation.EvaluateToNum(lhs, ctx, bindings), Evaluation.EvaluateToNum(rhs, ctx, bindings));
        }
        catch (EvaluationException)
        {
            return null;
        }
    }

    private static string Describe(AssumptionSet env) =>
        env.Atoms.Length == 0 ? "{}" : "{" + string.Join("; ", env.Atoms.Select(a => a.ToString())) + "}";
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
        // the shared read-only sweep: the same registry, the same gate run, the same report the
        // complex-treatment facts read. Assertions below are exactly as before.
        var sweep = SharedSweep.Value;
        var ctx = sweep.Ctx;
        var x = sweep.X;
        var registered = sweep.Rules;
        var shippedIds = Simplify.ShippedRuleIds(ctx);
        var report = sweep.Report;

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

    // -------------------------------------------------------------------------------------------
    // The complex treatment. Every registered rule is either complex-SAMPLED (its identity was
    // compared at complex points) or complex-EXCLUSION-PROVEN (no complex point was compared,
    // every complex skip carries a named failing atom, and a complex counterexample was OBSERVED).
    // Nothing else is allowed. The guard is untouched: a rule that needs real arguments is still
    // never compared at a complex point - the controls attack it OUTSIDE its boundary instead.
    // -------------------------------------------------------------------------------------------

    /// <summary>The rules the complex sweep reaches, observed over the registry at this round.
    /// Hard-coded on purpose: a rule appearing, vanishing or switching disposition must break the
    /// partition test loudly rather than quietly changing the treatment.</summary>
    private static readonly string[] ComplexSampledRuleIds =
    {
        "abs.abs-neg",
        "logexp.exp-log",
        "rat.cancel-x-over-x",
        "rat.cancel-zero-over-x",
        "trig.pythagorean-sin2-cos2",
    };

    /// <summary>The real-only rules, and the complex point at which each one's identity is observed
    /// to FAIL - the evidence that excluding them from complex sampling is necessary, not a dodge.</summary>
    private static readonly (string RuleId, string WitnessLabel)[] ComplexExclusionProvenRules =
    {
        ("abs.abs-square", "1+i"),
        ("logexp.log-exp", "5i"),
        ("pow.sqrt-square-nonnegative", "-1+i"),
        ("pow.sqrt-square-real", "1+i"),
    };

    private static (IReadOnlyList<RewriteRule> Rules, GateReport Report, List<ComplexTreatment> Treatments) RunComplexAudit(
        ExprContext ctx, Symbol x)
    {
        var rules = Simplify.RulesForTesting(ctx);
        var report = RegistryFalsificationGate.Run(rules, ctx, x);
        return (rules, report, ComplexTreatmentAudit.Run(rules, ctx, x, report));
    }

    /// <summary>One registry sweep, shared by the facts that read it. The sweep is deterministic and
    /// read-only - the complex audit adds no verdict to it - so repeating it once per fact would only
    /// buy test time (~9 s per sweep at 40-digit precision). The facts in this class run sequentially
    /// (one xunit collection), and every assertion below is unchanged by the sharing.</summary>
    private sealed record RegistrySweep(
        ExprContext Ctx, Symbol X, IReadOnlyList<RewriteRule> Rules, GateReport Report, List<ComplexTreatment> Treatments);

    private static readonly Lazy<RegistrySweep> SharedSweep = new(() =>
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var (rules, report, treatments) = RunComplexAudit(ctx, x);
        return new RegistrySweep(ctx, x, rules, report, treatments);
    });

    [Fact]
    public void ComplexTreatment_PartitionsTheRegistry_WithExplicitCounts()
    {
        var sweep = SharedSweep.Value;
        var rules = sweep.Rules;
        var report = sweep.Report;
        var treatments = sweep.Treatments;

        var sampled = treatments.Where(t => t.Disposition == ComplexDisposition.ComplexSampled)
            .Select(t => t.RuleId).OrderBy(i => i, StringComparer.Ordinal).ToArray();
        var excluded = treatments.Where(t => t.Disposition == ComplexDisposition.ComplexExclusionProven)
            .Select(t => t.RuleId).OrderBy(i => i, StringComparer.Ordinal).ToArray();

        _out.WriteLine("COMPLEX TREATMENT (registered / complex-sampled / complex-exclusion-proven) = " +
            rules.Count + " / " + sampled.Length + " / " + excluded.Length);
        foreach (var t in treatments.OrderBy(t => t.RuleId, StringComparer.Ordinal))
            _out.WriteLine("  " + t.RuleId + ": " + t.Disposition +
                " complexPointsCompared=" + t.SampledPoints +
                " envs=" + t.EnvsConsidered +
                " skips[named-atom=" + t.SkipsAttributedToNamedAtom +
                ",domain=" + t.SkipsAttributedToDomain +
                ",unexplained=" + t.UnexplainedSkips + "]" +
                (t.WitnessLabel.Length > 0 ? " witness=" + t.WitnessPoint : ""));

        // the registry, the two dispositions and both counts: explicit, so the set can neither
        // shrink nor grow without this failing
        Assert.Equal(9, rules.Count);
        Assert.Equal(9, treatments.Count);
        Assert.Equal(rules.Count, treatments.Select(t => t.RuleId).Distinct().Count());
        Assert.Equal(ComplexSampledRuleIds, sampled);
        Assert.Equal(ComplexExclusionProvenRules.Select(r => r.RuleId).OrderBy(i => i, StringComparer.Ordinal).ToArray(), excluded);
        Assert.Equal(5, sampled.Length);
        Assert.Equal(4, excluded.Length);
        Assert.Equal(rules.Count, sampled.Length + excluded.Length);

        // non-vacuity in both directions
        Assert.All(treatments.Where(t => t.Disposition == ComplexDisposition.ComplexSampled),
            t => Assert.True(t.SampledPoints > 0, $"'{t.RuleId}' is complex-sampled but compared no complex point"));
        Assert.All(treatments.Where(t => t.Disposition == ComplexDisposition.ComplexExclusionProven),
            t => Assert.Equal(0, t.SampledPoints));

        // no complex point is skipped silently: every (environment, point) pair is either compared,
        // or blocked by a NAMED atom, or undefined there by the kernel's own typed domain failure
        Assert.All(treatments, t => Assert.True(t.UnexplainedSkips == 0,
            $"'{t.RuleId}' skipped complex points with no stated reason: {string.Join(" | ", t.UnexplainedDetail)}"));
        Assert.Empty(report.UnhandledAtoms);

        // and the guard's strictness is untouched by any of this
        Assert.Empty(report.Threw);
        Assert.Empty(report.Falsified);
        Assert.False(report.Failed, report.Summary());
    }

    [Fact]
    public void ComplexExclusion_EveryRealOnlyRuleHasAnObservedComplexCounterexample()
    {
        var sweep = SharedSweep.Value;
        var report = sweep.Report;
        var treatments = sweep.Treatments;
        using var scope = Rl.WithPrecision(40, 20);
        var tol = NumOps.FromReal(Rl.Parse("0." + new string('0', 11) + "1", null));
        var tolSquared = NumOps.Multiply(tol, tol);

        var excluded = treatments.Where(t => t.Disposition == ComplexDisposition.ComplexExclusionProven)
            .OrderBy(t => t.RuleId, StringComparer.Ordinal).ToArray();

        Assert.Equal(4, excluded.Length);
        Assert.Equal(ComplexExclusionProvenRules.Select(r => r.RuleId).OrderBy(i => i, StringComparer.Ordinal).ToArray(),
            excluded.Select(t => t.RuleId).ToArray());

        foreach (var t in excluded)
        {
            _out.WriteLine("EXCLUSION PROOF " + t.RuleId + ": " + t.WitnessLhs + " vs " + t.WitnessRhs +
                " at " + t.WitnessPoint + " => lhs=" + t.WitnessLhsValue + ", rhs=" + t.WitnessRhsValue +
                ", |lhs-rhs|^2 = " + ComplexTreatmentAudit.Text(t.WitnessDeltaSquared!) +
                "; blocked by " + string.Join(" & ", t.BlockingAtoms) + " under " + t.WitnessEnv);

            // (1) the exclusion is decided by a NAMED atom at an OBSERVED complex point
            Assert.False(string.IsNullOrEmpty(t.WitnessLabel),
                $"'{t.RuleId}' is complex-excluded without a witnessed complex counterexample");
            Assert.NotEmpty(t.BlockingAtoms);

            // (2) the identity genuinely FAILS there. If someone later "fixes" the rule so it holds
            // over the complexes, this is the assertion that fails loudly.
            Assert.NotNull(t.WitnessDeltaSquared);
            Assert.True(NumOps.Compare(t.WitnessDeltaSquared!, tolSquared) >= 0,
                $"'{t.RuleId}' must genuinely fail at {t.WitnessPoint}: |lhs-rhs|^2 was below the gate's tolerance");
            Assert.True(NumOps.Compare(t.WitnessDeltaSquared!, NumOps.FromLong(1L)) > 0,
                $"'{t.RuleId}' witnessed difference must be O(1), not rounding noise: |lhs-rhs|^2 = " +
                ComplexTreatmentAudit.Text(t.WitnessDeltaSquared!));
        }

        // (3) the exact point each rule is falsified at, pinned so a control cannot drift to a
        // friendlier point. These are the observed witnesses, not chosen prose.
        foreach (var (ruleId, witnessLabel) in ComplexExclusionProvenRules)
            Assert.Equal(witnessLabel, excluded.Single(t => t.RuleId == ruleId).WitnessLabel);

        // the exclusion is a property of the guard, never a gate failure
        Assert.Empty(report.Falsified);
        Assert.False(report.Failed, report.Summary());
    }

    [Fact]
    public void ComplexExclusion_LogExp_IsStoppedByRealness_NotByUndefinedness()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        using var scope = Rl.WithPrecision(40, 20);
        var report = new GateReport();

        // The registry's own side condition for log(exp(z)) -> z is membership of im(z) in
        // (-pi, pi] (Simplify.cs, logexp.log-exp), and the gate can only discharge it for a PROVABLY
        // REAL probe - which is why the effective gate environment carries the realness atom.
        var strip = new IntervalAssumption(
            Exprs.Function(ctx.Function("im"), x), Exprs.Negate(Exprs.Pi), true, Exprs.Pi, false);
        var realness = new SymbolDomainAssumption(x, Domain.Real);
        Num At(string re, string im) => NumOps.FromComplex(new Cplx(Rl.Parse(re, null), Rl.Parse(im, null)));

        // inside the strip the rule's own condition HOLDS at a complex point: what stops the gate
        // there is the realness atom, not "complex points are undefined for this rule"
        Assert.True(RegistryFalsificationGate.AtomHoldsForTesting(strip, At("1", "1"), ctx, x, report));
        Assert.False(RegistryFalsificationGate.AtomHoldsForTesting(realness, At("1", "1"), ctx, x, report));
        // outside the strip the rule's own condition fails - and the identity fails with it (the
        // witness in ComplexExclusion_EveryRealOnlyRuleHasAnObservedComplexCounterexample):
        // complex points genuinely include values where the claim is false
        Assert.False(RegistryFalsificationGate.AtomHoldsForTesting(strip, At("0", "5"), ctx, x, report));
        _out.WriteLine("LOGEXP STRIP: strip atom holds at 1+i = True, at 5i = False; realness atom at 1+i = False");
    }

    [Fact]
    public void Gate_StillFalsifies_AComplexOnlyWrongRule()   // negative control for the complex tolerance path
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        // z^2 -> |z|^2 is TRUE at every real point and FALSE at complex ones (1+i: 2i vs 2), so only
        // the complex region can catch it: the tolerance check must still report the disagreement
        // there - the squared-magnitude form may not become a way of missing complex failures.
        var wrong = new RewriteRule(
            "test.control-complex-only-wrong-rule", "test",
            new PowPat(new WildPat("z"), new LiteralPat(Exprs.Integer(2))),
            (m, c) => RuleApplicability.Applicable,
            (m, c) => Exprs.Power(Exprs.Function(ctx.Function("abs"), m.Get("z")), Exprs.Integer(2)));

        var report = RegistryFalsificationGate.Run(new[] { wrong }, ctx, x);

        Assert.True(report.Falsified.Count > 0, "a rule that is wrong only over the complexes must still be falsified. " + report.Summary());
        Assert.Contains(report.Falsified, f => f.Contains("test.control-complex-only-wrong-rule") &&
                                               f.Contains(FalsificationRegions.ComplexRegion + ":"));
        Assert.True(report.Failed, report.Summary());
        _out.WriteLine("COMPLEX-ONLY WRONG-RULE CONTROL: Failed=" + report.Failed + " falsified=" + report.Falsified.Count);
        foreach (var line in report.Falsified.Take(2))
            _out.WriteLine("  " + line);
    }
}
