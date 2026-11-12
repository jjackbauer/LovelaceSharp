//! The workbench's keyboard commands, shared by the window and the headless acceptance mode.
//!
//! The window reads the shortcuts from egui's input state and maps them through
//! [`command_for_shortcut`] and [`file_command_for_shortcut`]; `--gui-selftest` drives the same
//! functions and the same [`crate::workbench::Workbench`] entry points, so the tested mapping is
//! the shipped mapping instead of a second copy.
//!
//! Ctrl+Enter (Command+Enter on macOS) runs the active document. Esc cancels the active run.
//! Both are global: they work whether the editor, the toolbar or another pane has focus.
//!
//! DOC-002 adds the file commands: Ctrl+N (new), Ctrl+O (open), Ctrl+S (save), Ctrl+Shift+S
//! (save as) and Ctrl+W (close tab). They are window-level commands because each of them may
//! open a native dialog, so they live in their own enum rather than in
//! [`WorkbenchCommand`], which the headless workbench applies without a window.

/// One command a keyboard shortcut can request from the workbench.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkbenchCommand {
    /// Evaluate the active document.
    Run,
    /// Cancel the active document's in-flight run.
    Cancel,
}

impl WorkbenchCommand {
    /// The command's stable name, for logs and for the acceptance output.
    pub fn label(self) -> &'static str {
        match self {
            WorkbenchCommand::Run => "Run",
            WorkbenchCommand::Cancel => "Cancel",
        }
    }
}

/// One file-lifecycle command a keyboard shortcut can request from the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileCommand {
    /// Add a new, empty document tab (Ctrl+N).
    New,
    /// Open a file chosen in the native Open dialog (Ctrl+O).
    Open,
    /// Write the active document to its own path, or ask for one (Ctrl+S).
    Save,
    /// Ask for a path and write the active document there (Ctrl+Shift+S).
    SaveAs,
    /// Close the active tab, prompting when it has unsaved changes (Ctrl+W).
    CloseTab,
}

impl FileCommand {
    /// The command's stable name, for logs and for the acceptance output.
    pub fn label(self) -> &'static str {
        match self {
            FileCommand::New => "New",
            FileCommand::Open => "Open",
            FileCommand::Save => "Save",
            FileCommand::SaveAs => "Save As",
            FileCommand::CloseTab => "Close",
        }
    }
}

/// Maps the two shortcut keys to a workbench command.
///
/// `command` is the platform shortcut modifier: Ctrl on Windows and Linux, Command on macOS.
/// Ctrl+Enter wins over Esc when both key presses arrive in the same frame. A bare Enter is not
/// a command: it belongs to the editor and inserts a newline.
pub fn command_for_shortcut(
    enter_pressed: bool,
    escape_pressed: bool,
    command_modifier: bool,
) -> Option<WorkbenchCommand> {
    if enter_pressed && command_modifier {
        return Some(WorkbenchCommand::Run);
    }
    if escape_pressed {
        return Some(WorkbenchCommand::Cancel);
    }
    None
}

/// Maps a letter typed with the command modifier to a file command.
///
/// `letter` is the character of the pressed key, compared case-insensitively; `modifier` is Ctrl
/// on Windows and Linux (Command on macOS) and must be held for every file command. `shift`
/// distinguishes Ctrl+Shift+S (Save As) from Ctrl+S (Save). Alt combinations are never file
/// commands; the caller passes `false` for them by not calling this function.
///
/// The returned command is what the window executes, so this function is the single definition of
/// DOC-002's shortcut table.
pub fn file_command_for_shortcut(letter: char, modifier: bool, shift: bool) -> Option<FileCommand> {
    if !modifier {
        return None;
    }
    match letter.to_ascii_lowercase() {
        'n' if !shift => Some(FileCommand::New),
        'o' if !shift => Some(FileCommand::Open),
        's' if shift => Some(FileCommand::SaveAs),
        's' => Some(FileCommand::Save),
        'w' if !shift => Some(FileCommand::CloseTab),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctrl_enter_runs_and_escape_cancels() {
        assert_eq!(
            command_for_shortcut(true, false, true),
            Some(WorkbenchCommand::Run)
        );
        assert_eq!(
            command_for_shortcut(false, true, false),
            Some(WorkbenchCommand::Cancel)
        );
        assert_eq!(
            command_for_shortcut(false, true, true),
            Some(WorkbenchCommand::Cancel),
            "Escape does not require a modifier"
        );
    }

    #[test]
    fn a_bare_enter_and_unrelated_keys_issue_no_command() {
        assert_eq!(command_for_shortcut(true, false, false), None);
        assert_eq!(command_for_shortcut(false, false, true), None);
        assert_eq!(command_for_shortcut(false, false, false), None);
    }

    #[test]
    fn ctrl_enter_wins_when_both_shortcuts_arrive_together() {
        assert_eq!(
            command_for_shortcut(true, true, true),
            Some(WorkbenchCommand::Run)
        );
    }

    #[test]
    fn ctrl_n_o_s_w_and_ctrl_shift_s_are_the_file_shortcuts() {
        assert_eq!(
            file_command_for_shortcut('n', true, false),
            Some(FileCommand::New)
        );
        assert_eq!(
            file_command_for_shortcut('O', true, false),
            Some(FileCommand::Open),
            "the letter is matched case-insensitively"
        );
        assert_eq!(
            file_command_for_shortcut('s', true, false),
            Some(FileCommand::Save)
        );
        assert_eq!(
            file_command_for_shortcut('s', true, true),
            Some(FileCommand::SaveAs),
            "Ctrl+Shift+S is Save As, not Save"
        );
        assert_eq!(
            file_command_for_shortcut('w', true, false),
            Some(FileCommand::CloseTab)
        );
    }

    #[test]
    fn file_shortcuts_require_the_modifier_and_reject_shift_where_it_is_not_bound() {
        for letter in ['n', 'o', 's', 'w'] {
            assert_eq!(
                file_command_for_shortcut(letter, false, false),
                None,
                "{letter} without the modifier must not be a command"
            );
        }
        assert_eq!(file_command_for_shortcut('n', true, true), None);
        assert_eq!(file_command_for_shortcut('o', true, true), None);
        assert_eq!(file_command_for_shortcut('w', true, true), None);
        assert_eq!(file_command_for_shortcut('x', true, false), None);
        assert_eq!(file_command_for_shortcut('q', true, false), None);
    }

    #[test]
    fn every_file_command_has_a_label() {
        assert_eq!(FileCommand::New.label(), "New");
        assert_eq!(FileCommand::Open.label(), "Open");
        assert_eq!(FileCommand::Save.label(), "Save");
        assert_eq!(FileCommand::SaveAs.label(), "Save As");
        assert_eq!(FileCommand::CloseTab.label(), "Close");
    }
}
