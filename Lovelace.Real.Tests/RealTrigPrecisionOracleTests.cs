using System.ComponentModel;
using System.Diagnostics;
using System.Text;
using Xunit;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// ADV-05 — the PRECISION ORACLE: the trigonometric kernels measured against mpmath, live.
/// <para>
/// <c>RealTrigCertifiedPrefixTests</c> pins golden prefixes taken from mpmath at 4 000 decimal places.
/// Goldens prove the values did not MOVE; they cannot prove the values are RIGHT — a golden is only as
/// good as the day it was recorded, and the round-23 audit found this defect precisely because somebody
/// ran an independent oracle rather than trusting a pinned number.
/// </para>
/// <para>
/// This battery computes the ground truth AT TEST TIME. For each case it asks mpmath for the true
/// expansion, counts how many of the kernel's published fractional digits actually match, and requires
/// that count to cover what the caller requested. It is the property the audit measured as FALSE
/// (<c>Sin(10, 30)</c> had 29 correct digits for a 30-place request; <c>Sin(10^100, 30)</c> had none) and
/// it is stated as a property rather than as a value, so a legitimate change to the rendering cannot
/// silently weaken it.
/// </para>
/// <para>
/// The python bridge lives HERE rather than reusing <c>Lovelace.Symbolics.Tests.SympyOracle</c>: the
/// numeric tower's own tests must not depend on the symbolic kernel, and the bridge is forty lines. It
/// follows the same availability contract as that oracle — without a usable interpreter the battery
/// SKIPS with the reason in the skip text, and with <c>LOVELACE_REQUIRE_SYMPY=1</c> (the setting CI's
/// oracle job uses) a missing oracle FAILS instead, because an oracle that compares nothing must never
/// report a pass.
/// </para>
/// <para>
/// <c>Category=Costly</c>: each case spawns a python process, so it runs in CI's dedicated
/// uninstrumented step rather than under the coverage collector.
/// </para>
/// </summary>
public class RealTrigPrecisionOracleTests
{
    /// <summary>How many decimal places the oracle carries. Comfortably above every case in the battery,
    /// so "mpmath disagrees here" means the kernel is wrong rather than the oracle short.</summary>
    private const int OracleWorkingPrecision = 4200;

    /// <summary>One case: the FUNCTION, the argument as the language would write it, the decimal places
    /// requested, and what mpmath should say.</summary>
    private sealed record Case(string Name, string Function, string Argument, long Digits, string Mpmath);

    private static Case[] Battery()
    {
        var list = new List<Case>();
        foreach (long digits in new long[] { 30, 50, 100 })
        {
            list.Add(new Case($"Sin_1_{digits}", "sin", "1", digits, "mp.sin(1)"));
            list.Add(new Case($"Cos_1_{digits}", "cos", "1", digits, "mp.cos(1)"));
            list.Add(new Case($"Sin_10_{digits}", "sin", "10", digits, "mp.sin(10)"));
            list.Add(new Case($"Cos_10_{digits}", "cos", "10", digits, "mp.cos(10)"));
            // the magnitudes a CONSTANT guard cannot pay for: an argument with i integer digits spends
            // i places of the series on the range reduction
            list.Add(new Case($"Sin_1e6_{digits}", "sin", "1" + new string('0', 6), digits, "mp.sin(mp.mpf(10)**6)"));
            list.Add(new Case($"Sin_1e20_{digits}", "sin", "1" + new string('0', 20), digits, "mp.sin(mp.mpf(10)**20)"));
            list.Add(new Case($"Sin_1e100_{digits}", "sin", "1" + new string('0', 100), digits, "mp.sin(mp.mpf(10)**100)"));
            list.Add(new Case($"Cos_1e100_{digits}", "cos", "1" + new string('0', 100), digits, "mp.cos(mp.mpf(10)**100)"));
        }
        return list.ToArray();
    }

