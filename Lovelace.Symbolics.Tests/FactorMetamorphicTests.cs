using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Metamorphic property over the factorization: for every polynomial p in the corpus,
/// <c>expand(factor(p))</c> must equal <c>expand(p)</c> — the sign of the leading coefficient
/// included. The corpus deliberately mixes NEGATIVE-leading polynomials (the reported defect:
/// factor(-x^2 + 1) printed (x - 1)*(x + 1), which expands to x^2 - 1, not to -x^2 + 1) with
/// positive controls, because a factorization that drops the sign is invisible in the printed
/// factors and shows up only when the product is expanded.
/// </summary>
public class FactorMetamorphicTests : IDisposable
{
    private readonly ExprContext _ctx = new();

    public FactorMetamorphicTests() => Exprs.Current = _ctx;

    public void Dispose() => Exprs.ClearCurrent();

    private static Expr Neg(Expr e) => Exprs.Negate(e);

    /// <summary>The corpus: (printed name, polynomial over Q[x]). Negative leading coefficients
    /// first — that is the half the old kernel got wrong.</summary>
    internal static IReadOnlyList<(string Name, Func<Symbol, Expr> Build)> Corpus => new[]
    {
        // ---- negative leading coefficient: the sign must survive the factorization ----
        ("-x^2 + 1", (Func<Symbol, Expr>)(x => Exprs.Add(Neg(Exprs.Power(x, 2)), 1))),
        ("-x^3 + x", x => Exprs.Add(Neg(Exprs.Power(x, 3)), x)),
        ("-x^4 + 5*x^2 - 4", x => Exprs.Add(Neg(Exprs.Power(x, 4)), Exprs.Multiply(5, Exprs.Power(x, 2)), -4)),
        ("-2*x^2 + 8", x => Exprs.Add(Exprs.Multiply(-2, Exprs.Power(x, 2)), 8)),
        ("-6*x^3 + 6*x", x => Exprs.Add(Exprs.Multiply(-6, Exprs.Power(x, 3)), Exprs.Multiply(6, x))),
        ("-x^2 + 2", x => Exprs.Add(Neg(Exprs.Power(x, 2)), 2)),
        ("-x^3 - x^2 + x + 1", x => Exprs.Add(Neg(Exprs.Power(x, 3)), Neg(Exprs.Power(x, 2)), x, 1)),
        ("-x^2 - 2*x - 1", x => Exprs.Add(Neg(Exprs.Power(x, 2)), Exprs.Multiply(-2, x), -1)),
        ("-3*x^2 + 3", x => Exprs.Add(Exprs.Multiply(-3, Exprs.Power(x, 2)), 3)),
        // ---- positive leading coefficient: the controls, correct before the fix ----
        ("x^2 - 1", x => Exprs.Add(Exprs.Power(x, 2), -1)),
        ("x^3 + 1", x => Exprs.Add(Exprs.Power(x, 3), 1)),
        ("x^4 - 5*x^2 + 4", x => Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4)),
        ("2*x^2 - 8", x => Exprs.Add(Exprs.Multiply(2, Exprs.Power(x, 2)), -8)),
        ("6*x^3 - 6*x", x => Exprs.Add(Exprs.Multiply(6, Exprs.Power(x, 3)), Exprs.Multiply(-6, x))),
        ("x^2 - 2", x => Exprs.Add(Exprs.Power(x, 2), -2)),
        ("x^3 + x^2 - x - 1", x => Exprs.Add(Exprs.Power(x, 3), Exprs.Power(x, 2), Neg(x), -1)),
        ("x^3 - 3*x^2 + 3*x - 1", x => Exprs.Add(Exprs.Power(x, 3), Exprs.Multiply(-3, Exprs.Power(x, 2)), Exprs.Multiply(3, x), -1)),
        ("x^4 - 1", x => Exprs.Add(Exprs.Power(x, 4), -1)),
    };

    [Fact]
    public void Factor_ExpandsBackToItsInput_OverTheWholeCorpus()
    {
        Symbol x = _ctx.Symbol("x");
        var failures = new List<string>();
        int negative = 0;
        foreach ((string name, Func<Symbol, Expr> build) in Corpus)
        {
            Expr p = build(x);
            if (name.StartsWith("-", StringComparison.Ordinal))
                negative++;
            Expr factored = Factoring.Factor(p, _ctx);
            Expr expanded = Algebra.Expand(factored, _ctx);
            Expr original = Algebra.Expand(p, _ctx);
            if (!expanded.Equals(original))
                failures.Add(
                    $"factor({name}) = {Printing.PrettyPrint(factored)} expands to {Printing.PrettyPrint(expanded)}, " +
                    $"not to the input {Printing.PrettyPrint(original)}");
        }
        Assert.True(negative >= 3, $"the corpus must contain negative-leading cases, found {negative}");
        Assert.True(failures.Count == 0,
            $"{failures.Count} of {Corpus.Count} corpus polynomials do not satisfy expand(factor(p)) == p:" +
            Environment.NewLine + string.Join(Environment.NewLine, failures));
    }
}