using System.Diagnostics;
using System.Globalization;
using Lovelace.Abstractions;
using Lovelace.Arrays;
using Lovelace.Suite;
using Rl = Lovelace.Real.Real;

// ---------------------------------------------------------------------------
// arraybench — Stage 0 characterization: cost attribution for the boxed
// NdArray<Value> elementwise/linear-algebra path vs typed references.
// Output: tab-separated RESULT lines (bench, ms/op, alloc bytes/op, n).
// ---------------------------------------------------------------------------



long sink = 0;

(double ms, long alloc) Bench(int reps, Action body)
{
    body(); // warmup
    GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
    long before = GC.GetAllocatedBytesForCurrentThread();
    var sw = Stopwatch.StartNew();
    for (int r = 0; r < reps; r++) body();
    sw.Stop();
    long alloc = (GC.GetAllocatedBytesForCurrentThread() - before) / reps;
    return (sw.Elapsed.TotalMilliseconds / reps, alloc);
}

void Report(string bench, double ms, long alloc, long n, int reps) =>
    Console.WriteLine($"RESULT\t{bench}\t{ms:F4}\t{alloc}\t{n}\t{reps}");

void SetPrecision(long digits)
{
    Rl.MaxComputationDecimalPlaces = digits;
    Rl.DisplayDecimalPlaces = digits;
}

// Operand pools of 16-significant-digit Reals (P16-class operands).
Rl[] poolA = Enumerable.Range(0, 10).Select(i => new Rl("2.34567890123456" + i)).ToArray();
Rl[] poolB = Enumerable.Range(0, 10).Select(i => new Rl("1.23456789012345" + i)).ToArray();

// =========================================================================
// 1. Scalar Real arithmetic (continuity with precbench), P8 and P16.
// =========================================================================
foreach (var p in new[] { (name: "P16", digits: 15L), (name: "P8", digits: 7L) })
{
    SetPrecision(p.digits);
    var a = new Rl("2.345678901234567");
    var b = new Rl("1.234567890123456");
    var (msAdd, alAdd) = Bench(20, () => { Rl x = a + b; sink += x.GetHashCode(); });
    Report("scalar-real-add-" + p.name, msAdd, alAdd, 1, 20);
    var (msMul, alMul) = Bench(20, () => { Rl x = a * b; sink += x.GetHashCode(); });
    Report("scalar-real-mul-" + p.name, msMul, alMul, 1, 20);
}

// =========================================================================
// 2. Elementwise add — attribution across element representations.
// =========================================================================
foreach (var n in new long[] { 1000, 1_000_000 })
{
    double[] da = new double[n], db = new double[n], dst = new double[n];
    Rl[] ra = new Rl[n], rb = new Rl[n], rdst = new Rl[n];
    var daL = new List<double>((int)n);
    var raL = new List<Rl>((int)n);
    var vaL = new List<Value>((int)n);
    for (int i = 0; i < n; i++)
    {
        double dv = i % 997;
        da[i] = dv; db[i] = dv + 1;
        var x = poolA[i % 10]; var y = poolB[i % 10];
        ra[i] = x; rb[i] = y;
        daL.Add(dv); raL.Add(x); vaL.Add(new Value(x));
    }
    var nd_d = new NdArray<double>(new[] { n }, daL);
    var nd_d2 = new NdArray<double>(new[] { n }, daL); // add: a+a
    var nd_r = new NdArray<Rl>(new[] { n }, raL);
    var nd_r2 = new NdArray<Rl>(new[] { n }, raL);
    var nd_v = new NdArray<Value>(new[] { n }, vaL);
    var nd_v2 = new NdArray<Value>(new[] { n }, vaL);

    int reps = n == 1000 ? 20 : 5;

    var (m0, a0) = Bench(reps, () => { for (int i = 0; i < n; i++) dst[i] = da[i] + db[i]; sink += (long)dst[n - 1]; });
    Report("elem-add-double-raw", m0, a0, n, reps);

    var (m1, a1) = Bench(reps, () => { for (int i = 0; i < n; i++) rdst[i] = ra[i] + rb[i]; sink += rdst[n - 1].GetHashCode(); });
    Report("elem-add-real-raw", m1, a1, n, reps);

    var (m2, a2) = Bench(reps, () => { var res = new List<double>((int)n); for (int i = 0; i < n; i++) res.Add(DoubleField.Instance.Add(nd_d.Data[i], nd_d2.Data[i])); sink += res.Count; });
    Report("elem-add-nd-double", m2, a2, n, reps);

    var (m3, a3) = Bench(reps, () => { var res = new List<Rl>((int)n); for (int i = 0; i < n; i++) res.Add(RealField.Instance.Add(nd_r.Data[i], nd_r2.Data[i])); sink += res.Count; });
    Report("elem-add-nd-real", m3, a3, n, reps);

    var (m4, a4) = Bench(reps, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Add, nd_v.Data[i], nd_v2.Data[i])); sink += res.Count; });
    Report("elem-add-nd-value", m4, a4, n, reps);
}

