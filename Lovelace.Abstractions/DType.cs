namespace Lovelace.Abstractions;

/// <summary>
/// The homogeneous element type of an <see cref="ArrayValue"/>. Members are ordered from
/// narrowest to widest so the language's scalar widening lattice
/// <c>Natural → Integer → Real</c> is expressible as numeric comparison.
/// </summary>
public enum DType
{
    /// <summary>Arbitrary-precision non-negative integers (<c>Natural</c>).</summary>
    Natural,

    /// <summary>Arbitrary-precision integers (<c>Integer</c>).</summary>
    Integer,

    /// <summary>Arbitrary-precision exact/truncated reals (<c>Real</c>).</summary>
    Real,
}
