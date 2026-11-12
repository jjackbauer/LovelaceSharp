//! A small Lovelace-flavoured syntax highlighter that produces an `egui::text::LayoutJob`.
//!
//! Colouring is per word for keywords/builtins, plus numbers, strings and `#` comments.
//! This is the layouter body used by the editor: it is only ever called for the rows
//! that are currently visible.

use egui::{Color32, FontId, TextFormat, text::LayoutJob};

#[derive(Clone, Copy)]
pub struct Palette {
    pub plain: Color32,
    pub number: Color32,
    pub string: Color32,
    pub keyword: Color32,
    pub builtin: Color32,
    pub comment: Color32,
}

pub const DARK: Palette = Palette {
    plain: Color32::from_rgb(0xd4, 0xd9, 0xe0),
    number: Color32::from_rgb(0xb5, 0xce, 0x8b),
    string: Color32::from_rgb(0xce, 0x91, 0x78),
    keyword: Color32::from_rgb(0xc5, 0x86, 0xc0),
    builtin: Color32::from_rgb(0x4f, 0xc1, 0xff),
    comment: Color32::from_rgb(0x6a, 0x99, 0x55),
};

pub const KEYWORDS: &[&str] = &[
    "let", "if", "else", "for", "while", "end", "do", "then", "return", "function", "set",
    "print", "assert", "in", "and", "or", "not", "solve", "plot", "elseif",
];

pub const BUILTINS: &[&str] = &[
    "diff", "integrate", "symbol", "simplify", "expand", "factor", "sqrt", "sin", "cos", "tan",
    "exp", "log", "abs", "round", "sum", "product", "matrix", "det", "eval", "latex", "trace",
    "subs", "limit", "series",
];

fn push(job: &mut LayoutJob, text: &str, font: &FontId, color: Color32) {
    if text.is_empty() {
        return;
    }
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: font.clone(),
            color,
            ..Default::default()
        },
    );
}

fn char_len_at(s: &str, i: usize) -> usize {
    s[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Build the coloured layout job for one source line.
pub fn job(line: &str, font: FontId, pal: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY; // a code editor row never wraps
    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'#' {
            push(&mut job, &line[i..], &font, pal.comment);
            break;
        } else if b == b'"' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                } else if bytes[i] == b'"' {
                    i += 1;
                    break;
                } else {
                    i += char_len_at(line, i);
                }
            }
            push(&mut job, &line[start..i.min(bytes.len())], &font, pal.string);
        } else if b.is_ascii_digit() {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            push(&mut job, &line[start..i], &font, pal.number);
        } else if is_ident_start(b) {
            let start = i;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let word = &line[start..i];
            let color = if KEYWORDS.contains(&word) {
                pal.keyword
            } else if BUILTINS.contains(&word) {
                pal.builtin
            } else {
                pal.plain
            };
            push(&mut job, word, &font, color);
        } else {
            let start = i;
            i += char_len_at(line, i);
            while i < bytes.len() {
                let c = bytes[i];
                if c == b'#'
                    || c == b'"'
                    || c.is_ascii_digit()
                    || is_ident_start(c)
                    || c >= 0x80
                {
                    break;
                }
                i += 1;
            }
            push(&mut job, &line[start..i], &font, pal.plain);
        }
    }
    job
}

/// Count of coloured sections, used as evidence that the highlighter really emitted
/// multiple spans for the fixture (and how expensive a whole-document job would be).
pub fn section_count(job: &LayoutJob) -> usize {
    job.sections.len()
}

/// Whole-document job. Only used by the `TextEdit` comparison probe in the benchmark,
/// because `TextEdit::layouter` is handed the entire buffer with no viewport argument.
pub fn job_multiline(text: &str, font: &FontId, pal: &Palette) -> LayoutJob {
    let mut out = LayoutJob::default();
    out.wrap.max_width = f32::INFINITY;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.append(
                "\n",
                0.0,
                TextFormat {
                    font_id: font.clone(),
                    color: pal.plain,
                    ..Default::default()
                },
            );
        }
        let base = out.text.len();
        let line_job = job(line, font.clone(), pal);
        out.text.push_str(line);
        for mut section in line_job.sections {
            section.byte_range.start += base;
            section.byte_range.end += base;
            out.sections.push(section);
        }
    }
    out
}
