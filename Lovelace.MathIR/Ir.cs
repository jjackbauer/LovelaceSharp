using System.Text;
using Lovelace.Symbolics;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.MathIR;

public enum IrOpKind
{
    Constant, Parameter,
    Add, Sub, Mul, Div, Negate, Reciprocal,
    PowInt, Pow, Sqrt,
    Exp, Log, Sin, Cos, Tan, Asin, Acos, Atan, Sinh, Cosh, Tanh,
    Abs, Sign, Floor, Ceil, Min, Max,
    Eq, Ne, Lt, Le, Gt, Ge, Select,
}

public sealed record IrNode(IrOpKind Op, int[] Operands, int Aux);

/// <summary>
/// A typed computational DAG below the symbolic layer (architecture §17): expression DAG with
/// explicit CSE sharing, a constant pool, and versioned text serialization. Assumptions are
/// erased at lowering (they licensed the transformations; the optimizer's provenance records
/// them).
/// </summary>
public sealed class IrProgram
{
    public const int FormatVersion = 1;

    public List<string> Constants { get; } = new();
    public List<string> Parameters { get; } = new();
    public List<IrNode> Nodes { get; } = new();

    public string Serialize()
    {
        var sb = new StringBuilder();
        sb.Append("#!mathir ").Append(FormatVersion).Append('\n');
        foreach (var p in Parameters)
            sb.Append("param ").Append(p).Append('\n');
        foreach (var c in Constants)
            sb.Append("const ").Append(c).Append('\n');
        foreach (var n in Nodes)
        {
            sb.Append(n.Op).Append(' ').Append(n.Aux).Append(' ').Append(string.Join(",", n.Operands)).Append('\n');
        }
        return sb.ToString();
    }

    public static IrProgram Deserialize(string text)
    {
        var p = new IrProgram();
        foreach (var raw in text.Split('\n'))
        {
            var line = raw.Trim();
            if (line.Length == 0 || line.StartsWith('#'))
                continue;
            if (line.StartsWith("param ", StringComparison.Ordinal))
            {
                p.Parameters.Add(line["param ".Length..]);
                continue;
            }
            if (line.StartsWith("const ", StringComparison.Ordinal))
            {
                p.Constants.Add(line["const ".Length..]);
                continue;
            }
            int sp1 = line.IndexOf(' ');
            int sp2 = line.IndexOf(' ', sp1 + 1);
            if (sp1 < 0 || sp2 < 0)
                throw new FormatException($"Malformed IR line: {line}");
            var op = Enum.Parse<IrOpKind>(line[..sp1]);
            var aux = int.Parse(line[(sp1 + 1)..sp2]);
            var opsText = line[(sp2 + 1)..];
            var ops = opsText.Length > 0
                ? opsText.Split(',').Select(int.Parse).ToArray()
                : Array.Empty<int>();
            p.Nodes.Add(new IrNode(op, ops, aux));
        }
        return p;
    }
}

/// <summary>Lowering from the symbolic DAG to MathIR.</summary>
public static class Lowering
{
    public static IrProgram Lower(Expr e, ExprContext ctx, Symbol[] parameters)
    {
        var prog = new IrProgram();
        var cache = new Dictionary<Expr, int>();
        foreach (var p in parameters)
            prog.Parameters.Add(p.Name);

        var paramIndex = new Dictionary<string, int>();
        for (int i = 0; i < parameters.Length; i++)
            paramIndex[parameters[i].Name] = i;

        Emit(e, ctx, prog, cache, paramIndex);
        return prog;
    }

