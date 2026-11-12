//! SYM-001: the typed model of the engine's symbolic inspection payload.
//!
//! The window renders this model; it never derives a tree, a trace or an IR from display text.
//! The payload is produced by the engine adapter at the moment a value is committed:
//!
//! | Section | Engine source (see `engine::real::inspection`) |
//! | --- | --- |
//! | pretty | `structured::project_value` of the symbolic subject |
//! | canonical | `structured::project_value` of the symbolic subject |
//! | expression tree | the engine's own `lovelace_symbolics::Expr` carried by `Value::Symbolic` |
//! | assumptions | `inspect(value).assumptions`, plus a `TransformResult`'s `conditions` |
//! | trace | a `TransformResult`'s `steps` (the `simplify_full` record) |
//! | MathIR | the engine's `mathir::compile_ordered` path over the subject |
//!
//! Every section is either supplied or carries the specific engine reason it is not. A missing
//! section is therefore a statement about this result, never an empty box.

use std::collections::BTreeMap;

use lovelace_json::Json;

/// One payload section: the engine's data, or the reason the engine supplied none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Section<T> {
    /// The engine supplied this section.
    Supplied(T),
    /// The engine supplied no section; the string is the specific reason.
    Missing(String),
}

impl<T> Section<T> {
    /// The section's value, when supplied.
    #[allow(dead_code)] // model accessor: the tests read it
    pub fn supplied(&self) -> Option<&T> {
        match self {
            Section::Supplied(value) => Some(value),
            Section::Missing(_) => None,
        }
    }

    /// The specific reason, when the section is missing.
    #[allow(dead_code)] // model accessor: the tests read it
    pub fn reason(&self) -> Option<&str> {
        match self {
            Section::Supplied(_) => None,
            Section::Missing(reason) => Some(reason),
        }
    }
}

/// The value the inspector is bound to and where it sits in the last committed result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// The engine kind or record type (`Symbolic`, `TransformResult`, `Integer`, ...).
    pub kind: String,
    /// The path inside the result (`result` or `result.expression`).
    pub path: String,
    /// Optional engine detail, such as a `TransformResult`'s status.
    pub detail: Option<String>,
}

/// One typed assumption / condition row.
///
/// A relation is kept typed: `op` is the engine's relation operator code (`gt`, `le`, ...) and
/// `left`/`right` are its operands, so the pane never has to split a sentence. Records such as a
/// `FiniteCondition` keep their record `kind` and the engine's pretty rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assumption {
    /// Where the row came from: `session` (inspect), `result` (TransformResult conditions) or
    /// `required` (one trace step's required conditions).
    pub source: String,
    /// The engine's structured condition kind (`Relation`, `FiniteCondition`, ...).
    pub kind: String,
    /// The engine's human rendering of the condition.
    pub label: String,
    /// The engine's canonical S-expression, when the condition is symbolic.
    pub canonical: Option<String>,
    /// The relation operator code, for a symbolic relation (`gt`, `le`, ...).
    pub op: Option<String>,
    /// The relation operator as published (`>`, `<=`, ...).
    pub op_label: Option<String>,
    /// The relation's left operand, pretty-printed by the engine.
    pub left: Option<String>,
    /// The relation's right operand, pretty-printed by the engine.
    pub right: Option<String>,
}

impl Assumption {
    /// The typed description the pane paints before the rendering: `relation >` or the record
    /// kind. It is derived from the payload's own fields, not from the display text.
    pub fn type_label(&self) -> String {
        match (&self.kind[..], self.op_label.as_deref()) {
            ("Relation", Some(op)) => format!("relation {op}"),
            (kind, _) => kind.to_string(),
        }
    }
}

