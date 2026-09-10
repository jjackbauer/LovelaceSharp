using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Symbolics;

/// <summary>Legacy spelling of the solution disposition, retained for source compatibility.
/// New code should read <see cref="SolutionSet.Status"/> and <see cref="SolutionSet.Complete"/>.</summary>
public enum SolutionKind { Exact, Empty, Unevaluated }

/// <summary>
/// The solver's completeness claim. A <see cref="Solved"/> result is a claim that the represented
/// solution set is the COMPLETE solution set over the requested domain; a solver that can only
/// represent part of the set must say <see cref="Partial"/> (with the unrepresented count and a
/// reason) or <see cref="Unevaluated"/> — never <see cref="Solved"/>.
/// </summary>
public enum SolveStatus
{
    Solved,
    Partial,
    NoSolutions,
    Unevaluated,
    BudgetExceeded,
}

/// <summary>How much of the solution set a result represents.</summary>
public enum Completeness { Complete, Partial, Unknown }

/// <summary>Provenance of a returned solution value.</summary>
public enum SolutionExactness
{
    /// <summary>An exact value in the represented field (rational, or a radical with no branch caveat).</summary>
    Exact,
    /// <summary>An exact algebraic value (radical / RootOf): exact but not a rational.</summary>
    AlgebraicExact,
    /// <summary>A numeric approximation.</summary>
    Approximate,
    /// <summary>An exact parametric template (a family).</summary>
    ParametricExact,
}

/// <summary>
/// The field the solution set ranges over. The default is <see cref="Complex"/> (unconstrained
/// symbols range over the complex field per the kernel constitution). Deg ≤ 3 radical formulas
/// are complex-capable; complex algebraic roots (deg ≥ 4 irreducible factors) are not
/// representable in v1 (RootOf is real-only) and are reported Partial/Unevaluated under the
/// Complex domain rather than silently switching domains.
/// </summary>
public enum SolveDomain { Real, Complex }

/// <summary>
/// One solution: the value plus everything the kernel knows about it. <see cref="Conditions"/>
/// are the conditions of THIS branch only — never a union across branches.
/// </summary>
public sealed record Solution(Expr Value, AssumptionSet Conditions)
{
    /// <summary>Root multiplicity (1 for a simple root). Preserved from the factorization.</summary>
    public int Multiplicity { get; init; } = 1;

    /// <summary>Exactness provenance of <see cref="Value"/>.</summary>
    public SolutionExactness Exactness { get; init; } = SolutionExactness.Exact;

    /// <summary>False when the value is counted but not representable (complex algebraic root).</summary>
    public bool Representable { get; init; } = true;
}

/// <summary>A variable/value pair. The one structural binding contract for solver output,
/// substitutions, and compiled parameter metadata (never an "x = ..." string).</summary>
public sealed record Binding(string Name, Expr Value);

/// <summary>Domain of a solution-family parameter.</summary>
public enum ParameterDomain { Integers, NonNegativeIntegers }

/// <summary>
/// A parametric solution family: Template(parameter) for parameter in the domain, e.g.
/// Template = k·π with parameter k ∈ ℤ covers every solution of sin(x) = 0.
/// </summary>
public sealed record SolutionFamily(Expr Template, Symbol Parameter, Expr Period, ParameterDomain Domain)
{
    /// <summary>Conditions this family requires (branch locality: never unioned with others).</summary>
    public AssumptionSet Conditions { get; init; } = AssumptionSet.Empty;

    /// <summary>Exactness provenance of the template.</summary>
    public SolutionExactness Exactness { get; init; } = SolutionExactness.ParametricExact;
}

/// <summary>
/// A solver outcome. The public invariant: <see cref="Status"/> == <see cref="SolveStatus.Solved"/>
/// implies the represented set is complete over <see cref="Domain"/>.
/// </summary>
public sealed class SolutionSet
{
    public SolveStatus Status { get; private set; }

    /// <summary>How much of the solution set is represented.</summary>
    public Completeness Complete => Status switch
    {
        SolveStatus.Solved => Completeness.Complete,
        SolveStatus.Partial => Completeness.Partial,
        _ => Completeness.Unknown,
    };

    /// <summary>The domain the solution set is complete over (the requested domain).</summary>
    public SolveDomain Domain { get; }

    public List<Solution> Solutions { get; } = new();

    /// <summary>Parametric families (e.g. periodic inverses).</summary>
    public List<SolutionFamily> Families { get; } = new();

    /// <summary>Human-readable diagnostic (never the sole carrier of semantics).</summary>
    public string? Note { get; private set; }

    /// <summary>Machine-readable reason when part of the set is not representable.</summary>
    public string? UnrepresentedReason { get; private set; }

    /// <summary>How many roots of the requested domain are known but not represented here.</summary>
    public int UnrepresentedCount { get; private set; }

    /// <summary>Whether a search budget stopped the solve (partial result may still be present).</summary>
    public bool BudgetExceeded { get; private set; }

    /// <summary>Legacy disposition. Exact only for a genuinely complete result.</summary>
    public SolutionKind Kind => Status switch
    {
        SolveStatus.Solved => SolutionKind.Exact,
        SolveStatus.NoSolutions => SolutionKind.Empty,
        _ => SolutionKind.Unevaluated,
    };

    public SolutionSet(SolveStatus status, SolveDomain domain = SolveDomain.Complex, string? note = null)
    {
        Status = status;
        Domain = domain;
        Note = note;
    }

    /// <summary>Legacy constructor: maps the old tri-state onto the new status vocabulary.</summary>
    public SolutionSet(SolutionKind kind, string? note = null)
        : this(kind switch
        {
            SolutionKind.Exact => SolveStatus.Solved,
            SolutionKind.Empty => SolveStatus.NoSolutions,
            _ => SolveStatus.Unevaluated,
        }, SolveDomain.Complex, note)
    {
    }

    /// <summary>Downgrades a complete result to Partial, recording what is missing.</summary>
    public SolutionSet AsPartial(string? reason, int unrepresented)
    {
        if (Status == SolveStatus.Solved)
            Status = SolveStatus.Partial;
        UnrepresentedCount += unrepresented;
        UnrepresentedReason ??= reason;
        return this;
    }

    public SolutionSet WithNote(string? note)
    {
        Note ??= note;
        return this;
    }

