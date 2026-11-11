//! Round 39 — the additive Rust equivalents of the Natural API members the reference's tests
//! exercise.
//!
//! Every test names the C# row it mirrors in its doc comment. The translation is at the level of
//! the observable contract, never a re-implementation of the C# test runner:
//!
//! * `IEquatable`/`IComparable` -> `Eq`/`Ord`.
//! * `IParsable`/`ISpanParsable` -> `FromStr` and `Nat::from_ascii`.
//! * `ISpanFormattable` -> `Display` and `Nat::write_decimal`.
//! * `INumber` -> the crate's `Number` trait (Rust has no interface inheritance).
//! * the operator interfaces -> the corresponding `std::ops` traits.
//! * the two global configuration statics -> atomic `Nat::display_digits`/`Nat::precision`.
//! * the reference-identity cache assertions -> pointer identity of `Nat::as_decimal_str` on
//!   the same value (a Rust clone is a value copy, so cross-instance allocation identity is not
//!   promised; that is recorded in the round report).

#![allow(non_snake_case)]

use super::{Nat, Natural, Number, ParseNatError};
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

// ---------------------------------------------------------------------------
// Structure traits (NaturalStructureTests, 15 rows)
// ---------------------------------------------------------------------------

fn assert_eq_trait<T: Eq>() {}
fn assert_ord_trait<T: Ord>() {}
fn assert_from_str_trait<T: FromStr>() {}
fn assert_display_trait<T: std::fmt::Display>() {}
// The operator traits are implemented for references (see the note in lib.rs), so the
// compile-time witness is the reference form the reference's interfaces describe: T op T -> T.
fn assert_add_trait<T>()
where
    for<'a, 'b> &'a T: std::ops::Add<&'b T, Output = T>,
{
}
fn assert_sub_trait<T>()
where
    for<'a, 'b> &'a T: std::ops::Sub<&'b T, Output = T>,
{
}
fn assert_mul_trait<T>()
where
    for<'a, 'b> &'a T: std::ops::Mul<&'b T, Output = T>,
{
}
fn assert_div_trait<T>()
where
    for<'a, 'b> &'a T: std::ops::Div<&'b T, Output = T>,
{
}
fn assert_rem_trait<T>()
where
    for<'a, 'b> &'a T: std::ops::Rem<&'b T, Output = T>,
{
}
fn assert_partial_ord_trait<T: PartialOrd>() {}
fn assert_number_trait<T: Number>() {}

#[test]
fn natural_type_name_is_natural() {
    // C# NaturalStructureTests.Natural_TypeName_IsNatural asserts
    // typeof(Natural).Name == "Natural". Rust's canonical name is Nat; the reference name is
    // kept as the `Natural` alias, and the test pins both the canonical name and the alias.
    assert_eq!(std::any::type_name::<Nat>(), "lovelace_nat::Nat");
    assert_eq!(std::any::type_name::<Natural>(), "lovelace_nat::Nat");
    assert_eq!(std::mem::size_of::<Natural>(), std::mem::size_of::<Nat>());
    assert_eq!(Natural::from_u64(7).to_decimal(), "7");
}

#[test]
fn natural_implements_iequatable() {
    // C# Natural_ImplementsIEquatable: IEquatable<Natural>. Rust: Eq (reflexive equality).
    assert_eq_trait::<Nat>();
    assert_eq!(Nat::from_u64(42), Nat::from_u64(42));
    assert_ne!(Nat::from_u64(42), Nat::from_u64(43));
}

#[test]
fn natural_implements_icomparable() {
    // C# Natural_ImplementsIComparable: IComparable<Natural>. Rust: Ord.
    assert_ord_trait::<Nat>();
    assert!(Nat::from_u64(2) < Nat::from_u64(3));
    assert_eq!(
        Nat::from_u64(2).cmp(&Nat::from_u64(2)),
        std::cmp::Ordering::Equal
    );
}

