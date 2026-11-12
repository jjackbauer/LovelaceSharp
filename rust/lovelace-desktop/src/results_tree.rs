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

use std::collections::BTreeMap;

use lovelace_json::Json;

/// The visible indicator painted before an expanded node's label.
pub const EXPANDED_GLYPH: &str = "\u{25be}"; // BLACK DOWN-POINTING SMALL TRIANGLE

/// The visible indicator painted before a collapsed node's label.
pub const COLLAPSED_GLYPH: &str = "\u{25b8}"; // BLACK RIGHT-POINTING SMALL TRIANGLE

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

/// The path of one record field: `result.solutions`.
///
/// Paths are the keys of [`ExpansionState`]. They are built from the projection's own names and
/// element indices only, so the same projection always produces the same path.
pub fn field_path(record_path: &str, field_name: &str) -> String {
    format!("{record_path}.{field_name}")
}

/// The path of one array element, matrix row or matrix cell: `result.solutions[0]`,
/// `result.inverse[1][0]`.
pub fn element_path(parent_path: &str, index: usize) -> String {
    format!("{parent_path}[{index}]")
}

/// The identity of the committed result an [`ExpansionState`] belongs to.
///
/// It is the engine display plus the compact serialization of the structured projection. The
/// buffer revision is deliberately not part of it: editing the document, moving the caret or
/// running a quick eval (which never publishes a snapshot) cannot change the identity, while a
/// later commit that publishes a different value does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultIdentity {
    display: String,
    structured: String,
}

impl ResultIdentity {
    /// The identity of the result this projection describes.
    pub fn of(projection: &ResultProjection) -> ResultIdentity {
        ResultIdentity {
            display: projection.display.clone(),
            structured: projection
                .structured
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        }
    }
}

/// Per-node collapse/expand state, keyed by the node path, scoped to the result it describes.
///
/// Every node starts expanded, so the map stores only the decisions the user has made: a path
/// present with `true` is collapsed, a path present with `false` is explicitly expanded, and an
/// absent path is the default (expanded). The map never owns tree data: collapsing a node only
/// hides its children, which stay in the [`StructuredNode`] the pane was handed.
///
/// [`Self::bind`] is called once per painted result. It keeps the map across repaints of the
/// same committed value and clears it when a different result replaces the old one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExpansionState {
    result: Option<ResultIdentity>,
    collapsed: BTreeMap<String, bool>,
}

impl ExpansionState {
    /// A state map with no decisions: every node is expanded.
    pub fn new() -> ExpansionState {
        ExpansionState::default()
    }

    /// Binds the map to the projection's result.
    ///
    /// Returns true when a previous result was replaced; in that case every decision is
    /// dropped, because a path only names a node of the result it was created for. A repaint of
    /// the same result returns false and keeps every decision.
    pub fn bind(&mut self, projection: &ResultProjection) -> bool {
        let identity = ResultIdentity::of(projection);
        if self.result.as_ref() == Some(&identity) {
            return false;
        }
        let replaced = self.result.is_some();
        self.result = Some(identity);
        self.collapsed.clear();
        replaced
    }

