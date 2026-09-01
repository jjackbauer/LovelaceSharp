namespace Lovelace.Arrays;

/// <summary>
/// N-dimensional numeric algorithms (construction, reductions, linear algebra)
/// parameterized by an <see cref="IField{T}"/> so the element type stays abstract.
/// </summary>
public static class ArrayMath
{
    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    public static NdArray<T> Zeros<T>(IField<T> f, IReadOnlyList<long> shape) => NdArray<T>.Fill(shape, f.Zero);

    public static NdArray<T> Ones<T>(IField<T> f, IReadOnlyList<long> shape) => NdArray<T>.Fill(shape, f.One);

    public static NdArray<T> Eye<T>(IField<T> f, long rows, long cols)
    {
        if (rows < 1 || cols < 1)
            throw new ArgumentException("eye() dimensions must be positive.");

        long[] shape = [rows, cols];
        long total = rows * cols;
        var data = Enumerable.Repeat(f.Zero, (int)total).ToList();
        long diag = Math.Min(rows, cols);
        for (long i = 0; i < diag; i++)
            data[(int)(i * cols + i)] = f.One;
        return new NdArray<T>(shape, data);
    }

    // ------------------------------------------------------------------
    // Reductions (all elements → scalar)
    // ------------------------------------------------------------------

    public static T Sum<T>(IField<T> f, NdArray<T> a) => ReduceAll(f, a, f.Zero, f.Add);

    public static T Prod<T>(IField<T> f, NdArray<T> a) => ReduceAll(f, a, f.One, f.Multiply);

    public static T Min<T>(IField<T> f, NdArray<T> a) => MinMaxAll(f, a, wantMin: true);

    public static T Max<T>(IField<T> f, NdArray<T> a) => MinMaxAll(f, a, wantMin: false);

    public static T Mean<T>(IField<T> f, NdArray<T> a) => f.Divide(Sum(f, a), f.FromLong(a.Numel));

    public static T Norm<T>(IField<T> f, NdArray<T> a) => f.Sqrt(SumSquares(f, a));

    // ------------------------------------------------------------------
    // Reductions (along one axis → rank-1-less array)
    // ------------------------------------------------------------------

    public static NdArray<T> Sum<T>(IField<T> f, NdArray<T> a, long axis) => ReduceAxis(f, a, axis, f.Zero, f.Add);

    public static NdArray<T> Prod<T>(IField<T> f, NdArray<T> a, long axis) => ReduceAxis(f, a, axis, f.One, f.Multiply);

    public static NdArray<T> Min<T>(IField<T> f, NdArray<T> a, long axis) => MinMaxAxis(f, a, axis, wantMin: true);

    public static NdArray<T> Max<T>(IField<T> f, NdArray<T> a, long axis) => MinMaxAxis(f, a, axis, wantMin: false);

    public static NdArray<T> Mean<T>(IField<T> f, NdArray<T> a, long axis)
    {
        int ax = CheckAxis(axis, a.Rank);
        var s = Sum(f, a, axis);
        return Map(s, x => f.Divide(x, f.FromLong(a.Shape[ax])));
    }

    public static NdArray<T> Norm<T>(IField<T> f, NdArray<T> a, long axis)
    {
        var s = SumSquaresAxis(f, a, axis);
        return Map(s, f.Sqrt);
    }

    // ------------------------------------------------------------------
    // Linear algebra
    // ------------------------------------------------------------------

    /// <summary>Inner product of two rank-1 vectors of equal length.</summary>
    public static T Dot<T>(IField<T> f, NdArray<T> a, NdArray<T> b)
    {
        RequireRank(a, 1, "dot() first operand");
        RequireRank(b, 1, "dot() second operand");
        if (a.Shape[0] != b.Shape[0])
            throw new ArgumentException($"dot() operands must have the same length ({a.Shape[0]} vs {b.Shape[0]}).");

        T acc = f.Zero;
        for (long i = 0; i < a.Shape[0]; i++)
            acc = f.Add(acc, f.Multiply(a.Data[(int)i], b.Data[(int)i]));
        return acc;
    }

