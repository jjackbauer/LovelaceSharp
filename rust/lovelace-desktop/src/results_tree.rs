//! RES-001: the typed, recursive model of the engine's structured result.
//!
//! The only structure input is the `Json` published by
//! `lovelace_lang::structured::project_value` (the same projection the CLI runner and the C#
//! parity reference publish). [`ResultProjection::tree`] reads `structured` and never
//! [`ResultProjection::display`]: the display text is carried so a pane can show the engine's
//! human rendering beside the tree, not as a source of structure.
//!
//! The model is deliberately the projection's own vocabulary:
//!
//! | Projection `kind` | [`StructuredNode`] | Children |
//! | --- | --- | --- |
//! | `Array` (including `Vector`) | [`ArrayNode`] | row-major `elements`, plus `shape` |
//! | `Record` | [`RecordNode`] | named `fields` |
//! | `Symbolic` | [`SymbolicNode`] | `pretty` + `canonical` (+ domain/free symbols) |
//! | every scalar kind | [`ScalarNode`] | none |
//! | `Null` | [`StructuredNode::Null`] | none |
//!
//! A 2-D array is rendered by rows and columns through [`ArrayNode::rows`]; the pane painter in
//! `ui::app` uses the same shape to lay the cells out. Nothing here looks at a display string.

use lovelace_json::Json;

/// One result as the engine published it: the display text and, alongside it, the structured
/// projection.
///
/// The window carries this pair per committed result. [`Self::tree`] is the only constructor of
/// the render tree and it uses `structured` alone, which is what the RES-001 negative test pins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultProjection {
    /// The engine's human display form. Kept for a label; never parsed.
    pub display: String,
    /// The engine's structured projection of the same value, when the engine published one.
    pub structured: Option<Json>,
}

impl ResultProjection {
    /// Pairs a display string with the engine's projection.
    pub fn new(display: impl Into<String>, structured: Option<Json>) -> ResultProjection {
        ResultProjection {
            display: display.into(),
            structured,
        }
    }

    /// Builds the typed tree from the structured projection only.
    ///
    /// Returns `None` when the engine published no projection (for example the deterministic
    /// fake engine); the caller then shows an explicit "no structured value" state. There is no
    /// fallback that parses [`Self::display`].
    pub fn tree(&self) -> Option<StructuredNode> {
        self.structured
            .as_ref()
            .and_then(StructuredNode::from_structured)
    }
}

/// One node of the typed result tree, named by the projection's own `kind` field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructuredNode {
    /// An array or vector: a shape and row-major elements.
    Array(ArrayNode),
    /// A record: a named type and named fields.
    Record(RecordNode),
    /// A symbolic expression: pretty and canonical renderings.
    Symbolic(SymbolicNode),
    /// A scalar: its kind and the projection's value fields.
    Scalar(ScalarNode),
    /// An absent value (the reference's Void, projected as `{"kind":"Null"}`).
    Null,
}

impl StructuredNode {
    /// Builds one node from a projected value object.
    ///
    /// The `kind` string is read from the projection and is the node's identity; it is never
    /// inferred from any display text.
    pub fn from_structured(json: &Json) -> Option<StructuredNode> {
        let kind = json.get("kind")?.as_str()?.to_string();
        match kind.as_str() {
            "Array" => Some(StructuredNode::Array(array_from(&kind, json))),
            "Record" => Some(StructuredNode::Record(record_from(&kind, json))),
            "Symbolic" => Some(StructuredNode::Symbolic(symbolic_from(&kind, json))),
            "Null" => Some(StructuredNode::Null),
            _ => Some(StructuredNode::Scalar(scalar_from(kind, json))),
        }
    }

    /// The node's projection kind (`Array`, `Record`, `Symbolic`, `Null`, `Natural`, ...).
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn kind(&self) -> &str {
        match self {
            StructuredNode::Array(array) => &array.kind,
            StructuredNode::Record(record) => &record.kind,
            StructuredNode::Symbolic(symbolic) => &symbolic.kind,
            StructuredNode::Scalar(scalar) => &scalar.kind,
            StructuredNode::Null => "Null",
        }
    }

