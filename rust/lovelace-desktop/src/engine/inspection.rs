//! SYM-001: the real engine adapter's symbolic inspection capture.
//!
//! Everything here is read from the engine, and every section names its engine source:
//!
//! - pretty / canonical: `lovelace_lang::structured::project_value` of the symbolic subject;
//! - expression tree: the engine's own `lovelace_symbolics::Expr` carried by `Value::Symbolic`
//!   (node kinds are the enum's variants, children are its child order, labels are the engine's
//!   pretty rendering - no display text is parsed);
//! - assumptions: the engine's `inspect(value)` builtin, through `lovelace_lang::builtins::call`,
//!   plus a `TransformResult`'s `conditions` field;
//! - trace: a `TransformResult`'s `steps` (the record `simplify_full` publishes);
//! - MathIR: `lovelace_lang::mathir::compile_ordered`, the same compile path `compile`/
//!   `compile_full` use, over the subject and its free symbols.
//!
//! The adapter is only compiled with the `real-engine` feature, so the fake engine and the
//! `--no-default-features` build stay engine-free.

use lovelace_json::Json;
use lovelace_lang::Value;
use lovelace_symbolics::{Expr, RelOp};

/// Captures the inspection payload for one evaluated value.
///
/// Always returns a payload: a non-symbolic result gets six specific "not supplied" reasons
/// instead of an empty object.
pub fn capture(engine: &mut lovelace_lang::Engine, value: &Value) -> Json {
    let subject = Subject::of(value);
    let Some(expression) = subject.expression else {
        return missing_payload(&subject);
    };
    let project = lovelace_lang::structured::project_value(
        engine,
        &Value::Symbolic(expression.clone()),
        None,
    );
    let pretty = text_or_reason(
        project.get("pretty").and_then(Json::as_str),
        "The engine's structured projection of this Symbolic value has no pretty field.",
    );
    let canonical = text_or_reason(
        project.get("canonical").and_then(Json::as_str),
        "The engine's structured projection of this Symbolic value has no canonical field.",
    );
    let tree = Json::obj([
        ("nodes", Json::Num(expression.node_count().to_string())),
        ("root", tree_node(expression)),
    ]);
    let assumptions = assumptions_section(engine, expression, &subject.conditions);
    let trace = if subject.is_transform {
        trace_section(&subject.steps)
    } else {
        Json::obj([(
            "reason",
            Json::Str(
                "The engine publishes rewrite steps only in a TransformResult (simplify_full). \
                 The last committed result is a plain Symbolic value, so it has no simplification \
                 trace."
                    .to_string(),
            ),
        )])
    };
    let mathir = mathir_section(expression);
    sections_payload(
        &subject,
        pretty,
        canonical,
        tree,
        assumptions,
        trace,
        mathir,
    )
}

/// The subject the inspector is bound to: a Symbolic value, or the `expression` field of a
/// `TransformResult` (the only record that also carries a trace).
struct Subject<'a> {
    kind: String,
    path: &'static str,
    detail: Option<String>,
    expression: Option<&'a Expr>,
    is_transform: bool,
    conditions: Vec<&'a Value>,
    steps: Vec<&'a Value>,
}

