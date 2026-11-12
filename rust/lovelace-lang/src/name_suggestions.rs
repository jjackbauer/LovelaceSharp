//! The reference `Lovelace.Suite.NameSuggestions` surface: closest registered names, syntax
//! hints and the structural edit distance.
//!
//! The engine's own unresolved-call path uses `registry::suggest_among` so a bare engine never
//! names a plugin it cannot answer. This module is the general contract the reference publishes
//! for a CALLER-SUPPLIED registry (and for the tests that pin the metric), delegating to the one
//! ranking/synonym table in `registry` rather than carrying a second copy.

pub use crate::registry::MAX_SUGGESTIONS;

/// The registered names closest to `queried`, best first, capped and order-independent.
pub fn suggest<S: AsRef<str>>(registered: &[S], queried: &str) -> Vec<String> {
    crate::registry::suggest_from(registered, queried)
}

/// The syntax notes for a name that is language syntax, not a missing builtin.
pub fn hints_for(queried: &str) -> Vec<String> {
    crate::registry::hints(queried)
}

/// Rank 0..=3, or -1 when unrelated (the reference's `NameSuggestions.Rank`).
pub fn rank(registered: &str, queried: &str) -> i32 {
    crate::registry::name_rank(registered, queried)
}

/// The reference's `NameSuggestions.EditDistance`, including the `int.MaxValue` sentinel.
pub fn edit_distance(a: &str, b: &str) -> i32 {
    crate::registry::edit_distance(a, b)
}
