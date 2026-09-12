using Lovelace.Abstractions;
using Lovelace.Symbolics;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;
// Lovelace.Suite.Expr is the language AST; the symbolic kernel's expression is aliased here
using SymExpr = Lovelace.Symbolics.Expr;

namespace Lovelace.Suite;

/// <summary>One structured field of a record value (name → value). Field names come from the
/// kernel records and are part of the protocol contract.</summary>
public sealed record StructuredFieldDto(string Name, StructuredValueDto Value);

/// <summary>
/// Machine-readable form of one value. Only the fields that apply to the kind are set; an absent
/// structured field is <c>Null</c> rather than an empty string. Shared by every host so an agent
/// and the IDE see identical structure.
/// </summary>
public sealed record StructuredValueDto(
    string Kind,
    string? Type = null,
    string? Value = null,
    string? Pretty = null,
    string? Canonical = null,
    string? Domain = null,
    bool? Exact = null,
    int? NodeCount = null,
    string[]? FreeSymbols = null,
    string? Re = null,
    string? Im = null,
    long[]? Shape = null,
    StructuredValueDto[]? Elements = null,
    StructuredFieldDto[]? Fields = null,
    // set only when a print budget abbreviated the rendering (never silently)
    bool? Truncated = null,
    string? TruncationReason = null,
    int? Budget = null,
    // exact rationals cross as numerator/denominator so an agent never parses "0.(3)"
    string? Numerator = null,
    string? Denominator = null);

/// <summary>
/// The one structured projection of a <see cref="Value"/>: total over the value kinds, so nothing
/// meaningful degrades to text merely because a serializer lacked a case. Hosts (the DSH runner
/// and Studio) consume this same projection — there is no second implementation.
/// </summary>
public static class StructuredProjection
{
    /// <summary>
    /// Projects one value. The nesting is measured against <see cref="InputDepth.MaxValueDepth"/>
    /// BEFORE the projection recurses (see <see cref="ValueDepth"/>): the walk below is a native
    /// recursion over the value's structure, and a value built by repetition at run time is deep in
    /// neither the source nor the parsed tree, so no input budget has seen it. The recursion itself
    /// goes through the private <see cref="Project"/>, so one projection measures once.
    /// </summary>
    public static StructuredValueDto ToStructured(Value value, Printing.PrintBudget? budget = null)
    {
        ValueDepth.EnsureWithin("value", value);
        return Project(value, budget);
    }

    private static StructuredValueDto Project(Value value, Printing.PrintBudget? budget) => value.Kind switch
    {
        ValueKind.Record => Record(value.AsRecord(), budget),
        ValueKind.Vector or ValueKind.Array => Array(value, budget),
        ValueKind.Symbolic => Symbolic(value.AsSymbolic(), budget),
        ValueKind.Complex => Complex(value.AsComplex()),
        ValueKind.Natural => new StructuredValueDto("Natural", Value: value.AsNatural().ToString(), Exact: true),
        ValueKind.Integer => new StructuredValueDto("Integer", Value: value.AsInteger().ToString(), Exact: true),
        ValueKind.Real => Real(value.AsReal()),
        ValueKind.Boolean => new StructuredValueDto("Boolean", Value: value.AsBoolean() ? "true" : "false"),
        ValueKind.Text => new StructuredValueDto("Text", Value: value.AsText()),
        // an enum carries its DECLARED type name, so "Partial" is never ambiguous between
        // SolveStatus and Completeness (alignment addendum §9.1)
        ValueKind.Enum => new StructuredValueDto("Enum",
            Type: value.AsEnum().TypeName,
            Value: value.AsEnum().Name),
        ValueKind.Domain => new StructuredValueDto("Domain", Domain: value.AsDomain().ToString().ToLowerInvariant()),
        ValueKind.Function => new StructuredValueDto("Function", Value: value.AsFunction().Name),
        // Void == an absent value: null, distinct from the empty string
        _ => new StructuredValueDto("Null"),
    };

