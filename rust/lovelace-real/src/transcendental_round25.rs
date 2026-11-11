//! Round 25: the arbitrary-precision transcendental kernel.
//!
//! The expected strings are the frozen C# oracle's \`evalf(expression, N)\` displays at N = 20,
//! 50 and 100, captured from the protocol runner. \`evalf\` evaluates at N + 20 fractional places
//! (the reference's working-precision guard) and truncates the published value back to N, so the
//! unit test reproduces that shape: compute at N + 20, then \`truncate_frac\` to N.

use crate::transcendental::truncate_frac;
use crate::Real;

const NAMES: [&str; 12] = [
    "exp(1)",
    "log(2)",
    "sin(1)",
    "cos(1)",
    "tan(1)",
    "atan(1)",
    "sinh(1)",
    "cosh(1)",
    "tanh(1)",
    "exp(-2.5)",
    "log(10)",
    "sin(100)",
];

/// The oracle's published strings at 20, 50 and 100 decimal places.
const EXPECTED: [(&str, [&str; 3]); 12] = [
    ("exp(1)", [
        "2.71828182845904523536",
        "2.71828182845904523536028747135266249775724709369995",
        "2.7182818284590452353602874713526624977572470936999595749669676277240766303535475945713821785251664274",
    ]),
    ("log(2)", [
        "0.69314718055994530941",
        "0.69314718055994530941723212145817656807550013436025",
        "0.6931471805599453094172321214581765680755001343602552541206800094933936219696947156058633269964186875",
    ]),
    ("sin(1)", [
        "0.84147098480789650665",
        "0.84147098480789650665250232163029899962256306079837",
        "0.8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958",
    ]),
    ("cos(1)", [
        "0.5403023058681397174",
        "0.54030230586813971740093660744297660373231042061792",
        "0.5403023058681397174009366074429766037323104206179222276700972553811003947744717645179518560871830893",
    ]),
    ("tan(1)", [
        "1.5574077246549022305",
        "1.55740772465490223050697480745836017308725077238152",
        "1.5574077246549022305069748074583601730872507723815200383839466056988613971517272895550999652022429838",
    ]),
    ("atan(1)", [
        "0.78539816339744830961",
        "0.78539816339744830961566084581987572104929234984377",
        "0.7853981633974483096156608458198757210492923498437764552437361480769541015715522496570087063355292669",
    ]),
    ("sinh(1)", [
        "1.17520119364380145688",
        "1.17520119364380145688238185059560081515571798133409",
        "1.1752011936438014568823818505956008151557179813340958702295654130133075673043238956071174520896233918",
    ]),
    ("cosh(1)", [
        "1.54308063481524377847",
        "1.54308063481524377847790562075706168260152911236586",
        "1.5430806348152437784779056207570616826015291123658637047374022147107690630492236989642647264355430355",
    ]),
    ("tanh(1)", [
        "0.76159415595576488811",
        "0.76159415595576488811945828260479359041276859725793",
        "0.7615941559557648881194582826047935904127685972579365515968105001219532445766384834589475216736767144",
    ]),
    ("exp(-2.5)", [
        "0.08208499862389879516",
        "0.08208499862389879516952867446715980783780412101543",
        "0.0820849986238987951695286744671598078378041210154366488457584105152247568804109713097515715212364659",
    ]),
    ("log(10)", [
        "2.30258509299404568401",
        "2.30258509299404568401799145468436420760110148862877",
        "2.3025850929940456840179914546843642076011014886287729760333279009675726096773524802359972050895982983",
    ]),
    ("sin(100)", [
        "-0.50636564110975879365",
        "-0.50636564110975879365655761045978543206503272129065",
        "-0.5063656411097587936565576104597854320650327212906573234433924735943579134194766964992366645129273922",
    ]),
];

fn value(name: &str, places: usize) -> Real {
    match name {
        "exp(1)" => Real::from_i64(1).exp(places),
        "log(2)" => Real::from_i64(2).ln(places).expect("log(2)"),
        "sin(1)" => Real::from_i64(1).sin(places),
        "cos(1)" => Real::from_i64(1).cos(places),
        "tan(1)" => Real::from_i64(1).tan(places).expect("tan(1)"),
        "atan(1)" => Real::from_i64(1).atan(places),
        "sinh(1)" => Real::from_i64(1).sinh(places),
        "cosh(1)" => Real::from_i64(1).cosh(places),
        "tanh(1)" => Real::from_i64(1).tanh(places),
        "exp(-2.5)" => Real::from_str("-2.5").expect("-2.5").exp(places),
        "log(10)" => Real::from_i64(10).ln(places).expect("log(10)"),
        "sin(100)" => Real::from_i64(100).sin(places),
        _ => unreachable!(),
    }
}

