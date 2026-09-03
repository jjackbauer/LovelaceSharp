using Lovelace.Abstractions;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

/// <summary>
/// Typed (ArrayValue-based) reference reductions and linear algebra — the seed of the
/// Stage-5 reference backend. These iterate <see cref="ArrayValue"/> elements directly
/// through <see cref="NumericOps"/> instead of materializing the boxed
/// <c>NdArray&lt;Value&gt;</c>.
/// </summary>
internal static class TypedArrayOps
{
    // ------------------------------------------------------------------
    // Reduce-all
    // ------------------------------------------------------------------

    public static Value SumAll(ArrayValue a) => ReduceAll(a, NumericOps.Zero, BinaryOp.Add);
    public static Value ProdAll(ArrayValue a) => ReduceAll(a, NumericOps.One, BinaryOp.Multiply);
    public static Value MinAll(ArrayValue a) => MinMaxAll(a, wantMin: true);
    public static Value MaxAll(ArrayValue a) => MinMaxAll(a, wantMin: false);
    public static Value MeanAll(ArrayValue a) => NumericOps.Apply(BinaryOp.Divide, SumAll(a), FromLong(a.Numel));
    public static Value NormAll(ArrayValue a) => new Value(Rl.Sqrt(SumSquares(a).Widen(ValueKind.Real).AsReal()));

    // ------------------------------------------------------------------
    // Reduce-along-axis
    // ------------------------------------------------------------------

    public static ArrayValue SumAxis(ArrayValue a, long axis) => ReduceAxis(a, axis, NumericOps.Zero, BinaryOp.Add);
    public static ArrayValue ProdAxis(ArrayValue a, long axis) => ReduceAxis(a, axis, NumericOps.One, BinaryOp.Multiply);
    public static ArrayValue MinAxis(ArrayValue a, long axis) => MinMaxAxis(a, axis, wantMin: true);
    public static ArrayValue MaxAxis(ArrayValue a, long axis) => MinMaxAxis(a, axis, wantMin: false);

    public static ArrayValue MeanAxis(ArrayValue a, long axis)
    {
        int ax = CheckAxis(axis, a.Rank);
        var sum = SumAxis(a, axis);
        var count = FromLong(a.Shape.ToArray()[ax]);
        return Map(sum, x => NumericOps.Apply(BinaryOp.Divide, x, count));
    }

    public static ArrayValue NormAxis(ArrayValue a, long axis)
    {
        var squares = SumSquaresAxis(a, axis);
        return Map(squares, x => new Value(Rl.Sqrt(x.Widen(ValueKind.Real).AsReal())));
    }

    // ------------------------------------------------------------------
    // Linear algebra
    // ------------------------------------------------------------------

    public static Value Dot(ArrayValue a, ArrayValue b)
    {
        if (a.Rank != 1 || b.Rank != 1)
            throw new ArgumentException("dot() operands must be rank-1 vectors.");
        long n = a.Shape.ToArray()[0];
        if (n != b.Shape.ToArray()[0])
            throw new ArgumentException($"dot() operands must have the same length ({n} vs {b.Shape.ToArray()[0]}).");

        Value acc = NumericOps.Zero;
        for (long i = 0; i < n; i++)
            acc = NumericOps.Apply(BinaryOp.Add, acc,
                NumericOps.Apply(BinaryOp.Multiply, (Value)a.GetElement(i), (Value)b.GetElement(i)));
        return acc;
    }

