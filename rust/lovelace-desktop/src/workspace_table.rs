//! The deterministic models behind the Workspace pane's two tables.
//!
//! The pane used to paint two unlabelled lists: variables as `name = typed display` and
//! functions as `name(parameters)` read from the engine's *user* function list. This module
//! owns the table data instead, so the visible rows, the sort order and the counts are testable
//! without an egui pass:
//!
//! - [`VariableTable`] shows every variable the committed snapshot published (name, kind and
//!   display) in the engine's own order by default, and both directions of every column on a
//!   header click;
//! - [`FunctionTable`] shows the engine's registry snapshot (name, parameters, origin and the
//!   summary when the registry publishes one) with the same sorting contract.
//!
//! Both tables keep the capture's ordinal on every row. Sorting never drops a row and always
//! breaks ties by that ordinal, so two rows with the same kind (or the same display) keep the
//! engine's order in both directions: a repeated sort is a permutation of exactly the same set.

use crate::engine::{EngineFunction, EngineState, FunctionOrigin};

/// Longest text a cell paints; a longer value is cut with an ellipsis and its full text stays
/// available to the caller for the hover tooltip.
pub const MAX_CELL_CHARS: usize = 40;

/// The ellipsis appended to a cut cell.
const ELLIPSIS: char = '…';

/// Cuts `text` to at most [`MAX_CELL_CHARS`] characters, ending in an ellipsis when it was cut.
///
/// The function counts characters, not bytes, so a value containing non-ASCII text is never
/// split inside a character. A short value is returned unchanged.
pub fn cell_text(text: &str) -> String {
    if text.chars().count() <= MAX_CELL_CHARS {
        return text.to_string();
    }
    let head: String = text.chars().take(MAX_CELL_CHARS - 1).collect();
    format!("{head}{ELLIPSIS}")
}

/// The direction arrow a sorted column shows: up for ascending, down for descending.
pub fn direction_arrow(ascending: bool) -> &'static str {
    if ascending { "▲" } else { "▼" }
}

/// A column's click state: which column sorts, and in which direction.
///
/// The contract is the requirement's: the default is the capture's ordinal (name) order, the
/// **first** click on a column sorts it ascending and the **second** click on that same column
/// sorts it descending; a third click returns to ascending. Selecting a different column starts
/// again at ascending. The `engaged` flag is what makes the first click on the already-default
/// name column stay ascending instead of reversing it before the user has asked for descending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortState<C> {
    /// The column currently sorted.
    pub key: C,
    /// True for ascending (the first click), false for descending (the second).
    pub ascending: bool,
    /// Whether the current column has received its first click.
    engaged: bool,
}

impl<C: Copy + PartialEq> SortState<C> {
    /// A state whose default column sorts ascending but has not been clicked yet.
    pub fn new(key: C) -> SortState<C> {
        SortState {
            key,
            ascending: true,
            engaged: false,
        }
    }

    /// Applies one header click: the first click on a column starts ascending, the next flips.
    pub fn clicked(&mut self, key: C) {
        if self.key == key {
            if self.engaged {
                self.ascending = !self.ascending;
            } else {
                self.engaged = true;
                self.ascending = true;
            }
        } else {
            self.key = key;
            self.engaged = true;
            self.ascending = true;
        }
    }

    /// True when `key` is the sorted column.
    pub fn is_active(&self, key: C) -> bool {
        self.key == key
    }

    /// The header text: the label plus an up/down arrow when the column is the sorted one.
    pub fn header_text(&self, key: C, label: &str) -> String {
        if self.is_active(key) {
            format!("{label} {}", direction_arrow(self.ascending))
        } else {
            label.to_string()
        }
    }

    /// The hovered direction wording for a sorted column.
    pub fn direction_label(&self) -> &'static str {
        if self.ascending {
            "ascending"
        } else {
            "descending"
        }
    }
}