/// One rewrite step of a `TransformResult`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    /// The engine's stable rewrite-rule id, e.g. `logexp.exp-log`.
    pub rule_id: String,
    /// The engine's classification: `Universal`, `Conditional` or `DomainSpecific`.
    pub classification: String,
    /// The rewritten sub-expression, pretty-printed by the engine.
    pub before: String,
    /// The rewritten sub-expression's canonical form.
    pub before_canonical: String,
    /// The produced sub-expression, pretty-printed by the engine.
    pub after: String,
    /// The produced sub-expression's canonical form.
    pub after_canonical: String,
    /// The conditions this step requires (typed condition rows).
    pub conditions: Vec<Assumption>,
}

/// The lowered MathIR text and its compilation contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathIr {
    /// The serialized IR, exactly as the engine's compile path emitted it.
    pub text: String,
    /// The parameter names the compile call was made with.
    pub parameters: Vec<String>,
    /// The engine's inferred result type, when it reported one.
    pub result_type: Option<String>,
}

/// One node of the engine's expression tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    /// The engine node kind (`Num`, `Sym`, `Add`, `Mul`, `Pow`, `Fun`, `Div`, `Rel`, `Real`).
    pub kind: String,
    /// The function name or relation operator for a `Fun`/`Rel` node.
    pub detail: Option<String>,
    /// The engine's pretty rendering of this sub-expression.
    pub label: String,
    /// The child nodes, in the engine's own child order.
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    /// The number of nodes in this subtree, root included.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(TreeNode::count).sum::<usize>()
    }

    /// The header text: node kind, optional detail, then the engine rendering.
    pub fn header(&self) -> String {
        match &self.detail {
            Some(detail) => format!("{} {} · {}", self.kind, detail, self.label),
            None => format!("{} · {}", self.kind, self.label),
        }
    }
}

/// One engine inspection payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    /// The subject value and its path in the result.
    pub subject: Subject,
    /// `pretty`: the engine's pretty rendering of the subject.
    pub pretty: Section<String>,
    /// `canonical`: the engine's canonical rendering of the subject.
    pub canonical: Section<String>,
    /// `tree`: the engine expression tree.
    pub tree: Section<TreeNode>,
    /// `assumptions`: typed session assumptions and TransformResult conditions.
    pub assumptions: Section<Vec<Assumption>>,
    /// `trace`: the `simplify_full` rewrite steps.
    pub trace: Section<Vec<TraceStep>>,
    /// `mathir`: the compiled IR.
    pub mathir: Section<MathIr>,
}

impl Inspection {
    /// Reads the desktop-owned payload the engine adapter publishes.
    ///
    /// Returns `None` only when the payload is not an object or has no subject. A malformed
    /// section is `Missing` with a reason saying the payload was unreadable, so a schema mismatch
    /// is visible instead of silently blanking the pane.
    pub fn from_json(json: &Json) -> Option<Inspection> {
        let subject_json = json.get("subject")?;
        let subject = Subject {
            kind: subject_json
                .get("kind")
                .and_then(Json::as_str)
                .unwrap_or("Unknown")
                .to_string(),
            path: subject_json
                .get("path")
                .and_then(Json::as_str)
                .unwrap_or("result")
                .to_string(),
            detail: subject_json
                .get("detail")
                .and_then(Json::as_str)
                .map(str::to_string),
        };
        let sections = json.get("sections");
        let section = |name: &str| sections.and_then(|sections| sections.get(name));
        Some(Inspection {
            subject,
            pretty: text_section(section("pretty"), "pretty"),
            canonical: text_section(section("canonical"), "canonical"),
            tree: tree_section(section("tree")),
            assumptions: assumptions_section(section("assumptions")),
            trace: trace_section(section("trace")),
            mathir: mathir_section(section("mathir")),
        })
    }

    /// The six section names, in the order the inspector paints them.
    #[allow(dead_code)] // model vocabulary: the round document and tests name them
    pub const SECTION_NAMES: [&'static str; 6] = [
        "pretty",
        "canonical",
        "expression tree",
        "assumptions",
        "simplification trace",
        "MathIR",
    ];
}

fn missing(name: &str, reason: &str) -> Section<String> {
    Section::Missing(format!("The engine supplied no {name}: {reason}"))
}

