//! The evaluator boundary between the desktop application and the Lovelace engine.
//!
//! The workspace contract fixes the engine API that lovelace-lang publishes, but that crate is
//! owned by another agent and may still be a stub while this one is built. Everything the desktop
//! application needs from an engine therefore goes through Evaluator, a one-method trait (plus
//! snapshot accessors with sensible defaults):
//!
//! - FakeEngine is a small, deterministic, integer-only interpreter that ships with this crate.
//!   It is what the unit tests and --selftest drive, and it keeps the application usable before
//!   the real engine lands.
//! - RealEngine (feature real-engine, enabled by default in Cargo.toml) is the adapter over
//!   lovelace_lang::Engine: the desktop app drives the same engine as the CLI and the corpus gate.
//!   Building with --no-default-features keeps the crate usable offline while that engine is
//!   being written, and switches every document to FakeEngine.
//!
//! The trait keeps the engine call shape from the contract exactly:
//! eval(source) -> Result<String, String>, where the string is the value's display form.

// The fake engine is the fallback for --no-default-features builds and the deterministic engine
// used by the tests; with the real engine enabled the only non-test use is the factory below.
#[cfg_attr(not(test), allow(dead_code))]
mod fake;
#[cfg(feature = "real-engine")]
pub mod real;

#[allow(unused_imports)]
// Re-exported for tests and for hosts that want the deterministic engine explicitly.
pub use fake::FakeEngine;
#[allow(unused_imports)]
#[cfg(feature = "real-engine")]
pub use real::RealEngine;

/// Default display precision, in decimal places.
pub const DEFAULT_PRECISION: usize = 1000;

/// Returns a directory the engine can actually write plot files into.
///
/// The engine's own default is the process temp directory, which is not writable for the
/// desktop process in every environment (the round-5 window run observed `Access is denied`
/// there). We prefer a per-user application directory, prove it is writable with a probe file,
/// and fall back to a temp subdirectory or a named directory under the working directory.
///
/// The result is computed once per process; every document and every run shares the root.
pub fn default_plot_dir() -> std::path::PathBuf {
    static DIRECTORY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIRECTORY.get_or_init(probe_plot_dir).clone()
}

fn probe_plot_dir() -> std::path::PathBuf {
    let mut candidates = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(std::path::PathBuf::from(local).join("Lovelace").join("plots"));
    }
    if let Some(roaming) = std::env::var_os("APPDATA") {
        candidates.push(std::path::PathBuf::from(roaming).join("Lovelace").join("plots"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        candidates.push(std::path::PathBuf::from(profile).join("Lovelace").join("plots"));
    }
    candidates.push(std::env::temp_dir().join("lovelace-desktop-plots"));
    // The process may be confined to its working directory (for example a sandboxed build
    // agent). Keep the fallback named so it is obvious what the directory is.
    candidates.push(
        std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("lovelace-plots"),
    );
    for candidate in candidates {
        if std::fs::create_dir_all(&candidate).is_err() {
            continue;
        }
        let probe = candidate.join(".write-probe");
        if std::fs::write(&probe, b"probe").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return candidate;
        }
    }
    std::path::PathBuf::from("lovelace-plots")
}

/// A plot produced by an evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotCapture {
    /// Path the engine wrote (or would write) the SVG to.
    pub path: String,
    /// Human-readable title.
    pub title: String,
    /// The SVG document itself.
    pub svg: String,
}

/// The complete, coherent result of one evaluation: what the panes need to update together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutput {
    /// The value display, or the error message.
    pub result: Result<String, String>,
    /// print() lines produced by the run, in order.
    pub output: Vec<String>,
    /// Variables visible to the document after the run: name, display.
    pub variables: Vec<(String, String)>,
    /// Functions visible to the document after the run: name, parameters.
    pub functions: Vec<(String, Vec<String>)>,
    /// The most recent plot, when the run produced one.
    pub plot: Option<PlotCapture>,
}

/// An engine instance owned by one document.
///
/// The only required method is the contract's eval. Everything else has a default so that a
/// minimal engine (and every test fake) can be written in a few lines, while a full engine can
/// publish variables, functions, output and the last plot.
pub trait Evaluator {
    /// Evaluates a script and returns its display form, or an error message.
    fn eval(&mut self, src: &str) -> Result<String, String>;

    /// Evaluates a script and snapshots everything the panes need, even on failure.
    fn eval_full(&mut self, src: &str) -> RunOutput {
        let result = self.eval(src);
        RunOutput {
            result,
            output: self.take_output(),
            variables: self.variables(),
            functions: self.functions(),
            plot: self.last_plot(),
        }
    }

    /// Sets the display precision, in decimal places.
    fn set_precision(&mut self, _decimals: usize) {}

    /// Sets the directory the engine writes plot files to.
    ///
    /// The engine's default is the process temp directory, which is not necessarily writable
    /// for the desktop process. The workbench points each run at its own writable directory so
    /// a plot statement cannot fail for a reason the user cannot see or fix.
    fn set_plot_dir(&mut self, _dir: &str) {}

    /// Returns the display precision, in decimal places.
    fn precision(&self) -> usize {
        DEFAULT_PRECISION
    }

    /// Returns the engine's revision counter.
    #[allow(dead_code)] // the status bar reports the document revision; hosts may use this
    fn revision(&self) -> u64 {
        0
    }

    /// Returns the current variables, sorted by name.
    fn variables(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// Returns the current functions, sorted by name.
    fn functions(&self) -> Vec<(String, Vec<String>)> {
        Vec::new()
    }

    /// Drains the print() output produced since the last call.
    fn take_output(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// Returns the most recent plot, if any.
    fn last_plot(&self) -> Option<PlotCapture> {
        None
    }

    /// Discards the engine's variables and transient state.
    fn clear(&mut self) {}

    /// Names the engine, for the log and the status bar.
    fn name(&self) -> &'static str {
        "evaluator"
    }
}

/// Creates a fresh evaluator for a new document.
pub type EngineFactory = fn() -> Box<dyn Evaluator>;

/// The engine a document gets by default.
#[cfg(not(feature = "real-engine"))]
pub fn default_engine_factory() -> EngineFactory {
    fake::boxed
}

/// The engine a document gets by default.
#[cfg(feature = "real-engine")]
pub fn default_engine_factory() -> EngineFactory {
    real::boxed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_full_snapshots_output_and_variables_in_one_step() {
        let mut engine = FakeEngine::new();
        let output = engine.eval_full("x = 2 + 3\nprint(\"hello\")\nx * 2");
        assert_eq!(output.result, Ok("10".to_string()));
        assert_eq!(output.output, vec!["hello".to_string()]);
        assert_eq!(output.variables, vec![("x".to_string(), "5".to_string())]);
    }

    #[test]
    fn eval_full_still_snapshots_state_when_the_run_fails() {
        let mut engine = FakeEngine::new();
        let output = engine.eval_full("y = 4\n1 / 0");
        assert!(output.result.is_err());
        assert_eq!(output.variables, vec![("y".to_string(), "4".to_string())]);
    }

    #[test]
    fn trait_defaults_are_sane() {
        struct Minimal;
        impl Evaluator for Minimal {
            fn eval(&mut self, _src: &str) -> Result<String, String> {
                Ok("ok".to_string())
            }
        }
        let mut engine = Minimal;
        assert_eq!(engine.eval("anything").unwrap(), "ok");
        assert_eq!(engine.precision(), DEFAULT_PRECISION);
        assert!(engine.variables().is_empty());
        assert!(engine.last_plot().is_none());
        assert_eq!(engine.name(), "evaluator");
        assert_eq!(engine.revision(), 0);
    }
}
