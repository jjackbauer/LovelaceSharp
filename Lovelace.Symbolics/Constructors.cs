using System.Collections.Immutable;
using Int = global::Lovelace.Integer.Integer;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// Canonical expression constructors. Every public construction path funnels through these
/// factories; nodes are canonicalized eagerly (A1–A6, M1–M6, P1–P7 of the architecture) and
/// hash-consed through the ambient <see cref="ExprContext"/>. Canonicalization is
/// assumption-free by invariant.
/// </summary>
public static class Exprs
{
    private static readonly System.Threading.AsyncLocal<ExprContext?> _current = new();

    /// <summary>Ambient context (per execution flow, like Real's precision scope).</summary>
    public static ExprContext Current
    {
        get => _current.Value ??= new ExprContext();
        set => _current.Value = value;
    }

    /// <summary>Clears the ambient context (test cleanup).</summary>
    public static void ClearCurrent() => _current.Value = null;

    // -----------------------------------------------------------------
    // Leaves
    // -----------------------------------------------------------------

    /// <summary>
    /// Canonical integer constant. Canonical composite nodes normalize all numeric constants
    /// to RationalConstant, so this factory also produces a RationalConstant (the
    /// IntegerConstant node kind is retained for taxonomy but never emitted canonically).
    /// </summary>
    public static Expr Integer(Int value)
    {
        var n = new RationalConstantExpr(Rat.From(value));
        n._hash = Expr.Combine((int)NodeKind.RationalConstant, n.Value.GetHashCode());
        n._nodeCount = 1;
        n._isExact = true;
        return Current.Intern(n);
    }

    public static Expr Integer(long value) => Integer(new Int(value));

    public static Expr Rational(Rat value)
    {
        var n = new RationalConstantExpr(value);
        n._hash = Expr.Combine((int)NodeKind.RationalConstant, value.GetHashCode());
        n._nodeCount = 1;
        n._isExact = true;
        return Current.Intern(n);
    }

    public static Expr Rational(Int num, Int den) => Rational(Rat.From(num, den));

    public static Expr Rational(long num, long den) => Rational(Rat.From(new Int(num), new Int(den)));

    public static Expr Real(RealLiteral value)
    {
        var n = new RealConstantExpr(value);
        n._hash = Expr.Combine((int)NodeKind.RealConstant, value.GetHashCode());
        n._nodeCount = 1;
        n._isExact = false;
        return Current.Intern(n);
    }

    public static Expr Complex(Rat re, Rat im)
    {
        if (im.IsZero)
            return Rational(re);
        var n = new ComplexConstantExpr(re, im);
        n._hash = Expr.Combine((int)NodeKind.ComplexConstant, re.GetHashCode(), im.GetHashCode());
        n._nodeCount = 1;
        n._isExact = true;
        return Current.Intern(n);
    }

    public static Expr Pi => Named(NamedConstant.Pi);
    public static Expr E => Named(NamedConstant.E);
    public static Expr I => Named(NamedConstant.I);
    public static Expr Infinity => Named(NamedConstant.Infinity);

    public static Expr Named(NamedConstant constant)
    {
        var n = new NamedConstantExpr(constant);
        n._hash = Expr.Combine((int)NodeKind.NamedConstant, (int)constant);
        n._nodeCount = 1;
        n._isExact = constant == NamedConstant.I;   // Pi/E/Infinity are not exact rational values
        return Current.Intern(n);
    }

    public static Expr Symbol(Symbol s)
    {
        var n = new SymbolExpr(s);
        n._hash = Expr.Combine((int)NodeKind.Symbol, s.GetHashCode());
        n._nodeCount = 1;
        n._isExact = true;
        return Current.Intern(n);
    }

    public static Expr Symbol(string name) => Symbol(Current.Symbol(name));

    // -----------------------------------------------------------------
    // Composite nodes
    // -----------------------------------------------------------------

    public static Expr Function(FunctionId id, params Expr[] args)
    {
        var imm = ImmutableArray.Create(args);
        if (id.Name == "sqrt")
        {
            if (imm.Length != 1) throw new ArgumentException("sqrt takes one argument.");
            return Power(imm[0], Rational(1, 2));
        }
        // numeric constant folding for total functions
        var folded = Evaluation.FoldFunction(id, imm);
        if (folded is not null)
            return folded;
        var n = new FunctionExpr(id, imm);
        n._hash = Expr.Combine((int)NodeKind.Function, id.GetHashCode(), CombineHashes(imm));
        n._nodeCount = 1 + SumCounts(imm);
        n._isExact = IsExactFunction(id, imm);
        return Current.Intern(n);
    }

