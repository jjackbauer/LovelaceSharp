using System.Diagnostics;
using Lovelace.Real;
using Rl = Lovelace.Real.Real;

if (args.Length > 0 && args[0] == "bench") return Bench();
return Check();

static int Check()
{
    int failures = 0;
    int promotes = 0;
    LReal64.DisplayDecimalPlaces = (int)Rl.DisplayDecimalPlaces; // parity: compare exact values
    LReal128.DisplayDecimalPlaces = (int)Rl.DisplayDecimalPlaces;

    void Check2(string label, Func<string> realFn, Func<string> lrealFn)
    {
        string r = realFn();
        try
        {
            string l = lrealFn();
            if (r != l) { failures++; Console.WriteLine($"MISMATCH {label}: Real={r} LReal64={l}"); }
        }
        catch (LRealPromoteException) { promotes++; }
    }
    void CheckParse(string s) => Check2("parse[" + s + "]", () => Rl.Parse(s).ToString(), () => LReal64.Parse(s).ToString());
    void CheckBin(string a, char op, string b)
    {
        string label = a + " " + op + " " + b;
        Check2(label,
            op switch {
                '+' => () => (Rl.Parse(a) + Rl.Parse(b)).ToString(),
                '-' => () => (Rl.Parse(a) - Rl.Parse(b)).ToString(),
                '*' => () => (Rl.Parse(a) * Rl.Parse(b)).ToString(),
                '/' => () => (Rl.Parse(a) / Rl.Parse(b)).ToString(),
                _ => throw new Exception()
            },
            op switch {
                '+' => () => (LReal64.Parse(a) + LReal64.Parse(b)).ToString(),
                '-' => () => (LReal64.Parse(a) - LReal64.Parse(b)).ToString(),
                '*' => () => (LReal64.Parse(a) * LReal64.Parse(b)).ToString(),
                '/' => () => (LReal64.Parse(a) / LReal64.Parse(b)).ToString(),
                _ => throw new Exception()
            });
    }

    foreach (var s in new[] { "0","1","-1","42","-42","3.14","0.5","0.05","0.005","1.5","10.5","100","100.0","0.1","0.10","1234567890123456789","0.1234567890123456789","-0.0001","5","50","500" })
        CheckParse(s);
    foreach (var s in new[] { "0.(3)","0.1(6)","0.(142857)","1.(3)","0.(9)","1.2(34)","0.0(3)" })
        CheckParse(s);

    string[] nums = { "1","2","0.5","0.1","0.2","1.5","10","100","-3","3.14","0.01","9999999999999999999","0.000000001" };
    foreach (var a in nums) foreach (var b in nums)
    { CheckBin(a,'+',b); CheckBin(a,'-',b); CheckBin(a,'*',b); }

    string[] dens = { "1","2","3","4","5","6","7","8","9","10","11","12","13","16","17","25","97","100" };
    foreach (var n in new[] { "1","2","3","7","10","100" }) foreach (var d in dens) CheckBin(n,'/',d);

    CheckBin("0.(3)",'+',"0.(3)");
    CheckBin("0.(3)",'+',"0.1(6)");
    CheckBin("0.(3)",'*',"2");
    CheckBin("0.(142857)",'+',"0.(142857)");
    CheckBin("1.(3)",'-',"1");

    var cmpVals = new[] { "0","1","-1","0.5","0.1","0.2","0.(3)","0.33333","0.1(6)","1.(3)","3.14","3.14159" };
    for (int i=0;i<cmpVals.Length;i++) for (int j=0;j<cmpVals.Length;j++)
    {
        int r = Rl.Parse(cmpVals[i]).CompareTo(Rl.Parse(cmpVals[j]));
        int l = LReal64.Parse(cmpVals[i]).CompareTo(LReal64.Parse(cmpVals[j]));
        if (Math.Sign(r) != Math.Sign(l)) { failures++; Console.WriteLine($"CMP MISMATCH {cmpVals[i]} vs {cmpVals[j]}"); }
    }

    Console.WriteLine($"CHECK RESULT: failures={failures} promotes={promotes}");
    return failures == 0 ? 0 : 1;
}