    public static ArrayValue Cross(ArrayValue a, ArrayValue b)
    {
        if (a.Rank != 1 || b.Rank != 1)
            throw new ArgumentException("cross() operands must be rank-1 vectors.");
        if (a.Shape.ToArray()[0] != 3 || b.Shape.ToArray()[0] != 3)
            throw new ArgumentException("cross() operands must be vectors of length 3.");

        var x0 = (Value)a.GetElement(0); var x1 = (Value)a.GetElement(1); var x2 = (Value)a.GetElement(2);
        var y0 = (Value)b.GetElement(0); var y1 = (Value)b.GetElement(1); var y2 = (Value)b.GetElement(2);

        var r0 = NumericOps.Apply(BinaryOp.Subtract, NumericOps.Apply(BinaryOp.Multiply, x1, y2), NumericOps.Apply(BinaryOp.Multiply, x2, y1));
        var r1 = NumericOps.Apply(BinaryOp.Subtract, NumericOps.Apply(BinaryOp.Multiply, x2, y0), NumericOps.Apply(BinaryOp.Multiply, x0, y2));
        var r2 = NumericOps.Apply(BinaryOp.Subtract, NumericOps.Apply(BinaryOp.Multiply, x0, y1), NumericOps.Apply(BinaryOp.Multiply, x1, y0));

        return TypedArrayAdapter.FromValues(new[] { r0, r1, r2 }, new long[] { 3 });
    }

    public static Value Trace(ArrayValue a)
    {
        if (a.Rank != 2 || a.Shape.ToArray()[0] != a.Shape.ToArray()[1])
            throw new ArgumentException("trace() requires a square matrix.");

        long n = a.Shape.ToArray()[0];
        Value acc = NumericOps.Zero;
        for (long i = 0; i < n; i++)
            acc = NumericOps.Apply(BinaryOp.Add, acc, (Value)a.GetElement(new long[] { i, i }));
        return acc;
    }

    public static ArrayValue MatMul(ArrayValue a, ArrayValue b)
    {
        int ra = a.Rank;
        int rb = b.Rank;
        var aShape = a.Shape.ToArray();
        var bShape = b.Shape.ToArray();

        // vector · matrix → (n)
        if (ra == 1 && rb == 2)
        {
            long k = aShape[0];
            if (k != bShape[0])
                throw new ArgumentException($"matmul() inner dimensions must match ({k} vs {bShape[0]}).");
            long n = bShape[1];
            var res = new List<Value>((int)n);
            for (long j = 0; j < n; j++)
            {
                Value acc = NumericOps.Zero;
                for (long t = 0; t < k; t++)
                    acc = NumericOps.Apply(BinaryOp.Add, acc,
                        NumericOps.Apply(BinaryOp.Multiply, (Value)a.GetElement(t), (Value)b.GetElement(new long[] { t, j })));
                res.Add(acc);
            }
            return TypedArrayAdapter.FromValues(res, new[] { n });
        }

        // matrix · vector → (m)
        if (ra == 2 && rb == 1)
        {
            long m = aShape[0];
            long k = aShape[1];
            if (k != bShape[0])
                throw new ArgumentException($"matmul() inner dimensions must match ({k} vs {bShape[0]}).");
            var res = new List<Value>((int)m);
            for (long i = 0; i < m; i++)
            {
                Value acc = NumericOps.Zero;
                for (long t = 0; t < k; t++)
                    acc = NumericOps.Apply(BinaryOp.Add, acc,
                        NumericOps.Apply(BinaryOp.Multiply, (Value)a.GetElement(new long[] { i, t }), (Value)b.GetElement(t)));
                res.Add(acc);
            }
            return TypedArrayAdapter.FromValues(res, new[] { m });
        }

        if (ra < 2 || rb < 2)
            throw new ArgumentException("matmul() operands must be rank >= 1; use dot() for two vectors.");

        long inner = aShape[ra - 1];
        if (inner != bShape[rb - 2])
            throw new ArgumentException($"matmul() inner dimensions must match ({inner} vs {bShape[rb - 2]}).");

        int B = ra - 2;
        if (rb - 2 != B)
            throw new ArgumentException("matmul() operands must have the same number of leading (batch) dimensions.");
        for (int d = 0; d < B; d++)
            if (aShape[d] != bShape[d])
                throw new ArgumentException("matmul() leading (batch) dimensions must match.");

        long mDim = aShape[ra - 2];
        long nDim = bShape[rb - 1];
        var outShape = aShape.Take(B).Concat(new[] { mDim, nDim }).ToArray();
        long outNumel = Product(outShape);

        var outData = new List<Value>(checked((int)outNumel));
        var c = new long[B + 2];
        var aCoords = new long[ra];
        var bCoords = new long[rb];
        for (long lin = 0; lin < outNumel; lin++)
        {
            long rem = lin;
            for (int d = B + 1; d >= 0; d--)
            {
                c[d] = rem % outShape[d];
                rem /= outShape[d];
            }
            for (int d = 0; d < B; d++) { aCoords[d] = c[d]; bCoords[d] = c[d]; }
            aCoords[ra - 2] = c[B];
            bCoords[rb - 1] = c[B + 1];

            Value acc = NumericOps.Zero;
            for (long t = 0; t < inner; t++)
            {
                aCoords[ra - 1] = t;
                bCoords[rb - 2] = t;
                acc = NumericOps.Apply(BinaryOp.Add, acc,
                    NumericOps.Apply(BinaryOp.Multiply, (Value)a.GetElement(aCoords), (Value)b.GetElement(bCoords)));
            }
            outData.Add(acc);
        }
        return TypedArrayAdapter.FromValues(outData, outShape);
    }