    private static bool IsExactFunction(FunctionId id, ImmutableArray<Expr> args) =>
        (id.Name is "abs" or "sign" or "floor" or "ceil" or "min" or "max") && args.All(a => a.IsExact);

    public static Expr Add(params Expr[] children) => AddImpl(children);

    public static Expr Add(IEnumerable<Expr> children) => AddImpl(children.ToArray());

    private static Expr AddImpl(Expr[] children)
    {
        var flat = new List<Expr>();
        foreach (var c in children)
        {
            if (c is AddExpr a)
                flat.AddRange(a.Terms);
            else
                flat.Add(c);
        }

        Rat exact = Rat.Zero;
        Rat real = Rat.Zero;
        bool hasReal = false;
        Rat cplxRe = Rat.Zero, cplxIm = Rat.Zero;
        var like = new Dictionary<Expr, Rat>();

        foreach (var c in flat)
        {
            switch (c)
            {
                case IntegerConstantExpr i:
                    exact = exact + Rat.From(i.Value);
                    break;
                case RationalConstantExpr r:
                    exact = exact + r.Value;
                    break;
                case RealConstantExpr rl:
                    real = real + rl.Value.ToRational();
                    hasReal = true;
                    break;
                case ComplexConstantExpr cx:
                    cplxRe = cplxRe + cx.Re;
                    cplxIm = cplxIm + cx.Im;
                    break;
                default:
                {
                    var (coef, rest) = SplitTerm(c);
                    if (!like.TryGetValue(rest, out var prev)) prev = Rat.Zero;
                    like[rest] = prev + coef;
                    break;
                }
            }
        }

        var terms = new List<Expr>();
        if (!exact.IsZero)
            terms.Add(Rational(exact));
        if (hasReal && !real.IsZero)
            terms.Add(Real(RealLiteral.FromRational(real, 64)));
        if (!(cplxRe.IsZero && cplxIm.IsZero))
            terms.Add(Complex(cplxRe, cplxIm));

        foreach (var (rest, coef) in like)
        {
            if (coef.IsZero)
                continue;
            terms.Add(coef.IsOne ? rest : Multiply(Rational(coef), rest));
        }

        if (terms.Count == 0)
            return Rational(Rat.Zero);
        if (terms.Count == 1)
            return terms[0];
        terms.Sort(TermOrder.Compare);
        var node = new AddExpr(ImmutableArray.CreateRange(terms));
        node._hash = Expr.Combine((int)NodeKind.Add, CombineHashes(node.Terms));
        node._nodeCount = 1 + SumCounts(node.Terms);
        node._isExact = node.Terms.All(t => t.IsExact);
        return Current.Intern(node);
    }

    /// <summary>Splits a canonical product term into (numeric coefficient, remainder).</summary>
    public static (Rat Coef, Expr Remainder) SplitTerm(Expr c)
    {
        if (c is IntegerConstantExpr i)
            return (Rat.From(i.Value), One);
        if (c is RationalConstantExpr r)
            return (r.Value, One);
        if (c is MultiplyExpr m && m.Factors.Length > 0 && TermOrder.IsNumericConstant(m.Factors[0]))
        {
            var coef = NumericToRational(m.Factors[0]) ?? Rat.One;
            var restFactors = m.Factors.Skip(1).ToArray();
            var rest = restFactors.Length == 0 ? One
                : restFactors.Length == 1 ? restFactors[0]
                : Multiply(restFactors);
            return (coef, rest);
        }
        return (Rat.One, c);
    }

    internal static Rat? NumericToRational(Expr c) => c switch
    {
        IntegerConstantExpr i => Rat.From(i.Value),
        RationalConstantExpr r => r.Value,
        _ => null,
    };

    public static Expr Multiply(params Expr[] children) => MultiplyImpl(children);

    public static Expr Multiply(IEnumerable<Expr> children) => MultiplyImpl(children.ToArray());

