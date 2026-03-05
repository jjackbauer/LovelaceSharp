using Lovelace.Natural;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Functional tests for <see cref="Natural"/> multiplication:
/// <c>operator*</c> (<c>IMultiplyOperators&lt;Natural,Natural,Natural&gt;</c>).
/// Checklist item: "static Natural Multiply(Natural left, Natural right) / operator*".
/// </summary>
public class NaturalMultiplyTests
{
    // ─────────────────────────────────────────────────────────────────────────
    // Basic multiplication
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenTwoPositiveNumbers_ReturnsCorrectProduct()
    {
        var left   = new Natural(12UL);
        var right  = new Natural(34UL);
        var result = left * right;
        Assert.Equal("408", result.ToString());
    }

    [Fact]
    public void Multiply_GivenZeroAndN_ReturnsZero()
    {
        var zero   = new Natural(0UL);
        var n      = new Natural(987UL);
        var result = zero * n;
        Assert.True(Natural.IsZero(result));
    }

    [Fact]
    public void Multiply_GivenNAndZero_ReturnsZero()
    {
        var n      = new Natural(987UL);
        var zero   = new Natural(0UL);
        var result = n * zero;
        Assert.True(Natural.IsZero(result));
    }

    [Fact]
    public void Multiply_GivenOneAndN_ReturnsN()
    {
        var one    = new Natural(1UL);
        var n      = new Natural(123456789UL);
        var result = one * n;
        Assert.Equal("123456789", result.ToString());
    }