    private static StructuredValueDto Record(RecordValue record, Printing.PrintBudget? budget) => new(
        "Record",
        Type: record.TypeName,
        Fields: record.Fields
            .Select(f => new StructuredFieldDto(f.Name, Project((Value)f.Value!, budget)))
            .ToArray());

    private static StructuredValueDto Array(Value value, Printing.PrintBudget? budget)
    {
        var elements = value.AsVector();
        var shape = value.AsArrayValue().Shape.ToArray().Select(s => (long)s).ToArray();
        return new StructuredValueDto(
            "Array",
            Type: value.Kind.ToString(),
            Shape: shape,
            Elements: elements.Select(e => Project(e, budget)).ToArray());
    }

    private static StructuredValueDto Symbolic(SymExpr e, Printing.PrintBudget? budget)
    {
        var pretty = EnforceNodeBudget(
            Printing.Print(e, new Printing.PrintOptions(Budget: budget)), budget, e.NodeCount);
        var canonical = EnforceNodeBudget(
            Printing.Print(e, new Printing.PrintOptions(Printing.PrintMode.Canonical, Budget: budget)),
            budget, e.NodeCount);
        var truncation = pretty.Truncation ?? canonical.Truncation;
        return new StructuredValueDto(
            "Symbolic",
            Pretty: pretty.Text,
            Canonical: canonical.Text,
            Domain: Domains.DomainOf(e, Exprs.Current).ToString().ToLowerInvariant(),
            Exact: e.IsExact,
            NodeCount: e.NodeCount,
            FreeSymbols: Printing.FreeSymbolNames(e).ToArray(),
            Truncated: pretty.Truncated || canonical.Truncated ? true : null,
            TruncationReason: truncation?.Reason,
            Budget: truncation?.Budget);
    }

    /// <summary>
    /// The node budget, made TOTAL. <see cref="Printing.Print"/> abbreviates only when the full
    /// rendering is longer than the character allowance the budget implies — <c>max(48, 6 ×
    /// budget)</c> characters — so a value whose node count is already above the budget but whose
    /// rendering is short (<c>x^2</c>, 3 nodes, at budget 1) came back WHOLE and reported no
    /// truncation, while a 13-node value at budget 8 truncated (A2-F19 / A4-F1). A node budget
    /// bounds nodes, so the projection completes the rule here, on the way to the wire: over budget
    /// is ALWAYS abbreviated and always says so, and the abbreviation is still a prefix of the real
    /// rendering (never a re-ordered one) terminated by " …".
    /// <para>
    /// The prefix is proportional to the fraction of the expression the budget allows, so a larger
    /// budget still returns at least as much; a rendering the kernel printer already abbreviated
    /// (the long ones) is passed through untouched, so the established cut points do not move.
    /// </para>
    /// </summary>
    private static Printing.PrintOutcome EnforceNodeBudget(Printing.PrintOutcome outcome,
        Printing.PrintBudget? budget, int nodeCount)
    {
        if (budget?.MaxNodes is not { } maxNodes || nodeCount <= maxNodes || outcome.Truncated)
            return outcome;

        string full = outcome.Text;
        if (full.Length == 0)
            return outcome;

        // maxNodes < nodeCount, so this is always a STRICT prefix: a prefix is never passed off as
        // the whole rendering.
        int allowed = (int)Math.Max(1L, (long)full.Length * maxNodes / nodeCount);
        int cut = CutAtTokenBoundary(full, allowed);
        string text = cut == 0 ? "…" : full[..cut] + " …";
        return new Printing.PrintOutcome(text, true,
            new Printing.PrintTruncation("node-budget", nodeCount, maxNodes, text));
    }