    /// The one-line label the pane paints for this node.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn headline(&self) -> String {
        match self {
            StructuredNode::Array(array) => array.headline(),
            StructuredNode::Record(record) => record.headline(),
            StructuredNode::Symbolic(symbolic) => symbolic.headline(),
            StructuredNode::Scalar(scalar) => scalar.headline(),
            StructuredNode::Null => "Null - no value".to_string(),
        }
    }
}

/// An array (or vector) node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayNode {
    /// The projection's `kind` (`Array`; a vector's kind is published as `Array`).
    pub kind: String,
    /// The published `type` (`Array` or `Vector`).
    pub type_name: String,
    /// The published shape, one entry per dimension.
    pub shape: Vec<usize>,
    /// The published elements in row-major order.
    pub elements: Vec<StructuredNode>,
}

impl ArrayNode {
    /// The number of dimensions.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn rank(&self) -> usize {
        self.shape.len()
    }

    /// True for a rank-2 array, the case the pane lays out as rows and columns.
    pub fn is_matrix(&self) -> bool {
        self.shape.len() == 2
    }

    /// The 2-D rows, each a slice of `shape[1]` elements. Empty when the array is not 2-D or the
    /// published shape does not describe the published element count (the pane then falls back
    /// to the flat element list rather than inventing a grid).
    pub fn rows(&self) -> Vec<&[StructuredNode]> {
        if self.shape.len() != 2 {
            return Vec::new();
        }
        let rows = self.shape[0];
        let columns = self.shape[1];
        match rows.checked_mul(columns) {
            Some(expected) if columns > 0 && expected == self.elements.len() => {
                self.elements.chunks(columns).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The element at `(row, column)`, when the 2-D shape describes the elements.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn cell(&self, row: usize, column: usize) -> Option<&StructuredNode> {
        let rows = self.rows();
        rows.get(row).and_then(|cells| cells.get(column))
    }

    /// The shape as `2 x 2` (or `4` for one dimension, `scalar` for rank 0).
    pub fn shape_text(&self) -> String {
        if self.shape.is_empty() {
            return "scalar".to_string();
        }
        self.shape
            .iter()
            .map(usize::to_string)
            .collect::<Vec<String>>()
            .join(" x ")
    }

    /// `Array 2 x 2 - 4 elements` / `Vector 3 - 3 elements`.
    pub fn headline(&self) -> String {
        format!(
            "{} {} - {} element{}",
            self.type_name,
            self.shape_text(),
            self.elements.len(),
            if self.elements.len() == 1 { "" } else { "s" }
        )
    }
}

/// A record node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordNode {
    /// The projection's `kind` (`Record`).
    pub kind: String,
    /// The published record type name (`DivRemResult`, `MatrixInverseResult`, ...).
    pub type_name: String,
    /// The published named fields, in the projection's order.
    pub fields: Vec<RecordField>,
}

impl RecordNode {
    /// The field names, in the projection's order.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|field| field.name.as_str()).collect()
    }

    /// Looks up one field's value by its published name.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn field(&self, name: &str) -> Option<&StructuredNode> {
        self.fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    }

    /// `DivRemResult - 2 fields`.
    pub fn headline(&self) -> String {
        format!(
            "{} - {} field{}",
            self.type_name,
            self.fields.len(),
            if self.fields.len() == 1 { "" } else { "s" }
        )
    }
}

/// One named record field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordField {
    /// The field name exactly as the projection published it.
    pub name: String,
    /// The field's value.
    pub value: StructuredNode,
}

/// A symbolic expression node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicNode {
    /// The projection's `kind` (`Symbolic`).
    pub kind: String,
    /// The human rendering the projection published.
    pub pretty: String,
    /// The canonical rendering the projection published.
    pub canonical: String,
    /// The published expression domain, when the projection carries one.
    pub domain: Option<String>,
    /// The published free-symbol names.
    pub free_symbols: Vec<String>,
    /// The published node count.
    pub node_count: Option<String>,
    /// Whether the projection marked the rendering as shortened by a budget.
    pub truncated: bool,
    /// The remaining published fields (`truncationReason`, `budget`, `exact`, ...) as text.
    pub details: Vec<(String, String)>,
}

