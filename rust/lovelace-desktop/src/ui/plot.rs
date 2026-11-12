//! SVG plot pane: resvg + tiny-skia -> egui texture, pointer-anchored wheel zoom, drag pan,
//! Fit, a 100% (natural size) reset, PNG/SVG export, an inline (non-panicking) error for
//! malformed SVG, and an explicit empty state.
//!
//! Adapted from `rust/lovelace-desktop-spike/src/plot.rs`. The spike loaded a fixed sample
//! SVG; this version loads exactly the `PlotCapture.svg` the engine attached to the document's
//! last run. When no run has produced a plot, the pane shows an empty state instead of sample
//! data.
//!
//! PLT-002: the renderer is configured for self-contained documents. External `xlink:href`
//! strings are refused (only `data:` images resolve) and `<script>` is never executed by usvg;
//! `raster_cli` exposes the same rasterisation path to the round-29 fidelity harness.
//!
//! PLT-004: [`PlotView`] is the whole zoom/pan transform as a value with no egui state in it.
//! The wheel handler calls [`PlotView::zoom_at`], which keeps the scene point under the pointer
//! stationary, and every mutation ends in [`PlotView::clamped`], which refuses a pan that would
//! push more than all but [`PlotView::MIN_VISIBLE`] screen points of the scene out of the pane.
//! Fit and 100% are two different views: Fit is "the whole document in the pane" and 100% is
//! "one SVG point per logical screen pixel", centred.
//!
//! PLT-006: [`Plot::export_svg_to`] writes `svg_source` unchanged (the engine's own bytes), and
//! [`Plot::export_png_to`] rasterises the parsed tree at a chosen scale and background. The
//! native save dialog itself lives in the window (`ui/app.rs`) over the platform dialog already
//! used by the legacy shell.

use std::path::{Path, PathBuf};
use std::time::Instant;

// resvg 0.50 pins tiny-skia 0.11.4 and re-exports it. A direct `tiny-skia` 0.12 crate would be
// a different, type-incompatible version.
use resvg::tiny_skia;
use resvg::usvg;

use egui::{
    Align2, Color32, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    TextureOptions, Ui, Vec2, pos2, vec2,
};

/// Plot background.
pub const PLOT_BG: Color32 = Color32::from_rgb(0x0c, 0x10, 0x14);
/// Plot border.
pub const PLOT_BORDER: Color32 = Color32::from_rgb(0x33, 0x3a, 0x44);
/// Error text and border.
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x7b, 0x72);
/// Secondary text.
pub const DIM: Color32 = Color32::from_rgb(0x8b, 0x98, 0xa9);

/// Texture side cap; also bounded by `max_texture_side` reported by the backend.
const MAX_SIDE: f32 = 4096.0;
/// Padding the fit scale leaves around the document, in screen points.
pub const FIT_MARGIN: f32 = 8.0;
/// Largest PNG export side, in pixels. A larger request is refused instead of exhausting memory.
pub const MAX_EXPORT_SIDE: u32 = 16_384;
/// One wheel notch of zoom change, as a multiplier applied around the pointer.
pub const WHEEL_STEP: f32 = 1.25;

/// A background choice for a PNG export.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PngBackground {
    /// Fill the canvas with the engine plot's own white before drawing.
    Opaque,
    /// Leave untouched pixels fully transparent.
    Transparent,
}

impl PngBackground {
    /// The UI label, also used in the export status line.
    pub fn label(self) -> &'static str {
        match self {
            PngBackground::Opaque => "Opaque white",
            PngBackground::Transparent => "Transparent",
        }
    }

    /// The colour the pixmap is filled with before rendering, if any.
    fn fill(self) -> Option<tiny_skia::Color> {
        match self {
            PngBackground::Opaque => Some(tiny_skia::Color::WHITE),
            PngBackground::Transparent => None,
        }
    }
}

/// The plot pane's zoom and pan, as a pure value.
///
/// `zoom` is relative to the fit scale: 1.0 means the whole document is visible in the pane.
/// `pan` is the offset of the document's centre from the pane's centre, in screen points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotView {
    /// Zoom factor relative to [`PlotView::fit_scale`].
    pub zoom: f32,
    /// Pan offset, in screen points.
    pub pan: Vec2,
}

impl Default for PlotView {
    fn default() -> PlotView {
        PlotView::IDENTITY
    }
}

impl PlotView {
    /// Fit-to-pane with no pan: the identity view a Fit click restores.
    pub const IDENTITY: PlotView = PlotView {
        zoom: 1.0,
        pan: Vec2::ZERO,
    };
    /// The furthest the wheel can zoom out.
    pub const MIN_ZOOM: f32 = 0.05;
    /// The furthest the wheel can zoom in.
    pub const MAX_ZOOM: f32 = 64.0;
    /// How many screen points of the scene must stay inside the pane on each axis.
    pub const MIN_VISIBLE: f32 = 32.0;

    /// Screen points per SVG point at zoom 1.0: the whole document, with a small margin,
    /// fitted in the pane.
    pub fn fit_scale(pane: Rect, svg: Vec2) -> f32 {
        let available = pane.shrink(FIT_MARGIN);
        (available.width() / svg.x.max(1.0))
            .min(available.height() / svg.y.max(1.0))
            .max(0.01)
    }

    /// Screen points per SVG point for this view.
    pub fn scale(self, fit: f32) -> f32 {
        fit * self.zoom
    }

    /// The scene's size on screen.
    pub fn scene_size(self, svg: Vec2, fit: f32) -> Vec2 {
        svg * self.scale(fit)
    }

    /// The document's rectangle on screen.
    pub fn scene_rect(self, pane: Rect, svg: Vec2, fit: f32) -> Rect {
        Rect::from_center_size(pane.center() + self.pan, self.scene_size(svg, fit))
    }

    /// The scene coordinate (relative to the document's centre, in SVG points) under a screen
    /// point.
    pub fn screen_to_scene(self, screen: Pos2, pane: Rect, fit: f32) -> Vec2 {
        (screen - pane.center() - self.pan) / self.scale(fit).max(1e-6)
    }

    /// The screen position of a scene coordinate (relative to the document's centre).
    #[allow(dead_code)] // the transform tests use it to check the round trip through the pane
    pub fn scene_to_screen(self, scene: Vec2, pane: Rect, fit: f32) -> Pos2 {
        pane.center() + self.pan + scene * self.scale(fit)
    }

