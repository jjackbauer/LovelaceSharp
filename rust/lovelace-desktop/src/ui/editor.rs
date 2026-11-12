//! Virtualised code editor: only the visible rows are laid out.
//!
//! Adapted from `rust/lovelace-desktop-spike/src/editor.rs` (unchanged behaviour) to edit one
//! document buffer supplied by the workbench instead of owning a list of sample documents.
//! The pane paints a line-number gutter, a current-line highlight, a caret and the syntax
//! layouter from [`super::syntax`]; the 1-based line and UTF-16 column readout is derived from
//! [`TextEditor::line_col_1based`].
//!
//! Why not `TextEdit`? `TextEdit::layouter` is handed the *whole* buffer once per pass and has
//! no viewport argument, so a 5 000-line document is laid out in full on every frame. This pane
//! lays out only `metrics.laid_out_rows` rows per pass.

use std::time::Instant;

use egui::{
    Align2, Color32, CursorIcon, FontId, Rect, RichText, Sense, Stroke, TextStyle, Ui, pos2, vec2,
};

use crate::completion::{self, CaretContext, CompletionItem, CompletionKind, CompletionSession};
use crate::diagnostic_map::{self, DiagnosticRange};
use crate::execution::Diagnostic;
use crate::history::{self, Caret, EditKind, History, HistoryCommand, Snapshot};

use super::syntax;

/// Editor background.
pub const EDITOR_BG: Color32 = Color32::from_rgb(0x14, 0x18, 0x1d);
/// Gutter background.
pub const GUTTER_BG: Color32 = Color32::from_rgb(0x10, 0x13, 0x18);
/// Current-line highlight.
pub const CURRENT_LINE: Color32 = Color32::from_rgb(0x1f, 0x2a, 0x33);
/// Gutter line numbers.
pub const GUTTER_TEXT: Color32 = Color32::from_rgb(0x5c, 0x6b, 0x7d);
/// Caret colour.
pub const CARET: Color32 = Color32::from_rgb(0xf0, 0xf4, 0xf8);
/// Selection highlight, painted behind the selected text.
pub const SELECTION: Color32 = Color32::from_rgb(0x28, 0x4a, 0x5c);
/// Diagnostic underline colour.
pub const DIAGNOSTIC_UNDERLINE: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);
/// Diagnostic gutter-marker colour; a little deeper than the underline so it reads as a marker.
pub const DIAGNOSTIC_MARKER: Color32 = Color32::from_rgb(0xe0, 0x4b, 0x45);
/// Completion popup background: one step above the pane background so it reads as an overlay.
pub const COMPLETION_BG: Color32 = Color32::from_rgb(0x1b, 0x22, 0x2a);
/// Completion popup border, in the accent colour.
pub const COMPLETION_BORDER: Color32 = Color32::from_rgb(0x4e, 0xc9, 0xb0);
/// The highlighted completion row.
pub const COMPLETION_SELECTED: Color32 = Color32::from_rgb(0x2b, 0x4a, 0x47);
/// The detail column of a completion row.
pub const COMPLETION_DETAIL: Color32 = Color32::from_rgb(0x8b, 0x98, 0xa9);
/// User-defined and host functions: a distinct accent from the builtins, which use the
/// highlighter's builtin blue. This is how the popup marks user functions apart.
pub const COMPLETION_FUNCTION: Color32 = Color32::from_rgb(0x4e, 0xc9, 0xb0);
/// Variables use the plain text colour; their detail carries the kind.
pub const COMPLETION_VARIABLE: Color32 = Color32::from_rgb(0xd8, 0xde, 0xe9);
/// Height of one completion row, in pixels.
const COMPLETION_ROW_H: f32 = 18.0;
/// The popup never grows past this many rows; the highlight scrolls the window inside it.
const COMPLETION_MAX_ROWS: usize = 9;

/// What one editor pass laid out, for the status strip and for measurement.
#[allow(dead_code)] // the benchmark-side fields stay part of the measured editor surface
#[derive(Clone, Copy, Default)]
pub struct EditorMetrics {
    /// First visible row (0-based).
    pub first_row: usize,
    /// Rows that fit in the viewport.
    pub visible_rows: usize,
    /// Rows actually laid out this pass.
    pub laid_out_rows: usize,
    /// Row height in pixels.
    pub row_h: f32,
    /// Monospace character width in pixels.
    pub char_w: f32,
    /// Gutter width in pixels.
    pub gutter_w: f32,
    /// Vertical scroll offset in pixels.
    pub scroll_y: f32,
}

/// One editable buffer with its own caret and scroll position.
pub struct TextEditor {
    /// Source lines, always at least one (an empty document has one empty line).
    pub lines: Vec<String>,
    /// Caret row, 0-based.
    pub caret_row: usize,
    /// Caret column in characters, 0-based.
    pub caret_col: usize,
    /// Vertical scroll offset in pixels.
    pub scroll_y: f32,
    /// Last pass's metrics.
    pub metrics: EditorMetrics,
    /// Time the last pass took, in milliseconds.
    pub last_ui_ms: f64,
    /// Number of source-mutating edits, used by the window to detect a changed buffer.
    pub edits: u64,
    /// The failed run's diagnostic to decorate, while it describes the current buffer.
    ///
    /// The window copies this from [`crate::workbench::Workbench::diagnostic`], which is already
    /// revision-gated; the editor additionally drops it on the same pass as an edit and refuses
    /// to paint a line past the buffer.
    pub diagnostic: Option<Diagnostic>,
    /// The live completion popup: filtered entries, highlight and the prefix acceptance replaces.
    pub completion: CompletionSession,
    /// This document's undo/redo history.
    ///
    /// The window owns one editor per tab, so the history belongs to exactly one document: an
    /// undo in one tab can never pop a step recorded in another. A typing burst is coalesced by
    /// [`self::history`], and every step restores the buffer, the caret and the selection.
    pub history: History,
    /// The fixed end of the current selection; the caret is the moving end.
    ///
    /// `None` means no selection. When this equals the caret position the selection is empty and
    /// [`TextEditor::selection_range`] reports none.
    pub selection: Option<(usize, usize)>,
    /// The builtins and symbols the current paint pass classifies. Refreshed from the live
    /// engine snapshot and the document's `symbol("...")` calls, so the highlighter holds no
    /// second builtin or symbol list.
    syntax: syntax::SyntaxContext,
    /// The `edits` counter the cached symbol set was discovered at; `u64::MAX` forces a refresh.
    syntax_edits: u64,
    /// The builtin labels the cached context was built from, for the per-pass change check.
    syntax_builtins: Vec<String>,
}

/// The stable egui id of the editor surface.
///
/// The window requests focus for this id on its first frame so a script typed immediately after
/// startup goes into the editor without a click. `Id::unique` is deterministic, so the same id
/// is used by every pass.
pub fn surface_id() -> egui::Id {
    egui::Id::unique("editor_surface")
}

fn split_lines(source: &str) -> Vec<String> {
    source.split('\n').map(|line| line.to_string()).collect()
}

impl TextEditor {
    /// Creates an editor for `source`, with the caret at the start.
    pub fn from_source(source: &str) -> TextEditor {
        TextEditor {
            lines: split_lines(source),
            caret_row: 0,
            caret_col: 0,
            scroll_y: 0.0,
            metrics: EditorMetrics::default(),
            last_ui_ms: 0.0,
            edits: 0,
            diagnostic: None,
            completion: CompletionSession::default(),
            history: History::new(),
            selection: None,
            syntax: syntax::SyntaxContext::new(),
            syntax_edits: u64::MAX,
            syntax_builtins: Vec::new(),
        }
    }

