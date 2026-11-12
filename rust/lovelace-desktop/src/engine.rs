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

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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

/// Returns the application-scoped directory the engine writes plot files into.
///
/// The directory is `%APPDATA%\Lovelace\plots` on Windows, derived exactly like the settings
/// path in [`crate::settings::default_config_path`] (`%APPDATA%\Lovelace\desktop.json`). It is
/// never below the process working directory: PLF-006 forbids writing beside the executable,
/// and the previous fallback that did so is gone on purpose. A process with no APPDATA, XDG or
/// HOME falls back to a temp directory, which is still not the working directory.
///
/// The result is computed once per process; every document and every run shares the root.
pub fn default_plot_dir() -> std::path::PathBuf {
    static DIRECTORY: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIRECTORY
        .get_or_init(|| {
            let fallback = || std::env::temp_dir().join("lovelace-desktop").join("plots");
            let candidate = match crate::settings::default_plot_dir() {
                Some(directory) => directory,
                None => return fallback(),
            };
            // Even a misconfigured HOME must never turn into a write below the working
            // directory: that is exactly the PLF-006 defect this function removes.
            match std::env::current_dir() {
                Ok(cwd) if candidate.starts_with(&cwd) => fallback(),
                _ => candidate,
            }
        })
        .clone()
}

/// Creates the application-scoped plot directory and reports a failure instead of ignoring it.
///
/// Returns the resolved path and, when the directory cannot be created or written, a message
/// naming the path and the operating-system error. The caller copies that message into the
/// product log at startup; silently reusing an unwritable directory is what made the old plot
/// path defect invisible.
pub fn prepare_plot_dir() -> (std::path::PathBuf, Option<String>) {
    let path = default_plot_dir();
    if let Err(error) = std::fs::create_dir_all(&path) {
        let message = format!(
            "Cannot create the plot directory {}: {error}. Plot statements will fail until it is writable.",
            path.display()
        );
        return (path, Some(message));
    }
    let probe = path.join(".write-probe");
    match std::fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            (path, None)
        }
        Err(error) => {
            let message = format!(
                "Cannot write in the plot directory {}: {error}. Plot statements will fail until it is writable.",
                path.display()
            );
            (path, Some(message))
        }
    }
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

/// One function the engine exposes, with its declared parameter names.
///
/// The completion catalogue consumes these entries verbatim; the desktop never carries a
/// builtin list of its own. The real engine fills them from its own snapshot
/// (`Engine::capture_state`), which publishes every builtin in the registry, every host or
/// plugin registration, and every user `func` definition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineFunction {
    /// The registered name.
    pub name: String,
    /// The declared parameter names, in declaration order.
    pub parameters: Vec<String>,
    /// True for a builtin, false for a user definition or a host/plugin registration.
    pub is_builtin: bool,
}

/// One variable visible to the document, with the kind the engine published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineVariable {
    /// The variable name as written in source.
    pub name: String,
    /// The published kind name (`Natural`, `Real`, `Text`, `Record`, ...).
    pub kind: String,
    /// The display rendering the engine produced.
    pub display: String,
}

/// A coherent capture of the engine's variables and functions.
///
/// This is the desktop-side view of the engine's snapshot API: one value that a completion
/// catalogue (and any future outline) can be built from without asking the engine twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EngineState {
    /// Variables, sorted by name.
    pub variables: Vec<EngineVariable>,
    /// Functions (builtins and user definitions), sorted by name.
    pub functions: Vec<EngineFunction>,
}

