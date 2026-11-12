//! SVG plot pane: resvg + tiny-skia -> egui texture, wheel zoom around the pointer, drag pan,
//! Fit, an inline (non-panicking) error for malformed SVG, and an explicit empty state.
//!
//! Adapted from `rust/lovelace-desktop-spike/src/plot.rs`. The spike loaded a fixed sample
//! SVG; this version loads exactly the `PlotCapture.svg` the engine attached to the document's
//! last run. When no run has produced a plot, the pane shows an empty state instead of sample
//! data.
//!
//! PLT-002: the renderer is configured for self-contained documents. External `xlink:href`
//! strings are refused (only `data:` images resolve) and `<script>` is never executed by usvg;
//! `raster_cli` exposes the same rasterisation path to the round-29 fidelity harness.

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

/// The plot pane's state: current SVG tree, texture, zoom, pan and last raster timings.
pub struct Plot {
    options: usvg::Options<'static>,
    tree: Option<usvg::Tree>,
    /// Current SVG source; empty means "nothing to show yet".
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
    /// Zoom factor applied around the pane centre.
    pub zoom: f32,
    /// Pan offset in pixels.
    pub pan: Vec2,
    /// The rectangle last painted.
    pub canvas: Rect,
    /// The screen rectangle last occupied by the SVG image.
    pub last_screen: Rect,
    /// Pointer position on the last pass.
    pub last_pointer: Option<Pos2>,
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
            zoom: 1.0,
            pan: Vec2::ZERO,
            canvas: Rect::NOTHING,
            last_screen: Rect::NOTHING,
            last_pointer: None,
        }
    }

    /// True when no plot has ever been loaded, i.e. the pane should show its empty state.
    pub fn is_empty(&self) -> bool {
        self.svg_source.is_empty()
    }

    /// Returns the pane to its empty state.
    pub fn clear(&mut self) {
        self.tree = None;
        self.texture = None;
        self.error = None;
        self.svg_source.clear();
        self.raster_scale = 0.0;
        self.raster_px = [0, 0];
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
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
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
                self.parse_ms = started.elapsed().as_secs_f64() * 1000.0;
                true
            }
            Err(error) => {
                self.tree = None;
                self.texture = None;
                self.raster_scale = 0.0;
                self.svg_source = source.to_string();
                self.error = Some(format!("{error}"));
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
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    /// Re-rasterises when the current texture is too coarse for the requested scale.
    pub fn ensure(&mut self, ctx: &egui::Context, desired_scale: f32) {
        if self.tree.is_none() {
            return;
        }
        let desired = desired_scale.clamp(0.5, 4.0);
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
                zoom: self.zoom,
                pan: self.pan,
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
                zoom: self.zoom,
                pan: self.pan,
                pointer: None,
            };
        }

        let available = rect.shrink(8.0);
        let base = (available.width() / self.svg_w.max(1.0))
            .min(available.height() / self.svg_h.max(1.0))
            .max(0.01);

        let response = ui.interact(
            rect,
            egui::Id::unique("plot_canvas"),
            Sense::click_and_drag(),
        );
        if response.dragged() {
            self.pan += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll.abs() > 0.01 {
                let old = self.zoom;
                let new = (old * (scroll * 0.0025).exp()).clamp(0.25, 16.0);
                if let Some(pointer) = ui.input(|input| input.pointer.hover_pos()) {
                    let relative = (pointer - rect.center() - self.pan) / (base * old);
                    self.pan = pointer - rect.center() - relative * (base * new);
                }
                self.zoom = new;
            }
            let dragging = response.dragged();
            ui.ctx().set_cursor_icon(if dragging {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Grab
            });
        }
        let pointer = ui.input(|input| input.pointer.hover_pos());

        let size = vec2(self.svg_w, self.svg_h) * base * self.zoom;
        let screen = Rect::from_center_size(rect.center() + self.pan, size);
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
            "SVG plot canvas, {:.0} by {:.0} points, zoom {:.2}",
            self.svg_w, self.svg_h, self.zoom
        );
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, label.clone()));

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
        assert!(pane.set_svg(&hostile), "hostile SVG rejected: {:?}", pane.error);
        assert!(pane.error.is_none(), "hostile SVG left an error: {:?}", pane.error);
        pane.clear();
        assert!(pane.set_svg(control));
        let (recovered, _) = pane
            .raster_pixmap(1.0, MAX_SIDE)
            .expect("pane pixmap after the hostile document");
        assert_eq!(recovered.data(), expected.data());

        let _ = std::fs::remove_file(&marker_path);
    }
}
