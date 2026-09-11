using System.ComponentModel;
using System.Diagnostics;
using Lovelace.Symbolics;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// What the environment probe found. <see cref="Usable"/> is true only when a python3 that can
/// actually <c>import sympy</c> was executed: a python3 without sympy counts as ABSENT, because
/// an oracle handed a process that cannot answer would compare nothing and say nothing.
/// </summary>
internal sealed record SympyProbe(bool Usable, string? Version, string Reason);

/// <summary>One python3 invocation: whether it started at all, its exit code and its streams.</summary>
internal sealed record PythonRun(bool Started, int ExitCode, string Stdout, string Stderr, string Failure);

/// <summary>
/// The differential oracle's environment probe and process bridge. SymPy is a TEST oracle only —
/// never a runtime dependency of the kernel.
/// </summary>
internal static class SympyOracle
{
    /// <summary>Set to "1" to turn "the oracle is unavailable" from a skip into a failure.</summary>
    internal const string RequireVariable = "LOVELACE_REQUIRE_SYMPY";

    // The probe is cached: xUnit constructs the attribute of every oracle test before running
    // anything, and the interpreter must be started once, not once per test.
    private static readonly Lazy<SympyProbe> s_probe = new(ProbeOnce);
    private static readonly TimeSpan s_probeTimeout = TimeSpan.FromSeconds(20);
    private static readonly TimeSpan s_evalTimeout = TimeSpan.FromSeconds(60);

    internal static SympyProbe Probe => s_probe.Value;

    internal static bool Required => Environment.GetEnvironmentVariable(RequireVariable) == "1";

    /// <summary>
    /// The only way to obtain an evaluation session. It refuses to hand one out when the probe
    /// failed, so a test body can never "compare nothing and pass": with
    /// <see cref="RequireVariable"/>=1 it throws and the test fails. Without the variable the
    /// attribute skips the test before the body runs; the throw is the backstop that keeps a
    /// mis-declared oracle test from reporting a pass.
    /// </summary>
    internal static SympySession Require()
    {
        SympyProbe probe = Probe;
        if (!probe.Usable)
        {
            string why = Required
                ? $"{RequireVariable}=1 requires the SymPy oracle to RUN, but it is unavailable"
                : "the SymPy oracle is unavailable";
            throw new InvalidOperationException(
                $"{why}: {probe.Reason}. The differential oracle would compare nothing, " +
                "so it fails instead of reporting a pass.");
        }
        return new SympySession(probe.Version ?? "unknown");
    }

    /// <summary>
    /// Probes python3 first, then sympy inside it, so the reason names what was actually
    /// missing: no interpreter at all, an interpreter that exits non-zero (a Windows Store
    /// app-execution alias stub does exactly this), or an interpreter without the module.
    /// </summary>
    private static SympyProbe ProbeOnce()
    {
        PythonRun version = RunScript("import sys; print(sys.version.split()[0])", s_probeTimeout);
        if (!version.Started)
            return new SympyProbe(false, null, $"python3 could not be started ({version.Failure})");
        if (version.ExitCode != 0 || version.Stdout.Length == 0)
            return new SympyProbe(false, null,
                $"python3 is on PATH but is not a working interpreter (exit {version.ExitCode}): {Detail(version.Stderr)}");

        string pythonVersion = version.Stdout.Trim();
        PythonRun sympy = RunScript("import sympy; print(sympy.__version__)", s_probeTimeout);
        if (!sympy.Started)
            return new SympyProbe(false, null, $"python3 {pythonVersion} could not run the sympy probe ({sympy.Failure})");
        if (sympy.ExitCode != 0 || sympy.Stdout.Length == 0)
            return new SympyProbe(false, null,
                $"python3 {pythonVersion} is installed but `import sympy` failed (exit {sympy.ExitCode}): {Detail(sympy.Stderr)}");

        return new SympyProbe(true, $"python3 {pythonVersion}, sympy {sympy.Stdout.Trim()}",
            $"python3 {pythonVersion} with sympy {sympy.Stdout.Trim()}");
    }

    /// <summary>First non-empty line of a stream, trimmed and bounded, for the reason text.</summary>
    private static string Detail(string text)
    {
        foreach (string line in text.Split('\n'))
        {
            string trimmed = line.Trim();
            if (trimmed.Length == 0)
                continue;
            return trimmed.Length <= 220 ? trimmed : trimmed[..220] + "...";
        }
        return "(no output)";
    }