impl SymbolicNode {
    /// `Symbolic 3*x^2 + 4*x + 5`.
    pub fn headline(&self) -> String {
        format!("Symbolic  {}", self.pretty)
    }
}

/// A scalar node: the projection kind plus the value fields it published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarNode {
    /// The projection's `kind` (`Natural`, `Integer`, `Real`, `Boolean`, `Text`, `Complex`, ...).
    pub kind: String,
    /// The primary value field (the projection's `value`, `domain`, or the complex pair).
    pub value: String,
    /// The remaining scalar fields, in the projection's key order, as text.
    pub details: Vec<(String, String)>,
}

impl ScalarNode {
    /// `Natural 42`.
    #[allow(dead_code)] // model accessor: tests and any host renderer read it
    pub fn headline(&self) -> String {
        format!("{} {}", self.kind, self.value)
    }
}

/// Builds an array node from a projected `{"kind":"Array",...}` object.
fn array_from(kind: &str, json: &Json) -> ArrayNode {
    let type_name = text_field(json, "type").unwrap_or_else(|| "Array".to_string());
    let shape = json
        .get("shape")
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|dimension| match dimension {
                    Json::Num(number) => number.parse::<usize>().ok(),
                    Json::Str(text) => text.parse::<usize>().ok(),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let elements = json
        .get("elements")
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(StructuredNode::from_structured)
                .collect()
        })
        .unwrap_or_default();
    ArrayNode {
        kind: kind.to_string(),
        type_name,
        shape,
        elements,
    }
}

/// Builds a record node from a projected `{"kind":"Record",...}` object.
fn record_from(kind: &str, json: &Json) -> RecordNode {
    let type_name = text_field(json, "type").unwrap_or_else(|| "Record".to_string());
    let fields = json
        .get("fields")
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|field| {
                    let name = field.get("name")?.as_str()?.to_string();
                    let value = StructuredNode::from_structured(field.get("value")?)?;
                    Some(RecordField { name, value })
                })
                .collect()
        })
        .unwrap_or_default();
    RecordNode {
        kind: kind.to_string(),
        type_name,
        fields,
    }
}

/// Builds a symbolic node from a projected `{"kind":"Symbolic",...}` object.
fn symbolic_from(kind: &str, json: &Json) -> SymbolicNode {
    let pretty = text_field(json, "pretty").unwrap_or_default();
    let canonical = text_field(json, "canonical").unwrap_or_default();
    let domain = text_field(json, "domain");
    let free_symbols = json
        .get("freeSymbols")
        .and_then(Json::as_arr)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let node_count = number_field(json, "nodeCount");
    let mut details = Vec::new();
    let truncated = bool_field(json, "truncated").unwrap_or(false);
    for key in ["exact", "truncationReason", "budget"] {
        if let Some(value) = json.get(key) {
            details.push((key.to_string(), json_text(value)));
        }
    }
    SymbolicNode {
        kind: kind.to_string(),
        pretty,
        canonical,
        domain,
        free_symbols,
        node_count,
        truncated,
        details,
    }
}

/// Builds a scalar node. The primary field is chosen by the projection's published kind only.
fn scalar_from(kind: String, json: &Json) -> ScalarNode {
    let mut consumed: Vec<&str> = vec!["kind"];
    let value = match kind.as_str() {
        "Complex" => {
            consumed.push("re");
            consumed.push("im");
            format!(
                "{} + {}i",
                text_field(json, "re").unwrap_or_default(),
                text_field(json, "im").unwrap_or_default()
            )
        }
        "Domain" => {
            consumed.push("domain");
            text_field(json, "domain").unwrap_or_default()
        }
        _ => {
            consumed.push("value");
            text_field(json, "value").unwrap_or_default()
        }
    };
    let mut details = Vec::new();
    if let Json::Obj(map) = json {
        for (key, field) in map {
            if !consumed.contains(&key.as_str()) {
                details.push((key.clone(), json_text(field)));
            }
        }
    }
    ScalarNode {
        kind,
        value,
        details,
    }
}

fn text_field(json: &Json, key: &str) -> Option<String> {
    json.get(key).and_then(Json::as_str).map(str::to_string)
}

