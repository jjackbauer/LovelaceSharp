namespace Lovelace.Abstractions;

/// <summary>
/// What an integer-semantic argument failed to be. Three distinguishable cases, because a machine
/// consumer's repair is different for each: a value that is not an integer at all cannot be fixed by
/// rewriting it; a value that IS an integer but arrived in the wrong KIND is fixed by writing it as
/// an integer; and a correctly-formed integer outside the call site's bound is fixed by choosing a
/// different one.
/// </summary>
public enum ArgumentContractFailure
{
    /// <summary>The argument does not denote an exact integer (a fractional <c>Real</c>, a text, a boolean…).</summary>
    NotAnInteger,

    /// <summary>The argument denotes an exact integer, but it arrived as the wrong KIND — a
    /// <c>Real</c> where the builtin takes a <c>Natural</c>/<c>Integer</c>.</summary>
    NotAnIntegerKind,

    /// <summary>The argument is an exact integer and the call site does not accept its magnitude.</summary>
    OutOfBounds,

    /// <summary>The argument denotes an exact integer too wide for the architecture to carry.</summary>
    TooWide,
}

/// <summary>
/// The one refusal an integer-semantic argument can produce.
/// <para>
/// It derives from <see cref="ArgumentException"/> on purpose: the runner's own taxonomy already
/// maps that type to the recoverable <c>InvalidArgument</c>/<c>TypeMismatch</c> pair
/// (<c>Lovelace.Run/Runner.cs</c>), the documented shape for a caller-side argument error and the
/// pair <c>pi(0)</c>/<c>e(0)</c> already produce. No eighth <c>ErrorCategory</c> is added — that
/// taxonomy is frozen at seven members and pinned by
/// <c>Lovelace.Symbolics.Tests.DiagnosticContractTests</c>.
/// </para>
/// <para>
/// Every field a consumer needs is a property, so no consumer ever parses the message:
/// <see cref="Builtin"/>, <see cref="Parameter"/>, <see cref="Bounds"/>, <see cref="Received"/>,
/// <see cref="Failure"/> and <see cref="ReceivedText"/>. <see cref="Received"/> is
/// <see langword="null"/> when the argument did not denote an <see cref="long"/> at all — which is
/// itself the answer, and is the case that used to escape as the raw CLR message "Value was either
/// too large or too small for an Int64." with no number in it.
/// </para>
/// </summary>
public sealed class ArgumentContractException : ArgumentException
{
    private ArgumentContractException(
        string message,
        ArgumentContractFailure failure,
        string? builtin,
        string parameter,
        int position,
        Int64Range bounds,
        long? received,
        string? receivedText)
        : base(message)
    {
        Failure = failure;
        Builtin = builtin;
        Parameter = parameter;
        Position = position;
        Bounds = bounds;
        Received = received;
        ReceivedText = receivedText;
    }

    /// <summary>Which of the four argument problems this is.</summary>
    public ArgumentContractFailure Failure { get; }

    /// <summary>The builtin whose argument contract was violated, when the call site knows its name.</summary>
    public string? Builtin { get; }

    /// <summary>The parameter NAME: what the builtin's own descriptor publishes for that position.</summary>
    public string Parameter { get; }

    /// <summary>The 1-based argument POSITION, when the call site knows it. It is what the protocol
    /// document's argument-refusal grammar names (`<c>series(): argument 4 (order) must be …</c>`);
    /// `0` means the call site could not supply one, and the message then names the parameter
    /// alone.</summary>
    public int Position { get; }

    /// <summary>The bound the call site declared.</summary>
    public Int64Range Bounds { get; }

    /// <summary>The exact value the caller supplied, or <see langword="null"/> when it did not denote
    /// an <see cref="long"/>.</summary>
    public long? Received { get; }

    /// <summary>The caller's value as text when it did not denote an <see cref="long"/>; otherwise
    /// <see langword="null"/>.</summary>
    public string? ReceivedText { get; }

    /// <summary>The refusal for a value that does not denote an exact integer. <paramref name="received"/>
    /// is the payload's own short text (the caller's vocabulary), not a type name, whenever the call
    /// site has one.</summary>
    public static ArgumentContractException NotAnInteger(
        string? builtin, string parameter, Int64Range bounds, string received, int position = 0)
    {
        string head = Head(builtin, parameter, position);
        return new ArgumentContractException(
            $"{head} must be a Natural or Integer that denotes an exact integer; got {received}.",
            ArgumentContractFailure.NotAnInteger, builtin, parameter, position, bounds, null, null);
    }

