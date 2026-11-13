//! An ordered JSON value and a .NET-compatible writer.
//!
//! The persisted graph and the CLI response have to be comparable with the C# reference
//! byte for byte, so this writer reproduces the two formatting decisions
//! System.Text.Json makes for JsonSerializer with WriteIndented = true:
//!
//! * two-space indentation, a colon and one space between a key and its value, and the
//!   platform newline (CRLF on Windows, matching .NET's Environment.NewLine default);
//! * the default JavaScriptEncoder escaping: \u0027 for an apostrophe, \u003C and \u003E for
//!   angle brackets, \u0026 for an ampersand, \u002B for a plus sign, and \uXXXX for
//!   non-ASCII characters;
//! * shortest-round-trip doubles in .NET notation (fixed for decimal exponents in
//!   [-4, 15), otherwise d.dddE+XX).

/// A JSON value with insertion-ordered object members.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// JSON null.
    Null,
    /// true / false.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A floating-point number, written with [`format_double`].
    Float(f64),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Out>),
    /// An object whose member order is preserved.
    Obj(Vec<(String, Out)>),
}

impl Out {
    /// Builds a string value.
    #[must_use]
    pub fn str(value: impl Into<String>) -> Self {
        Out::Str(value.into())
    }

    /// Builds a value from an optional string: None becomes JSON null.
    #[must_use]
    pub fn opt_str(value: Option<&str>) -> Self {
        match value {
            Some(text) => Out::Str(text.to_string()),
            None => Out::Null,
        }
    }

    /// Builds an object from string keys, preserving the given order.
    #[must_use]
    pub fn obj<const N: usize>(pairs: [(&str, Out); N]) -> Self {
        Out::Obj(
            pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect(),
        )
    }

    /// Renders compactly on a single line.
    #[must_use]
    pub fn compact(&self) -> String {
        let mut out = String::new();
        self.write_compact(&mut out);
        out
    }

    /// Renders with the C# WriteIndented layout.
    #[must_use]
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    fn write_compact(&self, out: &mut String) {
        match self {
            Out::Null => out.push_str("null"),
            Out::Bool(true) => out.push_str("true"),
            Out::Bool(false) => out.push_str("false"),
            Out::Int(value) => out.push_str(&value.to_string()),
            Out::Float(value) => out.push_str(&format_double(*value)),
            Out::Str(text) => write_escaped(out, text),
            Out::Arr(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write_compact(out);
                }
                out.push(']');
            }
            Out::Obj(members) => {
                out.push('{');
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_escaped(out, key);
                    out.push(':');
                    value.write_compact(out);
                }
                out.push('}');
            }
        }
    }

    fn write_pretty(&self, out: &mut String, indent: usize) {
        match self {
            Out::Arr(items) if !items.is_empty() => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    newline(out, indent + 1);
                    item.write_pretty(out, indent + 1);
                }
                newline(out, indent);
                out.push(']');
            }
            Out::Obj(members) if !members.is_empty() => {
                out.push('{');
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    newline(out, indent + 1);
                    write_escaped(out, key);
                    out.push_str(": ");
                    value.write_pretty(out, indent + 1);
                }
                newline(out, indent);
                out.push('}');
            }
            other => other.write_compact(out),
        }
    }
}

fn newline(out: &mut String, indent: usize) {
    out.push_str(if cfg!(windows) { "\r\n" } else { "\n" });
    for _ in 0..indent {
        out.push_str("  ");
    }
}

fn write_escaped(out: &mut String, text: &str) {
    out.push('"');
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\'' => out.push_str("\\u0027"),
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            '&' => out.push_str("\\u0026"),
            '+' => out.push_str("\\u002B"),
            other if (other as u32) < 0x20 || (other as u32) > 0x7E => {
                let mut units = [0u16; 2];
                for unit in other.encode_utf16(&mut units) {
                    out.push_str(&format!("\\u{unit:04X}"));
                }
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

/// Formats a double the way System.Text.Json writes it.
#[must_use]
pub fn format_double(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_string();
    }
    if !value.is_finite() {
        // The protocol never carries a non-finite metric; C# would throw on one.
        return "0".to_string();
    }
    let negative = value < 0.0;
    let magnitude = value.abs();
    let scientific = format!("{magnitude:e}");
    let (mantissa, exponent_text) = scientific
        .split_once('e')
        .expect("LowerExp always renders an exponent");
    let exponent: i32 = exponent_text
        .parse()
        .expect("LowerExp exponent is an integer");
    let digits: String = mantissa
        .chars()
        .filter(|character| *character != '.')
        .collect();
    let body = if !(-4..15).contains(&exponent) {
        scientific_notation(&digits, exponent)
    } else {
        fixed_notation(&digits, exponent)
    };
    if negative { format!("-{body}") } else { body }
}

fn fixed_notation(digits: &str, exponent: i32) -> String {
    if exponent >= 0 {
        let integer_digits = (exponent + 1) as usize;
        if digits.len() <= integer_digits {
            let mut out = String::from(digits);
            out.push_str(&"0".repeat(integer_digits - digits.len()));
            out
        } else {
            format!(
                "{}.{}",
                &digits[..integer_digits],
                &digits[integer_digits..]
            )
        }
    } else {
        let zeros = (-exponent - 1) as usize;
        format!("0.{}{}", "0".repeat(zeros), digits)
    }
}

fn scientific_notation(digits: &str, exponent: i32) -> String {
    let mut out = String::new();
    out.push_str(&digits[..1]);
    if digits.len() > 1 {
        out.push('.');
        out.push_str(&digits[1..]);
    }
    out.push('E');
    out.push(if exponent < 0 { '-' } else { '+' });
    out.push_str(&format!("{:02}", exponent.abs()));
    out
}
