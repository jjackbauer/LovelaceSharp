using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

public enum MonomialOrder { Lex, GrLex, GrRevLex }

/// <summary>The fixed sequence of variables a polynomial lives in.</summary>
public sealed class VariableOrder
{
    public Symbol[] Variables { get; }

    public VariableOrder(Symbol[] variables) => Variables = variables;

    public int IndexOf(string name)
    {
        for (int i = 0; i < Variables.Length; i++)
            if (Variables[i].Name == name)
                return i;
        return -1;
    }
}

/// <summary>A monomial: sorted (variableIndex → exponent) pairs, stored densely over the variable order.</summary>
public sealed class Monomial : IEquatable<Monomial>
{
    internal int[] Exps { get; }

    internal Monomial(int[] exps) => Exps = exps;

    internal static Monomial Unit(int varCount) => new(new int[varCount]);

    public int this[int varIndex] => varIndex < Exps.Length ? Exps[varIndex] : 0;

    public int TotalDegree
    {
        get
        {
            int d = 0;
            foreach (var e in Exps) d += e;
            return d;
        }
    }

    public bool Equals(Monomial? other) => other is not null && Exps.SequenceEqual(other.Exps);
    public override bool Equals(object? obj) => obj is Monomial m && Equals(m);
    public override int GetHashCode()
    {
        int h = 0;
        foreach (var e in Exps) h = System.HashCode.Combine(h, e);
        return h;
    }

    public override string ToString() => "x^(" + string.Join(",", Exps) + ")";
}

internal sealed class MonomialComparer : IComparer<Monomial>
{
    private readonly MonomialOrder _order;

    public MonomialComparer(MonomialOrder order) => _order = order;

    public int Compare(Monomial? a, Monomial? b)
    {
        if (a is null || b is null) return 0;
        int tdA = a.TotalDegree, tdB = b.TotalDegree;
        if (_order != MonomialOrder.Lex)
        {
            if (tdA != tdB) return tdA.CompareTo(tdB);
        }
        // lex comparison over the variable sequence (reversed exponents for GrRevLex)
        var ea = a.Exps;
        var eb = b.Exps;
        int n = Math.Max(ea.Length, eb.Length);
        for (int i = 0; i < n; i++)
        {
            int ia = _order == MonomialOrder.GrRevLex ? n - 1 - i : i;
            int va = ia < ea.Length ? ea[ia] : 0;
            int vb = ia < eb.Length ? eb[ia] : 0;
            if (_order == MonomialOrder.GrRevLex)
            {
                // smaller total exponent in the tail first
                if (va != vb) return vb.CompareTo(va);
            }
            else
            {
                if (va != vb) return va.CompareTo(vb);
            }
        }
        return 0;
    }
}

/// <summary>
/// Sparse multivariate polynomial with Rational coefficients. Canonical storage is
/// GrLex-sorted; algorithms take an explicit <see cref="MonomialOrder"/> parameter.
/// </summary>
public sealed class Polynomial : IEquatable<Polynomial>, IComparable<Polynomial>
{
    private readonly SortedDictionary<Monomial, Rat> _terms;

    public VariableOrder Order { get; }

    public Polynomial(VariableOrder order)
    {
        Order = order;
        _terms = new SortedDictionary<Monomial, Rat>(new MonomialComparer(MonomialOrder.GrLex));
    }

    private Polynomial(VariableOrder order, SortedDictionary<Monomial, Rat> terms)
    {
        Order = order;
        _terms = terms;
    }

    public IEnumerable<KeyValuePair<Monomial, Rat>> Terms => _terms;

    public int TermCount => _terms.Count;

    public bool IsZero => _terms.Count == 0;

    public bool IsOne => _terms.Count == 1 && _terms.First().Key.TotalDegree == 0 && _terms.First().Value.IsOne;

    public int TotalDegree => IsZero ? -1 : _terms.Keys.Max(m => m.TotalDegree);

