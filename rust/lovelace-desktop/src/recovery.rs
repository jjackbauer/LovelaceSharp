//! DOC-005: the crash-recovery journal.
//!
//! Every document that has unsaved changes is journaled to one small file under the
//! application-scoped recovery directory, keyed by the document's stable identity:
//!
//! ```text
//! LOVELACE-RECOVERY 1
//! id: 7
//! revision: 3
//! title: notes.lv
//! path: C:\scripts\notes.lv
//! bom: 0
//! caret: 4:2
//! selection: 4:2-4:5
//! mtime_unix_nanos: 1763000000000000000
//! saved_unix_nanos: 1763000001000000000
//! bytes: 27
//!
//! a = 1
//! b = a + 2
//! ```
//!
//! The header is a fixed, one-key-per-line ASCII preamble; the body after the blank line is the
//! buffer's bytes **exactly** as the editor holds them, so a recovered buffer is byte-identical
//! to the buffer that was journaled. `bytes` states the body length and is validated on load, so
//! a truncated file is refused instead of recovered as a shorter document.
//!
//! Two properties matter more than the format:
//!
//! - **atomicity.** [`RecoveryRecord::write_atomic`] writes a sibling temporary file, flushes it,
//!   and renames it over the record. A process killed mid-write leaves either the whole previous
//!   record or the whole new one, never a half-written record.
//! - **bounded loss.** [`RecoveryJournal::sync`] writes a document's first dirty revision
//!   immediately (the leading edge), then at most once per debounce window (one second) while
//!   typing continues. A forced kill therefore loses at most one debounce window of typing, not
//!   the whole session.
//!
//! A record that cannot be parsed is skipped with a warning, never trusted and never a crash
//! (PERF-010). The warning names the file and the reason, and the launch continues normally.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::workbench::WorkbenchDocument;

/// First line of every record: the magic bytes and the record schema number.
pub const MAGIC: &str = "LOVELACE-RECOVERY";
/// The record schema version this build writes and reads.
pub const SCHEMA_VERSION: u32 = 1;
/// Extension of a recovery record file.
pub const RECORD_EXTENSION: &str = "lvrec";
/// Prefix of the temporary file an atomic write renames from (`doc-7.lvrec.tmp-<pid>-<n>`).
pub const TEMP_INFIX: &str = ".tmp-";
/// How long one journaled document may keep changing before its record is rewritten.
pub const DEFAULT_DEBOUNCE: Duration = Duration::from_secs(1);
/// The one key that must be present in a header: the body length.
const BYTES_KEY: &str = "bytes";

/// The buffer state one document had when its recovery record was written.
///
/// It carries everything a relaunch needs to put the text back without touching the file it
/// came from: the bytes, the path (when the document had one), the caret, the selection, and the
/// source file's modification time as it was when the document was opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryRecord {
    /// The document identity the record is keyed by.
    pub document_id: u64,
    /// The document's source revision when the record was written.
    pub revision: u64,
    /// The tab title to restore.
    pub title: String,
    /// The file the document had, when it had one.
    pub path: Option<PathBuf>,
    /// True when the document's file began with a UTF-8 byte-order mark.
    pub bom: bool,
    /// Caret row and column, both 0-based.
    pub caret: (usize, usize),
    /// Selection anchor `(row, col)` and caret end `(row, col)`, when one was active.
    pub selection: Option<((usize, usize), (usize, usize))>,
    /// The source file's modification time when the document was opened or last saved.
    pub source_mtime: Option<SystemTime>,
    /// When the record itself was written.
    pub saved_at: SystemTime,
    /// The buffer's text, exactly as the editor held it.
    pub source: String,
}

impl RecoveryRecord {
    /// Builds the record for one live document.
    pub fn from_document(document: &WorkbenchDocument, saved_at: SystemTime) -> RecoveryRecord {
        RecoveryRecord {
            document_id: document.id.0,
            revision: document.revision,
            title: document.title.clone(),
            path: document.path.clone(),
            bom: document.bom,
            caret: document.caret,
            selection: document.selection,
            source_mtime: document.source_mtime,
            saved_at,
            source: document.source.clone(),
        }
    }

