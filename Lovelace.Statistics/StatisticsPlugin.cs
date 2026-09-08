using Lovelace.Abstractions;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Statistics;

/// <summary>
/// Proof Modus package: registers an exact <c>Real</c> elementwise-add kernel with zero
/// dependency on the interpreter. Consumes/produces typed arrays through the stable
/// <c>Lovelace.Abstractions</c> contract only; the core injects the exact
/// <see cref="Lovelace.Real.RealField"/> (inheriting the active precision scope) — no
/// machine types anywhere.
/// </summary>
public sealed class StatisticsPlugin : IModusPlugin
{
    public string Name => "Lovelace.Statistics";

    public void Register(IModusContext context) => context.RegisterKernel(new RealAddKernel());

    /// <summary>
    /// Exact <c>Real</c> addition over the injected field — the kernel composes the language's
    /// own arithmetic instead of a machine <c>double</c> fast path.
    /// </summary>
    private sealed class RealAddKernel : IFieldKernel<Rl>
    {
        public DType DType => DType.Real;

        public bool TryElementwise(ArrayOp op, ReadOnlySpan<Rl> left, ReadOnlySpan<Rl> right, Span<Rl> result, IField<Rl> field)
        {
            if (op != ArrayOp.Add || left.Length != right.Length || right.Length != result.Length)
                return false;

            for (int i = 0; i < left.Length; i++)
                result[i] = field.Add(left[i], right[i]);
            return true;
        }
    }
}
