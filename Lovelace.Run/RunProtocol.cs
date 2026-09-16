using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Lovelace.Suite;

namespace Lovelace.Run;

// ---------------------------------------------------------------------------
// Structured value projection lives in Lovelace.Suite.StructuredProjection: ONE
// implementation shared by this runner and Studio, so an agent and the IDE see
// identical structure.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// JSON envelope DTOs. These replace the anonymous types used previously, which
// the reflection-based serializer cannot serialize under Native AOT.
// ---------------------------------------------------------------------------
/// <summary>One variable in scope. <c>structured</c> is the SAME projection the result carries
/// (<see cref="StructuredProjection"/>), so a variable's meaning is recoverable from structure
/// instead of from its human Display string (A2-F24): an agent reads
/// <c>variables[0].structured.canonical</c>, never <c>variables[0].display</c>. It is total — every
/// value kind a script can leave in scope projects to a structured form.</summary>
internal sealed record VariableDto(string Name, string Kind, string Display, StructuredValueDto Structured);

/// <summary>A duration as a unit-scaled value plus its unit, so an agent never parses a suffix.</summary>
internal sealed record DurationDto(double Value, string Unit);

/// <summary>One timed top-level statement: its elapsed time as a structured duration, the kind of
/// the value it produced, and whether it wrote print output. <c>Position</c> is the ZERO-BASED OFFSET
/// OF THE STATEMENT IN THE CALLER'S SCRIPT — the text handed to <c>--eval</c>/<c>--file</c>/<c>--stdin</c>,
/// whatever its line endings (<c>docs/symbolics/dsh-protocol.md:153-155</c>). The engine itself
/// indexes into the semicolon-joined text it is handed; <see cref="ScriptPositions"/> translates, so a
/// Windows (CRLF) script reports the offsets a consumer can slice the caller's text with.</summary>
internal sealed record TimingDto(int Position, DurationDto Elapsed, string ResultKind, bool HasOutput);
/// <summary>One entry of the builtin registry, carrying the arity metadata the call-site
/// validator computes the arity contract from (A2-F15): <c>parameters</c> is the declared
/// signature, <c>parameterCount</c> its length, <c>minArity</c> the declared LOWER bound (resolved —
/// a definition that declares no shorter form publishes its parameter count, never the internal
/// "exactly the declared count" sentinel), and <c>variadic</c> says the last parameter may repeat,
/// which removes the upper bound. An accepted call therefore satisfies
/// <c>minArity &lt;= n &amp;&amp; (variadic || n &lt;= parameterCount)</c> with no error message parsed.
/// <para>
/// The registry is also the product's DISCOVERY surface, so each entry carries the descriptor metadata an
/// agent needs to choose a function without guessing a name and without reading prose: the category, the
/// one-line summary, the declared return kind, the see-also names, and the executable examples. Every one
/// of them comes from the SAME <c>BuiltinDescriptor</c> the help surface and the Studio autocomplete read,
/// so the wire cannot drift from the documentation (ADV-08). The fields are additive: the six names and
/// meanings recorded in earlier cycles (name, parameters, minArity, parameterCount, variadic, builtin) are
/// unchanged and stay first in this order, and <c>plugin</c> keeps its place after them.
/// </para>
/// <para>
/// <c>plugin</c>, <c>summary</c>, <c>returnKind</c>, <c>related</c> and <c>examples</c> are OMITTED when the
/// registry has nothing to say — never sent as an empty array or a placeholder string — so "no summary is
/// registered" and "the summary is empty" cannot look alike to a consumer.
/// </para></summary>
internal sealed record FunctionDto(string Name, string[] Parameters, int MinArity, int ParameterCount,
    bool Variadic, bool Builtin, string? Plugin, string Category, string? Summary, string? ReturnKind,
    string[]? Related, string[]? Examples);

