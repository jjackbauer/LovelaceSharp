//! The spike UI: four panes, a tab strip, splitters, the editor, the SVG plot, logs,
//! accessibility labels, and the windowed bench hooks for S7.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::{
    Align, Align2, Color32, CursorIcon, FontId, Id, Key, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2,
};

use crate::cpu;
use crate::editor::{Doc, Editor};
use crate::plot::Plot;
use crate::sample;

pub const PANE_BG: Color32 = Color32::from_rgb(0x17, 0x1b, 0x21);
pub const HEADER_BG: Color32 = Color32::from_rgb(0x22, 0x28, 0x30);
pub const BORDER: Color32 = Color32::from_rgb(0x33, 0x3a, 0x44);
pub const TEXT: Color32 = Color32::from_rgb(0xd8, 0xde, 0xe9);
pub const DIM: Color32 = Color32::from_rgb(0x8b, 0x98, 0xa9);
pub const ACCENT: Color32 = Color32::from_rgb(0x4e, 0xc9, 0xb0);
pub const SPLIT_BG: Color32 = Color32::from_rgb(0x0d, 0x10, 0x14);
pub const SPLIT_HOT: Color32 = Color32::from_rgb(0x2b, 0x4a, 0x47);

pub const SPLIT: f32 = 6.0;
pub const MIN_EDITOR: f32 = 320.0;
pub const MIN_PLOT: f32 = 240.0;
pub const MIN_WORKSPACE: f32 = 180.0;
pub const MIN_LOGS: f32 = 100.0;
pub const MIN_TOP: f32 = 240.0;

#[derive(Clone, Copy, Debug)]
pub struct PaneLayout {
    pub editor: Rect,
    pub plot: Rect,
    pub workspace: Rect,
    pub logs: Rect,
    pub split_a: Rect,
    pub split_b: Rect,
    pub split_c: Rect,
    pub full: Rect,
}

pub fn compute_layout(
    full: Rect,
    editor_frac: f32,
    workspace_frac: f32,
    logs_frac: f32,
) -> PaneLayout {
    // Minimum sizes are honoured whenever there is room. If the window is smaller than
    // the sum of the minimums, the minimums themselves are compressed proportionally so
    // the panes always fit the available rectangle instead of overlapping.
    let usable_w = (full.width() - 2.0 * SPLIT).max(0.0);
    let min_sum = MIN_EDITOR + MIN_PLOT + MIN_WORKSPACE;
    let shrink_w = if usable_w >= min_sum {
        1.0
    } else {
        usable_w / min_sum
    };
    let min_editor = MIN_EDITOR * shrink_w;
    let min_plot = MIN_PLOT * shrink_w;
    let min_workspace = MIN_WORKSPACE * shrink_w;

    let max_editor = (usable_w - min_plot - min_workspace).max(min_editor);
    let editor_w = (usable_w * editor_frac).clamp(min_editor, max_editor);
    let remaining = (usable_w - editor_w).max(0.0);
    let max_workspace = (remaining - min_plot).max(min_workspace);
    let workspace_w = (usable_w * workspace_frac).clamp(min_workspace, max_workspace);
    let plot_w = (remaining - workspace_w).max(0.0);

    let usable_h = (full.height() - SPLIT).max(0.0);
    let h_min_sum = MIN_TOP + MIN_LOGS;
    let shrink_h = if usable_h >= h_min_sum {
        1.0
    } else {
        usable_h / h_min_sum
    };
    let min_top = MIN_TOP * shrink_h;
    let min_logs = MIN_LOGS * shrink_h;
    let max_logs = (usable_h - min_top).max(min_logs);
    let logs_h = (full.height() * logs_frac).clamp(min_logs, max_logs);
    let top_h = (usable_h - logs_h).max(0.0);

    let top_rect = Rect::from_min_size(full.min, vec2(full.width(), top_h));
    let editor = Rect::from_min_size(top_rect.min, vec2(editor_w, top_h));
    let split_a = Rect::from_min_size(pos2(editor.right(), top_rect.top()), vec2(SPLIT, top_h));
    let plot = Rect::from_min_size(pos2(split_a.right(), top_rect.top()), vec2(plot_w, top_h));
    let split_b = Rect::from_min_size(pos2(plot.right(), top_rect.top()), vec2(SPLIT, top_h));
    let workspace =
        Rect::from_min_size(pos2(split_b.right(), top_rect.top()), vec2(workspace_w, top_h));
    let split_c =
        Rect::from_min_size(pos2(full.left(), top_rect.bottom()), vec2(full.width(), SPLIT));
    let logs = Rect::from_min_size(pos2(full.left(), split_c.bottom()), vec2(full.width(), logs_h));

    PaneLayout {
        editor,
        plot,
        workspace,
        logs,
        split_a,
        split_b,
        split_c,
        full,
    }
}