/// One variable row: the engine's published name, kind and display value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableRow {
    /// The variable name as written in source (including `_`).
    pub name: String,
    /// The kind the engine published (`Natural`, `Real`, `Text`, `Record`, ...).
    pub kind: String,
    /// The untyped display rendering the engine produced.
    pub display: String,
    /// The position in the committed capture; every sort breaks ties by this value.
    pub ordinal: usize,
}

impl VariableRow {
    /// The display cell, cut for the pane and paired with the full text as its tooltip.
    pub fn display_cell(&self) -> String {
        cell_text(&self.display)
    }
}

/// The sortable columns of the variables table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VariableColumn {
    /// Variable name.
    Name,
    /// Published kind.
    Kind,
    /// Display value.
    Display,
}

impl VariableColumn {
    /// Every column, in paint order.
    pub const ALL: [VariableColumn; 3] = [
        VariableColumn::Name,
        VariableColumn::Kind,
        VariableColumn::Display,
    ];

    /// The header label.
    pub fn label(self) -> &'static str {
        match self {
            VariableColumn::Name => "Name",
            VariableColumn::Kind => "Kind",
            VariableColumn::Display => "Value",
        }
    }

    /// The zero-based position, for the pane's recorded header rectangles.
    pub fn index(self) -> usize {
        match self {
            VariableColumn::Name => 0,
            VariableColumn::Kind => 1,
            VariableColumn::Display => 2,
        }
    }

    fn key(self, row: &VariableRow) -> &str {
        match self {
            VariableColumn::Name => row.name.as_str(),
            VariableColumn::Kind => row.kind.as_str(),
            VariableColumn::Display => row.display.as_str(),
        }
    }
}

/// One function row: the engine's registry entry, classified by origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionRow {
    /// The registered name.
    pub name: String,
    /// The declared parameter names, in declaration order.
    pub parameters: Vec<String>,
    /// `builtin`, `plugin` or `user`, from the engine's builtin flag and plugin name.
    pub origin: FunctionOrigin,
    /// The registering plugin, when one did.
    pub plugin: Option<String>,
    /// The registry summary, when the engine publishes one.
    pub summary: Option<String>,
    /// The position in the committed capture; every sort breaks ties by this value.
    pub ordinal: usize,
}

impl FunctionRow {
    /// The parameters as one comma-joined string (`a, b`).
    pub fn parameters_text(&self) -> String {
        self.parameters.join(", ")
    }

    /// The origin wording the pane paints.
    pub fn origin_label(&self) -> &'static str {
        self.origin.label()
    }

    /// The summary cell, cut for the pane; `None` when the registry published no summary.
    pub fn summary_cell(&self) -> Option<String> {
        self.summary.as_deref().map(cell_text)
    }
}

/// The sortable columns of the functions table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionColumn {
    /// Function name.
    Name,
    /// Comma-joined parameter names.
    Parameters,
    /// `builtin` / `plugin` / `user`.
    Origin,
}

impl FunctionColumn {
    /// Every column, in paint order.
    pub const ALL: [FunctionColumn; 3] = [
        FunctionColumn::Name,
        FunctionColumn::Parameters,
        FunctionColumn::Origin,
    ];

    /// The header label.
    pub fn label(self) -> &'static str {
        match self {
            FunctionColumn::Name => "Name",
            FunctionColumn::Parameters => "Parameters",
            FunctionColumn::Origin => "Origin",
        }
    }

    /// The zero-based position, for the pane's recorded header rectangles.
    pub fn index(self) -> usize {
        match self {
            FunctionColumn::Name => 0,
            FunctionColumn::Parameters => 1,
            FunctionColumn::Origin => 2,
        }
    }

    fn key(self, row: &FunctionRow) -> String {
        match self {
            FunctionColumn::Name => row.name.clone(),
            FunctionColumn::Parameters => row.parameters_text(),
            FunctionColumn::Origin => row.origin.label().to_string(),
        }
    }
}

