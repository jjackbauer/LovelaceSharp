using System.Collections.Immutable;
using Int = global::Lovelace.Integer.Integer;
using Rat = global::Lovelace.Rational.Rational;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;
using Lovelace.Complex;

namespace Lovelace.Symbolics;

/// <summary>A numeric evaluation value: exact integer/rational or approximate real/complex.</summary>
public abstract class Num
{
    public abstract bool IsExact { get; }
}

public sealed class NumInt : Num { public Int V { get; } public NumInt(Int v) { V = v; } public override bool IsExact => true; }
public sealed class NumRat : Num { public Rat V { get; } public NumRat(Rat v) { V = v; } public override bool IsExact => true; }
public sealed class NumReal : Num { public Rl V { get; } public NumReal(Rl v) { V = v; } public override bool IsExact => false; }
public sealed class NumComplex : Num { public Cplx V { get; } public NumComplex(Cplx v) { V = v; } public override bool IsExact => false; }

/// <summary>Exact/approximate arithmetic over the Num union with the documented promotion rules.</summary>
public static class NumOps
{
    public static Num FromInt(Int v) => new NumInt(v);
    public static Num FromRat(Rat v) => v.IsInteger ? new NumInt(v.ToInteger()) : new NumRat(v);
    public static Num FromReal(Rl v) => new NumReal(v);
    public static Num FromComplex(Cplx v) => new NumComplex(v);

    public static Num FromLong(long v) => new NumInt(new Int(v));

    private static int Tier(Num n) => n switch
    {
        NumInt => 0,
        NumRat => 1,
        NumReal => 2,
        _ => 3,
    };

    public static Rl ToReal(Num n)
    {
        var digits = Rl.MaxComputationDecimalPlaces;
        return n switch
        {
            NumInt i => new Rl(i.V),
            NumRat r => RationalReal.ToReal(r.V, (int)Math.Min(digits, int.MaxValue)),
            NumReal rl => rl.V,
            _ => throw new InvalidOperationException("Complex value cannot convert to Real (use re()/im())."),
        };
    }

    public static Cplx ToComplex(Num n) => n switch
    {
        NumComplex c => c.V,
        _ => new Cplx(ToReal(n)),
    };

    public static Num Promote(Num a, Num b)
    {
        int ta = Tier(a), tb = Tier(b);
        int t = Math.Max(ta, tb);
        if (t == 0) return a;
        if (t == 1) return FromRat(RatOf(a));
        if (t == 2) return FromReal(ToReal(a));
        return FromComplex(ToComplex(a));
    }

    public static Rat RatOf(Num n) => n switch
    {
        NumInt i => Rat.From(i.V),
        NumRat r => r.V,
        _ => throw new InvalidOperationException("Value is not exact."),
    };

    public static bool IsZero(Num n) => n switch
    {
        NumInt i => Int.IsZero(i.V),
        NumRat r => r.V.IsZero,
        NumReal rl => Rl.IsZero(rl.V),
        NumComplex c => c.V == Cplx.Zero,
        _ => false,
    };

    public static Num Add(Num a, Num b)
    {
        if (Tier(a) == 0 && Tier(b) == 0) return FromInt(((NumInt)a).V + ((NumInt)b).V);
        if (Tier(a) <= 1 && Tier(b) <= 1) return FromRat(RatOf(a) + RatOf(b));
        if (Tier(a) <= 2 && Tier(b) <= 2) return FromReal(ToReal(a) + ToReal(b));
        return FromComplex(ToComplex(a) + ToComplex(b));
    }

    public static Num Subtract(Num a, Num b)
    {
        if (Tier(a) == 0 && Tier(b) == 0) return FromInt(((NumInt)a).V - ((NumInt)b).V);
        if (Tier(a) <= 1 && Tier(b) <= 1) return FromRat(RatOf(a) - RatOf(b));
        if (Tier(a) <= 2 && Tier(b) <= 2) return FromReal(ToReal(a) - ToReal(b));
        return FromComplex(ToComplex(a) - ToComplex(b));
    }

    public static Num Multiply(Num a, Num b)
    {
        if (Tier(a) == 0 && Tier(b) == 0) return FromInt(((NumInt)a).V * ((NumInt)b).V);
        if (Tier(a) <= 1 && Tier(b) <= 1) return FromRat(RatOf(a) * RatOf(b));
        if (Tier(a) <= 2 && Tier(b) <= 2) return FromReal(ToReal(a) * ToReal(b));
        return FromComplex(ToComplex(a) * ToComplex(b));
    }

