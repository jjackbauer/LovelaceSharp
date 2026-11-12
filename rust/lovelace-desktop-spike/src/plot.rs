//! SVG plot pane: resvg + tiny-skia -> egui texture, wheel zoom around the pointer,
//! drag pan, Fit, and an inline (non-panicking) error for malformed SVG.

use std::time::Instant;

// resvg 0.50 pins tiny-skia 0.11.4 and re-exports it (resvg-0.50.0/src/lib.rs:17).
// The direct `tiny-skia` 0.12 crate would be a different, type-incompatible version.
use resvg::tiny_skia;
use resvg::usvg;

use egui::{
    Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, Ui, Vec2, pos2, vec2,
};

use crate::sample;

pub const PLOT_BG: Color32 = Color32::from_rgb(0x0c, 0x10, 0x14);
pub const PLOT_BORDER: Color32 = Color32::from_rgb(0x33, 0x3a, 0x44);
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);

/// Texture side cap; also bounded by `max_texture_side` reported by the backend.
const MAX_SIDE: f32 = 4096.0;

pub struct Plot {
    pub options: usvg::Options<'static>,
    pub tree: Option<usvg::Tree>,
    pub error: Option<String>,
    pub use_bad: bool,
    pub good_src: String,
    pub bad_src: String,
    pub svg_w: f32,
    pub svg_h: f32,
    pub texture: Option<TextureHandle>,
    pub raster_scale: f32,
    pub raster_px: [u32; 2],
    pub last_raster_ms: f64,
    pub last_upload_ms: f64,
    pub fontdb_ms: f64,
    pub parse_ms: f64,
    pub malformed_rejected: bool,
    pub malformed_error: String,
    pub zoom: f32,
    pub pan: Vec2,
    pub canvas: Rect,
    pub last_screen: Rect,
    pub last_pointer: Option<Pos2>,
}

#[derive(Clone, Copy)]
pub struct PlotMetrics {
    pub screen: Rect,
    pub zoom: f32,
    pub pan: Vec2,
    pub pointer: Option<Pos2>,
}

/// Render an SVG tree into a premultiplied RGBA pixmap at `scale`, returning the
/// pixmap and the wall time of the resvg call (excluding parse and texture upload).
pub fn render_to_pixmap(
    tree: &usvg::Tree,
    svg_w: f32,
    svg_h: f32,
    scale: f32,
    max_side: f32,
) -> Option<(tiny_skia::Pixmap, f64)> {
    let w_pt = svg_w.max(1.0);
    let h_pt = svg_h.max(1.0);
    let mut s = scale.clamp(0.25, 8.0);
    let cap = max_side.min(MAX_SIDE).max(64.0);
    let mut px_w = (w_pt * s).round().max(1.0);
    let mut px_h = (h_pt * s).round().max(1.0);
    if px_w.max(px_h) > cap {
        let f = cap / px_w.max(px_h);
        px_w = (px_w * f).max(1.0);
        px_h = (px_h * f).max(1.0);
        s *= f;
    }
    let mut pixmap = tiny_skia::Pixmap::new(px_w as u32, px_h as u32)?;
    let t0 = Instant::now();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(px_w / w_pt, px_h / h_pt),
        &mut pixmap.as_mut(),
    );
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let _ = s;
    Some((pixmap, ms))
}

/// Bench helper: median of `iters` rasterisations of the sample SVG at `scale`.
pub fn bench_raster(
    tree: &usvg::Tree,
    svg_w: f32,
    svg_h: f32,
    scale: f32,
    iters: usize,
) -> (f64, [u32; 2]) {
    let mut samples = Vec::with_capacity(iters);
    let mut px = [0u32; 2];
    for _ in 0..iters {
        if let Some((pixmap, ms)) = render_to_pixmap(tree, svg_w, svg_h, scale, MAX_SIDE) {
            px = [pixmap.width(), pixmap.height()];
            samples.push(ms);
        }
    }
    let mut tmp = samples.clone();
    let median = crate::json::median(&mut tmp);
    (median, px)
}

impl Plot {
    pub fn new(load_system_fonts: bool) -> Self {
        let t0 = Instant::now();
        let mut options = usvg::Options::default();
        if load_system_fonts {
            // resvg/usvg do not load system fonts by default; the usvg CLI does this
            // explicitly (usvg-0.50.0/src/main.rs:346).
            options.fontdb_mut().load_system_fonts();
        }
        let fontdb_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let good_src = sample::svg_plot();
        let bad_src = sample::SVG_MALFORMED.to_string();

        let mut plot = Plot {
            options,
            tree: None,
            error: None,
            use_bad: false,
            good_src,
            bad_src,
            svg_w: 640.0,
            svg_h: 400.0,
            texture: None,
            raster_scale: 0.0,
            raster_px: [0, 0],
            last_raster_ms: 0.0,
            last_upload_ms: 0.0,
            fontdb_ms,
            parse_ms: 0.0,
            malformed_rejected: false,
            malformed_error: String::new(),
            zoom: 1.0,
            pan: Vec2::ZERO,
            canvas: Rect::NOTHING,
            last_screen: Rect::NOTHING,
            last_pointer: None,
        };
        let good = plot.good_src.clone();
        plot.load(&good);

        // Prove the malformed path yields Err (never a panic) and remember the message.
        match usvg::Tree::from_str(&plot.bad_src, &plot.options) {
            Ok(_) => {
                plot.malformed_rejected = false;
                plot.malformed_error = "parser accepted the malformed SVG".to_string();
            }
            Err(e) => {
                plot.malformed_rejected = true;
                plot.malformed_error = format!("{e}");
            }
        }
        plot
    }