    /// <summary>3-D cross product of two rank-1 vectors of length 3.</summary>
    public static NdArray<T> Cross<T>(IField<T> f, NdArray<T> a, NdArray<T> b)
    {
        RequireRank(a, 1, "cross() first operand");
        RequireRank(b, 1, "cross() second operand");
        if (a.Shape[0] != 3 || b.Shape[0] != 3)
            throw new ArgumentException("cross() operands must be vectors of length 3.");

        var x0 = a.Data[0]; var x1 = a.Data[1]; var x2 = a.Data[2];
        var y0 = b.Data[0]; var y1 = b.Data[1]; var y2 = b.Data[2];

        var r0 = f.Subtract(f.Multiply(x1, y2), f.Multiply(x2, y1));
        var r1 = f.Subtract(f.Multiply(x2, y0), f.Multiply(x0, y2));
        var r2 = f.Subtract(f.Multiply(x0, y1), f.Multiply(x1, y0));
        return new NdArray<T>(new long[] { 3 }, new[] { r0, r1, r2 });
    }

    /// <summary>
    /// Matrix / batched-matrix product. Supports rank-2·rank-2, rank-2·rank-1,
    /// rank-1·rank-2, and batched rank &gt;= 2 · rank &gt;= 2 (equal leading dimensions).
    /// Rank-1·rank-1 is <see cref="Dot{T}"/>.
    /// </summary>
    public static NdArray<T> MatMul<T>(IField<T> f, NdArray<T> a, NdArray<T> b)
    {
        int ra = a.Rank;
        int rb = b.Rank;

        // vector · matrix → (n)
        if (ra == 1 && rb == 2)
        {
            long k = a.Shape[0];
            if (k != b.Shape[0])
                throw new ArgumentException($"matmul() inner dimensions must match ({k} vs {b.Shape[0]}).");
            long n = b.Shape[1];
            var res = new List<T>((int)n);
            for (long j = 0; j < n; j++)
            {
                T acc = f.Zero;
                for (long t = 0; t < k; t++)
                    acc = f.Add(acc, f.Multiply(a.Data[(int)t], b.Data[(int)(t * n + j)]));
                res.Add(acc);
            }
            return new NdArray<T>(new[] { n }, res);
        }

        // matrix · vector → (m)
        if (ra == 2 && rb == 1)
        {
            long m = a.Shape[0];
            long k = a.Shape[1];
            if (k != b.Shape[0])
                throw new ArgumentException($"matmul() inner dimensions must match ({k} vs {b.Shape[0]}).");
            var res = new List<T>((int)m);
            for (long i = 0; i < m; i++)
            {
                T acc = f.Zero;
                for (long t = 0; t < k; t++)
                    acc = f.Add(acc, f.Multiply(a.Data[(int)(i * k + t)], b.Data[(int)t]));
                res.Add(acc);
            }
            return new NdArray<T>(new[] { m }, res);
        }

        if (ra < 2 || rb < 2)
            throw new ArgumentException("matmul() operands must be rank >= 1; use dot() for two vectors.");

        long inner = a.Shape[ra - 1];
        if (inner != b.Shape[rb - 2])
            throw new ArgumentException($"matmul() inner dimensions must match ({inner} vs {b.Shape[rb - 2]}).");

        int B = ra - 2;
        if (rb - 2 != B)
            throw new ArgumentException("matmul() operands must have the same number of leading (batch) dimensions.");
        for (int d = 0; d < B; d++)
        {
            if (a.Shape[d] != b.Shape[d])
                throw new ArgumentException("matmul() leading (batch) dimensions must match.");
        }

        long mDim = a.Shape[ra - 2];
        long nDim = b.Shape[rb - 1];
        long[] outShape = a.Shape.Take(B).Concat(new[] { mDim, nDim }).ToArray();
        long outNumel = NdArray<T>.Product(outShape);
        long[] outStrides = NdArray<T>.ComputeStrides(outShape);

        var outData = new List<T>((int)outNumel);
        for (long lin = 0; lin < outNumel; lin++)
        {
            var c = new long[B + 2];
            for (int d = 0; d < B + 2; d++)
                c[d] = (lin / outStrides[d + 1]) % outShape[d];

            long i = c[B];
            long j = c[B + 1];

            long aBase = 0;
            for (int d = 0; d < B; d++)
                aBase += c[d] * a.Strides[d + 1];
            aBase += i * a.Strides[ra - 1];

            long bBase = 0;
            for (int d = 0; d < B; d++)
                bBase += c[d] * b.Strides[d + 1];
            bBase += j * b.Strides[rb]; // last-dim stride is 1

            T acc = f.Zero;
            for (long t = 0; t < inner; t++)
            {
                long ai = aBase + t * a.Strides[ra]; // a.Strides[ra] == 1
                long bi = bBase + t * b.Strides[rb - 1];
                acc = f.Add(acc, f.Multiply(a.Data[(int)ai], b.Data[(int)bi]));
            }
            outData.Add(acc);
        }

        return new NdArray<T>(outShape, outData);
    }

