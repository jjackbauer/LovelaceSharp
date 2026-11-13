//! SplitMix64: a tiny, deterministic, platform-independent PRNG.
//!
//! The C# pipeline refuses System.Random so that a seed reproduces the same sample
//! stream on every platform and runtime version. This is the same generator with the
//! same constants, so the two implementations emit the same stream for a given seed.

/// The SplitMix64 state.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Seeds the generator from the config's signed seed (two's-complement bit pattern).
    #[must_use]
    pub fn from_seed(seed: i64) -> Self {
        Self { state: seed as u64 }
    }

    /// Returns the next 64-bit draw.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns a uniform integer in [0, bound).
    pub fn next_bounded(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "the bound must be positive");
        (self.next_u64() % bound as u64) as usize
    }

    /// Returns a uniform double in [0, 1).
    pub fn next_double(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
}
