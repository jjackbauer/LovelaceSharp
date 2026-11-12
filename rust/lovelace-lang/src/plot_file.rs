//! The typed plot-file write refusal: the reference's Lovelace.Suite.PlotFileWriteException.
//!
//! The reference writes the rendered SVG with File.WriteAllText(full, svg) and lets a failure
//! escape as PlotFileWriteException, whose message names the path the caller asked for and
//! whose inner sentence is the CLR IOException text. The runner classifies that exception as
//! PlotFileError/TypeMismatch. This module is the Rust home of both halves: [write_plot_file]
//! performs the write and returns [PlotFileWriteException] on failure, and
//! [PlotFileWriteException::to_lang_error] is the one classification the engine and the runner
//! publish.
//!
//! The two caller-supplied path shapes the reference's PlotFileWriteRefusalTests pin are the
//! empty file name (the plot directory itself, which is an access denial) and a file under a
//! directory that does not exist (a missing path part). Both messages reproduce the CLR
//! sentence exactly, because the published envelope is the contract.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::LangError;

/// Why the write could not happen, in the reference's IOException vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlotFileWriteFailure {
    /// The target is a directory, or the process lacks write permission.
    AccessDenied,
    /// A directory in the target path does not exist.
    PathNotFound,
    /// Any other I/O failure; the OS sentence is carried as [PlotFileWriteException::detail].
    Other,
}

/// The reference's PlotFileWriteException: the requested path, the full path, and the
/// classified I/O failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotFileWriteException {
    requested: String,
    full_path: String,
    failure: PlotFileWriteFailure,
    detail: String,
}

impl PlotFileWriteException {
    /// The path the caller asked for: the plot directory joined with the plot file name.
    pub fn requested(&self) -> &str {
        &self.requested
    }

    /// The absolute path the write targeted.
    pub fn full_path(&self) -> &str {
        &self.full_path
    }

    /// What kind of I/O failure occurred.
    pub fn failure(&self) -> PlotFileWriteFailure {
        self.failure
    }

    /// The CLR inner sentence (or the raw OS sentence for [PlotFileWriteFailure::Other]).
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// The one classification the engine and runner publish for this exception.
    pub fn to_lang_error(&self) -> LangError {
        LangError::coded(self.to_string(), 0, "PlotFileError", "TypeMismatch")
    }
}

impl fmt::Display for PlotFileWriteException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot write the plot file '{}': {}",
            self.requested, self.detail
        )
    }
}

impl std::error::Error for PlotFileWriteException {}

/// Renders and writes one plot SVG, returning the absolute published path.
///
/// The directory is NOT created: the reference writes exactly the combined path, so a missing
/// parent is the refusal this surface reports. The message names the requested path before
/// absolutisation, exactly as the reference's exception does.
pub fn write_plot_file(
    directory: &str,
    file_name: &str,
    svg: &str,
) -> Result<String, PlotFileWriteException> {
    // Path.Combine(directory, "") is the directory itself, without a trailing separator; a plain
    // Path::join("") appends one, which would turn the access-denial case into a missing path.
    let requested_path = if file_name.is_empty() {
        PathBuf::from(directory)
    } else {
        Path::new(directory).join(file_name)
    };
    let requested = requested_path.to_string_lossy().into_owned();
    let full_path = std::path::absolute(&requested_path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| requested.clone());
    #[cfg(windows)]
    let full_path = full_path.replace('/', "\\");
    match std::fs::write(&full_path, svg) {
        Ok(()) => Ok(full_path),
        Err(error) => {
            let target_is_directory = Path::new(&full_path).is_dir();
            let failure = if target_is_directory {
                PlotFileWriteFailure::AccessDenied
            } else {
                match error.kind() {
                    std::io::ErrorKind::PermissionDenied => PlotFileWriteFailure::AccessDenied,
                    std::io::ErrorKind::NotFound => PlotFileWriteFailure::PathNotFound,
                    _ => PlotFileWriteFailure::Other,
                }
            };
            let detail = match failure {
                PlotFileWriteFailure::AccessDenied => {
                    format!("Access to the path '{}' is denied.", full_path)
                }
                PlotFileWriteFailure::PathNotFound => {
                    format!("Could not find a part of the path '{}'.", full_path)
                }
                PlotFileWriteFailure::Other => error.to_string(),
            };
            Err(PlotFileWriteException {
                requested,
                full_path,
                failure,
                detail,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe_directory(name: &str) -> String {
        let directory = format!("{}/../target/tmp/{}", env!("CARGO_MANIFEST_DIR"), name);
        let _ = std::fs::create_dir_all(&directory);
        directory
    }

    #[test]
    fn an_empty_file_name_is_an_access_denial_naming_the_directory() {
        let directory = probe_directory("round90-plot-file-empty");
        let failure =
            write_plot_file(&directory, "", "<svg/>").expect_err("a directory is refused");
        assert_eq!(failure.failure(), PlotFileWriteFailure::AccessDenied);
        assert_eq!(failure.requested(), directory);
        assert!(
            failure.detail().starts_with("Access to the path '"),
            "{}",
            failure.detail()
        );
        assert!(
            failure.detail().ends_with("' is denied."),
            "{}",
            failure.detail()
        );
    }

    #[test]
    fn a_missing_parent_directory_is_a_path_not_found_refusal() {
        let directory = probe_directory("round90-plot-file-missing");
        let requested = Path::new(&directory).join("nope/x.svg");
        let failure = write_plot_file(&directory, "nope/x.svg", "<svg/>")
            .expect_err("the missing parent is refused");
        assert_eq!(failure.failure(), PlotFileWriteFailure::PathNotFound);
        assert_eq!(failure.requested(), requested.to_string_lossy());
        assert!(
            failure
                .detail()
                .starts_with("Could not find a part of the path '"),
            "{}",
            failure.detail()
        );
        assert!(failure.detail().ends_with("'."), "{}", failure.detail());
    }

    #[test]
    fn the_classification_is_the_runner_plot_file_error() {
        let directory = probe_directory("round90-plot-file-classified");
        let failure =
            write_plot_file(&directory, "nope/x.svg", "<svg/>").expect_err("the write is refused");
        let error = failure.to_lang_error();
        assert_eq!(error.code, "PlotFileError");
        assert_eq!(error.category, "TypeMismatch");
        assert_eq!(error.message, failure.to_string());
        assert!(error.message.contains(failure.requested()));
    }
}