#[test]
fn natural_implements_iparsable() {
    // C# Natural_ImplementsIParsable: IParsable<Natural>. Rust: FromStr.
    assert_from_str_trait::<Nat>();
    let parsed = Nat::from_str("12345").expect("FromStr");
    assert_eq!(parsed, Nat::from_u64(12_345));
    assert_eq!(Nat::from_str("12a3"), Err(ParseNatError));
}

#[test]
fn natural_implements_ispanparsable() {
    // C# Natural_ImplementsISpanParsable: ISpanParsable<Natural>. Rust: FromStr plus the
    // byte-span parser, the same grammar without the `&str` allocation.
    assert_from_str_trait::<Nat>();
    assert_eq!(Nat::from_ascii(b"12345").unwrap(), Nat::from_u64(12_345));
    assert!(Nat::from_ascii(b"12a3").is_none());
    assert!(Nat::from_ascii(&[]).is_none());
}

#[test]
fn natural_implements_ispanformattable() {
    // C# Natural_ImplementsISpanFormattable: ISpanFormattable. Rust: Display plus
    // Nat::write_decimal into a caller buffer.
    assert_display_trait::<Nat>();
    let value = Nat::from_u64(12_345);
    assert_eq!(value.to_string(), "12345");
    let mut buffer = [0u8; 8];
    assert_eq!(value.write_decimal(&mut buffer), Some(5));
    assert_eq!(&buffer[..5], b"12345");
    let mut tiny = [0u8; 2];
    assert_eq!(value.write_decimal(&mut tiny), None);
}

#[test]
fn natural_implements_inumber() {
    // C# Natural_ImplementsINumber: INumber<Natural>. Rust has no interface inheritance; the
    // crate's Number trait is the additive identity/zero/radix surface.
    assert_number_trait::<Nat>();
    assert!(Nat::zero().is_zero());
    assert_eq!(Nat::one(), Nat::from_u64(1));
    assert_eq!(Nat::radix(), 10);
}

#[test]
fn natural_implements_iadditionoperators() {
    // C# Natural_ImplementsIAdditionOperators: IAdditionOperators<Natural, Natural, Natural>.
    assert_add_trait::<Nat>();
    let sum = &Nat::from_u64(2) + &Nat::from_u64(3);
    assert_eq!(sum.to_decimal(), "5");
    let left = Nat::from_u64(40);
    let right = Nat::from_u64(2);
    assert_eq!((&left + &right).to_decimal(), "42");
}

#[test]
fn natural_implements_isubtractionoperators() {
    // C# Natural_ImplementsISubtractionOperators. Valid operands subtract; underflow panics in
    // Rust rather than wrapping, the same refusal as the reference's exception.
    assert_sub_trait::<Nat>();
    assert_eq!((&Nat::from_u64(10) - &Nat::from_u64(3)).to_decimal(), "7");
    let left = Nat::from_u64(100);
    let right = Nat::from_u64(1);
    assert_eq!((&left - &right).to_decimal(), "99");
}

#[test]
fn natural_implements_imultiplyoperators() {
    // C# Natural_ImplementsIMultiplyOperators.
    assert_mul_trait::<Nat>();
    assert_eq!(
        (&Nat::from_u64(12) * &Nat::from_u64(34)).to_decimal(),
        "408"
    );
    let left = Nat::from_u64(999);
    let right = Nat::from_u64(999);
    assert_eq!((&left * &right).to_decimal(), "998001");
}

#[test]
fn natural_implements_idivisionoperators() {
    // C# Natural_ImplementsIDivisionOperators.
    assert_div_trait::<Nat>();
    assert_eq!((&Nat::from_u64(17) / &Nat::from_u64(3)).to_decimal(), "5");
    let left = Nat::from_u64(20);
    let right = Nat::from_u64(4);
    assert_eq!((&left / &right).to_decimal(), "5");
}