    /// <summary>Returns a set that recorded unrepresented roots to Unevaluated status: with
    /// nothing represented there is no partial result, only an honest non-answer.</summary>
    internal SolutionSet MarkUnevaluated()
    {
        Status = SolveStatus.Unevaluated;
        return this;
    }

    public SolutionSet WithBudgetExceeded()
    {
        BudgetExceeded = true;
        if (Status == SolveStatus.Solved)
            Status = SolveStatus.BudgetExceeded;
        return this;
    }
}

/// <summary>Equation solving: structural dispatch over polynomials, rationals, and elementary compositions.</summary>
public static class Solvers
{
    public static SolutionSet Solve(Expr equation, Symbol x, ExprContext? ctx = null, SolveDomain domain = SolveDomain.Complex)
        => SolveCore(equation, x, ctx, domain, 0);

    private static SolutionSet SolveCore(Expr equation, Symbol x, ExprContext? ctx, SolveDomain domain, int depth)
    {
        ctx ??= Exprs.Current;
        if (depth > 8)
            return new SolutionSet(SolveStatus.Unevaluated, domain, "inverse nesting budget exceeded.");
        Expr f;
        if (equation is RelationExpr r && r.Op == RelOp.Eq)
            f = Exprs.Subtract(r.Left, r.Right);
        else if (equation is RelationExpr)
            return new SolutionSet(SolveStatus.Unevaluated, domain, "Inequality solving is not supported in v1.");
        else
            f = equation;

        // 1. polynomial in x
        if (Polynomial.TryFromExpr(f, ctx, new[] { x }, out var poly, out _))
            return SolvePolynomial(poly, x, ctx, domain);

        // 2. rational function: solve numerator == 0, keep denominator != 0 conditions
        if (RationalFunctions.TryRationalize(f, ctx, out var num, out var den) &&
            Polynomial.TryFromExpr(num, ctx, new[] { x }, out var np, out _) &&
            Polynomial.TryFromExpr(den, ctx, new[] { x }, out var dp, out _))
        {
            if (!np.IsZero)
            {
                var inner = SolvePolynomial(np, x, ctx, domain);
                if (inner.Status is SolveStatus.Unevaluated or SolveStatus.NoSolutions)
                    return inner;
                var denExpr = dp.ToExpr();
                var cond = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(denExpr, SymbolPredicate.NonZero));
                var set = new SolutionSet(SolveStatus.Solved, domain);
                int excluded = 0;
                foreach (var sol in inner.Solutions)
                {
                    // a numerator root that zeroes the denominator is NOT a solution: the
                    // condition is violated by the value itself, which no bracket of
                    // assumptions can express. Drop it structurally, not by contradiction.
                    if (ZeroesAt(denExpr, x, sol.Value, ctx))
                    {
                        excluded++;
                        continue;
                    }
                    var merged = Merge(sol.Conditions, cond);
                    if (merged.IsUnsatisfiable)
                    {
                        excluded++;
                        continue;
                    }
                    set.Solutions.Add(sol with { Conditions = merged });
                }
                if (excluded > 0 && set.Solutions.Count == 0)
                    return new SolutionSet(SolveStatus.NoSolutions, domain,
                        "every root violates the denominator condition.");
                set.InheritFrom(inner);
                SortSolutions(set);
                return set;
            }
            // the rationalized numerator vanishes identically: the rational function is zero
            // everywhere except at the denominator's poles — that is not an empty solution set
            return new SolutionSet(SolveStatus.Unevaluated, domain,
                "the numerator vanishes identically; the solution set is the domain minus the denominator's poles.");
        }

