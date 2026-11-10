//! A deliberately tiny SVG reader, plus the generator that produces the plot documents
//! it consumes.
//!
//! The engine publishes a plot as an SVG string produced by the workspace's own code, not by
//! a third-party renderer, and that string uses a small, fixed subset of SVG: a root svg element
//! with a size, an optional title element, one or more polyline elements with a points attribute
//! and a handful of text labels. The plot pane does not need a general renderer, so this module
//! parses exactly that subset and ignores everything else. Nothing here uses floating point:
//! coordinates are kept in thousandths of an SVG user unit (MILLI) so the GDI layer can scale
//! them with integer arithmetic.

/// Fixed-point subdivisions per SVG user unit.
pub const MILLI: i64 = 1000;

/// A point in thousandths of an SVG user unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i64,
    /// Vertical coordinate.
    pub y: i64,
}

/// A parsed polyline element.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Polyline {
    /// Vertices, in document order.
    pub points: Vec<Point>,
    /// The stroke attribute, if present.
    pub stroke: Option<String>,
}

/// Horizontal anchoring of a text label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAnchor {
    /// Left-aligned at x.
    #[default]
    Start,
    /// Centred on x.
    Middle,
    /// Right-aligned at x.
    End,
}

/// A parsed text element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextItem {
    /// Baseline origin, in thousandths of a user unit.
    pub x: i64,
    /// Baseline origin, in thousandths of a user unit.
    pub y: i64,
    /// The label's text content, with entity references decoded.
    pub text: String,
    /// Font size, in thousandths of a user unit.
    pub size: i64,
    /// Text anchor.
    pub anchor: TextAnchor,
    /// Whether font-weight asked for bold.
    pub bold: bool,
}

/// The subset of an SVG document that the plot pane understands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plot {
    /// Canvas width, in thousandths of a user unit.
    pub width: i64,
    /// Canvas height, in thousandths of a user unit.
    pub height: i64,
    /// The viewBox attribute, when present.
    pub view_box: Option<[i64; 4]>,
    /// Document title, from the title element.
    pub title: Option<String>,
    /// Polylines, in document order.
    pub polylines: Vec<Polyline>,
    /// Text labels, in document order.
    pub texts: Vec<TextItem>,
}

impl Default for Plot {
    fn default() -> Plot {
        Plot {
            width: 640 * MILLI,
            height: 400 * MILLI,
            view_box: None,
            title: None,
            polylines: Vec::new(),
            texts: Vec::new(),
        }
    }
}

impl Plot {
    /// Returns true when there is nothing to draw but the background.
    pub fn is_empty(&self) -> bool {
        self.polylines.is_empty() && self.texts.is_empty() && self.title.is_none()
    }
}

/// A tag found by next_tag.
struct Tag<'a> {
    name: &'a str,
    attrs: &'a str,
    close: bool,
    self_closing: bool,
    end: usize,
}

/// Parses an SVG document into the plot subset. Unknown elements are ignored, and malformed
/// input yields whatever was parsed before the problem rather than an error: the plot pane is
/// never allowed to make the application fail.
pub fn parse(svg: &str) -> Plot {
    let mut plot = Plot::default();
    let mut pos = 0usize;
    while let Some(tag) = next_tag(svg, pos) {
        pos = tag.end;
        match tag.name {
            "svg" if !tag.close => {
                let attrs = attributes(tag.attrs);
                let width = attr(&attrs, "width").and_then(parse_milli).filter(|v| *v > 0);
                let height = attr(&attrs, "height").and_then(parse_milli).filter(|v| *v > 0);
                let view_box = attr(&attrs, "viewBox").and_then(parse_view_box);
                match (width, height, view_box) {
                    (Some(w), Some(h), vb) => {
                        plot.width = w;
                        plot.height = h;
                        plot.view_box = vb;
                    }
                    (w, h, Some(vb)) => {
                        plot.width = w.unwrap_or(vb[2]);
                        plot.height = h.unwrap_or(vb[3]);
                        plot.view_box = Some(vb);
                    }
                    _ => {}
                }
            }
            "title" if !tag.close && !tag.self_closing => {
                if let Some((text, next)) = text_until(svg, pos, "</title>") {
                    pos = next;
                    let text = text.trim();
                    if !text.is_empty() {
                        plot.title = Some(text.to_string());
                    }
                }
            }
            "text" if !tag.close && !tag.self_closing => {
                let attrs = attributes(tag.attrs);
                let x = attr(&attrs, "x").and_then(parse_milli).unwrap_or(0);
                let y = attr(&attrs, "y").and_then(parse_milli).unwrap_or(0);
                let size = attr(&attrs, "font-size")
                    .and_then(parse_milli)
                    .filter(|v| *v > 0)
                    .unwrap_or(12 * MILLI);
                let anchor = match attr(&attrs, "text-anchor") {
                    Some("middle") => TextAnchor::Middle,
                    Some("end") => TextAnchor::End,
                    _ => TextAnchor::Start,
                };
                let bold = attr(&attrs, "font-weight")
                    .map(|w| w == "bold" || w == "600" || w == "700" || w == "800" || w == "900")
                    .unwrap_or(false);
                if let Some((text, next)) = text_until(svg, pos, "</text>") {
                    pos = next;
                    let text = text.trim();
                    if !text.is_empty() {
                        plot.texts.push(TextItem {
                            x,
                            y,
                            text: text.to_string(),
                            size,
                            anchor,
                            bold,
                        });
                    }
                }
            }
            "polyline" if !tag.close => {
                let attrs = attributes(tag.attrs);
                let points = attr(&attrs, "points").map(parse_points).unwrap_or_default();
                let stroke = attr(&attrs, "stroke").map(|s| s.to_string());
                plot.polylines.push(Polyline { points, stroke });
            }
            _ => {}
        }
    }
    plot
}