    /// <summary>The refusal for an exactly integral value that arrived in the wrong KIND. The message
    /// names the integer to write instead, so the repair is in the refusal itself.</summary>
    public static ArgumentContractException NotAnIntegerKind(
        string? builtin, string parameter, Int64Range bounds, string exactIntegerText, int position = 0)
    {
        string head = Head(builtin, parameter, position);
        return new ArgumentContractException(
            $"{head} must be a Natural or Integer; got the Real {exactIntegerText}, which is integral — write it as {exactIntegerText}.",
            ArgumentContractFailure.NotAnIntegerKind, builtin, parameter, position, bounds, null, null);
    }

    /// <summary>The refusal for a correctly-formed integer outside the call site's bound.</summary>
    public static ArgumentContractException OutOfBounds(
        string? builtin, string parameter, Int64Range bounds, long received, int position = 0) =>
        new($"{Head(builtin, parameter, position)} must be a Natural or Integer {bounds.Describe()}; got {received}.",
            ArgumentContractFailure.OutOfBounds, builtin, parameter, position, bounds, received, null);

    /// <summary>The refusal for an exact integer wider than the architecture carries. The value's own
    /// text is the payload: it is the only faithful way to name a number wider than the type that
    /// would have reported it, and it is what the round-23 refusals already publish.</summary>
    public static ArgumentContractException TooWide(
        string? builtin, string parameter, Int64Range bounds, string valueText, int position = 0) =>
        new($"{Head(builtin, parameter, position)} must be a Natural or Integer {bounds.Describe()}; got {valueText}.",
            ArgumentContractFailure.TooWide, builtin, parameter, position, bounds, null, valueText);

    /// <summary>
    /// The refusal for a SHAPE argument whose dimensions do not describe the value they are reshaping —
    /// a caller-side argument error like the others, but one whose repair is "make the product agree"
    /// rather than "use a different number", so it names both counts.
    /// <para>
    /// It exists because the array kernel's own refusal is a framework <see cref="ArgumentException"/>
    /// carrying a <c>(Parameter 'shape')</c> tail and no builtin name — raw framework text on the wire,
    /// which the round-23 audit recorded as the last two rows of that family
    /// (<c>reshape([1,2,3,4], 0)</c> and <c>reshape([1,2,3,4], 100000000000)</c>).
    /// </para>
    /// </summary>
    public static ArgumentContractException ShapeElementCount(
        string? builtin, string parameter, int position, string shape, long requested, long actual) =>
        new($"{Head(builtin, parameter, position)} describes {requested} element(s), but the value it reshapes holds {actual}; " +
            $"the element count must agree.",
            ArgumentContractFailure.OutOfBounds, builtin, parameter, position, Int64Range.NonNegative, requested, shape);

    /// <summary>
    /// The refusal for an AXIS that does not name one of the value's own axes. The bound is the RANK,
    /// so only the call site can state it; the array kernel's own guard refuses with a framework
    /// <see cref="ArgumentOutOfRangeException"/> whose message is
    /// <c>Axis -1 is out of range for rank 1. (Parameter 'axis')</c> — raw framework text with no
    /// builtin name (round-23 audit Q, Q-4 for <c>reshape</c>; the reductions kept it until the
    /// ADV-03 boundary matrix drove <c>prod([1, 2, 3], -1)</c>).
    /// </summary>
    public static ArgumentContractException AxisOutOfRange(
        string? builtin, int position, long axis, int rank)
    {
        string axes = rank == 1 ? "the only axis (0)" : $"one of the value's {rank} axes, 0 to {rank - 1}";
        return new($"{Head(builtin, "axis", position)} must be {axes}; got {axis}.",
            ArgumentContractFailure.OutOfBounds, builtin, "axis", position, Int64Range.NonNegative, axis, null);
    }
    /// <summary>What arrived, in the caller's vocabulary: the runtime kind for the non-kind kinds,
    /// the value itself where its text is short and unambiguous.</summary>
    public static string DescribePayload(object? payload) => payload switch
    {
        null => "nothing",
        string s => "Text \"" + s + "\"",
        bool b => b ? "the Boolean true" : "the Boolean false",
        _ => payload.GetType().Name,
    };

    /// <summary>
    /// The message head, in the ONE grammar the protocol document fixes for an argument refusal
    /// (<c>dsh-protocol.md</c>): the builtin, the 1-based argument POSITION, and the parameter name
    /// the builtin's own descriptor publishes for it —
    /// <c>series(): argument 4 (order) must be …</c>. A call site that cannot name a position still
    /// gets a well-formed refusal, but the position-bearing form is the contract and the sweep in
    /// <c>Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests</c> enforces it.
    /// </summary>
    private static string Head(string? builtin, string parameter, int position)
    {
        string name = position > 0 ? $"argument {position} ({parameter})" : $"argument '{parameter}'";
        return builtin is null ? $"The {name}" : $"{builtin}(): {name}";
    }
}
