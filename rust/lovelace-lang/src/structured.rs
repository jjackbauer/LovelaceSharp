//! The structured projection of runtime values onto the agent-facing wire.
//!
//! One projection, total over the value kinds: nothing meaningful may degrade to plain text merely
//! because this module lacked a case (WIRE-005). The reference implementation is the C#
//! StructuredProjection in Lovelace.Suite; the golden fixtures are the contract this must match.
//!
//! The public interface is consumed by lovelace-run:
//!
//!   project_value(engine, value, budget) -> Json   one value's "structured" object
//!   project_function(engine, name)       -> Option<Json>  one full builtin descriptor
//!   project_functions(engine, pattern, limit) -> Json     the bounded registry listing
//!
//! It is defined here (and not in the runner) so the desktop host can publish the same structure.

use lovelace_array::NdArray;
use lovelace_json::Json;

use crate::{Engine, Value};

/// Projects one runtime value onto its structured wire form after refusing a value deeper than
/// the published value budget.
///
/// The reference's `StructuredProjection.ToStructured` refuses with
/// `InputDepthExceededException` before its recursive walk. This is the fallible entry point for
/// a value the caller constructed itself; the engine and runner only ever project values
/// `Engine::eval` has already admitted, so [`project_value`] stays the infallible form.
pub fn try_project_value(
    engine: &Engine,
    value: &Value,
    budget: Option<usize>,
) -> Result<Json, crate::LangError> {
    crate::check_value_depth(value)?;
    Ok(project_value(engine, value, budget))
}