    public static Num Divide(Num a, Num b)
    {
        if (IsZero(b))
            throw new DivideByZeroException("Division by zero during numeric evaluation.");
        if (Tier(a) == 0 && Tier(b) == 0)
        {
            // exact integer division: quotient is exact iff remainder is zero
            var (q, r) = IntegerDivRem(((NumInt)a).V, ((NumInt)b).V);
            return Int.IsZero(r) ? FromInt(q) : FromRat(Rat.From(((NumInt)a).V, ((NumInt)b).V));
        }
        if (Tier(a) <= 1 && Tier(b) <= 1)
            return FromRat(RatOf(a) / RatOf(b));
        if (Tier(a) <= 2 && Tier(b) <= 2)
            return FromReal(ToReal(a) / ToReal(b));
        return FromComplex(ToComplex(a) / ToComplex(b));
    }

    private static (Int Q, Int R) IntegerDivRem(Int a, Int b)
    {
        var q = a / b;
        return (q, a - q * b);
    }

    public static Num Negate(Num a) => a switch
    {
        NumInt i => FromInt(i.V.Negate()),
        NumRat r => FromRat(Rat.Negate(r.V)),
        NumReal rl => FromReal(Rl.Parse("0", null) - rl.V),
        NumComplex c => FromComplex(-c.V),
        _ => throw new InvalidOperationException(),
    };

    public static int Compare(Num a, Num b)
    {
        if (Tier(a) <= 1 && Tier(b) <= 1)
            return RatOf(a).CompareTo(RatOf(b));
        return ToReal(a).CompareTo(ToReal(b));
    }

    /// <summary>Exact/repeated-squaring integer power; negative exponents via reciprocal.
    /// The exponent is an arbitrary-precision Int — never narrowed through int/long.</summary>
    /// <summary>Integer-power entry taking an exponent value: exact integer exponents only.</summary>
    public static Num PowInt(Num b, Num e)
    {
        if (Tier(e) > 1 || !RatOf(e).IsInteger)
            throw new InvalidOperationException("PowInt requires an exact integer exponent.");
        return PowInt(b, RatOf(e).ToInteger());
    }

    public static Num PowInt(Num b, Int e)
    {
        if (Int.IsZero(e))
            return FromInt(Int.One);
        bool negExp = e < Int.Zero;
        var n = negExp ? Int.Abs(e) : e;
        Num result = FromInt(Int.One);
        var x = b;
        var two = new Int(2L);
        while (n > Int.Zero)
        {
            if (Int.IsOddInteger(n)) result = Multiply(result, x);
            x = Multiply(x, x);
            n = n / two;
        }
        return negExp ? Divide(FromInt(Int.One), result) : result;
    }

    public static Num Pow(Num b, Num e)
    {
        if (Tier(e) <= 1 && RatOf(e).IsInteger)
            return PowInt(b, RatOf(e).ToInteger());
        // b^e = exp(e ln b): real path for b > 0, complex principal branch otherwise
        if (Tier(b) <= 2 && Tier(e) <= 2)
        {
            var br = ToReal(b);
            if (br > Rl.Parse("0", null))
                return FromReal(ComplexMath.Pow(br, ToReal(e)));
        }
        var bc = ToComplex(b);
        return FromComplex(ComplexMath.Pow(bc, ToComplex(e)));
    }

    public static Num Abs(Num a, ExprContext ctx) => a switch
    {
        NumInt i => FromInt(Int.Abs(i.V)),
        NumRat r => FromRat(Rat.Abs(r.V)),
        NumReal rl => FromReal(Rl.Abs(rl.V)),
        NumComplex c => FromReal(c.V.Magnitude),
        _ => throw new InvalidOperationException(),
    };

    public static Num Sign(Num a, ExprContext ctx)
    {
        if (IsZero(a)) return FromInt(Int.Zero);
        if (Tier(a) <= 2)
            return Compare(a, FromInt(Int.Zero)) > 0 ? FromInt(Int.One) : FromInt(Int.NegativeOne);
        var z = ToComplex(a);
        return FromComplex(z / z.Magnitude);
    }