// 10M floor (double only — Real/Value are memory-prohibitive at 10M).
{
    long n = 10_000_000;
    double[] da = new double[n], db = new double[n], dst = new double[n];
    for (int i = 0; i < n; i++) { da[i] = i % 997; db[i] = (i % 997) + 1; }
    var (m0, a0) = Bench(3, () => { for (int i = 0; i < n; i++) dst[i] = da[i] + db[i]; sink += (long)dst[n - 1]; });
    Report("elem-add-double-raw", m0, a0, n, 3);
    var daL = new List<double>(da);
    var nd_d = new NdArray<double>(new[] { n }, daL);
    var (m2, a2) = Bench(3, () => { var res = new List<double>((int)n); for (int i = 0; i < n; i++) res.Add(DoubleField.Instance.Add(nd_d.Data[i], nd_d.Data[i])); sink += res.Count; });
    Report("elem-add-nd-double", m2, a2, n, 3);
}

// =========================================================================
// 3. Elementwise multiply at 1M.
// =========================================================================
{
    long n = 1_000_000;
    var raL = new List<Rl>((int)n); var vaL = new List<Value>((int)n); var daL = new List<double>((int)n);
    for (int i = 0; i < n; i++) { var x = poolA[i % 10]; raL.Add(x); vaL.Add(new Value(x)); daL.Add(i % 997); }
    var nd_r = new NdArray<Rl>(new[] { n }, raL);
    var nd_v = new NdArray<Value>(new[] { n }, vaL);
    var nd_d = new NdArray<double>(new[] { n }, daL);

    var (m0, a0) = Bench(5, () => { var res = new List<double>((int)n); for (int i = 0; i < n; i++) res.Add(DoubleField.Instance.Multiply(nd_d.Data[i], nd_d.Data[i])); sink += res.Count; });
    Report("elem-mul-nd-double", m0, a0, n, 5);
    var (m1, a1) = Bench(5, () => { var res = new List<Rl>((int)n); for (int i = 0; i < n; i++) res.Add(RealField.Instance.Multiply(nd_r.Data[i], nd_r.Data[i])); sink += res.Count; });
    Report("elem-mul-nd-real", m1, a1, n, 5);
    var (m2, a2) = Bench(5, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Multiply, nd_v.Data[i], nd_v.Data[i])); sink += res.Count; });
    Report("elem-mul-nd-value", m2, a2, n, 5);
}

// =========================================================================
// 4. Reduction (sum) at 1M.
// =========================================================================
{
    long n = 1_000_000;
    var raL = new List<Rl>((int)n); var vaL = new List<Value>((int)n); var daL = new List<double>((int)n);
    for (int i = 0; i < n; i++) { var x = poolA[i % 10]; raL.Add(x); vaL.Add(new Value(x)); daL.Add(i % 997); }
    var nd_r = new NdArray<Rl>(new[] { n }, raL);
    var nd_v = new NdArray<Value>(new[] { n }, vaL);
    var nd_d = new NdArray<double>(new[] { n }, daL);

    var (m0, a0) = Bench(5, () => { var s = ArrayMath.Sum(DoubleField.Instance, nd_d); sink += (long)s; });
    Report("sum-nd-double", m0, a0, n, 5);
    var (m1, a1) = Bench(3, () => { var s = ArrayMath.Sum(RealField.Instance, nd_r); sink += s.GetHashCode(); });
    Report("sum-nd-real", m1, a1, n, 3);
    var (m2, a2) = Bench(3, () => { var s = ArrayMath.Sum(ValueField.Instance, nd_v); sink += s.GetHashCode(); });
    Report("sum-nd-value", m2, a2, n, 3);
}