#[test]
fn evalf_probes_match_the_oracle_digit_for_digit() {
    for (name, expected) in EXPECTED.iter() {
        for (index, digits) in [20usize, 50, 100].iter().enumerate() {
            let published = truncate_frac(&value(name, digits + 20), *digits);
            assert_eq!(
                published.to_string(),
                expected[index],
                "{name} at {digits} places"
            );
        }
    }
}

#[test]
fn every_probe_is_an_inexact_value() {
    for (name, _) in EXPECTED.iter() {
        for digits in [20usize, 50, 100] {
            let published = truncate_frac(&value(name, digits + 20), digits);
            assert!(!published.is_exact(), "{name} at {digits} claims exactness");
        }
    }
}

#[test]
fn exp_of_an_exact_zero_is_one_and_keeps_the_argument_provenance() {
    let exact = Real::zero();
    assert_eq!(exact.exp(30).to_string(), "1");
    assert!(exact.exp(30).is_exact());
    let inexact = Real::zero().mark_inexact();
    let value = inexact.exp(30);
    assert_eq!(value.to_string(), "1");
    assert!(!value.is_exact(), "exp(inexact zero) claims exactness");
}

#[test]
fn sin_and_cos_of_zero_are_the_exact_table_values() {
    let zero = Real::zero();
    assert!(zero.sin(40).is_zero());
    assert!(zero.sin(40).is_exact());
    assert_eq!(zero.cos(40).to_string(), "1");
    assert!(zero.cos(40).is_exact());
    assert!(zero.tan(40).expect("tan(0)").is_zero());
    assert!(zero.tan(40).expect("tan(0)").is_exact());
}

#[test]
fn logarithm_and_inverse_trigonometry_keep_their_domains() {
    assert!(Real::zero().ln(30).is_none());
    assert!(Real::from_i64(-1).ln(30).is_none());
    assert!(Real::from_i64(2).asin(30).is_none());
    assert!(Real::from_i64(-2).acos(30).is_none());
    assert!(Real::from_str("0.5").expect("0.5").acosh(30).is_none());
    assert!(Real::from_i64(1).atanh(30).is_none());
    assert!(Real::from_i64(-1).atanh(30).is_none());
    assert!(Real::from_i64(1).acosh(30).is_some());
}

#[test]
fn exp_and_log_invert_each_other_at_a_hundred_places() {
    let log_two = Real::from_i64(2).ln(120).expect("log(2)");
    let back = log_two.exp(100);
    let tolerance = Real::from_str("0.0000000000000000000000000000000000000001").expect("tolerance");
    assert!(
        back.sub(&Real::from_i64(2)).abs().cmp(&tolerance) == std::cmp::Ordering::Less,
        "exp(log(2)) = {}",
        back.to_string()
    );
}

#[test]
fn atan_of_one_is_quarter_pi_at_a_hundred_places() {
    let atan = Real::from_i64(1).atan(100);
    let quarter = Real::pi(100).div(&Real::from_i64(4), 100).expect("pi/4");
    assert_eq!(atan.to_string(), truncate_frac(&quarter, 100).to_string());
}

#[test]
fn the_hyperbolic_identities_hold_at_fifty_places() {
    let one = Real::from_i64(1);
    let cosh = one.cosh(50);
    let sinh = one.sinh(50);
    let identity = cosh.mul(&cosh).sub(&sinh.mul(&sinh));
    let tolerance = Real::from_str("0.000000000000000000000000000000000000000000001").expect("tol");
    assert!(
        identity.sub(&Real::from_i64(1)).abs().cmp(&tolerance) == std::cmp::Ordering::Less,
        "cosh^2 - sinh^2 = {}",
        identity.to_string()
    );
    let tanh = one.tanh(50);
    assert!(
        tanh.sub(&sinh.div(&cosh, 50).expect("sinh/cosh")).abs().is_zero(),
        "tanh != sinh/cosh"
    );
}