    /// <summary>Determinant of a square rank-2 matrix via exact Gaussian elimination.</summary>
    public static T Det<T>(IField<T> f, NdArray<T> m)
    {
        RequireSquare(m, "det()");
        int n = (int)m.Shape[0];

        var a = new T[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                a[i, j] = m.Data[i * n + j];

        T det = f.One;
        for (int k = 0; k < n; k++)
        {
            int pivot = k;
            while (pivot < n && f.IsZero(a[pivot, k]))
                pivot++;
            if (pivot == n)
                return f.Zero;

            if (pivot != k)
            {
                for (int j = k; j < n; j++)
                    (a[k, j], a[pivot, j]) = (a[pivot, j], a[k, j]);
                det = f.Negate(det);
            }

            det = f.Multiply(det, a[k, k]);
            for (int i = k + 1; i < n; i++)
            {
                T factor = f.Divide(a[i, k], a[k, k]);
                for (int j = k + 1; j < n; j++)
                    a[i, j] = f.Subtract(a[i, j], f.Multiply(factor, a[k, j]));
            }
        }
        return det;
    }

    /// <summary>Inverse of a square non-singular rank-2 matrix via Gauss–Jordan elimination.</summary>
    public static NdArray<T> Inverse<T>(IField<T> f, NdArray<T> m)
    {
        RequireSquare(m, "inv()");
        int n = (int)m.Shape[0];

        var a = new T[n, 2 * n];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++)
                a[i, j] = m.Data[i * n + j];
            for (int j = 0; j < n; j++)
                a[i, n + j] = i == j ? f.One : f.Zero;
        }

        for (int k = 0; k < n; k++)
        {
            int pivot = k;
            while (pivot < n && f.IsZero(a[pivot, k]))
                pivot++;
            if (pivot == n)
                throw new InvalidOperationException("Matrix is singular and cannot be inverted.");

            if (pivot != k)
            {
                for (int j = 0; j < 2 * n; j++)
                    (a[k, j], a[pivot, j]) = (a[pivot, j], a[k, j]);
            }

            T divisor = a[k, k];
            for (int j = 0; j < 2 * n; j++)
                a[k, j] = f.Divide(a[k, j], divisor);

            for (int i = 0; i < n; i++)
            {
                if (i == k)
                    continue;
                T factor = a[i, k];
                if (f.IsZero(factor))
                    continue;
                for (int j = 0; j < 2 * n; j++)
                    a[i, j] = f.Subtract(a[i, j], f.Multiply(factor, a[k, j]));
            }
        }

        long[] shape = [n, n];
        var data = new List<T>(n * n);
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                data.Add(a[i, n + j]);