    /// The number of buffer bytes the record carries.
    pub fn buffer_bytes(&self) -> usize {
        self.source.len()
    }

    /// The path as the recovery prompt shows it, or a marker for a buffer with no file.
    pub fn path_label(&self) -> String {
        match &self.path {
            Some(path) => path.display().to_string(),
            None => "no file yet".to_string(),
        }
    }

    /// Serialises the record: an ASCII header, one blank line, then the buffer bytes verbatim.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header = String::new();
        header.push_str(MAGIC);
        header.push(' ');
        header.push_str(&SCHEMA_VERSION.to_string());
        header.push('\n');
        header.push_str(&format!("id: {}\n", self.document_id));
        header.push_str(&format!("revision: {}\n", self.revision));
        header.push_str(&format!("title: {}\n", self.title));
        if let Some(path) = &self.path {
            header.push_str(&format!("path: {}\n", path.display()));
        }
        header.push_str(&format!("bom: {}\n", u8::from(self.bom)));
        header.push_str(&format!("caret: {}:{}\n", self.caret.0, self.caret.1));
        if let Some(((anchor_row, anchor_col), (caret_row, caret_col))) = self.selection {
            header.push_str(&format!(
                "selection: {anchor_row}:{anchor_col}-{caret_row}:{caret_col}\n"
            ));
        }
        if let Some(mtime) = self.source_mtime {
            header.push_str(&format!("mtime_unix_nanos: {}\n", unix_nanos(mtime)));
        }
        header.push_str(&format!(
            "saved_unix_nanos: {}\n",
            unix_nanos(self.saved_at)
        ));
        header.push_str(&format!("{BYTES_KEY}: {}\n", self.source.len()));
        header.push('\n');

