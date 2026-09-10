using System.Collections;

namespace Lovelace.Abstractions;

/// <summary>
/// A rank-N array crossing the plugin boundary: the flat row-major elements plus their shape, so
/// a plugin can validate the caller's structure instead of silently reinterpreting it. Implements
/// <see cref="IReadOnlyList{T}"/> so plugins that only iterate are unaffected.
/// </summary>
public sealed class PayloadArray : IReadOnlyList<object?>
{
    private readonly IReadOnlyList<object?> _elements;

    public PayloadArray(IReadOnlyList<object?> elements, IReadOnlyList<long> shape)
    {
        _elements = elements;
        Shape = shape;
    }

    /// <summary>Row-major dimensions; always at least rank 1 (<c>[n]</c> for a flat list).</summary>
    public IReadOnlyList<long> Shape { get; }

    public int Rank => Shape.Count;

    /// <summary>Number of dimensions of the given row width, or -1 when unspecified.</summary>
    public long RowWidth => Rank >= 2 ? Shape[^1] : -1;

    public object? this[int index] => _elements[index];

    public int Count => _elements.Count;

    public IEnumerator<object?> GetEnumerator() => _elements.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}
