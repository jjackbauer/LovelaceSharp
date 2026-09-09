using Lovelace.Abstractions;
using Lovelace.Symbolics;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.MathIR;

/// <summary>MathIR execution: a portable recursive interpreter over the typed DAG.</summary>
public static class IrEvaluator
{
    public static Num Evaluate(IrProgram prog, IReadOnlyDictionary<Symbol, Num> bindings, ExprContext ctx)
    {
        var stack = new Num[prog.Nodes.Count];
        var constants = new Num[prog.Constants.Count];
        for (int i = 0; i < prog.Constants.Count; i++)
            constants[i] = ParseConstant(prog.Constants[i], ctx);

        for (int i = 0; i < prog.Nodes.Count; i++)
        {
            var n = prog.Nodes[i];
            stack[i] = n.Op switch
            {
                IrOpKind.Constant => constants[n.Operands[0]],
                IrOpKind.Parameter => LookupBinding(bindings, prog.Parameters[n.Operands[0]]),
                IrOpKind.Add => NumOps.Add(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Sub => NumOps.Subtract(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Mul => NumOps.Multiply(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Div => NumOps.Divide(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Negate => NumOps.Negate(stack[n.Operands[0]]),
                IrOpKind.Reciprocal => NumOps.Divide(NumOps.FromLong(1), stack[n.Operands[0]]),
                IrOpKind.PowInt => NumOps.PowInt(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Pow => NumOps.Pow(stack[n.Operands[0]], stack[n.Operands[1]]),
                IrOpKind.Sqrt => NumOps.Pow(stack[n.Operands[0]], new NumRat(Rat.From(1, 2))),
                IrOpKind.Exp => NumOps.Exp(stack[n.Operands[0]], ctx),
                IrOpKind.Log => NumOps.Ln(stack[n.Operands[0]], ctx),
                IrOpKind.Sin => NumOps.Sin(stack[n.Operands[0]], ctx),
                IrOpKind.Cos => NumOps.Cos(stack[n.Operands[0]], ctx),
                IrOpKind.Tan => NumOps.Tan(stack[n.Operands[0]], ctx),
                IrOpKind.Asin => NumOps.Asin(stack[n.Operands[0]], ctx),
                IrOpKind.Acos => NumOps.Acos(stack[n.Operands[0]], ctx),
                IrOpKind.Atan => NumOps.Atan(stack[n.Operands[0]], ctx),
                IrOpKind.Sinh => NumOps.Sinh(stack[n.Operands[0]], ctx),
                IrOpKind.Cosh => NumOps.Cosh(stack[n.Operands[0]], ctx),
                IrOpKind.Tanh => NumOps.Tanh(stack[n.Operands[0]], ctx),
                IrOpKind.Abs => NumOps.Abs(stack[n.Operands[0]], ctx),
                IrOpKind.Sign => NumOps.Sign(stack[n.Operands[0]], ctx),
                IrOpKind.Floor => NumOps.Floor(stack[n.Operands[0]], ctx),
                IrOpKind.Ceil => NumOps.Ceil(stack[n.Operands[0]], ctx),
                IrOpKind.Min => NumOps.Min(new[] { stack[n.Operands[0]], stack[n.Operands[1]] }, ctx),
                IrOpKind.Max => NumOps.Max(new[] { stack[n.Operands[0]], stack[n.Operands[1]] }, ctx),
                IrOpKind.Eq => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) == 0),
                IrOpKind.Ne => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) != 0),
                IrOpKind.Lt => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) < 0),
                IrOpKind.Le => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) <= 0),
                IrOpKind.Gt => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) > 0),
                IrOpKind.Ge => BoolNum(NumOps.Compare(stack[n.Operands[0]], stack[n.Operands[1]]) >= 0),
                IrOpKind.Select => NumOps.IsZero(stack[n.Operands[0]]) ? stack[n.Operands[2]] : stack[n.Operands[1]],
                _ => throw new InvalidOperationException($"Unknown IR op {n.Op}."),
            };
        }
        return stack[^1];
    }

    private static Num LookupBinding(IReadOnlyDictionary<Symbol, Num> bindings, string name)
    {
        foreach (var (s, v) in bindings)
            if (s.Name == name)
                return v;
        throw new InvalidOperationException($"No value for parameter '{name}'.");
    }

    private static Num BoolNum(bool b) => NumOps.FromLong(b ? 1 : 0);

    private static Num ParseConstant(string canon, ExprContext ctx)
    {
        // constants are stored as canonical symbolic text (aux 0) or a named-constant tag (aux 1)
        if (canon is "Pi" or "E" or "I" or "Infinity")
        {
            return canon switch
            {
                "Pi" => new NumReal(global::Lovelace.Real.Real.Pi),
                "E" => new NumReal(global::Lovelace.Real.Real.E),
                "I" => new NumComplex(global::Lovelace.Complex.Complex.I),
                _ => throw new InvalidOperationException("Infinity has no numeric value."),
            };
        }
        var expr = Printing.CanonicalParse(canon, ctx);
        return Evaluation.EvaluateToNum(expr, ctx, new Dictionary<Symbol, Num>());
    }
}

