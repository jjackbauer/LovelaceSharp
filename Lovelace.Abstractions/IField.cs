namespace Lovelace.Abstractions;

/// <summary>
/// Supplies the element-level arithmetic that generic array algorithms require. Because the
/// typed containers are generic over the element type, the field abstraction lets a consumer
/// (e.g. Lovelace.Suite over its widened <c>Value</c> union, or a plugin kernel over
/// <c>Natural</c>/<c>Integer</c>/<c>Real</c>) provide the exact arithmetic without the
/// algorithm depending on any concrete numeric type. Lives in <c>Lovelace.Abstractions</c>
/// (MOD-001/006) so plugins can target it without a second dependency.
/// </summary>
public interface IField<T>
{
    /// <summary>Additive identity.</summary>
    T Zero { get; }

    /// <summary>Multiplicative identity.</summary>
    T One { get; }

    /// <summary>Injects a plain count/size as a <typeparamref name="T"/> (used by <c>Mean</c>).</summary>
    T FromLong(long value);

    T Add(T a, T b);
    T Subtract(T a, T b);
    T Multiply(T a, T b);
    T Divide(T a, T b);
    T Negate(T a);

    /// <summary>Whether <paramref name="a"/> is the additive identity.</summary>
    bool IsZero(T a);

    /// <summary>Orders two values: -1, 0, or 1 (used by <c>Min</c>/<c>Max</c>).</summary>
    int Compare(T a, T b);

    /// <summary>Square root (used by <c>Norm</c>).</summary>
    T Sqrt(T a);
}
