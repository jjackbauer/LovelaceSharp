//! Round 25: language-level evalf and ComplexMath.
//!
//! The Real probes are the oracle's \`evalf(expression, N)\` displays at 20, 50 and 100 places
//! (the same strings the Real-crate test pins), now through the full engine. The complex half pins
//! \`ComplexMath\` at 60 places against values dumped from the frozen C# \`Lovelace.Complex\`
//! assembly.

use lovelace_json::Json;
use lovelace_real::Real;

use crate::Engine;
use crate::complex::Complex;
use crate::complex_math as cm;

const CASES: [(&str, [&str; 3]); 12] = [
    (
        "exp(1)",
        [
            "2.71828182845904523536",
            "2.71828182845904523536028747135266249775724709369995",
            "2.7182818284590452353602874713526624977572470936999595749669676277240766303535475945713821785251664274",
        ],
    ),
    (
        "log(2)",
        [
            "0.69314718055994530941",
            "0.69314718055994530941723212145817656807550013436025",
            "0.6931471805599453094172321214581765680755001343602552541206800094933936219696947156058633269964186875",
        ],
    ),
    (
        "sin(1)",
        [
            "0.84147098480789650665",
            "0.84147098480789650665250232163029899962256306079837",
            "0.8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958",
        ],
    ),
    (
        "cos(1)",
        [
            "0.5403023058681397174",
            "0.54030230586813971740093660744297660373231042061792",
            "0.5403023058681397174009366074429766037323104206179222276700972553811003947744717645179518560871830893",
        ],
    ),
    (
        "tan(1)",
        [
            "1.5574077246549022305",
            "1.55740772465490223050697480745836017308725077238152",
            "1.5574077246549022305069748074583601730872507723815200383839466056988613971517272895550999652022429838",
        ],
    ),
    (
        "atan(1)",
        [
            "0.78539816339744830961",
            "0.78539816339744830961566084581987572104929234984377",
            "0.7853981633974483096156608458198757210492923498437764552437361480769541015715522496570087063355292669",
        ],
    ),
    (
        "sinh(1)",
        [
            "1.17520119364380145688",
            "1.17520119364380145688238185059560081515571798133409",
            "1.1752011936438014568823818505956008151557179813340958702295654130133075673043238956071174520896233918",
        ],
    ),
    (
        "cosh(1)",
        [
            "1.54308063481524377847",
            "1.54308063481524377847790562075706168260152911236586",
            "1.5430806348152437784779056207570616826015291123658637047374022147107690630492236989642647264355430355",
        ],
    ),
    (
        "tanh(1)",
        [
            "0.76159415595576488811",
            "0.76159415595576488811945828260479359041276859725793",
            "0.7615941559557648881194582826047935904127685972579365515968105001219532445766384834589475216736767144",
        ],
    ),
    (
        "exp(-2.5)",
        [
            "0.08208499862389879516",
            "0.08208499862389879516952867446715980783780412101543",
            "0.0820849986238987951695286744671598078378041210154366488457584105152247568804109713097515715212364659",
        ],
    ),
    (
        "log(10)",
        [
            "2.30258509299404568401",
            "2.30258509299404568401799145468436420760110148862877",
            "2.3025850929940456840179914546843642076011014886287729760333279009675726096773524802359972050895982983",
        ],
    ),
    (
        "sin(100)",
        [
            "-0.50636564110975879365",
            "-0.50636564110975879365655761045978543206503272129065",
            "-0.5063656411097587936565576104597854320650327212906573234433924735943579134194766964992366645129273922",
        ],
    ),
];

fn display(script: &str) -> String {
    Engine::new()
        .eval(script)
        .unwrap_or_else(|error| panic!("{script}: {}", error.message))
        .display()
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
            .unwrap_or_else(|| panic!("missing {key} in {}", json)),
        other => panic!("not an object: {}", other),
    }
}

fn exact_flag(json: &Json) -> bool {
    match field(json, "exact") {
        Json::Bool(value) => *value,
        other => panic!("not a bool: {}", other),
    }
}

#[test]
fn evalf_functions_match_the_oracle_digit_for_digit() {
    for (expression, expected) in CASES.iter() {
        for (index, digits) in [20usize, 50, 100].iter().enumerate() {
            let script = format!("evalf({expression}, {digits})");
            assert_eq!(display(&script), expected[index], "{script}");
        }
    }
}

#[test]
fn evalf_transcendental_values_are_inexact_on_the_wire() {
    for (expression, _) in CASES.iter() {
        for digits in [20usize, 50, 100] {
            let script = format!("evalf({expression}, {digits})");
            let value = structured(&script);
            assert!(!exact_flag(&value), "{script} claims exactness");
            match field(&value, "kind") {
                Json::Str(kind) => assert_eq!(kind, "Real", "{script}"),
                other => panic!("not a string: {}", other),
            }
        }
    }
}

