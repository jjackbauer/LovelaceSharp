#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use crate::Nat;

    fn nat(limbs: &[u64]) -> Nat {
        crate::Nat::from_limbs(limbs.to_vec())
    }

    fn add_limbs(a: &[u64], b: &[u64]) -> Vec<u64> {
        let len = a.len().max(b.len());
        let mut out = Vec::with_capacity(len + 1);
        let mut carry = 0u64;
        for i in 0..len {
            let sum = a.get(i).copied().unwrap_or(0) as u128
                + b.get(i).copied().unwrap_or(0) as u128
                + carry as u128;
            out.push(sum as u64);
            carry = (sum >> 64) as u64;
        }
        if carry != 0 {
            out.push(carry);
        }
        out
    }

    /// Deterministic xorshift64 generator; the tests never depend on the platform RNG.
    struct XorShift(u64);

    impl XorShift {
        fn new(seed: u64) -> Self {
            XorShift(seed | 1)
        }

        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn limbs(&mut self, len: usize) -> Vec<u64> {
            let mut out = Vec::with_capacity(len);
            for _ in 0..len {
                out.push(self.next());
            }
            if let Some(top) = out.last_mut() {
                if *top == 0 {
                    *top = 1;
                }
            }
            out
        }

        fn normalized(&mut self, len: usize) -> Vec<u64> {
            let mut out = self.limbs(len);
            *out.last_mut().expect("nonzero length") |= 1 << 63;
            out
        }
    }

    /// Cross-checks a result against both the existing long division in lib.rs and the
    /// module's own schoolbook path, then asserts the division invariant.
    fn assert_case(num: &[u64], den: &[u64], label: &str) {
        let (q, r) = div_rem_newton(num, den).expect("nonzero divisor");
        assert!(q.last() != Some(&0), "{label}: quotient has a trailing zero limb");
        assert!(r.last() != Some(&0), "{label}: remainder has a trailing zero limb");

        let (expected_q, expected_r) = nat(num).div_rem(&nat(den)).expect("nonzero divisor");
        // Nat::div_rem can leak trailing zero limbs when its normalisation shift is zero
        // (lib.rs is out of scope); normalise the reference before comparing exact limbs.
        assert_eq!(q, norm(expected_q.limbs), "{label}: quotient differs from Nat::div_rem");
        assert_eq!(r, norm(expected_r.limbs), "{label}: remainder differs from Nat::div_rem");

        let (book_q, book_r) = schoolbook_div_rem(num, den).expect("nonzero divisor");
        assert_eq!(q, book_q, "{label}: quotient differs from schoolbook");
        assert_eq!(r, book_r, "{label}: remainder differs from schoolbook");

        let rebuilt = add_limbs(&mul(&q, den), &r);
        assert_eq!(rebuilt.as_slice(), num, "{label}: q*den + r != num");
        assert_eq!(cmp_limbs(&r, den), Ordering::Less, "{label}: remainder not below divisor");
    }

    #[test]
    fn div_rem_newton_GivenRandomOperands_AgreesWithLongDivision() {
        let mut rng = XorShift::new(0x0123_4567_89ab_cdef);
        for len in 1..=200usize {
            let den = rng.limbs(len);
            let num = rng.limbs(len);
            assert_case(&num, &den, &format!("comparable at {len} limbs"));

            let den = rng.limbs(len);
            let num = rng.limbs(len + 1);
            assert_case(&num, &den, &format!("one extra limb at {len}"));

            let value = rng.limbs(len);
            assert_case(&value, &value, &format!("equal operands at {len} limbs"));

            let den = rng.limbs(len + 1);
            let num = rng.limbs(len);
            assert_case(&num, &den, &format!("numerator smaller at {len} limbs"));
        }
    }

    #[test]
    fn div_rem_newton_GivenLargeRatioOperands_UsesNewtonPathAndIsExact() {
        let mut rng = XorShift::new(0x1111_2222_3333_4444);
        for &(num_len, den_len) in
            &[(200usize, 32usize), (200, 33), (200, 100), (128, 64), (64, 32), (201, 32)]
        {
            let mut den = rng.limbs(den_len);
            den[den_len - 1] |= 1 << 63;
            let num = rng.limbs(num_len);
            assert_case(&num, &den, &format!("ratio {num_len}/{den_len}"));
            let direct = newton_div_rem(&num, &den);
            assert_eq!(direct, div_rem_newton(&num, &den).unwrap(), "ratio {num_len}/{den_len}");
        }
    }

    #[test]
    fn div_rem_newton_GivenTopLimbOne_AgreesWithLongDivision() {
        let mut rng = XorShift::new(0x5555_6666_7777_8888);
        for &den_len in &[32usize, 48, 64, 100] {
            let mut den = rng.limbs(den_len);
            den[den_len - 1] = 1;
            for extra in 0..=2usize {
                let mut num = rng.limbs(den_len + extra);
                if cmp_limbs(&num, &den) != Ordering::Greater {
                    num = add_limbs(&den, &[1]);
                }
                assert_case(&num, &den, &format!("top limb one, {den_len} limbs, extra {extra}"));
            }
        }
    }

    #[test]
    fn div_rem_newton_GivenPowerOfTwoDivisor_AgreesWithLongDivision() {
        let mut rng = XorShift::new(0x9999_aaaa_bbbb_cccc);
        for &den_len in &[32usize, 40, 64] {
            for &top in &[1u64, 1u64 << 63] {
                let mut den = vec![0u64; den_len];
                den[den_len - 1] = top;
                for extra in 0..=2usize {
                    let mut num = rng.limbs(den_len + extra);
                    if cmp_limbs(&num, &den) != Ordering::Greater {
                        num = add_limbs(&den, &[12345]);
                    }
                    assert_case(&num, &den, &format!("power of two, {den_len} limbs, extra {extra}"));
                }
            }
        }
    }

    #[test]
    fn div_rem_newton_GivenQuotientOneAndNearDivisorRemainder_IsExact() {
        let mut rng = XorShift::new(0xabcd_ef01_2345_6789);
        for &den_len in &[32usize, 48, 64] {
            let den = rng.normalized(den_len);
            let num = dec(&add_limbs(&den, &den)); // 2*den - 1
            let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
            assert_eq!(q, vec![1], "quotient should be one for {den_len} limbs");
            assert_eq!(r, dec(&den), "remainder should be den - 1");
            assert_case(&num, &den, "quotient one, remainder nearly den");
        }
    }

    #[test]
    fn div_rem_newton_GivenExactDivision_CorrectionRepairsUnderestimate() {
        let mut rng = XorShift::new(0x0f0f_1e1e_2d2d_3c3c);
        let mut den = rng.normalized(64);
        den[0] |= 1; // odd divisor cannot divide B^k, so the raw estimate is strictly low
        let mut factor = rng.limbs(3);
        factor[2] |= 1 << 32;
        let num = mul(&den, &factor);

        let raw = estimate_quotient(&num, &den);
        assert_eq!(raw, dec(&factor), "raw Newton estimate should be one too small");
        assert_eq!(
            cmp_limbs(&mul(&raw, &den), &num),
            Ordering::Less,
            "the uncorrected estimate should violate the quotient bound"
        );

        let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
        assert_eq!(q, factor, "correction should recover the exact quotient");
        assert!(r.is_empty(), "exact division should leave no remainder");
        assert_case(&num, &den, "exact division correction");
    }

    #[test]
    fn correct_quotient_GivenEstimateOffByTwo_ReturnsExactQuotient() {
        let mut rng = XorShift::new(0x1234_5678_9abc_def0);
        let den = rng.normalized(40);
        let num = rng.limbs(80);
        let (q, r) = schoolbook_div_rem(&num, &den).expect("nonzero divisor");
        assert!(q.len() >= 2, "test needs a multi-limb quotient");
        for delta in [-2i64, -1, 0, 1, 2] {
            let estimate = if delta < 0 {
                sub(&q, &[(-delta) as u64])
            } else if delta > 0 {
                add_limbs(&q, &[delta as u64])
            } else {
                q.clone()
            };
            let (fixed_q, fixed_r) = correct_quotient(&num, &den, estimate);
            assert_eq!(fixed_q, q, "quotient not repaired for delta {delta}");
            assert_eq!(fixed_r, r, "remainder not repaired for delta {delta}");
        }
    }

    #[test]
    fn div_rem_newton_GivenRandomOperands_PreservesDivisionInvariant() {
        let mut rng = XorShift::new(0xfeed_face_cafe_beef);
        for case in 0..120usize {
            let den_len = 1 + (rng.next() as usize % 200);
            let num_len = if case % 2 == 0 { den_len } else { den_len + 1 };
            let den = rng.limbs(den_len);
            let num = rng.limbs(num_len);
            let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
            assert!(q.last() != Some(&0), "case {case}: quotient not canonical");
            assert!(r.last() != Some(&0), "case {case}: remainder not canonical");
            assert_eq!(
                add_limbs(&mul(&q, &den), &r).as_slice(),
                num.as_slice(),
                "case {case}: q*den + r != num"
            );
            assert_eq!(cmp_limbs(&r, &den), Ordering::Less, "case {case}: r >= den");

            let (expected_q, expected_r) = nat(&num).div_rem(&nat(&den)).expect("nonzero divisor");
            assert_eq!(q, norm(expected_q.limbs), "case {case}: quotient mismatch");
            assert_eq!(r, norm(expected_r.limbs), "case {case}: remainder mismatch");
        }
    }

    #[test]
    fn div_rem_newton_GivenAllMaxLimbs_AgreesWithLongDivision() {
        // Carry-chain stress: every limb and every partial product is saturated.
        for &(num_len, den_len) in
            &[(64usize, 32usize), (128, 64), (200, 32), (200, 100), (200, 199)]
        {
            let num = vec![u64::MAX; num_len];
            let den = vec![u64::MAX; den_len];
            assert_case(&num, &den, &format!("all-max limbs {num_len}/{den_len}"));
            assert_eq!(
                newton_div_rem(&num, &den),
                div_rem_newton(&num, &den).unwrap(),
                "direct Newton path disagrees for all-max limbs {num_len}/{den_len}"
            );
        }
    }

    #[test]
    fn div_rem_newton_GivenZeroOperands_ReturnsCanonicalValues() {
        assert!(div_rem_newton(&[], &[]).is_none());
        assert!(div_rem_newton(&[7, 8], &[]).is_none());
        assert!(div_rem_newton(&[7], &[0, 0]).is_none());
        assert_eq!(div_rem_newton(&[], &[9]).unwrap(), (Vec::new(), Vec::new()));
        assert_eq!(div_rem_newton(&[0, 0], &[9]).unwrap(), (Vec::new(), Vec::new()));
    }

    #[test]
    fn div_rem_newton_GivenSingleLimbDivisor_AgreesWithLongDivision() {
        let mut rng = XorShift::new(0x2468_ace0_1357_9bdf);
        for &len in &[1usize, 2, 31, 32, 64, 200] {
            let den = [rng.next() | 1];
            let num = rng.limbs(len);
            assert_case(&num, &den, &format!("single-limb divisor, {len} limbs"));
        }
        assert_case(&[u64::MAX], &[1], "single limb one");
        assert_case(&[0, 0, 1], &[u64::MAX], "single limb max");
    }

    #[test]
    fn div_rem_newton_GivenNumeratorSmallerThanDivisor_ReturnsNumerator() {
        let mut rng = XorShift::new(0x1357_9bdf_2468_ace0);
        let den = rng.normalized(64);
        let num = rng.limbs(63);
        let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
        assert!(q.is_empty(), "smaller numerator must give a zero quotient");
        assert_eq!(r, num, "remainder should be the numerator");
    }

    #[test]
    fn div_rem_newton_GivenOneExtraLimb_ReturnsExactQuotient() {
        // den = B^64/2 + 1, num = B^65 - 1: quotient is two limbs, no rounding.
        let mut den = vec![0u64; 64];
        den[0] = 1;
        den[63] = 1 << 63;
        let num = vec![u64::MAX; 65];
        let (q, r) = div_rem_newton(&num, &den).expect("nonzero divisor");
        assert_eq!(q.len(), 2, "quotient of a one-extra-limb division should have two limbs");
        assert_case(&num, &den, "one extra limb two-limb quotient");
        assert!(cmp_limbs(&r, &den) == Ordering::Less);
    }
}