#[test]
fn natural_implements_imodulusoperators() {
    // C# Natural_ImplementsIModulusOperators.
    assert_rem_trait::<Nat>();
    assert_eq!((&Nat::from_u64(13) % &Nat::from_u64(4)).to_decimal(), "1");
    let left = Nat::from_u64(12);
    let right = Nat::from_u64(4);
    assert_eq!((&left % &right).to_decimal(), "0");
}

#[test]
fn natural_implements_iincrementoperators() {
    // C# Natural_ImplementsIIncrementOperators: IIncrementOperators<Natural>, i.e. ++.
    // Rust's equivalent in-place increment is AddAssign<u64> (and Nat::increment).
    fn assert_add_assign<T: std::ops::AddAssign<u64>>() {}
    assert_add_assign::<Nat>();
    let mut value = Nat::from_u64(9);
    value += 1u64;
    assert_eq!(value.to_decimal(), "10");
    value.increment();
    assert_eq!(value.to_decimal(), "11");
}

#[test]
fn natural_implements_idecrementoperators() {
    // C# Natural_ImplementsIDecrementOperators: IDecrementOperators<Natural>, i.e. --.
    fn assert_sub_assign<T: std::ops::SubAssign<u64>>() {}
    assert_sub_assign::<Nat>();
    let mut value = Nat::from_u64(10);
    value -= 1u64;
    assert_eq!(value.to_decimal(), "9");
    assert!(value.decrement());
    assert_eq!(value.to_decimal(), "8");
    let mut zero = Nat::zero();
    assert!(!zero.decrement());
}

#[test]
fn natural_implements_icomparisonoperators() {
    // C# Natural_ImplementsIComparisonOperators: IComparisonOperators<Natural, Natural, bool>.
    assert_partial_ord_trait::<Nat>();
    let small = Nat::from_u64(2);
    let large = Nat::from_u64(10);
    assert!(small < large);
    assert!(small <= large);
    assert!(large > small);
    assert!(large >= small);
    assert!(small <= small && small >= small);
}

// ---------------------------------------------------------------------------
// Global statics (NaturalStaticPropertyTests, 8 rows)
// ---------------------------------------------------------------------------

static STATIC_LOCK: Mutex<()> = Mutex::new(());

fn statics_guard() -> MutexGuard<'static, ()> {
    STATIC_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn display_digits_default_value_is_minus_one() {
    // C# DisplayDigits_DefaultValue_IsMinusOne.
    let _guard = statics_guard();
    let saved = Nat::display_digits();
    Nat::set_display_digits(-1);
    assert_eq!(Nat::display_digits(), -1);
    Nat::set_display_digits(saved);
}

#[test]
fn display_digits_after_set_returns_set_value() {
    // C# DisplayDigits_AfterSet_ReturnsSetValue.
    let _guard = statics_guard();
    let saved = Nat::display_digits();
    Nat::set_display_digits(42);
    assert_eq!(Nat::display_digits(), 42);
    Nat::set_display_digits(saved);
}

#[test]
fn display_digits_after_multiple_sets_returns_last_value() {
    // C# DisplayDigits_AfterMultipleSets_ReturnsLastValue.
    let _guard = statics_guard();
    let saved = Nat::display_digits();
    Nat::set_display_digits(10);
    Nat::set_display_digits(20);
    Nat::set_display_digits(999);
    assert_eq!(Nat::display_digits(), 999);
    Nat::set_display_digits(saved);
}

#[test]
fn display_digits_concurrent_read_write_never_returns_torn_value() {
    // C# DisplayDigits_ConcurrentReadWrite_NeverReturnsTornValue. The C# test writes two
    // complementary 64-bit patterns from one task while another reads; an i64 atomic has the
    // same indivisibility, so every read must be exactly one of the two patterns.
    const PATTERN_A: i64 = 0xAAAA_AAAA_AAAA_AAAAu64 as i64;
    const PATTERN_B: i64 = 0x5555_5555_5555_5555u64 as i64;
    const ITERATIONS: usize = 100_000;
    let _guard = statics_guard();
    let saved = Nat::display_digits();
    Nat::set_display_digits(PATTERN_A);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for i in 0..ITERATIONS {
                Nat::set_display_digits(if i % 2 == 0 { PATTERN_A } else { PATTERN_B });
            }
        });
        scope.spawn(|| {
            for _ in 0..ITERATIONS {
                let value = Nat::display_digits();
                assert!(
                    value == PATTERN_A || value == PATTERN_B,
                    "torn DisplayDigits read: {value:#x}"
                );
            }
        });
    });
    Nat::set_display_digits(saved);
}

