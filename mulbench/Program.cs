using System.Diagnostics;
using System.Numerics;
using Lovelace.Natural;

static string RandomDigits(int n, int seed)
{
    var rng = new Random(seed);
    var sb = new System.Text.StringBuilder(n);
    sb.Append((char)('1' + rng.Next(9)));
    for (int i = 1; i < n; i++) sb.Append((char)('0' + rng.Next(10)));
    return sb.ToString();
}

if (args.Length >= 1 && args[0] == "check")
{
    // mul-only cross-check against BigInteger at the given digit size.
    int digits = int.Parse(args[1]);
    int cases = args.Length > 2 ? int.Parse(args[2]) : 3;
    var rng = new Random(20240607);
    for (int t = 0; t < cases; t++)
    {
        string sa = RandomDigits(rng.Next(digits / 2, digits + 1), rng.Next());
        string sb = RandomDigits(rng.Next(digits / 2, digits + 1), rng.Next());
        var a = new Natural(sa);
        var b = new Natural(sb);
        var expected = (BigInteger.Parse(sa) * BigInteger.Parse(sb)).ToString();
        var actual = (a * b).ToString();
        if (actual != expected)
        {
            Console.Error.WriteLine($"MUL MISMATCH at case {t} (len {sa.Length} x {sb.Length})");
            Environment.Exit(1);
        }
    }
    Console.WriteLine($"MULCHECK ok {cases} cases up to {digits} digits");
    return;
}

int d = int.Parse(args[0]);
int reps = args.Length > 1 ? int.Parse(args[1]) : 3;

var x = new Natural(RandomDigits(d, 12345));
var y = new Natural(RandomDigits(d, 67890));

var warm = x * y;
var sw = Stopwatch.StartNew();
Natural last = warm;
for (int i = 0; i < reps; i++)
    last = x * y;
sw.Stop();

Console.WriteLine($"MULBENCH digits={d} mean={sw.Elapsed.TotalMilliseconds / reps:F3}ms n={reps} checksum={last.ToString().Length}");
