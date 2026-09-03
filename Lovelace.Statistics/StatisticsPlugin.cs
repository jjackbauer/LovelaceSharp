using Lovelace.Abstractions;

namespace Lovelace.Statistics;

/// <summary>
/// Proof Modus package: registers an optimized <c>double</c> elementwise-add kernel with
/// zero dependency on the interpreter. Consumes/produces typed arrays through the stable
/// <c>Lovelace.Abstractions</c> contract only.
/// </summary>
public sealed class StatisticsPlugin : IModusPlugin
{
    public string Name => "Lovelace.Statistics";

    public void Register(IModusContext context) => context.RegisterKernel(new DoubleAddKernel());

    private sealed class DoubleAddKernel : IArrayKernel<double>
    {
        public DType DType => DType.Real;

        public bool TryElementwise(ArrayOp op, ReadOnlySpan<double> left, ReadOnlySpan<double> right, Span<double> result)
        {
            if (op != ArrayOp.Add || left.Length != right.Length || right.Length != result.Length)
                return false;

            for (int i = 0; i < left.Length; i++)
                result[i] = left[i] + right[i];
            return true;
        }
    }
}