static int Bench()
{
    Rl.MaxComputationDecimalPlaces = 15;
    Rl.DisplayDecimalPlaces = 15;
    LReal64.DisplayDecimalPlaces = 15;
    long sink = 0;

    var ra = new Rl("2.345678901234567");
    var rb = new Rl("1.234567890123456");
    var la = LReal64.Parse("2.345678901234567");
    var lb = LReal64.Parse("1.234567890123456");
    double da = 2.345678901234567, db = 1.234567890123456;
    // 8-digit operands so LReal64 multiply fits in 19 significant digits (8×8=16).
    var ram = new Rl("2.3456789");
    var rbm = new Rl("1.2345678");
    var lam = LReal64.Parse("2.3456789");
    var lbm = LReal64.Parse("1.2345678");

    (double ms, long alloc) B(int reps, Action act)
    {
        act();
        GC.Collect(); GC.WaitForPendingFinalizers(); GC.Collect();
        long b0 = GC.GetAllocatedBytesForCurrentThread();
        var sw = Stopwatch.StartNew();
        for (int i = 0; i < reps; i++) act();
        sw.Stop();
        long alloc = (GC.GetAllocatedBytesForCurrentThread() - b0) / reps;
        return (sw.Elapsed.TotalMilliseconds / reps, alloc);
    }
    void Report(string n, double ms, long alloc) => Console.WriteLine($"RESULT	{n}	{ms * 1_000_000:F0}	{alloc}");

    var (s0, sa0) = B(20000, () => { var x = ra + rb; sink += x.Exponent; });
    Report("scalar-add-real-ns", s0, sa0);
    var (s1, sa1) = B(20000, () => { var x = la + lb; sink += x.Exponent; });
    Report("scalar-add-lreal64-ns", s1, sa1);
    var (s2, sa2) = B(20000, () => { var x = da + db; sink += (long)x; });
    Report("scalar-add-double-ns", s2, sa2);

    var (m0, ma0) = B(20000, () => { var x = ram * rbm; sink += x.Exponent; });
    Report("scalar-mul-real-ns", m0, ma0);
    var (m1, ma1) = B(20000, () => { var x = lam * lbm; sink += x.Exponent; });
    Report("scalar-mul-lreal64-ns", m1, ma1);
    var (m2, ma2) = B(20000, () => { var x = da * db; sink += (long)x; });
    Report("scalar-mul-double-ns", m2, ma2);

    int n = 1_000_000;
    Rl[] rp = Enumerable.Range(0, 10).Select(i => new Rl("2.34567890123456" + i)).ToArray();
    LReal64[] lp = rp.Select(x => LReal64.Parse(x.ToString())).ToArray();
    Rl[] rA = new Rl[n], rB = new Rl[n], rD = new Rl[n];
    LReal64[] lA = new LReal64[n], lB = new LReal64[n], lD = new LReal64[n];
    double[] dA = new double[n], dB = new double[n], dD = new double[n];
    for (int i = 0; i < n; i++)
    {
        rA[i] = rp[i % 10]; rB[i] = rp[(i + 1) % 10];
        lA[i] = lp[i % 10]; lB[i] = lp[(i + 1) % 10];
        dA[i] = i % 997; dB[i] = (i % 997) + 1;
    }

    var (e0, ea0) = B(3, () => { for (int i = 0; i < n; i++) rD[i] = rA[i] + rB[i]; sink += rD[n - 1].Exponent; });
    Report("elem-add-real-1M-ns", e0, ea0);
    var (e1, ea1) = B(5, () => { for (int i = 0; i < n; i++) lD[i] = lA[i] + lB[i]; sink += lD[n - 1].Exponent; });
    Report("elem-add-lreal64-1M-ns", e1, ea1);
    var (e2, ea2) = B(5, () => { for (int i = 0; i < n; i++) dD[i] = dA[i] + dB[i]; sink += (long)dD[n - 1]; });
    Report("elem-add-double-1M-ns", e2, ea2);

    // 8-digit pools so LReal64 multiply fits (8×8 = 16 sig digits ≤ 19).
    Rl[] rp8 = Enumerable.Range(0, 10).Select(i => new Rl("2.345678" + i)).ToArray();
    LReal64[] lp8 = rp8.Select(x => LReal64.Parse(x.ToString())).ToArray();
    Rl[] rAm = new Rl[n], rBm = new Rl[n], rDm = new Rl[n];
    LReal64[] lAm = new LReal64[n], lBm = new LReal64[n], lDm = new LReal64[n];
    for (int i = 0; i < n; i++) { rAm[i] = rp8[i % 10]; rBm[i] = rp8[(i + 1) % 10]; lAm[i] = lp8[i % 10]; lBm[i] = lp8[(i + 1) % 10]; }

    var (f0, fa0) = B(3, () => { for (int i = 0; i < n; i++) rDm[i] = rAm[i] * rBm[i]; sink += rDm[n - 1].Exponent; });
    Report("elem-mul-real-1M-ns", f0, fa0);
    var (f1, fa1) = B(5, () => { for (int i = 0; i < n; i++) lDm[i] = lAm[i] * lBm[i]; sink += lDm[n - 1].Exponent; });
    Report("elem-mul-lreal64-1M-ns", f1, fa1);
    var (f2, fa2) = B(5, () => { for (int i = 0; i < n; i++) dD[i] = dA[i] * dB[i]; sink += (long)dD[n - 1]; });
    Report("elem-mul-double-1M-ns", f2, fa2);

    // ===== LReal128 (38-digit) benchmarks — 16-digit operands (multiply needs the wide tier) =====
    var la128 = LReal128.Parse("2.345678901234567");
    var lb128 = LReal128.Parse("1.234567890123456");

    var (t0, ta0) = B(20000, () => { var x = la128 + lb128; sink += x.Exponent; });
    Report("scalar-add-lreal128-ns", t0, ta0);
    var (t1, ta1) = B(20000, () => { var x = la128 * lb128; sink += x.Exponent; });
    Report("scalar-mul-lreal128-ns", t1, ta1);
    var (t2, ta2) = B(20000, () => { var x = ra * rb; sink += x.Exponent; });
    Report("scalar-mul-real-16digit-ns", t2, ta2);

    LReal128[] lp128 = rp.Select(x => LReal128.Parse(x.ToString())).ToArray();
    LReal128[] lA128 = new LReal128[n], lB128 = new LReal128[n], lD128 = new LReal128[n];
    for (int i = 0; i < n; i++) { lA128[i] = lp128[i % 10]; lB128[i] = lp128[(i + 1) % 10]; }

    var (u0, ua0) = B(5, () => { for (int i = 0; i < n; i++) lD128[i] = lA128[i] + lB128[i]; sink += lD128[n - 1].Exponent; });
    Report("elem-add-lreal128-1M-ns", u0, ua0);
    var (u1, ua1) = B(3, () => { for (int i = 0; i < n; i++) lD128[i] = lA128[i] * lB128[i]; sink += lD128[n - 1].Exponent; });
    Report("elem-mul-lreal128-1M-ns", u1, ua1);
    var (u2, ua2) = B(2, () => { for (int i = 0; i < n; i++) rD[i] = rA[i] * rB[i]; sink += rD[n - 1].Exponent; });
    Report("elem-mul-real-16digit-1M-ns", u2, ua2);

        // Dispatch-path cost: TryFromReal + LReal64 add + ToReal (the NumericOps fast path).
    var (d0, da0) = B(20000, () => { LReal64.TryFromReal(ra, out var a64); LReal64.TryFromReal(rb, out var b64); var x = a64 + b64; var rr = x.ToReal(); sink += rr.Exponent; });
    Report("dispatch-add-lreal64-ns", d0, da0);
    var (d1, da1) = B(20000, () => { LReal128.TryFromReal(ra, out var a128); LReal128.TryFromReal(rb, out var b128); var x = a128 + b128; var rr = x.ToReal(); sink += rr.Exponent; });
    Report("dispatch-add-lreal128-ns", d1, da1);

    Console.Error.WriteLine("SINK " + sink);
    return 0;
}