#[test]
fn evalf_exact_specials_stay_exact_integers() {
    for (script, expected) in [
        ("evalf(exp(0), 20)", "1"),
        ("evalf(sin(0), 20)", "0"),
        ("evalf(cos(0), 20)", "1"),
        ("evalf(tan(0), 20)", "0"),
        ("evalf(log(1), 20)", "0"),
        ("evalf(atan(0), 20)", "0"),
    ] {
        let value = structured(script);
        assert!(exact_flag(&value), "{script} lost its exactness");
        match field(&value, "kind") {
            Json::Str(kind) => assert_eq!(kind, "Integer", "{script}"),
            other => panic!("not a string: {}", other),
        }
        assert_eq!(display(script), expected, "{script}");
    }
}

#[test]
fn evalf_reports_the_reference_domain_refusals() {
    for (script, code, category, message) in [
        (
            "evalf(asin(2), 20)",
            "InvalidArgument",
            "TypeMismatch",
            "AsinReal(x) requires |x| <= 1. (Parameter 'x')",
        ),
        (
            "evalf(acos(2), 20)",
            "InvalidArgument",
            "TypeMismatch",
            "AsinReal(x) requires |x| <= 1. (Parameter 'x')",
        ),
        (
            "evalf(acosh(0.5), 20)",
            "InvalidArgument",
            "TypeMismatch",
            "AcoshReal(x) requires x >= 1. (Parameter 'x')",
        ),
        (
            "evalf(atanh(1), 20)",
            "InvalidArgument",
            "TypeMismatch",
            "AtanhReal(x) requires |x| < 1. (Parameter 'x')",
        ),
        (
            "evalf(log(0), 20)",
            "EvaluationError",
            "DomainError",
            "Ln(x) requires x > 0. (Parameter 'x')",
        ),
    ] {
        let error = Engine::new().eval(script).expect_err(script);
        assert_eq!(error.code, code, "{script}");
        assert_eq!(error.category, category, "{script}");
        assert_eq!(error.message, message, "{script}");
    }
}

fn c(re: &str, im: &str) -> Complex {
    Complex::new(
        Real::from_str(re).unwrap_or_else(Real::zero),
        Real::from_str(im).unwrap_or_else(Real::zero),
    )
}

fn assert_component(actual: &Real, expected: &str, exact: bool, label: &str) {
    let expected = Real::from_str(expected).expect("oracle component");
    assert_eq!(actual, &expected, "{label}");
    assert_eq!(actual.is_exact(), exact, "{label} exactness");
}

#[test]
fn complex_math_matches_the_reference_at_sixty_places() {
    let z = c("1", "2");
    let exp = cm::exp(&z, 60);
    assert_component(
        exp.re(),
        "-1.131204383756813638431255255510794710628867995826525750217719",
        false,
        "exp(1+2i).re",
    );
    assert_component(
        exp.im(),
        "2.471726672004818927616930893551664532736190369241008184200758",
        false,
        "exp(1+2i).im",
    );
    let log = cm::log(&z, 60).expect("log");
    assert_component(
        log.re(),
        "0.804718956217050187300379666613093819762800677134258860956323",
        false,
        "log(1+2i).re",
    );
    assert_component(
        log.im(),
        "1.107148717794090503017065460178537040070047645401432646676539",
        false,
        "log(1+2i).im",
    );
    let sin = cm::sin(&z, 60);
    assert_component(
        sin.re(),
        "3.165778513216168146740734617191905538379110767891468932289324",
        false,
        "sin(1+2i).re",
    );
    assert_component(
        sin.im(),
        "1.9596010414216058970703520499893582784363201601845596588019",
        false,
        "sin(1+2i).im",
    );
    let cos = cm::cos(&z, 60);
    assert_component(
        cos.re(),
        "2.032723007019665529436343448499514263731990406638752381945223",
        false,
        "cos(1+2i).re",
    );
    assert_component(
        cos.im(),
        "-3.051897799151800057512115686895105452888437617733319644663374",
        false,
        "cos(1+2i).im",
    );
    let tan = cm::tan(&z, 60).expect("tan");
    assert_component(
        tan.re(),
        "0.033812826079896690284370559725287301640187669330755874366730",
        false,
        "tan(1+2i).re",
    );
    assert_component(
        tan.im(),
        "1.014793616146633568117054175417967614163217471123533242464957",
        false,
        "tan(1+2i).im",
    );
    let sinh = cm::sinh(&z, 60);
    assert_component(
        sinh.re(),
        "-0.48905625904129367358645456854851592115851088467859496570288",
        false,
        "sinh(1+2i).re",
    );
    assert_component(
        sinh.im(),
        "1.403119250622040588019490859767712944070947553407072439773952",
        false,
        "sinh(1+2i).im",
    );
    let cosh = cm::cosh(&z, 60);
    assert_component(
        cosh.re(),
        "-0.642148124715519964844800686962278789470357111147930784514838",
        false,
        "cosh(1+2i).re",
    );
    assert_component(
        cosh.im(),
        "1.068607421382778339597440033783951588665242815833935744426806",
        false,
        "cosh(1+2i).im",
    );
}