    /// <summary>
    /// Runs one python3 process. The streams are read asynchronously so a hung interpreter
    /// cannot block the read past the timeout; a timeout kills the process tree.
    /// </summary>
    internal static PythonRun RunScript(string code, TimeSpan? timeout = null)
    {
        TimeSpan limit = timeout ?? s_evalTimeout;
        var psi = new ProcessStartInfo
        {
            FileName = "python3",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        psi.ArgumentList.Add("-c");
        psi.ArgumentList.Add(code);

        Process? process;
        try
        {
            process = Process.Start(psi);
        }
        catch (Win32Exception ex)
        {
            return new PythonRun(false, -1, string.Empty, string.Empty, ex.Message);
        }
        if (process is null)
            return new PythonRun(false, -1, string.Empty, string.Empty, "Process.Start returned null");

        using (process)
        {
            Task<string> stdout = process.StandardOutput.ReadToEndAsync();
            Task<string> stderr = process.StandardError.ReadToEndAsync();
            if (!process.WaitForExit((int)limit.TotalMilliseconds))
            {
                try { process.Kill(entireProcessTree: true); } catch (InvalidOperationException) { }
                return new PythonRun(true, -1, string.Empty, Drain(stderr), $"timed out after {limit.TotalSeconds:0}s");
            }
            return new PythonRun(true, process.ExitCode, Drain(stdout).Trim(), Drain(stderr), string.Empty);
        }
    }

    private static string Drain(Task<string> stream)
    {
        try
        {
            return stream.Wait(TimeSpan.FromSeconds(10)) ? stream.Result : string.Empty;
        }
        catch (AggregateException)
        {
            return string.Empty;
        }
    }
}

/// <summary>
/// A live SymPy evaluation session, obtainable only from <see cref="SympyOracle.Require"/>.
/// Every program runs in its own python3 process with <c>sympy</c> already imported.
/// </summary>
internal sealed class SympySession
{
    internal string Version { get; }

    internal SympySession(string version) => Version = version;

    /// <summary>
    /// Runs a python program and returns its stdout. A non-zero exit throws with the
    /// interpreter's stderr AND the failing program: an oracle answer that silently became
    /// empty is exactly the failure mode this file exists to prevent.
    /// </summary>
    internal string Eval(string body)
    {
        // N20: the corpora are written in natural SymPy syntax - sympy.diff(sin(x**2), x) - so the
        // prelude must expose sympy's function namespace. With only `import sympy`, every name the
        // corpus uses unqualified raised NameError, and the derivative and limit corpora compared
        // NOTHING while the failure surfaced as "the SymPy side did not evaluate". The symbols are
        // bound after this prelude, so the star import cannot shadow them.
        PythonRun run = SympyOracle.RunScript("import sympy\nfrom sympy import *\n" + body);
        if (!run.Started)
            throw new InvalidOperationException($"python3 could not be started ({run.Failure})");
        if (run.ExitCode != 0)
            throw new InvalidOperationException(
                $"the SymPy oracle program failed (exit {run.ExitCode}): {run.Stderr.Trim()}\n--- program ---\n{body}");
        return run.Stdout.Trim();
    }

    /// <summary>Runs a program that prints one answer per line and returns the non-empty lines.</summary>
    internal IReadOnlyList<string> EvalLines(string body) =>
        Eval(body).Split('\n', StringSplitOptions.RemoveEmptyEntries).Select(line => line.Trim()).ToArray();
}

/// <summary>Python source fragments shared by the corpora.</summary>
internal static class SympyScript
{
    /// <summary><c>x = sympy.symbols('x')</c> / <c>x, y = sympy.symbols('x y')</c>.</summary>
    internal static string Symbols(IReadOnlyList<string> names) =>
        names.Count == 1
            ? $"{names[0]} = sympy.symbols('{names[0]}')"
            : $"{string.Join(", ", names)} = sympy.symbols('{string.Join(" ", names)}')";
}

/// <summary>
/// Fact checks a kernel result against SymPy by independent numeric evaluation at rational
/// sample points. A disagreement is returned as a triage message naming BOTH forms, the point
/// and both values — never as a bare boolean.
/// </summary>
internal static class OracleCompare
{
    /// <summary>The tolerance of the original derivative corpus.</summary>
    internal const string RealTolerance = "0.0000000000001";