        let mut bytes = header.into_bytes();
        bytes.extend_from_slice(self.source.as_bytes());
        bytes
    }

    /// Parses one record. Every problem is a message naming the field, never a panic.
    pub fn parse(bytes: &[u8]) -> Result<RecoveryRecord, String> {
        let separator = bytes
            .windows(2)
            .position(|window| window == b"\n\n")
            .ok_or_else(|| "the header is not terminated by a blank line".to_string())?;
        let header = std::str::from_utf8(&bytes[..separator])
            .map_err(|_| "the header is not UTF-8 text".to_string())?;
        let body = &bytes[separator + 2..];

        let mut lines = header.split('\n');
        let first = lines.next().unwrap_or_default().trim_end_matches('\r');
        let mut magic = first.split(' ');
        if magic.next() != Some(MAGIC) {
            return Err(format!("the file does not start with {MAGIC}"));
        }
        let version = magic
            .next()
            .and_then(|text| text.parse::<u32>().ok())
            .ok_or_else(|| "the record schema number is missing".to_string())?;
        if version > SCHEMA_VERSION {
            return Err(format!(
                "record schema {version} is newer than supported {SCHEMA_VERSION}"
            ));
        }

        let mut document_id: Option<u64> = None;
        let mut revision = 0u64;
        let mut title: Option<String> = None;
        let mut path: Option<PathBuf> = None;
        let mut bom = false;
        let mut caret = (0usize, 0usize);
        let mut selection = None;
        let mut source_mtime = None;
        let mut saved_at = UNIX_EPOCH;
        let mut declared_bytes: Option<usize> = None;

        for line in lines {
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                return Err(format!("header line {line:?} is not `key: value`"));
            };
            let value = value.strip_prefix(' ').unwrap_or(value);
            match key {
                "id" => {
                    document_id = Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| format!("id {value:?} is not a number"))?,
                    );
                }
                "revision" => {
                    revision = value.parse::<u64>().unwrap_or(0);
                }
                "title" => title = Some(value.to_string()),
                "path" => {
                    if !value.is_empty() {
                        path = Some(PathBuf::from(value));
                    }
                }
                "bom" => bom = matches!(value, "1" | "true"),
                "caret" => caret = parse_position(value)?,
                "selection" => {
                    let (anchor, end) = value
                        .split_once('-')
                        .ok_or_else(|| format!("selection {value:?} is not ROW:COL-ROW:COL"))?;
                    selection = Some((parse_position(anchor)?, parse_position(end)?));
                }
                "mtime_unix_nanos" => {
                    source_mtime = value.parse::<u64>().ok().map(system_time_from_nanos);
                }
                "saved_unix_nanos" => {
                    saved_at = value.parse::<u64>().map(system_time_from_nanos).unwrap_or(UNIX_EPOCH);
                }
                BYTES_KEY => {
                    declared_bytes = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("bytes {value:?} is not a number"))?,
                    );
                }
                // Unknown keys are ignored so a future field never makes an older build refuse a
                // record it could otherwise recover.
                _ => {}
            }
        }

        let document_id = document_id.ok_or_else(|| "the record has no id".to_string())?;
        let title = title.ok_or_else(|| "the record has no title".to_string())?;
        let declared_bytes =
            declared_bytes.ok_or_else(|| "the record has no byte count".to_string())?;
        if declared_bytes != body.len() {
            return Err(format!(
                "the record declares {declared_bytes} buffer bytes but contains {}",
                body.len()
            ));
        }
        let source = std::str::from_utf8(body)
            .map_err(|_| "the buffer bytes are not valid UTF-8".to_string())?
            .to_string();

        Ok(RecoveryRecord {
            document_id,
            revision,
            title,
            path,
            bom,
            caret,
            selection,
            source_mtime,
            saved_at,
            source,
        })
    }

    /// Writes the record atomically: a flushed temporary sibling, then a rename over the record.
    ///
    /// The rename is the only step that makes the record visible, so a kill between the write and
    /// the rename leaves the previous record intact and a kill after the rename leaves the new one.
    /// The temporary file shares the record's directory, so the rename never crosses a volume.
    pub fn write_atomic(&self, path: &Path) -> std::io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the recovery record has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent)?;
        let temp = temp_path(path);
        let bytes = self.to_bytes();
        let written = (|| -> std::io::Result<()> {
            let mut file = std::fs::File::create(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(error) = written {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        if let Err(error) = std::fs::rename(&temp, path) {
            let _ = std::fs::remove_file(&temp);
            return Err(error);
        }
        Ok(())
    }
}

/// Reads and parses one record file.
pub fn read_record(path: &Path) -> Result<RecoveryRecord, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    RecoveryRecord::parse(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

/// The record path for one document identity under `root`.
pub fn record_path(root: &Path, document_id: u64) -> PathBuf {
    root.join(format!("doc-{document_id}.{RECORD_EXTENSION}"))
}

/// The temporary sibling an atomic write renames from.
fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "record".to_string());
    path.with_file_name(format!(
        "{name}{TEMP_INFIX}{}-{}",
        std::process::id(),
        next_temp_sequence()
    ))
}

/// A process-wide counter so two writes in the same process cannot share a temporary name.
fn next_temp_sequence() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

fn unix_nanos(time: SystemTime) -> u64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos().min(u64::MAX as u128) as u64,
        Err(_) => 0,
    }
}

fn system_time_from_nanos(nanos: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_nanos(nanos)
}

fn parse_position(text: &str) -> Result<(usize, usize), String> {
    let (row, column) = text
        .split_once(':')
        .ok_or_else(|| format!("position {text:?} is not ROW:COL"))?;
    Ok((
        row.parse::<usize>()
            .map_err(|_| format!("position {text:?} has no row"))?,
        column
            .parse::<usize>()
            .map_err(|_| format!("position {text:?} has no column"))?,
    ))
}