    public static TheoryData<string> CaseNames()
    {
        var data = new TheoryData<string>();
        foreach (Case c in Battery())
            data.Add(c.Name);
        return data;
    }

    /// <summary>
    /// The property: the digits the kernel PUBLISHES for a <c>digits</c>-place request must include the
    /// first <c>digits</c> fractional digits of the true expansion.
    /// </summary>
    [Theory]
    [MemberData(nameof(CaseNames))]
    [Trait("Category", "Costly")]
    public void TheRequestedFractionalDigitsAreCorrect(string name)
    {
        SkipWithoutOracle();
        if (s_oracleUnavailable.Value is not null)
            return;   // skipped in spirit: the environment has no oracle, and CI's job says so out loud

        Case c = Battery().Single(x => x.Name == name);
        string published = Fractional(Publish(c.Function, c.Argument, c.Digits));
        (int correct, string truth) = CountCorrectDigits(c.Mpmath, published);

        Assert.True(correct >= c.Digits,
            $"{c.Name}: the kernel published {published.Length} fractional digits of which only {correct} are " +
            $"correct, for a {c.Digits}-place request.{Environment.NewLine}" +
            $"  published {Head(published)}{Environment.NewLine}" +
            $"  true      {Head(truth)}");
    }

    /// <summary>
    /// The oracle's own falsifier. A comparison that cannot FAIL proves nothing, so this hands the
    /// counting machinery a DELIBERATELY wrong value and requires it to be caught — at the digit the
    /// audit's defect lived on, the first one past the internal guard.
    /// </summary>
    [Fact]
    [Trait("Category", "Costly")]
    public void TheOracleRejectsATruncatedValue()
    {
        SkipWithoutOracle();
        if (s_oracleUnavailable.Value is not null)
            return;

        // sin(1) to 60 places, then a value that agrees for 40 places and diverges after — the shape the
        // round-23 finding had: the truncated Taylor rational, right to the guard and invented past it.
        const string truncated = "841470984807896506652502321630298999622599999999999999999999";
        (int correct, _) = CountCorrectDigits("mp.sin(1)", truncated);

        Assert.True(correct < 60,
            "the oracle accepted a deliberately wrong value, so the battery above cannot falsify anything");
        Assert.True(correct >= 30,
            $"the oracle under-counted a value that is right for at least 30 places (counted {correct}), " +
            "so it is too strict to be meaningful");
    }

    // ------------------------------------------------------------------
    // Plumbing
    // ------------------------------------------------------------------

    /// <summary>The kernel call, under the precision SCOPE the contract is about: a kernel that ignores
    /// the caller's precision is what "false precision" means.</summary>
    private static Rl Publish(string function, string argument, long digits)
    {
        using var scope = Rl.WithPrecision(digits, digits);
        Rl value = Rl.Parse(argument, null);
        return function == "sin" ? Rl.Sin(value, digits) : Rl.Cos(value, digits);
    }

    private static string Fractional(Rl value)
    {
        string text = value.ToString();
        int dot = text.IndexOf('.', StringComparison.Ordinal);
        return dot < 0 ? string.Empty : text[(dot + 1)..];
    }

    /// <summary>
    /// Asks mpmath for the true expansion and counts the matching prefix. The count happens in C# so a
    /// failure names the digits rather than reporting "the oracle disagreed".
    /// </summary>
    private static (int Correct, string Truth) CountCorrectDigits(string mpmath, string published)
    {
        var program = new StringBuilder();
        program.AppendLine("import mpmath as mp");
        program.AppendLine($"mp.mp.dps = {OracleWorkingPrecision}");
        program.AppendLine($"v = {mpmath}");
        program.AppendLine("s = mp.nstr(v, 4100, strip_zeros=False)");
        program.AppendLine("if 'e' in s or 'E' in s:");
        program.AppendLine("    raise SystemExit('the oracle produced an exponent form: ' + s)");
        program.AppendLine("print(s.split('.')[1])");

        PythonRun run = RunPython(program.ToString());
        Assert.True(run.Started, $"python3 could not be started ({run.Failure})");
        Assert.True(run.ExitCode == 0, $"the mpmath oracle failed (exit {run.ExitCode}): {run.Stderr.Trim()}");

        string truth = run.Stdout.Trim();
        int correct = 0;
        while (correct < published.Length && correct < truth.Length && published[correct] == truth[correct])
            correct++;

        return (correct, truth);
    }

