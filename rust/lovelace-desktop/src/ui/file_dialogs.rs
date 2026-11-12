//! The native file dialogs the workbench's file lifecycle opens (DOC-002).
//!
//! The window talks to [`FileDialogs`], never to the Win32 wrappers directly, so a headless egui
//! test can supply a scripted answer and exercise New / Open / Save As / Close without a real
//! dialog. The shipped implementation is [`NativeFileDialogs`], which calls the same comdlg32
//! wrappers the plot export uses ([`crate::win32::ffi::open_file_dialog_with`] and
//! [`crate::win32::ffi::save_file_dialog_with`]) with the workbench window as the owner.
//!
//! Dialogs are owned by the workbench window. The egui/eframe integration owns that window on
//! this thread, so the process's active window is the one that should parent the dialog; the
//! ownership test in `foreground_window_of_this_process` is what keeps an unrelated foreground
//! window from becoming the owner.

use std::path::{Path, PathBuf};

/// The file extension a Lovelace document is saved with.
pub const DOCUMENT_EXTENSION: &str = "lv";
/// The filter pairs the Open dialog offers, in order.
pub const OPEN_FILTERS: &[(&str, &str)] = &[
    ("Lovelace scripts (*.lv;*.ls)", "*.lv;*.ls;*.lvl"),
    ("All files (*.*)", "*.*"),
];
/// The filter pairs the Save As dialog offers, in order.
pub const SAVE_FILTERS: &[(&str, &str)] = &[
    ("Lovelace script (*.lv)", "*.lv"),
    ("All files (*.*)", "*.*"),
];

/// The two native dialogs the file lifecycle needs.
pub trait FileDialogs {
    /// Shows the Open dialog and returns the chosen path, or `None` when the user cancels.
    fn open(&mut self) -> Option<PathBuf>;

    /// Shows the Save As dialog for a document and returns the chosen path, or `None` when the
    /// user cancels. `suggested` is the file name the field starts with; `initial_dir` is the
    /// folder the dialog opens in (the document's own folder, when it has one).
    fn save_as(&mut self, suggested: &str, initial_dir: Option<&Path>) -> Option<PathBuf>;
}

/// The shipped dialogs: the Windows common file dialogs, owned by the workbench window.
#[derive(Default)]
pub struct NativeFileDialogs;

impl FileDialogs for NativeFileDialogs {
    fn open(&mut self) -> Option<PathBuf> {
        let owner = crate::win32::ffi::foreground_window_of_this_process();
        crate::win32::ffi::open_file_dialog_with(owner, "Open Lovelace script", OPEN_FILTERS, None)
    }

    fn save_as(&mut self, suggested: &str, initial_dir: Option<&Path>) -> Option<PathBuf> {
        let owner = crate::win32::ffi::foreground_window_of_this_process();
        crate::win32::ffi::save_file_dialog_with(
            owner,
            suggested,
            "Save Lovelace script",
            SAVE_FILTERS,
            DOCUMENT_EXTENSION,
            initial_dir,
        )
    }
}

/// The file name suggested when a document with no path is saved.
///
/// A document that already has a name keeps it; an untitled one is offered as its tab title, which
/// is already a `.lv` name. A path is never suggested as the file name.
pub fn suggested_name(title: &str) -> String {
    if title.contains('.') {
        title.to_string()
    } else {
        format!("{title}.{DOCUMENT_EXTENSION}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_document_name_is_suggested_as_a_lv_file() {
        assert_eq!(suggested_name("untitled-2.lv"), "untitled-2.lv");
        assert_eq!(suggested_name("notes"), "notes.lv");
        assert_eq!(suggested_name("sample.ls"), "sample.ls");
    }

    #[test]
    fn both_dialogs_offer_the_lv_extension() {
        assert!(
            OPEN_FILTERS
                .iter()
                .any(|(_, pattern)| pattern.contains("*.lv")),
            "Open must list .lv files: {OPEN_FILTERS:?}"
        );
        assert_eq!(
            SAVE_FILTERS[0],
            ("Lovelace script (*.lv)", "*.lv"),
            "the save dialog writes .lv by default"
        );
        assert_eq!(DOCUMENT_EXTENSION, "lv");
    }
}
