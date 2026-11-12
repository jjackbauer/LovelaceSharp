//! The kind of every line in the workbench logs pane, and its visible marker.
//!
//! LOG-002: a log line is no longer just `[info]`/`[error]` severity text. Every line records
//! what produced it, and the logs pane prefixes a readable marker for that kind:
//!
//! | kind | marker | produced by |
//! | --- | --- | --- |
//! | [`LogKind::Echo`] | `[echo]` | a quick-eval input (unwired: the window has no such input) |
//! | [`LogKind::Stdout`] | `[stdout]` | a `print(...)` line reaching the typed `Output` event |
//! | [`LogKind::Result`] | `[result]` | the value of a successful run's last expression statement |
//! | [`LogKind::Diagnostic`] | `[diagnostic]` | a failed run's structured diagnostic (and hard errors) |
//! | [`LogKind::Timing`] | `[timing]` | a terminal run line carrying elapsed milliseconds |
//! | [`LogKind::Progress`] | `[progress]` | a per-statement `StepStarted` event |
//! | [`LogKind::Info`] | `[info]` | readiness, queue, warnings and every other advisory line |
//!
//! The marker is part of the model, not a paint-time afterthought: [`LogLine::render`] is the
//! exact string the window draws, so a unit test can pin the kind-to-marker mapping without
//! starting a window.

/// What produced a log line.
///
/// There is deliberately no separate warning/error severity: a caller that used to log a warning
/// logs [`LogKind::Info`], and a caller that used to log an error logs [`LogKind::Diagnostic`].
/// That keeps the seven kinds mutually exclusive and every kind visually distinct.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogKind {
    /// A quick-eval input echoed before evaluation (no producer in the current window).
    Echo,
    /// Raw `print(...)` output from the running document.
    Stdout,
    /// The value of the last expression statement of a successful run.
    Result,
    /// A structured run diagnostic, or a hard application error.
    Diagnostic,
    /// A terminal run line that reports elapsed milliseconds.
    Timing,
    /// Per-statement run progress.
    Progress,
    /// Readiness, queueing, cancellation requests and other advisory messages.
    Info,
}

impl LogKind {
    /// Every kind, in the order the marker table documents them.
    ///
    /// This is the single list a test iterates to prove no kind lacks a marker.
    pub const ALL: [LogKind; 7] = [
        LogKind::Echo,
        LogKind::Stdout,
        LogKind::Result,
        LogKind::Diagnostic,
        LogKind::Timing,
        LogKind::Progress,
        LogKind::Info,
    ];

    /// The bare kind name, without brackets.
    pub fn label(self) -> &'static str {
        match self {
            LogKind::Echo => "echo",
            LogKind::Stdout => "stdout",
            LogKind::Result => "result",
            LogKind::Diagnostic => "diagnostic",
            LogKind::Timing => "timing",
            LogKind::Progress => "progress",
            LogKind::Info => "info",
        }
    }

    /// The visible text prefix the logs pane puts before the message.
    ///
    /// The markers are full words in brackets: they stay readable at the log pane's monospace
    /// size, they need no icon font, and a colour-blind reader can still tell the kinds apart
    /// from the text alone.
    pub fn marker(self) -> &'static str {
        match self {
            LogKind::Echo => "[echo]",
            LogKind::Stdout => "[stdout]",
            LogKind::Result => "[result]",
            LogKind::Diagnostic => "[diagnostic]",
            LogKind::Timing => "[timing]",
            LogKind::Progress => "[progress]",
            LogKind::Info => "[info]",
        }
    }
}

/// One line in a document's log pane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    /// What produced the line.
    pub kind: LogKind,
    /// Message text without a trailing newline.
    pub text: String,
    /// The 0-based statement this line belongs to, when one produced it.
    ///
    /// RUN-003: per-statement progress and `print` lines carry the index of the statement that
    /// produced them. An assembled [`crate::workbench::DocumentView`] uses the tag to prove that
    /// its log tail belongs to the same committed prefix as its variables and revision, instead
    /// of comparing display strings.
    pub statement: Option<usize>,
}

impl LogLine {
    /// Builds an unattributed log line (run-level, result, diagnostic or timing).
    pub fn new(kind: LogKind, text: impl Into<String>) -> LogLine {
        LogLine {
            kind,
            text: text.into(),
            statement: None,
        }
    }

    /// Attributes the line to the 0-based statement that produced it.
    pub fn at_statement(mut self, statement: usize) -> LogLine {
        self.statement = Some(statement);
        self
    }

    /// The exact text the window paints: `marker`, one space, then the message.
    pub fn render(&self) -> String {
        format!("{} {}", self.kind.marker(), self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_kind_has_a_distinct_readable_marker() {
        let mut seen = BTreeSet::new();
        for kind in LogKind::ALL {
            let marker = kind.marker();
            let label = kind.label();
            assert_eq!(
                marker,
                format!("[{label}]"),
                "the marker must be the bracketed kind name"
            );
            assert!(
                seen.insert(marker),
                "two kinds share the marker {marker:?}: {seen:?}"
            );
        }
        assert_eq!(
            seen.len(),
            LogKind::ALL.len(),
            "every kind must contribute one marker"
        );
    }

    /// The model renders the marker itself, so the window cannot accidentally paint a bare line
    /// and the test cannot drift from what is on screen.
    #[test]
    fn a_log_line_renders_its_kind_marker_before_the_text() {
        for kind in LogKind::ALL {
            let line = LogLine::new(kind, "message");
            assert_eq!(line.render(), format!("{} message", kind.marker()));
        }
        assert_eq!(
            LogLine::new(LogKind::Result, "= 42").render(),
            "[result] = 42"
        );
        assert_eq!(LogLine::new(LogKind::Stdout, "hi").render(), "[stdout] hi");
    }

    /// A known event sequence fed through the model keeps every kind and its order. This is the
    /// model-level half of LOG-002; `workbench::tests` drives the same kinds through real
    /// `ExecutionHost` events.
    #[test]
    fn a_known_sequence_keeps_every_kind_once_in_order() {
        let events = [
            (LogKind::Echo, "> 6 * 7"),
            (LogKind::Info, "Run queued"),
            (LogKind::Progress, "statement 1/2: print(\"hi\")"),
            (LogKind::Stdout, "hi"),
            (LogKind::Progress, "statement 2/2: 6 * 7"),
            (LogKind::Result, "= 42"),
            (
                LogKind::Timing,
                "Run finished: 2 statements committed in 1 ms.",
            ),
            (LogKind::Diagnostic, "Diagnostic: Cannot divide by zero."),
            (LogKind::Info, "plot directory"),
        ];
        let lines = events
            .iter()
            .map(|(kind, text)| LogLine::new(*kind, *text))
            .collect::<Vec<_>>();
        let kinds = lines.iter().map(|line| line.kind).collect::<Vec<_>>();
        assert_eq!(
            kinds,
            events.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
            "the log model must preserve the event order"
        );
        let unique = kinds.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), 7, "the sequence must carry all seven kinds");
        for line in &lines {
            assert!(
                line.render().starts_with(line.kind.marker()),
                "{:?} lost its marker",
                line
            );
        }
    }
}