    /// <summary>Cuts at a token boundary so a number or identifier is never split mid-token —
    /// the same rule the kernel printer applies to its own abbreviations, including its fallback:
    /// the last boundary AT OR BEFORE <paramref name="cut"/> when there is one, otherwise the whole
    /// FIRST token (the boundary after it), otherwise nothing (0), because a prefix that reproduced
    /// the only token would drop nothing. Round-20 audit I, I-3 found the old fallback returning the
    /// raw cut here too: a 40-character identifier with a proportional allowance of 14 crossed as 14
    /// of its characters plus " …".</summary>
    private static int CutAtTokenBoundary(string text, int cut)
    {
        int i = Math.Min(cut, text.Length);
        while (i > 0 && IsTokenChar(text[i - 1]))
            i--;
        if (i > 0)
            return i;
        int after = Math.Min(cut, text.Length);
        while (after < text.Length && IsTokenChar(text[after]))
            after++;
        return after < text.Length ? after : 0;
    }

    /// <summary>The characters a token is made of: a number or an identifier.</summary>
    private static bool IsTokenChar(char c) => char.IsLetterOrDigit(c) || c == '.';

    /// <summary>One complex value: its two components and, when a producer clamped either of
    /// them, the SAME clamp a Real carries (see <see cref="Real"/>).</summary>
    private static StructuredValueDto Complex(Cplx value) => new(
        "Complex",
        Re: value.Re.ToString(),
        Im: value.Im.ToString(),
        Exact: RealExact(value.Re) && RealExact(value.Im),
        Truncated: value.Re.ClampNotice is null && value.Im.ClampNotice is null ? null : true,
        TruncationReason: (value.Re.ClampNotice ?? value.Im.ClampNotice)?.Reason,
        Budget: (int?)(value.Re.ClampNotice ?? value.Im.ClampNotice)?.Budget);

    /// <summary>One real value: its digits, its exactness, and — when a producer CLAMPED them — the
    /// clamp in the same three fields a bounded rendering uses (<c>truncated</c>,
    /// <c>truncationReason</c>, <c>budget</c>). The notice is the value's own provenance
    /// (<see cref="Rl.ClampNotice"/>), never a guess: <c>evalf</c>'s 1000-place cap sets it when the
    /// cap actually dropped digits, so a value the caller asked 5000 places for and got 1000 of
    /// cannot cross as if the count had been honoured (round-20 audit I, I-2). Its digits are
    /// unchanged by the notice: the same <c>value</c> and the same <c>exact</c> as before.</summary>
    private static StructuredValueDto Real(Rl value)
    {
        bool exact = RealExact(value);
        var dto = new StructuredValueDto("Real", Value: value.ToString(), Exact: exact,
            Truncated: value.ClampNotice is null ? null : true,
            TruncationReason: value.ClampNotice?.Reason,
            Budget: (int?)value.ClampNotice?.Budget);
        if (!exact)
            return dto;
        // an exact Real IS a rational: publish it as one, so 1/3 does not have to be parsed back
        // out of "0.(3)". A truncated irrational is not exact and deliberately carries neither.
        var rat = Lovelace.Symbolics.RationalReal.FromReal(value);
        return dto with
        {
            Numerator = rat.Numerator.ToString(),
            Denominator = rat.Denominator.ToString(),
        };
    }

    /// <summary>A Real is exact when the digits it carries ARE the value it came from — the
    /// provenance its own operations recorded, not a guess read off its exponent.
    /// <para>
    /// The old test here was <c>IsZero || IsPeriodic || -Exponent &lt;= 18</c>: an exponent shape.
    /// It certified an 18-digit truncation of <c>1/1009</c> as exact (its exponent is exactly −18)
    /// and called <c>1/(10^19)</c> inexact (its exponent is −19) although that quotient terminates
    /// and is carried digit for digit.  <see cref="Rl.IsExact"/> is cleared where the loss happens —
    /// the division that ran out of digits, the square root of a non-square, π/e, the Taylor series —
    /// and is propagated by every operation that consumes a truncated operand, so the answer no
    /// longer depends on how many places the value happens to occupy.</para>
    /// </summary>
    private static bool RealExact(Rl value) => value.IsExact;
}
