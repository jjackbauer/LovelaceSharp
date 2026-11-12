//! Headless benchmark driver. Produces every S1-S6 number plus the S7 window numbers
//! collected earlier, and serialises them as one JSON object.
//!
//! Metric definitions (also stated in the report):
//! * `open_*_ms`       - wall time of the first `Context::run_ui` + tessellate pass that
//!                       paints the pane with the document loaded.
//! * `typing_*_ms`     - wall time of one `Context::run_ui` + tessellate pass after a
//!                       synthetic `Event::Text` keystroke. CPU-side: includes UI build
//!                       and tessellation, excludes GPU present/swap.
//! * `raster_*_ms`     - wall time of `resvg::render` into a tiny-skia pixmap only,
//!                       median of 5 runs; parse and texture upload are reported apart.

use std::cell::Cell;
use std::time::Instant;

use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use resvg::usvg;

use crate::app::{self, PaneLayout, SpikeApp, WindowBenchResult};
use crate::editor::{Doc, Editor};
use crate::json::{self, Json};
use crate::plot;
use crate::sample;
use crate::syntax;

const W: f32 = 1280.0;
const H: f32 = 800.0;

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(W, H))
}

fn raw_input(events: Vec<Event>, time: f64, screen: Rect) -> RawInput {
    RawInput {
        screen_rect: Some(screen),
        time: Some(time),
        predicted_dt: 1.0 / 60.0,
        events,
        focused: true,
        ..Default::default()
    }
}

struct PassResult {
    ak: Option<egui::accesskit::TreeUpdate>,
    ms: f64,
    prims: usize,
    shapes: usize,
}

fn pass(ctx: &Context, app: &mut SpikeApp, events: Vec<Event>, t: &mut f64) -> PassResult {
    pass_on(ctx, app, events, t, screen())
}

fn pass_on(
    ctx: &Context,
    app: &mut SpikeApp,
    events: Vec<Event>,
    t: &mut f64,
    screen: Rect,
) -> PassResult {
    *t += 1.0 / 60.0;
    let raw = raw_input(events, *t, screen);
    let started = Instant::now();
    let out = ctx.run_ui(raw, |ui| app.ui_root(ui));
    let egui::FullOutput {
        platform_output,
        mut textures_delta,
        shapes,
        pixels_per_point,
        ..
    } = out;
    textures_delta.clear();
    let shape_count = shapes.len();
    let prims = ctx.tessellate(shapes, pixels_per_point).len();
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    PassResult {
        ak: platform_output.accesskit_update,
        ms,
        prims,
        shapes: shape_count,
    }
}

fn press(pos: Pos2) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }
}

fn release(pos: Pos2) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }
}

fn key_event(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn is_interactive(role: egui::Role) -> bool {
    use egui::Role;
    matches!(
        role,
        Role::Button
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::CheckBox
            | Role::RadioButton
            | Role::ComboBox
            | Role::Slider
            | Role::Splitter
    )
}

struct AkStats {
    total: usize,
    labeled: usize,
    interactive: usize,
    unlabeled_interactive: Vec<String>,
    panes: usize,
}

fn ak_stats(update: Option<&egui::accesskit::TreeUpdate>) -> AkStats {
    let mut stats = AkStats {
        total: 0,
        labeled: 0,
        interactive: 0,
        unlabeled_interactive: Vec::new(),
        panes: 0,
    };
    let Some(update) = update else {
        return stats;
    };
    for (_id, node) in &update.nodes {
        stats.total += 1;
        let has_label = node.label().map(|s| !s.is_empty()).unwrap_or(false)
            || !node.labelled_by().is_empty();
        if has_label {
            stats.labeled += 1;
        }
        if matches!(node.role(), egui::Role::Pane) {
            stats.panes += 1;
        }
        if is_interactive(node.role()) {
            stats.interactive += 1;
            if !has_label {
                stats.unlabeled_interactive.push(format!("{:?}", node.role()));
            }
        }
    }
    stats
}

/// S1: first pass of an editor-only root Ui with the 5,000-line document.
fn editor_open(src: &str) -> (f64, usize, usize, f32, f32) {
    let ctx = Context::default();
    let mut editor = Editor::default();
    editor.docs.push(Doc::from_source("script.lv", src));
    editor.set_caret(0, 4);
    let raw = raw_input(vec![], 0.0, screen());
    let started = Instant::now();
    let mut out = ctx.run_ui(raw, |ui| {
        editor.ui(ui, screen());
    });
    out.textures_delta.clear();
    let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    (
        ms,
        editor.metrics.visible_rows,
        editor.metrics.laid_out_rows,
        editor.metrics.row_h,
        editor.metrics.char_w,
    )
}

/// S1: prove that `TextEdit::layouter` is handed the whole buffer (no viewport).
fn textedit_probe(src: &str) -> (f64, usize, usize, usize, usize) {
    let ctx = Context::default();
    let mut text = src.to_string();
    let bytes_seen = Cell::new(0usize);
    let spans = Cell::new(0usize);
    let calls = Cell::new(0usize);
    let mut layouter = |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _wrap: f32| {
        let s = buf.as_str().to_string();
        bytes_seen.set(s.len());
        calls.set(calls.get() + 1);
        let job = syntax::job_multiline(&s, &egui::FontId::monospace(13.0), &syntax::DARK);
        spans.set(job.sections.len());
        ui.painter().layout_job(job)
    };
    let raw = raw_input(vec![], 0.0, screen());
    let started = Instant::now();
    let mut out = ctx.run_ui(raw, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut text)
                .layouter(&mut layouter)
                .desired_width(f32::INFINITY)
                .desired_rows(30),
        );
    });
    out.textures_delta.clear();
    let _ = ctx.tessellate(out.shapes, out.pixels_per_point);
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    (ms, bytes_seen.get(), spans.get(), calls.get(), text.len())
}

