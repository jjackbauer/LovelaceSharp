//! A base-10 BCD digit store mirroring `Lovelace.Representation.DigitStore`.
//!
//! The reference store keeps two decimal digits per byte: the high nibble holds the
//! even-indexed digit and the low nibble the odd-indexed digit. Position 0 is the
//! least-significant digit. `0x0C` marks an allocated but unwritten byte and
//! `0x0F` marks a freed low nibble.
//!
//! This crate's `Nat` is a different representation: little-endian base-2^64 limbs.
//! The two forms meet through canonical decimal text (and through the explicit
//! `u32` limb helpers below), never by sharing a backing buffer. `DigitStore` is a
//! standalone value; adding it changes no `Nat` operation.
//!
//! # Invariants
//!
//! For values produced by the value-level API (`new`, `from_decimal`, `from_digits`,
//! `from_u32_limbs`, `from_nat`, `set_digits_bulk`, `trim_leading_zeros`):
//!
//! * no leading zero decimal digit (the most-significant digit is non-zero unless
//!   the value is zero);
//! * zero is canonical: `is_zero()`, `digit_count() == 0` and `byte_count() == 0`;
//! * `byte_count() == (digit_count() + 1) / 2`.
//!
//! The low-level mutators deliberately reproduce the reference store's escape
//! hatches, so the same caveats apply: `grow_digits` / `set_bitwise` can allocate
//! bytes ahead of the digit count, and `set_digit_count` / `set_is_zero` can create
//! metadata that no digit write would produce. Those states are not canonical and
//! are only used by the ported tests that assert the mutators themselves. Rendering
//! a non-zero-flagged store whose digit count is zero yields `"0"`; the reference
//! would index its empty byte buffer and throw.
//!
//! # Thread safety
//!
//! Every method takes `&self` and synchronises through one internal mutex, matching
//! the reference store's `_syncRoot` discipline. Cross-copies lock both stores in
//! instance-id order, so two threads copying from each other cannot deadlock.

use crate::Nat;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Sentinel byte appended by `grow_digits`: slot allocated, no digit written.
const SENTINEL_ALLOCATED: u8 = 0x0C;
/// Sentinel nibble written when `shrink_digits` frees an odd-position digit.
const SENTINEL_FREED: u8 = 0x0F;

/// The result of a `DigitStore` operation that can reject its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigitStoreError {
    /// `set_digit` accepts positions in `0..=digit_count` only.
    PositionOutOfRange {
        /// The rejected position (negative values are rejected too).
        position: i64,
        /// The store's digit count at the time of the call.
        digit_count: usize,
    },
}

impl fmt::Display for DigitStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DigitStoreError::PositionOutOfRange {
                position,
                digit_count,
            } => write!(
                f,
                "position {position} is out of range (DigitCount={digit_count})"
            ),
        }
    }
}

impl std::error::Error for DigitStoreError {}

#[derive(Debug, Clone)]
struct Inner {
    /// Little-endian packed BCD bytes; two logical digits per byte.
    bytes: Vec<u8>,
    /// Number of logical decimal digits.
    digit_count: usize,
    /// True when the value is zero, independently of allocated bytes.
    is_zero: bool,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Mutable, thread-safe base-10 BCD storage, the Rust counterpart of
/// `Lovelace.Representation.DigitStore`.
///
/// The public surface mirrors the reference member-for-member; see the crate
/// module documentation for the base difference and invariants.
pub struct DigitStore {
    inner: Mutex<Inner>,
    /// Construction-order id used to order the two locks taken by `copy_digits_from`.
    id: u64,
}

impl Default for DigitStore {
    fn default() -> DigitStore {
        DigitStore::new()
    }
}

impl Clone for DigitStore {
    /// Deep copy, matching `DigitStore(DigitStore other)`: metadata is copied and
    /// a zero source contributes no bytes.
    fn clone(&self) -> DigitStore {
        let inner = self.lock();
        let bytes = if inner.is_zero {
            Vec::new()
        } else {
            inner.bytes.clone()
        };
        DigitStore::from_inner(Inner {
            bytes,
            digit_count: inner.digit_count,
            is_zero: inner.is_zero,
        })
    }
}

impl fmt::Display for DigitStore {
    /// The no-separator decimal rendering, most-significant digit first.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render(None))
    }
}

impl DigitStore {
    /// Creates an empty zero store: `DigitCount = 0`, `ByteCount = 0`, `IsZero = true`.
    pub fn new() -> DigitStore {
        DigitStore::from_inner(Inner {
            bytes: Vec::new(),
            digit_count: 0,
            is_zero: true,
        })
    }

