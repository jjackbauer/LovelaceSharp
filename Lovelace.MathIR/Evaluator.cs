using Lovelace.Abstractions;
using Lovelace.Symbolics;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.MathIR;

/// <summary>
/// MathIR execution: an iterative demand-driven interpreter over the typed DAG. Select is
/// LAZY: only the taken branch is evaluated, so piecewise(x = 0: 1, otherwise 1/x) evaluates
/// to 1 at x = 0 instead of throwing in the untaken branch.
/// </summary>
public static class IrEvaluator
{
    public static Num Evaluate(IrProgram prog, IReadOnlyDictionary<Symbol, Num> bindings, ExprContext ctx)
    {
        var memo = new Num?[prog.Nodes.Count];
        var constants = new Num[prog.Constants.Count];
        for (int i = 0; i < prog.Constants.Count; i++)
            constants[i] = ParseConstant(prog.Constants[i], ctx);

        int root = prog.Nodes.Count - 1;
        var work = new Stack<int>();
        work.Push(root);
        long pushes = 1;
        while (work.Count > 0)
        {
            if (++pushes > (long)prog.Nodes.Count * 64 + 64)
                throw new InvalidOperationException(
                    $"IR evaluator cycle near node {work.Peek()} (op {prog.Nodes[work.Peek()].Op}).");
            int i = work.Peek();
            if (memo[i] is not null)
            {
                work.Pop();
                continue;
            }
            var n = prog.Nodes[i];

            // Constant/Parameter operands index the constant pool / parameter list, not
            // the node array — they are always immediately computable
            if (n.Op is IrOpKind.Constant or IrOpKind.Parameter)
            {
                memo[i] = Compute(prog, n, memo, constants, bindings, ctx);
                work.Pop();
                continue;
            }

            // lazy Select: only the guard and the taken branch are ever demanded
            if (n.Op == IrOpKind.Select)
            {
                int guard = n.Operands[0];
                if (memo[guard] is null)
                {
                    work.Push(guard);
                    continue;
                }
                int taken = NumOps.IsZero(memo[guard]!) ? n.Operands[2] : n.Operands[1];
                if (memo[taken] is null)
                {
                    work.Push(taken);
                    continue;
                }
                memo[i] = memo[taken];
                work.Pop();
                continue;
            }

            bool ready = true;
            foreach (var o in n.Operands)
            {
                if (memo[o] is null)
                {
                    work.Push(o);
                    ready = false;
                }
            }
            if (!ready)
                continue;

            memo[i] = Compute(prog, n, memo, constants, bindings, ctx);
            work.Pop();
        }
        return memo[root] ?? throw new InvalidOperationException("Empty IR program.");
    }