    /// <summary>1e-20: for values obtained at 30+ correct significant digits.</summary>
    internal static readonly string TightTolerance = "0." + new string('0', 19) + "1";

    /// <summary>
    /// RootOf has no closed form to evaluate; the kernel's own exact isolator gives its value.
    /// Everything else goes through the ordinary numeric evaluator.
    /// </summary>
    internal static Num ToNum(Expr e, ExprContext ctx, IReadOnlyDictionary<Symbol, Num> bindings) =>
        e is RootOfExpr ? NumOps.FromReal(Roots.N(e, 30, ctx)) : Evaluation.EvaluateToNum(e, ctx, bindings);

    /// <summary>A Num printed for a human, without depending on Num.ToString.</summary>
    internal static string Show(Num n)
    {
        try
        {
            return NumOps.ToReal(n).ToString();
        }
        catch (Exception)
        {
            return n.ToString() ?? "?";
        }
    }

    /// <summary>Compares one kernel form against one SymPy expression.</summary>
    internal static string? NumberAgrees(
        SympySession sympy,
        string label,
        string kernelCanonical,
        string sympyExpression,
        ExprContext ctx,
        IReadOnlyList<string> symbols,
        IReadOnlyList<IReadOnlyList<string>> points,
        string? tolerance = null) =>
        NumberListsAgree(sympy, label, new[] { kernelCanonical }, new[] { sympyExpression }, ctx, symbols, points, tolerance);

