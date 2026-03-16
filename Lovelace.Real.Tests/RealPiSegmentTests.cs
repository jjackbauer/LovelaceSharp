using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Functional tests for the internal <c>Real.PiSegment(long termStart, long termEnd)</c>
/// Binary Splitting decomposition helper.
/// Checklist item: PiSegment — Add internal static method PiSegment(long termStart, long termEnd)
/// returning (Nat P, Nat Q, Int T) for the Chudnovsky BSP recurrence.
/// </summary>
public class RealPiSegmentTests
{
    // -------------------------------------------------------------------------
    // Base-case correctness
    // -------------------------------------------------------------------------

    [Fact]
    public void PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation()
    {
        // For k=1 (single-term range [1, 2)):
        //   a_1 = 6·5·4·3·2·1 = 720
        //   b_1 = 3·2·1 · 1³ · 640320³ = 6 · 262537412640768000 = 1575224475844608000
        //   T(1,2) = (−1)^1 · a_1 · (A + B·1) = −720 · 558731543 = −402286710960
        var (P, Q, T) = Real.PiSegment(1L, 2L);

        Assert.Equal("720", P.ToString());
        Assert.Equal("1575224475844608000", Q.ToString());
        Assert.Equal("-402286710960", T.ToString());
    }

    // -------------------------------------------------------------------------
    // Full-range consistency with the serial Pi algorithm
    // -------------------------------------------------------------------------

    [Fact]
    public void PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial()
    {
        // Pi(10) uses guardDigits = 20, numTerms = ceil(20/14.0)+2 = 4.
        // PiSegment(0, numTerms+1) covers terms k=0..4 and must produce
        // (Q = denS_4, T = numS_4) identical to the sequential Chudnovsky loop.
        const long A = 13591409L;
        const long B = 545140134L;
        long guardDigits = 20L;
        long numTerms    = (long)Math.Ceiling((double)guardDigits / 14.0) + 2L; // 4

        // Reference: inline sequential accumulation (mirrors Real.Pi internals).
        var c3    = Lovelace.Natural.Natural.Parse("262537412640768000", null);
        var numS  = new Lovelace.Integer.Integer(new Lovelace.Natural.Natural((ulong)A), false);
        var denS  = Lovelace.Natural.Natural.One;
        var prodP = Lovelace.Natural.Natural.One;
        for (long k = 1; k <= numTerms; k++)
        {
            long k6 = 6 * k, k3 = 3 * k;
            var ak = new Lovelace.Natural.Natural((ulong)(k6 * (k6 - 1) * (k6 - 2)))
                   * new Lovelace.Natural.Natural((ulong)((k6 - 3) * (k6 - 4) * (k6 - 5)));
            var bk = new Lovelace.Natural.Natural((ulong)(k3 * (k3 - 1) * (k3 - 2)))
                   * new Lovelace.Natural.Natural((ulong)(k * k * k)) * c3;
            prodP  = prodP * ak;
            var bkInt = new Lovelace.Integer.Integer(bk, false);
            var lf    = new Lovelace.Natural.Natural((ulong)(A + B * k));
            var term  = new Lovelace.Integer.Integer(prodP * lf, k % 2 == 1);
            numS = numS * bkInt + term;
            denS = denS * bk;
        }

        // BSP result — must algebraically equal the sequential result.
        var (_, Q, T) = Real.PiSegment(0L, numTerms + 1L);

        Assert.Equal(denS.ToString(), Q.ToString());
        Assert.Equal(numS.ToString(), T.ToString());
    }

    // -------------------------------------------------------------------------
    // BSP merge identity
    // -------------------------------------------------------------------------

    [Fact]
    public void PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT()
    {
        // Split [0, 10) at midpoint 5.
        // Merge identity: T(a,b) = T(a,m)·Q(m,b) + P(a,m)·T(m,b)
        long a = 0L, b = 10L, m = (a + b) / 2L;   // m = 5

        var (lP, lQ, lT) = Real.PiSegment(a, m);
        var (rP, rQ, rT) = Real.PiSegment(m, b);
        var (_ , _  , tFull) = Real.PiSegment(a, b);

        // Reconstruct merged T using the BSP identity.
        var mergedT = lT * new Lovelace.Integer.Integer(rQ, false)
                    + new Lovelace.Integer.Integer(lP, false) * rT;

        Assert.Equal(tFull.ToString(), mergedT.ToString());
    }
}
