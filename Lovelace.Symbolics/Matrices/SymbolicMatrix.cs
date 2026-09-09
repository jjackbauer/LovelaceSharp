using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;

namespace Lovelace.Symbolics;

/// <summary>
/// Symbolic matrices with fraction-free algorithms (Bareiss determinant, adjugate inverse,
/// fraction-free linear solve). Pivots use conservative structural zero tests; exact division
/// goes through the polynomial engine over the matrix's variables.
/// </summary>
public sealed class SymbolicMatrix : IEquatable<SymbolicMatrix>
{
    public int Rows { get; }
    public int Columns { get; }
    private readonly Expr[] _data;

    private SymbolicMatrix(int rows, int columns, Expr[] data)
    {
        Rows = rows;
        Columns = columns;
        _data = data;
    }

    public Expr this[int r, int c]
    {
        get => _data[r * Columns + c];
    }

    public static SymbolicMatrix From(Expr[][] rows)
    {
        int r = rows.Length;
        int c = rows.Length == 0 ? 0 : rows[0].Length;
        var data = new Expr[r * c];
        for (int i = 0; i < r; i++)
            for (int j = 0; j < c; j++)
                data[i * c + j] = rows[i][j];
        return new SymbolicMatrix(r, c, data);
    }

    public static SymbolicMatrix From(Expr[,] m)
    {
        int r = m.GetLength(0), c = m.GetLength(1);
        var data = new Expr[r * c];
        for (int i = 0; i < r; i++)
            for (int j = 0; j < c; j++)
                data[i * c + j] = m[i, j];
        return new SymbolicMatrix(r, c, data);
    }

    public Expr[] ToFlatArray() => (Expr[])_data.Clone();

    public SymbolicMatrix Transpose()
    {
        var t = new Expr[Rows * Columns];
        for (int i = 0; i < Rows; i++)
            for (int j = 0; j < Columns; j++)
                t[j * Rows + i] = _data[i * Columns + j];
        return new SymbolicMatrix(Columns, Rows, t);
    }

    public static SymbolicMatrix Add(SymbolicMatrix a, SymbolicMatrix b)
    {
        var data = new Expr[a._data.Length];
        for (int i = 0; i < data.Length; i++)
            data[i] = Exprs.Add(a._data[i], b._data[i]);
        return new SymbolicMatrix(a.Rows, a.Columns, data);
    }

    public static SymbolicMatrix Multiply(SymbolicMatrix a, SymbolicMatrix b)
    {
        var data = new Expr[a.Rows * b.Columns];
        for (int i = 0; i < a.Rows; i++)
        {
            for (int j = 0; j < b.Columns; j++)
            {
                var terms = new List<Expr>();
                for (int k = 0; k < a.Columns; k++)
                    terms.Add(Exprs.Multiply(a[i, k], b[k, j]));
                data[i * b.Columns + j] = Exprs.Add(terms);
            }
        }
        return new SymbolicMatrix(a.Rows, b.Columns, data);
    }

    public Expr Trace()
    {
        var terms = new List<Expr>();
        for (int i = 0; i < Math.Min(Rows, Columns); i++)
            terms.Add(this[i, i]);
        return Exprs.Add(terms);
    }

    /// <summary>
    /// True when the expression is a provably-zero numeric constant (structural zero test):
    /// a rational/integer zero, the canonical zero, or any numeric constant evaluating to zero.
    /// </summary>
    private static bool IsProvablyZero(Expr e) =>
        e is RationalConstantExpr r && r.Value.IsZero ||
        e is IntegerConstantExpr i && Int.IsZero(i.Value) ||
        Evaluation.ConstantToNum(e) is { } n && NumOps.IsZero(n);

