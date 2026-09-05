namespace Lovelace.Abstractions;

/// <summary>
/// The homogeneous element type of an <see cref="ArrayValue"/>. The first three members are
/// ordered from narrowest to widest so the language's scalar widening lattice
/// <c>Natural → Integer → Real</c> is expressible as numeric comparison. <see cref="Complex"/>
/// is a distinct domain type outside that lattice: it is never a widening/promotion target and
/// is only constructed explicitly (e.g. by a DSP extension).
/// </summary>
public enum DType
{
    /// <summary>Arbitrary-precision non-negative integers (<c>Natural</c>).</summary>
    Natural,

    /// <summary>Arbitrary-precision integers (<c>Integer</c>).</summary>
    Integer,

    /// <summary>Arbitrary-precision exact/truncated reals (<c>Real</c>).</summary>
    Real,

    /// <summary>
    /// Arbitrary-precision complex numbers (a pair of <c>Real</c> components). A domain type,
    /// not part of the <c>Natural → Integer → Real</c> widening lattice.
    /// </summary>
    Complex,
}