    /// Replaces the buffer from an external source (file load, tab switch back after a reload).
    ///
    /// The history is discarded: its snapshots name rows and columns of the text that was just
    /// replaced, so keeping them would let an undo move the caret into unrelated text.
    pub fn load(&mut self, source: &str) {
        self.lines = split_lines(source);
        self.caret_row = self.caret_row.min(self.line_count().saturating_sub(1));
        self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        self.scroll_y = 0.0;
        self.selection = None;
        self.history.reset();
        self.completion.dismiss();
        // The source changed without going through an edit method, so the cached symbol set must
        // be rediscovered on the next paint pass.
        self.syntax_edits = u64::MAX;
    }

    /// The current buffer as source text. Exactly the text supplied to [`TextEditor::load`]
    /// round-trips, including a trailing newline.
    pub fn source(&self) -> String {
        self.lines.join("\n")
    }

    /// Number of lines (an empty buffer has one empty line).
    pub fn line_count(&self) -> usize {
        self.lines.len().max(1)
    }

    /// Byte size of the current source.
    pub fn bytes(&self) -> usize {
        let newlines = self.lines.len().saturating_sub(1);
        self.lines.iter().map(|line| line.len()).sum::<usize>() + newlines
    }

    /// Character count of a row.
    pub fn line_chars(&self, row: usize) -> usize {
        self.lines
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    /// The caret as 1-based line and 1-based **UTF-16** column.
    ///
    /// Columns count UTF-16 code units, the engine's and the diagnostic mapping's unit, so an
    /// astral character is two columns wide. The caret itself is a 0-based char index (the
    /// editor's cell unit); this converts at the readout boundary.
    pub fn line_col_1based(&self) -> (usize, usize) {
        (self.caret_row + 1, self.caret_column_utf16() + 1)
    }

    /// The caret's 0-based UTF-16 code-unit column on its line.
    ///
    /// The caret is stored as a 0-based char index (the editor's cell unit), but the engine,
    /// [`TextEditor::diagnostic_range`] and the status strip all count columns in UTF-16 code
    /// units: an astral character is two units wide. This is the conversion at that boundary.
    pub fn caret_column_utf16(&self) -> usize {
        diagnostic_map::utf16_column(&self.lines, self.caret_row, self.caret_col)
            .unwrap_or_default()
    }

    /// The active diagnostic resolved to a concrete line span, if its line is still in the buffer.
    pub fn diagnostic_range(&self) -> Option<DiagnosticRange> {
        let diagnostic = self.diagnostic.as_ref()?;
        diagnostic_map::range_for(&self.lines, diagnostic.line, diagnostic.column)
    }

    /// Moves the caret to the diagnostic's reported position and returns true when one was drawn.
    ///
    /// This is the gutter-marker click and F8 action: it puts the editor's caret on the exact
    /// 1-based line and UTF-16 column the engine named, not merely somewhere on the line.
    pub fn jump_to_diagnostic(&mut self) -> bool {
        let Some(range) = self.diagnostic_range() else {
            return false;
        };
        self.caret_row = range.line - 1;
        self.caret_col = range.start_char;
        self.selection = None;
        self.history.break_burst();
        true
    }

    /// Moves the caret, clamping row and column into the buffer and clearing any selection.
    #[allow(dead_code)] // part of the editor surface; the window reads the caret instead of setting it
    pub fn set_caret(&mut self, row: usize, col: usize) {
        self.caret_row = row.min(self.line_count().saturating_sub(1));
        self.caret_col = col.min(self.line_chars(self.caret_row));
        self.selection = None;
        self.history.break_burst();
    }

    /// The caret as a [`Caret`] value.
    pub fn caret(&self) -> Caret {
        Caret {
            row: self.caret_row,
            col: self.caret_col,
        }
    }

    /// The current editor state, which is exactly what one undo step stores.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            caret: self.caret(),
            selection: self.selection.map(|(row, col)| Caret { row, col }),
        }
    }

    /// Restores one history snapshot: the text, the caret and the selection together.
    ///
    /// The snapshot was a valid editor state, but the row/column clamps are still applied so a
    /// future edit to the snapshot format can never point past the buffer. The completion popup
    /// is dismissed because the caret it was positioned for has moved.
    fn restore_snapshot(&mut self, snapshot: Snapshot) {
        self.lines = snapshot.lines;
        self.caret_row = snapshot.caret.row.min(self.line_count().saturating_sub(1));
        self.caret_col = snapshot.caret.col.min(self.line_chars(self.caret_row));
        self.selection = snapshot.selection.map(|anchor| {
            let row = anchor.row.min(self.line_count().saturating_sub(1));
            (row, anchor.col.min(self.line_chars(row)))
        });
        if self.selection == Some((self.caret_row, self.caret_col)) {
            self.selection = None;
        }
        self.completion.dismiss();
        self.edits += 1;
    }

    /// Starts one undoable edit, recording the pre-edit state when it opens a new step.
    ///
    /// A coalesced typing character records nothing: the open burst's step already holds the
    /// state the run started from, so typing does not clone the buffer per keystroke.
    fn begin_edit(&mut self, kind: EditKind) {
        self.begin_edit_at(kind, Instant::now());
    }

    /// [`TextEditor::begin_edit`] with an explicit clock, for deterministic burst tests.
    fn begin_edit_at(&mut self, kind: EditKind, now: Instant) {
        if self.history.begin(kind, now) {
            let before = self.snapshot();
            self.history.push(before);
        }
    }

    /// Applies one undo or redo command. Returns true when the buffer changed.
    pub fn command(&mut self, command: HistoryCommand) -> bool {
        match command {
            HistoryCommand::Undo => self.undo(),
            HistoryCommand::Redo => self.redo(),
        }
    }

    /// Restores the state before the last edit. Returns false when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        if self.history.undo_steps() == 0 {
            return false;
        }
        let current = self.snapshot();
        let Some(previous) = self.history.undo(current) else {
            return false;
        };
        self.restore_snapshot(previous);
        true
    }

    /// Restores the state the last undo replaced. Returns false when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        if self.history.redo_steps() == 0 {
            return false;
        }
        let current = self.snapshot();
        let Some(next) = self.history.redo(current) else {
            return false;
        };
        self.restore_snapshot(next);
        true
    }

    /// The selection as `(start, end)` buffer positions in reading order, when non-empty.
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        self.ordered_selection()
    }

    /// The selection as byte offsets in [`TextEditor::source`], when non-empty.
    ///
    /// [`TextEditor::selection_range`] is stored in char columns (the editor paints one char per
    /// cell); the engine and the diagnostics use UTF-16 code units and the buffer is UTF-8. This
    /// converts both endpoints from char space to byte space in one place, so the byte range
    /// slices exactly the characters the editor highlights.
    #[allow(dead_code)] // part of the editor surface; tests and a future find/replace read it
    pub fn selection_bytes(&self) -> Option<(usize, usize)> {
        let (start, end) = self.ordered_selection()?;
        diagnostic_map::selection_byte_range(&self.lines, start, end)
    }

    /// The selected text, when a non-empty range is selected.
    #[allow(dead_code)] // part of the editor surface; tests and a future find/replace read it
    pub fn selected_text(&self) -> Option<String> {
        let ((start_row, start_col), (end_row, end_col)) = self.ordered_selection()?;
        if start_row == end_row {
            let line = self.lines.get(start_row)?;
            return Some(
                line.chars()
                    .skip(start_col)
                    .take(end_col - start_col)
                    .collect(),
            );
        }
        let mut parts = vec![
            self.lines
                .get(start_row)?
                .chars()
                .skip(start_col)
                .collect::<String>(),
        ];
        for row in start_row + 1..end_row {
            parts.push(self.lines.get(row)?.clone());
        }
        parts.push(
            self.lines
                .get(end_row)?
                .chars()
                .take(end_col)
                .collect::<String>(),
        );
        Some(parts.join("\n"))
    }

    /// Selects from the caret to `(row, col)` (shift-click semantics) and moves the caret there.
    ///
    /// The existing anchor is kept when a selection is already active, so a second shift-click
    /// extends from the original fixed end rather than from the moved caret.
    pub fn select_to(&mut self, row: usize, col: usize) {
        let anchor = self.selection.unwrap_or((self.caret_row, self.caret_col));
        let row = row.min(self.line_count().saturating_sub(1));
        let col = col.min(self.line_chars(row));
        self.caret_row = row;
        self.caret_col = col;
        self.selection = if anchor == (row, col) {
            None
        } else {
            Some(anchor)
        };
        self.completion.dismiss();
        self.history.break_burst();
    }

    /// The selection ordered from the start of the buffer to its end, when non-empty.
    fn ordered_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.selection?;
        let caret = (self.caret_row, self.caret_col);
        if anchor == caret {
            return None;
        }
        Some(if anchor < caret {
            (anchor, caret)
        } else {
            (caret, anchor)
        })
    }

    /// Removes the selected range as part of the caller's edit. Returns true when it removed one.
    fn delete_selection(&mut self) -> bool {
        let Some(((start_row, start_col), (end_row, end_col))) = self.ordered_selection() else {
            return false;
        };
        let prefix: String = self.lines[start_row].chars().take(start_col).collect();
        let suffix: String = self.lines[end_row].chars().skip(end_col).collect();
        self.lines
            .splice(start_row..=end_row, [format!("{prefix}{suffix}")]);
        self.caret_row = start_row;
        self.caret_col = start_col;
        self.selection = None;
        true
    }

    /// Inserts typed text at the caret, splitting on newlines.
    ///
    /// Consecutive typed insertions coalesce into one undo step until a pause or a non-typing
    /// action. A non-empty selection is replaced as part of the same step.
    pub fn insert_str(&mut self, text: &str) {
        self.insert_text(text, EditKind::Typing);
    }

    /// Inserts `text` as one undoable edit of `kind`.
    fn insert_text(&mut self, text: &str, kind: EditKind) {
        self.insert_text_at(text, kind, Instant::now());
    }

    /// [`TextEditor::insert_text`] with an explicit clock, for deterministic burst tests.
    fn insert_text_at(&mut self, text: &str, kind: EditKind, now: Instant) {
        if text.is_empty() && self.ordered_selection().is_none() {
            return;
        }
        self.begin_edit_at(kind, now);
        self.delete_selection();
        for (index, part) in text.split('\n').enumerate() {
            if index > 0 {
                self.split_line_at_caret();
            }
            if part.is_empty() {
                continue;
            }
            let col = self.caret_col;
            let line = &mut self.lines[self.caret_row];
            let byte = byte_index(line, col);
            line.insert_str(byte, part);
            self.caret_col += part.chars().count();
        }
        self.edits += 1;
    }

    /// Splits the current line at the caret without touching the history or the edit counter.
    fn split_line_at_caret(&mut self) {
        let col = self.caret_col;
        let row = self.caret_row;
        let line = &mut self.lines[row];
        let byte = byte_index(line, col);
        let tail = line.split_off(byte);
        self.lines.insert(row + 1, tail);
        self.caret_row = row + 1;
        self.caret_col = 0;
    }

    /// Splits the current line at the caret as its own undoable edit.
    pub fn newline(&mut self) {
        self.begin_edit(EditKind::Atomic);
        self.delete_selection();
        self.split_line_at_caret();
        self.edits += 1;
    }

    /// Deletes the character before the caret (or joins with the previous line).
    pub fn backspace(&mut self) {
        if self.ordered_selection().is_some() {
            self.begin_edit(EditKind::Atomic);
            self.delete_selection();
            self.edits += 1;
            return;
        }
        if self.caret_col == 0 && self.caret_row == 0 {
            // Nothing to delete, but the key press still ends an open typing burst.
            self.history.break_burst();
            return;
        }
        self.begin_edit(EditKind::Atomic);
        if self.caret_col > 0 {
            let col = self.caret_col;
            let line = &mut self.lines[self.caret_row];
            let end = byte_index(line, col);
            let start = line[..end]
                .char_indices()
                .last()
                .map(|(index, _)| index)
                .unwrap_or(0);
            line.replace_range(start..end, "");
            self.caret_col -= 1;
        } else if self.caret_row > 0 {
            let row = self.caret_row;
            let tail = self.lines.remove(row);
            let previous_len = self.lines[row - 1].chars().count();
            self.lines[row - 1].push_str(&tail);
            self.caret_row = row - 1;
            self.caret_col = previous_len;
        }
        self.edits += 1;
    }

    /// Deletes the character after the caret (or joins with the next line).
    pub fn delete_forward(&mut self) {
        if self.ordered_selection().is_some() {
            self.begin_edit(EditKind::Atomic);
            self.delete_selection();
            self.edits += 1;
            return;
        }
        let row = self.caret_row;
        let col = self.caret_col;
        let can_delete = col < self.line_chars(row) || row + 1 < self.line_count();
        if !can_delete {
            self.history.break_burst();
            return;
        }
        self.begin_edit(EditKind::Atomic);
        if col < self.line_chars(row) {
            let line = &mut self.lines[row];
            let start = byte_index(line, col);
            let end = line[start..]
                .char_indices()
                .nth(1)
                .map(|(index, _)| start + index)
                .unwrap_or(line.len());
            line.replace_range(start..end, "");
        } else if row + 1 < self.line_count() {
            let tail = self.lines.remove(row + 1);
            self.lines[row].push_str(&tail);
        }
        self.edits += 1;
    }

    /// Extends the selection back to where the caret was, or collapses it when they meet.
    ///
    /// This is the Shift+arrow / Shift+Home / Shift+End rule: the first extended move makes the
    /// old caret the anchor, and every following one moves only the caret.
    fn extend_selection_from(&mut self, before: (usize, usize)) {
        if self.selection.is_none() {
            self.selection = Some(before);
        }
        if self.selection == Some((self.caret_row, self.caret_col)) {
            self.selection = None;
        }
        self.history.break_burst();
    }

    /// Moves the caret up one row. A caret move ends an open typing burst.
    pub fn move_up(&mut self) {
        self.history.break_burst();
        if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        }
    }

    /// Moves the caret down one row. A caret move ends an open typing burst.
    pub fn move_down(&mut self) {
        self.history.break_burst();
        if self.caret_row + 1 < self.line_count() {
            self.caret_row += 1;
            self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        }
    }

    /// Moves the caret left, wrapping at the start of a line. A caret move ends a typing burst.
    pub fn move_left(&mut self) {
        self.history.break_burst();
        if self.caret_col > 0 {
            self.caret_col -= 1;
        } else if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.line_chars(self.caret_row);
        }
    }

    /// Moves the caret right, wrapping at the end of a line. A caret move ends a typing burst.
    pub fn move_right(&mut self) {
        self.history.break_burst();
        if self.caret_col < self.line_chars(self.caret_row) {
            self.caret_col += 1;
        } else if self.caret_row + 1 < self.line_count() {
            self.caret_row += 1;
            self.caret_col = 0;
        }
    }

    /// Opens completion for the caret's identifier prefix; Ctrl+Space also opens it with an
    /// empty prefix (the whole catalogue). Strings and comments suppress it even explicitly.
    fn open_completion(&mut self, catalogue: &[CompletionItem]) {
        let line = self.lines[self.caret_row].clone();
        if completion::caret_context(&line, self.caret_col) != CaretContext::Code {
            self.completion.dismiss();
            return;
        }
        let (prefix, start) = completion::identifier_prefix(&line, self.caret_col)
            .map(|prefix| (prefix.text, prefix.start_col))
            .unwrap_or_else(|| (String::new(), self.caret_col));
        self.completion.show(catalogue, &prefix, start);
    }

    /// Refreshes completion after typed text: opens on an identifier prefix, closes otherwise.
    ///
    /// This is the "automatically after typing an identifier prefix of at least one character"
    /// trigger. A character typed inside a string or comment (or after punctuation, or into a
    /// number) closes the popup instead of showing a stale filter.
    fn refresh_completion(&mut self, catalogue: &[CompletionItem]) {
        let line = self.lines[self.caret_row].clone();
        if completion::caret_context(&line, self.caret_col) != CaretContext::Code {
            self.completion.dismiss();
            return;
        }
        match completion::identifier_prefix(&line, self.caret_col) {
            Some(prefix) => {
                self.completion.show(catalogue, &prefix.text, prefix.start_col);
            }
            None => self.completion.dismiss(),
        }
    }

    /// Accepts the highlighted entry, replacing exactly the identifier prefix.
    ///
    /// The acceptance is one atomic undo step: the snapshot is taken before the prefix is
    /// touched, so one Ctrl+Z restores the typed prefix and its caret and does not also undo the
    /// typing that produced it.
    fn accept_completion(&mut self) {
        let before = self.snapshot();
        if let Some((row, col)) =
            self.completion
                .accept(&mut self.lines, self.caret_row, self.caret_col)
        {
            if self.history.begin(EditKind::Atomic, Instant::now()) {
                self.history.push(before);
            }
            self.caret_row = row;
            self.caret_col = col;
            self.selection = None;
            self.edits += 1;
        }
    }

    /// The one-line completion state the pane header publishes (and the capture records).
    ///
    /// It is a plain egui label in the header, so the typed prefix, the selected signature and
    /// the entry's origin are part of the window's accessibility tree even though the popup
    /// itself is painted.
    pub fn completion_summary(&self) -> Option<String> {
        let item = self.completion.selected()?;
        Some(format!(
            "Completion: {} -> {} [{}] {}/{}",
            self.completion.prefix(),
            item.detail,
            item.kind.tag(),
            self.completion.selected_index() + 1,
            self.completion.items().len()
        ))
    }

    /// The buffer position under a pointer position, clamped into the buffer.
    fn caret_at(
        &self,
        position: egui::Pos2,
        body: Rect,
        row_h: f32,
        text_left: f32,
        char_w: f32,
    ) -> (usize, usize) {
        let total = self.line_count().max(1);
        let row = row_at(body, self.scroll_y, row_h, total, position.y);
        let col = (((position.x - text_left) / char_w).round() as isize)
            .clamp(0, self.line_chars(row) as isize) as usize;
        (row, col)
    }

    /// Places the caret under a pointer position and clears any selection.
    fn set_caret_from_pos(
        &mut self,
        position: egui::Pos2,
        body: Rect,
        row_h: f32,
        text_left: f32,
        char_w: f32,
    ) {
        let (row, col) = self.caret_at(position, body, row_h, text_left, char_w);
        self.caret_row = row;
        self.caret_col = col;
        self.selection = None;
    }

    /// Refreshes the syntax context from the live engine snapshot and the current buffer.
    ///
    /// The snapshot's `is_builtin` functions are the engine registry's own labels (the same
    /// capture completion offers), so the highlighter has no builtin table of its own. The
    /// snapshot is used directly rather than the deduplicated catalogue: a variable named like a
    /// builtin must not hide that builtin's colour. The symbol set is rediscovered only when the
    /// buffer's edit counter moved; the builtin comparison is a cheap per-pass scan.
    fn refresh_syntax(&mut self, engine: &crate::engine::EngineState) {
        let builtins: Vec<&str> = engine
            .functions
            .iter()
            .filter(|function| function.is_builtin)
            .map(|function| function.name.as_str())
            .collect();
        let unchanged = builtins.len() == self.syntax_builtins.len()
            && builtins
                .iter()
                .zip(self.syntax_builtins.iter())
                .all(|(live, cached)| *live == cached.as_str());
        if !unchanged {
            self.syntax.set_builtins(builtins.iter().copied());
            self.syntax_builtins = builtins.into_iter().map(str::to_string).collect();
        }
        if self.syntax_edits != self.edits {
            self.syntax
                .set_symbols(syntax::discover_symbols(&self.source()));
            self.syntax_edits = self.edits;
        }
    }

    /// Draws and edits the buffer inside `rect`.
    ///
    /// `catalogue` is the live completion catalogue for the active document (the window
    /// rebuilds it from the engine's registry and the last committed snapshot each pass);
    /// `engine` is the same capture the catalogue was built from, and supplies the highlighter's
    /// builtin names.
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        rect: Rect,
        catalogue: &[CompletionItem],
        engine: &crate::engine::EngineState,
    ) -> EditorMetrics {
        let started = Instant::now();
        let edits_before = self.edits;
        let font = FontId::monospace(13.0);
        let row_h = ui.text_style_height(&TextStyle::Monospace) + 3.0;
        let char_w = ui
            .painter()
            .layout_no_wrap("M".to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
            .max(1.0);
        let total = self.line_count();
        let digits = total.to_string().len().max(4);
        let gutter_w = 10.0 + digits as f32 * char_w;
        let text_left = rect.left() + gutter_w + 6.0;
        let max_scroll = (total as f32 * row_h - rect.height()).max(0.0);

        let id = surface_id();
        let response = ui.interact(rect, id, Sense::click_and_drag());
        if response.clicked() {
            response.request_focus();
        }
        let focused = ui.memory(|memory| memory.has_focus(id));
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.scroll_y = (self.scroll_y - scroll).clamp(0.0, max_scroll);
            }
        }

        if focused {
            let events = ui.input(|input| input.events.clone());
            // Ctrl+Space opens the popup explicitly, with the caret's prefix or the whole
            // catalogue when the prefix is empty. Some backends also deliver the space as a
            // Text event; that one is swallowed so the shortcut can never insert a space.
            let mut explicit = false;
            for event in &events {
                if let egui::Event::Key {
                    key: egui::Key::Space,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if modifiers.command || modifiers.ctrl {
                        explicit = true;
                    }
                }
            }
            let mut swallow_space = explicit;
            if explicit {
                // Ctrl+Space is a non-typing action: it ends an open typing burst.
                self.history.break_burst();
                self.open_completion(catalogue);
            }
            for event in events {
                match event {
                    egui::Event::Text(text) => {
                        if swallow_space && text == " " {
                            swallow_space = false;
                            continue;
                        }
                        self.insert_str(&text);
                        self.refresh_completion(catalogue);
                    }
                    egui::Event::Paste(text) => {
                        // A paste is one atomic step: it never joins an open typing burst.
                        self.insert_text(&text, EditKind::Atomic);
                        self.completion.dismiss();
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        // Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y are the editor's own history
                        // shortcuts. They are read through the shared mapping so the window
                        // and `--gui-selftest` exercise the same rule.
                        if let Some(command) = history::command_for_shortcut(
                            matches!(key, egui::Key::Z),
                            matches!(key, egui::Key::Y),
                            modifiers.shift,
                            modifiers.command || modifiers.ctrl,
                        ) {
                            self.command(command);
                            self.completion.dismiss();
                            continue;
                        }
                        // Only keys the editor treats as actions end a typing burst. A letter
                        // key also arrives as a Key event, but its Text event is the typing
                        // edit: breaking on the Key would split every typed word.
                        match key {
                            egui::Key::Backspace => {
                                self.completion.dismiss();
                                self.backspace();
                            }
                            egui::Key::Delete => {
                                self.completion.dismiss();
                                self.delete_forward();
                            }
                            egui::Key::Enter => {
                                // A command or a newline both end the typing burst; Ctrl+Enter
                                // (Run) must not let the next typed character join it.
                                self.history.break_burst();
                                // Ctrl+Enter is the workbench's Run shortcut; it must not also
                                // insert a newline into the document, and it never accepts a
                                // completion.
                                if !(modifiers.command || modifiers.ctrl) {
                                    if self.completion.is_open() {
                                        self.accept_completion();
                                    } else {
                                        self.newline();
                                    }
                                }
                            }
                            egui::Key::Tab => {
                                if self.completion.is_open() {
                                    self.accept_completion();
                                } else {
                                    // Indentation is its own atomic step, not part of a typing
                                    // burst, so one undo removes the whole two-space indent.
                                    self.insert_text("  ", EditKind::Atomic);
                                }
                            }
                            egui::Key::Escape => {
                                // Dismiss only: the buffer and the caret stay exactly as they
                                // were.
                                self.history.break_burst();
                                self.completion.dismiss();
                            }
                            egui::Key::ArrowLeft => {
                                self.history.break_burst();
                                self.completion.dismiss();
                                let before = (self.caret_row, self.caret_col);
                                self.move_left();
                                if modifiers.shift {
                                    self.extend_selection_from(before);
                                } else {
                                    self.selection = None;
                                }
                            }
                            egui::Key::ArrowRight => {
                                self.history.break_burst();
                                self.completion.dismiss();
                                let before = (self.caret_row, self.caret_col);
                                self.move_right();
                                if modifiers.shift {
                                    self.extend_selection_from(before);
                                } else {
                                    self.selection = None;
                                }
                            }
                            egui::Key::ArrowUp => {
                                self.history.break_burst();
                                if self.completion.is_open() {
                                    self.completion.move_selection(-1);
                                } else {
                                    let before = (self.caret_row, self.caret_col);
                                    self.move_up();
                                    if modifiers.shift {
                                        self.extend_selection_from(before);
                                    } else {
                                        self.selection = None;
                                    }
                                }
                            }
                            egui::Key::ArrowDown => {
                                self.history.break_burst();
                                if self.completion.is_open() {
                                    self.completion.move_selection(1);
                                } else {
                                    let before = (self.caret_row, self.caret_col);
                                    self.move_down();
                                    if modifiers.shift {
                                        self.extend_selection_from(before);
                                    } else {
                                        self.selection = None;
                                    }
                                }
                            }
                            egui::Key::Home => {
                                self.history.break_burst();
                                self.completion.dismiss();
                                let before = (self.caret_row, self.caret_col);
                                self.caret_col = 0;
                                if modifiers.shift {
                                    self.extend_selection_from(before);
                                } else {
                                    self.selection = None;
                                }
                            }
                            egui::Key::End => {
                                self.history.break_burst();
                                self.completion.dismiss();
                                let before = (self.caret_row, self.caret_col);
                                self.caret_col = self.line_chars(self.caret_row);
                                if modifiers.shift {
                                    self.extend_selection_from(before);
                                } else {
                                    self.selection = None;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        if focused {
            // Keep the arrow keys for the caret. egui's focus system otherwise treats a bare
            // arrow as "move keyboard focus to the closest widget in that direction" before the
            // editor can consume it, which drops focus out of the editor after one Up/Down
            // (DEF-002). While the popup is open, Tab and Escape belong to it too: Tab accepts
            // a completion instead of moving focus, and Escape dismisses the popup instead of
            // surrendering focus. The filter is stored for the NEXT pass, so it is written here,
            // after the events, when this pass's popup state is final; otherwise the Escape that
            // dismisses the popup would be seen by egui first and surrender the editor's focus.
            let completion_open = self.completion.is_open();
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    id,
                    egui::EventFilter {
                        tab: completion_open,
                        escape: completion_open,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                    },
                );
            });
        }

        // An edit invalidates the diagnostic's coordinates: it names a line and column in the
        // text that was run. Drop it in this same pass so a deleted line cannot be decorated and
        // no marker is ever painted against text the failure never named.
        if self.edits != edits_before {
            self.diagnostic = None;
        }

        // Editing shrinks or grows the buffer (Backspace at a line start removes a line). The
        // loop above may have changed `lines`, so recompute the bounds this pass paints with;
        // the values captured at the top of the pass describe the pre-edit buffer and would let
        // the paint loop index a line that no longer exists (DEF-001).
        let total = self.line_count();
        let max_scroll = (total as f32 * row_h - rect.height()).max(0.0);

        // Resolve the diagnostic against the post-edit buffer. `range_for` refuses a line past
        // the buffer, so the decoration can never outlive the text it describes.
        let decoration: Option<(DiagnosticRange, String)> =
            self.diagnostic.as_ref().and_then(|diagnostic| {
                diagnostic_map::range_for(&self.lines, diagnostic.line, diagnostic.column)
                    .map(|range| (range, diagnostic.render()))
            });

        // A pointer click in the gutter marker jumps to the reported line and column; any other
        // pointer click positions the caret as before and dismisses the popup (the caret left the
        // prefix it was filtered for). A keyboard activation of this focused surface also reports
        // `clicked()`, but has no pointer position; it must not be mistaken for a click here —
        // Ctrl+Space is exactly such an activation.
        if response.clicked() {
            if let Some(position) = response.interact_pointer_pos() {
                self.completion.dismiss();
                self.history.break_burst();
                let row = row_at(rect, self.scroll_y, row_h, total, position.y);
                let on_marker = position.x < text_left
                    && decoration
                        .as_ref()
                        .is_some_and(|(range, _)| range.line == row + 1);
                if on_marker {
                    self.jump_to_diagnostic();
                } else if ui.input(|input| input.modifiers.shift) {
                    // Shift+click extends the selection from its fixed end (or from the old
                    // caret when none was selected).
                    let (row, col) = self.caret_at(position, rect, row_h, text_left, char_w);
                    self.select_to(row, col);
                } else {
                    self.set_caret_from_pos(position, rect, row_h, text_left, char_w);
                }
            }
        }

        // A pointer drag selects text: the press origin is the fixed end and the moving pointer
        // the caret. A plain click (no drag) positions the caret and clears the selection above.
        if response.drag_started() {
            if let Some(origin) = ui.input(|input| input.pointer.press_origin()) {
                let (row, col) = self.caret_at(origin, rect, row_h, text_left, char_w);
                self.completion.dismiss();
                self.selection = Some((row, col));
                self.caret_row = row;
                self.caret_col = col;
                self.history.break_burst();
            }
        }
        if response.dragged() {
            if let Some(position) = response.interact_pointer_pos() {
                let (row, col) = self.caret_at(position, rect, row_h, text_left, char_w);
                self.caret_row = row;
                self.caret_col = col;
                self.history.break_burst();
            }
        }
        if response.drag_stopped() && self.selection == Some((self.caret_row, self.caret_col)) {
            // A press that never moved past the drag threshold is not a selection.
            self.selection = None;
        }

        // Keep the caret inside the viewport.
        let caret_y = self.caret_row as f32 * row_h;
        if caret_y < self.scroll_y {
            self.scroll_y = caret_y;
        } else if caret_y + row_h > self.scroll_y + rect.height() {
            self.scroll_y = caret_y + row_h - rect.height();
        }
        self.scroll_y = self.scroll_y.clamp(0.0, max_scroll);

        let first_row = (self.scroll_y / row_h).floor().max(0.0) as usize;
        let visible_rows = ((rect.height() / row_h).ceil() as usize + 1).max(1);
        let last_row = (first_row + visible_rows).min(total);

        // Refresh after this pass's typing events, so a symbol declared by the character just
        // typed is painted in the same frame rather than one frame late.
        self.refresh_syntax(engine);

        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, EDITOR_BG);
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(gutter_w, rect.height())),
            0.0,
            GUTTER_BG,
        );

        // Current-line highlight.
        let row_top = rect.top() - self.scroll_y + self.caret_row as f32 * row_h;
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left(), row_top), vec2(rect.width(), row_h)),
            0.0,
            CURRENT_LINE,
        );

        let mut laid_out = 0usize;
        // The selection is painted under the text, one row at a time, so a multi-line range
        // reads as one continuous highlight and never covers a glyph.
        let selection = self.selection_range();
        for row in first_row..last_row {
            let y = rect.top() - self.scroll_y + row as f32 * row_h;
            if y + row_h < rect.top() || y > rect.bottom() {
                continue;
            }
            if let Some((start, end)) = selection {
                if row >= start.0 && row <= end.0 {
                    let from = if row == start.0 { start.1 } else { 0 };
                    let to = if row == end.0 {
                        end.1
                    } else {
                        self.line_chars(row)
                    };
                    if to > from {
                        painter.rect_filled(
                            Rect::from_min_max(
                                pos2(text_left + from as f32 * char_w, y + 1.0),
                                pos2(text_left + to as f32 * char_w, y + row_h - 1.0),
                            ),
                            1.0,
                            SELECTION,
                        );
                    }
                }
            }
            let line = &self.lines[row];
            let job = syntax::job(line, font.clone(), &syntax::DARK, &self.syntax);
            let galley = painter.layout_job(job);
            laid_out += 1;
            painter.galley(pos2(text_left, y), galley, syntax::DARK.plain);
            painter.text(
                pos2(rect.left() + gutter_w - 4.0, y),
                Align2::RIGHT_TOP,
                (row + 1).to_string(),
                FontId::monospace(11.0),
                GUTTER_TEXT,
            );
            if let Some((range, _)) = &decoration {
                if range.line == row + 1 {
                    // Underline the offending span, from the reported column to end-of-line.
                    // An empty span (column at EOL) still gets one cell so the failure is visible.
                    let x0 = text_left + range.start_char as f32 * char_w;
                    let cells = range.end_char.saturating_sub(range.start_char).max(1);
                    let x1 = x0 + cells as f32 * char_w;
                    painter.rect_filled(
                        Rect::from_min_max(pos2(x0, y + row_h - 3.0), pos2(x1, y + row_h - 1.0)),
                        1.0,
                        DIAGNOSTIC_UNDERLINE,
                    );
                    // A distinct gutter marker on the same line: a solid bar at the gutter edge,
                    // clear of the right-aligned line number.
                    painter.rect_filled(
                        Rect::from_min_max(
                            pos2(rect.left() + 2.0, y + 2.0),
                            pos2(rect.left() + 6.0, y + row_h - 2.0),
                        ),
                        1.0,
                        DIAGNOSTIC_MARKER,
                    );
                }
            }
        }

        // Hovering the underlined line (or its gutter marker) shows the diagnostic message. This
        // is a tooltip attached to the pointer, not a widget, so it cannot steal the editor's
        // click or its keyboard focus.
        if response.hovered() {
            if let (Some((range, message)), Some(pointer)) = (
                decoration.as_ref(),
                ui.input(|input| input.pointer.hover_pos()),
            ) {
                let row = row_at(rect, self.scroll_y, row_h, total, pointer.y);
                let row_top = rect.top() - self.scroll_y + row as f32 * row_h;
                let on_decorated_line =
                    range.line == row + 1 && pointer.y >= row_top && pointer.y < row_top + row_h;
                if on_decorated_line {
                    let _ = egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        ui.layer_id(),
                        egui::Id::unique("editor_diagnostic_tooltip"),
                        egui::PopupAnchor::Pointer,
                    )
                    .at_pointer()
                    .show(|ui| {
                        ui.set_max_width(520.0);
                        ui.label(RichText::new(message).monospace());
                    });
                }
            }
        }

        if focused {
            let caret_x = text_left + self.caret_col as f32 * char_w;
            let caret_y = rect.top() - self.scroll_y + self.caret_row as f32 * row_h;
            painter.line_segment(
                [
                    pos2(caret_x, caret_y + 1.0),
                    pos2(caret_x, caret_y + row_h - 1.0),
                ],
                Stroke::new(1.5, CARET),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0x4e, 0xc9, 0xb0)),
                egui::StrokeKind::Inside,
            );
        }

        // The completion popup paints last, above the text and the caret. It is a pure painter
        // overlay: no widget, no interaction, no focus, so it can never block typing or swallow
        // a click. It is anchored one row BELOW the caret line (or, when that side has no room,
        // above it), so it does not cover the line being typed on.
        self.paint_completion(ui, rect, text_left, row_h, char_w);
        #[cfg(test)]
        eprintln!(
            "[dbg] editor end open={} items={} pass={}",
            self.completion.is_open(),
            self.completion.items().len(),
            ui.ctx().cumulative_pass_nr()
        );

        let (line, column) = self.line_col_1based();
        let label = format!(
            "Script editor, {} lines, caret line {} column {}",
            total, line, column
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, label.clone())
        });

        self.metrics = EditorMetrics {
            first_row,
            visible_rows,
            laid_out_rows: laid_out,
            row_h,
            char_w,
            gutter_w,
            scroll_y: self.scroll_y,
        };
        self.last_ui_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.metrics
    }

    /// Paints the completion popup over the editor surface.
    ///
    /// The popup is painter-only on purpose: it owns no widget, requests no focus and senses no
    /// pointer, so typing continues to reach the editor and no click is swallowed. Placement
    /// prefers the free space below the caret row and falls back to the free space above it;
    /// the number of rows is capped by that free space, so the popup never covers the caret
    /// line unless the pane is shorter than one row (in which case nothing is painted).
    fn paint_completion(&self, ui: &Ui, rect: Rect, text_left: f32, row_h: f32, char_w: f32) {
        let items = self.completion.items();
        if !self.completion.is_open() || items.is_empty() {
            return;
        }
        let selected = self.completion.selected_index();
        let font = FontId::monospace(12.0);
        let detail_font = FontId::monospace(11.0);
        let label_char_w = ui
            .painter()
            .layout_no_wrap("M".to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
            .max(1.0);
        let detail_char_w = ui
            .painter()
            .layout_no_wrap("M".to_owned(), detail_font.clone(), Color32::WHITE)
            .size()
            .x
            .max(1.0);
        let label_cols = items
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let detail_cols = items
            .iter()
            .map(|item| item.detail.chars().count())
            .max()
            .unwrap_or(0) as f32;
        let padding = 8.0;
        let column_gap = 14.0;
        let wanted =
            label_cols * label_char_w + column_gap + detail_cols * detail_char_w + padding * 2.0 + 4.0;
        let max_width = (rect.width() - 8.0).max(120.0);
        let width = wanted.clamp(180.0f32.min(max_width), max_width);
        let height_for = |rows: usize| rows as f32 * COMPLETION_ROW_H + 6.0;

        let caret_top = rect.top() - self.scroll_y + self.caret_row as f32 * row_h;
        let below_top = caret_top + row_h + 3.0;
        let above_top = rect.top() + 4.0;
        let below_space = (rect.bottom() - 4.0 - below_top).max(0.0);
        let above_space = (caret_top - 3.0 - above_top).max(0.0);
        let place_below = below_space >= above_space;
        let space = if place_below { below_space } else { above_space };
        let capacity = ((space - 6.0) / COMPLETION_ROW_H).floor().max(0.0) as usize;
        let visible = items.len().min(COMPLETION_MAX_ROWS).min(capacity);
        if visible == 0 {
            return;
        }
        let height = height_for(visible);
        let y = if place_below {
            below_top
        } else {
            caret_top - height - 3.0
        };
        let caret_x = text_left + self.caret_col as f32 * char_w;
        let mut x = caret_x;
        let right_limit = rect.right() - 4.0 - width;
        if x > right_limit {
            x = right_limit;
        }
        x = x.max(rect.left() + 4.0);

        // Scroll the visible window so the highlighted row stays inside it.
        let first = if selected < visible {
            0
        } else {
            (selected + 1 - visible).min(items.len() - visible)
        };

        let painter = ui.painter().with_clip_rect(rect);
        let popup = Rect::from_min_size(pos2(x, y), vec2(width, height));
        painter.rect_filled(popup, 3.0, COMPLETION_BG);
        painter.rect_stroke(
            popup,
            3.0,
            Stroke::new(1.0, COMPLETION_BORDER),
            egui::StrokeKind::Inside,
        );
        for (offset, item) in items[first..first + visible].iter().enumerate() {
            let index = first + offset;
            let row_top = popup.top() + 3.0 + offset as f32 * COMPLETION_ROW_H;
            let row = Rect::from_min_size(
                pos2(popup.left() + 1.0, row_top),
                vec2(popup.width() - 2.0, COMPLETION_ROW_H),
            );
            if index == selected {
                painter.rect_filled(row, 2.0, COMPLETION_SELECTED);
            }
            let accent = completion_colour(item.kind);
            // A two-pixel kind bar: builtins keep the highlighter's blue, user functions and
            // variables get their own colour, keywords the highlighter's keyword colour.
            painter.rect_filled(
                Rect::from_min_size(
                    pos2(row.left() + 3.0, row_top + 3.0),
                    vec2(2.0, COMPLETION_ROW_H - 6.0),
                ),
                1.0,
                accent,
            );
            painter.text(
                pos2(popup.left() + padding, row_top + 2.0),
                Align2::LEFT_TOP,
                &item.label,
                font.clone(),
                accent,
            );
            painter.text(
                pos2(
                    popup.left() + padding + label_cols * label_char_w + column_gap,
                    row_top + 3.0,
                ),
                Align2::LEFT_TOP,
                &item.detail,
                detail_font.clone(),
                COMPLETION_DETAIL,
            );
        }
    }
}