    /// Zooms by `factor` while keeping the scene point under `pointer` stationary.
    ///
    /// The anchor is computed from the *current* transform, then the new pan is derived so the
    /// same scene point maps back to the same screen point. The result is clamped: a wheel turn
    /// at a pane edge can move the anchor only when the clamp has to keep the scene visible.
    pub fn zoom_at(self, pointer: Pos2, factor: f32, pane: Rect, svg: Vec2, fit: f32) -> PlotView {
        let old_zoom = self.zoom.max(1e-6);
        let new_zoom = (old_zoom * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if (new_zoom - old_zoom).abs() <= f32::EPSILON {
            return self.clamped(pane, svg, fit);
        }
        let anchor = self.screen_to_scene(pointer, pane, fit);
        let offset = pointer - pane.center();
        let view = PlotView {
            zoom: new_zoom,
            pan: offset - anchor * fit * new_zoom,
        };
        view.clamped(pane, svg, fit)
    }

    /// Pans by a screen delta, clamped.
    pub fn pan_by(self, delta: Vec2, pane: Rect, svg: Vec2, fit: f32) -> PlotView {
        PlotView {
            zoom: self.zoom,
            pan: self.pan + delta,
        }
        .clamped(pane, svg, fit)
    }

    /// Returns the view at natural size: one SVG point per logical screen pixel, centred.
    pub fn actual_size(self, pane: Rect, svg: Vec2, fit: f32) -> PlotView {
        let zoom = (1.0 / fit.max(1e-6)).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        PlotView {
            zoom,
            pan: Vec2::ZERO,
        }
        .clamped(pane, svg, fit)
    }

    /// The zoom as a percentage of the document's natural size (1 SVG point = 1 logical pixel).
    pub fn percent(self, fit: f32) -> f32 {
        self.scale(fit) * 100.0
    }

    /// Clamps the zoom into range and the pan so the scene cannot leave the pane.
    ///
    /// A scene that fits on an axis is centred on that axis: panning it would only hide part of
    /// a document that is already fully visible, so the transform refuses it. A scene larger
    /// than the pane may be panned until exactly [`Self::MIN_VISIBLE`] screen points remain
    /// visible at the leading or trailing edge, never further.
    pub fn clamped(self, pane: Rect, svg: Vec2, fit: f32) -> PlotView {
        let zoom = self.zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        if !pane.is_positive() {
            return PlotView {
                zoom,
                pan: self.pan,
            };
        }
        let size = svg * fit * zoom;
        let pane_size = pane.size();
        let mut pan = self.pan;
        for axis in 0..2 {
            let scene = size[axis];
            let available = pane_size[axis];
            if scene <= available {
                pan[axis] = 0.0;
            } else {
                // The scene centre may move until only MIN_VISIBLE points of it remain visible
                // on one side: |pan| <= pane/2 - MIN_VISIBLE + scene/2.
                let limit = (available * 0.5 - Self::MIN_VISIBLE + scene * 0.5).max(0.0);
                pan[axis] = pan[axis].clamp(-limit, limit);
            }
        }
        PlotView { zoom, pan }
    }
}

/// What one plot pass painted, for the status strip and for measurement.
#[allow(dead_code)] // the measurement fields stay part of the measured plot surface
#[derive(Clone, Copy)]
pub struct PlotMetrics {
    /// The pane rectangle.
    pub screen: Rect,
    /// Zoom factor.
    pub zoom: f32,
    /// Pan offset.
    pub pan: Vec2,
    /// Pointer position, if hovering.
    pub pointer: Option<Pos2>,
}

/// Rasterises an SVG tree into a premultiplied RGBA pixmap at `scale`, returning the pixmap and
/// the wall time of the resvg call (excluding parse and texture upload).
pub fn render_to_pixmap(
    tree: &usvg::Tree,
    svg_w: f32,
    svg_h: f32,
    scale: f32,
    max_side: f32,
) -> Option<(tiny_skia::Pixmap, f64)> {
    let width_points = svg_w.max(1.0);
    let height_points = svg_h.max(1.0);
    let scale = scale.clamp(0.25, 8.0);
    let cap = max_side.clamp(64.0, MAX_SIDE);
    let mut pixel_width = (width_points * scale).round().max(1.0);
    let mut pixel_height = (height_points * scale).round().max(1.0);
    if pixel_width.max(pixel_height) > cap {
        let factor = cap / pixel_width.max(pixel_height);
        pixel_width = (pixel_width * factor).max(1.0);
        pixel_height = (pixel_height * factor).max(1.0);
    }
    let mut pixmap = tiny_skia::Pixmap::new(pixel_width as u32, pixel_height as u32)?;
    let started = Instant::now();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(pixel_width / width_points, pixel_height / height_points),
        &mut pixmap.as_mut(),
    );
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    Some((pixmap, elapsed))
}

/// The exact pixel size a PNG export at `scale` produces.
///
/// This is the size [`Plot::export_png_to`] writes and the size the export dialog advertises:
/// `round(svg_width * scale)` by `round(svg_height * scale)`, never resampled afterwards.
pub fn export_pixels(svg_w: f32, svg_h: f32, scale: f32) -> Result<(u32, u32), String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!("export scale {scale} is not a positive number"));
    }
    let width = (svg_w.max(1.0) * scale).round();
    let height = (svg_h.max(1.0) * scale).round();
    if width > MAX_EXPORT_SIDE as f32 || height > MAX_EXPORT_SIDE as f32 {
        return Err(format!(
            "export at {scale}x would be {width} by {height} px; the limit is {MAX_EXPORT_SIDE} px per side"
        ));
    }
    Ok(((width.max(1.0)) as u32, (height.max(1.0)) as u32))
}

/// The plot pane's state: current SVG tree, texture, view, export source and last raster timings.
pub struct Plot {
    options: usvg::Options<'static>,
    tree: Option<usvg::Tree>,
    /// Current SVG source; empty means "nothing to show yet". This is the engine's capture
    /// string, byte-for-byte, and SVG export writes it unchanged.
    pub svg_source: String,
    /// Parse or rasterisation error, shown inline.
    pub error: Option<String>,
    /// Width of the SVG in points.
    pub svg_w: f32,
    /// Height of the SVG in points.
    pub svg_h: f32,
    /// Rasterised texture, when the SVG parsed.
    pub texture: Option<TextureHandle>,
    /// Scale the texture was rasterised at.
    pub raster_scale: f32,
    /// Pixel size of the last raster.
    pub raster_px: [u32; 2],
    /// Last resvg raster time in milliseconds.
    pub last_raster_ms: f64,
    /// Last texture upload time in milliseconds.
    pub last_upload_ms: f64,
    /// Time spent loading system fonts at startup.
    #[allow(dead_code)] // startup measurement kept for the performance report
    pub fontdb_ms: f64,
    /// Last parse time in milliseconds.
    pub parse_ms: f64,
    /// Current zoom and pan.
    pub view: PlotView,
    /// Fit scale measured on the last painted pass; the header's percentage uses it.
    pub fit_scale: f32,
    /// The rectangle last painted.
    pub canvas: Rect,
    /// The screen rectangle last occupied by the SVG image.
    pub last_screen: Rect,
    /// Pointer position on the last pass.
    pub last_pointer: Option<Pos2>,
}