impl<'a> Subject<'a> {
    fn of(value: &'a Value) -> Subject<'a> {
        match value {
            Value::Symbolic(expression) => Subject {
                kind: "Symbolic".to_string(),
                path: "result",
                detail: None,
                expression: Some(expression),
                is_transform: false,
                conditions: Vec::new(),
                steps: Vec::new(),
            },
            Value::Record(type_name, fields) if type_name == "TransformResult" => {
                let expression = record_field(fields, "expression").and_then(|value| match value {
                    Value::Symbolic(expression) => Some(expression),
                    _ => None,
                });
                let detail = record_field(fields, "status")
                    .and_then(enum_value)
                    .map(str::to_string);
                Subject {
                    kind: type_name.clone(),
                    path: "result.expression",
                    detail,
                    expression,
                    is_transform: true,
                    conditions: record_list(fields, "conditions"),
                    steps: record_list(fields, "steps"),
                }
            }
            Value::Record(type_name, _) => Subject {
                kind: type_name.clone(),
                path: "result",
                detail: None,
                expression: None,
                is_transform: false,
                conditions: Vec::new(),
                steps: Vec::new(),
            },
            other => Subject {
                kind: other.kind_name().to_string(),
                path: "result",
                detail: None,
                expression: None,
                is_transform: false,
                conditions: Vec::new(),
                steps: Vec::new(),
            },
        }
    }
}

fn subject_json(subject: &Subject<'_>) -> Json {
    Json::obj([
        ("kind", Json::Str(subject.kind.clone())),
        ("path", Json::Str(subject.path.to_string())),
        (
            "detail",
            subject.detail.clone().map(Json::Str).unwrap_or(Json::Null),
        ),
    ])
}

fn sections_payload(
    subject: &Subject<'_>,
    pretty: Json,
    canonical: Json,
    tree: Json,
    assumptions: Json,
    trace: Json,
    mathir: Json,
) -> Json {
    Json::obj([
        ("subject", subject_json(subject)),
        (
            "sections",
            Json::obj([
                ("pretty", pretty),
                ("canonical", canonical),
                ("tree", tree),
                ("assumptions", assumptions),
                ("trace", trace),
                ("mathir", mathir),
            ]),
        ),
    ])
}

/// A payload for a result that is not symbolic: each of the six sections says why it is absent.
fn missing_payload(subject: &Subject<'_>) -> Json {
    let kind = &subject.kind;
    let sections = Json::obj([
        (
            "pretty",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine publishes pretty for a Symbolic value or a TransformResult's \
                     expression. The last committed result is {kind}."
                )),
            )]),
        ),
        (
            "canonical",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine publishes canonical for a Symbolic value or a TransformResult's \
                     expression. The last committed result is {kind}."
                )),
            )]),
        ),
        (
            "tree",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine carries an expression tree only for a symbolic subject. The last \
                     committed result is {kind}."
                )),
            )]),
        ),
        (
            "assumptions",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine's inspect path publishes assumptions for a symbolic subject. The \
                     last committed result is {kind}."
                )),
            )]),
        ),
        (
            "trace",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine publishes rewrite steps only in a TransformResult. The last \
                     committed result is {kind}."
                )),
            )]),
        ),
        (
            "mathir",
            Json::obj([(
                "reason",
                Json::Str(format!(
                    "The engine's compile path needs a symbolic subject. The last committed \
                     result is {kind}."
                )),
            )]),
        ),
    ]);
    Json::obj([("subject", subject_json(subject)), ("sections", sections)])
}

fn text_or_reason(text: Option<&str>, reason: &str) -> Json {
    match text {
        Some(text) => Json::obj([("text", Json::Str(text.to_string()))]),
        None => Json::obj([("reason", Json::Str(reason.to_string()))]),
    }
}

/// The engine's expression tree: node kind and children come from the `Expr` variants, the label
/// is the engine's pretty rendering of that sub-expression.
fn tree_node(expression: &Expr) -> Json {
    let (kind, detail, children): (&str, Option<String>, Vec<Json>) = match expression {
        Expr::Num(_) => ("Num", None, Vec::new()),
        Expr::Sym(_) => ("Sym", None, Vec::new()),
        Expr::Add(terms) => ("Add", None, terms.iter().map(tree_node).collect()),
        Expr::Mul(factors) => ("Mul", None, factors.iter().map(tree_node).collect()),
        Expr::Pow(base, exponent) => ("Pow", None, vec![tree_node(base), tree_node(exponent)]),
        Expr::Fun(name, arguments) => (
            "Fun",
            Some(name.clone()),
            arguments.iter().map(tree_node).collect(),
        ),
        Expr::Div(numerator, denominator) => (
            "Div",
            None,
            vec![tree_node(numerator), tree_node(denominator)],
        ),
        Expr::Rel(op, left, right) => (
            "Rel",
            Some(operator_code(*op).to_string()),
            vec![tree_node(left), tree_node(right)],
        ),
        Expr::Real(_) => ("Real", None, Vec::new()),
        // A complex constant is a leaf of the engine's expression model, like `Num` and `Real`:
        // it has no sub-expressions, and its label is the engine's own rendering.
        Expr::Complex(_, _) => ("Complex", None, Vec::new()),
    };
    Json::obj([
        ("kind", Json::Str(kind.to_string())),
        ("detail", detail.map(Json::Str).unwrap_or(Json::Null)),
        ("label", Json::Str(expression.to_string())),
        ("children", Json::Arr(children)),
    ])
}