/// Builds one row per captured variable, in the capture's ordinal order.
pub fn variable_rows(state: &EngineState) -> Vec<VariableRow> {
    state
        .variables
        .iter()
        .enumerate()
        .map(|(ordinal, variable)| VariableRow {
            name: variable.name.clone(),
            kind: variable.kind.clone(),
            display: variable.display.clone(),
            ordinal,
        })
        .collect()
}

/// Builds one row per captured function, in the capture's ordinal order.
pub fn function_rows(state: &EngineState) -> Vec<FunctionRow> {
    state
        .functions
        .iter()
        .enumerate()
        .map(|(ordinal, function)| function_row(ordinal, function))
        .collect()
}

fn function_row(ordinal: usize, function: &EngineFunction) -> FunctionRow {
    FunctionRow {
        name: function.name.clone(),
        parameters: function.parameters.clone(),
        origin: function.origin(),
        plugin: function.plugin.clone(),
        summary: function.summary.clone(),
        ordinal,
    }
}

/// The variable row indices in `sort` order.
///
/// Equal keys keep capture order in both directions: the ordinal is the final comparison, so
/// descending by kind cannot reverse two rows that share a kind, and every result is a
/// permutation of `0..rows.len()` (sorting never drops a row).
pub fn ordered_variables(rows: &[VariableRow], sort: SortState<VariableColumn>) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..rows.len()).collect();
    let descending = !sort.ascending;
    let column = sort.key;
    indices.sort_by(|&left, &right| {
        let left_row = &rows[left];
        let right_row = &rows[right];
        let ordering = column.key(left_row).cmp(column.key(right_row));
        let ordering = if descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| left_row.ordinal.cmp(&right_row.ordinal))
    });
    indices
}

/// The function row indices in `sort` order, with the same stability contract as
/// [`ordered_variables`].
pub fn ordered_functions(rows: &[FunctionRow], sort: SortState<FunctionColumn>) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..rows.len()).collect();
    let descending = !sort.ascending;
    let column = sort.key;
    indices.sort_by(|&left, &right| {
        let left_row = &rows[left];
        let right_row = &rows[right];
        let ordering = column.key(left_row).cmp(&column.key(right_row));
        let ordering = if descending {
            ordering.reverse()
        } else {
            ordering
        };
        ordering.then_with(|| left_row.ordinal.cmp(&right_row.ordinal))
    });
    indices
}

/// The variables table, with its rows and the current sort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableTable {
    /// Every captured variable, in capture order; sorting never mutates this list.
    pub rows: Vec<VariableRow>,
    /// The current column and direction.
    pub sort: SortState<VariableColumn>,
}

impl Default for VariableTable {
    fn default() -> VariableTable {
        VariableTable {
            rows: Vec::new(),
            sort: SortState::new(VariableColumn::Name),
        }
    }
}

impl VariableTable {
    /// Builds the table from a capture; the default sort is name ascending (ordinal).
    pub fn new(state: &EngineState) -> VariableTable {
        VariableTable {
            rows: variable_rows(state),
            sort: SortState::new(VariableColumn::Name),
        }
    }

    /// Replaces the rows with a new capture, keeping the user's current sort.
    pub fn rebuild(&mut self, state: &EngineState) {
        self.rows = variable_rows(state);
    }

    /// The row indices in current sort order.
    ///
    /// Equal keys keep capture order in both directions: the ordinal is the final comparison,
    /// so descending by kind cannot reverse two rows that share a kind.
    #[cfg_attr(not(test), allow(dead_code))] // host/test accessor; the pane uses `ordered_variables`
    pub fn ordered(&self) -> Vec<usize> {
        ordered_variables(&self.rows, self.sort)
    }
}

/// The functions table, with its rows and the current sort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionTable {
    /// Every captured function, in capture order; sorting never mutates this list.
    pub rows: Vec<FunctionRow>,
    /// The current column and direction.
    pub sort: SortState<FunctionColumn>,
}

impl Default for FunctionTable {
    fn default() -> FunctionTable {
        FunctionTable {
            rows: Vec::new(),
            sort: SortState::new(FunctionColumn::Name),
        }
    }
}