/// True when every pane and splitter lies inside the available rectangle, with no
/// overlap between neighbours. Used as a measured invariant by the benchmark.
pub fn layout_fits(l: &PaneLayout) -> bool {
    let eps = 0.01;
    let f = l.full;
    let within = |r: Rect| {
        r.left() >= f.left() - eps
            && r.top() >= f.top() - eps
            && r.right() <= f.right() + eps
            && r.bottom() <= f.bottom() + eps
    };
    within(l.editor)
        && within(l.plot)
        && within(l.workspace)
        && within(l.logs)
        && l.editor.right() <= l.split_a.left() + eps
        && l.split_a.right() <= l.plot.left() + eps
        && l.plot.right() <= l.split_b.left() + eps
        && l.split_b.right() <= l.workspace.left() + eps
        && l.workspace.right() <= f.right() + eps
        && l.workspace.bottom() <= l.split_c.top() + eps
        && l.plot.bottom() <= l.split_c.top() + eps
        && l.editor.bottom() <= l.split_c.top() + eps
        && l.split_c.bottom() <= l.logs.top() + eps
}
/// S7 windowed measurement record, filled by the app and printed by `--bench`.
#[derive(Clone, Default)]
pub struct WindowBenchResult {
    pub startup_first_ui_pass_ms: Option<f64>,
    pub startup_first_ui_build_ms: Option<f64>,
    pub startup_first_paint_ms: Option<f64>,
    pub idle_wall_ms: f64,
    pub idle_cpu_ms: Option<f64>,
    pub idle_cpu_pct: Option<f64>,
    pub idle_frames: u64,
    pub idle_pass: u64,
    pub window_error: Option<String>,
}

struct IdleState {
    start: Instant,
    cpu0: Option<Duration>,
    pass0: u64,
}

pub struct SpikeApp {
    pub editor: Editor,
    pub plot: Plot,
    pub editor_frac: f32,
    pub workspace_frac: f32,
    pub logs_frac: f32,
    pub logs: Vec<String>,
    pub quick_eval: String,
    pub ime_events: u64,
    pub filter: String,
    pub vars: Vec<(String, String)>,
    pub layout: Option<PaneLayout>,
    pub tab_rects: Vec<Rect>,
    pub fit_rect: Option<Rect>,
    pub passes: u64,
    pub startup_first_ui_pass_ms: Option<f64>,
    pub startup_first_ui_build_ms: Option<f64>,
    pub startup_first_paint_ms: Option<f64>,
    idle: Option<IdleState>,
    pub bench_window: Option<Arc<Mutex<Option<WindowBenchResult>>>>,
    bench_finished: bool,
}