    /// <summary>
    /// Exact division when provable (polynomial division over the union of symbols);
    /// falls back to rational-function division (value-correct, may introduce fractions).
    /// </summary>
    private static Expr ExactDiv(Expr a, Expr b, Symbol[] vars, ExprContext ctx)
    {
        if (b is RationalConstantExpr br && !br.Value.IsZero && a is (RationalConstantExpr or IntegerConstantExpr))
        {
            var av = a is RationalConstantExpr ar ? ar.Value : Rat.From(((IntegerConstantExpr)a).Value);
            return Exprs.Rational(av / br.Value);
        }
        if (Polynomial.TryFromExpr(a, ctx, vars, out var ap, out _) &&
            Polynomial.TryFromExpr(b, ctx, vars, out var bp, out _))
        {
            var (q, rem) = ap.DivRem(bp, MonomialOrder.Lex);
            if (rem.IsZero)
                return q.ToExpr();
        }
        // fallback: rational division (breaks fraction-freeness but stays value-correct)
        return Exprs.Divide(a, b);
    }

    /// <summary>Bareiss fraction-free determinant.</summary>
    public Expr Det(ExprContext ctx)
    {
        if (Rows != Columns)
            throw new InvalidOperationException("Determinant requires a square matrix.");
        int n = Rows;
        if (n == 0)
            return Exprs.One;
        if (n == 1)
            return _data[0];
        var vars = CollectSymbols();
        var a = new Expr[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                a[i, j] = _data[i * n + j];
        Expr prev = Exprs.One;
        int sign = 1;
        for (int k = 0; k < n - 1; k++)
        {
            int pivot = -1;
            for (int i = k; i < n; i++)
            {
                if (!IsProvablyZero(a[i, k]))
                {
                    pivot = i;
                    break;
                }
            }
            if (pivot < 0)
                return Exprs.Zero;
            if (pivot != k)
            {
                for (int j = k; j < n; j++)
                    (a[pivot, j], a[k, j]) = (a[k, j], a[pivot, j]);
                sign = -sign;
            }
            for (int i = k + 1; i < n; i++)
            {
                for (int j = k + 1; j < n; j++)
                {
                    a[i, j] = ExactDiv(
                        Exprs.Subtract(Exprs.Multiply(a[i, j], a[k, k]), Exprs.Multiply(a[i, k], a[k, j])),
                        prev, vars, ctx);
                }
            }
            prev = a[k, k];
        }
        var det = a[n - 1, n - 1];
        return sign < 0 ? Exprs.Negate(det) : det;
    }

    /// <summary>
    /// Generic rank over Q(x): fraction-free (Bareiss) elimination with full pivoting. Each
    /// pivot that is not a provably-zero constant contributes one to the rank; symbolic
    /// pivots count (generic rank), so the result is the rank over the rational-function field.
    /// </summary>
    public int Rank(ExprContext ctx)
    {
        int n = Rows, m = Columns;
        if (n == 0 || m == 0)
            return 0;
        var vars = CollectSymbols();
        var a = new Expr[n, m];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < m; j++)
                a[i, j] = _data[i * m + j];
        Expr prev = Exprs.One;
        int rank = 0;
        int r = 0, c = 0;
        while (r < n && c < m)
        {
            int pr = -1, pc = -1;
            for (int i = r; i < n && pr < 0; i++)
                for (int j = c; j < m; j++)
                    if (!IsProvablyZero(a[i, j]))
                    {
                        pr = i;
                        pc = j;
                        break;
                    }
            if (pr < 0)
                break;
            if (pr != r)
                for (int j = 0; j < m; j++)
                    (a[r, j], a[pr, j]) = (a[pr, j], a[r, j]);
            if (pc != c)
                for (int i = 0; i < n; i++)
                    (a[i, c], a[i, pc]) = (a[i, pc], a[i, c]);
            rank++;
            var pivot = a[r, c];
            for (int i = r + 1; i < n; i++)
            {
                for (int j = c + 1; j < m; j++)
                {
                    a[i, j] = ExactDiv(
                        Exprs.Subtract(Exprs.Multiply(a[i, j], pivot), Exprs.Multiply(a[i, c], a[r, j])),
                        prev, vars, ctx);
                }
                a[i, c] = Exprs.Zero;
            }
            prev = pivot;
            r++;
            c++;
        }
        return rank;
    }

