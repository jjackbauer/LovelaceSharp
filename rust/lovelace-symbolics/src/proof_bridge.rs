//! The rule to proof-obligation bridge (the reference's `RewriteProofs`).
//!
//! The reference's rewrite registry and its proof bridge are deliberately two datasets: every
//! shipped rule has exactly one stated obligation, and an obligation never names a rule that
//! does not exist. The table is not a proof. Every shipped rule is `Unproven` today, so every
//! `theorem` is `None`; a future machine-checked development attaches its prover-side name
//! here without changing the registry.
//!
//! The registry ids already match the oracle; this module supplies the missing join point so
//! the census rows can assert the invariant instead of approximating it.

use crate::rewrite_registry::shipped_rule_ids;

/// How far a rule's proof obligation has got (the reference's `ProofStatus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofStatus {
    /// Stated, not proved by a machine. The honest default for every shipped rule today.
    Unproven,
    /// A development exists but is not wired into CI.
    InProgress,
    /// Checked by an external prover, keyed by `theorem`.
    MachineChecked,
}

/// A rule's proof obligation: what must be true for the rewrite to be sound, and where a
/// machine-checked development would attach (the reference's `ProofObligation`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofObligation {
    /// The stable rewrite-rule id this obligation belongs to.
    pub rule_id: String,
    /// The claim, in words, that a proof would establish.
    pub statement: String,
    /// The prover-side name once one exists; `None` while unproven.
    pub theorem: Option<String>,
    /// How far the obligation has got.
    pub status: ProofStatus,
}

impl ProofObligation {
    fn unproven(rule_id: &str, statement: &str) -> ProofObligation {
        ProofObligation {
            rule_id: rule_id.to_string(),
            statement: statement.to_string(),
            theorem: None,
            status: ProofStatus::Unproven,
        }
    }
}

/// The obligations, one per shipped rule, in the reference's table order.
pub fn proof_obligations() -> Vec<ProofObligation> {
    vec![
        ProofObligation::unproven(
            "trig.pythagorean-sin2-cos2",
            "sin(x)^2 + cos(x)^2 = 1 for every complex x",
        ),
        ProofObligation::unproven(
            "pow.sqrt-square-nonnegative",
            "sqrt(w^2) = w for every w >= 0",
        ),
        ProofObligation::unproven("pow.sqrt-square-real", "sqrt(x^2) = |x| for every real x"),
        ProofObligation::unproven("logexp.exp-log", "exp(log(x)) = x for every x > 0"),
        ProofObligation::unproven("logexp.log-exp", "log(exp(x)) = x for every real x"),
        ProofObligation::unproven("abs.abs-square", "|x|^2 = x^2 for every real x"),
        ProofObligation::unproven("abs.abs-neg", "|-x| = |x| for every x"),
        ProofObligation::unproven("rat.cancel-x-over-x", "x / x = 1 for every x != 0"),
        ProofObligation::unproven("rat.cancel-zero-over-x", "0 / x = 0 for every x != 0"),
    ]
}

/// The obligation for `rule_id`, or `None` when the rule has none.
pub fn proof_obligation_for(rule_id: &str) -> Option<ProofObligation> {
    proof_obligations()
        .into_iter()
        .find(|obligation| obligation.rule_id == rule_id)
}

/// The rule ids that have an obligation, in the reference's table order.
pub fn proof_rule_ids() -> Vec<String> {
    proof_obligations()
        .into_iter()
        .map(|obligation| obligation.rule_id)
        .collect()
}

/// True when every shipped registry id has exactly one obligation and no obligation names a
/// rule outside the registry.
pub fn bridge_is_in_step() -> bool {
    let mut shipped = shipped_rule_ids();
    let mut stated = proof_rule_ids();
    shipped.sort();
    stated.sort();
    shipped == stated
}
