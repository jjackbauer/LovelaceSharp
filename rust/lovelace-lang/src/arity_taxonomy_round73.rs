//! Round 73: the arity-taxonomy family — the five deferred census rows where the reference
//! classifies an argument-count (or argument-shape) mistake with a specific code, category,
//! message, position and signature, and this tree did not.
//!
//! The C# assertion (cited per test) is the contract. Every builtin's declared argument-count
//! range travels on its descriptor (\`MinArity\`, \`Parameters.Count\`, \`Variadic\`); the call-site
//! validator in \`builtins::call\` now enforces that one declaration before any body runs, and a
//! wrong count crosses as the documented recoverable argument error:
//! \`InvalidArgument\`/\`TypeMismatch\`, exit 1 on the wire, \`recoverable: true\`, the exact sentence
//! \`<name>(): expected <range>; got <n>.\` and the \`signature\` the call was measured against.
//!
//! The five rows:
//! * \`Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0\`
//! * \`Lovelace.Run.Tests/Lovelace.Run.Tests.ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0\`
//! * \`Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0\`
//! * \`Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0\`
//! * \`Lovelace.Run.Tests/Lovelace.Run.Tests.CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0\`

use crate::{Engine, LangError, Value, registry};

fn eval(source: &str) -> Result<Value, LangError> {
    Engine::new().eval(source)
}

fn refusal(source: &str) -> LangError {
    match eval(source) {
        Ok(value) => panic!("'{source}': expected a refusal, got {value:?}"),
        Err(error) => error,
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "argument" } else { "arguments" }
}

/// The one documented arity sentence (\`ArityPropertyTests.ExpectedMessage\`).
fn expected_arity_message(
    name: &str,
    min: usize,
    max: usize,
    variadic: bool,
    actual: usize,
) -> String {
    let expected = if variadic {
        format!("at least {} {}", min, plural(min))
    } else if min == max {
        format!("{} {}", min, plural(min))
    } else {
        format!("{min} to {max} arguments")
    };
    format!("{name}(): expected {expected}; got {actual}.")
}

/// The position in \`<builtin>(): argument <n> ... ; got ... .\` when the sentence is the
/// documented argument-error grammar (the C# \`DocumentedArgumentError\` regex). The reference
/// requirement is: names the builtin, the 1-based position, an expectation and what arrived.
fn documented_argument_position(name: &str, message: &str) -> Option<usize> {
    let rest = message.strip_prefix(&format!("{name}(): argument "))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let tail = &rest[digits.len()..];
    if tail.contains(';') && tail.contains("; got ") && tail.ends_with('.') {
        digits.parse().ok()
    } else {
        None
    }
}

