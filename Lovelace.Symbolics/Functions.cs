using System.Collections.Immutable;
using Lovelace.Symbolics.Rewriting;

namespace Lovelace.Symbolics;

/// <summary>A registered mathematical function: identity, arity, and hooks.</summary>
public sealed class FunctionDefinition
{
    public string Name { get; }
    public FunctionId Id { get; }
    public int Arity { get; }

    /// <summary>True when any argument count is accepted (variadic); Arity is then a minimum.</summary>
    public bool Variadic { get; }

    /// <summary>f'(arg) as an expression template: given the argument expression, returns the derivative of f at it.</summary>
    public Func<Expr, Expr>? DerivativeTemplate { get; }

    /// <summary>Numeric evaluation over the Num union.</summary>
    public Func<Num[], ExprContext, Num>? NumericEvaluator { get; }

    public RewriteRule[]? Rules { get; }

    public FunctionDefinition(
        string name,
        FunctionId id,
        int arity,
        Func<Expr, Expr>? derivativeTemplate = null,
        Func<Num[], ExprContext, Num>? numericEvaluator = null,
        RewriteRule[]? rules = null,
        bool variadic = false)
    {
        Name = name;
        Id = id;
        Arity = arity;
        DerivativeTemplate = derivativeTemplate;
        NumericEvaluator = numericEvaluator;
        Rules = rules;
        Variadic = variadic;
    }
}

/// <summary>Name → definition registry. Register before use; duplicate names throw.</summary>
public sealed class FunctionRegistry
{
    private readonly Dictionary<string, FunctionDefinition> _defs = new(StringComparer.Ordinal);

    public void Register(FunctionDefinition def)
    {
        if (_defs.ContainsKey(def.Name))
            throw new InvalidOperationException($"Function '{def.Name}' is already registered.");
        _defs[def.Name] = def;
    }

    public bool TryGet(string name, out FunctionDefinition def) => _defs.TryGetValue(name, out def!);

    public FunctionDefinition? Get(string name) => _defs.TryGetValue(name, out var d) ? d : null;

    public IEnumerable<FunctionDefinition> All => _defs.Values;
}

/// <summary>Compile-time registration of the v1 elementary function family.</summary>
public static class CoreFunctions
{
    public static void Register(ExprContext ctx)
    {
        var reg = ctx.Functions;
        void Add(string name, int arity, Func<Expr, Expr>? deriv = null, Func<Num[], ExprContext, Num>? eval = null)
            => reg.Register(new FunctionDefinition(name, ctx.Function(name), arity, deriv, eval));

        Add("exp", 1, a => Exprs.Function(ctx.Function("exp"), a),
            (args, c) => NumOps.Exp(args[0], c));
        Add("log", 1, a => Exprs.Divide(Exprs.One, a),
            (args, c) => NumOps.Ln(args[0], c));
        Add("sin", 1, a => Exprs.Function(ctx.Function("cos"), a),
            (args, c) => NumOps.Sin(args[0], c));
        Add("cos", 1, a => Exprs.Negate(Exprs.Function(ctx.Function("sin"), a)),
            (args, c) => NumOps.Cos(args[0], c));
        Add("tan", 1, a => Exprs.Power(Exprs.Function(ctx.Function("cos"), a), Exprs.Rational(-2)),
            (args, c) => NumOps.Tan(args[0], c));
        Add("asin", 1, a => Exprs.Divide(Exprs.One, Exprs.Sqrt(Exprs.Subtract(Exprs.One, Exprs.Power(a, Exprs.Integer(2))))),
            (args, c) => NumOps.Asin(args[0], c));
        Add("acos", 1, a => Exprs.Negate(Exprs.Divide(Exprs.One, Exprs.Sqrt(Exprs.Subtract(Exprs.One, Exprs.Power(a, Exprs.Integer(2)))))),
            (args, c) => NumOps.Acos(args[0], c));
        Add("atan", 1, a => Exprs.Divide(Exprs.One, Exprs.Add(Exprs.One, Exprs.Power(a, Exprs.Integer(2)))),
            (args, c) => NumOps.Atan(args[0], c));
        Add("sinh", 1, a => Exprs.Function(ctx.Function("cosh"), a),
            (args, c) => NumOps.Sinh(args[0], c));
        Add("cosh", 1, a => Exprs.Function(ctx.Function("sinh"), a),
            (args, c) => NumOps.Cosh(args[0], c));
        Add("tanh", 1, a => Exprs.Power(Exprs.Function(ctx.Function("cosh"), a), Exprs.Rational(-2)),
            (args, c) => NumOps.Tanh(args[0], c));
        Add("asinh", 1, a => Exprs.Divide(Exprs.One, Exprs.Sqrt(Exprs.Add(Exprs.Power(a, Exprs.Integer(2)), Exprs.One))),
            (args, c) => NumOps.Asinh(args[0], c));
        Add("acosh", 1, a => Exprs.Divide(Exprs.One, Exprs.Sqrt(Exprs.Subtract(Exprs.Power(a, Exprs.Integer(2)), Exprs.One))),
            (args, c) => NumOps.Acosh(args[0], c));
        Add("atanh", 1, a => Exprs.Divide(Exprs.One, Exprs.Subtract(Exprs.One, Exprs.Power(a, Exprs.Integer(2)))),
            (args, c) => NumOps.Atanh(args[0], c));
        Add("re", 1, null,
            (args, c) => NumOps.Re(args[0], c));
        Add("im", 1, null,
            (args, c) => NumOps.Im(args[0], c));
        // abs has no template here: its derivative is sign(x) only away from 0, so Diff
        // constructs a conditional piecewise itself. sign/floor/ceil are nondifferentiable
        // on sets and therefore have NO derivative template — differentiation leaves
        // Derivative(...) unevaluated instead of silently claiming a smooth 0.
        Add("abs", 1, null,
            (args, c) => NumOps.Abs(args[0], c));
        Add("sign", 1, null,
            (args, c) => NumOps.Sign(args[0], c));
        Add("floor", 1, null,
            (args, c) => NumOps.Floor(args[0], c));
        Add("ceil", 1, null,
            (args, c) => NumOps.Ceil(args[0], c));
        reg.Register(new FunctionDefinition(
            "min", ctx.Function("min"), 2,
            null, (args, c) => NumOps.Min(args, c), null, variadic: true));
        reg.Register(new FunctionDefinition(
            "max", ctx.Function("max"), 2,
            null, (args, c) => NumOps.Max(args, c), null, variadic: true));
    }
}
