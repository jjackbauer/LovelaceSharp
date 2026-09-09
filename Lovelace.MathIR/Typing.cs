namespace Lovelace.MathIR;

/// <summary>Scalar MathIR value types (the ordered types are Integer/Rational/Real).</summary>
public enum IrType { Integer, Rational, Real, Complex, Bool }

/// <summary>
/// MathIR type inference and validation (Phase 5). Types are inferred forward over the node
/// list (children precede parents); validation enforces operand ranges, op arity, ordered
/// comparisons over ordered types only, and Bool guards on Select.
/// </summary>
public static class IrTyping
{
    /// <summary>Forward type inference; null = unconstrained (parameters) or unknown.</summary>
    public static IrType?[] Infer(IrProgram prog)
    {
        var types = new IrType?[prog.Nodes.Count];
        for (int i = 0; i < prog.Nodes.Count; i++)
        {
            var n = prog.Nodes[i];
            IrType? T(int idx) => types[n.Operands[idx]];
            types[i] = n.Op switch
            {
                IrOpKind.Constant => ClassifyConstant(prog.Constants[n.Operands[0]]),
                IrOpKind.Parameter => null,
                IrOpKind.Add or IrOpKind.Sub or IrOpKind.Mul or IrOpKind.Min or IrOpKind.Max => Promote(T(0), T(1)),
                IrOpKind.Negate => T(0),
                IrOpKind.PowInt => T(0),
                IrOpKind.Div or IrOpKind.Reciprocal or IrOpKind.Pow or IrOpKind.Sqrt
                    or IrOpKind.Exp or IrOpKind.Log or IrOpKind.Sin or IrOpKind.Cos or IrOpKind.Tan
                    or IrOpKind.Asin or IrOpKind.Acos or IrOpKind.Atan or IrOpKind.Sinh or IrOpKind.Cosh or IrOpKind.Tanh
                    => n.Op == IrOpKind.Pow ? Transcendental(T(0), T(1)) : Transcendental(T(0)),
                IrOpKind.Abs => IrType.Real,
                IrOpKind.Sign => T(0) == IrType.Complex ? IrType.Complex : IrType.Integer,
                IrOpKind.Floor or IrOpKind.Ceil => T(0) == IrType.Complex ? null : IrType.Integer,
                IrOpKind.Eq or IrOpKind.Ne or IrOpKind.Lt or IrOpKind.Le or IrOpKind.Gt or IrOpKind.Ge => IrType.Bool,
                IrOpKind.Select => Promote(T(1), T(2)),
                _ => null,
            };
        }
        return types;
    }

    /// <summary>Throws InvalidOperationException on any structural or type violation.</summary>
    public static void Validate(IrProgram prog)
    {
        // structural pass first: arity, operand ranges, pool/parameter ranges — before any
        // type inference touches the operand indices
        for (int i = 0; i < prog.Nodes.Count; i++)
        {
            var n = prog.Nodes[i];
            int arity = n.Op switch
            {
                IrOpKind.Constant or IrOpKind.Parameter
                    or IrOpKind.Negate or IrOpKind.Reciprocal or IrOpKind.Sqrt
                    or IrOpKind.Exp or IrOpKind.Log or IrOpKind.Sin or IrOpKind.Cos or IrOpKind.Tan
                    or IrOpKind.Asin or IrOpKind.Acos or IrOpKind.Atan or IrOpKind.Sinh or IrOpKind.Cosh or IrOpKind.Tanh
                    or IrOpKind.Abs or IrOpKind.Sign or IrOpKind.Floor or IrOpKind.Ceil => 1,
                IrOpKind.Add or IrOpKind.Sub or IrOpKind.Mul or IrOpKind.Div or IrOpKind.Pow or IrOpKind.PowInt
                    or IrOpKind.Min or IrOpKind.Max
                    or IrOpKind.Eq or IrOpKind.Ne or IrOpKind.Lt or IrOpKind.Le or IrOpKind.Gt or IrOpKind.Ge => 2,
                IrOpKind.Select => 3,
                _ => 0,
            };
            if (n.Operands.Length != arity)
                throw new InvalidOperationException($"IR validation: node {i} ({n.Op}) has {n.Operands.Length} operands; expected {arity}.");
            foreach (var o in n.Operands)
            {
                if (o < 0 || o >= prog.Nodes.Count)
                    throw new InvalidOperationException($"IR validation: node {i} ({n.Op}) references out-of-range operand {o}.");
            }
            if (n.Op == IrOpKind.Constant && (n.Operands[0] < 0 || n.Operands[0] >= prog.Constants.Count))
                throw new InvalidOperationException($"IR validation: Constant node {i} references out-of-range pool index {n.Operands[0]}.");
            if (n.Op == IrOpKind.Parameter && (n.Operands[0] < 0 || n.Operands[0] >= prog.Parameters.Count))
                throw new InvalidOperationException($"IR validation: Parameter node {i} references out-of-range parameter index {n.Operands[0]}.");
        }

        // type-rule pass (safe: every index already range-checked)
        var types = Infer(prog);
        for (int i = 0; i < prog.Nodes.Count; i++)
        {
            var n = prog.Nodes[i];
            if (n.Op is IrOpKind.Lt or IrOpKind.Le or IrOpKind.Gt or IrOpKind.Ge or IrOpKind.Min or IrOpKind.Max)
            {
                foreach (var o in n.Operands)
                {
                    if (types[o] == IrType.Complex)
                        throw new InvalidOperationException($"IR validation: ordered comparison {n.Op} requires real operands, node {i} operand {o} is Complex.");
                }
            }
            if (n.Op == IrOpKind.Select && types[n.Operands[0]] != IrType.Bool)
                throw new InvalidOperationException($"IR validation: Select node {i} requires a Bool guard, got {types[n.Operands[0]]?.ToString() ?? "unconstrained"}.");
        }
    }

    private static IrType ClassifyConstant(string canonical)
    {
        if (canonical.StartsWith("(int ", StringComparison.Ordinal))
            return IrType.Integer;
        if (canonical.StartsWith("(rat ", StringComparison.Ordinal))
        {
            // rational with denominator 1 is an integer value
            var parts = canonical.TrimEnd(')').Split(' ');
            if (parts.Length >= 3 && parts[2] == "1")
                return IrType.Integer;
            return IrType.Rational;
        }
        if (canonical.StartsWith("(real ", StringComparison.Ordinal))
            return IrType.Real;
        if (canonical.StartsWith("(cplx ", StringComparison.Ordinal))
            return IrType.Complex;
        // named constants: Pi/E -> Real, I -> Complex, Infinity -> Real
        return canonical == "I" ? IrType.Complex : IrType.Real;
    }

    private static IrType? Promote(IrType? a, IrType? b)
    {
        if (a == IrType.Complex || b == IrType.Complex) return IrType.Complex;
        if (a == IrType.Real || b == IrType.Real) return IrType.Real;
        if (a == IrType.Rational || b == IrType.Rational) return IrType.Rational;
        if (a == IrType.Integer && b == IrType.Integer) return IrType.Integer;
        if (a is null || b is null) return a ?? b;
        return null;
    }

    private static IrType? Transcendental(params IrType?[] inputs)
    {
        foreach (var t in inputs)
        {
            if (t == IrType.Complex) return IrType.Complex;
        }
        foreach (var t in inputs)
        {
            if (t is null) return null;
        }
        return IrType.Real;
    }
}
