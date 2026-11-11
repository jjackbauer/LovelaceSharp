//! The adapter over the workspace engine (lovelace_lang::Engine).
//!
//! This module is compiled only with the real-engine feature, because lovelace-lang is owned by
//! another agent and may not yet expose the contract API. The mapping below is exactly the
//! contract in rust/CONTRACT.md:
//!
//! - eval -> Engine::eval, display -> Engine::format
//! - variables / functions / take_output / last_plot / set_precision / precision / revision
//!
//! The contract exposes no selective "clear variables" operation, so clear() rebuilds the engine
//! and restores its precision. That discards user functions as well as variables; the fake engine
//! behaves the same way, and the app labels the command "Clear variables" to match the Studio.

use super::{Evaluator, PlotCapture};

/// Creates a boxed RealEngine.
pub fn boxed() -> Box<dyn Evaluator> {
    Box::new(RealEngine::new())
}

/// An adapter that owns one lovelace_lang engine.
pub struct RealEngine {
    engine: lovelace_lang::Engine,
}

impl RealEngine {
    /// Creates an engine with the core builtins loaded.
    pub fn new() -> RealEngine {
        RealEngine {
            engine: lovelace_lang::Engine::new(),
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
        match self.engine.eval(src) {
            Ok(value) => Ok(self.engine.format(&value)),
            Err(error) => Err(error.message),
        }
    }

    fn set_precision(&mut self, decimals: usize) {
        self.engine.set_precision(decimals);
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

    fn clear(&mut self) {
        let precision = self.engine.precision();
        self.engine = lovelace_lang::Engine::new();
        self.engine.set_precision(precision);
    }

    fn name(&self) -> &'static str {
        "lovelace-lang"
    }
}