/// <summary>
/// A FILE an evaluation committed, published on BOTH the success and the failure path (ADV-09).
/// <para>
/// The invariant this exists for: a statement that RETURNED wrote its file, so the file is committed state
/// — and a failure in a LATER statement cannot un-write it. Before this, `plot([1, 2, 3]); det(1)` exited 1
/// with an envelope carrying no plot block and no path while <c>plot.svg</c> sat on disk (round-23 audit N,
/// N-5), so the artifact existed and nothing machine-readable said so; a consumer had to know the plot
/// directory convention and go looking.
/// </para>
/// <para>
/// <c>kind</c> names what produced the file rather than what it contains, because that is what a consumer
/// branches on when more than one builtin can write: today only <c>plot</c>. <c>title</c> is the label the
/// script gave it, when it gave one, and <c>bytes</c> is the committed file's size — read from the file
/// itself, and <see langword="null"/> when it could not be read, so a size is never invented. The array is
/// present and EMPTY on a run that wrote nothing, never absent: "nothing was written" is an answer.
/// </para>
/// </summary>
internal sealed record ArtifactDto(string Kind, string Path, string? Title, long? Bytes);
/// <summary>
/// The CALL-SITE signature an argument failure was measured against, so a consumer repairs the call from
/// structure instead of from the sentence (ADV-07). The types the runner already classified carry every one
/// of these fields — BuiltinArityException publishes the builtin, both counts and the variadic flag, and
/// BuiltinShapeException publishes the position, what that position required and the kind that arrived —
/// and the runner used to throw all of it away by forwarding ex.Message alone.
/// <para>
/// function, parameters, expectedMin, expectedMax, variadic, actual and position describe the SIGNATURE;
/// parameter, expectedShape and arrivedKind describe the ONE argument that was wrong. Fields are omitted
/// when the failure does not have them, and a failure that is not a call-site refusal carries no signature
/// at all — which is how a consumer tells a malformed CALL from a mathematical refusal.
/// </para>
/// <para>
/// The counts agree with the registry's own functions[] entry for the same name by construction: both come
/// from FunctionDefinition.DeclaredArity, and the signature sweep asserts that over every builtin.
/// </para>
/// </summary>
internal sealed record CallSignatureDto(
    string Function,
    string[]? Parameters = null,
    int? ExpectedMin = null,
    int? ExpectedMax = null,
    bool? Variadic = null,
    int? Actual = null,
    // the 1-based position of the argument the failure is about; 0 when the call site could not name one
    int? Position = null,
    string? Parameter = null,
    string? ExpectedShape = null,
    string? ArrivedKind = null);

/// <summary>
/// The answer to a REGISTRY QUERY — <c>--functions [pattern]</c> and <c>--function &lt;name&gt;</c> — which
/// is a read of the live registry and NOT an evaluation: no script is parsed, no statement runs, no
/// statement timing exists, and the revision is unchanged.
/// <para>
/// It is bounded and it says so. <c>total</c> is how many entries matched the pattern, <c>returned</c> how
/// many the page carries, and <c>truncated</c> is true exactly when the page is short — so an agent can
/// tell a complete answer from a page without comparing array lengths against a limit it guessed. The
/// order is the registry's own (ordinal by name), so two queries over one registry never disagree.
/// </para>
/// </summary>
/// <summary>
/// One entry of a <c>--functions [pattern]</c> LISTING: the name and the arity metadata, and nothing else.
/// <para>
/// The listing is the cheap surface — the question it answers is "does this runtime have something like
/// what I am reaching for, and how is it called" — so it deliberately omits summaries, examples and
/// see-also lists. Those are what <c>--function &lt;name&gt;</c> is for, and keeping them out is what makes a
/// listing a few hundred bytes instead of a few kilobytes (measured: the full registry is ~15 KB for 123
/// builtins, of which the names and arity are a small fraction).
/// </para>
/// </summary>
internal sealed record RegistryFunctionDto(string Name, string[] Parameters, int MinArity,
    int ParameterCount, bool Variadic, string Category);