/// Finds the next element tag at or after from, skipping comments, processing instructions
/// and other declarations.
fn next_tag(src: &str, from: usize) -> Option<Tag<'_>> {
    let bytes = src.as_bytes();
    let mut search = from;
    loop {
        let mut i = search;
        while i < bytes.len() && bytes[i] != b'<' {
            i += 1;
        }
        if i + 1 >= bytes.len() {
            return None;
        }
        if src[i..].starts_with("<!--") {
            let end = src[i + 4..].find("-->")?;
            search = i + 4 + end + 3;
            continue;
        }
        if src[i..].starts_with("<?") || src[i..].starts_with("<!") {
            let end = src[i..].find('>')?;
            search = i + end + 1;
            continue;
        }
        let mut j = i + 1;
        let close = bytes[j] == b'/';
        if close {
            j += 1;
        }
        let name_start = j;
        while j < bytes.len() && !bytes[j].is_ascii_whitespace() && bytes[j] != b'>' && bytes[j] != b'/'
        {
            j += 1;
        }
        if j == name_start {
            search = i + 1;
            continue;
        }
        let name = &src[name_start..j];
        let attrs_start = j;
        let mut k = j;
        let mut quote: Option<u8> = None;
        while k < bytes.len() {
            let b = bytes[k];
            match quote {
                Some(q) => {
                    if b == q {
                        quote = None;
                    }
                }
                None => {
                    if b == b'"' || b == b'\'' {
                        quote = Some(b);
                    } else if b == b'>' {
                        break;
                    }
                }
            }
            k += 1;
        }
        if k >= bytes.len() {
            return None;
        }
        let mut attrs_end = k;
        let mut self_closing = false;
        if attrs_end > attrs_start && bytes[attrs_end - 1] == b'/' {
            self_closing = true;
            attrs_end -= 1;
        }
        return Some(Tag {
            name,
            attrs: &src[attrs_start..attrs_end],
            close,
            self_closing,
            end: k + 1,
        });
    }
}

/// Reads element content up to close_tag, strips nested markup and decodes entities.
fn text_until(src: &str, from: usize, close_tag: &str) -> Option<(String, usize)> {
    let idx = src[from..].find(close_tag)?;
    let raw = &src[from..from + idx];
    let text = decode_entities(&strip_tags(raw));
    Some((text, from + idx + close_tag.len()))
}

/// Removes nested tags from an element body, keeping their text.
fn strip_tags(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut depth = 0usize;
    for ch in src.chars() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            c if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// Decodes the five predefined XML entities and numeric character references.
fn decode_entities(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = src[i..].find(';') {
                let entity = &src[i + 1..i + semi];
                let decoded = match entity {
                    "amp" => Some('&'),
                    "lt" => Some('<'),
                    "gt" => Some('>'),
                    "quot" => Some('"'),
                    "apos" => Some('\''),
                    other => other
                        .strip_prefix('#')
                        .and_then(|num| {
                            if let Some(hex) =
                                num.strip_prefix('x').or_else(|| num.strip_prefix('X'))
                            {
                                u32::from_str_radix(hex, 16).ok()
                            } else {
                                num.parse::<u32>().ok()
                            }
                        })
                        .and_then(char::from_u32),
                };
                if let Some(ch) = decoded {
                    out.push(ch);
                    i += semi + 1;
                    continue;
                }
            }
        }
        let ch = src[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Splits an attribute list into (name, value) pairs. Values may be quoted with either quote
/// character or unquoted; malformed pairs are kept with an empty value.
fn attributes(attrs: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let bytes = attrs.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let name_start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'=' {
            i += 1;
        }
        let name = &attrs[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                let quote = bytes[i];
                i += 1;
                let value_start = i;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                let value = &attrs[value_start..i];
                if i < bytes.len() {
                    i += 1;
                }
                if !name.is_empty() {
                    out.push((name.to_string(), value.to_string()));
                }
            } else {
                let value_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if !name.is_empty() {
                    out.push((name.to_string(), attrs[value_start..i].to_string()));
                }
            }
        } else if !name.is_empty() {
            out.push((name.to_string(), String::new()));
        }
    }
    out
}

