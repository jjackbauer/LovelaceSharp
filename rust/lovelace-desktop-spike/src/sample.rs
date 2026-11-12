//! Deterministic fixtures: a 5,000-line Lovelace-style script and an engine-style SVG.

/// The generated fixture document. Exactly 5,000 lines, every line padded to 100
/// columns so the byte size is stable at roughly 500 KB.
pub const LINE_COUNT: usize = 5_000;
pub const COLUMNS: usize = 100;

pub fn script_5000() -> String {
    let templates: [&str; 14] = [
        "x{i} = symbol(\"x{i}\");  # independent variable for section {i:04}",
        "f{i} = diff(x{i}^3 + {i}*x{i}^2 - 2*x{i} + {i}.5, x{i});  # polynomial derivative {i}",
        "g{i} = simplify(f{i} / ({i} + 1));  # normalise the derivative before integration",
        "h{i} = integrate(g{i}, x{i}, 0..{i});  # definite integral of section {i:04}",
        "if abs(f{i}) > {i}.25 then print(\"large at {i}\") else print(\"small at {i}\");",
        "for k in 1..{i} do total{i} = total{i} + sqrt(k) * {i}.5; end;  # accumulate series",
        "m{i} = matrix([1, {i}, 3; 4, 5, {i}; 7, 8, 9]);  # determinant computed below",
        "d{i} = det(m{i}) + sin({i}.5) + cos({i}.25) + exp(-{i}.1);  # scalar invariant {i}",
        "plot(0..{i}, sin(0..{i}) * {i}.5, title=\"wave {i}\");  # one SVG per section",
        "let label{i} = \"section {i:04} sample string with spaces and digits {i}\";",
        "result{i} = round(d{i} * 1000) / 1000;  # keep three decimals for the report",
        "assert(result{i} >= -{i}.0 and result{i} <= {i}.0.5);  # invariant of section {i}",
        "trace{i} = \"symbolic trace for section {i:04}: diff -> simplify -> integrate -> eval\";",
        "set precision {i} digits;  # low precision keeps the fixture fast to evaluate",
    ];

    let mut out = String::with_capacity(LINE_COUNT * (COLUMNS + 1) + 64);
    for i in 0..LINE_COUNT {
        let base = templates[i % templates.len()].replace("{i:04}", &format!("{i:04}"));
        let mut line = base.replace("{i}", &i.to_string());
        let len = line.chars().count();
        if len < COLUMNS {
            line.extend(std::iter::repeat_n(' ', COLUMNS - len));
        }
        out.push_str(&line);
        if i + 1 < LINE_COUNT {
            out.push('\n');
        }
    }
    out
}

/// Second tab: a short analysis document, so the tab strip has two documents.
pub fn analysis_script() -> String {
    "x = symbol(\"x\")\nf = diff(x^3 + 2*x, x)\ng = simplify(f)\n\
     h = integrate(g, x, 0..10)\n\
     m = matrix([1, 2, 3; 4, 5, 6; 7, 8, 10])\n\
     d = det(m)\n\
     plot(0..10, sin(0..10), title=\"sin\")\n\
     print(\"determinant = \" + str(d))\n"
        .to_string()
}

/// Engine-style SVG that exercises every element the plot pane must survive.
pub fn svg_plot() -> String {
    let mut points = String::with_capacity(200 * 12);
    for i in 0..=199 {
        let t = i as f32 / 199.0;
        let x = 60.0 + 540.0 * t;
        let y = 190.0 - 120.0 * (t * 18.0).sin() * (-t * 2.0).exp();
        points.push_str(&format!("{x:.1},{y:.1} "));
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400" viewBox="0 0 640 400">
  <defs>
    <clipPath id="plot-area"><rect x="60" y="30" width="540" height="300"/></clipPath>
  </defs>
  <rect x="0" y="0" width="640" height="400" fill="#0e1216"/>
  <g transform="translate(0,0)" clip-path="url(#plot-area)">
    <rect x="60" y="30" width="540" height="300" fill="#11161b" stroke="#2a323c" stroke-width="1"/>
    <line x1="60" y1="180" x2="600" y2="180" stroke="#2a323c" stroke-width="1"/>
    <line x1="330" y1="30" x2="330" y2="330" stroke="#2a323c" stroke-width="1"/>
    <polyline points="{points}" fill="none" stroke="#4ec9b0" stroke-width="2"/>
    <path d="M60 190 C 150 60, 240 320, 330 190 S 510 60, 600 190" fill="none" stroke="#d19a66" stroke-width="2" stroke-dasharray="6 4"/>
    <text x="72" y="52" font-family="Segoe UI" font-size="14" fill="#d8dee9">f(x) = sin(18x) * e^(-2x)</text>
  </g>
  <g transform="translate(0,0)">
    <text x="60" y="360" font-family="Segoe UI" font-size="13" fill="#8b98a9">Lovelace engine SVG: rect / polyline / path / text / clipPath / g transform</text>
    <text x="60" y="382" font-family="Segoe UI" font-size="12" fill="#6f7c8d">axes origin (330, 180)   samples 200   clip 540x300</text>
  </g>
</svg>"##
    )
}

/// Deliberately malformed: an unclosed `rect` start tag. Parsing must produce an
/// `Err` that the pane renders inline, never a panic.
pub const SVG_MALFORMED: &str =
    r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="120"><rect width="20" height="20"</svg>"#;
