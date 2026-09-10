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

    /// <summary>
    /// Vectorized batch evaluation (Q8): one traversal of the DAG carrying Num[] lanes per node
    /// instead of re-evaluating per lane. Elementwise semantics per lane; Select is lazy per
    /// branch set (when every lane takes the same branch only that branch is evaluated; mixed
    /// guards evaluate both). Constants broadcast across lanes.
    /// </summary>
    public static Num[] EvaluateBatch(
        IrProgram prog, IReadOnlyDictionary<string, Num[]> columns, ExprContext ctx)
    {
        int lanes = columns.Count == 0 ? 0 : columns.Values.Select(c => c.Length).Max();
        foreach (var (name, column) in columns)
        {
            if (column.Length != lanes)
                throw new ArgumentException($"Column '{name}' has {column.Length} lanes; expected {lanes}.");
        }
        if (lanes == 0)
            return Array.Empty<Num>();

        var memo = new Num[]?[prog.Nodes.Count];
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
                throw new InvalidOperationException($"IR evaluator cycle near node {work.Peek()}.");
            int i = work.Peek();
            if (memo[i] is not null)
            {
                work.Pop();
                continue;
            }
            var n = prog.Nodes[i];

            if (n.Op is IrOpKind.Constant or IrOpKind.Parameter)
            {
                memo[i] = ComputeVector(prog, n, memo, constants, columns, ctx, lanes);
                work.Pop();
                continue;
            }

            if (n.Op == IrOpKind.Select)
            {
                int guard = n.Operands[0];
                if (memo[guard] is null)
                {
                    work.Push(guard);
                    continue;
                }
                var cond = memo[guard]!;
                bool anyLeft = false, anyRight = false;
                for (int lane = 0; lane < lanes; lane++)
                {
                    if (NumOps.IsZero(cond[lane])) anyRight = true;
                    else anyLeft = true;
                }
                if (anyLeft && memo[n.Operands[1]] is null)
                {
                    work.Push(n.Operands[1]);
                    continue;
                }
                if (anyRight && memo[n.Operands[2]] is null)
                {
                    work.Push(n.Operands[2]);
                    continue;
                }
                var result = new Num[lanes];
                for (int lane = 0; lane < lanes; lane++)
                {
                    if (NumOps.IsZero(cond[lane]))
                        result[lane] = anyRight ? memo[n.Operands[2]]![lane] : NumOps.FromLong(0);
                    else
                        result[lane] = anyLeft ? memo[n.Operands[1]]![lane] : NumOps.FromLong(0);
                }
                memo[i] = result;
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

            memo[i] = ComputeVector(prog, n, memo, constants, columns, ctx, lanes);
            work.Pop();
        }
        return memo[root]!;
    }

    private static Num[] ComputeVector(
        IrProgram prog, IrNode n, Num[]?[] memo, Num[] constants,
        IReadOnlyDictionary<string, Num[]> columns, ExprContext ctx, int lanes)
    {
        Num[]? O(int idx) => memo[n.Operands[idx]];
        Num[] Fill(Num scalar)
        {
            var v = new Num[lanes];
            for (int i = 0; i < lanes; i++)
                v[i] = scalar;
            return v;
        }
        switch (n.Op)
        {
            case IrOpKind.Constant:
                return Fill(constants[n.Operands[0]]);
            case IrOpKind.Parameter:
                return columns.TryGetValue(prog.Parameters[n.Operands[0]], out var col)
                    ? col
                    : throw new InvalidOperationException($"Missing column for parameter '{prog.Parameters[n.Operands[0]]}'.");
            case IrOpKind.Add: return Zip(O(0)!, O(1)!, (a, b) => NumOps.Add(a, b));
            case IrOpKind.Sub: return Zip(O(0)!, O(1)!, NumOps.Subtract);
            case IrOpKind.Mul: return Zip(O(0)!, O(1)!, NumOps.Multiply);
            case IrOpKind.Div: return Zip(O(0)!, O(1)!, NumOps.Divide);
            case IrOpKind.Negate: return Map(O(0)!, NumOps.Negate);
            case IrOpKind.Reciprocal: return Map(O(0)!, v => NumOps.Divide(NumOps.FromLong(1), v));
            case IrOpKind.PowInt: return Zip(O(0)!, O(1)!, NumOps.PowInt);
            case IrOpKind.Pow: return Zip(O(0)!, O(1)!, NumOps.Pow);
            case IrOpKind.Sqrt: return Map(O(0)!, v => NumOps.Pow(v, new NumRat(Rat.From(1, 2))));
            case IrOpKind.Exp: return Map(O(0)!, v => NumOps.Exp(v, ctx));
            case IrOpKind.Log: return Map(O(0)!, v => NumOps.Ln(v, ctx));
            case IrOpKind.Sin: return Map(O(0)!, v => NumOps.Sin(v, ctx));
            case IrOpKind.Cos: return Map(O(0)!, v => NumOps.Cos(v, ctx));
            case IrOpKind.Tan: return Map(O(0)!, v => NumOps.Tan(v, ctx));
            case IrOpKind.Asin: return Map(O(0)!, v => NumOps.Asin(v, ctx));
            case IrOpKind.Acos: return Map(O(0)!, v => NumOps.Acos(v, ctx));
            case IrOpKind.Atan: return Map(O(0)!, v => NumOps.Atan(v, ctx));
            case IrOpKind.Sinh: return Map(O(0)!, v => NumOps.Sinh(v, ctx));
            case IrOpKind.Cosh: return Map(O(0)!, v => NumOps.Cosh(v, ctx));
            case IrOpKind.Tanh: return Map(O(0)!, v => NumOps.Tanh(v, ctx));
            case IrOpKind.Abs: return Map(O(0)!, v => NumOps.Abs(v, ctx));
            case IrOpKind.Sign: return Map(O(0)!, v => NumOps.Sign(v, ctx));
            case IrOpKind.Floor: return Map(O(0)!, v => NumOps.Floor(v, ctx));
            case IrOpKind.Ceil: return Map(O(0)!, v => NumOps.Ceil(v, ctx));
            case IrOpKind.Min: return Zip(O(0)!, O(1)!, (a, b) => NumOps.Min(new[] { a, b }, ctx));
            case IrOpKind.Max: return Zip(O(0)!, O(1)!, (a, b) => NumOps.Max(new[] { a, b }, ctx));
            case IrOpKind.Eq: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) == 0));
            case IrOpKind.Ne: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) != 0));
            case IrOpKind.Lt: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) < 0));
            case IrOpKind.Le: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) <= 0));
            case IrOpKind.Gt: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) > 0));
            case IrOpKind.Ge: return Zip(O(0)!, O(1)!, (a, b) => BoolNum(NumOps.Compare(a, b) >= 0));
            default:
                throw new InvalidOperationException($"Unsupported vectorized IR op {n.Op}.");
        }
    }

    private static Num[] Map(Num[] v, Func<Num, Num> f)
    {
        var r = new Num[v.Length];
        for (int i = 0; i < v.Length; i++)
            r[i] = f(v[i]);
        return r;
    }

    private static Num[] Zip(Num[] a, Num[] b, Func<Num, Num, Num> f)
    {
        var r = new Num[a.Length];
        for (int i = 0; i < a.Length; i++)
            r[i] = f(a[i], b[i]);
        return r;
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
        c.RegisterBuiltin(new global::Lovelace.Abstractions.BuiltinDescriptor(
            "lower", new[] { "f", "params" }, global::Lovelace.Abstractions.BuiltinCategories.Compilation,
            "Lowers a symbolic expression to MathIR (the typed, validated computational DAG).",
            ["lower(x^2 + 1, [x])"], "Text", ["compile"]), args =>
        {
            var previous = Exprs.Current;
            Exprs.Current = _symbolics.Context;
            try
            {
                var f = AsExpr(args[0]!);
                var names = ListArg(args[1], "lower", 2, "a list of parameter symbols, e.g. [x]");
                var ps = names.Select(n => Exprs.Current.Symbol(NameOf(n, "lower"))).ToArray();
                var prog = Lowering.Lower(f, Exprs.Current, ps);
                return prog.Serialize().TrimEnd('\n');
            }
            finally
            {
                Exprs.Current = previous;
            }
        });
        c.RegisterBuiltin(new global::Lovelace.Abstractions.BuiltinDescriptor(
            "compile", new[] { "f", "params" }, global::Lovelace.Abstractions.BuiltinCategories.Compilation,
            "Compiles a symbolic expression to a validated MathIR kernel (serialized IR text). Use compile_full for the structured kernel metadata.",
            ["compile(x^2 + 1, [x])"], "Text", ["compile_full", "evalir", "evalir_batch"]), args =>
        {
            var previous = Exprs.Current;
            Exprs.Current = _symbolics.Context;
            try
            {
                var f = AsExpr(args[0]!);
                var names = ListArg(args[1], "compile", 2, "a list of parameter symbols, e.g. [x]");
                var ps = names.Select(n => Exprs.Current.Symbol(NameOf(n, "compile"))).ToArray();
                return Compilation.Compile(f, Exprs.Current, ps).IrText.TrimEnd('\n');
            }
            finally
            {
                Exprs.Current = previous;
            }
        });
        c.RegisterBuiltin(new global::Lovelace.Abstractions.BuiltinDescriptor(
            "compile_full", new[] { "f", "params" }, global::Lovelace.Abstractions.BuiltinCategories.Compilation,
            "Structured compile: a CompilationResult record with the IR, per-parameter name and domain, result domain and type, the precision and optimization policies, target, MathIR version, and exactness.",
            ["compile_full(x^2 + 1, [x])"], "CompilationResult", ["compile", "evalir_batch"]), args =>
        {
            var previous = Exprs.Current;
            Exprs.Current = _symbolics.Context;
            try
            {
                var f = AsExpr(args[0]!);
                var names = ListArg(args[1], "compile_full", 2, "a list of parameter symbols, e.g. [x]");
                var ps = names.Select(n => Exprs.Current.Symbol(NameOf(n, "compile_full"))).ToArray();
                var res = Compilation.Compile(f, Exprs.Current, ps);
                var types = IrTyping.Infer(res.Program);
                var resultType = types.Length > 0 && types[^1] is { } rt ? rt.ToString() : "unknown";
                var parameters = ps.Select(p => (object)new global::Lovelace.Abstractions.RecordValue(
                    "ParameterInfo",
                    new global::Lovelace.Abstractions.RecordField("name", p.Name),
                    new global::Lovelace.Abstractions.RecordField(
                        "domain", Lovelace.Symbolics.Domains.ToMathDomain(
                            Lovelace.Symbolics.Domains.DomainOf(Exprs.Symbol(p), Exprs.Current))))).ToArray();
                return new global::Lovelace.Abstractions.RecordValue("CompilationResult",
                    new global::Lovelace.Abstractions.RecordField("ir", res.IrText.TrimEnd('\n')),
                    new global::Lovelace.Abstractions.RecordField("parameters", parameters),
                    new global::Lovelace.Abstractions.RecordField(
                        "result_domain", Lovelace.Symbolics.Domains.ToMathDomain(
                            Lovelace.Symbolics.Domains.DomainOf(f, Exprs.Current))),
                    new global::Lovelace.Abstractions.RecordField("result_type", resultType),
                    new global::Lovelace.Abstractions.RecordField("precision_policy", PrecisionPolicy),
                    new global::Lovelace.Abstractions.RecordField("optimization_policy", OptimizationPolicy),
                    new global::Lovelace.Abstractions.RecordField("target", "mathir"),
                    new global::Lovelace.Abstractions.RecordField("mathir_version", 2),
                    new global::Lovelace.Abstractions.RecordField("exact", f.IsExact));
            }
            finally
            {
                Exprs.Current = previous;
            }
        });
        c.RegisterBuiltin(new global::Lovelace.Abstractions.BuiltinDescriptor(
            "evalir_batch", new[] { "ir", "values", "digits" }, global::Lovelace.Abstractions.BuiltinCategories.Compilation,
            "Batch-evaluates a compiled kernel (IR text or CompilationResult) over flat row-major values at the given precision.",
            ["k = compile(x^2 + 1, [x]); evalir_batch(k, [1, 2, 3], 40)"], "Vector", ["compile", "evalir"]), args =>
        {
            var previous = Exprs.Current;
            Exprs.Current = _symbolics.Context;
            try
            {
                var prog = IrProgram.Deserialize(IrTextOf(args[0]!));
                var digits = (int)AsLong(args[2]!);
                int width = prog.Parameters.Count;
                var (values, rows) = BatchRows(args[1], width);
                var columns = new Dictionary<string, Num[]>(width);
                for (int p = 0; p < width; p++)
                    columns[prog.Parameters[p]] = new Num[rows];
                for (int row = 0; row < rows; row++)
                {
                    // a cancelled batch stops between rows; the caller keeps the rows it has
                    global::Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
                    for (int p = 0; p < width; p++)
                        columns[prog.Parameters[p]][row] = ToNum(values[row * width + p]!);
                }
                using (global::Lovelace.Real.Real.WithPrecision(digits, Math.Min(digits, 50)))
                {
                    var results = new CompiledKernel(prog, prog.Parameters.Select(n => Exprs.Current.Symbol(n)).ToArray(), Exprs.Current)
                        .EvaluateBatch(columns);
                    return results.Select(ToPayload).ToArray();
                }
            }
            finally
            {
                Exprs.Current = previous;
            }
        });
        c.RegisterBuiltin(new global::Lovelace.Abstractions.BuiltinDescriptor(
            "evalir", new[] { "ir", "values", "digits" }, global::Lovelace.Abstractions.BuiltinCategories.Compilation,
            "Evaluates a compiled kernel (IR text or CompilationResult) at one point at the given precision.",
            ["evalir(compile(x^2 + 1, [x]), [3], 40)"], "Real | Complex", ["evalir_batch", "compile"]), args =>
        {
            var previous = Exprs.Current;
            Exprs.Current = _symbolics.Context;
            try
            {
                var prog = IrProgram.Deserialize(IrTextOf(args[0]!));
                var values = (IReadOnlyList<object?>)args[1]!;
                var digits = (int)AsLong(args[2]!);
                if (values.Count != prog.Parameters.Count)
                    throw new InvalidOperationException(
                        $"evalir(): kernel expects {prog.Parameters.Count} parameters; got {values.Count} values.");
                var bindings = new Dictionary<Symbol, Num>();
                for (int i = 0; i < prog.Parameters.Count; i++)
                    bindings[Exprs.Current.Symbol(prog.Parameters[i])] = ToNum(values[i]!);
                using (global::Lovelace.Real.Real.WithPrecision(digits, Math.Min(digits, 50)))
                {
                    var result = IrEvaluator.Evaluate(prog, bindings, Exprs.Current);
                    return ToPayload(result);
                }
            }
            finally
            {
                Exprs.Current = previous;
            }
        });
    }

    /// <summary>Splits the batch argument into (flat row-major values, row count). Row-oriented
    /// input arrives with its shape (<c>[[1, 2], [3, 4]]</c> is rank 2), so a row whose width does
    /// not match the kernel is rejected instead of being silently re-flowed into different rows.
    /// A flat list is read as consecutive rows of <paramref name="width"/> values.</summary>
    private static (IReadOnlyList<object?> Values, int Rows) BatchRows(object? payload, int width)
    {
        if (payload is not IReadOnlyList<object?> values)
            throw new InvalidOperationException(
                "evalir_batch(): values must be a list of numbers or a list of rows.");
        if (values.Count == 0)
            throw new InvalidOperationException("evalir_batch(): values must not be empty.");

        if (payload is PayloadArray { Rank: >= 2 } shaped)
        {
            if (shaped.Rank > 2)
                throw new InvalidOperationException(
                    $"evalir_batch(): values must be a flat list or a list of rows (rank 2); got rank {shaped.Rank}.");
            long rowWidth = shaped.RowWidth;
            if (rowWidth != width)
                throw new InvalidOperationException(
                    $"evalir_batch(): kernel expects {width} parameters per row; row 0 contains {rowWidth}.");
            int rowCount = (int)shaped.Shape[0];
            return (values, rowCount);
        }

        if (values.Count % width != 0)
            throw new InvalidOperationException(
                $"evalir_batch(): kernel expects {width} parameters per row; got {values.Count} values, " +
                "which is not a whole number of rows.");
        return (values, values.Count / width);
    }

    /// <summary>The compiled kernel takes its precision per call, from the digits argument of
    /// evalir/evalir_batch — it does not bake one in.</summary>
    private const string PrecisionPolicy = "caller_supplied";

    /// <summary>Lowering preserves the expression as written; optimize() applies an optimization
    /// policy explicitly and reports it in OptimizationResult.policy.</summary>
    private const string OptimizationPolicy = "none";

    /// <summary>Accepts the IR text from compile() or the CompilationResult record from
    /// compile_full() (its <c>ir</c> field).</summary>
    private static string IrTextOf(object? ir) => ir switch
    {
        string s => s,
        global::Lovelace.Abstractions.RecordValue r when r.TryGetField("ir", out var f) && f is string s => s,
        _ => throw new InvalidOperationException("evalir/evalir_batch expect the IR from compile() or compile_full()."),
    };

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

    /// <summary>Argument coercion with the function and position in the message: a wrong payload
    /// kind must never surface as a raw cast failure.</summary>
    private static IReadOnlyList<object?> ListArg(object? o, string function, int position, string expected) =>
        o as IReadOnlyList<object?> ?? throw new InvalidOperationException(
            $"{function}(): argument {position} must be {expected}; got {Describe(o)}.");

    private static string Describe(object? payload) => payload switch
    {
        null => "nothing",
        global::Lovelace.Symbolics.Expr => "Symbolic",
        global::Lovelace.Abstractions.RecordValue => "Record",
        string => "Text",
        _ => payload.GetType().Name,
    };

    private static string NameOf(object? o, string function) => o switch
    {
        string s => s,
        SymbolExpr sx => sx.Symbol.Name,
        _ => throw new InvalidOperationException(
            $"{function}(): argument 2 must contain parameter symbols; got {Describe(o)}."),
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
