using System.Globalization;

namespace Lovelace.Suite;

/// <summary>
/// The single-allocation budget for ONE array a script asks the engine to build.
/// <para>
/// It exists for the same reason <see cref="Lovelace.Abstractions.InputDepth"/> does (see its remarks): a
/// request the process cannot serve is not a defect in the input, and it must not cross as one. Before this
/// budget, <c>zeros(1000000000)</c> — a legitimate call of the documented constructor
/// (<c>Lovelace.Suite/docs/Language.md:928-935</c>) — reached the allocator, the CLR raised
/// <c>OutOfMemoryException</c>, and the runner's generic handler published it as
/// <c>InternalError/InternalInvariantFailure</c> with <c>recoverable:false</c>: the caller was told "do not
/// retry" about a request that is merely too large (F2-C,
/// <c>docs/goal-cycle-6/round-13/audit-C-hostile.md:82-127</c>).
/// </para>
/// <para>
/// The budget is a fixed, documented number rather than a probe of the machine's free memory on purpose: the
/// same request is served by one runtime and refused by another (the published Native AOT binary raised
/// <c>OutOfMemoryException</c> for 10^9 elements with 39 GB free, while the CoreCLR test host allocated the
/// same buffer), so a machine-derived limit would make the refusal — and every test of it — depend on the
/// host. The limit is enforced BEFORE the buffer is created, so the refusal costs nothing and cannot itself
/// be a memory failure (the same "refuse before the walk" shape as the depth budgets).
/// </para>
/// </summary>
public static class ArrayAllocationBudget
{
    /// <summary>
    /// The greatest number of elements ONE array may hold: 2^28 = 268 435 456. At the 8 bytes a
    /// <c>Value</c> reference occupies that is a 2 GiB single buffer, which is the largest request the engine
    /// will make of the allocator; the audit's shape (10^9 elements) is 8 GiB and is refused. Every size the
    /// product has measured as legitimately servable stays inside it by a wide margin — <c>zeros(10^6)</c>,
    /// <c>sum(1..10000000)</c>, <c>len(1..100000000)</c> (audit C, "coverage that held",
    /// <c>docs/goal-cycle-6/round-13/audit-C-hostile.md:233-257</c>).
    /// </summary>
    public const long MaxElements = 1L << 28;

    /// <summary>
    /// Refuses <paramref name="shape"/> when the array it describes would exceed
    /// <see cref="MaxElements"/>. Zero dimensions are the D5 contract and are never refused by this budget
    /// (an array with a zero dimension holds no elements), and a negative dimension is left to the existing
    /// path, which refuses it exactly as it does today.
    /// </summary>
    /// <param name="shape">The requested dimensions, outermost first.</param>
    /// <param name="builtin">The builtin that asked, named in the refusal.</param>
    /// <exception cref="ArrayAllocationRefusedException">The element count exceeds the budget.</exception>
    public static void EnsureServable(IReadOnlyList<long> shape, string builtin)
    {
        var (total, overflowed) = Measure(shape);
        if (total > MaxElements)
            throw new ArrayAllocationRefusedException(builtin, shape, total, MaxElements, overflowed);
    }

    /// <summary>
    /// The exact element count of <paramref name="shape"/> when it fits in a signed 64-bit count, else
    /// <see cref="long.MaxValue"/> with <paramref name="overflowed"/> set. A negative dimension returns
    /// <c>(0, false)</c>: it is not an allocation-size question and this budget does not answer it.
    /// </summary>
    private static (long Total, bool Overflowed) Measure(IReadOnlyList<long> shape)
    {
        long total = 1;
        foreach (long dimension in shape)
        {
            if (dimension < 0)
                return (0, false);
            if (dimension == 0)
            {
                total = 0;
                continue;
            }

            try
            {
                total = checked(total * dimension);
            }
            catch (OverflowException)
            {
                return (long.MaxValue, true);
            }
        }

        return (total, false);
    }
}

/// <summary>
/// A caller-supplied shape whose element count exceeds <see cref="ArrayAllocationBudget.MaxElements"/>. This
/// is a budget stop, not a defect: the caller can ask for a smaller shape or process the data in pieces, so
/// the runner classifies it as the recoverable <c>AllocationRefused</c>/<c>BudgetExceeded</c> failure rather
/// than as an internal invariant failure or an out-of-memory error (F2-C). It derives from
/// <see cref="Exception"/> directly (not from <see cref="ArgumentException"/>) for the same reason
/// <see cref="Lovelace.Abstractions.InputDepthExceededException"/> does: a kernel that converts ordinary
/// argument failures into domain diagnostics must not be able to swallow a resource refusal into a
/// <c>TypeMismatch</c>.
/// </summary>
public sealed class ArrayAllocationRefusedException : Exception
{
    /// <summary>The shape that was refused, outermost dimension first.</summary>
    public IReadOnlyList<long> Shape { get; }

    /// <summary>The element count that was refused: the exact product of <see cref="Shape"/>, or
    /// <see cref="long.MaxValue"/> when the product does not fit in a signed 64-bit count.</summary>
    public long RequestedElements { get; }

    /// <summary>The budget it exceeded.</summary>
    public long Limit { get; }

    /// <param name="builtin">The builtin that asked for the array.</param>
    /// <param name="shape">The refused shape.</param>
    /// <param name="requestedElements">The refused element count (see <see cref="RequestedElements"/>).</param>
    /// <param name="limit">The budget it exceeded.</param>
    /// <param name="productOverflowed">Whether <paramref name="requestedElements"/> saturated because the
    /// true product does not fit in a signed 64-bit count.</param>
    public ArrayAllocationRefusedException(string builtin, IReadOnlyList<long> shape, long requestedElements,
        long limit, bool productOverflowed = false)
        : base($"{builtin}() requested one array of shape [{string.Join(", ", shape)}], which holds "
               + (productOverflowed
                   ? $"more than {long.MaxValue.ToString(CultureInfo.InvariantCulture)}"
                   : requestedElements.ToString(CultureInfo.InvariantCulture))
               + $" element(s) — more than the maximum single-allocation budget of "
               + $"{limit.ToString(CultureInfo.InvariantCulture)} element(s). The request is refused BEFORE "
               + "the allocation is attempted. Ask for a smaller shape, or process the data in pieces.")
    {
        Shape = shape;
        RequestedElements = requestedElements;
        Limit = limit;
    }
}