    public Rat LeadingCoefficient(MonomialOrder order)
    {
        if (IsZero) return Rat.Zero;
        var cmp = new MonomialComparer(order);
        var best = _terms.Keys.First();
        foreach (var m in _terms.Keys.Skip(1))
            if (cmp.Compare(m, best) > 0) best = m;
        return _terms[best];
    }

    public Rat ConstantTerm => _terms.TryGetValue(Monomial.Unit(Order.Variables.Length), out var c) ? c : Rat.Zero;

    public static Polynomial Zero(VariableOrder order) => new(order);

    public static Polynomial FromConstant(VariableOrder order, Rat c)
    {
        var p = new Polynomial(order);
        if (!c.IsZero)
            p._terms[Monomial.Unit(order.Variables.Length)] = c;
        return p;
    }

    public static Polynomial FromMonomial(VariableOrder order, Monomial m, Rat coef)
    {
        var p = new Polynomial(order);
        if (!coef.IsZero)
            p._terms[m] = coef;
        return p;
    }

    public static Polynomial FromVar(VariableOrder order, int varIndex)
    {
        var exps = new int[order.Variables.Length];
        exps[varIndex] = 1;
        return FromMonomial(order, new Monomial(exps), Rat.One);
    }

    // -----------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------

    public static Polynomial Add(Polynomial a, Polynomial b)
    {
        var r = new Polynomial(a.Order);
        foreach (var (m, c) in a._terms) r._terms[m] = c;
        foreach (var (m, c) in b._terms)
        {
            if (!r._terms.TryGetValue(m, out var prev)) prev = Rat.Zero;
            var sum = prev + c;
            if (sum.IsZero) r._terms.Remove(m);
            else r._terms[m] = sum;
        }
        return r;
    }

    public static Polynomial Subtract(Polynomial a, Polynomial b)
    {
        var r = new Polynomial(a.Order);
        foreach (var (m, c) in a._terms) r._terms[m] = c;
        foreach (var (m, c) in b._terms)
        {
            if (!r._terms.TryGetValue(m, out var prev)) prev = Rat.Zero;
            var sum = prev - c;
            if (sum.IsZero) r._terms.Remove(m);
            else r._terms[m] = sum;
        }
        return r;
    }

    public static Polynomial Negate(Polynomial a)
    {
        var r = new Polynomial(a.Order);
        foreach (var (m, c) in a._terms) r._terms[m] = Rat.Negate(c);
        return r;
    }

    public static Polynomial Multiply(Polynomial a, Polynomial b)
    {
        var r = new Polynomial(a.Order);
        foreach (var (ma, ca) in a._terms)
        {
            foreach (var (mb, cb) in b._terms)
            {
                var m = MultMonomial(ma, mb);
                var c = ca * cb;
                if (!r._terms.TryGetValue(m, out var prev)) prev = Rat.Zero;
                var sum = prev + c;
                if (sum.IsZero) r._terms.Remove(m);
                else r._terms[m] = sum;
            }
        }
        return r;
    }

    private static Monomial MultMonomial(Monomial a, Monomial b)
    {
        int n = Math.Max(a.Exps.Length, b.Exps.Length);
        var e = new int[n];
        for (int i = 0; i < n; i++)
            e[i] = (i < a.Exps.Length ? a.Exps[i] : 0) + (i < b.Exps.Length ? b.Exps[i] : 0);
        return new Monomial(e);
    }

    /// <summary>Multivariate division by a single divisor under the given monomial order.</summary>
    public (Polynomial Quotient, Polynomial Remainder) DivRem(Polynomial divisor, MonomialOrder order)
    {
        var cmp = new MonomialComparer(order);
        var q = new Polynomial(Order);
        var p = this;
        var r = new Polynomial(Order);
        while (!p.IsZero)
        {
            var (ltm, ltc) = LeadingTerm(p, cmp);
            var (dlm, dlc) = LeadingTerm(divisor, cmp);
            if (dlc.IsZero)
                throw new DivideByZeroException("Division by zero polynomial.");
            if (Divisible(ltm, dlm))
            {
                var (qm, qc) = DivideMonomial(ltm, ltc, dlm, dlc);
                q = Add(q, FromMonomial(Order, qm, qc));
                p = Subtract(p, Multiply(FromMonomial(Order, qm, qc), divisor));
            }
            else
            {
                r = Add(r, FromMonomial(Order, ltm, ltc));
                p = Subtract(p, FromMonomial(Order, ltm, ltc));
            }
        }
        return (q, r);
    }