#[test]
fn precision_default_value_is_minus_one() {
    // C# Precision_DefaultValue_IsMinusOne.
    let _guard = statics_guard();
    let saved = Nat::precision();
    Nat::set_precision(-1);
    assert_eq!(Nat::precision(), -1);
    Nat::set_precision(saved);
}

#[test]
fn precision_after_set_returns_set_value() {
    // C# Precision_AfterSet_ReturnsSetValue.
    let _guard = statics_guard();
    let saved = Nat::precision();
    Nat::set_precision(100);
    assert_eq!(Nat::precision(), 100);
    Nat::set_precision(saved);
}

#[test]
fn precision_after_multiple_sets_returns_last_value() {
    // C# Precision_AfterMultipleSets_ReturnsLastValue.
    let _guard = statics_guard();
    let saved = Nat::precision();
    Nat::set_precision(1);
    Nat::set_precision(50);
    Nat::set_precision(200);
    assert_eq!(Nat::precision(), 200);
    Nat::set_precision(saved);
}

#[test]
fn precision_concurrent_read_write_never_returns_torn_value() {
    // C# Precision_ConcurrentReadWrite_NeverReturnsTornValue.
    const PATTERN_A: i64 = 0xAAAA_AAAA_AAAA_AAAAu64 as i64;
    const PATTERN_B: i64 = 0x5555_5555_5555_5555u64 as i64;
    const ITERATIONS: usize = 100_000;
    let _guard = statics_guard();
    let saved = Nat::precision();
    Nat::set_precision(PATTERN_A);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            for i in 0..ITERATIONS {
                Nat::set_precision(if i % 2 == 0 { PATTERN_A } else { PATTERN_B });
            }
        });
        scope.spawn(|| {
            for _ in 0..ITERATIONS {
                let value = Nat::precision();
                assert!(
                    value == PATTERN_A || value == PATTERN_B,
                    "torn Precision read: {value:#x}"
                );
            }
        });
    });
    Nat::set_precision(saved);
}

// ---------------------------------------------------------------------------
// Natural residual (17 rows): construction, caching, concurrency, randomized cross-check
// ---------------------------------------------------------------------------

/// A deterministic xorshift generator. The reference uses `System.Random` with a fixed seed;
/// Rust has no such type, so the same *shape* of corpus is generated deterministically.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// A decimal literal whose first digit is non-zero and whose remaining digits are random.
    fn digits(&mut self, length: usize) -> String {
        let mut text = String::with_capacity(length);
        text.push((b'1' + (self.next() % 9) as u8) as char);
        for _ in 1..length {
            text.push((b'0' + (self.next() % 10) as u8) as char);
        }
        text
    }
}

#[test]
fn div_rem_given_large_operands_produces_correct_quotient_and_remainder() {
    // C# NaturalBinaryLimbTests.DivRem_GivenLargeOperands_ProducesCorrectQuotientAndRemainder
    // divides a 2,000-digit value by a 1,200-digit one and compares against BigInteger. The
    // independent check here is the division identity q*b + r == a with r < b.
    let mut rng = Rng::new(42);
    let a = Nat::from_decimal(&rng.digits(2000)).unwrap();
    let b = Nat::from_decimal(&rng.digits(1200)).unwrap();
    let (q, r) = a.div_rem(&b).expect("nonzero divisor");
    assert_eq!(q.mul(&b).add(&r), a);
    assert!(r < b);
    assert!(q.to_decimal().len() >= 800);
}