    public static Num Floor(Num a, ExprContext ctx) => a switch
    {
        NumInt or NumRat => FromRat(Rat.From(RatOf(a).Floor())),
        NumReal rl => FromReal(RealFloor(rl.V)),
        _ => throw new InvalidOperationException("floor() is real-only."),
    };

    public static Num Ceil(Num a, ExprContext ctx)
    {
        var f = Floor(a, ctx);
        return IsZero(Subtract(a, f)) ? f : Add(f, FromInt(Int.One));
    }

    private static Rl RealFloor(Rl x)
    {
        // digit-exact floor: operate on the full-precision magnitude, never the
        // display-truncated ToString form
        var neg = Rl.IsNegative(x);
        if (x.Exponent >= 0)
            return x;   // already integral
        var mag = new Int(x.ToNatural());
        var p = new Int(10).Pow(new Int(-x.Exponent));
        var q = mag / p;
        if (neg && q * p != mag)
            q = q - Int.One;
        return new Rl(q);
    }

    public static Num Re(Num a, ExprContext ctx) => a switch
    {
        NumComplex c => FromReal(c.V.Re),
        _ => a,
    };

    public static Num Im(Num a, ExprContext ctx) => a switch
    {
        NumComplex c => FromReal(c.V.Im),
        _ => FromInt(Int.Zero),
    };

    public static Num Min(Num[] args, ExprContext ctx)
    {
        var m = args[0];
        foreach (var a in args.Skip(1))
            if (Compare(a, m) < 0) m = a;
        return m;
    }

    public static Num Max(Num[] args, ExprContext ctx)
    {
        var m = args[0];
        foreach (var a in args.Skip(1))
            if (Compare(a, m) > 0) m = a;
        return m;
    }

    public static Num Exp(Num a, ExprContext ctx) => Tier(a) <= 2 ? FromReal(ComplexMath.Exp(ToReal(a))) : FromComplex(ComplexMath.Exp(ToComplex(a)));

    public static Num Ln(Num a, ExprContext ctx) => Tier(a) <= 2 ? FromReal(ComplexMath.Ln(ToReal(a))) : FromComplex(ComplexMath.Log(ToComplex(a)));

    public static Num Sin(Num a, ExprContext ctx) => Tier(a) <= 2 ? FromReal(ComplexMath.Sin(ToReal(a))) : FromComplex(ComplexMath.Sin(ToComplex(a)));

    public static Num Cos(Num a, ExprContext ctx) => Tier(a) <= 2 ? FromReal(ComplexMath.Cos(ToReal(a))) : FromComplex(ComplexMath.Cos(ToComplex(a)));

    public static Num Tan(Num a, ExprContext ctx) => Divide(Sin(a, ctx), Cos(a, ctx));

    public static Num Asin(Num a, ExprContext ctx) => FromReal(ComplexMath.AsinReal(ToReal(a)));

    public static Num Acos(Num a, ExprContext ctx) => FromReal(ComplexMath.AcosReal(ToReal(a)));

    public static Num Atan(Num a, ExprContext ctx) => Tier(a) <= 2 ? FromReal(ComplexMath.Atan(ToReal(a))) : throw new InvalidOperationException("atan is real-only in v1.");

    public static Num Sinh(Num a, ExprContext ctx)
    {
        var e = ComplexMath.Exp(ToReal(a));
        var one = Rl.Parse("1", null);
        var two = Rl.Parse("2", null);
        return FromReal((e - one / e) / two);
    }

    public static Num Cosh(Num a, ExprContext ctx)
    {
        var e = ComplexMath.Exp(ToReal(a));
        var one = Rl.Parse("1", null);
        var two = Rl.Parse("2", null);
        return FromReal((e + one / e) / two);
    }

    public static Num Tanh(Num a, ExprContext ctx) => Divide(Sinh(a, ctx), Cosh(a, ctx));

    public static Num Asinh(Num a, ExprContext ctx) => FromReal(ComplexMath.AsinhReal(ToReal(a)));

    public static Num Acosh(Num a, ExprContext ctx) => FromReal(ComplexMath.AcoshReal(ToReal(a)));

    public static Num Atanh(Num a, ExprContext ctx) => FromReal(ComplexMath.AtanhReal(ToReal(a)));
}

