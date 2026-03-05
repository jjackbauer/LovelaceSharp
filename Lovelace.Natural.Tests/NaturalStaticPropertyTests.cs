using Lovelace.Natural;
using System.Threading.Tasks;
using Xunit;

namespace Lovelace.Natural.Tests;

/// <summary>
/// Tests for the static configuration properties <see cref="Natural.DisplayDigits"/>
/// and <see cref="Natural.Precision"/>.
///
/// Phase 0 of the parallelization audit: verify that both properties offer
/// correct default values, consistent round-trip behaviour, and — crucially —
/// atomic 64-bit reads/writes across threads.  A torn-read test using two
/// complementary bit patterns (0xAAAA…AAAA and 0x5555…5555) would detect a
/// non-atomic read on a 32-bit runtime because any half-write intermediate is
/// guaranteed to be neither of the two reference patterns.
/// </summary>
public class NaturalStaticPropertyTests
{
    // -------------------------------------------------------------------------
    // Helper: save and restore static state so tests do not interfere with each
    // other (xUnit runs tests in the same process).
    // -------------------------------------------------------------------------

    private static long SaveAndResetDisplayDigits()
    {
        long saved = Natural.DisplayDigits;
        Natural.DisplayDigits = -1L;
        return saved;
    }

    private static long SaveAndResetPrecision()
    {
        long saved = Natural.Precision;
        Natural.Precision = -1L;
        return saved;
    }

    // =========================================================================
    // DisplayDigits
    // =========================================================================

    [Fact]
    public void DisplayDigits_DefaultValue_IsMinusOne()
    {
        long saved = SaveAndResetDisplayDigits();
        try
        {
            Assert.Equal(-1L, Natural.DisplayDigits);
        }
        finally
        {
            Natural.DisplayDigits = saved;
        }
    }

    [Fact]
    public void DisplayDigits_AfterSet_ReturnsSetValue()
    {
        long saved = Natural.DisplayDigits;
        try
        {
            Natural.DisplayDigits = 42L;
            Assert.Equal(42L, Natural.DisplayDigits);
        }
        finally
        {
            Natural.DisplayDigits = saved;
        }
    }

    [Fact]
    public void DisplayDigits_AfterMultipleSets_ReturnsLastValue()
    {
        long saved = Natural.DisplayDigits;
        try
        {
            Natural.DisplayDigits = 10L;
            Natural.DisplayDigits = 20L;
            Natural.DisplayDigits = 999L;
            Assert.Equal(999L, Natural.DisplayDigits);
        }
        finally
        {
            Natural.DisplayDigits = saved;
        }
    }

    /// <summary>
    /// Torn-read atomicity test.
    /// Two complementary bit patterns (all-A-nibbles = 0xAAAA_AAAA_AAAA_AAAA
    /// and all-5-nibbles = 0x5555_5555_5555_5555) are written by alternating
    /// threads.  A concurrent reader verifies that every read is exactly one of
    /// the two patterns; any other value would indicate a 32-bit torn read where
    /// the high and low halves came from different writes.
    /// </summary>
    [Fact]
    public async Task DisplayDigits_ConcurrentReadWrite_NeverReturnsTornValue()
    {
        const long patternA = unchecked((long)0xAAAA_AAAA_AAAA_AAAAL);
        const long patternB = unchecked((long)0x5555_5555_5555_5555L);
        const int iterations = 100_000;

        long saved = Natural.DisplayDigits;
        bool tornReadDetected = false;

        try
        {
            Natural.DisplayDigits = patternA;

            var writer = Task.Run(() =>
            {
                for (int i = 0; i < iterations; i++)
                    Natural.DisplayDigits = (i % 2 == 0) ? patternA : patternB;
            });

            var reader = Task.Run(() =>
            {
                for (int i = 0; i < iterations; i++)
                {
                    long v = Natural.DisplayDigits;
                    if (v != patternA && v != patternB)
                    {
                        tornReadDetected = true;
                        break;
                    }
                }
            });

            await Task.WhenAll(writer, reader);
            Assert.False(tornReadDetected, "A torn (partial) 64-bit read was detected: DisplayDigits returned a value that is neither patternA nor patternB.");
        }
        finally
        {
            Natural.DisplayDigits = saved;
        }
    }

    // =========================================================================
    // Precision
    // =========================================================================

    [Fact]
    public void Precision_DefaultValue_IsMinusOne()
    {
        long saved = SaveAndResetPrecision();
        try
        {
            Assert.Equal(-1L, Natural.Precision);
        }
        finally
        {
            Natural.Precision = saved;
        }
    }

    [Fact]
    public void Precision_AfterSet_ReturnsSetValue()
    {
        long saved = Natural.Precision;
        try
        {
            Natural.Precision = 100L;
            Assert.Equal(100L, Natural.Precision);
        }
        finally
        {
            Natural.Precision = saved;
        }
    }

    [Fact]
    public void Precision_AfterMultipleSets_ReturnsLastValue()
    {
        long saved = Natural.Precision;
        try
        {
            Natural.Precision = 1L;
            Natural.Precision = 50L;
            Natural.Precision = 200L;
            Assert.Equal(200L, Natural.Precision);
        }
        finally
        {
            Natural.Precision = saved;
        }
    }

    /// <summary>
    /// Same torn-read atomicity test as for <see cref="Natural.DisplayDigits"/>,
    /// applied to <see cref="Natural.Precision"/>.
    /// </summary>
    [Fact]
    public async Task Precision_ConcurrentReadWrite_NeverReturnsTornValue()
    {
        const long patternA = unchecked((long)0xAAAA_AAAA_AAAA_AAAAL);
        const long patternB = unchecked((long)0x5555_5555_5555_5555L);
        const int iterations = 100_000;

        long saved = Natural.Precision;
        bool tornReadDetected = false;

        try
        {
            Natural.Precision = patternA;

            var writer = Task.Run(() =>
            {
                for (int i = 0; i < iterations; i++)
                    Natural.Precision = (i % 2 == 0) ? patternA : patternB;
            });

            var reader = Task.Run(() =>
            {
                for (int i = 0; i < iterations; i++)
                {
                    long v = Natural.Precision;
                    if (v != patternA && v != patternB)
                    {
                        tornReadDetected = true;
                        break;
                    }
                }
            });

            await Task.WhenAll(writer, reader);
            Assert.False(tornReadDetected, "A torn (partial) 64-bit read was detected: Precision returned a value that is neither patternA nor patternB.");
        }
        finally
        {
            Natural.Precision = saved;
        }
    }
}