impl SpikeApp {
    pub fn new(bench_window: Option<Arc<Mutex<Option<WindowBenchResult>>>>) -> Self {
        let mut editor = Editor::default();
        editor
            .docs
            .push(Doc::from_source("script.lv", &sample::script_5000()));
        editor
            .docs
            .push(Doc::from_source("analysis.lv", &sample::analysis_script()));
        editor.set_caret(0, 8);

        let plot = Plot::new(true);

        let vars = vec![
            ("a".to_string(), "3.14159".to_string()),
            ("f".to_string(), "3*x^2 + 4*x".to_string()),
            ("g".to_string(), "(3*x^2 + 4*x) / 7".to_string()),
            ("h".to_string(), "0.428571428571".to_string()),
            ("m".to_string(), "3x3 matrix".to_string()),
            ("d".to_string(), "-3.0".to_string()),
            ("precision".to_string(), "1000 digits".to_string()),
            ("session".to_string(), "spike-1".to_string()),
        ];

        let mut app = SpikeApp {
            editor,
            plot,
            editor_frac: 0.40,
            workspace_frac: 0.20,
            logs_frac: 0.18,
            logs: vec![
                "[0.00s] Lovelace Desktop spike started (eframe/egui 0.37)".to_string(),
                "[0.00s] loaded script.lv: 5000 lines, ~500 KB fixture".to_string(),
                "[0.00s] SVG plot loaded: rect / polyline / path / text / clipPath / g".to_string(),
                "[0.00s] accessibility: accesskit enabled by eframe (Windows UIA bridge)".to_string(),
            ],
            quick_eval: String::new(),
            ime_events: 0,
            filter: String::new(),
            vars,
            layout: None,
            tab_rects: Vec::new(),
            fit_rect: None,
            passes: 0,
            startup_first_ui_pass_ms: None,
            startup_first_ui_build_ms: None,
            startup_first_paint_ms: None,
            idle: None,
            bench_window,
            bench_finished: false,
        };
        app.log("editor pane virtualised: only visible rows are laid out per pass");
        app
    }

    pub fn log(&mut self, msg: impl Into<String>) {
        let t = cpu::since_process_start_ms();
        let stamp = if t.is_finite() {
            format!("[{:>7.2}s] ", t / 1000.0)
        } else {
            "[   ?   ] ".to_string()
        };
        self.logs.push(format!("{stamp}{}", msg.into()));
        if self.logs.len() > 500 {
            self.logs.remove(0);
        }
    }

    fn count_ime(&mut self, ui: &Ui) {
        let n = ui.input(|i| {
            i.raw
                .events
                .iter()
                .filter(|e| matches!(e, egui::Event::Ime(_)))
                .count()
        });
        self.ime_events += n as u64;
    }

    pub fn ui_root(&mut self, ui: &mut Ui) {
        self.passes += 1;
        if self.startup_first_ui_pass_ms.is_none() {
            self.startup_first_ui_pass_ms = Some(cpu::since_process_start_ms());
        }
        self.count_ime(ui);

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::top("tabstrip").show(ui, |ui| self.tab_strip(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            let full = ui.available_rect_before_wrap();
            self.panes(ui, full);
        });

        if self.startup_first_ui_build_ms.is_none() {
            if let Some(start) = self.startup_first_ui_pass_ms {
                self.startup_first_ui_build_ms = Some(cpu::since_process_start_ms() - start);
            }
        }
        self.bench_post_pass(ui.ctx());
    }

