using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

public class DebugCubic
{
    [Fact]
    public void Cubic_Roots_SatisfyPolynomial()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var p = Exprs.Subtract(Exprs.Power(x, 3), 1);
        var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, p, Exprs.Zero), x, ctx);
        var roots = set.Solutions.Select(s => s.Value).ToArray();
        var canon = string.Join(" ; ", roots.Select(r => Printing.CanonicalPrint(r)));
        foreach (var r in roots)
        {
            var at = Evaluation.Substitute(p, ctx, new Dictionary<Symbol, Expr> { [x] = r });
            var expanded = Algebra.Expand(at, ctx);
            if (!(expanded is RationalConstantExpr rc && rc.Value.IsZero))
                throw new Exception("root " + Printing.PrettyPrint(r) + " does not satisfy p: " + Printing.PrettyPrint(expanded) + " | all: " + canon);
        }
    }
}