fn operator_code(op: RelOp) -> &'static str {
    match op {
        RelOp::Eq => "eq",
        RelOp::Ne => "ne",
        RelOp::Gt => "gt",
        RelOp::Lt => "lt",
        RelOp::Ge => "ge",
        RelOp::Le => "le",
    }
}

fn operator_label(op: RelOp) -> &'static str {
    match op {
        RelOp::Eq => "==",
        RelOp::Ne => "!=",
        RelOp::Gt => ">",
        RelOp::Lt => "<",
        RelOp::Ge => ">=",
        RelOp::Le => "<=",
    }
}

/// The assumptions section: the engine's own `inspect(value).assumptions` for the session, plus
/// the `TransformResult`'s result conditions. Each row keeps the engine's structured kind.
fn assumptions_section(
    engine: &mut lovelace_lang::Engine,
    expression: &Expr,
    result_conditions: &[&Value],
) -> Json {
    let inspected = lovelace_lang::builtins::call(
        engine,
        "inspect",
        vec![Value::Symbolic(expression.clone())],
        0,
    );
    let mut items: Vec<Json> = Vec::new();
    let mut inspect_reason: Option<String> = None;
    match inspected {
        Ok(value) => match record_field_value(&value, "assumptions") {
            Some(Value::Vector(values)) => {
                items.extend(values.iter().map(|item| assumption_json("session", item)));
            }
            Some(Value::Array(array)) => {
                items.extend(array.iter().map(|item| assumption_json("session", item)));
            }
            Some(other) => items.push(assumption_json("session", other)),
            None => {
                inspect_reason = Some(
                    "The engine's inspect record did not publish an assumptions field.".to_string(),
                )
            }
        },
        Err(error) => inspect_reason = Some(format!("inspect() refused: {}", error.message)),
    }
    for condition in result_conditions {
        items.push(assumption_json("result", condition));
    }
    if items.is_empty() {
        if let Some(reason) = inspect_reason {
            return Json::obj([("reason", Json::Str(reason))]);
        }
    } else if let Some(reason) = inspect_reason {
        // The result conditions are still inspectable, so keep the section usable and say that
        // the session half could not be read instead of dropping it silently.
        items.push(Json::obj([
            ("source", Json::Str("session".to_string())),
            ("kind", Json::Str("Unavailable".to_string())),
            ("label", Json::Str(reason)),
        ]));
    }
    Json::obj([("items", Json::Arr(items))])
}

fn assumption_json(source: &str, value: &Value) -> Json {
    match value {
        Value::Symbolic(Expr::Rel(op, left, right)) => {
            let relation = Expr::Rel(*op, left.clone(), right.clone());
            Json::obj([
                ("source", Json::Str(source.to_string())),
                ("kind", Json::Str("Relation".to_string())),
                ("op", Json::Str(operator_code(*op).to_string())),
                ("opLabel", Json::Str(operator_label(*op).to_string())),
                ("left", Json::Str(left.to_string())),
                ("right", Json::Str(right.to_string())),
                ("label", Json::Str(value.display())),
                ("canonical", Json::Str(relation.canonical())),
            ])
        }
        Value::Symbolic(expression) => Json::obj([
            ("source", Json::Str(source.to_string())),
            ("kind", Json::Str("Symbolic".to_string())),
            ("label", Json::Str(expression.to_string())),
            ("canonical", Json::Str(expression.canonical())),
        ]),
        Value::Record(type_name, _) => Json::obj([
            ("source", Json::Str(source.to_string())),
            ("kind", Json::Str(type_name.clone())),
            ("label", Json::Str(value.display())),
        ]),
        other => Json::obj([
            ("source", Json::Str(source.to_string())),
            ("kind", Json::Str(other.kind_name().to_string())),
            ("label", Json::Str(other.display())),
        ]),
    }
}

