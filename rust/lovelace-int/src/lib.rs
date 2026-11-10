//! Arbitrary-precision signed integers built on lovelace-nat.
//!
//! An Int is a sign-magnitude pair. The sign is always one of -1, 0, or +1, and
//! zero is always non-negative: there is no negative zero. Magnitude arithmetic is
//! delegated to the canonical base-10^9 limbs of lovelace_nat::Nat.
//!
//! Division uses C# truncating semantics: the quotient rounds toward zero and the
//! remainder takes the sign of the dividend. Division by zero returns None.
//!
//! No floating-point arithmetic, external crates, or unsafe code are used.

#![forbid(unsafe_code)]

use core::cmp::Ordering;
use core::fmt;

pub use lovelace_nat::Nat;

/// An arbitrary-precision signed integer.
///
/// The fields are private so the sign and magnitude cannot be desynchronized. Zero
/// is canonicalized to sign 0 and an empty magnitude.
#[derive(Clone, Eq, PartialEq, Hash, Default)]
pub struct Int {
    sign: i8,
    magnitude: Nat,
}

impl Int {
    /// Creates the additive identity, zero, with a non-negative sign.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            sign: 0,
            magnitude: Nat::zero(),
        }
    }

    /// Creates an integer from an i64 without losing i64::MIN.
    #[must_use]
    pub fn from_i64(value: i64) -> Self {
        if value == 0 {
            return Self::zero();
        }
        let sign = if value < 0 { -1 } else { 1 };
        Self::from_parts(sign, Nat::from_u64(value.unsigned_abs()))
    }

    /// Parses an optional leading minus sign followed by one or more decimal digits.
    ///
    /// A leading plus sign, whitespace, an empty payload, or any non-digit character
    /// returns None. Leading zeroes are accepted. "-0" normalizes to ordinary zero.
    #[must_use]
    pub fn from_decimal(text: &str) -> Option<Self> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(remainder) => (true, remainder),
            None => (false, text),
        };
        if digits.is_empty() {
            return None;
        }
        let magnitude = Nat::from_decimal(digits)?;
        Some(Self::from_parts(if negative { -1 } else { 1 }, magnitude))
    }

    /// Formats the value in decimal, including a leading minus sign when negative.
    ///
    /// Zero is formatted as "0" with no sign.
    #[must_use]
    pub fn to_decimal(&self) -> String {
        let magnitude = self.magnitude.to_decimal();
        if self.is_negative() {
            let mut text = String::with_capacity(magnitude.len() + 1);
            text.push('-');
            text.push_str(&magnitude);
            text
        } else {
            magnitude
        }
    }

    /// Returns true when the value is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.sign == 0
    }

    /// Returns true when the value is strictly less than zero.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.sign < 0
    }

    /// Returns the sign: -1 for negative, 0 for zero, and +1 for positive.
    #[must_use]
    pub fn sign(&self) -> i8 {
        self.sign
    }

    /// Returns a reference to the non-negative magnitude.
    #[must_use]
    pub fn magnitude(&self) -> &Nat {
        &self.magnitude
    }

    /// Returns the additive inverse, keeping zero non-negative.
    #[must_use]
    pub fn negate(&self) -> Self {
        if self.is_zero() {
            Self::zero()
        } else {
            Self::from_parts(-self.sign, self.magnitude.clone())
        }
    }

    /// Adds two signed integers exactly.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        if self.is_zero() {
            return other.clone();
        }
        if other.is_zero() {
            return self.clone();
        }

        if self.sign == other.sign {
            return Self::from_parts(self.sign, self.magnitude.add(&other.magnitude));
        }

        match self.magnitude.cmp(&other.magnitude) {
            Ordering::Greater => Self::from_parts(
                self.sign,
                self.magnitude
                    .sub(&other.magnitude)
                    .expect("larger magnitude minus smaller magnitude cannot underflow"),
            ),
            Ordering::Less => Self::from_parts(
                other.sign,
                other
                    .magnitude
                    .sub(&self.magnitude)
                    .expect("larger magnitude minus smaller magnitude cannot underflow"),
            ),
            Ordering::Equal => Self::zero(),
        }
    }

    /// Subtracts other from self exactly.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.negate())
    }

    /// Multiplies two signed integers exactly.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        Self::from_parts(
            if self.sign == other.sign { 1 } else { -1 },
            self.magnitude.mul(&other.magnitude),
        )
    }

    /// Divides self by a non-negative Nat, truncating the quotient toward zero.
    ///
    /// Returns exact (quotient, remainder). The remainder has the dividend's sign
    /// (or is zero), matching C# integer division. Returns None for a zero divisor.
    #[must_use]
    pub fn div_rem(&self, divisor: &Nat) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let (quotient, remainder) = self.magnitude.div_rem(divisor)?;
        Some((
            Self::from_parts(self.sign, quotient),
            Self::from_parts(self.sign, remainder),
        ))
    }

    /// Divides self by a signed Int using C# truncating semantics.
    ///
    /// The quotient rounds toward zero. The remainder takes the dividend's sign (or
    /// is zero). Returns None for a zero divisor. For a positive divisor this agrees
    /// exactly with div_rem.
    #[must_use]
    pub fn div_rem_signed(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let (quotient, remainder) = self.magnitude.div_rem(&divisor.magnitude)?;
        let quotient_sign = if quotient.is_zero() {
            0
        } else if (self.sign < 0) ^ (divisor.sign < 0) {
            -1
        } else {
            1
        };
        Some((
            Self::from_parts(quotient_sign, quotient),
            Self::from_parts(self.sign, remainder),
        ))
    }

    /// Raises self to a u64 exponent.
    ///
    /// Exponent zero returns one, including for a zero base. A negative base is
    /// negative exactly when the exponent is odd.
    #[must_use]
    pub fn pow(&self, exponent: u64) -> Self {
        if exponent == 0 {
            return Self::from_i64(1);
        }
        if self.is_zero() {
            return Self::zero();
        }
        let sign = if self.sign < 0 && exponent % 2 == 1 {
            -1
        } else {
            1
        };
        Self::from_parts(sign, self.magnitude.pow(exponent))
    }

    /// Computes n! as a signed integer.
    ///
    /// Both 0! and 1! are one. Factorials are never negative.
    #[must_use]
    pub fn factorial(n: u64) -> Self {
        Self::from_parts(1, Nat::factorial(n))
    }

    /// Builds a normalized sign-magnitude value.
    fn from_parts(sign: i8, magnitude: Nat) -> Self {
        let normalized_sign = if magnitude.is_zero() {
            0
        } else if sign < 0 {
            -1
        } else {
            1
        };
        Self {
            sign: normalized_sign,
            magnitude,
        }
    }
}