// =========================================================================
// 5. Transpose 1000x1000 (copy cost in NdArray.Transpose).
// =========================================================================
{
    long r = 1000, c = 1000, n = r * c;
    var daL = new List<double>((int)n); var vaL = new List<Value>((int)n); var raL = new List<Rl>((int)n);
    for (int i = 0; i < n; i++) { var x = poolA[i % 10]; daL.Add(i % 997); vaL.Add(new Value(x)); raL.Add(x); }
    var nd_d = new NdArray<double>(new[] { r, c }, daL);
    var nd_v = new NdArray<Value>(new[] { r, c }, vaL);
    var nd_r = new NdArray<Rl>(new[] { r, c }, raL);

    var (m0, a0) = Bench(3, () => { var t = nd_d.Transpose(); sink += t.Data.Count; });
    Report("transpose-1000x1000-nd-double", m0, a0, n, 3);
    var (m1, a1) = Bench(3, () => { var t = nd_r.Transpose(); sink += t.Data.Count; });
    Report("transpose-1000x1000-nd-real", m1, a1, n, 3);
    var (m2, a2) = Bench(2, () => { var t = nd_v.Transpose(); sink += t.Data.Count; });
    Report("transpose-1000x1000-nd-value", m2, a2, n, 2);
}

// =========================================================================
// 6. Matmul (ArrayMath.MatMul) 100x100 all impls; 1000x1000 double only.
// =========================================================================
{
    long s = 100, n = s * s;
    var daL = new List<double>((int)n); var raL = new List<Rl>((int)n); var vaL = new List<Value>((int)n);
    for (int i = 0; i < n; i++) { var x = poolA[i % 10]; daL.Add(i % 997); raL.Add(x); vaL.Add(new Value(x)); }
    var dd = new NdArray<double>(new[] { s, s }, daL);
    var rr = new NdArray<Rl>(new[] { s, s }, raL);
    var vv = new NdArray<Value>(new[] { s, s }, vaL);

    var (m0, a0) = Bench(5, () => { var t = ArrayMath.MatMul(DoubleField.Instance, dd, dd); sink += t.Data.Count; });
    Report("matmul-100x100-nd-double", m0, a0, n, 5);
    var (m1, a1) = Bench(2, () => { var t = ArrayMath.MatMul(RealField.Instance, rr, rr); sink += t.Data.Count; });
    Report("matmul-100x100-nd-real", m1, a1, n, 2);
    var (m2, a2) = Bench(2, () => { var t = ArrayMath.MatMul(ValueField.Instance, vv, vv); sink += t.Data.Count; });
    Report("matmul-100x100-nd-value", m2, a2, n, 2);

    long big = 1000, nb = big * big;
    var bigL = new List<double>((int)nb);
    for (int i = 0; i < nb; i++) bigL.Add(i % 997);
    var bd = new NdArray<double>(new[] { big, big }, bigL);
    var (mb, ab) = Bench(2, () => { var t = ArrayMath.MatMul(DoubleField.Instance, bd, bd); sink += t.Data.Count; });
    Report("matmul-1000x1000-nd-double", mb, ab, nb, 2);
}

// =========================================================================
// 7. Typed path (DenseArray<Value>) — the "after" migration representation,
//    compared head-to-head against the boxed NdArray<Value> reference.
// =========================================================================
foreach (var n in new long[] { 1000, 1_000_000 })
{
    var vaL = new List<Value>((int)n);
    for (int i = 0; i < n; i++) vaL.Add(new Value(poolA[i % 10]));
    var buf = vaL.ToArray();
    var nd_v = new NdArray<Value>(new[] { n }, vaL);
    var da_v = new DenseArray<Value>(new[] { n }, buf, DType.Real, new Precision(0));
    var da_v2 = new DenseArray<Value>(new[] { n }, buf, DType.Real, new Precision(0));

    int reps = n == 1000 ? 20 : 5;

    var (b0, ba0) = Bench(reps, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Add, nd_v.Data[i], nd_v.Data[i])); sink += res.Count; });
    Report("elem-add-boxed-value", b0, ba0, n, reps);

    var (t0, ta0) = Bench(reps, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Add, (Value)da_v.GetElement(i), (Value)da_v2.GetElement(i))); sink += res.Count; });
    Report("elem-add-typed-value", t0, ta0, n, reps);
}

