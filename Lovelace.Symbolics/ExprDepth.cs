namespace Lovelace.Symbolics;

/// <summary>
/// Depth of a canonical expression: one plus the deepest child, i.e. the length of the longest
/// root-to-leaf path. Computed from the children's cached depths while a node is being interned,
/// exactly like <see cref="Expr.NodeCount"/> is, so it costs one pass over the children at
/// construction and nothing afterwards.
/// <para>
/// Kept total on purpose: every node kind is listed, and an unknown kind is an error rather than a
/// default. A silent default would let a new node kind bypass the shared nesting budget.
/// </para>
/// </summary>
internal static class ExprDepth
{
    internal static int Of(Expr node) => node switch
    {
        AddExpr a => 1 + MaxOf(a.Terms),
        MultiplyExpr m => 1 + MaxOf(m.Factors),
        FunctionExpr f => 1 + MaxOf(f.Arguments),
        AndExpr a => 1 + MaxOf(a.Operands),
        OrExpr o => 1 + MaxOf(o.Operands),
        PowerExpr p => 1 + Math.Max(p.Base._depth, p.Exponent._depth),
        RelationExpr r => 1 + Math.Max(r.Left._depth, r.Right._depth),
        OrderExpr o => 1 + Math.Max(o.Variable._depth, Math.Max(o.Point._depth, o.Degree._depth)),
        NotExpr n => 1 + n.Operand._depth,
        DerivativeExpr d => 1 + d.Operand._depth,
        IntegralExpr i => 1 + i.Operand._depth,
        PiecewiseExpr pw => 1 + MaxOf(pw),
        // the defining polynomial is not an Expr subtree (Polynomial holds its own flat terms)
        RootOfExpr => 1,
        SymbolExpr or IntegerConstantExpr or RationalConstantExpr or RealConstantExpr
            or ComplexConstantExpr or NamedConstantExpr => 1,
        _ => throw new InvalidOperationException(
            $"ExprDepth: unhandled node kind '{node.Kind}' — the nesting budget cannot be guaranteed "
            + "for a node this measure does not know."),
    };

    private static int MaxOf(System.Collections.Immutable.ImmutableArray<Expr> items)
    {
        int max = 0;
        foreach (var item in items)
            if (item._depth > max) max = item._depth;
        return max;
    }

    private static int MaxOf(PiecewiseExpr piecewise)
    {
        int max = piecewise.Otherwise._depth;
        foreach (var branch in piecewise.Branches)
        {
            if (branch.Guard._depth > max) max = branch.Guard._depth;
            if (branch.Value._depth > max) max = branch.Value._depth;
        }
        return max;
    }
}
