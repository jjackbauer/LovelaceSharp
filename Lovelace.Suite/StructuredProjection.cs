using Lovelace.Abstractions;
using Lovelace.Symbolics;
using Rl = global::Lovelace.Real.Real;
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
    public static StructuredValueDto ToStructured(Value value, Printing.PrintBudget? budget = null) => value.Kind switch
    {
        ValueKind.Record => Record(value.AsRecord(), budget),
        ValueKind.Vector or ValueKind.Array => Array(value, budget),
        ValueKind.Symbolic => Symbolic(value.AsSymbolic(), budget),
        ValueKind.Complex => new StructuredValueDto("Complex",
            Re: value.AsComplex().Re.ToString(),
            Im: value.AsComplex().Im.ToString(),
            Exact: ExactOf(value)),
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
            .Select(f => new StructuredFieldDto(f.Name, ToStructured((Value)f.Value!, budget)))
            .ToArray());

    private static StructuredValueDto Array(Value value, Printing.PrintBudget? budget)
    {
        var elements = value.AsVector();
        var shape = value.AsArrayValue().Shape.ToArray().Select(s => (long)s).ToArray();
        return new StructuredValueDto(
            "Array",
            Type: value.Kind.ToString(),
            Shape: shape,
            Elements: elements.Select(e => ToStructured(e, budget)).ToArray());
    }

    private static StructuredValueDto Symbolic(SymExpr e, Printing.PrintBudget? budget)
    {
        var pretty = Printing.Print(e, new Printing.PrintOptions(Budget: budget));
        var canonical = Printing.Print(e,
            new Printing.PrintOptions(Printing.PrintMode.Canonical, Budget: budget));
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

    private static StructuredValueDto Real(Rl value)
    {
        bool exact = RealExact(value);
        var dto = new StructuredValueDto("Real", Value: value.ToString(), Exact: exact);
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

    private static bool ExactOf(Value value) => value.Kind switch
    {
        ValueKind.Complex => RealExact(value.AsComplex().Re) && RealExact(value.AsComplex().Im),
        _ => true,
    };
}