/// <summary>Thrown when a symbolic expression cannot be evaluated numerically as requested.</summary>
public sealed class EvaluationException : Exception
{
    public EvaluationException(string message) : base(message) { }
}

/// <summary>Substitution and numeric evaluation of expressions.</summary>
public static class Evaluation
{
    /// <summary>Converts a numeric-constant expression into a Num (null if not a constant).</summary>
    public static Num? ConstantToNum(Expr e)
    {
        var digits = Rl.MaxComputationDecimalPlaces;
        return e switch
        {
            IntegerConstantExpr i => new NumInt(i.Value),
            RationalConstantExpr r => new NumRat(r.Value),
            RealConstantExpr rl => new NumReal(rl.Value.ToReal()),
            ComplexConstantExpr c => new NumComplex(new Cplx(RationalReal.ToReal(c.Re, (int)Math.Min(digits, int.MaxValue)), RationalReal.ToReal(c.Im, (int)Math.Min(digits, int.MaxValue)))),
            _ => null,
        };
    }

    /// <summary>Converts a Num back to an expression constant.</summary>
    public static Expr NumToExpr(Num n)
    {
        var digits = Rl.MaxComputationDecimalPlaces;
        return n switch
        {
            NumInt i => Exprs.Integer(i.V),
            NumRat r => Exprs.Rational(r.V),
            NumReal rl => Exprs.Real(RealLiteral.FromRealExact(rl.V)),
            NumComplex c => Exprs.Add(
                Exprs.Real(RealLiteral.FromRealExact(c.V.Re)),
                Exprs.Multiply(Exprs.Real(RealLiteral.FromRealExact(c.V.Im)), Exprs.I)),
            _ => throw new InvalidOperationException(),
        };
    }

    /// <summary>
    /// Numeric evaluation with symbol bindings. Evaluates the whole tree to a Num;
    /// throws <see cref="EvaluationException"/> for missing bindings or unsupported nodes.
    /// </summary>
    public static Num EvaluateToNum(Expr e, ExprContext ctx, IReadOnlyDictionary<Symbol, Num> bindings)
    {
        switch (e)
        {
            case IntegerConstantExpr i: return new NumInt(i.Value);
            case RationalConstantExpr r: return new NumRat(r.Value);
            case RealConstantExpr rl: return new NumReal(rl.Value.ToReal());
            case ComplexConstantExpr c:
            {
                var d = Rl.MaxComputationDecimalPlaces;
                return new NumComplex(new Cplx(RationalReal.ToReal(c.Re, (int)Math.Min(d, int.MaxValue)), RationalReal.ToReal(c.Im, (int)Math.Min(d, int.MaxValue))));
            }
            case NamedConstantExpr n: return n.Constant switch
            {
                NamedConstant.Pi => new NumReal(Rl.Pi),
                NamedConstant.E => new NumReal(Rl.E),
                NamedConstant.I => new NumComplex(Cplx.I),
                _ => throw new EvaluationException("Infinity has no numeric value."),
            };
            case SymbolExpr s:
            {
                if (bindings.TryGetValue(s.Symbol, out var v))
                    return v;
                // symbols may also be matched by name for cross-context bindings
                foreach (var (k, val) in bindings)
                    if (k.Name == s.Symbol.Name)
                        return val;
                throw new EvaluationException($"No value for symbol '{s.Symbol.Name}'.");
            }
            case AddExpr a:
            {
                Num acc = new NumInt(Int.Zero);
                foreach (var t in a.Terms)
                    acc = NumOps.Add(acc, EvaluateToNum(t, ctx, bindings));
                return acc;
            }
            case MultiplyExpr m:
            {
                Num acc = new NumInt(Int.One);
                foreach (var f in m.Factors)
                    acc = NumOps.Multiply(acc, EvaluateToNum(f, ctx, bindings));
                return acc;
            }
            case PowerExpr p:
            {
                var b = EvaluateToNum(p.Base, ctx, bindings);
                var ex = EvaluateToNum(p.Exponent, ctx, bindings);
                if (NumOps.IsZero(b) && (NumOps.Compare(ex, new NumInt(Int.Zero)) <= 0))
                    throw new EvaluationException("0 raised to a non-positive power.");
                return NumOps.Pow(b, ex);
            }
            case FunctionExpr f:
            {
                var args = new Num[f.Arguments.Length];
                for (int i = 0; i < args.Length; i++)
                    args[i] = EvaluateToNum(f.Arguments[i], ctx, bindings);
                var def = ctx.Functions.Get(f.Function.Name);
                if (def?.NumericEvaluator is { } ev)
                    return ev(args, ctx);
                if (f.Function.Name == "sqrt")
                    return NumOps.Pow(args[0], new NumRat(Rat.From(1, 2)));
                throw new EvaluationException($"Function '{f.Function.Name}' has no numeric evaluator.");
            }
            case PiecewiseExpr pw:
            {
                foreach (var br in pw.Branches)
                {
                    var g = EvaluateRelation(br.Guard, ctx, bindings);
                    if (g == Tristate.True)
                        return EvaluateToNum(br.Value, ctx, bindings);
                    if (g == Tristate.False)
                        continue;
                    throw new EvaluationException("Piecewise guard cannot be decided numerically.");
                }
                return EvaluateToNum(pw.Otherwise, ctx, bindings);
            }
            case RootOfExpr ro:
                return new NumReal(Roots.N(ro, Rl.MaxComputationDecimalPlaces, ctx));
            default:
                throw new EvaluationException($"Node of kind {e.Kind} cannot be evaluated numerically.");
        }
    }

