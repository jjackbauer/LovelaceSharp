//! Round 24: the Complex value surface.
//!
//! The oracle does NOT accept a complex literal: 1+2i and 2i are parse refusals, i is an
//! undefined variable, and complex() takes zero arguments (the domain constant). The complex
//! VALUES the language produces come from the DSP builtins (fft/dft) and are bridged back to
//! Real by re/im/conj/abs. Every literal-shape expectation below is the exact envelope sentence
//! and position the dotnet oracle publishes; every value expectation is pinned to its rendering
//! and its structured projection. The library half pins the reference-value semantics of
//! Lovelace.Complex (operators, Parse/ToString, magnitude/reciprocal, fixed-width promotion).

use lovelace_json::Json;
use lovelace_real::Real;

use crate::complex::{Complex, LComplex64, LComplex128};
use crate::{Engine, LangError, Value};

fn eval(script: &str) -> Value {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message))
}

fn display(script: &str) -> String {
    eval(script).display()
}

fn eval_err(script: &str) -> LangError {
    Engine::new()
        .eval(script)
        .expect_err(&format!("{script} must be refused"))
}

fn structured(script: &str) -> Json {
    let mut engine = Engine::new();
    let value = engine
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message));
    crate::structured::project_value(&engine, &value, None)
}

fn field<'a>(json: &'a Json, key: &str) -> &'a Json {
    match json {
        Json::Obj(map) => map
            .get(key)
            .unwrap_or_else(|| panic!("missing field {key} in {}", json.to_string())),
        other => panic!("not an object: {}", other.to_string()),
    }
}

fn text(json: &Json) -> &str {
    match json {
        Json::Str(value) => value,
        other => panic!("not a string: {}", other.to_string()),
    }
}