impl Default for Plot {
    fn default() -> Plot {
        Plot::new()
    }
}

impl Plot {
    /// Creates an empty plot pane and loads the system fonts used by engine SVG text.
    pub fn new() -> Plot {
        let started = Instant::now();
        let mut options = usvg::Options::default();
        // resvg/usvg do not load system fonts by default; the usvg CLI does this explicitly.
        options.fontdb_mut().load_system_fonts();
        // PLT-002: an engine plot is self-contained. usvg's default string resolver would open
        // an absolute path (`<image href="C:/...">`), so refuse every non-data href instead of
        // letting a hostile document make the renderer read the local disk. Data URLs still
        // resolve; scripts are not an SVG feature usvg implements, so `<script>` is inert.
        options.image_href_resolver = usvg::ImageHrefResolver {
            resolve_data: usvg::ImageHrefResolver::default_data_resolver(),
            resolve_string: Box::new(|_, _| None),
        };
        let fontdb_ms = started.elapsed().as_secs_f64() * 1000.0;
        Plot {
            options,
            tree: None,
            svg_source: String::new(),
            error: None,
            svg_w: 640.0,
            svg_h: 400.0,
            texture: None,
            raster_scale: 0.0,
            raster_px: [0, 0],
            last_raster_ms: 0.0,
            last_upload_ms: 0.0,
            fontdb_ms,
            parse_ms: 0.0,
            view: PlotView::IDENTITY,
            fit_scale: 1.0,
            canvas: Rect::NOTHING,
            last_screen: Rect::NOTHING,
            last_pointer: None,
        }
    }

    /// True when no plot has ever been loaded, i.e. the pane should show its empty state.
    pub fn is_empty(&self) -> bool {
        self.svg_source.is_empty()
    }

    /// The document's size, in SVG points.
    pub fn svg_size(&self) -> Vec2 {
        vec2(self.svg_w.max(1.0), self.svg_h.max(1.0))
    }

    /// Returns the pane to its empty state.
    pub fn clear(&mut self) {
        self.tree = None;
        self.texture = None;
        self.error = None;
        self.svg_source.clear();
        self.raster_scale = 0.0;
        self.raster_px = [0, 0];
        self.view = PlotView::IDENTITY;
        self.fit_scale = 1.0;
        self.last_raster_ms = 0.0;
        self.last_upload_ms = 0.0;
    }

    /// Loads an SVG document (the engine's `PlotCapture.svg`). Returns false when resvg rejects
    /// it; the parse error is kept and rendered inline.
    pub fn set_svg(&mut self, source: &str) -> bool {
        let started = Instant::now();
        match usvg::Tree::from_str(source, &self.options) {
            Ok(tree) => {
                self.svg_w = tree.size().width();
                self.svg_h = tree.size().height();
                self.tree = Some(tree);
                self.error = None;
                self.texture = None;
                self.raster_scale = 0.0;
                self.svg_source = source.to_string();
                self.view = PlotView::IDENTITY;
                self.parse_ms = started.elapsed().as_secs_f64() * 1000.0;
                true
            }
            Err(error) => {
                self.tree = None;
                self.texture = None;
                self.raster_scale = 0.0;
                self.svg_source = source.to_string();
                self.error = Some(format!("{error}"));
                self.view = PlotView::IDENTITY;
                self.parse_ms = started.elapsed().as_secs_f64() * 1000.0;
                false
            }
        }
    }

    /// Rasterises the currently parsed tree through the pane's renderer at `scale`.
    ///
    /// This is the exact put-the-tree-on-a-pixmap path the pane uses in [`Plot::rasterize`]; the
    /// round-29 fidelity harness calls it through [`raster_cli`] so the product raster and the
    /// pane cannot drift apart.
    pub fn raster_pixmap(&self, scale: f32, max_side: f32) -> Option<(tiny_skia::Pixmap, f64)> {
        let tree = self.tree.as_ref()?;
        render_to_pixmap(tree, self.svg_w, self.svg_h, scale, max_side)
    }

    /// Resets zoom and pan so the whole plot fits the pane.
    pub fn fit(&mut self) {
        self.view = PlotView::IDENTITY;
    }

    /// Resets to natural size: one SVG point per logical screen pixel, centred.
    pub fn actual_size(&mut self) {
        let fit = self.fit_scale.max(1e-6);
        self.view = PlotView::IDENTITY.actual_size(self.canvas, self.svg_size(), fit);
    }

    /// One zoom step around the last hovered point (or the pane centre when not hovering).
    pub fn zoom_step(&mut self, factor: f32) {
        let pane = self.canvas;
        if !pane.is_positive() {
            return;
        }
        let fit = PlotView::fit_scale(pane, self.svg_size());
        self.fit_scale = fit;
        let pointer = self
            .last_pointer
            .filter(|pointer| pane.contains(*pointer))
            .unwrap_or_else(|| pane.center());
        self.view = self
            .view
            .zoom_at(pointer, factor, pane, self.svg_size(), fit);
    }

    /// The current zoom as a percentage of natural size.
    pub fn zoom_percent(&self) -> f32 {
        self.view.percent(self.fit_scale.max(1e-6))
    }

    /// Re-rasterises when the current texture is too coarse for the requested scale.
    pub fn ensure(&mut self, ctx: &egui::Context, desired_scale: f32) {
        if self.tree.is_none() {
            return;
        }
        let desired = desired_scale.clamp(0.25, 8.0);
        let stale =
            self.texture.is_none() || (desired / self.raster_scale.max(0.01) - 1.0).abs() > 0.12;
        if stale {
            self.rasterize(ctx, desired);
        }
    }

    /// Rasterises the current tree at `scale` and uploads it as a texture.
    pub fn rasterize(&mut self, ctx: &egui::Context, scale: f32) -> bool {
        let Some(tree) = &self.tree else {
            return false;
        };
        let max_side = ctx.input(|input| input.max_texture_side).max(512) as f32;
        let Some((pixmap, raster_ms)) =
            render_to_pixmap(tree, self.svg_w, self.svg_h, scale, max_side)
        else {
            self.error = Some("pixmap allocation failed".to_string());
            return false;
        };
        let started = Instant::now();
        let image = egui::ColorImage::from_rgba_premultiplied(
            [pixmap.width() as usize, pixmap.height() as usize],
            pixmap.data(),
        );
        self.texture = Some(ctx.load_texture("svg_plot", image, TextureOptions::LINEAR));
        self.last_upload_ms = started.elapsed().as_secs_f64() * 1000.0;
        self.last_raster_ms = raster_ms;
        self.raster_px = [pixmap.width(), pixmap.height()];
        self.raster_scale = (pixmap.width() as f32 / self.svg_w.max(1.0)).max(0.01);
        true
    }