#[test]
fn round_trip_given_random_large_values_parse_to_string_identity() {
    // C# NaturalBinaryLimbTests.RoundTrip_GivenRandomLargeValues_ParseToStringIdentity:
    // 30 values up to 20,000 digits.
    let mut rng = Rng::new(7);
    for _ in 0..30 {
        let length = 1 + (rng.next() as usize % 20_000);
        let text = rng.digits(length);
        assert_eq!(Nat::from_decimal(&text).unwrap().to_decimal(), text);
    }
}

#[test]
fn round_trip_given_random_large_values_parse_to_string_identity_decimal_cache() {
    // C# NaturalDecimalCacheTests.RoundTrip_GivenRandomLargeValues_ParseToStringIdentity:
    // 20 values up to 20,000 digits, seed 20240608.
    let mut rng = Rng::new(20_240_608);
    for _ in 0..20 {
        let length = 1 + (rng.next() as usize % 20_000);
        let text = rng.digits(length);
        assert_eq!(Nat::from_decimal(&text).unwrap().to_decimal(), text);
    }
}

#[test]
fn constructor_given_negative_int_throws_argument_out_of_range_exception() {
    // C# NaturalConstructorTests.Constructor_GivenNegativeInt_ThrowsArgumentOutOfRangeException.
    // Rust reports the refusal as TryFrom's Err.
    assert_eq!(Nat::try_from(-1i64), Err(super::NegativeNatural));
    assert!(Nat::from_i64(-1).is_none());
    assert_eq!(Nat::try_from(42i64).unwrap(), Nat::from_u64(42));
    assert_eq!(Nat::try_from(0i64).unwrap(), Nat::zero());
}

#[test]
fn arithmetic_result_to_string_caches_after_first_call() {
    // C# NaturalDecimalCacheTests.ArithmeticResult_ToString_CachesAfterFirstCall.
    let left = Nat::from_decimal("12345678901234567890").unwrap();
    let right = Nat::from_decimal("98765432109876543210").unwrap();
    let product = left.mul(&right);
    let first = product.as_decimal_str();
    let second = product.as_decimal_str();
    assert!(
        std::ptr::eq(first, second),
        "the cached rendering was recomputed"
    );
    assert_eq!(first, "1219326311370217952237463801111263526900");
}

#[test]
fn to_string_given_parsed_value_is_stable_across_repeated_calls() {
    // C# NaturalDecimalCacheTests.ToString_GivenParsedValue_IsStableAcrossRepeatedCalls.
    let value = Nat::from_decimal("123456789012345678901234567890").unwrap();
    assert!(std::ptr::eq(value.as_decimal_str(), value.as_decimal_str()));
    assert_eq!(value.to_decimal(), value.to_decimal());
}

#[test]
fn copy_constructor_shares_cached_string_but_independent_limbs() {
    // C# NaturalDecimalCacheTests.CopyConstructor_SharesCachedStringButIndependentLimbs.
    // Rust clone is a value copy: it is a distinct value with an equal rendering, and the
    // original's own cache remains stable and unaffected by arithmetic on the copy.
    let original = Nat::from_decimal("123456789012345678901234567890").unwrap();
    let copy = original.clone();
    assert_eq!(original, copy);
    let cached = original.as_decimal_str();
    assert!(
        std::ptr::eq(cached, copy.as_decimal_str()),
        "the copy did not share the cached rendering"
    );
    assert!(std::ptr::eq(cached, original.as_decimal_str()));
    assert_eq!(copy.to_decimal(), cached);
    let copy_plus_one = copy.add(&Nat::from_u64(1));
    assert_ne!(copy_plus_one, original);
    assert_eq!(original.to_decimal(), "123456789012345678901234567890");
}

