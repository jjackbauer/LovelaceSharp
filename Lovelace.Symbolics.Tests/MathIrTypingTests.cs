using Lovelace.MathIR;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>MathIR type inference and validation (Phase 5).</summary>
public class MathIrTypingTests
{
    private static ExprContext NewCtx()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx;
    }

    private static IrNode Node(IrOpKind op, params int[] operands) => new(op, operands, 0);

    [Fact]
    public void Infer_TypesForLoweredExpressions()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var p1 = Lowering.Lower(Exprs.Add(Exprs.Power(x, 2), 1), ctx, new[] { x });
        var t1 = IrTyping.Infer(p1);
        Assert.Equal(IrType.Integer, t1[p1.Nodes.Count - 1]);
        var p2 = Lowering.Lower(Exprs.Divide(x, 2), ctx, new[] { x });
        var t2 = IrTyping.Infer(p2);
        Assert.Equal(IrType.Rational, t2[p2.Nodes.Count - 1]);
        // exp of a typed (integer) argument is Real; exp of an unconstrained parameter is unknown
        var p3 = Lowering.Lower(Exprs.Function(ctx.Function("exp"), Exprs.Integer(2)), ctx, new[] { x });
        var t3 = IrTyping.Infer(p3);
        Assert.Equal(IrType.Real, t3[p3.Nodes.Count - 1]);
        var p3u = Lowering.Lower(Exprs.Function(ctx.Function("exp"), x), ctx, new[] { x });
        Assert.Null(IrTyping.Infer(p3u)[p3u.Nodes.Count - 1]);
        // relations lower as Piecewise guards: the guard node types as Bool
        var pw = Exprs.Piecewise(new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), x) }, Exprs.Zero);
        var p4 = Lowering.Lower(pw, ctx, new[] { x });
        var t4 = IrTyping.Infer(p4);
        int guardIndex = p4.Nodes.FindIndex(n => n.Op == IrOpKind.Gt);
        Assert.True(guardIndex >= 0);
        Assert.Equal(IrType.Bool, t4[guardIndex]);
        var cplx = Exprs.Add(Exprs.One, Exprs.I);
        var p5 = Lowering.Lower(Exprs.Multiply(cplx, x), ctx, new[] { x });
        var t5 = IrTyping.Infer(p5);
        Assert.Equal(IrType.Complex, t5[p5.Nodes.Count - 1]);
    }

    [Fact]
    public void Validate_AcceptsLoweredCorpus()
    {
        var ctx = NewCtx();
        var x = ctx.Symbol("x");
        var y = ctx.Symbol("y");
        var corpus = new Expr[]
        {
            Exprs.Add(Exprs.Power(x, 2), 1),
            Exprs.Multiply(Exprs.Function(ctx.Function("sin"), x), Exprs.Function(ctx.Function("exp"), x)),
            Exprs.Piecewise(new[] { new PiecewiseBranch(Exprs.Relation(RelOp.Gt, x, Exprs.Zero), x) }, Exprs.Negate(x)),
            Exprs.Divide(x, Exprs.Add(x, 1)),
            Exprs.Power(x, y),
        };
        foreach (var e in corpus)
        {
            var program = Lowering.Lower(e, ctx, new[] { x, y });
            IrTyping.Validate(program);
        }
    }

    [Fact]
    public void Validate_RejectsOutOfRangeOperand()
    {
        var prog = new IrProgram();
        prog.Parameters.Add("x");
        prog.Constants.Add("(rat 1 1)");
        prog.Nodes.Add(Node(IrOpKind.Parameter, 0));
        prog.Nodes.Add(Node(IrOpKind.Constant, 0));
        prog.Nodes.Add(Node(IrOpKind.Add, 0, 42));
        Assert.Throws<InvalidOperationException>(() => IrTyping.Validate(prog));
    }

    [Fact]
    public void Validate_RejectsNonBoolSelectGuard()
    {
        var prog = new IrProgram();
        prog.Parameters.Add("x");
        prog.Constants.Add("(rat 1 1)");
        prog.Nodes.Add(Node(IrOpKind.Parameter, 0));
        prog.Nodes.Add(Node(IrOpKind.Constant, 0));
        prog.Nodes.Add(Node(IrOpKind.Add, 0, 1));
        prog.Nodes.Add(Node(IrOpKind.Select, 2, 0, 1));
        Assert.Throws<InvalidOperationException>(() => IrTyping.Validate(prog));
    }

    [Fact]
    public void Validate_RejectsOrderedComparisonOfComplex()
    {
        var prog = new IrProgram();
        prog.Parameters.Add("x");
        prog.Constants.Add("(cplx 1 1 1 1)");
        prog.Nodes.Add(Node(IrOpKind.Constant, 0));
        prog.Nodes.Add(Node(IrOpKind.Parameter, 0));
        prog.Nodes.Add(Node(IrOpKind.Lt, 0, 1));
        Assert.Throws<InvalidOperationException>(() => IrTyping.Validate(prog));
    }

    [Fact]
    public void Validate_RejectsWrongArity()
    {
        var prog = new IrProgram();
        prog.Parameters.Add("x");
        prog.Nodes.Add(Node(IrOpKind.Parameter, 0));
        prog.Nodes.Add(Node(IrOpKind.Add, 0, 0, 0));
        Assert.Throws<InvalidOperationException>(() => IrTyping.Validate(prog));
    }

    [Fact]
    public void Deserialize_RejectsMalformedProgram()
    {
        var text = "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,42\n";
        Assert.Throws<InvalidOperationException>(() => IrProgram.Deserialize(text));
    }
}
