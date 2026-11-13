//! The MGIR observation-driven behavioural graph (the Rust port of `Lovelace.Knowledge`).
//!
//! The pipeline has five stages, each in its own module:
//!
//! 1. [`proposal`] — the explicit proposal distribution q(z): deterministic one-dimensional
//!    sweeps, seeded Monte Carlo draws, frontier-guided bisection, and held-out validation
//!    probes. Every draw is a pure function of the config and the seed.
//! 2. [`runner`] — each script is executed by the **real** `lovelace-run` process over
//!    stdin/stdout; nothing is simulated.
//! 3. [`observation`] — canonicalization of the runner's JSON envelope into the observation
//!    sigma and the behavioural plane class.
//! 4. [`reducer`] — plane clustering, boundary detection by finite differences, guard
//!    fitting, and frontier marking.
//! 5. [`convergence`] — the C1-C4 metrics and the explicit stopping criterion.
//!
//! [`graph`] persists the product of a run, [`query`] renders read-only reports, and
//! [`cli`] implements the JSON-over-stdio command protocol.
//!
//! `Lovelace.Knowledge.Tests` is matched by the stage tests in `tests/`, which additionally
//! pin the seeded reproduction of the frozen C# graph (seed 20240617).

#![forbid(unsafe_code)]

pub mod cli;
pub mod converge;
pub mod convergence;
pub mod exact;
pub mod graph;
pub mod json;
pub mod model;
pub mod observation;
pub mod proposal;
pub mod query;
pub mod random;
pub mod reducer;
pub mod runner;