    /// True when the node at `path` is collapsed. Unknown paths are expanded.
    pub fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.get(path).copied().unwrap_or(false)
    }

    /// Records one node's state without touching any other path.
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
    #[allow(dead_code)] // model accessor: the tests and the round-27 record read it
    pub fn collapsed_paths(&self) -> Vec<&str> {
        self.collapsed
            .iter()
            .filter(|(_, collapsed)| **collapsed)
            .map(|(path, _)| path.as_str())
            .collect()
    }

    /// The number of paths the user has decided, collapsed or explicitly expanded.
    #[allow(dead_code)] // model accessor: the tests read it
    pub fn len(&self) -> usize {
        self.collapsed.len()
    }

    /// True when the user has made no decision yet.
    #[allow(dead_code)] // model accessor: the tests read it
    pub fn is_empty(&self) -> bool {
        self.collapsed.is_empty()
    }

    /// The visible indicator for `path`.
    pub fn glyph(&self, path: &str) -> &'static str {
        if self.is_collapsed(path) {
            COLLAPSED_GLYPH
        } else {
            EXPANDED_GLYPH
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

    /// The visible header text: the indicator, then the node's label.
    pub fn header_text(&self, path: &str, label: &str) -> String {
        format!("{} {label}", self.glyph(path))
    }

    /// The accessible name of a header: the label, the node path and the state, so a UIA client
    /// can tell which node is collapsed and where it sits in the tree.
    pub fn accessible_name(&self, path: &str, label: &str) -> String {
        format!("{label} [{path}] {}", self.state_word(path))
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
        self.fields
            .iter()
            .map(|field| field.name.as_str())
            .collect()
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
        assert_eq!(
            quotient.details,
            [("exact".to_string(), "true".to_string())]
        );
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
        let from_ambiguous = ResultProjection::new(ambiguous, Some(matrix_projection())).tree();
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

    /// One symbolic expression in the projection's shape.
    fn symbolic_json(pretty: &str) -> Json {
        Json::obj([
            ("kind", Json::Str("Symbolic".to_string())),
            ("pretty", Json::Str(pretty.to_string())),
            ("canonical", Json::Str(format!("(sym {pretty})"))),
            ("freeSymbols", Json::str_array(["x"])),
            ("nodeCount", Json::Str("1".to_string())),
        ])
    }

    /// One nested `Solution` record, as `solve_full` publishes it inside `solutions`.
    fn solution_json(value: &str) -> Json {
        Json::obj([
            ("kind", Json::Str("Record".to_string())),
            ("type", Json::Str("Solution".to_string())),
            (
                "fields",
                Json::Arr(vec![
                    Json::obj([
                        ("name", Json::Str("value".to_string())),
                        ("value", symbolic_json(value)),
                    ]),
                    Json::obj([
                        ("name", Json::Str("multiplicity".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Integer".to_string())),
                                ("value", Json::Str("1".to_string())),
                            ]),
                        ),
                    ]),
                ]),
            ),
        ])
    }

    /// One hand-built `SolveResult` projection: a record whose `solutions` field is a vector of
    /// nested `Solution` records, each with a symbolic `value` and a scalar `multiplicity`.
    fn solve_projection(values: [&str; 2]) -> Json {
        Json::obj([
            ("kind", Json::Str("Record".to_string())),
            ("type", Json::Str("SolveResult".to_string())),
            (
                "fields",
                Json::Arr(vec![
                    Json::obj([
                        ("name", Json::Str("status".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Enum".to_string())),
                                ("value", Json::Str("Solved".to_string())),
                                ("type", Json::Str("SolveStatus".to_string())),
                            ]),
                        ),
                    ]),
                    Json::obj([
                        ("name", Json::Str("variable".to_string())),
                        ("value", symbolic_json("x")),
                    ]),
                    Json::obj([
                        ("name", Json::Str("solutions".to_string())),
                        (
                            "value",
                            Json::obj([
                                ("kind", Json::Str("Array".to_string())),
                                ("type", Json::Str("Vector".to_string())),
                                ("shape", Json::num_array([2])),
                                (
                                    "elements",
                                    Json::Arr(values.iter().map(|v| solution_json(v)).collect()),
                                ),
                            ]),
                        ),
                    ]),
                ]),
            ),
        ])
    }

    /// Every headline and published value in render order: the bytes a repaint must reproduce
    /// after a collapse/expand round trip. It is computed from the model only, so it cannot be
    /// satisfied by keeping the visible rows instead of the children.
    fn content_digest(node: &StructuredNode, out: &mut String) {
        out.push_str(&node.headline());
        out.push('\n');
        match node {
            StructuredNode::Record(record) => {
                for field in &record.fields {
                    out.push_str(&field.name);
                    out.push(':');
                    content_digest(&field.value, out);
                }
            }
            StructuredNode::Array(array) => {
                for element in &array.elements {
                    content_digest(element, out);
                }
            }
            StructuredNode::Symbolic(symbolic) => {
                out.push_str(&symbolic.pretty);
                out.push('\n');
                out.push_str(&symbolic.canonical);
                out.push('\n');
            }
            StructuredNode::Scalar(_) | StructuredNode::Null => {}
        }
    }

    /// RES-004: collapsing a nested record field only records a decision. The projection's
    /// children stay in the tree the pane was handed, with every value intact.
    #[test]
    fn collapsing_a_nested_record_field_keeps_the_children_in_the_model() {
        let projection = ResultProjection::new(
            "SolveResult(status: Solved, variable: x, solutions: [...])",
            Some(solve_projection(["-2", "2"])),
        );
        let tree = projection.tree().expect("the projection yields a tree");
        let mut state = ExpansionState::new();
        assert!(
            !state.bind(&projection),
            "the first bind has no previous result to replace"
        );
        assert!(
            !state.is_collapsed("result.solutions"),
            "every node starts expanded"
        );

        assert!(
            state.toggle("result.solutions"),
            "the toggle reports the new collapsed state"
        );
        assert!(state.is_collapsed("result.solutions"));
        assert_eq!(state.collapsed_paths(), ["result.solutions"]);

        let StructuredNode::Record(record) = &tree else {
            panic!("the projection is a record");
        };
        let StructuredNode::Array(solutions) = record
            .field("solutions")
            .expect("the record still publishes its solutions")
        else {
            panic!("solutions is a vector");
        };
        assert_eq!(
            solutions.elements.len(),
            2,
            "collapsing must not remove the vector's children"
        );
        let StructuredNode::Record(first) = &solutions.elements[0] else {
            panic!("each solution is a nested record");
        };
        assert_eq!(first.type_name, "Solution");
        assert_eq!(first.field_names(), ["value", "multiplicity"]);
        assert_eq!(
            first.field("value").map(StructuredNode::headline),
            Some("Symbolic  -2".to_string()),
            "the nested value is untouched by the collapse"
        );
    }

    /// RES-004: collapsing and re-expanding a field is content-preserving. The model bytes
    /// before the collapse and after the re-expand are identical.
    #[test]
    fn re_expanding_a_collapsed_field_restores_identical_model_content() {
        let projection =
            ResultProjection::new("SolveResult(...)", Some(solve_projection(["-2", "2"])));
        let tree = projection.tree().expect("the projection yields a tree");
        let mut state = ExpansionState::new();
        state.bind(&projection);

        let mut before = String::new();
        content_digest(&tree, &mut before);

        state.toggle("result.solutions");
        assert!(state.is_collapsed("result.solutions"));
        let mut collapsed = String::new();
        content_digest(&tree, &mut collapsed);
        assert_eq!(
            collapsed, before,
            "hiding a node must not change the model content"
        );

        state.toggle("result.solutions");
        assert!(!state.is_collapsed("result.solutions"));
        let mut after = String::new();
        content_digest(&tree, &mut after);
        assert_eq!(
            after.as_bytes(),
            before.as_bytes(),
            "re-expanding must show byte-identical content"
        );
    }

    /// RES-004: the state map keeps every path across a repaint of the same result, and a new
    /// result replaces it. The result identity is the committed value, so an unrelated update
    /// (an edit, a caret move, a quick eval that publishes no snapshot) cannot clear the map.
    #[test]
    fn the_expansion_map_survives_a_repaint_and_a_new_result_clears_it() {
        let first = ResultProjection::new(
            "SolveResult(status: Solved, ...)",
            Some(solve_projection(["-2", "2"])),
        );
        let second = ResultProjection::new(
            "SolveResult(status: Solved, ...)",
            Some(solve_projection(["-3", "3"])),
        );
        let mut state = ExpansionState::new();
        state.bind(&first);
        state.toggle("result.solutions");
        state.toggle("result.status");
        assert_eq!(
            state.collapsed_paths(),
            ["result.solutions", "result.status"],
            "both decisions are keyed by their node path"
        );

        assert!(
            !state.bind(&first),
            "a repaint of the same committed result must not replace it"
        );
        assert!(state.is_collapsed("result.solutions"));
        assert!(state.is_collapsed("result.status"));
        assert_eq!(
            state.len(),
            2,
            "a repaint must not add or drop a path decision"
        );

        assert!(
            state.bind(&second),
            "a later commit that publishes a different value is a new result"
        );
        assert!(
            state.is_empty(),
            "a new result clears the map instead of applying stale paths"
        );
        assert!(!state.is_collapsed("result.solutions"));
        assert!(!state.is_collapsed("result.status"));
    }

    /// RES-004: the visible indicator and the accessible name state the node path and the
    /// state; UIA reads the name.
    #[test]
    fn the_accessible_name_carries_the_node_path_and_the_state() {
        let projection =
            ResultProjection::new("SolveResult(...)", Some(solve_projection(["-2", "2"])));
        let mut state = ExpansionState::new();
        state.bind(&projection);
        let label = "solutions: Vector 2 - 2 elements";
        assert_eq!(
            state.header_text("result.solutions", label),
            "\u{25be} solutions: Vector 2 - 2 elements",
            "an expanded node shows the down-pointing indicator"
        );
        assert_eq!(
            state.accessible_name("result.solutions", label),
            "solutions: Vector 2 - 2 elements [result.solutions] expanded"
        );

        state.toggle("result.solutions");
        assert_eq!(
            state.header_text("result.solutions", label),
            "\u{25b8} solutions: Vector 2 - 2 elements",
            "a collapsed node shows the right-pointing indicator"
        );
        assert_eq!(
            state.accessible_name("result.solutions", label),
            "solutions: Vector 2 - 2 elements [result.solutions] collapsed"
        );
        assert_eq!(
            state.header_text("result.status", "status:"),
            "\u{25be} status:",
            "a sibling keeps its own expanded state"
        );
    }

    /// RES-004: paths name each kind of node the pane paints: record fields by published name,
    /// array elements and matrix cells by index.
    #[test]
    fn paths_name_record_fields_and_array_rows() {
        assert_eq!(field_path("result", "solutions"), "result.solutions");
        assert_eq!(element_path("result.solutions", 0), "result.solutions[0]");
        assert_eq!(
            element_path(&element_path("result.inverse", 1), 0),
            "result.inverse[1][0]"
        );
        assert_eq!(
            field_path(&element_path("result.solutions", 1), "value"),
            "result.solutions[1].value"
        );
    }

    /// RES-004 with the real engine: a solve result is the nested record the pane collapses, and
    /// the paths the pane builds describe its published structure.
    #[test]
    #[cfg(feature = "real-engine")]
    fn a_real_solve_record_publishes_the_nested_fields_the_tree_collapses() {
        let mut engine = crate::engine::RealEngine::new();
        let display = engine
            .eval("x = symbol(\"x\"); solve_full(x^2 - 4 == 0, x)")
            .expect("the script evaluates");
        let projection = ResultProjection::new(display, engine.last_structured());
        let tree = projection.tree().expect("the projection is a tree");
        let StructuredNode::Record(record) = tree else {
            panic!("solve_full publishes a SolveResult record");
        };
        assert_eq!(record.type_name, "SolveResult");
        for field in [
            "status",
            "variable",
            "solutions",
            "families",
            "common_conditions",
            "diagnostics",
        ] {
            assert!(
                record.field(field).is_some(),
                "the solve record publishes {field}"
            );
        }
        let StructuredNode::Array(solutions) = record
            .field("solutions")
            .expect("the record publishes its solutions")
        else {
            panic!("solutions is a vector");
        };
        assert_eq!(solutions.type_name, "Vector");
        assert_eq!(
            solutions.elements.len(),
            2,
            "x^2 - 4 has the two solutions the projection publishes"
        );
        let mut paths = Vec::new();
        for (index, element) in solutions.elements.iter().enumerate() {
            let row = element_path(&field_path("result", "solutions"), index);
            let StructuredNode::Record(solution) = element else {
                panic!("each solution is a nested record");
            };
            assert_eq!(solution.type_name, "Solution");
            assert!(solution.field("value").is_some());
            assert!(solution.field("multiplicity").is_some());
            paths.push(row);
        }
        assert_eq!(paths, ["result.solutions[0]", "result.solutions[1]"]);

        let mut state = ExpansionState::new();
        state.bind(&projection);
        state.toggle(&paths[0]);
        assert!(state.is_collapsed("result.solutions[0]"));
        assert!(
            !state.is_collapsed("result.solutions[1]"),
            "collapsing one solution must not collapse its sibling"
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
        assert!(
            !symbolic.pretty.is_empty(),
            "the projection publishes pretty"
        );
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
            symbolic
                .node_count
                .as_deref()
                .is_some_and(|count| count != "0"),
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
        let state = engine
            .state()
            .expect("the real engine publishes its capture");
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
