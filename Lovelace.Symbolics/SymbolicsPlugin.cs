using Lovelace.Abstractions;
using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;

namespace Lovelace.Symbolics;

/// <summary>
/// The language-facing surface of the symbolic kernel: a compile-time-linked Modus plugin
/// (the DspPlugin pattern) that registers the CAS builtins. Symbolic values cross the Modus
/// payload boundary as <see cref="Expr"/> objects via the Suite core bridge
/// (ValueKind.Symbolic).
/// </summary>
public sealed class SymbolicsPlugin : IModusPlugin
{
    public string Name => "Lovelace.Symbolics";

    public ExprContext Context { get; } = new();

    private AssumptionSet _assumptions = AssumptionSet.Empty;

    public void Register(IModusContext c)
    {
        void Add(string name, string[] parameters, Func<IReadOnlyList<object?>, object?> impl)
            => c.RegisterBuiltin(name, parameters, args => Run(() => impl(args)));

        Add("symbol", new[] { "name" }, args => Exprs.Symbol((string)args[0]!));
        Add("inf", Array.Empty<string>(), _ => Exprs.Infinity);

        // elementary functions as symbolic builtins (numeric versions do not exist in the core)
        foreach (var fn in new[] { "exp", "log", "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh" })
        {
            var name = fn;
            Add(name, new[] { "x" }, args => Exprs.Function(Context.Function(name), AsExpr(args[0])));
        }
        Add("assume", new[] { "relation" }, args =>
        {
            var rel = (Expr)args[0]!;
            if (rel is RelationExpr r)
            {
                Expr bound = r.Right;
                if (r.Left is SymbolExpr sx && bound is (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr))
                    _assumptions = _assumptions.Add(new SymbolRelationAssumption(sx.Symbol, r.Op, bound));
            }
            return rel;
        });
        Add("assume_positive", new[] { "x" }, args => AssumePred(args, SymbolPredicate.Positive));
        Add("assume_nonnegative", new[] { "x" }, args => AssumePred(args, SymbolPredicate.NonNegative));
        Add("assume_negative", new[] { "x" }, args => AssumePred(args, SymbolPredicate.Negative));
        Add("assume_real", new[] { "x" }, args => AssumeDomain(args, Domain.Real));
        Add("assume_integer", new[] { "x" }, args => AssumeDomain(args, Domain.Integer));
        Add("assume_clear", Array.Empty<string>(), _ =>
        {
            _assumptions = AssumptionSet.Empty;
            return "assumptions cleared";
        });
        Add("assumptions", Array.Empty<string>(), _ =>
            string.Join("; ", _assumptions.Atoms.Select(a => a switch
            {
                SymbolRelationAssumption r => Printing.PrettyPrint(Exprs.Relation(r.Op, Exprs.Symbol(r.S), r.Bound)),
                SymbolDomainAssumption d => d.S.Name + " in " + d.D,
                SymbolPropertyAssumption p => p.S.Name + " is " + p.P,
                _ => a.ToString(),
            })));

        Add("diff", new[] { "f", "x" }, args =>
            Calculus.Diff(AsExpr(args[0]), AsSymbol(args[1]), Context));
        Add("integrate", new[] { "f", "x" }, args =>
            Integration.Integrate(AsExpr(args[0]), AsSymbol(args[1]), Context));
        Add("simplify", new[] { "f" }, args =>
            Simplify.SimplifyExpr(AsExpr(args[0]), Context));
        Add("expand", new[] { "f" }, args =>
            Algebra.Expand(AsExpr(args[0]), Context));
        Add("factor", new[] { "f" }, args =>
            Factoring.Factor(AsExpr(args[0]), Context));
        Add("collect", new[] { "f", "x" }, args =>
            Algebra.Collect(AsExpr(args[0]), AsSymbol(args[1]), Context));
        Add("cancel", new[] { "f" }, args =>
            RationalFunctions.Cancel(AsExpr(args[0]), Context));
        Add("apart", new[] { "f", "x" }, args =>
            RationalFunctions.Apart(AsExpr(args[0]), AsSymbol(args[1]), Context));
        Add("series", new[] { "f", "x", "x0", "order" }, args =>
            Series.Of(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), AsInt(args[3]), Context).ToExpression());
        Add("limit", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.TwoSided, Context)));
        Add("limit_left", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromLeft, Context)));
        Add("limit_right", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromRight, Context)));
        Add("solve", new[] { "f", "x" }, args =>
        {
            var fx = AsExpr(args[0]);
            var sx = AsSymbol(args[1]);
            var set = Solvers.Solve(fx, sx, Context);
            if (set.Kind == SolutionKind.Exact)
            {
                // conditions are part of the solution: a value violating a provable
                // excluded-domain condition (e.g. the pole of a cancelled denominator) is dropped
                var kept = new List<Expr>();
                foreach (var sol in set.Solutions)
                {
                    if (ViolatesConditions(sol, sx))
                        continue;
                    kept.Add(sol.Value);
                }
                return (object)kept.ToArray();
            }
            return Printing.PrettyPrint(fx);
        });
        Add("subs", new[] { "f", "x", "value" }, args =>
            Evaluation.Substitute(AsExpr(args[0]), Context,
                new Dictionary<Symbol, Expr> { [AsSymbol(args[1])] = AsExpr(args[2]) }));
        Add("evalf", new[] { "f", "digits" }, args =>
        {
            var f = AsExpr(args[0]);
            var digits = (int)AsLong(args[1]);
            using (Rl.WithPrecision(digits, Math.Min(digits, 50)))
            {
                var num = Evaluation.EvaluateToNum(f, Context, new Dictionary<Symbol, Num>());
                return NumToPayload(num);
            }
        });
        Add("hessian", new[] { "f", "vars" }, args =>
        {
            var f = AsExpr(args[0]);
            var vars = NameList(args[1]).Select(Context.Symbol).ToArray();
            return (object)SymbolicMatrix.Hessian(f, vars, Context).ToFlatArray();
        });
        Add("jacobian", new[] { "fs", "vars" }, args =>
        {
            var fs = ((IReadOnlyList<object?>)args[0]!).Select(AsExpr).ToArray();
            var vars = NameList(args[1]).Select(Context.Symbol).ToArray();
            return (object)SymbolicMatrix.Jacobian(fs, vars, Context).ToFlatArray();
        });
        Add("optimize", new[] { "f", "params" }, args =>
        {
            var f = AsExpr(args[0]);
            var ps = NameList(args[1]).Select(Context.Symbol).ToArray();
            return Optimizer.Optimize(f, ps, Context).Expression;
        });
    }

    private static IEnumerable<string> NameList(object? o)
    {
        var list = (IReadOnlyList<object?>)o!;
        foreach (var n in list)
        {
            if (n is string s)
                yield return s;
            else if (n is SymbolExpr sx)
                yield return sx.Symbol.Name;
            else
                throw new InvalidOperationException("Expected symbol names.");
        }
    }

    private object Run(Func<object?> impl)
    {
        Exprs.Current = Context;
        Context.Assumptions = _assumptions;
        return impl() ?? throw new InvalidOperationException("Symbolic builtin returned null.");
    }

    private object AssumePred(IReadOnlyList<object?> args, SymbolPredicate pred)
    {
        if (args[0] is SymbolExpr or Expr)
        {
            var e = AsExpr(args[0]);
            if (e is SymbolExpr sx)
            {
                _assumptions = _assumptions.Add(new SymbolPropertyAssumption(sx.Symbol, pred));
                return e;
            }
            _assumptions = _assumptions.Add(new ExpressionPropertyAssumption(e, pred));
            return e;
        }
        var s = Context.Symbol((string)args[0]!);
        _assumptions = _assumptions.Add(new SymbolPropertyAssumption(s, pred));
        return Exprs.Symbol(s);
    }

    private object AssumeDomain(IReadOnlyList<object?> args, Domain domain)
    {
        if (args[0] is Expr e && e is SymbolExpr sx)
        {
            _assumptions = _assumptions.Add(new SymbolDomainAssumption(sx.Symbol, domain));
            return e;
        }
        var s = Context.Symbol((string)args[0]!);
        _assumptions = _assumptions.Add(new SymbolDomainAssumption(s, domain));
        return Exprs.Symbol(s);
    }

    private static object NumToPayload(Num n) => n switch
    {
        NumInt i => i.V,
        // the Modus payload vocabulary carries Real (exact periodic), not Rational
        NumRat r => r.V.IsInteger ? r.V.ToInteger() : RationalReal.ToReal(r.V, (int)Math.Min(Rl.MaxComputationDecimalPlaces, 1000)),
        NumReal rl => rl.V,
        NumComplex c => c.V,
        _ => throw new InvalidOperationException(),
    };

    private static object LimitToExpr(LimitResult r) => r.Status switch
    {
        LimitStatus.Value => r.Value!,
        LimitStatus.PlusInfinity => Exprs.Infinity,
        LimitStatus.MinusInfinity => Exprs.Negate(Exprs.Infinity),
        LimitStatus.DoesNotExist =>
            $"does not exist (left: {SideText(r.FromLeft)}, right: {SideText(r.FromRight)})",
        _ => "unevaluated: " + (r.FailureReason ?? "no limit"),
    };

    private static string SideText(LimitResult? r) => r?.Status switch
    {
        LimitStatus.PlusInfinity => "+inf",
        LimitStatus.MinusInfinity => "-inf",
        LimitStatus.Value => Printing.PrettyPrint(r.Value!),
        _ => "unknown",
    };

    /// <summary>True when the solution provably violates one of its excluded-domain conditions.</summary>
    private static bool ViolatesConditions(Solution sol, Symbol x)
    {
        foreach (var atom in sol.Conditions.Atoms)
        {
            if (atom is ExpressionPropertyAssumption { P: SymbolPredicate.NonZero } ep)
            {
                var sub = Evaluation.Substitute(ep.E, Exprs.Current,
                    new Dictionary<Symbol, Expr> { [x] = sol.Value });
                if (Evaluation.ConstantToNum(sub) is { } nv && NumOps.IsZero(nv))
                    return true;
            }
        }
        return false;
    }

    private static Symbol AsSymbol(object? o)
    {
        if (o is Expr e && e is SymbolExpr sx)
            return sx.Symbol;
        if (o is string s)
            return Exprs.Current.Symbol(s);
        throw new InvalidOperationException("Expected a symbol.");
    }

    private static Expr AsExpr(object? o) => o switch
    {
        Expr e => e,
        Rl r => RealToExpr(r),                 // Real inherits Integer: check before Int!
        Int i => Exprs.Integer(i),
        Nat n => Exprs.Integer(new Int(n)),
        Cplx c => Exprs.Add(RealToExpr(c.Re), Exprs.Multiply(RealToExpr(c.Im), Exprs.I)),
        string s => Exprs.Symbol(s),
        _ => throw new InvalidOperationException($"Cannot convert payload of type {o?.GetType().Name} to an expression."),
    };

    private static Expr RealToExpr(Rl r)
    {
        if (r.IsPeriodic || -r.Exponent <= 18)
            return Exprs.Rational(RationalReal.FromReal(r));
        return Exprs.Real(RealLiteral.FromRealExact(r));
    }

    private static int AsInt(object? o) => (int)AsLong(o);

    private static long AsLong(object? o) => o switch
    {
        Rl r => long.Parse(r.ToString().Split('.')[0]),   // Real inherits Integer: check first
        Nat n => long.Parse(n.ToString()),
        Int i => long.Parse(i.ToString()),
        long l => l,
        _ => throw new InvalidOperationException("Expected an integer."),
    };
}
