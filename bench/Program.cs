using System.Diagnostics;
using Lovelace.Natural;
using Lovelace.Integer;
using Lovelace.Real;

// LovelaceSharp micro-benchmark harness (scratch, not part of the solution).
//
// Usage: bench <op> <digits> [reps]
//   op: add | sub | mul | div | pow | factorial | tostring | parse | mulpar
//   digits: size of operand A (operand B sized to make the op meaningful)
//   reps:   number of timed repetitions (default 20)
//
// Output: one line "RESULT <op> <digits> <mean_ms> <median_ms>" to stdout for
//         easy machine parsing, plus a human-readable summary.

static string RandomDigits(int n, int seed)
{
    var rng = new Random(seed);
    var sb = new System.Text.StringBuilder(n);
    // first digit nonzero so the number has exactly n digits
    sb.Append((char)('1' + rng.Next(9)));
    for (int i = 1; i < n; i++)
        sb.Append((char)('0' + rng.Next(10)));
    return sb.ToString();
}

if (args.Length < 2)
{
    Console.Error.WriteLine("usage: bench <op> <digits> [reps]");
    return 1;
}

string op = args[0];
int digits = int.Parse(args[1]);
int reps = args.Length >= 3 ? int.Parse(args[2]) : 20;

// Randomized cross-check against System.Numerics.BigInteger (exact reference)
// for add / sub / mul / div. Exits 0 on success, 1 on any mismatch.
if (op == "check")
{
    var rng = new Random(20240607);
    int cases = reps;
    for (int t = 0; t < cases; t++)
    {
        int na = rng.Next(1, digits + 1);
        int nb = rng.Next(1, digits + 1);
        var sa = RandomDigits(na, rng.Next());
        var sb = RandomDigits(nb, rng.Next());
        var A = new Natural(sa);
        var B = new Natural(sb);
        var BA = System.Numerics.BigInteger.Parse(sa);
        var BB = System.Numerics.BigInteger.Parse(sb);

        if ((A + B).ToString() != (BA + BB).ToString()) { Console.Error.WriteLine($"ADD mismatch {sa} {sb}"); return 1; }
        if ((A * B).ToString() != (BA * BB).ToString()) { Console.Error.WriteLine($"MUL mismatch {sa} {sb}"); return 1; }
        if (A >= B)
        {
            if ((A - B).ToString() != (BA - BB).ToString()) { Console.Error.WriteLine($"SUB mismatch {sa} {sb}"); return 1; }
        }
        var q = Natural.DivRem(A, B, out var rem);
        var (bq, brm) = System.Numerics.BigInteger.DivRem(BA, BB);
        if (q.ToString() != bq.ToString()) { Console.Error.WriteLine($"DIV q mismatch {sa} / {sb}"); return 1; }
        if (rem.ToString() != brm.ToString()) { Console.Error.WriteLine($"DIV rem mismatch {sa} / {sb}"); return 1; }
    }
    Console.WriteLine($"CHECK ok {cases} cases up to {digits} digits");
    return 0;
}

// diagmul: run one large division and dump the multiply-path diagnostic counters.
if (op == "diagmul")
{
    Natural.ResetMultiplyDiag();
    var da = new Natural(RandomDigits(digits, 1));
    var db = new Natural(RandomDigits(Math.Max(2, digits / 2), 2));
    _ = Natural.DivRem(da, db, out _);
    Console.WriteLine(Natural.MultiplyDiag());
    return 0;
}

// mulcheck: dense multiplication-only cross-check against BigInteger.
if (op == "mulcheck")
{
    var rng = new Random(123456789);
    int fails = 0;
    foreach (int n in Enumerable.Range(200, 401))  // 200..600
    {
        for (int rep = 0; rep < 20; rep++)
        {
            var sa = RandomDigits(n, rng.Next());
            var sb = RandomDigits(n, rng.Next());
            var A = new Natural(sa);
            var B = new Natural(sb);
            var expect = (System.Numerics.BigInteger.Parse(sa) * System.Numerics.BigInteger.Parse(sb)).ToString();
            var got = (A * B).ToString();
            if (got != expect)
            {
                fails++;
                if (fails <= 3) Console.Error.WriteLine($"MULCHECK mismatch at {n} digits: {sa} x {sb}");
            }
        }
    }
    Console.WriteLine(fails == 0 ? "MULCHECK all ok (200..600 x20)" : $"MULCHECK FAILS {fails}");
    return fails == 0 ? 0 : 1;
}

