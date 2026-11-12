//! Round 87 — descriptor examples #97/#98: the plot SVG's published element structure.
//!
//! `docs/goal-parity-contract/round-86/descriptor-examples.md` §3.8-3.9 records the divergence
//! this module pins: the oracle wrote a 14,636/14,595-byte 800x600 chart for the two published
//! `plot` examples while the runner wrote a 326/331-byte 640x400 stub carrying one background
//! rect and one polyline. Byte identity is not the contract; the PUBLISHED STRUCTURE is, so each
//! test asserts the element kinds and counts, the canvas, the plot-area frame, both axes, the
//! tick geometry and labels, the title text, and the series stroke plus its sampled point count.

use crate::{Engine, Value};

/// The element kinds the reference chart publishes, counted per document.
#[derive(Debug, Default, PartialEq, Eq)]
struct Elements {
    svg: usize,
    rect: usize,
    line: usize,
    text: usize,
    polyline: usize,
}

/// Counts the element kinds of a rendered document.
fn elements(svg: &str) -> Elements {
    let mut counts = Elements::default();
    for line in svg.lines() {
        let tag = line.trim_start();
        if tag.starts_with("<svg") {
            counts.svg += 1;
        } else if tag.starts_with("<rect") {
            counts.rect += 1;
        } else if tag.starts_with("<line") {
            counts.line += 1;
        } else if tag.starts_with("<text") {
            counts.text += 1;
        } else if tag.starts_with("<polyline") {
            counts.polyline += 1;
        }
    }
    counts
}

fn plot_dir(tag: &str) -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/tmp/round87-plot-").to_string() + tag;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a fresh plot directory");
    dir
}

/// Evaluates `script` and returns the SVG the `plot()` builtin wrote.
fn render(tag: &str, script: &str) -> String {
    let dir = plot_dir(tag);
    let mut engine = Engine::new();
    engine.set_plot_dir(&dir);
    engine.set_plot_file("plot.svg");
    let value = engine
        .eval(script)
        .unwrap_or_else(|e| panic!("'{script}': {}", e.message));
    let Value::Text(path) = value else {
        panic!("plot() answers a Text path, got {}", value.kind_name());
    };
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read '{path}': {e}"))
}

/// The `points` attribute of the series polyline, split into its coordinate tokens.
fn series_points(svg: &str) -> Vec<&str> {
    let needle = "points=\"";
    let start = svg.find(needle).expect("a polyline carries points") + needle.len();
    let tail = &svg[start..];
    tail[..tail.find('"').expect("the attribute closes")]
        .split(' ')
        .collect()
}

/// Every `<text>` element's body, in document order (the title first when there is one).
fn labels(svg: &str) -> Vec<String> {
    svg.lines()
        .filter(|line| line.trim_start().starts_with("<text"))
        .map(|line| {
            line.split_once('>')
                .expect("a text element has a body")
                .1
                .trim_end_matches("</text>")
                .to_string()
        })
        .collect()
}

/// The canvas, background, frame and axes every reference chart publishes.
fn assert_canvas(svg: &str) {
    assert!(
        svg.starts_with(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"600\" viewBox=\"0 0 800 600\">\n"
        ),
        "the document is the reference's 800x600 canvas: {svg:.200}"
    );
    assert!(
        svg.contains("  <rect width=\"800\" height=\"600\" fill=\"white\"/>\n"),
        "a white background rect: {svg:.400}"
    );
    assert!(
        svg.contains(
            "  <rect x=\"70\" y=\"50\" width=\"700\" height=\"495\" fill=\"none\" stroke=\"#cccccc\"/>\n"
        ),
        "the plot-area frame: {svg:.600}"
    );
    assert!(
        svg.contains("  <line x1=\"70\" y1=\"50\" x2=\"70\" y2=\"545\" stroke=\"#333333\"/>\n"),
        "the y axis runs down the plot area's left edge: {svg:.800}"
    );
    assert!(
        svg.contains("  <line x1=\"70\" y1=\"545\" x2=\"770\" y2=\"545\" stroke=\"#333333\"/>\n"),
        "the x axis runs along the plot area's bottom edge: {svg:.800}"
    );
    assert!(svg.ends_with("</svg>\n"), "the document closes: {svg:.200}");
}