struct SvgProbe {
    fontdb_ms: f64,
    parse_ms: f64,
    w: f32,
    h: f32,
    scale_100: f64,
    scale_150: f64,
    scale_200: f64,
    px_100: [u32; 2],
    px_150: [u32; 2],
    px_200: [u32; 2],
    upload_ms: f64,
    malformed_rejected: bool,
    malformed_error: String,
}

fn svg_probe(ctx: &Context) -> SvgProbe {
    let t0 = Instant::now();
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let fontdb_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let src = sample::svg_plot();
    let t1 = Instant::now();
    let tree = usvg::Tree::from_str(&src, &options).expect("sample SVG must parse");
    let parse_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let w = tree.size().width();
    let h = tree.size().height();

    let (r100, px100) = plot::bench_raster(&tree, w, h, 1.0, 5);
    let (r150, px150) = plot::bench_raster(&tree, w, h, 1.5, 5);
    let (r200, px200) = plot::bench_raster(&tree, w, h, 2.0, 5);

    let t2 = Instant::now();
    let (pixmap, _ms) = plot::render_to_pixmap(&tree, w, h, 1.0, 4096.0).expect("pixmap");
    let image = egui::ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    );
    let _handle = ctx.load_texture("svg_probe", image, egui::TextureOptions::LINEAR);
    let upload_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let malformed = usvg::Tree::from_str(sample::SVG_MALFORMED, &options);
    let (malformed_rejected, malformed_error) = match malformed {
        Ok(_) => (false, "parser accepted malformed SVG".to_string()),
        Err(e) => (true, format!("{e}")),
    };

    SvgProbe {
        fontdb_ms,
        parse_ms,
        w,
        h,
        scale_100: r100,
        scale_150: r150,
        scale_200: r200,
        px_100: px100,
        px_150: px150,
        px_200: px200,
        upload_ms,
        malformed_rejected,
        malformed_error,
    }
}

struct ZoomRow {
    zoom: f32,
    ppp: f32,
    screen: [f32; 2],
    full_w: f32,
    full_h: f32,
    editor_w: f32,
    plot_w: f32,
    workspace_w: f32,
    logs_h: f32,
    row_h_pt: f32,
    char_w_pt: f32,
    char_w_px: f32,
    tex_w: u32,
    tex_h: u32,
    raster_scale: f32,
    shapes: usize,
    prims: usize,
    fits: bool,
}

impl ZoomRow {
    fn fractions(&self) -> (f32, f32, f32, f32) {
        let usable = (self.full_w - 2.0 * app::SPLIT).max(1.0);
        (
            self.editor_w / usable,
            self.plot_w / usable,
            self.workspace_w / usable,
            self.logs_h / self.full_h.max(1.0),
        )
    }