    /// Paints the pane and handles zoom/pan. Returns what was painted.
    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) -> PlotMetrics {
        self.canvas = rect;
        if self.is_empty() {
            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, PLOT_BG);
            painter.rect_stroke(
                rect.shrink(1.0),
                0.0,
                Stroke::new(1.0, PLOT_BORDER),
                StrokeKind::Inside,
            );
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "No plot yet. Run a script that calls plot(...).",
                FontId::proportional(14.0),
                DIM,
            );
            return PlotMetrics {
                screen: rect,
                zoom: self.view.zoom,
                pan: self.view.pan,
                pointer: None,
            };
        }

        if let Some(error) = self.error.clone() {
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
                        egui::RichText::new(error).monospace().color(ERROR),
                    ));
                },
            );
            return PlotMetrics {
                screen: rect,
                zoom: self.view.zoom,
                pan: self.view.pan,
                pointer: None,
            };
        }

        let svg = self.svg_size();
        let fit = PlotView::fit_scale(rect, svg);
        self.fit_scale = fit;

        let response = ui.interact(
            rect,
            egui::Id::unique("plot_canvas"),
            Sense::click_and_drag(),
        );
        if response.dragged() {
            self.view = self.view.pan_by(response.drag_delta(), rect, svg, fit);
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                // An exponential response keeps every wheel notch the same *ratio*, which is
                // what makes the zoom feel linear to the user. The anchor is the live pointer.
                let factor = (scroll * 0.0025).exp();
                if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                    self.view = self.view.zoom_at(pointer, factor, rect, svg, fit);
                }
            }
            let dragging = response.dragged();
            ui.ctx().set_cursor_icon(if dragging {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Grab
            });
        }
        let pointer = ui.input(|input| input.pointer.hover_pos());
        // The texture must carry at least the pixels the view puts on screen, so zooming in
        // requests a sharper raster instead of stretching the fit-resolution one.
        let pixels_per_point = ui.ctx().pixels_per_point();
        self.ensure(ui.ctx(), pixels_per_point * self.view.scale(fit));

        let screen = self.view.scene_rect(rect, svg, fit);
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, PLOT_BG);
        if let Some(texture) = &self.texture {
            painter.image(
                texture.id(),
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
            "SVG plot canvas, {:.0} by {:.0} points, zoom {:.2}, {:.0}%",
            self.svg_w,
            self.svg_h,
            self.view.zoom,
            self.view.percent(fit)
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, label.clone()));

        self.last_screen = screen;
        self.last_pointer = pointer;
        PlotMetrics {
            screen,
            zoom: self.view.zoom,
            pan: self.view.pan,
            pointer,
        }
    }

    /// The engine's SVG bytes, exactly as captured.
    pub fn svg_bytes(&self) -> Result<&[u8], String> {
        if self.svg_source.is_empty() {
            return Err("there is no plot SVG to export".to_string());
        }
        Ok(self.svg_source.as_bytes())
    }

    /// Writes the engine's captured SVG bytes unchanged to `path`.
    ///
    /// Nothing here re-serialises, normalises or reformats the document: the file is
    /// `PlotCapture.svg` byte for byte, so a consumer receives exactly what the engine produced.
    pub fn export_svg_to(&self, path: &Path) -> Result<u64, String> {
        let bytes = self.svg_bytes()?;
        std::fs::write(path, bytes)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        Ok(bytes.len() as u64)
    }

    /// Rasterises the parsed document for a PNG export at `scale` and `background`.
    ///
    /// Unlike the pane's texture path there is no texture-side cap here: the pixmap is exactly
    /// [`export_pixels`], and a request above [`MAX_EXPORT_SIDE`] is refused rather than
    /// silently shrunk.
    pub fn export_png(
        &self,
        scale: f32,
        background: PngBackground,
    ) -> Result<tiny_skia::Pixmap, String> {
        let tree = self
            .tree
            .as_ref()
            .ok_or_else(|| "there is no parsed plot to rasterise".to_string())?;
        let (width, height) = export_pixels(self.svg_w, self.svg_h, scale)?;
        let mut pixmap = tiny_skia::Pixmap::new(width, height)
            .ok_or_else(|| format!("cannot allocate a {width} by {height} px pixmap"))?;
        if let Some(color) = background.fill() {
            pixmap.fill(color);
        }
        resvg::render(
            tree,
            tiny_skia::Transform::from_scale(
                width as f32 / self.svg_w.max(1.0),
                height as f32 / self.svg_h.max(1.0),
            ),
            &mut pixmap.as_mut(),
        );
        Ok(pixmap)
    }

    /// Writes a PNG export and returns its pixel dimensions.
    pub fn export_png_to(
        &self,
        path: &Path,
        scale: f32,
        background: PngBackground,
    ) -> Result<[u32; 2], String> {
        let pixmap = self.export_png(scale, background)?;
        pixmap
            .save_png(path)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        Ok([pixmap.width(), pixmap.height()])
    }
}

/// A safe default file name for an export, derived from the plot title.
///
/// Only alphanumerics, spaces, dashes and underscores survive; everything else becomes a dash.
/// The result always ends in `.{extension}` and never contains a path separator, so the save
/// dialog opens in the current folder with a name the user can accept immediately.
pub fn export_file_name(title: Option<&str>, extension: &str) -> String {
    let stem: String = title
        .unwrap_or("plot")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ' ' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let stem = stem.trim().trim_matches('-').trim();
    let stem: String = stem.chars().take(64).collect();
    let stem = if stem.is_empty() {
        "plot"
    } else {
        stem.as_str()
    };
    format!("{stem}.{extension}")
}

