using System;
using System.Collections.Generic;
using System.Diagnostics;
using Lovelace.Natural;

// A factorial benchmark with a real warm-up, so tiered JIT compilation is not charged to the
// first measurements. The repository bench harness times cold calls, which made 5000! look
// slower than 40000!.
string op = args.Length > 0 ? args[0] : "factorial";
int n = args.Length > 1 ? int.Parse(args[1]) : 20000;
int reps = args.Length > 2 ? int.Parse(args[2]) : 10;

// The repository bench caps factorial at 5000, so it cannot report a comparison at the size it
// is asked for. This harness uses the real size and warms the JIT before timing.
// For rendering the interesting value is an n-digit number, not the number n.
var value = op == "render" ? new Natural(Digits(n)) : new Natural((ulong)n);

static string Digits(int count)
{
    var state = 12345UL;
    var chars = new char[count];
    for (int i = 0; i < count; i++)
    {
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        char c = (char)('0' + (int)(state % 10));
        if (i == 0 && c == '0') c = '1';
        chars[i] = c;
    }
    return new string(chars);
}

Func<object> work = op switch
{
    "factorial" => () => value.Factorial(),
    // A value that exists only as limbs: the rendering is then a real conversion, not a cache hit.
    "render" => () => (value + new Natural(1UL)).ToString(),
    _ => throw new ArgumentException($"unknown op {op}"),
};

for (int i = 0; i < 3; i++) _ = work();

var times = new List<double>();
for (int i = 0; i < reps; i++)
{
    var sw = Stopwatch.StartNew();
    var result = work();
    sw.Stop();
    times.Add(sw.Elapsed.TotalMilliseconds);
    _ = result;
}
times.Sort();
double sum = 0; foreach (var t in times) sum += t;
double mean = sum / times.Count;
double median = times[times.Count / 2];
Console.WriteLine($"RESULT {op} {n} mean={mean:F3}ms median={median:F3}ms n={reps}");