/// <summary>
/// The answer to a REGISTRY QUERY — <c>--functions [pattern]</c> and <c>--function &lt;name&gt;</c> — which
/// is a read of the live registry and NOT an evaluation: no script is parsed, no statement runs, no
/// statement timing exists, and the revision is unchanged.
/// <para>
/// It is bounded and it says so. <c>total</c> is how many entries matched, <c>returned</c> how many the page
/// carries, and <c>truncated</c> is true exactly when the page is short — so an agent can tell a complete
/// answer from a page without comparing array lengths against a limit it guessed. The order is the
/// registry's own (ordinal by name), so two queries over one registry never disagree.
/// </para>
/// <para>
/// Exactly ONE of the two payloads is present: <c>functions</c> (the listing) or <c>function</c> (the single
/// full entry). An unknown NAME is not an empty success — it is the documented <c>UnknownFunction</c>
/// refusal, with the registry's suggestions, exactly as a call to that name would produce.
/// </para>
/// </summary>
internal sealed record RegistryQueryDto(
    int ProtocolVersion,
    string SymbolicFormatVersion,
    int MathIrVersion,
    bool Ok,
    // the pattern as it was understood; absent for --function, which names one entry instead
    string? Pattern,
    RegistryFunctionDto[]? Functions,
    FunctionDto? Function,
    int Total,
    int Returned,
    bool Truncated);
internal sealed record PlotDto(string Path, string Title, string Svg);
internal sealed record ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured);
/// <summary>One entry of the error envelope's <c>diagnostics</c> array: the parser's source-position
/// form (<c>docs/symbolics/dsh-protocol.md:225-228</c>). <c>Position</c> is the zero-based offset of the
/// failing place in the CALLER's script, and <c>Line</c>/<c>Column</c> are its 1-based line and column
/// there (CRLF, CR and LF each end a line once). A runtime failure names the statement that was
/// running when it threw; a lexer/parser refusal names the token the engine refused. All four fields
/// are unchanged in NAME and COUNT from the version-1 contract.</summary>
internal sealed record DiagnosticDto(string Message, int Position, int Line, int Column);

/// <summary>The cancellation-budget ledger, published whenever <c>--cancel-after</c> was given (and
/// ABSENT — never a placeholder — when it was not, so an un-budgeted envelope is byte-for-byte what
/// it always was). Two independent facts, both machine-readable:
/// <list type="bullet">
/// <item><c>stopped</c> — the deadline actually stopped this evaluation: the envelope is a failure
/// with <c>code</c> <c>Cancelled</c> and <c>category</c> <c>BudgetExceeded</c>, carrying the partial
/// state. False means the evaluation ended on its own (it completed, or it failed for an unrelated
/// reason) and the deadline never got to stop it.</item>
/// <item><c>exceeded</c> — the evaluation consumed more than its budget, with <c>excessMs</c> saying
/// by how much. This is the fact an agent must never have to infer: <c>stopped=false</c> with
/// <c>exceeded=true</c> is a DROPPED deadline — before the kernels polled, <c>sum(1..10000000)</c>
/// under a 1 ms budget reported <c>ok</c> after 3368 ms with nothing in the envelope saying so.
/// <c>stopped=true</c> with <c>exceeded=true</c> is a deadline that was observed LATE; the excess is
/// the stop latency (kernel poll stride plus, on a small budget, the OS timer granularity).</item>
/// </list>
/// <c>elapsedMs</c> is the SAME wall time the envelope publishes as <c>elapsedTime</c>, in
/// milliseconds, so budget and elapsed are directly comparable: <c>excessMs = max(0, elapsedMs -
/// budgetMs)</c>. A run that overruns still reports the outcome it actually reached (<c>ok</c> plus
/// the result, or the error it hit); the ledger says the deadline did not hold.</summary>
internal sealed record CancellationDto(
    int BudgetMs, double ElapsedMs, bool Stopped, bool Exceeded, double ExcessMs);

internal sealed record RunEnvelopeDto(
    int ProtocolVersion,
    string SymbolicFormatVersion,
    int MathIrVersion,
    bool Ok,
    long Revision,
    ResultDto? Result,
    string[] Output,
    VariableDto[] Variables,
    FunctionDto[] Functions,
    PlotDto? Plot,
    string Elapsed,
    DurationDto ElapsedTime,
    TimingDto[] Timings,
    // present only when --cancel-after was given: the deadline verdict (see CancellationDto)
    CancellationDto? Cancellation = null,
    // present only when the deadline was overrun: one diagnostic naming the overrun, so a consumer
    // that reads only the diagnostics array still learns that the budget was dropped
    DiagnosticDto[]? Diagnostics = null,
    // every file this evaluation committed (ADV-09). Present and EMPTY when the run wrote nothing, so a
    // consumer reads one array instead of inferring artifacts from the plot block.
    ArtifactDto[]? Artifacts = null);