impl fmt::Debug for Int {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Int")
            .field(&self.to_decimal())
            .finish()
    }
}

impl fmt::Display for Int {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_decimal())
    }
}

impl PartialOrd for Int {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Int {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.sign.cmp(&other.sign) {
            Ordering::Equal => {
                if self.sign < 0 {
                    other.magnitude.cmp(&self.magnitude)
                } else {
                    self.magnitude.cmp(&other.magnitude)
                }
            }
            non_equal => non_equal,
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::{Int, Nat};

    #[test]
    fn zero_GivenNoArguments_IsNonNegativeZero() {
        let zero = Int::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_negative());
        assert_eq!(zero.sign(), 0);
        assert_eq!(zero.to_decimal(), "0");
        assert_eq!(zero.magnitude().to_decimal(), "0");
    }

    #[test]
    fn default_GivenNoArguments_EqualsZero() {
        assert_eq!(Int::default(), Int::zero());
    }

    #[test]
    fn from_i64_GivenMinimum_HandlesMagnitudeWithoutOverflow() {
        let value = Int::from_i64(i64::MIN);
        assert_eq!(value.to_decimal(), "-9223372036854775808");
        assert_eq!(value.magnitude().to_decimal(), "9223372036854775808");
        assert!(value.is_negative());
    }

