//! Writer tests: the exact .NET escaping and double notation the reference uses.

use lovelace_know::json::{Out, format_double};

#[test]
fn formats_doubles_like_dotnet() {
    // Every distinct weight spelling in the frozen reference must round trip exactly.
    let expected = [
        "0",
        "1",
        "17",
        "17.00016436554898",
        "32.00005478851632",
        "65",
        "83",
        "5.478851632697787E-05",
        "8.903133903133904E-05",
        "0.00010957703265395574",
        "0.00014467592592592592",
        "0.0001643655489809336",
        "0.00017806267806267807",
        "0.00021915406530791147",
        "0.0002670940170940171",
        "0.00028935185185185184",
        "0.00035612535612535614",
        "0.00044515669515669517",
    ];
    for literal in expected {
        let value: f64 = literal.parse().expect("the reference literal is a double");
        assert_eq!(format_double(value), literal, "formatting {literal}");
    }
}

#[test]
fn escapes_like_the_default_javascript_encoder() {
    let value =
        Out::str("err|Exponent must be positive. (Parameter 'exponent') has support 1 (< 2)");
    assert_eq!(
        value.compact(),
        "\"err|Exponent must be positive. (Parameter \\u0027exponent\\u0027) has support 1 (\\u003C 2)\""
    );
    assert_eq!(
        Out::str("a+b&c>d").compact(),
        "\"a\\u002Bb\\u0026c\\u003Ed\""
    );
    assert_eq!(Out::str("line\n\ttab").compact(), "\"line\\n\\ttab\"");
}

#[test]
fn renders_pretty_objects_with_two_spaces() {
    let value = Out::obj([
        ("seed", Out::Int(20_240_617)),
        (
            "naturalValues",
            Out::Arr(vec![Out::str("0"), Out::str("1")]),
        ),
        ("metrics", Out::Null),
    ]);
    let expected = concat!(
        "{\n",
        "  \"seed\": 20240617,\n",
        "  \"naturalValues\": [\n",
        "    \"0\",\n",
        "    \"1\"\n",
        "  ],\n",
        "  \"metrics\": null\n",
        "}"
    )
    .replace('\n', if cfg!(windows) { "\r\n" } else { "\n" });
    assert_eq!(value.pretty(), expected);
}

#[test]
fn empty_containers_stay_inline() {
    assert_eq!(Out::Arr(Vec::new()).pretty(), "[]");
    assert_eq!(Out::Obj(Vec::new()).pretty(), "{}");
}
