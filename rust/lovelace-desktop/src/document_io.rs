//! The on-disk form of one document: UTF-8 validation, the byte-order mark and the newline style.
//!
//! DOC-002 asks for a file lifecycle that never rewrites bytes the user did not change. Two
//! decisions make that possible here:
//!
//! - the decoder returns the text **exactly as stored** (`\r` characters stay in the string), so
//!   `encode` reproduces the file body for every file that decoded successfully;
//! - the loaded bytes are kept beside the text, so a clean save writes the file's own bytes back
//!   even after the editor's line model (one separator per line) has had the text.
//!
//! Nothing in this module normalises text: it does not replace CRLF with LF, it does not drop a
//! BOM, and it does not re-encode a file that reads as valid UTF-8.

use std::path::Path;

/// The UTF-8 byte-order mark, as it appears at the start of a file.
pub const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// One line ending, the separator new text is written with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    /// A single line-feed byte (`\n`).
    Lf,
    /// A carriage-return/line-feed pair (`\r\n`).
    CrLf,
}

impl LineEnding {
    /// The separator as it appears in the file.
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }

    /// The name used in the window's encoding badge and in the log.
    pub fn label(self) -> &'static str {
        match self {
            LineEnding::Lf => "LF",
            LineEnding::CrLf => "CRLF",
        }
    }
}

/// The newline style observed in one file, recorded when the document is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineStyle {
    /// Every line ends with `\n`.
    Lf,
    /// Every line ends with `\r\n`.
    CrLf,
    /// The file uses both endings (or a lone `\r`). `first` is the ending of the first line, which
    /// is the separator new lines are written with; an unmodified save still writes the file's own
    /// bytes, so a mixed file is only rewritten from the moment the user edits it.
    Mixed {
        /// The ending new lines are written with.
        first: LineEnding,
    },
    /// No line ending occurs (an empty or single-line file).
    None,
}

impl NewlineStyle {
    /// Classifies the newlines in `text`.
    pub fn detect(text: &str) -> NewlineStyle {
        let bytes = text.as_bytes();
        let mut crlf = 0usize;
        let mut lf = 0usize;
        let mut cr = 0usize;
        let mut first = None;
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    crlf += 1;
                    first.get_or_insert(LineEnding::CrLf);
                    index += 2;
                    continue;
                }
                b'\r' => cr += 1,
                b'\n' => {
                    lf += 1;
                    first.get_or_insert(LineEnding::Lf);
                }
                _ => {}
            }
            index += 1;
        }
        match (crlf, lf, cr) {
            (0, 0, 0) => NewlineStyle::None,
            (_, 0, 0) => NewlineStyle::CrLf,
            (0, _, 0) => NewlineStyle::Lf,
            _ => NewlineStyle::Mixed {
                first: first.unwrap_or(LineEnding::Lf),
            },
        }
    }

    /// The separator new text is written with.
    pub fn ending(self) -> LineEnding {
        match self {
            NewlineStyle::Lf | NewlineStyle::None => LineEnding::Lf,
            NewlineStyle::CrLf => LineEnding::CrLf,
            NewlineStyle::Mixed { first } => first,
        }
    }

    /// The name used in the window's encoding badge and in the log.
    pub fn label(self) -> &'static str {
        match self {
            NewlineStyle::Lf => LineEnding::Lf.label(),
            NewlineStyle::CrLf => LineEnding::CrLf.label(),
            NewlineStyle::Mixed { .. } => "mixed newlines",
            NewlineStyle::None => "no newline",
        }
    }
}

/// Why a file could not be read as a Lovelace document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The file starts with a UTF-16 byte-order mark, the usual sign of a Windows editor that
    /// saved as "Unicode" instead of UTF-8.
    Utf16Bom {
        /// True for a big-endian byte-order mark.
        big_endian: bool,
    },
    /// The bytes are not a UTF-8 sequence.
    InvalidUtf8 {
        /// Byte offset of the first byte that does not belong to a valid sequence.
        offset: usize,
        /// That byte.
        byte: u8,
    },
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Utf16Bom { big_endian } => write!(
                formatter,
                "the file starts with a UTF-16 {} byte-order mark, not UTF-8; re-save it as UTF-8 \
                 (a Lovelace script is UTF-8 text)",
                if *big_endian {
                    "big-endian"
                } else {
                    "little-endian"
                }
            ),
            DecodeError::InvalidUtf8 { offset, byte } => write!(
                formatter,
                "the file is not UTF-8 text: byte 0x{byte:02X} at offset {offset} is not valid \
                 UTF-8; re-save it as UTF-8 (a Lovelace script is UTF-8 text)"
            ),
        }
    }
}