    public static Value Det(ArrayValue a)
    {
        if (a.Rank != 2 || a.Shape.ToArray()[0] != a.Shape.ToArray()[1])
            throw new ArgumentException("det() requires a square matrix.");
        int n = (int)a.Shape.ToArray()[0];

        var m = new Value[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                m[i, j] = (Value)a.GetElement(new long[] { i, j });

        Value det = NumericOps.One;
        for (int k = 0; k < n; k++)
        {
            int pivot = k;
            while (pivot < n && NumericOps.IsZero(m[pivot, k]))
                pivot++;
            if (pivot == n)
                return NumericOps.Zero;

            if (pivot != k)
            {
                for (int j = k; j < n; j++)
                    (m[k, j], m[pivot, j]) = (m[pivot, j], m[k, j]);
                det = NumericOps.Negate(det);
            }

            det = NumericOps.Apply(BinaryOp.Multiply, det, m[k, k]);
            for (int i = k + 1; i < n; i++)
            {
                Value factor = NumericOps.Apply(BinaryOp.Divide, m[i, k], m[k, k]);
                for (int j = k + 1; j < n; j++)
                    m[i, j] = NumericOps.Apply(BinaryOp.Subtract, m[i, j],
                        NumericOps.Apply(BinaryOp.Multiply, factor, m[k, j]));
            }
        }
        return det;
    }

    public static ArrayValue Inverse(ArrayValue a)
    {
        if (a.Rank != 2 || a.Shape.ToArray()[0] != a.Shape.ToArray()[1])
            throw new ArgumentException("inv() requires a square matrix.");
        int n = (int)a.Shape.ToArray()[0];

        var m = new Value[n, 2 * n];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++)
                m[i, j] = (Value)a.GetElement(new long[] { i, j });
            for (int j = 0; j < n; j++)
                m[i, n + j] = i == j ? NumericOps.One : NumericOps.Zero;
        }

        for (int k = 0; k < n; k++)
        {
            int pivot = k;
            while (pivot < n && NumericOps.IsZero(m[pivot, k]))
                pivot++;
            if (pivot == n)
                throw new InvalidOperationException("Matrix is singular and cannot be inverted.");

            if (pivot != k)
                for (int j = 0; j < 2 * n; j++)
                    (m[k, j], m[pivot, j]) = (m[pivot, j], m[k, j]);

            Value divisor = m[k, k];
            for (int j = 0; j < 2 * n; j++)
                m[k, j] = NumericOps.Apply(BinaryOp.Divide, m[k, j], divisor);

            for (int i = 0; i < n; i++)
            {
                if (i == k)
                    continue;
                Value factor = m[i, k];
                if (NumericOps.IsZero(factor))
                    continue;
                for (int j = 0; j < 2 * n; j++)
                    m[i, j] = NumericOps.Apply(BinaryOp.Subtract, m[i, j],
                        NumericOps.Apply(BinaryOp.Multiply, factor, m[k, j]));
            }
        }