    private static Expr MultiplyImpl(Expr[] children)
    {
        var flat = new List<Expr>();
        foreach (var c in children)
        {
            if (c is MultiplyExpr m)
                flat.AddRange(m.Factors);
            else
                flat.Add(c);
        }

        Rat exact = Rat.One;
        Rat real = Rat.One;
        bool hasReal = false;
        Rat cplxRe = Rat.One, cplxIm = Rat.Zero;
        var powers = new Dictionary<Expr, Rat>();

        foreach (var c in flat)
        {
            switch (c)
            {
                case IntegerConstantExpr i:
                {
                    var v = Rat.From(i.Value);
                    if (v.IsZero) return Rational(Rat.Zero);
                    if (v.IsOne) break;
                    exact = exact * v;
                    break;
                }
                case RationalConstantExpr r:
                {
                    if (r.Value.IsZero) return Rational(Rat.Zero);
                    if (r.Value.IsOne) break;
                    exact = exact * r.Value;
                    break;
                }
                case RealConstantExpr rl:
                {
                    var v = rl.Value.ToRational();
                    if (v.IsZero) return Rational(Rat.Zero);
                    if (v.IsOne) break;
                    real = real * v;
                    hasReal = true;
                    break;
                }
                case ComplexConstantExpr cx:
                {
                    if (cx.Re.IsZero && cx.Im.IsZero) return Rational(Rat.Zero);
                    var nr = cplxRe * cx.Re - cplxIm * cx.Im;
                    var ni = cplxRe * cx.Im + cplxIm * cx.Re;
                    cplxRe = nr;
                    cplxIm = ni;
                    break;
                }
                case PowerExpr p when ExponentIsInteger(p.Exponent):
                {
                    var e = NumericToRational(p.Exponent) ?? Rat.One;
                    if (!powers.TryGetValue(p.Base, out var prev)) prev = Rat.Zero;
                    powers[p.Base] = prev + e;
                    break;
                }
                default:
                {
                    if (!powers.TryGetValue(c, out var prev)) prev = Rat.Zero;
                    powers[c] = prev + Rat.One;
                    break;
                }
            }
        }

        var factors = new List<Expr>();
        if (!exact.IsOne)
            factors.Add(Rational(exact));
        if (hasReal && !real.IsOne)
            factors.Add(Real(RealLiteral.FromRational(real, 64)));
        if (!(cplxRe.IsOne && cplxIm.IsZero))
            factors.Add(Complex(cplxRe, cplxIm));

        foreach (var (b, e) in powers)
        {
            if (e.IsZero)
                continue;
            factors.Add(e.IsOne ? b : Power(b, Rational(e)));
        }

        if (factors.Count == 0)
            return Rational(Rat.One);
        if (factors.Count == 1)
            return factors[0];
        factors.Sort(TermOrder.Compare);
        var node = new MultiplyExpr(ImmutableArray.CreateRange(factors));
        node._hash = Expr.Combine((int)NodeKind.Multiply, CombineHashes(node.Factors));
        node._nodeCount = 1 + SumCounts(node.Factors);
        node._isExact = node.Factors.All(f => f.IsExact);
        return Current.Intern(node);
    }

    internal static bool ExponentIsInteger(Expr e) =>
        NumericToRational(e) is Rat r && r.IsInteger;

