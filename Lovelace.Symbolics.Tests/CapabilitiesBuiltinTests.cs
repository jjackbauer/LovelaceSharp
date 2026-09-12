using System.Text.Json.Nodes;
using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Xunit;
using Xunit.Abstractions;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Round 22: the runtime's real-only limits must be LEARNABLE FROM STRUCTURE. <c>capabilities()</c>
/// publishes the supported/rejected domains and the known-unsupported operation classes, each
/// carrying the exact error code and <see cref="ErrorCategory"/> that the live call produces.
/// <para>
/// Round 10 (goal cycle 4) turned this from a claim into a checked one. An adversarial audit
/// (docs/goal-cycle-4/round-09/audit-P2P6-wire.md, part 6(b)) falsified the old "exhaustive over the
/// plugin's message-stable classes" reading by exhibiting EIGHT refused operations the record did
/// not list. The tests below now assert every advertised entry against the live call, assert WHICH
/// field carries the refusal (the error envelope or a result record's diagnostics), assert the
/// integration refusal is typed and structured, and keep the <c>BestEffort</c> verdict falsifiable
/// by driving genuine refusals the list does not carry.
/// </para>
/// </summary>
public class CapabilitiesBuiltinTests
{
    private readonly ITestOutputHelper _output;

    public CapabilitiesBuiltinTests(ITestOutputHelper output) => _output = output;

    /// <summary>The marker an <c>operation_class</c> carries when the refusal rides in a RESULT
    /// RECORD's <c>diagnostics</c> while the call succeeds (exit 0) instead of being the error
    /// envelope. <see cref="AdvertisedCodesAndCategories_MatchTheLiveEnvelope"/> derives the
    /// expected carrier from this suffix, so the wording of the class and the assertion the test
    /// makes can never drift apart.</summary>
    private const string RecordDiagnosticSuffix = "-in-record-diagnostics";

    /// <summary>The status members a result record uses to say "I did not answer". A record-carried
    /// refusal must report one of these, so the refusal is not only present in <c>diagnostics</c>
    /// but also declared by the record's own authoritative status field.</summary>
    private static readonly string[] RecordRefusalStatusMembers =
        { "Unevaluated", "Partial", "Failed", "BudgetExceeded", "Unsatisfiable" };