/// The `--raster-svg <in.svg> --out <out.png> [--scale N]` acceptance mode.
///
/// It builds the same [`Plot`] the pane builds (including the system font database and the
/// PLT-002 resolver), parses the file through [`Plot::set_svg`] and rasterises it through
/// [`Plot::raster_pixmap`], so the round-29 fidelity harness compares the product rasteriser
/// rather than a look-alike. Exit codes: 0 rasterised, 1 rejected or failed to write, 2 usage.
pub fn raster_cli(args: &[String]) -> i32 {
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut scale = 1.0f32;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                let Some(value) = args.get(i + 1) else {
                    eprintln!("--out needs a path");
                    return 2;
                };
                output = Some(value.clone());
                i += 2;
            }
            "--scale" => {
                let Some(value) = args.get(i + 1) else {
                    eprintln!("--scale needs a number");
                    return 2;
                };
                match value.parse::<f32>() {
                    Ok(value) if value.is_finite() && value > 0.0 => scale = value,
                    _ => {
                        eprintln!("--scale is not a positive number: {value}");
                        return 2;
                    }
                }
                i += 2;
            }
            value if input.is_none() => {
                input = Some(value.to_string());
                i += 1;
            }
            other => {
                eprintln!("unexpected raster argument: {other}");
                return 2;
            }
        }
    }
    let (Some(input), Some(output)) = (input, output) else {
        eprintln!("usage: lovelace-desktop --raster-svg <in.svg> --out <out.png> [--scale N]");
        return 2;
    };
    let source = match std::fs::read_to_string(&input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {input}: {error}");
            return 1;
        }
    };
    let mut plot = Plot::new();
    if !plot.set_svg(&source) {
        eprintln!(
            "SVG rejected: {}",
            plot.error.as_deref().unwrap_or("unknown error")
        );
        return 1;
    }
    let Some((pixmap, raster_ms)) = plot.raster_pixmap(scale, MAX_SIDE) else {
        eprintln!("pixmap allocation failed for {input}");
        return 1;
    };
    if let Err(error) = pixmap.save_png(&output) {
        eprintln!("cannot write {output}: {error}");
        return 1;
    }
    println!(
        "raster svg={input} out={output} px={}x{} scale={:.3} raster_ms={raster_ms:.3}",
        pixmap.width(),
        pixmap.height(),
        scale
    );
    0
}

/// The PNG filter pair `save_png_dialog` offers.
pub const PNG_FILTER: (&str, &str) = ("PNG image (*.png)", "*.png");
/// The SVG filter pair `save_svg_dialog` offers.
pub const SVG_FILTER: (&str, &str) = ("SVG document (*.svg)", "*.svg");

/// Shows a save dialog for a plot export and returns the chosen path.
///
/// The window owns the dialog (see `ui/app.rs`); this helper only supplies the plot-specific
/// filter, title, extension and starting folder. `initial_dir` is the directory the engine
/// wrote the capture into: it is writable by construction, so a plain Save works even when the
/// shell's remembered folder is not writable (a redirected profile folder, for example). It is
/// Windows-only because the whole pane is.
#[cfg(windows)]
pub fn save_plot_dialog(
    title: &str,
    filter: (&str, &str),
    extension: &str,
    suggested: &str,
    initial_dir: Option<&Path>,
) -> Option<PathBuf> {
    let owner = crate::win32::ffi::foreground_window_of_this_process();
    crate::win32::ffi::save_file_dialog_with(
        owner,
        suggested,
        title,
        &[filter],
        extension,
        initial_dir,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rasterises a document through the product's own pipeline at 1:1.
    fn render(svg: &str) -> tiny_skia::Pixmap {
        let mut plot = Plot::new();
        assert!(plot.set_svg(svg), "SVG rejected: {:?}", plot.error);
        let (pixmap, _) = plot
            .raster_pixmap(1.0, MAX_SIDE)
            .expect("pixmap allocation");
        pixmap
    }

    fn rgb(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> (u8, u8, u8) {
        let color = pixmap.pixel(x, y).expect("pixel inside the pixmap");
        (color.red(), color.green(), color.blue())
    }

    fn is_white(pixel: (u8, u8, u8)) -> bool {
        pixel.0 > 245 && pixel.1 > 245 && pixel.2 > 245
    }

    fn has_dark_pixel(pixmap: &tiny_skia::Pixmap, x0: u32, y0: u32, x1: u32, y1: u32) -> bool {
        (y0..y1).any(|y| {
            (x0..x1).any(|x| {
                let (r, g, b) = rgb(pixmap, x, y);
                r < 128 && g < 128 && b < 128
            })
        })
    }

    /// A pane and an SVG size used by the transform tests: 800 x 600 points in a 1000 x 800
    /// pane, the round-29 window's proportions.
    fn pane() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0))
    }

    fn svg_size() -> Vec2 {
        vec2(800.0, 600.0)
    }

    fn fit() -> f32 {
        PlotView::fit_scale(pane(), svg_size())
    }

    fn scratch(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join("lovelace-desktop-plot-tests");
        std::fs::create_dir_all(&directory).expect("plot test directory");
        directory.join(name)
    }

    #[test]
    fn renders_clipping_transforms_dashed_paths_and_labels() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
  <defs><clipPath id="plot-clip"><rect x="0" y="0" width="50" height="50"/></clipPath></defs>
  <rect width="100" height="100" fill="#ffffff"/>
  <g clip-path="url(#plot-clip)"><rect x="0" y="0" width="100" height="100" fill="#ff0000"/></g>
  <g transform="translate(50 50)"><rect width="20" height="20" fill="#00ff00"/></g>
  <path d="M0 90 L100 90" fill="none" stroke="#0000ff" stroke-width="4" stroke-dasharray="10 6"/>
  <polyline points="10,70 40,80" fill="none" stroke="#000000" stroke-width="2"/>
  <text x="5" y="15" font-family="sans-serif" font-size="10">label</text>
