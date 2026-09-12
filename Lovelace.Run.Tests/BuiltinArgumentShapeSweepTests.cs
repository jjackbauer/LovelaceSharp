using System.Globalization;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Run.Tests;

/// <summary>
/// Audit finding F1 (docs/goal-cycle-6/round-13/audit-D-workflow.md:31-60): a wrong-SHAPED argument
/// — a scalar where a matrix is required, a symbolic value where a text name is required, a vector
/// where a relation is required — crosses the wire as
/// <c>InternalError</c>/<c>InternalInvariantFailure</c> with the raw CLR message and
/// <c>recoverable: false</c>, instead of the documented recoverable argument error
/// (<c>code: InvalidArgument</c>, <c>category: TypeMismatch</c>) that
/// <see cref="SystemBuiltinArgumentShapeTests"/> froze for the same class of mistake
/// (Lovelace.Run.Tests/SystemBuiltinArgumentShapeTests.cs:6-19, docs/symbolics/dsh-protocol.md:207-218).
/// <para>
/// The property is asserted over the LIVE registry — every builtin, core and plugin alike, driven
/// with the three wrong shapes of the audit sweep (a bare scalar, a four-element vector, a symbolic
/// value) — rather than over a hand-written list of the 26 builtins the audit named: a builtin
/// added tomorrow is covered by construction, and a per-call-site patch cannot dodge the sweep.
/// </para>
/// <para>
/// The refinement the sweep makes: a refusal that is NOT the documented argument error is not
/// automatically a violation — the kernel also refuses well-shaped arguments for domain reasons
/// (<c>InvalidOperation</c>/<c>DomainError</c>), and <c>capabilities()</c> advertises exactly the
/// coercion refusals that the frozen contract keeps in that family
/// (Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs:69-73,284-321). What is NEVER allowed is an
/// internal invariant failure, an unrecoverable refusal, or a raw CLR exception message as the
/// answer to a caller-side mistake; and any refusal that does use the argument-error family must be
/// well formed — naming the builtin, the 1-based argument position and what arrived.
/// </para>
/// </summary>
public class BuiltinArgumentShapeSweepTests
{
    /// <summary>The registry the published runner wires (Lovelace.Run/Program.cs is the reference
    /// wiring: core builtins plus Dsp, Symbolics and MathIR).</summary>
    private static SuiteEngine NewRegistry()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    /// <summary>Runs the PUBLISHED runner (the same entry point the AOT binary uses) over an
    /// inline script and parses the one JSON envelope it emits.</summary>
    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        string plotDirectory = Path.Combine(Path.GetTempPath(), "lovelace-shape-sweep-plot");
        int exitCode = await Runner.RunAsync(
            new[] { "--eval", script, "--omit-functions", "--omit-variables", "--plot-dir", plotDirectory },
            stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    /// <summary>The symbolic value the third shape needs; the other two shapes need no declaration.</summary>
    private const string Declare = "x = symbol(\"x\"); ";

    /// <summary>The three wrong shapes of the audit sweep, one call per (builtin, shape), every
    /// declared argument position filled from the builtin's own descriptor.</summary>
    private static readonly (string Shape, string Literal)[] WrongShapes =
    {
        ("a bare scalar", "1"),
        ("a four-element vector", "[1, 2, 3, 4]"),
        ("a symbolic value", "x"),
    };

    private static string Call(string builtin, string literal, int arity) =>
        $"{builtin}({string.Join(", ", Enumerable.Repeat(literal, arity))})";

    /// <summary>The one documented refusal grammar for an argument-shape mistake:
    /// <c>det(): argument 1 must be an array or vector; got Natural.</c> — the builtin, the 1-based
    /// position, the expectation and the kind that arrived. The declared parameter name may ride
    /// along parenthetically (<c>evalf(): argument 2 (digits) must be …; got Vector.</c>), which the
    /// frozen evalf digit-count contract pins (Lovelace.Run.Tests/BuiltinSurfaceContractTests.cs:117,
    /// Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs:59).</summary>
    private static readonly Regex DocumentedArgumentError = new(
        @"^(?<builtin>[A-Za-z_][A-Za-z0-9_]*)\(\): argument (?<position>[1-9][0-9]*)( \([A-Za-z_][A-Za-z0-9_]*\))? must be .+; got .+\.$",
        RegexOptions.Compiled);

    /// <summary>Raw framework-failure vocabulary. A caller-side mistake never crosses as one of
    /// these: the answer is a typed refusal in the DSH taxonomy, not a CLR exception text.</summary>
    private static readonly string[] RawFrameworkVocabulary =
    {
        "cast is not valid",
        "Unable to cast",
        "Object reference not set",
        "Index was outside the bounds",
        "Value cannot be null",
        "was not in a correct format",
    };

    [Fact]
    public async Task EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition()
    {
        var registry = NewRegistry().Functions.Values
            .Where(fn => fn.IsBuiltin)
            .OrderBy(fn => fn.Name, StringComparer.Ordinal)
            .ToArray();

        // the property is only meaningful over the whole shipped surface
        Assert.True(registry.Length >= 100, $"the live registry exposed {registry.Length} builtins");

        var violations = new List<string>();
        int probes = 0, accepted = 0, refusals = 0, argumentRefusals = 0, internalFailures = 0;

        foreach (var fn in registry)
        {
            int arity = fn.Parameters.Count;
            if (arity == 0)
                continue;   // no argument position exists to give the wrong shape to

            Assert.True(Regex.IsMatch(fn.Name, "^[A-Za-z_][A-Za-z0-9_]*$"),
                $"'{fn.Name}' is not a plain identifier, so the registry cannot be driven by name");

            foreach (var (shape, literal) in WrongShapes)
            {
                probes++;
                string call = Call(fn.Name, literal, arity);
                var (exitCode, envelope) = await RunAsync(Declare + call);

                string where = $"{call} [{shape}]";
                bool ok = envelope["ok"]!.GetValue<bool>();
                if (ok)
                {
                    accepted++;   // a shape this builtin genuinely accepts is not a violation
                    continue;
                }

                refusals++;
                string code = envelope["code"]?.GetValue<string>() ?? "(absent)";
                string category = envelope["category"]?.GetValue<string>() ?? "(absent)";
                string message = envelope["message"]?.GetValue<string>() ?? string.Empty;
                bool recoverable = envelope["recoverable"]?.GetValue<bool>() ?? false;
                string observed = $"code={code}, category={category}, recoverable={recoverable}, message='{message}'";

                if (code == "InternalError" || category == "InternalInvariantFailure")
                {
                    internalFailures++;
                    violations.Add($"{where}: a caller-side mistake crossed as an INTERNAL failure ({observed})");
                    continue;
                }

                if (!recoverable)
                {
                    violations.Add($"{where}: the refusal is not recoverable ({observed})");
                    continue;
                }

                if (RawFrameworkVocabulary.Any(v => message.Contains(v, StringComparison.OrdinalIgnoreCase)))
                {
                    violations.Add($"{where}: the refusal carries raw framework text ({observed})");
                    continue;
                }

                if (exitCode != 1)
                    violations.Add($"{where}: a refusal must exit 1, not {exitCode} ({observed})");

                if (code != "InvalidArgument")
                    continue;   // a domain refusal: a different, already-classified family

                argumentRefusals++;

                if (category != "TypeMismatch")
                {
                    violations.Add($"{where}: an argument refusal must carry category TypeMismatch ({observed})");
                    continue;
                }

                Match documented = DocumentedArgumentError.Match(message);
                if (!documented.Success)
                {
                    violations.Add($"{where}: an argument refusal must name the builtin, the 1-based position, " +
                        $"the expectation and the kind that arrived ({observed})");
                    continue;
                }

                if (documented.Groups["builtin"].Value != fn.Name)
                    violations.Add($"{where}: the refusal names '{documented.Groups["builtin"].Value}' ({observed})");

                int position = int.Parse(documented.Groups["position"].Value, CultureInfo.InvariantCulture);
                if (position > arity)
                    violations.Add($"{where}: the refusal names argument {position}, beyond the {arity} declared " +
                        $"position(s) ({observed})");
            }
        }

        // non-vacuity: the sweep really drove the registry and really saw both outcomes
        Assert.True(probes >= 300, $"the sweep drove only {probes} probes");
        Assert.True(accepted >= 100, $"only {accepted} of {probes} probes were accepted at all");
        Assert.True(refusals >= 1, $"no probe was refused, so the sweep asserts nothing about refusals");
        Assert.True(argumentRefusals >= 1,
            $"none of the {refusals} refusals used the documented argument-error family: F1 is not closed by " +
            "refusing the call, only by refusing it in the documented shape");

        Assert.True(violations.Count == 0,
            $"{violations.Count} of {refusals} refusals ({probes} probes over {registry.Length} builtins) violate the " +
            $"frozen argument-shape contract ({internalFailures} crossed as internal invariant failures):" +
            Environment.NewLine + string.Join(Environment.NewLine, violations));
    }

    /// <summary>The audit measured <c>52 / 26 builtins</c> over three shapes and recorded it as a
    /// LOWER BOUND ("3 shapes per builtin, not the full type lattice",
    /// docs/goal-cycle-6/round-13/audit-D-workflow.md:312). This test widens the lattice — a Record,
    /// a matrix, text, a Boolean, an empty vector, on top of the three audit shapes — and asserts
    /// the part of the contract that is unconditional over every shape: no builtin may answer a
    /// caller-side argument with an internal invariant failure, an unrecoverable refusal, or the raw
    /// text of a framework exception.</summary>
    [Fact]
    public async Task EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure()
    {
        var registry = NewRegistry().Functions.Values
            .Where(fn => fn.IsBuiltin)
            .OrderBy(fn => fn.Name, StringComparer.Ordinal)
            .ToArray();

        var shapes = new (string Shape, string Literal)[]
        {
            ("a bare scalar", "1"),
            ("a four-element vector", "[1, 2, 3, 4]"),
            ("a symbolic value", "x"),
            ("a record", "inspect(1)"),
            ("a 2x2 matrix", "[[1, 2], [3, 4]]"),
            ("a text value", "\"x\""),
            ("a Boolean", "1 == 1"),
            ("an empty vector", "[]"),
        };

        var violations = new List<string>();
        int probes = 0, accepted = 0, refusals = 0;

        foreach (var fn in registry)
        {
            int arity = fn.Parameters.Count;
            if (arity == 0)
                continue;

            foreach (var (shape, literal) in shapes)
            {
                probes++;
                string call = Call(fn.Name, literal, arity);
                var (exitCode, envelope) = await RunAsync(Declare + call);
                if (envelope["ok"]!.GetValue<bool>())
                {
                    accepted++;
                    continue;
                }

                refusals++;
                string where = $"{call} [{shape}]";
                string code = envelope["code"]?.GetValue<string>() ?? "(absent)";
                string category = envelope["category"]?.GetValue<string>() ?? "(absent)";
                string message = envelope["message"]?.GetValue<string>() ?? string.Empty;
                bool recoverable = envelope["recoverable"]?.GetValue<bool>() ?? false;
                string observed = $"code={code}, category={category}, recoverable={recoverable}, message='{message}'";

                if (code == "InternalError" || category == "InternalInvariantFailure")
                    violations.Add($"{where}: an INTERNAL failure ({observed})");
                else if (!recoverable)
                    violations.Add($"{where}: the refusal is not recoverable ({observed})");
                else if (RawFrameworkVocabulary.Any(v => message.Contains(v, StringComparison.OrdinalIgnoreCase)))
                    violations.Add($"{where}: the refusal carries raw framework text ({observed})");
                else if (exitCode != 1)
                    violations.Add($"{where}: a refusal must exit 1, not {exitCode} ({observed})");
            }
        }

        Assert.True(probes >= 800, $"the wide sweep drove only {probes} probes");
        Assert.True(accepted >= 100, $"only {accepted} of {probes} probes were accepted at all");
        Assert.True(violations.Count == 0,
            $"{violations.Count} of {refusals} refusals ({probes} probes over {registry.Length} builtins × " +
            $"{shapes.Length} shapes) are not typed refusals:" + Environment.NewLine +
            string.Join(Environment.NewLine, violations));
    }

    /// <summary>The five calls the audit reproduced on the published binary
    /// (docs/goal-cycle-6/round-13/audit-D-workflow.md:36-40). Each crossed as
    /// <c>InternalError</c>/<c>InternalInvariantFailure</c> <c>"Specified cast is not valid."</c> with
    /// <c>recoverable: false</c>; each must cross as the documented argument error naming the
    /// builtin, the position and the kind that arrived.</summary>
    [Theory]
    [InlineData("det(1)", "det", 1)]
    [InlineData("matmul(1, 1)", "matmul", 1)]
    [InlineData("symbol(1)", "symbol", 1)]
    [InlineData("assume(1)", "assume", 1)]
    [InlineData("transpose(1, 1)", "transpose", 1)]
    public async Task TheAuditsFiveReproductions_CrossAsTheDocumentedArgumentError(
        string call, string builtin, int position)
    {
        var (exitCode, envelope) = await RunAsync(call);

        Assert.Equal(1, exitCode);
        Assert.False(envelope["ok"]!.GetValue<bool>(), "a rejected call reports ok == false");
        Assert.Equal("InvalidArgument", envelope["code"]!.GetValue<string>());
        Assert.Equal("TypeMismatch", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>(),
            "a wrong argument shape is a user mistake the caller can fix and retry");

        string message = envelope["message"]!.GetValue<string>();
        Assert.StartsWith($"{builtin}(): argument {position} must be ", message);
        Assert.EndsWith("; got Natural.", message);
        Assert.DoesNotContain("cast", message);
    }

    /// <summary>The positive control: the declared call forms are untouched by the guard, so the
    /// contract is "the wrong shape is refused", not "the builtin is now unreachable".</summary>
    [Theory]
    [InlineData("det([[1, 2], [3, 4]])")]
    [InlineData("matmul([[1, 2], [3, 4]], [[5], [6]])")]
    [InlineData("transpose([[1, 2], [3, 4]])")]
    [InlineData("trace([[1, 2], [3, 4]])")]
    [InlineData("shape([[1, 2], [3, 4]])")]
    [InlineData("symbol(\"x\")")]
    [InlineData("x = symbol(\"x\"); assume(x > 5)")]
    [InlineData("x = symbol(\"x\"); assume_positive(x)")]
    [InlineData("x = symbol(\"x\"); assume_real(\"y\")")]
    public async Task TheDeclaredForm_StaysCallable(string script)
    {
        var (exitCode, envelope) = await RunAsync(script);

        Assert.True(exitCode == 0, $"'{script}' exited {exitCode}: {envelope.ToJsonString()}");
        Assert.True(envelope["ok"]!.GetValue<bool>(), $"'{script}' must evaluate");
    }
}