    /// <summary>Every class the record must publish, in the order the audit's eight findings and the
    /// four pre-round-10 entries account for. Pinning the WHOLE set (not a sample) is the point of
    /// round 10: the statement is only as good as the enumeration it makes, and a change to that
    /// enumeration has to be a deliberate edit here.</summary>
    private static readonly string[] ExpectedOperationClasses =
    {
        // round 11 (audit-2 F1) NARROWED this class. It used to be "pow.non-integer-exponent",
        // which — read together with the kernel's own message — claimed every non-integer exponent
        // while (-4)^(1/2) answers 2*i and 0^(1/2) answers 0. The message is a byte-for-byte
        // transcription of the kernel's text and cannot be narrowed, so the CLASS ID now names the
        // refused shape and the entry's `scope` field names the supported counterexamples.
        "pow.non-integer-exponent-of-a-positive-base",
        "pow.negative-base-unrepresentable-exponent",
        "solve.unsupported-domain",
        "rootof.complex-algebraic",
        // audit P6b-1: a symbolic limit that cannot be determined (record-carried)
        "limit.unevaluated-in-record-diagnostics",
        // audit P6b-2/P6b-3: unsupported integration (record-carried, typed and structured by
        // round 10 — before this round the refusal had no code and an EMPTY diagnostics array)
        "integration.unevaluated-in-record-diagnostics",
        // audit P6b-4: the symbolic plot path (used to be an InternalInvariantFailure)
        "plot.symbolic-expression",
        // the same plot defect on the element axis
        "plot.symbolic-element",
        // audit P6b-5: the argument-coercion guards, one entry per refusing surface
        "solve.non-symbolic-variable",
        "solve.non-symbolic-expression",
        "diff.non-symbolic-variable",
        "integrate.non-symbolic-variable",
        "limit.non-symbolic-variable",
        // audit P6b-6
        "dsp.symbolic-element",
        // audit P6b-7
        "linsolve.non-symbolic-matrix",
        // audit P6b-8
        "fft.non-power-of-two-length",
        // round 11 (audit-2 F7): the three reachable refusal classes the statement omitted —
        // solve.unevaluated, system-solve.unevaluated and integration.no-closed-form. All three
        // ride in a record's diagnostics while the call succeeds.
        "solve.unevaluated-in-record-diagnostics",
        "system-solve.unevaluated-in-record-diagnostics",
        "integration.no-closed-form-in-record-diagnostics",
    };

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value Field(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    private static RecordValue Capabilities(SuiteEngine engine) => engine.Evaluate("capabilities()").AsRecord();

    /// <summary>The advertised <c>scope</c> text of one entry: "" when the field is the absent-field
    /// Null, the text otherwise. A null payload maps onto <c>Value.Void</c> in the object model and
    /// onto <c>{"kind":"Null"}</c> on the wire (PayloadMap.Wrap plus the Run projection), which is
    /// why the Void kind is this helper's "no scope stated" case.</summary>
    private static string ScopeOf(RecordValue record) => Field(record, "scope") switch
    {
        { Kind: ValueKind.Void } => "",
        Value v when v.Kind == ValueKind.Text => v.AsText(),
        Value other => throw new Xunit.Sdk.XunitException($"scope must be Text or the absent-field Null, got '{other.Kind}'"),
    };

    private static IReadOnlyDictionary<string, (string Code, string Category, string Message, string Trigger, string Scope)>
        AdvertisedByOperationClass(SuiteEngine engine)
    {
        var map = new Dictionary<string, (string, string, string, string, string)>(StringComparer.Ordinal);
        foreach (Value entry in Field(Capabilities(engine), "unsupported_operations").AsVector())
        {
            var record = entry.AsRecord();
            string operationClass = Field(record, "operation_class").AsText();
            // a duplicate would make the honesty loop below verify one entry and silently skip the
            // other, so it is reported here rather than lost
            if (!map.TryAdd(operationClass, (
                Field(record, "code").AsText(),
                Field(record, "category").AsEnum().Name,
                Field(record, "message").AsText(),
                Field(record, "trigger").AsText(),
                ScopeOf(record))))
            {
                throw new Xunit.Sdk.XunitException(
                    $"two advertised entries share the operation_class '{operationClass}': the honesty loop would silently drop one of them");
            }
        }
        return map;
    }

    // ------------------------------------------------------------------
    // Shape
    // ------------------------------------------------------------------

    [Fact]
    public void Capabilities_ReturnsARecordNamedCapabilitiesResult()
    {
        var engine = NewEngine();
        var value = engine.Evaluate("capabilities()");

        Assert.Equal(ValueKind.Record, value.Kind);
        Assert.Equal("CapabilitiesResult", value.AsRecord().TypeName);
    }

    private static MathDomain[] DomainField(RecordValue capabilities, string field) =>
        Field(capabilities, field).AsVector()
            .Select(d => d.Kind == ValueKind.Domain
                ? d.AsDomain()
                : throw new Xunit.Sdk.XunitException($"not a Domain value: {d.Kind}"))
            .ToArray();

    /// <summary>
    /// Audit A2-F12. The record's domain pair is scoped to the operation it constrains: as
    /// <c>supported_domains</c>/<c>unsupported_domains</c> it read as a claim about the RUNTIME, and
    /// two live counterexamples existed — <c>symbol("x", integer)</c> succeeds, and a
    /// SolutionFamily's <c>parameter_domain</c> IS the integer domain. The fields now name the
    /// solver, and this test walks both directions of that scope against the live runner:
    /// every REFUSED domain must refuse (with the advertised code/category), every ACCEPTED domain
    /// must answer with the requested domain on the record, and every domain value must stay usable
    /// by the other domain-consuming surface the name no longer speaks for.
    /// </summary>
    [Fact]
    public async Task SolveDomainStatement_NamesItsOperation_AndEveryDomainStaysUsableElsewhere()
    {
        var engine = NewEngine();
        RecordValue capabilities = Capabilities(engine);

        Assert.Equal(new[] { MathDomain.Real, MathDomain.Complex }, DomainField(capabilities, "solve_domains_accepted"));
        Assert.Equal(new[] { MathDomain.Integer, MathDomain.Rational }, DomainField(capabilities, "solve_domains_refused"));
        // the ambiguous runtime-wide claim is GONE, not merely re-valued
        Assert.DoesNotContain(capabilities.Fields, f => f.Name is "supported_domains" or "unsupported_domains");

        var refusal = AdvertisedByOperationClass(engine)["solve.unsupported-domain"];
        foreach (string domain in new[] { "integer", "rational" })
        {
            JsonNode envelope = await RunEnvelopeAsync($"x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})");
            Assert.False(envelope["ok"]!.GetValue<bool>(), $"the statement refuses {domain}, so the live call must refuse it");
            Assert.Equal(refusal.Code, envelope["code"]!.GetValue<string>());
            Assert.Equal(refusal.Category, envelope["category"]!.GetValue<string>());
            Assert.Contains($"got {domain}", envelope["message"]!.GetValue<string>());
        }

        foreach (string domain in new[] { "real", "complex" })
        {
            JsonNode envelope = await RunEnvelopeAsync($"x = symbol(\"x\"); solve(x^2 - 2 == 0, x, {domain})");
            Assert.True(envelope["ok"]!.GetValue<bool>(), $"the statement accepts {domain}, so the live call must answer");
            JsonNode structured = envelope["result"]!["structured"]!;
            Assert.Equal("SolveResult", structured["type"]!.GetValue<string>());
            Assert.Equal(domain, StructuredFields(structured)["domain"]!["domain"]!.GetValue<string>());
        }

        // the surface the renamed fields no longer speak for: every domain value works, including
        // the two the SOLVER refuses
        foreach (string domain in new[] { "real", "complex", "integer", "rational" })
        {
            JsonNode envelope = await RunEnvelopeAsync($"symbol(\"s\", {domain})");
            Assert.True(envelope["ok"]!.GetValue<bool>(),
                $"symbol(name, {domain}) is supported; the solver-scoped statement must not deny it");
        }
    }

    [Fact]
    public void UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage()
    {
        var engine = NewEngine();
        var operations = Field(Capabilities(engine), "unsupported_operations").AsVector();

        Assert.NotEmpty(operations);
        foreach (Value element in operations)
        {
            Assert.Equal(ValueKind.Record, element.Kind);
            var record = element.AsRecord();
            Assert.Equal("UnsupportedCapability", record.TypeName);

            Assert.False(string.IsNullOrWhiteSpace(Field(record, "operation_class").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "code").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "message").AsText()));
            Assert.False(string.IsNullOrWhiteSpace(Field(record, "trigger").AsText()));

            Value category = Field(record, "category");
            Assert.Equal(ValueKind.Enum, category.Kind);
            Assert.Equal(DiagnosticProjection.CategoryTypeName, category.AsEnum().TypeName);
            Assert.True(Enum.IsDefined(typeof(ErrorCategory), category.AsEnum().Name),
                $"'{category.AsEnum().Name}' is not an ErrorCategory member");

            // round 11: `scope` is part of the shape. It is TEXT when the kernel's own message is
            // wider than the refusal, and the Null of the absent-field invariant when the id and
            // the message already fix the scope — never a missing field, never "".
            Value scope = Field(record, "scope");
            Assert.True(scope.Kind is ValueKind.Text or ValueKind.Void,
                $"scope must be Text or the absent-field Null, got '{scope.Kind}'");
        }