        var data = new List<Value>(n * n);
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++)
                data.Add(m[i, n + j]);
        return TypedArrayAdapter.FromValues(data, new long[] { n, n });
    }

    public static ArrayValue Concat(ArrayValue a, ArrayValue b, long axis)
    {
        if (a.Rank != b.Rank)
            throw new ArgumentException($"concat() operands must have the same rank ({a.Rank} vs {b.Rank}).");

        int r = a.Rank;
        int ax = CheckAxis(axis, r);
        var aShape = a.Shape.ToArray();
        var bShape = b.Shape.ToArray();
        for (int i = 0; i < r; i++)
            if (i != ax && aShape[i] != bShape[i])
                throw new ArgumentException($"concat() operands must have the same shape except along axis {ax}.");

        var outShape = (long[])aShape.Clone();
        outShape[ax] = aShape[ax] + bShape[ax];
        long numel = Product(outShape);

        var result = new List<Value>(checked((int)numel));
        var coords = new long[r];
        var bCoords = new long[r];
        for (long lin = 0; lin < numel; lin++)
        {
            long rem = lin;
            for (int i = r - 1; i >= 0; i--)
            {
                coords[i] = rem % outShape[i];
                rem /= outShape[i];
            }

            if (coords[ax] < aShape[ax])
            {
                result.Add((Value)a.GetElement(coords));
            }
            else
            {
                Array.Copy(coords, bCoords, r);
                bCoords[ax] -= aShape[ax];
                result.Add((Value)b.GetElement(bCoords));
            }
        }

        return TypedArrayAdapter.FromValues(result, outShape);
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    private static Value ReduceAll(ArrayValue a, Value seed, BinaryOp op)
    {
        Value acc = seed;
        for (long i = 0; i < a.Numel; i++)
            acc = NumericOps.Apply(op, acc, (Value)a.GetElement(i));
        return acc;
    }

    private static Value SumSquares(ArrayValue a)
    {
        Value acc = NumericOps.Zero;
        for (long i = 0; i < a.Numel; i++)
        {
            var e = (Value)a.GetElement(i);
            acc = NumericOps.Apply(BinaryOp.Add, acc, NumericOps.Apply(BinaryOp.Multiply, e, e));
        }
        return acc;
    }

    private static Value MinMaxAll(ArrayValue a, bool wantMin)
    {
        if (a.Numel == 0)
            throw new InvalidOperationException("Cannot reduce an empty array.");

        Value best = (Value)a.GetElement(0);
        for (long i = 1; i < a.Numel; i++)
        {
            var e = (Value)a.GetElement(i);
            int c = NumericOps.Compare(e, best);
            if (wantMin ? c < 0 : c > 0)
                best = e;
        }
        return best;
    }

    private static ArrayValue ReduceAxis(ArrayValue a, long axis, Value seed, BinaryOp op)
    {
        int ax = CheckAxis(axis, a.Rank);
        var shape = a.Shape.ToArray();
        var outShape = shape.Where((_, i) => i != ax).ToArray();
        long outNumel = outShape.Length == 0 ? 1 : Product(outShape);

        var result = new List<Value>(checked((int)outNumel));
        var srcCoords = new long[a.Rank];
        for (long o = 0; o < outNumel; o++)
        {
            long rem = o;
            for (int j = outShape.Length - 1; j >= 0; j--)
            {
                long c = rem % outShape[j];
                rem /= outShape[j];
                srcCoords[j < ax ? j : j + 1] = c;
            }

            Value acc = seed;
            for (long t = 0; t < shape[ax]; t++)
            {
                srcCoords[ax] = t;
                acc = NumericOps.Apply(op, acc, (Value)a.GetElement(srcCoords));
            }
            result.Add(acc);
        }

        var resShape = outShape.Length == 0 ? new long[] { 1 } : outShape;
        return TypedArrayAdapter.FromValues(result, resShape);
    }

    private static ArrayValue MinMaxAxis(ArrayValue a, long axis, bool wantMin)
    {
        int ax = CheckAxis(axis, a.Rank);
        var shape = a.Shape.ToArray();
        var outShape = shape.Where((_, i) => i != ax).ToArray();
        long outNumel = outShape.Length == 0 ? 1 : Product(outShape);

        var result = new List<Value>(checked((int)outNumel));
        var srcCoords = new long[a.Rank];
        for (long o = 0; o < outNumel; o++)
        {
            long rem = o;
            for (int j = outShape.Length - 1; j >= 0; j--)
            {
                long c = rem % outShape[j];
                rem /= outShape[j];
                srcCoords[j < ax ? j : j + 1] = c;
            }

            srcCoords[ax] = 0;
            Value best = (Value)a.GetElement(srcCoords);
            for (long t = 1; t < shape[ax]; t++)
            {
                srcCoords[ax] = t;
                var e = (Value)a.GetElement(srcCoords);
                int c = NumericOps.Compare(e, best);
                if (wantMin ? c < 0 : c > 0)
                    best = e;
            }
            result.Add(best);
        }

        var resShape = outShape.Length == 0 ? new long[] { 1 } : outShape;
        return TypedArrayAdapter.FromValues(result, resShape);
    }

    private static ArrayValue SumSquaresAxis(ArrayValue a, long axis)
    {
        int ax = CheckAxis(axis, a.Rank);
        var shape = a.Shape.ToArray();
        var outShape = shape.Where((_, i) => i != ax).ToArray();
        long outNumel = outShape.Length == 0 ? 1 : Product(outShape);

        var result = new List<Value>(checked((int)outNumel));
        var srcCoords = new long[a.Rank];
        for (long o = 0; o < outNumel; o++)
        {
            long rem = o;
            for (int j = outShape.Length - 1; j >= 0; j--)
            {
                long c = rem % outShape[j];
                rem /= outShape[j];
                srcCoords[j < ax ? j : j + 1] = c;
            }

            Value acc = NumericOps.Zero;
            for (long t = 0; t < shape[ax]; t++)
            {
                srcCoords[ax] = t;
                var e = (Value)a.GetElement(srcCoords);
                acc = NumericOps.Apply(BinaryOp.Add, acc, NumericOps.Apply(BinaryOp.Multiply, e, e));
            }
            result.Add(acc);
        }

        var resShape = outShape.Length == 0 ? new long[] { 1 } : outShape;
        return TypedArrayAdapter.FromValues(result, resShape);
    }

    private static ArrayValue Map(ArrayValue a, Func<Value, Value> fn)
    {
        var result = new List<Value>(checked((int)a.Numel));
        for (long i = 0; i < a.Numel; i++)
            result.Add(fn((Value)a.GetElement(i)));
        return TypedArrayAdapter.FromValues(result, a.Shape.ToArray());
    }

    private static Value FromLong(long n) => new Value(Nat.Parse(n.ToString(), null));

    private static long Product(long[] shape)
    {
        long total = 1;
        foreach (var d in shape)
            total = checked(total * d);
        return total;
    }

    private static int CheckAxis(long axis, int rank)
    {
        if (axis < 0 || axis >= rank)
            throw new ArgumentOutOfRangeException(nameof(axis), $"Axis {axis} is out of range for rank {rank}.");
        return (int)axis;
    }
}
