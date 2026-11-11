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
        assert_eq!(Int::from_i64(-2).add(&Int::from_i64(-3)), Int::from_i64(-5));
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
        assert_eq!(Int::from_i64(-3).sub(&Int::from_i64(-7)), Int::from_i64(4));
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
        assert_eq!(Int::from_i64(-7).mul(&Int::from_i64(6)), Int::from_i64(-42));
    }

    #[test]
    fn mul_GivenTwoNegatives_ReturnsPositive() {
        assert_eq!(Int::from_i64(-7).mul(&Int::from_i64(-6)), Int::from_i64(42));
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
        let (quotient, remainder) = Int::from_i64(7).div_rem_signed(&Int::from_i64(-2)).unwrap();
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

    // ------------------------------------------------------------------
    // Ports of Lovelace.Integer.Tests. Each test reproduces the C# method's
    // concrete operands and concrete expected value. The C# operators
    // (+, -, *, /, %, unary -) delegate to the named methods, so the Rust
    // tests call those methods directly; Rust has no operator overloads.
    // ------------------------------------------------------------------
    use core::cmp::Ordering;

    #[test]
    fn add_GivenTwoPositiveIntegers_ReturnsSeven() {
        // IntegerAddTests.Add_GivenTwoPositiveIntegers_ReturnsCorrectSum: 3 + 4 == 7.
        assert_eq!(Int::from_i64(3).add(&Int::from_i64(4)), Int::from_i64(7));
    }

    #[test]
    fn add_GivenTwoNegativeIntegers_ReturnsNegativeSeven() {
        // IntegerAddTests.Add_GivenTwoNegativeIntegers_ReturnsNegativeSum:
        // -3 + -4 == -7 and the result is negative.
        let sum = Int::from_i64(-3).add(&Int::from_i64(-4));
        assert_eq!(sum, Int::from_i64(-7));
        assert!(sum.is_negative());
    }

    #[test]
    fn add_GivenPositiveAndNegativePositiveLarger_ReturnsPositiveSeven() {
        // IntegerAddTests.Add_GivenPositiveAndNegativeWherePositiveLarger:
        // 10 + (-3) == 7 and the result is positive.
        let sum = Int::from_i64(10).add(&Int::from_i64(-3));
        assert_eq!(sum, Int::from_i64(7));
        assert_eq!(sum.sign(), 1);
    }

    #[test]
    fn add_GivenZeroOnRight_ReturnsOriginal() {
        // IntegerAddTests.Add_GivenZeroOperandOnRight: 42 + 0 == 42.
        assert_eq!(Int::from_i64(42).add(&Int::zero()), Int::from_i64(42));
    }

    #[test]
    fn add_GivenZeroOnLeft_ReturnsOriginal() {
        // IntegerAddTests.Add_GivenZeroOperandOnLeft: 0 + (-17) == -17.
        assert_eq!(Int::zero().add(&Int::from_i64(-17)), Int::from_i64(-17));
    }

    #[test]
    fn operator_plus_GivenTwoPositives_ReturnsOneHundredTwentyThree() {
        // IntegerAddTests.OperatorPlus_GivenTwoPositives: 100 + 23 == 123.
        assert_eq!(
            Int::from_i64(100).add(&Int::from_i64(23)),
            Int::from_i64(123)
        );
    }

    #[test]
    fn operator_plus_GivenMixedSigns_ReturnsSeven() {
        // IntegerAddTests.OperatorPlus_GivenPositiveAndNegative: 10 + (-3) == 7.
        assert_eq!(Int::from_i64(10).add(&Int::from_i64(-3)), Int::from_i64(7));
    }

    #[test]
    fn operator_plus_GivenLargeValues_ExceedsI64Maximum() {
        // IntegerAddTests.OperatorPlus_GivenLargeValues:
        // "99999999999999999999" + "1" == "100000000000000000000".
        let sum = Int::from_decimal("99999999999999999999")
            .unwrap()
            .add(&Int::from_i64(1));
        assert_eq!(sum.to_decimal(), "100000000000000000000");
    }

    #[test]
    fn default_GivenNoArguments_IsZeroAndNonNegative() {
        // IntegerConstructorTests.Constructor_Default_SignIsPositive:
        // default Integer is zero and not negative.
        let value = Int::default();
        assert!(value.is_zero());
        assert_eq!(value.sign(), 0);
        assert!(!value.is_negative());
    }

    #[test]
    fn from_i64_GivenPositiveValue_SetsSignAndDecimal() {
        // IntegerConstructorTests.Constructor_GivenPositiveLong:
        // new Integer(42) renders "42" and is positive.
        let value = Int::from_i64(42);
        assert_eq!(value.to_decimal(), "42");
        assert_eq!(value.sign(), 1);
    }

    #[test]
    fn from_decimal_GivenPositiveDigits_ReturnsFortyTwo() {
        // IntegerConstructorTests.Constructor_GivenPositiveDecimalString: "42" -> 42.
        assert_eq!(Int::from_decimal("42").unwrap(), Int::from_i64(42));
    }

    #[test]
    fn from_decimal_GivenNegativeDigits_ReturnsMinusSeven() {
        // IntegerConstructorTests.Constructor_GivenNegativeDecimalString:
        // "-7" renders "-7" and is negative. Also Parse_GivenSpan ("-7" span).
        let value = Int::from_decimal("-7").unwrap();
        assert_eq!(value.to_decimal(), "-7");
        assert!(value.is_negative());
    }

    #[test]
    fn from_decimal_GivenZeroLiteral_IsZeroAndNonNegative() {
        // IntegerConstructorTests.Constructor_GivenZeroString: "0" is zero, not negative.
        let value = Int::from_decimal("0").unwrap();
        assert!(value.is_zero());
        assert!(!value.is_negative());
    }

    #[test]
    fn from_decimal_GivenLeadingZeros_StripsThem() {
        // IntegerConstructorTests.Constructor_GivenStringWithLeadingZeros:
        // "007" equals 7. Also IntegerParseTests.Parse_GivenLeadingZeros_SkipsThem.
        assert_eq!(Int::from_decimal("007").unwrap(), Int::from_i64(7));
    }

    #[test]
    fn from_decimal_GivenTwoDigitLiteral_ReturnsNinetyNine() {
        // IntegerConstructorTests.Constructor_GivenPositiveSpan: "99" -> "99", positive.
        let value = Int::from_decimal("99").unwrap();
        assert_eq!(value.to_decimal(), "99");
        assert_eq!(value.sign(), 1);
    }

    #[test]
    fn from_decimal_GivenNegativeThreeLiteral_ReturnsMinusThree() {
        // IntegerConstructorTests.Constructor_GivenNegativeSpan: "-3" -> "-3", negative.
        let value = Int::from_decimal("-3").unwrap();
        assert_eq!(value.to_decimal(), "-3");
        assert!(value.is_negative());
    }

    #[test]
    fn from_decimal_GivenThirtyDigitLiteral_RoundTrips() {
        // IntegerConstructorTests.Constructor_GivenArbitraryPrecisionString and
        // IntegerParseTests.Parse_GivenArbitraryPrecisionString assert the same
        // 30-digit literal round-trips through ToString.
        let literal = "123456789012345678901234567890";
        assert_eq!(Int::from_decimal(literal).unwrap().to_decimal(), literal);
    }

    #[test]
    fn from_decimal_GivenNegativeThirtyDigitLiteral_RoundTrips() {
        // IntegerParseTests.Parse_GivenNegativeArbitraryPrecisionString.
        let literal = "-123456789012345678901234567890";
        assert_eq!(Int::from_decimal(literal).unwrap().to_decimal(), literal);
    }

    #[test]
    fn from_decimal_GivenMinusFortyTwo_ReturnsMinusFortyTwo() {
        // IntegerParseTests.Parse_GivenLeadingMinus: "-42" -> -42.
        assert_eq!(Int::from_decimal("-42").unwrap(), Int::from_i64(-42));
    }

    #[test]
    fn div_rem_signed_GivenTenOverThree_ReturnsThreeAndOne() {
        // IntegerDivideTests.DivRem_GivenPositiveDividendAndDivisor (10/3 -> 3 r 1),
        // DivisionOperator_GivenPositives (10/3 == 3) and ModulusOperator_GivenPositives
        // (10%3 == 1) all describe the same concrete quotient and remainder.
        let (quotient, remainder) = Int::from_i64(10).div_rem_signed(&Int::from_i64(3)).unwrap();
        assert_eq!(quotient, Int::from_i64(3));
        assert_eq!(remainder, Int::from_i64(1));
    }

    #[test]
    fn div_rem_signed_GivenNegativeTenOverThree_YieldsNegativeThreeAndMagnitudeOne() {
        // IntegerDivideTests.DivRem_GivenNegativeDividend_QuotientIsNegative,
        // DivRem_GivenNegativeDividend_ReturnsCorrectMagnitudes and
        // DivisionOperator_GivenNegativeDividend all use -10 / 3.
        let (quotient, remainder) = Int::from_i64(-10)
            .div_rem_signed(&Int::from_i64(3))
            .unwrap();
        assert!(quotient.is_negative());
        assert_eq!(quotient.magnitude().to_decimal(), "3");
        assert_eq!(remainder.magnitude().to_decimal(), "1");
    }

    #[test]
    fn div_rem_signed_GivenBothNegative_ReturnsPositiveThreeAndMagnitudeOne() {
        // IntegerDivideTests.DivRem_GivenBothNegative_QuotientIsPositive,
        // DivRem_GivenBothNegative_ReturnsCorrectMagnitudes and
        // DivisionOperator_GivenBothNegative all use -10 / -3.
        let (quotient, remainder) = Int::from_i64(-10)
            .div_rem_signed(&Int::from_i64(-3))
            .unwrap();
        assert_eq!(quotient, Int::from_i64(3));
        assert_eq!(remainder.magnitude().to_decimal(), "1");
    }

    #[test]
    fn div_rem_signed_GivenExactDivision_ReturnsZeroRemainder() {
        // IntegerDivideTests.DivRem_GivenExactDivision_RemainderIsZero (12/4) and
        // ModulusOperator_GivenExactDivision (12%4 == 0).
        let (quotient, remainder) = Int::from_i64(12).div_rem_signed(&Int::from_i64(4)).unwrap();
        assert_eq!(quotient, Int::from_i64(3));
        assert!(remainder.is_zero());
    }

    #[test]
    fn div_rem_signed_GivenHundredOverSeven_ReturnsRemainderTwo() {
        // IntegerDivideTests.ModulusOperator_GivenLargeDividend: 100 % 7 == 2.
        let (_, remainder) = Int::from_i64(100)
            .div_rem_signed(&Int::from_i64(7))
            .unwrap();
        assert_eq!(remainder, Int::from_i64(2));
    }

    #[test]
    fn equality_GivenSameValue_IsEqual() {
        // IntegerEqualityTests.Equals_GivenSameValue_ReturnsTrue: two 42s are equal.
        assert_eq!(Int::from_i64(42), Int::from_i64(42));
    }

    #[test]
    fn equality_GivenTwoZeros_IsEqual() {
        // IntegerEqualityTests.Equals_GivenTwoZeros_ReturnsTrue.
        assert_eq!(Int::from_i64(0), Int::from_i64(0));
    }

    #[test]
    fn equality_GivenSameMagnitudeDifferentSign_IsNotEqual() {
        // IntegerEqualityTests.Equals_GivenSameMagnitudeDifferentSign: 42 != -42.
        assert_ne!(Int::from_i64(42), Int::from_i64(-42));
    }

    #[test]
    fn cmp_GivenPositiveAndNegative_OrdersBothWays() {
        // IntegerEqualityTests.CompareTo_GivenPositiveGreaterThanNegative (1 vs -1),
        // GreaterThanOperator_GivenPositiveVsNegative and
        // LessThanOperator_GivenNegativeVsPositive all use the pair 1 and -1.
        assert_eq!(Int::from_i64(1).cmp(&Int::from_i64(-1)), Ordering::Greater);
        assert!(Int::from_i64(1) > Int::from_i64(-1));
        assert_eq!(Int::from_i64(-1).cmp(&Int::from_i64(1)), Ordering::Less);
        assert!(Int::from_i64(-1) < Int::from_i64(1));
    }

    #[test]
    fn cmp_GivenEqualValues_ReportsZero() {
        // IntegerEqualityTests.CompareTo_GivenEqualValues: 5.CompareTo(5) == 0.
        assert_eq!(Int::from_i64(5).cmp(&Int::from_i64(5)), Ordering::Equal);
    }

    #[test]
    fn cmp_GivenTwoNegatives_OrdersByMagnitude() {
        // IntegerEqualityTests.CompareTo_GivenTwoNegatives: -10 < -3.
        assert!(Int::from_i64(-10) < Int::from_i64(-3));
    }

    #[test]
    fn relational_operators_GivenConcreteOperands_MatchCSharp() {
        // IntegerEqualityTests.LessThanOperator_GivenSameSign (3 < 5),
        // GreaterThanOrEqual_GivenEqualValues (3 >= 3),
        // LessThanOrEqual_GivenSmallerValue (-5 <= -4), and
        // GreaterThanOrEqual_GivenLargerValue (10 >= 5).
        assert!(Int::from_i64(3) < Int::from_i64(5));
        assert!(Int::from_i64(3) >= Int::from_i64(3));
        assert!(Int::from_i64(-5) <= Int::from_i64(-4));
        assert!(Int::from_i64(10) >= Int::from_i64(5));
    }

    #[test]
    fn factorial_GivenFive_ReturnsOneHundredTwenty() {
        // IntegerFactorialTests.Factorial_GivenPositiveInteger_ReturnsCorrectValue.
        assert_eq!(Int::factorial(5), Int::from_i64(120));
    }

    #[test]
    fn factorial_GivenOne_ReturnsOne() {
        // IntegerFactorialTests.Factorial_GivenOne_ReturnsOne.
        assert_eq!(Int::factorial(1), Int::from_i64(1));
    }

    #[test]
    fn mul_GivenTwoPositives_ReturnsTwelve() {
        // IntegerMultiplyTests.Multiply_GivenTwoPositives: 3 * 4 == 12, positive.
        let product = Int::from_i64(3).mul(&Int::from_i64(4));
        assert_eq!(product, Int::from_i64(12));
        assert_eq!(product.sign(), 1);
    }

    #[test]
    fn mul_GivenPositiveAndNegative_ReturnsMinusTwelve() {
        // IntegerMultiplyTests.Multiply_GivenPositiveAndNegative (3 * -4) and
        // Multiply_GivenNegativeAndPositive (-3 * 4) both expect -12, negative.
        let left = Int::from_i64(3).mul(&Int::from_i64(-4));
        assert_eq!(left, Int::from_i64(-12));
        assert!(left.is_negative());
        let right = Int::from_i64(-3).mul(&Int::from_i64(4));
        assert_eq!(right, Int::from_i64(-12));
        assert!(right.is_negative());
    }

    #[test]
    fn mul_GivenTwoNegatives_ReturnsTwelve() {
        // IntegerMultiplyTests.Multiply_GivenTwoNegatives: -3 * -4 == 12, positive.
        let product = Int::from_i64(-3).mul(&Int::from_i64(-4));
        assert_eq!(product, Int::from_i64(12));
        assert_eq!(product.sign(), 1);
    }

    #[test]
    fn mul_GivenZeroOperandCases_ReturnsNonNegativeZero() {
        // IntegerMultiplyTests.Multiply_GivenZeroOperand rows (0,7), (7,0), (0,0);
        // each product is zero and not negative.
        for (left, right) in [(0_i64, 7_i64), (7, 0), (0, 0)] {
            let product = Int::from_i64(left).mul(&Int::from_i64(right));
            assert!(product.is_zero());
            assert!(!product.is_negative());
        }
    }

    #[test]
    fn mul_GivenOneOperand_ReturnsOriginalValue() {
        // IntegerMultiplyTests.Multiply_GivenOne rows 42, -7 and 0, both operand orders.
        for value in [42_i64, -7, 0] {
            let original = Int::from_i64(value);
            assert_eq!(original.mul(&Int::from_i64(1)), original);
            assert_eq!(Int::from_i64(1).mul(&original), original);
        }
    }

    #[test]
    fn mul_GivenBillionSquared_ReturnsExactProduct() {
        // IntegerMultiplyTests.Multiply_GivenLargeValues:
        // 1_000_000_000 * 1_000_000_000 == "1000000000000000000".
        let product = Int::from_i64(1_000_000_000).mul(&Int::from_i64(1_000_000_000));
        assert_eq!(product, Int::from_decimal("1000000000000000000").unwrap());
    }

    #[test]
    fn mul_GivenLargeNegativeAndPositive_ReturnsNegativeProduct() {
        // IntegerMultiplyTests.Multiply_GivenLargeNegativeAndPositive:
        // -1_000_000 * 1_000_000 == "-1000000000000", negative.
        let product = Int::from_i64(-1_000_000).mul(&Int::from_i64(1_000_000));
        assert_eq!(product, Int::from_decimal("-1000000000000").unwrap());
        assert!(product.is_negative());
    }

    #[test]
    fn negate_GivenPositiveFive_ReturnsMinusFive() {
        // IntegerNegationTests.Negate_GivenPositiveValue: 5.Negate() is negative "-5".
        let value = Int::from_i64(5).negate();
        assert!(value.is_negative());
        assert_eq!(value.to_decimal(), "-5");
    }

    #[test]
    fn negate_GivenNegativeFive_ReturnsFive() {
        // IntegerNegationTests.Negate_GivenNegativeValue: -5.Negate() is positive "5".
        let value = Int::from_i64(-5).negate();
        assert_eq!(value.sign(), 1);
        assert_eq!(value.to_decimal(), "5");
    }

    #[test]
    fn negate_GivenZero_ReturnsNonNegativeZero() {
        // IntegerNegationTests.UnaryMinus_GivenZero: -0 is zero and not negative.
        let value = Int::zero().negate();
        assert!(value.is_zero());
        assert!(!value.is_negative());
    }

    #[test]
    fn negate_GivenValueTwice_ReturnsOriginal() {
        // IntegerNegationTests.Negate_IsItsOwnInverse: 42 and -99 are fixed points.
        for value in [42_i64, -99] {
            assert_eq!(Int::from_i64(value).negate().negate(), Int::from_i64(value));
        }
    }

    #[test]
    fn negate_GivenThree_ReturnsMinusThree() {
        // IntegerNegationTests.UnaryMinus_GivenPositive: -3.
        assert_eq!(Int::from_i64(3).negate(), Int::from_i64(-3));
    }

    #[test]
    fn negate_GivenMinusSeven_ReturnsSeven() {
        // IntegerNegationTests.UnaryMinus_GivenNegative: 7.
        assert_eq!(Int::from_i64(-7).negate(), Int::from_i64(7));
    }

    #[test]
    fn from_decimal_GivenNegativeNinetyNine_ReturnsMinusNinetyNine() {
        // IntegerParseTests.TryParse_GivenValidInput: "-99" parses to -99.
        assert_eq!(Int::from_decimal("-99").unwrap(), Int::from_i64(-99));
    }

    #[test]
    fn from_decimal_GivenNonDigitAlpha_ReturnsNone() {
        // IntegerParseTests.TryParse_GivenInvalidInput: "abc" is rejected.
        assert!(Int::from_decimal("abc").is_none());
    }

    #[test]
    fn pow_GivenTwoToTen_ReturnsOneThousandTwentyFour() {
        // IntegerPowTests.Pow_GivenPositiveBaseAndPositiveExponent: 2^10 == 1024.
        assert_eq!(Int::from_i64(2).pow(10), Int::from_i64(1024));
    }

    #[test]
    fn pow_GivenNegativeTwoToFour_ReturnsSixteen() {
        // IntegerPowTests.Pow_GivenNegativeBaseAndEvenExponent: (-2)^4 == 16, positive.
        let value = Int::from_i64(-2).pow(4);
        assert_eq!(value, Int::from_i64(16));
        assert_eq!(value.sign(), 1);
    }

    #[test]
    fn pow_GivenNegativeTwoToThree_ReturnsMinusEight() {
        // IntegerPowTests.Pow_GivenNegativeBaseAndOddExponent: (-2)^3 == -8, negative.
        let value = Int::from_i64(-2).pow(3);
        assert_eq!(value, Int::from_i64(-8));
        assert!(value.is_negative());
    }

    #[test]
    fn is_zero_GivenDefault_IsTrue() {
        // IntegerPredicateTests.IsZero_GivenDefaultInstance_ReturnsTrue.
        assert!(Int::default().is_zero());
    }

    #[test]
    fn is_zero_GivenOne_IsFalse() {
        // IntegerPredicateTests.IsZero_GivenNonZeroValue_ReturnsFalse.
        assert!(!Int::from_i64(1).is_zero());
    }

    #[test]
    fn is_zero_GivenMinusFive_IsFalse() {
        // IntegerPredicateTests.IsZero_GivenNegativeNonZeroValue_ReturnsFalse.
        assert!(!Int::from_i64(-5).is_zero());
    }

    #[test]
    fn is_positive_GivenFive_HasSignOne() {
        // IntegerPredicateTests.IsPositive_GivenPositiveValue_ReturnsTrue.
        assert_eq!(Int::from_i64(5).sign(), 1);
    }

    #[test]
    fn is_positive_GivenMinusFive_IsNegative() {
        // IntegerPredicateTests.IsPositive_GivenNegativeValue_ReturnsFalse.
        assert_eq!(Int::from_i64(-5).sign(), -1);
        assert!(Int::from_i64(-5).is_negative());
    }

    #[test]
    fn is_positive_GivenZero_IsNonNegative() {
        // IntegerPredicateTests.IsPositive_GivenZero_ReturnsTrue: C# treats zero as
        // positive; Rust's equivalent is a non-negative sign of 0.
        assert_eq!(Int::from_i64(0).sign(), 0);
        assert!(!Int::from_i64(0).is_negative());
    }

    #[test]
    fn is_negative_GivenMinusOne_IsTrue() {
        // IntegerPredicateTests.IsNegative_GivenNegativeValue_ReturnsTrue.
        assert!(Int::from_i64(-1).is_negative());
    }

    #[test]
    fn is_negative_GivenThree_IsFalse() {
        // IntegerPredicateTests.IsNegative_GivenPositiveValue_ReturnsFalse.
        assert!(!Int::from_i64(3).is_negative());
    }

    #[test]
    fn is_negative_GivenZero_IsFalse() {
        // IntegerPredicateTests.IsNegative_GivenZero_ReturnsFalse.
        assert!(!Int::from_i64(0).is_negative());
    }

    #[test]
    fn sign_GivenTenMinusTenAndZero_ReturnsExpectedSigns() {
        // IntegerPredicateTests.Sign_GivenPositive (10 -> 1), Sign_GivenNegative
        // (-10 -> -1), Sign_GivenZero (0 -> 0).
        assert_eq!(Int::from_i64(10).sign(), 1);
        assert_eq!(Int::from_i64(-10).sign(), -1);
        assert_eq!(Int::from_i64(0).sign(), 0);
    }

    #[test]
    fn sub_GivenTenMinusThree_ReturnsSeven() {
        // IntegerSubtractTests.Subtract_GivenPositiveFromLargerPositive: 10-3 == 7, positive.
        let difference = Int::from_i64(10).sub(&Int::from_i64(3));
        assert_eq!(difference, Int::from_i64(7));
        assert_eq!(difference.sign(), 1);
    }

    #[test]
    fn sub_GivenThreeMinusNegativeFour_ReturnsSeven() {
        // IntegerSubtractTests.Subtract_GivenNegativeFromPositive: 3-(-4) == 7, positive.
        let difference = Int::from_i64(3).sub(&Int::from_i64(-4));
        assert_eq!(difference, Int::from_i64(7));
        assert_eq!(difference.sign(), 1);
    }

    #[test]
    fn sub_GivenNegativeThreeMinusNegativeFour_ReturnsOne() {
        // IntegerSubtractTests.Subtract_GivenNegativeFromNegative: -3-(-4) == 1, positive.
        let difference = Int::from_i64(-3).sub(&Int::from_i64(-4));
        assert_eq!(difference, Int::from_i64(1));
        assert_eq!(difference.sign(), 1);
    }

    #[test]
    fn sub_GivenNegativeThreeMinusFour_ReturnsNegativeSeven() {
        // IntegerSubtractTests.Subtract_GivenPositiveFromNegative: -3-4 == -7, negative.
        let difference = Int::from_i64(-3).sub(&Int::from_i64(4));
        assert_eq!(difference, Int::from_i64(-7));
        assert!(difference.is_negative());
    }

    #[test]
    fn sub_GivenEqualPositiveValues_ReturnsZero() {
        // IntegerSubtractTests.Subtract_GivenEqualValues: 42-42 == 0.
        let value = Int::from_i64(42);
        assert!(value.sub(&value).is_zero());
    }

    #[test]
    fn sub_GivenEqualNegativeValues_ReturnsZero() {
        // IntegerSubtractTests.Subtract_GivenEqualNegativeValues: -99-(-99) == 0.
        let value = Int::from_i64(-99);
        assert!(value.sub(&value).is_zero());
    }

    #[test]
    fn sub_GivenZeroSubtrahend_ReturnsOriginal() {
        // IntegerSubtractTests.Subtract_GivenZeroSubtrahend: 17-0 == 17.
        let value = Int::from_i64(17);
        assert_eq!(value.sub(&Int::zero()), value);
    }

    #[test]
    fn sub_GivenNegativeSeventeenMinusZero_ReturnsOriginal() {
        // IntegerSubtractTests.Subtract_GivenNegativeZeroSubtrahend: -17-0 == -17.
        let value = Int::from_i64(-17);
        assert_eq!(value.sub(&Int::zero()), value);
    }

    #[test]
    fn sub_GivenZeroMinusFive_ReturnsMinusFive() {
        // IntegerSubtractTests.Subtract_GivenZeroMinuend: 0-5 == -5.
        assert_eq!(Int::zero().sub(&Int::from_i64(5)), Int::from_i64(-5));
    }

    #[test]
    fn sub_GivenLargeValues_ReturnsOne() {
        // IntegerSubtractTests.Subtract_GivenLargeValues:
        // 1_000_000_000 - 999_999_999 == 1.
        let difference = Int::from_i64(1_000_000_000).sub(&Int::from_i64(999_999_999));
        assert_eq!(difference, Int::from_i64(1));
    }

    #[test]
    fn to_decimal_GivenPositiveValue_ReturnsDigitsWithoutSign() {
        // IntegerToStringTests.ToString_GivenPositiveValue: 123 -> "123".
        assert_eq!(Int::from_i64(123).to_decimal(), "123");
    }

    #[test]
    fn to_decimal_GivenNegativeValue_PrependsMinus() {
        // IntegerToStringTests.ToString_GivenNegativeValue: -123 -> "-123".
        assert_eq!(Int::from_i64(-123).to_decimal(), "-123");
    }

    #[test]
    fn sign_GivenPositiveNegativeAndZero_ReturnsExpectedSigns() {
        assert_eq!(Int::from_i64(9).sign(), 1);
        assert_eq!(Int::from_i64(-9).sign(), -1);
        assert_eq!(Int::from_i64(0).sign(), 0);
    }
}
