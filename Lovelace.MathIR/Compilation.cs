using Lovelace.Symbolics;

namespace Lovelace.MathIR;

/// <summary>Compilation targets (the backend seam; new backends arrive without changing user code).</summary>
public enum CompilationTarget { MathIR }

/// <summary>A compilation outcome: the executable IR plus its parameter contract.</summary>
public sealed record CompilationResult(IrProgram Program, Symbol[] Parameters)
{
    public string IrText => Program.Serialize();
}

/// <summary>
/// An executable kernel over the compiled expression: scalar Evaluate and column-wise
/// EvaluateBatch over the Num union (exact tiers stay exact; approximate tiers follow the
/// ambient Real precision scope). The MathIR interpreter is the v1 backend; later backends
/// (native/SIMD/GPU) replace the evaluator without changing this surface.
/// </summary>
public sealed class CompiledKernel
{
    private readonly IrProgram _program;
    private readonly Symbol[] _parameters;
    private readonly ExprContext _ctx;

    internal CompiledKernel(IrProgram program, Symbol[] parameters, ExprContext ctx)
    {
        _program = program;
        _parameters = parameters;
        _ctx = ctx;
    }

    public IReadOnlyList<Symbol> Parameters => _parameters;

    public string IrText => _program.Serialize();

    /// <summary>Scalar evaluation with positional arguments (arity-checked).</summary>
    public Num Evaluate(params Num[] args)
    {
        if (args.Length != _parameters.Length)
            throw new ArgumentException($"Kernel expects {_parameters.Length} argument(s), got {args.Length}.");
        var bindings = new Dictionary<Symbol, Num>(_parameters.Length);
        for (int i = 0; i < _parameters.Length; i++)
            bindings[_parameters[i]] = args[i];
        return IrEvaluator.Evaluate(_program, bindings, _ctx);
    }

    /// <summary>Scalar evaluation with named arguments.</summary>
    public Num Evaluate(IReadOnlyDictionary<string, Num> args)
    {
        var bindings = new Dictionary<Symbol, Num>(args.Count);
        foreach (var (name, value) in args)
            bindings[_ctx.Symbol(name)] = value;
        return IrEvaluator.Evaluate(_program, bindings, _ctx);
    }

    /// <summary>
    /// Batch evaluation: one lane per row of the provided columns, executed by the vectorized
    /// MathIR evaluator (a single DAG traversal carrying Num[] lanes). Every column must have
    /// the same length; no vector-width or precision shortcuts.
    /// </summary>
    public Num[] EvaluateBatch(IReadOnlyDictionary<string, Num[]> columns)
    {
        if (columns.Count == 0)
            return Array.Empty<Num>();
        int lanes = columns.Values.Select(c => c.Length).Max();
        foreach (var (name, column) in columns)
        {
            if (column.Length != lanes)
                throw new ArgumentException($"Column '{name}' has {column.Length} lanes; expected {lanes}.");
        }
        foreach (var p in _parameters)
        {
            if (!columns.ContainsKey(p.Name))
                throw new ArgumentException($"Missing column for parameter '{p.Name}'.");
        }
        return IrEvaluator.EvaluateBatch(_program, columns, _ctx);
    }
}

/// <summary>The compilation surface: symbolic expression → validated executable kernel.</summary>
public static class Compilation
{
    /// <summary>Lowers the expression to MathIR over the given parameters (current context).</summary>
    public static CompilationResult Compile(Expr e, params Symbol[] parameters)
        => Compile(e, Exprs.Current, parameters);

    public static CompilationResult Compile(Expr e, ExprContext ctx, params Symbol[] parameters)
    {
        if (parameters.Length == 0)
            throw new ArgumentException("Compilation requires at least one parameter.", nameof(parameters));
        var program = Lowering.Lower(e, ctx, parameters);
        IrTyping.Validate(program);
        return new CompilationResult(program, parameters);
    }

    /// <summary>Compiles directly to an executable kernel.</summary>
    public static CompiledKernel CompileKernel(Expr e, Symbol[] parameters, ExprContext? ctx = null)
    {
        ctx ??= Exprs.Current;
        return new CompiledKernel(Lowering.Lower(e, ctx, parameters), parameters, ctx);
    }

    /// <summary>Wraps an existing compilation as a kernel.</summary>
    public static CompiledKernel ToKernel(CompilationResult result, ExprContext? ctx = null)
        => new(result.Program, result.Parameters, ctx ?? Exprs.Current);
}