    #[test]
    fn from_i64_GivenMaximum_PreservesExactValue() {
        assert_eq!(Int::from_i64(i64::MAX).to_decimal(), "9223372036854775807");
    }

    #[test]
    fn from_i64_GivenNegativeValue_SetsSign() {
        let value = Int::from_i64(-42);
        assert_eq!(value.sign(), -1);
        assert_eq!(value.to_decimal(), "-42");
    }

    #[test]
    fn from_i64_GivenZero_NormalizesToNonNegative() {
        let value = Int::from_i64(0);
        assert_eq!(value.sign(), 0);
        assert!(!value.is_negative());
    }

    #[test]
    fn from_decimal_GivenNegativeZero_NormalizesToPositiveZero() {
        let value = Int::from_decimal("-0").unwrap();
        assert!(value.is_zero());
        assert!(!value.is_negative());
        assert_eq!(value.to_decimal(), "0");
    }

    #[test]
    fn from_decimal_GivenLeadingZerosAndMinus_Canonicalizes() {
        let value = Int::from_decimal("-00042").unwrap();
        assert_eq!(value.to_decimal(), "-42");
    }

    #[test]
    fn from_decimal_GivenEmptyString_ReturnsNone() {
        assert!(Int::from_decimal("").is_none());
    }

    #[test]
    fn from_decimal_GivenBareMinus_ReturnsNone() {
        assert!(Int::from_decimal("-").is_none());
    }

    #[test]
    fn from_decimal_GivenPlusSign_ReturnsNone() {
        assert!(Int::from_decimal("+7").is_none());
    }

    #[test]
    fn from_decimal_GivenNonDigit_ReturnsNone() {
        assert!(Int::from_decimal("12a3").is_none());
    }

    #[test]
    fn from_decimal_GivenArbitraryPrecision_ParsesExactly() {
        let literal = "1234567890123456789012345678901234567890";
        let value = Int::from_decimal(literal).unwrap();
        assert_eq!(value.to_decimal(), literal);
    }

    #[test]
    fn to_decimal_GivenNegativeLarge_PrependsMinus() {
        let value = Int::from_decimal("-98765432109876543210").unwrap();
        assert_eq!(value.to_decimal(), "-98765432109876543210");
    }

    #[test]
    fn parse_then_format_GivenLargeNegative_RoundTrips() {
        let literal = "-314159265358979323846264338327950288419716939937510";
        let value = Int::from_decimal(literal).unwrap();
        assert_eq!(value.to_decimal(), literal);
    }

    #[test]
    fn magnitude_GivenNegative_ReturnsAbsoluteValue() {
        let value = Int::from_i64(-123);
        assert_eq!(value.magnitude().to_decimal(), "123");
    }

    #[test]
    fn negate_GivenPositive_ReturnsNegative() {
        assert_eq!(Int::from_i64(42).negate(), Int::from_i64(-42));
    }

    #[test]
    fn negate_GivenNegative_ReturnsPositive() {
        assert_eq!(Int::from_i64(-42).negate(), Int::from_i64(42));
    }

    #[test]
    fn negate_GivenZero_RemainsPositiveZero() {
        let value = Int::zero().negate();
        assert!(value.is_zero());
        assert!(!value.is_negative());
        assert_eq!(value.sign(), 0);
    }

    #[test]
    fn add_GivenSameSignNegative_AddsMagnitudes() {
        assert_eq!(
            Int::from_i64(-2).add(&Int::from_i64(-3)),
            Int::from_i64(-5)
        );
    }

    #[test]
    fn add_GivenOppositeSigns_LargerPositive_ReturnsPositiveDifference() {
        assert_eq!(Int::from_i64(-3).add(&Int::from_i64(10)), Int::from_i64(7));
    }