        // 3. linear in x with arbitrary (non-polynomial) coefficients
        if (Algebra.TryCoefficients(f, x, out var coeffs) && coeffs.Count <= 2)
        {
            var c1 = coeffs.Count == 2 ? coeffs[1] : Exprs.Zero;
            var c0 = coeffs[0];
            if (!(c1 is RationalConstantExpr r1 && r1.Value.IsZero))
            {
                var set = new SolutionSet(SolveStatus.Solved, domain);
                var root = Exprs.Divide(Exprs.Negate(c0), c1);
                var cond = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(c1, SymbolPredicate.NonZero));
                set.Solutions.Add(new Solution(root, cond) { Exactness = root.IsExact ? SolutionExactness.Exact : SolutionExactness.AlgebraicExact });
                return set;
            }
        }

        // 4. elementary compositions
        var elementary = SolveElementary(f, x, ctx, domain, depth);
        if (elementary is not null)
            return elementary;

        return new SolutionSet(SolveStatus.Unevaluated, domain, "No solver for this structure.");
    }

    /// <summary>True when <paramref name="e"/> evaluates to numeric zero at x = value. Used to
    /// exclude numerator roots that are poles of the original rational function.</summary>
    private static bool ZeroesAt(Expr e, Symbol x, Expr value, ExprContext ctx)
    {
        var at = Evaluation.Substitute(e, ctx, new Dictionary<Symbol, Expr> { [x] = value });
        if (Evaluation.ConstantToNum(at) is not { } n)
            return false;   // not decidable numerically: keep the value, carry the condition
        return NumOps.IsZero(n);
    }

    /// <summary>Deterministic solution ordering: ascending numeric value when every solution is
    /// numerically comparable, otherwise canonical-print ordinal (documented contract).</summary>
    private static void SortSolutions(SolutionSet set)
    {
        if (set.Solutions.Count < 2)
            return;
        var keys = new (Solution S, Rat? Num)[set.Solutions.Count];
        bool allNumeric = true;
        for (int i = 0; i < set.Solutions.Count; i++)
        {
            Rat? num = null;
            var v = set.Solutions[i].Value;
            if (v is RationalConstantExpr rc)
                num = rc.Value;
            else if (v is IntegerConstantExpr ic)
                num = Rat.From(ic.Value);
            if (num is null)
                allNumeric = false;
            keys[i] = (set.Solutions[i], num);
        }
        var ordered = allNumeric
            ? keys.OrderBy(k => k.Num).ThenBy(k => Printing.CanonicalPrint(k.S.Value), StringComparer.Ordinal)
            : keys.OrderBy(k => Printing.CanonicalPrint(k.S.Value), StringComparer.Ordinal);
        set.Solutions.Clear();
        set.Solutions.AddRange(ordered.Select(k => k.S));
    }

    /// <summary>Copies the completeness bookkeeping of a filtered inner result onto the outer set.</summary>
    private static void InheritFrom(this SolutionSet set, SolutionSet inner)
    {
        if (inner.UnrepresentedCount > 0)
            set.AsPartial(inner.UnrepresentedReason, inner.UnrepresentedCount);
        if (inner.BudgetExceeded)
            set.WithBudgetExceeded();
    }

    /// <summary>Merges condition sets; contradictory requirements make the branch
    /// unsatisfiable rather than silently keeping the first set.</summary>
    private static AssumptionSet Merge(AssumptionSet a, AssumptionSet b)
    {
        if (a.IsUnsatisfiable || b.IsUnsatisfiable)
            return AssumptionSet.Unsatisfiable;
        var result = a;
        foreach (var atom in b.Atoms)
        {
            try
            {
                result = result.Add(atom);
            }
            catch (AssumptionContradictionException)
            {
                return AssumptionSet.Unsatisfiable;
            }
        }
        return result;
    }

    /// <summary>Univariate polynomial solving: linear/quadratic/cubic formulas, RootOf otherwise.
    /// The domain is explicit: radical formulas (deg ≤ 3) are complex-capable and are filtered
    /// to real roots under <see cref="SolveDomain.Real"/>; deg ≥ 4 factors use the real-only
    /// RootOf, so a Complex-domain request that needs non-real algebraic roots is reported
    /// Partial (when some real roots are representable) or Unevaluated (when none are) — never
    /// as a complete solution set.</summary>
    public static SolutionSet SolvePolynomial(Polynomial poly, Symbol x, ExprContext ctx, SolveDomain domain = SolveDomain.Complex)
    {
        if (poly.IsZero)
            return new SolutionSet(SolveStatus.Unevaluated, domain, "0 = 0: every value is a solution.");
        if (poly.TotalDegree <= 0)
            return new SolutionSet(SolveStatus.NoSolutions, domain, "the equation reduces to a nonzero constant.");
        var factors = Factoring.FactorPoly(poly);
        var set = new SolutionSet(SolveStatus.Solved, domain);
        int unrepresented = 0;
        string? reason = null;
        foreach (var (factor, mult) in factors.Factors)
        {
            int deg = factor.TotalDegree;
            Expr[]? roots;
            switch (deg)
            {
                case 1:
                    roots = new[] { LinearRoot(factor) };
                    break;
                case 2:
                    roots = QuadraticRoots(factor, domain);
                    break;
                case 3:
                    roots = CubicRoots(factor, ctx, domain);
                    break;
                default:
                {
                    // degree >= 4: RootOf over the exact Sturm count of real roots — never deg
                    // assumed-real roots (that invented nonexistent roots and shifted indices)
                    var high = HighDegreeRoots(factor, domain);
                    if (high is null)
                    {
                        // no representable root for this factor. Earlier factors may already have
                        // produced exact roots: report them honestly as Partial rather than
                        // discarding them or, worse, presenting them as complete.
                        unrepresented += deg;
                        reason ??= "complex algebraic roots not supported (RootOf is real-only in v1).";
                        continue;
                    }
                    var highValue = high.Value;
                    roots = highValue.Roots;
                    unrepresented += highValue.Missing;
                    reason ??= highValue.Reason;
                    break;
                }
            }
            if (roots.Length == 0)
                continue;
            foreach (var r in roots)
                set.Solutions.Add(new Solution(r, AssumptionSet.Empty)
                {
                    Multiplicity = mult,
                    Exactness = ExactnessOfValue(r),
                });
        }

        if (unrepresented > 0)
        {
            if (set.Solutions.Count == 0 && set.Families.Count == 0)
            {
                // nothing is representable: there is no partial result to report. An empty set
                // is not a partial answer — it is an honest "we cannot represent this".
                var none = new SolutionSet(SolveStatus.Unevaluated, domain, reason);
                none.AsPartial(reason, unrepresented);
                none.MarkUnevaluated();
                return none;
            }
            set.AsPartial(reason, unrepresented);
        }
        else if (set.Solutions.Count == 0)
        {
            return new SolutionSet(SolveStatus.NoSolutions, domain,
                domain == SolveDomain.Real ? "no real solutions" : "no solutions");
        }
        SortSolutions(set);
        return set;
    }

    /// <summary>Exactness of a returned value: a rational/integer constant is Exact; anything
    /// else the solver produces is an exact algebraic value. Radicals that fold to rationals
    /// (e.g. sqrt(4) = 2) are therefore reported as Exact automatically.</summary>
    private static SolutionExactness ExactnessOfValue(Expr e) =>
        e is RationalConstantExpr or IntegerConstantExpr ? SolutionExactness.Exact : SolutionExactness.AlgebraicExact;

    /// <summary>Real roots of a deg ≥ 4 factor. Returns null when NO root of the factor is
    /// representable (nothing to add); otherwise the representable roots plus how many roots of
    /// the requested domain are missing and why.</summary>
    private static (Expr[] Roots, int Missing, string? Reason)? HighDegreeRoots(Polynomial p, SolveDomain domain)
    {
        int deg = p.TotalDegree;
        int count = Roots.RealRootCount(p);
        if (count == 0)
            return domain == SolveDomain.Complex ? null : (Array.Empty<Expr>(), 0, null);
        if (domain == SolveDomain.Real)
        {
            // over the reals the count real roots ARE the complete solution set
            return (Enumerable.Range(0, count).Select(i => (Expr)Exprs.RootOf(p, i)).ToArray(), 0, null);
        }
        // Complex domain: only the real roots are representable
        var roots = Enumerable.Range(0, count).Select(i => (Expr)Exprs.RootOf(p, i)).ToArray();
        return (roots, deg - count, "complex algebraic roots not supported (RootOf is real-only in v1).");
    }

    private static Expr LinearRoot(Polynomial p)
    {
        var c0 = p.ConstantTerm;
        var c1 = p.LeadingCoefficient(MonomialOrder.Lex);
        return Exprs.Divide(Exprs.Rational(Rat.Negate(c0)), Exprs.Rational(c1));
    }

    private static Expr[] QuadraticRoots(Polynomial p, SolveDomain domain)
    {
        // a x^2 + b x + c
        var c = p.ConstantTerm;
        Rat b = Rat.Zero, a = Rat.Zero;
        foreach (var (m, coef) in p.Terms)
        {
            int d = m.TotalDegree;
            if (d == 1) b = coef;
            else if (d == 2) a = coef;
        }
        var disc = b * b - Rat.FromLong(4L) * a * c;
        if (domain == SolveDomain.Real && disc < Rat.Zero)
            return Array.Empty<Expr>();   // no real roots
        var sqrtDisc = Exprs.Power(Exprs.Rational(disc), Exprs.Rational(1, 2));
        var twoA = Exprs.Rational(Rat.FromLong(2L) * a);
        var negB = Exprs.Rational(Rat.Negate(b));
        return new[]
        {
            Exprs.Divide(Exprs.Add(negB, sqrtDisc), twoA),
            Exprs.Divide(Exprs.Subtract(negB, sqrtDisc), twoA),
        };
    }

    private static Expr[] CubicRoots(Polynomial p, ExprContext ctx, SolveDomain domain)
    {
        // normalize to monic x^3 + A x^2 + B x + C
        Rat A = Rat.Zero, B = Rat.Zero, C = Rat.Zero;
        var lc = p.LeadingCoefficient(MonomialOrder.Lex);
        foreach (var (m, coef) in p.Terms)
        {
            var cn = coef / lc;
            int d = m.TotalDegree;
            if (d == 2) A = cn;
            else if (d == 1) B = cn;
            else if (d == 0) C = cn;
        }
        // depress: x = t - A/3  →  t^3 + P t + Q
        var a3 = A / Rat.FromLong(3L);
        var P = B - A * A / Rat.FromLong(3L);
        var Q = Rat.FromLong(2L) * A * A * A / Rat.FromLong(27L) - A * B / Rat.FromLong(3L) + C;
        var disc = Q * Q / Rat.FromLong(4L) + P * P * P / Rat.FromLong(27L);

        Expr[] ts;
        if (disc >= Rat.Zero)
        {
            // one real root via Cardano. The pair (u, v) is BRANCH-COUPLED: u is the principal
            // cube root of the positive radicand −Q/2 + √disc, and v is chosen so that
            // u·v = −P/3 exactly. Choosing both roots independently with principal branches
            // breaks the coupling for negative radicands and yields roots that do not satisfy
            // the polynomial.
            var halfQ = Rat.Negate(Q) / Rat.FromLong(2L);
            var sqrtD = Exprs.Power(Exprs.Rational(disc), Exprs.Rational(1, 2));
            var r1 = Exprs.Add(Exprs.Rational(halfQ), sqrtD);
            var u = Exprs.Power(r1, Exprs.Rational(1, 3));
            var v = P.IsZero
                ? Exprs.Zero
                : Exprs.Multiply(Exprs.Rational(Rat.Negate(P) / Rat.FromLong(3L)), Exprs.Power(r1, Exprs.Rational(-1, 3)));
            // omega = -1/2 + i*sqrt(3)/2
            var omega = Exprs.Add(
                Exprs.Rational(-1, 2),
                Exprs.Multiply(Exprs.Divide(Exprs.Power(Exprs.Rational(3L), Exprs.Rational(1, 2)), Exprs.Rational(2L)), Exprs.I));
            var omega2 = Exprs.Subtract(
                Exprs.Rational(-1, 2),
                Exprs.Multiply(Exprs.Divide(Exprs.Power(Exprs.Rational(3L), Exprs.Rational(1, 2)), Exprs.Rational(2L)), Exprs.I));
            if (domain == SolveDomain.Real)
            {
                // disc > 0: exactly one real root (the Cardano real branch); disc == 0: all
                // three roots are real and coincide pairwise
                ts = disc > Rat.Zero
                    ? new[] { Exprs.Add(u, v) }
                    : new[]
                    {
                        Exprs.Add(u, v),
                        Exprs.Add(Exprs.Multiply(omega, u), Exprs.Multiply(omega2, v)),
                        Exprs.Add(Exprs.Multiply(omega2, u), Exprs.Multiply(omega, v)),
                    };
            }
            else
            {
                ts = new[]
                {
                    Exprs.Add(u, v),
                    Exprs.Add(Exprs.Multiply(omega, u), Exprs.Multiply(omega2, v)),
                    Exprs.Add(Exprs.Multiply(omega2, u), Exprs.Multiply(omega, v)),
                };
            }
        }
        else
        {
            // three real roots via trigonometry
            var m = Exprs.Multiply(Exprs.Rational(Rat.FromLong(2L)), ExprsSqrt(Exprs.Rational(Rat.Negate(P) / Rat.FromLong(3L))));
            var theta = Exprs.Divide(
                Exprs.Function(ctx.Function("acos"), Exprs.Divide(
                    Exprs.Multiply(Exprs.Rational(Rat.FromLong(3L) * Q), ExprsSqrt(Exprs.Rational(Rat.FromLong(-3L) / P))),
                    Exprs.Multiply(Exprs.Rational(Rat.FromLong(2L) * P), Exprs.Rational(Rat.One)))),
                Exprs.Rational(3L));
            ts = Enumerable.Range(0, 3).Select(k =>
            {
                var angle = Exprs.Add(theta, Exprs.Divide(Exprs.Multiply(Exprs.Rational(2L * k), Exprs.Pi), Exprs.Rational(3L)));
                return Exprs.Multiply(m, Exprs.Function(ctx.Function("cos"), angle));
            }).ToArray();
        }

        var shift = Exprs.Rational(Rat.Negate(a3));
        return ts.Select(t => Exprs.Add(t, shift)).ToArray();
    }

    private static Expr ExprsSqrt(Expr e) => Exprs.Power(e, Exprs.Rational(1, 2));

    /// <summary>Solves exp(u)=c, log(u)=c, u^n=c, sin/cos/tan(u)=c via registry inverses (principal branches).</summary>
    private static SolutionSet? SolveElementary(Expr f, Symbol x, ExprContext ctx, SolveDomain domain, int depth = 0)
    {
        // bare h(u) = 0 with h invertible (e.g. sin(x) = 0)
        if (f is (FunctionExpr or PowerExpr))
        {
            var direct = SolveInverse(f, Exprs.Zero, x, ctx, domain, depth);
            if (direct is not null)
                return direct;
        }

        // f is of the form h(u) + c with h invertible and c a folded constant
        if (f is AddExpr add)
        {
            foreach (var term in add.Terms)
            {
                Expr rhs;
                if (term is RationalConstantExpr rc0)
                    rhs = Exprs.Rational(Rat.Negate(rc0.Value));   // h(u) + c = 0  ⇒  h(u) = -c
                else if (term is MultiplyExpr m && m.Factors.Length == 2 &&
                         m.Factors[0] is RationalConstantExpr rc && rc.Value.IsMinusOne)
                    rhs = m.Factors[1];
                else
                    continue;
                var u = Exprs.Subtract(f, term);   // h(u-part)
                if (u is not (FunctionExpr or PowerExpr))
                    continue;
                var result = SolveInverse(u, rhs, x, ctx, domain, depth);
                if (result is not null)
                    return result;
            }
        }
        return null;
    }

    private static SolutionSet? SolveInverse(Expr h, Expr c, Symbol x, ExprContext ctx, SolveDomain domain, int depth = 0)
    {
        // nested inverses grow one level per radical; refuse rather than overflow the stack
        if (depth > 8)
            return new SolutionSet(SolveStatus.Unevaluated, domain, "inverse nesting budget exceeded.");

        Expr? u = null;
        IReadOnlyList<Expr>? inverses = null;
        switch (h)
        {
            case FunctionExpr f when f.Arguments.Length == 1:
            {
                u = f.Arguments[0];
                switch (f.Function.Name)
                {
                    case "sin" or "cos" or "tan":
                        return SolvePeriodic(f.Function.Name, u, c, x, ctx, domain);
                    case "exp":
                        // exp(u) = c has no solution for c = 0; over the reals it needs c > 0,
                        // over the complex field log is defined for every c != 0 (principal branch)
                        if (Evaluation.ConstantToNum(c) is { } cv)
                        {
                            if (NumOps.IsZero(cv))
                                return new SolutionSet(SolveStatus.NoSolutions, domain, "exp(u) = 0 has no solution.");
                            if (domain == SolveDomain.Real && NumOps.Compare(cv, NumOps.FromLong(0L)) < 0)
                                return new SolutionSet(SolveStatus.NoSolutions, domain, "exp(u) = c has no real solution for c < 0.");
                        }
                        inverses = new[] { Exprs.Function(ctx.Function("log"), c) };
                        break;
                    case "log":
                        inverses = new[] { Exprs.Function(ctx.Function("exp"), c) };
                        break;
                    default:
                        return null;
                }
                break;
            }
            case PowerExpr p when p.Exponent is RationalConstantExpr pe && pe.Value != Rat.One:
            {
                u = p.Base;
                var n = pe.Value;
                if (n.IsZero)
                    return null;
                // u^n = c has exactly n solutions over the complex field. Emitting only the
                // principal root would under-report the solution set, so every root of unity is
                // generated whenever the exponent is a small positive integer.
                var principal = Exprs.Power(c, Exprs.Rational(Rat.One / n));
                if (n.IsInteger)
                {
                    var ni = n.ToInteger();
                    if (Int.IsPositive(ni) && int.TryParse(ni.ToString(), out int count) && count is >= 1 and <= 16)
                    {
                        inverses = count == 1
                            ? new[] { principal }
                            : UnityRoots(count, ctx).Select(z => (Expr)Exprs.Multiply(principal, z)).ToArray();
                        break;
                    }
                }
                inverses = new[] { principal };
                break;
            }
        }
        if (u is null || inverses is null)
            return null;

        // u is the part containing x: solve u == inverse for x recursively, once per branch
        var set = new SolutionSet(SolveStatus.Solved, domain);
        foreach (var inverse in inverses)
        {
            var inner = SolveCore(Exprs.Subtract(u, inverse), x, ctx, domain, depth + 1);
            switch (inner.Status)
            {
                case SolveStatus.NoSolutions:
                    continue;
                case SolveStatus.Solved:
                    set.Solutions.AddRange(inner.Solutions);
                    set.Families.AddRange(inner.Families);
                    break;
                default:
                    return inner;
            }
        }
        if (set.Solutions.Count == 0 && set.Families.Count == 0)
            return new SolutionSet(SolveStatus.NoSolutions, domain, "no branch produced a solution.");
        SortSolutions(set);
        return set;
    }

    /// <summary>The n-th roots of unity as exact expressions, in ascending k order
    /// (z_k = e^(2πik/n)); small n use exact radicals, larger n the exact trigonometric form.</summary>
    private static IReadOnlyList<Expr> UnityRoots(int n, ExprContext ctx)
    {
        switch (n)
        {
            case 1:
                return new[] { Exprs.One };
            case 2:
                return new[] { Exprs.One, Exprs.Rational(Rat.MinusOne) };
            case 4:
                return new[] { Exprs.One, Exprs.I, Exprs.Rational(Rat.MinusOne), Exprs.Negate(Exprs.I) };
            case 3:
            {
                var sqrt3 = Exprs.Power(Exprs.Rational(3L), Exprs.Rational(1, 2));
                var half = Exprs.Divide(sqrt3, Exprs.Rational(2L));
                var omega = Exprs.Add(Exprs.Rational(-1, 2), Exprs.Multiply(half, Exprs.I));
                var omega2 = Exprs.Subtract(Exprs.Rational(-1, 2), Exprs.Multiply(half, Exprs.I));
                return new[] { Exprs.One, omega, omega2 };
            }
            default:
            {
                var twoPi = Exprs.Multiply(2, Exprs.Pi);
                var list = new List<Expr>(n);
                for (int k = 0; k < n; k++)
                {
                    if (k == 0)
                    {
                        list.Add(Exprs.One);
                        continue;
                    }
                    if (n % 2 == 0 && k == n / 2)
                    {
                        list.Add(Exprs.Rational(Rat.MinusOne));
                        continue;
                    }
                    var angle = Exprs.Divide(Exprs.Multiply(k, twoPi), Exprs.Rational(n));
                    list.Add(Exprs.Add(
                        Exprs.Function(ctx.Function("cos"), angle),
                        Exprs.Multiply(Exprs.I, Exprs.Function(ctx.Function("sin"), angle))));
                }
                return list;
            }
        }
    }

    /// <summary>
    /// Periodic inverses produce parametric families: sin/cos have period 2π, tan has π.
    /// Families are returned only when the inner argument is the bare solved symbol (nested
    /// arguments would need nested families).
    /// </summary>
    private static SolutionSet SolvePeriodic(string name, Expr u, Expr c, Symbol x, ExprContext ctx, SolveDomain domain)
    {
        if (u is not SymbolExpr)
            return new SolutionSet(SolveStatus.Unevaluated, domain,
                $"{name}(u) = c with composite u is not supported in v1.");
        if (Evaluation.ConstantToNum(c) is not { } cv)
            return new SolutionSet(SolveStatus.Unevaluated, domain,
                $"{name}(x) = c with symbolic c is not supported in v1.");

        var k = FreshParameter(ctx, c, u);
        Expr principal = name switch
        {
            "sin" => Exprs.Function(ctx.Function("asin"), c),
            "cos" => Exprs.Function(ctx.Function("acos"), c),
            _ => Exprs.Function(ctx.Function("atan"), c),
        };
        var twoPi = Exprs.Multiply(2, Exprs.Pi);
        var pi = Exprs.Pi;
        var kExpr = Exprs.Symbol(k);
        var set = new SolutionSet(SolveStatus.Solved, domain);

        if (name == "tan")
        {
            set.Families.Add(new SolutionFamily(
                Exprs.Add(principal, Exprs.Multiply(kExpr, pi)), k, pi, ParameterDomain.Integers));
            return set;
        }

        // sin/cos: classify |c| against 1
        bool cZero = NumOps.IsZero(cv);
        int cmp1 = NumOps.Compare(NumOps.Abs(cv, ctx), NumOps.FromLong(1L));
        if (cmp1 > 0)
        {
            // |c| > 1: there are no REAL solutions, but sin/cos are surjective onto the complex
            // plane. Under the Complex domain the solutions exist and are not representable here.
            return domain == SolveDomain.Real
                ? new SolutionSet(SolveStatus.NoSolutions, domain, $"{name}(x) = c has no real solution for |c| > 1.")
                : new SolutionSet(SolveStatus.Unevaluated, domain,
                    $"{name}(x) = c has complex solutions for |c| > 1 that are not representable in v1.");
        }

        if (cZero)
        {
            // sin(x) = 0 → x = kπ; cos(x) = 0 → x = π/2 + kπ
            var template = name == "sin"
                ? Exprs.Multiply(kExpr, pi)
                : Exprs.Add(Exprs.Divide(pi, 2), Exprs.Multiply(kExpr, pi));
            set.Families.Add(new SolutionFamily(template, k, pi, ParameterDomain.Integers));
            return set;
        }

        if (cmp1 == 0)
        {
            // |c| = 1: the two branches merge into one family
            set.Families.Add(new SolutionFamily(
                Exprs.Add(principal, Exprs.Multiply(kExpr, twoPi)), k, twoPi, ParameterDomain.Integers));
            return set;
        }

        // |c| < 1: two families, one per branch
        set.Families.Add(new SolutionFamily(
            Exprs.Add(principal, Exprs.Multiply(kExpr, twoPi)), k, twoPi, ParameterDomain.Integers));
        set.Families.Add(new SolutionFamily(
            name == "sin"
                ? Exprs.Add(Exprs.Subtract(pi, principal), Exprs.Multiply(kExpr, twoPi))
                : Exprs.Add(Exprs.Negate(principal), Exprs.Multiply(kExpr, twoPi)),
            k, twoPi, ParameterDomain.Integers));
        return set;
    }

    /// <summary>Chooses a family parameter symbol not colliding with the equation's symbols.</summary>
    private static Symbol FreshParameter(ExprContext ctx, params Expr[] avoid)
    {
        var used = new HashSet<string>(StringComparer.Ordinal);
        foreach (var e in avoid)
            CollectNames(e, used);
        string name = "k";
        int i = 1;
        while (used.Contains(name))
            name = "k" + i++;
        return ctx.Symbol(name);
    }

    private static void CollectNames(Expr e, HashSet<string> into)
    {
        switch (e)
        {
            case SymbolExpr s: into.Add(s.Symbol.Name); break;
            case AddExpr a: foreach (var t in a.Terms) CollectNames(t, into); break;
            case MultiplyExpr m: foreach (var f in m.Factors) CollectNames(f, into); break;
            case PowerExpr p: CollectNames(p.Base, into); CollectNames(p.Exponent, into); break;
            case FunctionExpr f: foreach (var arg in f.Arguments) CollectNames(arg, into); break;
        }
    }
}