impl FunctionTable {
    /// Builds the table from a capture; the default sort is name ascending (ordinal).
    pub fn new(state: &EngineState) -> FunctionTable {
        FunctionTable {
            rows: function_rows(state),
            sort: SortState::new(FunctionColumn::Name),
        }
    }

    /// Replaces the rows with a new capture, keeping the user's current sort.
    pub fn rebuild(&mut self, state: &EngineState) {
        self.rows = function_rows(state);
    }

    /// The row indices in current sort order; ties keep capture order like the variables table.
    #[cfg_attr(not(test), allow(dead_code))] // host/test accessor; the pane uses `ordered_functions`
    pub fn ordered(&self) -> Vec<usize> {
        ordered_functions(&self.rows, self.sort)
    }

    /// The engine registry rows, by origin. The table tests use it to check the visible split.
    #[cfg_attr(not(test), allow(dead_code))] // invariant accessor used by the table tests
    pub fn count_by_origin(&self, origin: FunctionOrigin) -> usize {
        self.rows.iter().filter(|row| row.origin == origin).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{EngineFunction, EngineState, EngineVariable};

    fn variable(name: &str, kind: &str, display: &str) -> EngineVariable {
        EngineVariable {
            name: name.to_string(),
            kind: kind.to_string(),
            display: display.to_string(),
            structured: None,
        }
    }

    fn function(
        name: &str,
        parameters: &[&str],
        is_builtin: bool,
        plugin: Option<&str>,
        summary: Option<&str>,
    ) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin,
            plugin: plugin.map(str::to_string),
            summary: summary.map(str::to_string),
        }
    }

    fn names(table: &VariableTable) -> Vec<String> {
        table
            .ordered()
            .into_iter()
            .map(|index| table.rows[index].name.clone())
            .collect()
    }

    fn function_names(table: &FunctionTable) -> Vec<String> {
        table
            .ordered()
            .into_iter()
            .map(|index| table.rows[index].name.clone())
            .collect()
    }

    /// WKS-001: the default is ordinal name ascending, and `_` is a normal row with its kind
    /// and display value, not a special case that a filter or a dedup can lose.
    #[test]
    fn variables_default_to_name_ascending_and_include_the_underscore_global() {
        let state = EngineState {
            variables: vec![
                variable("b", "Text", "\"two\""),
                variable("_", "Natural", "42"),
                variable("a", "Natural", "1"),
            ],
            functions: Vec::new(),
        };
        let table = VariableTable::new(&state);
        assert_eq!(table.rows.len(), 3);
        assert_eq!(names(&table), vec!["_", "a", "b"]);
        let underscore = table
            .rows
            .iter()
            .find(|row| row.name == "_")
            .expect("the answer global must be a row");
        assert_eq!(underscore.kind, "Natural");
        assert_eq!(underscore.display, "42");

        // Sort by kind: Natural rows tie, so their capture order (_, a) is kept; Text follows.
        let mut kinded = table.clone();
        kinded.sort.clicked(VariableColumn::Kind);
        let ordered: Vec<&VariableRow> =
            kinded.ordered().iter().map(|i| &kinded.rows[*i]).collect();
        assert_eq!(
            ordered
                .iter()
                .map(|row| row.name.as_str())
                .collect::<Vec<_>>(),
            vec!["_", "a", "b"]
        );

        // The same set sorted both directions by name: `_` is first ascending and last
        // descending, and no row disappears in either direction.
        let mut by_name = table.clone();
        by_name.sort.clicked(VariableColumn::Name); // first click: ascending
        assert_eq!(names(&by_name), vec!["_", "a", "b"]);
        by_name.sort.clicked(VariableColumn::Name); // second click: descending
        assert_eq!(names(&by_name), vec!["b", "a", "_"]);
        by_name.sort.clicked(VariableColumn::Name); // third click: ascending
        assert_eq!(names(&by_name), vec!["_", "a", "b"]);
    }

    /// WKS-001: first click ascending, second descending, third ascending; a new column starts
    /// ascending instead of inheriting the previous column's direction.
    #[test]
    fn a_header_click_sorts_ascending_then_descending_and_a_new_column_restarts_ascending() {
        let state = EngineState {
            variables: vec![
                variable("a", "Text", "1"),
                variable("b", "Natural", "2"),
                variable("c", "Real", "3"),
            ],
            functions: Vec::new(),
        };
        let mut table = VariableTable::new(&state);
        assert_eq!(
            names(&table),
            vec!["a", "b", "c"],
            "default is name ascending"
        );
        assert!(table.sort.ascending);

        // The default column starts on its ascending side: its first click stays ascending and
        // its second click is the descending one.
        table.sort.clicked(VariableColumn::Name);
        assert!(table.sort.ascending);
        assert_eq!(
            names(&table),
            vec!["a", "b", "c"],
            "first click is ascending"
        );

        table.sort.clicked(VariableColumn::Name);
        assert!(!table.sort.ascending);
        assert_eq!(names(&table), vec!["c", "b", "a"], "second click descends");

        table.sort.clicked(VariableColumn::Name);
        assert!(table.sort.ascending);
        assert_eq!(
            names(&table),
            vec!["a", "b", "c"],
            "third click ascends again"
        );

        // A different column also starts at ascending even though Name was just descending.
        table.sort.clicked(VariableColumn::Display);
        assert_eq!(table.sort.key, VariableColumn::Display);
        assert!(table.sort.ascending, "a new column starts ascending");
        table.sort.clicked(VariableColumn::Display);
        assert!(!table.sort.ascending, "second click on Display descends");
        table.sort.clicked(VariableColumn::Name);
        assert_eq!(table.sort.key, VariableColumn::Name);
        assert!(table.sort.ascending, "returning to Name starts ascending");
        assert_eq!(names(&table), vec!["a", "b", "c"]);
    }

    /// Every sort is a permutation of the same rows, and tied keys stay in capture order in
    /// both directions (the ordinal tie-break; no unstable reorder and no lost row).
    #[test]
    fn sorting_is_stable_for_tied_keys_and_never_loses_a_row() {
        let state = EngineState {
            variables: vec![
                variable("z", "Natural", "7"),
                variable("m", "Natural", "7"),
                variable("a", "Natural", "7"),
                variable("n", "Text", "same"),
            ],
            functions: Vec::new(),
        };
        let mut table = VariableTable::new(&state);
        let ordinals = |table: &VariableTable| -> Vec<usize> {
            table
                .ordered()
                .into_iter()
                .map(|i| table.rows[i].ordinal)
                .collect()
        };

        // Descending by kind: Text before Natural, and the three Natural rows keep 0,1,2.
        table.sort.clicked(VariableColumn::Kind);
        table.sort.clicked(VariableColumn::Kind);
        assert!(!table.sort.ascending);
        let ordered = table.ordered();
        assert_eq!(ordered.len(), table.rows.len(), "no row may be dropped");
        let mut seen: Vec<usize> = ordered.iter().map(|i| table.rows[*i].ordinal).collect();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2, 3], "every row appears exactly once");
        assert_eq!(
            ordinals(&table),
            vec![3, 0, 1, 2],
            "the three Natural rows (capture order z, m, a) stay in capture order"
        );
    }

    /// WKS-002: the functions table reads origin from the engine's builtin flag and plugin
    /// name, joins the parameters with a comma, and carries the registry summary when present.
    #[test]
    fn functions_show_parameters_origin_and_summary() {
        let state = EngineState {
            variables: Vec::new(),
            functions: vec![
                function("aaa", &["x"], false, None, None),
                function("abs", &["x"], true, None, Some("Absolute value.")),
                function(
                    "acos",
                    &["x"],
                    true,
                    Some("Lovelace.Symbolics"),
                    Some("The principal arccosine."),
                ),
            ],
        };
        let table = FunctionTable::new(&state);
        assert_eq!(function_names(&table), vec!["aaa", "abs", "acos"]);

        let user = table.rows.iter().find(|row| row.name == "aaa").unwrap();
        assert_eq!(user.origin, FunctionOrigin::User);
        assert_eq!(user.origin_label(), "user");
        assert_eq!(user.parameters_text(), "x");
        assert_eq!(user.summary_cell(), None);

        let builtin = table.rows.iter().find(|row| row.name == "abs").unwrap();
        assert_eq!(builtin.origin, FunctionOrigin::Builtin);
        assert_eq!(builtin.origin_label(), "builtin");
        assert_eq!(builtin.summary_cell().as_deref(), Some("Absolute value."));

        let plugin = table.rows.iter().find(|row| row.name == "acos").unwrap();
        assert_eq!(plugin.origin, FunctionOrigin::Plugin);
        assert_eq!(plugin.origin_label(), "plugin");
        assert_eq!(plugin.plugin.as_deref(), Some("Lovelace.Symbolics"));
        assert_eq!(
            plugin.summary_cell().as_deref(),
            Some("The principal arccosine.")
        );
        assert_eq!(table.count_by_origin(FunctionOrigin::User), 1);
        assert_eq!(table.count_by_origin(FunctionOrigin::Builtin), 1);
        assert_eq!(table.count_by_origin(FunctionOrigin::Plugin), 1);
    }

    /// WKS-002 count agreement: the table is built from the snapshot's registry capture, so the
    /// displayed count is exactly the snapshot's function count (a legacy `functions` list that
    /// names only the user definition cannot shrink it).
    #[test]
    fn the_function_count_is_the_snapshot_registry_count() {
        let state = EngineState {
            variables: Vec::new(),
            functions: vec![
                function("aaa", &["x"], false, None, None),
                function("abs", &["x"], true, None, Some("Absolute value.")),
                function("acos", &["x"], true, Some("Lovelace.Symbolics"), None),
            ],
        };
        let legacy_functions = vec![("aaa".to_string(), vec!["x".to_string()])];
        let snapshot = crate::execution::WorkspaceSnapshot {
            result: String::new(),
            result_structured: None,
            result_inspection: None,
            variables: Vec::new(),
            functions: legacy_functions,
            state: state.clone(),
            plot: None,
            output: Vec::new(),
            revision: crate::execution::SourceRevision(1),
            committed_steps: 1,
            total: 1,
            precision: crate::engine::DEFAULT_PRECISION,
            diagnostic: None,
        };
        let table = FunctionTable::new(&snapshot.state);
        assert_eq!(table.rows.len(), 3);
        assert_eq!(table.rows.len(), snapshot.state.functions.len());
        assert_ne!(
            table.rows.len(),
            snapshot.functions.len(),
            "the table must use the registry capture, not the legacy user-only list"
        );
    }

    /// Large values are cut in the cell but the model keeps the full text for the tooltip.
    #[test]
    fn a_large_value_is_truncated_for_the_cell_and_kept_for_the_tooltip() {
        let long = "1234567890".repeat(10);
        let row = VariableRow {
            name: "big".to_string(),
            kind: "Array".to_string(),
            display: long.clone(),
            ordinal: 0,
        };
        let cell = row.display_cell();
        assert_eq!(cell.chars().count(), MAX_CELL_CHARS);
        assert!(
            cell.ends_with(ELLIPSIS),
            "cut cells end in an ellipsis: {cell:?}"
        );
        assert_eq!(row.display, long, "the tooltip text is not truncated");
        assert_eq!(cell_text("short"), "short");
    }

    /// A multi-byte value is cut at a character boundary, never inside a UTF-8 sequence.
    #[test]
    fn truncation_counts_characters_not_bytes() {
        let text = "é".repeat(MAX_CELL_CHARS + 5);
        let cut = cell_text(&text);
        assert_eq!(cut.chars().count(), MAX_CELL_CHARS);
        assert!(cut.ends_with(ELLIPSIS));
        assert!(cut.chars().all(|ch| ch == 'é' || ch == ELLIPSIS));
    }
}