/// Looks up one attribute by exact name.
fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

/// Parses a points attribute: coordinate pairs separated by commas and/or whitespace.
fn parse_points(spec: &str) -> Vec<Point> {
    let mut numbers: Vec<i64> = Vec::new();
    for part in spec.split(|c: char| c == ',' || c.is_ascii_whitespace()) {
        if part.is_empty() {
            continue;
        }
        match parse_milli(part) {
            Some(v) => numbers.push(v),
            None => return Vec::new(),
        }
    }
    numbers
        .chunks_exact(2)
        .map(|pair| Point {
            x: pair[0],
            y: pair[1],
        })
        .collect()
}

/// Parses a decimal number into thousandths of a unit. Returns None for anything that is not a
/// plain decimal (exponents, units, empty text).
pub fn parse_milli(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut sign = 1i64;
    if bytes[0] == b'-' {
        sign = -1;
        i = 1;
    } else if bytes[0] == b'+' {
        i = 1;
    }
    let mut integer: i64 = 0;
    let mut fraction: i64 = 0;
    let mut fraction_digits = 0usize;
    let mut any = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        integer = integer.checked_mul(10)?.checked_add((bytes[i] - b'0') as i64)?;
        i += 1;
        any = true;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            if fraction_digits < 3 {
                fraction = fraction * 10 + (bytes[i] - b'0') as i64;
                fraction_digits += 1;
            }
            i += 1;
            any = true;
        }
    }
    if !any || i != bytes.len() {
        return None;
    }
    while fraction_digits < 3 {
        fraction *= 10;
        fraction_digits += 1;
    }
    Some(sign * (integer.saturating_mul(MILLI).saturating_add(fraction)))
}

/// Parses a viewBox attribute into fixed-point numbers.
fn parse_view_box(spec: &str) -> Option<[i64; 4]> {
    let numbers: Vec<i64> = spec
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .filter_map(parse_milli)
        .collect();
    if numbers.len() == 4 {
        Some([numbers[0], numbers[1], numbers[2], numbers[3]])
    } else {
        None
    }
}

/// Escapes text for inclusion in XML element content or an attribute value.
pub fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// Maps value from the range lo..=hi onto out_lo..=out_hi using integer arithmetic only.
fn map_range(value: i64, lo: i64, hi: i64, out_lo: i64, out_hi: i64) -> i64 {
    if hi == lo {
        return (out_lo + out_hi) / 2;
    }
    let numerator = (value as i128 - lo as i128) * (out_hi as i128 - out_lo as i128);
    let denominator = hi as i128 - lo as i128;
    (out_lo as i128 + numerator / denominator) as i64
}