/// <summary>Structured system solution: one assignment per solution map.</summary>
public sealed record SystemSolution(IReadOnlyDictionary<Symbol, Expr> Assignment, AssumptionSet Conditions);

public sealed class SystemSolveResult
{
    public List<SystemSolution> Solutions { get; } = new();
    public string? Note { get; }

    /// <summary>True when the enumeration hit the solution cap: the returned list is a subset.</summary>
    public bool Truncated { get; internal set; }

    /// <summary>The cap that was hit when <see cref="Truncated"/> is set.</summary>
    public int Limit { get; internal set; }

    /// <summary>The field the system is solved over (Complex in v1).</summary>
    public SolveDomain Domain => SolveDomain.Complex;

    /// <summary>Honest disposition: a truncated enumeration is Partial, never Solved.</summary>
    public SolveStatus Status
    {
        get
        {
            if (Truncated)
                return SolveStatus.Partial;
            if (Solutions.Count > 0)
                return SolveStatus.Solved;
            return Note is null ? SolveStatus.NoSolutions : SolveStatus.Unevaluated;
        }
    }

    public Completeness Complete => Status switch
    {
        SolveStatus.Solved => Completeness.Complete,
        SolveStatus.Partial => Completeness.Partial,
        _ => Completeness.Unknown,
    };

