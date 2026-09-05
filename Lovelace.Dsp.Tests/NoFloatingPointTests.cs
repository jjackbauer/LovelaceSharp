using System.Text.RegularExpressions;

namespace Lovelace.Dsp.Tests;

/// <summary>
/// Mechanical enforcement of the "no IEEE floating point" invariant: <c>Lovelace.Dsp</c> and
/// <c>Lovelace.Complex</c> must contain no <c>double</c>/<c>float</c>/<c>System.Numerics.Complex</c>,
/// and <c>Lovelace.Real</c> must contain no <c>System.Numerics.Complex</c> or IEEE
/// transcendental/random calls (<c>Math.Cos/Sin/Exp/Tan</c>, <c>Random.NextDouble</c>). Relocated
/// from the benchmark's hand-rolled gate so it is a first-class xUnit assertion.
/// </summary>
[Collection("DSP precision")]
public class NoFloatingPointTests
{
    [Fact]
    public void DspAndComplexSources_ContainNoFloatingPoint()
    {
        string root = FindRepoRoot();
        string pattern = @"\bdouble\b|\bfloat\b|System\.Numerics\.Complex";

        var violations = Scan(root, "Lovelace.Dsp", pattern)
            .Concat(Scan(root, "Lovelace.Complex", pattern))
            .ToList();

        Assert.Empty(violations);
    }

    [Fact]
    public void RealSource_ContainsNoIeeeTranscendentalsOrComplex()
    {
        string root = FindRepoRoot();
        string pattern = @"System\.Numerics\.Complex|Math\.(Cos|Sin|Exp|Tan)|Random\.NextDouble";

        var violations = Scan(root, "Lovelace.Real", pattern).ToList();

        Assert.Empty(violations);
    }

    private static IEnumerable<string> Scan(string root, string dir, string pattern)
    {
        var dirPath = Path.Combine(root, dir);
        foreach (var file in Directory.EnumerateFiles(dirPath, "*.cs", SearchOption.AllDirectories))
        {
            if (file.Contains("\\obj\\", StringComparison.OrdinalIgnoreCase)
                || file.Contains("\\bin\\", StringComparison.OrdinalIgnoreCase))
                continue;

            int lineNo = 0;
            foreach (var line in File.ReadLines(file))
            {
                lineNo++;
                if (Regex.IsMatch(line, pattern))
                    yield return $"{file}:{lineNo}: {line.Trim()}";
            }
        }
    }

    private static string FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            if (File.Exists(Path.Combine(dir.FullName, "LovelaceSharp.slnx")))
                return dir.FullName;
            dir = dir.Parent;
        }
        throw new InvalidOperationException("Could not locate the repository root (LovelaceSharp.slnx).");
    }
}