{
    long n = 1_000_000;
    var vaL = new List<Value>((int)n);
    for (int i = 0; i < n; i++) vaL.Add(new Value(poolA[i % 10]));
    var buf = vaL.ToArray();
    var nd_v = new NdArray<Value>(new[] { n }, vaL);
    var da_v = new DenseArray<Value>(new[] { n }, buf, DType.Real, new Precision(0));
    var da_v2 = new DenseArray<Value>(new[] { n }, buf, DType.Real, new Precision(0));

    var (bm, bam) = Bench(5, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Multiply, nd_v.Data[i], nd_v.Data[i])); sink += res.Count; });
    Report("elem-mul-boxed-value", bm, bam, n, 5);

    var (tm, tam) = Bench(5, () => { var res = new List<Value>((int)n); for (int i = 0; i < n; i++) res.Add(NumericOps.Apply(BinaryOp.Multiply, (Value)da_v.GetElement(i), (Value)da_v2.GetElement(i))); sink += res.Count; });
    Report("elem-mul-typed-value", tm, tam, n, 5);

    var (bs, bas) = Bench(3, () => { var s = ArrayMath.Sum(ValueField.Instance, nd_v); sink += s.GetHashCode(); });
    Report("sum-boxed-value", bs, bas, n, 3);

    var (ts, tas) = Bench(3, () => { Value acc = NumericOps.Zero; for (long i = 0; i < n; i++) acc = NumericOps.Apply(BinaryOp.Add, acc, (Value)da_v.GetElement(i)); sink += acc.GetHashCode(); });
    Report("sum-typed-value", ts, tas, n, 3);
}

{
    long r = 1000, c = 1000, n = r * c;
    var vaL = new List<Value>((int)n);
    for (int i = 0; i < n; i++) vaL.Add(new Value(poolA[i % 10]));
    var buf = vaL.ToArray();
    var nd_v = new NdArray<Value>(new[] { r, c }, vaL);
    var da_v = new DenseArray<Value>(new[] { r, c }, buf, DType.Real, new Precision(0));

    var (bt, bat) = Bench(2, () => { var t = nd_v.Transpose(); sink += t.Data.Count; });
    Report("transpose-1000x1000-boxed-value", bt, bat, n, 2);

    var (tt, tat) = Bench(200, () => { var t = da_v.Transpose(null); sink += t.Numel; });
    Report("transpose-1000x1000-typed-value", tt, tat, n, 200);
}

Console.Error.WriteLine("SINK " + sink);

sealed class DoubleField : IField<double>
{
    public static readonly DoubleField Instance = new();
    public double Zero => 0;
    public double One => 1;
    public double FromLong(long v) => v;
    public double Add(double a, double b) => a + b;
    public double Subtract(double a, double b) => a - b;
    public double Multiply(double a, double b) => a * b;
    public double Divide(double a, double b) => a / b;
    public double Negate(double a) => -a;
    public bool IsZero(double a) => a == 0;
    public int Compare(double a, double b) => a.CompareTo(b);
    public double Sqrt(double a) => Math.Sqrt(a);
}

sealed class RealField : IField<Rl>
{
    public static readonly RealField Instance = new();
    public Rl Zero => Rl.Zero;
    public Rl One => Rl.One;
    public Rl FromLong(long v) => new Rl(v.ToString());
    public Rl Add(Rl a, Rl b) => a + b;
    public Rl Subtract(Rl a, Rl b) => a - b;
    public Rl Multiply(Rl a, Rl b) => a * b;
    public Rl Divide(Rl a, Rl b) => a / b;
    public Rl Negate(Rl a) => -a;
    public bool IsZero(Rl a) => Rl.IsZero(a);
    public int Compare(Rl a, Rl b) => a.CompareTo(b);
    public Rl Sqrt(Rl a) => Rl.Sqrt(a);
}