    fn from_inner(inner: Inner) -> DigitStore {
        DigitStore {
            inner: Mutex::new(inner),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    // ---------------------------------------------------------------------
    // Constructors / bulk input
    // ---------------------------------------------------------------------

    /// Builds from unpacked decimal digits, least-significant first.
    ///
    /// This is the constructor analogue of the reference `SetDigitsBulk`: leading
    /// (most-significant) zeros are trimmed, and an all-zero input becomes the
    /// canonical zero state.
    pub fn from_digits(digits: &[u8]) -> DigitStore {
        let store = DigitStore::new();
        store.set_digits_bulk(digits);
        store
    }

    /// Builds from canonical decimal text, the same grammar as `Nat::from_decimal`:
    /// surrounding whitespace is trimmed; empty, signed or non-digit text is rejected.
    ///
    /// This member has no C# counterpart; it makes the decimal conversion direction
    /// of the representation contract explicit.
    pub fn from_decimal(text: &str) -> Option<DigitStore> {
        let trimmed = text.trim();
        if trimmed.is_empty() || !trimmed.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits: Vec<u8> = trimmed.bytes().rev().map(|b| b - b'0').collect();
        Some(DigitStore::from_digits(&digits))
    }

    /// Builds from a canonical BCD backing buffer (little-endian, two digits per byte).
    ///
    /// Nibbles `0x0C` and `0x0F` are the reference sentinels and are treated as
    /// unwritten slots. Returns `None` when any other non-decimal nibble appears.
    /// Allocated-but-unwritten bytes are normalised away, so the result is canonical.
    ///
    /// This member has no C# counterpart; it exposes the byte level that the
    /// reference only reaches through `GetBitwise` / `SetBitwise`.
    pub fn from_bcd_bytes(bytes: &[u8]) -> Option<DigitStore> {
        let mut digit_count = 0usize;
        for (index, &packed) in bytes.iter().enumerate() {
            let high = packed >> 4;
            let low = packed & 0x0F;
            for (nibble, position) in [(high, index * 2), (low, index * 2 + 1)] {
                match nibble {
                    0..=9 => {
                        if nibble != 0 {
                            digit_count = digit_count.max(position + 1);
                        }
                    }
                    SENTINEL_ALLOCATED | SENTINEL_FREED => {}
                    _ => return None,
                }
            }
        }
        if digit_count == 0 {
            return Some(DigitStore::new());
        }
        let mut digits = Vec::with_capacity(digit_count);
        for position in 0..digit_count {
            let packed = bytes.get(position / 2).copied().unwrap_or(0);
            let nibble = if position % 2 == 0 {
                packed >> 4
            } else {
                packed & 0x0F
            };
            digits.push(if nibble <= 9 { nibble } else { 0 });
        }
        Some(DigitStore::from_digits(&digits))
    }

    /// Builds from little-endian base-2^32 limbs, trimming leading zero limbs.
    ///
    /// This member has no C# counterpart; it is the binary-limb conversion the Rust
    /// side needs, and its canonical form is the same no-leading-zero-limb rule
    /// `Nat` applies to base-2^64 limbs.
    pub fn from_u32_limbs(limbs: &[u32]) -> DigitStore {
        let mut end = limbs.len();
        while end > 0 && limbs[end - 1] == 0 {
            end -= 1;
        }
        if end == 0 {
            return DigitStore::new();
        }
        // Value = prefix; for each limb, value = value * 2^32 + limb. The carry
        // chain is over decimal digits so no big-integer type is needed.
        let mut digits: Vec<u8> = Vec::new();
        for &limb in limbs[..end].iter().rev() {
            let mut carry = limb as u64;
            for digit in digits.iter_mut() {
                let current = (*digit as u64) * (1u64 << 32) + carry;
                *digit = (current % 10) as u8;
                carry = current / 10;
            }
            while carry > 0 {
                digits.push((carry % 10) as u8);
                carry /= 10;
            }
        }
        DigitStore::from_digits(&digits)
    }

    /// Builds from a `Nat` through its canonical decimal rendering.
    ///
    /// This member has no C# counterpart; it is the bridge between the crate's
    /// binary-limb natural numbers and this decimal store.
    pub fn from_nat(value: &Nat) -> DigitStore {
        DigitStore::from_decimal(&value.to_decimal()).expect("Nat renders canonical decimal")
    }

    // ---------------------------------------------------------------------
    // Properties
    // ---------------------------------------------------------------------

    /// Number of bytes in the backing store.
    pub fn byte_count(&self) -> usize {
        self.lock().bytes.len()
    }

    /// Number of logical decimal digits stored.
    pub fn digit_count(&self) -> usize {
        self.lock().digit_count
    }

    /// True when the number represented is zero.
    pub fn is_zero(&self) -> bool {
        self.lock().is_zero
    }

    /// Sets the digit count directly, under the store lock.
    ///
    /// The reference exposes this internally; it is public here so the ported
    /// `DigitStoreMutatorTests` can exercise it.
    pub fn set_digit_count(&self, value: usize) {
        self.lock().digit_count = value;
    }

    /// Sets the zero flag directly, under the store lock.
    ///
    /// The reference exposes this internally; it is public here so the ported
    /// `DigitStoreMutatorTests` can exercise it.
    pub fn set_is_zero(&self, value: bool) {
        self.lock().is_zero = value;
    }

    // ---------------------------------------------------------------------
    // Digit access
    // ---------------------------------------------------------------------

    /// Returns the decimal digit at `position` (0 = least-significant).
    ///
    /// Returns 0 when the store is zero or the position is outside
    /// `0..digit_count`, as the reference does.
    pub fn get_digit(&self, position: i64) -> u8 {
        let inner = self.lock();
        digit_public(&inner, position)
    }

    /// Writes a decimal digit at `position`.
    ///
    /// The reference's `ArgumentOutOfRangeException` is returned as
    /// `DigitStoreError::PositionOutOfRange`. Positions after the first must be
    /// written sequentially: `position <= digit_count`.
    pub fn set_digit(&self, position: i64, digit: u8) -> Result<(), DigitStoreError> {
        let mut inner = self.lock();
        if position < 0 || position as usize > inner.digit_count {
            return Err(DigitStoreError::PositionOutOfRange {
                position,
                digit_count: inner.digit_count,
            });
        }
        let position = position as usize;
        let byte_index = position / 2;
        let (mut high, mut low) = if byte_index < inner.bytes.len() {
            unpack(inner.bytes[byte_index])
        } else {
            // A new byte starts with the low-nibble sentinel, exactly as the reference.
            (digit, SENTINEL_FREED)
        };
        if position % 2 == 0 {
            high = digit;
        } else {
            low = digit;
        }
        if position >= inner.digit_count {
            inner.digit_count += 1;
        }
        if position == 0 {
            inner.is_zero = false;
        }
        set_bitwise_inner(&mut inner, (position / 2) as i64, high, low);
        Ok(())
    }

    /// Returns every digit as a plain vector, least-significant first.
    ///
    /// The snapshot is independent of the store and empty in the zero state.
    pub fn snapshot_digits(&self) -> Vec<u8> {
        let inner = self.lock();
        snapshot_locked(&inner)
    }

    /// Rents a snapshot buffer and returns it with its valid length.
    ///
    /// The reference rents from `ArrayPool<byte>.Shared`; this implementation owns a
    /// fresh `Vec<u8>`, so the only observable contract is the digit content and the
    /// valid length. Pass the vector to `return_digit_snapshot` when done.
    pub fn rent_digit_snapshot(&self) -> (Vec<u8>, usize) {
        let snapshot = self.snapshot_digits();
        let length = snapshot.len();
        (snapshot, length)
    }

    /// Releases a buffer previously produced by `rent_digit_snapshot`.
    ///
    /// There is no shared pool in Rust: dropping the owned vector is the release.
    pub fn return_digit_snapshot(&self, buffer: Vec<u8>) {
        drop(buffer);
    }

    // ---------------------------------------------------------------------
    // BCD infrastructure
    // ---------------------------------------------------------------------

    /// Splits the byte at `pos` into its high and low nibbles.
    ///
    /// Silent `(0, 0)` when `pos` is out of range, matching the reference.
    pub fn get_bitwise(&self, pos: i64) -> (u8, u8) {
        let inner = self.lock();
        if pos >= 0 {
            if let Some(&packed) = inner.bytes.get(pos as usize) {
                return unpack(packed);
            }
        }
        (0, 0)
    }

    /// Packs `high` and `low` into the byte at `pos`.
    ///
    /// Calls `grow_digits` when `pos == byte_count` and is a silent no-op when
    /// `pos > byte_count`, matching the reference.
    pub fn set_bitwise(&self, pos: i64, high: u8, low: u8) {
        let mut inner = self.lock();
        set_bitwise_inner(&mut inner, pos, high, low);
    }

    /// Appends a sentinel byte (`0x0C`), growing the backing store by one byte.
    pub fn grow_digits(&self) {
        let mut inner = self.lock();
        inner.bytes.push(SENTINEL_ALLOCATED);
    }

    /// Removes the most-significant digit slot.
    ///
    /// Odd `digit_count` removes the last byte; even count overwrites the low nibble
    /// of the last byte with `0x0F`. Decrements `digit_count` in both cases. On an
    /// empty store this saturates at zero instead of the reference's negative count.
    pub fn shrink_digits(&self) {
        let mut inner = self.lock();
        if inner.digit_count % 2 == 1 {
            inner.bytes.pop();
        } else if let Some(last) = inner.bytes.last_mut() {
            *last = (*last & 0xF0) | SENTINEL_FREED;
        }
        inner.digit_count = inner.digit_count.saturating_sub(1);
    }

    /// Clears all bytes from the backing store without resetting the metadata.
    pub fn clear_digits(&self) {
        self.lock().bytes.clear();
    }

    /// Deep-copies the backing bytes from `other`.
    ///
    /// No-op when `other` is the same store or is zero. Only bytes are copied:
    /// the reference deliberately leaves `digit_count` and `is_zero` to the caller.
    /// Both stores are locked in instance-id order to prevent cross-copy deadlock.
    pub fn copy_digits_from(&self, other: &DigitStore) {
        if self.id == other.id {
            return;
        }
        let copy = |source: &Inner, destination: &mut Inner| {
            if source.is_zero {
                return;
            }
            destination.bytes.clear();
            destination.bytes.extend_from_slice(&source.bytes);
        };
        if self.id < other.id {
            let mut destination = self.lock();
            let source = other.lock();
            copy(&source, &mut destination);
        } else {
            let source = other.lock();
            let mut destination = self.lock();
            copy(&source, &mut destination);
        }
    }

    /// Resets `digit_count` and `is_zero` without touching the byte list.
    pub fn initialize(&self) {
        let mut inner = self.lock();
        inner.digit_count = 0;
        inner.is_zero = true;
    }

    /// Clears all digits and reinitialises to the zero state; no-op when already zero.
    pub fn reset(&self) {
        let mut inner = self.lock();
        reset_inner(&mut inner);
    }

    /// Overwrites the store from unpacked decimal digits, least-significant first.
    ///
    /// Leading zeros are trimmed and an all-zero value produces the canonical zero
    /// state. One lock covers the whole write.
    pub fn set_digits_bulk(&self, digits: &[u8]) {
        let mut inner = self.lock();
        let mut len = digits.len();
        while len > 1 && digits[len - 1] == 0 {
            len -= 1;
        }
        if len == 0 || (len == 1 && digits[0] == 0) {
            reset_inner(&mut inner);
            return;
        }
        inner.bytes.clear();
        for byte_index in 0..len.div_ceil(2) {
            let high = digits[byte_index * 2];
            let low = if byte_index * 2 + 1 < len {
                digits[byte_index * 2 + 1]
            } else {
                SENTINEL_FREED
            };
            inner.bytes.push(high.wrapping_shl(4) | (low & 0x0F));
        }
        inner.digit_count = len;
        inner.is_zero = false;
    }

    /// Removes non-significant leading zeros and restores zero state for an all-zero value.
    pub fn trim_leading_zeros(&self) {
        let mut inner = self.lock();
        while inner.digit_count > 1 && digit_public(&inner, (inner.digit_count - 1) as i64) == 0 {
            shrink_inner(&mut inner);
        }
        if inner.digit_count == 1 && digit_public(&inner, 0) == 0 {
            reset_inner(&mut inner);
        }
    }

    // ---------------------------------------------------------------------
    // Rendering and conversions
    // ---------------------------------------------------------------------

    /// Returns the decimal representation with `separator` inserted every three
    /// digits from the right. A `'\0'` separator means no separator.
    pub fn to_string_with_separator(&self, separator: char) -> String {
        self.render(Some(separator))
    }

    /// Returns the backing BCD bytes, including any sentinel bytes.
    ///
    /// The buffer is little-endian and packed exactly like the reference's private
    /// `_bytes`. This member has no C# counterpart.
    pub fn to_bcd_bytes(&self) -> Vec<u8> {
        self.lock().bytes.clone()
    }

    /// Returns the value as little-endian base-2^32 limbs, with no leading zero limb.
    ///
    /// Zero is the empty vector. This member has no C# counterpart.
    pub fn to_u32_limbs(&self) -> Vec<u32> {
        let text = self.to_string();
        let mut limbs: Vec<u32> = Vec::new();
        for byte in text.bytes() {
            let mut carry = (byte - b'0') as u64;
            for limb in limbs.iter_mut() {
                let current = (*limb as u64) * 10 + carry;
                *limb = (current & 0xFFFF_FFFF) as u32;
                carry = current >> 32;
            }
            if carry > 0 {
                limbs.push(carry as u32);
            }
        }
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        limbs
    }

    /// Number of little-endian base-2^32 limbs in this value; zero has none.
    ///
    /// This member has no C# counterpart; it is the indexed form of
    /// `to_u32_limbs` for the binary-limb side of the representation contract.
    pub fn limb_count(&self) -> usize {
        self.to_u32_limbs().len()
    }

    /// Returns the little-endian base-2^32 limb at `index`, or 0 when out of range.
    ///
    /// This member has no C# counterpart; it is the indexed form of
    /// `to_u32_limbs` for the binary-limb side of the representation contract.
    pub fn get_u32_limb(&self, index: usize) -> u32 {
        self.to_u32_limbs().get(index).copied().unwrap_or(0)
    }

    /// Returns the value as a `Nat` through its canonical decimal rendering.
    ///
    /// This member has no C# counterpart; it is the bridge between this decimal
    /// store and the crate's binary-limb natural numbers.
    pub fn to_nat(&self) -> Nat {
        Nat::from_decimal(&self.to_string()).expect("DigitStore renders canonical decimal")
    }

    /// Debug helper: prints the byte count, digit count, zero flag, value and,
    /// when `show_nibbles`, every packed nibble pair. The reference spells this
    /// member `Dump(bool showNibbles = false)`.
    pub fn dump(&self, show_nibbles: bool) {
        let (bytes, digit_count, is_zero) = {
            let inner = self.lock();
            (inner.bytes.clone(), inner.digit_count, inner.is_zero)
        };
        println!("ByteCount    : {}", bytes.len());
        println!("DigitCount   : {digit_count}");
        println!("IsZero       : {is_zero}");
        println!("Value        : {}", self.to_string_with_separator('.'));
        if show_nibbles {
            for (index, packed) in bytes.iter().enumerate() {
                println!("_bytes[{index}] low  (1): {}", packed & 0x0F);
                println!("_bytes[{index}] high (2): {}", (packed & 0xF0) >> 4);
            }
        }
        println!();
    }

    fn render(&self, separator: Option<char>) -> String {
        let inner = self.lock();
        if inner.is_zero || inner.digit_count == 0 {
            return "0".to_string();
        }
        let mut raw = String::with_capacity(inner.digit_count);
        for position in (0..inner.digit_count).rev() {
            let digit = digit_public(&inner, position as i64);
            raw.push(char::from(b'0' + digit));
        }
        match separator {
            None | Some('\0') => raw,
            Some(separator) => {
                let length = raw.len();
                let mut out = String::with_capacity(length + length / 3);
                for (index, character) in raw.chars().enumerate() {
                    let distance_from_right = length - 1 - index;
                    out.push(character);
                    if distance_from_right > 0 && distance_from_right % 3 == 0 {
                        out.push(separator);
                    }
                }
                out
            }
        }
    }
}

// -------------------------------------------------------------------------
// Free helpers over the locked inner state
// -------------------------------------------------------------------------

fn unpack(packed: u8) -> (u8, u8) {
    (packed >> 4, packed & 0x0F)
}

fn digit_public(inner: &Inner, position: i64) -> u8 {
    if inner.is_zero || position < 0 || position as usize >= inner.digit_count {
        return 0;
    }
    let packed = inner.bytes.get(position as usize / 2).copied().unwrap_or(0);
    if position as usize % 2 == 0 {
        packed >> 4
    } else {
        packed & 0x0F
    }
}

fn set_bitwise_inner(inner: &mut Inner, pos: i64, high: u8, low: u8) {
    if pos < 0 {
        return;
    }
    let pos = pos as usize;
    if pos <= inner.bytes.len() {
        let packed = high.wrapping_shl(4) | (low & 0x0F);
        if pos == inner.bytes.len() {
            inner.bytes.push(SENTINEL_ALLOCATED);
        }
        inner.bytes[pos] = packed;
    }
}

fn shrink_inner(inner: &mut Inner) {
    if inner.digit_count % 2 == 1 {
        inner.bytes.pop();
    } else if let Some(last) = inner.bytes.last_mut() {
        *last = (*last & 0xF0) | SENTINEL_FREED;
    }
    inner.digit_count = inner.digit_count.saturating_sub(1);
}

fn reset_inner(inner: &mut Inner) {
    if !inner.is_zero {
        inner.bytes.clear();
        inner.digit_count = 0;
        inner.is_zero = true;
    }
}

fn snapshot_locked(inner: &Inner) -> Vec<u8> {
    if inner.is_zero {
        return Vec::new();
    }
    let mut snapshot = vec![0u8; inner.digit_count];
    for byte_index in 0..inner.digit_count.div_ceil(2) {
        let (high, low) = unpack(inner.bytes.get(byte_index).copied().unwrap_or(0));
        snapshot[byte_index * 2] = high;
        let odd_position = byte_index * 2 + 1;
        if odd_position < inner.digit_count {
            snapshot[odd_position] = low;
        }
    }
    snapshot
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn store_with(digits: &[u8]) -> DigitStore {
        let store = DigitStore::new();
        for (position, &digit) in digits.iter().enumerate() {
            store.set_digit(position as i64, digit).unwrap();
        }
        store
    }

    fn build_store(digit_count: usize) -> DigitStore {
        let store = DigitStore::new();
        for position in 0..digit_count {
            store
                .set_digit(position as i64, (position % 10) as u8)
                .unwrap();
        }
        store
    }

    fn expected_string(digit_count: usize) -> String {
        (0..digit_count)
            .map(|index| char::from(b'0' + ((digit_count - 1 - index) % 10) as u8))
            .collect()
    }

    fn separated(raw: &str, separator: char) -> String {
        let length = raw.len();
        let mut out = String::with_capacity(length + length / 3);
        for (index, character) in raw.chars().enumerate() {
            let distance_from_right = length - 1 - index;
            out.push(character);
            if distance_from_right > 0 && distance_from_right % 3 == 0 {
                out.push(separator);
            }
        }
        out
    }

    // =========================================================
    // DigitStoreTests (44 census cases)
    // =========================================================

    #[test]
    fn Constructor_GivenDefault_DigitCountIsZero() {
        assert_eq!(DigitStore::new().digit_count(), 0);
    }

    #[test]
    fn Constructor_GivenDefault_IsZeroIsTrue() {
        assert!(DigitStore::new().is_zero());
    }

    #[test]
    fn Constructor_GivenDefault_ByteCountIsZero() {
        assert_eq!(DigitStore::new().byte_count(), 0);
    }

    #[test]
    fn CopyConstructor_GivenNonZeroSource_ProducesBitwiseIdenticalStore() {
        let source = store_with(&[3, 7, 1]);
        let copy = source.clone();
        assert_eq!(source.digit_count(), copy.digit_count());
        assert_eq!(source.is_zero(), copy.is_zero());
        assert_eq!(source.byte_count(), copy.byte_count());
        for position in 0..source.digit_count() as i64 {
            assert_eq!(source.get_digit(position), copy.get_digit(position));
        }
        for byte_index in 0..source.byte_count() as i64 {
            assert_eq!(source.get_bitwise(byte_index), copy.get_bitwise(byte_index));
        }
    }

    #[test]
    fn CopyConstructor_GivenZeroSource_ProducesZeroInstanceWithNoBytes() {
        let copy = DigitStore::new().clone();
        assert!(copy.is_zero());
        assert_eq!(copy.byte_count(), 0);
    }

    #[test]
    fn CopyConstructor_GivenSelf_DoesNotCorruptState() {
        let store = store_with(&[5]);
        let digit_count = store.digit_count();
        let is_zero = store.is_zero();
        let digit = store.get_digit(0);
        let copy = store.clone();
        store.copy_digits_from(&store);
        assert_eq!(digit_count, store.digit_count());
        assert_eq!(is_zero, store.is_zero());
        assert_eq!(digit, store.get_digit(0));
        assert_eq!(copy.get_digit(0), 5);
    }

    #[test]
    fn GetBitwise_GivenPackedByte_ExtractsHighNibbleCorrectly() {
        let store = store_with(&[9]);
        assert_eq!(store.get_bitwise(0).0, 9);
    }

    #[test]
    fn GetBitwise_GivenPackedByte_ExtractsLowNibbleCorrectly() {
        let store = store_with(&[4, 6]);
        assert_eq!(store.get_bitwise(0).1, 6);
    }

    #[test]
    fn SetBitwise_GivenTwoNibbles_PacksIntoExpectedByte() {
        let store = store_with(&[3, 7]);
        assert_eq!(store.get_bitwise(0), (3, 7));
    }

    #[test]
    fn SetBitwise_GivenPositionEqualsByteCount_CallsGrowDigits() {
        let store = DigitStore::new();
        let bytes_before = store.byte_count();
        store.set_bitwise(0, 5, 3);
        assert_eq!(store.byte_count(), bytes_before + 1);
        assert_eq!(store.get_bitwise(0), (5, 3));
    }

    #[test]
    fn SetBitwise_GivenPositionBeyondByteCountPlusOne_DoesNotWrite() {
        let store = DigitStore::new();
        store.set_bitwise(2, 5, 3);
        assert_eq!(store.byte_count(), 0);
    }

    #[test]
    fn GrowDigits_GivenCall_IncreasesByteCountByOne() {
        let store = DigitStore::new();
        let before = store.byte_count();
        store.grow_digits();
        assert_eq!(store.byte_count(), before + 1);
    }

    #[test]
    fn GrowDigits_GivenCall_NewByteLowNibbleIsSentinel0x0C() {
        let store = DigitStore::new();
        store.grow_digits();
        assert_eq!(store.get_bitwise(0), (0, 0x0C));
    }

    #[test]
    fn ShrinkDigits_GivenOddDigitCount_RemovesLastByteAndDecrementsCount() {
        let store = store_with(&[5]);
        let bytes_before = store.byte_count();
        let count_before = store.digit_count();
        store.shrink_digits();
        assert_eq!(store.byte_count(), bytes_before - 1);
        assert_eq!(store.digit_count(), count_before - 1);
    }

    #[test]
    fn ShrinkDigits_GivenEvenDigitCount_SetsLowNibbleToFifteenAndDecrementsCount() {
        let store = store_with(&[5, 3]);
        let bytes_before = store.byte_count();
        let count_before = store.digit_count();
        store.shrink_digits();
        assert_eq!(store.byte_count(), bytes_before);
        assert_eq!(store.digit_count(), count_before - 1);
        assert_eq!(store.get_bitwise(0).1, 0x0F);
    }

    #[test]
    fn GetDigit_GivenIsZeroTrue_ReturnsZeroForAnyPosition() {
        let store = DigitStore::new();
        assert_eq!(store.get_digit(0), 0);
        assert_eq!(store.get_digit(5), 0);
    }

    #[test]
    fn GetDigit_GivenEvenPosition_ReturnsHighNibbleOfByteAtHalfPosition() {
        let store = store_with(&[7]);
        assert_eq!(store.get_digit(0), 7);
    }

    #[test]
    fn GetDigit_GivenOddPosition_ReturnsLowNibbleOfByteAtHalfPosition() {
        let store = store_with(&[4, 9]);
        assert_eq!(store.get_digit(1), 9);
    }

    #[test]
    fn GetDigit_GivenPositionAtOrBeyondDigitCount_ReturnsZero() {
        let store = store_with(&[3]);
        assert_eq!(store.get_digit(1), 0);
        assert_eq!(store.get_digit(100), 0);
    }

    #[test]
    fn GetDigit_GivenTwoConsecutiveDigits_BothRoundTripCorrectly() {
        let store = store_with(&[5, 8]);
        assert_eq!(store.get_digit(0), 5);
        assert_eq!(store.get_digit(1), 8);
    }

    #[test]
    fn SetDigit_GivenPositionZero_SetsIsZeroToFalse() {
        let store = DigitStore::new();
        assert!(store.is_zero());
        store.set_digit(0, 3).unwrap();
        assert!(!store.is_zero());
    }

    #[test]
    fn SetDigit_GivenNewPosition_IncrementsDigitCount() {
        let store = DigitStore::new();
        assert_eq!(store.digit_count(), 0);
        store.set_digit(0, 1).unwrap();
        assert_eq!(store.digit_count(), 1);
        store.set_digit(1, 2).unwrap();
        assert_eq!(store.digit_count(), 2);
    }

    #[test]
    fn SetDigit_GivenEvenPosition_StoresDigitInHighNibble() {
        let store = store_with(&[6]);
        assert_eq!(store.get_bitwise(0).0, 6);
    }

    #[test]
    fn SetDigit_GivenOddPosition_StoresDigitInLowNibble() {
        let store = store_with(&[2, 8]);
        assert_eq!(store.get_bitwise(0).1, 8);
    }

    #[test]
    fn SetDigit_GivenOddPositionOnNewByte_InitializesHighNibbleToFifteen() {
        let store = store_with(&[4]);
        assert_eq!(store.get_bitwise(0), (4, 0x0F));
    }

    #[test]
    fn SetDigit_GivenPositionBeyondDigitCountByMoreThanOne_DoesNotWrite() {
        let store = DigitStore::new();
        assert!(matches!(
            store.set_digit(2, 5),
            Err(DigitStoreError::PositionOutOfRange {
                position: 2,
                digit_count: 0
            })
        ));
        assert_eq!(store.digit_count(), 0);
    }

    #[test]
    fn SetDigit_GivenPositionNonZeroOnly_IsZeroRemainsTrue() {
        let store = DigitStore::new();
        assert!(store.is_zero());
        store.set_digit(0, 7).unwrap();
        assert!(!store.is_zero());
        store.set_digit(1, 3).unwrap();
        assert!(!store.is_zero());
    }

    #[test]
    fn ByteCount_GivenNoDigits_ReturnsZero() {
        assert_eq!(DigitStore::new().byte_count(), 0);
    }

    #[test]
    fn ByteCount_GivenOneDigit_ReturnsOne() {
        assert_eq!(store_with(&[3]).byte_count(), 1);
    }

    #[test]
    fn ByteCount_GivenTwoDigits_ReturnsOne() {
        assert_eq!(store_with(&[3, 7]).byte_count(), 1);
    }

    #[test]
    fn ByteCount_GivenThreeDigits_ReturnsTwo() {
        assert_eq!(store_with(&[3, 7, 1]).byte_count(), 2);
    }

    #[test]
    fn DigitCount_GivenDefault_ReturnsZero() {
        assert_eq!(DigitStore::new().digit_count(), 0);
    }

    #[test]
    fn DigitCount_GivenOneSetDigit_ReturnsOne() {
        assert_eq!(store_with(&[4]).digit_count(), 1);
    }

    #[test]
    fn DigitCount_GivenTwoSetDigitsInSequence_ReturnsTwo() {
        assert_eq!(store_with(&[4, 6]).digit_count(), 2);
    }

    #[test]
    fn IsZero_GivenDefault_ReturnsTrue() {
        assert!(DigitStore::new().is_zero());
    }

    #[test]
    fn IsZero_GivenDigitSetAtPositionZero_ReturnsFalse() {
        let store = store_with(&[9]);
        assert!(!store.is_zero());
    }

    #[test]
    fn IsZero_GivenDigitSetAtPositionNonZeroOnly_RemainsTrue() {
        let store = DigitStore::new();
        assert!(store.is_zero());
        store.set_digit(0, 5).unwrap();
        assert!(!store.is_zero());
        store.set_digit(1, 3).unwrap();
        assert!(!store.is_zero());
    }

    #[test]
    fn ToString_GivenIsZeroTrue_ReturnsStringZero() {
        assert_eq!(DigitStore::new().to_string(), "0");
    }

    #[test]
    fn ToString_GivenSingleDigit_ReturnsThatDigitAsString() {
        assert_eq!(store_with(&[7]).to_string(), "7");
    }

    #[test]
    fn ToString_GivenOddDigitCount_PrintsHighNibbleOfLastByte() {
        assert_eq!(store_with(&[3, 2, 1]).to_string(), "123");
    }

    #[test]
    fn ToString_GivenEvenDigitCount_PrintsBothNibblesOfLastByte() {
        assert_eq!(store_with(&[4, 3, 2, 1]).to_string(), "1234");
    }

    #[test]
    fn ToString_GivenMultipleBytes_IteratesInReverseOrder() {
        assert_eq!(store_with(&[5, 4, 3, 2, 1]).to_string(), "12345");
    }

    #[test]
    fn ToString_GivenSeparator_InsertsCharacterEveryThreeDigitsFromRight() {
        assert_eq!(
            store_with(&[7, 6, 5, 4, 3, 2, 1]).to_string_with_separator(','),
            "1,234,567"
        );
    }

    #[test]
    fn ToString_GivenSeparatorAndLessThanFourDigits_NoSeparatorInserted() {
        assert_eq!(store_with(&[3, 2, 1]).to_string_with_separator(','), "123");
    }

    // =========================================================
    // DigitStoreSnapshotDigitsTests (13 census cases)
    // =========================================================

    #[test]
    fn SnapshotDigits_GivenZeroStore_ReturnsEmptyArray() {
        assert!(DigitStore::new().snapshot_digits().is_empty());
    }

    #[test]
    fn SnapshotDigits_GivenSingleDigit_ReturnsSingleElementArray() {
        assert_eq!(store_with(&[5]).snapshot_digits(), vec![5]);
    }

    #[test]
    fn SnapshotDigits_GivenPairedDigitsInSameByte_CorrectlyUnpacksBothNibbles() {
        assert_eq!(store_with(&[4, 9]).snapshot_digits(), vec![4, 9]);
    }

    #[test]
    fn SnapshotDigits_GivenMultipleDigits_ReturnsAllDigitsLeastSignificantFirst() {
        assert_eq!(
            store_with(&[3, 7, 2, 5]).snapshot_digits(),
            vec![3, 7, 2, 5]
        );
    }

    #[test]
    fn SnapshotDigits_GivenTenDigitSequence_ReturnsIdenticalSequence() {
        let sequences: [[u8; 10]; 3] = [
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            [1, 0, 1, 0, 1, 0, 1, 0, 1, 0],
        ];
        for digits in sequences {
            let store = store_with(&digits);
            assert_eq!(store.snapshot_digits(), digits.to_vec());
        }
    }

    #[test]
    fn SnapshotDigits_ReturnsCopyNotReference_MutatingSnapshotDoesNotAffectStore() {
        let store = store_with(&[7]);
        let mut snapshot = store.snapshot_digits();
        snapshot[0] = 0xFF;
        assert_eq!(store.get_digit(0), 7);
    }

    #[test]
    fn SnapshotDigits_LengthEqualsDigitCount() {
        for count in [1usize, 3, 7, 11] {
            let store = build_store(count);
            assert_eq!(store.snapshot_digits().len(), store.digit_count());
        }
    }

    #[test]
    fn SnapshotDigits_GivenConcurrentReads_AllSnapshotsAreIdentical() {
        let expected: Vec<u8> = (0..20).map(|index| (index % 10) as u8).collect();
        let store = Arc::new(store_with(&expected));
        let mut handles = Vec::new();
        for _ in 0..32 {
            let store = Arc::clone(&store);
            let expected = expected.clone();
            handles.push(std::thread::spawn(move || {
                assert_eq!(store.snapshot_digits(), expected);
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }

    // =========================================================
    // DigitStoreResetTests (7 census cases)
    // =========================================================

    #[test]
    fn Reset_GivenNonZeroStore_IsZeroBecomesTrue() {
        let store = store_with(&[5]);
        store.reset();
        assert!(store.is_zero());
    }

    #[test]
    fn Reset_GivenNonZeroStore_DigitCountBecomesZero() {
        let store = store_with(&[7]);
        store.reset();
        assert_eq!(store.digit_count(), 0);
    }

    #[test]
    fn Reset_GivenNonZeroStore_ByteCountBecomesZero() {
        let store = store_with(&[3]);
        store.reset();
        assert_eq!(store.byte_count(), 0);
    }

    #[test]
    fn Reset_GivenNonZeroStore_GetDigitReturnsZero() {
        let store = store_with(&[9]);
        store.reset();
        assert_eq!(store.get_digit(0), 0);
    }

    #[test]
    fn Reset_GivenMultiDigitStore_ClearsEverything() {
        let store = store_with(&[1, 2, 3]);
        store.reset();
        assert!(store.is_zero());
        assert_eq!(store.digit_count(), 0);
        assert_eq!(store.byte_count(), 0);
    }

    #[test]
    fn Reset_GivenAlreadyZeroStore_RemainsZero() {
        let store = DigitStore::new();
        store.reset();
        assert!(store.is_zero());
        assert_eq!(store.digit_count(), 0);
    }

    #[test]
    fn Reset_ThenSetDigit_StoreIsReusable() {
        let store = store_with(&[4]);
        store.reset();
        store.set_digit(0, 6).unwrap();
        store.set_digit(1, 8).unwrap();
        assert!(!store.is_zero());
        assert_eq!(store.digit_count(), 2);
        assert_eq!(store.get_digit(0), 6);
        assert_eq!(store.get_digit(1), 8);
    }

    // =========================================================
    // DigitStoreCopyDigitsFromTests (6 census cases)
    // =========================================================

    #[test]
    fn CopyDigitsFrom_GivenFourDigitSource_ByteCountAndDigitsMatchSource() {
        let source = store_with(&[3, 7, 1, 5]);
        let destination = source.clone();
        let updated = store_with(&[6, 2, 8, 4]);
        destination.copy_digits_from(&updated);
        assert_eq!(destination.byte_count(), 2);
        assert_eq!(destination.digit_count(), 4);
        assert_eq!(destination.get_digit(0), 6);
        assert_eq!(destination.get_digit(1), 2);
        assert_eq!(destination.get_digit(2), 8);
        assert_eq!(destination.get_digit(3), 4);
    }

    #[test]
    fn CopyDigitsFrom_GivenThreeDigitSource_ByteCountAndDigitsMatchSource() {
        let source = store_with(&[1, 2, 9]);
        let destination = source.clone();
        let updated = store_with(&[4, 5, 7]);
        destination.copy_digits_from(&updated);
        assert_eq!(destination.byte_count(), 2);
        assert_eq!(destination.digit_count(), 3);
        assert_eq!(destination.get_digit(0), 4);
        assert_eq!(destination.get_digit(1), 5);
        assert_eq!(destination.get_digit(2), 7);
    }

    #[test]
    fn CopyDigitsFrom_GivenZeroSource_DestinationBytesNotOverwritten() {
        let destination = store_with(&[8, 3]);
        let zero_source = DigitStore::new();
        destination.copy_digits_from(&zero_source);
        assert_eq!(destination.byte_count(), 1);
        assert_eq!(destination.digit_count(), 2);
        assert_eq!(destination.get_digit(0), 8);
        assert_eq!(destination.get_digit(1), 3);
    }

    #[test]
    fn CopyDigitsFrom_GivenNonZeroSource_MutatingSourceAfterCopyDoesNotAffectDest() {
        let source = store_with(&[5, 9]);
        let destination = source.clone();
        destination.copy_digits_from(&source);
        source.set_digit(0, 0).unwrap();
        source.set_digit(1, 0).unwrap();
        assert_eq!(destination.get_digit(0), 5);
        assert_eq!(destination.get_digit(1), 9);
    }

    #[test]
    fn CopyDigitsFrom_GivenTenDigitSource_AllDigitsPreserved() {
        let expected: [u8; 10] = [2, 4, 6, 8, 1, 3, 5, 7, 9, 0];
        let source = store_with(&expected);
        let destination = source.clone();
        let expected2: [u8; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];
        let source2 = store_with(&expected2);
        destination.copy_digits_from(&source2);
        assert_eq!(destination.byte_count(), 5);
        for (position, &digit) in expected2.iter().enumerate() {
            assert_eq!(destination.get_digit(position as i64), digit);
        }
    }

    #[test]
    fn CopyDigitsFrom_CrossCopyConcurrent_NeitherDeadlocksNorThrows() {
        let a = Arc::new(store_with(&[1, 2]));
        let b = Arc::new(store_with(&[3, 4]));
        let dest_a = Arc::new(a.as_ref().clone());
        let dest_b = Arc::new(b.as_ref().clone());
        let thread_a = {
            let destination = Arc::clone(&dest_a);
            let source = Arc::clone(&b);
            std::thread::spawn(move || {
                for _ in 0..500 {
                    destination.copy_digits_from(&source);
                }
            })
        };
        let thread_b = {
            let destination = Arc::clone(&dest_b);
            let source = Arc::clone(&a);
            std::thread::spawn(move || {
                for _ in 0..500 {
                    destination.copy_digits_from(&source);
                }
            })
        };
        thread_a.join().unwrap();
        thread_b.join().unwrap();
        assert_eq!(dest_a.byte_count(), 1);
        assert_eq!(dest_b.byte_count(), 1);
    }

    // =========================================================
    // DigitStoreMutatorTests (5 census cases)
    // =========================================================

    #[test]
    fn SetDigitCount_GivenPositiveValue_DigitCountIsUpdated() {
        let store = DigitStore::new();
        store.set_digit_count(7);
        assert_eq!(store.digit_count(), 7);
    }

    #[test]
    fn SetDigitCount_GivenZero_DigitCountIsZero() {
        let store = store_with(&[3]);
        store.set_digit_count(0);
        assert_eq!(store.digit_count(), 0);
    }

    #[test]
    fn SetDigitCount_ConcurrentCalls_FinalValueIsOneOfTheWrittenValues() {
        let store = Arc::new(DigitStore::new());
        let mut handles = Vec::new();
        for value in 0..200usize {
            let store = Arc::clone(&store);
            handles.push(std::thread::spawn(move || store.set_digit_count(value)));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert!(store.digit_count() < 200);
    }

    #[test]
    fn SetIsZero_GivenTrue_IsZeroBecomesTrue() {
        let store = store_with(&[5]);
        store.set_is_zero(true);
        assert!(store.is_zero());
    }

    #[test]
    fn SetIsZero_GivenFalse_IsZeroBecomesFalse() {
        let store = DigitStore::new();
        store.set_is_zero(false);
        assert!(!store.is_zero());
    }

    // =========================================================
    // DigitStoreTrimLeadingZerosTests (6 census cases)
    // =========================================================

    #[test]
    fn TrimLeadingZeros_GivenMultipleLeadingZeroDigits_DigitCountReducedAndValuePreserved() {
        let store = store_with(&[5, 0, 0]);
        store.trim_leading_zeros();
        assert_eq!(store.digit_count(), 1);
        assert_eq!(store.get_digit(0), 5);
        assert!(!store.is_zero());
    }

    #[test]
    fn TrimLeadingZeros_GivenEvenDigitCountWithSingleLeadingZero_TrimsToSingleDigit() {
        let store = store_with(&[4, 0]);
        store.trim_leading_zeros();
        assert_eq!(store.digit_count(), 1);
        assert_eq!(store.get_digit(0), 4);
    }

    #[test]
    fn TrimLeadingZeros_GivenNoLeadingZeros_LeavesValueUnchanged() {
        let store = store_with(&[3, 2, 1]);
        store.trim_leading_zeros();
        assert_eq!(store.digit_count(), 3);
        assert_eq!(store.to_string(), "123");
    }

    #[test]
    fn TrimLeadingZeros_GivenSingleNonZeroDigit_LeavesValueUnchanged() {
        let store = store_with(&[7]);
        store.trim_leading_zeros();
        assert_eq!(store.digit_count(), 1);
        assert_eq!(store.get_digit(0), 7);
    }

    #[test]
    fn TrimLeadingZeros_GivenAllZeroDigits_ResetsToZeroState() {
        let store = store_with(&[0, 0, 0]);
        assert!(!store.is_zero());
        store.trim_leading_zeros();
        assert!(store.is_zero());
    }

    #[test]
    fn TrimLeadingZeros_GivenSingleZeroDigit_ResetsToZeroState() {
        let store = store_with(&[0]);
        assert!(!store.is_zero());
        store.trim_leading_zeros();
        assert!(store.is_zero());
    }

    // =========================================================
    // DigitStoreToStringParallelTests (5 census cases)
    // =========================================================

    #[test]
    fn ToString_Given20Digits_ReturnsCorrectDigitOrder() {
        let expected = expected_string(20);
        assert_eq!(expected, "98765432109876543210");
        assert_eq!(build_store(20).to_string(), expected);
    }

    #[test]
    fn ToString_Given100Digits_ReturnsCorrectDigitOrder() {
        let expected = expected_string(100);
        assert_eq!(build_store(100).to_string(), expected);
    }

    #[test]
    fn ToString_Given19Digits_ReturnsCorrectDigitOrder() {
        let expected = expected_string(19);
        assert_eq!(expected, "8765432109876543210");
        assert_eq!(build_store(19).to_string(), expected);
    }

    #[test]
    fn ToString_Given99Digits_ReturnsCorrectDigitOrder() {
        let expected = expected_string(99);
        assert_eq!(build_store(99).to_string(), expected);
    }

    #[test]
    fn ToString_Given20DigitsWithSeparator_ReturnsCorrectSeparatedString() {
        let raw = expected_string(20);
        assert_eq!(
            build_store(20).to_string_with_separator(','),
            separated(&raw, ',')
        );
        assert_eq!(
            build_store(20).to_string_with_separator(','),
            "98,765,432,109,876,543,210"
        );
    }

    // =========================================================
    // DigitStoreThreadSafetyTests (4 census cases)
    // =========================================================

    #[test]
    fn TrimLeadingZeros_ConcurrentWithGetDigit_NeverThrowsAndDigitsInRange() {
        let store = Arc::new(store_with(&[5, 0, 0]));
        let reader = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    let digit = store.get_digit(0);
                    assert!(digit <= 9);
                }
            })
        };
        let trimmer = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    store.set_digit(0, 5).unwrap();
                    store.set_digit(1, 0).unwrap();
                    store.set_digit(2, 0).unwrap();
                    store.trim_leading_zeros();
                }
            })
        };
        reader.join().unwrap();
        trimmer.join().unwrap();
        assert_eq!(store.get_digit(0), 5);
    }

    #[test]
    fn ToString_ConcurrentWithReset_AlwaysReturnsValidDecimalString() {
        let store = Arc::new(store_with(&[1, 2, 3]));
        let reader = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    let text = store.to_string();
                    assert!(!text.is_empty());
                    assert!(text.bytes().all(|byte| byte.is_ascii_digit()));
                    assert!(
                        text.len() == 1 || !text.starts_with('0'),
                        "leading zero in {text}"
                    );
                }
            })
        };
        let resetter = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    store.reset();
                    store.set_digit(0, 1).unwrap();
                    store.set_digit(1, 2).unwrap();
                    store.set_digit(2, 3).unwrap();
                }
            })
        };
        reader.join().unwrap();
        resetter.join().unwrap();
    }

    #[test]
    fn CopyDigitsFrom_ConcurrentToSameDestination_NeverThrowsAndFinalStateValid() {
        let source = Arc::new(store_with(&[4, 2]));
        let destination = Arc::new(source.as_ref().clone());
        let first = {
            let destination = Arc::clone(&destination);
            let source = Arc::clone(&source);
            std::thread::spawn(move || {
                for _ in 0..500 {
                    destination.copy_digits_from(&source);
                }
            })
        };
        let second = {
            let destination = Arc::clone(&destination);
            let source = Arc::clone(&source);
            std::thread::spawn(move || {
                for _ in 0..500 {
                    destination.copy_digits_from(&source);
                }
            })
        };
        first.join().unwrap();
        second.join().unwrap();
        assert_eq!(destination.byte_count(), 1);
        assert_eq!(destination.digit_count(), 2);
        assert_eq!(destination.get_digit(0), 4);
        assert_eq!(destination.get_digit(1), 2);
    }

    #[test]
    fn CopyDigitsFrom_SelfCopyUnderConcurrentRead_NeverCorruptsDigit() {
        let store = Arc::new(store_with(&[7]));
        let reader = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    assert_eq!(store.get_digit(0), 7);
                }
            })
        };
        let self_copier = {
            let store = Arc::clone(&store);
            std::thread::spawn(move || {
                for _ in 0..1_000 {
                    store.copy_digits_from(&store);
                }
            })
        };
        reader.join().unwrap();
        self_copier.join().unwrap();
    }

    // =========================================================
    // Additive conversion coverage (not census-mapped)
    // =========================================================

    #[test]
    fn FromDecimal_GivenLeadingZeros_CanonicalisesAndRejectsNonDigits() {
        let store = DigitStore::from_decimal("000123").unwrap();
        assert_eq!(store.to_string(), "123");
        assert_eq!(store.digit_count(), 3);
        assert_eq!(store.byte_count(), 2);
        let zero = DigitStore::from_decimal("0000").unwrap();
        assert!(zero.is_zero());
        assert_eq!(zero.digit_count(), 0);
        assert_eq!(zero.byte_count(), 0);
        assert!(DigitStore::from_decimal("").is_none());
        assert!(DigitStore::from_decimal("  ").is_none());
        assert!(DigitStore::from_decimal("-5").is_none());
        assert!(DigitStore::from_decimal("12a").is_none());
        assert_eq!(DigitStore::from_decimal(" 42 ").unwrap().to_string(), "42");
    }

    #[test]
    fn FromDigits_GivenLeadingZeroDigits_TrimsToCanonicalValue() {
        let store = DigitStore::from_digits(&[1, 2, 0, 0]);
        assert_eq!(store.to_string(), "21");
        assert_eq!(store.digit_count(), 2);
        assert_eq!(store.byte_count(), 1);
        let zero = DigitStore::from_digits(&[0, 0, 0]);
        assert!(zero.is_zero());
        assert_eq!(zero.byte_count(), 0);
    }

    #[test]
    fn FromBcdBytes_GivenCanonicalBytes_RoundTripsValue() {
        let store = store_with(&[5, 4, 3, 2, 1]);
        let bytes = store.to_bcd_bytes();
        assert_eq!(bytes, vec![0x54, 0x32, 0x1F]);
        let decoded = DigitStore::from_bcd_bytes(&bytes).unwrap();
        assert_eq!(decoded.to_string(), "12345");
        assert_eq!(decoded.digit_count(), 5);
        assert_eq!(decoded.byte_count(), 3);
        let padded = DigitStore::from_bcd_bytes(&[0x54, 0x32, 0x00]).unwrap();
        assert_eq!(padded.to_string(), "2345");
        let allocated = DigitStore::from_bcd_bytes(&[0x0C]).unwrap();
        assert!(allocated.is_zero());
        assert_eq!(allocated.byte_count(), 0);
    }

    #[test]
    fn FromBcdBytes_GivenNonDecimalNibble_ReturnsNone() {
        assert!(DigitStore::from_bcd_bytes(&[0x0A]).is_none());
        assert!(DigitStore::from_bcd_bytes(&[0x5A]).is_none());
        assert!(DigitStore::from_bcd_bytes(&[0xE5]).is_none());
    }

    #[test]
    fn FromU32Limbs_GivenLeadingZeroLimbs_NormalisesAndRoundTrips() {
        let value = DigitStore::from_u32_limbs(&[u32::MAX, 0]);
        assert_eq!(value.to_string(), "4294967295");
        assert_eq!(value.to_u32_limbs(), vec![u32::MAX]);
        let wide = DigitStore::from_u32_limbs(&[1, 1]);
        assert_eq!(wide.to_string(), "4294967297");
        assert_eq!(wide.to_u32_limbs(), vec![1, 1]);
        let zero = DigitStore::from_u32_limbs(&[0, 0]);
        assert!(zero.is_zero());
        assert!(zero.to_u32_limbs().is_empty());
    }

    #[test]
    fn U32LimbAccess_GivenValue_ReturnsLittleEndianLimbsByIndex() {
        let store = DigitStore::from_u32_limbs(&[1, 2, 3]);
        assert_eq!(store.limb_count(), 3);
        assert_eq!(store.get_u32_limb(0), 1);
        assert_eq!(store.get_u32_limb(1), 2);
        assert_eq!(store.get_u32_limb(2), 3);
        assert_eq!(store.get_u32_limb(9), 0);
        assert_eq!(DigitStore::new().limb_count(), 0);
        assert_eq!(
            DigitStore::from_u32_limbs(&[u32::MAX]).get_u32_limb(0),
            u32::MAX
        );
    }

    #[test]
    fn NatConversions_GivenCanonicalValue_RoundTrip() {
        let literal = "123456789012345678901234567890";
        let nat = Nat::from_decimal(literal).unwrap();
        let store = DigitStore::from_nat(&nat);
        assert_eq!(store.to_string(), literal);
        assert_eq!(store.digit_count(), literal.len());
        assert_eq!(store.to_nat(), nat);
        let zero = DigitStore::from_nat(&Nat::zero());
        assert!(zero.is_zero());
        assert!(zero.to_nat().is_zero());
    }
}