    #[test]
    fn add_GivenOppositeSigns_LargerNegative_ReturnsNegativeDifference() {
        assert_eq!(Int::from_i64(3).add(&Int::from_i64(-10)), Int::from_i64(-7));
    }

    #[test]
    fn add_GivenOpposites_ReturnsNonNegativeZero() {
        let value = Int::from_i64(5).add(&Int::from_i64(-5));
        assert!(value.is_zero());
        assert!(!value.is_negative());
        assert_eq!(value.sign(), 0);
    }

    #[test]
    fn add_GivenCarryAcrossLimbs_Propagates() {
        let left = Int::from_decimal("999999999999999999").unwrap();
        let value = left.add(&Int::from_i64(1));
        assert_eq!(value.to_decimal(), "1000000000000000000");
    }

    #[test]
    fn sub_GivenSmallerMinusLarger_ReturnsNegative() {
        assert_eq!(Int::from_i64(3).sub(&Int::from_i64(10)), Int::from_i64(-7));
    }

    #[test]
    fn sub_GivenNegativeMinusNegative_CanBePositive() {
        assert_eq!(
            Int::from_i64(-3).sub(&Int::from_i64(-7)),
            Int::from_i64(4)
        );
    }

    #[test]
    fn sub_GivenEqualValues_ReturnsZero() {
        let value = Int::from_i64(123456789);
        assert!(value.sub(&value).is_zero());
    }

    #[test]
    fn sub_GivenBorrowAcrossLimbs_Propagates() {
        let left = Int::from_decimal("1000000000000000000").unwrap();
        let value = left.sub(&Int::from_i64(1));
        assert_eq!(value.to_decimal(), "999999999999999999");
    }

    #[test]
    fn mul_GivenNegativeAndPositive_ReturnsNegative() {
        assert_eq!(
            Int::from_i64(-7).mul(&Int::from_i64(6)),
            Int::from_i64(-42)
        );
    }

    #[test]
    fn mul_GivenTwoNegatives_ReturnsPositive() {
        assert_eq!(
            Int::from_i64(-7).mul(&Int::from_i64(-6)),
            Int::from_i64(42)
        );
    }

    #[test]
    fn mul_GivenZeroOperand_ReturnsNonNegativeZero() {
        let value = Int::from_i64(-5).mul(&Int::zero());
        assert!(value.is_zero());
        assert!(!value.is_negative());
        assert_eq!(value.sign(), 0);
    }

    #[test]
    fn mul_GivenLargeOperands_ReturnsExactNegativeProduct() {
        let left = Int::from_decimal("-12345678901234567890").unwrap();
        let right = Int::from_decimal("98765432109876543210").unwrap();
        assert_eq!(
            left.mul(&right).to_decimal(),
            "-1219326311370217952237463801111263526900"
        );
    }

    #[test]
    fn div_rem_GivenNegativeDividendAndPositiveDivisor_TruncatesTowardZero() {
        let dividend = Int::from_i64(-7);
        let divisor = Nat::from_u64(2);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient, Int::from_i64(-3));
        assert_eq!(remainder, Int::from_i64(-1));
    }

    #[test]
    fn div_rem_GivenPositiveDividendAndDivisor_ReturnsExact() {
        let (quotient, remainder) = Int::from_i64(42).div_rem(&Nat::from_u64(5)).unwrap();
        assert_eq!(quotient, Int::from_i64(8));
        assert_eq!(remainder, Int::from_i64(2));
    }

    #[test]
    fn div_rem_GivenZeroDivisor_ReturnsNone() {
        assert!(Int::from_i64(42).div_rem(&Nat::zero()).is_none());
    }

    #[test]
    fn div_rem_GivenNegativeSmallerMagnitude_ReturnsZeroQuotientAndNegativeRemainder() {
        let (quotient, remainder) = Int::from_i64(-3).div_rem(&Nat::from_u64(7)).unwrap();
        assert!(quotient.is_zero());
        assert!(quotient.sign() == 0);
        assert_eq!(remainder, Int::from_i64(-3));
    }