    pub fn load(&mut self, src: &str) -> bool {
        let t0 = Instant::now();
        match usvg::Tree::from_str(src, &self.options) {
            Ok(tree) => {
                self.svg_w = tree.size().width();
                self.svg_h = tree.size().height();
                self.tree = Some(tree);
                self.error = None;
                self.texture = None;
                self.raster_scale = 0.0;
                self.parse_ms = t0.elapsed().as_secs_f64() * 1000.0;
                true
            }
            Err(e) => {
                self.tree = None;
                self.texture = None;
                self.raster_scale = 0.0;
                self.error = Some(format!("{e}"));
                self.parse_ms = t0.elapsed().as_secs_f64() * 1000.0;
                false
            }
        }
    }

    pub fn toggle_bad(&mut self) {
        self.use_bad = !self.use_bad;
        if self.use_bad {
            let bad = self.bad_src.clone();
            self.load(&bad);
        } else {
            let good = self.good_src.clone();
            self.load(&good);
        }
    }

    pub fn fit(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    pub fn ensure(&mut self, ctx: &egui::Context, desired_scale: f32) {
        if self.tree.is_none() {
            return;
        }
        let desired = desired_scale.clamp(0.5, 4.0);
        let stale = self.texture.is_none()
            || (desired / self.raster_scale.max(0.01) - 1.0).abs() > 0.12;
        if stale {
            self.rasterize(ctx, desired);
        }
    }

    pub fn rasterize(&mut self, ctx: &egui::Context, scale: f32) -> bool {
        let Some(tree) = &self.tree else {
            return false;
        };
        let max_side = ctx.input(|i| i.max_texture_side).max(512) as f32;
        let Some((pixmap, raster_ms)) =
            render_to_pixmap(tree, self.svg_w, self.svg_h, scale, max_side)
        else {
            self.error = Some("pixmap allocation failed".to_string());
            return false;
        };
        let t1 = Instant::now();
        let image = egui::ColorImage::from_rgba_premultiplied(
            [pixmap.width() as usize, pixmap.height() as usize],
            pixmap.data(),
        );
        self.texture = Some(ctx.load_texture("svg_plot", image, TextureOptions::LINEAR));
        self.last_upload_ms = t1.elapsed().as_secs_f64() * 1000.0;
        self.last_raster_ms = raster_ms;
        self.raster_px = [pixmap.width(), pixmap.height()];
        self.raster_scale = (pixmap.width() as f32 / self.svg_w.max(1.0)).max(0.01);
        true
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) -> PlotMetrics {
        self.canvas = rect;
        if let Some(err) = self.error.clone() {
            ui.painter().rect_filled(rect, 0.0, PLOT_BG);
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                0.0,
                Stroke::new(1.0, ERROR),
                StrokeKind::Inside,
            );
            let inner = rect.shrink(12.0);
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(inner)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| {
                    ui.colored_label(ERROR, "SVG parse error (shown inline, no panic):");
                    ui.add(egui::Label::new(
                        egui::RichText::new(err).monospace().color(ERROR),
                    ));
                },
            );
            return PlotMetrics {
                screen: rect,
                zoom: self.zoom,
                pan: self.pan,
                pointer: None,
            };
        }

        let avail = rect.shrink(8.0);
        let base = (avail.width() / self.svg_w.max(1.0))
            .min(avail.height() / self.svg_h.max(1.0))
            .max(0.01);

        let resp = ui.interact(rect, egui::Id::unique("plot_canvas"), Sense::click_and_drag());
        if resp.dragged() {
            self.pan += resp.drag_delta();
        }
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let old = self.zoom;
                let new = (old * (scroll * 0.0025).exp()).clamp(0.25, 16.0);
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let rel = (p - rect.center() - self.pan) / (base * old);
                    self.pan = p - rect.center() - rel * (base * new);
                }
                self.zoom = new;
            }
            let dragging = resp.dragged();
            ui.ctx().set_cursor_icon(if dragging {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Grab
            });
        }
        let pointer = ui.input(|i| i.pointer.hover_pos());

        let size = vec2(self.svg_w, self.svg_h) * base * self.zoom;
        let screen = Rect::from_center_size(rect.center() + self.pan, size);
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, PLOT_BG);
        if let Some(tex) = &self.texture {
            painter.image(
                tex.id(),
                screen,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "raster unavailable",
                FontId::proportional(14.0),
                ERROR,
            );
        }
        painter.rect_stroke(
            rect.shrink(1.0),
            0.0,
            Stroke::new(1.0, PLOT_BORDER),
            StrokeKind::Inside,
        );

        let label = format!(
            "SVG plot canvas, {:.0} by {:.0} points, zoom {:.2}",
            self.svg_w, self.svg_h, self.zoom
        );
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Image, true, label.clone())
        });

        self.last_screen = screen;
        self.last_pointer = pointer;
        PlotMetrics {
            screen,
            zoom: self.zoom,
            pan: self.pan,
            pointer,
        }
    }
}