/// Every record found by [`scan`], plus the problems that were skipped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecoveryScan {
    /// The usable records, ordered by document identity.
    pub records: Vec<RecoveryRecord>,
    /// One message per file that was skipped because it could not be read or parsed.
    pub warnings: Vec<String>,
}

/// Reads every recovery record under `root`.
///
/// A missing directory is an empty scan, not a problem: a machine that never crashed has no
/// recovery directory at all. A record that cannot be read or parsed is skipped with a warning
/// naming the file, so one corrupt file can never keep the application from starting (PERF-010).
/// Temporary files left by an interrupted atomic write are ignored: the record they would have
/// replaced is still there.
pub fn scan(root: &Path) -> RecoveryScan {
    let mut scan = RecoveryScan::default();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return scan,
        Err(error) => {
            scan.warnings
                .push(format!("cannot read {}: {error}", root.display()));
            return scan;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_record = path
            .extension()
            .map(|extension| extension == RECORD_EXTENSION)
            .unwrap_or(false);
        if !is_record {
            continue;
        }
        match read_record(&path) {
            Ok(record) => scan.records.push(record),
            Err(message) => scan
                .warnings
                .push(format!("skipped a corrupt recovery record: {message}")),
        }
    }
    scan.records.sort_by_key(|record| record.document_id);
    scan
}