    #[test]
    fn div_rem_GivenExactDivision_ReturnsZeroRemainder() {
        let (quotient, remainder) = Int::from_i64(-12).div_rem(&Nat::from_u64(4)).unwrap();
        assert_eq!(quotient, Int::from_i64(-3));
        assert!(remainder.is_zero());
        assert_eq!(remainder.sign(), 0);
    }

    #[test]
    fn div_rem_signed_GivenNegativeDivisor_TruncatesTowardZero() {
        let (quotient, remainder) = Int::from_i64(7)
            .div_rem_signed(&Int::from_i64(-2))
            .unwrap();
        assert_eq!(quotient, Int::from_i64(-3));
        assert_eq!(remainder, Int::from_i64(1));
    }

    #[test]
    fn div_rem_signed_GivenBothNegative_ReturnsPositiveQuotientAndNegativeRemainder() {
        let (quotient, remainder) = Int::from_i64(-7)
            .div_rem_signed(&Int::from_i64(-2))
            .unwrap();
        assert_eq!(quotient, Int::from_i64(3));
        assert_eq!(remainder, Int::from_i64(-1));
    }

    #[test]
    fn div_rem_signed_GivenZeroDivisor_ReturnsNone() {
        assert!(Int::from_i64(7).div_rem_signed(&Int::zero()).is_none());
    }

    #[test]
    fn pow_GivenNegativeBaseEvenExponent_ReturnsPositive() {
        assert_eq!(Int::from_i64(-2).pow(10), Int::from_i64(1024));
    }

    #[test]
    fn pow_GivenNegativeBaseOddExponent_ReturnsNegative() {
        assert_eq!(Int::from_i64(-2).pow(11), Int::from_i64(-2048));
    }

    #[test]
    fn pow_GivenZeroExponent_ReturnsOne() {
        assert_eq!(Int::from_i64(-7).pow(0), Int::from_i64(1));
    }

    #[test]
    fn pow_GivenZeroBaseAndPositiveExponent_ReturnsZero() {
        let value = Int::zero().pow(9);
        assert!(value.is_zero());
        assert_eq!(value.sign(), 0);
    }

    #[test]
    fn factorial_GivenZero_ReturnsOne() {
        assert_eq!(Int::factorial(0), Int::from_i64(1));
    }

    #[test]
    fn factorial_GivenTen_ReturnsKnownValue() {
        assert_eq!(Int::factorial(10), Int::from_i64(3_628_800));
    }

    #[test]
    fn factorial_GivenHundred_Has158Digits() {
        let value = Int::factorial(100);
        assert_eq!(value.to_decimal().len(), 158);
        assert_eq!(
            value.to_decimal(),
            "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"
        );
    }

    #[test]
    fn factorial_GivenThousand_Has2568DigitsAndPositiveSign() {
        let value = Int::factorial(1000);
        let text = value.to_decimal();
        assert_eq!(text.len(), 2568);
        assert!(text.starts_with("402387260077093773543702433923"));
        assert_eq!(value.sign(), 1);
    }

    #[test]
    fn ord_GivenNegativeAndPositive_OrdersBySign() {
        assert!(Int::from_i64(-1) < Int::from_i64(1));
    }

    #[test]
    fn ord_GivenTwoNegatives_OrdersByMagnitude() {
        assert!(Int::from_i64(-10) < Int::from_i64(-2));
    }

    #[test]
    fn ord_GivenEqualValues_ReportsEqual() {
        assert_eq!(
            Int::from_decimal("-12345678901234567890")
                .unwrap()
                .cmp(&Int::from_decimal("-12345678901234567890").unwrap()),
            core::cmp::Ordering::Equal
        );
    }

    #[test]
    fn sign_GivenPositiveNegativeAndZero_ReturnsExpectedSigns() {
        assert_eq!(Int::from_i64(9).sign(), 1);
        assert_eq!(Int::from_i64(-9).sign(), -1);
        assert_eq!(Int::from_i64(0).sign(), 0);
    }
}
