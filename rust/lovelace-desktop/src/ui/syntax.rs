//! A small Lovelace-flavoured syntax highlighter that produces an `egui::text::LayoutJob`.
//!
//! Copied from `rust/lovelace-desktop-spike/src/syntax.rs`: colouring is per word for
//! keywords/builtins, plus numbers, strings and `#` comments. The editor only calls the
//! layouter for rows that are currently visible.

use egui::{Color32, FontId, TextFormat, text::LayoutJob};

/// Colours for one syntax theme.
#[derive(Clone, Copy)]
pub struct Palette {
    /// Ordinary code and punctuation.
    pub plain: Color32,
    /// Numeric literals.
    pub number: Color32,
    /// String literals.
    pub string: Color32,
    /// Language keywords.
    pub keyword: Color32,
    /// Builtin function names.
    pub builtin: Color32,
    /// `#` comments.
    pub comment: Color32,
}

/// The dark editor palette.
pub const DARK: Palette = Palette {
    plain: Color32::from_rgb(0xd4, 0xd9, 0xe0),
    number: Color32::from_rgb(0xb5, 0xce, 0x8b),
    string: Color32::from_rgb(0xce, 0x91, 0x78),
    keyword: Color32::from_rgb(0xc5, 0x86, 0xc0),
    builtin: Color32::from_rgb(0x4f, 0xc1, 0xff),
    comment: Color32::from_rgb(0x6a, 0x99, 0x55),
};

/// Keywords coloured as language keywords.
pub const KEYWORDS: &[&str] = &[
    "let", "if", "else", "for", "while", "end", "do", "then", "return", "function", "set",
    "print", "assert", "in", "and", "or", "not", "solve", "plot", "elseif", "assume",
];

/// Builtin names coloured as builtins.
pub const BUILTINS: &[&str] = &[
    "diff", "integrate", "symbol", "simplify", "expand", "factor", "sqrt", "sin", "cos", "tan",
    "exp", "log", "abs", "round", "sum", "product", "matrix", "det", "eval", "latex", "trace",
    "subs", "limit", "series", "pi",
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

fn char_len_at(source: &str, index: usize) -> usize {
    source[index..]
        .chars()
        .next()
        .map(|character| character.len_utf8())
        .unwrap_or(1)
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Builds the coloured layout job for one source line.
pub fn job(line: &str, font: FontId, palette: &Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = f32::INFINITY; // a code editor row never wraps
    let bytes = line.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'#' {
            push(&mut job, &line[index..], &font, palette.comment);
            break;
        } else if byte == b'"' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' && index + 1 < bytes.len() {
                    index += 2;
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += char_len_at(line, index);
                }
            }
            push(
                &mut job,
                &line[start..index.min(bytes.len())],
                &font,
                palette.string,
            );
        } else if byte.is_ascii_digit() {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || bytes[index] == b'.' || bytes[index] == b'_')
            {
                index += 1;
            }
            push(&mut job, &line[start..index], &font, palette.number);
        } else if is_ident_start(byte) {
            let start = index;
            while index < bytes.len() && is_ident_continue(bytes[index]) {
                index += 1;
            }
            let word = &line[start..index];
            let color = if KEYWORDS.contains(&word) {
                palette.keyword
            } else if BUILTINS.contains(&word) {
                palette.builtin
            } else {
                palette.plain
            };
            push(&mut job, word, &font, color);
        } else {
            let start = index;
            index += char_len_at(line, index);
            while index < bytes.len() {
                let next = bytes[index];
                if next == b'#'
                    || next == b'"'
                    || next.is_ascii_digit()
                    || is_ident_start(next)
                    || next >= 0x80
                {
                    break;
                }
                index += 1;
            }
            push(&mut job, &line[start..index], &font, palette.plain);
        }
    }
    job
}