impl EngineState {
    /// True when the capture names no symbol at all.
    #[allow(dead_code)] // invariant accessor used by the engine tests
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty() && self.functions.is_empty()
    }

    /// Builds a state from the older `(name, typed display)` contract accessors.
    ///
    /// Engines that override [`Evaluator::state`] (both shipped engines do) never take this
    /// path. It exists so a minimal third-party evaluator that only implements the two older
    /// accessors still contributes its symbols; the kind is recovered from a `"value (Kind)"`
    /// typed rendering when there is one, and the function origin is unknown, so functions are
    /// reported as non-builtins rather than claimed as registry entries.
    pub fn from_legacy(
        variables: &[(String, String)],
        functions: &[(String, Vec<String>)],
    ) -> EngineState {
        EngineState {
            variables: variables
                .iter()
                .map(|(name, display)| {
                    let (display, kind) = split_typed(display);
                    EngineVariable {
                        name: name.clone(),
                        kind,
                        display,
                    }
                })
                .collect(),
            functions: functions
                .iter()
                .map(|(name, parameters)| EngineFunction {
                    name: name.clone(),
                    parameters: parameters.clone(),
                    is_builtin: false,
                })
                .collect(),
        }
    }
}

/// Splits `"value (Kind)"` into its display and kind halves; an untyped display stays whole.
fn split_typed(display: &str) -> (String, String) {
    if let Some(index) = display.rfind(" (") {
        let wrapped = &display[index + 2..];
        if let Some(kind) = wrapped.strip_suffix(')') {
            let plausible = !kind.is_empty()
                && kind
                    .chars()
                    .all(|ch| ch.is_ascii_alphabetic() || ch == ' ' || ch == '|' || ch == '_');
            if plausible {
                return (display[..index].to_string(), kind.to_string());
            }
        }
    }
    (display.to_string(), "Value".to_string())
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

    /// Installs (or clears) the live cancellation flag the engine polls inside long kernels.
    ///
    /// The execution core creates one flag per run and sets it from [`crate::execution::ExecutionHost::cancel`],
    /// so a run whose work is one long kernel stops without waiting for a statement boundary. An
    /// engine that has no kernel polling seam ignores it; the statement-boundary token still
    /// applies, so the default is a no-op rather than an error.
    fn set_cancel_flag(&mut self, _flag: Option<Arc<AtomicBool>>) {}

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

    /// Captures the engine's own symbol table: variables with their kinds and every function
    /// with its parameter names and builtin flag.
    ///
    /// This is the source of the completion catalogue. Both shipped engines override it: the
    /// real engine copies `lovelace_lang::Engine::capture_state` (the registry itself, never a
    /// desktop copy), and the deterministic fake reports the kinds it actually stores. The
    /// default below is only for minimal evaluators; it derives a best-effort capture from the
    /// two older accessors.
    fn state(&self) -> Option<EngineState> {
        Some(EngineState::from_legacy(&self.variables(), &self.functions()))
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
    fn plot_dir_is_the_app_scoped_application_directory_plots_child() {
        // Windows: %APPDATA%\Lovelace\plots. Elsewhere: the same XDG/HOME derivation as
        // settings::default_config_path, with its parent as the application directory.
        let expected = if let Some(appdata) = std::env::var_os("APPDATA") {
            Some(
                std::path::PathBuf::from(appdata)
                    .join("Lovelace")
                    .join("plots"),
            )
        } else {
            crate::settings::default_plot_dir()
        };
        let Some(expected) = expected else {
            // No APPDATA/XDG/HOME: the temp fallback is the only app-scoped option left.
            return;
        };
        assert_eq!(
            default_plot_dir(),
            expected,
            "the plot directory must be derived from the app-scoped application directory"
        );
    }

    #[test]
    fn plot_dir_is_never_under_the_working_directory() {
        let cwd = std::env::current_dir().expect("current directory");
        let plot_dir = default_plot_dir();
        assert!(
            !plot_dir.starts_with(&cwd),
            "plot directory {} must not be under the working directory {}",
            plot_dir.display(),
            cwd.display()
        );
    }

    #[test]
    fn prepare_plot_dir_creates_the_resolved_directory_and_reports_failure() {
        let (directory, error) = prepare_plot_dir();
        assert_eq!(
            directory,
            default_plot_dir(),
            "the prepared directory is the resolved application-scoped directory"
        );
        assert!(
            directory.is_dir(),
            "prepare_plot_dir must create {} so the engine has a place to write",
            directory.display()
        );
        if let Some(message) = error {
            assert!(
                message.contains(&directory.display().to_string()),
                "a creation failure must name the resolved directory: {message}"
            );
        }
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
        assert!(engine.state().expect("a default capture").is_empty());
    }

    #[test]
    fn legacy_capture_recovers_a_typed_kind_and_never_claims_a_builtin() {
        let state = EngineState::from_legacy(
            &[
                ("n".to_string(), "3 (Natural)".to_string()),
                ("flag".to_string(), "true".to_string()),
                ("pair".to_string(), "[1, 2] (Array)".to_string()),
            ],
            &[("f".to_string(), vec!["a".to_string()])],
        );
        assert_eq!(state.variables[0].name, "n");
        assert_eq!(state.variables[0].kind, "Natural");
        assert_eq!(state.variables[0].display, "3");
        assert_eq!(state.variables[1].kind, "Value", "an untyped display has no kind");
        assert_eq!(state.variables[1].display, "true");
        assert_eq!(state.variables[2].kind, "Array");
        assert_eq!(state.variables[2].display, "[1, 2]");
        assert!(!state.functions[0].is_builtin);
        assert_eq!(state.functions[0].parameters, vec!["a".to_string()]);
    }

    #[test]
    fn the_fake_engine_reports_its_own_variable_kinds_and_builtins() {
        let mut engine = FakeEngine::new();
        engine.eval("x = 3\ntext = \"hi\"").expect("the fake run");
        let state = engine.state().expect("the fake publishes a state");
        let x = state.variables.iter().find(|v| v.name == "x").unwrap();
        assert_eq!(x.kind, "Natural");
        let text = state.variables.iter().find(|v| v.name == "text").unwrap();
        assert_eq!(text.kind, "Text");
        assert!(
            state.functions.iter().any(|f| f.name == "abs" && f.is_builtin),
            "the fake's own registry is marked builtin: {:?}",
            state.functions
        );
        assert!(
            state
                .functions
                .iter()
                .all(|f| !f.parameters.is_empty() || f.name == "clear"),
            "every function carries its declared parameter names"
        );
    }

    /// Proves the catalogue's function source is the *engine's* registry: the real adapter
    /// publishes the real builtins (with their declared parameter names) and marks a script's
    /// own `func` definition as a non-builtin. If the engine registry changed, this test moves
    /// with it; the desktop has no second list that could drift.
    #[test]
    #[cfg(feature = "real-engine")]
    fn the_real_engine_publishes_its_registry_and_marks_user_functions() {
        let mut engine = RealEngine::new();
        let state = engine.state().expect("the real engine publishes its snapshot");
        let sqrt = state
            .functions
            .iter()
            .find(|function| function.name == "sqrt")
            .unwrap_or_else(|| panic!("sqrt is missing from {} functions", state.functions.len()));
        assert!(sqrt.is_builtin, "sqrt comes from the engine's registry");
        assert_eq!(sqrt.parameters, vec!["x".to_string()]);

        engine
            .eval("func myfunc(a, b) = a + b\nx = 3")
            .expect("the real engine evaluates the definition");
        let after = engine.state().expect("a second capture");
        let norm = after
            .functions
            .iter()
            .find(|function| function.name == "myfunc")
            .expect("the user function is in the capture");
        assert!(!norm.is_builtin, "a script `func` is not a builtin");
        assert_eq!(norm.parameters, vec!["a".to_string(), "b".to_string()]);
        assert!(
            after.functions.iter().filter(|f| f.is_builtin).count()
                >= state.functions.iter().filter(|f| f.is_builtin).count(),
            "the user definition does not replace the registry"
        );
        let x = after
            .variables
            .iter()
            .find(|variable| variable.name == "x")
            .expect("the run's variable is captured with its kind");
        assert_eq!(x.kind, "Natural");
    }
}