    /// <summary>
    /// Three-valued evaluation of a relation guard: True/False when both sides evaluate and
    /// compare (exact compare for exact tiers, approximate at the precision scope otherwise),
    /// Unknown when a side cannot be evaluated or is complex. Unknown is never coerced to
    /// False.
    /// </summary>
    public static Tristate EvaluateRelation(Expr guard, ExprContext ctx, IReadOnlyDictionary<Symbol, Num> bindings)
    {
        if (guard is not RelationExpr r)
            return Tristate.Unknown;
        Num l, right;
        try
        {
            l = EvaluateToNum(r.Left, ctx, bindings);
            right = EvaluateToNum(r.Right, ctx, bindings);
        }
        catch (EvaluationException)
        {
            return Tristate.Unknown;
        }
        int cmp;
        try
        {
            cmp = NumOps.Compare(l, right);
        }
        catch (Exception)
        {
            return Tristate.Unknown;   // unordered (complex) values are not False, they are unknown
        }
        bool result = r.Op switch
        {
            RelOp.Eq => cmp == 0,
            RelOp.Ne => cmp != 0,
            RelOp.Lt => cmp < 0,
            RelOp.Le => cmp <= 0,
            RelOp.Gt => cmp > 0,
            _ => cmp >= 0,
        };
        return result ? Tristate.True : Tristate.False;
    }

    /// <summary>
    /// Structural substitution: replaces symbols with expressions and rebuilds canonically.
    /// Does not evaluate; the result is an expression.
    /// </summary>
    public static Expr Substitute(Expr e, ExprContext ctx, IReadOnlyDictionary<Symbol, Expr> replacements)
    {
        Expr Sub(Expr x)
        {
            switch (x)
            {
                case SymbolExpr s:
                    foreach (var (k, v) in replacements)
                        if (k.Name == s.Symbol.Name)
                            return v;
                    return x;
                case AddExpr a:
                    return Exprs.Add(a.Terms.Select(Sub));
                case MultiplyExpr m:
                    return Exprs.Multiply(m.Factors.Select(Sub));
                case PowerExpr p:
                    return Exprs.Power(Sub(p.Base), Sub(p.Exponent));
                case FunctionExpr f:
                    return Exprs.Function(f.Function, f.Arguments.Select(Sub).ToArray());
                case RelationExpr r:
                    return Exprs.Relation(r.Op, Sub(r.Left), Sub(r.Right));
                case PiecewiseExpr pw:
                    return Exprs.Piecewise(
                        pw.Branches.Select(br => new PiecewiseBranch(Sub(br.Guard), Sub(br.Value))),
                        Sub(pw.Otherwise));
                case DerivativeExpr d:
                    return Exprs.Derivative(Sub(d.Operand), d.Variables.ToArray());
                case IntegralExpr i:
                    return Exprs.Integral(Sub(i.Operand), i.Variables.ToArray());
                default:
                    return x;
            }
        }
        return Sub(e);
    }

