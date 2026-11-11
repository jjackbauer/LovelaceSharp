//! A small, dependency-free JSON reader and writer.
//!
//! The workspace builds with no external crates (the build host cannot reach a crate
//! registry), so the wire protocol's JSON is produced and consumed here. Numbers are kept
//! as their original text: every numeric field the protocol publishes is either small enough
//! to fit an integer or must be reproduced byte-for-byte, and keeping the literal avoids a
//! binary floating-point round trip.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// A JSON value, preserving object key order and numeric literals as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Json {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// A number, kept as its source text (e.g. `"1.5"`, `"-2"`).
    Num(String),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object; insertion ordered so published field order is stable.
    Obj(BTreeMap<String, Json>),
}

impl Json {
    /// Builds an object from (key, value) pairs, preserving the given order.
    pub fn obj<I, K>(pairs: I) -> Json
    where
        I: IntoIterator<Item = (K, Json)>,
        K: Into<String>,
    {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.into(), v);
        }
        Json::Obj(m)
    }

    /// Builds a string array.
    pub fn str_array<I, S>(items: I) -> Json
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Json::Arr(items.into_iter().map(|s| Json::Str(s.into())).collect())
    }

    /// Builds an array of numbers from integers.
    pub fn num_array<I>(items: I) -> Json
    where
        I: IntoIterator<Item = i64>,
    {
        Json::Arr(
            items
                .into_iter()
                .map(|n| Json::Num(n.to_string()))
                .collect(),
        )
    }

    /// Returns the value for a key when this is an object.
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }

    /// Returns the string payload, if this is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the elements, if this is an array.
    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    /// Serialises with two-space indentation (used by fixtures and human-facing output).
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write_pretty(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(true) => out.push_str("true"),
            Json::Bool(false) => out.push_str("false"),
            Json::Num(n) => out.push_str(n),
            Json::Str(s) => write_escaped(out, s),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(map) => {
                out.push('{');
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_escaped(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }

    fn write_pretty(&self, out: &mut String, indent: usize) {
        let pad = |out: &mut String, n: usize| {
            for _ in 0..n {
                out.push_str("  ");
            }
        };
        match self {
            Json::Arr(items) if !items.is_empty() => {
                out.push_str("[\n");
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(",\n");
                    }
                    pad(out, indent + 1);
                    v.write_pretty(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push(']');
            }
            Json::Obj(map) if !map.is_empty() => {
                out.push_str("{\n");
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push_str(",\n");
                    }
                    pad(out, indent + 1);
                    write_escaped(out, k);
                    out.push_str(": ");
                    v.write_pretty(out, indent + 1);
                }
                out.push('\n');
                pad(out, indent);
                out.push('}');
            }
            other => other.write(out),
        }
    }

    /// Parses a JSON document. Returns an error with a byte offset on malformed input.
    pub fn parse(input: &str) -> Result<Json, JsonError> {
        let mut p = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        p.skip_ws();
        let v = p.value()?;
        p.skip_ws();
        if p.pos != p.bytes.len() {
            return Err(JsonError {
                offset: p.pos,
                message: "trailing characters".into(),
            });
        }
        Ok(v)
    }
}

impl std::fmt::Display for Json {
    /// Serialises compactly. Objects render keys in map order.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.write(&mut out);
        f.write_str(&out)
    }
}

/// A parse failure with the byte offset at which it was detected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    /// Byte offset into the source.
    pub offset: usize,
    /// Human-readable description.
    pub message: String,
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for JsonError {}

fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            // Non-ASCII is escaped as \uXXXX to match the wire contract published by the C#
            // runner (System.Text.Json escapes by default); surrogate pairs are emitted for
            // astral characters so the output is valid JSON for any decoder.
            c if (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let _ = write!(out, "\\u{:04x}", unit);
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, msg: &str) -> Result<T, JsonError> {
        Err(JsonError {
            offset: self.pos,
            message: msg.to_string(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn value(&mut self) -> Result<Json, JsonError> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            _ => self.err("expected a value"),
        }
    }

    fn literal(&mut self, text: &str, value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(text.as_bytes()) {
            self.pos += text.len();
            Ok(value)
        } else {
            self.err("invalid literal")
        }
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() || b == b'.' || b == b'e' || b == b'E' || b == b'+' || b == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| JsonError {
            offset: start,
            message: "invalid utf-8 in number".into(),
        })?;
        Ok(Json::Num(text.to_string()))
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let b = match self.peek() {
                Some(b) => b,
                None => return self.err("unterminated string"),
            };
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = match self.peek() {
                        Some(e) => e,
                        None => return self.err("unterminated escape"),
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            if (0xD800..0xDC00).contains(&hi) {
                                if self.peek() == Some(b'\\') {
                                    self.pos += 1;
                                    if self.peek() == Some(b'u') {
                                        self.pos += 1;
                                        let lo = self.hex4()?;
                                        let c = 0x10000
                                            + ((hi as u32 - 0xD800) << 10)
                                            + (lo as u32 - 0xDC00);
                                        out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
                                    } else {
                                        out.push('\u{fffd}');
                                    }
                                } else {
                                    out.push('\u{fffd}');
                                }
                            } else {
                                out.push(char::from_u32(hi as u32).unwrap_or('\u{fffd}'));
                            }
                        }
                        _ => return self.err("invalid escape"),
                    }
                }
                _ => {
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    let end = (start + len).min(self.bytes.len());
                    match std::str::from_utf8(&self.bytes[start..end]) {
                        Ok(s) => out.push_str(s),
                        Err(_) => return self.err("invalid utf-8 in string"),
                    }
                    self.pos = end;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, JsonError> {
        let mut v: u16 = 0;
        for _ in 0..4 {
            let b = match self.peek() {
                Some(b) => b,
                None => return self.err("truncated \\u escape"),
            };
            self.pos += 1;
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return self.err("invalid hex digit"),
            };
            v = (v << 4) | d as u16;
        }
        Ok(v)
    }

    fn array(&mut self) -> Result<Json, JsonError> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return self.err("expected ',' or ']'"),
            }
        }
    }

    fn object(&mut self) -> Result<Json, JsonError> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(map));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return self.err("expected object key");
            }
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return self.err("expected ':'");
            }
            self.pos += 1;
            let v = self.value()?;
            map.insert(key, v);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(map));
                }
                _ => return self.err("expected ',' or '}'"),
            }
        }
    }
}

fn utf8_len(b: u8) -> usize {
    match b {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
// The test names below mirror the C# census method names one-for-one; parity/manifest-map.json
// keys on the exact lovelace-json::tests::Parse_... path, so renaming would break gate-manifest.
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn Parse_GivenFlatObject_PreservesFields() {
        let v = Json::parse(r#"{"a": 1, "b": "x"}"#).unwrap();
        assert_eq!(v.get("a"), Some(&Json::Num("1".into())));
        assert_eq!(v.get("b").and_then(|x| x.as_str()), Some("x"));
    }

    #[test]
    fn Parse_GivenArrayOfObjects_ReadsEach() {
        let v = Json::parse(r#"[{"id":"lang/001"},{"id":"lang/002"}]"#).unwrap();
        let arr = v.as_arr().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[1].get("id").and_then(|x| x.as_str()), Some("lang/002"));
    }

    #[test]
    fn Parse_GivenEscapes_DecodesUnicode() {
        let v = Json::parse(r#""café µ""#).unwrap();
        assert_eq!(v.as_str(), Some("café µ"));
    }

    #[test]
    fn Parse_GivenAstralSurrogatePair_DecodesOneChar() {
        let v = Json::parse(r#""😀""#).unwrap();
        assert_eq!(v.as_str().unwrap().chars().count(), 1);
    }

    #[test]
    fn Write_GivenNonAscii_EscapesAsUnicode() {
        let j = Json::Str("µ".into());
        assert_eq!(j.to_string(), r#""\u00b5""#);
    }

    #[test]
    fn RoundTrip_GivenCorpusShapedRecord_IsStable() {
        let src = r#"{"id":"lang/003","script":"1/3","kind":"value","text":"0.(3) (Real)"}"#;
        let v = Json::parse(src).unwrap();
        let again = Json::parse(&v.to_string()).unwrap();
        assert_eq!(v, again);
    }

    #[test]
    fn Parse_GivenTrailingGarbage_ReturnsError() {
        assert!(Json::parse("{} x").is_err());
    }
}
