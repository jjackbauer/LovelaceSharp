namespace Lovelace.Knowledge;

/// <summary>
/// SplitMix64 — a tiny, deterministic, platform-independent PRNG. Used instead of
/// <c>System.Random</c> so that a given seed reproduces the exact same sample stream
/// across .NET versions and platforms (P5).
/// </summary>
public sealed class SplitMix64
{
    private ulong _state;

    public SplitMix64(ulong seed) => _state = seed;

    public static SplitMix64 FromLong(long seed) => new(unchecked((ulong)seed));

    public ulong NextUInt64()
    {
        ulong z = (_state += 0x9E3779B97F4A7C15UL);
        z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
        z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
        return z ^ (z >> 31);
    }

    /// <summary>Uniform integer in [0, bound).</summary>
    public int NextInt(int bound)
    {
        if (bound <= 0) throw new ArgumentOutOfRangeException(nameof(bound));
        return (int)(NextUInt64() % (ulong)bound);
    }

    /// <summary>Uniform double in [0, 1).</summary>
    public double NextDouble() => (NextUInt64() >> 11) * (1.0 / 9007199254740992.0);
}