/// \`ArityPropertyTests.cs:102\`: every registered builtin, every count in \`0..parameters+2\`.
/// A count OUTSIDE \`[MinArity, Parameters.Count]\` (the upper bound removed when Variadic) is the
/// one documented arity refusal; a count INSIDE it is never refused as a wrong-arity call. The
/// only builtin whose declaration covers every probed count is \`print\` (MinArity 0, Variadic).
#[test]
fn every_builtin_refuses_every_count_outside_its_declared_arity() {
    assert!(
        registry::BUILTINS.len() >= 100,
        "the live registry exposed {} builtins",
        registry::BUILTINS.len()
    );

    let mut violations: Vec<String> = Vec::new();
    let mut no_wrong_arity_exists: Vec<&str> = Vec::new();
    let mut probed = 0usize;

    for info in registry::BUILTINS {
        let min = info.min_arity;
        let max = info.parameter_count;
        let mut refusals = 0usize;

        for count in 0..=max + 2 {
            probed += 1;
            let args = vec!["1"; count].join(", ");
            let source = format!("{}({args})", info.name);
            let outside = count < min || (!info.variadic && count > max);
            match eval(&source) {
                Ok(value) => {
                    if outside {
                        violations.push(format!(
                            "{}: {count} argument(s) is OUTSIDE the declared range [{min}, {max}] but the call evaluated: {value:?}",
                            info.name
                        ));
                    }
                }
                Err(error) => {
                    let arity_shaped = error.code == "InvalidArgument"
                        && error
                            .message
                            .starts_with(&format!("{}(): expected ", info.name));
                    if outside {
                        let expected =
                            expected_arity_message(info.name, min, max, info.variadic, count);
                        if !arity_shaped
                            || error.category != "TypeMismatch"
                            || error.message != expected
                        {
                            violations.push(format!(
                                "{}: {count} argument(s) is OUTSIDE [{min}, {max}] but answered {}/{} '{}' (expected InvalidArgument/TypeMismatch '{expected}')",
                                info.name, error.code, error.category, error.message
                            ));
                        } else {
                            refusals += 1;
                        }
                    } else if arity_shaped {
                        violations.push(format!(
                            "{}: {count} argument(s) is INSIDE [{min}, {max}] but the call was refused as a wrong-arity call: '{}'",
                            info.name, error.message
                        ));
                    }
                }
            }
        }

        if refusals == 0 {
            no_wrong_arity_exists.push(info.name);
        }
    }

    assert!(
        violations.is_empty(),
        "{} violation(s) across {} builtins ({probed} probes):\n{}",
        violations.len(),
        registry::BUILTINS.len(),
        violations.join("\n")
    );
    assert!(probed >= 500, "the property drove only {probed} probes");
    // print is the one declaration that covers every count (0..n values followed by a newline):
    // MinArity 0 + Variadic. Any other entry here would mean a declaration was widened to dodge
    // the validator (the C# row asserts exactly this list).
    assert_eq!(no_wrong_arity_exists, vec!["print"]);
}

/// \`ArityValidationTests.cs:26\`: the too-few call is a typed STRUCTURAL error with the exact
/// reference sentence and the declared signature, never an internal failure.
#[test]
fn too_few_arguments_is_a_typed_structural_error() {
    let error = refusal("x = symbol(\"x\"); compile_full(x^2 + 1)");

    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
    assert_eq!(
        error.message,
        "compile_full(): expected 2 arguments; got 1."
    );
    // The call site is the second statement: 'x = symbol("x"); ' is 17 UTF-16 units.
    assert_eq!(error.position, 17);

    let signature = error
        .signature
        .expect("an arity refusal carries its signature");
    assert_eq!(signature.function, "compile_full");
    let expected_parameters = vec!["f".to_string(), "params".to_string()];
    assert_eq!(
        signature.parameters.as_deref(),
        Some(expected_parameters.as_slice())
    );
    assert_eq!(signature.expected_min, Some(2));
    assert_eq!(signature.expected_max, Some(2));
    assert_eq!(signature.variadic, Some(false));
    assert_eq!(signature.actual, Some(1));
}

