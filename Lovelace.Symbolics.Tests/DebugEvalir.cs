using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;
using Rat = Lovelace.Rational.Rational;

namespace Lovelace.Symbolics.Tests;

public class DebugEvalir
{
    [Fact]
    public void Lower_Evaluate_FiveTerm()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var f = Exprs.Add(Exprs.Power(x, 5), Exprs.Multiply(2, Exprs.Power(x, 4)), Exprs.Multiply(3, Exprs.Power(x, 3)), Exprs.Power(x, 2));
        var opt = Optimizer.Optimize(f, new[] { x }, ctx);
        var prog = Lowering.Lower(opt.Expression, ctx, new[] { x });
        var text = prog.Serialize();
        var back = IrProgram.Deserialize(text);
        string detail = "consts=[" + string.Join(" ; ", back.Constants) + "] params=[" + string.Join(",", back.Parameters) + "] nodes=" + back.Nodes.Count + " serialized=" + text.Replace("\n", "|");
        try
        {
            foreach (var c in back.Constants)
                Printing.CanonicalParse(c, ctx);
        }
        catch (Exception ex)
        {
            Assert.Fail(detail + " parse-error=" + ex.Message);
        }
        var result = IrEvaluator.Evaluate(back, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(2L)) }, ctx);
        Assert.Equal(0, NumOps.Compare(result, NumOps.FromLong(32 + 32 + 24 + 4)));
    }
}