/// Projects one runtime value onto its structured wire form.
///
/// Total over the value kinds: every kind publishes the fields that identify it, and an absent
/// value publishes `{"kind":"Null"}` rather than an empty object.
pub fn project_value(_engine: &Engine, value: &Value, budget: Option<usize>) -> Json {
    let mut fields: Vec<(&str, Json)> = vec![("kind", Json::Str(value.kind_name().to_string()))];
    let _ = array_elements;
    match value {
        Value::Natural(n) => {
            fields.push(("value", Json::Str(n.to_decimal())));
            fields.push(("exact", Json::Bool(true)));
        }
        Value::Integer(i) => {
            fields.push(("value", Json::Str(i.to_decimal())));
            fields.push(("exact", Json::Bool(true)));
        }
        Value::Real(r) | Value::RealClamped(r, _, _) => {
            // The digits the value STORES, never what the display bound in force would have shown:
            // the reference's StructuredProjection publishes the machine payload, so the façade
            // and the CLI cannot publish different digits for one value and a display bound stays
            // a display bound. (One projection, round-95 engine singletons.)
            fields.push((
                "value",
                Json::Str(lovelace_real::with_display_decimal_places(
                    usize::MAX,
                    || r.to_string(),
                )),
            ));
            fields.push(("exact", Json::Bool(r.is_exact())));
            // An exact Real IS a rational: publish it as one, so 1/3 never has to be parsed back
            // out of "0.(3)" (the reference's StructuredProjection.Real rule).
            if r.is_exact() {
                let (numerator, denominator) = r.to_rational();
                fields.push(("numerator", Json::Str(numerator.to_decimal())));
                fields.push(("denominator", Json::Str(denominator.to_decimal())));
            }
            // A value whose digits a builtin's own cap clamped says so on the wire, in the same
            // vocabulary a bounded rendering uses: the value is not indistinguishable from one
            // the request was honoured for (round-20 audit I, I-2).
            if let Value::RealClamped(_, reason, budget) = value {
                fields.push(("truncated", Json::Bool(true)));
                fields.push(("truncationReason", Json::Str(reason.to_string())));
                fields.push(("budget", Json::Num(budget.to_string())));
            }
        }
        Value::Boolean(b) => {
            fields.push((
                "value",
                Json::Str(if *b { "true" } else { "false" }.to_string()),
            ));
        }
        Value::Text(t) => {
            fields.push(("value", Json::Str(t.clone())));
        }
        Value::Vector(items) => {
            fields[0] = ("kind", Json::Str("Array".to_string()));
            fields.push(("type", Json::Str("Vector".to_string())));
            fields.push(("shape", Json::num_array(vec![items.len() as i64])));
            fields.push((
                "elements",
                Json::Arr(
                    items
                        .iter()
                        .map(|v| project_value(_engine, v, budget))
                        .collect(),
                ),
            ));
        }
        Value::Array(a) => {
            fields.push(("type", Json::Str("Array".to_string())));
            fields.push((
                "shape",
                Json::num_array(a.shape().iter().map(|d| *d as i64).collect::<Vec<i64>>()),
            ));
            fields.push((
                "elements",
                Json::Arr(
                    a.iter()
                        .map(|v| project_value(_engine, v, budget))
                        .collect::<Vec<Json>>(),
                ),
            ));
        }
        Value::Function(name) => {
            fields.push(("value", Json::Str(name.clone())));
        }
        // Void is an ABSENT value: the wire shape is Null, never a kind named "Void".
        Value::Void => {
            fields[0] = ("kind", Json::Str("Null".to_string()));
        }
        Value::Record(type_name, record_fields) => {
            fields.push(("type", Json::Str(type_name.clone())));
            fields.push((
                "fields",
                Json::Arr(
                    record_fields
                        .iter()
                        .map(|(name, v)| {
                            Json::obj(vec![
                                ("name", Json::Str(name.clone())),
                                ("value", project_value(_engine, v, budget)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        Value::Complex(re, im) => {
            fields.push(("re", Json::Str(re.to_string())));
            fields.push(("im", Json::Str(im.to_string())));
            fields.push(("exact", Json::Bool(re.is_exact() && im.is_exact())));
        }
        Value::Symbolic(e) => {
            let pretty = truncate(&e.to_string(), budget, e.node_count());
            let canonical = truncate(&e.canonical(), budget, e.node_count());
            let truncated = pretty.1 || canonical.1;
            fields.push(("pretty", Json::Str(pretty.0)));
            fields.push(("canonical", Json::Str(canonical.0)));
            fields.push(("domain", Json::Str(e.domain().name().to_string())));
            fields.push(("exact", Json::Bool(e.is_exact())));
            fields.push(("nodeCount", Json::Num(e.node_count().to_string())));
            fields.push((
                "freeSymbols",
                Json::Arr(e.free_symbols().into_iter().map(Json::Str).collect()),
            ));
            // A budgeted rendering that was shortened says so, and names the budget that stopped
            // it: an abbreviation is never passed off as the whole expression.
            if truncated {
                fields.push(("truncated", Json::Bool(true)));
                fields.push(("truncationReason", Json::Str("node-budget".to_string())));
                if let Some(max_nodes) = budget {
                    fields.push(("budget", Json::Num(max_nodes.to_string())));
                }
            }
        }
        Value::Domain(name) => {
            fields.push(("domain", Json::Str(name.to_ascii_lowercase())));
        }
        Value::Enum(type_name, variant) => {
            fields.push(("type", Json::Str(type_name.clone())));
            fields.push(("value", Json::Str(variant.clone())));
        }
    }
    let _ = budget;
    Json::obj(fields)
}

/// A budgeted rendering: the text and whether the budget shortened it.
///
/// A node budget bounds nodes, so a rendering whose node count is over the budget is ALWAYS
/// abbreviated and always says so; the abbreviation is a prefix of the real rendering cut at a
/// token boundary, terminated by " …".
fn truncate(full: &str, budget: Option<usize>, node_count: usize) -> (String, bool) {
    let Some(max_nodes) = budget else {
        return (full.to_string(), false);
    };
    if node_count <= max_nodes || full.is_empty() {
        return (full.to_string(), false);
    }
    // TWO allowances, in the reference's order. The kernel printer abbreviates only when the
    // rendering is longer than max(48, 6 x budget) CHARACTERS; when it does, that character
    // allowance is the cut. The suite projection then makes the node budget TOTAL for the short
    // renderings the kernel leaves whole: a proportional cut (full x budget / nodes) that is
    // always a strict prefix. Using only the proportional rule moved every long rendering's cut
    // point, which is the print-budget mismatch class (77 stored regressions).
    let chars = full.chars().count();
    let character_allowance = max_nodes.saturating_mul(6).max(48);
    let allowed = if chars > character_allowance {
        character_allowance
    } else {
        ((chars as u128 * max_nodes as u128) / node_count as u128).max(1) as usize
    };
    let cut = cut_at_token_boundary(full, allowed);
    if cut == 0 {
        ("…".to_string(), true)
    } else {
        (format!("{} …", &full[..cut]), true)
    }
}

/// Cuts at a token boundary so a number or identifier is never split mid-token.
fn cut_at_token_boundary(text: &str, cut: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut i = cut.min(chars.len());
    while i > 0 && is_token_char(chars[i - 1]) {
        i -= 1;
    }
    if i > 0 {
        return i;
    }
    let mut after = cut.min(chars.len());
    while after < chars.len() && is_token_char(chars[after]) {
        after += 1;
    }
    if after < chars.len() { after } else { 0 }
}

fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.'
}

/// The array elements in row-major order.
fn array_elements(a: &NdArray<Value>) -> Vec<Value> {
    a.iter().cloned().collect()
}

/// One builtin's full discovery descriptor (the envelope's functions[] entry).
pub fn descriptor_json(info: &crate::registry::BuiltinInfo) -> Json {
    let mut fields: Vec<(&str, Json)> = vec![
        ("name", Json::Str(info.name.to_string())),
        (
            "parameters",
            Json::str_array(info.parameters.iter().copied()),
        ),
        ("minArity", Json::Num(info.min_arity.to_string())),
        (
            "parameterCount",
            Json::Num(info.parameter_count.to_string()),
        ),
        ("variadic", Json::Bool(info.variadic)),
        ("builtin", Json::Bool(true)),
    ];
    if let Some(plugin) = info.plugin {
        fields.push(("plugin", Json::Str(plugin.to_string())));
    }
    fields.push(("category", Json::Str(info.category.to_string())));
    if let Some(summary) = info.summary {
        fields.push(("summary", Json::Str(summary.to_string())));
    }
    if let Some(return_kind) = info.return_kind {
        fields.push(("returnKind", Json::Str(return_kind.to_string())));
    }
    if !info.related.is_empty() {
        fields.push(("related", Json::str_array(info.related.iter().copied())));
    }
    if !info.examples.is_empty() {
        fields.push(("examples", Json::str_array(info.examples.iter().copied())));
    }
    Json::obj(fields)
}

/// One entry of the bounded registry listing: the signature and category, no discovery prose.
pub fn listing_json(info: &crate::registry::BuiltinInfo) -> Json {
    Json::obj(vec![
        ("name", Json::Str(info.name.to_string())),
        (
            "parameters",
            Json::str_array(info.parameters.iter().copied()),
        ),
        ("minArity", Json::Num(info.min_arity.to_string())),
        (
            "parameterCount",
            Json::Num(info.parameter_count.to_string()),
        ),
        ("variadic", Json::Bool(info.variadic)),
        ("category", Json::Str(info.category.to_string())),
    ])
}

/// Projects one builtin's full descriptor, or None when the name is not registered.
pub fn project_function(_engine: &Engine, name: &str) -> Option<Json> {
    crate::registry::find(name).map(descriptor_json)
}

/// The bounded registry listing: names matching an optional pattern, with arity and category.
pub fn project_functions(_engine: &Engine, pattern: Option<&str>, limit: Option<usize>) -> Json {
    let pattern = pattern.unwrap_or("");
    let matches = crate::registry::matching(pattern);
    let total = matches.len();
    let limit = limit.unwrap_or(50);
    let page: Vec<Json> = matches
        .iter()
        .take(limit)
        .map(|info| listing_json(info))
        .collect();
    let returned = page.len();
    Json::obj(vec![
        ("pattern", Json::Str(pattern.to_string())),
        ("functions", Json::Arr(page)),
        ("total", Json::Num(total.to_string())),
        ("returned", Json::Num(returned.to_string())),
        ("truncated", Json::Bool(returned < total)),
    ])
}