    public SystemSolveResult(string? note = null) => Note = note;
}

/// <summary>
/// Polynomial systems: Gröbner-basis elimination into a triangular system, then univariate
/// solving and recursive back-substitution. Solutions are returned as variable assignments
/// and are verified where feasible.
/// </summary>
public static class SystemSolvers
{
    private const int MaxSolutions = 128;

    public static SystemSolveResult Solve(
        IReadOnlyList<Expr> equations, IReadOnlyList<Symbol> variables, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        var polys = new List<Expr>();
        foreach (var eq in equations)
        {
            var f = eq is RelationExpr r && r.Op == RelOp.Eq ? Exprs.Subtract(r.Left, r.Right) : eq;
            if (!Polynomial.TryFromExpr(f, ctx, variables.ToArray(), out _, out _))
                return new SystemSolveResult("Only polynomial systems are supported in v1.");
            polys.Add(f);
        }
        var result = new SystemSolveResult();
        SolveRecursive(polys, variables.ToArray(), ctx, AssumptionSet.Empty, new Dictionary<Symbol, Expr>(), result, 0);
        return result;
    }

    private static void SolveRecursive(
        IReadOnlyList<Expr> polys, Symbol[] vars, ExprContext ctx,
        AssumptionSet conditions, Dictionary<Symbol, Expr> partial, SystemSolveResult result, int depth)
    {
        if (result.Solutions.Count >= MaxSolutions)
        {
            result.Truncated = true;
            result.Limit = MaxSolutions;
            return;
        }
        int n = vars.Length;
        if (n == 0)
        {
            // no variables left: every remaining equation must vanish identically
            foreach (var p in polys)
            {
                if (Evaluation.ConstantToNum(p) is { } cv && !NumOps.IsZero(cv))
                    return;   // an inconsistent constant remains
            }
            result.Solutions.Add(new SystemSolution(new Dictionary<Symbol, Expr>(partial), conditions));
            return;
        }
        if (n == 1)
        {
            var x = vars[0];
            if (!Polynomial.TryFromExpr(polys[0], ctx, new[] { x }, out var poly, out _))
                return;
            var set = Solvers.SolvePolynomial(poly, x, ctx);
            foreach (var sol in set.Solutions)
            {
                if (!SatisfiesAll(sol.Value, polys.Skip(1), x, ctx))
                    continue;
                var assignment = new Dictionary<Symbol, Expr>(partial) { [x] = sol.Value };
                result.Solutions.Add(new SystemSolution(assignment, conditions));
                if (result.Solutions.Count >= MaxSolutions)
                {
                    result.Truncated = true;
                    result.Limit = MaxSolutions;
                    return;
                }
            }
            return;
        }

        // lex Gröbner basis; eliminate all but the last variable. Only expected typed failures
        // degrade to "not enumerated" — defects surface instead of being masked as no-solution.
        var last = vars[n - 1];
        List<Polynomial> basis;
        try
        {
            var gens = polys.Select(p => Polynomial.FromExpr(p, ctx, vars)).ToList();
            basis = Groebner.Basis(gens, MonomialOrder.Lex);
        }
        catch (NotPolynomialException)
        {
            return;
        }
        var eliminated = basis
            .Where(p => FreeOfVariables(p, vars[..^1]))
            .OrderBy(p => p.TotalDegree)
            .ToList();
        if (eliminated.Count == 0)
            return;   // no elimination polynomial: not enumerated in v1
        if (eliminated[0].IsOne)
            return;   // ideal is the whole ring: no solutions
        var univariate = eliminated[0];
        if (!Polynomial.TryFromExpr(univariate.ToExpr(), ctx, new[] { last }, out var lastPoly, out _))
            return;
        var lastSet = Solvers.SolvePolynomial(lastPoly, last, ctx);
        foreach (var sol in lastSet.Solutions)
        {
            var substituted = polys
                .Select(p => Evaluation.Substitute(p, ctx, new Dictionary<Symbol, Expr> { [last] = sol.Value }))
                .ToList();
            var nextPartial = new Dictionary<Symbol, Expr>(partial) { [last] = sol.Value };
            SolveRecursive(substituted, vars[..^1], ctx, conditions, nextPartial, result, depth + 1);
            if (result.Solutions.Count >= MaxSolutions)
            {
                result.Truncated = true;
                result.Limit = MaxSolutions;
                return;
            }
        }
    }