#[test]
fn descriptor_example_097_plot_single_vector_publishes_the_reference_chart() {
    // #97 `plot([1, 2, 3])`: y is the argument, x is the implicit index vector 1..3.
    let svg = render("097", "plot([1, 2, 3])");
    assert_canvas(&svg);
    assert_eq!(
        elements(&svg),
        Elements {
            svg: 1,
            rect: 2,
            line: 12,
            text: 10,
            polyline: 1,
        },
        "2 short of 12 lines would drop a tick pair: {svg:.400}"
    );
    assert!(
        !svg.contains("font-size=\"20\""),
        "an absent title emits no title text: {svg:.400}"
    );

    // x ticks: 1, 1.5, 2, 2.5, 3 at the reference's mapped positions (tick, then label).
    assert!(
        svg.contains(
            "  <line x1=\"101.81818\" y1=\"541\" x2=\"101.81818\" y2=\"549\" stroke=\"#333333\"/>\n"
        ),
        "{svg:.800}"
    );
    assert!(
        svg.contains(
            "  <line x1=\"738.18182\" y1=\"541\" x2=\"738.18182\" y2=\"549\" stroke=\"#333333\"/>\n"
        ),
        "{svg:.800}"
    );
    // y ticks carry the same five values for this y range; the labels interleave x then y.
    assert_eq!(
        labels(&svg),
        ["1", "1.5", "2", "2.5", "3", "1", "1.5", "2", "2.5", "3"]
    );

    // One densely sampled red spline polyline through the mapped data points.
    assert!(
        svg.contains("  <polyline fill=\"none\" stroke=\"#e6194b\" stroke-width=\"2\" points=\""),
        "the palette's first stroke: {svg:.400}"
    );
    let points = series_points(&svg);
    assert_eq!(points.len(), 639, "the reference samples 639 points");
    assert_eq!(points[0], "101.81818,522.5");
    assert_eq!(points[638], "738.18182,72.5");
    assert!(
        svg.len() > 10_000,
        "a chart, not the 326-byte stub: {} bytes",
        svg.len()
    );
}

#[test]
fn descriptor_example_098_plot_two_vectors_and_title_publishes_the_reference_chart() {
    // #98 `plot([1, 2, 3], [1, 4, 9], "title")`: the title is a 20px centred text above the frame.
    let svg = render("098", "plot([1, 2, 3], [1, 4, 9], \"title\")");
    assert_canvas(&svg);
    assert_eq!(
        elements(&svg),
        Elements {
            svg: 1,
            rect: 2,
            line: 11,
            text: 10,
            polyline: 1,
        },
        "the y range [1, 9] asks for four ticks, so one line fewer than #97: {svg:.400}"
    );
    assert!(
        svg.contains(
            "  <text x=\"400\" y=\"28\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"20\">title</text>\n"
        ),
        "the title text: {svg:.400}"
    );

    // y ticks: 2, 4, 6, 8 over the padded [1, 9] range; x ticks are the index vector again.
    assert!(
        svg.contains(
            "  <line x1=\"66\" y1=\"466.25\" x2=\"74\" y2=\"466.25\" stroke=\"#333333\"/>\n"
        ),
        "{svg:.800}"
    );
    assert_eq!(
        labels(&svg),
        ["title", "1", "1.5", "2", "2.5", "3", "2", "4", "6", "8"]
    );

    let points = series_points(&svg);
    assert_eq!(points.len(), 639, "the reference samples 639 points");
    assert_eq!(points[0], "101.81818,522.5");
    assert_eq!(points[638], "738.18182,72.5");
    assert!(
        svg.len() > 10_000,
        "a chart, not the 331-byte stub: {} bytes",
        svg.len()
    );
}