    /// <summary>
    /// Inverse via the adjugate over the determinant, carrying the det ≠ 0 condition.
    /// Returns a null <see cref="MatrixSolveResult.Matrix"/> with a note when singular.
    /// </summary>
    public MatrixSolveResult InverseWithConditions(ExprContext ctx)
    {
        var det = Det(ctx);
        if (IsProvablyZero(det))
            return new MatrixSolveResult(null, null, AssumptionSet.Empty, "matrix is singular");
        var conditions = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(det, SymbolPredicate.NonZero));
        var adj = new Expr[Rows, Columns];
        for (int i = 0; i < Rows; i++)
        {
            for (int j = 0; j < Columns; j++)
            {
                var minor = Minor(i, j);
                var cofactor = minor.Det(ctx);
                adj[j, i] = ((i + j) % 2 == 0 ? cofactor : Exprs.Negate(cofactor));
            }
        }
        var data = new Expr[Rows * Columns];
        for (int i = 0; i < Rows; i++)
            for (int j = 0; j < Columns; j++)
                data[i * Columns + j] = Exprs.Divide(adj[i, j], det);
        return new MatrixSolveResult(new SymbolicMatrix(Rows, Columns, data), null, conditions, null);
    }

    /// <summary>Inverse via the adjugate over the determinant (throws when singular).</summary>
    public SymbolicMatrix Inverse(ExprContext ctx)
    {
        var result = InverseWithConditions(ctx);
        return result.Matrix ?? throw new InvalidOperationException("Matrix is singular (det = 0).");
    }

    private SymbolicMatrix Minor(int skipRow, int skipCol)
    {
        var rows = new List<Expr[]>();
        for (int i = 0; i < Rows; i++)
        {
            if (i == skipRow) continue;
            var row = new List<Expr>();
            for (int j = 0; j < Columns; j++)
                if (j != skipCol) row.Add(this[i, j]);
            rows.Add(row.ToArray());
        }
        return From(rows.ToArray());
    }

    /// <summary>Fraction-free linear solve A·x = b (augmented Bareiss elimination).</summary>
    public static Expr[] Solve(SymbolicMatrix a, Expr[] b, ExprContext ctx)
    {
        var result = a.SolveWithConditions(b, ctx);
        return result.Vector ?? throw new InvalidOperationException("System is singular or underdetermined.");
    }

    /// <summary>
    /// Fraction-free linear solve A·x = b (augmented Bareiss elimination), carrying the
    /// det ≠ 0 condition. Returns a null <see cref="MatrixSolveResult.Vector"/> with a note
    /// when singular.
    /// </summary>
    public MatrixSolveResult SolveWithConditions(Expr[] b, ExprContext ctx)
    {
        var det = Det(ctx);
        if (IsProvablyZero(det))
            return new MatrixSolveResult(null, null, AssumptionSet.Empty, "matrix is singular");
        var conditions = AssumptionSet.Empty.Add(new ExpressionPropertyAssumption(det, SymbolPredicate.NonZero));
        return new MatrixSolveResult(null, SolveCore(this, b, ctx), conditions, null);
    }

    /// <summary>Static convenience for a structured linear solve A·x = b.</summary>
    public static MatrixSolveResult LinearSystem(SymbolicMatrix a, Expr[] b, ExprContext ctx) =>
        a.SolveWithConditions(b, ctx);

    private static Expr[] SolveCore(SymbolicMatrix a, Expr[] b, ExprContext ctx)
    {
        int n = a.Rows;
        var vars = a.CollectSymbols();
        var m = new Expr[n, n + 1];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++)
                m[i, j] = a[i, j];
            m[i, n] = b[i];
        }
        Expr prev = Exprs.One;
        for (int k = 0; k < n; k++)
        {
            int pivot = -1;
            for (int i = k; i < n; i++)
            {
                if (!IsProvablyZero(m[i, k]))
                {
                    pivot = i;
                    break;
                }
            }
            if (pivot < 0)
                throw new InvalidOperationException("System is singular or underdetermined.");
            if (pivot != k)
                for (int j = k; j <= n; j++)
                    (m[pivot, j], m[k, j]) = (m[k, j], m[pivot, j]);
            // Forward Bareiss elimination: rows BELOW the pivot only. The exact-divisibility
            // guarantee of Bareiss holds for the trailing submatrix; eliminating rows above
            // the pivot with the previous-pivot division is not exact and produces wrong
            // solutions (e.g. [[x,1],[0,y]]·v = [0,1] yielded v1 = -1/x instead of -1/(x·y)).
            for (int i = k + 1; i < n; i++)
            {
                for (int j = k + 1; j <= n; j++)
                {
                    m[i, j] = ExactDiv(
                        Exprs.Subtract(Exprs.Multiply(m[i, j], m[k, k]), Exprs.Multiply(m[i, k], m[k, j])),
                        prev, vars, ctx);
                }
                m[i, k] = Exprs.Zero;
            }
            prev = m[k, k];
        }
        // back-substitution over the upper-triangular system
        var x = new Expr[n];
        for (int i = n - 1; i >= 0; i--)
        {
            Expr acc = m[i, n];
            for (int j = i + 1; j < n; j++)
                acc = Exprs.Subtract(acc, Exprs.Multiply(m[i, j], x[j]));
            x[i] = Exprs.Divide(acc, m[i, i]);
        }
        return x;
    }

    private Symbol[] CollectSymbols()
    {
        var set = new SortedSet<Symbol>();
        void Walk(Expr e)
        {
            switch (e)
            {
                case SymbolExpr s: set.Add(s.Symbol); break;
                case AddExpr a: foreach (var t in a.Terms) Walk(t); break;
                case MultiplyExpr m: foreach (var f in m.Factors) Walk(f); break;
                case PowerExpr p: Walk(p.Base); Walk(p.Exponent); break;
                case FunctionExpr f: foreach (var arg in f.Arguments) Walk(arg); break;
            }
        }
        foreach (var e in _data) Walk(e);
        return set.ToArray();
    }

    public bool Equals(SymbolicMatrix? other) =>
        other is not null && Rows == other.Rows && Columns == other.Columns && _data.SequenceEqual(other._data);

    public override bool Equals(object? obj) => obj is SymbolicMatrix m && Equals(m);

    public override int GetHashCode()
    {
        int h = HashCode.Combine(Rows, Columns);
        foreach (var e in _data) h = HashCode.Combine(h, e.GetHashCode());
        return h;
    }

    /// <summary>Jacobian of a vector of expressions w.r.t. variables.</summary>
    public static SymbolicMatrix Jacobian(Expr[] functions, Symbol[] variables, ExprContext ctx)
    {
        var rows = new Expr[functions.Length][];
        for (int i = 0; i < functions.Length; i++)
        {
            rows[i] = new Expr[variables.Length];
            for (int j = 0; j < variables.Length; j++)
                rows[i][j] = Calculus.Diff(functions[i], variables[j], ctx);
        }
        return From(rows);
    }

    public static SymbolicMatrix Hessian(Expr f, Symbol[] variables, ExprContext ctx)
    {
        var rows = new Expr[variables.Length][];
        for (int i = 0; i < variables.Length; i++)
        {
            rows[i] = new Expr[variables.Length];
            for (int j = 0; j < variables.Length; j++)
                rows[i][j] = Calculus.Diff(Calculus.Diff(f, variables[i], ctx), variables[j], ctx);
        }
        return From(rows);
    }

    public static SymbolicMatrix Gradient(Expr f, Symbol[] variables, ExprContext ctx)
    {
        var data = new Expr[1][];
        data[0] = variables.Select(v => Calculus.Diff(f, v, ctx)).ToArray();
        return From(data);
    }
}

/// <summary>
/// Result of a structured matrix operation: the computed matrix (inverse) or vector (solution),
/// the condition set under which the result is valid (e.g. det ≠ 0), and an optional note.
/// </summary>
public sealed record MatrixSolveResult(SymbolicMatrix? Matrix, Expr[]? Vector, AssumptionSet Conditions, string? Note);