fn number_field(json: &Json, key: &str) -> Option<String> {
    match json.get(key) {
        Some(Json::Num(number)) => Some(number.clone()),
        Some(Json::Str(text)) => Some(text.clone()),
        _ => None,
    }
}

fn bool_field(json: &Json, key: &str) -> Option<bool> {
    match json.get(key) {
        Some(Json::Bool(flag)) => Some(*flag),
        _ => None,
    }
}

/// Renders one published JSON leaf as text (numbers as written, booleans, strings, arrays).
fn json_text(json: &Json) -> String {
    match json {
        Json::Null => "null".to_string(),
        Json::Bool(flag) => flag.to_string(),
        Json::Num(number) => number.clone(),
        Json::Str(text) => text.clone(),
        Json::Arr(items) => items
            .iter()
            .map(json_text)
            .collect::<Vec<String>>()
            .join(", "),
        Json::Obj(_) => json.to_pretty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "real-engine")]
    use crate::engine::Evaluator;

    /// One hand-built projection in the exact shape `structured::project_value` publishes for
    /// `divrem(7, 2)`: a record whose fields carry scalar kinds and values.
    fn divrem_projection() -> Json {
        Json::obj([
            ("kind", Json::Str("Record".to_string())),
            ("type", Json::Str("DivRemResult".to_string())),
            (
                "fields",
                Json::Arr(vec![
                    Json::obj([
                        ("name", Json::Str("quotient".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Natural".to_string())),
                                ("value", Json::Str("3".to_string())),
                                ("exact", Json::Bool(true)),
                            ]),
                        ),
                    ]),
                    Json::obj([
                        ("name", Json::Str("remainder".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Natural".to_string())),
                                ("value", Json::Str("1".to_string())),
                                ("exact", Json::Bool(true)),
                            ]),
                        ),
                    ]),
                ]),
            ),
        ])
    }

    /// One hand-built 2x2 projection, matching `[[1, 2], [3, 4]]`.
    fn matrix_projection() -> Json {
        let cell = |value: &str| {
            Json::obj([
                ("kind", Json::Str("Natural".to_string())),
                ("value", Json::Str(value.to_string())),
                ("exact", Json::Bool(true)),
            ])
        };
        Json::obj([
            ("kind", Json::Str("Array".to_string())),
            ("type", Json::Str("Array".to_string())),
            ("shape", Json::num_array([2, 2])),
            (
                "elements",
                Json::Arr(vec![cell("1"), cell("2"), cell("3"), cell("4")]),
            ),
        ])
    }

    #[test]
    fn the_tree_reads_node_kinds_and_record_field_names_from_the_projection() {
        let tree = ResultProjection::new("ignored display", Some(divrem_projection()))
            .tree()
            .expect("a projected record yields a tree");
        let StructuredNode::Record(record) = &tree else {
            panic!("the projection's kind is Record, so the tree node must be Record: {tree:?}");
        };
        assert_eq!(record.kind, "Record");
        assert_eq!(record.type_name, "DivRemResult");
        assert_eq!(record.field_names(), ["quotient", "remainder"]);
        let StructuredNode::Scalar(quotient) = record.field("quotient").unwrap() else {
            panic!("the quotient field is a scalar");
        };
        assert_eq!(quotient.kind, "Natural");
        assert_eq!(quotient.value, "3");
        assert_eq!(quotient.details, [("exact".to_string(), "true".to_string())]);
        let StructuredNode::Scalar(remainder) = record.field("remainder").unwrap() else {
            panic!("the remainder field is a scalar");
        };
        assert_eq!(remainder.value, "1");
    }

    #[test]
    fn a_2x2_array_publishes_its_shape_and_rows_from_the_projection() {
        let tree = ResultProjection::new("ignored display", Some(matrix_projection()))
            .tree()
            .expect("a projected array yields a tree");
        let StructuredNode::Array(array) = &tree else {
            panic!("the projection's kind is Array, so the tree node must be Array: {tree:?}");
        };
        assert_eq!(array.kind, "Array");
        assert_eq!(array.type_name, "Array");
        assert_eq!(array.shape, [2, 2]);
        assert_eq!(array.rank(), 2);
        assert!(array.is_matrix());
        assert_eq!(array.shape_text(), "2 x 2");
        let rows = array.rows();
        assert_eq!(rows.len(), 2, "a 2x2 array has two rows");
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[1].len(), 2);
        for (row, expected) in rows.iter().zip([["1", "2"], ["3", "4"]]) {
            for (cell, expected_value) in row.iter().zip(expected) {
                let StructuredNode::Scalar(scalar) = cell else {
                    panic!("every matrix cell is a scalar in this projection");
                };
                assert_eq!(scalar.kind, "Natural");
                assert_eq!(scalar.value, expected_value);
            }
        }
        assert_eq!(array.cell(1, 0).unwrap().kind(), "Natural");
    }

    /// RES-001 negative test: the display string is deliberately ambiguous (it names a record
    /// and a 1-D list while the projection is a 2x2 array). The tree is built from the
    /// projection only, so the array structure still appears; two contradictory displays produce
    /// the identical tree.
    #[test]
    fn the_display_string_cannot_change_the_rendered_tree() {
        let ambiguous = "DivRemResult(quotient: 9, remainder: 9) [1, 2, 3, 4]";
        let other = "[[9, 9], [9, 9]]";
        let from_ambiguous =
            ResultProjection::new(ambiguous, Some(matrix_projection())).tree();
        let from_other = ResultProjection::new(other, Some(matrix_projection())).tree();
        assert_eq!(
            from_ambiguous, from_other,
            "the display text must not influence the tree"
        );
        let tree = from_ambiguous.expect("the projection yields a tree");
        assert_eq!(tree.kind(), "Array", "the tree follows the projection kind");
        let StructuredNode::Array(array) = tree else {
            panic!("a 2x2 projection must render as an array");
        };
        assert_eq!(array.shape, [2, 2]);
        assert_eq!(array.rows().len(), 2);
        assert_eq!(array.cell(0, 0).unwrap().headline(), "Natural 1");
        assert!(
            !array.elements.iter().any(|node| node.kind() == "Record"),
            "the record-looking display must not create record nodes"
        );
    }

    #[test]
    fn no_projection_renders_no_tree_and_never_parses_the_display() {
        let projection = ResultProjection::new("[[1, 2], [3, 4]]", None);
        assert_eq!(
            projection.tree(),
            None,
            "without an engine projection the display is not parsed"
        );
    }

    /// The full engine path: the real adapter projects the value the engine returned.
    #[cfg(feature = "real-engine")]
    fn engine_tree(script: &str) -> (String, StructuredNode) {
        let mut engine = crate::engine::RealEngine::new();
        let display = engine.eval(script).expect("the script evaluates");
        let structured = engine
            .last_structured()
            .expect("the real engine publishes its structured projection");
        let projection = ResultProjection::new(display, Some(structured));
        let tree = projection.tree().expect("the projection is a tree");
        (projection.display, tree)
    }

    #[test]
    #[cfg(feature = "real-engine")]
    fn a_real_2x2_matrix_renders_its_rows_from_the_engine_projection() {
        let (_display, tree) = engine_tree("[[1, 2], [3, 4]]");
        assert_eq!(tree.kind(), "Array");
        let StructuredNode::Array(array) = tree else {
            panic!("[[1, 2], [3, 4]] must render as an array");
        };
        assert_eq!(array.shape, [2, 2]);
        assert_eq!(array.rows().len(), 2);
        let values: Vec<&str> = array
            .rows()
            .iter()
            .flat_map(|row| row.iter())
            .map(|cell| match cell {
                StructuredNode::Scalar(scalar) => scalar.value.as_str(),
                other => panic!("matrix cells are Natural scalars: {other:?}"),
            })
            .collect();
        assert_eq!(values, ["1", "2", "3", "4"]);
    }

    #[test]
    #[cfg(feature = "real-engine")]
    fn a_real_record_renders_its_published_field_names() {
        let (_display, tree) = engine_tree("divrem(7, 2)");
        assert_eq!(tree.kind(), "Record");
        let StructuredNode::Record(record) = tree else {
            panic!("divrem publishes a record");
        };
        assert_eq!(record.kind, "Record");
        assert_eq!(record.type_name, "DivRemResult");
        assert_eq!(record.field_names(), ["quotient", "remainder"]);
        assert_eq!(
            record.field("quotient").map(StructuredNode::headline),
            Some("Natural 3".to_string())
        );
        assert_eq!(
            record.field("remainder").map(StructuredNode::headline),
            Some("Natural 1".to_string())
        );
    }

    /// A nested record produced by the engine: `inv_full`'s `inverse` field is itself a 2x2
    /// array of symbolic entries and `conditions` is an array of symbolic expressions.
    #[test]
    #[cfg(feature = "real-engine")]
    fn a_real_nested_record_publishes_its_fields_and_nested_matrix() {
        let (_display, tree) = engine_tree("x = symbol(\"x\"); inv_full([[x, 1], [1, x]])");
        assert_eq!(tree.kind(), "Record");
        let StructuredNode::Record(record) = tree else {
            panic!("inv_full publishes a record");
        };
        assert_eq!(record.type_name, "MatrixInverseResult");
        assert_eq!(
            record.field_names(),
            [
                "status",
                "complete",
                "completeness",
                "inverse",
                "conditions",
                "diagnostics"
            ]
        );
        let inverse = record
            .field("inverse")
            .expect("the record publishes its inverse");
        let StructuredNode::Array(inverse) = inverse else {
            panic!("the inverse field is a matrix: {inverse:?}");
        };
        assert_eq!(inverse.shape, [2, 2]);
        assert_eq!(inverse.rows().len(), 2);
        assert_eq!(inverse.cell(0, 0).unwrap().kind(), "Symbolic");
        let conditions = record
            .field("conditions")
            .expect("the record publishes its conditions");
        let StructuredNode::Array(conditions) = conditions else {
            panic!("the conditions field is a vector: {conditions:?}");
        };
        assert_eq!(conditions.type_name, "Vector");
        assert_eq!(conditions.elements.len(), 1);
    }

    #[test]
    #[cfg(feature = "real-engine")]
    fn a_real_symbolic_diff_publishes_pretty_and_canonical_renderings() {
        let (_display, tree) = engine_tree("x = symbol(\"x\"); diff(x^3 + 2*x^2 + 5*x + 7, x)");
        assert_eq!(tree.kind(), "Symbolic");
        let StructuredNode::Symbolic(symbolic) = tree else {
            panic!("diff publishes a symbolic expression");
        };
        assert!(!symbolic.pretty.is_empty(), "the projection publishes pretty");
        assert!(
            !symbolic.canonical.is_empty(),
            "the projection publishes canonical"
        );
        assert_eq!(symbolic.free_symbols, ["x"]);
        assert!(
            symbolic.pretty.contains('x') && symbolic.canonical.contains('x'),
            "both renderings name the derivative's variable: {symbolic:?}"
        );
        assert!(
            symbolic.node_count.as_deref().is_some_and(|count| count != "0"),
            "the projection publishes a node count: {symbolic:?}"
        );
    }

    /// The structured value is carried on every captured variable too, from the engine's own
    /// projection, and it is the same object a result of that value would carry.
    #[test]
    #[cfg(feature = "real-engine")]
    fn every_captured_variable_carries_the_engine_projection() {
        let mut engine = crate::engine::RealEngine::new();
        engine
            .eval("m = [[1, 2], [3, 4]]\nr = divrem(7, 2)")
            .expect("the script evaluates");
        let state = engine.state().expect("the real engine publishes its capture");
        let m = state
            .variables
            .iter()
            .find(|variable| variable.name == "m")
            .expect("m is in the capture");
        let Some(Json::Obj(_)) = &m.structured else {
            panic!("m must carry the engine's structured projection: {m:?}");
        };
        let tree = StructuredNode::from_structured(m.structured.as_ref().unwrap())
            .expect("the projection is a tree");
        let StructuredNode::Array(array) = tree else {
            panic!("m is an array");
        };
        assert_eq!(array.shape, [2, 2]);
        let r = state
            .variables
            .iter()
            .find(|variable| variable.name == "r")
            .expect("r is in the capture");
        let record = StructuredNode::from_structured(r.structured.as_ref().unwrap())
            .expect("the projection is a tree");
        assert_eq!(record.kind(), "Record");
        assert_eq!(record.headline(), "DivRemResult - 2 fields");
    }
}