/// \`BuiltinArgumentShapeSweepTests.cs:112\`: every wrong-shaped argument at every declared
/// position, over the three audit shapes. The unconditional rules: a caller-side mistake is never
/// an internal invariant failure, and every \`InvalidArgument\` refusal names the builtin and a
/// position inside the declaration. The two calls the deferred family measured are pinned to the
/// reference's exact classification: \`min(1, 1)\` is the argument-1 shape refusal (the reference
/// refuses immediately rather than reducing a scalar), and \`compile_full(x, x)\` is the argument-2
/// list refusal (a scalar is not a parameter list).
#[test]
fn every_wrong_shaped_argument_is_a_typed_refusal_naming_the_position() {
    const SHAPES: [(&str, &str); 3] = [
        ("a bare scalar", "1"),
        ("a four-element vector", "[1, 2, 3, 4]"),
        ("a symbolic value", "x"),
    ];

    let mut violations: Vec<String> = Vec::new();
    let mut probes = 0usize;
    let mut refusals = 0usize;
    let mut argument_refusals = 0usize;

    for info in registry::BUILTINS {
        if info.parameter_count == 0 {
            continue; // no argument position exists to give the wrong shape to
        }
        for (shape, literal) in SHAPES {
            probes += 1;
            let call = format!(
                "{}({})",
                info.name,
                vec![literal; info.parameter_count].join(", ")
            );
            let source = format!("x = symbol(\"x\"); {call}");
            let error = match eval(&source) {
                Ok(_) => continue, // a shape this builtin genuinely accepts
                Err(error) => error,
            };
            refusals += 1;
            if error.code == "InternalError" || error.category == "InternalInvariantFailure" {
                violations.push(format!(
                    "{call} [{shape}]: an INTERNAL failure ({}/{}: '{}')",
                    error.code, error.category, error.message
                ));
                continue;
            }
            if error.code != "InvalidArgument" {
                continue; // a domain refusal: a different, already-classified family
            }
            argument_refusals += 1;
            if error.category != "TypeMismatch" {
                violations.push(format!(
                    "{call} [{shape}]: an argument refusal must carry TypeMismatch, got {}",
                    error.category
                ));
                continue;
            }
            match documented_argument_position(info.name, &error.message) {
                None => violations.push(format!(
                    "{call} [{shape}]: an argument refusal must name the builtin, the 1-based position, the expectation and what arrived ('{}')",
                    error.message
                )),
                Some(position) if position == 0 || position > info.parameter_count => {
                    violations.push(format!(
                        "{call} [{shape}]: the refusal names argument {position}, outside the {} declared position(s) ('{}')",
                        info.parameter_count, error.message
                    ))
                }
                Some(_) => {}
            }
        }
    }

    assert!(probes >= 300, "the sweep drove only {probes} probes");
    assert!(
        refusals >= 1,
        "no probe was refused, so the sweep asserts nothing"
    );
    assert!(
        argument_refusals >= 1,
        "none of the {refusals} refusals used the documented argument-error family"
    );
    assert!(
        violations.is_empty(),
        "{} of {refusals} refusals ({probes} probes) violate the argument-shape contract:\n{}",
        violations.len(),
        violations.join("\n")
    );

    // The reference classifications this family measured, pinned exactly.
    let min_error = refusal("min(1, 1)");
    assert_eq!(min_error.code, "InvalidArgument");
    assert_eq!(min_error.category, "TypeMismatch");
    assert_eq!(
        min_error.message,
        "min(): argument 1 must be an array or vector; got Natural."
    );
    let signature = min_error
        .signature
        .expect("the shape refusal carries its signature");
    assert_eq!(signature.position, Some(1));
    assert_eq!(signature.parameter.as_deref(), Some("a"));
    assert_eq!(
        signature.expected_shape.as_deref(),
        Some("an array or vector")
    );
    assert_eq!(signature.arrived_kind.as_deref(), Some("Natural"));

    let compile_error = refusal("x = symbol(\"x\"); compile_full(x, x)");
    assert_eq!(compile_error.code, "InvalidOperation");
    assert_eq!(compile_error.category, "DomainError");
    assert_eq!(
        compile_error.message,
        "compile_full(): argument 2 must be a list of parameter symbols, e.g. [x]; got Symbolic."
    );
    assert_eq!(compile_error.position, 17);
}