    private static (Monomial, Rat) LeadingTerm(Polynomial p, MonomialComparer cmp)
    {
        Monomial best = p._terms.Keys.First();
        foreach (var m in p._terms.Keys.Skip(1))
            if (cmp.Compare(m, best) > 0) best = m;
        return (best, p._terms[best]);
    }

    private static bool Divisible(Monomial a, Monomial b)
    {
        int n = Math.Max(a.Exps.Length, b.Exps.Length);
        for (int i = 0; i < n; i++)
        {
            int ea = i < a.Exps.Length ? a.Exps[i] : 0;
            int eb = i < b.Exps.Length ? b.Exps[i] : 0;
            if (ea < eb) return false;
        }
        return true;
    }

    private static (Monomial, Rat) DivideMonomial(Monomial a, Rat ca, Monomial b, Rat cb)
    {
        int n = Math.Max(a.Exps.Length, b.Exps.Length);
        var e = new int[n];
        for (int i = 0; i < n; i++)
            e[i] = (i < a.Exps.Length ? a.Exps[i] : 0) - (i < b.Exps.Length ? b.Exps[i] : 0);
        return (new Monomial(e), ca / cb);
    }

    public static Polynomial Pow(Polynomial a, int n)
    {
        var r = FromConstant(a.Order, Rat.One);
        var x = a;
        while (n > 0)
        {
            if ((n & 1) == 1) r = Multiply(r, x);
            x = Multiply(x, x);
            n >>= 1;
        }
        return r;
    }

    public Rat EvaluateAt(Rat[] point)
    {
        // direct term evaluation (Horner later)
        Rat sum = Rat.Zero;
        foreach (var (m, c) in _terms)
        {
            Rat term = c;
            for (int i = 0; i < point.Length && i < m.Exps.Length; i++)
            {
                if (m.Exps[i] == 0) continue;
                term = term * Rat.Pow(point[i], m.Exps[i]);
            }
            sum = sum + term;
        }
        return sum;
    }

    public Polynomial Derivative(int varIndex)
    {
        var r = new Polynomial(Order);
        foreach (var (m, c) in _terms)
        {
            if (varIndex >= m.Exps.Length || m.Exps[varIndex] == 0) continue;
            var e = (int[])m.Exps.Clone();
            e[varIndex]--;
            var coef = c * Rat.FromLong(e[varIndex] + 1);
            if (!r._terms.TryGetValue(new Monomial(e), out var prev)) prev = Rat.Zero;
            var sum = prev + coef;
            if (sum.IsZero) r._terms.Remove(new Monomial(e));
            else r._terms[new Monomial(e)] = sum;
        }
        return r;
    }

    // -----------------------------------------------------------------
    // Univariate algorithms (order may carry several variables; these treat var 0)
    // -----------------------------------------------------------------

    /// <summary>Euclidean GCD over Q for univariate polynomials.</summary>
    public static Polynomial GcdUnivariate(Polynomial a, Polynomial b)
    {
        a = NormalizeUnivariate(a);
        b = NormalizeUnivariate(b);
        while (!b.IsZero)
        {
            var (_, r) = a.DivRem(b, MonomialOrder.Lex);
            a = b;
            b = NormalizeUnivariate(r);
        }
        return NormalizeUnivariate(a);
    }

    private static Polynomial NormalizeUnivariate(Polynomial p)
    {
        // make monic (leading coefficient 1)
        if (p.IsZero) return p;
        var lc = p.LeadingCoefficient(MonomialOrder.Lex);
        if (lc.IsOne) return p;
        var r = new Polynomial(p.Order);
        foreach (var (m, c) in p._terms)
            r._terms[m] = c / lc;
        return r;
    }

