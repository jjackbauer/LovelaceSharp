//! Round-7 parity batch: the periodic exact-rational corpus rows that the preperiod conversion
//! defect blocked in round 6. Test-only module; production code stays in `lib.rs`.
#![allow(non_snake_case)]

use super::*;

fn r(s: &str) -> Real {
    Real::from_str(s).unwrap()
}

fn is_periodic(v: &Real) -> bool {
    v.to_string().contains('(')
}

fn gcd(mut a: Nat, mut b: Nat) -> Nat {
    while !b.is_zero() {
        let (_, rem) = a.div_rem(&b).unwrap();
        a = b;
        b = rem;
    }
    a
}

fn lowest_terms(num: Int, den: Nat) -> (Int, Nat) {
    if num.is_zero() {
        return (Int::zero(), Nat::from_u64(1));
    }
    let g = gcd(num.magnitude().clone(), den.clone());
    if g.is_one() {
        return (num, den);
    }
    let (n, _) = num.div_rem(&g).unwrap();
    let (d, _) = den.div_rem(&g).unwrap();
    (n, d)
}

fn check_add_row(p: i64, q: i64, r: i64, s: i64) {
    let left = Real::from_i64(p).div(&Real::from_i64(q), 1000).unwrap();
    let right = Real::from_i64(r).div(&Real::from_i64(s), 1000).unwrap();
    assert!(
        is_periodic(&left),
        "corpus defect: {p}/{q} is not periodic (renders {})",
        left
    );
    let sum = left.add(&right);
    let (en, ed) = lowest_terms(Int::from_i64(p * s + r * q), Nat::from_u64((q * s) as u64));
    let (an, ad) = sum.to_rational();
    assert!(
        an == en && ad == ed,
        "({p}/{q}) + ({r}/{s}) denotes {}/{} ({}), expected {}/{}",
        an.to_decimal(),
        ad.to_decimal(),
        sum,
        en.to_decimal(),
        ed.to_decimal()
    );
    let expected = fraction_real(en, ed, 1000);
    assert!(
        sum == expected,
        "({p}/{q}) + ({r}/{s}) = {}, expected {}",
        sum,
        expected
    );
}

fn check_sub_row(p: i64, q: i64, r: i64, s: i64) {
    let left = Real::from_i64(p).div(&Real::from_i64(q), 1000).unwrap();
    let right = Real::from_i64(r).div(&Real::from_i64(s), 1000).unwrap();
    assert!(
        is_periodic(&left),
        "corpus defect: {p}/{q} is not periodic (renders {})",
        left
    );
    let difference = left.sub(&right);
    let (en, ed) = lowest_terms(Int::from_i64(p * s - r * q), Nat::from_u64((q * s) as u64));
    let (an, ad) = difference.to_rational();
    assert!(
        an == en && ad == ed,
        "({p}/{q}) - ({r}/{s}) denotes {}/{} ({}), expected {}/{}",
        an.to_decimal(),
        ad.to_decimal(),
        difference,
        en.to_decimal(),
        ed.to_decimal()
    );
    let expected = fraction_real(en, ed, 1000);
    assert!(
        difference == expected,
        "({p}/{q}) - ({r}/{s}) = {}, expected {}",
        difference,
        expected
    );
}

fn check_mul_row(p: i64, divisor: &str) {
    let q = r(divisor);
    let quotient = Real::from_i64(p).div(&q, 1000).unwrap();
    let product = quotient.mul(&q);
    assert!(
        product == Real::from_i64(p),
        "p={p}, q={divisor}: (p/q)*q = {}, expected {p}",
        product
    );
    let (n, d) = product.to_rational();
    assert!(
        n == Int::from_i64(p) && d == Nat::from_u64(1),
        "p={p}, q={divisor}: (p/q)*q denotes {}/{}",
        n.to_decimal(),
        d.to_decimal()
    );
}