#[test]
fn complex_sqrt_and_pow_match_the_reference() {
    let root = cm::sqrt(&c("1", "2"), 60);
    assert_component(
        root.re(),
        "1.27201964951406896425242246173749149171560804184009624861664",
        false,
        "sqrt(1+2i).re",
    );
    assert_component(
        root.im(),
        "0.786151377757423286069558585842958929523122057837723237664901",
        false,
        "sqrt(1+2i).im",
    );
    let exact = cm::sqrt(&c("-4", "0"), 60);
    assert_component(exact.re(), "0", true, "sqrt(-4+0i).re");
    assert_component(exact.im(), "2", true, "sqrt(-4+0i).im");
    let pow = cm::pow(&c("1", "2"), &c("3", "-4"), 60).expect("pow");
    assert_component(
        pow.re(),
        "932.139194643221377037990521386002319202631311217932612164482002",
        false,
        "pow(1+2i,3-4i).re",
    );
    assert_component(
        pow.im(),
        "95.946533660342017314086982354678947782067994589733694956741179",
        false,
        "pow(1+2i,3-4i).im",
    );
    let root_two = cm::pow(&c("2", "0"), &c("0.5", "0"), 60).expect("sqrt(2)");
    assert_component(
        root_two.re(),
        "1.414213562373095048801688724209698078569671875376948073176679",
        false,
        "pow(2,0.5).re",
    );
    assert_component(root_two.im(), "0", false, "pow(2,0.5).im");
}

#[test]
fn complex_exact_specials_match_the_reference() {
    let zero = c("0", "0");
    let exp = cm::exp(&zero, 60);
    assert_component(exp.re(), "1", true, "exp(0).re");
    assert_component(exp.im(), "0", true, "exp(0).im");
    let sin = cm::sin(&zero, 60);
    assert_component(sin.re(), "0", true, "sin(0).re");
    assert_component(sin.im(), "0", true, "sin(0).im");
    let cos = cm::cos(&zero, 60);
    assert_component(cos.re(), "1", true, "cos(0).re");
    assert_component(cos.im(), "0", true, "cos(0).im");
    let tan = cm::tan(&zero, 60).expect("tan(0)");
    assert_component(tan.re(), "0", true, "tan(0).re");
    assert_component(tan.im(), "0", true, "tan(0).im");
    let sinh = cm::sinh(&zero, 60);
    assert_component(sinh.re(), "0", true, "sinh(0).re");
    assert_component(sinh.im(), "0", true, "sinh(0).im");
    let cosh = cm::cosh(&zero, 60);
    assert_component(cosh.re(), "1", true, "cosh(0).re");
    assert_component(cosh.im(), "0", true, "cosh(0).im");
    let log = cm::log(&c("1", "0"), 60).expect("log(1)");
    assert_component(log.re(), "0", false, "log(1).re");
    assert_component(log.im(), "0", true, "log(1).im");
    let root = cm::sqrt(&c("4", "0"), 60);
    assert_component(root.re(), "2", true, "sqrt(4).re");
    assert_component(root.im(), "0", true, "sqrt(4).im");
    let two = c("2", "0");
    let square = cm::pow(&two, &two, 60).expect("pow(2,2)");
    assert_component(
        square.re(),
        "3.999999999999999999999999999999999999999999999999999999999999",
        false,
        "pow(2,2).re",
    );
    assert_component(square.im(), "0", false, "pow(2,2).im");
}

#[test]
fn complex_exponential_inverts_the_principal_logarithm() {
    let z = c("1", "2");
    let back = cm::exp(&cm::log(&z, 70).expect("log"), 60);
    let tolerance = Real::from_str("0.000000000000000000000000000000000000000000000000000001")
        .expect("tolerance");
    assert!(
        back.re().sub(z.re()).abs().cmp(&tolerance) == std::cmp::Ordering::Less,
        "exp(log z).re = {}",
        back.re()
    );
    assert!(
        back.im().sub(z.im()).abs().cmp(&tolerance) == std::cmp::Ordering::Less,
        "exp(log z).im = {}",
        back.im()
    );
}