/// The trace section: the `TransformResult.steps` records, with before/after taken from the
/// engine's symbolic projection of each field and each step's required conditions kept typed.
fn trace_section(steps: &[&Value]) -> Json {
    let mut out = Vec::new();
    for step in steps {
        let Value::Record(type_name, fields) = step else {
            continue;
        };
        if type_name != "RewriteStep" {
            continue;
        }
        let rule_id = record_field(fields, "rule_id")
            .and_then(text_value)
            .unwrap_or("unknown");
        let classification = record_field(fields, "classification")
            .and_then(enum_value)
            .unwrap_or("Unclassified");
        let before = record_field(fields, "before")
            .map(rendering_json)
            .unwrap_or_else(|| Json::obj([("pretty", Json::Str(String::new()))]));
        let after = record_field(fields, "after")
            .map(rendering_json)
            .unwrap_or_else(|| Json::obj([("pretty", Json::Str(String::new()))]));
        let conditions = record_field(fields, "required_conditions")
            .map(|value| match value {
                Value::Vector(values) => values
                    .iter()
                    .map(|condition| assumption_json("required", condition))
                    .collect::<Vec<Json>>(),
                Value::Array(array) => array
                    .iter()
                    .map(|condition| assumption_json("required", condition))
                    .collect::<Vec<Json>>(),
                other => vec![assumption_json("required", other)],
            })
            .unwrap_or_default();
        out.push(Json::obj([
            ("ruleId", Json::Str(rule_id.to_string())),
            ("classification", Json::Str(classification.to_string())),
            ("before", before),
            ("after", after),
            ("conditions", Json::Arr(conditions)),
        ]));
    }
    Json::obj([("steps", Json::Arr(out))])
}

/// The engine's pretty + canonical rendering of one symbolic step field.
fn rendering_json(value: &Value) -> Json {
    match value {
        Value::Symbolic(expression) => Json::obj([
            ("pretty", Json::Str(expression.to_string())),
            ("canonical", Json::Str(expression.canonical())),
        ]),
        other => Json::obj([("pretty", Json::Str(other.display()))]),
    }
}

/// The MathIR section: the language surface's compile path over the subject and its free symbols.
fn mathir_section(expression: &Expr) -> Json {
    let parameters = expression.free_symbols();
    if parameters.is_empty() {
        return Json::obj([(
            "reason",
            Json::Str(
                "The engine's compile path requires at least one parameter, and this expression \
                 has no free symbols."
                    .to_string(),
            ),
        )]);
    }
    match lovelace_lang::mathir::compile_ordered(expression, &parameters) {
        Ok(result) => Json::obj([
            (
                "text",
                Json::Str(result.ir_text().trim_end_matches('\n').to_string()),
            ),
            ("parameters", Json::str_array(parameters)),
            (
                "resultType",
                result
                    .result_type()
                    .map(|kind| Json::Str(kind.name().to_string()))
                    .unwrap_or(Json::Null),
            ),
        ]),
        Err(error) => Json::obj([("reason", Json::Str(error.message))]),
    }
}

fn record_field<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, value)| value)
}

fn record_field_value<'a>(record: &'a Value, name: &str) -> Option<&'a Value> {
    match record {
        Value::Record(_, fields) => record_field(fields, name),
        _ => None,
    }
}

fn record_list<'a>(fields: &'a [(String, Value)], name: &str) -> Vec<&'a Value> {
    match record_field(fields, name) {
        Some(Value::Vector(values)) => values.iter().collect(),
        Some(Value::Array(array)) => array.iter().collect(),
        Some(other) => vec![other],
        None => Vec::new(),
    }
}