/// <summary>The language-facing surface for MathIR: lower/evalir builtins (second plugin).</summary>
public sealed class MathIRPlugin : IModusPlugin
{
    public string Name => "Lovelace.MathIR";

    private readonly SymbolicsPlugin _symbolics;

    public MathIRPlugin(SymbolicsPlugin symbolics) => _symbolics = symbolics;

    public void Register(IModusContext c)
    {
        c.RegisterBuiltin("lower", new[] { "f", "params" }, args =>
        {
            Exprs.Current = _symbolics.Context;
            var f = AsExpr(args[0]!);
            var names = (IReadOnlyList<object?>)args[1]!;
            var ps = names.Select(n => Exprs.Current.Symbol(NameOf(n))).ToArray();
            var prog = Lowering.Lower(f, Exprs.Current, ps);
            return prog.Serialize().TrimEnd('\n');
        });
        c.RegisterBuiltin("evalir", new[] { "ir", "values", "digits" }, args =>
        {
            Exprs.Current = _symbolics.Context;
            var prog = IrProgram.Deserialize((string)args[0]!);
            var values = (IReadOnlyList<object?>)args[1]!;
            var digits = (int)AsLong(args[2]!);
            var bindings = new Dictionary<Symbol, Num>();
            for (int i = 0; i < prog.Parameters.Count && i < values.Count; i++)
                bindings[Exprs.Current.Symbol(prog.Parameters[i])] = ToNum(values[i]!);
            using (global::Lovelace.Real.Real.WithPrecision(digits, Math.Min(digits, 50)))
            {
                var result = IrEvaluator.Evaluate(prog, bindings, Exprs.Current);
                return ToPayload(result);
            }
        });
    }

    private static Num ToNum(object? o) => o switch
    {
        global::Lovelace.Real.Real r => new NumReal(r),
        global::Lovelace.Integer.Integer i => new NumInt(i),
        global::Lovelace.Natural.Natural n => new NumInt(new global::Lovelace.Integer.Integer(n)),
        global::Lovelace.Complex.Complex c => new NumComplex(c),
        _ => throw new InvalidOperationException("Expected a numeric value."),
    };

    private static object ToPayload(Num n) => n switch
    {
        NumInt i => i.V,
        NumRat r => r.V.IsInteger ? r.V.ToInteger() : RationalReal.ToReal(r.V, (int)Math.Min(global::Lovelace.Real.Real.MaxComputationDecimalPlaces, 1000)),
        NumReal rl => rl.V,
        NumComplex c => c.V,
        _ => throw new InvalidOperationException(),
    };

    private static string NameOf(object? o) => o switch
    {
        string s => s,
        SymbolExpr sx => sx.Symbol.Name,
        _ => throw new InvalidOperationException("Expected symbol names."),
    };

    private static Expr AsExpr(object? o) => o switch
    {
        Expr e => e,
        _ => throw new InvalidOperationException("Expected a symbolic expression."),
    };

    private static long AsLong(object? o) => o switch
    {
        global::Lovelace.Real.Real r => long.Parse(r.ToString().Split('.')[0]),
        global::Lovelace.Integer.Integer i => long.Parse(i.ToString()),
        global::Lovelace.Natural.Natural n => long.Parse(n.ToString()),
        long l => l,
        _ => throw new InvalidOperationException("Expected an integer."),
    };
}