    /// <summary>True when the solution satisfies every remaining equation (symbolically where the
    /// simplifier can prove it, numerically at high precision otherwise).</summary>
    private static bool SatisfiesAll(Expr value, IEnumerable<Expr> equations, Symbol x, ExprContext ctx)
    {
        using var scope = global::Lovelace.Real.Real.WithPrecision(40, 20);
        var tolerance = NumOps.FromReal(global::Lovelace.Real.Real.Parse("0." + new string('0', 24) + "1", null));
        foreach (var eq in equations)
        {
            var at = Evaluation.Substitute(eq, ctx, new Dictionary<Symbol, Expr> { [x] = value });
            if (Evaluation.ConstantToNum(at) is { } cv)
            {
                if (!NumOps.IsZero(cv))
                    return false;
                continue;
            }
            var expanded = Algebra.Expand(at, ctx);
            if (expanded is RationalConstantExpr rc && rc.Value.IsZero)
                continue;
            try
            {
                var residual = Evaluation.EvaluateToNum(at, ctx, new Dictionary<Symbol, Num>());
                if (NumOps.Compare(NumOps.Abs(residual, ctx), tolerance) >= 0)
                    return false;
            }
            catch (EvaluationException)
            {
                return false;   // not numerically checkable: refuse rather than guess
            }
        }
        return true;
    }

