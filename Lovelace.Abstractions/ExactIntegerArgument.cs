namespace Lovelace.Abstractions;

/// <summary>
/// Reads an argument as the exact integer it denotes, or answers "not an exact integer".
/// <para>
/// It is a DELEGATE rather than a shared interface on purpose: <c>Lovelace.Abstractions</c> is what
/// the numeric tower is built on, so it cannot name <c>Natural</c>, <c>Integer</c> or <c>Real</c>.
/// Each layer that owns a payload channel supplies its own reader, and the RULE below stays in one
/// place, written once.
/// </para>
/// <para>
/// A reader must answer from the value's own representation and must never floor, round, truncate or
/// wrap. A value that denotes an integer too wide for a <see cref="long"/> answers
/// <see langword="false"/> — the contract then refuses it and NAMES it, which is what used to escape
/// as the raw CLR message "Value was either too large or too small for an Int64.".
/// </para>
/// <para>
/// A reader is also where the product's KIND rule lives, and that is deliberate rather than an
/// oversight: the language's count/index arguments take a <c>Natural</c> or an <c>Integer</c>, and a
/// <c>Real</c> is refused however exactly integral it is — <c>zeros(2.0)</c>,
/// <c>[1,2,3][1.0]</c> and <c>setprecision(30.0)</c> are all refused today, and
/// <c>Lovelace.Suite.Tests.InterpreterBuiltinPiTests</c> pins the rule ("digit counts must be Natural
/// or Integer"). A reader that accepted a <c>Real</c> would therefore make the contract WIDER at one
/// call site than at its neighbours, which is the drift this abstraction exists to prevent. The
/// refusal is still two-valued and machine-readable — see
/// <see cref="ArgumentContractException.NotAnInteger"/> against
/// <see cref="ArgumentContractException.NotAnIntegerKind"/> — so a caller can tell "not a number I
/// can use" from "right number, wrong spelling".
/// </para>
/// </summary>
/// <param name="payload">The argument as the owning layer received it.</param>
/// <param name="value">The exact integer, when the reader could produce one.</param>
public delegate bool ExactIntegerReader(object? payload, out long value);

