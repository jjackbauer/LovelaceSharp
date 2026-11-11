//! Round 46, ADV-11 falsification harness: the C# test-support surface, ported additively.
//!
//! The C# harness lives in the test project (Lovelace.Symbolics.Tests/Falsification/*.cs and
//! FalsificationTests.cs / FalsificationGateTests.cs) and evaluates expressions through
//! Evaluation.EvaluateToNum. The Rust analogue of that evaluator is numeric::evaluate, which
//! lives in this crate; lovelace-symbolics has neither a numeric evaluator nor a dependency on
//! lovelace-real, so this crate is the only home in which the harness can actually evaluate the
//! identities it attacks. The module is test-support and does not change any shipped behaviour.

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]

mod boundary;
mod gate;
mod looping;
mod observation;
mod probe;
mod probeset;
mod tests;