    private static bool FreeOfVariables(Polynomial p, Symbol[] vars)
    {
        foreach (var v in vars)
        {
            int idx = p.Order.IndexOf(v.Name);
            if (idx < 0)
                continue;
            foreach (var (m, _) in p.Terms)
            {
                if (idx < m.Exps.Length && m.Exps[idx] != 0)
                    return false;
            }
        }
        return true;
    }
}

/// <summary>
/// Rigorous real-root isolation and evaluation for RootOf: Sturm sequences with exact
/// rational coefficient arithmetic and bisection over rational intervals. No sampling grids,
/// no sign-scan heuristics — closely spaced and multiple roots cannot be missed or reordered.
/// RootOf(p, i) denotes the i-th real root of the square-free part of p in ascending order.
/// </summary>
public static class Roots
{
    /// <summary>Sturm sequence of the square-free part of p (exact rational coefficients).</summary>
    public static List<Polynomial> SturmSequence(Polynomial p)
    {
        var seq = new List<Polynomial>();
        var sf = Polynomial.SquareFreePart(p);
        if (sf.IsZero)
            return seq;
        seq.Add(Monic(sf));
        var d = sf.Derivative(0);
        if (!d.IsZero)
            seq.Add(Monic(d));
        while (seq[^1].TotalDegree > 0)
        {
            var (_, rem) = seq[^2].DivRem(seq[^1], MonomialOrder.Lex);
            if (rem.IsZero)
                break;
            seq.Add(Monic(Polynomial.Negate(rem)));
        }
        return seq;
    }