fn malformed(name: &str) -> Section<String> {
    Section::Missing(format!(
        "The engine's {name} section was not readable (unexpected payload shape)."
    ))
}

fn text_section(json: Option<&Json>, name: &str) -> Section<String> {
    let Some(json) = json else {
        return missing(name, "the payload has no such section.");
    };
    if let Some(reason) = json.get("reason").and_then(Json::as_str) {
        return Section::Missing(reason.to_string());
    }
    match json.get("text").and_then(Json::as_str) {
        Some(text) => Section::Supplied(text.to_string()),
        None => malformed(name),
    }
}

fn tree_section(json: Option<&Json>) -> Section<TreeNode> {
    let Some(json) = json else {
        return Section::Missing(
            "The engine supplied no expression tree: the payload has no such section.".to_string(),
        );
    };
    if let Some(reason) = json.get("reason").and_then(Json::as_str) {
        return Section::Missing(reason.to_string());
    }
    match json.get("root").and_then(tree_node) {
        Some(root) => Section::Supplied(root),
        None => Section::Missing(
            "The engine's expression tree section was not readable (unexpected payload shape)."
                .to_string(),
        ),
    }
}

fn tree_node(json: &Json) -> Option<TreeNode> {
    let kind = json.get("kind").and_then(Json::as_str)?.to_string();
    let label = json.get("label").and_then(Json::as_str)?.to_string();
    let detail = json
        .get("detail")
        .and_then(Json::as_str)
        .map(str::to_string);
    let children = json
        .get("children")
        .and_then(Json::as_arr)
        .map(|children| children.iter().filter_map(tree_node).collect())
        .unwrap_or_default();
    Some(TreeNode {
        kind,
        detail,
        label,
        children,
    })
}

fn assumptions_section(json: Option<&Json>) -> Section<Vec<Assumption>> {
    let Some(json) = json else {
        return Section::Missing(
            "The engine supplied no assumptions: the payload has no such section.".to_string(),
        );
    };
    if let Some(reason) = json.get("reason").and_then(Json::as_str) {
        return Section::Missing(reason.to_string());
    }
    match json.get("items").and_then(Json::as_arr) {
        Some(items) => Section::Supplied(items.iter().filter_map(assumption).collect()),
        None => Section::Missing(
            "The engine's assumptions section was not readable (unexpected payload shape)."
                .to_string(),
        ),
    }
}

fn assumption(json: &Json) -> Option<Assumption> {
    Some(Assumption {
        source: json
            .get("source")
            .and_then(Json::as_str)
            .unwrap_or("session")
            .to_string(),
        kind: json.get("kind").and_then(Json::as_str)?.to_string(),
        label: json
            .get("label")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_string(),
        canonical: json
            .get("canonical")
            .and_then(Json::as_str)
            .map(str::to_string),
        op: json.get("op").and_then(Json::as_str).map(str::to_string),
        op_label: json
            .get("opLabel")
            .and_then(Json::as_str)
            .map(str::to_string),
        left: json.get("left").and_then(Json::as_str).map(str::to_string),
        right: json.get("right").and_then(Json::as_str).map(str::to_string),
    })
}

fn trace_section(json: Option<&Json>) -> Section<Vec<TraceStep>> {
    let Some(json) = json else {
        return Section::Missing(
            "The engine supplied no simplification trace: the payload has no such section."
                .to_string(),
        );
    };
    if let Some(reason) = json.get("reason").and_then(Json::as_str) {
        return Section::Missing(reason.to_string());
    }
    match json.get("steps").and_then(Json::as_arr) {
        Some(steps) => Section::Supplied(steps.iter().filter_map(trace_step).collect()),
        None => Section::Missing(
            "The engine's trace section was not readable (unexpected payload shape).".to_string(),
        ),
    }
}

