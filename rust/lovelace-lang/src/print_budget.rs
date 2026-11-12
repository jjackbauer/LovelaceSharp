//! The reference's budgeted rendering (`Printing.Print`, `PrintOptions`/`PrintBudget`/
//! `PrintOutcome`) as a library surface.
//!
//! `lovelace_symbolics::Expr::print` owns the four renderings but no budget. The reference exposes
//! a budget as a NODE cap and/or a DEPTH cap; a rendering over either is abbreviated at a token
//! boundary and the caller receives the reason, the expression's true node count, the limit and
//! the partial text. The abbreviation is only reported when something was actually DROPPED: a
//! compact rendering that fits the kernel's character allowance is returned whole, with no
//! truncation, even when the expression's node count is over the budget. That is the reference's
//! `Printing.Print` rule (Lovelace.Symbolics/Printing.cs:360-399), and it is what the two deferred
//! `PrintBudgetTests` rows and the two `PrintBudgetTokenBoundaryTests` rows assert.

use lovelace_symbolics::{Expr, PrintMode};

/// The reference's `Printing.PrintBudget`: a node cap, a depth cap, or both. `None` on a limit
/// means "no limit", so the default is the unbounded rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrintBudget {
    /// The maximum node count a rendering may have before it is abbreviated.
    pub max_nodes: Option<usize>,
    /// The maximum expression depth a rendering may have before it is abbreviated.
    pub max_depth: Option<usize>,
}

/// The reference's `Printing.PrintTruncation`: why the rendering was abbreviated, how big the
/// expression really is, the budget that stopped it, and the abbreviated text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintTruncation {
    /// `"node-budget"` or `"depth-limit"`, exactly as the reference publishes it.
    pub reason: &'static str,
    /// The expression's true node count.
    pub node_count: usize,
    /// The limit that stopped the rendering (nodes or depth, whichever the reason names).
    pub budget: usize,
    /// The abbreviated text the caller may show.
    pub partial_result: String,
}

/// The reference's `Printing.PrintOutcome`: the text plus, when the budget stopped a full
/// rendering, the structured reason. A whole rendering has no truncation at all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintOutcome {
    /// The rendering, whole or abbreviated.
    pub text: String,
    /// True only when the caller is NOT looking at the whole expression.
    pub truncated: bool,
    /// The structured reason, present exactly when `truncated` is true.
    pub truncation: Option<PrintTruncation>,
}

/// Renders `expr` under `budget`, mirroring the reference's `Printing.Print`.
///
/// Without a budget this is exactly `expr.print(mode)` and reports no truncation. With one, the
/// node cap is consulted first and the depth cap only when no node cap was given (the reference's
/// `if ... else if ...`); once a cap is over, the rendering is still produced by the normal
/// printer and is abbreviated only when it is longer than the kernel's character allowance
/// `max(48, 6 x limit)`. A rendering that fits is returned whole: "truncated" means something
/// was dropped, never merely that the value is over the node budget.
pub fn print(expr: &Expr, mode: PrintMode, budget: Option<PrintBudget>) -> PrintOutcome {
    let Some(budget) = budget else {
        return whole(expr.print(mode));
    };

    let nodes = expr.node_count();
    let mut reason: Option<(&'static str, usize)> = None;
    if let Some(max_nodes) = budget.max_nodes {
        if nodes > max_nodes {
            reason = Some(("node-budget", max_nodes));
        }
    } else if let Some(max_depth) = budget.max_depth {
        if expr_depth(expr) > max_depth {
            reason = Some(("depth-limit", max_depth));
        }
    }

    let Some((reason, limit)) = reason else {
        return whole(expr.print(mode));
    };

    let full = expr.print(mode);
    let chars: Vec<char> = full.chars().collect();
    let character_allowance = limit.saturating_mul(6).max(48);
    // Nothing was dropped, so nothing is reported: "truncated" means the caller is NOT looking
    // at the whole expression (the reference's Printing.cs:388-391).
    if chars.len() <= character_allowance {
        return whole(full);
    }

    let cut = safe_cut(&chars, character_allowance);
    let text = if cut == 0 {
        "…".to_string()
    } else {
        format!("{} …", chars[..cut].iter().collect::<String>())
    };
    PrintOutcome {
        truncated: true,
        truncation: Some(PrintTruncation {
            reason,
            node_count: nodes,
            budget: limit,
            partial_result: text.clone(),
        }),
        text,
    }
}

/// Renders with the pretty mode under `budget`.
pub fn print_pretty(expr: &Expr, budget: Option<PrintBudget>) -> PrintOutcome {
    print(expr, PrintMode::Pretty, budget)
}

/// The reference's `Printing.ExprDepth`: the structural depth of the expression.
///
/// The walk is iterative because this measure exists to REFUSE a value a recursive printer would
/// die on. The node kinds are the reference's switch: a sum, product, application or relation is
/// one plus its deepest child and every leaf is one. The explicit quotient is the Rust tree's
/// extra binary node; it is counted like the other binary nodes so a deep quotient cannot slip
/// past the cap.
pub fn expr_depth(root: &Expr) -> usize {
    let mut max = 1usize;
    let mut stack: Vec<(&Expr, usize)> = vec![(root, 1)];
    while let Some((expr, depth)) = stack.pop() {
        if depth > max {
            max = depth;
        }
        let next = depth + 1;
        match expr {
            Expr::Add(items) | Expr::Mul(items) | Expr::Fun(_, items) => {
                for item in items {
                    stack.push((item, next));
                }
            }
            Expr::Pow(left, right) | Expr::Div(left, right) | Expr::Rel(_, left, right) => {
                stack.push((left, next));
                stack.push((right, next));
            }
            Expr::Num(_) | Expr::Sym(_) | Expr::Real(_) => {}
        }
    }
    max
}

fn whole(text: String) -> PrintOutcome {
    PrintOutcome {
        text,
        truncated: false,
        truncation: None,
    }
}

/// The reference's `Printing.SafeCut`: the last token boundary at or before `cut`; when the
/// allowance falls inside the FIRST token, the boundary after that whole token instead (keeping
/// one token may exceed the allowance, splitting it would publish a token the value does not
/// contain); and `0` when the rendering IS one token longer than the allowance, because a prefix
/// that reproduced the whole token would drop nothing.
fn safe_cut(chars: &[char], cut: usize) -> usize {
    let mut i = cut.min(chars.len());
    while i > 0 && is_token_char(chars[i - 1]) {
        i -= 1;
    }
    if i > 0 {
        return i;
    }
    let mut after = cut.min(chars.len());
    while after < chars.len() && is_token_char(chars[after]) {
        after += 1;
    }
    if after < chars.len() { after } else { 0 }
}

fn is_token_char(c: char) -> bool {
    c.is_alphanumeric() || c == '.'
}