    /// <summary>Scales to a leading coefficient of ±1 by a POSITIVE factor. Sturm sequences
    /// are invariant under positive scaling only — dividing by a negative leading coefficient
    /// would flip signs and corrupt the variation counts.</summary>
    private static Polynomial Monic(Polynomial p)
    {
        if (p.IsZero)
            return p;
        var lc = p.LeadingCoefficient(MonomialOrder.Lex);
        if (lc.IsOne)
            return p;
        var scale = lc.IsNegative ? Rat.Negate(lc) : lc;
        var r = new Polynomial(p.Order);
        foreach (var (m, c) in p.Terms)
            r = Polynomial.Add(r, Polynomial.FromMonomial(p.Order, m, c / scale));
        return r;
    }

    /// <summary>Number of distinct real roots (Sturm's theorem, exact).</summary>
    public static int RealRootCount(Polynomial p)
    {
        if (p.IsZero)
            return 0;
        var seq = SturmSequence(p);
        Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
        return SignVariationsAtInfinity(seq, plus: false) - SignVariationsAtInfinity(seq, plus: true);
    }

    /// <summary>Cauchy bound: every real root satisfies |x| &lt; 1 + max |a_i / a_n|.</summary>
    public static Rat CauchyBound(Polynomial p)
    {
        var an = p.LeadingCoefficient(MonomialOrder.Lex);
        Rat m = Rat.Zero;
        foreach (var (mon, c) in p.Terms)
        {
            if (mon.TotalDegree < p.TotalDegree)
            {
                var t = Rat.Abs(c / an);
                if (t > m)
                    m = t;
            }
        }
        return m + Rat.One;
    }

    private static int SignVariationsAt(List<Polynomial> seq, Rat x)
    {
        int variations = 0;
        int prev = 0;
        for (int i = 0; i < seq.Count; i++)
        {
            var v = seq[i].EvaluateAt(new[] { x });
            int s = v.IsZero ? 0 : (v.IsNegative ? -1 : 1);
            if (s == 0 && i == 0)
            {
                // x is a root of p0 (after square-free normalization only p0 can vanish at a
                // root): use the left-hand sign −sign(p1(x)) so variation counts behave as if
                // x were evaluated slightly to the left — Sturm's theorem needs non-root points
                var p1v = seq.Count > 1 ? seq[1].EvaluateAt(new[] { x }) : Rat.One;
                s = p1v.IsZero ? 0 : (p1v.IsNegative ? 1 : -1);
            }
            if (s != 0)
            {
                if (prev != 0 && s != prev)
                    variations++;
                prev = s;
            }
        }
        return variations;
    }

    private static int SignVariationsAtInfinity(List<Polynomial> seq, bool plus)
    {
        int variations = 0;
        int prev = 0;
        foreach (var p in seq)
        {
            if (p.IsZero)
                continue;
            var lc = p.LeadingCoefficient(MonomialOrder.Lex);
            int s = lc.IsNegative ? -1 : 1;
            if (!plus && p.TotalDegree % 2 == 1)
                s = -s;
            if (prev != 0 && s != prev)
                variations++;
            prev = s;
        }
        return variations;
    }

    private static bool IsRootAt(List<Polynomial> seq, Rat x) => seq[0].EvaluateAt(new[] { x }).IsZero;

    /// <summary>Distinct real roots in (−∞, x], exact — including the endpoint when x is a root.
    /// Standard Sturm count for (a, b) plus the endpoint membership; consistent even when the
    /// bisection endpoints land exactly on roots.</summary>
    private static int CountRootsLE(List<Polynomial> seq, Rat x) =>
        SignVariationsAtInfinity(seq, plus: false)
        - SignVariationsAt(seq, x)
        + (IsRootAt(seq, x) ? 1 : 0);

    private static (Rat Lo, Rat Hi) IsolateIndex(List<Polynomial> seq, Rat bound, int k, int bits)
    {
        Rat a = Rat.Negate(bound);
        Rat b = bound;
        // invariant: the k-th root (0-based globally) lies in (a, b]; each step halves the width
        for (int i = 0; i < bits + 16; i++)
        {
            Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
            var mid = (a + b) / Rat.FromLong(2L);
            int offset = CountRootsLE(seq, a);                       // roots already passed
            int inHalfOpen = CountRootsLE(seq, mid) - offset;        // roots in (a, mid]
            if (inHalfOpen > k - offset)
                b = mid;
            else
                a = mid;
        }
        return (a, b);
    }

    /// <summary>Evaluates RootOf(p, i) to about <paramref name="digits"/> decimal digits.</summary>
    public static Rl N(Expr rootOf, long digits, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        if (rootOf is not RootOfExpr r)
            throw new InvalidOperationException("N() requires a RootOf expression.");
        var p = r.DefiningPolynomial;
        var seq = SturmSequence(p);
        int count = SignVariationsAtInfinity(seq, plus: false) - SignVariationsAtInfinity(seq, plus: true);
        if (r.RootIndex < 0 || r.RootIndex >= count)
            throw new InvalidOperationException(
                $"RootOf index {r.RootIndex} is out of range: the polynomial has {count} real root(s).");

        // isolate the index-th root by exact rational bisection, then convert the dyadic
        // midpoint (a finite decimal) to a Real at the requested precision
        int bits = (int)Math.Min(int.MaxValue / 2, digits * 4 + 16);
        var (lo, hi) = IsolateIndex(seq, CauchyBound(p), r.RootIndex, bits);
        var mid = (lo + hi) / Rat.FromLong(2L);
        return RationalReal.ToReal(mid, (int)Math.Min(digits + 2, int.MaxValue));
    }
}
