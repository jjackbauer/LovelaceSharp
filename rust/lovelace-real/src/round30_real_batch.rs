//! Round-30: the RealDivideExactnessPropertyTests row the round-29 exact-fraction fix unblocks.
//!
//! C# Lovelace.Real.Tests/RealDivideExactnessPropertyTests.cs:32 iterates the 10-value family
//! {1,2,3,5,6,7,11,13,17,23} over (a,b,c,d) and requires:
//!   (a/b)/(c/d) == (a/b)*(d/c) == (a*d)/(b*c), the same exactness, quotient exact,
//!   quotient*(c/d) == a/b, and (a/b)/(c/d) == (a/b)*(d/c).
//! The round-28 refusal was the mul path expanding the periodic twin with max(operand fractional
//! digits)+40 places (1/7 * 1/23 was truncated and inexact while 1/7 / 23 was exact). Round 29
//! moved both periodic paths onto the exact fraction; this test repeats the whole property.

use crate::Real;

fn of(n: i64) -> Real {
    Real::from_i64(n)
}

#[test]
fn division_and_its_multiplication_twin_agree_on_value_and_exactness() {
    const VALUES: [i64; 10] = [1, 2, 3, 5, 6, 7, 11, 13, 17, 23];
    for &a in &VALUES {
        for &b in &VALUES {
            for &c in &VALUES {
                for &d in &VALUES {
                    let left = of(a).div(&of(b), 1000).expect("b is nonzero");
                    let right = of(c).div(&of(d), 1000).expect("d is nonzero");
                    let inverse = of(d).div(&of(c), 1000).expect("c is nonzero");
                    let quotient = left.div(&right, 1000).expect("right is nonzero");
                    let twin = left.mul(&inverse);
                    let direct = of(a * d).div(&of(b * c), 1000).expect("b*c is nonzero");
                    let what = format!("({a}/{b})/({c}/{d})");

                    assert_eq!(
                        quotient, twin,
                        "{what} = {quotient} but the twin *{d}/{c} = {twin}"
                    );
                    assert_eq!(
                        quotient, direct,
                        "{what} = {quotient} but ({a}*{d})/({b}*{c}) = {direct}"
                    );
                    assert_eq!(
                        twin.is_exact(),
                        quotient.is_exact(),
                        "{what}: twin exact {} but quotient exact {}",
                        twin.is_exact(),
                        quotient.is_exact()
                    );
                    assert!(
                        quotient.is_exact(),
                        "{what} = {quotient} was reported inexact"
                    );
                    assert_eq!(
                        quotient.mul(&right),
                        left,
                        "{what} * ({c}/{d}) did not return {a}/{b}"
                    );
                    assert_eq!(
                        quotient,
                        left.mul(&inverse),
                        "{what} disagrees with multiplying by the inverse"
                    );
                }
            }
        }
    }
}
