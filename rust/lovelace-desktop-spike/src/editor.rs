//! Virtualised code editor: only the visible rows are ever laid out.
//!
//! The pane paints a line-number gutter, a current-line highlight, a caret and the
//! 1-based line/column readout is derived from `line_col_1based()`.
//!
//! Why not `TextEdit`? `TextEdit::layouter` is handed the *whole* buffer
//! (`&dyn TextBuffer`) once per pass and has no viewport argument, so a 5,000-line
//! document is laid out in full on every frame. The benchmark measures that cost
//! separately (`s1_textedit_*`) and compares it to this pane, which lays out
//! `visible_rows` rows per pass.

use std::time::Instant;

use egui::{
    Align2, Color32, CursorIcon, FontId, Rect, Sense, Stroke, TextStyle, Ui, pos2, vec2,
};

use crate::syntax;

pub const EDITOR_BG: Color32 = Color32::from_rgb(0x14, 0x18, 0x1d);
pub const GUTTER_BG: Color32 = Color32::from_rgb(0x10, 0x13, 0x18);
pub const CURRENT_LINE: Color32 = Color32::from_rgb(0x1f, 0x2a, 0x33);
pub const GUTTER_TEXT: Color32 = Color32::from_rgb(0x5c, 0x6b, 0x7d);
pub const CARET: Color32 = Color32::from_rgb(0xf0, 0xf4, 0xf8);

pub struct Doc {
    pub name: String,
    pub lines: Vec<String>,
    pub bytes: usize,
}

impl Doc {
    pub fn from_source(name: impl Into<String>, src: &str) -> Self {
        Doc {
            name: name.into(),
            lines: src.split('\n').map(|l| l.to_string()).collect(),
            bytes: src.len(),
        }
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn line_chars(&self, row: usize) -> usize {
        self.lines.get(row).map(|l| l.chars().count()).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Default)]
pub struct EditorMetrics {
    pub first_row: usize,
    pub visible_rows: usize,
    pub laid_out_rows: usize,
    pub row_h: f32,
    pub char_w: f32,
    pub gutter_w: f32,
    pub scroll_y: f32,
}

pub struct Editor {
    pub docs: Vec<Doc>,
    pub active: usize,
    pub caret_row: usize,
    pub caret_col: usize,
    pub scroll_y: f32,
    pub metrics: EditorMetrics,
    pub last_ui_ms: f64,
    pub edits: u64,
    /// Bench hook: ask for keyboard focus without a synthetic click.
    pub force_focus: bool,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            docs: Vec::new(),
            active: 0,
            caret_row: 0,
            caret_col: 0,
            scroll_y: 0.0,
            metrics: EditorMetrics::default(),
            last_ui_ms: 0.0,
            edits: 0,
            force_focus: false,
        }
    }
}