</svg>"##;
        let pixmap = render(svg);
        assert_eq!((pixmap.width(), pixmap.height()), (100, 100));

        // The red rect is clipped to the left half; the right half stays white.
        assert_eq!(rgb(&pixmap, 25, 25), (255, 0, 0), "inside the clip path");
        assert!(is_white(rgb(&pixmap, 75, 25)), "outside the clip path");

        // The translated group lands at (50,50)..(70,70), not at the origin.
        assert_eq!(rgb(&pixmap, 55, 55), (0, 255, 0), "inside the transform");
        assert!(is_white(rgb(&pixmap, 5, 55)), "the untransformed position");

        // The dashed path paints its first dash and leaves the gap unpainted.
        let dash = rgb(&pixmap, 5, 90);
        assert!(dash.2 > 200 && dash.0 < 80, "dash pixel is blue: {dash:?}");
        assert!(
            is_white(rgb(&pixmap, 13, 90)),
            "dash gap stays unpainted: {:?}",
            rgb(&pixmap, 13, 90)
        );

        assert!(
            has_dark_pixel(&pixmap, 5, 4, 45, 18),
            "the text label paints ink"
        );
        assert!(
            has_dark_pixel(&pixmap, 8, 68, 43, 83),
            "the polyline paints ink"
        );
    }

    #[test]
    fn external_references_are_refused_and_scripts_do_not_run() {
        // A real local PNG proves that a resolvable absolute path is still refused. usvg's
        // default string resolver would open it; the pane's resolver must not.
        let mut marker = tiny_skia::Pixmap::new(16, 16).expect("marker pixmap");
        marker.fill(tiny_skia::Color::from_rgba8(255, 0, 255, 255));
        let marker_path = std::env::temp_dir().join("lovelace-desktop-plot-security-marker.png");
        marker.save_png(&marker_path).expect("write marker png");
        let absolute = marker_path.display().to_string().replace('\\', "/");
        let uri = format!("file:///{}", absolute.trim_start_matches('/'));

        let control = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80">
  <rect width="120" height="80" fill="#ffffff"/>
  <rect id="marker" x="10" y="10" width="40" height="40" fill="#ffffff"/>
  <text x="8" y="72" font-family="sans-serif" font-size="10">safe</text>
</svg>"##;
        let hostile = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="120" height="80">
  <script type="text/javascript">document.getElementById('marker').setAttribute('fill', '#ff00ff');</script>
  <rect width="120" height="80" fill="#ffffff"/>
  <image xlink:href="{uri}" x="0" y="0" width="120" height="80"/>
  <image href="{absolute}" x="0" y="0" width="120" height="80"/>
  <rect id="marker" x="10" y="10" width="40" height="40" fill="#ffffff"/>
  <text x="8" y="72" font-family="sans-serif" font-size="10">safe</text>
</svg>"##
        );

        let expected = render(control);
        let actual = render(&hostile);
        assert_eq!(
            (actual.width(), actual.height()),
            (expected.width(), expected.height())
        );
        assert_eq!(
            actual.data(),
            expected.data(),
            "an external image reference loaded or a script changed the document"
        );

        // The pane itself survives the hostile document: it reports no parse error, clears, and
        // renders a valid document again (the "app remains usable" half of PLT-002).
        let mut pane = Plot::new();
        assert!(
            pane.set_svg(&hostile),
            "hostile SVG rejected: {:?}",
            pane.error
        );
        assert!(
            pane.error.is_none(),
            "hostile SVG left an error: {:?}",
            pane.error
        );
        pane.clear();
        assert!(pane.set_svg(control));
        let (recovered, _) = pane
            .raster_pixmap(1.0, MAX_SIDE)
            .expect("pane pixmap after the hostile document");
        assert_eq!(recovered.data(), expected.data());

        let _ = std::fs::remove_file(&marker_path);
    }

    // -----------------------------------------------------------------------------------------
    // PLT-004: the transform math
    // -----------------------------------------------------------------------------------------

    #[test]
    fn fit_scale_shows_the_whole_document_inside_the_pane_margin() {
        let fit = fit();
        let scene = svg_size() * fit;
        assert!(
            scene.x <= pane().width() - 2.0 * FIT_MARGIN + 0.001,
            "width {} does not fit {}",
            scene.x,
            pane().width()
        );
        assert!(
            scene.y <= pane().height() - 2.0 * FIT_MARGIN + 0.001,
            "height {} does not fit {}",
            scene.y,
            pane().height()
        );
        // It is the largest such scale: one axis touches the margin exactly.
        let touches = (scene.x - (pane().width() - 2.0 * FIT_MARGIN)).abs() < 0.001
            || (scene.y - (pane().height() - 2.0 * FIT_MARGIN)).abs() < 0.001;
        assert!(touches, "fit scale does not fill either axis: {scene:?}");
    }

    #[test]
    fn zooming_at_a_known_point_keeps_that_point_stationary() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        let view = PlotView::IDENTITY;
        // Two anchors: the exact centre and a point well off-centre. A 2x step makes the scene
        // larger than the pane, so the clamp does not bind and the anchor must hold exactly.
        for pointer in [pane.center(), pane.center() + vec2(180.0, -120.0)] {
            let before = view.screen_to_scene(pointer, pane, fit);
            let zoomed = view.zoom_at(pointer, 2.0, pane, svg, fit);
            let after = zoomed.screen_to_scene(pointer, pane, fit);
            assert!(
                (after - before).length() < 1e-3,
                "the point under {pointer:?} moved from {before:?} to {after:?} \
                 (zoom {:.3} -> {:.3}, pan {:?})",
                view.zoom,
                zoomed.zoom,
                zoomed.pan
            );
            // And the same scene point maps back to the pointer, the stronger direction.
            let screen = zoomed.scene_to_screen(before, pane, fit);
            assert!(
                (screen - pointer).length() < 1e-3,
                "scene point {before:?} landed at {screen:?}, not under {pointer:?}"
            );
        }
    }

    #[test]
    fn successive_pointer_zoom_steps_keep_the_anchor_through_the_clamp() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        let pointer = pane.center() + vec2(-160.0, 90.0);
        let scene = PlotView::IDENTITY.screen_to_scene(pointer, pane, fit);
        let mut view = PlotView::IDENTITY;
        for _ in 0..6 {
            view = view.zoom_at(pointer, WHEEL_STEP, pane, svg, fit);
        }
        let landed = view.screen_to_scene(pointer, pane, fit);
        assert!(
            (landed - scene).length() < 0.01,
            "six wheel steps moved the anchor by {:?}",
            landed - scene
        );
        assert!(view.zoom > 2.0, "six steps did not zoom in: {}", view.zoom);
    }

    #[test]
    fn pan_then_fit_returns_the_identity_view() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        let mut view = PlotView::IDENTITY;
        // Pan around (the scene is slightly larger than the pane's fit box after a zoom).
        view = view.zoom_at(pane.center(), 1.5, pane, svg, fit);
        for delta in [vec2(240.0, -180.0), vec2(-400.0, 260.0), vec2(90.0, 40.0)] {
            view = view.pan_by(delta, pane, svg, fit);
        }
        assert!(view.pan != Vec2::ZERO, "the pan steps did nothing");
        // Fit is the one-click reset the pane's button performs.
        let fitted = PlotView::IDENTITY;
        assert_eq!(fitted.zoom, 1.0);
        assert_eq!(fitted.pan, Vec2::ZERO);
        let scene_rect = fitted.scene_rect(pane, svg, fit);
        assert!(
            pane.shrink(FIT_MARGIN).contains_rect(scene_rect),
            "the fitted document is not fully inside the pane: {scene_rect:?} vs {pane:?}"
        );
    }

    #[test]
    fn pan_is_clamped_so_the_scene_never_leaves_the_pane() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        // At 3x the scene is much larger than the pane; every pan is clamped.
        let zoomed = PlotView {
            zoom: 3.0,
            pan: Vec2::ZERO,
        };
        for delta in [
            vec2(-10_000.0, 0.0),
            vec2(10_000.0, 0.0),
            vec2(0.0, -10_000.0),
            vec2(0.0, 10_000.0),
            vec2(-10_000.0, 10_000.0),
        ] {
            let view = zoomed.pan_by(delta, pane, svg, fit);
            let scene = view.scene_rect(pane, svg, fit);
            let visible = scene.intersect(pane);
            assert!(
                visible.width() > 0.0 && visible.height() > 0.0,
                "the scene left the pane after a {delta:?} drag: {scene:?} vs {pane:?}"
            );
            assert!(
                visible.width() >= PlotView::MIN_VISIBLE - 0.001,
                "only {} px of width remain visible after {delta:?}",
                visible.width()
            );
            assert!(
                visible.height() >= PlotView::MIN_VISIBLE - 0.001,
                "only {} px of height remain visible after {delta:?}",
                visible.height()
            );
        }
    }

    #[test]
    fn a_scene_that_fits_is_centred_and_cannot_be_panned_away() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        // 0.5x is well inside the pane; a drag must not move it.
        let view = PlotView {
            zoom: 0.5,
            pan: Vec2::ZERO,
        };
        let dragged = view.pan_by(vec2(900.0, -700.0), pane, svg, fit);
        assert_eq!(dragged.pan, Vec2::ZERO, "a fully visible scene was panned");
        let scene = dragged.scene_rect(pane, svg, fit);
        assert!(pane.contains_rect(scene), "the scene is not fully inside");
    }

    #[test]
    fn zoom_is_clamped_to_the_supported_range() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        let mut view = PlotView::IDENTITY;
        for _ in 0..100 {
            view = view.zoom_at(pane.center(), 2.0, pane, svg, fit);
        }
        assert_eq!(view.zoom, PlotView::MAX_ZOOM);
        for _ in 0..200 {
            view = view.zoom_at(pane.center(), 0.5, pane, svg, fit);
        }
        assert_eq!(view.zoom, PlotView::MIN_ZOOM);
        assert!(view.zoom > 0.0);
    }

    #[test]
    fn actual_size_is_one_svg_point_per_screen_point_and_fit_is_not() {
        let pane = pane();
        let svg = svg_size();
        let fit = fit();
        assert!(
            (fit - 1.0).abs() > 0.01,
            "this test needs a pane whose fit scale is not 1:1 ({fit})"
        );
        let actual = PlotView::IDENTITY.actual_size(pane, svg, fit);
        assert!(
            (actual.scale(fit) - 1.0).abs() < 1e-4,
            "100% is {} screen points per SVG point, expected 1.0",
            actual.scale(fit)
        );
        assert_eq!(actual.pan, Vec2::ZERO);
        assert!(
            (actual.percent(fit) - 100.0).abs() < 1e-2,
            "the percentage label for natural size is {}",
            actual.percent(fit)
        );
        // Fit is a different view on this pane: it scales the document up to the pane.
        assert!((PlotView::IDENTITY.percent(fit) - fit * 100.0).abs() < 1e-2);
        assert!(
            (PlotView::IDENTITY.percent(fit) - 100.0).abs() > 1.0,
            "the fit view and the 100% reset must not be the same view"
        );
    }

    /// End-to-end PLT-004: wheel input through the real `Plot::ui` keeps the point under the
    /// pointer, and the drag path moves the scene by the drag delta.
    #[test]
    fn the_pane_wheel_and_drag_handlers_use_the_anchored_transform() {
        let ctx = egui::Context::default();
        let pane = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 800.0));
        let mut plot = Plot::new();
        assert!(plot.set_svg(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="600"><rect width="800" height="600" fill="#ffffff"/><polyline points="10,10 700,500" fill="none" stroke="#000000" stroke-width="2"/></svg>"##
        ));
        let pointer = pane.center() + vec2(120.0, -80.0);

        let run = |events: Vec<egui::Event>, plot: &mut Plot| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 900.0))),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                plot.ui(ui, pane);
            });
            // A headless pass has no painter to upload the font and plot textures to; mark the
            // deltas handled so dropping the output does not panic (same as `ui::app` tests).
            output.textures_delta.clear();
        };

        // A first pass establishes the pane, the fit scale and the texture.
        run(vec![egui::Event::PointerMoved(pointer)], &mut plot);
        let fit = plot.fit_scale;
        assert!(fit > 0.0);
        let before = plot.view.screen_to_scene(pointer, pane, fit);

        // One wheel gesture. egui smooths a wheel event over the following frames, so the zoom
        // keeps changing for a few passes; the pointer anchor must hold on every one of them,
        // and the loop settles before the drag so the drag's delta is not mixed with a residual
        // zoom.
        run(
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: vec2(0.0, 60.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: egui::Modifiers::default(),
            }],
            &mut plot,
        );
        let mut previous = f32::NAN;
        for settle in 0..40 {
            let anchor = plot.view.screen_to_scene(pointer, pane, fit);
            assert!(
                (anchor - before).length() < 0.01,
                "pass {settle} moved the point under the pointer from {before:?} to {anchor:?}"
            );
            if plot.view.zoom == previous {
                break;
            }
            previous = plot.view.zoom;
            run(Vec::new(), &mut plot);
        }
        assert!(
            plot.view.zoom > 1.05,
            "the wheel did not zoom in: {}",
            plot.view.zoom
        );

        // Drag: egui reports a drag delta once the pointer button is held and moved. Press at
        // the pointer, move by a fixed delta, and check the view panned by exactly that delta.
        // At this zoom the scene overflows the pane on both axes and the pointer is far from
        // the clamp boundary, so the clamp cannot change the result.
        let start = plot.view;
        run(
            vec![egui::Event::PointerButton {
                pos: pointer,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            }],
            &mut plot,
        );
        run(
            vec![egui::Event::PointerMoved(pointer + vec2(40.0, 30.0))],
            &mut plot,
        );
        let panned = plot.view;
        let expected = start.pan + vec2(40.0, 30.0);
        assert!(
            (panned.pan - expected).length() < 1e-2,
            "the drag panned from {:?} to {:?}, expected {expected:?}",
            start.pan,
            panned.pan
        );
    }

    // -----------------------------------------------------------------------------------------
    // PLT-006: export
    // -----------------------------------------------------------------------------------------

    #[test]
    fn png_export_writes_the_requested_dimensions_and_decodes() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect x="0" y="0" width="50" height="80" fill="#ff0000"/>