internal sealed record RunErrorDto(
    int ProtocolVersion,
    string SymbolicFormatVersion,
    int MathIrVersion,
    bool Ok,
    string Code,
    string Category,
    string Message,
    bool Recoverable,
    DiagnosticDto[] Diagnostics,
    // the SAME structural duration pair a success envelope carries: the human string next to the
    // unit-scaled {value, unit}, produced by the same unit selector, so no consumer parses a suffix
    string Elapsed,
    DurationDto ElapsedTime,
    // one entry per top-level statement that ran (empty when none did), exactly as on success
    TimingDto[] Timings,
    // The SAME top-level output array the success envelope carries, holding everything the script
    // printed before the run ended: all of it on success, the lines committed before the failure
    // here — and never a line whose print did not run. Invariant 1
    // (docs/symbolics/dsh-protocol.md:9-10) is not scoped to a successful run, so a consumer reads
    // ONE key on both paths; a failure that printed nothing carries the array empty rather than
    // absent, and a failure raised before any engine exists (an unreadable script file, an unusable
    // plot directory) carries it empty too. Nothing on this path can print.
    string[] Output,
    // present only for a cancelled evaluation: everything the engine had already committed. The
    // cancelled envelope keeps this pair byte-for-byte as published (cycle 3/4/5 recorded it); the
    // lines are the same ones Output carries, so the two cannot disagree.
    string[]? PartialOutput = null,
    VariableDto[]? PartialVariables = null,
    // present only when --cancel-after was given: the deadline verdict (see CancellationDto). The
    // diagnosis travels with the FAILED envelope too, because a run that errored after blowing its
    // budget must not look like a run that respected it.
    CancellationDto? Cancellation = null,
    // Present only when the failure is a name the engine does not register
    // (UnknownFunctionException): the registered names closest to the one that failed, any syntax
    // note that applies, and how many names the engine registers. A consumer resolves the repair
    // from these without parsing the message, and the message carries the same answer in prose for
    // a caller that only reads text. Omitted for every other failure.
    string[]? Suggestions = null,
    string[]? Hints = null,
    int? RegistrySize = null,
    // Present only when the failure IS a call-site refusal (ADV-07): the declared signature the call was
    // measured against, so an agent reads the parameter names and the arity instead of guessing them from
    // the sentence. Omitted for every other failure, which is how a consumer distinguishes a malformed
    // CALL from a mathematical refusal without parsing English.
    CallSignatureDto? Signature = null,
    // the same array the success envelope carries, and the reason it exists: a statement that returned
    // before the failure had already committed its file, so a failure cannot make it un-happen. Present
    // and EMPTY when nothing was written.
    ArtifactDto[]? Artifacts = null);

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(RunEnvelopeDto))]
[JsonSerializable(typeof(RunErrorDto))]
[JsonSerializable(typeof(ResultDto))]
[JsonSerializable(typeof(StructuredValueDto))]
[JsonSerializable(typeof(StructuredFieldDto))]
[JsonSerializable(typeof(VariableDto))]
[JsonSerializable(typeof(FunctionDto))]
[JsonSerializable(typeof(RegistryQueryDto))]
[JsonSerializable(typeof(CallSignatureDto))]
[JsonSerializable(typeof(ArtifactDto))]
[JsonSerializable(typeof(RegistryFunctionDto))]
[JsonSerializable(typeof(PlotDto))]
[JsonSerializable(typeof(DiagnosticDto))]
[JsonSerializable(typeof(CancellationDto))]
internal sealed partial class RunJsonContext : JsonSerializerContext
{
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, WriteIndented = true,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(RunEnvelopeDto))]
internal sealed partial class RunJsonPrettyContext : JsonSerializerContext
{
}