    public static Expr Power(Expr b, Expr e)
    {
        if (NumericToRational(e) is { } eNorm)
            e = Rational(eNorm);   // canonical: every numeric exponent is a RationalConstant
        var eRat = NumericToRational(e);

        if (eRat is { IsZero: true }) return Rational(Rat.One);
        if (eRat is { IsOne: true }) return b;
        if (b is IntegerConstantExpr bi && bi.Value == Int.One) return Rational(Rat.One);
        if (b is RationalConstantExpr br && br.Value.IsOne) return Rational(Rat.One);

        // (b2^e1)^k → b2^(e1*k) when k is an integer (e1 any rational; principal branch convention)
        if (b is PowerExpr inner && eRat is { } ek && ek.IsInteger)
            return Power(inner.Base, Rational((NumericToRational(inner.Exponent) ?? Rat.One) * ek));

        // base 0: positive exponent → 0
        if (b is RationalConstantExpr b0 && b0.Value.IsZero)
        {
            if (eRat is { } er && er.IsPositive)
                return Rational(Rat.Zero);
            return MakePower(b, e);
        }

        // base -1 with integer exponent: parity
        if (b is RationalConstantExpr bm1 && bm1.Value.IsMinusOne && eRat is { IsInteger: true } ep)
            return ep.ToInteger().IsEvenInteger() ? Rational(Rat.One) : Rational(Rat.MinusOne);

        // rational base with integer exponent
        if (b is (IntegerConstantExpr or RationalConstantExpr) && eRat is { IsInteger: true } ei)
        {
            var bv = NumericToRational(b) ?? Rat.Zero;
            return Rational(RationalPowerInt(bv, (int)ei.ToInteger().ToInt64Saturating()));
        }

        // rational base with unit-fraction exponent: exact root test
        if (b is (IntegerConstantExpr or RationalConstantExpr) && eRat is { } eu)
        {
            var bv = NumericToRational(b) ?? Rat.Zero;
            if (!eu.IsNegative)
            {
                var inv = Rat.One / eu;   // = n (eu = 1/n)
                if (inv.IsInteger && inv.ToInteger() > Int.One)
                {
                    int n = (int)inv.ToInteger().ToInt64Saturating();
                    var root = RationalRoot(bv, n);
                    if (root is { } r)
                        return Rational(r);
                }
            }
            if (!eu.IsInteger && bv.IsNegative)
                return MakePower(b, e);   // complex-valued: leave unevaluated
        }

        // complex constant base with integer exponent
        if (b is ComplexConstantExpr cb && eRat is { IsInteger: true } eci)
        {
            var n = (int)eci.ToInteger().ToInt64Saturating();
            Rat re = Rat.One, im = Rat.Zero;
            for (int i = 0; i < Math.Abs(n); i++)
            {
                var nr = re * cb.Re - im * cb.Im;
                var ni = re * cb.Im + im * cb.Re;
                re = nr; im = ni;
            }
            if (n < 0)
            {
                var mag2 = cb.Re * cb.Re + cb.Im * cb.Im;
                re = re / mag2;
                im = (Rat.Zero - im) / mag2;
            }
            return Complex(re, im);
        }

        return MakePower(b, e);
    }

    private static Expr MakePower(Expr b, Expr e)
    {
        var node = new PowerExpr(b, e);
        node._hash = Expr.Combine((int)NodeKind.Power, b._hash, e._hash);
        node._nodeCount = 1 + b._nodeCount + e._nodeCount;
        node._isExact = b.IsExact && ExponentIsInteger(e);
        return Current.Intern(node);
    }

    /// <summary>a / b → a · b⁻¹ (canonical division).</summary>
    public static Expr Divide(Expr a, Expr b) => Multiply(a, Power(b, Rational(Rat.MinusOne)));

    /// <summary>-a → -1 · a.</summary>
    public static Expr Negate(Expr a) => Multiply(Rational(Rat.MinusOne), a);

    /// <summary>a - b.</summary>
    public static Expr Subtract(Expr a, Expr b) => Add(a, Negate(b));

    public static Expr Sqrt(Expr a) => Power(a, Rational(1, 2));

    public static Expr Relation(RelOp op, Expr left, Expr right)
    {
        var node = new RelationExpr(op, left, right);
        node._hash = Expr.Combine((int)NodeKind.Relation, (int)op, left._hash, right._hash);
        node._nodeCount = 1 + left._nodeCount + right._nodeCount;
        node._isExact = left.IsExact && right.IsExact;
        return Current.Intern(node);
    }

    public static Expr Piecewise(IEnumerable<PiecewiseBranch> branches, Expr otherwise)
    {
        var list = branches.ToImmutableArray();
        var node = new PiecewiseExpr(list, otherwise);
        int h = otherwise._hash;
        int count = 1 + otherwise._nodeCount;
        bool exact = otherwise.IsExact;
        foreach (var br in list)
        {
            h = System.HashCode.Combine(h, br.Guard._hash, br.Value._hash);
            count += 1 + br.Guard._nodeCount + br.Value._nodeCount;
            exact &= br.Guard.IsExact && br.Value.IsExact;
        }
        node._hash = Expr.Combine((int)NodeKind.Piecewise, h);
        node._nodeCount = count;
        node._isExact = exact;
        return Current.Intern(node);
    }

