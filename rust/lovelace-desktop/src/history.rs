//! Per-document undo/redo history for the native editor, shared with `--gui-selftest`.
//!
//! The window owns one `TextEditor` (in `ui::editor`, a Windows-only module) per document tab,
//! and each editor owns one [`History`]. A step is a complete [`Snapshot`] of the buffer, the
//! caret and the selection, so every editing primitive - typing, newline, paste, delete, line
//! join, a completion insertion - is undone and redone by exactly the same mechanism, and a
//! history can never reach another document.
//!
//! Typing coalescing: the first character of a run pushes one snapshot; following characters do
//! not while they arrive within [`TYPING_BURST_GAP`]. Any non-typing edit, a caret move, a
//! pointer click or a pause ends the burst and starts a new step. [`History::begin`] is asked
//! *before* the editor clones its buffer, so a coalesced keystroke performs no full-buffer copy
//! (PERF-009): one snapshot is taken per undo step, not per character.
//!
//! The key mapping lives here too, [`command_for_shortcut`], so the window and `--gui-selftest`
//! drive the same Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z rule instead of two copies.

use std::time::{Duration, Instant};

/// Maximum number of undo steps kept per document; the oldest is evicted first.
///
/// The redo stack is capped the same way. The cap bounds the memory one document's history can
/// use; a step is at most the buffer size, and typing bursts do not add a step per character.
pub const UNDO_CAP: usize = 128;

/// The pause after which the next typed character begins a new undo step.
///
/// A typing burst is a run of inserted characters with no other action and no pause longer than
/// this between two characters.
pub const TYPING_BURST_GAP: Duration = Duration::from_millis(600);

/// A caret position: 0-based buffer row and 0-based character column (never a byte offset).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caret {
    /// Buffer row, 0-based.
    pub row: usize,
    /// Character column in the row, 0-based.
    pub col: usize,
}

/// Everything one undo or redo step restores.
///
/// The snapshot carries the whole line vector rather than an edit delta, which keeps the
/// history honest for multi-line edits (a paste, a selected-range delete, a line join) without
/// a second representation of the buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// The buffer's lines, exactly as the editor holds them.
    pub lines: Vec<String>,
    /// Where the caret sat.
    pub caret: Caret,
    /// The fixed end of the selection; the caret is the moving end. `None` when nothing was
    /// selected. Restoring it restores the whole selection, not just the caret.
    pub selection: Option<Caret>,
}

/// How an edit affects the open typing burst.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// Inserted characters: adjacent insertions coalesce into one step.
    Typing,
    /// Any other edit: always its own step, and it ends an open typing burst.
    Atomic,
}

/// One command the editor's history accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryCommand {
    /// Restore the state before the last edit.
    Undo,
    /// Restore the state the last undo replaced.
    Redo,
}

impl HistoryCommand {
    /// The command's stable name, for logs and for the acceptance output.
    pub fn label(self) -> &'static str {
        match self {
            HistoryCommand::Undo => "Undo",
            HistoryCommand::Redo => "Redo",
        }
    }
}

/// Maps Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y to a history command.
///
/// `undo_key` is true when the key press is `Z`, `redo_key` when it is `Y`; `shift` and
/// `command_modifier` are the modifiers carried by that same key event. Without the command
/// modifier no command is produced: a bare `z` or `y` is ordinary typed text and belongs to the
/// editor's text input.
pub fn command_for_shortcut(
    undo_key: bool,
    redo_key: bool,
    shift: bool,
    command_modifier: bool,
) -> Option<HistoryCommand> {
    if !command_modifier {
        return None;
    }
    if redo_key {
        return Some(HistoryCommand::Redo);
    }
    if undo_key {
        return Some(if shift {
            HistoryCommand::Redo
        } else {
            HistoryCommand::Undo
        });
    }
    None
}

/// The undo and redo stacks of one document.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// True while the previous edit is a typing run that the next insertion may continue.
    typing_open: bool,
    /// When the last insertion of the open burst happened.
    last_typing: Option<Instant>,
}

impl History {
    /// Creates an empty history.
    pub fn new() -> History {
        History::default()
    }

    /// Asks to start one edit.
    ///
    /// Returns true when the caller must hand over the pre-edit snapshot to [`History::push`];
    /// false when this edit continues the open typing burst, whose one step already holds the
    /// state the whole burst started from. Either way the redo branch is discarded, because an
    /// edit after an undo makes the undone future unreachable.
    pub fn begin(&mut self, kind: EditKind, now: Instant) -> bool {
        let continues = kind == EditKind::Typing
            && self.typing_open
            && self
                .last_typing
                .is_some_and(|last| now.saturating_duration_since(last) <= TYPING_BURST_GAP);
        self.redo.clear();
        if continues {
            self.last_typing = Some(now);
            return false;
        }
        self.typing_open = kind == EditKind::Typing;
        self.last_typing = if self.typing_open { Some(now) } else { None };
        true
    }

    /// Pushes the pre-edit state that [`History::begin`] asked for, evicting the oldest step.
    pub fn push(&mut self, before: Snapshot) {
        self.undo.push(before);
        if self.undo.len() > UNDO_CAP {
            self.undo.remove(0);
        }
    }

    /// Ends the open typing burst, so the next typed character starts a new undo step.
    ///
    /// The editor calls this for every non-typing action (caret move, click, Escape, a command
    /// key, a history command) and for a pause.
    pub fn break_burst(&mut self) {
        self.typing_open = false;
        self.last_typing = None;
    }