/// A file that decoded, with everything an unmodified save needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedFile {
    /// The text exactly as stored: no BOM, and every `\r` kept.
    pub text: String,
    /// True when the file began with a UTF-8 byte-order mark.
    pub bom: bool,
    /// The newline style the file used.
    pub newline: NewlineStyle,
    /// The file's own bytes, so a save of an unmodified buffer is byte-identical.
    pub bytes: Vec<u8>,
}

/// Decodes file bytes as a Lovelace document.
///
/// A UTF-8 byte-order mark is detected and removed from the text; a UTF-16 byte-order mark and any
/// invalid sequence are refused with a message naming the problem, so a non-UTF-8 file is never
/// written back through the decoder. No normalisation of any kind happens here.
pub fn decode(bytes: &[u8]) -> Result<LoadedFile, DecodeError> {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return Err(DecodeError::Utf16Bom { big_endian: false });
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return Err(DecodeError::Utf16Bom { big_endian: true });
    }
    let (bom, body) = match bytes.strip_prefix(&UTF8_BOM) {
        Some(body) => (true, body),
        None => (false, bytes),
    };
    let text = match std::str::from_utf8(body) {
        Ok(text) => text.to_string(),
        Err(error) => {
            // Offsets are reported against the file the user chose, so the skipped byte-order
            // mark is added back.
            let offset = error.valid_up_to() + (bytes.len() - body.len());
            return Err(DecodeError::InvalidUtf8 {
                offset,
                byte: bytes.get(offset).copied().unwrap_or(0),
            });
        }
    };
    let newline = NewlineStyle::detect(&text);
    Ok(LoadedFile {
        text,
        bom,
        newline,
        bytes: bytes.to_vec(),
    })
}

/// Encodes a document's text, prefixing the byte-order mark when the document had one.
///
/// The text is written exactly as it stands: a buffer loaded from a CRLF file still contains its
/// `\r` characters, so this function never rewrites line endings.
pub fn encode(text: &str, bom: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + UTF8_BOM.len());
    if bom {
        bytes.extend_from_slice(&UTF8_BOM);
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// Reads a file and decodes it, with failures phrased for the log and the prompt.
pub fn read(path: &Path) -> Result<LoadedFile, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    decode(&bytes).map_err(|error| format!("Cannot open {}: {error}.", path.display()))
}

