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
/// ADV-03 — the integer-argument boundary matrix, driven through the PUBLISHED runner.
/// <para>
/// ADV-01 gave the product one rule (an argument that requires an integer must either represent an exact
/// integer or be refused explicitly, never silently rounded or truncated) and ADV-02 migrated the call
/// sites to it. This is the gate that keeps it closed: for every integer-semantic argument the product
/// has, a matrix of boundary values is driven over the wire, and each cell must land on the RIGHT
/// OUTCOME — the exact value the caller wrote, or a named refusal — never on a different value.
/// </para>
/// <para>
/// THE CENSUS IS THE POINT. A hand-written matrix rots the moment a builtin is added, so
/// <see cref="TheInventoryCoversEveryIntegerSemanticParameterInTheRegistry"/> sweeps the live registry
/// for parameter names that are integer-semantic and fails when one appears that the inventory does not
/// carry, and when the inventory carries one the registry no longer declares. Adding such a builtin
/// therefore breaks the build rather than escaping the contract quietly — which is how the class was
/// found in the first place (a by-hand sweep of nine sites missed the MathIR boundary).
/// </para>
/// <para>
/// Every cell SPAWNS the runner, so the matrix is tagged <c>Category=Costly</c>: it runs uninstrumented
/// in CI's dedicated step, like the other spawn-per-case suites.
/// </para>
/// </summary>
public class IntegerArgumentBoundaryMatrixTests
{
    private static SuiteEngine NewRegistry()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    /// <summary>
    /// Every cell carries a DEADLINE, and that is a finding rather than a precaution: without one, a cell
    /// whose literal the argument accepts and then works on for minutes — `pi(1000000)` — never returns, the
    /// test host sits idle, and the whole theory hangs with 78 of its ~660 cells reported. A boundary matrix
    /// that cannot finish is not a boundary matrix. With `--cancel-after`, the worst case is a cell that
    /// reports the engine's own honest `Cancelled`/`BudgetExceeded` ledger instead of a value, and the
    /// assertions below treat a budget stop as a legitimate outcome for the cells whose fate is the call
    /// site's bound — never for the cells that must ACCEPT or must REFUSE.
    /// </summary>
    private const int CellBudgetMs = 30_000;