    /// <summary>Yun's square-free decomposition: returns (factor, multiplicity) pairs.</summary>
    public static List<(Polynomial Factor, int Multiplicity)> SquareFreeUnivariate(Polynomial f)
    {
        var result = new List<(Polynomial, int)>();
        if (f.IsZero) return result;
        var fPrime = f.Derivative(0);
        var a = GcdUnivariate(f, fPrime);
        if (a.IsOne || a.IsZero)
        {
            // f is already squarefree
            result.Add((NormalizeUnivariate(f), 1));
            return result;
        }
        var b = f.DivRem(a, MonomialOrder.Lex).Quotient;
        var c = fPrime.DivRem(a, MonomialOrder.Lex).Quotient;
        var d = Subtract(c, b.Derivative(0));
        int i = 1;
        while (!b.IsOne && !b.IsZero)
        {
            var g = GcdUnivariate(b, d);
            // Yun: the gcd a_i IS the square-free factor of multiplicity i; the loop state
            // advances along the quotient b/a_i. Emitting the quotient instead corrupts
            // every multiplicity (e.g. x^2(x-1) yielded [(x,1),(x-1,3)]).
            if (!g.IsOne)
                result.Add((NormalizeUnivariate(g), i));
            b = b.DivRem(g, MonomialOrder.Lex).Quotient;
            c = d.DivRem(g, MonomialOrder.Lex).Quotient;
            d = Subtract(c, b.Derivative(0));
            i++;
        }
        if (result.Count == 0)
            result.Add((NormalizeUnivariate(f), 1));
        return result;
    }

    /// <summary>Product of the distinct irreducible square-free factors (the square-free part).</summary>
    public static Polynomial SquareFreePart(Polynomial f)
    {
        if (f.IsZero)
            return f;
        var acc = FromConstant(f.Order, Rat.One);
        foreach (var (fac, _) in SquareFreeUnivariate(f))
            acc = Multiply(acc, fac);
        return acc;
    }

    /// <summary>Rational roots of a univariate polynomial (rational root theorem, complete divisor search).</summary>
    public List<Rat> RationalRoots()
    {
        var roots = new List<Rat>();
        if (IsZero) return roots;
        // scale to integer coefficients
        var scaled = ToIntegerCoefficients();
        var ints = scaled._terms;
        if (ints.Count == 0) return roots;

        // strip the x^k factor: with a zero constant term the rational root theorem's
        // p | a0 condition is vacuous, so other rational roots would be missed (e.g.
        // x(x-1/1000)(x-1) missed 1/1000 and 1)
        var reduced = new Polynomial(Order);
        long minExp = long.MaxValue;
        foreach (var (m, _) in ints)
            minExp = Math.Min(minExp, m.Exps.Length > 0 ? m.Exps[0] : 0);
        if (minExp > 0)
        {
            roots.Add(Rat.Zero);
            foreach (var (m, c) in ints)
            {
                var e = m.Exps.ToArray();
                e[0] -= (int)minExp;
                reduced = Add(reduced, FromMonomial(Order, new Monomial(e), c));
            }
        }
        else
        {
            reduced = scaled;
        }

        var a0 = reduced._terms.TryGetValue(Monomial.Unit(Order.Variables.Length), out var c0) ? c0 : Rat.Zero;
        var an = reduced.LeadingCoefficient(MonomialOrder.Lex);
        foreach (var p in DivisorsOf(a0.Numerator))
        {
            foreach (var q in DivisorsOf(an.Numerator))
            {
                if (Int.IsZero(q)) continue;
                var cand = Rat.From(p, q);
                foreach (var r in new[] { cand, Rat.Negate(cand) })
                {
                    if (roots.Contains(r)) continue;
                    if (reduced.EvaluateAt(new[] { r }).IsZero)
                        roots.Add(r);
                }
            }
        }
        roots.Sort();
        return roots;
    }

