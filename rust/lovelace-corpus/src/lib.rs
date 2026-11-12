//! Deterministic corpus support for the Lovelace parity work.
//!
//! The binary in this crate is the documentation-corpus gate. The library adds two
//! additive surfaces used to reproduce the C# `Lovelace.Symbolics.Tests.PropertyTests`
//! corpora exactly:
//!
//! * [`dotnet_random`] is a faithful port of the seeded `System.Random` subtractive
//!   generator that .NET keeps for compatibility.
//! * [`property_corpus`] builds the seven generated corpora with the same draw order as
//!   the C# tests, and records the draw transcript and the canonical expression text.

pub mod dotnet_random;
#[cfg(test)]
mod property_assertions;
pub mod property_corpus;

/// FNV-1a 64 over the UTF-8 bytes of `text`.
///
/// The parity report compares transcripts by hash; FNV-1a is deterministic across
/// languages and needs no dependency. It is a regression checksum, not a security
/// primitive.
pub(crate) fn fnv1a64(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