    private static int Emit(Expr e, ExprContext ctx, IrProgram prog, Dictionary<Expr, int> cache, Dictionary<string, int> paramIndex)
    {
        if (cache.TryGetValue(e, out var hit))
            return hit;

        int result;
        switch (e)
        {
            case SymbolExpr s when paramIndex.TryGetValue(s.Symbol.Name, out var pi):
                result = AddNode(prog, new IrNode(IrOpKind.Parameter, new[] { pi }, 0));
                break;
            case IntegerConstantExpr i:
            case RationalConstantExpr r:
            case RealConstantExpr:
            case ComplexConstantExpr:
            {
                var canon = Printing.CanonicalPrint(e);
                int idx = prog.Constants.IndexOf(canon);
                if (idx < 0)
                {
                    idx = prog.Constants.Count;
                    prog.Constants.Add(canon);
                }
                result = AddNode(prog, new IrNode(IrOpKind.Constant, new[] { idx }, 0));
                break;
            }
            case NamedConstantExpr n:
            {
                int idx = prog.Constants.IndexOf(n.Constant.ToString());
                if (idx < 0)
                {
                    idx = prog.Constants.Count;
                    prog.Constants.Add(n.Constant.ToString());
                }
                result = AddNode(prog, new IrNode(IrOpKind.Constant, new[] { idx }, 1));
                break;
            }
            case AddExpr a:
                result = EmitNAry(a.Terms, IrOpKind.Add, e, ctx, prog, cache, paramIndex);
                break;
            case MultiplyExpr m:
                result = EmitNAry(m.Factors, IrOpKind.Mul, e, ctx, prog, cache, paramIndex);
                break;
            case PowerExpr p:
            {
                var b = Emit(p.Base, ctx, prog, cache, paramIndex);
                if (p.Exponent is RationalConstantExpr re)
                {
                    if (re.Value.IsMinusOne)
                        result = AddNode(prog, new IrNode(IrOpKind.Reciprocal, new[] { b }, 0));
                    else if (re.Value == RatHalf)
                        result = AddNode(prog, new IrNode(IrOpKind.Sqrt, new[] { b }, 0));
                    else if (re.Value.IsInteger)
                    {
                        int exp = (int)re.Value.ToInteger().ToInt64Saturating();
                        result = EmitPowInt(prog, b, exp);
                    }
                    else
                    {
                        var ex = Emit(p.Exponent, ctx, prog, cache, paramIndex);
                        result = AddNode(prog, new IrNode(IrOpKind.Pow, new[] { b, ex }, 0));
                    }
                }
                else
                {
                    var ex = Emit(p.Exponent, ctx, prog, cache, paramIndex);
                    result = AddNode(prog, new IrNode(IrOpKind.Pow, new[] { b, ex }, 0));
                }
                break;
            }
            case FunctionExpr f:
            {
                var args = f.Arguments.Select(a => Emit(a, ctx, prog, cache, paramIndex)).ToArray();
                var op = f.Function.Name switch
                {
                    "exp" => IrOpKind.Exp,
                    "log" => IrOpKind.Log,
                    "sin" => IrOpKind.Sin,
                    "cos" => IrOpKind.Cos,
                    "tan" => IrOpKind.Tan,
                    "asin" => IrOpKind.Asin,
                    "acos" => IrOpKind.Acos,
                    "atan" => IrOpKind.Atan,
                    "sinh" => IrOpKind.Sinh,
                    "cosh" => IrOpKind.Cosh,
                    "tanh" => IrOpKind.Tanh,
                    "abs" => IrOpKind.Abs,
                    "sign" => IrOpKind.Sign,
                    "floor" => IrOpKind.Floor,
                    "ceil" => IrOpKind.Ceil,
                    "min" => IrOpKind.Min,
                    "max" => IrOpKind.Max,
                    _ => throw new InvalidOperationException($"Function '{f.Function.Name}' cannot be lowered."),
                };
                result = AddNode(prog, new IrNode(op, args, 0));
                break;
            }
            case PiecewiseExpr pw:
            {
                var otherwise = Emit(pw.Otherwise, ctx, prog, cache, paramIndex);
                result = otherwise;
                for (int i = pw.Branches.Length - 1; i >= 0; i--)
                {
                    var guard = EmitGuard(pw.Branches[i].Guard, ctx, prog, cache, paramIndex);
                    var value = Emit(pw.Branches[i].Value, ctx, prog, cache, paramIndex);
                    result = AddNode(prog, new IrNode(IrOpKind.Select, new[] { guard, value, result }, 0));
                }
                break;
            }
            default:
                throw new InvalidOperationException($"Node kind {e.Kind} cannot be lowered to MathIR.");
        }
        cache[e] = result;
        return result;
    }

    private static int EmitGuard(Expr guard, ExprContext ctx, IrProgram prog, Dictionary<Expr, int> cache, Dictionary<string, int> paramIndex)
    {
        if (guard is RelationExpr r)
        {
            var l = Emit(r.Left, ctx, prog, cache, paramIndex);
            var rr = Emit(r.Right, ctx, prog, cache, paramIndex);
            var op = r.Op switch
            {
                RelOp.Eq => IrOpKind.Eq,
                RelOp.Ne => IrOpKind.Ne,
                RelOp.Lt => IrOpKind.Lt,
                RelOp.Le => IrOpKind.Le,
                RelOp.Gt => IrOpKind.Gt,
                _ => IrOpKind.Ge,
            };
            return AddNode(prog, new IrNode(op, new[] { l, rr }, 0));
        }
        throw new InvalidOperationException("Piecewise guards must be relations.");
    }

    private static int EmitNAry(IEnumerable<Expr> items, IrOpKind op, Expr whole, ExprContext ctx, IrProgram prog, Dictionary<Expr, int> cache, Dictionary<string, int> paramIndex)
    {
        var list = items.ToArray();
        if (list.Length == 0)
        {
            // identity constant
            int idx = AddConstant(prog, "0");
            return AddNode(prog, new IrNode(IrOpKind.Constant, new[] { idx }, 0));
        }
        var emitted = list.Select(x => Emit(x, ctx, prog, cache, paramIndex)).ToArray();
        int acc = emitted[0];
        for (int i = 1; i < emitted.Length; i++)
            acc = AddNode(prog, new IrNode(op, new[] { acc, emitted[i] }, 0));
        return acc;
    }

    private static int EmitPowInt(IrProgram prog, int b, int exp)
    {
        // binary exponentiation chain: x^n via squaring (the power-chain optimization)
        if (exp == 0)
            return AddConstant(prog, "1");
        if (exp == 1)
            return b;
        var neg = exp < 0;
        var n = neg ? -exp : exp;
        int result = -1;
        int x = b;
        while (n > 0)
        {
            if ((n & 1) == 1)
                result = result < 0 ? x : AddNode(prog, new IrNode(IrOpKind.Mul, new[] { result, x }, 0));
            n >>= 1;
            if (n > 0)
                x = AddNode(prog, new IrNode(IrOpKind.Mul, new[] { x, x }, 0));
        }
        return neg ? AddNode(prog, new IrNode(IrOpKind.Reciprocal, new[] { result }, 0)) : result;
    }

    private static int AddConstant(IrProgram prog, string canon)
    {
        int idx = prog.Constants.IndexOf(canon);
        if (idx < 0)
        {
            idx = prog.Constants.Count;
            prog.Constants.Add(canon);
        }
        return AddNode(prog, new IrNode(IrOpKind.Constant, new[] { idx }, 0));
    }

    private static int AddNode(IrProgram prog, IrNode n)
    {
        prog.Nodes.Add(n);
        return prog.Nodes.Count - 1;
    }

    private static readonly Rat RatHalf = Rat.From(1, 2);
}
