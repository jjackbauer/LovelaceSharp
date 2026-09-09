namespace Lovelace.Abstractions;

/// <summary>
/// The Suite↔Symbolics matrix seam: the core interpreter no longer embeds symbolic-matrix
/// knowledge (determinant/rank/inverse/solve algorithms); a plugin supplies this bridge at
/// load time and the core dispatches symbolic matrix arguments through it. Elements arrive
/// as the plugin payload vocabulary (flat, row-major) together with the shape, so the plugin
/// never references the core value model. Returns are payloads (Expr objects, flat/nested
/// lists, integers); null means "not handled" or "singular".
/// </summary>
public interface ISymbolicMatrixBridge
{
    /// <summary>True when every element of the matrix is a symbolic expression.</summary>
    bool IsSymbolicMatrix(IReadOnlyList<object?> elements, long[] shape);

    /// <summary>Fraction-free determinant, or null when not applicable.</summary>
    object? TryDet(IReadOnlyList<object?> elements, long[] shape);

    /// <summary>Exact rank (an integer payload), or null when not applicable.</summary>
    object? TryRank(IReadOnlyList<object?> elements, long[] shape);

    /// <summary>Condition-carrying inverse: nested row payload, or null when singular.</summary>
    object? TryInverse(IReadOnlyList<object?> elements, long[] shape, out object? conditions);

    /// <summary>Fraction-free linear solve A·x = b: flat solution payload, or null when singular.</summary>
    object? TrySolve(IReadOnlyList<object?> elements, long[] shape, IReadOnlyList<object?> rhs, out object? conditions);

    /// <summary>Structured linear solve for linsolve_full: solutions payload (flat, possibly empty),
    /// conditions payload, a human note, and whether the system is singular.</summary>
    object? TrySolveFull(
        IReadOnlyList<object?> elements, long[] shape, IReadOnlyList<object?> rhs,
        out object? conditions, out string? note, out bool singular);
}