    fn json(&self) -> String {
        let (ef, pf, wf, lf) = self.fractions();
        format!(
            "{{\"zoom\":{:.2},\"ppp\":{:.3},\"screen_phys\":[{:.0},{:.0}],\"full_pt\":[{:.2},{:.2}],\"editor_w_pt\":{:.2},\"plot_w_pt\":{:.2},\"workspace_w_pt\":{:.2},\"logs_h_pt\":{:.2},\"editor_frac\":{:.4},\"plot_frac\":{:.4},\"workspace_frac\":{:.4},\"logs_frac\":{:.4},\"row_h_pt\":{:.3},\"char_w_pt\":{:.3},\"char_w_px\":{:.3},\"plot_tex\":[{},{}],\"raster_scale\":{:.3},\"shapes\":{},\"primitives\":{},\"layout_fits\":{}}}",
            self.zoom,
            self.ppp,
            self.screen[0],
            self.screen[1],
            self.full_w,
            self.full_h,
            self.editor_w,
            self.plot_w,
            self.workspace_w,
            self.logs_h,
            ef,
            pf,
            wf,
            lf,
            self.row_h_pt,
            self.char_w_pt,
            self.char_w_px,
            self.tex_w,
            self.tex_h,
            self.raster_scale,
            self.shapes,
            self.prims,
            self.fits,
        )
    }
}

struct S6Scenario {
    rows: Vec<ZoomRow>,
    proportions_stable: bool,
    proportions_match_config: bool,
    no_overflow: bool,
}

impl S6Scenario {
    fn to_json(&self) -> String {
        let rows = self
            .rows
            .iter()
            .map(ZoomRow::json)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"per_zoom\":[{}],\"pane_fractions_stable\":{},\"pane_fractions_match_config\":{},\"no_overflow\":{}}}",
            rows, self.proportions_stable, self.proportions_match_config, self.no_overflow
        )
    }
}