// verify: exact reference checks for Pi and Sqrt (self-contained, no test framework).
if (op == "verify")
{
    const string pi50 = "3.14159265358979323846264338327950288419716939937510";
    Real.MaxComputationDecimalPlaces = 1000;
    Real.DisplayDecimalPlaces = 1000;

    void Fail(string msg) { Console.Error.WriteLine("VERIFY FAIL: " + msg); Environment.Exit(1); }

    if (Real.Pi(1).ToString() != "3.1") Fail("Pi(1) != 3.1");
    if (Real.Pi(10).ToString() != "3.1415926535") Fail("Pi(10) != 3.1415926535");
    if (Real.Pi(50).ToString() != pi50) Fail("Pi(50) mismatch: " + Real.Pi(50));
    if (!Real.Pi(200).ToString().StartsWith(pi50)) Fail("Pi(200) prefix mismatch");
    if (!Real.Pi(1000).ToString().StartsWith(pi50)) Fail("Pi(1000) prefix mismatch");

    if (Real.Sqrt(new Real("4")).ToString() != "2") Fail("Sqrt(4) != 2");
    if (Real.Sqrt(new Real("9")).ToString() != "3") Fail("Sqrt(9) != 3");
    if (!Real.Sqrt(new Real("2")).ToString().StartsWith("1.41421356237309504880168872420969807856967187537694"))
        Fail("Sqrt(2) prefix mismatch");
    if (!Real.Sqrt(new Real("3")).ToString().StartsWith("1.73205080756887729352744634150587236694280525381038"))
        Fail("Sqrt(3) prefix mismatch");
    if (!Real.Sqrt(new Real("5")).ToString().StartsWith("2.23606797749978969640917366873127623544061835961152"))
        Fail("Sqrt(5) prefix mismatch");

    // Exact division (non-periodic): 1 / 2 = 0.5, 3 / 4 = 0.75, 1 / 8 = 0.125.
    if ((Real.One / new Real("2")).ToString() != "0.5") Fail("1/2 != 0.5");
    if ((new Real("3") / new Real("4")).ToString() != "0.75") Fail("3/4 != 0.75");
    if ((new Real("1") / new Real("8")).ToString() != "0.125") Fail("1/8 != 0.125");

    Console.WriteLine("VERIFY ok");
    return 0;
}

var a = new Natural(RandomDigits(digits, 12345));
var b = new Natural(RandomDigits(Math.Max(2, digits / 2), 67890));
var c = new Natural(RandomDigits(Math.Max(1, digits / 4), 11111));

// For division, ensure a > b so we exercise real long division.
Natural bDiv = b < a ? b : new Natural(RandomDigits(digits / 2, 99999));

// pi: lift the precision cap so Pi(digits) is legal at any requested size.
if (op == "pi")
{
    Real.MaxComputationDecimalPlaces = Math.Max(digits, Real.MaxComputationDecimalPlaces);
    Real.DisplayDecimalPlaces = Math.Max(digits, Real.DisplayDecimalPlaces);
}

// Warm up JIT.
_ = a + b;
_ = a * b;
_ = Natural.DivRem(a, bDiv, out _);

long Do(Func<object> fn)
{
    var sw = Stopwatch.StartNew();
    fn();
    sw.Stop();
    return sw.ElapsedTicks;
}

var times = new List<double>(reps);
for (int r = 0; r < reps; r++)
{
    long ticks;
    switch (op)
    {
        case "add":
            ticks = Do(() => a + b);
            break;
        case "sub":
            ticks = Do(() => a - b);
            break;
        case "mul":
            ticks = Do(() => a * b);
            break;
        case "mulpar":
            // force large operands so the parallel path engages
            var big1 = new Natural(RandomDigits(digits, 12345));
            var big2 = new Natural(RandomDigits(digits, 67890));
            ticks = Do(() => big1 * big2);
            break;
        case "div":
            ticks = Do(() => Natural.DivRem(a, bDiv, out _));
            break;
        case "pi":
            ticks = Do(() => Real.Pi(digits));
            break;
        case "pow":
            var smallExp = new Natural((ulong)Math.Min(digits, 2000));
            ticks = Do(() => a.Pow(smallExp));
            break;
        case "factorial":
            var nf = new Natural((ulong)Math.Min(digits, 5000));
            ticks = Do(() => nf.Factorial());
            break;
        case "tostring":
            ticks = Do(() => a.ToString());
            break;
        case "parse":
            string s = a.ToString();
            ticks = Do(() => new Natural(s));
            break;
        default:
            Console.Error.WriteLine($"unknown op: {op}");
            return 2;
    }
    times.Add(ticks * 1000.0 / Stopwatch.Frequency); // milliseconds
}

times.Sort();
double mean = times.Average();
double median = times[times.Count / 2];

Console.WriteLine($"RESULT {op} {digits} mean={mean:F4}ms median={median:F4}ms n={reps}");
return 0;
