//! Round 36: ExactnessFlagTests rows 4 and 5 (abs/ceil of an approximate argument).
//!
//! The C# row builds Exprs.Function(name, RealLeaf(3, 2)) and asserts the node IsExact is false.
//! The Rust surface builds the same node (Expr::Fun over Expr::Real) and asserts the same rule.

use crate::Expr;

#[test]
fn elementary_function_of_an_approximate_argument_is_inexact() {
    for name in ["abs", "ceil"] {
        let node = Expr::Fun(name.to_string(), vec![Expr::real("3.2")]);
        assert!(!node.is_exact(), "{name}(approximate) must stay inexact: {node}");
    }
}