fn s6_scenario(ctx: &Context, app: &mut SpikeApp, t: &mut f64, screen: Rect) -> S6Scenario {
    let mut rows: Vec<ZoomRow> = Vec::new();
    for zoom in [1.0f32, 1.5, 2.0] {
        ctx.set_zoom_factor(zoom);
        let run = pass_on(ctx, app, vec![], t, screen);
        let ppp = ctx.pixels_per_point();
        let layout = app.layout.expect("layout");
        let raster = app.plot.raster_px;
        let char_w = app.editor.metrics.char_w;
        rows.push(ZoomRow {
            zoom,
            ppp,
            screen: [screen.width(), screen.height()],
            full_w: layout.full.width(),
            full_h: layout.full.height(),
            editor_w: layout.editor.width(),
            plot_w: layout.plot.width(),
            workspace_w: layout.workspace.width(),
            logs_h: layout.logs.height(),
            row_h_pt: app.editor.metrics.row_h,
            char_w_pt: char_w,
            char_w_px: char_w * ppp,
            tex_w: raster[0],
            tex_h: raster[1],
            raster_scale: app.plot.raster_scale,
            shapes: run.shapes,
            prims: run.prims,
            fits: app::layout_fits(&layout),
        });
    }
    ctx.set_zoom_factor(1.0);
    let _ = pass_on(ctx, app, vec![], t, screen);

    let (c0, c1, c2, c3) = (0.40f32, 0.40f32, 0.20f32, 0.18f32);
    let first = rows[0].fractions();
    let proportions_stable = rows.iter().all(|r| {
        let f = r.fractions();
        (f.0 - first.0).abs() < 0.01
            && (f.1 - first.1).abs() < 0.01
            && (f.2 - first.2).abs() < 0.01
            && (f.3 - first.3).abs() < 0.01
    });
    let proportions_match_config = rows.iter().all(|r| {
        let f = r.fractions();
        (f.0 - c0).abs() < 0.015
            && (f.1 - c1).abs() < 0.015
            && (f.2 - c2).abs() < 0.015
            && (f.3 - c3).abs() < 0.015
    });
    S6Scenario {
        no_overflow: rows.iter().all(|r| r.fits),
        rows,
        proportions_stable,
        proportions_match_config,
    }
}
pub fn run_headless() -> HeadlessReport {
    let mut root = Json::new();
    root.str("schema", "lovelace-desktop-spike/1")
        .str("os", std::env::consts::OS)
        .str("arch", std::env::consts::ARCH)
        .str(
            "cpu",
            &std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string()),
        )
        .str(
            "build_profile",
            "dev profile with opt-level=2 (cargo build, not --release)",
        )
        .str("screen_points", "1280x800")
        .str("measurement_phase", "headless S1-S6");

    let mut deps = Json::new();
    deps.str("eframe", "0.37.0")
        .str("egui", "0.37.0")
        .str("resvg", "0.50.0")
        .str("usvg", "0.50.0")
        .str("tiny-skia", "0.11.4 (pinned by resvg 0.50; used via resvg::tiny_skia)")
        .str("winit", "0.30.13 (transitive)")
        .str("accesskit", "0.25.2 (transitive)")
        .str("rustc", "1.99.1");
    root.obj("deps", deps);

    // ---- S1: fixture + open + typing -------------------------------------------------
    let t0 = Instant::now();
    let src = sample::script_5000();
    let generate_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let line_count = src.lines().count();
    let byte_count = src.len();

    let t1 = Instant::now();
    let parsed = Doc::from_source("script.lv", &src);
    let parse_lines_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let parsed_lines = parsed.line_count();
    drop(parsed);

    let (open_editor_ms, open_visible, open_laid_out, row_h, char_w) = editor_open(&src);

    // Full workbench: fresh context, fresh app, first pass.
    let ctx = Context::default();
    ctx.enable_accesskit();
    let mut app = SpikeApp::new(None);
    let mut t = 0.0;
    let first = pass(&ctx, &mut app, vec![], &mut t);
    let workbench_open_ms = first.ms;
    let open_shapes = first.shapes;
    let open_prims = first.prims;
    let ak_update = first.ak;

    let second = pass(&ctx, &mut app, vec![], &mut t);
    let ak_from_second = second.ak;

    app.editor.force_focus = true;
    let _ = pass(&ctx, &mut app, vec![], &mut t); // establish keyboard focus

    let mut typing = Vec::with_capacity(200);
    for k in 0..200 {
        let text = if k % 3 == 0 { "7" } else { "x" };
        let run = pass(&ctx, &mut app, vec![Event::Text(text.to_string())], &mut t);
        typing.push(run.ms);
    }
    let mut typing_sorted = typing.clone();
    let (p50, p95, max) = json::percentiles(&mut typing_sorted);
    let typing_mean = typing.iter().sum::<f64>() / typing.len() as f64;
    app.editor.force_focus = false;
    let laid_out = app.editor.metrics.laid_out_rows;
    let visible_rows = app.editor.metrics.visible_rows;
    let edits_applied = app.editor.edits;
    let caret_after = app.editor.line_col_1based();

    let (te_ms, te_bytes, te_spans, te_calls, _te_text_len) = textedit_probe(&src);
    let line0 = app.editor.doc().lines.first().cloned().unwrap_or_default();
    let line0_spans = syntax::section_count(&syntax::job(
        &line0,
        egui::FontId::monospace(13.0),
        &syntax::DARK,
    ));

    let mut s1 = Json::new();
    s1.int("lines", line_count as u64)
        .int("bytes", byte_count as u64)
        .f("generate_ms", generate_ms)
        .f("parse_lines_ms", parse_lines_ms)
        .int("parsed_lines", parsed_lines as u64)
        .f("open_editor_first_pass_ms", open_editor_ms)
        .int("open_visible_rows", open_visible as u64)
        .int("open_laid_out_rows", open_laid_out as u64)
        .int("editor_total_lines", line_count as u64)
        .f("open_workbench_first_pass_ms", workbench_open_ms)
        .int("open_shapes", open_shapes as u64)
        .int("open_primitives", open_prims as u64)
        .f("typing_p50_ms", p50)
        .f("typing_p95_ms", p95)
        .f("typing_max_ms", max)
        .f("typing_mean_ms", typing_mean)
        .int("typing_samples", typing.len() as u64)
        .int("typing_edits_applied", edits_applied as u64)
        .int("typing_laid_out_rows", laid_out as u64)
        .int("typing_visible_rows", visible_rows as u64)
        .int("typing_caret_line", caret_after.0 as u64)
        .int("typing_caret_col", caret_after.1 as u64)
        .f("textedit_full_buffer_pass_ms", te_ms)
        .int("textedit_layouter_buffer_bytes", te_bytes as u64)
        .int("textedit_layouter_sections", te_spans as u64)
        .int("textedit_layouter_calls", te_calls as u64)
        .int("line0_highlight_sections", line0_spans as u64)
        .f("metric_row_h_pt", row_h as f64)
        .f("metric_char_w_pt", char_w as f64);
    root.obj("s1", s1);

    // ---- S2: IME ---------------------------------------------------------------------
    let mut s2 = Json::new();
    s2.bool("text_field_present", true)
        .str("text_field", "logs/toolbar Quick eval TextEdit (singleline, IME-capable)")
        .int("ime_events_observed", app.ime_events)
        .str(
            "status",
            "enabled via winit; automated composition not verified (no synthetic composition was driven)",
        );
    root.obj("s2", s2);

    // ---- S3: accessibility -----------------------------------------------------------
    let stats = ak_stats(ak_update.as_ref().or(ak_from_second.as_ref()));
    let mut s3 = Json::new();
    s3.bool("accesskit_enabled", true)
        .int("accesskit_nodes", stats.total as u64)
        .int("accesskit_labeled_nodes", stats.labeled as u64)
        .int("accesskit_interactive_nodes", stats.interactive as u64)
        .int(
            "accesskit_interactive_unlabeled",
            stats.unlabeled_interactive.len() as u64,
        )
        .int("accesskit_pane_nodes", stats.panes as u64)
        .arr_str("unlabeled_roles", &stats.unlabeled_interactive)
        .str(
            "note",
            "eframe's default features enable accesskit; the windowed app therefore publishes a UIA tree on Windows",
        );
    root.obj("s3", s3);

    // ---- S4: SVG raster --------------------------------------------------------------

    // Exercise the in-pane malformed-SVG path: parse error rendered inline, no panic.
    app.plot.toggle_bad();
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let inline_error_rendered = app.plot.error.is_some();
    app.plot.toggle_bad();
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let svg = svg_probe(&ctx);

    // S4 interactions, driven through synthetic egui input against the real widget code:
    // wheel zoom around the pointer, drag pan, and a click on the Fit button.
    let canvas = app.plot.canvas;
    let plot_base = ((canvas.width() - 16.0) / app.plot.svg_w)
        .min((canvas.height() - 16.0) / app.plot.svg_h)
        .max(0.01);
    let anchor = canvas.center() + vec2(30.0, 10.0);
    let world_before = (anchor - canvas.center() - app.plot.pan) / (plot_base * app.plot.zoom);
    let zoom_before = app.plot.zoom;
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(anchor)], &mut t);
    let _ = pass(
        &ctx,
        &mut app,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 60.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        }],
        &mut t,
    );
    let zoom_after_wheel = app.plot.zoom;
    let world_screen_after =
        canvas.center() + app.plot.pan + world_before * (plot_base * zoom_after_wheel);
    let anchor_error = (world_screen_after - anchor).length();

    let pan_before = app.plot.pan;
    let drag_from = canvas.center();
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(drag_from)], &mut t);
    let _ = pass(&ctx, &mut app, vec![press(drag_from)], &mut t);
    let _ = pass(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(drag_from + vec2(30.0, 20.0))],
        &mut t,
    );
    let _ = pass(
        &ctx,
        &mut app,
        vec![release(drag_from + vec2(30.0, 20.0))],
        &mut t,
    );
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let pan_delta = app.plot.pan - pan_before;

    let fit_pos = app.fit_rect.unwrap_or(Rect::NOTHING).center();
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(fit_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![press(fit_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![release(fit_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let fit_click_works = (app.plot.zoom - 1.0).abs() < 0.001 && app.plot.pan.length() < 0.001;
    let src_svg = sample::svg_plot();
    let mut s4 = Json::new();
    s4.f("fontdb_load_ms", svg.fontdb_ms)
        .f("parse_ms", svg.parse_ms)
        .f("svg_width_pt", svg.w as f64)
        .f("svg_height_pt", svg.h as f64)
        .f("raster_100_ms", svg.scale_100)
        .f("raster_150_ms", svg.scale_150)
        .f("raster_200_ms", svg.scale_200)
        .int("px_100_w", svg.px_100[0] as u64)
        .int("px_100_h", svg.px_100[1] as u64)
        .int("px_150_w", svg.px_150[0] as u64)
        .int("px_150_h", svg.px_150[1] as u64)
        .int("px_200_w", svg.px_200[0] as u64)
        .int("px_200_h", svg.px_200[1] as u64)
        .f("texture_upload_ms", svg.upload_ms)
        .bool("malformed_rejected", svg.malformed_rejected)
        .str("malformed_error", &svg.malformed_error)
        .bool("inline_error_rendered", inline_error_rendered)
        .bool("wheel_zoom_works", (zoom_after_wheel - zoom_before).abs() > 0.01)
        .f("wheel_zoom_anchor_error_px", anchor_error as f64)
        .f("zoom_before_wheel", zoom_before as f64)
        .f("zoom_after_wheel", zoom_after_wheel as f64)
        .arr_f("drag_pan_delta_px", &[pan_delta.x as f64, pan_delta.y as f64])
        .bool(
            "drag_pan_works",
            (pan_delta.x - 30.0).abs() < 8.0 && (pan_delta.y - 20.0).abs() < 8.0,
        )
        .bool("fit_button_click_works", fit_click_works)
        .bool("has_rect", src_svg.contains("<rect"))
        .bool("has_polyline", src_svg.contains("<polyline"))
        .bool("has_text", src_svg.contains("<text"))
        .bool("has_path", src_svg.contains("<path "))
        .bool("has_clip_path", src_svg.contains("<clipPath"))
        .bool("has_g_transform", src_svg.contains("<g transform"));
    root.obj("s4", s4);

    // ---- S5: layout, splitters, tabs --------------------------------------------------
    // Reset to the default proportions, then measure the splitter interactions.
    app.editor_frac = 0.40;
    app.workspace_frac = 0.20;
    app.logs_frac = 0.18;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let layout0: PaneLayout = app.layout.expect("layout after pass");
    let width_before = layout0.editor.width();
    let split_pos = layout0.split_a.center();

    // Click the splitter (press/release over two passes) so it takes keyboard focus.
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(split_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![press(split_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![release(split_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let width_after_click = app.layout.expect("layout").editor.width();

    // Arrow-key resize while the splitter has focus.
    let _ = pass(&ctx, &mut app, vec![key_event(Key::ArrowRight)], &mut t);
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let width_after_key = app.layout.expect("layout").editor.width();

    // Drag resize: press, move 40 px right, release.
    let split_pos2 = app.layout.expect("layout").split_a.center();
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(split_pos2)], &mut t);
    let _ = pass(&ctx, &mut app, vec![press(split_pos2)], &mut t);
    let _ = pass(
        &ctx,
        &mut app,
        vec![Event::PointerMoved(split_pos2 + vec2(40.0, 0.0))],
        &mut t,
    );
    let _ = pass(
        &ctx,
        &mut app,
        vec![release(split_pos2 + vec2(40.0, 0.0))],
        &mut t,
    );
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let width_after_drag = app.layout.expect("layout").editor.width();

    // Minimum-size clamps, driven by extreme fractions.
    app.editor_frac = 0.99;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let editor_max = app.layout.expect("layout").editor.width();
    app.editor_frac = 0.01;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let editor_min = app.layout.expect("layout").editor.width();
    app.editor_frac = 0.40;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let editor_w_default = app.layout.expect("layout").editor.width();
    let usable_w = app.layout.expect("layout").full.width() - 2.0 * app::SPLIT;

    app.workspace_frac = 0.99;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let workspace_max = app.layout.expect("layout").workspace.width();
    app.workspace_frac = 0.01;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let workspace_min = app.layout.expect("layout").workspace.width();
    app.workspace_frac = 0.20;

    app.logs_frac = 0.99;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let logs_max = app.layout.expect("layout").logs.height();
    app.logs_frac = 0.01;
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let logs_min = app.layout.expect("layout").logs.height();
    app.logs_frac = 0.18;

    // The same four-pane layout must stay inside the window at smaller sizes.
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let fits_1280 = app::layout_fits(&app.layout.expect("layout"));
    let _ = pass_on(
        &ctx,
        &mut app,
        vec![],
        &mut t,
        Rect::from_min_size(Pos2::ZERO, vec2(900.0, 600.0)),
    );
    let fits_900 = app::layout_fits(&app.layout.expect("layout"));
    let _ = pass_on(
        &ctx,
        &mut app,
        vec![],
        &mut t,
        Rect::from_min_size(Pos2::ZERO, vec2(640.0, 400.0)),
    );
    let fits_640 = app::layout_fits(&app.layout.expect("layout"));
    let _ = pass(&ctx, &mut app, vec![], &mut t); // back to 1280x800

    // Tab strip: click the second tab.
    let tab_count = app.tab_rects.len();
    let tab2 = app.tab_rects.get(1).copied().unwrap_or(Rect::NOTHING);
    let tab_pos = tab2.center();
    let _ = pass(&ctx, &mut app, vec![Event::PointerMoved(tab_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![press(tab_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![release(tab_pos)], &mut t);
    let _ = pass(&ctx, &mut app, vec![], &mut t);
    let active_tab = app.editor.active;
    let active_doc = app.editor.doc().name.clone();

    let expected_editor_max = (usable_w - app::MIN_PLOT - app::MIN_WORKSPACE).max(app::MIN_EDITOR);
    let expected_workspace_max =
        (usable_w - editor_w_default - app::MIN_PLOT).max(app::MIN_WORKSPACE);
    let expected_logs_max = (app.layout.expect("layout").full.height()
        - app::MIN_TOP
        - app::SPLIT)
        .max(app::MIN_LOGS);

    let mut s5 = Json::new();
    s5.int("tabs", tab_count as u64)
        .bool("tab_switch_works", active_tab == 1)
        .str("active_document", &active_doc)
        .f("editor_width_before_click_px", width_before as f64)
        .f("editor_width_after_click_px", width_after_click as f64)
        .f("editor_width_after_key_px", width_after_key as f64)
        .f(
            "keyboard_resize_delta_px",
            (width_after_key - width_after_click) as f64,
        )
        .bool(
            "keyboard_resize_works",
            (width_after_key - width_after_click).abs() > 4.0,
        )
        .f(
            "drag_resize_delta_px",
            (width_after_drag - width_after_key) as f64,
        )
        .bool(
            "drag_resize_works",
            (width_after_drag - width_after_key).abs() > 20.0,
        )
        .f("editor_min_px", editor_min as f64)
        .f("editor_max_px", editor_max as f64)
        .f("editor_min_expected_px", app::MIN_EDITOR as f64)
        .f("editor_max_expected_px", expected_editor_max as f64)
        .f("workspace_min_px", workspace_min as f64)
        .f("workspace_max_px", workspace_max as f64)
        .f("workspace_max_expected_px", expected_workspace_max as f64)
        .f("logs_min_px", logs_min as f64)
        .f("logs_max_px", logs_max as f64)
        .f("logs_max_expected_px", expected_logs_max as f64)
        .bool("layout_fits_1280x800", fits_1280)
        .bool("layout_fits_900x600", fits_900)
        .bool("layout_fits_640x400", fits_640)
        .bool(
            "minimum_sizes_hold",
            (editor_min - app::MIN_EDITOR).abs() < 0.5
                && (editor_max - expected_editor_max).abs() < 0.5
                && (workspace_min - app::MIN_WORKSPACE).abs() < 0.5
                && (workspace_max - expected_workspace_max).abs() < 0.5
                && (logs_min - app::MIN_LOGS).abs() < 0.5
                && (logs_max - expected_logs_max).abs() < 0.5,
        );
    root.obj("s5", s5);


    // ---- S6: HiDPI -------------------------------------------------------------------
    // A real HiDPI window keeps its PHYSICAL size while the zoom factor changes, so the
    // available space in points shrinks as ppp grows. Two physical window sizes are
    // measured: 1920x1200 (room for the configured proportions at every zoom) and
    // 1280x800 (where the minimum pane sizes start to bind at 1.5x/2.0x - reported, and
    // still required to fit without overlap because minimums are compressed when tight).
    app.editor_frac = 0.40;
    app.workspace_frac = 0.20;
    app.logs_frac = 0.18;
    app.editor.set_active(0);
    ctx.set_zoom_factor(1.0);
    let _ = pass(&ctx, &mut app, vec![], &mut t);

    let roomy = s6_scenario(
        &ctx,
        &mut app,
        &mut t,
        Rect::from_min_size(Pos2::ZERO, vec2(2560.0, 1600.0)),
    );
    let tight = s6_scenario(&ctx, &mut app, &mut t, screen());

    let row_h_equal = {
        let all: Vec<f32> = roomy
            .rows
            .iter()
            .chain(tight.rows.iter())
            .map(|r| r.row_h_pt)
            .collect();
        all.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01)
    };
    let text_scales = {
        let a = &roomy.rows;
        ((a[1].char_w_px / a[0].char_w_px) - 1.5).abs() < 0.03
            && ((a[2].char_w_px / a[0].char_w_px) - 2.0).abs() < 0.03
            && ((tight.rows[1].char_w_px / tight.rows[0].char_w_px) - 1.5).abs() < 0.03
            && ((tight.rows[2].char_w_px / tight.rows[0].char_w_px) - 2.0).abs() < 0.03
    };
    let rasters_scale = roomy.rows[1].tex_w as f32 > roomy.rows[0].tex_w as f32 * 1.3
        && roomy.rows[2].tex_w as f32 > roomy.rows[1].tex_w as f32 * 1.1;

    let mut s6 = Json::new();
    s6.raw("physical_2560x1600", &roomy.to_json())
        .raw("physical_1280x800", &tight.to_json())
        .bool("text_row_height_stable_pt", row_h_equal)
        .bool("glyph_pixels_scale_with_ppp", text_scales)
        .bool("plot_texture_pixels_scale", rasters_scale)
        .bool(
            "pane_proportions_hold_when_room",
            roomy.proportions_stable && roomy.proportions_match_config,
        )
        .bool("minimums_bind_at_1280x800_high_zoom", !tight.proportions_match_config)
        .bool("no_overflow_at_any_zoom", roomy.no_overflow && tight.no_overflow)
        .str(
            "crispness_note",
            "bitmap crispness itself is verified by the orchestrator's screenshots; this records ppp and physical glyph sizes",
        );
    root.obj("s6", s6);

    HeadlessReport { root }
}

/// Everything measured without a window. S7 is added once the window phase ends.
#[derive(Clone)]
pub struct HeadlessReport {
    pub root: Json,
}

fn limits() -> Vec<String> {
    vec![
        "typing latency is CPU-side (UI build + tessellation), not GPU present/swap".to_string(),
        "IME composition was not driven; only the field and event counter are verified".to_string(),
        "crispness at 1.5x/2.0x is asserted from ppp/physical glyph sizes, not pixel diffing".to_string(),
        "UIA tree enumeration is done by the orchestrator against the open window".to_string(),
        "one machine, one GPU backend (wgpu), dev profile with opt-level=2".to_string(),
        "the windowed phase's self-close path is flaky in this environment; a watchdog can print the headless S1-S6 JSON alone".to_string(),
    ]
}

fn s7_json(win: &WindowBenchResult) -> Json {
    let mut s7 = Json::new();
    s7.f(
        "startup_first_ui_pass_ms",
        win.startup_first_ui_pass_ms.unwrap_or(f64::NAN),
    )
    .f(
        "startup_first_ui_build_ms",
        win.startup_first_ui_build_ms.unwrap_or(f64::NAN),
    )
    .f(
        "startup_first_paint_ms",
        win.startup_first_paint_ms.unwrap_or(f64::NAN),
    )
    .int("startup_first_paint_frame_number", 2)
    .f("idle_wall_ms", win.idle_wall_ms)
    .f("idle_cpu_ms", win.idle_cpu_ms.unwrap_or(f64::NAN))
    .f("idle_cpu_pct", win.idle_cpu_pct.unwrap_or(f64::NAN))
    .int("idle_frames", win.idle_frames)
    .str("idle_window", "5.0 s, real window, no synthetic input")
    .str(
        "startup_metric",
        "ms from OS process creation (GetProcessTimes) to the first App::ui pass / to the start of pass 2 (first frame submitted)",
    );
    if let Some(err) = &win.window_error {
        s7.str("window_error", err);
    }
    s7
}

/// Complete line: headless S1-S6 + windowed S7 (the normal path).
pub fn finalize(
    report: HeadlessReport,
    window: Option<WindowBenchResult>,
    bench_total_ms: f64,
) -> String {
    let mut root = report.root;
    root.f("bench_total_ms", bench_total_ms);
    let win = window.unwrap_or_default();
    let timeout = win.startup_first_paint_ms.is_none();
    let mut s7 = s7_json(&win);
    s7.bool("window_timeout", false).bool("window_closed", !timeout);
    root.obj("s7", s7);
    root.bool("window_closed", !timeout);
    root.bool("window_timeout", false);
    let mut ls = limits();
    if timeout {
        ls.push("window phase ended without startup/idle numbers".to_string());
    }
    root.arr_str("limits", &ls);
    root.finish()
}

/// Fallback printed by the watchdog when the window never closes: every headless S1-S6
/// number is intact, S7 is explicitly null and the timeout is flagged.
pub fn window_timeout_line(report: &HeadlessReport, bench_total_ms: f64, reason: &str) -> String {
    let mut root = report.root.clone();
    root.f("bench_total_ms", bench_total_ms);
    let mut s7 = Json::new();
    s7.bool("window_timeout", true)
        .bool("window_closed", false)
        .str("window_error", reason)
        .f("startup_first_ui_pass_ms", f64::NAN)
        .f("startup_first_ui_build_ms", f64::NAN)
        .f("startup_first_paint_ms", f64::NAN)
        .f("idle_wall_ms", f64::NAN)
        .f("idle_cpu_ms", f64::NAN)
        .f("idle_cpu_pct", f64::NAN)
        .int("idle_frames", 0)
        .str(
            "idle_window",
            "not measured: the window did not close before the watchdog deadline",
        )
        .str(
            "startup_metric",
            "not measured: the window did not close before the watchdog deadline",
        );
    root.obj("s7", s7);
    root.bool("window_closed", false).bool("window_timeout", true);
    let mut ls = limits();
    ls.push(reason.to_string());
    ls.push(
        "windowed metrics (S7) are null because the window never closed; S1-S6 are complete"
            .to_string(),
    );
    root.arr_str("limits", &ls);
    root.finish()
}
