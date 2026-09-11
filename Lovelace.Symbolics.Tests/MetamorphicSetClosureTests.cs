using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Lovelace.Symbolics.Rewriting;
using Xunit;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 26: the two §116 metamorphic properties that had no real test.
/// <list type="number">
/// <item><b>A·A⁻¹ = I symbolically</b>, under the det ≠ 0 condition the kernel emits. The raw
/// product does NOT simplify to the identity (the kernel does not put sums over a common
/// denominator), so the assertion clears the denominators the kernel itself reports: with every
/// emitted entry written as adj/det, the claim A·A⁻¹ = I is exactly the polynomial identity
/// Σ_k a_ik·adj_kj = δ_ij·det — checked with exact polynomial arithmetic, never by substitution.</item>
/// <item><b>cancel preserves value off poles</b>: for a corpus with known removable singularities
/// the reduced form agrees with the original at sampled rational points that are machine-verified
/// to be in the original's domain, and at the pole the original is undefined
/// (<see cref="EvaluationException"/>) while the reduced form is defined — which is exactly why the
/// kernel's rewrite path carries a NonZero side condition.</item>
/// </list>
/// Both capabilities were already implemented; these tests are new coverage, not a failing-first
/// fix. Falsifiability is demonstrated explicitly in
/// <see cref="InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck"/>.
/// </summary>
public class MetamorphicSetClosureTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    // ==================================================================
    // Gap 1 — A·A⁻¹ = I, symbolically, under det ≠ 0
    // ==================================================================

    private sealed record MatrixCase(string Name, Expr[][] Entries, Expr ExpectedDet);

    private static MatrixCase[] InverseCorpus(ExprContext ctx)
    {
        var a = ctx.Symbol("a");
        var b = ctx.Symbol("b");
        var c = ctx.Symbol("c");
        var d = ctx.Symbol("d");
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var z = ctx.Symbol("z");
        return new[]
        {
            // 2x2 general: the fully symbolic case
            new MatrixCase("2x2 general [[a,b],[c,d]]",
                new[] { new Expr[] { a, b }, new Expr[] { c, d } },
                Exprs.Subtract(Exprs.Multiply(a, d), Exprs.Multiply(b, c))),
            // 2x2 with a genuinely symbolic determinant x^2 - y (the matrix the old numeric smoke test used)
            new MatrixCase("2x2 [[x,1],[y,x]]",
                new[] { new Expr[] { x, Exprs.One }, new Expr[] { y, x } },
                Exprs.Subtract(Exprs.Power(x, 2), y)),
            // 3x3 tridiagonal: det = x*y*z - x - z
            new MatrixCase("3x3 tridiagonal",
                new[]
                {
                    new Expr[] { x, Exprs.One, Exprs.Zero },
                    new Expr[] { Exprs.One, y, Exprs.One },
                    new Expr[] { Exprs.Zero, Exprs.One, z },
                },
                Exprs.Subtract(Exprs.Subtract(Exprs.Multiply(x, y, z), x), z)),
            // 3x3 cyclic: det = x^3 + y^3 - x*y
            new MatrixCase("3x3 cyclic [[x,y,1],[0,x,y],[y,0,x]]",
                new[]
                {
                    new Expr[] { x, y, Exprs.One },
                    new Expr[] { Exprs.Zero, x, y },
                    new Expr[] { y, Exprs.Zero, x },
                },
                Exprs.Subtract(Exprs.Add(Exprs.Power(x, 3), Exprs.Power(y, 3)), Exprs.Multiply(x, y))),
            // 3x3 parameter cycle: det = 1 + a*b*c
            new MatrixCase("3x3 [[1,a,0],[0,1,b],[c,0,1]]",
                new[]
                {
                    new Expr[] { Exprs.One, a, Exprs.Zero },
                    new Expr[] { Exprs.Zero, Exprs.One, b },
                    new Expr[] { c, Exprs.Zero, Exprs.One },
                },
                Exprs.Add(Exprs.One, Exprs.Multiply(a, b, c))),
            // 3x3 Vandermonde: det = (y-x)(z-x)(z-y), expanded ground truth
            new MatrixCase("3x3 Vandermonde",
                new[]
                {
                    new Expr[] { Exprs.One, x, Exprs.Power(x, 2) },
                    new Expr[] { Exprs.One, y, Exprs.Power(y, 2) },
                    new Expr[] { Exprs.One, z, Exprs.Power(z, 2) },
                },
                Algebra.Expand(Exprs.Multiply(
                    Exprs.Subtract(y, x), Exprs.Subtract(z, x), Exprs.Subtract(z, y)), ctx)),
        };
    }

    /// <summary>The symbols of the whole matrix, in a deterministic order (one order for every
    /// polynomial built for that matrix).</summary>
    private static Symbol[] VariablesOf(SymbolicMatrix m)
    {
        var byName = new SortedDictionary<string, Symbol>(StringComparer.Ordinal);
        for (int i = 0; i < m.Rows; i++)
            for (int j = 0; j < m.Columns; j++)
                foreach (var s in Printing.CollectSymbols(m[i, j]))
                    byName[s.Name] = s;
        return byName.Values.ToArray();
    }

    private static Polynomial Poly(Expr e, ExprContext ctx, Symbol[] vars, string what)
    {
        Assert.True(Polynomial.TryFromExpr(e, ctx, vars, out var p, out var reason),
            $"{what}: not a polynomial in [{string.Join(",", vars.Select(v => v.Name))}] ({reason}): {Printing.PrettyPrint(e)}");
        return p;
    }

    /// <summary>
    /// Proves A·candidate = I as an identity of rational functions, by clearing the denominators the
    /// kernel reports for the candidate. Each candidate entry is split with the kernel's own
    /// <see cref="RationalFunctions.TryRationalize"/> into n_kj/d_kj; the entry (i,j) of the product
    /// equals Σ_k a_ik·n_kj/d_kj, so with D = Π_k d_kj the identity is the polynomial identity
    /// Σ_k a_ik·n_kj·(D/d_kj) = δ_ij·D. Polynomial equality is exact and complete over Q, so a wrong
    /// inverse (wrong adjugate entry, wrong sign, wrong transpose) makes this difference a non-zero
    /// polynomial and the check returns false.
    /// </summary>
    private static bool TryProveInverseIdentity(
        SymbolicMatrix a, SymbolicMatrix candidate, Polynomial detPoly, ExprContext ctx, Symbol[] vars, out string detail)
    {
        int n = a.Rows;
        detail = "";
        var A = new Polynomial[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                if (!Polynomial.TryFromExpr(a[i, j], ctx, vars, out A[i, j], out var ra))
                {
                    detail = $"A[{i},{j}] is not a polynomial ({ra})";
                    return false;
                }
        var N = new Polynomial[n, n];
        var D = new Polynomial[n, n];
        for (int k = 0; k < n; k++)
            for (int j = 0; j < n; j++)
            {
                if (!RationalFunctions.TryRationalize(candidate[k, j], ctx, out var ne, out var de))
                {
                    detail = $"candidate[{k},{j}] did not rationalize: {Printing.PrettyPrint(candidate[k, j])}";
                    return false;
                }
                if (!Polynomial.TryFromExpr(ne, ctx, vars, out N[k, j], out var rn))
                {
                    detail = $"candidate[{k},{j}] numerator is not a polynomial ({rn}): {Printing.PrettyPrint(ne)}";
                    return false;
                }
                if (!Polynomial.TryFromExpr(de, ctx, vars, out D[k, j], out var rd))
                {
                    detail = $"candidate[{k},{j}] denominator is not a polynomial ({rd}): {Printing.PrettyPrint(de)}";
                    return false;
                }
                if (D[k, j].IsZero)
                {
                    detail = $"candidate[{k},{j}] has an identically zero denominator";
                    return false;
                }
                // the emitted denominator is the determinant the condition names (a zero adjugate
                // entry is emitted as the plain polynomial 0, denominator 1)
                if (!N[k, j].IsZero && !D[k, j].Equals(detPoly))
                {
                    detail = $"candidate[{k},{j}] denominator is not the determinant condition: " +
                             $"{Printing.PrettyPrint(de)} vs {Printing.PrettyPrint(candidate[k, j])}";
                    return false;
                }
            }
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
            {
                var total = Polynomial.FromConstant(new VariableOrder(vars), Rat.One);
                for (int k = 0; k < n; k++)
                    total = Polynomial.Multiply(total, D[k, j]);
                var sum = Polynomial.Zero(new VariableOrder(vars));
                for (int k = 0; k < n; k++)
                {
                    var (share, remainder) = total.DivRem(D[k, j], MonomialOrder.Lex);
                    if (!remainder.IsZero)
                    {
                        detail = $"internal: D/D[{k},{j}] did not divide exactly";
                        return false;
                    }
                    sum = Polynomial.Add(sum, Polynomial.Multiply(A[i, k], Polynomial.Multiply(N[k, j], share)));
                }
                var rhs = i == j ? total : Polynomial.Zero(new VariableOrder(vars));
                var diff = Polynomial.Subtract(sum, rhs);
                if (!diff.IsZero)
                {
                    detail = $"(A·A⁻¹)[{i},{j}] is not δ_{{{i}{j}}}: the cleared numerator difference is a " +
                             $"non-zero polynomial ({diff.TermCount} terms)";
                    return false;
                }
            }
        return true;
    }

    [Fact]
    public void Inverse_TimesOriginal_IsIdentitySymbolically_UnderDetNonZeroCondition()
    {
        var ctx = NewCtx();
        foreach (var c in InverseCorpus(ctx))
        {
            var a = SymbolicMatrix.From(c.Entries);
            var vars = VariablesOf(a);
            var result = a.InverseWithConditions(ctx);
            Assert.True(result.Matrix is not null,
                $"{c.Name}: InverseWithConditions returned no inverse (note: {result.Note})");
            var inv = result.Matrix!;

            // (1) the carried condition is exactly det ≠ 0, and the determinant is the
            //     independently known one for this matrix (expanded ground truth, not the kernel's
            //     own Det output re-used as its own witness)
            var atoms = result.Conditions.Atoms.ToArray();
            Assert.True(atoms.Length == 1,
                $"{c.Name}: expected one condition, got [{string.Join(", ", atoms.Select(AssumptionSet.Describe))}]");
            var cond = Assert.IsType<ExpressionPropertyAssumption>(atoms[0]);
            Assert.Equal(SymbolPredicate.NonZero, cond.P);
            Assert.Equal(a.Det(ctx), cond.E);
            var detPoly = Poly(cond.E, ctx, vars, $"{c.Name}: condition expression");
            Assert.False(detPoly.IsZero, $"{c.Name}: the emitted condition is 0 != 0");
            var expected = Poly(c.ExpectedDet, ctx, vars, $"{c.Name}: expected determinant");
            Assert.True(Polynomial.Subtract(detPoly, expected).IsZero,
                $"{c.Name}: emitted det {Printing.PrettyPrint(cond.E)} != expected {Printing.PrettyPrint(c.ExpectedDet)}");

            // (2)+(3) the symbolic identity, under that condition
            Assert.True(TryProveInverseIdentity(a, inv, detPoly, ctx, vars, out var detail),
                $"{c.Name}: {detail}");
        }
    }

    /// <summary>
    /// The identity check must be able to FAIL, otherwise the test above proves nothing. Three
    /// corrupted inverses of [[x,1],[y,x]] are rejected; the last one is built so that it agrees
    /// with the true inverse at the very point the old numeric spot-check used — (x,y) = (3,2) —
    /// which the assertion below confirms: the numeric check at that point accepts it, the symbolic
    /// check rejects it. That is the difference between "equal at a sample" and "equal identically".
    /// </summary>
    [Fact]
    public void InverseIdentityCheck_RejectsInversesThatPassTheNumericSpotCheck()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, Exprs.One }, new Expr[] { y, x } });
        var vars = VariablesOf(a);
        var detPoly = Poly(a.Det(ctx), ctx, vars, "det");
        var inv = a.InverseWithConditions(ctx).Matrix!;

        // the unmutated inverse passes: the checker is not merely rejecting everything
        Assert.True(TryProveInverseIdentity(a, inv, detPoly, ctx, vars, out var clean),
            $"the kernel's own inverse was rejected: {clean}");

        // (a) transposed adjugate: the classic [[d,-b],[-c,a]] vs [[d,-c],[-b,a]] confusion
        var transposed = SymbolicMatrix.From(new[]
        {
            new Expr[] { inv[0, 0], inv[1, 0] },
            new Expr[] { inv[0, 1], inv[1, 1] },
        });
        Assert.False(TryProveInverseIdentity(a, transposed, detPoly, ctx, vars, out var dA), "transposed adjugate accepted");
        Assert.False(string.IsNullOrEmpty(dA));

        // (b) sign error on one cofactor
        var signFlipped = WithEntry(inv, 0, 1, ctx, n => Exprs.Negate(n));
        Assert.False(TryProveInverseIdentity(a, signFlipped, detPoly, ctx, vars, out var dB), "sign-flipped cofactor accepted");
        Assert.False(string.IsNullOrEmpty(dB));

        // (c) +1 in one numerator
        var perturbed = WithEntry(inv, 0, 0, ctx, n => Exprs.Add(n, Exprs.One));
        Assert.False(TryProveInverseIdentity(a, perturbed, detPoly, ctx, vars, out var dC), "numerator perturbation accepted");
        Assert.False(string.IsNullOrEmpty(dC));

        // (d) perturbation that VANISHES at (x,y) = (3,2): the numerator -1 becomes
        //     -1 - (x-3)^2, so this wrong inverse takes the true inverse's values at that point
        var blindSpot = WithEntry(inv, 0, 1, ctx, n => Exprs.Subtract(n, Exprs.Power(Exprs.Subtract(x, 3), 2)));
        var product = SymbolicMatrix.Multiply(a, blindSpot);
        using (Rl.WithPrecision(50, 20))
        {
            var bindings = new Dictionary<Symbol, Num>
            {
                [x] = NumOps.FromRat(Rat.From(3, 1)),
                [y] = NumOps.FromRat(Rat.From(2, 1)),
            };
            Num Entry(int i, int j) => Evaluation.EvaluateToNum(product[i, j], ctx, bindings);
            // the old numeric smoke-test assertion passes on this wrong inverse
            Assert.Equal(0, NumOps.Compare(Entry(0, 0), NumOps.FromLong(1)));
            Assert.Equal(0, NumOps.Compare(Entry(0, 1), NumOps.FromLong(0)));
            Assert.Equal(0, NumOps.Compare(Entry(1, 0), NumOps.FromLong(0)));
            Assert.Equal(0, NumOps.Compare(Entry(1, 1), NumOps.FromLong(1)));
        }
        Assert.False(TryProveInverseIdentity(a, blindSpot, detPoly, ctx, vars, out var dD),
            "an inverse that is only correct at (3,2) was accepted symbolically");
        Assert.False(string.IsNullOrEmpty(dD));
    }

    /// <summary>Replaces one entry by (mutated numerator)/denominator, keeping the kernel's own
    /// split so the checker sees a candidate of the same shape.</summary>
    private static SymbolicMatrix WithEntry(SymbolicMatrix m, int row, int col, ExprContext ctx, Func<Expr, Expr> mutate)
    {
        Assert.True(RationalFunctions.TryRationalize(m[row, col], ctx, out var n, out var d),
            $"probe entry [{row},{col}] did not rationalize");
        var rows = new Expr[m.Rows][];
        for (int i = 0; i < m.Rows; i++)
        {
            rows[i] = new Expr[m.Columns];
            for (int j = 0; j < m.Columns; j++)
                rows[i][j] = i == row && j == col ? Exprs.Divide(mutate(n), d) : m[i, j];
        }
        return SymbolicMatrix.From(rows);
    }

    /// <summary>The identity is claimed exactly where the condition holds: on the zero set of the
    /// determinant the emitted inverse entries are undefined.</summary>
    [Fact]
    public void InverseIdentity_HoldsOnlyWhereDetIsNonZero()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var a = SymbolicMatrix.From(new[] { new Expr[] { x, Exprs.One }, new Expr[] { y, x } });
        var inv = a.InverseWithConditions(ctx).Matrix!;
        using var scope = Rl.WithPrecision(50, 20);

        // det = x^2 - y = 0 at (2,4): the inverse entry x/(x^2-y) is undefined there
        var onCondition = new Dictionary<Symbol, Num>
        {
            [x] = NumOps.FromRat(Rat.From(2, 1)),
            [y] = NumOps.FromRat(Rat.From(4, 1)),
        };
        Assert.Throws<EvaluationException>(() => Evaluation.EvaluateToNum(inv[0, 0], ctx, onCondition));
        Assert.Throws<EvaluationException>(() => Evaluation.EvaluateToNum(inv[1, 1], ctx, onCondition));

        // det = 1 at (2,3): defined, and the product is the identity there
        var offCondition = new Dictionary<Symbol, Num>
        {
            [x] = NumOps.FromRat(Rat.From(2, 1)),
            [y] = NumOps.FromRat(Rat.From(3, 1)),
        };
        var product = SymbolicMatrix.Multiply(a, inv);
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[0, 0], ctx, offCondition), NumOps.FromLong(1)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[0, 1], ctx, offCondition), NumOps.FromLong(0)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[1, 0], ctx, offCondition), NumOps.FromLong(0)));
        Assert.Equal(0, NumOps.Compare(Evaluation.EvaluateToNum(product[1, 1], ctx, offCondition), NumOps.FromLong(1)));
    }

    // ==================================================================
    // Gap 2 — cancel preserves value off poles, and the pole is excluded
    // ==================================================================

    private sealed record CancelCase(
        string Name,
        Expr Original,
        Expr ExpectedCancelled,
        (Symbol S, Rat V)[] ExtraBindings,
        Rat[] Poles,
        bool PoleRemoved);

    /// <summary>
    /// Sampled points for the off-pole agreement. They are fixed rationals of mixed sign and
    /// magnitude, none of which is a pole of any corpus member (the corpus poles are all ±integers
    /// in {0, 1, 2} — the roots of the removed common factors); the test does not ASSUME this, it
    /// asserts that evaluating the original there does not raise <see cref="EvaluationException"/>,
    /// i.e. every sampled point is machine-verified to be in the original's domain.
    /// </summary>
    private static readonly Rat[] SamplePoints =
    {
        Rat.From(-4, 1), Rat.From(-3, 2), Rat.From(-1, 3), Rat.From(2, 3),
        Rat.From(4, 3), Rat.From(5, 2), Rat.From(3, 1), Rat.From(7, 1),
    };

    private static CancelCase[] CancelCorpus(ExprContext ctx)
    {
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        Expr Num(params Expr[] t) => Exprs.Add(t);
        return new[]
        {
            new CancelCase("x/x", Exprs.Divide(x, x), Exprs.One,
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.Zero }, true),
            new CancelCase("x^2/x", Exprs.Divide(Exprs.Power(x, 2), x), x,
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.Zero }, true),
            new CancelCase("(x^2-1)/(x-1)",
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 2), Exprs.One), Exprs.Subtract(x, Exprs.One)),
                Exprs.Add(x, Exprs.One),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(1, 1) }, true),
            new CancelCase("(x^2-4)/(x-2)",
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 2), 4), Exprs.Subtract(x, 2)),
                Exprs.Add(x, 2),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(2, 1) }, true),
            // (x-1)^2 spelled UNEXPANDED: the parser still refuses Pow(sum, k), so cancel expands
            // the numerator/denominator once before parsing (N16b) — same result as the expanded case
            new CancelCase("(x-1)^2/(x-1)",
                Exprs.Divide(Exprs.Power(Exprs.Subtract(x, Exprs.One), 2), Exprs.Subtract(x, Exprs.One)),
                Exprs.Subtract(x, Exprs.One),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(1, 1) }, true),
            // (x-1)^2 spelled expanded: the parser sees a polynomial, so the double pole collapses
            new CancelCase("(x^2-2x+1)/(x-1)",
                Exprs.Divide(Num(Exprs.One, Exprs.Multiply(-2, x), Exprs.Power(x, 2)), Exprs.Subtract(x, Exprs.One)),
                Exprs.Subtract(x, Exprs.One),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(1, 1) }, true),
            new CancelCase("(x^3-x)/(x^2-x)",
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 3), x), Exprs.Subtract(Exprs.Power(x, 2), x)),
                Exprs.Add(x, Exprs.One),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.Zero, Rat.From(1, 1) }, true),
            new CancelCase("(x^3-8)/(x-2)",
                Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 3), 8), Exprs.Subtract(x, 2)),
                Num(Exprs.Power(x, 2), Exprs.Multiply(2, x), 4),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(2, 1) }, true),
            // multivariate: the common factor (x-1) is removed, y survives
            new CancelCase("(x*y-y)/(x-1)",
                Exprs.Divide(Exprs.Subtract(Exprs.Multiply(x, y), y), Exprs.Subtract(x, Exprs.One)),
                y,
                new[] { (y, Rat.From(3, 1)) }, new[] { Rat.From(1, 1) }, true),
            // no common factor: nothing is cancelled, so the pole is NOT removed
            new CancelCase("(x^2+1)/(x-1) [irreducible]",
                Exprs.Divide(Exprs.Add(Exprs.Power(x, 2), Exprs.One), Exprs.Subtract(x, Exprs.One)),
                Exprs.Divide(Exprs.Add(Exprs.Power(x, 2), Exprs.One), Exprs.Subtract(x, Exprs.One)),
                Array.Empty<(Symbol, Rat)>(), new[] { Rat.From(1, 1) }, false),
        };
    }

    [Fact]
    public void Cancel_PreservesValueOffPoles_AndThePoleIsExcluded()
    {
        var ctx = NewCtx();
        using var scope = Rl.WithPrecision(50, 20);
        foreach (var c in CancelCorpus(ctx))
        {
            var x = ctx.Symbol("x");
            var cancelled = RationalFunctions.Cancel(c.Original, ctx);
            Assert.Equal(c.ExpectedCancelled, cancelled);

            Num Eval(Expr e, Rat point)
            {
                var bindings = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(point) };
                foreach (var (s, v) in c.ExtraBindings)
                    bindings[s] = NumOps.FromRat(v);
                return Evaluation.EvaluateToNum(e, ctx, bindings);
            }

            // (a) off-pole: the original is DEFINED at every sampled point (this is the off-pole
            //     claim, checked rather than asserted by construction) and the two sides agree
            //     exactly — the identity is algebraic, so no tolerance is involved
            foreach (var p in SamplePoints)
            {
                Assert.DoesNotContain(c.Poles, pole => pole.Equals(p));
                Num before;
                try
                {
                    before = Eval(c.Original, p);
                }
                catch (EvaluationException ex)
                {
                    Assert.Fail($"{c.Name}: sampled point x={p} is not in the original's domain: {ex.Message}");
                    throw;
                }
                var after = Eval(cancelled, p);
                Assert.True(NumOps.Compare(before, after) == 0,
                    $"{c.Name}: cancel changed the value at x={p}: " +
                    $"{Printing.PrettyPrint(c.Original)} -> {Printing.PrettyPrint(cancelled)}");
            }

            // (b) the pole: the original is undefined there; the reduced form is defined exactly
            //     when the singular factor was actually removed
            foreach (var pole in c.Poles)
            {
                Assert.Throws<EvaluationException>(() => Eval(c.Original, pole));
                if (c.PoleRemoved)
                {
                    var after = Eval(cancelled, pole);   // does not throw: the pole is gone
                    Assert.NotNull(after);
                }
                else
                {
                    Assert.Throws<EvaluationException>(() => Eval(cancelled, pole));
                }
            }
        }
    }

    /// <summary>
    /// How the kernel REPRESENTS the pole exclusion. The rewrite path attaches the NonZero side
    /// condition to the step that performs the cancellation, and safe mode refuses the rewrite
    /// while that condition is unproven — so the exclusion cannot be dropped silently. The bare
    /// <c>cancel</c> builtin is a VALUE builtin and cannot carry that condition (N16a); the
    /// structured <c>cancel_full</c> exposes the very same atom to the machine API.
    /// </summary>
    [Fact]
    public void Cancel_PoleExclusionIsCarriedAsANonZeroConditionByTheRewritePath()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var quotient = Exprs.Divide(x, x);

        var transformed = Simplify.Transform(quotient, ctx, new Simplify.Options(Trace: true));
        Assert.Equal(Exprs.One, transformed.Expression);
        var atom = Assert.Single(transformed.Conditions.Atoms);
        var cond = Assert.IsType<SymbolPropertyAssumption>(atom);
        Assert.Equal(x, cond.S);
        Assert.Equal(SymbolPredicate.NonZero, cond.P);
        Assert.Contains(transformed.Steps,
            s => s.RuleId == "rat.cancel-x-over-x" && s.Classification == RuleClassification.Conditional);

        // safe mode will not cancel while x != 0 is unproven ...
        Assert.Equal(quotient, Simplify.SimplifyExpr(quotient, ctx));
        // ... and licenses exactly the same rewrite once the condition is assumed
        using (ctx.WithAssumptions(AssumptionSet.Empty.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonZero))))
            Assert.Equal(Exprs.One, Simplify.SimplifyExpr(quotient, ctx));

        // the exclusion is semantically necessary: x/x is undefined at 0, the reduced form is 1
        using var scope = Rl.WithPrecision(50, 20);
        var atZero = new Dictionary<Symbol, Num> { [x] = NumOps.FromRat(Rat.Zero) };
        Assert.Throws<EvaluationException>(() => Evaluation.EvaluateToNum(quotient, ctx, atZero));
        Assert.Equal(0, NumOps.Compare(
            Evaluation.EvaluateToNum(Exprs.One, ctx, atZero), NumOps.FromLong(1)));
    }

    /// <summary>The <c>cancel</c> builtin (the user-facing surface) is the corpus reduction and, on
    /// the rewritten path, drops nothing: values agree off the pole and the pole is excluded.</summary>
    [Fact]
    public void CancelBuiltin_ReducesAndRefusesToCancelWhenNoFactorIsCommon()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");
        Assert.Equal("1 (Symbolic)", ValueFormatter.FormatTyped(engine.Evaluate("cancel(x/x)")));
        Assert.Equal("x + 1 (Symbolic)", ValueFormatter.FormatTyped(engine.Evaluate("cancel((x^2-1)/(x-1))")));
        // no common factor: the rational function comes back unchanged, not silently reduced
        Assert.Equal("(x^2 + 1)/(x - 1) (Symbolic)",
            ValueFormatter.FormatTyped(engine.Evaluate("cancel((x^2+1)/(x-1))")));
    }

    /// <summary>
    /// N16a: the bare <c>cancel</c> builtin returns a VALUE (an <c>Expr</c>), and the value model has
    /// no place to carry a side condition — so the honest answer is NOT to bolt <c>x != 0</c> onto the
    /// expression, but to expose the structured form alongside it, exactly as
    /// <c>simplify_full</c>/<c>integrate_full</c>/<c>limit_full</c> do. <c>cancel_full</c> is therefore
    /// the machine API a caller must use when the definedness delta matters: it reports the same
    /// NonZero condition the rewrite path attaches to <c>rat.cancel-x-over-x</c>, as a structured
    /// condition leaf, plus a status that says whether the reduction changed the domain.
    /// </summary>
    [Fact]
    public void CancelFull_ReportsTheConditionsTheBareValueCannotCarry()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        engine.Evaluate("x = symbol(\"x\")");

        static Value F(RecordValue rec, string name) =>
            (Value)rec.Fields.First(f => f.Name == name).Value!;

        var r = engine.Evaluate("cancel_full(x/x)").AsRecord();
        Assert.Equal("CancelResult", r.TypeName);
        // CancelStatus.Conditional: the reduction removed a pole, so it is not an everywhere identity
        Assert.Equal("Conditional", ValueFormatter.Format(F(r, "status")));
        Assert.Equal("x/x", ValueFormatter.Format(F(r, "original")));
        Assert.Equal("1", ValueFormatter.Format(F(r, "expression")));
        Assert.True(F(r, "changed").AsBoolean());
        // the removed factor's non-vanishing IS the condition: the same atom the rewrite path
        // attaches to rat.cancel-x-over-x (x != 0), a structured relation, never a string
        Assert.Equal("[x != 0] (Vector)", ValueFormatter.FormatTyped(F(r, "conditions")));
        Assert.Equal("x != 0", ValueFormatter.Format(F(r, "conditions").AsVector()[0]));

        // an exact reduction (nothing was removed) reports no conditions at all — an EMPTY vector,
        // never a fabricated condition and never the marker for an unsatisfiable set
        var exact = engine.Evaluate("cancel_full((x^2+1)/(x-1))").AsRecord();
        Assert.Equal("Exact", ValueFormatter.Format(F(exact, "status")));
        Assert.Equal("(x^2 + 1)/(x - 1)", ValueFormatter.Format(F(exact, "expression")));
        Assert.False(F(exact, "changed").AsBoolean());
        Assert.Empty(F(exact, "conditions").AsVector());

        // N16b through the structured surface: the condition is on the REMOVED FACTOR, not on a
        // symbol — cancel_full must report x - 1 != 0, the pole it just removed
        var pow = engine.Evaluate("cancel_full((x-1)^2/(x-1))").AsRecord();
        Assert.Equal("x - 1", ValueFormatter.Format(F(pow, "expression")));
        Assert.Equal("[x - 1 != 0] (Vector)", ValueFormatter.FormatTyped(F(pow, "conditions")));

        // property access into the record, as for simplify_full; the bare builtin keeps returning the
        // VALUE (no condition can be attached to it — that is the point of the split)
        Assert.Equal("1", ValueFormatter.Format(engine.Evaluate("cancel_full(x/x).expression")));
        Assert.Equal("x != 0", ValueFormatter.Format(engine.Evaluate("cancel_full(x/x).conditions[0]")));
        Assert.Equal("1 (Symbolic)", ValueFormatter.FormatTyped(engine.Evaluate("cancel(x/x)")));
    }

    /// <summary>
    /// The kernel half of N16a, checked atom for atom: the structured cancel builds the SAME condition
    /// the rewrite path attaches to <c>rat.cancel-x-over-x</c> — a symbol becomes a
    /// SymbolPropertyAssumption, a removed factor that is not a symbol becomes an
    /// ExpressionPropertyAssumption — so there is one convention, not two parallel ones.
    /// </summary>
    [Fact]
    public void CancelWithConditions_BuildsTheSameConditionAtomsAsTheRewritePath()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        using var scope = Rl.WithPrecision(50, 20);

        var r = RationalFunctions.CancelWithConditions(Exprs.Divide(x, x), ctx);
        Assert.Equal(Exprs.One, r.Expression);
        Assert.True(r.Changed);
        Assert.Equal(RationalFunctions.CancelStatus.Conditional, r.Status);
        var atom = Assert.Single(r.Conditions.Atoms);
        var asSymbol = Assert.IsType<SymbolPropertyAssumption>(atom);
        Assert.Equal(x, asSymbol.S);
        Assert.Equal(SymbolPredicate.NonZero, asSymbol.P);

        // the rewrite path's condition for the same input: the very same atom
        var transformed = Simplify.Transform(Exprs.Divide(x, x), ctx, new Simplify.Options(Trace: true));
        Assert.Equal(transformed.Conditions.Atoms[0], atom);

        // a removed factor that is a sum carries the expression form of the same predicate
        var shift = RationalFunctions.CancelWithConditions(
            Exprs.Divide(Exprs.Power(Exprs.Subtract(x, Exprs.One), 2), Exprs.Subtract(x, Exprs.One)), ctx);
        Assert.Equal(Exprs.Subtract(x, Exprs.One), shift.Expression);
        var asExpr = Assert.IsType<ExpressionPropertyAssumption>(Assert.Single(shift.Conditions.Atoms));
        Assert.Equal(Exprs.Subtract(x, Exprs.One), asExpr.E);
        Assert.Equal(SymbolPredicate.NonZero, asExpr.P);

        // nothing removed: Exact, no conditions, and the bare and structured forms agree
        var exact = RationalFunctions.CancelWithConditions(
            Exprs.Divide(Exprs.Add(Exprs.Power(x, 2), Exprs.One), Exprs.Subtract(x, Exprs.One)), ctx);
        Assert.Equal(RationalFunctions.CancelStatus.Exact, exact.Status);
        Assert.Empty(exact.Conditions.Atoms);
        Assert.False(exact.Changed);
        Assert.Equal(RationalFunctions.Cancel(exact.Original, ctx), exact.Expression);
    }

    /// <summary>
    /// N16b (FIXED — this converts the former characterization test
    /// <c>Cancel_UnexpandedPowerOfASum_IsNotReduced_KnownGap</c>, which pinned the old behaviour
    /// <c>cancel((x-1)^2/(x-1)) == (x-1)^2/(x-1)</c>): the unexpanded spelling now reduces.
    /// <para>
    /// The fix is in <see cref="RationalFunctions.Cancel"/>, NOT in the parser: when the direct
    /// <see cref="Polynomial.TryFromExpr"/> call refuses a numerator/denominator, cancel expands it
    /// once and parses again. Expansion is value-preserving on every input (it introduces no
    /// definedness delta and needs no side condition), so this is unconditionally sound — unlike the
    /// cancellation itself. <see cref="Polynomial.TryFromExpr"/> keeps its documented contract, and
    /// the boundary assertions below stay exactly as they were: a power of a sum is still not a
    /// polynomial TO THE PARSER; the fix sits above it.
    /// </para>
    /// </summary>
    [Fact]
    public void Cancel_ExpandsAnUnexpandedPowerOfASumBeforeReducing()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var unexpanded = Exprs.Divide(Exprs.Power(Exprs.Subtract(x, Exprs.One), 2), Exprs.Subtract(x, Exprs.One));
        Assert.Equal(Exprs.Subtract(x, Exprs.One), RationalFunctions.Cancel(unexpanded, ctx));

        // the expanded spelling of the same function reduces to the same thing: both spellings agree
        var expanded = Exprs.Divide(
            Algebra.Expand(Exprs.Power(Exprs.Subtract(x, Exprs.One), 2), ctx), Exprs.Subtract(x, Exprs.One));
        Assert.Equal(Exprs.Subtract(x, Exprs.One), RationalFunctions.Cancel(expanded, ctx));

        // the parser boundary itself (UNCHANGED): a power of a sum is not a polynomial to the
        // parser, a power of a symbol is. The fix is above the parser, not inside it.
        Assert.False(Polynomial.TryFromExpr(
            Exprs.Power(Exprs.Subtract(x, Exprs.One), 2), ctx, new[] { x }, out _, out _));
        Assert.True(Polynomial.TryFromExpr(Exprs.Power(x, 2), ctx, new[] { x }, out _, out _));
    }
}