/// Produces the plot document that this workspace's engine carries for a plot call: a white
/// canvas, an axis frame, one polyline through ys and a title label. This is the generator the
/// fake engine uses, and it is the writer that the reader in parse is tested against.
pub fn make_plot_svg(title: &str, ys: &[i64]) -> String {
    let width: i64 = 640;
    let height: i64 = 400;
    let margin: i64 = 48;
    let (lo, hi) = match (ys.iter().min(), ys.iter().max()) {
        (Some(lo), Some(hi)) => (*lo, *hi),
        _ => (0, 1),
    };
    let mut points = String::new();
    for (index, y) in ys.iter().enumerate() {
        let x = if ys.len() <= 1 {
            width / 2
        } else {
            margin + (index as i64) * (width - 2 * margin) / (ys.len() as i64 - 1)
        };
        let py = map_range(*y, lo, hi, height - margin, margin);
        if index > 0 {
            points.push(' ');
        }
        points.push_str(&format!("{x},{py}"));
    }

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">\n"
    ));
    svg.push_str(&format!("<title>{}</title>\n", escape_xml(title)));
    svg.push_str(&format!(
        "<rect x=\"0\" y=\"0\" width=\"{width}\" height=\"{height}\" fill=\"#ffffff\" stroke=\"#c8c8c8\"/>\n"
    ));
    svg.push_str(&format!(
        "<line x1=\"{margin}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#d0d0d0\"/>\n",
        height - margin,
        width - margin,
        height - margin
    ));
    svg.push_str(&format!(
        "<line x1=\"{margin}\" y1=\"{margin}\" x2=\"{margin}\" y2=\"{}\" stroke=\"#d0d0d0\"/>\n",
        height - margin
    ));
    if !points.is_empty() {
        svg.push_str(&format!(
            "<polyline fill=\"none\" stroke=\"#1f4e79\" stroke-width=\"2\" points=\"{points}\"/>\n"
        ));
    }
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"26\" font-size=\"16\" font-weight=\"bold\" text-anchor=\"middle\">{}</text>\n",
        width / 2,
        escape_xml(title)
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"11\">max {hi}</text>\n",
        margin + 4,
        margin - 6
    ));
    svg.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-size=\"11\">min {lo}</text>\n",
        margin + 4,
        height - margin + 16
    ));
    svg.push_str("</svg>\n");
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_milli_reads_decimals_without_floating_point() {
        assert_eq!(parse_milli("12"), Some(12_000));
        assert_eq!(parse_milli("12.5"), Some(12_500));
        assert_eq!(parse_milli("-3.125"), Some(-3_125));
        assert_eq!(parse_milli("0.1"), Some(100));
        assert_eq!(parse_milli(".5"), Some(500));
        assert_eq!(parse_milli("1e3"), None);
        assert_eq!(parse_milli("px"), None);
        assert_eq!(parse_milli(""), None);
    }

    #[test]
    fn generated_plot_round_trips_through_the_reader() {
        let ys = [3, 1, 4, 1, 5, 9, 2, 6];
        let svg = make_plot_svg("first 8 digits of pi", &ys);
        let plot = parse(&svg);
        assert_eq!(plot.title.as_deref(), Some("first 8 digits of pi"));
        assert_eq!(plot.width, 640 * MILLI);
        assert_eq!(plot.height, 400 * MILLI);
        assert_eq!(plot.view_box, Some([0, 0, 640 * MILLI, 400 * MILLI]));
        assert_eq!(plot.polylines.len(), 1);
        assert_eq!(plot.polylines[0].points.len(), ys.len());
        assert_eq!(plot.polylines[0].stroke.as_deref(), Some("#1f4e79"));
        assert!(plot.texts.iter().any(|t| t.text == "first 8 digits of pi"));
        assert!(plot.texts.iter().any(|t| t.text == "max 9"));
    }

    #[test]
    fn parse_reads_handwritten_tags_anchors_and_entities() {
        let svg = r##"<svg width="200" height="100">
            <!-- a comment with <fake> tags -->
            <title>Caf&#233; &amp; co</title>
            <polyline points="0,0 10.5,20 20,40" stroke="#ff0000"/>
            <text x="100" y="50" font-size="14" text-anchor="middle" font-weight="bold">A &lt; B</text>
        </svg>"##;
        let plot = parse(svg);
        assert_eq!(plot.width, 200 * MILLI);
        assert_eq!(plot.height, 100 * MILLI);
        assert_eq!(plot.title.as_deref(), Some("Café & co"));
        assert_eq!(plot.polylines[0].points[1], Point { x: 10_500, y: 20_000 });
        assert_eq!(plot.texts[0].anchor, TextAnchor::Middle);
        assert_eq!(plot.texts[0].size, 14_000);
        assert!(plot.texts[0].bold);
        assert_eq!(plot.texts[0].text, "A < B");
    }

    #[test]
    fn parse_uses_view_box_when_size_is_absent() {
        let plot = parse("<svg viewBox=\"0 0 320 240\"><polyline points=\"1,2 3,4\"/></svg>");
        assert_eq!(plot.width, 320 * MILLI);
        assert_eq!(plot.height, 240 * MILLI);
    }

    #[test]
    fn parse_never_panics_on_truncated_documents() {
        let svg = make_plot_svg("t", &[1, 2, 3]);
        for cut in [0usize, 1, 5, 17, 40, 120, svg.len()] {
            let prefix = svg.get(..cut).unwrap_or("");
            let _ = parse(prefix);
        }
    }

    #[test]
    fn parse_ignores_nested_tspan_markup() {
        let plot = parse("<svg><text x=\"1\" y=\"2\">a<tspan>b</tspan>c</text></svg>");
        assert_eq!(plot.texts[0].text, "abc");
    }

    #[test]
    fn escape_xml_round_trips_through_parse() {
        let svg = make_plot_svg("a < b & c > d", &[1, 2]);
        let plot = parse(&svg);
        assert_eq!(plot.title.as_deref(), Some("a < b & c > d"));
    }
}
