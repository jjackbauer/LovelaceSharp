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
        // arity contract: registered definitions are enforced; unknown function names remain
        // open so plugins/extensions can introduce functions before registering them
        if (Current.Functions.Get(id.Name) is { } def && !def.Variadic && def.Arity != imm.Length)
            throw new ArgumentException($"Function '{id.Name}' takes {def.Arity} argument(s), got {imm.Length}.");
        // numeric constant folding for total functions
        var folded = Evaluation.FoldFunction(id, imm);
        if (folded is not null)
            return folded;
        var n = new FunctionExpr(id, imm);
        n._hash = Expr.Combine((int)NodeKind.Function, id.GetHashCode(), CombineHashes(imm));
        n._nodeCount = 1 + SumCounts(imm);
        n._isExact = IsExactFunction(imm);
        return Current.Intern(n);
    }

    /// <summary>
    /// Exactness of a function application: true EXACTLY when no argument carries an approximation
    /// or a transcendental/indeterminate named constant (the leaf rule stated on
    /// <see cref="Expr.IsExact"/>).
    /// <para>The function's own identity is deliberately NOT consulted. The previous rule admitted
    /// only "abs", "sign", "floor", "ceil", "min" and "max", so sin(x), cos(x), sqrt(x), log(x),
    /// exp(x) and every other elementary closed form reported INEXACT over exact arguments — the
    /// opposite of what the flag means. Nothing here inspects a function's values, domain or
    /// branch: an application over exact arguments denotes exactly what it denotes, and whether
    /// that value is rational, irrational or complex is a different question from whether an
    /// approximation entered the expression.</para>
    /// </summary>
    private static bool IsExactFunction(ImmutableArray<Expr> args) => args.All(a => a.IsExact);

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
            terms.Add(Real(RealLiteral.FromRationalExact(real)));
        if (!(cplxRe.IsZero && cplxIm.IsZero))
            terms.Add(Complex(cplxRe, cplxIm));

        foreach (var (rest, coef) in like)
        {
            if (coef.IsZero)
            {
                // a zero coefficient over a pole-carrying or non-finite remainder must not
                // vanish: dropping 0·x⁻¹ would define the sum at 0 where it is undefined, and
                // dropping 0·inf (or inf - inf, whose coefficient is 0 over the remainder inf)
                // would return a definite 0 for an indeterminate form
                if (HasUndefinedRisk(rest))
                    terms.Add(Multiply(Rational(coef), rest));
                continue;
            }
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
        // exact Real literals (e.g. 1.5) are rationals; without this, bound reasoning silently
        // stops working for real-typed bounds while the term order still compares them
        RealConstantExpr rl => rl.Value.ToRational(),
        _ => null,
    };

    /// <summary>
    /// True when the expression may be undefined somewhere because of a negative power, or a
    /// power whose exponent is not a provably nonnegative rational. Conservative: unknown
    /// exponents count as pole risks.
    /// </summary>
    internal static bool HasPoleRisk(Expr e) => e switch
    {
        PowerExpr p => NumericToRational(p.Exponent) is { } er
            ? er.IsNegative || HasPoleRisk(p.Base)
            : true,
        AddExpr a => a.Terms.Any(HasPoleRisk),
        MultiplyExpr m => m.Factors.Any(HasPoleRisk),
        FunctionExpr f => f.Arguments.Any(HasPoleRisk),
        PiecewiseExpr pw => pw.Branches.Any(b => HasPoleRisk(b.Guard) || HasPoleRisk(b.Value)) || HasPoleRisk(pw.Otherwise),
        DerivativeExpr d => HasPoleRisk(d.Operand),
        IntegralExpr i => HasPoleRisk(i.Operand),
        RelationExpr r => HasPoleRisk(r.Left) || HasPoleRisk(r.Right),
        _ => false,
    };

    /// <summary>
    /// True when the expression may fail to denote a defined, finite value: a pole risk
    /// (see <see cref="HasPoleRisk"/>) or the symbolic constant Infinity, possibly nested.
    /// The identities a - a = 0 and 0·a = 0 are only valid for a defined, finite a, so the
    /// zero-coefficient and zero-factor folds must consult this guard: over Infinity both
    /// forms are indeterminate (inf - inf, 0·inf) and folding them yields a definite wrong
    /// value. Pi and E are finite and are deliberately NOT hazards.
    /// </summary>
    internal static bool HasUndefinedRisk(Expr e) => e switch
    {
        NamedConstantExpr n => n.Constant == NamedConstant.Infinity,
        PowerExpr p => NumericToRational(p.Exponent) is { } er
            ? er.IsNegative || HasPoleRisk(p.Base) || HasUndefinedRisk(p.Base)
            : true,
        AddExpr a => a.Terms.Any(HasUndefinedRisk),
        MultiplyExpr m => m.Factors.Any(HasUndefinedRisk),
        FunctionExpr f => f.Arguments.Any(HasUndefinedRisk),
        PiecewiseExpr pw => pw.Branches.Any(b => HasUndefinedRisk(b.Guard) || HasUndefinedRisk(b.Value)) || HasUndefinedRisk(pw.Otherwise),
        DerivativeExpr d => HasUndefinedRisk(d.Operand),
        IntegralExpr i => HasUndefinedRisk(i.Operand),
        RelationExpr r => HasUndefinedRisk(r.Left) || HasUndefinedRisk(r.Right),
        _ => false,
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
        // Definedness-preserving merge: nonnegative and negative integer exponents accumulate
        // separately per base, so x·x⁻¹ can never collapse to 1 (that would define the
        // product at 0 where x⁻¹ is undefined). Negative sums merge with each other because
        // the pole set is unchanged.
        var posPowers = new Dictionary<Expr, Rat>();
        var negPowers = new Dictionary<Expr, Rat>();
        bool anyUndefinedRisk = flat.Any(HasUndefinedRisk);

        foreach (var c in flat)
        {
            switch (c)
            {
                case IntegerConstantExpr i:
                {
                    var v = Rat.From(i.Value);
                    if (v.IsZero)
                    {
                        // 0·(pole) and 0·inf must stay visible: 0/x is undefined at 0 while 0
                        // is defined, and 0·inf is indeterminate while 0 is defined
                        if (!anyUndefinedRisk) return Rational(Rat.Zero);
                        exact = Rat.Zero;
                        break;
                    }
                    if (v.IsOne) break;
                    exact = exact * v;
                    break;
                }
                case RationalConstantExpr r:
                {
                    if (r.Value.IsZero)
                    {
                        if (!anyUndefinedRisk) return Rational(Rat.Zero);
                        exact = Rat.Zero;
                        break;
                    }
                    if (r.Value.IsOne) break;
                    exact = exact * r.Value;
                    break;
                }
                case RealConstantExpr rl:
                {
                    var v = rl.Value.ToRational();
                    if (v.IsZero)
                    {
                        if (!anyUndefinedRisk) return Rational(Rat.Zero);
                        real = Rat.Zero;
                        hasReal = true;
                        break;
                    }
                    if (v.IsOne) break;
                    real = real * v;
                    hasReal = true;
                    break;
                }
                case ComplexConstantExpr cx:
                {
                    if (cx.Re.IsZero && cx.Im.IsZero)
                    {
                        if (!anyUndefinedRisk) return Rational(Rat.Zero);
                        cplxRe = Rat.Zero;
                        cplxIm = Rat.Zero;
                        break;
                    }
                    var nr = cplxRe * cx.Re - cplxIm * cx.Im;
                    var ni = cplxRe * cx.Im + cplxIm * cx.Re;
                    cplxRe = nr;
                    cplxIm = ni;
                    break;
                }
                case PowerExpr p when ExponentIsInteger(p.Exponent):
                {
                    var e = NumericToRational(p.Exponent) ?? Rat.One;
                    if (e.IsNegative)
                    {
                        if (!negPowers.TryGetValue(p.Base, out var prev)) prev = Rat.Zero;
                        negPowers[p.Base] = prev + e;
                    }
                    else
                    {
                        if (!posPowers.TryGetValue(p.Base, out var prev)) prev = Rat.Zero;
                        posPowers[p.Base] = prev + e;
                    }
                    break;
                }
                default:
                {
                    // a bare factor contributes a nonnegative exponent 1 over its own base
                    if (!posPowers.TryGetValue(c, out var prev)) prev = Rat.Zero;
                    posPowers[c] = prev + Rat.One;
                    break;
                }
            }
        }

        var factors = new List<Expr>();
        if (!exact.IsOne)
            factors.Add(Rational(exact));
        if (hasReal && !real.IsOne)
            factors.Add(Real(RealLiteral.FromRationalExact(real)));
        if (!(cplxRe.IsOne && cplxIm.IsZero))
            factors.Add(Complex(cplxRe, cplxIm));

        // per base: merge a positive and a negative exponent only when the combined exponent
        // stays negative (the pole set is unchanged); otherwise keep them separate so that
        // x·x⁻¹ can never collapse to 1 (that would define the product at 0)
        foreach (var b in posPowers.Keys.Concat(negPowers.Keys).Distinct())
        {
            var pe = posPowers.TryGetValue(b, out var pv) ? pv : Rat.Zero;
            var ne = negPowers.TryGetValue(b, out var nv) ? nv : Rat.Zero;
            if (pe.IsZero && ne.IsZero)
                continue;
            if (!pe.IsZero && ne.IsNegative && (pe + ne).IsNegative)
            {
                factors.Add(Power(b, Rational(pe + ne)));
                continue;
            }
            if (!pe.IsZero)
                factors.Add(pe.IsOne ? b : Power(b, Rational(pe)));
            if (!ne.IsZero)
                factors.Add(Power(b, Rational(ne)));
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

        // (b^e1)^k → b^(e1*k) only when both exponents are numeric constants.
        // A symbolic inner exponent (x^y)^k must never be replaced by a guessed exponent:
        // the previous form lost y entirely when y was not a rational constant.
        if (b is PowerExpr inner && eRat is { } ek && ek.IsInteger && NumericToRational(inner.Exponent) is { } e1)
            return Power(inner.Base, Rational(e1 * ek));

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

        // rational base with integer exponent: the exponent is an arbitrary-precision Int —
        // never narrowed through int/long (huge exponents must not wrap into a different value)
        if (b is (IntegerConstantExpr or RationalConstantExpr) && eRat is { IsInteger: true } ei)
        {
            var bv = NumericToRational(b) ?? Rat.Zero;
            return Rational(RationalPowerInt(bv, ei.ToInteger()));
        }

        // A NEGATIVE rational base with a HALF-INTEGER exponent (denominator exactly 2): the
        // principal branch of the square-root family, which is the branch SymPy returns for
        // (-a)**e. (-a)^(m/2) = a^(m/2)·i^m for a > 0, so the value is an exact imaginary
        // multiple — i, 2i, sqrt(3)·i, -8i — never a decimal approximation. This sits BEFORE the
        // unit-fraction exact-root test because that test's odd-denominator real-root shortcut must
        // not be reachable from an even denominator, and BEFORE the "leave it unevaluated" exit.
        // Exponents whose denominator is >= 3 are deliberately NOT handled here: their principal
        // value is a root of unity that is not a complex constant, so the kernel keeps its existing
        // behaviour and the host keeps reporting them unsupported (round 03 bounded scope).
        if (b is (IntegerConstantExpr or RationalConstantExpr) && eRat is { IsInteger: false } eHalf &&
            NumericToRational(b) is { IsNegative: true } bNegative &&
            PrincipalSquareRootPower(bNegative, eHalf) is { } principal)
            return principal;

        // rational base with unit-fraction exponent: exact root test
        if (b is (IntegerConstantExpr or RationalConstantExpr) && eRat is { } eu)
        {
            var bv = NumericToRational(b) ?? Rat.Zero;
            if (!eu.IsNegative)
            {
                var inv = Rat.One / eu;   // = n (eu = 1/n)
                if (inv.IsInteger && inv.ToInteger() > Int.One)
                {
                    var root = RationalRoot(bv, inv.ToInteger());
                    if (root is { } r)
                        return Rational(r);
                }
            }
            if (!eu.IsInteger && bv.IsNegative)
                return MakePower(b, e);   // complex-valued: leave unevaluated
        }

        // the imaginary unit with an INTEGER exponent: i^0 = 1, i^1 = i, i^2 = -1, i^3 = -i.
        // Folding this is what lets a complex solution substitute back to EXACTLY zero (i^2 must
        // become -1 rather than stay an opaque power), which the differential oracle's solve corpus
        // requires. The explicit 4-cycle is used instead of the general complex-exponentiation block
        // below so that the spelling of -i is the one the solver's own complex closed forms use.
        if (b is NamedConstantExpr { Constant: NamedConstant.I } && eRat is { IsInteger: true } eImaginary)
        {
            var twoI = new Int(2L);
            var fourI = new Int(4L);
            var residue = eImaginary.ToInteger() % fourI;
            if (Int.IsNegative(residue))
                residue = residue + fourI;      // canonical remainder, so i^-1 = i^3
            if (Int.IsZero(residue))
                return Rational(Rat.One);
            if (residue == twoI)
                return Rational(Rat.MinusOne);
            return residue == Int.One ? I : Negate(I);
        }

        // complex constant base with integer exponent: exact binary exponentiation over Int
        if (b is ComplexConstantExpr cb && eRat is { IsInteger: true } eci)
        {
            var n = eci.ToInteger();
            Rat re = Rat.One, im = Rat.Zero;
            Rat xr = cb.Re, xi = cb.Im;
            var k = Int.Abs(n);
            var two = new Int(2L);
            while (k > Int.Zero)
            {
                if (Int.IsOddInteger(k))
                {
                    var nr = re * xr - im * xi;
                    var ni = re * xi + im * xr;
                    re = nr; im = ni;
                }
                var sr = xr * xr - xi * xi;
                var si = Rat.FromLong(2L) * xr * xi;
                xr = sr; xi = si;
                k = k / two;
            }
            if (n < Int.Zero)
            {
                var mag2 = cb.Re * cb.Re + cb.Im * cb.Im;
                re = re / mag2;
                im = (Rat.Zero - im) / mag2;
            }
            return Complex(re, im);
        }

        return MakePower(b, e);
    }

    /// <summary>
    /// The PRINCIPAL branch of <c>b^e</c> for an exact NEGATIVE rational base and an exponent whose
    /// denominator is exactly 2; null for every other shape, so the caller falls through to the
    /// kernel's existing behaviour.
    /// <para><c>(-a)^(m/2) = a^(m/2)·i^m</c> for <c>a &gt; 0</c> and odd <c>m</c> — an exact
    /// imaginary multiple of a radical. SymPy 1.14.0 returns exactly these values, verified per case
    /// rather than assumed (<c>(-1)^(1/2) = I</c>, <c>(-4)^(1/2) = 2*I</c>,
    /// <c>(-3)^(1/2) = sqrt(3)*I</c>, <c>(-4)^(3/2) = -8*I</c>, <c>(-4)^(-1/2) = -I/2</c>; the
    /// commands and their output are in docs/goal-cycle-4/round-03/implementation.md). The former
    /// real-root shortcut is not reachable from here: it lives in the unit-fraction test below and
    /// only fires for ODD denominators, which is the kernel's separate, non-principal convention.</para>
    /// </summary>
    private static Expr? PrincipalSquareRootPower(Rat b, Rat e)
    {
        var two = new Int(2L);
        if (e.Denominator != two)
            return null;                     // an integer exponent was answered earlier
        var q = Rat.Abs(b);
        var m = e.Numerator;                 // odd: e is in lowest terms with denominator 2
        // a^(m/2) = a^((m-1)/2)·sqrt(a). The first factor is an exact integer power of a rational (m
        // is odd, so (m-1)/2 is an integer, and q > 0 keeps the negative exponent well defined); the
        // second folds to an exact rational when a is a perfect square and stays an exact radical
        // otherwise — both exact, so no approximation enters anywhere.
        var scale = RationalPowerInt(q, (m - Int.One) / two);
        var radical = Power(Rational(q), Rational(1, 2));
        // i^m for odd m: m ≡ 1 (mod 4) gives i, m ≡ 3 (mod 4) gives -i
        var four = new Int(4L);
        var unit = m % four == Int.One ? I : Negate(I);
        return Multiply(Rational(scale), radical, unit);
    }

    private static Expr MakePower(Expr b, Expr e)
    {
        var node = new PowerExpr(b, e);
        node._hash = Expr.Combine((int)NodeKind.Power, b._hash, e._hash);
        node._nodeCount = 1 + b._nodeCount + e._nodeCount;
        // A radical is exact: sqrt(2) and x^(1/2) are closed forms, not approximations, so the
        // question is the exponent's own exactness, not its integrality. This is the same
        // all-children rule Add (:215) and Multiply (:441) apply, and it keeps a power over an
        // approximating operand inexact: a numeric exponent was already normalized to a rational
        // constant above (so it carries no approximation), while a Real BASE fails b.IsExact and
        // a symbolic exponent that contains a Real leaf fails e.IsExact.
        node._isExact = b.IsExact && e.IsExact;
        return Current.Intern(node);
    }

    // -----------------------------------------------------------------
    // Logical connectives (boolean-valued operands; 0 = False, 1 = True)
    // -----------------------------------------------------------------

    public static Expr And(params Expr[] operands) => AndImpl(operands);

    public static Expr And(IEnumerable<Expr> operands) => AndImpl(operands.ToArray());

    private static Expr AndImpl(Expr[] operands)
    {
        var flat = new List<Expr>();
        foreach (var o in operands)
        {
            if (o is AndExpr a)
                flat.AddRange(a.Operands);
            else
                flat.Add(o);
        }
        var set = new List<Expr>();
        foreach (var o in flat)
        {
            if (o is RationalConstantExpr rc)
            {
                if (rc.Value.IsZero) return Zero;      // False ∧ … = False
                if (rc.Value.IsOne) continue;          // True is the identity
            }
            if (!set.Any(e => e.Equals(o)))
                set.Add(o);
        }
        if (set.Count == 0) return One;
        if (set.Count == 1) return set[0];
        set.Sort(TermOrder.Compare);
        var node = new AndExpr(ImmutableArray.CreateRange(set));
        node._hash = Expr.Combine((int)NodeKind.And, CombineHashes(node.Operands));
        node._nodeCount = 1 + SumCounts(node.Operands);
        node._isExact = node.Operands.All(o => o.IsExact);
        return Current.Intern(node);
    }

    public static Expr Or(params Expr[] operands) => OrImpl(operands);

    public static Expr Or(IEnumerable<Expr> operands) => OrImpl(operands.ToArray());

    private static Expr OrImpl(Expr[] operands)
    {
        var flat = new List<Expr>();
        foreach (var o in operands)
        {
            if (o is OrExpr or2)
                flat.AddRange(or2.Operands);
            else
                flat.Add(o);
        }
        var set = new List<Expr>();
        foreach (var o in flat)
        {
            if (o is RationalConstantExpr rc)
            {
                if (rc.Value.IsOne) return One;        // True ∨ … = True
                if (rc.Value.IsZero) continue;         // False is the identity
            }
            if (!set.Any(e => e.Equals(o)))
                set.Add(o);
        }
        if (set.Count == 0) return Zero;
        if (set.Count == 1) return set[0];
        set.Sort(TermOrder.Compare);
        var node = new OrExpr(ImmutableArray.CreateRange(set));
        node._hash = Expr.Combine((int)NodeKind.Or, CombineHashes(node.Operands));
        node._nodeCount = 1 + SumCounts(node.Operands);
        node._isExact = node.Operands.All(o => o.IsExact);
        return Current.Intern(node);
    }

    public static Expr Not(Expr operand) => operand switch
    {
        RationalConstantExpr rc when rc.Value.IsZero => One,
        RationalConstantExpr rc when rc.Value.IsOne => Zero,
        NotExpr n => n.Operand,
        _ => MakeNot(operand),
    };

    private static Expr MakeNot(Expr operand)
    {
        var node = new NotExpr(operand);
        node._hash = Expr.Combine((int)NodeKind.Not, operand._hash);
        node._nodeCount = 1 + operand._nodeCount;
        node._isExact = operand.IsExact;
        return Current.Intern(node);
    }

    /// <summary>
    /// Big-O truncation term: O((variable − point)^degree). The degree is normalized to a
    /// canonical integer constant when it is a numeric constant; no folding is performed.
    /// </summary>
    public static Expr Order(Expr variable, Expr point, Expr degree)
    {
        if (NumericToRational(degree) is { IsInteger: true } dr)
            degree = Integer(dr.ToInteger());
        var node = new OrderExpr(variable, point, degree);
        node._hash = Expr.Combine((int)NodeKind.Order, variable._hash, point._hash, degree._hash);
        node._nodeCount = 1 + variable._nodeCount + point._nodeCount + degree._nodeCount;
        node._isExact = variable.IsExact && point.IsExact && degree.IsExact;
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

    /// <summary>
    /// RootOf(p, i): the i-th real root (ascending) of the square-free part of the univariate
    /// polynomial p. Construction normalizes to the square-free part, so multiplicities never
    /// shift root indices.
    /// </summary>
    public static Expr RootOf(Polynomial definingPolynomial, int rootIndex)
    {
        var sf = Polynomial.SquareFreePart(definingPolynomial);
        if (sf.IsZero || sf.IsOne || sf.TotalDegree <= 0)
            throw new ArgumentException("RootOf requires a non-constant univariate polynomial.", nameof(definingPolynomial));
        var node = new RootOfExpr(sf, rootIndex);
        node._hash = Expr.Combine((int)NodeKind.RootOf, sf.GetHashCode(), rootIndex);
        node._nodeCount = 1 + sf.TermCount;
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

    // Per-ambient-context interned constants: static readonly fields would be interned in a
    // throwaway context, breaking reference identity against the caller's context pool.
    public static Expr Zero => Rational(Rat.Zero);
    public static Expr One => Rational(Rat.One);
    public static Expr MinusOne => Rational(Rat.MinusOne);

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

    private static Rat RationalPowerInt(Rat b, Int e)
    {
        if (Int.IsZero(e)) return Rat.One;
        if (e < Int.Zero)
        {
            if (b.IsZero) throw new DivideByZeroException("0 raised to a negative power.");
            return Rat.One / RationalPowerInt(b, Int.Abs(e));
        }
        var r = Rat.One;
        var x = b;
        var n = e;
        var two = new Int(2L);
        while (n > Int.Zero)
        {
            if (Int.IsOddInteger(n)) r = r * x;
            x = x * x;
            n = n / two;
        }
        return r;
    }

    /// <summary>Exact integer n-th root of a non-negative Integer, or null. n is arbitrary precision.</summary>
    internal static Int? IntegerRoot(Int x, Int n)
    {
        if (x < Int.Zero || n < Int.One)
            return null;
        if (x == Int.Zero || x == Int.One)
            return x;
        // initial guess: 10^(digits/n)
        int digits = x.ToString().Length;
        long nLong = n.ToInt64Saturating();
        var guess = new Int(10).Pow(new Int(Math.Max(1, digits / Math.Max(1, nLong))));
        var nm1 = n - Int.One;
        while (true)
        {
            var gnm1 = guess.Pow(nm1);
            var next = (guess * nm1 + x / gnm1) / n;
            // Newton iterates for integer roots oscillate between floor and floor+1 for
            // non-perfect powers (n=3: 10,5,2,1,2,1,...) — stop on either fixed point
            if (next == guess || next == guess + Int.One)
                break;
            guess = next;
        }
        if (guess.Pow(n) == x)
            return guess;
        if ((guess + Int.One).Pow(n) == x)
            return guess + Int.One;
        return null;
    }

    /// <summary>Exact n-th root of a rational (odd n handles negative values), or null.</summary>
    private static Rat? RationalRoot(Rat b, Int n)
    {
        var neg = b.IsNegative;
        var p = Rat.Abs(b).Numerator;
        var q = Rat.Abs(b).Denominator;
        if (neg && Int.IsEvenInteger(n))
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
