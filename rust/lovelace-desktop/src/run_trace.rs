//! Opt-in monotonic timestamp trace for the RUN-001 run-start path.
//!
//! The window, the execution core and the workbench record a mark at each stage between the
//! Ctrl+Enter key event and the first painted frame that shows `run Running`:
//!
//! - `frame_begin` (the egui frame that carries the key event),
//! - `key_event_handled`, `command_run_enter`,
//! - `submit_enter`, `submit_exit`, `worker_start`, `engine_constructed`,
//! - `run_started_emitted` (typed event sent by the worker),
//! - `run_started_applied` (event consumed by the UI's poll),
//! - `frame_end_after_key` (the running state was drawn), and
//! - `frame_after_paint_begin` (the next frame starts, so the previous one was presented).
//!
//! It is off by default and costs one relaxed atomic load per mark when off. Set
//! `LOVELACE_RUN_TRACE=<file>` (or `stderr`) to write one line per mark; `--gui-selftest`
//! enables the in-memory mode and prints a relative breakdown.
//!
//! Each line carries monotonic microseconds since the first mark in this process, UTC
//! milliseconds, the thread name, the label and a detail string. The UTC timestamp lets the
//! real-window driver correlate a mark with its own wall clock without sharing a clock.

use std::io::Write;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// `0` = not decided yet, `1` = enabled, `2` = disabled.
static ENABLED: AtomicU8 = AtomicU8::new(0);
static ORIGIN: OnceLock<Instant> = OnceLock::new();
static MARKS: Mutex<Vec<Mark>> = Mutex::new(Vec::new());
static SINK: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

#[derive(Clone, Debug)]
struct Mark {
    mono_us: u128,
    wall_ms: u128,
    thread: String,
    label: &'static str,
    detail: String,
}

fn now() -> (u128, u128) {
    let origin = ORIGIN.get_or_init(Instant::now);
    let mono_us = origin.elapsed().as_micros();
    let wall_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    (mono_us, wall_ms)
}

/// True when a trace destination is configured (`LOVELACE_RUN_TRACE`).
///
/// The answer is computed once per process; changing the environment afterwards has no effect.
pub fn enabled() -> bool {
    match ENABLED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let on = std::env::var_os("LOVELACE_RUN_TRACE")
                .map(|value| !value.is_empty() && value != "0")
                .unwrap_or(false);
            ENABLED.store(if on { 1 } else { 2 }, Ordering::Relaxed);
            on
        }
    }
}

/// Enables the trace without a file sink, for the in-memory breakdown `--gui-selftest` prints.
pub fn enable_memory() {
    ENABLED.store(1, Ordering::Relaxed);
}

/// Disables the trace and drops the collected marks.
pub fn reset() {
    ENABLED.store(2, Ordering::Relaxed);
    MARKS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clear();
}

fn sink() -> &'static Mutex<Option<std::fs::File>> {
    SINK.get_or_init(|| {
        let file = match std::env::var_os("LOVELACE_RUN_TRACE") {
            None => None,
            Some(path) if path == "stderr" || path == "1" => None,
            Some(path) => std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(std::path::PathBuf::from(path))
                .ok(),
        };
        Mutex::new(file)
    })
}

/// Records one mark. Cheap no-op unless `LOVELACE_RUN_TRACE` is set or the selftest enabled it.
pub fn mark(label: &'static str, detail: impl Into<String>) {
    if !enabled() {
        return;
    }
    let (mono_us, wall_ms) = now();
    let detail = detail.into();
    let mark = Mark {
        mono_us,
        wall_ms,
        thread: std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string(),
        label,
        detail,
    };
    if let Ok(mut marks) = MARKS.lock() {
        if marks.len() < 4096 {
            marks.push(mark.clone());
        }
    }
    let mut guard = sink().lock().unwrap_or_else(|error| error.into_inner());
    if let Some(file) = guard.as_mut() {
        let _ = writeln!(
            file,
            "{} mono_us={} wall_ms={} thread={} {}",
            label, mark.mono_us, mark.wall_ms, mark.thread, mark.detail
        );
        let _ = file.flush();
    } else if std::env::var_os("LOVELACE_RUN_TRACE").as_deref()
        == Some(std::ffi::OsStr::new("stderr"))
    {
        eprintln!(
            "TRACE {label} mono_us={} wall_ms={} thread={} {}",
            mark.mono_us, mark.wall_ms, mark.thread, mark.detail
        );
    }
}

/// The collected marks, oldest first.
#[allow(dead_code)] // used when reading the trace file manually; the selftest uses the breakdown
pub fn marks() -> Vec<String> {
    MARKS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .map(|mark| {
            format!(
                "{} mono_us={} wall_ms={} thread={} {}",
                mark.label, mark.mono_us, mark.wall_ms, mark.thread, mark.detail
            )
        })
        .collect()
}

/// A one-line relative breakdown starting at `from`, for `--gui-selftest`.
///
/// Returns `None` when no mark with that label was recorded. Each stage is printed as
/// `label=+X.XXXms` relative to the first `from` mark; stages that have not happened yet are
/// omitted, so a partial trace still explains where the path currently is.
pub fn breakdown_from(from: &str) -> Option<String> {
    let marks = MARKS.lock().unwrap_or_else(|error| error.into_inner());
    let start = marks.iter().find(|mark| mark.label == from)?;
    let start_us = start.mono_us;
    let mut text = String::new();
    let mut seen: Vec<&'static str> = Vec::new();
    for mark in marks.iter().filter(|mark| mark.mono_us >= start_us) {
        if seen.contains(&mark.label) {
            continue;
        }
        seen.push(mark.label);
        if !text.is_empty() {
            text.push_str(" | ");
        }
        text.push_str(&format!(
            "{} +{:.3}ms",
            mark.label,
            (mark.mono_us - start_us) as f64 / 1000.0
        ));
        if !mark.detail.is_empty() && mark.label != "frame_begin" {
            text.push_str(&format!("({})", mark.detail.replace('|', "/")));
        }
    }
    Some(text)
}