    /// Pops the state before the last edit and records `current` as the redo state.
    ///
    /// Returns `None` when there is nothing to undo; the caller then leaves the buffer alone.
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        if self.redo.len() > UNDO_CAP {
            self.redo.remove(0);
        }
        self.break_burst();
        Some(previous)
    }

    /// Pops the last undone state and records `current` as the undo state.
    ///
    /// Returns `None` when there is nothing to redo.
    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        if self.undo.len() > UNDO_CAP {
            self.undo.remove(0);
        }
        self.break_burst();
        Some(next)
    }

    /// Forgets every step. Used when a document is loaded or replaced from outside the editor,
    /// where the old coordinates no longer describe the buffer.
    pub fn reset(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.break_burst();
    }

    /// Number of undo steps currently available.
    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }

    /// Number of redo steps currently available.
    pub fn redo_steps(&self) -> usize {
        self.redo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str, row: usize, col: usize) -> Snapshot {
        Snapshot {
            lines: text.split('\n').map(|line| line.to_string()).collect(),
            caret: Caret { row, col },
            selection: None,
        }
    }

    #[test]
    fn ctrl_z_undoes_ctrl_y_and_ctrl_shift_z_redo() {
        assert_eq!(
            command_for_shortcut(true, false, false, true),
            Some(HistoryCommand::Undo),
            "Ctrl+Z is undo"
        );
        assert_eq!(
            command_for_shortcut(false, true, false, true),
            Some(HistoryCommand::Redo),
            "Ctrl+Y is redo"
        );
        assert_eq!(
            command_for_shortcut(true, false, true, true),
            Some(HistoryCommand::Redo),
            "Ctrl+Shift+Z is redo"
        );
        assert_eq!(
            command_for_shortcut(false, true, true, true),
            Some(HistoryCommand::Redo),
            "the shift modifier does not change Ctrl+Y"
        );
        assert_eq!(
            command_for_shortcut(true, false, false, false),
            None,
            "a bare z is typed text, not a command"
        );
        assert_eq!(
            command_for_shortcut(false, true, false, false),
            None,
            "a bare y is typed text, not a command"
        );
        assert_eq!(
            command_for_shortcut(false, false, true, true),
            None,
            "a modifier press with no Z or Y is not a command"
        );
    }

    #[test]
    fn a_typing_burst_coalesces_until_a_pause_or_a_non_typing_edit() {
        let t0 = Instant::now();
        let mut history = History::new();

        // Three typed characters in one burst: one step, recorded before the first character.
        assert!(history.begin(EditKind::Typing, t0));
        history.push(snapshot("", 0, 0));
        assert!(!history.begin(EditKind::Typing, t0 + Duration::from_millis(50)));
        assert!(!history.begin(EditKind::Typing, t0 + Duration::from_millis(400)));
        assert_eq!(history.undo_steps(), 1, "one burst is one undo step");

        // A pause longer than the gap opens a new step.
        let after_pause =
            t0 + Duration::from_millis(400) + TYPING_BURST_GAP + Duration::from_millis(1);
        assert!(history.begin(EditKind::Typing, after_pause));
        history.push(snapshot("abc", 0, 3));
        assert_eq!(history.undo_steps(), 2, "a pause starts a new step");

        // A non-typing edit also starts its own step and closes the burst.
        assert!(history.begin(EditKind::Atomic, after_pause));
        history.push(snapshot("abcd", 0, 4));
        assert_eq!(history.undo_steps(), 3);
        assert!(
            history.begin(EditKind::Typing, after_pause + Duration::from_millis(1)),
            "typing after a non-typing edit is a new step"
        );
    }

    #[test]
    fn undo_and_redo_round_trip_and_a_new_edit_discards_the_redo_branch() {
        let t0 = Instant::now();
        let mut history = History::new();

        history.begin(EditKind::Typing, t0);
        history.push(snapshot("", 0, 0));
        let restored = history.undo(snapshot("ab", 0, 2)).expect("one undo step");
        assert_eq!(restored, snapshot("", 0, 0));
        assert_eq!(history.undo_steps(), 0);
        assert_eq!(history.redo_steps(), 1);

        let redone = history.redo(snapshot("", 0, 0)).expect("one redo step");
        assert_eq!(redone, snapshot("ab", 0, 2), "redo restores text and caret");
        assert_eq!(history.undo_steps(), 1);
        assert_eq!(history.redo_steps(), 0);

        // Undo, then edit: the redo branch is gone, not merely hidden.
        history.undo(snapshot("ab", 0, 2)).expect("one undo step");
        assert_eq!(history.redo_steps(), 1);
        history.begin(EditKind::Atomic, t0);
        assert_eq!(history.redo_steps(), 0, "an edit clears redo");
        assert!(history.redo(snapshot("", 0, 0)).is_none());
    }

    #[test]
    fn an_empty_history_refuses_to_undo_or_redo() {
        let mut history = History::new();
        assert!(history.undo(snapshot("", 0, 0)).is_none());
        assert!(history.redo(snapshot("", 0, 0)).is_none());
    }

    #[test]
    fn the_oldest_step_is_evicted_at_the_cap() {
        let t0 = Instant::now();
        let mut history = History::new();
        for index in 0..UNDO_CAP + 5 {
            history.begin(EditKind::Atomic, t0);
            history.push(snapshot("", index, 0));
        }
        assert_eq!(history.undo_steps(), UNDO_CAP);
        let mut oldest = None;
        for _ in 0..UNDO_CAP {
            oldest = history.undo(snapshot("", 0, 0));
        }
        assert_eq!(
            oldest.map(|snapshot| snapshot.caret.row),
            Some(5),
            "the five oldest snapshots were evicted, not the newest"
        );
        assert!(
            history.undo(snapshot("", 0, 0)).is_none(),
            "the cap is a real cap"
        );
    }
}