/// \`BuiltinArgumentShapeSweepTests.cs:228\`: the same unconditional rules over the wider lattice
/// (scalar, vector, symbolic, record, matrix, text, Boolean, empty vector). The two wide-shape
/// classifications the family touches are pinned exactly: a Record handed to a reduction is the
/// argument-1 array/vector refusal, and a Text handed to \`compile_full\` is the symbolic-expression
/// coercion's own DomainError sentence.
#[test]
fn every_wrong_shaped_argument_over_the_wider_lattice_is_still_typed() {
    const SHAPES: [(&str, &str); 8] = [
        ("a bare scalar", "1"),
        ("a four-element vector", "[1, 2, 3, 4]"),
        ("a symbolic value", "x"),
        ("a record", "inspect(1)"),
        ("a 2x2 matrix", "[[1, 2], [3, 4]]"),
        ("a text value", "\"x\""),
        ("a Boolean", "1 == 1"),
        ("an empty vector", "[]"),
    ];
    const RAW_FRAMEWORK: [&str; 6] = [
        "cast is not valid",
        "Unable to cast",
        "Object reference not set",
        "Index was outside the bounds",
        "Value cannot be null",
        "was not in a correct format",
    ];

    let mut violations: Vec<String> = Vec::new();
    let mut probes = 0usize;

    for info in registry::BUILTINS {
        if info.parameter_count == 0 {
            continue;
        }
        for (shape, literal) in SHAPES {
            probes += 1;
            let call = format!(
                "{}({})",
                info.name,
                vec![literal; info.parameter_count].join(", ")
            );
            let source = format!("x = symbol(\"x\"); {call}");
            let error = match eval(&source) {
                Ok(_) => continue,
                Err(error) => error,
            };
            if error.code == "InternalError" || error.category == "InternalInvariantFailure" {
                violations.push(format!(
                    "{call} [{shape}]: an INTERNAL failure ({}/{}: '{}')",
                    error.code, error.category, error.message
                ));
            } else if RAW_FRAMEWORK.iter().any(|phrase| {
                error
                    .message
                    .to_ascii_lowercase()
                    .contains(&phrase.to_ascii_lowercase())
            }) {
                violations.push(format!(
                    "{call} [{shape}]: the refusal carries raw framework text ('{}')",
                    error.message
                ));
            }
        }
    }

    assert!(probes >= 800, "the wide sweep drove only {probes} probes");
    assert!(
        violations.is_empty(),
        "{} of {probes} probes are not typed refusals:\n{}",
        violations.len(),
        violations.join("\n")
    );

    let min_error = refusal("min(inspect(1), inspect(1))");
    assert_eq!(min_error.code, "InvalidArgument");
    assert_eq!(min_error.category, "TypeMismatch");
    assert_eq!(
        min_error.message,
        "min(): argument 1 must be an array or vector; got Record."
    );

    let compile_error = refusal("compile_full(\"x\", \"x\")");
    assert_eq!(compile_error.code, "InvalidOperation");
    assert_eq!(compile_error.category, "DomainError");
    assert_eq!(compile_error.message, "Expected a symbolic expression.");
}

/// \`CallSignatureDiagnosticsTests.cs:78\`: for every probe, the arity refusal's published counts
/// agree with what the registry declares for the same name — two surfaces, one declaration.
#[test]
fn every_arity_refusal_agrees_with_the_registry_entry() {
    // The C# row's own probes; each supplies FEWER arguments than the declared minimum.
    let probes = [
        "matmul([1, 2])",
        "cross([1, 2, 3])",
        "append([1, 2])",
        "conv([1, 2])",
        "filter([1], [1])",
    ];

    for source in probes {
        let name = source.split('(').next().expect("a call");
        let info = registry::find(name).expect("the probe names a registered builtin");
        let error = refusal(source);

        assert_eq!(error.code, "InvalidArgument", "{source}");
        assert_eq!(error.category, "TypeMismatch", "{source}");

        let signature = error
            .signature
            .expect("an arity refusal carries the declared signature");
        assert_eq!(signature.function, name, "{source}");
        assert_eq!(signature.expected_min, Some(info.min_arity), "{source}");
        assert_eq!(
            signature.expected_max,
            if info.variadic {
                None
            } else {
                Some(info.parameter_count)
            },
            "{source}"
        );
        assert_eq!(signature.variadic, Some(info.variadic), "{source}");
        let actual = signature.actual.expect("the actual count is published");
        assert!(
            actual < info.min_arity,
            "{source}: the probe must supply FEWER arguments than the declared minimum"
        );
        // The message is the same declaration rendered, and the signature's parameter list is
        // the registry's.
        assert_eq!(
            error.message,
            expected_arity_message(
                name,
                info.min_arity,
                info.parameter_count,
                info.variadic,
                actual
            ),
            "{source}"
        );
        let expected_parameters: Vec<String> = info
            .parameters
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        assert_eq!(
            signature.parameters.as_deref(),
            Some(expected_parameters.as_slice()),
            "{source}"
        );
    }
}