        // The enumeration itself is the claim: EVERY class the round-09 audit found unlisted, plus
        // the four the record already carried, and nothing else. Round 03 replaced the
        // negative-base entry by TWO entries, because the live envelopes turn out to be identical
        // while the operation classes are not: a positive base with a non-integer exponent
        // (2^(1/2)) and a negative base with an exponent whose denominator is >= 3 ((-8)^(1/3)).
        // Round 10 added the twelve classes parts 6(b) found (the four pre-existing entries are
        // unchanged). A set comparison, not a sample: an entry that disappears fails here.
        var classes = operations.Select(o => Field(o.AsRecord(), "operation_class").AsText()).ToArray();
        Assert.Equal(ExpectedOperationClasses.Length, classes.Length);
        Assert.Equal(
            ExpectedOperationClasses.OrderBy(c => c, StringComparer.Ordinal),
            classes.OrderBy(c => c, StringComparer.Ordinal));

        // ... and the class round 03 MADE SUPPORTED must be gone. Advertising it would now be
        // false: the live call answers, so there is no error left to promise.
        Assert.DoesNotContain("complex.sqrt-negative", classes);
    }

    [Fact]
    public void Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort()
    {
        var engine = NewEngine();
        Value exactness = Field(Capabilities(engine), "exactness");

        Assert.Equal(ValueKind.Enum, exactness.Kind);
        Assert.Equal("CapabilitiesExactness", exactness.AsEnum().TypeName);
        Assert.Contains(exactness.AsEnum().Name, new[] { "Exhaustive", "BestEffort" });
    }

    /// <summary>
    /// <c>BestEffort</c> is a CLAIM, and this test is the evidence for it: every refusal driven
    /// here is real, live and NOT described by any advertised entry. If a later round enumerates
    /// them, this test fails and the verdict has to be re-decided on the record — the statement
    /// cannot drift into an overclaim (Exhaustive while these exist) or silently keep an
    /// underclaim it no longer needs.
    /// <para>
    /// The first three probes are audit finding F4: a caller-side argument error surfacing as
    /// <c>InternalError</c>/<c>InternalInvariantFailure</c> with a raw CLR message. That defect is
    /// REPAIRED — a scalar handed to a reduction is now a typed argument error
    /// (<c>InvalidArgument</c>/<c>TypeMismatch</c>) — so the probes below pin the repair instead of
    /// the defect. The refusals that keep the verdict at BestEffort are the parse layer and the Real
    /// type's zero base: live, unlisted, and carrying codes no advertised entry claims.
    /// </para>
    /// </summary>
    [Fact]
    public async Task ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry()
    {
        var advertised = AdvertisedByOperationClass(NewEngine());
        Assert.Equal("BestEffort", Field(Capabilities(NewEngine()), "exactness").AsEnum().Name);

        string[] unenumerated =
        {
            "mean(1);",                            // F4: raw InvalidCastException from the builtin body
            "max(1,2);",                           // F4: same
            "x = symbol(\"x\"); sum(x, 5);",       // F4: same
            "1 +;",                                // the parse layer refuses a malformed script
            "0^-1;",                               // the Real type refuses a zero base
        };

        var observed = new List<string>();
        foreach (string script in unenumerated)
        {
            JsonNode envelope = await RunEnvelopeAsync(script);
            Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' was expected to be refused by the kernel");
            string code = envelope["code"]!.GetValue<string>();
            string category = envelope["category"]!.GetValue<string>();
            string message = envelope["message"]!.GetValue<string>();
            observed.Add($"{code}/{category}");

            Assert.DoesNotContain(advertised.Values,
                e => e.Code == code && e.Category == category && e.Message == message);
        }

        // the decisive part: these refusals are live, unlisted (asserted row by row above), and at
        // least one of them is a CALLER-SIDE ARGUMENT ERROR — which must never surface as an
        // internal invariant failure (docs/symbolics/dsh-protocol.md:187-195). The first three
        // probes used to; if that regresses, this assertion names the probe list.
        Assert.Contains("InvalidArgument/TypeMismatch", observed);
        Assert.DoesNotContain("InternalError/InternalInvariantFailure", observed);
        Assert.DoesNotContain(advertised.Values, e => e.Code == "InternalError");
        _output.WriteLine("live refusals the enumeration does not carry: " + string.Join(" | ", observed));
    }

    // ------------------------------------------------------------------
    // The honesty test: advertised fields == the fields the live call produces
    // ------------------------------------------------------------------

    private static JsonNode StructuredFields(JsonNode structured)
    {
        var fields = new JsonObject();
        foreach (JsonNode? field in structured["fields"]!.AsArray())
            fields[field!["name"]!.GetValue<string>()] = field["value"]!.DeepClone();
        return fields;
    }

    private async Task<JsonNode> RunEnvelopeAsync(string script)
    {
        var stdout = new StringWriter();
        var stderr = new StringWriter();
        int exitCode = await Lovelace.Run.Runner.RunAsync(
            new[] { "--eval", script, "--json", "--omit-functions" }, stdout, stderr);
        JsonNode? envelope = JsonNode.Parse(stdout.ToString());
        Assert.True(envelope is not null, $"'{script}' produced no envelope (exit {exitCode}). stderr: {stderr}");
        return envelope!;
    }

    /// <summary>
    /// The honesty property, EXHAUSTIVE over the advertised list: every entry the record publishes
    /// is RUN through the published <c>Lovelace.Run</c> envelope, and the field that CARRIES the
    /// refusal is asserted — not just the strings. Two rules:
    /// <list type="bullet">
    /// <item>a class ending in <c>-in-record-diagnostics</c> MAKES A CLAIM: the live call must
    /// SUCCEED (exit 0, <c>ok:true</c>) with a Record result whose <c>status</c> enum is a refusal
    /// member and whose FIRST diagnostic equals the advertised code/category/message. This is the
    /// shape audit P6b-1 (limits) and P6b-2/P6b-3 (integration) have.</item>
    /// <item>every other class is verified against whichever carrier the live call uses — the error
    /// envelope (exit 1, <c>ok:false</c>) or a record diagnostic — and the carrier observed is
    /// reported. Exactly one unmarked class, the pre-round-10 <c>rootof.complex-algebraic</c>, is
    /// record-carried; its published class id is deliberately NOT renamed (consumers enumerate on
    /// it), so the marker is a guarantee the marked classes make rather than a partition the whole
    /// list is forced into.</item>
    /// </list>
    /// Round 03 replaced the four literal probes with this loop; round 10 extended it with the
    /// carrier rule and the twelve classes part 6(b) of the round-09 audit falsified. The test name
    /// is unchanged: it still asserts exactly what it always did (advertised code and category ==
    /// observed), over a longer list and now over the carrier as well.
    /// </summary>
    [Fact]
    public async Task AdvertisedCodesAndCategories_MatchTheLiveEnvelope()
    {
        var advertised = AdvertisedByOperationClass(NewEngine());
        Assert.NotEmpty(advertised);

        var failures = new List<string>();
        var verified = new List<string>();
        foreach (var (operationClass, entry) in advertised)
        {
            JsonNode envelope;
            try
            {
                envelope = await RunEnvelopeAsync(entry.Trigger);
            }
            catch (Xunit.Sdk.XunitException ex)
            {
                failures.Add($"'{operationClass}' trigger '{entry.Trigger}': {ex.Message}");
                continue;
            }

            bool markedRecordCarried = operationClass.EndsWith(RecordDiagnosticSuffix, StringComparison.Ordinal);
            bool ok = envelope["ok"]!.GetValue<bool>();
            string liveCode;
            string liveCategory;
            string liveMessage;
            string carrier;

            if (ok)
            {
                JsonNode structured = envelope["result"]!["structured"]!;
                if (!string.Equals(structured["kind"]!.GetValue<string>(), "Record", StringComparison.Ordinal))
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the live call SUCCEEDED with a {structured["kind"]!.GetValue<string>()} result and reported nothing to compare against");
                    continue;
                }

                JsonNode fields = StructuredFields(structured);
                string status = fields["status"]?["value"]?.GetValue<string>() ?? "";
                JsonArray diagnostics = fields["diagnostics"]?["elements"]?.AsArray() ?? new JsonArray();
                if (diagnostics.Count == 0)
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the live call SUCCEEDED and reported no diagnostic, so the list advertises an unsupported operation the runtime can actually perform");
                    continue;
                }

                // A class that carries the marker CLAIMS this carrier, so the claim itself is checked:
                // the record must also declare the refusal in its own authoritative status field.
                if (markedRecordCarried && !RecordRefusalStatusMembers.Contains(status, StringComparer.Ordinal))
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the class marks the refusal as record-carried, but the record's status field says '{status}', which is not a refusal member ({string.Join("/", RecordRefusalStatusMembers)})");
                    continue;
                }

                // The advertised code is looked up among the record's TOP-LEVEL diagnostics —
                // the array an agent matches on without walking nested `details` — and it must
                // appear there EXACTLY once, so "the diagnostic this class advertises" is never
                // ambiguous. (Round 11: the integration refusal publishes its class-level code
                // AND its input-specific code at the top level, which is why this is a lookup and
                // not a fixed index.)
                var carriedCodes = diagnostics
                    .Select(d => StructuredFields(d!)["code"]!["value"]!.GetValue<string>())
                    .ToArray();
                JsonNode? diagnostic = diagnostics
                    .FirstOrDefault(d => StructuredFields(d!)["code"]!["value"]!.GetValue<string>() == entry.Code);
                if (diagnostic is null)
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the record carries {carriedCodes.Length} diagnostic(s) [{string.Join(", ", carriedCodes)}] and NONE of them has the advertised code '{entry.Code}'");
                    continue;
                }
                if (carriedCodes.Count(c => c == entry.Code) != 1)
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the advertised code '{entry.Code}' is carried {carriedCodes.Count(c => c == entry.Code)} times by [{string.Join(", ", carriedCodes)}], so the diagnostic the class names is ambiguous");
                    continue;
                }

                JsonNode diagnosticFields = StructuredFields(diagnostic);
                liveCode = diagnosticFields["code"]!["value"]!.GetValue<string>();
                liveCategory = diagnosticFields["category"]!["value"]!.GetValue<string>();
                liveMessage = diagnosticFields["message"]!["value"]!.GetValue<string>();
                carrier = $"record {structured["type"]!.GetValue<string>()} status={status} diagnostics[code={entry.Code}]";
            }
            else
            {
                if (markedRecordCarried)
                {
                    failures.Add($"'{operationClass}' trigger '{entry.Trigger}': the class says the refusal rides in a result record's diagnostics, but the live call FAILED with the envelope {envelope["code"]}/{envelope["category"]}");
                    continue;
                }

                liveCode = envelope["code"]!.GetValue<string>();
                liveCategory = envelope["category"]!.GetValue<string>();
                liveMessage = envelope["message"]!.GetValue<string>();
                carrier = "error envelope";
            }

            if (liveCode != entry.Code || liveCategory != entry.Category || liveMessage != entry.Message)
            {
                failures.Add(
                    $"'{operationClass}' trigger '{entry.Trigger}': advertised {entry.Code}/{entry.Category}/\"{entry.Message}\" " +
                    $"but the live call produced {liveCode}/{liveCategory}/\"{liveMessage}\"");
                continue;
            }

            verified.Add($"{operationClass} [{carrier}]");
        }

        Assert.True(failures.Count == 0, string.Join("\n", failures));
        foreach (string row in verified)
            _output.WriteLine("live-verified " + row);
    }

    // ------------------------------------------------------------------
    // Round 11: the power-exponent boundary, in BOTH directions
    // ------------------------------------------------------------------

    /// <summary>
    /// Audit-2 F1. The statement advertised <c>pow.non-integer-exponent</c> with the kernel's own
    /// message "Non-integer exponents are not yet supported." — a pair that reads as a claim about
    /// EVERY non-integer exponent, while the live call answers an exact negative base under the
    /// principal square root. The repair is a NARROWER class (the id names the refused shape), the
    /// <c>scope</c> field that names what IS supported, and this test, which drives both directions
    /// against the live runner: every shape the two power classes refuse produces the advertised
    /// envelope, every counterexample the scope names is SUPPORTED (the advertisement no longer
    /// denies it), and the over-broad id is GONE rather than kept alongside as an alias.
    /// </summary>
    [Fact]
    public async Task PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported()
    {
        var advertised = AdvertisedByOperationClass(NewEngine());
        var positiveBase = advertised["pow.non-integer-exponent-of-a-positive-base"];
        var negativeBase = advertised["pow.negative-base-unrepresentable-exponent"];

        // the id an agent enumerates on must not stay ambiguous: the old class is REMOVED
        Assert.DoesNotContain("pow.non-integer-exponent", advertised.Keys);

        // the refusal direction: a positive base with a non-integer exponent, and a negative base
        // with any non-integer exponent other than the principal square root
        foreach (string script in new[]
                 {
                     "2^(1/2)", "4^(1/2)", "9^(1/2)", "0.5^(1/2)", "2^(1/3)", "2^(2/3)", "2^(-1/2)",
                     "(-8)^(1/3)", "(-4)^(1/4)", "(-4)^(2/3)", "(-4)^(3/2)", "(-4)^(-1/2)",
                 })
        {
            JsonNode envelope = await RunEnvelopeAsync(script);
            Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' is refused by the statement, so the live call must refuse it");
            Assert.Equal("UnsupportedOperation", envelope["code"]!.GetValue<string>());
            Assert.Equal("UnsupportedOperation", envelope["category"]!.GetValue<string>());
            Assert.Equal(positiveBase.Message, envelope["message"]!.GetValue<string>());
        }

        // the supported direction the old advertisement DENIED — the exact negative base under
        // the principal square root, and a zero base. These are the shapes the scope field names.
        foreach (var (script, expected) in new[]
                 {
                     ("(-4)^(1/2)", "2*i"), ("(-1)^(1/2)", "i"), ("(-9)^(1/2)", "3*i"),
                     ("(-2)^(1/2)", "i*sqrt(2)"), ("(-4)^0.5", "2*i"), ("(-4)^(2/4)", "2*i"),
                     ("0^(1/2)", "0"),
                 })
        {
            JsonNode envelope = await RunEnvelopeAsync(script);
            Assert.True(envelope["ok"]!.GetValue<bool>(), $"the class's scope says '{script}' is supported, so it must answer");
            Assert.Equal(expected, envelope["result"]!["display"]!.GetValue<string>());
        }

        // the correction rides on the wire, not only in this test: the two power classes must
        // carry the scope, and each must name a counterexample this test actually ran
        Assert.Contains("(-4)^(1/2)", positiveBase.Scope);
        Assert.Contains("(-4)^(1/2)", negativeBase.Scope);
        Assert.Contains("positive base", positiveBase.Scope, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("negative base", negativeBase.Scope, StringComparison.OrdinalIgnoreCase);
    }

    // ------------------------------------------------------------------
    // The two refusal classes round 10 had to make honest before advertising
    // ------------------------------------------------------------------

    /// <summary>
    /// Audit P6b-2/P6b-3. The integration refusal USED to be a status with no signal at all:
    /// <c>IntegrationResult.status = Unevaluated</c> with an EMPTY <c>diagnostics</c> array, so
    /// "did anything go wrong?" was not answerable from structure. The kernel now always supplies a
    /// reason, which crosses as a structured diagnostic. Two properties are asserted, and the
    /// second is what makes ONE advertised message truthful for the whole class: the outer message
    /// is identical for every refused integrand (class-level, so the advertisement can state it),
    /// while the input-specific reason differs per integrand and rides in <c>details</c>.
    /// </summary>
    [Fact]
    public async Task IntegrationRefusal_IsTypedStructuredAndClassLevel()
    {
        string[] integrands = { "exp(x^2)", "sin(x^2)", "1/(x^5+1)" };
        var messages = new List<string>();
        var reasons = new List<string>();

        foreach (string integrand in integrands)
        {
            JsonNode envelope = await RunEnvelopeAsync($"x = symbol(\"x\"); integrate_full({integrand}, x);");
            Assert.True(envelope["ok"]!.GetValue<bool>(), $"integrate_full({integrand}, x) must return a record, not an error envelope");

            JsonNode structured = envelope["result"]!["structured"]!;
            Assert.Equal("Record", structured["kind"]!.GetValue<string>());
            Assert.Equal("IntegrationResult", structured["type"]!.GetValue<string>());

            JsonNode fields = StructuredFields(structured);
            Assert.Equal("Unevaluated", fields["status"]!["value"]!.GetValue<string>());

            // Round 11 (audit-2 F7): the record publishes the refusal under BOTH codes at the top
            // level — the class-level integration.unevaluated FIRST (its message is the one the
            // capability statement advertises byte for byte) and the input-specific
            // integration.no-closed-form after it — so a consumer that only reads
            // diagnostics[].code finds the class the statement advertises without walking details.
            JsonArray topLevel = fields["diagnostics"]!["elements"]!.AsArray();
            Assert.Equal(2, topLevel.Count);

            JsonNode diagnostic = topLevel[0]!;
            JsonNode diagnosticFields = StructuredFields(diagnostic);
            Assert.Equal("integration.unevaluated", diagnosticFields["code"]!["value"]!.GetValue<string>());
            Assert.Equal("UnsupportedOperation", diagnosticFields["category"]!["value"]!.GetValue<string>());
            Assert.False(string.IsNullOrWhiteSpace(diagnosticFields["message"]!["value"]!.GetValue<string>()),
                "the refusal must carry a non-empty message");

            JsonArray details = diagnosticFields["details"]!["elements"]!.AsArray();
            JsonNode detail = Assert.Single(details)!;
            JsonNode detailFields = StructuredFields(detail);
            Assert.Equal("integration.no-closed-form", detailFields["code"]!["value"]!.GetValue<string>());
            Assert.Equal("UnsupportedOperation", detailFields["category"]!["value"]!.GetValue<string>());
            string reason = detailFields["message"]!["value"]!.GetValue<string>();
            Assert.False(string.IsNullOrWhiteSpace(reason), "the refusal's details must carry a non-empty reason");
            Assert.Contains("integrate(", reason);

            JsonNode specificFields = StructuredFields(topLevel[1]!);
            Assert.Equal("integration.no-closed-form", specificFields["code"]!["value"]!.GetValue<string>());
            Assert.Equal("UnsupportedOperation", specificFields["category"]!["value"]!.GetValue<string>());
            Assert.Equal(reason, specificFields["message"]!["value"]!.GetValue<string>());
            Assert.Empty(specificFields["details"]!["elements"]!.AsArray());

            messages.Add(diagnosticFields["message"]!["value"]!.GetValue<string>());
            reasons.Add(reason);
        }

        // class-level message, input-specific details
        Assert.Single(messages.Distinct());
        Assert.Equal(integrands.Length, reasons.Distinct().Count());
    }

    /// <summary>
    /// Audit P6b-4. The symbolic plot path used to die as
    /// <c>InternalError</c>/<c>InternalInvariantFailure</c> ("Specified cast is not valid.",
    /// <c>recoverable:false</c>) because the single-vector form cast its argument before checking
    /// its kind: a crash, not a refusal, and therefore not advertisable as a capability. It is now
    /// the same typed envelope as every other argument error, and the advertised entry for it is
    /// the live message. <c>plot(1)</c> is the same defect with a non-symbolic scalar, so it is
    /// driven too: the guard must refuse anything that is not a vector.
    /// </summary>
    [Fact]
    public async Task SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure()
    {
        foreach (string script in new[] { "x = symbol(\"x\"); plot(sin(x));", "plot(1);" })
        {
            JsonNode envelope = await RunEnvelopeAsync(script);
            Assert.False(envelope["ok"]!.GetValue<bool>(), $"'{script}' must be refused");
            Assert.Equal("InvalidOperation", envelope["code"]!.GetValue<string>());
            Assert.Equal("DomainError", envelope["category"]!.GetValue<string>());
            Assert.NotEqual("InternalInvariantFailure", envelope["category"]!.GetValue<string>());
            Assert.True(envelope["recoverable"]!.GetValue<bool>(), $"'{script}' must be a recoverable refusal");
            Assert.False(string.IsNullOrWhiteSpace(envelope["message"]!.GetValue<string>()));
        }

        // the element axis of the same defect: a vector whose elements are symbolic
        JsonNode elementEnvelope = await RunEnvelopeAsync("x = symbol(\"x\"); plot([x, 1, 2]);");
        Assert.False(elementEnvelope["ok"]!.GetValue<bool>());
        Assert.Equal("InvalidOperation", elementEnvelope["code"]!.GetValue<string>());
        Assert.Equal("DomainError", elementEnvelope["category"]!.GetValue<string>());

        // and the advertisement is the live message, byte for byte
        JsonNode symbolic = await RunEnvelopeAsync("x = symbol(\"x\"); plot(sin(x));");
        var advertised = AdvertisedByOperationClass(NewEngine());
        Assert.Equal(advertised["plot.symbolic-expression"].Code, symbolic["code"]!.GetValue<string>());
        Assert.Equal(advertised["plot.symbolic-expression"].Category, symbolic["category"]!.GetValue<string>());
        Assert.Equal(advertised["plot.symbolic-expression"].Message, symbolic["message"]!.GetValue<string>());
    }

    // ------------------------------------------------------------------
    // Registration / discoverability
    // ------------------------------------------------------------------

    [Fact]
    public void Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata()
    {
        var engine = NewEngine();
        var snapshot = engine.CaptureState();

        var builtins = snapshot.Functions.Values.Where(f => f.IsBuiltin).ToArray();
        int count = builtins.Length;
        _output.WriteLine($"builtin count with capabilities(): {count}");

        Assert.Single(builtins, f => f.Name == "capabilities");
        Assert.True(engine.Functions.TryGetValue("capabilities", out var fn), "capabilities() is not registered");
        Assert.Equal("Lovelace.Symbolics", fn!.PluginName);

        Assert.True(engine.InterpreterBuiltinDescriptors.TryGetValue("capabilities", out var descriptor),
            "capabilities() has no BuiltinDescriptor");
        Assert.Equal(BuiltinCategories.Introspection, descriptor!.Category);
        Assert.Empty(descriptor.Parameters);
        Assert.False(string.IsNullOrWhiteSpace(descriptor.Summary));
        Assert.NotEmpty(descriptor.Examples);
        Assert.NotEmpty(descriptor.Related);
        Assert.Equal("CapabilitiesResult", descriptor.ReturnKind);

        string? help = engine.Help.Function("capabilities");
        Assert.NotNull(help);
        Assert.DoesNotContain("(no summary registered)", help);
        Assert.DoesNotContain("Returns: Value", help);
    }
}
