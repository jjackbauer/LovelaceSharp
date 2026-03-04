using Lovelace.Integer;

namespace Lovelace.Integer.Tests;

/// <summary>
/// Functional tests for <see cref="Integer.Multiply"/> and <c>operator *</c>.
/// Test plan items 40–44 from .github/requirements/Lovelace.Integer.md.
/// </summary>
public class IntegerMultiplyTests
{
    // -------------------------------------------------------------------------
    // Test 40: two positive operands → positive product
    // -------------------------------------------------------------------------

    [Fact]
    public void Multiply_GivenTwoPositives_ReturnsPositiveProduct()
    {
        var a = new Integer(3L);
        var b = new Integer(4L);

        var product = a * b;

        Assert.Equal(new Integer(12L), product);
        Assert.True(Integer.IsPositive(product));
    }

    // -------------------------------------------------------------------------
    // Test 41: positive × negative → negative product
    // -------------------------------------------------------------------------

    [Fact]
    public void Multiply_GivenPositiveAndNegative_ReturnsNegativeProduct()
    {
        var a = new Integer(3L);
        var b = new Integer(-4L);

        var product = a * b;

        Assert.Equal(new Integer(-12L), product);
        Assert.True(Integer.IsNegative(product));
    }

    [Fact]
    public void Multiply_GivenNegativeAndPositive_ReturnsNegativeProduct()
    {
        var a = new Integer(-3L);
        var b = new Integer(4L);

        var product = a * b;

        Assert.Equal(new Integer(-12L), product);
        Assert.True(Integer.IsNegative(product));
    }

    // -------------------------------------------------------------------------
    // Test 42: two negative operands → positive product
    // -------------------------------------------------------------------------

    [Fact]
    public void Multiply_GivenTwoNegatives_ReturnsPositiveProduct()
    {
        var a = new Integer(-3L);
        var b = new Integer(-4L);

        var product = a * b;

        Assert.Equal(new Integer(12L), product);
        Assert.True(Integer.IsPositive(product));
    }

    // -------------------------------------------------------------------------
    // Test 43: one operand is zero → zero result
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData(0L,   7L)]
    [InlineData(7L,   0L)]
    [InlineData(-5L,  0L)]
    [InlineData(0L,   0L)]
    public void Multiply_GivenZeroOperand_ReturnsZero(long left, long right)
    {
        var product = new Integer(left) * new Integer(right);

        Assert.True(Integer.IsZero(product));
        // Zero is always positive by convention.
        Assert.False(Integer.IsNegative(product));
    }

    // -------------------------------------------------------------------------
    // Test 44: multiply by one → identity
    // -------------------------------------------------------------------------

    [Theory]
    [InlineData(42L)]
    [InlineData(-7L)]
    [InlineData(0L)]
    public void Multiply_GivenOne_ReturnsOriginalValue(long n)
    {
        var value    = new Integer(n);
        var one      = new Integer(1L);

        Assert.Equal(value, value * one);
        Assert.Equal(value, one * value);
    }

    // -------------------------------------------------------------------------
    // Large-value sanity check
    // -------------------------------------------------------------------------

    [Fact]
    public void Multiply_GivenLargeValues_ReturnsCorrectProduct()
    {
        // 1_000_000_000 × 1_000_000_000 = 1_000_000_000_000_000_000
        var a = new Integer(1_000_000_000L);
        var b = new Integer(1_000_000_000L);

        var product = a * b;

        Assert.Equal(new Integer("1000000000000000000"), product);
    }

    [Fact]
    public void Multiply_GivenLargeNegativeAndPositive_ReturnsNegativeProduct()
    {
        var a = new Integer(-1_000_000L);
        var b = new Integer(1_000_000L);

        var product = a * b;

        Assert.Equal(new Integer("-1000000000000"), product);
        Assert.True(Integer.IsNegative(product));
    }

    // -------------------------------------------------------------------------
    // Verify operator* delegates to Multiply (same result via both paths)
    // -------------------------------------------------------------------------

    [Fact]
    public void OperatorStar_ProducesSameResultAsMultiplyMethod()
    {
        var a = new Integer(6L);
        var b = new Integer(-7L);

        Assert.Equal(a.Multiply(b), a * b);
    }
}