    private Polynomial ToIntegerCoefficients()
    {
        // multiply by lcm of coefficient denominators
        Rat lcm = Rat.One;
        foreach (var (_, c) in _terms)
        {
            if (c.Denominator != Int.One)
            {
                lcm = LcmRat(lcm, c);
            }
        }
        var r = new Polynomial(Order);
        foreach (var (m, c) in _terms)
            r._terms[m] = c * lcm;
        return r;
    }

    private static Rat LcmRat(Rat a, Rat b)
    {
        // lcm of the denominators (values here are integers)
        var l = Int.Lcm(a.Denominator, b.Denominator);
        return Rat.From(l);
    }

    private static List<Int> DivisorsOf(Int n)
    {
        var result = new List<Int>();
        if (Int.IsZero(n)) return result;
        var abs = Int.Abs(n);
        var limit = IntSqrt(abs);
        for (var d = Int.One; d <= limit; d = d + Int.One)
        {
            if (abs % d == Int.Zero)
            {
                result.Add(d);
                var pair = abs / d;
                if (pair != d)
                    result.Add(pair);
            }
        }
        result.Sort();
        return result;
    }

    private static Int IntSqrt(Int n)
    {
        if (n < Int.One) return Int.Zero;
        var x = new Int(10).Pow(new Int(Math.Max(1, n.ToString().Length / 2)));
        var two = new Int(2L);
        while (true)
        {
            var next = (x + n / x) / two;
            if (next == x || next == x + Int.One)
                return next;
            x = next;
        }
    }

    // -----------------------------------------------------------------
    // Expression conversion
    // -----------------------------------------------------------------

    public Expr ToExpr()
    {
        var terms = new List<Expr>();
        foreach (var (m, c) in _terms)
        {
            var factors = new List<Expr>();
            if (!c.IsOne)
                factors.Add(Exprs.Rational(c));
            for (int i = 0; i < m.Exps.Length; i++)
            {
                if (m.Exps[i] == 0) continue;
                var v = Exprs.Symbol(Order.Variables[i]);
                factors.Add(m.Exps[i] == 1 ? v : Exprs.Power(v, Exprs.Integer(m.Exps[i])));
            }
            terms.Add(factors.Count == 0 ? Exprs.One : factors.Count == 1 ? factors[0] : Exprs.Multiply(factors));
        }
        return terms.Count == 0 ? Exprs.Zero : Exprs.Add(terms);
    }

    public static bool TryFromExpr(Expr e, ExprContext ctx, Symbol[] variables, out Polynomial poly, out string? reason)
    {
        var order = new VariableOrder(variables);
        poly = new Polynomial(order);
        reason = null;
        if (TryConvert(e, ctx, order, out var p, out reason))
        {
            poly = p;
            return true;
        }
        return false;
    }

    public static Polynomial FromExpr(Expr e, ExprContext ctx, Symbol[] variables)
    {
        if (TryFromExpr(e, ctx, variables, out var poly, out var reason))
            return poly;
        throw new NotPolynomialException(reason ?? "Expression is not a polynomial.");
    }

