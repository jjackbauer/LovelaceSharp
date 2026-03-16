using System.Collections.Generic;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for <see cref="Real.Sqrt(System.Collections.Generic.IReadOnlyList{Real})"/>.
/// Checklist item: Batch Sqrt — dispatches each element concurrently via Task.WhenAll.
/// </summary>
public class RealSqrtBatchTests
{
    // -------------------------------------------------------------------------
    // Empty batch
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenEmptyBatch_ReturnsEmptyArray()
    {
        Real[] result = Real.Sqrt(Array.Empty<Real>());
        Assert.Empty(result);
    }

    // -------------------------------------------------------------------------
    // Perfect squares — exact results
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots()
    {
        IReadOnlyList<Real> values = [new Real(4.0), new Real(9.0), new Real(16.0), new Real(25.0)];
        Real[] expected = [new Real("2"), new Real("3"), new Real("4"), new Real("5")];

        Real[] result = Real.Sqrt(values);

        Assert.Equal(expected.Length, result.Length);
        for (int i = 0; i < expected.Length; i++)
            Assert.Equal(expected[i], result[i]);
    }

    // -------------------------------------------------------------------------
    // Irrational values — match scalar serial results
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial()
    {
        IReadOnlyList<Real> values = [new Real("2"), new Real("3"), new Real("5")];

        Real[] batch = Real.Sqrt(values);
        Real[] serial =
        [
            Real.Sqrt(new Real("2")),
            Real.Sqrt(new Real("3")),
            Real.Sqrt(new Real("5")),
        ];

        Assert.Equal(serial.Length, batch.Length);
        for (int i = 0; i < serial.Length; i++)
            Assert.Equal(serial[i], batch[i]);
    }

    // -------------------------------------------------------------------------
    // Negative value — exception propagation
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException()
    {
        IReadOnlyList<Real> values = [new Real("1"), new Real("-1"), new Real("4")];

        // Task.WhenAll(...).GetAwaiter().GetResult() unwraps the AggregateException
        // and re-throws the single inner exception directly, consistent with the
        // scalar Real.Sqrt exception contract.
        Assert.Throws<ArithmeticException>(() => Real.Sqrt(values));
    }

    // -------------------------------------------------------------------------
    // Single-element batch — matches scalar call
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial()
    {
        IReadOnlyList<Real> values = [new Real(9.0)];

        Real[] result = Real.Sqrt(values);

        Assert.Single(result);
        Assert.Equal(Real.Sqrt(new Real(9.0)), result[0]);
    }
}