fn elements(json: &Json) -> &[Json] {
    match field(json, "elements") {
        Json::Arr(items) => items,
        other => panic!("not an array: {}", other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Literal grammar: the oracle refuses every complex literal form
// ---------------------------------------------------------------------------

#[test]
fn a_juxtaposed_complex_literal_is_the_oracles_separator_refusal() {
    let error = eval_err("1+2i");
    assert_eq!(error.code, "ParseError");
    assert_eq!(error.category, "ParseError");
    assert_eq!(
        error.message,
        "Expected ';' or end of input but found 'i' at position 3."
    );
    assert_eq!(error.position, 3);
}

#[test]
fn a_pure_imaginary_literal_is_refused_at_its_own_position() {
    assert_eq!(
        eval_err("2i").message,
        "Expected ';' or end of input but found 'i' at position 1."
    );
    assert_eq!(
        eval_err("3i").message,
        "Expected ';' or end of input but found 'i' at position 1."
    );
}

#[test]
fn a_bare_i_is_an_undefined_variable_not_a_literal() {
    let error = eval_err("i");
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.message, "Undefined variable 'i'.");
}

#[test]
fn complex_forms_inside_parentheses_are_the_rparen_refusal() {
    assert_eq!(
        eval_err("(1+2i)*(3-4i)").message,
        "Expected 'RParen' but found 'i' at position 4."
    );
    assert_eq!(
        eval_err("abs(3+4i)").message,
        "Expected 'RParen' but found 'i' at position 7."
    );
    assert_eq!(
        eval_err("conj(1+2i)").message,
        "Expected 'RParen' but found 'i' at position 8."
    );
}

#[test]
fn complex_is_a_domain_constant_and_complex_takes_no_arguments() {
    assert_eq!(display("complex"), "complex");
    assert!(matches!(eval("complex"), Value::Domain(name) if name == "complex"));
    assert_eq!(display("type(complex)"), "Domain");
    assert_eq!(
        eval_err("complex(1, 2)").message,
        "complex(): expected 0 arguments; got 2."
    );
}

#[test]
fn adjacent_statements_need_a_separator_like_the_oracle() {
    assert_eq!(
        eval_err("1 2").message,
        "Expected ';' or end of input but found '2' at position 2."
    );
    assert_eq!(
        eval_err("x = 3; 2x").message,
        "Expected ';' or end of input but found 'x' at position 8."
    );
    assert_eq!(
        eval_err("2 pi").message,
        "Expected ';' or end of input but found 'pi' at position 2."
    );
    assert_eq!(
        eval_err("{1 2}").message,
        "Expected ';' or '}' but found '2' at position 3."
    );
    assert_eq!(
        eval_err("1 }").message,
        "Unexpected token '}' at position 2: expected a number, string, identifier, '[', or '('."
    );
}

// ---------------------------------------------------------------------------
// The DSP complex value surface
// ---------------------------------------------------------------------------

#[test]
fn fft_publishes_complex_elements_with_re_im_and_exactness() {
    assert_eq!(display("fft([1,2,3,4])"), "[10, -2 + 2i, -2, -2 - 2i]");
    let projection = structured("fft([1,2,3,4])");
    assert_eq!(text(field(&projection, "type")), "Vector");
    let items = elements(&projection);
    assert_eq!(items.len(), 4);
    for item in items {
        assert_eq!(text(field(item, "kind")), "Complex");
        assert_eq!(field(item, "exact"), &Json::Bool(true));
    }
    assert_eq!(text(field(&items[1], "re")), "-2");
    assert_eq!(text(field(&items[1], "im")), "2");
    assert_eq!(text(field(&items[3], "im")), "-2");
}

#[test]
fn re_im_and_conj_map_a_complex_signal_like_the_oracle() {
    assert_eq!(display("re(fft([1,2,3,4]))"), "[10, -2, -2, -2]");
    assert_eq!(display("im(fft([1,2,3,4]))"), "[0, 2, 0, -2]");
    assert_eq!(display("conj(fft([1,2,3,4]))"), "[10, -2 - 2i, -2, -2 + 2i]");
    let conj = structured("conj(fft([1,2,3,4]))");
    let items = elements(&conj);
    assert_eq!(text(field(&items[1], "im")), "-2");
    assert_eq!(text(field(&items[3], "im")), "2");
    // A real input is promoted through ToComplex too: re/im answer Real, conj Complex. Any
    // array rank is flattened to a rank-1 vector.
    assert_eq!(display("re([1,2])"), "[1, 2]");
    assert_eq!(display("im([1,2])"), "[0, 0]");
    assert_eq!(display("conj([1,2])"), "[1, 2]");
    let real = structured("re([1,2])");
    assert_eq!(text(field(&elements(&real)[0], "kind")), "Real");
    let conj = structured("conj([1,2])");
    assert_eq!(text(field(&elements(&conj)[0], "kind")), "Complex");
    assert_eq!(display("re(reshape([1,2,3,4],2,2))"), "[1, 2, 3, 4]");
}

#[test]
fn scalar_re_im_conj_promote_every_numeric_kind() {
    assert_eq!(display("re(5)"), "5");
    assert_eq!(display("im(5)"), "0");
    assert_eq!(display("conj(5)"), "5");
    let re = structured("re(5)");
    assert_eq!(text(field(&re, "kind")), "Real");
    let im = structured("im(5)");
    assert_eq!(text(field(&im, "kind")), "Real");
    let conj = structured("conj(5)");
    assert_eq!(text(field(&conj, "kind")), "Complex");
    assert_eq!(text(field(&conj, "re")), "5");
    assert_eq!(text(field(&conj, "im")), "0");
    assert_eq!(display("conj(fft([1,2,3,4])[1])"), "-2 - 2i");
}

#[test]
fn abs_of_a_complex_value_is_its_magnitude_at_the_ambient_precision() {
    assert_eq!(display("abs(fft([1,2])[0])"), "3");
    let projection = structured("abs(fft([1,2,3,4])[1])");
    assert_eq!(text(field(&projection, "kind")), "Real");
    assert_eq!(field(&projection, "exact"), &Json::Bool(false));
    let expected = Complex::new(Real::from_i64(-2), Real::from_i64(2)).magnitude(1000);
    assert_eq!(text(field(&projection, "value")), expected.to_string());
}

#[test]
fn abs_of_a_complex_array_is_the_magnitude_spectrum() {
    let projection = structured("abs(fft([1,2,3,4]))");
    assert_eq!(text(field(&projection, "type")), "Vector");
    let items = elements(&projection);
    assert_eq!(items.len(), 4);
    assert_eq!(text(field(&items[0], "value")), "10");
    assert_eq!(field(&items[0], "exact"), &Json::Bool(true));
    assert_eq!(text(field(&items[2], "value")), "2");
    // (-2 + 2i) has magnitude sqrt(8) at the default 1000-place budget, published inexact.
    let expected = Complex::new(Real::from_i64(-2), Real::from_i64(2)).magnitude(1000);
    assert_eq!(text(field(&items[1], "value")), expected.to_string());
    assert_eq!(field(&items[1], "exact"), &Json::Bool(false));
    assert_eq!(text(field(&items[1], "value")), text(field(&items[3], "value")));
}

#[test]
fn abs_of_a_non_complex_array_names_the_inferred_dtype() {
    assert_eq!(
        eval_err("abs([1,2])").message,
        "abs() on arrays requires a Complex array (got DType.Natural); use abs() on individual scalars for Natural/Integer/Real."
    );
    assert_eq!(
        eval_err("abs([1.5])").message,
        "abs() on arrays requires a Complex array (got DType.Real); use abs() on individual scalars for Natural/Integer/Real."
    );
}

#[test]
fn the_domain_constant_is_refused_by_the_complex_bridge() {
    assert_eq!(
        eval_err("conj(complex)").message,
        "DSP builtins expect numeric/complex elements, but got 'MathDomain'."
    );
    assert_eq!(
        eval_err("re(complex)").message,
        "DSP builtins expect numeric/complex elements, but got 'MathDomain'."
    );
}

// ---------------------------------------------------------------------------
// The arbitrary-precision Complex value type (Lovelace.Complex.Complex)
// ---------------------------------------------------------------------------

fn c(re: &str, im: &str) -> Complex {
    Complex::new(Real::from_str(re).unwrap(), Real::from_str(im).unwrap())
}

#[test]
fn complex_arithmetic_is_component_wise_and_exact() {
    let a = c("1", "2");
    let b = c("3", "4");
    assert_eq!(a.add(&b), c("4", "6"));
    assert_eq!(a.sub(&b), c("-2", "-2"));
    assert_eq!(a.mul(&b), c("-5", "10"));
    assert_eq!(Complex::i().mul(&Complex::i()), Complex::one().neg());
    assert_eq!(a.conjugate(), c("1", "-2"));
    assert_eq!(a.div(&c("1", "1")), Some(c("1.5", "0.5")));
    assert_eq!(a.mul(&a.reciprocal(1000).unwrap()), Complex::one());
}

#[test]
fn complex_magnitude_is_the_three_four_five_triangle() {
    let z = c("3", "4");
    assert_eq!(z.magnitude_squared(), Real::from_str("25").unwrap());
    assert_eq!(z.magnitude(1000), Real::from_str("5").unwrap());
}

#[test]
fn complex_parse_and_display_round_trip_the_reference_forms() {
    for (text, re, im) in [
        ("2", "2", "0"),
        ("4i", "0", "4"),
        ("-3i", "0", "-3"),
        ("i", "0", "1"),
        ("-i", "0", "-1"),
        ("1.5+0.5i", "1.5", "0.5"),
        ("1.5 - 0.5i", "1.5", "-0.5"),
        ("-2+3i", "-2", "3"),
    ] {
        let z = Complex::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"));
        assert_eq!(z, c(re, im), "{text}");
    }
    assert_eq!(c("2", "0").to_string(), "2");
    assert_eq!(c("0", "4").to_string(), "4i");
    assert_eq!(c("1.5", "0.5").to_string(), "1.5 + 0.5i");
    assert_eq!(c("1.5", "-0.5").to_string(), "1.5 - 0.5i");
    for text in ["2", "4i", "1.5 + 0.5i", "-2+3i"] {
        let z = Complex::parse(text).unwrap();
        assert_eq!(Complex::parse(&z.to_string()).unwrap(), z);
    }
    assert_eq!(Complex::parse("").unwrap_err(), "Input string was empty.");
}

#[test]
fn complex_exactness_is_both_parts() {
    assert!(c("1", "2").is_exact());
    let inexact = Complex::new(Real::from_str("1").unwrap().mark_inexact(), Real::from_i64(2));
    assert!(!inexact.is_exact());
    assert!(!Complex::pi(30).is_exact());
    assert_eq!(Complex::pi(30).im(), &Real::zero());
}

#[test]
fn limited_precision_takes_the_fixed_path_and_stays_exact() {
    let a = c("1", "2");
    let b = c("3", "4");
    assert_eq!(a.add_at(&b, 20), c("4", "6"));
    assert_eq!(a.sub_at(&b, 20), c("-2", "-2"));
    assert_eq!(a.mul_at(&b, 20), c("-5", "10"));
    // 10-digit x 10-digit overflows the 19-digit form but fits the 38-digit form: the fast path
    // promotes one width and still answers the exact product.
    let ten = c("9999999999", "0");
    assert_eq!(ten.mul_at(&ten, 20), c("99999999980000000001", "0"));
    // 37-digit x 37-digit overflows both fixed widths: the arbitrary-precision path answers the
    // exact 74-digit product (no rounding, no saturation).
    let wide = "9999999999999999999999999999999999999";
    let wide_c = Complex::new(Real::from_str(wide).unwrap(), Real::zero());
    let product = wide_c.mul_at(&wide_c, 37);
    assert_eq!(product.re().to_string().len(), 74);
    assert!(product.im().is_zero());
    assert_eq!(product.re().to_string(), "99999999999999999999999999999999999980000000000000000000000000000000000001");
}

#[test]
fn complex_integer_powers_are_exact() {
    let z = c("1", "2");
    assert_eq!(z.pow_int(0, 1000), Some(Complex::one()));
    assert_eq!(z.pow_int(2, 1000), Some(z.mul(&z)));
    assert_eq!(z.pow_int(-1, 1000), z.reciprocal(1000));
    assert_eq!(Complex::zero().pow_int(-1, 1000), None);
}

// ---------------------------------------------------------------------------
// Fixed-width complex forms (LComplex64 / LComplex128)
// ---------------------------------------------------------------------------

#[test]
fn lcomplex64_parses_adds_and_conjugates_like_the_reference() {
    let a = LComplex64::parse("1+2i").unwrap();
    let b = LComplex64::parse("3+4i").unwrap();
    assert_eq!(a.checked_add(&b).unwrap(), LComplex64::parse("4+6i").unwrap());
    let i = LComplex64::parse("i").unwrap();
    assert_eq!(i.checked_mul(&i).unwrap(), LComplex64::parse("-1").unwrap());
    assert_eq!(
        LComplex64::parse("2+3i").unwrap().conjugate(),
        LComplex64::parse("2-3i").unwrap()
    );
    for text in ["2", "4i", "1.5 + 0.5i", "-2+3i"] {
        let z = LComplex64::parse(text).unwrap();
        assert_eq!(LComplex64::parse(&z.to_string()).unwrap(), z, "{text}");
    }
}

#[test]
fn lcomplex64_reciprocal_multiplies_to_one() {
    let z = LComplex64::parse("1+2i").unwrap();
    assert_eq!(z.checked_mul(&z.reciprocal().unwrap()).unwrap(), LComplex64::parse("1").unwrap());
}

#[test]
fn lcomplex64_converts_to_and_from_the_arbitrary_precision_type() {
    let source = c("1.5", "-2.5");
    let narrow = LComplex64::try_from_complex(&source).expect("1.5 - 2.5i fits 19 digits");
    assert_eq!(narrow.to_complex(), source);
    let too_wide = c("99999999999999999999", "0");
    assert!(LComplex64::try_from_complex(&too_wide).is_none());
}

#[test]
fn lcomplex64_promotes_on_overflow_instead_of_rounding() {
    let z = LComplex64::parse("9999999999").unwrap();
    let error = z.checked_mul(&z).expect_err("the 20-digit square does not fit 19 digits");
    assert!(error.message().contains("more than 19 significant digits"));
    assert_eq!(LComplex64::MAX_SIGNIFICANT_DIGITS, 19);
}

#[test]
fn lcomplex128_covers_the_same_surface_at_38_digits() {
    let a = LComplex128::parse("1+2i").unwrap();
    let b = LComplex128::parse("3+4i").unwrap();
    assert_eq!(a.checked_add(&b).unwrap(), LComplex128::parse("4+6i").unwrap());
    let i = LComplex128::parse("i").unwrap();
    assert_eq!(i.checked_mul(&i).unwrap(), LComplex128::parse("-1").unwrap());
    assert_eq!(
        LComplex128::parse("2+3i").unwrap().conjugate(),
        LComplex128::parse("2-3i").unwrap()
    );
    let z = LComplex128::parse("1+2i").unwrap();
    assert_eq!(z.checked_mul(&z.reciprocal().unwrap()).unwrap(), LComplex128::parse("1").unwrap());
    let source = c("1.5", "-2.5");
    assert_eq!(LComplex128::try_from_complex(&source).unwrap().to_complex(), source);
    let too_wide = c("999999999999999999999999999999999999999", "0");
    assert!(LComplex128::try_from_complex(&too_wide).is_none());
    let overflow = LComplex128::parse("99999999999999999999").unwrap();
    assert!(overflow.checked_mul(&overflow).is_err());
    assert_eq!(LComplex128::MAX_SIGNIFICANT_DIGITS, 38);
}