    /// <summary>
    /// Substitutes numeric values and evaluates subexpressions to constants where possible,
    /// keeping unevaluated (symbolic) parts symbolic.
    /// </summary>
    public static Expr EvaluateToExpr(Expr e, ExprContext ctx, IReadOnlyDictionary<Symbol, Num> bindings)
    {
        Expr Ev(Expr x)
        {
            if (ConstantToNum(x) is { } n)
                return x;
            switch (x)
            {
                case SymbolExpr s:
                    foreach (var (k, v) in bindings)
                        if (k.Name == s.Symbol.Name)
                            return NumToExpr(v);
                    return x;
                case AddExpr a:
                    return Exprs.Add(a.Terms.Select(Ev));
                case MultiplyExpr m:
                    return Exprs.Multiply(m.Factors.Select(Ev));
                case PowerExpr p:
                    return Exprs.Power(Ev(p.Base), Ev(p.Exponent));
                case FunctionExpr f:
                {
                    var args = f.Arguments.Select(Ev).ToArray();
                    if (args.All(a => ConstantToNum(a) is not null))
                    {
                        var folded = FoldFunction(f.Function, args.ToImmutableArray());
                        if (folded is not null)
                            return folded;
                        if (ctx.Functions.Get(f.Function.Name)?.NumericEvaluator is { } ev)
                        {
                            var nums = args.Select(a => ConstantToNum(a)!).ToArray();
                            try
                            {
                                return NumToExpr(ev(nums, ctx));
                            }
                            catch (Exception)
                            {
                                // fall through to the symbolic node
                            }
                        }
                    }
                    return Exprs.Function(f.Function, args);
                }
                default:
                    return x;
            }
        }
        return Ev(e);
    }

    /// <summary>Constant folding hook used by Exprs.Function: folds total functions of numeric constants.</summary>
    internal static Expr? FoldFunction(FunctionId id, ImmutableArray<Expr> args)
    {
        var ctx = Exprs.Current;
        bool allExact = true;
        var nums = new Num[args.Length];
        for (int i = 0; i < args.Length; i++)
        {
            var n = ConstantToNum(args[i]);
            if (n is null)
                return null;
            nums[i] = n;
            allExact &= n.IsExact;
        }

        // exact rational folding table (never introduces approximation)
        if (allExact && args.Length == 1 && NumOps.RatOf(nums[0]) is { } r)
        {
            var exact = id.Name switch
            {
                "exp" when r.IsZero => Rat.One,
                "sin" when r.IsZero => Rat.Zero,
                "cos" when r.IsZero => Rat.One,
                "tan" when r.IsZero => Rat.Zero,
                "log" when r.IsOne => Rat.Zero,
                "atan" when r.IsZero => Rat.Zero,
                "asin" when r.IsZero => Rat.Zero,
                "acos" when r.IsOne => Rat.Zero,
                "sinh" when r.IsZero => Rat.Zero,
                "cosh" when r.IsZero => Rat.One,
                "tanh" when r.IsZero => Rat.Zero,
                "asinh" when r.IsZero => Rat.Zero,
                "acosh" when r.IsOne => Rat.Zero,
                "atanh" when r.IsZero => Rat.Zero,
                "abs" => Rat.Abs(r),
                "sign" => r.IsZero ? Rat.Zero : (r.IsNegative ? Rat.MinusOne : Rat.One),
                "floor" => Rat.From(r.Floor()),
                "ceil" => Rat.From(r.IsInteger ? r.ToInteger() : r.Floor() + new Int(1L)),
                "min" or "max" => null,
                _ => null,
            };
            if (exact is not null)
                return NumToExpr(NumOps.FromRat(exact));
        }

        if (allExact && args.Length == 2 && id.Name is "min" or "max")
        {
            var a = NumOps.RatOf(nums[0]);
            var b = NumOps.RatOf(nums[1]);
            int cmp = a.CompareTo(b);
            var picked = id.Name == "min" ? (cmp <= 0 ? a : b) : (cmp >= 0 ? a : b);
            return NumToExpr(NumOps.FromRat(picked));
        }

        // approximate folding: only when an approximate constant is already present
        if (!allExact)
        {
            var def = ctx.Functions.Get(id.Name);
            if (def?.NumericEvaluator is null)
                return null;
            try
            {
                return NumToExpr(def.NumericEvaluator(nums, ctx));
            }
            catch (Exception)
            {
                return null;
            }
        }
        return null;
    }
}