/// The colour and kind bar one completion entry is painted with.
fn completion_colour(kind: CompletionKind) -> Color32 {
    match kind {
        CompletionKind::Keyword => syntax::DARK.keyword,
        CompletionKind::Builtin => syntax::DARK.builtin,
        CompletionKind::Function => COMPLETION_FUNCTION,
        CompletionKind::Variable => COMPLETION_VARIABLE,
    }
}

fn byte_index(line: &str, char_col: usize) -> usize {
    line.char_indices()
        .nth(char_col)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}

/// The 0-based buffer row under a screen `y` position, clamped into a one-line-minimum buffer.
fn row_at(rect: Rect, scroll_y: f32, row_h: f32, total: usize, y: f32) -> usize {
    (((y - rect.top() + scroll_y) / row_h).floor() as isize).clamp(0, total.max(1) as isize - 1)
        as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::build_catalogue;
    use crate::engine::EngineFunction;
    use std::time::Duration;

    fn builtin(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: true,
            plugin: None,
            summary: None,
        }
    }

    fn caret(editor: &TextEditor) -> (usize, usize) {
        (editor.caret_row, editor.caret_col)
    }

    #[test]
    fn syntax_context_refreshes_builtins_from_the_engine_and_symbols_from_the_buffer() {
        let state = crate::engine::EngineState {
            variables: Vec::new(),
            functions: vec![builtin("squeeze", &["a"])],
        };
        let mut editor = TextEditor::from_source("t = symbol(\"t\")");
        editor.refresh_syntax(&state);
        assert!(
            editor.syntax.is_builtin("squeeze"),
            "the engine's builtin is used"
        );
        assert!(
            editor.syntax.is_symbol("t"),
            "the buffer's symbol is discovered"
        );
        assert!(
            !editor.syntax.is_builtin("round"),
            "a name absent from the engine stays plain"
        );

        // A buffer replacement invalidates the cached symbol set.
        editor.load("u = symbol(\"u\")");
        editor.refresh_syntax(&state);
        assert!(editor.syntax.is_symbol("u"));
        assert!(!editor.syntax.is_symbol("t"));

        // A changed engine snapshot refreshes the builtin set without a buffer edit.
        let state = crate::engine::EngineState {
            variables: Vec::new(),
            functions: vec![builtin("round", &["x"])],
        };
        editor.refresh_syntax(&state);
        assert!(editor.syntax.is_builtin("round"));
        assert!(!editor.syntax.is_builtin("squeeze"));
    }

    #[test]
    fn typing_burst_is_one_step_and_undo_redo_restore_the_caret() {
        let mut editor = TextEditor::from_source("");
        editor.insert_str("a");
        editor.insert_str("b");
        editor.insert_str("c");

        assert_eq!(editor.source(), "abc");
        assert_eq!(caret(&editor), (0, 3));
        assert_eq!(
            editor.history.undo_steps(),
            1,
            "three typed characters in one burst are one undo step"
        );

        assert!(editor.undo());
        assert_eq!(editor.source(), "", "Ctrl+Z restores the empty buffer");
        assert_eq!(caret(&editor), (0, 0), "and the caret at 0");

        assert!(editor.redo());
        assert_eq!(editor.source(), "abc", "Ctrl+Y restores the burst");
        assert_eq!(caret(&editor), (0, 3), "and the caret after the text");
    }

    #[test]
    fn a_pause_starts_a_new_typing_step() {
        let mut editor = TextEditor::from_source("");
        let start = Instant::now();
        editor.insert_text_at("ab", EditKind::Typing, start);
        editor.insert_text_at("cd", EditKind::Typing, start + Duration::from_millis(300));
        assert_eq!(
            editor.history.undo_steps(),
            1,
            "characters within the burst gap coalesce"
        );

        let after_pause = start
            + Duration::from_millis(300)
            + history::TYPING_BURST_GAP
            + Duration::from_millis(1);
        editor.insert_text_at("ef", EditKind::Typing, after_pause);
        assert_eq!(
            editor.history.undo_steps(),
            2,
            "a pause longer than the gap starts a new step"
        );
        assert!(editor.undo());
        assert_eq!(editor.source(), "abcd");
        assert_eq!(caret(&editor), (0, 4));
        assert!(editor.undo());
        assert_eq!(editor.source(), "");
    }

    #[test]
    fn a_non_typing_action_starts_a_new_step() {
        let mut editor = TextEditor::from_source("");
        editor.insert_str("ab");
        editor.newline();
        editor.insert_str("cd");
        assert_eq!(editor.source(), "ab\ncd");
        assert_eq!(editor.history.undo_steps(), 3);

        assert!(editor.undo());
        assert_eq!(editor.source(), "ab\n", "the second burst is its own step");
        assert_eq!(caret(&editor), (1, 0));
        assert!(editor.undo());
        assert_eq!(editor.source(), "ab");
        assert_eq!(caret(&editor), (0, 2));
        assert!(editor.undo());
        assert_eq!(editor.source(), "");
    }

    #[test]
    fn backspace_line_join_undo_restores_the_caret_on_line_two() {
        let mut editor = TextEditor::from_source("a\nb");
        editor.set_caret(1, 0);
        editor.backspace();
        assert_eq!(editor.source(), "ab");
        assert_eq!(caret(&editor), (0, 1));

        assert!(editor.undo());
        assert_eq!(editor.source(), "a\nb", "the joined newline is restored");
        assert_eq!(caret(&editor), (1, 0), "the caret is back on line 2");
        assert_eq!(editor.line_col_1based(), (2, 1));
    }

    #[test]
    fn a_caret_move_between_typing_runs_starts_a_new_step() {
        let mut editor = TextEditor::from_source("");
        editor.insert_str("ab");
        editor.move_left();
        editor.insert_str("X");
        assert_eq!(editor.source(), "aXb");
        assert_eq!(caret(&editor), (0, 2));
        assert_eq!(editor.history.undo_steps(), 2);

        assert!(editor.undo());
        assert_eq!(editor.source(), "ab", "the second run is its own step");
        assert_eq!(caret(&editor), (0, 1), "the moved caret is restored");
        assert!(editor.undo());
        assert_eq!(editor.source(), "");
    }

    #[test]
    fn deleting_a_selection_undoes_to_the_text_and_the_selection() {
        let mut editor = TextEditor::from_source("abcdef");
        editor.set_caret(0, 1);
        editor.select_to(0, 4);
        assert_eq!(editor.selected_text().as_deref(), Some("bcd"));

        editor.delete_forward();
        assert_eq!(editor.source(), "aef");
        assert_eq!(caret(&editor), (0, 1));
        assert!(editor.selection_range().is_none());

        assert!(editor.undo());
        assert_eq!(editor.source(), "abcdef", "the deleted text is restored");
        assert_eq!(
            editor.selection_range(),
            Some(((0, 1), (0, 4))),
            "the selection is restored with the text"
        );
        assert_eq!(editor.selected_text().as_deref(), Some("bcd"));
        assert_eq!(caret(&editor), (0, 4));
    }

    /// EDT-014 (2): an astral character is three selection chars but six bytes and four UTF-16
    /// units. The byte range must slice exactly `a😀b`, and the caret readout must be the UTF-16
    /// column (5, 1-based), not the 3-char editor cell.
    #[test]
    fn astral_selection_converts_to_exact_bytes_and_the_caret_utf16_column() {
        let mut editor = TextEditor::from_source("a\u{1f600}b");
        editor.select_to(0, 3); // the whole line in the editor's char cells
        assert_eq!(
            editor.selection_range(),
            Some(((0, 0), (0, 3))),
            "the selection is stored in char columns: 'a', the emoji and 'b'"
        );

        let (start, end) = editor.selection_bytes().expect("a non-empty selection");
        assert_eq!(
            (start, end),
            (0, 6),
            "'a' is 1 byte, U+1F600 is 4 bytes and 'b' is 1 byte"
        );
        assert_eq!(&editor.source()[start..end], "a\u{1f600}b");
        assert_eq!(editor.selected_text().as_deref(), Some("a\u{1f600}b"));

        // The caret sits after the selection, so it is four UTF-16 units into the line.
        assert_eq!(editor.caret_column_utf16(), 4);
        assert_eq!(
            editor.line_col_1based(),
            (1, 5),
            "the readout column is 1-based UTF-16, not the 3-char caret column"
        );
    }

    /// EDT-014: a combining mark is a second `char` in the same grapheme cluster. Selecting the
    /// cluster must cover its full three UTF-8 bytes, not only the base character's one byte.
    #[test]
    fn a_combining_mark_selection_covers_the_whole_clusters_bytes() {
        let mut editor = TextEditor::from_source("e\u{0301} = 1");
        editor.select_to(0, 2); // 'e' plus U+0301: two chars, one grapheme on screen
        let (start, end) = editor.selection_bytes().expect("a non-empty selection");
        assert_eq!((start, end), (0, 3), "'e' is 1 byte and U+0301 is 2 bytes");
        assert_eq!(&editor.source()[start..end], "e\u{0301}");
        assert_eq!(
            editor.caret_column_utf16(),
            2,
            "both code points are one UTF-16 unit, so the caret is at unit 2"
        );
    }

    #[test]
    fn a_multiline_paste_is_one_undo_step() {
        let mut editor = TextEditor::from_source("x");
        editor.set_caret(0, 1);
        // `Event::Paste` calls exactly this atomic insertion.
        editor.insert_text("a\nb", EditKind::Atomic);
        assert_eq!(editor.source(), "xa\nb");
        assert_eq!(caret(&editor), (1, 1));
        assert_eq!(editor.history.undo_steps(), 1);

        assert!(editor.undo());
        assert_eq!(editor.source(), "x", "the whole paste is one step");
        assert_eq!(caret(&editor), (0, 1));
    }

    #[test]
    fn a_completion_insertion_is_its_own_undo_step() {
        let mut editor = TextEditor::from_source("");
        let catalogue = build_catalogue(&[], &[builtin("sqrt", &["x"])], &[]);
        editor.insert_str("sq");
        assert!(editor.completion.show(&catalogue, "sq", 0));

        editor.accept_completion();
        assert_eq!(editor.source(), "sqrt");
        assert_eq!(caret(&editor), (0, 4));
        assert!(
            !editor.completion.is_open(),
            "acceptance dismisses the popup"
        );

        assert!(editor.undo());
        assert_eq!(
            editor.source(),
            "sq",
            "one undo removes the insertion without the typing that produced the prefix"
        );
        assert_eq!(caret(&editor), (0, 2));
        assert!(!editor.completion.is_open());

        assert!(editor.undo());
        assert_eq!(editor.source(), "", "the typing is the next step back");
    }

    #[test]
    fn each_editor_has_its_own_history() {
        let mut first = TextEditor::from_source("");
        let mut second = TextEditor::from_source("");
        first.insert_str("alpha");
        second.insert_str("beta");

        assert!(first.undo());
        assert_eq!(first.source(), "");
        assert_eq!(
            second.source(),
            "beta",
            "an undo in one document never changes another document"
        );
        assert!(second.undo());
        assert_eq!(second.source(), "");
        assert!(
            !second.undo(),
            "the other document's history was never shared"
        );
    }
}