</svg>"##;
        let mut plot = Plot::new();
        assert!(plot.set_svg(svg));
        for (scale, expected) in [
            (1.0f32, [100u32, 80u32]),
            (2.0, [200, 160]),
            (3.5, [350, 280]),
        ] {
            let path = scratch(&format!("export-{scale}.png"));
            let _ = std::fs::remove_file(&path);
            let dimensions = plot
                .export_png_to(&path, scale, PngBackground::Transparent)
                .expect("png export");
            assert_eq!(dimensions, expected, "scale {scale}");
            let bytes = std::fs::read(&path).expect("png bytes");
            // The PNG signature and the IHDR dimensions, read without trusting our encoder.
            assert_eq!(
                &bytes[..8],
                b"\x89PNG\r\n\x1a\n".as_slice(),
                "PNG signature"
            );
            assert_eq!(&bytes[12..16], b"IHDR".as_slice(), "IHDR chunk");
            let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
            let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
            assert_eq!([width, height], expected, "IHDR dimensions");
            // And the file decodes back to the same dimensions.
            let decoded = tiny_skia::Pixmap::load_png(&path).expect("decoded PNG");
            assert_eq!([decoded.width(), decoded.height()], expected);
            let _ = std::fs::remove_file(&path);
        }
        // The advertised size is the written size.
        assert_eq!(export_pixels(100.0, 80.0, 3.5), Ok((350, 280)));
        assert!(export_pixels(100.0, 80.0, 0.0).is_err());
        assert!(export_pixels(100.0, 80.0, f32::NAN).is_err());
        assert!(export_pixels(20_000.0, 10.0, 1.0).is_err());
    }

    #[test]
    fn png_export_honours_the_transparent_and_opaque_background() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80" viewBox="0 0 100 80">
  <rect x="0" y="0" width="50" height="80" fill="#ff0000"/>