#[test]
fn r7_add_p2_q6_r1_s2() {
    check_add_row(2, 6, 1, 2);
}
#[test]
fn r7_add_p2_q6_rm1_s2() {
    check_add_row(2, 6, -1, 2);
}
#[test]
fn r7_add_p2_q6_r5_s2() {
    check_add_row(2, 6, 5, 2);
}
#[test]
fn r7_add_p2_q6_r1_s4() {
    check_add_row(2, 6, 1, 4);
}
#[test]
fn r7_add_p2_q6_rm1_s4() {
    check_add_row(2, 6, -1, 4);
}
#[test]
fn r7_add_p2_q6_r5_s4() {
    check_add_row(2, 6, 5, 4);
}
#[test]
fn r7_add_p2_q6_r1_s25() {
    check_add_row(2, 6, 1, 25);
}
#[test]
fn r7_add_p2_q6_rm1_s25() {
    check_add_row(2, 6, -1, 25);
}
#[test]
fn r7_add_p2_q6_r5_s25() {
    check_add_row(2, 6, 5, 25);
}
#[test]
fn r7_add_p2_q6_r1_s1000() {
    check_add_row(2, 6, 1, 1000);
}
#[test]
fn r7_add_p2_q6_rm1_s1000() {
    check_add_row(2, 6, -1, 1000);
}
#[test]
fn r7_add_p2_q6_r5_s1000() {
    check_add_row(2, 6, 5, 1000);
}
#[test]
fn r7_add_p1_q3_r1_s2() {
    check_add_row(1, 3, 1, 2);
}
#[test]
fn r7_add_pm5_q6_r1_s3() {
    check_add_row(-5, 6, 1, 3);
}
#[test]
fn r7_add_pm5_q6_rm1_s3() {
    check_add_row(-5, 6, -1, 3);
}
#[test]
fn r7_add_pm5_q6_r5_s3() {
    check_add_row(-5, 6, 5, 3);
}
#[test]
fn r7_add_pm5_q6_r1_s7() {
    check_add_row(-5, 6, 1, 7);
}
#[test]
fn r7_add_pm5_q6_rm1_s7() {
    check_add_row(-5, 6, -1, 7);
}
#[test]
fn r7_add_pm5_q6_r5_s7() {
    check_add_row(-5, 6, 5, 7);
}
#[test]
fn r7_add_pm5_q6_r1_s17() {
    check_add_row(-5, 6, 1, 17);
}
#[test]
fn r7_add_pm5_q6_rm1_s17() {
    check_add_row(-5, 6, -1, 17);
}
#[test]
fn r7_add_pm5_q6_r5_s17() {
    check_add_row(-5, 6, 5, 17);
}
#[test]
fn r7_add_pm5_q6_r1_s99() {
    check_add_row(-5, 6, 1, 99);
}
#[test]
fn r7_add_p1_q3_rm1_s2() {
    check_add_row(1, 3, -1, 2);
}
#[test]
fn r7_add_pm5_q6_rm1_s99() {
    check_add_row(-5, 6, -1, 99);
}
#[test]
fn r7_add_pm5_q6_r5_s99() {
    check_add_row(-5, 6, 5, 99);
}
#[test]
fn r7_add_pm5_q6_r1_s2() {
    check_add_row(-5, 6, 1, 2);
}
#[test]
fn r7_add_pm5_q6_rm1_s2() {
    check_add_row(-5, 6, -1, 2);
}
#[test]
fn r7_add_pm5_q6_r5_s2() {
    check_add_row(-5, 6, 5, 2);
}
#[test]
fn r7_add_pm5_q6_r1_s4() {
    check_add_row(-5, 6, 1, 4);
}
#[test]
fn r7_add_pm5_q6_rm1_s4() {
    check_add_row(-5, 6, -1, 4);
}
#[test]
fn r7_add_pm5_q6_r5_s4() {
    check_add_row(-5, 6, 5, 4);
}
#[test]
fn r7_add_pm5_q6_r1_s25() {
    check_add_row(-5, 6, 1, 25);
}
#[test]
fn r7_add_pm5_q6_rm1_s25() {
    check_add_row(-5, 6, -1, 25);
}
#[test]
fn r7_add_p1_q3_r5_s2() {
    check_add_row(1, 3, 5, 2);
}
#[test]
fn r7_add_pm5_q6_r5_s25() {
    check_add_row(-5, 6, 5, 25);
}
#[test]
fn r7_add_pm5_q6_r1_s1000() {
    check_add_row(-5, 6, 1, 1000);
}
#[test]
fn r7_add_pm5_q6_rm1_s1000() {
    check_add_row(-5, 6, -1, 1000);
}
#[test]
fn r7_add_pm5_q6_r5_s1000() {
    check_add_row(-5, 6, 5, 1000);
}
#[test]
fn r7_add_p1_q3_r1_s4() {
    check_add_row(1, 3, 1, 4);
}
#[test]
fn r7_add_p1_q7_r1_s2() {
    check_add_row(1, 7, 1, 2);
}
#[test]
fn r7_add_p1_q7_rm1_s2() {
    check_add_row(1, 7, -1, 2);
}
#[test]
fn r7_add_p1_q7_r5_s2() {
    check_add_row(1, 7, 5, 2);
}
#[test]
fn r7_add_p1_q7_r1_s4() {
    check_add_row(1, 7, 1, 4);
}
#[test]
fn r7_add_p1_q3_rm1_s4() {
    check_add_row(1, 3, -1, 4);
}
#[test]
fn r7_add_p1_q7_rm1_s4() {
    check_add_row(1, 7, -1, 4);
}
#[test]
fn r7_add_p1_q7_r5_s4() {
    check_add_row(1, 7, 5, 4);
}
#[test]
fn r7_add_p1_q7_r1_s25() {
    check_add_row(1, 7, 1, 25);
}
#[test]
fn r7_add_p1_q7_rm1_s25() {
    check_add_row(1, 7, -1, 25);
}
#[test]
fn r7_add_p1_q7_r5_s25() {
    check_add_row(1, 7, 5, 25);
}
#[test]
fn r7_add_p1_q7_r1_s1000() {
    check_add_row(1, 7, 1, 1000);
}
#[test]
fn r7_add_p1_q7_rm1_s1000() {
    check_add_row(1, 7, -1, 1000);
}
#[test]
fn r7_add_p1_q7_r5_s1000() {
    check_add_row(1, 7, 5, 1000);
}
#[test]
fn r7_add_p1_q3_r5_s4() {
    check_add_row(1, 3, 5, 4);
}
#[test]
fn r7_add_p1_q3_r1_s25() {
    check_add_row(1, 3, 1, 25);
}
#[test]
fn r7_add_p2_q7_r1_s2() {
    check_add_row(2, 7, 1, 2);
}
#[test]
fn r7_add_p2_q7_rm1_s2() {
    check_add_row(2, 7, -1, 2);
}
#[test]
fn r7_add_p2_q7_r5_s2() {
    check_add_row(2, 7, 5, 2);
}
#[test]
fn r7_add_p2_q7_r1_s4() {
    check_add_row(2, 7, 1, 4);
}
#[test]
fn r7_add_p2_q7_rm1_s4() {
    check_add_row(2, 7, -1, 4);
}
#[test]
fn r7_add_p2_q7_r5_s4() {
    check_add_row(2, 7, 5, 4);
}
#[test]
fn r7_add_p2_q7_r1_s25() {
    check_add_row(2, 7, 1, 25);
}
#[test]
fn r7_add_p2_q7_rm1_s25() {
    check_add_row(2, 7, -1, 25);
}
#[test]
fn r7_add_p2_q7_r5_s25() {
    check_add_row(2, 7, 5, 25);
}
#[test]
fn r7_add_p2_q7_r1_s1000() {
    check_add_row(2, 7, 1, 1000);
}
#[test]
fn r7_add_p1_q3_rm1_s25() {
    check_add_row(1, 3, -1, 25);
}
#[test]
fn r7_add_p2_q7_rm1_s1000() {
    check_add_row(2, 7, -1, 1000);
}
#[test]
fn r7_add_p2_q7_r5_s1000() {
    check_add_row(2, 7, 5, 1000);
}
#[test]
fn r7_add_p1_q3_r5_s25() {
    check_add_row(1, 3, 5, 25);
}
#[test]
fn r7_add_pm5_q7_r1_s2() {
    check_add_row(-5, 7, 1, 2);
}
#[test]
fn r7_add_pm5_q7_rm1_s2() {
    check_add_row(-5, 7, -1, 2);
}
#[test]
fn r7_add_pm5_q7_r5_s2() {
    check_add_row(-5, 7, 5, 2);
}
#[test]
fn r7_add_pm5_q7_r1_s4() {
    check_add_row(-5, 7, 1, 4);
}
#[test]
fn r7_add_pm5_q7_rm1_s4() {
    check_add_row(-5, 7, -1, 4);
}
#[test]
fn r7_add_pm5_q7_r5_s4() {
    check_add_row(-5, 7, 5, 4);
}
#[test]
fn r7_add_p1_q3_r1_s1000() {
    check_add_row(1, 3, 1, 1000);
}
#[test]
fn r7_add_pm5_q7_r1_s25() {
    check_add_row(-5, 7, 1, 25);
}
#[test]
fn r7_add_pm5_q7_rm1_s25() {
    check_add_row(-5, 7, -1, 25);
}
#[test]
fn r7_add_pm5_q7_r5_s25() {
    check_add_row(-5, 7, 5, 25);
}
#[test]
fn r7_add_pm5_q7_r1_s1000() {
    check_add_row(-5, 7, 1, 1000);
}
#[test]
fn r7_add_pm5_q7_rm1_s1000() {
    check_add_row(-5, 7, -1, 1000);
}
#[test]
fn r7_add_pm5_q7_r5_s1000() {
    check_add_row(-5, 7, 5, 1000);
}
#[test]
fn r7_add_p1_q3_rm1_s1000() {
    check_add_row(1, 3, -1, 1000);
}
#[test]
fn r7_add_p1_q9_r1_s2() {
    check_add_row(1, 9, 1, 2);
}
#[test]
fn r7_add_p1_q9_rm1_s2() {
    check_add_row(1, 9, -1, 2);
}
#[test]
fn r7_add_p1_q3_r5_s1000() {
    check_add_row(1, 3, 5, 1000);
}
#[test]
fn r7_add_p1_q9_r5_s2() {
    check_add_row(1, 9, 5, 2);
}
#[test]
fn r7_add_p1_q9_r1_s4() {
    check_add_row(1, 9, 1, 4);
}
#[test]
fn r7_add_p1_q9_rm1_s4() {
    check_add_row(1, 9, -1, 4);
}
#[test]
fn r7_add_p1_q9_r5_s4() {
    check_add_row(1, 9, 5, 4);
}
#[test]
fn r7_add_p1_q9_r1_s25() {
    check_add_row(1, 9, 1, 25);
}
#[test]
fn r7_add_p1_q9_rm1_s25() {
    check_add_row(1, 9, -1, 25);
}
#[test]
fn r7_add_p1_q9_r5_s25() {
    check_add_row(1, 9, 5, 25);
}
#[test]
fn r7_add_p1_q9_r1_s1000() {
    check_add_row(1, 9, 1, 1000);
}
#[test]
fn r7_add_p1_q9_rm1_s1000() {
    check_add_row(1, 9, -1, 1000);
}
#[test]
fn r7_add_p1_q9_r5_s1000() {
    check_add_row(1, 9, 5, 1000);
}
#[test]
fn r7_add_p2_q9_r1_s2() {
    check_add_row(2, 9, 1, 2);
}
#[test]
fn r7_add_p2_q9_rm1_s2() {
    check_add_row(2, 9, -1, 2);
}
#[test]
fn r7_add_p2_q9_r5_s2() {
    check_add_row(2, 9, 5, 2);
}
#[test]
fn r7_add_p2_q9_r1_s4() {
    check_add_row(2, 9, 1, 4);
}
#[test]
fn r7_add_p2_q9_rm1_s4() {
    check_add_row(2, 9, -1, 4);
}
#[test]
fn r7_add_p2_q9_r5_s4() {
    check_add_row(2, 9, 5, 4);
}
#[test]
fn r7_add_p2_q9_r1_s25() {
    check_add_row(2, 9, 1, 25);
}
#[test]
fn r7_add_p2_q9_rm1_s25() {
    check_add_row(2, 9, -1, 25);
}
#[test]
fn r7_add_p2_q9_r5_s25() {
    check_add_row(2, 9, 5, 25);
}
#[test]
fn r7_add_p2_q9_r1_s1000() {
    check_add_row(2, 9, 1, 1000);
}
#[test]
fn r7_add_p2_q9_rm1_s1000() {
    check_add_row(2, 9, -1, 1000);
}
#[test]
fn r7_add_p2_q9_r5_s1000() {
    check_add_row(2, 9, 5, 1000);
}
#[test]
fn r7_add_pm5_q9_r1_s2() {
    check_add_row(-5, 9, 1, 2);
}
#[test]
fn r7_add_pm5_q9_rm1_s2() {
    check_add_row(-5, 9, -1, 2);
}
#[test]
fn r7_add_pm5_q9_r5_s2() {
    check_add_row(-5, 9, 5, 2);
}
#[test]
fn r7_add_pm5_q9_r1_s4() {
    check_add_row(-5, 9, 1, 4);
}
#[test]
fn r7_add_pm5_q9_rm1_s4() {
    check_add_row(-5, 9, -1, 4);
}
#[test]
fn r7_add_pm5_q9_r5_s4() {
    check_add_row(-5, 9, 5, 4);
}
#[test]
fn r7_add_pm5_q9_r1_s25() {
    check_add_row(-5, 9, 1, 25);
}
#[test]
fn r7_add_pm5_q9_rm1_s25() {
    check_add_row(-5, 9, -1, 25);
}
#[test]
fn r7_add_pm5_q9_r5_s25() {
    check_add_row(-5, 9, 5, 25);
}
#[test]
fn r7_add_pm5_q9_r1_s1000() {
    check_add_row(-5, 9, 1, 1000);
}
#[test]
fn r7_add_pm5_q9_rm1_s1000() {
    check_add_row(-5, 9, -1, 1000);
}
#[test]
fn r7_add_pm5_q9_r5_s1000() {
    check_add_row(-5, 9, 5, 1000);
}
#[test]
fn r7_add_p1_q11_r1_s2() {
    check_add_row(1, 11, 1, 2);
}
#[test]
fn r7_add_p1_q11_rm1_s2() {
    check_add_row(1, 11, -1, 2);
}
#[test]
fn r7_add_p1_q11_r5_s2() {
    check_add_row(1, 11, 5, 2);
}
#[test]
fn r7_add_p1_q11_r1_s4() {
    check_add_row(1, 11, 1, 4);
}
#[test]
fn r7_add_p1_q11_rm1_s4() {
    check_add_row(1, 11, -1, 4);
}
#[test]
fn r7_add_p1_q11_r5_s4() {
    check_add_row(1, 11, 5, 4);
}
#[test]
fn r7_add_p1_q11_r1_s25() {
    check_add_row(1, 11, 1, 25);
}
#[test]
fn r7_add_p1_q11_rm1_s25() {
    check_add_row(1, 11, -1, 25);
}
#[test]
fn r7_add_p1_q11_r5_s25() {
    check_add_row(1, 11, 5, 25);
}
#[test]
fn r7_add_p1_q11_r1_s1000() {
    check_add_row(1, 11, 1, 1000);
}
#[test]
fn r7_add_p1_q11_rm1_s1000() {
    check_add_row(1, 11, -1, 1000);
}
#[test]
fn r7_add_p1_q11_r5_s1000() {
    check_add_row(1, 11, 5, 1000);
}
#[test]
fn r7_add_p2_q11_r1_s2() {
    check_add_row(2, 11, 1, 2);
}
#[test]
fn r7_add_p2_q11_rm1_s2() {
    check_add_row(2, 11, -1, 2);
}
#[test]
fn r7_add_p2_q11_r5_s2() {
    check_add_row(2, 11, 5, 2);
}
#[test]
fn r7_add_p2_q11_r1_s4() {
    check_add_row(2, 11, 1, 4);
}
#[test]
fn r7_add_p2_q11_rm1_s4() {
    check_add_row(2, 11, -1, 4);
}
#[test]
fn r7_add_p2_q11_r5_s4() {
    check_add_row(2, 11, 5, 4);
}
#[test]
fn r7_add_p2_q11_r1_s25() {
    check_add_row(2, 11, 1, 25);
}
#[test]
fn r7_add_p2_q11_rm1_s25() {
    check_add_row(2, 11, -1, 25);
}
#[test]
fn r7_add_p2_q11_r5_s25() {
    check_add_row(2, 11, 5, 25);
}
#[test]
fn r7_add_p2_q11_r1_s1000() {
    check_add_row(2, 11, 1, 1000);
}
#[test]
fn r7_add_p2_q11_rm1_s1000() {
    check_add_row(2, 11, -1, 1000);
}
#[test]
fn r7_add_p2_q11_r5_s1000() {
    check_add_row(2, 11, 5, 1000);
}
#[test]
fn r7_add_pm5_q11_r1_s2() {
    check_add_row(-5, 11, 1, 2);
}
#[test]
fn r7_add_pm5_q11_rm1_s2() {
    check_add_row(-5, 11, -1, 2);
}
#[test]
fn r7_add_pm5_q11_r5_s2() {
    check_add_row(-5, 11, 5, 2);
}
#[test]
fn r7_add_pm5_q11_r1_s4() {
    check_add_row(-5, 11, 1, 4);
}
#[test]
fn r7_add_pm5_q11_rm1_s4() {
    check_add_row(-5, 11, -1, 4);
}
#[test]
fn r7_add_pm5_q11_r5_s4() {
    check_add_row(-5, 11, 5, 4);
}
#[test]
fn r7_add_pm5_q11_r1_s25() {
    check_add_row(-5, 11, 1, 25);
}
#[test]
fn r7_add_pm5_q11_rm1_s25() {
    check_add_row(-5, 11, -1, 25);
}
#[test]
fn r7_add_pm5_q11_r5_s25() {
    check_add_row(-5, 11, 5, 25);
}
#[test]
fn r7_add_pm5_q11_r1_s1000() {
    check_add_row(-5, 11, 1, 1000);
}
#[test]
fn r7_add_pm5_q11_rm1_s1000() {
    check_add_row(-5, 11, -1, 1000);
}
#[test]
fn r7_add_pm5_q11_r5_s1000() {
    check_add_row(-5, 11, 5, 1000);
}
#[test]
fn r7_add_p2_q3_r1_s2() {
    check_add_row(2, 3, 1, 2);
}
#[test]
fn r7_add_p1_q12_r1_s3() {
    check_add_row(1, 12, 1, 3);
}
#[test]
fn r7_add_p1_q12_rm1_s3() {
    check_add_row(1, 12, -1, 3);
}
#[test]
fn r7_add_p1_q12_r5_s3() {
    check_add_row(1, 12, 5, 3);
}
#[test]
fn r7_add_p1_q12_r1_s7() {
    check_add_row(1, 12, 1, 7);
}
#[test]
fn r7_add_p1_q12_rm1_s7() {
    check_add_row(1, 12, -1, 7);
}
#[test]
fn r7_add_p1_q12_r5_s7() {
    check_add_row(1, 12, 5, 7);
}
#[test]
fn r7_add_p1_q12_r1_s17() {
    check_add_row(1, 12, 1, 17);
}
#[test]
fn r7_add_p1_q12_rm1_s17() {
    check_add_row(1, 12, -1, 17);
}
#[test]
fn r7_add_p1_q12_r5_s17() {
    check_add_row(1, 12, 5, 17);
}
#[test]
fn r7_add_p1_q12_r1_s99() {
    check_add_row(1, 12, 1, 99);
}
#[test]
fn r7_add_p2_q3_rm1_s2() {
    check_add_row(2, 3, -1, 2);
}
#[test]
fn r7_add_p1_q12_rm1_s99() {
    check_add_row(1, 12, -1, 99);
}
#[test]
fn r7_add_p1_q12_r5_s99() {
    check_add_row(1, 12, 5, 99);
}
#[test]
fn r7_add_p1_q12_r1_s2() {
    check_add_row(1, 12, 1, 2);
}
#[test]
fn r7_add_p1_q12_rm1_s2() {
    check_add_row(1, 12, -1, 2);
}
#[test]
fn r7_add_p1_q12_r5_s2() {
    check_add_row(1, 12, 5, 2);
}
#[test]
fn r7_add_p1_q12_r1_s4() {
    check_add_row(1, 12, 1, 4);
}
#[test]
fn r7_add_p1_q12_rm1_s4() {
    check_add_row(1, 12, -1, 4);
}
#[test]
fn r7_add_p1_q12_r5_s4() {
    check_add_row(1, 12, 5, 4);
}
#[test]
fn r7_add_p1_q12_r1_s25() {
    check_add_row(1, 12, 1, 25);
}
#[test]
fn r7_add_p1_q12_rm1_s25() {
    check_add_row(1, 12, -1, 25);
}
#[test]
fn r7_add_p2_q3_r5_s2() {
    check_add_row(2, 3, 5, 2);
}
#[test]
fn r7_add_p1_q12_r5_s25() {
    check_add_row(1, 12, 5, 25);
}
#[test]
fn r7_add_p1_q12_r1_s1000() {
    check_add_row(1, 12, 1, 1000);
}
#[test]
fn r7_add_p1_q12_rm1_s1000() {
    check_add_row(1, 12, -1, 1000);
}
#[test]
fn r7_add_p1_q12_r5_s1000() {
    check_add_row(1, 12, 5, 1000);
}
#[test]
fn r7_add_p2_q12_r1_s3() {
    check_add_row(2, 12, 1, 3);
}
#[test]
fn r7_add_p2_q12_rm1_s3() {
    check_add_row(2, 12, -1, 3);
}
#[test]
fn r7_add_p2_q12_r5_s3() {
    check_add_row(2, 12, 5, 3);
}
#[test]
fn r7_add_p2_q12_r1_s7() {
    check_add_row(2, 12, 1, 7);
}
#[test]
fn r7_add_p2_q12_rm1_s7() {
    check_add_row(2, 12, -1, 7);
}
#[test]
fn r7_add_p2_q12_r5_s7() {
    check_add_row(2, 12, 5, 7);
}
#[test]
fn r7_add_p2_q3_r1_s4() {
    check_add_row(2, 3, 1, 4);
}
#[test]
fn r7_add_p2_q12_r1_s17() {
    check_add_row(2, 12, 1, 17);
}
#[test]
fn r7_add_p2_q12_rm1_s17() {
    check_add_row(2, 12, -1, 17);
}
#[test]
fn r7_add_p2_q12_r5_s17() {
    check_add_row(2, 12, 5, 17);
}
#[test]
fn r7_add_p2_q12_r1_s99() {
    check_add_row(2, 12, 1, 99);
}
#[test]
fn r7_add_p2_q12_rm1_s99() {
    check_add_row(2, 12, -1, 99);
}
#[test]
fn r7_add_p2_q12_r5_s99() {
    check_add_row(2, 12, 5, 99);
}
#[test]
fn r7_add_p2_q12_r1_s2() {
    check_add_row(2, 12, 1, 2);
}
#[test]
fn r7_add_p2_q12_rm1_s2() {
    check_add_row(2, 12, -1, 2);
}
#[test]
fn r7_add_p2_q12_r5_s2() {
    check_add_row(2, 12, 5, 2);
}
#[test]
fn r7_add_p2_q12_r1_s4() {
    check_add_row(2, 12, 1, 4);
}
#[test]
fn r7_add_p2_q3_rm1_s4() {
    check_add_row(2, 3, -1, 4);
}
#[test]
fn r7_add_p2_q12_rm1_s4() {
    check_add_row(2, 12, -1, 4);
}
#[test]
fn r7_add_p2_q12_r5_s4() {
    check_add_row(2, 12, 5, 4);
}
#[test]
fn r7_add_p2_q12_r1_s25() {
    check_add_row(2, 12, 1, 25);
}
#[test]
fn r7_add_p2_q12_rm1_s25() {
    check_add_row(2, 12, -1, 25);
}
#[test]
fn r7_add_p2_q12_r5_s25() {
    check_add_row(2, 12, 5, 25);
}
#[test]
fn r7_add_p2_q12_r1_s1000() {
    check_add_row(2, 12, 1, 1000);
}
#[test]
fn r7_add_p2_q12_rm1_s1000() {
    check_add_row(2, 12, -1, 1000);
}
#[test]
fn r7_add_p2_q12_r5_s1000() {
    check_add_row(2, 12, 5, 1000);
}
#[test]
fn r7_add_pm5_q12_r1_s3() {
    check_add_row(-5, 12, 1, 3);
}
#[test]
fn r7_add_pm5_q12_rm1_s3() {
    check_add_row(-5, 12, -1, 3);
}
#[test]
fn r7_add_p2_q3_r5_s4() {
    check_add_row(2, 3, 5, 4);
}
#[test]
fn r7_add_pm5_q12_r5_s3() {
    check_add_row(-5, 12, 5, 3);
}
#[test]
fn r7_add_pm5_q12_r1_s7() {
    check_add_row(-5, 12, 1, 7);
}
#[test]
fn r7_add_pm5_q12_rm1_s7() {
    check_add_row(-5, 12, -1, 7);
}
#[test]
fn r7_add_pm5_q12_r5_s7() {
    check_add_row(-5, 12, 5, 7);
}
#[test]
fn r7_add_pm5_q12_r1_s17() {
    check_add_row(-5, 12, 1, 17);
}
#[test]
fn r7_add_pm5_q12_rm1_s17() {
    check_add_row(-5, 12, -1, 17);
}
#[test]
fn r7_add_pm5_q12_r5_s17() {
    check_add_row(-5, 12, 5, 17);
}
#[test]
fn r7_add_pm5_q12_r1_s99() {
    check_add_row(-5, 12, 1, 99);
}
#[test]
fn r7_add_pm5_q12_rm1_s99() {
    check_add_row(-5, 12, -1, 99);
}
#[test]
fn r7_add_pm5_q12_r5_s99() {
    check_add_row(-5, 12, 5, 99);
}
#[test]
fn r7_add_p2_q3_r1_s25() {
    check_add_row(2, 3, 1, 25);
}
#[test]
fn r7_add_pm5_q12_r1_s2() {
    check_add_row(-5, 12, 1, 2);
}
#[test]
fn r7_add_pm5_q12_rm1_s2() {
    check_add_row(-5, 12, -1, 2);
}
#[test]
fn r7_add_pm5_q12_r5_s2() {
    check_add_row(-5, 12, 5, 2);
}
#[test]
fn r7_add_pm5_q12_r1_s4() {
    check_add_row(-5, 12, 1, 4);
}
#[test]
fn r7_add_pm5_q12_rm1_s4() {
    check_add_row(-5, 12, -1, 4);
}
#[test]
fn r7_add_pm5_q12_r5_s4() {
    check_add_row(-5, 12, 5, 4);
}
#[test]
fn r7_add_pm5_q12_r1_s25() {
    check_add_row(-5, 12, 1, 25);
}
#[test]
fn r7_add_pm5_q12_rm1_s25() {
    check_add_row(-5, 12, -1, 25);
}
#[test]
fn r7_add_pm5_q12_r5_s25() {
    check_add_row(-5, 12, 5, 25);
}
#[test]
fn r7_add_pm5_q12_r1_s1000() {
    check_add_row(-5, 12, 1, 1000);
}
#[test]
fn r7_add_p2_q3_rm1_s25() {
    check_add_row(2, 3, -1, 25);
}
#[test]
fn r7_add_pm5_q12_rm1_s1000() {
    check_add_row(-5, 12, -1, 1000);
}
#[test]
fn r7_add_pm5_q12_r5_s1000() {
    check_add_row(-5, 12, 5, 1000);
}
#[test]
fn r7_add_p2_q3_r5_s25() {
    check_add_row(2, 3, 5, 25);
}
#[test]
fn r7_add_p1_q13_r1_s2() {
    check_add_row(1, 13, 1, 2);
}
#[test]
fn r7_add_p1_q13_rm1_s2() {
    check_add_row(1, 13, -1, 2);
}
#[test]
fn r7_add_p1_q13_r5_s2() {
    check_add_row(1, 13, 5, 2);
}
#[test]
fn r7_add_p1_q13_r1_s4() {
    check_add_row(1, 13, 1, 4);
}
#[test]
fn r7_add_p1_q13_rm1_s4() {
    check_add_row(1, 13, -1, 4);
}
#[test]
fn r7_add_p1_q13_r5_s4() {
    check_add_row(1, 13, 5, 4);
}
#[test]
fn r7_add_p2_q3_r1_s1000() {
    check_add_row(2, 3, 1, 1000);
}
#[test]
fn r7_add_p1_q13_r1_s25() {
    check_add_row(1, 13, 1, 25);
}
#[test]
fn r7_add_p1_q13_rm1_s25() {
    check_add_row(1, 13, -1, 25);
}
#[test]
fn r7_add_p1_q13_r5_s25() {
    check_add_row(1, 13, 5, 25);
}
#[test]
fn r7_add_p1_q13_r1_s1000() {
    check_add_row(1, 13, 1, 1000);
}
#[test]
fn r7_add_p1_q13_rm1_s1000() {
    check_add_row(1, 13, -1, 1000);
}
#[test]
fn r7_add_p1_q13_r5_s1000() {
    check_add_row(1, 13, 5, 1000);
}
#[test]
fn r7_add_p2_q3_rm1_s1000() {
    check_add_row(2, 3, -1, 1000);
}
#[test]
fn r7_add_p2_q13_r1_s2() {
    check_add_row(2, 13, 1, 2);
}
#[test]
fn r7_add_p2_q13_rm1_s2() {
    check_add_row(2, 13, -1, 2);
}
#[test]
fn r7_add_p2_q3_r5_s1000() {
    check_add_row(2, 3, 5, 1000);
}
#[test]
fn r7_add_p2_q13_r5_s2() {
    check_add_row(2, 13, 5, 2);
}
#[test]
fn r7_add_p2_q13_r1_s4() {
    check_add_row(2, 13, 1, 4);
}
#[test]
fn r7_add_p2_q13_rm1_s4() {
    check_add_row(2, 13, -1, 4);
}
#[test]
fn r7_add_p2_q13_r5_s4() {
    check_add_row(2, 13, 5, 4);
}
#[test]
fn r7_add_p2_q13_r1_s25() {
    check_add_row(2, 13, 1, 25);
}
#[test]
fn r7_add_p2_q13_rm1_s25() {
    check_add_row(2, 13, -1, 25);
}
#[test]
fn r7_add_p2_q13_r5_s25() {
    check_add_row(2, 13, 5, 25);
}
#[test]
fn r7_add_p2_q13_r1_s1000() {
    check_add_row(2, 13, 1, 1000);
}
#[test]
fn r7_add_p2_q13_rm1_s1000() {
    check_add_row(2, 13, -1, 1000);
}
#[test]
fn r7_add_p2_q13_r5_s1000() {
    check_add_row(2, 13, 5, 1000);
}
#[test]
fn r7_add_pm5_q13_r1_s2() {
    check_add_row(-5, 13, 1, 2);
}
#[test]
fn r7_add_pm5_q13_rm1_s2() {
    check_add_row(-5, 13, -1, 2);
}
#[test]
fn r7_add_pm5_q13_r5_s2() {
    check_add_row(-5, 13, 5, 2);
}
#[test]
fn r7_add_pm5_q13_r1_s4() {
    check_add_row(-5, 13, 1, 4);
}
#[test]
fn r7_add_pm5_q13_rm1_s4() {
    check_add_row(-5, 13, -1, 4);
}
#[test]
fn r7_add_pm5_q13_r5_s4() {
    check_add_row(-5, 13, 5, 4);
}
#[test]
fn r7_add_pm5_q13_r1_s25() {
    check_add_row(-5, 13, 1, 25);
}
#[test]
fn r7_add_pm5_q13_rm1_s25() {
    check_add_row(-5, 13, -1, 25);
}
#[test]
fn r7_add_pm5_q13_r5_s25() {
    check_add_row(-5, 13, 5, 25);
}
#[test]
fn r7_add_pm5_q13_r1_s1000() {
    check_add_row(-5, 13, 1, 1000);
}
#[test]
fn r7_add_pm5_q13_rm1_s1000() {
    check_add_row(-5, 13, -1, 1000);
}
#[test]
fn r7_add_pm5_q13_r5_s1000() {
    check_add_row(-5, 13, 5, 1000);
}
#[test]
fn r7_add_p1_q17_r1_s2() {
    check_add_row(1, 17, 1, 2);
}
#[test]
fn r7_add_p1_q17_rm1_s2() {
    check_add_row(1, 17, -1, 2);
}
#[test]
fn r7_add_p1_q17_r5_s2() {
    check_add_row(1, 17, 5, 2);
}
#[test]
fn r7_add_p1_q17_r1_s4() {
    check_add_row(1, 17, 1, 4);
}
#[test]
fn r7_add_p1_q17_rm1_s4() {
    check_add_row(1, 17, -1, 4);
}
#[test]
fn r7_add_p1_q17_r5_s4() {
    check_add_row(1, 17, 5, 4);
}
#[test]
fn r7_add_p1_q17_r1_s25() {
    check_add_row(1, 17, 1, 25);
}
#[test]
fn r7_add_p1_q17_rm1_s25() {
    check_add_row(1, 17, -1, 25);
}
#[test]
fn r7_add_p1_q17_r5_s25() {
    check_add_row(1, 17, 5, 25);
}
#[test]
fn r7_add_p1_q17_r1_s1000() {
    check_add_row(1, 17, 1, 1000);
}
#[test]
fn r7_add_p1_q17_rm1_s1000() {
    check_add_row(1, 17, -1, 1000);
}
#[test]
fn r7_add_p1_q17_r5_s1000() {
    check_add_row(1, 17, 5, 1000);
}
#[test]
fn r7_add_p2_q17_r1_s2() {
    check_add_row(2, 17, 1, 2);
}
#[test]
fn r7_add_p2_q17_rm1_s2() {
    check_add_row(2, 17, -1, 2);
}
#[test]
fn r7_add_p2_q17_r5_s2() {
    check_add_row(2, 17, 5, 2);
}
#[test]
fn r7_add_p2_q17_r1_s4() {
    check_add_row(2, 17, 1, 4);
}
#[test]
fn r7_add_p2_q17_rm1_s4() {
    check_add_row(2, 17, -1, 4);
}
#[test]
fn r7_add_p2_q17_r5_s4() {
    check_add_row(2, 17, 5, 4);
}
#[test]
fn r7_add_p2_q17_r1_s25() {
    check_add_row(2, 17, 1, 25);
}
#[test]
fn r7_add_p2_q17_rm1_s25() {
    check_add_row(2, 17, -1, 25);
}
#[test]
fn r7_add_p2_q17_r5_s25() {
    check_add_row(2, 17, 5, 25);
}
#[test]
fn r7_add_p2_q17_r1_s1000() {
    check_add_row(2, 17, 1, 1000);
}
#[test]
fn r7_add_p2_q17_rm1_s1000() {
    check_add_row(2, 17, -1, 1000);
}
#[test]
fn r7_add_p2_q17_r5_s1000() {
    check_add_row(2, 17, 5, 1000);
}
#[test]
fn r7_add_pm5_q17_r1_s2() {
    check_add_row(-5, 17, 1, 2);
}
#[test]
fn r7_add_pm5_q17_rm1_s2() {
    check_add_row(-5, 17, -1, 2);
}
#[test]
fn r7_add_pm5_q17_r5_s2() {
    check_add_row(-5, 17, 5, 2);
}
#[test]
fn r7_add_pm5_q17_r1_s4() {
    check_add_row(-5, 17, 1, 4);
}
#[test]
fn r7_add_pm5_q17_rm1_s4() {
    check_add_row(-5, 17, -1, 4);
}
#[test]
fn r7_add_pm5_q17_r5_s4() {
    check_add_row(-5, 17, 5, 4);
}
#[test]
fn r7_add_pm5_q17_r1_s25() {
    check_add_row(-5, 17, 1, 25);
}
#[test]
fn r7_add_pm5_q17_rm1_s25() {
    check_add_row(-5, 17, -1, 25);
}
#[test]
fn r7_add_pm5_q17_r5_s25() {
    check_add_row(-5, 17, 5, 25);
}
#[test]
fn r7_add_pm5_q17_r1_s1000() {
    check_add_row(-5, 17, 1, 1000);
}
#[test]
fn r7_add_pm5_q17_rm1_s1000() {
    check_add_row(-5, 17, -1, 1000);
}
#[test]
fn r7_add_pm5_q17_r5_s1000() {
    check_add_row(-5, 17, 5, 1000);
}
#[test]
fn r7_add_p1_q19_r1_s2() {
    check_add_row(1, 19, 1, 2);
}
#[test]
fn r7_add_p1_q19_rm1_s2() {
    check_add_row(1, 19, -1, 2);
}
#[test]
fn r7_add_p1_q19_r5_s2() {
    check_add_row(1, 19, 5, 2);
}
#[test]
fn r7_add_p1_q19_r1_s4() {
    check_add_row(1, 19, 1, 4);
}
#[test]
fn r7_add_p1_q19_rm1_s4() {
    check_add_row(1, 19, -1, 4);
}
#[test]
fn r7_add_p1_q19_r5_s4() {
    check_add_row(1, 19, 5, 4);
}
#[test]
fn r7_add_p1_q19_r1_s25() {
    check_add_row(1, 19, 1, 25);
}
#[test]
fn r7_add_p1_q19_rm1_s25() {
    check_add_row(1, 19, -1, 25);
}
#[test]
fn r7_add_p1_q19_r5_s25() {
    check_add_row(1, 19, 5, 25);
}
#[test]
fn r7_add_p1_q19_r1_s1000() {
    check_add_row(1, 19, 1, 1000);
}
#[test]
fn r7_add_p1_q19_rm1_s1000() {
    check_add_row(1, 19, -1, 1000);
}
#[test]
fn r7_add_p1_q19_r5_s1000() {
    check_add_row(1, 19, 5, 1000);
}
#[test]
fn r7_add_pm5_q3_r1_s2() {
    check_add_row(-5, 3, 1, 2);
}
#[test]
fn r7_add_pm5_q3_rm1_s2() {
    check_add_row(-5, 3, -1, 2);
}
#[test]
fn r7_add_p2_q19_r1_s2() {
    check_add_row(2, 19, 1, 2);
}
#[test]
fn r7_add_p2_q19_rm1_s2() {
    check_add_row(2, 19, -1, 2);
}
#[test]
fn r7_add_p2_q19_r5_s2() {
    check_add_row(2, 19, 5, 2);
}
#[test]
fn r7_add_p2_q19_r1_s4() {
    check_add_row(2, 19, 1, 4);
}
#[test]
fn r7_add_p2_q19_rm1_s4() {
    check_add_row(2, 19, -1, 4);
}
#[test]
fn r7_add_p2_q19_r5_s4() {
    check_add_row(2, 19, 5, 4);
}
#[test]
fn r7_add_p2_q19_r1_s25() {
    check_add_row(2, 19, 1, 25);
}
#[test]
fn r7_add_p2_q19_rm1_s25() {
    check_add_row(2, 19, -1, 25);
}
#[test]
fn r7_add_pm5_q3_r5_s2() {
    check_add_row(-5, 3, 5, 2);
}
#[test]
fn r7_add_p2_q19_r5_s25() {
    check_add_row(2, 19, 5, 25);
}
#[test]
fn r7_add_p2_q19_r1_s1000() {
    check_add_row(2, 19, 1, 1000);
}
#[test]
fn r7_add_p2_q19_rm1_s1000() {
    check_add_row(2, 19, -1, 1000);
}
#[test]
fn r7_add_p2_q19_r5_s1000() {
    check_add_row(2, 19, 5, 1000);
}
#[test]
fn r7_add_pm5_q3_r1_s4() {
    check_add_row(-5, 3, 1, 4);
}
#[test]
fn r7_add_pm5_q19_r1_s2() {
    check_add_row(-5, 19, 1, 2);
}
#[test]
fn r7_add_pm5_q19_rm1_s2() {
    check_add_row(-5, 19, -1, 2);
}
#[test]
fn r7_add_pm5_q19_r5_s2() {
    check_add_row(-5, 19, 5, 2);
}
#[test]
fn r7_add_pm5_q19_r1_s4() {
    check_add_row(-5, 19, 1, 4);
}
#[test]
fn r7_add_pm5_q3_rm1_s4() {
    check_add_row(-5, 3, -1, 4);
}
#[test]
fn r7_add_pm5_q19_rm1_s4() {
    check_add_row(-5, 19, -1, 4);
}
#[test]
fn r7_add_pm5_q19_r5_s4() {
    check_add_row(-5, 19, 5, 4);
}
#[test]
fn r7_add_pm5_q19_r1_s25() {
    check_add_row(-5, 19, 1, 25);
}
#[test]
fn r7_add_pm5_q19_rm1_s25() {
    check_add_row(-5, 19, -1, 25);
}
#[test]
fn r7_add_pm5_q19_r5_s25() {
    check_add_row(-5, 19, 5, 25);
}
#[test]
fn r7_add_pm5_q19_r1_s1000() {
    check_add_row(-5, 19, 1, 1000);
}
#[test]
fn r7_add_pm5_q19_rm1_s1000() {
    check_add_row(-5, 19, -1, 1000);
}
#[test]
fn r7_add_pm5_q19_r5_s1000() {
    check_add_row(-5, 19, 5, 1000);
}
#[test]
fn r7_add_pm5_q3_r5_s4() {
    check_add_row(-5, 3, 5, 4);
}
#[test]
fn r7_add_pm5_q3_r1_s25() {
    check_add_row(-5, 3, 1, 25);
}
#[test]
fn r7_add_p1_q21_r1_s2() {
    check_add_row(1, 21, 1, 2);
}
#[test]
fn r7_add_p1_q21_rm1_s2() {
    check_add_row(1, 21, -1, 2);
}
#[test]
fn r7_add_p1_q21_r5_s2() {
    check_add_row(1, 21, 5, 2);
}
#[test]
fn r7_add_p1_q21_r1_s4() {
    check_add_row(1, 21, 1, 4);
}
#[test]
fn r7_add_p1_q21_rm1_s4() {
    check_add_row(1, 21, -1, 4);
}
#[test]
fn r7_add_p1_q21_r5_s4() {
    check_add_row(1, 21, 5, 4);
}
#[test]
fn r7_add_p1_q21_r1_s25() {
    check_add_row(1, 21, 1, 25);
}
#[test]
fn r7_add_p1_q21_rm1_s25() {
    check_add_row(1, 21, -1, 25);
}
#[test]
fn r7_add_p1_q21_r5_s25() {
    check_add_row(1, 21, 5, 25);
}
#[test]
fn r7_add_p1_q21_r1_s1000() {
    check_add_row(1, 21, 1, 1000);
}
#[test]
fn r7_add_pm5_q3_rm1_s25() {
    check_add_row(-5, 3, -1, 25);
}
#[test]
fn r7_add_p1_q21_rm1_s1000() {
    check_add_row(1, 21, -1, 1000);
}
#[test]
fn r7_add_p1_q21_r5_s1000() {
    check_add_row(1, 21, 5, 1000);
}
#[test]
fn r7_add_pm5_q3_r5_s25() {
    check_add_row(-5, 3, 5, 25);
}
#[test]
fn r7_add_p2_q21_r1_s2() {
    check_add_row(2, 21, 1, 2);
}
#[test]
fn r7_add_p2_q21_rm1_s2() {
    check_add_row(2, 21, -1, 2);
}
#[test]
fn r7_add_p2_q21_r5_s2() {
    check_add_row(2, 21, 5, 2);
}
#[test]
fn r7_add_p2_q21_r1_s4() {
    check_add_row(2, 21, 1, 4);
}
#[test]
fn r7_add_p2_q21_rm1_s4() {
    check_add_row(2, 21, -1, 4);
}
#[test]
fn r7_add_p2_q21_r5_s4() {
    check_add_row(2, 21, 5, 4);
}
#[test]
fn r7_add_pm5_q3_r1_s1000() {
    check_add_row(-5, 3, 1, 1000);
}
#[test]
fn r7_add_p2_q21_r1_s25() {
    check_add_row(2, 21, 1, 25);
}
#[test]
fn r7_add_p2_q21_rm1_s25() {
    check_add_row(2, 21, -1, 25);
}
#[test]
fn r7_add_p2_q21_r5_s25() {
    check_add_row(2, 21, 5, 25);
}
#[test]
fn r7_add_p2_q21_r1_s1000() {
    check_add_row(2, 21, 1, 1000);
}
#[test]
fn r7_add_p2_q21_rm1_s1000() {
    check_add_row(2, 21, -1, 1000);
}
#[test]
fn r7_add_p2_q21_r5_s1000() {
    check_add_row(2, 21, 5, 1000);
}
#[test]
fn r7_add_pm5_q3_rm1_s1000() {
    check_add_row(-5, 3, -1, 1000);
}
#[test]
fn r7_add_pm5_q21_r1_s2() {
    check_add_row(-5, 21, 1, 2);
}
#[test]
fn r7_add_pm5_q21_rm1_s2() {
    check_add_row(-5, 21, -1, 2);
}
#[test]
fn r7_add_pm5_q3_r5_s1000() {
    check_add_row(-5, 3, 5, 1000);
}
#[test]
fn r7_add_pm5_q21_r5_s2() {
    check_add_row(-5, 21, 5, 2);
}
#[test]
fn r7_add_pm5_q21_r1_s4() {
    check_add_row(-5, 21, 1, 4);
}
#[test]
fn r7_add_pm5_q21_rm1_s4() {
    check_add_row(-5, 21, -1, 4);
}
#[test]
fn r7_add_pm5_q21_r5_s4() {
    check_add_row(-5, 21, 5, 4);
}
#[test]
fn r7_add_pm5_q21_r1_s25() {
    check_add_row(-5, 21, 1, 25);
}
#[test]
fn r7_add_pm5_q21_rm1_s25() {
    check_add_row(-5, 21, -1, 25);
}
#[test]
fn r7_add_pm5_q21_r5_s25() {
    check_add_row(-5, 21, 5, 25);
}
#[test]
fn r7_add_pm5_q21_r1_s1000() {
    check_add_row(-5, 21, 1, 1000);
}
#[test]
fn r7_add_pm5_q21_rm1_s1000() {
    check_add_row(-5, 21, -1, 1000);
}
#[test]
fn r7_add_pm5_q21_r5_s1000() {
    check_add_row(-5, 21, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r1_s3() {
    check_add_row(1, 6, 1, 3);
}
#[test]
fn r7_add_p1_q6_rm1_s3() {
    check_add_row(1, 6, -1, 3);
}
#[test]
fn r7_add_p1_q37_r1_s2() {
    check_add_row(1, 37, 1, 2);
}
#[test]
fn r7_add_p1_q37_rm1_s2() {
    check_add_row(1, 37, -1, 2);
}
#[test]
fn r7_add_p1_q37_r5_s2() {
    check_add_row(1, 37, 5, 2);
}
#[test]
fn r7_add_p1_q37_r1_s4() {
    check_add_row(1, 37, 1, 4);
}
#[test]
fn r7_add_p1_q37_rm1_s4() {
    check_add_row(1, 37, -1, 4);
}
#[test]
fn r7_add_p1_q37_r5_s4() {
    check_add_row(1, 37, 5, 4);
}
#[test]
fn r7_add_p1_q37_r1_s25() {
    check_add_row(1, 37, 1, 25);
}
#[test]
fn r7_add_p1_q37_rm1_s25() {
    check_add_row(1, 37, -1, 25);
}
#[test]
fn r7_add_p1_q6_r5_s3() {
    check_add_row(1, 6, 5, 3);
}
#[test]
fn r7_add_p1_q37_r5_s25() {
    check_add_row(1, 37, 5, 25);
}
#[test]
fn r7_add_p1_q37_r1_s1000() {
    check_add_row(1, 37, 1, 1000);
}
#[test]
fn r7_add_p1_q37_rm1_s1000() {
    check_add_row(1, 37, -1, 1000);
}
#[test]
fn r7_add_p1_q37_r5_s1000() {
    check_add_row(1, 37, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r1_s7() {
    check_add_row(1, 6, 1, 7);
}
#[test]
fn r7_add_p2_q37_r1_s2() {
    check_add_row(2, 37, 1, 2);
}
#[test]
fn r7_add_p2_q37_rm1_s2() {
    check_add_row(2, 37, -1, 2);
}
#[test]
fn r7_add_p2_q37_r5_s2() {
    check_add_row(2, 37, 5, 2);
}
#[test]
fn r7_add_p2_q37_r1_s4() {
    check_add_row(2, 37, 1, 4);
}
#[test]
fn r7_add_p1_q6_rm1_s7() {
    check_add_row(1, 6, -1, 7);
}
#[test]
fn r7_add_p2_q37_rm1_s4() {
    check_add_row(2, 37, -1, 4);
}
#[test]
fn r7_add_p2_q37_r5_s4() {
    check_add_row(2, 37, 5, 4);
}
#[test]
fn r7_add_p2_q37_r1_s25() {
    check_add_row(2, 37, 1, 25);
}
#[test]
fn r7_add_p2_q37_rm1_s25() {
    check_add_row(2, 37, -1, 25);
}
#[test]
fn r7_add_p2_q37_r5_s25() {
    check_add_row(2, 37, 5, 25);
}
#[test]
fn r7_add_p2_q37_r1_s1000() {
    check_add_row(2, 37, 1, 1000);
}
#[test]
fn r7_add_p2_q37_rm1_s1000() {
    check_add_row(2, 37, -1, 1000);
}
#[test]
fn r7_add_p2_q37_r5_s1000() {
    check_add_row(2, 37, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r5_s7() {
    check_add_row(1, 6, 5, 7);
}
#[test]
fn r7_add_p1_q6_r1_s17() {
    check_add_row(1, 6, 1, 17);
}
#[test]
fn r7_add_pm5_q37_r1_s2() {
    check_add_row(-5, 37, 1, 2);
}
#[test]
fn r7_add_pm5_q37_rm1_s2() {
    check_add_row(-5, 37, -1, 2);
}
#[test]
fn r7_add_pm5_q37_r5_s2() {
    check_add_row(-5, 37, 5, 2);
}
#[test]
fn r7_add_pm5_q37_r1_s4() {
    check_add_row(-5, 37, 1, 4);
}
#[test]
fn r7_add_pm5_q37_rm1_s4() {
    check_add_row(-5, 37, -1, 4);
}
#[test]
fn r7_add_pm5_q37_r5_s4() {
    check_add_row(-5, 37, 5, 4);
}
#[test]
fn r7_add_pm5_q37_r1_s25() {
    check_add_row(-5, 37, 1, 25);
}
#[test]
fn r7_add_pm5_q37_rm1_s25() {
    check_add_row(-5, 37, -1, 25);
}
#[test]
fn r7_add_pm5_q37_r5_s25() {
    check_add_row(-5, 37, 5, 25);
}
#[test]
fn r7_add_pm5_q37_r1_s1000() {
    check_add_row(-5, 37, 1, 1000);
}
#[test]
fn r7_add_p1_q6_rm1_s17() {
    check_add_row(1, 6, -1, 17);
}
#[test]
fn r7_add_pm5_q37_rm1_s1000() {
    check_add_row(-5, 37, -1, 1000);
}
#[test]
fn r7_add_pm5_q37_r5_s1000() {
    check_add_row(-5, 37, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r5_s17() {
    check_add_row(1, 6, 5, 17);
}
#[test]
fn r7_add_p1_q99_r1_s2() {
    check_add_row(1, 99, 1, 2);
}
#[test]
fn r7_add_p1_q99_rm1_s2() {
    check_add_row(1, 99, -1, 2);
}
#[test]
fn r7_add_p1_q99_r5_s2() {
    check_add_row(1, 99, 5, 2);
}
#[test]
fn r7_add_p1_q99_r1_s4() {
    check_add_row(1, 99, 1, 4);
}
#[test]
fn r7_add_p1_q99_rm1_s4() {
    check_add_row(1, 99, -1, 4);
}
#[test]
fn r7_add_p1_q99_r5_s4() {
    check_add_row(1, 99, 5, 4);
}
#[test]
fn r7_add_p1_q6_r1_s99() {
    check_add_row(1, 6, 1, 99);
}
#[test]
fn r7_add_p1_q99_r1_s25() {
    check_add_row(1, 99, 1, 25);
}
#[test]
fn r7_add_p1_q99_rm1_s25() {
    check_add_row(1, 99, -1, 25);
}
#[test]
fn r7_add_p1_q99_r5_s25() {
    check_add_row(1, 99, 5, 25);
}
#[test]
fn r7_add_p1_q99_r1_s1000() {
    check_add_row(1, 99, 1, 1000);
}
#[test]
fn r7_add_p1_q99_rm1_s1000() {
    check_add_row(1, 99, -1, 1000);
}
#[test]
fn r7_add_p1_q99_r5_s1000() {
    check_add_row(1, 99, 5, 1000);
}
#[test]
fn r7_add_p1_q6_rm1_s99() {
    check_add_row(1, 6, -1, 99);
}
#[test]
fn r7_add_p2_q99_r1_s2() {
    check_add_row(2, 99, 1, 2);
}
#[test]
fn r7_add_p2_q99_rm1_s2() {
    check_add_row(2, 99, -1, 2);
}
#[test]
fn r7_add_p1_q6_r5_s99() {
    check_add_row(1, 6, 5, 99);
}
#[test]
fn r7_add_p2_q99_r5_s2() {
    check_add_row(2, 99, 5, 2);
}
#[test]
fn r7_add_p2_q99_r1_s4() {
    check_add_row(2, 99, 1, 4);
}
#[test]
fn r7_add_p2_q99_rm1_s4() {
    check_add_row(2, 99, -1, 4);
}
#[test]
fn r7_add_p2_q99_r5_s4() {
    check_add_row(2, 99, 5, 4);
}
#[test]
fn r7_add_p2_q99_r1_s25() {
    check_add_row(2, 99, 1, 25);
}
#[test]
fn r7_add_p2_q99_rm1_s25() {
    check_add_row(2, 99, -1, 25);
}
#[test]
fn r7_add_p2_q99_r5_s25() {
    check_add_row(2, 99, 5, 25);
}
#[test]
fn r7_add_p2_q99_r1_s1000() {
    check_add_row(2, 99, 1, 1000);
}
#[test]
fn r7_add_p2_q99_rm1_s1000() {
    check_add_row(2, 99, -1, 1000);
}
#[test]
fn r7_add_p2_q99_r5_s1000() {
    check_add_row(2, 99, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r1_s2() {
    check_add_row(1, 6, 1, 2);
}
#[test]
fn r7_add_p1_q6_rm1_s2() {
    check_add_row(1, 6, -1, 2);
}
#[test]
fn r7_add_pm5_q99_r1_s2() {
    check_add_row(-5, 99, 1, 2);
}
#[test]
fn r7_add_pm5_q99_rm1_s2() {
    check_add_row(-5, 99, -1, 2);
}
#[test]
fn r7_add_pm5_q99_r5_s2() {
    check_add_row(-5, 99, 5, 2);
}
#[test]
fn r7_add_pm5_q99_r1_s4() {
    check_add_row(-5, 99, 1, 4);
}
#[test]
fn r7_add_pm5_q99_rm1_s4() {
    check_add_row(-5, 99, -1, 4);
}
#[test]
fn r7_add_pm5_q99_r5_s4() {
    check_add_row(-5, 99, 5, 4);
}
#[test]
fn r7_add_pm5_q99_r1_s25() {
    check_add_row(-5, 99, 1, 25);
}
#[test]
fn r7_add_pm5_q99_rm1_s25() {
    check_add_row(-5, 99, -1, 25);
}
#[test]
fn r7_add_p1_q6_r5_s2() {
    check_add_row(1, 6, 5, 2);
}
#[test]
fn r7_add_pm5_q99_r5_s25() {
    check_add_row(-5, 99, 5, 25);
}
#[test]
fn r7_add_pm5_q99_r1_s1000() {
    check_add_row(-5, 99, 1, 1000);
}
#[test]
fn r7_add_pm5_q99_rm1_s1000() {
    check_add_row(-5, 99, -1, 1000);
}
#[test]
fn r7_add_pm5_q99_r5_s1000() {
    check_add_row(-5, 99, 5, 1000);
}
#[test]
fn r7_add_p1_q6_r1_s4() {
    check_add_row(1, 6, 1, 4);
}
#[test]
fn r7_add_p1_q6_rm1_s4() {
    check_add_row(1, 6, -1, 4);
}
#[test]
fn r7_add_p1_q6_r5_s4() {
    check_add_row(1, 6, 5, 4);
}
#[test]
fn r7_add_p1_q6_r1_s25() {
    check_add_row(1, 6, 1, 25);
}
#[test]
fn r7_add_p1_q6_rm1_s25() {
    check_add_row(1, 6, -1, 25);
}
#[test]
fn r7_add_p1_q6_r5_s25() {
    check_add_row(1, 6, 5, 25);
}
#[test]
fn r7_add_p1_q6_r1_s1000() {
    check_add_row(1, 6, 1, 1000);
}
#[test]
fn r7_add_p1_q6_rm1_s1000() {
    check_add_row(1, 6, -1, 1000);
}
#[test]
fn r7_add_p1_q6_r5_s1000() {
    check_add_row(1, 6, 5, 1000);
}
#[test]
fn r7_sub_p2_q6_r1_s2() {
    check_sub_row(2, 6, 1, 2);
}
#[test]
fn r7_sub_p2_q6_rm1_s2() {
    check_sub_row(2, 6, -1, 2);
}
#[test]
fn r7_sub_p2_q6_r5_s2() {
    check_sub_row(2, 6, 5, 2);
}
#[test]
fn r7_sub_p2_q6_r1_s4() {
    check_sub_row(2, 6, 1, 4);
}
#[test]
fn r7_sub_p2_q6_rm1_s4() {
    check_sub_row(2, 6, -1, 4);
}
#[test]
fn r7_sub_p2_q6_r5_s4() {
    check_sub_row(2, 6, 5, 4);
}
#[test]
fn r7_sub_p2_q6_r1_s25() {
    check_sub_row(2, 6, 1, 25);
}
#[test]
fn r7_sub_p2_q6_rm1_s25() {
    check_sub_row(2, 6, -1, 25);
}
#[test]
fn r7_sub_p2_q6_r5_s25() {
    check_sub_row(2, 6, 5, 25);
}
#[test]
fn r7_sub_p2_q6_r1_s1000() {
    check_sub_row(2, 6, 1, 1000);
}
#[test]
fn r7_sub_p2_q6_rm1_s1000() {
    check_sub_row(2, 6, -1, 1000);
}
#[test]
fn r7_sub_p2_q6_r5_s1000() {
    check_sub_row(2, 6, 5, 1000);
}
#[test]
fn r7_sub_p1_q3_r1_s2() {
    check_sub_row(1, 3, 1, 2);
}
#[test]
fn r7_sub_pm5_q6_r1_s3() {
    check_sub_row(-5, 6, 1, 3);
}
#[test]
fn r7_sub_pm5_q6_rm1_s3() {
    check_sub_row(-5, 6, -1, 3);
}
#[test]
fn r7_sub_pm5_q6_r5_s3() {
    check_sub_row(-5, 6, 5, 3);
}
#[test]
fn r7_sub_pm5_q6_r1_s7() {
    check_sub_row(-5, 6, 1, 7);
}
#[test]
fn r7_sub_pm5_q6_rm1_s7() {
    check_sub_row(-5, 6, -1, 7);
}
#[test]
fn r7_sub_pm5_q6_r5_s7() {
    check_sub_row(-5, 6, 5, 7);
}
#[test]
fn r7_sub_pm5_q6_r1_s17() {
    check_sub_row(-5, 6, 1, 17);
}
#[test]
fn r7_sub_pm5_q6_rm1_s17() {
    check_sub_row(-5, 6, -1, 17);
}
#[test]
fn r7_sub_pm5_q6_r5_s17() {
    check_sub_row(-5, 6, 5, 17);
}
#[test]
fn r7_sub_pm5_q6_r1_s99() {
    check_sub_row(-5, 6, 1, 99);
}
#[test]
fn r7_sub_p1_q3_rm1_s2() {
    check_sub_row(1, 3, -1, 2);
}
#[test]
fn r7_sub_pm5_q6_rm1_s99() {
    check_sub_row(-5, 6, -1, 99);
}
#[test]
fn r7_sub_pm5_q6_r5_s99() {
    check_sub_row(-5, 6, 5, 99);
}
#[test]
fn r7_sub_pm5_q6_r1_s2() {
    check_sub_row(-5, 6, 1, 2);
}
#[test]
fn r7_sub_pm5_q6_rm1_s2() {
    check_sub_row(-5, 6, -1, 2);
}
#[test]
fn r7_sub_pm5_q6_r5_s2() {
    check_sub_row(-5, 6, 5, 2);
}
#[test]
fn r7_sub_pm5_q6_r1_s4() {
    check_sub_row(-5, 6, 1, 4);
}
#[test]
fn r7_sub_pm5_q6_rm1_s4() {
    check_sub_row(-5, 6, -1, 4);
}
#[test]
fn r7_sub_pm5_q6_r5_s4() {
    check_sub_row(-5, 6, 5, 4);
}
#[test]
fn r7_sub_pm5_q6_r1_s25() {
    check_sub_row(-5, 6, 1, 25);
}
#[test]
fn r7_sub_pm5_q6_rm1_s25() {
    check_sub_row(-5, 6, -1, 25);
}
#[test]
fn r7_sub_p1_q3_r5_s2() {
    check_sub_row(1, 3, 5, 2);
}
#[test]
fn r7_sub_pm5_q6_r5_s25() {
    check_sub_row(-5, 6, 5, 25);
}
#[test]
fn r7_sub_pm5_q6_r1_s1000() {
    check_sub_row(-5, 6, 1, 1000);
}
#[test]
fn r7_sub_pm5_q6_rm1_s1000() {
    check_sub_row(-5, 6, -1, 1000);
}
#[test]
fn r7_sub_pm5_q6_r5_s1000() {
    check_sub_row(-5, 6, 5, 1000);
}
#[test]
fn r7_sub_p1_q3_r1_s4() {
    check_sub_row(1, 3, 1, 4);
}
#[test]
fn r7_sub_p1_q7_r1_s2() {
    check_sub_row(1, 7, 1, 2);
}
#[test]
fn r7_sub_p1_q7_rm1_s2() {
    check_sub_row(1, 7, -1, 2);
}
#[test]
fn r7_sub_p1_q7_r5_s2() {
    check_sub_row(1, 7, 5, 2);
}
#[test]
fn r7_sub_p1_q7_r1_s4() {
    check_sub_row(1, 7, 1, 4);
}
#[test]
fn r7_sub_p1_q3_rm1_s4() {
    check_sub_row(1, 3, -1, 4);
}
#[test]
fn r7_sub_p1_q7_rm1_s4() {
    check_sub_row(1, 7, -1, 4);
}
#[test]
fn r7_sub_p1_q7_r5_s4() {
    check_sub_row(1, 7, 5, 4);
}
#[test]
fn r7_sub_p1_q7_r1_s25() {
    check_sub_row(1, 7, 1, 25);
}
#[test]
fn r7_sub_p1_q7_rm1_s25() {
    check_sub_row(1, 7, -1, 25);
}
#[test]
fn r7_sub_p1_q7_r5_s25() {
    check_sub_row(1, 7, 5, 25);
}
#[test]
fn r7_sub_p1_q7_r1_s1000() {
    check_sub_row(1, 7, 1, 1000);
}
#[test]
fn r7_sub_p1_q7_rm1_s1000() {
    check_sub_row(1, 7, -1, 1000);
}
#[test]
fn r7_sub_p1_q7_r5_s1000() {
    check_sub_row(1, 7, 5, 1000);
}
#[test]
fn r7_sub_p1_q3_r5_s4() {
    check_sub_row(1, 3, 5, 4);
}
#[test]
fn r7_sub_p1_q3_r1_s25() {
    check_sub_row(1, 3, 1, 25);
}
#[test]
fn r7_sub_p2_q7_r1_s2() {
    check_sub_row(2, 7, 1, 2);
}
#[test]
fn r7_sub_p2_q7_rm1_s2() {
    check_sub_row(2, 7, -1, 2);
}
#[test]
fn r7_sub_p2_q7_r5_s2() {
    check_sub_row(2, 7, 5, 2);
}
#[test]
fn r7_sub_p2_q7_r1_s4() {
    check_sub_row(2, 7, 1, 4);
}
#[test]
fn r7_sub_p2_q7_rm1_s4() {
    check_sub_row(2, 7, -1, 4);
}
#[test]
fn r7_sub_p2_q7_r5_s4() {
    check_sub_row(2, 7, 5, 4);
}
#[test]
fn r7_sub_p2_q7_r1_s25() {
    check_sub_row(2, 7, 1, 25);
}
#[test]
fn r7_sub_p2_q7_rm1_s25() {
    check_sub_row(2, 7, -1, 25);
}
#[test]
fn r7_sub_p2_q7_r5_s25() {
    check_sub_row(2, 7, 5, 25);
}
#[test]
fn r7_sub_p2_q7_r1_s1000() {
    check_sub_row(2, 7, 1, 1000);
}
#[test]
fn r7_sub_p1_q3_rm1_s25() {
    check_sub_row(1, 3, -1, 25);
}
#[test]
fn r7_sub_p2_q7_rm1_s1000() {
    check_sub_row(2, 7, -1, 1000);
}
#[test]
fn r7_sub_p2_q7_r5_s1000() {
    check_sub_row(2, 7, 5, 1000);
}
#[test]
fn r7_sub_p1_q3_r5_s25() {
    check_sub_row(1, 3, 5, 25);
}
#[test]
fn r7_sub_pm5_q7_r1_s2() {
    check_sub_row(-5, 7, 1, 2);
}
#[test]
fn r7_sub_pm5_q7_rm1_s2() {
    check_sub_row(-5, 7, -1, 2);
}
#[test]
fn r7_sub_pm5_q7_r5_s2() {
    check_sub_row(-5, 7, 5, 2);
}
#[test]
fn r7_sub_pm5_q7_r1_s4() {
    check_sub_row(-5, 7, 1, 4);
}
#[test]
fn r7_sub_pm5_q7_rm1_s4() {
    check_sub_row(-5, 7, -1, 4);
}
#[test]
fn r7_sub_pm5_q7_r5_s4() {
    check_sub_row(-5, 7, 5, 4);
}
#[test]
fn r7_sub_p1_q3_r1_s1000() {
    check_sub_row(1, 3, 1, 1000);
}
#[test]
fn r7_sub_pm5_q7_r1_s25() {
    check_sub_row(-5, 7, 1, 25);
}
#[test]
fn r7_sub_pm5_q7_rm1_s25() {
    check_sub_row(-5, 7, -1, 25);
}
#[test]
fn r7_sub_pm5_q7_r5_s25() {
    check_sub_row(-5, 7, 5, 25);
}
#[test]
fn r7_sub_pm5_q7_r1_s1000() {
    check_sub_row(-5, 7, 1, 1000);
}
#[test]
fn r7_sub_pm5_q7_rm1_s1000() {
    check_sub_row(-5, 7, -1, 1000);
}
#[test]
fn r7_sub_pm5_q7_r5_s1000() {
    check_sub_row(-5, 7, 5, 1000);
}
#[test]
fn r7_sub_p1_q3_rm1_s1000() {
    check_sub_row(1, 3, -1, 1000);
}
#[test]
fn r7_sub_p1_q9_r1_s2() {
    check_sub_row(1, 9, 1, 2);
}
#[test]
fn r7_sub_p1_q9_rm1_s2() {
    check_sub_row(1, 9, -1, 2);
}
#[test]
fn r7_sub_p1_q3_r5_s1000() {
    check_sub_row(1, 3, 5, 1000);
}
#[test]
fn r7_sub_p1_q9_r5_s2() {
    check_sub_row(1, 9, 5, 2);
}
#[test]
fn r7_sub_p1_q9_r1_s4() {
    check_sub_row(1, 9, 1, 4);
}
#[test]
fn r7_sub_p1_q9_rm1_s4() {
    check_sub_row(1, 9, -1, 4);
}
#[test]
fn r7_sub_p1_q9_r5_s4() {
    check_sub_row(1, 9, 5, 4);
}
#[test]
fn r7_sub_p1_q9_r1_s25() {
    check_sub_row(1, 9, 1, 25);
}
#[test]
fn r7_sub_p1_q9_rm1_s25() {
    check_sub_row(1, 9, -1, 25);
}
#[test]
fn r7_sub_p1_q9_r5_s25() {
    check_sub_row(1, 9, 5, 25);
}
#[test]
fn r7_sub_p1_q9_r1_s1000() {
    check_sub_row(1, 9, 1, 1000);
}
#[test]
fn r7_sub_p1_q9_rm1_s1000() {
    check_sub_row(1, 9, -1, 1000);
}
#[test]
fn r7_sub_p1_q9_r5_s1000() {
    check_sub_row(1, 9, 5, 1000);
}
#[test]
fn r7_sub_p2_q9_r1_s2() {
    check_sub_row(2, 9, 1, 2);
}
#[test]
fn r7_sub_p2_q9_rm1_s2() {
    check_sub_row(2, 9, -1, 2);
}
#[test]
fn r7_sub_p2_q9_r5_s2() {
    check_sub_row(2, 9, 5, 2);
}
#[test]
fn r7_sub_p2_q9_r1_s4() {
    check_sub_row(2, 9, 1, 4);
}
#[test]
fn r7_sub_p2_q9_rm1_s4() {
    check_sub_row(2, 9, -1, 4);
}
#[test]
fn r7_sub_p2_q9_r5_s4() {
    check_sub_row(2, 9, 5, 4);
}
#[test]
fn r7_sub_p2_q9_r1_s25() {
    check_sub_row(2, 9, 1, 25);
}
#[test]
fn r7_sub_p2_q9_rm1_s25() {
    check_sub_row(2, 9, -1, 25);
}
#[test]
fn r7_sub_p2_q9_r5_s25() {
    check_sub_row(2, 9, 5, 25);
}
#[test]
fn r7_sub_p2_q9_r1_s1000() {
    check_sub_row(2, 9, 1, 1000);
}
#[test]
fn r7_sub_p2_q9_rm1_s1000() {
    check_sub_row(2, 9, -1, 1000);
}
#[test]
fn r7_sub_p2_q9_r5_s1000() {
    check_sub_row(2, 9, 5, 1000);
}
#[test]
fn r7_sub_pm5_q9_r1_s2() {
    check_sub_row(-5, 9, 1, 2);
}
#[test]
fn r7_sub_pm5_q9_rm1_s2() {
    check_sub_row(-5, 9, -1, 2);
}
#[test]
fn r7_sub_pm5_q9_r5_s2() {
    check_sub_row(-5, 9, 5, 2);
}
#[test]
fn r7_sub_pm5_q9_r1_s4() {
    check_sub_row(-5, 9, 1, 4);
}
#[test]
fn r7_sub_pm5_q9_rm1_s4() {
    check_sub_row(-5, 9, -1, 4);
}
#[test]
fn r7_sub_pm5_q9_r5_s4() {
    check_sub_row(-5, 9, 5, 4);
}
#[test]
fn r7_sub_pm5_q9_r1_s25() {
    check_sub_row(-5, 9, 1, 25);
}
#[test]
fn r7_sub_pm5_q9_rm1_s25() {
    check_sub_row(-5, 9, -1, 25);
}
#[test]
fn r7_sub_pm5_q9_r5_s25() {
    check_sub_row(-5, 9, 5, 25);
}
#[test]
fn r7_sub_pm5_q9_r1_s1000() {
    check_sub_row(-5, 9, 1, 1000);
}
#[test]
fn r7_sub_pm5_q9_rm1_s1000() {
    check_sub_row(-5, 9, -1, 1000);
}
#[test]
fn r7_sub_pm5_q9_r5_s1000() {
    check_sub_row(-5, 9, 5, 1000);
}
#[test]
fn r7_sub_p1_q11_r1_s2() {
    check_sub_row(1, 11, 1, 2);
}
#[test]
fn r7_sub_p1_q11_rm1_s2() {
    check_sub_row(1, 11, -1, 2);
}
#[test]
fn r7_sub_p1_q11_r5_s2() {
    check_sub_row(1, 11, 5, 2);
}
#[test]
fn r7_sub_p1_q11_r1_s4() {
    check_sub_row(1, 11, 1, 4);
}
#[test]
fn r7_sub_p1_q11_rm1_s4() {
    check_sub_row(1, 11, -1, 4);
}
#[test]
fn r7_sub_p1_q11_r5_s4() {
    check_sub_row(1, 11, 5, 4);
}
#[test]
fn r7_sub_p1_q11_r1_s25() {
    check_sub_row(1, 11, 1, 25);
}
#[test]
fn r7_sub_p1_q11_rm1_s25() {
    check_sub_row(1, 11, -1, 25);
}
#[test]
fn r7_sub_p1_q11_r5_s25() {
    check_sub_row(1, 11, 5, 25);
}
#[test]
fn r7_sub_p1_q11_r1_s1000() {
    check_sub_row(1, 11, 1, 1000);
}
#[test]
fn r7_sub_p1_q11_rm1_s1000() {
    check_sub_row(1, 11, -1, 1000);
}
#[test]
fn r7_sub_p1_q11_r5_s1000() {
    check_sub_row(1, 11, 5, 1000);
}
#[test]
fn r7_sub_p2_q11_r1_s2() {
    check_sub_row(2, 11, 1, 2);
}
#[test]
fn r7_sub_p2_q11_rm1_s2() {
    check_sub_row(2, 11, -1, 2);
}
#[test]
fn r7_sub_p2_q11_r5_s2() {
    check_sub_row(2, 11, 5, 2);
}
#[test]
fn r7_sub_p2_q11_r1_s4() {
    check_sub_row(2, 11, 1, 4);
}
#[test]
fn r7_sub_p2_q11_rm1_s4() {
    check_sub_row(2, 11, -1, 4);
}
#[test]
fn r7_sub_p2_q11_r5_s4() {
    check_sub_row(2, 11, 5, 4);
}
#[test]
fn r7_sub_p2_q11_r1_s25() {
    check_sub_row(2, 11, 1, 25);
}
#[test]
fn r7_sub_p2_q11_rm1_s25() {
    check_sub_row(2, 11, -1, 25);
}
#[test]
fn r7_sub_p2_q11_r5_s25() {
    check_sub_row(2, 11, 5, 25);
}
#[test]
fn r7_sub_p2_q11_r1_s1000() {
    check_sub_row(2, 11, 1, 1000);
}
#[test]
fn r7_sub_p2_q11_rm1_s1000() {
    check_sub_row(2, 11, -1, 1000);
}
#[test]
fn r7_sub_p2_q11_r5_s1000() {
    check_sub_row(2, 11, 5, 1000);
}
#[test]
fn r7_sub_pm5_q11_r1_s2() {
    check_sub_row(-5, 11, 1, 2);
}
#[test]
fn r7_sub_pm5_q11_rm1_s2() {
    check_sub_row(-5, 11, -1, 2);
}
#[test]
fn r7_sub_pm5_q11_r5_s2() {
    check_sub_row(-5, 11, 5, 2);
}
#[test]
fn r7_sub_pm5_q11_r1_s4() {
    check_sub_row(-5, 11, 1, 4);
}
#[test]
fn r7_sub_pm5_q11_rm1_s4() {
    check_sub_row(-5, 11, -1, 4);
}
#[test]
fn r7_sub_pm5_q11_r5_s4() {
    check_sub_row(-5, 11, 5, 4);
}
#[test]
fn r7_sub_pm5_q11_r1_s25() {
    check_sub_row(-5, 11, 1, 25);
}
#[test]
fn r7_sub_pm5_q11_rm1_s25() {
    check_sub_row(-5, 11, -1, 25);
}
#[test]
fn r7_sub_pm5_q11_r5_s25() {
    check_sub_row(-5, 11, 5, 25);
}
#[test]
fn r7_sub_pm5_q11_r1_s1000() {
    check_sub_row(-5, 11, 1, 1000);
}
#[test]
fn r7_sub_pm5_q11_rm1_s1000() {
    check_sub_row(-5, 11, -1, 1000);
}
#[test]
fn r7_sub_pm5_q11_r5_s1000() {
    check_sub_row(-5, 11, 5, 1000);
}
#[test]
fn r7_sub_p2_q3_r1_s2() {
    check_sub_row(2, 3, 1, 2);
}
#[test]
fn r7_sub_p1_q12_r1_s3() {
    check_sub_row(1, 12, 1, 3);
}
#[test]
fn r7_sub_p1_q12_rm1_s3() {
    check_sub_row(1, 12, -1, 3);
}
#[test]
fn r7_sub_p1_q12_r5_s3() {
    check_sub_row(1, 12, 5, 3);
}
#[test]
fn r7_sub_p1_q12_r1_s7() {
    check_sub_row(1, 12, 1, 7);
}
#[test]
fn r7_sub_p1_q12_rm1_s7() {
    check_sub_row(1, 12, -1, 7);
}
#[test]
fn r7_sub_p1_q12_r5_s7() {
    check_sub_row(1, 12, 5, 7);
}
#[test]
fn r7_sub_p1_q12_r1_s17() {
    check_sub_row(1, 12, 1, 17);
}
#[test]
fn r7_sub_p1_q12_rm1_s17() {
    check_sub_row(1, 12, -1, 17);
}
#[test]
fn r7_sub_p1_q12_r5_s17() {
    check_sub_row(1, 12, 5, 17);
}
#[test]
fn r7_sub_p1_q12_r1_s99() {
    check_sub_row(1, 12, 1, 99);
}
#[test]
fn r7_sub_p2_q3_rm1_s2() {
    check_sub_row(2, 3, -1, 2);
}
#[test]
fn r7_sub_p1_q12_rm1_s99() {
    check_sub_row(1, 12, -1, 99);
}
#[test]
fn r7_sub_p1_q12_r5_s99() {
    check_sub_row(1, 12, 5, 99);
}
#[test]
fn r7_sub_p1_q12_r1_s2() {
    check_sub_row(1, 12, 1, 2);
}
#[test]
fn r7_sub_p1_q12_rm1_s2() {
    check_sub_row(1, 12, -1, 2);
}
#[test]
fn r7_sub_p1_q12_r5_s2() {
    check_sub_row(1, 12, 5, 2);
}
#[test]
fn r7_sub_p1_q12_r1_s4() {
    check_sub_row(1, 12, 1, 4);
}
#[test]
fn r7_sub_p1_q12_rm1_s4() {
    check_sub_row(1, 12, -1, 4);
}
#[test]
fn r7_sub_p1_q12_r5_s4() {
    check_sub_row(1, 12, 5, 4);
}
#[test]
fn r7_sub_p1_q12_r1_s25() {
    check_sub_row(1, 12, 1, 25);
}
#[test]
fn r7_sub_p1_q12_rm1_s25() {
    check_sub_row(1, 12, -1, 25);
}
#[test]
fn r7_sub_p2_q3_r5_s2() {
    check_sub_row(2, 3, 5, 2);
}
#[test]
fn r7_sub_p1_q12_r5_s25() {
    check_sub_row(1, 12, 5, 25);
}
#[test]
fn r7_sub_p1_q12_r1_s1000() {
    check_sub_row(1, 12, 1, 1000);
}
#[test]
fn r7_sub_p1_q12_rm1_s1000() {
    check_sub_row(1, 12, -1, 1000);
}
#[test]
fn r7_sub_p1_q12_r5_s1000() {
    check_sub_row(1, 12, 5, 1000);
}
#[test]
fn r7_sub_p2_q12_r1_s3() {
    check_sub_row(2, 12, 1, 3);
}
#[test]
fn r7_sub_p2_q12_rm1_s3() {
    check_sub_row(2, 12, -1, 3);
}
#[test]
fn r7_sub_p2_q12_r5_s3() {
    check_sub_row(2, 12, 5, 3);
}
#[test]
fn r7_sub_p2_q12_r1_s7() {
    check_sub_row(2, 12, 1, 7);
}
#[test]
fn r7_sub_p2_q12_rm1_s7() {
    check_sub_row(2, 12, -1, 7);
}
#[test]
fn r7_sub_p2_q12_r5_s7() {
    check_sub_row(2, 12, 5, 7);
}
#[test]
fn r7_sub_p2_q3_r1_s4() {
    check_sub_row(2, 3, 1, 4);
}
#[test]
fn r7_sub_p2_q12_r1_s17() {
    check_sub_row(2, 12, 1, 17);
}
#[test]
fn r7_sub_p2_q12_rm1_s17() {
    check_sub_row(2, 12, -1, 17);
}
#[test]
fn r7_sub_p2_q12_r5_s17() {
    check_sub_row(2, 12, 5, 17);
}
#[test]
fn r7_sub_p2_q12_r1_s99() {
    check_sub_row(2, 12, 1, 99);
}
#[test]
fn r7_sub_p2_q12_rm1_s99() {
    check_sub_row(2, 12, -1, 99);
}
#[test]
fn r7_sub_p2_q12_r5_s99() {
    check_sub_row(2, 12, 5, 99);
}
#[test]
fn r7_sub_p2_q12_r1_s2() {
    check_sub_row(2, 12, 1, 2);
}
#[test]
fn r7_sub_p2_q12_rm1_s2() {
    check_sub_row(2, 12, -1, 2);
}
#[test]
fn r7_sub_p2_q12_r5_s2() {
    check_sub_row(2, 12, 5, 2);
}
#[test]
fn r7_sub_p2_q12_r1_s4() {
    check_sub_row(2, 12, 1, 4);
}
#[test]
fn r7_sub_p2_q3_rm1_s4() {
    check_sub_row(2, 3, -1, 4);
}
#[test]
fn r7_sub_p2_q12_rm1_s4() {
    check_sub_row(2, 12, -1, 4);
}
#[test]
fn r7_sub_p2_q12_r5_s4() {
    check_sub_row(2, 12, 5, 4);
}
#[test]
fn r7_sub_p2_q12_r1_s25() {
    check_sub_row(2, 12, 1, 25);
}
#[test]
fn r7_sub_p2_q12_rm1_s25() {
    check_sub_row(2, 12, -1, 25);
}
#[test]
fn r7_sub_p2_q12_r5_s25() {
    check_sub_row(2, 12, 5, 25);
}
#[test]
fn r7_sub_p2_q12_r1_s1000() {
    check_sub_row(2, 12, 1, 1000);
}
#[test]
fn r7_sub_p2_q12_rm1_s1000() {
    check_sub_row(2, 12, -1, 1000);
}
#[test]
fn r7_sub_p2_q12_r5_s1000() {
    check_sub_row(2, 12, 5, 1000);
}
#[test]
fn r7_sub_pm5_q12_r1_s3() {
    check_sub_row(-5, 12, 1, 3);
}
#[test]
fn r7_sub_pm5_q12_rm1_s3() {
    check_sub_row(-5, 12, -1, 3);
}
#[test]
fn r7_sub_p2_q3_r5_s4() {
    check_sub_row(2, 3, 5, 4);
}
#[test]
fn r7_sub_pm5_q12_r5_s3() {
    check_sub_row(-5, 12, 5, 3);
}
#[test]
fn r7_sub_pm5_q12_r1_s7() {
    check_sub_row(-5, 12, 1, 7);
}
#[test]
fn r7_sub_pm5_q12_rm1_s7() {
    check_sub_row(-5, 12, -1, 7);
}
#[test]
fn r7_sub_pm5_q12_r5_s7() {
    check_sub_row(-5, 12, 5, 7);
}
#[test]
fn r7_sub_pm5_q12_r1_s17() {
    check_sub_row(-5, 12, 1, 17);
}
#[test]
fn r7_sub_pm5_q12_rm1_s17() {
    check_sub_row(-5, 12, -1, 17);
}
#[test]
fn r7_sub_pm5_q12_r5_s17() {
    check_sub_row(-5, 12, 5, 17);
}
#[test]
fn r7_sub_pm5_q12_r1_s99() {
    check_sub_row(-5, 12, 1, 99);
}
#[test]
fn r7_sub_pm5_q12_rm1_s99() {
    check_sub_row(-5, 12, -1, 99);
}
#[test]
fn r7_sub_pm5_q12_r5_s99() {
    check_sub_row(-5, 12, 5, 99);
}
#[test]
fn r7_sub_p2_q3_r1_s25() {
    check_sub_row(2, 3, 1, 25);
}
#[test]
fn r7_sub_pm5_q12_r1_s2() {
    check_sub_row(-5, 12, 1, 2);
}
#[test]
fn r7_sub_pm5_q12_rm1_s2() {
    check_sub_row(-5, 12, -1, 2);
}
#[test]
fn r7_sub_pm5_q12_r5_s2() {
    check_sub_row(-5, 12, 5, 2);
}
#[test]
fn r7_sub_pm5_q12_r1_s4() {
    check_sub_row(-5, 12, 1, 4);
}
#[test]
fn r7_sub_pm5_q12_rm1_s4() {
    check_sub_row(-5, 12, -1, 4);
}
#[test]
fn r7_sub_pm5_q12_r5_s4() {
    check_sub_row(-5, 12, 5, 4);
}
#[test]
fn r7_sub_pm5_q12_r1_s25() {
    check_sub_row(-5, 12, 1, 25);
}
#[test]
fn r7_sub_pm5_q12_rm1_s25() {
    check_sub_row(-5, 12, -1, 25);
}
#[test]
fn r7_sub_pm5_q12_r5_s25() {
    check_sub_row(-5, 12, 5, 25);
}
#[test]
fn r7_sub_pm5_q12_r1_s1000() {
    check_sub_row(-5, 12, 1, 1000);
}
#[test]
fn r7_sub_p2_q3_rm1_s25() {
    check_sub_row(2, 3, -1, 25);
}
#[test]
fn r7_sub_pm5_q12_rm1_s1000() {
    check_sub_row(-5, 12, -1, 1000);
}
#[test]
fn r7_sub_pm5_q12_r5_s1000() {
    check_sub_row(-5, 12, 5, 1000);
}
#[test]
fn r7_sub_p2_q3_r5_s25() {
    check_sub_row(2, 3, 5, 25);
}
#[test]
fn r7_sub_p1_q13_r1_s2() {
    check_sub_row(1, 13, 1, 2);
}
#[test]
fn r7_sub_p1_q13_rm1_s2() {
    check_sub_row(1, 13, -1, 2);
}
#[test]
fn r7_sub_p1_q13_r5_s2() {
    check_sub_row(1, 13, 5, 2);
}
#[test]
fn r7_sub_p1_q13_r1_s4() {
    check_sub_row(1, 13, 1, 4);
}
#[test]
fn r7_sub_p1_q13_rm1_s4() {
    check_sub_row(1, 13, -1, 4);
}
#[test]
fn r7_sub_p1_q13_r5_s4() {
    check_sub_row(1, 13, 5, 4);
}
#[test]
fn r7_sub_p2_q3_r1_s1000() {
    check_sub_row(2, 3, 1, 1000);
}
#[test]
fn r7_sub_p1_q13_r1_s25() {
    check_sub_row(1, 13, 1, 25);
}
#[test]
fn r7_sub_p1_q13_rm1_s25() {
    check_sub_row(1, 13, -1, 25);
}
#[test]
fn r7_sub_p1_q13_r5_s25() {
    check_sub_row(1, 13, 5, 25);
}
#[test]
fn r7_sub_p1_q13_r1_s1000() {
    check_sub_row(1, 13, 1, 1000);
}
#[test]
fn r7_sub_p1_q13_rm1_s1000() {
    check_sub_row(1, 13, -1, 1000);
}
#[test]
fn r7_sub_p1_q13_r5_s1000() {
    check_sub_row(1, 13, 5, 1000);
}
#[test]
fn r7_sub_p2_q3_rm1_s1000() {
    check_sub_row(2, 3, -1, 1000);
}
#[test]
fn r7_sub_p2_q13_r1_s2() {
    check_sub_row(2, 13, 1, 2);
}
#[test]
fn r7_sub_p2_q13_rm1_s2() {
    check_sub_row(2, 13, -1, 2);
}
#[test]
fn r7_sub_p2_q3_r5_s1000() {
    check_sub_row(2, 3, 5, 1000);
}
#[test]
fn r7_sub_p2_q13_r5_s2() {
    check_sub_row(2, 13, 5, 2);
}
#[test]
fn r7_sub_p2_q13_r1_s4() {
    check_sub_row(2, 13, 1, 4);
}
#[test]
fn r7_sub_p2_q13_rm1_s4() {
    check_sub_row(2, 13, -1, 4);
}
#[test]
fn r7_sub_p2_q13_r5_s4() {
    check_sub_row(2, 13, 5, 4);
}
#[test]
fn r7_sub_p2_q13_r1_s25() {
    check_sub_row(2, 13, 1, 25);
}
#[test]
fn r7_sub_p2_q13_rm1_s25() {
    check_sub_row(2, 13, -1, 25);
}
#[test]
fn r7_sub_p2_q13_r5_s25() {
    check_sub_row(2, 13, 5, 25);
}
#[test]
fn r7_sub_p2_q13_r1_s1000() {
    check_sub_row(2, 13, 1, 1000);
}
#[test]
fn r7_sub_p2_q13_rm1_s1000() {
    check_sub_row(2, 13, -1, 1000);
}
#[test]
fn r7_sub_p2_q13_r5_s1000() {
    check_sub_row(2, 13, 5, 1000);
}
#[test]
fn r7_sub_pm5_q13_r1_s2() {
    check_sub_row(-5, 13, 1, 2);
}
#[test]
fn r7_sub_pm5_q13_rm1_s2() {
    check_sub_row(-5, 13, -1, 2);
}
#[test]
fn r7_sub_pm5_q13_r5_s2() {
    check_sub_row(-5, 13, 5, 2);
}
#[test]
fn r7_sub_pm5_q13_r1_s4() {
    check_sub_row(-5, 13, 1, 4);
}
#[test]
fn r7_sub_pm5_q13_rm1_s4() {
    check_sub_row(-5, 13, -1, 4);
}
#[test]
fn r7_sub_pm5_q13_r5_s4() {
    check_sub_row(-5, 13, 5, 4);
}
#[test]
fn r7_sub_pm5_q13_r1_s25() {
    check_sub_row(-5, 13, 1, 25);
}
#[test]
fn r7_sub_pm5_q13_rm1_s25() {
    check_sub_row(-5, 13, -1, 25);
}
#[test]
fn r7_sub_pm5_q13_r5_s25() {
    check_sub_row(-5, 13, 5, 25);
}
#[test]
fn r7_sub_pm5_q13_r1_s1000() {
    check_sub_row(-5, 13, 1, 1000);
}
#[test]
fn r7_sub_pm5_q13_rm1_s1000() {
    check_sub_row(-5, 13, -1, 1000);
}
#[test]
fn r7_sub_pm5_q13_r5_s1000() {
    check_sub_row(-5, 13, 5, 1000);
}
#[test]
fn r7_sub_p1_q17_r1_s2() {
    check_sub_row(1, 17, 1, 2);
}
#[test]
fn r7_sub_p1_q17_rm1_s2() {
    check_sub_row(1, 17, -1, 2);
}
#[test]
fn r7_sub_p1_q17_r5_s2() {
    check_sub_row(1, 17, 5, 2);
}
#[test]
fn r7_sub_p1_q17_r1_s4() {
    check_sub_row(1, 17, 1, 4);
}
#[test]
fn r7_sub_p1_q17_rm1_s4() {
    check_sub_row(1, 17, -1, 4);
}
#[test]
fn r7_sub_p1_q17_r5_s4() {
    check_sub_row(1, 17, 5, 4);
}
#[test]
fn r7_sub_p1_q17_r1_s25() {
    check_sub_row(1, 17, 1, 25);
}
#[test]
fn r7_sub_p1_q17_rm1_s25() {
    check_sub_row(1, 17, -1, 25);
}
#[test]
fn r7_sub_p1_q17_r5_s25() {
    check_sub_row(1, 17, 5, 25);
}
#[test]
fn r7_sub_p1_q17_r1_s1000() {
    check_sub_row(1, 17, 1, 1000);
}
#[test]
fn r7_sub_p1_q17_rm1_s1000() {
    check_sub_row(1, 17, -1, 1000);
}
#[test]
fn r7_sub_p1_q17_r5_s1000() {
    check_sub_row(1, 17, 5, 1000);
}
#[test]
fn r7_sub_p2_q17_r1_s2() {
    check_sub_row(2, 17, 1, 2);
}
#[test]
fn r7_sub_p2_q17_rm1_s2() {
    check_sub_row(2, 17, -1, 2);
}
#[test]
fn r7_sub_p2_q17_r5_s2() {
    check_sub_row(2, 17, 5, 2);
}
#[test]
fn r7_sub_p2_q17_r1_s4() {
    check_sub_row(2, 17, 1, 4);
}
#[test]
fn r7_sub_p2_q17_rm1_s4() {
    check_sub_row(2, 17, -1, 4);
}
#[test]
fn r7_sub_p2_q17_r5_s4() {
    check_sub_row(2, 17, 5, 4);
}
#[test]
fn r7_sub_p2_q17_r1_s25() {
    check_sub_row(2, 17, 1, 25);
}
#[test]
fn r7_sub_p2_q17_rm1_s25() {
    check_sub_row(2, 17, -1, 25);
}
#[test]
fn r7_sub_p2_q17_r5_s25() {
    check_sub_row(2, 17, 5, 25);
}
#[test]
fn r7_sub_p2_q17_r1_s1000() {
    check_sub_row(2, 17, 1, 1000);
}
#[test]
fn r7_sub_p2_q17_rm1_s1000() {
    check_sub_row(2, 17, -1, 1000);
}
#[test]
fn r7_sub_p2_q17_r5_s1000() {
    check_sub_row(2, 17, 5, 1000);
}
#[test]
fn r7_sub_pm5_q17_r1_s2() {
    check_sub_row(-5, 17, 1, 2);
}
#[test]
fn r7_sub_pm5_q17_rm1_s2() {
    check_sub_row(-5, 17, -1, 2);
}
#[test]
fn r7_sub_pm5_q17_r5_s2() {
    check_sub_row(-5, 17, 5, 2);
}
#[test]
fn r7_sub_pm5_q17_r1_s4() {
    check_sub_row(-5, 17, 1, 4);
}
#[test]
fn r7_sub_pm5_q17_rm1_s4() {
    check_sub_row(-5, 17, -1, 4);
}
#[test]
fn r7_sub_pm5_q17_r5_s4() {
    check_sub_row(-5, 17, 5, 4);
}
#[test]
fn r7_sub_pm5_q17_r1_s25() {
    check_sub_row(-5, 17, 1, 25);
}
#[test]
fn r7_sub_pm5_q17_rm1_s25() {
    check_sub_row(-5, 17, -1, 25);
}
#[test]
fn r7_sub_pm5_q17_r5_s25() {
    check_sub_row(-5, 17, 5, 25);
}
#[test]
fn r7_sub_pm5_q17_r1_s1000() {
    check_sub_row(-5, 17, 1, 1000);
}
#[test]
fn r7_sub_pm5_q17_rm1_s1000() {
    check_sub_row(-5, 17, -1, 1000);
}
#[test]
fn r7_sub_pm5_q17_r5_s1000() {
    check_sub_row(-5, 17, 5, 1000);
}
#[test]
fn r7_sub_p1_q19_r1_s2() {
    check_sub_row(1, 19, 1, 2);
}
#[test]
fn r7_sub_p1_q19_rm1_s2() {
    check_sub_row(1, 19, -1, 2);
}
#[test]
fn r7_sub_p1_q19_r5_s2() {
    check_sub_row(1, 19, 5, 2);
}
#[test]
fn r7_sub_p1_q19_r1_s4() {
    check_sub_row(1, 19, 1, 4);
}
#[test]
fn r7_sub_p1_q19_rm1_s4() {
    check_sub_row(1, 19, -1, 4);
}
#[test]
fn r7_sub_p1_q19_r5_s4() {
    check_sub_row(1, 19, 5, 4);
}
#[test]
fn r7_sub_p1_q19_r1_s25() {
    check_sub_row(1, 19, 1, 25);
}
#[test]
fn r7_sub_p1_q19_rm1_s25() {
    check_sub_row(1, 19, -1, 25);
}
#[test]
fn r7_sub_p1_q19_r5_s25() {
    check_sub_row(1, 19, 5, 25);
}
#[test]
fn r7_sub_p1_q19_r1_s1000() {
    check_sub_row(1, 19, 1, 1000);
}
#[test]
fn r7_sub_p1_q19_rm1_s1000() {
    check_sub_row(1, 19, -1, 1000);
}
#[test]
fn r7_sub_p1_q19_r5_s1000() {
    check_sub_row(1, 19, 5, 1000);
}
#[test]
fn r7_sub_pm5_q3_r1_s2() {
    check_sub_row(-5, 3, 1, 2);
}
#[test]
fn r7_sub_pm5_q3_rm1_s2() {
    check_sub_row(-5, 3, -1, 2);
}
#[test]
fn r7_sub_p2_q19_r1_s2() {
    check_sub_row(2, 19, 1, 2);
}
#[test]
fn r7_sub_p2_q19_rm1_s2() {
    check_sub_row(2, 19, -1, 2);
}
#[test]
fn r7_sub_p2_q19_r5_s2() {
    check_sub_row(2, 19, 5, 2);
}
#[test]
fn r7_sub_p2_q19_r1_s4() {
    check_sub_row(2, 19, 1, 4);
}
#[test]
fn r7_sub_p2_q19_rm1_s4() {
    check_sub_row(2, 19, -1, 4);
}
#[test]
fn r7_sub_p2_q19_r5_s4() {
    check_sub_row(2, 19, 5, 4);
}
#[test]
fn r7_sub_p2_q19_r1_s25() {
    check_sub_row(2, 19, 1, 25);
}
#[test]
fn r7_sub_p2_q19_rm1_s25() {
    check_sub_row(2, 19, -1, 25);
}
#[test]
fn r7_sub_pm5_q3_r5_s2() {
    check_sub_row(-5, 3, 5, 2);
}
#[test]
fn r7_sub_p2_q19_r5_s25() {
    check_sub_row(2, 19, 5, 25);
}
#[test]
fn r7_sub_p2_q19_r1_s1000() {
    check_sub_row(2, 19, 1, 1000);
}
#[test]
fn r7_sub_p2_q19_rm1_s1000() {
    check_sub_row(2, 19, -1, 1000);
}
#[test]
fn r7_sub_p2_q19_r5_s1000() {
    check_sub_row(2, 19, 5, 1000);
}
#[test]
fn r7_sub_pm5_q3_r1_s4() {
    check_sub_row(-5, 3, 1, 4);
}
#[test]
fn r7_sub_pm5_q19_r1_s2() {
    check_sub_row(-5, 19, 1, 2);
}
#[test]
fn r7_sub_pm5_q19_rm1_s2() {
    check_sub_row(-5, 19, -1, 2);
}
#[test]
fn r7_sub_pm5_q19_r5_s2() {
    check_sub_row(-5, 19, 5, 2);
}
#[test]
fn r7_sub_pm5_q19_r1_s4() {
    check_sub_row(-5, 19, 1, 4);
}
#[test]
fn r7_sub_pm5_q3_rm1_s4() {
    check_sub_row(-5, 3, -1, 4);
}
#[test]
fn r7_sub_pm5_q19_rm1_s4() {
    check_sub_row(-5, 19, -1, 4);
}
#[test]
fn r7_sub_pm5_q19_r5_s4() {
    check_sub_row(-5, 19, 5, 4);
}
#[test]
fn r7_sub_pm5_q19_r1_s25() {
    check_sub_row(-5, 19, 1, 25);
}
#[test]
fn r7_sub_pm5_q19_rm1_s25() {
    check_sub_row(-5, 19, -1, 25);
}
#[test]
fn r7_sub_pm5_q19_r5_s25() {
    check_sub_row(-5, 19, 5, 25);
}
#[test]
fn r7_sub_pm5_q19_r1_s1000() {
    check_sub_row(-5, 19, 1, 1000);
}
#[test]
fn r7_sub_pm5_q19_rm1_s1000() {
    check_sub_row(-5, 19, -1, 1000);
}
#[test]
fn r7_sub_pm5_q19_r5_s1000() {
    check_sub_row(-5, 19, 5, 1000);
}
#[test]
fn r7_sub_pm5_q3_r5_s4() {
    check_sub_row(-5, 3, 5, 4);
}
#[test]
fn r7_sub_pm5_q3_r1_s25() {
    check_sub_row(-5, 3, 1, 25);
}
#[test]
fn r7_sub_p1_q21_r1_s2() {
    check_sub_row(1, 21, 1, 2);
}
#[test]
fn r7_sub_p1_q21_rm1_s2() {
    check_sub_row(1, 21, -1, 2);
}
#[test]
fn r7_sub_p1_q21_r5_s2() {
    check_sub_row(1, 21, 5, 2);
}
#[test]
fn r7_sub_p1_q21_r1_s4() {
    check_sub_row(1, 21, 1, 4);
}
#[test]
fn r7_sub_p1_q21_rm1_s4() {
    check_sub_row(1, 21, -1, 4);
}
#[test]
fn r7_sub_p1_q21_r5_s4() {
    check_sub_row(1, 21, 5, 4);
}
#[test]
fn r7_sub_p1_q21_r1_s25() {
    check_sub_row(1, 21, 1, 25);
}
#[test]
fn r7_sub_p1_q21_rm1_s25() {
    check_sub_row(1, 21, -1, 25);
}
#[test]
fn r7_sub_p1_q21_r5_s25() {
    check_sub_row(1, 21, 5, 25);
}
#[test]
fn r7_sub_p1_q21_r1_s1000() {
    check_sub_row(1, 21, 1, 1000);
}
#[test]
fn r7_sub_pm5_q3_rm1_s25() {
    check_sub_row(-5, 3, -1, 25);
}
#[test]
fn r7_sub_p1_q21_rm1_s1000() {
    check_sub_row(1, 21, -1, 1000);
}
#[test]
fn r7_sub_p1_q21_r5_s1000() {
    check_sub_row(1, 21, 5, 1000);
}
#[test]
fn r7_sub_pm5_q3_r5_s25() {
    check_sub_row(-5, 3, 5, 25);
}
#[test]
fn r7_sub_p2_q21_r1_s2() {
    check_sub_row(2, 21, 1, 2);
}
#[test]
fn r7_sub_p2_q21_rm1_s2() {
    check_sub_row(2, 21, -1, 2);
}
#[test]
fn r7_sub_p2_q21_r5_s2() {
    check_sub_row(2, 21, 5, 2);
}
#[test]
fn r7_sub_p2_q21_r1_s4() {
    check_sub_row(2, 21, 1, 4);
}
#[test]
fn r7_sub_p2_q21_rm1_s4() {
    check_sub_row(2, 21, -1, 4);
}
#[test]
fn r7_sub_p2_q21_r5_s4() {
    check_sub_row(2, 21, 5, 4);
}
#[test]
fn r7_sub_pm5_q3_r1_s1000() {
    check_sub_row(-5, 3, 1, 1000);
}
#[test]
fn r7_sub_p2_q21_r1_s25() {
    check_sub_row(2, 21, 1, 25);
}
#[test]
fn r7_sub_p2_q21_rm1_s25() {
    check_sub_row(2, 21, -1, 25);
}
#[test]
fn r7_sub_p2_q21_r5_s25() {
    check_sub_row(2, 21, 5, 25);
}
#[test]
fn r7_sub_p2_q21_r1_s1000() {
    check_sub_row(2, 21, 1, 1000);
}
#[test]
fn r7_sub_p2_q21_rm1_s1000() {
    check_sub_row(2, 21, -1, 1000);
}
#[test]
fn r7_sub_p2_q21_r5_s1000() {
    check_sub_row(2, 21, 5, 1000);
}
#[test]
fn r7_sub_pm5_q3_rm1_s1000() {
    check_sub_row(-5, 3, -1, 1000);
}
#[test]
fn r7_sub_pm5_q21_r1_s2() {
    check_sub_row(-5, 21, 1, 2);
}
#[test]
fn r7_sub_pm5_q21_rm1_s2() {
    check_sub_row(-5, 21, -1, 2);
}
#[test]
fn r7_sub_pm5_q3_r5_s1000() {
    check_sub_row(-5, 3, 5, 1000);
}
#[test]
fn r7_sub_pm5_q21_r5_s2() {
    check_sub_row(-5, 21, 5, 2);
}
#[test]
fn r7_sub_pm5_q21_r1_s4() {
    check_sub_row(-5, 21, 1, 4);
}
#[test]
fn r7_sub_pm5_q21_rm1_s4() {
    check_sub_row(-5, 21, -1, 4);
}
#[test]
fn r7_sub_pm5_q21_r5_s4() {
    check_sub_row(-5, 21, 5, 4);
}
#[test]
fn r7_sub_pm5_q21_r1_s25() {
    check_sub_row(-5, 21, 1, 25);
}
#[test]
fn r7_sub_pm5_q21_rm1_s25() {
    check_sub_row(-5, 21, -1, 25);
}
#[test]
fn r7_sub_pm5_q21_r5_s25() {
    check_sub_row(-5, 21, 5, 25);
}
#[test]
fn r7_sub_pm5_q21_r1_s1000() {
    check_sub_row(-5, 21, 1, 1000);
}
#[test]
fn r7_sub_pm5_q21_rm1_s1000() {
    check_sub_row(-5, 21, -1, 1000);
}
#[test]
fn r7_sub_pm5_q21_r5_s1000() {
    check_sub_row(-5, 21, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r1_s3() {
    check_sub_row(1, 6, 1, 3);
}
#[test]
fn r7_sub_p1_q6_rm1_s3() {
    check_sub_row(1, 6, -1, 3);
}
#[test]
fn r7_sub_p1_q37_r1_s2() {
    check_sub_row(1, 37, 1, 2);
}
#[test]
fn r7_sub_p1_q37_rm1_s2() {
    check_sub_row(1, 37, -1, 2);
}
#[test]
fn r7_sub_p1_q37_r5_s2() {
    check_sub_row(1, 37, 5, 2);
}
#[test]
fn r7_sub_p1_q37_r1_s4() {
    check_sub_row(1, 37, 1, 4);
}
#[test]
fn r7_sub_p1_q37_rm1_s4() {
    check_sub_row(1, 37, -1, 4);
}
#[test]
fn r7_sub_p1_q37_r5_s4() {
    check_sub_row(1, 37, 5, 4);
}
#[test]
fn r7_sub_p1_q37_r1_s25() {
    check_sub_row(1, 37, 1, 25);
}
#[test]
fn r7_sub_p1_q37_rm1_s25() {
    check_sub_row(1, 37, -1, 25);
}
#[test]
fn r7_sub_p1_q6_r5_s3() {
    check_sub_row(1, 6, 5, 3);
}
#[test]
fn r7_sub_p1_q37_r5_s25() {
    check_sub_row(1, 37, 5, 25);
}
#[test]
fn r7_sub_p1_q37_r1_s1000() {
    check_sub_row(1, 37, 1, 1000);
}
#[test]
fn r7_sub_p1_q37_rm1_s1000() {
    check_sub_row(1, 37, -1, 1000);
}
#[test]
fn r7_sub_p1_q37_r5_s1000() {
    check_sub_row(1, 37, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r1_s7() {
    check_sub_row(1, 6, 1, 7);
}
#[test]
fn r7_sub_p2_q37_r1_s2() {
    check_sub_row(2, 37, 1, 2);
}
#[test]
fn r7_sub_p2_q37_rm1_s2() {
    check_sub_row(2, 37, -1, 2);
}
#[test]
fn r7_sub_p2_q37_r5_s2() {
    check_sub_row(2, 37, 5, 2);
}
#[test]
fn r7_sub_p2_q37_r1_s4() {
    check_sub_row(2, 37, 1, 4);
}
#[test]
fn r7_sub_p1_q6_rm1_s7() {
    check_sub_row(1, 6, -1, 7);
}
#[test]
fn r7_sub_p2_q37_rm1_s4() {
    check_sub_row(2, 37, -1, 4);
}
#[test]
fn r7_sub_p2_q37_r5_s4() {
    check_sub_row(2, 37, 5, 4);
}
#[test]
fn r7_sub_p2_q37_r1_s25() {
    check_sub_row(2, 37, 1, 25);
}
#[test]
fn r7_sub_p2_q37_rm1_s25() {
    check_sub_row(2, 37, -1, 25);
}
#[test]
fn r7_sub_p2_q37_r5_s25() {
    check_sub_row(2, 37, 5, 25);
}
#[test]
fn r7_sub_p2_q37_r1_s1000() {
    check_sub_row(2, 37, 1, 1000);
}
#[test]
fn r7_sub_p2_q37_rm1_s1000() {
    check_sub_row(2, 37, -1, 1000);
}
#[test]
fn r7_sub_p2_q37_r5_s1000() {
    check_sub_row(2, 37, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r5_s7() {
    check_sub_row(1, 6, 5, 7);
}
#[test]
fn r7_sub_p1_q6_r1_s17() {
    check_sub_row(1, 6, 1, 17);
}
#[test]
fn r7_sub_pm5_q37_r1_s2() {
    check_sub_row(-5, 37, 1, 2);
}
#[test]
fn r7_sub_pm5_q37_rm1_s2() {
    check_sub_row(-5, 37, -1, 2);
}
#[test]
fn r7_sub_pm5_q37_r5_s2() {
    check_sub_row(-5, 37, 5, 2);
}
#[test]
fn r7_sub_pm5_q37_r1_s4() {
    check_sub_row(-5, 37, 1, 4);
}
#[test]
fn r7_sub_pm5_q37_rm1_s4() {
    check_sub_row(-5, 37, -1, 4);
}
#[test]
fn r7_sub_pm5_q37_r5_s4() {
    check_sub_row(-5, 37, 5, 4);
}
#[test]
fn r7_sub_pm5_q37_r1_s25() {
    check_sub_row(-5, 37, 1, 25);
}
#[test]
fn r7_sub_pm5_q37_rm1_s25() {
    check_sub_row(-5, 37, -1, 25);
}
#[test]
fn r7_sub_pm5_q37_r5_s25() {
    check_sub_row(-5, 37, 5, 25);
}
#[test]
fn r7_sub_pm5_q37_r1_s1000() {
    check_sub_row(-5, 37, 1, 1000);
}
#[test]
fn r7_sub_p1_q6_rm1_s17() {
    check_sub_row(1, 6, -1, 17);
}
#[test]
fn r7_sub_pm5_q37_rm1_s1000() {
    check_sub_row(-5, 37, -1, 1000);
}
#[test]
fn r7_sub_pm5_q37_r5_s1000() {
    check_sub_row(-5, 37, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r5_s17() {
    check_sub_row(1, 6, 5, 17);
}
#[test]
fn r7_sub_p1_q99_r1_s2() {
    check_sub_row(1, 99, 1, 2);
}
#[test]
fn r7_sub_p1_q99_rm1_s2() {
    check_sub_row(1, 99, -1, 2);
}
#[test]
fn r7_sub_p1_q99_r5_s2() {
    check_sub_row(1, 99, 5, 2);
}
#[test]
fn r7_sub_p1_q99_r1_s4() {
    check_sub_row(1, 99, 1, 4);
}
#[test]
fn r7_sub_p1_q99_rm1_s4() {
    check_sub_row(1, 99, -1, 4);
}
#[test]
fn r7_sub_p1_q99_r5_s4() {
    check_sub_row(1, 99, 5, 4);
}
#[test]
fn r7_sub_p1_q6_r1_s99() {
    check_sub_row(1, 6, 1, 99);
}
#[test]
fn r7_sub_p1_q99_r1_s25() {
    check_sub_row(1, 99, 1, 25);
}
#[test]
fn r7_sub_p1_q99_rm1_s25() {
    check_sub_row(1, 99, -1, 25);
}
#[test]
fn r7_sub_p1_q99_r5_s25() {
    check_sub_row(1, 99, 5, 25);
}
#[test]
fn r7_sub_p1_q99_r1_s1000() {
    check_sub_row(1, 99, 1, 1000);
}
#[test]
fn r7_sub_p1_q99_rm1_s1000() {
    check_sub_row(1, 99, -1, 1000);
}
#[test]
fn r7_sub_p1_q99_r5_s1000() {
    check_sub_row(1, 99, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_rm1_s99() {
    check_sub_row(1, 6, -1, 99);
}
#[test]
fn r7_sub_p2_q99_r1_s2() {
    check_sub_row(2, 99, 1, 2);
}
#[test]
fn r7_sub_p2_q99_rm1_s2() {
    check_sub_row(2, 99, -1, 2);
}
#[test]
fn r7_sub_p1_q6_r5_s99() {
    check_sub_row(1, 6, 5, 99);
}
#[test]
fn r7_sub_p2_q99_r5_s2() {
    check_sub_row(2, 99, 5, 2);
}
#[test]
fn r7_sub_p2_q99_r1_s4() {
    check_sub_row(2, 99, 1, 4);
}
#[test]
fn r7_sub_p2_q99_rm1_s4() {
    check_sub_row(2, 99, -1, 4);
}
#[test]
fn r7_sub_p2_q99_r5_s4() {
    check_sub_row(2, 99, 5, 4);
}
#[test]
fn r7_sub_p2_q99_r1_s25() {
    check_sub_row(2, 99, 1, 25);
}
#[test]
fn r7_sub_p2_q99_rm1_s25() {
    check_sub_row(2, 99, -1, 25);
}
#[test]
fn r7_sub_p2_q99_r5_s25() {
    check_sub_row(2, 99, 5, 25);
}
#[test]
fn r7_sub_p2_q99_r1_s1000() {
    check_sub_row(2, 99, 1, 1000);
}
#[test]
fn r7_sub_p2_q99_rm1_s1000() {
    check_sub_row(2, 99, -1, 1000);
}
#[test]
fn r7_sub_p2_q99_r5_s1000() {
    check_sub_row(2, 99, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r1_s2() {
    check_sub_row(1, 6, 1, 2);
}
#[test]
fn r7_sub_p1_q6_rm1_s2() {
    check_sub_row(1, 6, -1, 2);
}
#[test]
fn r7_sub_pm5_q99_r1_s2() {
    check_sub_row(-5, 99, 1, 2);
}
#[test]
fn r7_sub_pm5_q99_rm1_s2() {
    check_sub_row(-5, 99, -1, 2);
}
#[test]
fn r7_sub_pm5_q99_r5_s2() {
    check_sub_row(-5, 99, 5, 2);
}
#[test]
fn r7_sub_pm5_q99_r1_s4() {
    check_sub_row(-5, 99, 1, 4);
}
#[test]
fn r7_sub_pm5_q99_rm1_s4() {
    check_sub_row(-5, 99, -1, 4);
}
#[test]
fn r7_sub_pm5_q99_r5_s4() {
    check_sub_row(-5, 99, 5, 4);
}
#[test]
fn r7_sub_pm5_q99_r1_s25() {
    check_sub_row(-5, 99, 1, 25);
}
#[test]
fn r7_sub_pm5_q99_rm1_s25() {
    check_sub_row(-5, 99, -1, 25);
}
#[test]
fn r7_sub_p1_q6_r5_s2() {
    check_sub_row(1, 6, 5, 2);
}
#[test]
fn r7_sub_pm5_q99_r5_s25() {
    check_sub_row(-5, 99, 5, 25);
}
#[test]
fn r7_sub_pm5_q99_r1_s1000() {
    check_sub_row(-5, 99, 1, 1000);
}
#[test]
fn r7_sub_pm5_q99_rm1_s1000() {
    check_sub_row(-5, 99, -1, 1000);
}
#[test]
fn r7_sub_pm5_q99_r5_s1000() {
    check_sub_row(-5, 99, 5, 1000);
}
#[test]
fn r7_sub_p1_q6_r1_s4() {
    check_sub_row(1, 6, 1, 4);
}
#[test]
fn r7_sub_p1_q6_rm1_s4() {
    check_sub_row(1, 6, -1, 4);
}
#[test]
fn r7_sub_p1_q6_r5_s4() {
    check_sub_row(1, 6, 5, 4);
}
#[test]
fn r7_sub_p1_q6_r1_s25() {
    check_sub_row(1, 6, 1, 25);
}
#[test]
fn r7_sub_p1_q6_rm1_s25() {
    check_sub_row(1, 6, -1, 25);
}
#[test]
fn r7_sub_p1_q6_r5_s25() {
    check_sub_row(1, 6, 5, 25);
}
#[test]
fn r7_sub_p1_q6_r1_s1000() {
    check_sub_row(1, 6, 1, 1000);
}
#[test]
fn r7_sub_p1_q6_rm1_s1000() {
    check_sub_row(1, 6, -1, 1000);
}
#[test]
fn r7_sub_p1_q6_r5_s1000() {
    check_sub_row(1, 6, 5, 1000);
}
#[test]
fn r7_mul_p1_q22() {
    check_mul_row(1, "22");
}
#[test]
fn r7_mul_p7_q22() {
    check_mul_row(7, "22");
}
#[test]
fn r7_mul_p123_q22() {
    check_mul_row(123, "22");
}
#[test]
fn r7_mul_p1_q24() {
    check_mul_row(1, "24");
}
#[test]
fn r7_mul_p2_q24() {
    check_mul_row(2, "24");
}
#[test]
fn r7_mul_p7_q24() {
    check_mul_row(7, "24");
}
#[test]
fn r7_mul_p1_q26() {
    check_mul_row(1, "26");
}
#[test]
fn r7_mul_p7_q26() {
    check_mul_row(7, "26");
}
#[test]
fn r7_mul_p123_q26() {
    check_mul_row(123, "26");
}
#[test]
fn r7_mul_p9999_q26() {
    check_mul_row(9999, "26");
}
#[test]
fn r7_mul_p1_q28() {
    check_mul_row(1, "28");
}
#[test]
fn r7_mul_p2_q28() {
    check_mul_row(2, "28");
}
#[test]
fn r7_mul_p123_q28() {
    check_mul_row(123, "28");
}
#[test]
fn r7_mul_p9999_q28() {
    check_mul_row(9999, "28");
}
#[test]
fn r7_mul_p1_q30() {
    check_mul_row(1, "30");
}
#[test]
fn r7_mul_p2_q30() {
    check_mul_row(2, "30");
}
#[test]
fn r7_mul_p7_q30() {
    check_mul_row(7, "30");
}
#[test]
fn r7_mul_p1_q34() {
    check_mul_row(1, "34");
}
#[test]
fn r7_mul_p7_q34() {
    check_mul_row(7, "34");
}
#[test]
fn r7_mul_p123_q34() {
    check_mul_row(123, "34");
}
#[test]
fn r7_mul_p9999_q34() {
    check_mul_row(9999, "34");
}
#[test]
fn r7_mul_p1_q35() {
    check_mul_row(1, "35");
}
#[test]
fn r7_mul_p2_q35() {
    check_mul_row(2, "35");
}
#[test]
fn r7_mul_p123_q35() {
    check_mul_row(123, "35");
}
#[test]
fn r7_mul_p9999_q35() {
    check_mul_row(9999, "35");
}
#[test]
fn r7_mul_p1_q36() {
    check_mul_row(1, "36");
}
#[test]
fn r7_mul_p2_q36() {
    check_mul_row(2, "36");
}
#[test]
fn r7_mul_p7_q36() {
    check_mul_row(7, "36");
}
#[test]
fn r7_mul_p123_q36() {
    check_mul_row(123, "36");
}
#[test]
fn r7_mul_p1_q38() {
    check_mul_row(1, "38");
}
#[test]
fn r7_mul_p7_q38() {
    check_mul_row(7, "38");
}
#[test]
fn r7_mul_p123_q38() {
    check_mul_row(123, "38");
}
#[test]
fn r7_mul_p9999_q38() {
    check_mul_row(9999, "38");
}
#[test]
fn r7_mul_p1_q6() {
    check_mul_row(1, "6");
}
#[test]
fn r7_mul_p1_q42() {
    check_mul_row(1, "42");
}
#[test]
fn r7_mul_p7_q42() {
    check_mul_row(7, "42");
}
#[test]
fn r7_mul_p123_q42() {
    check_mul_row(123, "42");
}
#[test]
fn r7_mul_p9999_q42() {
    check_mul_row(9999, "42");
}
#[test]
fn r7_mul_p1_q44() {
    check_mul_row(1, "44");
}
#[test]
fn r7_mul_p2_q44() {
    check_mul_row(2, "44");
}
#[test]
fn r7_mul_p7_q44() {
    check_mul_row(7, "44");
}
#[test]
fn r7_mul_p123_q44() {
    check_mul_row(123, "44");
}
#[test]
fn r7_mul_p1_q45() {
    check_mul_row(1, "45");
}
#[test]
fn r7_mul_p2_q45() {
    check_mul_row(2, "45");
}
#[test]
fn r7_mul_p7_q45() {
    check_mul_row(7, "45");
}
#[test]
fn r7_mul_p123_q45() {
    check_mul_row(123, "45");
}
#[test]
fn r7_mul_p7_q6() {
    check_mul_row(7, "6");
}
#[test]
fn r7_mul_p1_q46() {
    check_mul_row(1, "46");
}
#[test]
fn r7_mul_p7_q46() {
    check_mul_row(7, "46");
}
#[test]
fn r7_mul_p123_q46() {
    check_mul_row(123, "46");
}
#[test]
fn r7_mul_p9999_q46() {
    check_mul_row(9999, "46");
}
#[test]
fn r7_mul_p1_q48() {
    check_mul_row(1, "48");
}
#[test]
fn r7_mul_p2_q48() {
    check_mul_row(2, "48");
}
#[test]
fn r7_mul_p7_q48() {
    check_mul_row(7, "48");
}
#[test]
fn r7_mul_p1_q52() {
    check_mul_row(1, "52");
}
#[test]
fn r7_mul_p2_q52() {
    check_mul_row(2, "52");
}
#[test]
fn r7_mul_p7_q52() {
    check_mul_row(7, "52");
}
#[test]
fn r7_mul_p123_q52() {
    check_mul_row(123, "52");
}
#[test]
fn r7_mul_p9999_q52() {
    check_mul_row(9999, "52");
}
#[test]
fn r7_mul_p1_q54() {
    check_mul_row(1, "54");
}
#[test]
fn r7_mul_p7_q54() {
    check_mul_row(7, "54");
}
#[test]
fn r7_mul_p123_q54() {
    check_mul_row(123, "54");
}
#[test]
fn r7_mul_p9999_q54() {
    check_mul_row(9999, "54");
}
#[test]
fn r7_mul_p1_q55() {
    check_mul_row(1, "55");
}
#[test]
fn r7_mul_p2_q55() {
    check_mul_row(2, "55");
}
#[test]
fn r7_mul_p7_q55() {
    check_mul_row(7, "55");
}
#[test]
fn r7_mul_p123_q55() {
    check_mul_row(123, "55");
}
#[test]
fn r7_mul_p1_q56() {
    check_mul_row(1, "56");
}
#[test]
fn r7_mul_p2_q56() {
    check_mul_row(2, "56");
}
#[test]
fn r7_mul_p123_q56() {
    check_mul_row(123, "56");
}
#[test]
fn r7_mul_p9999_q56() {
    check_mul_row(9999, "56");
}
#[test]
fn r7_mul_p1_q58() {
    check_mul_row(1, "58");
}
#[test]
fn r7_mul_p7_q58() {
    check_mul_row(7, "58");
}
#[test]
fn r7_mul_p123_q58() {
    check_mul_row(123, "58");
}
#[test]
fn r7_mul_p9999_q58() {
    check_mul_row(9999, "58");
}
#[test]
fn r7_mul_p1_q60() {
    check_mul_row(1, "60");
}
#[test]
fn r7_mul_p2_q60() {
    check_mul_row(2, "60");
}
#[test]
fn r7_mul_p7_q60() {
    check_mul_row(7, "60");
}
#[test]
fn r7_mul_p1_q12() {
    check_mul_row(1, "12");
}
#[test]
fn r7_mul_p2_q12() {
    check_mul_row(2, "12");
}
#[test]
fn r7_mul_p7_q12() {
    check_mul_row(7, "12");
}
#[test]
fn r7_mul_p1_q14() {
    check_mul_row(1, "14");
}
#[test]
fn r7_mul_p123_q14() {
    check_mul_row(123, "14");
}
#[test]
fn r7_mul_p9999_q14() {
    check_mul_row(9999, "14");
}
#[test]
fn r7_mul_p1_q15() {
    check_mul_row(1, "15");
}
#[test]
fn r7_mul_p2_q15() {
    check_mul_row(2, "15");
}
#[test]
fn r7_mul_p7_q15() {
    check_mul_row(7, "15");
}
#[test]
fn r7_mul_p1_q18() {
    check_mul_row(1, "18");
}
#[test]
fn r7_mul_p7_q18() {
    check_mul_row(7, "18");
}
#[test]
fn r7_mul_p123_q18() {
    check_mul_row(123, "18");
}

// ------------------------------------------------------------------
// The twelve fact rows round 6 also refused as PREC.
// ------------------------------------------------------------------

fn pow10(n: usize) -> Nat {
    Nat::from_u64(10).pow(n as u64)
}

#[test]
fn r7_add_one_sixth_plus_one_sixth() {
    // C# RealAddTests.cs:128: 0.1(6) + 0.1(6) = 1/6 + 1/6 = 1/3 = 0.(3),
    // PeriodStart = 0 and PeriodLength = 1.
    let sixth = r("0.1(6)");
    let result = sixth.add(&sixth);
    assert!(result == r("0.(3)"));
    assert!(is_periodic(&result));
    assert_eq!(result.to_string(), "0.(3)");
    assert_eq!(result.to_rational(), (Int::from_i64(1), Nat::from_u64(3)));
}

#[test]
fn r7_sub_third_minus_998_threes() {
    check_third_minus_n_threes(998);
}

#[test]
fn r7_sub_third_minus_999_threes() {
    check_third_minus_n_threes(999);
}

#[test]
fn r7_sub_third_minus_1000_threes() {
    check_third_minus_n_threes(1000);
}

fn check_third_minus_n_threes(n: usize) {
    // C# RealAddPeriodicExactTests.cs:213: 1/3 - 0.(n threes) = 1/(3*10^n), a period of
    // length 1 starting at fractional place n (so the rendering carries n zeros then "(3)").
    let third = Real::from_i64(1).div(&Real::from_i64(3), 1000).unwrap();
    let literal = r(&format!("0.{}", "3".repeat(n)));
    let difference = third.sub(&literal);
    let denominator = Nat::from_u64(3).mul(&pow10(n));
    assert_eq!(
        difference.to_rational(),
        (Int::from_i64(1), denominator.clone())
    );
    assert_eq!(difference.to_string(), format!("0.{}(3)", "0".repeat(n)));

    // The direct helper re-derives the same exact period once the budget has room for the
    // digit that closes it (the period spans indices n..=n, so it is discovered on the
    // iteration after position n + 1).
    assert_eq!(
        difference,
        fraction_real(Int::from_i64(1), denominator.clone(), n + 2)
    );
    // At exactly n + 1 places the reference's digit loop reaches the budget BEFORE the
    // repeated-remainder test, so the direct division is the budgeted TRUNCATION (n zeros and
    // one 3), inexact and without a period. This is the boundary rule of round 34
    // (round34_inexact_division::the_reference_truncates_a_period_that_closes_exactly_at_the_cap).
    let capped = fraction_real(Int::from_i64(1), denominator, n + 1);
    assert!(
        !capped.is_exact(),
        "a cap truncation must not advertise exactness"
    );
    assert_eq!(capped.to_string(), format!("0.{}3", "0".repeat(n)));
}

#[test]
fn r7_sub_periodic_operand_minus_its_own_truncation() {
    // C# RealAddPeriodicExactTests.cs:159: 1/3 - 0.(1000 threes) = 1/(3*10^1000),
    // PeriodStart = 1000, PeriodLength = 1, Exponent = -1001.
    let third = Real::from_i64(1).div(&Real::from_i64(3), 1000).unwrap();
    let truncated = r(&format!("0.{}", "3".repeat(1000)));
    let ten_to_1000 = pow10(1000);
    let truncated_num = ten_to_1000
        .sub(&Nat::from_u64(1))
        .unwrap()
        .div_rem(&Nat::from_u64(3))
        .unwrap()
        .0;
    assert_eq!(
        truncated.to_rational(),
        (int_from_nat(&truncated_num), ten_to_1000.clone())
    );

    let difference = third.sub(&truncated);
    assert!(difference != Real::zero());
    assert!(!difference.is_zero());
    let denominator = Nat::from_u64(3).mul(&ten_to_1000);
    assert_eq!(difference.to_rational(), (Int::from_i64(1), denominator));
    assert_eq!(difference.to_string(), format!("0.{}(3)", "0".repeat(1000)));

    let scaled = Real::from_i64(3).mul(&difference);
    let scaled_literal = r(&format!("0.{}1", "0".repeat(999)));
    assert_eq!(scaled, scaled_literal);
    assert_eq!(scaled.to_rational(), (Int::from_i64(1), ten_to_1000));
}

#[test]
fn r7_div_periodic_operands_keep_representable_quotients_exact() {
    // C# RealDivideExactnessPropertyTests.cs:68, including the audit probe
    // (5/6)/(7/11) = 55/42.
    let cases = [
        ("0.(3)", "0.(3)", "1", "1"),
        ("0.(9)", "2", "1", "2"),
        ("0.(3)", "0.5", "2", "3"),
        ("0.(142857)", "0.(142857)", "1", "1"),
        ("1.(3)", "0.(3)", "4", "1"),
        ("0.(6)", "0.(3)", "2", "1"),
        ("0.1(6)", "0.(3)", "1", "2"),
        ("0.(9)", "0.(3)", "3", "1"),
    ];
    for (left_text, right_text, exp_num, exp_den) in cases {
        let left = r(left_text);
        let right = r(right_text);
        let expected = Real::from_str(exp_num)
            .unwrap()
            .div(&Real::from_str(exp_den).unwrap(), 1000)
            .unwrap();
        let quotient = left.div(&right, 1000).unwrap();
        assert!(
            quotient == expected,
            "{left_text}/{right_text} = {quotient}, expected {expected}"
        );
        assert!(
            quotient.is_exact(),
            "{left_text}/{right_text} = {quotient} was reported inexact"
        );
    }
    let by_division = Real::from_i64(5)
        .div(&Real::from_i64(6), 1000)
        .unwrap()
        .div(
            &Real::from_i64(7).div(&Real::from_i64(11), 1000).unwrap(),
            1000,
        )
        .unwrap();
    let by_multiplication = Real::from_i64(5)
        .div(&Real::from_i64(6), 1000)
        .unwrap()
        .mul(&Real::from_i64(11).div(&Real::from_i64(7), 1000).unwrap());
    assert!(by_division == by_multiplication);
    assert!(by_division.is_exact() && by_multiplication.is_exact());
    assert!(by_division == Real::from_i64(55).div(&Real::from_i64(42), 1000).unwrap());
}

#[test]
fn r7_div_unrepresentable_quotients_stay_inexact() {
    // C# RealDivideExactnessPropertyTests.cs:116, at the 10-place budget it pins.
    let period_beyond_budget = Real::from_i64(1)
        .div(&Real::from_i64(3), 10)
        .unwrap()
        .div(&Real::from_i64(97), 10)
        .unwrap();
    assert!(
        !period_beyond_budget.is_exact(),
        "1/291 at 10 places was reported exact"
    );
    assert!(period_beyond_budget > Real::zero() && period_beyond_budget < Real::from_i64(1));

    let irrational = r("2").sqrt(1000);
    let third = r("0.(3)");
    assert!(!irrational.div(&third, 1000).unwrap().is_exact());
    assert!(!third.div(&irrational, 1000).unwrap().is_exact());
}

#[test]
fn r7_mul_audit_1_over_6_times_6() {
    // C# RealMultiplyPeriodicExactTests.cs:125 audit shape (1, "6"): 0.1(6) * 6 = 1.
    let product = Real::from_i64(1).div(&r("6"), 1000).unwrap().mul(&r("6"));
    assert!(product == Real::from_i64(1));
    assert_eq!(product.to_rational(), (Int::from_i64(1), Nat::from_u64(1)));
}

#[test]
fn r7_mul_periodic_times_nonperiodic_is_exact() {
    // C# RealMultiplyPeriodicExactTests.cs:208: 0.(3)*3 = 1 and 0.(3)*0.5 = 0.1(6).
    let third = Real::from_i64(1).div(&Real::from_i64(3), 1000).unwrap();
    assert!(third.mul(&r("3")) == Real::from_i64(1));
    let half = third.mul(&r("0.5"));
    assert_eq!(half.to_string(), "0.1(6)");
    assert_eq!(half.to_rational(), (Int::from_i64(1), Nat::from_u64(6)));
}

#[test]
fn r7_mul_periodic_times_truncated_irrational_is_exact_for_stored_values() {
    // C# RealMultiplyPeriodicExactTests.cs:228: sqrt(2) truncated at 50 places times 1/3
    // must equal the product of the two stored rationals, and the division path must agree.
    let sqrt2 = r("1.41421356237309504880168872420969807856967187537694");
    let third = Real::from_i64(1).div(&Real::from_i64(3), 1000).unwrap();
    assert!(is_periodic(&third));
    let product = sqrt2.mul(&third);
    let (sn, sd) = sqrt2.to_rational();
    let (tn, td) = third.to_rational();
    let (en, ed) = lowest_terms(sn.mul(&tn), sd.mul(&td));
    assert_eq!(product.to_rational(), (en, ed));
    assert!(product == sqrt2.div(&r("3"), 1000).unwrap());
}

#[test]
fn r7_periodic_tail_after_nine_is_the_next_integer() {
    // C# RealPeriodicIntegerValuePropertyTests.cs:78: k.9(9) is k + 1.
    for k in 0i64..=5 {
        let literal = format!("{k}.9(9)");
        let value = r(&literal);
        let expected = Real::from_i64(k + 1);
        assert!(value == expected, "{literal} == {expected} was false");
        assert_eq!(value.cmp(&expected), Ordering::Equal);
        assert!(value.sub(&expected).is_zero());
        assert_eq!(
            value.to_rational(),
            (Int::from_i64(k + 1), Nat::from_u64(1))
        );
    }
}

// ------------------------------------------------------------------
// Direct conversion pins for the defect itself.
// ------------------------------------------------------------------

#[test]
fn r7_conversion_0_1_6_is_one_sixth() {
    assert_eq!(
        r("0.1(6)").to_rational(),
        (Int::from_i64(1), Nat::from_u64(6))
    );
}

#[test]
fn r7_conversion_0_27_02_is_107_over_396() {
    assert_eq!(
        r("0.27(02)").to_rational(),
        (Int::from_i64(107), Nat::from_u64(396))
    );
}

#[test]
fn r7_conversion_terminating_decimals_keep_their_value() {
    for (literal, num, den) in [
        ("2.5", 5i64, 2u64),
        ("0.125", 1, 8),
        ("123.456", 15432, 125),
        ("0.000", 0, 1),
        ("-1.5", -3, 2),
    ] {
        assert_eq!(
            r(literal).to_rational(),
            (Int::from_i64(num), Nat::from_u64(den)),
            "{literal}"
        );
    }
}

#[test]
fn r7_conversion_negative_and_period_start_zero() {
    assert_eq!(
        r("-0.1(6)").to_rational(),
        (Int::from_i64(-1), Nat::from_u64(6))
    );
    assert_eq!(
        r("0.(6)").to_rational(),
        (Int::from_i64(2), Nat::from_u64(3))
    );
    assert_eq!(
        r("0.0(6)").to_rational(),
        (Int::from_i64(1), Nat::from_u64(15))
    );
    assert_eq!(
        r("0.123(45)").to_rational(),
        (Int::from_i64(679), Nat::from_u64(5500))
    );
}