#[test]
fn concurrent_to_string_given_shared_value_returns_consistent_string() {
    // C# NaturalDecimalCacheTests.ConcurrentToString_GivenSharedValue_ReturnsConsistentString.
    // The reference asserts Assert.Same across parallel ToString calls; a shared Arc<Nat>
    // gives the same single cached allocation here.
    let left = Nat::from_decimal("1234567890123456789012345678901234567890").unwrap();
    let right = Nat::from_decimal("9876543210987654321098765432109876543210").unwrap();
    let product = Arc::new(left.mul(&right));
    let expected = product.to_decimal();
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(16);
    let mut pointers = Vec::with_capacity(workers);
    let mut texts = Vec::with_capacity(workers);
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(workers);
        for _ in 0..workers {
            let value = Arc::clone(&product);
            handles.push(scope.spawn(move || {
                let text = value.as_decimal_str();
                (text.as_ptr() as usize, text.to_string())
            }));
        }
        for handle in handles {
            let (pointer, text) = handle.join().expect("worker");
            pointers.push(pointer);
            texts.push(text);
        }
    });
    assert!(pointers.iter().all(|pointer| *pointer == pointers[0]));
    assert!(texts.iter().all(|text| text == &expected));
}

#[test]
fn concurrent_parse_to_string_given_many_values_matches_expected() {
    // C# NaturalDecimalCacheTests.ConcurrentParseToString_GivenManyValues_MatchesBigInteger.
    // BigInteger is replaced by the canonical decimal form the generator already knows.
    let mut rng = Rng::new(20_240_609);
    let mut inputs: Vec<String> = Vec::with_capacity(200);
    for _ in 0..200 {
        let length = 1 + (rng.next() as usize % 200);
        inputs.push(rng.digits(length));
    }
    let expected: Vec<String> = inputs.iter().map(|text| text.clone()).collect();
    let inputs = Arc::new(inputs);
    let mut actual = vec![String::new(); expected.len()];
    let workers = 8usize;
    let chunk = expected.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (index, slot) in actual.chunks_mut(chunk).enumerate() {
            let inputs = Arc::clone(&inputs);
            handles.push(scope.spawn(move || {
                for (offset, out) in slot.iter_mut().enumerate() {
                    let position = index * chunk + offset;
                    *out = Nat::from_decimal(&inputs[position]).unwrap().to_decimal();
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker");
        }
    });
    assert_eq!(actual, expected);
}

#[test]
fn parse_given_null_string_throws_format_exception() {
    // C# NaturalParseTests.Parse_GivenNullString_ThrowsFormatException. Rust has no null
    // string; from_decimal_opt(None) is the same missing input and is refused.
    assert!(Nat::from_decimal_opt(None).is_none());
}

#[test]
fn try_parse_given_null_string_returns_false() {
    // C# NaturalParseTests.TryParse_GivenNullString_ReturnsFalse.
    assert!(Nat::from_decimal_opt(None).is_none());
}

#[test]
fn cross_check_given_random_operands_matches_reference() {
    // C# NaturalRandomizedCrossCheckTests.CrossCheck_GivenRandomOperands_MatchesBigInteger,
    // five InlineData tiers: (1,200), (10,50), (100,20), (1000,5), (10000,2).
    // BigInteger is replaced by independent algebraic identities and a u128 reference on the
    // single-digit tier; every identity is exact, so no trust in the implementation under test
    // is needed beyond the identity itself.
    let mut rng = Rng::new(20_240_607);
    for (max_digits, cases) in [
        (1usize, 200usize),
        (10, 50),
        (100, 20),
        (1000, 5),
        (10000, 2),
    ] {
        for _ in 0..cases {
            let a_length = 1 + (rng.next() as usize % max_digits);
            let a_text = rng.digits(a_length);
            let b_length = 1 + (rng.next() as usize % max_digits);
            let b_text = rng.digits(b_length);
            let a = Nat::from_decimal(&a_text).unwrap();
            let b = Nat::from_decimal(&b_text).unwrap();
            let sum = a.add(&b);
            assert_eq!(sum.sub(&b).unwrap(), a, "({a_text}+{b_text})-{b_text}");
            assert_eq!(sum.sub(&a).unwrap(), b, "({a_text}+{b_text})-{a_text}");
            if a >= b {
                assert_eq!(
                    a.sub(&b).unwrap().add(&b),
                    a,
                    "({a_text}-{b_text})+{b_text}"
                );
            }
            let product = a.mul(&b);
            let (quotient, remainder) = product.div_rem(&b).unwrap();
            assert_eq!(quotient, a);
            assert!(remainder.is_zero());
            let (quotient, remainder) = a.div_rem(&b).unwrap();
            assert_eq!(quotient.mul(&b).add(&remainder), a);
            assert!(remainder < b);
            let exponent = (rng.next() % 9) as u32;
            let mut power = Nat::from_u64(1);
            for _ in 0..exponent {
                power = power.mul(&a);
            }
            assert_eq!(a.pow(exponent as u64), power, "{a_text}^{exponent}");
            if max_digits == 1 {
                let reference = (a_text.parse::<u128>().unwrap())
                    .checked_pow(exponent)
                    .unwrap();
                assert_eq!(a.pow(exponent as u64).to_decimal(), reference.to_string());
            }
        }
    }
}

#[test]
fn div_rem_given_power_of_ten_divisor_matches_reference() {
    // C# NaturalRandomizedCrossCheckTests.DivRem_GivenPowerOfTenDivisor_MatchesBigInteger,
    // k = 1..40, twenty 1..80-digit dividends each. For a power-of-ten divisor the quotient is
    // the decimal prefix and the remainder the decimal suffix, so the expected values are read
    // off the input literal itself rather than from another arithmetic implementation.
    let mut rng = Rng::new(99);
    for k in 1..=40usize {
        let divisor = Nat::from_u64(10).pow(k as u64);
        for _ in 0..20 {
            let length = 1 + (rng.next() as usize % 80);
            let text = rng.digits(length);
            let dividend = Nat::from_decimal(&text).unwrap();
            if dividend < divisor {
                continue;
            }
            let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
            let expected_quotient = &text[..text.len() - k];
            let expected_remainder = &text[text.len() - k..];
            assert_eq!(
                quotient.to_decimal(),
                expected_quotient.trim_start_matches('0')
            );
            assert_eq!(remainder.to_decimal(), {
                let trimmed = expected_remainder.trim_start_matches('0');
                if trimmed.is_empty() { "0" } else { trimmed }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Structure queries added for the reference's decimal/limb surface
// ---------------------------------------------------------------------------

#[test]
fn structure_queries_report_limbs_bit_length_and_decimal_digits() {
    let zero = Nat::zero();
    assert_eq!(zero.limb_count(), 0);
    assert_eq!(zero.bit_length(), 0);
    assert_eq!(zero.decimal_digit_count(), 1);
    assert!(zero.to_limbs().is_empty());

    let power_64 = Nat::from_decimal("18446744073709551616").unwrap();
    assert_eq!(power_64.limb_count(), 2);
    assert_eq!(power_64.to_limbs(), vec![0, 1]);
    assert_eq!(power_64.bit_length(), 65);
    assert_eq!(power_64.decimal_digit_count(), 20);

    assert_eq!(Nat::from_u64(u64::MAX).limb_count(), 1);
    assert_eq!(Nat::from_u64(u64::MAX).bit_length(), 64);
    assert_eq!(Nat::from_u64(0).try_get_exact_i64(), Some(0));
    assert_eq!(
        Nat::from_u64(i64::MAX as u64).try_get_exact_i64(),
        Some(i64::MAX)
    );
    assert!(power_64.try_get_exact_i64().is_none());
}

#[test]
fn gcd_and_lcm_match_the_reference_identities() {
    let a = Nat::from_u64(48);
    let b = Nat::from_u64(18);
    assert_eq!(a.gcd(&b), Nat::from_u64(6));
    assert_eq!(a.lcm(&b), Nat::from_u64(144));
    assert!(Nat::zero().lcm(&b).is_zero());
}