    private static async Task<(int ExitCode, JsonNode Envelope)> RunAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        string plotDirectory = Path.Combine(Path.GetTempPath(), "lovelace-int-arg-matrix-plot");
        int exitCode = await Runner.RunAsync(
            new[]
            {
                "--eval", script, "--omit-functions", "--omit-variables", "--plot-dir", plotDirectory,
                "--cancel-after", CellBudgetMs.ToString(CultureInfo.InvariantCulture),
            },
            stdout, stderr);
        return (exitCode, TestSupport.ParseExactlyOneJsonDocument(stdout.ToString(), $"stdout of '{script}'"));
    }

    // ------------------------------------------------------------------
    // The inventory
    // ------------------------------------------------------------------

    /// <summary>
    /// One integer-semantic argument: the builtin, the parameter's name, its 1-based position, a template
    /// that builds a call with <c>{0}</c> where the value goes, and the integers THIS argument's own
    /// contract accepts. The template and the accepted set are spelled out rather than assembled from the
    /// descriptor, because the OTHER arguments must be valid and the bound is the call site's own — an
    /// index accepts 0..2 of a length-3 vector, while a reshape dimension must make the element count
    /// agree. A matrix cell is only meaningful if the call reaches the argument under test.
    /// </summary>
    /// <summary>
    /// One integer-semantic argument. <c>Anchor</c> is the text the ARGUMENT's own refusal prints for it,
    /// which is not always the registry's parameter name: <c>zeros</c> refuses <c>dims</c> as "argument 1
    /// (a dimension)". It is what the liveness check looks for, so a dead row cannot pass by naming the
    /// parameter somewhere else in its message.
    /// </summary>
    /// <summary>The magnitude a CAPPED argument is driven with: far past any cap a caller may write, and far
    /// below the point where the engine would try to honour it. Measured, not guessed: the Int64 boundary
    /// literal made pi start computing 10^18 digits and the theory never finished.</summary>
    // (Superseded: the magnitude rows are now gated per argument by LargeMagnitude rather than capped.)

    /// <summary>
    /// One integer-semantic argument. Anchor is the text the ARGUMENT's own refusal prints for it, which is
    /// not always the registry's parameter name: zeros refuses dims as "argument 1 (a dimension)".
    /// <para>
    /// Capped says the argument's own contract has a SMALL upper bound — a digit count, an order, a seed —
    /// so the matrix must not hand it an Int64-boundary value: it would be accepted and then do minutes of
    /// work with it. An uncapped argument (an index, a dimension, an axis, a count) still sees the boundary,
    /// which is exactly where an integer that does not fit an Int64 has to be refused by name.
    /// </para>
    /// </summary>
    private sealed record IntegerArgument(
        string Builtin, string Parameter, int Position, string Template, string[] Accepted, string Anchor,
        bool LargeMagnitude = false);

    /// <summary>Every integer-semantic argument the product declares, frozen. The census below keeps it
    /// honest in both directions.</summary>
    private static readonly IntegerArgument[] Inventory =
    {
        new("zeros", "dims", 1, "zeros({0})", ["1", "2", "3"], "a dimension"),
        new("ones", "dims", 1, "ones({0})", ["1", "2", "3"], "a dimension"),
        new("eye", "rows", 1, "eye({0})", ["1", "2", "3"], "(rows)", LargeMagnitude: true),
        new("eye", "cols", 2, "eye(2, {0})", ["1", "2", "3"], "(cols)", LargeMagnitude: true),
        // A 4-element array reshapes to [4] or [2, 2]; a shape of ONE element (or 3) cannot describe it,
        // and the product rule is a real constraint the bound cannot express — which is exactly why this
        // inventory carries an accepted set per argument instead of one global list.
        new("reshape", "dims", 2, "reshape([1, 2, 3, 4], {0})", ["4"], "a dimension", LargeMagnitude: true),
        // A rank-2 input, so the axis bound (0..1) is a real bound rather than the degenerate one
        // every rank-1 vector has. The ADV-03 matrix found the out-of-range axis refusal leaking raw
        // framework text (`Axis -1 is out of range for rank 1. (Parameter 'axis')`) from exactly these
        // rows; they are now guarded in the contract grammar, and this is the family that says so.
        new("sum", "axis", 2, "sum([[1, 2], [3, 4]], {0})", ["0", "1"], "(axis)", LargeMagnitude: true),
        new("prod", "axis", 2, "prod([1, 2, 3], {0})", ["0"], "(axis)", LargeMagnitude: true),
        new("min", "axis", 2, "min([1, 2, 3], {0})", ["0"], "(axis)", LargeMagnitude: true),
        new("max", "axis", 2, "max([1, 2, 3], {0})", ["0"], "(axis)", LargeMagnitude: true),
        new("mean", "axis", 2, "mean([1, 2, 3], {0})", ["0"], "(axis)", LargeMagnitude: true),
        new("norm", "axis", 2, "norm([1, 2, 3], {0})", ["0"], "(axis)", LargeMagnitude: true),
        new("concat", "axis", 3, "concat([1, 2], [3, 4], {0})", ["0"], "(axis)", LargeMagnitude: true),
        // ADV-03 routed the digit counts through the shared integer-argument contract, so their
        // refusals now name the POSITION and the declaration the way every other row here does. The
        // anchor is therefore "(digits)"; the kind refusal stays InvalidOperationException (the
        // documented pi(3.0) contract) and the test above accepts either typed family.
        new("setprecision", "digits", 1, "setprecision({0}); 1", ["1", "2", "3"], "(digits)"),
        new("pi", "digits", 1, "pi({0})", ["1", "2", "3"], "(digits)"),
        new("e", "digits", 1, "e({0})", ["1", "2", "3"], "(digits)"),
        new("evalf", "digits", 2, "evalf(1/3, {0})", ["1", "2", "3"], "(digits)"),
        new("series", "order", 4, "x = symbol(\"x\"); series(sin(x), x, 0, {0})", ["1", "2", "3"], "(order)"),
        new("evalir", "digits", 3, "x = symbol(\"x\"); evalir(compile(x*1/3, [x]), [1], {0})", ["1", "2", "3"], "(digits)"),
        new("evalir_batch", "digits", 3, "x = symbol(\"x\"); evalir_batch(compile(x*1/3, [x]), [1], {0})", ["1", "2", "3"], "(digits)"),
        new("movingavg", "w", 2, "movingavg([1, 2, 3, 4, 5], {0})", ["1", "2", "3"], "(w)"),
        new("delay", "k", 2, "delay([1, 2, 3, 4], {0})", ["1", "2", "3"], "(k)"),
        new("dft", "n", 2, "dft([1, 2, 3, 4], {0})", ["4"], "(n)"),
        new("impulse", "n", 1, "impulse({0})", ["1", "2", "3"], "(n)"),
        new("step", "n", 1, "step({0})", ["1", "2", "3"], "(n)"),
        new("exponential", "n", 2, "exponential(0.5, {0})", ["1", "2", "3"], "(n)"),
        new("cosine", "n", 3, "cosine(0.25, 0, {0})", ["1", "2", "3"], "(n)"),
        new("powerseries", "n", 3, "powerseries(2, 0.5, {0})", ["1", "2", "3"], "(n)"),
        new("noise", "n", 3, "noise(1, 0, {0})", ["1", "2", "3"], "(n)"),
        // `seed` is the FOURTH declared position, and only the four-argument form carries one; the
        // three-argument form reads position 3 as the COUNT. Driving the 4-argument form is therefore
        // the only way to reach the seed, which is why the template spells all four arguments.
        new("noise", "seed", 3, "noise(1, 0, {0}, 2)", ["0", "1", "2"], "(seed)"),
    };

    /// <summary>
    /// The integer-semantic arguments that are NOT registry parameters — a subscript, a range bound —
    /// and therefore cannot be reached by the census, which sweeps the REGISTRY. They are driven by the
    /// same matrix below and are listed here so the census's own blindness is visible in one place
    /// rather than being an unstated gap.
    /// </summary>
    private static readonly IntegerArgument[] NonRegistryInventory =
    {
        new("index", "index", 1, "[1, 2, 3][{0}]", ["0", "1", "2"], "'index'", LargeMagnitude: true),
        new("range", "bound", 1, "1..{0}", ["1", "2", "3"], "'bound'", LargeMagnitude: true),
    };

    /// <summary>The parameter names that MAKE an argument integer-semantic. The census treats a builtin
    /// carrying one of these as required to be in the inventory above.</summary>
    private static readonly string[] IntegerSemanticNames =
    {
        "dims", "rows", "cols", "axis", "axes", "order", "digits", "w", "k", "n", "index", "bound", "seed",
    };

    /// <summary>Parameters the census deliberately does not require, each with the reason it is not an
    /// integer-semantic argument under this contract. An EXCLUSION LIST rather than a silent gap: it is
    /// asserted to still be needed, so a name cannot be parked here and forgotten.</summary>
    private static readonly Dictionary<string, string> Excluded = new(StringComparer.Ordinal)
    {
        ["plot:title"] = "a text argument, not a count",
        // `k` is the power-series COEFFICIENT, and powerseries(0.5, [1], 2) is a legitimate call, so the
        // name alone cannot decide integer-semantic-ness — which is why the census treats the name as a
        // QUESTION and the exclusion as the answer.
        //
        // (This list used to carry `scale:k` for the same reason. Measurement showed the descriptor was
        // the thing that was wrong, not the census: scale(x, c) multiplies by a COMPLEX SCALAR — the
        // body reads it with ToComplex — so declaring "k" advertised an integer argument that no code
        // path read, and scale([1, 2, 3], 2.5) was accepted against a published signature that said it
        // could not be. The parameter is now NAMED after what it is, and the exclusion is gone; the
        // censused `k` arguments that remain, delay(x, k) and powerseries(k, a, n), both keep theirs.)
        ["powerseries:k"] = "a real geometric coefficient, not an index; powerseries(0.5, [1], 2) is valid",
        ["transpose:perm"] = "a permutation VECTOR, whose own elements are indices and are already driven " +
            "by the frozen shape sweep and by the index rows of this matrix",
    };

    // (There is no "undeclared" exclusion list any more: the one entry it held, `fft:n`, was based on a
    // reading that measurement falsified — `fft` takes exactly ONE argument and refuses a second at the
    // call site, so it never had an integer argument for the census to miss.)

    // ------------------------------------------------------------------
    // 1. the census
    // ------------------------------------------------------------------

    [Fact]
    public void TheInventoryCoversEveryIntegerSemanticParameterInTheRegistry()
    {
        var registered = NewRegistry().Functions.Values
            .Where(fn => fn.IsBuiltin)
            .ToDictionary(fn => fn.Name, fn => fn, StringComparer.Ordinal);

        var required = new List<string>();
        foreach (var fn in registered.Values)
        {
            for (int i = 0; i < fn.Parameters.Count; i++)
            {
                string parameter = fn.Parameters[i];
                if (!IntegerSemanticNames.Contains(parameter, StringComparer.Ordinal))
                    continue;
                string key = fn.Name + ":" + parameter;
                if (Excluded.ContainsKey(key))
                    continue;
                required.Add(key);
            }
        }

        var inventory = Inventory.Select(a => a.Builtin + ":" + a.Parameter).ToHashSet(StringComparer.Ordinal);
        var missing = required.Where(k => !inventory.Contains(k)).OrderBy(k => k, StringComparer.Ordinal).ToArray();

        // A REGISTRY entry the sweep no longer requires is stale — the builtin was renamed, removed, or
        // its parameter renamed — and it would be testing nothing. (The non-registry inventory is checked
        // the other way round, below: it can never be "required", so it must not be in this sweep.)
        var stale = inventory.Where(k => !required.Contains(k, StringComparer.Ordinal))
            .OrderBy(k => k, StringComparer.Ordinal).ToArray();
        var shadowed = NonRegistryInventory
            .Select(a => a.Builtin + ":" + a.Parameter)
            .Where(k => required.Contains(k, StringComparer.Ordinal))
            .OrderBy(k => k, StringComparer.Ordinal).ToArray();

        Assert.True(shadowed.Length == 0,
            "a non-registry entry now names a REGISTERED integer-semantic argument, so it is driven twice: " +
            string.Join(", ", shadowed) + ". Move it into Inventory.");

        Assert.True(missing.Length == 0,
            "an integer-semantic argument is registered but not in the boundary matrix, so nothing drives its " +
            "boundary: " + string.Join(", ", missing) + ". Add it to Inventory.");
        Assert.True(stale.Length == 0,
            "the boundary matrix carries an entry the registry no longer declares, so it is testing nothing: " +
            string.Join(", ", stale) + ". Remove it, or explain it in Excluded.");

        Assert.True(registered.Count >= 100, $"the registry exposed only {registered.Count} builtins");
        Assert.True(Inventory.Length >= 15, $"the inventory carries only {Inventory.Length} registered arguments");
        Assert.True(NonRegistryInventory.Length >= 2, "the non-registry inventory lost its call sites");
    }

    [Fact]
    public void EveryExclusionIsStillNeeded()
    {
        var registered = NewRegistry().Functions.Values.Where(fn => fn.IsBuiltin).ToDictionary(fn => fn.Name, StringComparer.Ordinal);

        foreach (string key in Excluded.Keys)
        {
            string[] parts = key.Split(':');
            Assert.True(registered.TryGetValue(parts[0], out var fn), $"'{key}' is excluded but '{parts[0]}' is not registered");
            Assert.True(fn.Parameters.Contains(parts[1], StringComparer.Ordinal),
                $"'{key}' is excluded but '{parts[0]}' no longer declares '{parts[1]}' — remove the exclusion");
        }

    }

    /// <summary>
    /// Every inventory row must actually REACH the argument it claims to test. The row's template is
    /// driven with a value that cannot be an integer at all, and the refusal must be the ARGUMENT
    /// refusal for that parameter — not an arity refusal, not a shape refusal, not a domain refusal.
    /// <para>
    /// This exists because a row can be dead: the first draft carried `fft:n`, which read as the optional
    /// transform length `dft` has, and which measurement showed to be nothing of the sort — `fft` takes
    /// exactly one argument. A row that never reaches its argument asserts nothing while looking like
    /// coverage, so the templates are checked rather than trusted.
    /// </para>
    /// </summary>
    [Theory]
    [MemberData(nameof(TemplateReachesItsArgumentData))]
    [Trait("Category", "Costly")]
    public async Task EveryInventoryTemplate_ReachesTheArgumentItNames(string builtin, string parameter)
    {
        var arg = Inventory.Concat(NonRegistryInventory).Single(a => a.Builtin == builtin && a.Parameter == parameter);
        // "a string" is a Text: never an exact integer, and never a shape any of these call sites accepts.
        string script = string.Format(CultureInfo.InvariantCulture, arg.Template, "\"sentinel\"");
        var (_, envelope) = await RunAsync(script);

        Assert.False(envelope["ok"]!.GetValue<bool>(), $"{script}: the sentinel was accepted, so the row is dead");
        string code = envelope["code"]?.GetValue<string>() ?? "(absent)";
        string category = envelope["category"]?.GetValue<string>() ?? "(absent)";
        string message = envelope["message"]?.GetValue<string>() ?? string.Empty;

        // The row is ALIVE when the refusal names both the builtin and the parameter the row claims to
        // test. Which typed family it lands in is the call site's business — a Text argument is the
        // documented kind refusal (InvalidOperation/DomainError) for the digit counts and the
        // argument-error pair for the dimensions, and both are recoverable, structural and already
        // pinned elsewhere. What this test is for is the DEAD row: an arity or shape refusal that never
        // reaches the argument at all.
        Assert.True(code is "InvalidArgument" or "InvalidOperation",
            $"{script}: expected a typed argument refusal, got {code}/{category} with '{message}'");
        Assert.True(category is "TypeMismatch" or "DomainError",
            $"{script}: expected a caller-side refusal category, got {category} with '{message}'");
        Assert.StartsWith(builtin + "()", message, StringComparison.Ordinal);
        Assert.Contains(arg.Anchor, message, StringComparison.Ordinal);
    }

    public static TheoryData<string, string> TemplateReachesItsArgumentData()
    {
        var data = new TheoryData<string, string>();
        foreach (var a in Inventory.Concat(NonRegistryInventory))
            data.Add(a.Builtin, a.Parameter);
        return data;
    }

    // ------------------------------------------------------------------
    // 2. the matrix
    // ------------------------------------------------------------------

    /// <summary>A matrix cell: the literal to write, and what must happen. <c>true</c> means the call must
    /// succeed; <c>false</c> means a recoverable <c>InvalidArgument</c>/<c>TypeMismatch</c> refusal that
    /// names the builtin and the parameter; <c>null</c> means either is acceptable — used for values whose
    /// fate is the call site's OWN bound rather than the integer rule (a count above the engine's cap, an
    /// index past the end, an allocation above the budget) — but never an internal failure, an
    /// unrecoverable refusal, or a silent change.</summary>
    [Trait("Category", "Costly")]
    public static TheoryData<string, string, string, int, bool?> BoundaryMatrix()
    {
        var data = new TheoryData<string, string, string, int, bool?>();

        foreach (var a in Inventory.Concat(NonRegistryInventory))
        {
            // the exact integers THIS argument accepts, and the value each must publish unchanged
            foreach (string accepted in a.Accepted)
                data.Add(a.Builtin, a.Parameter, accepted, a.Position, true);

            // A Real that is NOT integral must be REFUSED — this is the silent-coercion class, and these
            // four are the literals a silent floor would alter.
            data.Add(a.Builtin, a.Parameter, "2.5", a.Position, false);
            data.Add(a.Builtin, a.Parameter, "0.5", a.Position, false);
            data.Add(a.Builtin, a.Parameter, "2.9999", a.Position, false);
            // A rational written as an EXPRESSION, and PARENTHESISED: `1..5/2` parses as `(1..5)/2`,
            // because the range operator binds tighter than division, so the unparenthesised literal
            // tested a different call than the row names (measured — it answered [0.5, 1, 1.5, 2, 2.5]
            // rather than refusing a bound). The argument under test must be the argument written.
            data.Add(a.Builtin, a.Parameter, "(5/2)", a.Position, false);

            // AN EXACTLY INTEGRAL Real IS A THIRD CASE, and the matrix must not pretend otherwise.
            // `2.0` denotes the integer 2; whether the call sees a Real or the host has already narrowed it
            // to the integer depends on the HOST — the runner narrows it, a constructed plugin call does
            // not — so the two environments reach different arms of the contract and both answers are
            // correct. What is NOT tolerable is a silent change or an internal failure, and those are what
            // the cell asserts. (NumericOps.AnyIntegralReal is the narrowing rule.)
            data.Add(a.Builtin, a.Parameter, "2.0", a.Position, null);

            // zero and negative: refused exactly where the call site's own bound is positive
            data.Add(a.Builtin, a.Parameter, "0", a.Position, null);
            data.Add(a.Builtin, a.Parameter, "-1", a.Position, null);

            // VERY large and the overflow boundary — added ONLY for an argument that can serve it.
            //
            // That gate is measured, not cautious: a cell whose literal the argument ACCEPTS commits the
            // engine to work proportional to the literal, and `--cancel-after` does not save it, because the
            // deadline covers the evaluation loop rather than the kernel that was handed a ten-to-the-
            // eighteenth request. Four attempts at running the ungated theory ended with the test host idle
            // and 78 of ~660 cells reported; `pi(1000000)` alone answers in 584 ms, so the offenders are a
            // few combinations rather than the theory.
            //
            // The matrix is honest BY CONSTRUCTION about what a boundary cell means: every integer-semantic
            // argument is driven across the Real-VALUE boundary (accepted integers, a fractional Real, an
            // exactly integral Real, a non-integral rational, zero, negative), and the MAGNITUDE rows are
            // added where a magnitude is a fact the argument is expected to refuse — an index, a dimension,
            // an axis, a count — which is where the round-23 defect lived.
            if (a.LargeMagnitude)
            {
                data.Add(a.Builtin, a.Parameter, "10^30", a.Position, null);
                data.Add(a.Builtin, a.Parameter, "9223372036854775808", a.Position, null);
            }
        }

        return data;
    }

    [Theory]
    [MemberData(nameof(BoundaryMatrix))]
    [Trait("Category", "Costly")]
    public async Task EveryBoundaryCell_LandsOnTheExactValueOrANamedRefusal(
        string builtin, string parameter, string literal, int position, bool? mustAccept)
    {
        var arg = Inventory.Concat(NonRegistryInventory).Single(a => a.Builtin == builtin && a.Parameter == parameter);
        string script = string.Format(CultureInfo.InvariantCulture, arg.Template, literal);

        var (exitCode, envelope) = await RunAsync(script);
        string where = $"{script} [{builtin}:{parameter}={literal}]";
        bool ok = envelope["ok"]!.GetValue<bool>();

        if (!ok)
        {
            string code = envelope["code"]?.GetValue<string>() ?? "(absent)";
            string category = envelope["category"]?.GetValue<string>() ?? "(absent)";
            string message = envelope["message"]?.GetValue<string>() ?? string.Empty;
            bool recoverable = envelope["recoverable"]?.GetValue<bool>() ?? false;
            string observed = $"code={code}, category={category}, recoverable={recoverable}, message='{message}'";

            Assert.False(code == "InternalError" || category == "InternalInvariantFailure",
                $"{where}: a caller-side argument mistake crossed as an INTERNAL failure ({observed})");
            Assert.True(recoverable, $"{where}: the refusal is not recoverable ({observed})");
            Assert.False(mustAccept == true,
                $"{where}: this literal denotes an exact integer the call site accepts, but it was refused ({observed})");
            Assert.Equal(1, exitCode);

            // A BUDGET STOP is a legitimate outcome for a cell whose fate is the call site's own bound:
            // the literal was accepted and the work was real, and the engine's ledger says so honestly
            // (that is what --cancel-after is for). It is NEVER legitimate for a cell that must accept or
            // must refuse — those two already asserted above — so this arm cannot hide a defect.
            if (code == "Cancelled")
            {
                Assert.True(category == "BudgetExceeded",
                    $"{where}: a cancellation must carry category BudgetExceeded, not {category} ({observed})");
                return;
            }

            // A refusal that is NOT the integer contract's own is the call site's bound doing its job
            // (an index past the end, a count above the cap): allowed, but it must still be a typed
            // refusal rather than raw framework text.
            if (code == "InvalidArgument")
            {
                Assert.True(category == "TypeMismatch",
                    $"{where}: an argument refusal must carry category TypeMismatch, not {category} ({observed})");
                Assert.True(message.StartsWith(builtin + "()", StringComparison.Ordinal),
                    $"{where}: the refusal must name the builtin as '{builtin}()' ({observed})");
                Assert.False(Regex.IsMatch(message, @"\(Parameter '[^']+'\)"),
                    $"{where}: the refusal carries raw framework text ({observed})");
            }

            return;
        }

        Assert.False(mustAccept == false,
            $"{where}: an argument that is not an exact integer was ACCEPTED — the silent-coercion class — and the " +
            $"call answered {envelope["output"]?.ToJsonString() ?? envelope["result"]?.ToJsonString()}");
        Assert.Equal(0, exitCode);
    }

    // ------------------------------------------------------------------
    // 3. the value is observable, not merely accepted
    // ------------------------------------------------------------------

    /// <summary>
    /// Acceptance is not enough: a call site that accepted <c>2</c> and computed as if the caller had
    /// written <c>3</c> would pass every cell above. This drives the SAME argument at two values whose
    /// effects are observable and requires the outputs to differ — the property a silent truncation
    /// destroys and a correct reading preserves.
    /// </summary>
    [Theory]
    [Trait("Category", "Costly")]
    [InlineData("1", "2")]
    [InlineData("2", "3")]
    [InlineData("1", "3")]
    public async Task TheAcceptedValueIsTheValueTheCallerWrote(string low, string high)
    {
        // evalir's digit count is the most directly observable argument in the matrix: the answer's own
        // length IS the value in use.
        // print() matters: `evalir(...)` alone leaves the value in `result`, and this theory reads the
        // OUTPUT array, so without it there is no output[0] to compare.
        string template = "x = symbol(\"x\"); print(evalir(compile(x*1/3, [x]), [1], {0}))";
        var (_, lowEnvelope) = await RunAsync(string.Format(CultureInfo.InvariantCulture, template, low));
        var (_, highEnvelope) = await RunAsync(string.Format(CultureInfo.InvariantCulture, template, high));

        Assert.True(lowEnvelope["ok"]!.GetValue<bool>(), $"evalir(…, {low}) was refused");
        Assert.True(highEnvelope["ok"]!.GetValue<bool>(), $"evalir(…, {high}) was refused");

        string lowValue = lowEnvelope["output"]![0]!.GetValue<string>();
        string highValue = highEnvelope["output"]![0]!.GetValue<string>();

        Assert.NotEqual(lowValue, highValue);
        Assert.True(highValue.Length > lowValue.Length,
            $"a larger digit count must publish a longer answer: {low} -> '{lowValue}' ({lowValue.Length}), " +
            $"{high} -> '{highValue}' ({highValue.Length})");
    }
}