    /// <summary>
    /// Compares parallel lists (e.g. the row-major entries of a matrix) against SymPy at every
    /// sample point in ONE python process: the program prints point-major, expression-minor
    /// values, and each is compared with the kernel's own evaluation.
    ///
    /// <para>Every value is printed as its REAL and IMAGINARY parts, and the kernel side is
    /// compared part by part. This is strictly stronger than comparing one real number per value:
    /// a case where SymPy answers with a complex value and the kernel with a real one (or with a
    /// different imaginary part) is now a reported MISMATCH instead of "the SymPy side returned
    /// something that is not a finite real number". It is what makes the principal-branch complex
    /// values in the corpora comparable at all — the tolerance is unchanged, and both parts must
    /// pass it.</para>
    /// </summary>
    internal static string? NumberListsAgree(
        SympySession sympy,
        string label,
        IReadOnlyList<string> kernelCanonicals,
        IReadOnlyList<string> sympyExpressions,
        ExprContext ctx,
        IReadOnlyList<string> symbols,
        IReadOnlyList<IReadOnlyList<string>> points,
        string? tolerance = null)
    {
        if (kernelCanonicals.Count != sympyExpressions.Count)
            return $"{label}: the corpus is malformed ({kernelCanonicals.Count} kernel forms vs {sympyExpressions.Count} SymPy expressions)";
        if (points.Count == 0)
            return $"{label}: the corpus is malformed (no sample point to compare at)";

        using var scope = Rl.WithPrecision(80, 40);
        Rl allowed = Rl.Parse(tolerance ?? RealTolerance, null);
        Expr[] kernel = kernelCanonicals.Select(text => Printing.CanonicalParse(text, ctx)).ToArray();

        var substitutions = new List<string>();
        foreach (IReadOnlyList<string> point in points)
        {
            var bindings = new List<string>();
            for (int i = 0; i < symbols.Count; i++)
            {
                Rat value = Rat.Parse(point[i], null);
                bindings.Add($"{symbols[i]}: sympy.Rational({value.Numerator}, {value.Denominator})");
            }
            substitutions.Add("{" + string.Join(", ", bindings) + "}");
        }

        string program =
            $"{SympyScript.Symbols(symbols)}\n" +
            "expressions = [" + string.Join(", ", sympyExpressions.Select(e => "(" + e + ")")) + "]\n" +
            "points = [" + string.Join(", ", substitutions) + "]\n" +
            "for point in points:\n" +
            "    for expression in expressions:\n" +
            "        value = expression.subs(point)\n" +
            "        print(sympy.sstr(sympy.N(sympy.re(value), 30)))\n" +
            "        print(sympy.sstr(sympy.N(sympy.im(value), 30)))";
        string theirText;
        try
        {
            theirText = sympy.Eval(program);
        }
        catch (InvalidOperationException ex)
        {
            return $"{label}: the SymPy side did not evaluate ({ex.Message}); kernel forms [{string.Join(", ", kernelCanonicals)}]";
        }
        string[] theirValues = theirText.Split('\n', StringSplitOptions.RemoveEmptyEntries).Select(l => l.Trim()).ToArray();
        int expected = points.Count * kernelCanonicals.Count * 2;   // real and imaginary part per value
        if (theirValues.Length != expected)
            return $"{label}: the SymPy side returned {theirValues.Length} value(s), expected {expected}; kernel forms [{string.Join(", ", kernelCanonicals)}]";

        for (int p = 0; p < points.Count; p++)
        {
            string where = string.Join(", ", symbols.Select((name, i) => $"{name} = {points[p][i]}"));
            var bindings = new Dictionary<Symbol, Num>();
            for (int i = 0; i < symbols.Count; i++)
                bindings[ctx.Symbol(symbols[i])] = NumOps.FromRat(Rat.Parse(points[p][i], null));

            for (int k = 0; k < kernel.Length; k++)
            {
                string theirRealText = theirValues[(p * kernel.Length + k) * 2];
                string theirImaginaryText = theirValues[((p * kernel.Length + k) * 2) + 1];
                if (!Rl.TryParse(theirRealText, null, out Rl? theirReal) ||
                    !Rl.TryParse(theirImaginaryText, null, out Rl? theirImaginary))
                    return $"{label} at {where}: sympy returned '{theirRealText}' + '{theirImaginaryText}'*I, which is not a finite complex number; kernel form {kernelCanonicals[k]}";
                Num mine;
                try
                {
                    mine = ToNum(kernel[k], ctx, bindings);
                }
                catch (Exception ex) when (ex is EvaluationException or InvalidOperationException)
                {
                    return $"{label} at {where}: the kernel side did not evaluate ({ex.GetType().Name}: {ex.Message}); kernel form {kernelCanonicals[k]} vs sympy {sympyExpressions[k]}";
                }
                Num realDifference = NumOps.Abs(NumOps.Subtract(NumOps.Re(mine, ctx), NumOps.FromReal(theirReal)), ctx);
                if (NumOps.Compare(realDifference, NumOps.FromReal(allowed)) >= 0)
                    return $"{label} at {where}: MISMATCH — kernel {kernelCanonicals[k]} = {Show(mine)} vs sympy {sympyExpressions[k]} = {theirRealText} (real part)";
                Num imaginaryDifference = NumOps.Abs(NumOps.Subtract(NumOps.Im(mine, ctx), NumOps.FromReal(theirImaginary)), ctx);
                if (NumOps.Compare(imaginaryDifference, NumOps.FromReal(allowed)) >= 0)
                    return $"{label} at {where}: MISMATCH — kernel {kernelCanonicals[k]} = {Show(mine)} vs sympy {sympyExpressions[k]} = {theirImaginaryText} (imaginary part)";
            }
        }
        return null;
    }
}

/// <summary>
/// Runs an oracle test only when python3 + sympy are actually present. xUnit v2 has no dynamic
/// skip (<c>Xunit.Sdk.SkipException.ForSkip</c> is documented as v3-only), so the precondition is
/// evaluated when the attribute is constructed and the run reports the reason verbatim —
/// the same construction-time pattern as
/// <c>Lovelace.Run.Tests.StdoutPurityTests.RequiresRunnerProcessFactAttribute</c>.
///
/// <para><c>LOVELACE_REQUIRE_SYMPY=1</c> inverts the decision: nothing is skipped, the test body
/// runs, <see cref="SympyOracle.Require"/> throws and the test FAILS. An oracle that was
/// required must never be reported as a pass.</para>
/// </summary>
internal sealed class RequiresSympyFactAttribute : FactAttribute
{
    public RequiresSympyFactAttribute()
    {
        SympyProbe probe = SympyOracle.Probe;
        if (probe.Usable || SympyOracle.Required)
            return;
        Skip = $"SymPy oracle unavailable: {probe.Reason}. These tests compare kernel results " +
               "against SymPy and cannot run here, so they are reported as SKIPPED, not passed. " +
               $"Set {SympyOracle.RequireVariable}=1 to make an unavailable oracle a failure.";
    }
}