    [Fact]
    public void Multiply_GivenNAndOne_ReturnsN()
    {
        var n      = new Natural(987654321UL);
        var one    = new Natural(1UL);
        var result = n * one;
        Assert.Equal("987654321", result.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Carry propagation
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenCarryAcrossAllDigits_ProducesCorrectResult()
    {
        // 9 × 9 = 81 — single-digit carry test
        var left   = new Natural(9UL);
        var right  = new Natural(9UL);
        var result = left * right;
        Assert.Equal("81", result.ToString());
    }

    [Theory]
    [InlineData(99UL,   99UL,   "9801")]
    [InlineData(999UL,  999UL,  "998001")]
    [InlineData(1000UL, 1000UL, "1000000")]
    public void Multiply_GivenPowersAndNinesVariants_ReturnsCorrectProduct(ulong a, ulong b, string expected)
    {
        var result = new Natural(a) * new Natural(b);
        Assert.Equal(expected, result.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Algebraic properties
    // ─────────────────────────────────────────────────────────────────────────

    [Theory]
    [InlineData(3UL,  7UL)]
    [InlineData(12UL, 34UL)]
    [InlineData(100UL, 999UL)]
    public void Multiply_IsCommutative(ulong a, ulong b)
    {
        var na = new Natural(a);
        var nb = new Natural(b);
        Assert.Equal((na * nb).ToString(), (nb * na).ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Large numbers (beyond ulong range)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenLargeNumbers_ExceedsUlongMax()
    {
        // ulong.MaxValue = 18446744073709551615
        // 18446744073709551615 × 2 = 36893488147419103230
        var ulongMax = new Natural(ulong.MaxValue);
        var two      = new Natural(2UL);
        var result   = ulongMax * two;
        Assert.Equal("36893488147419103230", result.ToString());
    }

    [Fact]
    public void Multiply_GivenTwoLargeNumbers_ProducesCorrect40DigitResult()
    {
        // 10000000000 × 10000000000 = 100000000000000000000 (21 digits)
        var tenBillion = new Natural(10_000_000_000UL);
        var result     = tenBillion * tenBillion;
        Assert.Equal("100000000000000000000", result.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Round-trip with previously tested operations
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenResult_HasNoLeadingZeros()
    {
        // 10 × 10 = 100 — product with trailing-zero structure, no leading zeros
        var result = new Natural(10UL) * new Natural(10UL);
        Assert.Equal("100", result.ToString());
    }

    [Fact]
    public void Multiply_GivenMultiDigitCarryChain_ProducesCorrectResult()
    {
        // 25 × 4 = 100 — carry produces an exact power of ten
        var result = new Natural(25UL) * new Natural(4UL);
        Assert.Equal("100", result.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Parallel partial-products path (outer operand has many digits, so the
    // Parallel.For code path is exercised when the threshold is exceeded)
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenFifteenDigitOperand_ReturnsCorrectProduct()
    {
        // 100000000000000 × 2 = 200000000000000
        // Forces the outer operand ("2") to be 1 digit while the inner operand
        // has 15 digits, exercising the inner digit loop over many positions.
        var a      = Natural.Parse("100000000000000"); // 10^14
        var two    = new Natural(2UL);
        var result = a * two;
        Assert.Equal("200000000000000", result.ToString());
    }

    [Fact]
    public void Multiply_GivenTwoFifteenDigitNumbers_ReturnsCorrectProduct()
    {
        // (10^15 - 1)^2 = 10^30 - 2×10^15 + 1
        //               = 999999999999998000000000000001
        var n = Natural.Parse("999999999999999");   // 15 nines
        var result = n * n;
        Assert.Equal("999999999999998000000000000001", result.ToString());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Snapshot-path correctness (parallel path, 50-digit operands)
    // Validates that SnapshotDigits-backed reads yield the same result as
    // per-digit GetDigit reads for very large operands that always exceed the
    // processorCount * 2 threshold.
    // ─────────────────────────────────────────────────────────────────────────

    [Fact]
    public void Multiply_GivenHundredFiftyDigitPowersOfTen_ReturnsCorrectThreeHundredDigitResult()
    {
        // 10^150 × 10^150 = 10^300  — "1" followed by 300 zeros
        // 151-digit operands guarantee the parallel path even on machines with
        // up to 75 processors (75 × 2 = 150 < 151).
        var a = Natural.Parse("1" + new string('0', 150)); // 10^150 (151 digits)
        var b = Natural.Parse("1" + new string('0', 150));
        var result = a * b;
        Assert.Equal("1" + new string('0', 300), result.ToString());
    }

    [Fact]
    public async System.Threading.Tasks.Task Multiply_GivenSharedLargeOperandReadConcurrently_AllResultsConsistent()
    {
        // Create one large shared Natural that multiple concurrent multiplications read.
        // 200-digit operands guarantee the parallel path even on machines with up to
        // 99 processorCount (99 * 2 = 198 < 200).
        // Expected: 10^200 × 10^200 = 10^400
        var shared = Natural.Parse("1" + new string('0', 200)); // 10^200 (201 digits)
        string expectedResult = "1" + new string('0', 400);     // 10^400 (401 chars)

        const int taskCount = 16;
        var tasks = new System.Threading.Tasks.Task<string>[taskCount];
        for (int i = 0; i < taskCount; i++)
            tasks[i] = System.Threading.Tasks.Task.Run(() => (shared * shared).ToString());

        string[] results = await System.Threading.Tasks.Task.WhenAll(tasks);

        foreach (var r in results)
            Assert.Equal(expectedResult, r);
    }

    [Fact]
    public async System.Threading.Tasks.Task Multiply_GivenLargeMultiplicationsRunConcurrently_AllReturnCorrectResults()
    {
        // Stress-test the parallel partial-products path with 32 concurrent
        // multiplications. If there is any shared-state race across Natural
        // instances the results will diverge from the expected value.
        // 100000000000 × 100000000000 = 10000000000000000000000 (10^22)
        const string expected = "10000000000000000000000";
        const int taskCount   = 32;

        var tasks = new System.Threading.Tasks.Task<string>[taskCount];
        for (int i = 0; i < taskCount; i++)
        {
            tasks[i] = System.Threading.Tasks.Task.Run(() =>
            {
                var a = Natural.Parse("100000000000");   // 10^11
                var b = Natural.Parse("100000000000");
                return (a * b).ToString();
            });
        }

        string[] results = await System.Threading.Tasks.Task.WhenAll(tasks);

        foreach (string r in results)
            Assert.Equal(expected, r);
    }
}