fn byte_index(line: &str, char_col: usize) -> usize {
    line.char_indices()
        .nth(char_col)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

impl Editor {
    pub fn doc(&self) -> &Doc {
        &self.docs[self.active]
    }

    pub fn doc_mut(&mut self) -> &mut Doc {
        &mut self.docs[self.active]
    }

    pub fn set_active(&mut self, idx: usize) {
        if idx < self.docs.len() {
            self.active = idx;
            self.caret_row = self.caret_row.min(self.doc().lines.len().saturating_sub(1));
            self.caret_col = self
                .caret_col
                .min(self.doc().line_chars(self.caret_row));
            self.scroll_y = 0.0;
        }
    }

    pub fn set_caret(&mut self, row: usize, col: usize) {
        self.caret_row = row.min(self.doc().lines.len().saturating_sub(1));
        self.caret_col = col.min(self.doc().line_chars(self.caret_row));
    }

    pub fn line_col_1based(&self) -> (usize, usize) {
        (self.caret_row + 1, self.caret_col + 1)
    }

    pub fn insert_str(&mut self, text: &str) {
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.newline();
            }
            if part.is_empty() {
                continue;
            }
            let col = self.caret_col;
            let line = &mut self.docs[self.active].lines[self.caret_row];
            let byte = byte_index(line, col);
            line.insert_str(byte, part);
            self.caret_col += part.chars().count();
        }
        self.edits += 1;
    }

    pub fn newline(&mut self) {
        let col = self.caret_col;
        let row = self.caret_row;
        let line = &mut self.docs[self.active].lines[row];
        let byte = byte_index(line, col);
        let tail = line.split_off(byte);
        self.docs[self.active].lines.insert(row + 1, tail);
        self.caret_row = row + 1;
        self.caret_col = 0;
        self.edits += 1;
    }

    pub fn backspace(&mut self) {
        if self.caret_col > 0 {
            let col = self.caret_col;
            let line = &mut self.docs[self.active].lines[self.caret_row];
            let end = byte_index(line, col);
            let start = line[..end]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            line.replace_range(start..end, "");
            self.caret_col -= 1;
        } else if self.caret_row > 0 {
            let row = self.caret_row;
            let tail = self.docs[self.active].lines.remove(row);
            let prev_len = self.docs[self.active].lines[row - 1].chars().count();
            self.docs[self.active].lines[row - 1].push_str(&tail);
            self.caret_row = row - 1;
            self.caret_col = prev_len;
        }
        self.edits += 1;
    }

    pub fn delete_forward(&mut self) {
        let row = self.caret_row;
        let col = self.caret_col;
        if col < self.doc().line_chars(row) {
            let line = &mut self.docs[self.active].lines[row];
            let start = byte_index(line, col);
            let end = line[start..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| start + i)
                .unwrap_or(line.len());
            line.replace_range(start..end, "");
        } else if row + 1 < self.doc().lines.len() {
            let tail = self.docs[self.active].lines.remove(row + 1);
            self.docs[self.active].lines[row].push_str(&tail);
        }
        self.edits += 1;
    }

    pub fn move_up(&mut self) {
        if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.caret_col.min(self.doc().line_chars(self.caret_row));
        }
    }

    pub fn move_down(&mut self) {
        if self.caret_row + 1 < self.doc().line_count() {
            self.caret_row += 1;
            self.caret_col = self.caret_col.min(self.doc().line_chars(self.caret_row));
        }
    }

    pub fn move_left(&mut self) {
        if self.caret_col > 0 {
            self.caret_col -= 1;
        } else if self.caret_row > 0 {
            self.caret_row -= 1;
            self.caret_col = self.doc().line_chars(self.caret_row);
        }
    }

    pub fn move_right(&mut self) {
        if self.caret_col < self.doc().line_chars(self.caret_row) {
            self.caret_col += 1;
        } else if self.caret_row + 1 < self.doc().line_count() {
            self.caret_row += 1;
            self.caret_col = 0;
        }
    }

    fn set_caret_from_pos(&mut self, p: egui::Pos2, body: Rect, row_h: f32, text_left: f32, char_w: f32) {
        let total = self.doc().line_count().max(1);
        let row = (((p.y - body.top() + self.scroll_y) / row_h).floor() as isize)
            .clamp(0, total as isize - 1) as usize;
        let col = (((p.x - text_left) / char_w).round() as isize)
            .clamp(0, self.doc().line_chars(row) as isize) as usize;
        self.caret_row = row;
        self.caret_col = col;
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) -> EditorMetrics {
        let t0 = Instant::now();
        let font = FontId::monospace(13.0);
        let row_h = ui.text_style_height(&TextStyle::Monospace) + 3.0;
        let char_w = ui
            .painter()
            .layout_no_wrap("M".to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
            .max(1.0);
        let total = self.doc().line_count().max(1);
        let digits = total.to_string().len().max(4);
        let gutter_w = 10.0 + digits as f32 * char_w;
        let text_left = rect.left() + gutter_w + 6.0;
        let max_scroll = (total as f32 * row_h - rect.height()).max(0.0);

        let id = egui::Id::unique("editor_surface");
        let resp = ui.interact(rect, id, Sense::click_and_drag());
        if self.force_focus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        if resp.clicked() {
            resp.request_focus();
            if let Some(p) = resp.interact_pointer_pos() {
                self.set_caret_from_pos(p, rect, row_h, text_left, char_w);
            }
        }
        let focused = ui.memory(|m| m.has_focus(id));
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.scroll_y = (self.scroll_y - scroll).clamp(0.0, max_scroll);
            }
        }

        if focused {
            let events = ui.input(|i| i.events.clone());
            for ev in events {
                match ev {
                    egui::Event::Text(t) => self.insert_str(&t),
                    egui::Event::Paste(t) => self.insert_str(&t),
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => match key {
                        egui::Key::Backspace => self.backspace(),
                        egui::Key::Delete => self.delete_forward(),
                        egui::Key::Enter => self.newline(),
                        egui::Key::Tab => self.insert_str("  "),
                        egui::Key::ArrowLeft => self.move_left(),
                        egui::Key::ArrowRight => self.move_right(),
                        egui::Key::ArrowUp => self.move_up(),
                        egui::Key::ArrowDown => self.move_down(),
                        egui::Key::Home => self.caret_col = 0,
                        egui::Key::End => self.caret_col = self.doc().line_chars(self.caret_row),
                        _ => {
                            let _ = modifiers;
                        }
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
            Rect::from_min_size(
                pos2(rect.left(), row_top),
                vec2(rect.width(), row_h),
            ),
            0.0,
            CURRENT_LINE,
        );

        let mut laid_out = 0usize;
        for row in first_row..last_row {
            let y = rect.top() - self.scroll_y + row as f32 * row_h;
            if y + row_h < rect.top() || y > rect.bottom() {
                continue;
            }
            let line = &self.doc().lines[row];
            let job = syntax::job(line, font.clone(), &syntax::DARK);
            let galley = painter.layout_job(job);
            laid_out += 1;
            let text_x = rect.left() + gutter_w + 6.0;
            painter.galley(pos2(text_x, y), galley, syntax::DARK.plain);
            painter.text(
                pos2(rect.left() + gutter_w - 4.0, y),
                Align2::RIGHT_TOP,
                (row + 1).to_string(),
                FontId::monospace(11.0),
                GUTTER_TEXT,
            );
        }

        if focused {
            let cx = rect.left() + gutter_w + 6.0 + self.caret_col as f32 * char_w;
            let cy = rect.top() - self.scroll_y + self.caret_row as f32 * row_h;
            painter.line_segment(
                [pos2(cx, cy + 1.0), pos2(cx, cy + row_h - 1.0)],
                Stroke::new(1.5, CARET),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, Color32::from_rgb(0x4e, 0xc9, 0xb0)),
                egui::StrokeKind::Inside,
            );
        }

        let (line1, col1) = self.line_col_1based();
        let label = format!(
            "Script editor, {} lines, caret line {} column {}",
            total, line1, col1
        );
        resp.widget_info(|| {
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
        self.last_ui_ms = t0.elapsed().as_secs_f64() * 1000.0;
        self.metrics
    }
}