    fn bench_post_pass(&mut self, ctx: &egui::Context) {
        if self.bench_window.is_none() || self.bench_finished {
            return;
        }
        if self.startup_first_paint_ms.is_none() {
            if self.passes >= 2 {
                // Pass 2 can only start once pass 1 was submitted, so this is the best
                // CPU-side proxy for "first frame painted" (documented in the report).
                self.startup_first_paint_ms = Some(cpu::since_process_start_ms());
                self.idle = Some(IdleState {
                    start: Instant::now(),
                    cpu0: cpu::cpu_time(),
                    pass0: self.passes,
                });
                ctx.request_repaint_after(Duration::from_secs(5));
            } else {
                ctx.request_repaint();
            }
            return;
        }

        let Some(idle) = &self.idle else {
            return;
        };
        let elapsed = idle.start.elapsed();
        if std::env::var_os("SPIKE_IDLE_DEBUG").is_some() {
            let causes = ctx.repaint_causes();
            eprintln!(
                "[idle-debug] pass {} at {:.1} ms causes={:?}",
                self.passes,
                elapsed.as_secs_f64() * 1000.0,
                causes.iter().map(|c| c.to_string()).collect::<Vec<_>>()
            );
        }
        if elapsed >= Duration::from_secs(5) {
            let wall_ms = elapsed.as_secs_f64() * 1000.0;
            let cpu_ms = match (cpu::cpu_time(), idle.cpu0) {
                (Some(a), Some(b)) => Some(a.saturating_sub(b).as_secs_f64() * 1000.0),
                _ => None,
            };
            let result = WindowBenchResult {
                startup_first_ui_pass_ms: self.startup_first_ui_pass_ms,
                startup_first_ui_build_ms: self.startup_first_ui_build_ms,
                startup_first_paint_ms: self.startup_first_paint_ms,
                idle_wall_ms: wall_ms,
                idle_cpu_ms: cpu_ms,
                idle_cpu_pct: cpu_ms.map(|c| c / wall_ms * 100.0),
                idle_frames: self.passes.saturating_sub(idle.pass0),
                idle_pass: idle.pass0,
                window_error: None,
            };
            if let Some(shared) = &self.bench_window {
                *shared.lock().unwrap() = Some(result);
            }
            self.bench_finished = true;
            self.idle = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx.request_repaint();
        } else {
            let remaining = Duration::from_secs(5)
                .saturating_sub(elapsed)
                .saturating_add(Duration::from_millis(20));
            ctx.request_repaint_after(remaining);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Lovelace Desktop Spike")
                    .strong()
                    .color(TEXT)
                    .size(15.0),
            );
            ui.label(RichText::new("eframe/egui 0.37").color(DIM));
            ui.separator();
            if ui.button("Run").clicked() {
                self.log("run: evaluation is a stub in this spike");
            }
            if ui.button("Cancel").clicked() {
                self.log("cancel: no running evaluation in this spike");
            }
            ui.separator();
            ui.label(RichText::new("Zoom").color(DIM));
            let ppp = ui.ctx().pixels_per_point();
            for z in [1.0f32, 1.5, 2.0] {
                let selected = (ppp - z).abs() < 0.02;
                if ui.selectable_label(selected, format!("{z:.1}x")).clicked() {
                    ui.ctx().set_zoom_factor(z);
                }
            }
            ui.label(RichText::new(format!("ppp {ppp:.2}")).color(DIM));
            ui.separator();
            ui.label(RichText::new("Quick eval (IME field):").color(DIM));
            let label = ui.label("expression");
            ui.add(
                egui::TextEdit::singleline(&mut self.quick_eval)
                    .hint_text("type here (IME test)")
                    .desired_width(200.0),
            )
            .labelled_by(label.id);
        });
    }

    fn tab_strip(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.tab_rects.clear();
            for i in 0..self.editor.docs.len() {
                let name = self.editor.docs[i].name.clone();
                let selected = i == self.editor.active;
                let resp = ui.selectable_label(selected, name.clone());
                self.tab_rects.push(resp.rect);
                if resp.clicked() {
                    self.editor.set_active(i);
                    self.log(format!("tab selected: {name}"));
                }
            }
            ui.separator();
            let doc = &self.editor.docs[self.editor.active];
            ui.label(
                RichText::new(format!("{} lines / {} bytes", doc.lines.len(), doc.bytes))
                    .color(DIM),
            );
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let (line, col) = self.editor.line_col_1based();
            ui.label(
                RichText::new(format!("Ln {line}, Col {col}"))
                    .monospace()
                    .color(ACCENT),
            );
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "{} lines / {} bytes",
                    self.editor.doc().line_count(),
                    self.editor.doc().bytes
                ))
                .color(DIM),
            );
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "visible rows {} / laid out {}",
                    self.editor.metrics.visible_rows, self.editor.metrics.laid_out_rows
                ))
                .color(DIM),
            );
            ui.separator();
            ui.label(
                RichText::new(format!("IME events: {}", self.ime_events))
                    .monospace()
                    .color(DIM),
            );
            ui.separator();
            ui.label(
                RichText::new(format!(
                    "zoom {:.2}x  ppp {:.2}",
                    ui.ctx().zoom_factor(),
                    ui.ctx().pixels_per_point()
                ))
                .color(DIM),
            );
            ui.separator();
            if let Some(l) = self.layout {
                ui.label(
                    RichText::new(format!(
                        "editor {:.0}px  plot {:.0}px  logs {:.0}px",
                        l.editor.width(),
                        l.plot.width(),
                        l.logs.height()
                    ))
                    .color(DIM),
                );
            }
        });
    }

    fn panes(&mut self, ui: &mut Ui, full: Rect) {
        let l = compute_layout(full, self.editor_frac, self.workspace_frac, self.logs_frac);
        self.layout = Some(l);
        ui.painter().rect_filled(full, 0.0, Color32::from_rgb(0x0b, 0x0e, 0x12));

        self.editor_pane(ui, l.editor);
        self.plot_pane(ui, l.plot);
        self.workspace_pane(ui, l.workspace);
        self.logs_pane(ui, l.logs);

        let d1 = splitter(ui, l.split_a, "split_editor", true, "Editor / plot splitter");
        let d2 = splitter(ui, l.split_b, "split_workspace", true, "Plot / workspace splitter");
        let d3 = splitter(ui, l.split_c, "split_logs", false, "Plot / logs splitter");

        if d1 != 0.0 {
            self.editor_frac = (self.editor_frac + d1 / full.width().max(1.0)).clamp(0.02, 0.95);
        }
        if d2 != 0.0 {
            self.workspace_frac =
                (self.workspace_frac - d2 / full.width().max(1.0)).clamp(0.02, 0.95);
        }
        if d3 != 0.0 {
            self.logs_frac = (self.logs_frac - d3 / full.height().max(1.0)).clamp(0.02, 0.9);
        }
    }

    fn editor_pane(&mut self, ui: &mut Ui, rect: Rect) {
        with_pane(ui, rect, "Editor - script.lv", |ui, header, body| {
            let name = self.editor.doc().name.clone();
            let lines = self.editor.doc().line_count();
            let bytes = self.editor.doc().bytes;
            ui.scope_builder(header_ui(header), |ui| {
                ui.label(RichText::new(name).color(ACCENT).monospace());
                ui.label(RichText::new(format!("{lines} lines / {bytes} bytes")).color(DIM));
            });
            self.editor.ui(ui, body);
        });
    }

    fn plot_pane(&mut self, ui: &mut Ui, rect: Rect) {
        with_pane(ui, rect, "Plot - engine SVG", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                let fit_resp = ui.button("Fit");
                self.fit_rect = Some(fit_resp.rect);
                if fit_resp.clicked() {
                    self.plot.fit();
                }
                ui.separator();
                for z in [1.0f32, 1.5, 2.0] {
                    if ui.button(format!("{z:.1}x")).clicked() {
                        self.plot.zoom = z;
                        self.plot.pan = Vec2::ZERO;
                    }
                }
                ui.separator();
                let toggle = if self.plot.use_bad {
                    "Load valid SVG"
                } else {
                    "Load malformed SVG"
                };
                if ui.button(toggle).clicked() {
                    self.plot.toggle_bad();
                    let state = if self.plot.use_bad { "malformed" } else { "valid" };
                    self.log(format!("plot source switched to {state} SVG"));
                }
            });
            let ppp = ui.ctx().pixels_per_point();
            self.plot.ensure(ui.ctx(), ppp * self.plot.zoom);
            let n = ui.ctx().zoom_factor();
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(body.min, vec2(body.width(), 18.0)))
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.label(
                        RichText::new(format!(
                            "raster {}x{} @ {:.2}x - {:.2} ms raster, {:.2} ms upload (zoom {n:.2}x)",
                            self.plot.raster_px[0],
                            self.plot.raster_px[1],
                            self.plot.raster_scale,
                            self.plot.last_raster_ms,
                            self.plot.last_upload_ms,
                        ))
                        .small()
                        .color(DIM),
                    );
                },
            );
            let canvas = Rect::from_min_max(pos2(body.left(), body.top() + 18.0), body.max);
            self.plot.ui(ui, canvas);
        });
    }

    fn workspace_pane(&mut self, ui: &mut Ui, rect: Rect) {
        with_pane(ui, rect, "Workspace", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                let label = ui.label("Filter");
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .hint_text("variable name")
                        .desired_width(120.0),
                )
                .labelled_by(label.id);
            });
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(body.shrink(8.0))
                    .layout(Layout::top_down(Align::Min)),
                |ui| {
                    let mut shown = 0usize;
                    for (name, value) in &self.vars {
                        if !self.filter.is_empty() && !name.contains(self.filter.as_str()) {
                            continue;
                        }
                        shown += 1;
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).monospace().color(ACCENT));
                            ui.label(RichText::new(value).monospace().color(TEXT));
                        });
                    }
                    if shown == 0 {
                        ui.label(RichText::new("no variables match the filter").color(DIM));
                    }
                    ui.add_space(6.0);
                    if ui.button("Clear variables").clicked() {
                        self.vars.clear();
                        self.log("variables cleared");
                    }
                },
            );
        });
    }

    fn logs_pane(&mut self, ui: &mut Ui, rect: Rect) {
        with_pane(ui, rect, "Logs", |ui, header, body| {
            ui.scope_builder(header_ui(header), |ui| {
                if ui.button("Clear logs").clicked() {
                    self.logs.clear();
                }
                ui.label(
                    RichText::new(format!("IME events: {}", self.ime_events))
                        .monospace()
                        .color(DIM),
                );
            });
            let row_h = 16.0;
            let rows = ((body.height() - 6.0) / row_h).floor().max(0.0) as usize;
            let start = self.logs.len().saturating_sub(rows);
            let painter = ui.painter();
            for (k, line) in self.logs[start..].iter().enumerate() {
                let y = body.top() + 3.0 + k as f32 * row_h;
                painter.text(
                    pos2(body.left() + 8.0, y),
                    Align2::LEFT_TOP,
                    line,
                    FontId::monospace(12.0),
                    TEXT,
                );
            }
        });
    }
}