fn trace_step(json: &Json) -> Option<TraceStep> {
    let pair = |key: &str, part: &str| -> String {
        json.get(key)
            .and_then(|value| value.get(part))
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Some(TraceStep {
        rule_id: json.get("ruleId").and_then(Json::as_str)?.to_string(),
        classification: json
            .get("classification")
            .and_then(Json::as_str)
            .unwrap_or("Unclassified")
            .to_string(),
        before: pair("before", "pretty"),
        before_canonical: pair("before", "canonical"),
        after: pair("after", "pretty"),
        after_canonical: pair("after", "canonical"),
        conditions: json
            .get("conditions")
            .and_then(Json::as_arr)
            .map(|items| items.iter().filter_map(assumption).collect())
            .unwrap_or_default(),
    })
}

fn mathir_section(json: Option<&Json>) -> Section<MathIr> {
    let Some(json) = json else {
        return Section::Missing(
            "The engine supplied no MathIR: the payload has no such section.".to_string(),
        );
    };
    if let Some(reason) = json.get("reason").and_then(Json::as_str) {
        return Section::Missing(reason.to_string());
    }
    let Some(text) = json.get("text").and_then(Json::as_str) else {
        return Section::Missing(
            "The engine's MathIR section was not readable (unexpected payload shape).".to_string(),
        );
    };
    Section::Supplied(MathIr {
        text: text.to_string(),
        parameters: json
            .get("parameters")
            .and_then(Json::as_arr)
            .map(|parameters| {
                parameters
                    .iter()
                    .filter_map(Json::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        result_type: json
            .get("resultType")
            .and_then(Json::as_str)
            .map(str::to_string),
    })
}

/// The collapse state of the inspector's expression tree, scoped to one inspection payload.
///
/// The key is the payload's compact serialization, the same rule round 27 uses for the result
/// tree: an edit to the buffer cannot change it, while a new committed result can.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InspectorExpansion {
    identity: Option<String>,
    collapsed: BTreeMap<String, bool>,
}

impl InspectorExpansion {
    /// A state map with no decisions: every tree node starts expanded.
    pub fn new() -> InspectorExpansion {
        InspectorExpansion::default()
    }

    /// Binds the map to one payload. Returns true when a previous result was replaced.
    pub fn bind(&mut self, inspection: &Json) -> bool {
        let identity = inspection.to_string();
        if self.identity.as_deref() == Some(identity.as_str()) {
            return false;
        }
        let replaced = self.identity.is_some();
        self.identity = Some(identity);
        self.collapsed.clear();
        replaced
    }

    /// True when the node at `path` is collapsed. Unknown paths are expanded.
    pub fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.get(path).copied().unwrap_or(false)
    }

    /// Records one node's collapsed state without touching any other path.
    pub fn set_collapsed(&mut self, path: &str, collapsed: bool) {
        self.collapsed.insert(path.to_string(), collapsed);
    }

    /// Flips one node and returns its new collapsed state.
    pub fn toggle(&mut self, path: &str) -> bool {
        let collapsed = !self.is_collapsed(path);
        self.set_collapsed(path, collapsed);
        collapsed
    }

    /// The paths the user has collapsed, in path order.
    #[allow(dead_code)] // model accessor: the tests read it
    pub fn collapsed_paths(&self) -> Vec<&str> {
        self.collapsed
            .iter()
            .filter(|(_, collapsed)| **collapsed)
            .map(|(path, _)| path.as_str())
            .collect()
    }

    /// The visible indicator for `path`.
    pub fn glyph(&self, path: &str) -> &'static str {
        if self.is_collapsed(path) {
            crate::results_tree::COLLAPSED_GLYPH
        } else {
            crate::results_tree::EXPANDED_GLYPH
        }
    }

    /// The state word a UIA client reads: `collapsed` or `expanded`.
    pub fn state_word(&self, path: &str) -> &'static str {
        if self.is_collapsed(path) {
            "collapsed"
        } else {
            "expanded"
        }
    }

    /// The accessible name of a tree header: text, node path and state.
    pub fn accessible_name(&self, path: &str, label: &str) -> String {
        format!("{label} [{path}] {}", self.state_word(path))
    }
}