    private static bool TryConvert(Expr e, ExprContext ctx, VariableOrder order, out Polynomial p, out string? reason)
    {
        switch (e)
        {
            case IntegerConstantExpr i:
                p = FromConstant(order, Rat.From(i.Value));
                reason = null;
                return true;
            case RationalConstantExpr r:
                p = FromConstant(order, r.Value);
                reason = null;
                return true;
            case SymbolExpr s:
            {
                int idx = order.IndexOf(s.Symbol.Name);
                if (idx < 0)
                {
                    p = Zero(order);
                    reason = $"Symbol '{s.Symbol.Name}' is not a polynomial variable.";
                    return false;
                }
                p = FromVar(order, idx);
                reason = null;
                return true;
            }
            case AddExpr a:
            {
                p = Zero(order);
                foreach (var t in a.Terms)
                {
                    if (!TryConvert(t, ctx, order, out var tp, out reason))
                        return false;
                    p = Add(p, tp);
                }
                reason = null;
                return true;
            }
            case MultiplyExpr m:
            {
                p = FromConstant(order, Rat.One);
                foreach (var f in m.Factors)
                {
                    if (!TryConvert(f, ctx, order, out var fp, out reason))
                        return false;
                    p = Multiply(p, fp);
                }
                reason = null;
                return true;
            }
            case PowerExpr pw:
            {
                if (pw.Base is SymbolExpr sv && pw.Exponent is RationalConstantExpr pe && pe.Value.IsInteger && !pe.Value.IsNegative)
                {
                    int idx = order.IndexOf(sv.Symbol.Name);
                    if (idx < 0)
                    {
                        p = Zero(order);
                        reason = $"Symbol '{sv.Symbol.Name}' is not a polynomial variable.";
                        return false;
                    }
                    int n = (int)pe.Value.ToInteger().ToInt64Saturating();
                    var exps = new int[order.Variables.Length];
                    exps[idx] = n;
                    p = FromMonomial(order, new Monomial(exps), Rat.One);
                    reason = null;
                    return true;
                }
                p = Zero(order);
                reason = "Non-integer or non-variable powers are not polynomial.";
                return false;
            }
            default:
                p = Zero(order);
                reason = $"Node kind {e.Kind} is not polynomial.";
                return false;
        }
    }

    public bool Equals(Polynomial? other) =>
        other is not null && other._terms.Count == _terms.Count &&
        other._terms.SequenceEqual(_terms) && Order.Variables.SequenceEqual(other.Order.Variables);

    public override bool Equals(object? obj) => obj is Polynomial p && Equals(p);

    public override int GetHashCode()
    {
        int h = 0;
        foreach (var (m, c) in _terms) h = System.HashCode.Combine(h, m.GetHashCode(), c.GetHashCode());
        return h;
    }

    public int CompareTo(Polynomial? other)
    {
        if (other is null) return 1;
        var cmp = new MonomialComparer(MonomialOrder.GrLex);
        var ea = _terms.GetEnumerator();
        var eb = other._terms.GetEnumerator();
        while (ea.MoveNext() && eb.MoveNext())
        {
            int c = cmp.Compare(ea.Current.Key, eb.Current.Key);
            if (c != 0) return c;
            c = ea.Current.Value.CompareTo(eb.Current.Value);
            if (c != 0) return c;
        }
        if (ea.MoveNext()) return 1;
        if (eb.MoveNext()) return -1;
        return 0;
    }
}

public sealed class NotPolynomialException : Exception
{
    public NotPolynomialException(string message) : base(message) { }
}

/// <summary>Exact dense linear solve over Rational (fraction-free Gaussian elimination).</summary>
public static class RationalLinear
{
    public static bool TrySolve(Rat[,] a, Rat[] b, out Rat[] x)
    {
        x = Array.Empty<Rat>();
        int n = b.Length;
        if (a.GetLength(0) != n || a.GetLength(1) != n)
            return false;
        var m = new Rat[n, n + 1];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++)
                m[i, j] = a[i, j];
            m[i, n] = b[i];
        }
        for (int k = 0; k < n; k++)
        {
            int pivot = k;
            for (int i = k; i < n; i++)
                if (!m[i, k].IsZero) { pivot = i; break; }
            if (m[pivot, k].IsZero)
                return false;
            if (pivot != k)
                for (int j = k; j <= n; j++)
                    (m[pivot, j], m[k, j]) = (m[k, j], m[pivot, j]);
            for (int i = 0; i < n; i++)
            {
                if (i == k || m[i, k].IsZero) continue;
                var factor = m[i, k] / m[k, k];
                for (int j = k; j <= n; j++)
                    m[i, j] = m[i, j] - factor * m[k, j];
            }
        }
        x = new Rat[n];
        for (int i = 0; i < n; i++)
            x[i] = m[i, n] / m[i, i];
        return true;
    }
}