impl eframe::App for SpikeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.ui_root(ui);
    }
}

fn header_ui(header: Rect) -> UiBuilder {
    UiBuilder::new()
        .max_rect(header.shrink2(vec2(8.0, 3.0)))
        .layout(Layout::left_to_right(Align::Center))
}

fn with_pane<R>(
    ui: &mut Ui,
    rect: Rect,
    title: &'static str,
    add: impl FnOnce(&mut Ui, Rect, Rect) -> R,
) -> R {
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .accessibility_role(egui::Role::Pane)
            .accessibility_label(title),
        |ui| {
            ui.painter().rect_filled(rect, 4.0, PANE_BG);
            ui.painter()
                .rect_stroke(rect, 4.0, Stroke::new(1.0, BORDER), StrokeKind::Inside);
            let header = Rect::from_min_size(rect.min, vec2(rect.width(), 24.0));
            ui.painter().rect_filled(header, 4.0, HEADER_BG);
            let body = Rect::from_min_max(
                pos2(rect.left() + 1.0, header.bottom() + 1.0),
                pos2(rect.right() - 1.0, rect.bottom() - 1.0),
            );
            add(ui, header, body)
        },
    )
    .inner
}

fn splitter(ui: &mut Ui, rect: Rect, salt: &str, horizontal_axis: bool, label: &str) -> f32 {
    let id = Id::unique(salt);
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    if resp.clicked() || resp.drag_started() || resp.is_pointer_button_down_on() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    let hot = resp.hovered() || resp.dragged() || focused;
    ui.painter().rect_filled(rect, 0.0, if hot { SPLIT_HOT } else { SPLIT_BG });
    let grip = if horizontal_axis {
        Rect::from_center_size(rect.center(), vec2(2.0, rect.height()))
    } else {
        Rect::from_center_size(rect.center(), vec2(rect.width(), 2.0))
    };
    ui.painter()
        .rect_filled(grip, 1.0, if hot { ACCENT } else { BORDER });

    if hot {
        ui.ctx().set_cursor_icon(if horizontal_axis {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        });
    }

    let mut delta = if resp.dragged() {
        if horizontal_axis {
            resp.drag_delta().x
        } else {
            resp.drag_delta().y
        }
    } else {
        0.0
    };

    if focused {
        let step = if ui.input(|i| i.modifiers.shift) {
            32.0
        } else {
            8.0
        };
        if horizontal_axis {
            if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
                delta += step;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
                delta -= step;
            }
            if ui.input(|i| i.key_pressed(Key::Home)) {
                delta -= 1000.0;
            }
            if ui.input(|i| i.key_pressed(Key::End)) {
                delta += 1000.0;
            }
        } else {
            if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                delta += step;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                delta -= step;
            }
        }
    }

    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Splitter, true, label));
    delta
}



