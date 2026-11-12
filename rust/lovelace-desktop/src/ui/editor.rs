//! Virtualised code editor: only the visible rows are laid out.
//!
//! Adapted from `rust/lovelace-desktop-spike/src/editor.rs` (unchanged behaviour) to edit one
//! document buffer supplied by the workbench instead of owning a list of sample documents.
//! The pane paints a line-number gutter, a current-line highlight, a caret and the syntax
//! layouter from [`super::syntax`]; the 1-based line/column readout is derived from
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
        }
    }

    /// Replaces the buffer from an external source (file load, tab switch back after a reload).
    pub fn load(&mut self, source: &str) {
        self.lines = split_lines(source);
        self.caret_row = self.caret_row.min(self.line_count().saturating_sub(1));
        self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        self.scroll_y = 0.0;
        self.completion.dismiss();
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

    /// The caret as 1-based line and column.
    pub fn line_col_1based(&self) -> (usize, usize) {
        (self.caret_row + 1, self.caret_col + 1)
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
        true
    }

    /// Moves the caret, clamping row and column into the buffer.
    #[allow(dead_code)] // part of the editor surface; the window reads the caret instead of setting it
    pub fn set_caret(&mut self, row: usize, col: usize) {
        self.caret_row = row.min(self.line_count().saturating_sub(1));
        self.caret_col = col.min(self.line_chars(self.caret_row));
    }

    /// Inserts text at the caret, splitting on newlines.
    pub fn insert_str(&mut self, text: &str) {
        for (index, part) in text.split('\n').enumerate() {
            if index > 0 {
                self.newline();
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

    /// Splits the current line at the caret.
    pub fn newline(&mut self) {
        let col = self.caret_col;
        let row = self.caret_row;
        let line = &mut self.lines[row];
        let byte = byte_index(line, col);
        let tail = line.split_off(byte);
        self.lines.insert(row + 1, tail);
        self.caret_row = row + 1;
        self.caret_col = 0;
        self.edits += 1;
    }

    /// Deletes the character before the caret (or joins with the previous line).
    pub fn backspace(&mut self) {
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
        let row = self.caret_row;
        let col = self.caret_col;
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

    /// Moves the caret up one row.
    pub fn move_up(&mut self) {
        if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        }
    }

    /// Moves the caret down one row.
    pub fn move_down(&mut self) {
        if self.caret_row + 1 < self.line_count() {
            self.caret_row += 1;
            self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        }
    }

    /// Moves the caret left, wrapping at the start of a line.
    pub fn move_left(&mut self) {
        if self.caret_col > 0 {
            self.caret_col -= 1;
        } else if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.line_chars(self.caret_row);
        }
    }

    /// Moves the caret right, wrapping at the end of a line.
    pub fn move_right(&mut self) {
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
    fn accept_completion(&mut self) {
        if let Some((row, col)) =
            self.completion
                .accept(&mut self.lines, self.caret_row, self.caret_col)
        {
            self.caret_row = row;
            self.caret_col = col;
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

    fn set_caret_from_pos(
        &mut self,
        position: egui::Pos2,
        body: Rect,
        row_h: f32,
        text_left: f32,
        char_w: f32,
    ) {
        let total = self.line_count().max(1);
        let row = row_at(body, self.scroll_y, row_h, total, position.y);
        let col = (((position.x - text_left) / char_w).round() as isize)
            .clamp(0, self.line_chars(row) as isize) as usize;
        self.caret_row = row;
        self.caret_col = col;
    }

    /// Draws and edits the buffer inside `rect`.
    ///
    /// `catalogue` is the live completion catalogue for the active document (the window
    /// rebuilds it from the engine's registry and the last committed snapshot each pass).
    pub fn ui(&mut self, ui: &mut Ui, rect: Rect, catalogue: &[CompletionItem]) -> EditorMetrics {
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
                        self.insert_str(&text);
                        self.completion.dismiss();
                    }
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => match key {
                        egui::Key::Backspace => {
                            self.completion.dismiss();
                            self.backspace();
                        }
                        egui::Key::Delete => {
                            self.completion.dismiss();
                            self.delete_forward();
                        }
                        egui::Key::Enter => {
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
                                self.insert_str("  ");
                            }
                        }
                        egui::Key::Escape => {
                            // Dismiss only: the buffer and the caret stay exactly as they were.
                            self.completion.dismiss();
                        }
                        egui::Key::ArrowLeft => {
                            self.completion.dismiss();
                            self.move_left();
                        }
                        egui::Key::ArrowRight => {
                            self.completion.dismiss();
                            self.move_right();
                        }
                        egui::Key::ArrowUp => {
                            if self.completion.is_open() {
                                self.completion.move_selection(-1);
                            } else {
                                self.move_up();
                            }
                        }
                        egui::Key::ArrowDown => {
                            if self.completion.is_open() {
                                self.completion.move_selection(1);
                            } else {
                                self.move_down();
                            }
                        }
                        egui::Key::Home => {
                            self.completion.dismiss();
                            self.caret_col = 0;
                        }
                        egui::Key::End => {
                            self.completion.dismiss();
                            self.caret_col = self.line_chars(self.caret_row);
                        }
                        _ => {}
                    },
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
                let row = row_at(rect, self.scroll_y, row_h, total, position.y);
                let on_marker = position.x < text_left
                    && decoration
                        .as_ref()
                        .is_some_and(|(range, _)| range.line == row + 1);
                if on_marker {
                    self.jump_to_diagnostic();
                } else {
                    self.set_caret_from_pos(position, rect, row_h, text_left, char_w);
                }
            }
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
        for row in first_row..last_row {
            let y = rect.top() - self.scroll_y + row as f32 * row_h;
            if y + row_h < rect.top() || y > rect.bottom() {
                continue;
            }
            let line = &self.lines[row];
            let job = syntax::job(line, font.clone(), &syntax::DARK);
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