/// Writes `bytes` to `path` and returns the number of bytes written.
pub fn write(path: &Path, bytes: &[u8]) -> Result<usize, String> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)
        .map_err(|error| format!("Cannot save {}: {error}", path.display()))?;
    file.write_all(bytes)
        .map_err(|error| format!("Cannot save {}: {error}", path.display()))?;
    Ok(bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let directory = std::env::temp_dir().join("lovelace-desktop-file-io-tests");
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory.join(format!("{name}-{}.lv", std::process::id()))
    }

    #[test]
    fn a_utf8_bom_is_detected_and_removed_from_the_text() {
        let bytes = b"\xEF\xBB\xBFa = 1\n";
        let loaded = decode(bytes).expect("valid UTF-8");
        assert!(loaded.bom, "the byte-order mark must be recorded");
        assert_eq!(
            loaded.text, "a = 1\n",
            "the text must not start with U+FEFF"
        );
        assert_eq!(loaded.bytes, bytes, "the file's own bytes must be kept");
        assert_eq!(encode(&loaded.text, loaded.bom), bytes, "and written back");
    }

    #[test]
    fn a_text_without_a_bom_encodes_without_one() {
        let loaded = decode(b"a = 1\r\nb = 2\r\n").expect("valid UTF-8");
        assert!(!loaded.bom);
        assert_eq!(loaded.newline, NewlineStyle::CrLf);
        assert_eq!(encode(&loaded.text, loaded.bom), b"a = 1\r\nb = 2\r\n");
    }

    #[test]
    fn every_newline_style_is_classified() {
        assert_eq!(NewlineStyle::detect(""), NewlineStyle::None);
        assert_eq!(NewlineStyle::detect("a = 1"), NewlineStyle::None);
        assert_eq!(NewlineStyle::detect("a\nb\n"), NewlineStyle::Lf);
        assert_eq!(NewlineStyle::detect("a\r\nb\r\n"), NewlineStyle::CrLf);
        assert_eq!(
            NewlineStyle::detect("a\r\nb\n"),
            NewlineStyle::Mixed {
                first: LineEnding::CrLf
            },
            "the first ending is the one new lines are written with"
        );
        assert_eq!(
            NewlineStyle::detect("a\nb\r\n"),
            NewlineStyle::Mixed {
                first: LineEnding::Lf
            }
        );
        assert_eq!(
            NewlineStyle::detect("a\rb\n"),
            NewlineStyle::Mixed {
                first: LineEnding::Lf
            },
            "a lone carriage return is not a line ending, so the file is mixed"
        );
    }

    #[test]
    fn a_mixed_file_keeps_its_own_bytes_and_writes_new_lines_with_the_first_ending() {
        let original = b"one\r\ntwo\nthree\r\n";
        let loaded = decode(original).expect("valid UTF-8");
        assert_eq!(
            loaded.newline,
            NewlineStyle::Mixed {
                first: LineEnding::CrLf
            }
        );
        assert_eq!(loaded.newline.ending(), LineEnding::CrLf);
        // The decoded text keeps every `\r`, so even a mixed file re-encodes to its own bytes.
        assert_eq!(encode(&loaded.text, loaded.bom), original);
        // What no single separator can reproduce is the editor's line model (lines joined with
        // one separator), which is exactly why a clean save writes the loaded bytes instead.
        let editor_style = loaded
            .text
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .collect::<Vec<_>>()
            .join(loaded.newline.ending().as_str());
        assert_ne!(editor_style.as_bytes(), original);
        assert_eq!(loaded.bytes, original);
    }

    #[test]
    fn invalid_utf8_is_refused_and_the_message_names_the_byte_and_offset() {
        let error = decode(b"a = 1\nb = \xE9\n").expect_err("0xE9 alone is not UTF-8");
        assert_eq!(
            error,
            DecodeError::InvalidUtf8 {
                offset: 10,
                byte: 0xE9
            }
        );
        let message = error.to_string();
        for expected in ["not UTF-8", "0xE9", "offset 10", "re-save it as UTF-8"] {
            assert!(
                message.contains(expected),
                "the message must name {expected:?}: {message}"
            );
        }
    }

    #[test]
    fn a_utf16_bom_is_refused_with_its_own_message() {
        let error = decode(b"\xFF\xFEa\x00").expect_err("UTF-16 LE is refused");
        assert_eq!(error, DecodeError::Utf16Bom { big_endian: false });
        assert!(error.to_string().contains("UTF-16 little-endian"));
        let error = decode(b"\xFE\xFF\x00a").expect_err("UTF-16 BE is refused");
        assert_eq!(error, DecodeError::Utf16Bom { big_endian: true });
        assert!(error.to_string().contains("UTF-16 big-endian"));
    }

    #[test]
    fn a_truncated_multibyte_sequence_is_refused_at_its_first_byte() {
        let error = decode(b"ok\n\xC3").expect_err("a lone continuation start is invalid");
        assert_eq!(
            error,
            DecodeError::InvalidUtf8 {
                offset: 3,
                byte: 0xC3
            }
        );
    }

    #[test]
    fn reading_a_bom_file_from_disk_round_trips_its_bytes() {
        let path = scratch("bom");
        let bytes = b"\xEF\xBB\xBFplot([1], [2], \"bom\")\r\n";
        std::fs::write(&path, bytes).expect("write the fixture");
        let loaded = read(&path).expect("decode");
        assert!(loaded.bom);
        assert_eq!(loaded.newline, NewlineStyle::CrLf);
        assert_eq!(encode(&loaded.text, loaded.bom), bytes);
        assert_eq!(
            write(&path, &encode(&loaded.text, loaded.bom)),
            Ok(bytes.len())
        );
        assert_eq!(std::fs::read(&path).expect("read back"), bytes);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reading_a_non_utf8_file_reports_the_path_and_the_refusal() {
        let path = scratch("non-utf8");
        std::fs::write(&path, b"a = 1\nb = \xE9\n").expect("write the fixture");
        let message = read(&path).expect_err("refused");
        assert!(message.contains(&path.display().to_string()), "{message}");
        assert!(message.contains("not UTF-8"), "{message}");
        assert_eq!(
            std::fs::read(&path).expect("the file is untouched"),
            b"a = 1\nb = \xE9\n",
            "a refused file must never be rewritten"
        );
        let _ = std::fs::remove_file(&path);
    }
}
