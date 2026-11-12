//! The adapter over the workspace engine (lovelace_lang::Engine).
//!
//! This module is compiled only with the real-engine feature, because lovelace-lang is owned by
//! another agent and may not yet expose the contract API. The mapping below is exactly the
//! contract in rust/CONTRACT.md:
//!
//! - eval -> Engine::eval, display -> Engine::format
//! - variables / functions / take_output / last_plot / set_precision / precision / revision
//!
//! WKS-004: Clear Variables removes every global by name through the engine's own
//! `Engine::remove_variable` seam, so user `func` definitions (and the builtin registry) stay
//! registered. Rebuilding the engine would have cleared both and is no longer what the command
//! does.

use super::{EngineFunction, EngineState, EngineVariable, Evaluator, PlotCapture};
use lovelace_json::Json;

/// Creates a boxed RealEngine.
pub fn boxed() -> Box<dyn Evaluator> {
    Box::new(RealEngine::new())
}

/// An adapter that owns one lovelace_lang engine.
pub struct RealEngine {
    engine: lovelace_lang::Engine,
    /// The engine's structured projection of the value its last successful `eval` returned.
    ///
    /// `Engine::eval` hands back a `Value`; the projection is produced here with the engine's
    /// own `structured::project_value` at the moment the value exists, so the snapshot and the
    /// window never have to reconstruct structure from a display string.
    last_structured: Option<Json>,
    /// SYM-001: the engine's inspection payload for that same value, captured at the same point.
    ///
    /// The payload is built by [`super::inspection::capture`] from the value's structured
    /// projection, the expression AST, `inspect(value)`, the TransformResult record and the
    /// compile path. It is never built from a display string and never re-evaluates the buffer.
    last_inspection: Option<Json>,
}

impl RealEngine {
    /// Creates an engine with the core builtins loaded.
    pub fn new() -> RealEngine {
        RealEngine {
            engine: lovelace_lang::Engine::new(),
            last_structured: None,
            last_inspection: None,
        }
    }
}

impl Default for RealEngine {
    fn default() -> RealEngine {
        RealEngine::new()
    }
}

impl Evaluator for RealEngine {
    fn eval(&mut self, src: &str) -> Result<String, String> {
        // A failed eval must not leave the previous run's structure looking like this run's.
        self.last_structured = None;
        self.last_inspection = None;
        match self.engine.eval(src) {
            Ok(value) => {
                // RES-001: the engine's own structured projection, the same function the CLI
                // runner and the C# parity reference use. The desktop only carries the Json.
                self.last_structured = Some(lovelace_lang::structured::project_value(
                    &self.engine,
                    &value,
                    None,
                ));
                // SYM-001: the engine's inspection of that same value, captured at the same
                // commit point. A value that is not symbolic still gets a payload whose six
                // sections name the reason they are absent.
                self.last_inspection = Some(super::inspection::capture(&mut self.engine, &value));
                Ok(self.engine.format(&value))
            }
            Err(error) => Err(error.message),
        }
    }

    fn last_structured(&self) -> Option<Json> {
        self.last_structured.clone()
    }

    fn last_inspection(&self) -> Option<Json> {
        self.last_inspection.clone()
    }

    fn set_precision(&mut self, decimals: usize) {
        self.engine.set_precision(decimals);
    }

    fn set_cancel_flag(&mut self, flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
        self.engine.set_cancel_flag(flag);
    }

    fn set_plot_dir(&mut self, dir: &str) {
        self.engine.set_plot_dir(dir);
    }

    fn precision(&self) -> usize {
        self.engine.precision()
    }

    fn revision(&self) -> u64 {
        self.engine.revision()
    }

    fn variables(&self) -> Vec<(String, String)> {
        self.engine.variables()
    }

    fn functions(&self) -> Vec<(String, Vec<String>)> {
        self.engine.functions()
    }

    /// Copies the engine's own `capture_state` snapshot.
    ///
    /// The engine's capture is the registry: every enabled builtin with its declared parameter
    /// names and plugin, every host registration, and the script's user `func` definitions with
    /// `is_builtin == false`. The desktop maps it field for field and never invents an entry, so
    /// a builtin added to (or removed from) the engine appears in (or disappears from)
    /// completion without a desktop change.
    fn state(&self) -> Option<EngineState> {
        // The structured projection of every variable value, from the engine's own project_value.
        // `capture_state` publishes (name, kind, display); the values are published separately by
        // `variable_values`, which is the same engine data the display was formatted from.
        let structured_variables: std::collections::HashMap<String, Json> = self
            .engine
            .variable_values()
            .into_iter()
            .map(|(name, value)| {
                (
                    name,
                    lovelace_lang::structured::project_value(&self.engine, &value, None),
                )
            })
            .collect();
        let snapshot = self.engine.capture_state();
        Some(EngineState {
            variables: snapshot
                .variables
                .into_values()
                .map(|variable| {
                    let structured = structured_variables.get(&variable.name).cloned();
                    EngineVariable {
                        name: variable.name,
                        kind: variable.kind,
                        display: variable.display,
                        structured,
                    }
                })
                .collect(),
            functions: snapshot
                .functions
                .into_values()
                .map(|function| {
                    // `StateFunction` carries the plugin but not the registry's one-line summary.
                    // The registry entry is the engine's own published data for the same
                    // function, so the summary is read from there instead of being invented or
                    // transcribed into the desktop: an unregistered user definition has none.
                    let summary = if function.is_builtin {
                        lovelace_lang::registry::find(&function.name)
                            .and_then(|info| info.summary)
                            .map(str::to_string)
                    } else {
                        None
                    };
                    EngineFunction {
                        name: function.name,
                        parameters: function.parameters,
                        is_builtin: function.is_builtin,
                        plugin: function.plugin,
                        summary,
                    }
                })
                .collect(),
        })
    }

    fn take_output(&mut self) -> Vec<String> {
        self.engine.take_output()
    }

    fn last_plot(&self) -> Option<PlotCapture> {
        self.engine.last_plot().map(|plot| PlotCapture {
            path: plot.path,
            title: plot.title,
            svg: plot.svg,
        })
    }

    /// Removes one global variable through the engine's own selective seam.
    ///
    /// `lovelace_lang::Engine::remove_variable` removes exactly that binding and raises the
    /// engine's `VariableChanged(removed: true)` event, leaving the function registry untouched.
    fn remove_variable(&mut self, name: &str) -> bool {
        self.engine.remove_variable(name)
    }

    /// Clear Variables: remove every global, keep the function table.
    ///
    /// WKS-004: this used to rebuild `lovelace_lang::Engine`, which silently discarded the user's
    /// `func` definitions even though the command is labelled "Clear variables". The engine has a
    /// selective removal seam, so the adapter uses it and the registry (builtins and user
    /// definitions) is exactly what it was before the clear.
    fn clear(&mut self) {
        self.clear_variables();
    }

    fn name(&self) -> &'static str {
        "lovelace-lang"
    }
}
