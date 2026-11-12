//! The workbench's keyboard commands, shared by the window and the headless acceptance mode.
//!
//! The window reads the two shortcuts from egui's input state and maps them through
//! [`command_for_shortcut`]; `--gui-selftest` drives the same function and the same
//! [`crate::workbench::Workbench::command`] entry point, so the tested mapping is the shipped
//! mapping instead of a second copy.
//!
//! Ctrl+Enter (Command+Enter on macOS) runs the active document. Esc cancels the active run.
//! Both are global: they work whether the editor, the toolbar or another pane has focus.

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
}