/// The path of the tree root in the inspector.
pub const TREE_ROOT_PATH: &str = "inspect.tree";

/// The path of a child node: `inspect.tree.0`, `inspect.tree.0.1`.
pub fn tree_path(parent: &str, index: usize) -> String {
    format!("{parent}.{index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str) -> Json {
        Json::obj([("text", Json::Str(text.to_string()))])
    }

    fn relation() -> Json {
        Json::obj([
            ("source", Json::Str("session".to_string())),
            ("kind", Json::Str("Relation".to_string())),
            ("op", Json::Str("gt".to_string())),
            ("opLabel", Json::Str(">".to_string())),
            ("left", Json::Str("x".to_string())),
            ("right", Json::Str("0".to_string())),
            ("label", Json::Str("x > 0".to_string())),
            ("canonical", Json::Str("(gt (sym x) (rat 0 1))".to_string())),
        ])
    }

    fn full_payload() -> Json {
        Json::obj([
            (
                "subject",
                Json::obj([
                    ("kind", Json::Str("Symbolic".to_string())),
                    ("path", Json::Str("result".to_string())),
                ]),
            ),
            (
                "sections",
                Json::obj([
                    ("pretty", text("x^2 - 1")),
                    (
                        "canonical",
                        text("(add (pow (sym x) (rat 2 1)) (rat -1 1))"),
                    ),
                    (
                        "tree",
                        Json::obj([
                            ("nodes", Json::Num("5".to_string())),
                            (
                                "root",
                                Json::obj([
                                    ("kind", Json::Str("Add".to_string())),
                                    ("label", Json::Str("x^2 - 1".to_string())),
                                    (
                                        "children",
                                        Json::Arr(vec![
                                            Json::obj([
                                                ("kind", Json::Str("Pow".to_string())),
                                                ("label", Json::Str("x^2".to_string())),
                                                ("children", Json::Arr(Vec::new())),
                                            ]),
                                            Json::obj([
                                                ("kind", Json::Str("Num".to_string())),
                                                ("label", Json::Str("-1".to_string())),
                                                ("children", Json::Arr(Vec::new())),
                                            ]),
                                        ]),
                                    ),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "assumptions",
                        Json::obj([("items", Json::Arr(vec![relation()]))]),
                    ),
                    (
                        "trace",
                        Json::obj([(
                            "steps",
                            Json::Arr(vec![Json::obj([
                                (
                                    "ruleId",
                                    Json::Str("trig.pythagorean-sin2-cos2".to_string()),
                                ),
                                ("classification", Json::Str("Universal".to_string())),
                                (
                                    "before",
                                    Json::obj([
                                        ("pretty", Json::Str("sin(x)^2 + cos(x)^2".to_string())),
                                        (
                                            "canonical",
                                            Json::Str(
                                                "(add (pow (fun sin (sym x)) ...))".to_string(),
                                            ),
                                        ),
                                    ]),
                                ),
                                (
                                    "after",
                                    Json::obj([
                                        ("pretty", Json::Str("1".to_string())),
                                        ("canonical", Json::Str("(rat 1 1)".to_string())),
                                    ]),
                                ),
                                ("conditions", Json::Arr(vec![relation()])),
                            ])]),
                        )]),
                    ),
                    (
                        "mathir",
                        Json::obj([
                            ("text", Json::Str("#!mathir 2\nparam x".to_string())),
                            ("parameters", Json::str_array(["x"])),
                            ("resultType", Json::Str("Integer".to_string())),
                        ]),
                    ),
                ]),
            ),
        ])
    }

    #[test]
    fn a_full_payload_reads_all_six_sections_from_the_engine_data() {
        let inspection = Inspection::from_json(&full_payload()).expect("payload");
        assert_eq!(inspection.subject.kind, "Symbolic");
        assert_eq!(inspection.subject.path, "result");
        assert_eq!(
            inspection.pretty.supplied().map(String::as_str),
            Some("x^2 - 1")
        );
        assert!(inspection.canonical.supplied().is_some());
        let tree = inspection.tree.supplied().expect("tree");
        assert_eq!(tree.kind, "Add");
        assert_eq!(tree.count(), 3);
        assert_eq!(tree.children[0].kind, "Pow");
        let assumption = &inspection.assumptions.supplied().expect("assumptions")[0];
        assert_eq!(assumption.type_label(), "relation >");
        assert_eq!(assumption.left.as_deref(), Some("x"));
        let step = &inspection.trace.supplied().expect("trace")[0];
        assert_eq!(step.rule_id, "trig.pythagorean-sin2-cos2");
        assert_eq!(step.classification, "Universal");
        assert_eq!(step.before, "sin(x)^2 + cos(x)^2");
        assert_eq!(step.conditions[0].source, "session");
        let mathir = inspection.mathir.supplied().expect("mathir");
        assert_eq!(mathir.parameters, vec!["x".to_string()]);
        assert_eq!(mathir.result_type.as_deref(), Some("Integer"));
    }

    #[test]
    fn a_missing_mathir_keeps_its_reason_and_leaves_the_other_sections() {
        let payload = Json::obj([
            (
                "subject",
                Json::obj([
                    ("kind", Json::Str("Symbolic".to_string())),
                    ("path", Json::Str("result".to_string())),
                ]),
            ),
            (
                "sections",
                Json::obj([
                    ("pretty", text("x < 0")),
                    ("canonical", text("(lt (sym x) (rat 0 1))")),
                    (
                        "tree",
                        Json::obj([
                            ("nodes", Json::Num("3".to_string())),
                            (
                                "root",
                                Json::obj([
                                    ("kind", Json::Str("Rel".to_string())),
                                    ("detail", Json::Str("lt".to_string())),
                                    ("label", Json::Str("x < 0".to_string())),
                                    ("children", Json::Arr(Vec::new())),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "mathir",
                        Json::obj([(
                            "reason",
                            Json::Str(
                                "Node kind Relation cannot be lowered to MathIR.".to_string(),
                            ),
                        )]),
                    ),
                ]),
            ),
        ]);
        let inspection = Inspection::from_json(&payload).expect("payload");
        assert_eq!(
            inspection.mathir.reason(),
            Some("Node kind Relation cannot be lowered to MathIR.")
        );
        assert!(inspection.pretty.supplied().is_some());
        assert!(inspection.canonical.supplied().is_some());
        assert!(inspection.tree.supplied().is_some());
        // A section the payload omits is still a specific missing state, not None.
        assert!(
            inspection
                .trace
                .reason()
                .expect("trace reason")
                .contains("has no such section")
        );
    }

    #[test]
    fn the_expansion_map_survives_the_same_payload_and_clears_on_a_new_one() {
        let payload = full_payload();
        let mut expansion = InspectorExpansion::new();
        assert!(!expansion.bind(&payload));
        expansion.set_collapsed("inspect.tree.0", true);
        assert!(!expansion.bind(&payload));
        assert_eq!(expansion.collapsed_paths(), vec!["inspect.tree.0"]);
        // A different inspection payload (one pretty text differs) is a new result identity.
        let other = Json::obj([
            (
                "subject",
                Json::obj([
                    ("kind", Json::Str("Symbolic".to_string())),
                    ("path", Json::Str("result".to_string())),
                ]),
            ),
            ("sections", Json::obj([("pretty", text("x + 1"))])),
        ]);
        assert!(expansion.bind(&other));
        assert!(expansion.collapsed_paths().is_empty());
    }

    #[test]
    fn tree_paths_name_child_order() {
        assert_eq!(tree_path(TREE_ROOT_PATH, 0), "inspect.tree.0");
        assert_eq!(tree_path("inspect.tree.0", 1), "inspect.tree.0.1");
    }
}