/// <summary>
/// The ONE contract every integer-semantic argument in the product goes through. The rule it
/// enforces is the product's, not this file's:
/// <para>
/// <b>An argument that requires an integer must either represent an exact integer or be refused
/// explicitly. It is never silently rounded or truncated.</b>
/// </para>
/// <para>
/// The rule exists because the two helpers this replaces both FLOORED a non-integral <c>Real</c> by
/// taking its decimal text and cutting at the point — <c>r.ToString().Split('.')[0]</c> in
/// <c>Lovelace.Symbolics</c> and again in <c>Lovelace.MathIR</c> — so
/// <c>series(sin(x), x, 0, 2.5)</c> published <c>x + O(x^2)</c> and <c>evalir(k, v, 2.9)</c>
/// evaluated at 2 places, both with <c>ok: true</c> and no diagnostic. A precision or a count is a
/// claim about the answer, so altering it silently is a wrong answer that looks like a right one.
/// </para>
/// <para>
/// The two different refusals for a <c>Real</c> are the CALL SITE's to choose, because only the call
/// site knows what arrived: a non-integral <c>Real</c> (<c>2.5</c>) is
/// <see cref="ArgumentContractException.NotAnInteger"/>, and an exactly integral one (<c>2.0</c>) is
/// <see cref="ArgumentContractException.NotAnIntegerKind"/>, whose message names the integer to
/// write instead (<c>2</c>). The language's kind rule for these arguments — pinned by
/// <c>Lovelace.Suite.Tests.InterpreterBuiltinPiTests</c>, "digit counts must be Natural or
/// Integer" — is a property of the VALUE KIND, and a consumer must be able to tell the two apart
/// without parsing prose.
/// </para>
/// </summary>
public static class ExactIntegerArgument
{
    /// <summary>
    /// Returns the exact integer <paramref name="payload"/> denotes, or throws a refusal built by
    /// the CALL SITE — which owns the "not an integer" versus "right number, wrong kind" distinction
    /// — while this method owns only the rule.
    /// <para>
    /// <paramref name="read"/> is a parameter and not a shared interface because
    /// <c>Lovelace.Abstractions</c> is what the numeric tower is BUILT ON and cannot name
    /// <c>Natural</c>, <c>Integer</c> or <c>Real</c>: each layer that owns a payload channel supplies
    /// its own reader, and whether that reader accepts a <c>Real</c> is the LAYER's kind policy, not
    /// this method's.
    /// </para>
    /// </summary>
    /// <param name="payload">The builtin argument, in whatever form its owning layer received it.</param>
    /// <param name="parameter">The parameter NAME, for the refusal message.</param>
    /// <param name="bounds">The bound the call site declares. Never optional in spirit: an unbounded
    /// integer argument is how <c>setprecision(10^30)</c> and <c>series(..., order = 10^11)</c> were
    /// accepted and then could not finish.</param>
    /// <param name="read">The layer's exact-integer reader. Required — a call site that forgot one is
    /// a programmer error and is reported as such rather than silently accepting nothing.</param>
    /// <param name="refuse">Builds the refusal for a payload the reader rejected.</param>
    /// <param name="builtin">The builtin name, for the refusal message; <see langword="null"/> when
    /// the call site has no name to give.</param>
    /// <param name="position">The 1-based argument position, so the refusal can speak the protocol
    /// document's grammar (<c>argument 4 (order)</c>). <c>0</c> when the call site cannot supply one
    /// (an index inside a subscript, a range bound): the refusal still names the parameter and value.</param>
    /// <exception cref="ArgumentContractException">The argument does not denote an exact integer in
    /// <paramref name="bounds"/>.</exception>
    /// <exception cref="Exception">Whatever <paramref name="refuse"/> built, for a call site whose own
    /// contract fixes a different refusal type (the digit counts pin
    /// <see cref="InvalidOperationException"/>).</exception>
    public static long Require(
        object? payload,
        string parameter,
        Int64Range bounds,
        ExactIntegerReader? read = null,
        Func<object?, Exception>? refuse = null,
        string? builtin = null,
        int position = 0)
    {
        ExactIntegerReader reader = read
            ?? throw new InvalidOperationException(
                "ExactIntegerArgument.Require needs the owning layer's ExactIntegerReader: " +
                "Lovelace.Abstractions cannot name the numeric tower's types, so no default can exist.");

        if (!reader(payload, out long value))
            throw refuse is null ? DefaultRefusal(builtin, parameter, bounds, payload, position) : refuse(payload);
        if (!bounds.Contains(value))
            throw ArgumentContractException.OutOfBounds(builtin, parameter, bounds, value, position);
        return value;
    }

    /// <summary>The refusal a call site with no reader-specific wording gets: the two <c>Real</c>
    /// cases kept apart (an exactly integral <c>Real</c> names the integer to write instead),
    /// everything else described by its payload.</summary>
    private static ArgumentContractException DefaultRefusal(
        string? builtin, string parameter, Int64Range bounds, object? payload, int position)
    {
        // A Real the reader REJECTED is reported as the wrong KIND when its digits are integral, and
        // as not-an-integer otherwise. The payload is inspected structurally: an exactly integral Real
        // is one whose own integer denotation fits the range of a long, which is a property of the
        // VALUE and is answered by the value.
        string describe = ArgumentContractException.DescribePayload(payload);
        return ArgumentContractException.NotAnInteger(builtin, parameter, bounds, describe, position);
    }

    /// <summary>
    /// The non-throwing probe half of the contract, for validation and for property tests. Whether a
    /// value IS an integer is decided by the reader, never by string inspection.
    /// </summary>
    public static bool TryGetExact(object? payload, ExactIntegerReader read, out long value) =>
        read(payload, out value);
}
