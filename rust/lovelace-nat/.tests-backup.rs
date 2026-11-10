#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::Nat;

    #[test]
    fn zero_GivenNoArguments_ReturnsCanonicalZero() {
        let zero = Nat::zero();
        assert!(zero.is_zero());
        assert!(!zero.is_one());
        assert_eq!(zero.to_decimal(), "0");
        assert_eq!(zero.to_u64(), Some(0));
    }

    #[test]
    fn default_GivenNoArguments_EqualsZero() {
        assert_eq!(Nat::default(), Nat::zero());
    }

    #[test]
    fn from_u64_GivenZero_ReturnsZero() {
        assert_eq!(Nat::from_u64(0).to_decimal(), "0");
    }

    #[test]
    fn from_u64_GivenOne_IsOne() {
        assert!(Nat::from_u64(1).is_one());
    }

    #[test]
    fn from_u64_GivenMaximum_RoundTripsThroughU64() {
        let value = Nat::from_u64(u64::MAX);
        assert_eq!(value.to_decimal(), "18446744073709551615");
        assert_eq!(value.to_u64(), Some(u64::MAX));
    }

    #[test]
    fn from_u64_GivenLimbBoundary_EncodesTwoLimbs() {
        assert_eq!(Nat::from_u64(1_000_000_000).to_decimal(), "1000000000");
    }

    #[test]
    fn from_decimal_GivenEmptyString_ReturnsNone() {
        assert!(Nat::from_decimal("").is_none());
    }

    #[test]
    fn from_decimal_GivenNonDigit_ReturnsNone() {
        assert!(Nat::from_decimal("12a3").is_none());
    }

    #[test]
    fn from_decimal_GivenLeadingMinus_ReturnsNone() {
        assert!(Nat::from_decimal("-1").is_none());
    }

    #[test]
    fn from_decimal_GivenLeadingZeros_Canonicalizes() {
        let value = Nat::from_decimal("00000123").unwrap();
        assert_eq!(value.to_decimal(), "123");
    }

    #[test]
    fn from_decimal_GivenAllZeros_ReturnsZero() {
        let value = Nat::from_decimal("000000000000").unwrap();
        assert!(value.is_zero());
        assert_eq!(value.to_decimal(), "0");
    }

    #[test]
    fn to_decimal_GivenInteriorZeroLimb_PadsNineDigits() {
        let value = Nat::from_decimal("1000000000000000001").unwrap();
        assert_eq!(value.to_decimal(), "1000000000000000001");
    }

    #[test]
    fn add_GivenZeroOperand_ReturnsOther() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert_eq!(value.add(&Nat::zero()), value);
    }

    #[test]
    fn add_GivenCarryAcrossLimbBoundary_Propagates() {
        let value = Nat::from_u64(999_999_999).add(&Nat::from_u64(1));
        assert_eq!(value.to_decimal(), "1000000000");
    }

    #[test]
    fn add_GivenCarryChainAcrossThreeLimbs_Propagates() {
        let value = Nat::from_decimal("999999999999999999")
            .unwrap()
            .add(&Nat::from_u64(1));
        assert_eq!(value.to_decimal(), "1000000000000000000");
    }

    #[test]
    fn sub_GivenLargerFromSmaller_ReturnsNone() {
        assert!(Nat::from_u64(1).sub(&Nat::from_u64(2)).is_none());
    }

    #[test]
    fn sub_GivenEqualOperands_ReturnsZero() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert!(value.sub(&value).unwrap().is_zero());
    }

    #[test]
    fn sub_GivenBorrowAcrossLimbs_Propagates() {
        let value = Nat::from_decimal("1000000000000000000")
            .unwrap()
            .sub(&Nat::from_u64(1))
            .unwrap();
        assert_eq!(value.to_decimal(), "999999999999999999");
    }

    #[test]
    fn sub_GivenOneLessThanLimbBoundary_ReturnsLimbMinusOne() {
        let value = Nat::from_u64(1_000_000_000)
            .sub(&Nat::from_u64(1))
            .unwrap();
        assert_eq!(value.to_decimal(), "999999999");
    }

    #[test]
    fn mul_GivenZeroOperand_ReturnsZero() {
        let product = Nat::from_decimal("98765432109876543210")
            .unwrap()
            .mul(&Nat::zero());
        assert!(product.is_zero());
    }

    #[test]
    fn mul_GivenOneOperand_ReturnsSame() {
        let value = Nat::from_decimal("98765432109876543210").unwrap();
        assert_eq!(value.mul(&Nat::from_u64(1)), value);
    }

    #[test]
    fn mul_GivenLimbBoundaryValues_ReturnsExactSquare() {
        let value = Nat::from_u64(999_999_999);
        assert_eq!(value.mul(&value).to_decimal(), "999999998000000001");
    }

    #[test]
    fn mul_GivenLargeOperands_ReturnsExactProduct() {
        let left = Nat::from_decimal("12345678901234567890").unwrap();
        let right = Nat::from_decimal("98765432109876543210").unwrap();
        assert_eq!(
            left.mul(&right).to_decimal(),
            "1219326311370217952237463801111263526900"
        );
    }

    #[test]
    fn div_rem_GivenZeroDivisor_ReturnsNone() {
        assert!(Nat::from_u64(42).div_rem(&Nat::zero()).is_none());
    }

    #[test]
    fn div_rem_GivenSmallerDividend_ReturnsZeroQuotientAndDividendRemainder() {
        let (quotient, remainder) = Nat::from_u64(42).div_rem(&Nat::from_u64(100)).unwrap();
        assert_eq!(quotient.to_decimal(), "0");
        assert_eq!(remainder.to_decimal(), "42");
    }

    #[test]
    fn div_rem_GivenExactDivision_ReturnsZeroRemainder() {
        let dividend = Nat::from_decimal("1000000000000000000").unwrap();
        let divisor = Nat::from_u64(1_000_000_000);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "1000000000");
        assert!(remainder.is_zero());
    }

    #[test]
    fn div_rem_GivenRemainder_ReturnsExactQuotientAndRemainder() {
        let dividend = Nat::from_decimal("1000000000000000001").unwrap();
        let divisor = Nat::from_u64(1_000_000_000);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "1000000000");
        assert_eq!(remainder.to_decimal(), "1");
    }

    #[test]
    fn div_rem_GivenMultiLimbDivisor_ReturnsExactQuotientAndRemainder() {
        let dividend = Nat::from_decimal("98765432109876543210").unwrap();
        let divisor = Nat::from_u64(123_456_789);
        let (quotient, remainder) = dividend.div_rem(&divisor).unwrap();
        assert_eq!(quotient.to_decimal(), "800000007370");
        assert_eq!(remainder.to_decimal(), "8280");
    }

    #[test]
    fn div_rem_GivenEqualOperands_ReturnsOneAndZero() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        let (quotient, remainder) = value.div_rem(&value).unwrap();
        assert!(quotient.is_one());
        assert!(remainder.is_zero());
    }

    #[test]
    fn pow_GivenZeroExponent_ReturnsOne() {
        assert!(Nat::from_decimal("12345678901234567890")
            .unwrap()
            .pow(0)
            .is_one());
    }

    #[test]
    fn pow_GivenZeroBaseAndZeroExponent_ReturnsOne() {
        assert!(Nat::zero().pow(0).is_one());
    }

    #[test]
    fn pow_GivenTwoToHundred_ReturnsKnownValue() {
        assert_eq!(
            Nat::from_u64(2).pow(100).to_decimal(),
            "1267650600228229401496703205376"
        );
    }

    #[test]
    fn pow_GivenZeroBaseAndPositiveExponent_ReturnsZero() {
        assert!(Nat::zero().pow(37).is_zero());
    }

    #[test]
    fn factorial_GivenZero_ReturnsOne() {
        assert!(Nat::factorial(0).is_one());
    }

    #[test]
    fn factorial_GivenOne_ReturnsOne() {
        assert!(Nat::factorial(1).is_one());
    }

    #[test]
    fn factorial_GivenTwenty_ReturnsKnownValue() {
        assert_eq!(Nat::factorial(20).to_decimal(), "2432902008176640000");
    }

    #[test]
    fn factorial_GivenHundred_Has158DigitsAndKnownValue() {
        let value = Nat::factorial(100);
        assert_eq!(value.to_decimal().len(), 158);
        assert_eq!(
            value.to_decimal(),
            "93326215443944152681699238856266700490715968264381621468592963895217599993229915608941463976156518286253697920827223758251185210916864000000000000000000000000"
        );
    }

    #[test]
    fn factorial_GivenThousand_Has2568DigitsAndKnownChecksum() {
        let text = Nat::factorial(1000).to_decimal();
        assert_eq!(text.len(), 2568);
        assert!(text.starts_with("402387260077093773543702433923"));
        let trailing_zeroes = text.bytes().rev().take_while(|byte| *byte == b'0').count();
        assert_eq!(trailing_zeroes, 249);
        let digit_sum: u32 = text
            .bytes()
            .map(|byte| u32::from(byte - b'0'))
            .sum();
        assert_eq!(digit_sum, 10539);
    }

    #[test]
    fn mul_pow10_GivenZeroExponent_ReturnsSame() {
        let value = Nat::from_decimal("12345678901234567890").unwrap();
        assert_eq!(value.mul_pow10(0), value);
    }

    #[test]
    fn mul_pow10_GivenNine_AppendsZeroLimb() {
        assert_eq!(
            Nat::from_u64(123).mul_pow10(9).to_decimal(),
            "123000000000"
        );
    }

    #[test]
    fn mul_pow10_GivenNineteen_CrossesTwoLimbs() {
        assert_eq!(
            Nat::from_u64(42).mul_pow10(19).to_decimal(),
            "420000000000000000000"
        );
    }

    #[test]
    fn to_u64_GivenValueAboveMaximum_ReturnsNone() {
        let value = Nat::from_decimal("18446744073709551616").unwrap();
        assert_eq!(value.to_u64(), None);
    }

    #[test]
    fn ord_GivenDifferentMagnitudes_OrdersNumerically() {
        let larger = Nat::from_decimal("1000000000000000000").unwrap();
        let smaller = Nat::from_decimal("999999999999999999").unwrap();
        assert!(larger > smaller);
    }

    #[test]
    fn parse_then_format_GivenLargeLiteral_RoundTrips() {
        let literal = "314159265358979323846264338327950288419716939937510";
        let value = Nat::from_decimal(literal).unwrap();
        assert_eq!(value.to_decimal(), literal);
    }
}