        return new NdArray<T>(shape, data);
    }

    /// <summary>Sum of the main diagonal of a square rank-2 matrix.</summary>
    public static T Trace<T>(IField<T> f, NdArray<T> m)
    {
        RequireSquare(m, "trace()");
        int n = (int)m.Shape[0];
        T acc = f.Zero;
        for (int i = 0; i < n; i++)
            acc = f.Add(acc, m.Data[i * n + i]);
        return acc;
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    private static T ReduceAll<T>(IField<T> f, NdArray<T> a, T seed, Func<T, T, T> folder)
    {
        if (a.Data.Count == 0)
            throw new InvalidOperationException("Cannot reduce an empty array.");
        T acc = seed;
        foreach (var e in a.Data)
            acc = folder(acc, e);
        return acc;
    }

    private static NdArray<T> ReduceAxis<T>(IField<T> f, NdArray<T> a, long axis, T seed, Func<T, T, T> folder)
    {
        int r = a.Rank;
        int ax = CheckAxis(axis, r);

        long[] outShape = a.Shape.Where((_, i) => i != ax).ToArray();
        long[] outStrides = NdArray<T>.ComputeStrides(outShape);
        long outNumel = outShape.Length == 0 ? 1 : NdArray<T>.Product(outShape);

        var result = new List<T>((int)outNumel);
        for (long o = 0; o < outNumel; o++)
        {
            var coords = new long[outShape.Length];
            for (int j = 0; j < outShape.Length; j++)
                coords[j] = (o / outStrides[j + 1]) % outShape[j];

            long baseOffset = 0;
            for (int j = 0; j < outShape.Length; j++)
            {
                int srcDim = j < ax ? j : j + 1;
                baseOffset += coords[j] * a.Strides[srcDim + 1];
            }

            T acc = seed;
            for (long t = 0; t < a.Shape[ax]; t++)
                acc = folder(acc, a.Data[(int)(baseOffset + t * a.Strides[ax + 1])]);

            result.Add(acc);
        }

        return new NdArray<T>(outShape.Length == 0 ? new long[] { 1 } : outShape, result);
    }

    private static T MinMaxAll<T>(IField<T> f, NdArray<T> a, bool wantMin)
    {
        if (a.Data.Count == 0)
            throw new InvalidOperationException("Cannot reduce an empty array.");

        T best = a.Data[0];
        for (int i = 1; i < a.Data.Count; i++)
        {
            int c = f.Compare(a.Data[i], best);
            if (wantMin ? c < 0 : c > 0)
                best = a.Data[i];
        }
        return best;
    }

    private static NdArray<T> MinMaxAxis<T>(IField<T> f, NdArray<T> a, long axis, bool wantMin)
    {
        int r = a.Rank;
        int ax = CheckAxis(axis, r);

        long[] outShape = a.Shape.Where((_, i) => i != ax).ToArray();
        long[] outStrides = NdArray<T>.ComputeStrides(outShape);
        long outNumel = outShape.Length == 0 ? 1 : NdArray<T>.Product(outShape);

        var result = new List<T>((int)outNumel);
        for (long o = 0; o < outNumel; o++)
        {
            var coords = new long[outShape.Length];
            for (int j = 0; j < outShape.Length; j++)
                coords[j] = (o / outStrides[j + 1]) % outShape[j];

            long baseOffset = 0;
            for (int j = 0; j < outShape.Length; j++)
            {
                int srcDim = j < ax ? j : j + 1;
                baseOffset += coords[j] * a.Strides[srcDim + 1];
            }

            T best = a.Data[(int)baseOffset];
            for (long t = 1; t < a.Shape[ax]; t++)
            {
                var e = a.Data[(int)(baseOffset + t * a.Strides[ax + 1])];
                int c = f.Compare(e, best);
                if (wantMin ? c < 0 : c > 0)
                    best = e;
            }
            result.Add(best);
        }

        return new NdArray<T>(outShape.Length == 0 ? new long[] { 1 } : outShape, result);
    }

    private static T SumSquares<T>(IField<T> f, NdArray<T> a) =>
        ReduceAll(f, a, f.Zero, (acc, e) => f.Add(acc, f.Multiply(e, e)));

    private static NdArray<T> SumSquaresAxis<T>(IField<T> f, NdArray<T> a, long axis) =>
        ReduceAxis(f, a, axis, f.Zero, (acc, e) => f.Add(acc, f.Multiply(e, e)));

    private static NdArray<T> Map<T>(NdArray<T> a, Func<T, T> fn) =>
        new NdArray<T>(a.Shape, a.Data.Select(fn).ToList());

    private static void RequireRank<T>(NdArray<T> a, int rank, string what)
    {
        if (a.Rank != rank)
            throw new ArgumentException($"{what} must be a rank-{rank} array, but got rank {a.Rank}.");
    }

    private static void RequireSquare<T>(NdArray<T> m, string what)
    {
        if (m.Rank != 2)
            throw new ArgumentException($"{what} requires a rank-2 matrix, but got rank {m.Rank}.");
        if (m.Shape[0] != m.Shape[1])
            throw new ArgumentException($"{what} requires a square matrix, but got shape [{m.Shape[0]}, {m.Shape[1]}].");
    }

    private static int CheckAxis(long axis, int rank)
    {
        if (axis < 0 || axis >= rank)
            throw new ArgumentOutOfRangeException(nameof(axis), $"Axis {axis} is out of range for rank {rank}.");
        return (int)axis;
    }
}