/// Deletes one document's record file, if it exists.
pub fn discard_record(root: &Path, document_id: u64) -> std::io::Result<()> {
    match std::fs::remove_file(record_path(root, document_id)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// One document's journal write, as tracked by this process.
#[derive(Clone, Copy, Debug)]
struct Tracked {
    /// When the current record file was last written (or adopted).
    last_write: Instant,
    /// The source revision the record file holds.
    revision: u64,
}

/// What one [`RecoveryJournal::sync`] pass did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JournalReport {
    /// Document identities whose record was written this pass.
    pub written: Vec<u64>,
    /// Document identities whose obsolete record was deleted this pass.
    pub deleted: Vec<u64>,
    /// Human-readable failures; the window logs them and keeps running.
    pub failed: Vec<String>,
}

impl JournalReport {
    /// True when the pass changed nothing.
    #[cfg_attr(not(test), allow(dead_code))] // the journal tests assert on the empty report
    pub fn is_empty(&self) -> bool {
        self.written.is_empty() && self.deleted.is_empty() && self.failed.is_empty()
    }
}

/// The debounced writer that keeps one recovery record per dirty document.
///
/// The journal knows only which document identities it has a current record for and when it last
/// wrote them. [`RecoveryJournal::sync`] is called once per frame; it writes a document's first
/// dirty revision immediately and then at most one write per debounce window while edits keep
/// arriving, and it deletes a record as soon as the document is clean again (saved, discarded or
/// closed).
pub struct RecoveryJournal {
    root: Option<PathBuf>,
    debounce: Duration,
    tracked: HashMap<u64, Tracked>,
}

impl RecoveryJournal {
    /// A journal over `root` with the one-second debounce.
    pub fn new(root: PathBuf) -> RecoveryJournal {
        RecoveryJournal::with_debounce(root, DEFAULT_DEBOUNCE)
    }

    /// A journal over `root` with an explicit debounce window (tests use a longer one).
    pub fn with_debounce(root: PathBuf, debounce: Duration) -> RecoveryJournal {
        RecoveryJournal {
            root: Some(root),
            debounce,
            tracked: HashMap::new(),
        }
    }

    /// A journal that does nothing: used when no application directory could be resolved and by
    /// the window tests that must not touch the disk.
    pub fn disabled() -> RecoveryJournal {
        RecoveryJournal {
            root: None,
            debounce: DEFAULT_DEBOUNCE,
            tracked: HashMap::new(),
        }
    }

    /// The directory records are written under, when journaling is enabled.
    #[cfg_attr(not(test), allow(dead_code))] // the window reads the directory from its storage
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The debounce window.
    #[cfg_attr(not(test), allow(dead_code))] // the journal tests pin the one-second window
    pub fn debounce(&self) -> Duration {
        self.debounce
    }

    /// The record path for a document, when journaling is enabled.
    #[cfg_attr(not(test), allow(dead_code))] // the journal tests read the file back by path
    pub fn record_path(&self, document_id: u64) -> Option<PathBuf> {
        self.root
            .as_deref()
            .map(|root| record_path(root, document_id))
    }

    /// Declares that the record file for `document_id` already holds `revision`'s text.
    ///
    /// Used after a launch restore or a recovery: the file is exactly the text now in the tab, so
    /// the journal must not rewrite it on the next pass. The file is still deleted as soon as the
    /// document becomes clean or closes.
    pub fn track(&mut self, document_id: u64, revision: u64, now: Instant) {
        if self.root.is_none() {
            return;
        }
        self.tracked.insert(
            document_id,
            Tracked {
                last_write: now,
                revision,
            },
        );
    }

    /// Deletes one document's record and stops tracking it, returning a message on failure.
    pub fn forget(&mut self, document_id: u64) -> Option<String> {
        self.tracked.remove(&document_id);
        let root = self.root.as_deref()?;
        match discard_record(root, document_id) {
            Ok(()) => None,
            Err(error) => Some(format!(
                "cannot delete the recovery record for document {document_id}: {error}"
            )),
        }
    }

    /// Writes one document's record when the debounce allows it.
    ///
    /// Returns true when a file was written. The first dirty revision of a document is written
    /// immediately; later revisions wait for the debounce window to elapse since the last write.
    fn write_if_due(
        &mut self,
        record: &RecoveryRecord,
        now: Instant,
    ) -> Result<bool, String> {
        let Some(root) = self.root.clone() else {
            return Ok(false);
        };
        let document_id = record.document_id;
        if let Some(tracked) = self.tracked.get(&document_id) {
            if tracked.revision == record.revision {
                return Ok(false);
            }
            let elapsed = now
                .checked_duration_since(tracked.last_write)
                .unwrap_or_default();
            if elapsed < self.debounce {
                return Ok(false);
            }
        }
        let path = record_path(&root, document_id);
        record
            .write_atomic(&path)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        self.tracked.insert(
            document_id,
            Tracked {
                last_write: now,
                revision: record.revision,
            },
        );
        Ok(true)
    }

    /// Journals every dirty document and deletes the record of every document that is now clean.
    ///
    /// `now` is the frame's clock. The window passes [`Instant::now`]; tests pass synthetic
    /// instants, which is what makes the debounce window observable without sleeping.
    pub fn sync(&mut self, documents: &[WorkbenchDocument], now: Instant) -> JournalReport {
        self.sync_at(documents, now, SystemTime::now())
    }

    /// [`RecoveryJournal::sync`] with an explicit record timestamp (tests).
    pub fn sync_at(
        &mut self,
        documents: &[WorkbenchDocument],
        now: Instant,
        saved_at: SystemTime,
    ) -> JournalReport {
        let mut report = JournalReport::default();
        if self.root.is_none() {
            return report;
        }
        for document in documents {
            let document_id = document.id.0;
            if document.dirty() {
                let record = RecoveryRecord::from_document(document, saved_at);
                match self.write_if_due(&record, now) {
                    Ok(true) => report.written.push(document_id),
                    Ok(false) => {}
                    Err(message) => report.failed.push(message),
                }
            } else if self.tracked.contains_key(&document_id) {
                // The document is clean again: its journaled text is no longer unsaved work. Only
                // records this process owns are deleted; a record that is still waiting in the
                // recovery prompt belongs to no document yet and is never touched here.
                match self.forget(document_id) {
                    Some(message) => report.failed.push(message),
                    None => report.deleted.push(document_id),
                }
            }
        }
        report
    }

    /// The time until the next journal write is due, or `None` when every dirty document's newest
    /// revision is already on disk.
    ///
    /// DOC-005: the window asks for a frame after this delay. Without it the trailing write would
    /// wait for the next input event, and a process killed after the user stopped typing could lose
    /// more than one debounce window of work.
    pub fn pending_delay(
        &self,
        documents: &[WorkbenchDocument],
        now: Instant,
    ) -> Option<Duration> {
        self.root.as_ref()?;
        let mut soonest: Option<Duration> = None;
        for document in documents {
            if !document.dirty() {
                continue;
            }
            let remaining = match self.tracked.get(&document.id.0) {
                Some(tracked) if tracked.revision == document.revision => continue,
                Some(tracked) => self
                    .debounce
                    .saturating_sub(now.checked_duration_since(tracked.last_write).unwrap_or_default()),
                None => Duration::ZERO,
            };
            soonest = Some(match soonest {
                Some(current) => current.min(remaining),
                None => remaining,
            });
        }
        soonest
    }

    /// Deletes the records of every document still open at a clean exit.
    ///
    /// A clean exit only happens after every dirty document has been saved or discarded, so the
    /// records this session wrote describe nothing that is not already on disk or in the user's
    /// open tabs. Only tracked records are deleted: a record that was dismissed with "Later" is
    /// not part of this session's documents and must survive for the next launch.
    pub fn clean_exit(&mut self, documents: &[WorkbenchDocument]) -> JournalReport {
        let mut report = JournalReport::default();
        for document in documents {
            let document_id = document.id.0;
            if !self.tracked.contains_key(&document_id) {
                continue;
            }
            match self.forget(document_id) {
                Some(message) => report.failed.push(message),
                None => report.deleted.push(document_id),
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn scratch(name: &str) -> PathBuf {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "lovelace-recovery-{}-{}-{name}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).expect("scratch directory");
        directory
    }

    fn record() -> RecoveryRecord {
        RecoveryRecord {
            document_id: 7,
            revision: 3,
            title: "notes.lv".to_string(),
            path: Some(PathBuf::from(r"C:\scripts\notes.lv")),
            bom: true,
            caret: (4, 2),
            selection: Some(((4, 2), (4, 5))),
            source_mtime: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            saved_at: UNIX_EPOCH + Duration::from_secs(1_700_000_100),
            source: "a = 1\r\nb = \u{3bb} + 2\r\n".to_string(),
        }
    }

    #[test]
    fn a_record_round_trips_and_its_body_is_the_buffer_bytes() {
        let bytes = record().to_bytes();
        let parsed = RecoveryRecord::parse(&bytes).expect("the record parses");
        assert_eq!(parsed, record(), "every field survives the round trip");

        let separator = bytes
            .windows(2)
            .position(|window| window == b"\n\n")
            .expect("a blank line separates the header");
        assert_eq!(
            &bytes[separator + 2..],
            record().source.as_bytes(),
            "the body must be byte-identical to the buffer"
        );
        assert!(
            bytes.starts_with(b"LOVELACE-RECOVERY 1\n"),
            "the header starts with the magic and the schema"
        );
        assert!(
            String::from_utf8_lossy(&bytes[..separator]).contains("mtime_unix_nanos: 1700000000"),
            "the source's mtime at open is part of the record"
        );
    }

    #[test]
    fn a_record_without_a_path_or_selection_round_trips() {
        let mut record = record();
        record.path = None;
        record.selection = None;
        record.source_mtime = None;
        record.bom = false;
        let parsed = RecoveryRecord::parse(&record.to_bytes()).expect("parses");
        assert_eq!(parsed, record);
        assert_eq!(parsed.path_label(), "no file yet");
    }

    #[test]
    fn an_atomic_write_replaces_the_previous_record_and_leaves_no_temporary_file() {
        let directory = scratch("atomic");
        let path = record_path(&directory, 7);
        record().write_atomic(&path).expect("first write");
        let first = std::fs::read(&path).expect("read the first record");

        let mut updated = record();
        updated.source = "a = 1\r\nb = 3\r\n".to_string();
        updated.revision = 4;
        updated.write_atomic(&path).expect("second write");
        let second = std::fs::read(&path).expect("read the second record");
        assert_ne!(first, second);
        assert_eq!(
            RecoveryRecord::parse(&second).expect("parses").source,
            "a = 1\r\nb = 3\r\n"
        );
        let leftovers: Vec<String> = std::fs::read_dir(&directory)
            .expect("list")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(TEMP_INFIX))
            .collect();
        assert!(leftovers.is_empty(), "temporary files left: {leftovers:?}");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn scanning_skips_corrupt_and_truncated_records_with_a_warning() {
        let directory = scratch("scan-corrupt");
        record().write_atomic(&record_path(&directory, 7)).expect("valid");

        let full = record().to_bytes();
        let truncated = &full[..full.len() - 4];
        std::fs::write(record_path(&directory, 8), truncated).expect("truncated");
        std::fs::write(record_path(&directory, 9), b"not a recovery record at all")
            .expect("garbage");
        std::fs::write(record_path(&directory, 10), b"").expect("empty");
        // A temporary file from an interrupted write is ignored, not reported as corrupt.
        std::fs::write(
            directory.join(format!("doc-7.lvrec{TEMP_INFIX}999-0")),
            b"partial",
        )
        .expect("temp file");

        let scan = scan(&directory);
        assert_eq!(scan.records.len(), 1, "only the valid record survives");
        assert_eq!(scan.records[0].document_id, 7);
        assert_eq!(scan.warnings.len(), 3, "warnings: {:?}", scan.warnings);
        for warning in &scan.warnings {
            assert!(
                warning.contains("corrupt recovery record"),
                "a skipped record is explained: {warning}"
            );
        }
        assert!(
            scan.warnings.iter().any(|warning| warning.contains("doc-8.lvrec")),
            "the warning names the truncated file: {:?}",
            scan.warnings
        );
        assert!(scan.warnings.iter().any(|warning| warning.contains("declares")));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn scanning_a_missing_or_empty_directory_finds_nothing() {
        let directory = scratch("scan-empty");
        let missing = scan(&directory.join("does-not-exist"));
        assert_eq!(missing, RecoveryScan::default());
        let empty = scan(&directory);
        assert_eq!(empty, RecoveryScan::default());
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn discarding_a_record_deletes_only_that_file() {
        let directory = scratch("discard");
        record().write_atomic(&record_path(&directory, 7)).expect("keep");
        let mut second = record();
        second.document_id = 8;
        second.write_atomic(&record_path(&directory, 8)).expect("drop");

        discard_record(&directory, 8).expect("discard");
        assert!(!record_path(&directory, 8).exists());
        assert!(record_path(&directory, 7).exists());
        // Discarding a record that is already gone is not an error.
        discard_record(&directory, 8).expect("second discard is a no-op");
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_record_from_the_future_schema_is_refused_without_a_crash() {
        let mut bytes = record().to_bytes();
        let text = String::from_utf8(bytes.clone()).expect("header is ASCII");
        bytes = text
            .replacen("LOVELACE-RECOVERY 1", "LOVELACE-RECOVERY 99", 1)
            .into_bytes();
        let error = RecoveryRecord::parse(&bytes).expect_err("a newer schema is refused");
        assert!(error.contains("newer than supported"), "{error}");
    }

    #[test]
    fn a_journal_without_a_root_never_touches_the_disk() {
        let mut journal = RecoveryJournal::disabled();
        assert!(journal.root().is_none());
        assert!(journal.record_path(1).is_none());
        let report = journal.sync(&[], Instant::now());
        assert!(report.is_empty());
        assert!(journal.forget(1).is_none());
    }
}