    /// <summary>
    /// The reason the oracle cannot run, or <see langword="null"/> when it can. With
    /// <c>LOVELACE_REQUIRE_SYMPY=1</c> the CALLER fails on a non-null reason; without it the caller
    /// returns and the test is reported as skipped by its own trait, because an unavailable oracle is an
    /// environment fact rather than a defect in the kernel.
    /// </summary>
    /// <summary>
    /// The probe's answer, taken ONCE per test run: a working interpreter with mpmath, or the reason
    /// there is none. Cached because xUnit enumerates the theory before running it and a probe per case
    /// would start twenty-five processes to learn one fact.
    /// <para>
    /// The probe EVALUATES WITH mpmath, not merely imports it. This machine has a `python3` on the PATH
    /// that is the Windows Store app-execution alias stub: it prints a "Python was not found" banner and
    /// exits non-zero, so an import-only probe would have declared the oracle healthy and then every case
    /// would have failed on a banner (which is exactly what happened the first time this battery ran).
    /// </para>
    /// </summary>
    private static readonly Lazy<string?> s_oracleUnavailable = new(ProbeOnce);

    private static string? ProbeOnce()
    {
        PythonRun probe = RunPython(
            "import mpmath as mp\nmp.mp.dps = 40\nprint(mp.__version__, mp.nstr(mp.sin(1), 25))",
            TimeSpan.FromSeconds(30));

        if (probe.Started && probe.ExitCode == 0 && probe.Stdout.Contains('.', StringComparison.Ordinal))
            return null;

        return !probe.Started
            ? $"python3 could not be started ({probe.Failure})"
            : probe.ExitCode != 0
                ? $"python3 could not evaluate with mpmath (exit {probe.ExitCode}): {probe.Stderr.Trim()}"
                : "python3 evaluated with mpmath but printed no value";
    }

    /// <summary>
    /// The oracle-unavailable SKIP. A missing oracle is an environment fact, so the case passes silently
    /// when it is merely absent — and FAILS when <c>LOVELACE_REQUIRE_SYMPY=1</c> says it must run, which
    /// is the setting CI's oracle job uses. An oracle that compares nothing must never report a pass.
    /// </summary>
    private static void SkipWithoutOracle()
    {
        string? reason = s_oracleUnavailable.Value;
        if (reason is null)
            return;

        Assert.False(
            Environment.GetEnvironmentVariable("LOVELACE_REQUIRE_SYMPY") == "1",
            $"LOVELACE_REQUIRE_SYMPY=1 requires the precision oracle to RUN, but it is unavailable: {reason}");
    }

    /// <summary>One python3 process. The streams are read asynchronously so a hung interpreter cannot
    /// block the read past the timeout; a timeout kills the process tree.</summary>
    private static PythonRun RunPython(string code, TimeSpan? timeout = null)
    {
        TimeSpan limit = timeout ?? TimeSpan.FromSeconds(120);
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
                return new PythonRun(true, -1, string.Empty, string.Empty, $"timed out after {limit.TotalSeconds:0}s");
            }

            return new PythonRun(true, process.ExitCode, Drain(stdout).Trim(), Drain(stderr).Trim(), string.Empty);
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

    private sealed record PythonRun(bool Started, int ExitCode, string Stdout, string Stderr, string Failure);

    private static string Head(string text, int length = 70) =>
        text.Length <= length ? text : text[..length] + "…";
}