</svg>"##;
        let mut plot = Plot::new();
        assert!(plot.set_svg(svg));

        let transparent_path = scratch("export-transparent.png");
        plot.export_png_to(&transparent_path, 2.0, PngBackground::Transparent)
            .expect("transparent export");
        let transparent = tiny_skia::Pixmap::load_png(&transparent_path).expect("decode");
        let outside = transparent.pixel(150, 40).expect("pixel outside the rect");
        assert_eq!(
            outside.alpha(),
            0,
            "an untouched pixel is not transparent: {outside:?}"
        );
        let inside = transparent.pixel(50, 40).expect("pixel inside the rect");
        assert_eq!(
            (inside.red(), inside.green(), inside.blue(), inside.alpha()),
            (255, 0, 0, 255)
        );

        let opaque_path = scratch("export-opaque.png");
        plot.export_png_to(&opaque_path, 2.0, PngBackground::Opaque)
            .expect("opaque export");
        let opaque = tiny_skia::Pixmap::load_png(&opaque_path).expect("decode");
        let outside = opaque.pixel(150, 40).expect("pixel outside the rect");
        assert_eq!(
            (
                outside.red(),
                outside.green(),
                outside.blue(),
                outside.alpha()
            ),
            (255, 255, 255, 255),
            "the opaque background is not white"
        );
        assert_ne!(
            std::fs::read(&transparent_path).expect("transparent bytes"),
            std::fs::read(&opaque_path).expect("opaque bytes"),
            "the two background choices produced identical files on a transparent document"
        );
        let _ = std::fs::remove_file(&transparent_path);
        let _ = std::fs::remove_file(&opaque_path);
    }

    #[test]
    fn svg_export_copies_the_loaded_bytes_verbatim() {
        // A document with bytes a re-serialiser would change: CRLF, a non-ASCII title, an
        // untouched attribute order and a redundant self-closing space.
        let svg = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
                   <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"60\" viewBox=\"0 0 120 60\" >\r\n\
                   <title>Café &amp; co</title>\r\n\
                   <rect x=\"0\" y=\"0\" width=\"120\" height=\"60\" fill=\"#ffffff\" />\r\n\
                   </svg>\r\n";
        let mut plot = Plot::new();
        assert!(plot.set_svg(svg), "SVG rejected: {:?}", plot.error);
        let path = scratch("export-verbatim.svg");
        let _ = std::fs::remove_file(&path);
        let written = plot.export_svg_to(&path).expect("svg export");
        assert_eq!(written as usize, svg.len());
        assert_eq!(
            std::fs::read(&path).expect("exported bytes"),
            svg.as_bytes(),
            "the SVG export is not the loaded document byte for byte"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_file_names_are_safe_and_title_derived() {
        assert_eq!(export_file_name(Some("selftest"), "svg"), "selftest.svg");
        assert_eq!(export_file_name(None, "png"), "plot.png");
        assert_eq!(export_file_name(Some("   "), "svg"), "plot.svg");
        let hostile = export_file_name(Some("a/b: c*d"), "png");
        assert_eq!(hostile, "a-b- c-d.png");
        assert!(!hostile.contains('/') && !hostile.contains('\\'));
    }

    /// The strongest PLT-006 evidence: export a real engine `PlotCapture.svg` and compare the
    /// file with the capture string and with the file the engine itself wrote.
    #[cfg(feature = "real-engine")]
    #[test]
    fn svg_export_matches_the_captured_plot_capture_bytes() {
        use crate::engine::Evaluator;

        let directory = std::env::temp_dir().join("lovelace-desktop-plot-capture-test");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("engine plot directory");
        let mut engine = crate::engine::real::RealEngine::new();
        engine.set_plot_dir(&directory.to_string_lossy());
        engine
            .eval("plot([1, 4, 2], [4, 5, 6], \"export byte identity\")")
            .expect("the engine plot statement");
        let capture = engine.last_plot().expect("a plot capture");
        assert!(
            capture.svg.contains("<polyline"),
            "the capture has no series polyline"
        );

        let mut plot = Plot::new();
        assert!(plot.set_svg(&capture.svg));
        let path = directory.join("exported.svg");
        let written = plot.export_svg_to(&path).expect("svg export");
        let exported = std::fs::read(&path).expect("exported file");
        assert_eq!(
            exported.as_slice(),
            capture.svg.as_bytes(),
            "the exported SVG differs from PlotCapture.svg"
        );
        assert_eq!(written as usize, capture.svg.len());

        // The engine's own file is the same capture, which is what makes the on-disk file a
        // usable reference in the window proof.
        let engine_file = std::fs::read(&capture.path).expect("engine-written plot file");
        assert_eq!(
            engine_file.as_slice(),
            capture.svg.as_bytes(),
            "the engine's plot file differs from the capture it published"
        );

        // And a PNG of that same capture has the engine canvas's dimensions at 2x.
        let dimensions = plot
            .export_png_to(&directory.join("exported.png"), 2.0, PngBackground::Opaque)
            .expect("png export");
        assert_eq!(dimensions, [1600, 1200]);
        assert!(tiny_skia::Pixmap::load_png(directory.join("exported.png")).is_ok());
        let _ = std::fs::remove_dir_all(&directory);
    }
}