    public static Expr Derivative(Expr operand, params Symbol[] variables)
    {
        var vars = variables.ToImmutableArray();
        var node = new DerivativeExpr(operand, vars);
        int h = operand._hash;
        foreach (var v in vars) h = System.HashCode.Combine(h, v.GetHashCode());
        node._hash = Expr.Combine((int)NodeKind.Derivative, h);
        node._nodeCount = 1 + operand._nodeCount;
        node._isExact = operand.IsExact;
        return Current.Intern(node);
    }

    public static Expr RootOf(Polynomial definingPolynomial, int rootIndex)
    {
        var node = new RootOfExpr(definingPolynomial, rootIndex);
        node._hash = Expr.Combine((int)NodeKind.RootOf, definingPolynomial.GetHashCode(), rootIndex);
        node._nodeCount = 1 + definingPolynomial.TermCount;
        node._isExact = true;
        return Current.Intern(node);
    }

    public static Expr Integral(Expr operand, params Symbol[] variables)
    {
        var vars = variables.ToImmutableArray();
        var node = new IntegralExpr(operand, vars);
        int h = operand._hash;
        foreach (var v in vars) h = System.HashCode.Combine(h, v.GetHashCode());
        node._hash = Expr.Combine((int)NodeKind.Integral, h);
        node._nodeCount = 1 + operand._nodeCount;
        node._isExact = operand.IsExact;
        return Current.Intern(node);
    }

    // -----------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------

    public static readonly Expr Zero = Rational(Rat.Zero);
    public static readonly Expr One = Rational(Rat.One);
    public static readonly Expr MinusOne = Rational(Rat.MinusOne);

    internal static int CombineHashes(ImmutableArray<Expr> items)
    {
        int h = 0;
        foreach (var i in items) h = System.HashCode.Combine(h, i._hash);
        return h;
    }

    internal static int SumCounts(ImmutableArray<Expr> items)
    {
        int c = 0;
        foreach (var i in items) c += i._nodeCount;
        return c;
    }

    private static Rat RationalPowerInt(Rat b, int e)
    {
        if (e == 0) return Rat.One;
        if (e < 0)
        {
            if (b.IsZero) throw new DivideByZeroException("0 raised to a negative power.");
            return Rat.One / RationalPowerInt(b, -e);
        }
        var r = Rat.One;
        var x = b;
        int n = e;
        while (n > 0)
        {
            if ((n & 1) == 1) r = r * x;
            x = x * x;
            n >>= 1;
        }
        return r;
    }

    /// <summary>Exact integer n-th root of a non-negative Integer, or null.</summary>
    internal static Int? IntegerRoot(Int x, int n)
    {
        if (x < Int.Zero || n < 1)
            return null;
        if (x == Int.Zero || x == Int.One)
            return x;
        // initial guess: 10^(digits/n)
        int digits = x.ToString().Length;
        var guess = new Int(10).Pow(new Int(Math.Max(1, digits / n)));
        var nm1 = new Int(n - 1L);
        while (true)
        {
            var gnm1 = guess.Pow(nm1);
            var next = (guess * nm1 + x / gnm1) / new Int(n);
            if (next == guess)
                break;
            guess = next;
        }
        if (guess.Pow(new Int(n)) == x)
            return guess;
        if ((guess + Int.One).Pow(new Int(n)) == x)
            return guess + Int.One;
        return null;
    }

    /// <summary>Exact n-th root of a rational (Gaussian handling for negative values with odd n), or null.</summary>
    private static Rat? RationalRoot(Rat b, int n)
    {
        var neg = b.IsNegative;
        var p = Rat.Abs(b).Numerator;
        var q = Rat.Abs(b).Denominator;
        if (neg && n % 2 == 0)
            return null;   // even root of negative → complex
        var rp = IntegerRoot(p, n);
        if (rp is null) return null;
        var rq = IntegerRoot(q, n);
        if (rq is null) return null;
        var root = Rat.From(rp, rq);
        return neg ? Rat.Negate(root) : root;
    }
}

public static class IntegerExtensions
{
    public static long ToInt64Saturating(this Int value)
    {
        var s = value.ToString();
        return long.TryParse(s, out var r) ? r : (value < Int.Zero ? long.MinValue : long.MaxValue);
    }

    public static bool IsEvenInteger(this Int value) => Int.IsEvenInteger(value);
}