    private static Num Compute(
        IrProgram prog, IrNode n, Num?[] memo, Num[] constants,
        IReadOnlyDictionary<Symbol, Num> bindings, ExprContext ctx)
    {
        Num O(int idx) => memo[n.Operands[idx]]!;
        return n.Op switch
        {
            IrOpKind.Constant => constants[n.Operands[0]],
            IrOpKind.Parameter => LookupBinding(bindings, prog.Parameters[n.Operands[0]]),
            IrOpKind.Add => NumOps.Add(O(0), O(1)),
            IrOpKind.Sub => NumOps.Subtract(O(0), O(1)),
            IrOpKind.Mul => NumOps.Multiply(O(0), O(1)),
            IrOpKind.Div => NumOps.Divide(O(0), O(1)),
            IrOpKind.Negate => NumOps.Negate(O(0)),
            IrOpKind.Reciprocal => NumOps.Divide(NumOps.FromLong(1), O(0)),
            IrOpKind.PowInt => NumOps.PowInt(O(0), O(1)),
            IrOpKind.Pow => NumOps.Pow(O(0), O(1)),
            IrOpKind.Sqrt => NumOps.Pow(O(0), new NumRat(Rat.From(1, 2))),
            IrOpKind.Exp => NumOps.Exp(O(0), ctx),
            IrOpKind.Log => NumOps.Ln(O(0), ctx),
            IrOpKind.Sin => NumOps.Sin(O(0), ctx),
            IrOpKind.Cos => NumOps.Cos(O(0), ctx),
            IrOpKind.Tan => NumOps.Tan(O(0), ctx),
            IrOpKind.Asin => NumOps.Asin(O(0), ctx),
            IrOpKind.Acos => NumOps.Acos(O(0), ctx),
            IrOpKind.Atan => NumOps.Atan(O(0), ctx),
            IrOpKind.Sinh => NumOps.Sinh(O(0), ctx),
            IrOpKind.Cosh => NumOps.Cosh(O(0), ctx),
            IrOpKind.Tanh => NumOps.Tanh(O(0), ctx),
            IrOpKind.Abs => NumOps.Abs(O(0), ctx),
            IrOpKind.Sign => NumOps.Sign(O(0), ctx),
            IrOpKind.Floor => NumOps.Floor(O(0), ctx),
            IrOpKind.Ceil => NumOps.Ceil(O(0), ctx),
            IrOpKind.Min => NumOps.Min(new[] { O(0), O(1) }, ctx),
            IrOpKind.Max => NumOps.Max(new[] { O(0), O(1) }, ctx),
            IrOpKind.Eq => BoolNum(NumOps.Compare(O(0), O(1)) == 0),
            IrOpKind.Ne => BoolNum(NumOps.Compare(O(0), O(1)) != 0),
            IrOpKind.Lt => BoolNum(NumOps.Compare(O(0), O(1)) < 0),
            IrOpKind.Le => BoolNum(NumOps.Compare(O(0), O(1)) <= 0),
            IrOpKind.Gt => BoolNum(NumOps.Compare(O(0), O(1)) > 0),
            IrOpKind.Ge => BoolNum(NumOps.Compare(O(0), O(1)) >= 0),
            IrOpKind.Select => throw new InvalidOperationException("Select handled by the lazy path."),
            _ => throw new InvalidOperationException($"Unknown IR op {n.Op}."),
        };
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
        c.RegisterBuiltin("compile", new[] { "f", "params" }, args =>
        {
            Exprs.Current = _symbolics.Context;
            var f = AsExpr(args[0]!);
            var names = (IReadOnlyList<object?>)args[1]!;
            var ps = names.Select(n => Exprs.Current.Symbol(NameOf(n))).ToArray();
            return Compilation.Compile(f, Exprs.Current, ps).IrText.TrimEnd('\n');
        });
        c.RegisterBuiltin("evalir_batch", new[] { "ir", "values", "digits" }, args =>
        {
            Exprs.Current = _symbolics.Context;
            var prog = IrProgram.Deserialize((string)args[0]!);
            var values = (IReadOnlyList<object?>)args[1]!;
            var digits = (int)AsLong(args[2]!);
            // flat row-major values: width = number of parameters, one row per width entries
            int width = prog.Parameters.Count;
            if (values.Count == 0 || values.Count % width != 0)
                throw new InvalidOperationException($"Values must be a multiple of {width} (the parameter count).");
            int rows = values.Count / width;
            var columns = new Dictionary<string, Num[]>(width);
            for (int p = 0; p < width; p++)
                columns[prog.Parameters[p]] = new Num[rows];
            for (int row = 0; row < rows; row++)
                for (int p = 0; p < width; p++)
                    columns[prog.Parameters[p]][row] = ToNum(values[row * width + p]!);
            using (global::Lovelace.Real.Real.WithPrecision(digits, Math.Min(digits, 50)))
            {
                var results = new CompiledKernel(prog, prog.Parameters.Select(n => Exprs.Current.Symbol(n)).ToArray(), Exprs.Current)
                    .EvaluateBatch(columns);
                return results.Select(ToPayload).ToArray();
            }
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