fn text_value(value: &Value) -> Option<&str> {
    match value {
        Value::Text(text) => Some(text),
        _ => None,
    }
}

fn enum_value(value: &Value) -> Option<&str> {
    match value {
        Value::Enum(_, variant) => Some(variant),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbolic_expression_projects_its_tree_from_the_engine_ast() {
        let mut engine = lovelace_lang::Engine::new();
        engine.eval("x = symbol(\"x\")").expect("define the symbol");
        let value = engine.eval("diff(x^3, x)").expect("diff");
        let payload = capture(&mut engine, &value);
        let subject = payload.get("subject").expect("subject");
        assert_eq!(subject.get("kind").and_then(Json::as_str), Some("Symbolic"));
        let sections = payload.get("sections").expect("sections");
        let pretty = sections
            .get("pretty")
            .and_then(|section| section.get("text"))
            .and_then(Json::as_str)
            .expect("pretty");
        assert_eq!(pretty, "3*x^2");
        let canonical = sections
            .get("canonical")
            .and_then(|section| section.get("text"))
            .and_then(Json::as_str)
            .expect("canonical");
        assert!(canonical.contains("(mul "), "canonical={canonical}");
        let tree = sections
            .get("tree")
            .and_then(|section| section.get("root"))
            .expect("tree root");
        assert_eq!(tree.get("kind").and_then(Json::as_str), Some("Mul"));
        let children = tree
            .get("children")
            .and_then(Json::as_arr)
            .expect("children");
        assert_eq!(children.len(), 2);
        let mathir = sections
            .get("mathir")
            .and_then(|section| section.get("text"))
            .and_then(Json::as_str)
            .expect("mathir");
        assert!(mathir.starts_with("#!mathir 2"), "mathir={mathir}");
        assert!(mathir.contains("PowInt"), "mathir={mathir}");
        // A plain Symbolic value has no trace, and the payload says why.
        let trace_reason = sections
            .get("trace")
            .and_then(|section| section.get("reason"))
            .and_then(Json::as_str)
            .expect("trace reason");
        assert!(trace_reason.contains("simplify_full"), "{trace_reason}");
    }

    #[test]
    fn inspect_publishes_the_session_assumptions_as_typed_relations() {
        let mut engine = lovelace_lang::Engine::new();
        engine.eval("x = symbol(\"x\")").expect("symbol");
        engine.eval("assume(x > 0)").expect("assume");
        let value = engine.eval("diff(x^3, x)").expect("diff");
        let payload = capture(&mut engine, &value);
        let items = payload
            .get("sections")
            .and_then(|sections| sections.get("assumptions"))
            .and_then(|section| section.get("items"))
            .and_then(Json::as_arr)
            .expect("assumption items");
        assert!(
            !items.is_empty(),
            "the session assumption must be published"
        );
        let first = &items[0];
        assert_eq!(first.get("source").and_then(Json::as_str), Some("session"));
        assert_eq!(first.get("kind").and_then(Json::as_str), Some("Relation"));
        assert_eq!(first.get("op").and_then(Json::as_str), Some("gt"));
        assert_eq!(first.get("left").and_then(Json::as_str), Some("x"));
        assert_eq!(first.get("right").and_then(Json::as_str), Some("0"));
        assert!(
            first
                .get("canonical")
                .and_then(Json::as_str)
                .expect("canonical")
                .starts_with("(gt ")
        );
    }

    #[test]
    fn a_transform_result_publishes_the_steps_and_its_conditions() {
        let mut engine = lovelace_lang::Engine::new();
        engine.eval("x = symbol(\"x\")").expect("symbol");
        engine.eval("assume(x > 0)").expect("assume");
        let value = engine
            .eval("simplify_full(exp(log(x)))")
            .expect("simplify_full");
        let payload = capture(&mut engine, &value);
        let sections = payload.get("sections").expect("sections");
        let subject = payload.get("subject").expect("subject");
        assert_eq!(
            subject.get("kind").and_then(Json::as_str),
            Some("TransformResult")
        );
        assert_eq!(
            subject.get("path").and_then(Json::as_str),
            Some("result.expression")
        );
        assert_eq!(
            sections
                .get("pretty")
                .and_then(|section| section.get("text"))
                .and_then(Json::as_str),
            Some("x")
        );
        let steps = sections
            .get("trace")
            .and_then(|section| section.get("steps"))
            .and_then(Json::as_arr)
            .expect("steps");
        assert_eq!(steps.len(), 1);
        let step = &steps[0];
        assert_eq!(
            step.get("ruleId").and_then(Json::as_str),
            Some("logexp.exp-log")
        );
        assert_eq!(
            step.get("classification").and_then(Json::as_str),
            Some("Conditional")
        );
        assert_eq!(
            step.get("before")
                .and_then(|before| before.get("pretty"))
                .and_then(Json::as_str),
            Some("exp(log(x))")
        );
        assert_eq!(
            step.get("after")
                .and_then(|after| after.get("pretty"))
                .and_then(Json::as_str),
            Some("x")
        );
        let conditions = step
            .get("conditions")
            .and_then(Json::as_arr)
            .expect("required conditions");
        assert_eq!(conditions.len(), 1);
        assert_eq!(
            conditions[0].get("kind").and_then(Json::as_str),
            Some("FiniteCondition")
        );
        // The TransformResult's own conditions field is published alongside the session one.
        let assumptions = sections
            .get("assumptions")
            .and_then(|section| section.get("items"))
            .and_then(Json::as_arr)
            .expect("assumptions");
        assert!(assumptions.iter().any(|item| {
            item.get("source").and_then(Json::as_str) == Some("result")
                && item.get("kind").and_then(Json::as_str) == Some("FiniteCondition")
        }));
        // The expression x lowers with its free symbol as the parameter.
        let mathir = sections
            .get("mathir")
            .and_then(|section| section.get("text"))
            .and_then(Json::as_str)
            .expect("mathir");
        assert!(mathir.contains("param x"), "mathir={mathir}");
    }

    #[test]
    fn a_relation_that_cannot_lower_reports_the_engine_reason_and_keeps_the_tree() {
        let mut engine = lovelace_lang::Engine::new();
        engine.eval("x = symbol(\"x\")").expect("symbol");
        let value = engine.eval("x < 0").expect("relation");
        let payload = capture(&mut engine, &value);
        let sections = payload.get("sections").expect("sections");
        assert_eq!(
            sections
                .get("mathir")
                .and_then(|section| section.get("reason"))
                .and_then(Json::as_str),
            Some("Node kind Relation cannot be lowered to MathIR.")
        );
        assert_eq!(
            sections
                .get("tree")
                .and_then(|tree| tree.get("root"))
                .and_then(|root| root.get("kind"))
                .and_then(Json::as_str),
            Some("Rel")
        );
        assert_eq!(
            sections
                .get("pretty")
                .and_then(|section| section.get("text"))
                .and_then(Json::as_str),
            Some("x < 0")
        );
    }

    #[test]
    fn a_non_symbolic_result_reports_a_specific_reason_in_every_section() {
        let mut engine = lovelace_lang::Engine::new();
        let value = engine.eval("1 + 1").expect("arithmetic");
        let payload = capture(&mut engine, &value);
        assert_eq!(
            payload
                .get("subject")
                .and_then(|subject| subject.get("kind"))
                .and_then(Json::as_str),
            Some("Natural")
        );
        for section in [
            "pretty",
            "canonical",
            "tree",
            "assumptions",
            "trace",
            "mathir",
        ] {
            let reason = payload
                .get("sections")
                .and_then(|sections| sections.get(section))
                .and_then(|value| value.get("reason"))
                .and_then(Json::as_str)
                .unwrap_or_else(|| panic!("{section} has no reason"));
            assert!(reason.contains("Natural"), "{section}: {reason}");
        }
    }
}
