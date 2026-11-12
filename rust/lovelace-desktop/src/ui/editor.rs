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

use egui::{Align2, Color32, CursorIcon, FontId, Rect, Sense, Stroke, TextStyle, Ui, pos2, vec2};

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
        }
    }

    /// Replaces the buffer from an external source (file load, tab switch back after a reload).
    pub fn load(&mut self, source: &str) {
        self.lines = split_lines(source);
        self.caret_row = self.caret_row.min(self.line_count().saturating_sub(1));
        self.caret_col = self.caret_col.min(self.line_chars(self.caret_row));
        self.scroll_y = 0.0;
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

    fn set_caret_from_pos(
        &mut self,
        position: egui::Pos2,
        body: Rect,
        row_h: f32,
        text_left: f32,
        char_w: f32,
    ) {
        let total = self.line_count().max(1);
        let row = (((position.y - body.top() + self.scroll_y) / row_h).floor() as isize)
            .clamp(0, total as isize - 1) as usize;
        let col = (((position.x - text_left) / char_w).round() as isize)
            .clamp(0, self.line_chars(row) as isize) as usize;
        self.caret_row = row;
        self.caret_col = col;
    }

    /// Draws and edits the buffer inside `rect`.
    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) -> EditorMetrics {
        let started = Instant::now();
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
            if let Some(position) = response.interact_pointer_pos() {
                self.set_caret_from_pos(position, rect, row_h, text_left, char_w);
            }
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
            for event in events {
                match event {
                    egui::Event::Text(text) => self.insert_str(&text),
                    egui::Event::Paste(text) => self.insert_str(&text),
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => match key {
                        egui::Key::Backspace => self.backspace(),
                        egui::Key::Delete => self.delete_forward(),
                        egui::Key::Enter => {
                            // Ctrl+Enter is the workbench's Run shortcut; it must not also
                            // insert a newline into the document.
                            if !(modifiers.command || modifiers.ctrl) {
                                self.newline();
                            }
                        }
                        egui::Key::Tab => self.insert_str("  "),
                        egui::Key::ArrowLeft => self.move_left(),
                        egui::Key::ArrowRight => self.move_right(),
                        egui::Key::ArrowUp => self.move_up(),
                        egui::Key::ArrowDown => self.move_down(),
                        egui::Key::Home => self.caret_col = 0,
                        egui::Key::End => self.caret_col = self.line_chars(self.caret_row),
                        _ => {}
                    },
                    _ => {}
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
}

fn byte_index(line: &str, char_col: usize) -> usize {
    line.char_indices()
        .nth(char_col)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}
