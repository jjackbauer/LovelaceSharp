using System.Diagnostics;
using System.Globalization;
using Lovelace.Natural;

if (args.Length != 4)
    throw new ArgumentException("usage: audit OP A_FILE B_FILE REPS");
string op = args[0];
string aText = File.ReadAllText(args[1]).Trim();
string bText = File.ReadAllText(args[2]).Trim();
int reps = int.Parse(args[3], CultureInfo.InvariantCulture);
if (reps < 1) throw new ArgumentOutOfRangeException(nameof(reps));
var a = new Natural(aText);
var b = new Natural(bText);

(Natural Value, Natural? Rem) Evaluate() => op switch
{
    "mul" => (a * b, null),
    "div" => Div(),
    "parse" => (new Natural(aText), null),
    _ => throw new ArgumentException($"unknown operation {op}"),
};

(Natural Value, Natural? Rem) Div()
{
    var quotient = Natural.DivRem(a, b, out var remainder);
    return (quotient, remainder);
}

ulong Digest(string text)
{
    ulong hash = 0xcbf29ce484222325UL;
    foreach (char c in text)
        hash = unchecked((hash ^ (byte)c) * 0x100000001b3UL);
    return hash;
}

for (int i = 0; i < 3; i++) GC.KeepAlive(Evaluate());
var times = new List<double>(reps);
for (int i = 0; i < reps; i++)
{
    var start = Stopwatch.StartNew();
    var result = Evaluate();
    start.Stop();
    times.Add(start.Elapsed.TotalMilliseconds);
    GC.KeepAlive(result);
}
times.Sort();
var final = Evaluate();
string valueText = final.Value.ToString();
string remainderText = final.Rem?.ToString() ?? string.Empty;
double mean = times.Average();
double median = times[times.Count / 2];
Console.WriteLine(string.Format(
    CultureInfo.InvariantCulture,
    "AUDIT C# {0} mean={1:F3}ms median={2:F3}ms len={3} fnv={4:x16} rem_len={5} rem_fnv={6:x16}",
    op, mean, median, valueText.Length, Digest(valueText), remainderText.Length, Digest(remainderText)));
