//! Round 95 numeric singletons: the four Integer-API rows that were deferred as
//! library-only. Each test restates the C# row's observable against the Rust API.

use crate::Int;

/// C# IntegerConstructorTests.Constructor_GivenInt_DelegatesToLongOverload:
/// `new Integer(-7)` and `new Integer(-7L)` format identically and agree on sign.
#[test]
fn constructor_given_int_delegates_to_long_overload() {
    let from_int = Int::from(-7i32);
    let from_long = Int::from(-7i64);
    assert_eq!(from_long.to_decimal(), from_int.to_decimal());
    assert_eq!(from_long.is_negative(), from_int.is_negative());
    assert_eq!(from_long, from_int);
}

/// C# IntegerEqualityTests.Equals_GivenReferenceToSelf_ReturnsTrue: `x.Equals(x)` is true.
#[test]
fn equals_given_reference_to_self_returns_true() {
    let value = Int::from(7i64);
    let reference = &value;
    assert!(reference.eq(reference));
    assert!(value.eq(&value));
}

/// C# IntegerMultiplyTests.OperatorStar_ProducesSameResultAsMultiplyMethod:
/// `a.Multiply(b)` and `a * b` are the same product, 6 * -7 = -42.
#[test]
fn operator_star_produces_same_result_as_multiply_method() {
    let a = Int::from(6i64);
    let b = Int::from(-7i64);
    let product = &a * &b;
    assert_eq!(a.mul(&b), product);
    assert_eq!(Int::from(-42i64), product);
    assert_eq!("-42", product.to_decimal());
}

/// C# IntegerSubtractTests.SubtractOperator_IsEquivalentToSubtractMethod:
/// `a.Subtract(b)` and `a - b` are the same difference, 100 - 37 = 63.
#[test]
fn subtract_operator_is_equivalent_to_subtract_method() {
    let a = Int::from(100i64);
    let b = Int::from(37i64);
    let difference = &a - &b;
    assert_eq!(a.sub(&b), difference);
    assert_eq!(Int::from(63i64), difference);
}
