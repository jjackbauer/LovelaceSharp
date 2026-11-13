//! Fault injection for the RUN-017 recovery path.
//!
//! A worker failure has to be observable in the shipped binary, not only in a unit test with a
//! test-local factory. `LOVELACE_ENGINE_FAULT=panic-once` makes the first statement of the first
//! run panic **inside** the evaluator — the exact failure mode `RunWorker::run` isolates with
//! `catch_unwind`. Every later run in the same process evaluates through the normal engine, so
//! one process demonstrates the panic, the recoverable `Failed` state, the released run lock and
//! the next successful run on the same buffer.
//!
//! The switch is off unless the variable is set, and it is the only fault mode defined. It is a
//! diagnostic seam for the acceptance capture, not a user-facing option; no script can reach it.

use std::sync::atomic::{AtomicBool, Ordering};

use super::{EngineError, EngineState, Evaluator, PlotCapture};

/// The environment variable that selects a fault mode.
pub const FAULT_ENV: &str = "LOVELACE_ENGINE_FAULT";

/// The only defined fault mode.
pub const PANIC_ONCE: &str = "panic-once";

static PANICKED: AtomicBool = AtomicBool::new(false);

/// True when this process was started with the panic-once fault switch.
pub fn panic_once_requested() -> bool {
    std::env::var_os(FAULT_ENV)
        .map(|value| value == PANIC_ONCE)
        .unwrap_or(false)
}

/// Creates the fault-injecting evaluator (the panic fires on the first `eval` only).
pub fn boxed() -> Box<dyn Evaluator> {
    Box::new(FaultEngine {
        inner: base_engine(),
    })
}

/// The evaluator the fault wrapper delegates to after the injected panic.
fn base_engine() -> Box<dyn Evaluator> {
    #[cfg(feature = "real-engine")]
    {
        super::real::boxed()
    }
    #[cfg(not(feature = "real-engine"))]
    {
        super::fake::boxed()
    }
}

/// An evaluator that panics on its first `eval` and then behaves as the base engine.
struct FaultEngine {
    inner: Box<dyn Evaluator>,
}

impl Evaluator for FaultEngine {
    fn eval(&mut self, src: &str) -> Result<String, String> {
        if !PANICKED.swap(true, Ordering::SeqCst) {
            panic!("simulated engine fault: the evaluator panicked inside a statement");
        }
        self.inner.eval(src)
    }

    fn last_error(&self) -> Option<EngineError> {
        self.inner.last_error()
    }

    fn set_precision(&mut self, decimals: usize) {
        self.inner.set_precision(decimals);
    }

    fn set_plot_dir(&mut self, dir: &str) {
        self.inner.set_plot_dir(dir);
    }

    fn set_cancel_flag(&mut self, flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>) {
        self.inner.set_cancel_flag(flag);
    }

    fn precision(&self) -> usize {
        self.inner.precision()
    }

    fn revision(&self) -> u64 {
        self.inner.revision()
    }

    fn variables(&self) -> Vec<(String, String)> {
        self.inner.variables()
    }

    fn functions(&self) -> Vec<(String, Vec<String>)> {
        self.inner.functions()
    }

    fn state(&self) -> Option<EngineState> {
        self.inner.state()
    }

    fn last_structured(&self) -> Option<lovelace_json::Json> {
        self.inner.last_structured()
    }

    fn last_inspection(&self) -> Option<lovelace_json::Json> {
        self.inner.last_inspection()
    }

    fn take_output(&mut self) -> Vec<String> {
        self.inner.take_output()
    }

    fn last_plot(&self) -> Option<PlotCapture> {
        self.inner.last_plot()
    }

    fn name(&self) -> &'static str {
        "fault-injected"
    }
}
