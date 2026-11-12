//! Round 54: the eight CompilationTests census rows, mirrored over the additive MathIR surface.
//!
//! Sources: Lovelace.Symbolics.Tests/CompilationTests.cs (lines 32-157) and Lovelace.MathIR
//! (Ir.cs, Compilation.cs, Typing.cs, Evaluator.cs). Every expected instruction sequence below
//! is derived from the reference's Lowering/Serialize rules, not copied from this implementation:
//! parameters index the parameter list, constants the pool, PowInt carries a fresh exponent
//! constant node, Add/Mul are left-folded binaries, and a piecewise lowers its otherwise branch
//! first and then wraps the branches from last to first in lazy Select nodes.

use std::cmp::Ordering;
use std::collections::HashMap;

use lovelace_int::Int;
use lovelace_nat::Nat;
use lovelace_real::Real;
use lovelace_symbolics::{Expr as Sym, Rat, RelOp};

use crate::mathir::{
    IrNode, IrOpKind, IrProgram, IrType, IrTyping, compile, compile_kernel, to_kernel,
};
use crate::numeric::Num;

fn params(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn num(value: i64) -> Num {
    Num::Exact(Rat::from_i64(value))
}

fn rat(numerator: i64, denominator: u64) -> Num {
    Num::Exact(Rat::from_parts(
        Int::from_i64(numerator),
        Nat::from_u64(denominator),
    ))
}

fn node(op: IrOpKind, operands: &[usize]) -> IrNode {
    IrNode::new(op, operands.to_vec(), 0)
}

/// NumOps.Compare == 0: exact values compare as rationals, approximate tiers as Reals.
fn assert_same(actual: &Num, expected: &Num) {
    match (actual, expected) {
        (Num::Exact(a), Num::Exact(b)) => assert_eq!(a, b),
        _ => {
            let places = 40;
            let left = actual.to_real(places).expect("real value");
            let right = expected.to_real(places).expect("real value");
            assert_eq!(
                left.cmp(&right),
                Ordering::Equal,
                "{actual:?} != {expected:?}"
            );
        }
    }
}

// CompilationTests.cs:149 - CompilationResult_IrText_RoundTrips
#[test]
fn compilation_result_ir_text_round_trips() {
    let expression = Sym::add(
        Sym::pow(Sym::sym("x"), Sym::num(5)),
        Sym::mul(Sym::num(3), Sym::sym("x")),
    );
    let compilation = compile(&expression, &params(&["x"])).expect("compile");
    let expected = "#!mathir 2\n\
param x\n\
const (rat 5 1)\n\
const (rat 3 1)\n\
Parameter 0 0\n\
Constant 0 0\n\
PowInt 0 0,1\n\
Constant 0 1\n\
Mul 0 3,0\n\
Add 0 2,4\n";
    assert_eq!(compilation.ir_text(), expected);
    assert_eq!(compilation.parameters().to_vec(), params(&["x"]));
    let program = compilation.program();
    assert_eq!(program.constants(), ["(rat 5 1)", "(rat 3 1)"]);
    assert_eq!(program.parameters(), ["x"]);
    assert_eq!(program.root(), Some(5));
    assert_eq!(
        program.nodes(),
        [
            node(IrOpKind::Parameter, &[0]),
            node(IrOpKind::Constant, &[0]),
            node(IrOpKind::PowInt, &[0, 1]),
            node(IrOpKind::Constant, &[1]),
            node(IrOpKind::Mul, &[3, 0]),
            node(IrOpKind::Add, &[2, 4]),
        ]
    );
    assert_eq!(
        program.node(2).map(|node| node.op()),
        Some(IrOpKind::PowInt)
    );
    assert_eq!(
        program.node(2).map(|node| node.operands()),
        Some(&[0, 1][..])
    );
    assert_eq!(program.node(2).map(|node| node.aux()), Some(0));

    let back = IrProgram::deserialize(&compilation.ir_text()).expect("deserialize");
    assert_eq!(back.serialize(), compilation.ir_text());
    assert_eq!(&back, program);

    // Compilation.ToKernel wraps the same program.
    let kernel = to_kernel(&compilation);
    assert_eq!(kernel.ir_text(), compilation.ir_text());
}

// CompilationTests.cs:46 - Kernel_BatchEvaluation_Columnwise
#[test]
fn kernel_batch_evaluation_columnwise() {
    let expression = Sym::add(Sym::pow(Sym::sym("x"), Sym::num(2)), Sym::num(1));
    let kernel = compile_kernel(&expression, &params(&["x"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
const (rat 1 1)\n\
const (rat 2 1)\n\
Constant 0 0\n\
Parameter 0 0\n\
Constant 0 1\n\
PowInt 0 1,2\n\
Add 0 0,3\n";
    assert_eq!(kernel.ir_text(), expected);

    let mut columns: HashMap<String, Vec<Num>> = HashMap::new();
    columns.insert("x".to_string(), vec![num(1), num(2), num(3)]);
    let results = kernel.evaluate_batch(&columns).expect("batch");
    assert_eq!(results.len(), 3);
    assert_same(&results[0], &num(2));
    assert_same(&results[1], &num(5));
    assert_same(&results[2], &num(10));

    let mut mismatched: HashMap<String, Vec<Num>> = HashMap::new();
    mismatched.insert("x".to_string(), vec![num(1), num(2)]);
    mismatched.insert("y".to_string(), vec![num(3)]);
    let error = kernel
        .evaluate_batch(&mismatched)
        .expect_err("column width");
    assert_eq!(error.message, "Column 'y' has 1 lanes; expected 2.");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

// CompilationTests.cs:94 - Kernel_BatchLazySelect_MixedGuards
#[test]
fn kernel_batch_lazy_select_mixed_guards() {
    let piecewise = Sym::piecewise(
        vec![(
            Sym::relation(RelOp::Gt, Sym::sym("x"), Sym::num(0)),
            Sym::div(Sym::num(1), Sym::sym("x")),
        )],
        Sym::neg(Sym::sym("x")),
    );
    let kernel = compile_kernel(&piecewise, &params(&["x"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
const (rat -1 1)\n\
const (rat 0 1)\n\
Constant 0 0\n\
Parameter 0 0\n\
Mul 0 0,1\n\
Constant 0 1\n\
Gt 0 1,3\n\
Reciprocal 0 1\n\
Select 0 4,5,2\n";
    assert_eq!(kernel.ir_text(), expected);

    let mut columns: HashMap<String, Vec<Num>> = HashMap::new();
    columns.insert("x".to_string(), vec![num(2), num(-3), rat(1, 2)]);
    let results = kernel.evaluate_batch(&columns).expect("batch");
    assert_eq!(results.len(), 3);
    assert_same(&results[0], &rat(1, 2));
    assert_same(&results[1], &num(3));
    assert_same(&results[2], &num(2));
}

// CompilationTests.cs:111 - Kernel_BatchMatchesScalarEvaluation
#[test]
fn kernel_batch_matches_scalar_evaluation() {
    let expression = Sym::add(
        Sym::Fun(
            "sin".to_string(),
            vec![Sym::mul(Sym::sym("x"), Sym::sym("y"))],
        ),
        Sym::pow(Sym::sym("x"), Sym::num(2)),
    );
    let kernel = compile_kernel(&expression, &params(&["x", "y"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
param y\n\
const (rat 2 1)\n\
Parameter 0 0\n\
Parameter 0 1\n\
Mul 0 0,1\n\
Sin 0 2\n\
Constant 0 0\n\
PowInt 0 0,4\n\
Add 0 3,5\n";
    assert_eq!(kernel.ir_text(), expected);

    let _scope = Real::with_precision(40, 20);
    let xs = vec![rat(1, 2), rat(-3, 4), rat(7, 5)];
    let ys = vec![rat(2, 3), rat(5, 4), rat(-1, 3)];
    let mut columns: HashMap<String, Vec<Num>> = HashMap::new();
    columns.insert("x".to_string(), xs.clone());
    columns.insert("y".to_string(), ys.clone());
    let batch = kernel.evaluate_batch(&columns).expect("batch");
    assert_eq!(batch.len(), xs.len());
    for lane in 0..xs.len() {
        let scalar = kernel.evaluate(&[xs[lane].clone(), ys[lane].clone()]);
        assert_same(&batch[lane], &scalar.expect("scalar"));
    }
}

// CompilationTests.cs:68 - Kernel_ExactTiers_Preserved
#[test]
fn kernel_exact_tiers_preserved() {
    let expression = Sym::add(
        Sym::mul(Sym::num(2), Sym::pow(Sym::sym("x"), Sym::num(3))),
        Sym::rational(&Int::from_i64(1), &Nat::from_u64(2)),
    );
    let kernel = compile_kernel(&expression, &params(&["x"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
const (rat 1 2)\n\
const (rat 2 1)\n\
const (rat 3 1)\n\
Constant 0 0\n\
Constant 0 1\n\
Parameter 0 0\n\
Constant 0 2\n\
PowInt 0 2,3\n\
Mul 0 1,4\n\
Add 0 0,5\n";
    assert_eq!(kernel.ir_text(), expected);

    let result = kernel.evaluate(&[rat(1, 2)]).expect("evaluate");
    // 2*(1/2)^3 + 1/2 = 3/4, exact rational (the reference asserts NumRat).
    assert!(
        matches!(result, Num::Exact(_)),
        "expected the exact tier, got {result:?}"
    );
    assert_same(&result, &rat(3, 4));
}

// CompilationTests.cs:133 - Kernel_LazySelect_UntakenBranchNeverEvaluates
#[test]
fn kernel_lazy_select_untaken_branch_never_evaluates() {
    let piecewise = Sym::piecewise(
        vec![(
            Sym::relation(RelOp::Eq, Sym::sym("x"), Sym::num(0)),
            Sym::num(1),
        )],
        Sym::div(Sym::num(1), Sym::sym("x")),
    );
    let kernel = compile_kernel(&piecewise, &params(&["x"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
const (rat 0 1)\n\
const (rat 1 1)\n\
Parameter 0 0\n\
Reciprocal 0 0\n\
Constant 0 0\n\
Eq 0 0,2\n\
Constant 0 1\n\
Select 0 3,4,1\n";
    assert_eq!(kernel.ir_text(), expected);

    // The otherwise branch 1/x would divide by zero if Select were strict.
    let at_zero = kernel.evaluate(&[num(0)]).expect("piecewise at zero");
    assert_same(&at_zero, &num(1));
    let at_two = kernel.evaluate(&[num(2)]).expect("piecewise at two");
    assert_same(&at_two, &rat(1, 2));
}

// CompilationTests.cs:31 - Kernel_NamedAndPositional_Evaluate
#[test]
fn kernel_named_and_positional_evaluate() {
    let expression = Sym::add(Sym::mul(Sym::sym("x"), Sym::sym("y")), Sym::num(1));
    let kernel = compile_kernel(&expression, &params(&["x", "y"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
param y\n\
const (rat 1 1)\n\
Constant 0 0\n\
Parameter 0 0\n\
Parameter 0 1\n\
Mul 0 1,2\n\
Add 0 0,3\n";
    assert_eq!(kernel.ir_text(), expected);

    assert_same(
        &kernel.evaluate(&[num(2), num(3)]).expect("positional"),
        &num(7),
    );
    let mut named: HashMap<String, Num> = HashMap::new();
    named.insert("x".to_string(), num(2));
    named.insert("y".to_string(), num(3));
    assert_same(&kernel.evaluate_named(&named).expect("named"), &num(7));
    let error = kernel.evaluate(&[num(1)]).expect_err("arity");
    assert_eq!(error.message, "Kernel expects 2 argument(s), got 1.");
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

// CompilationTests.cs:81 - Kernel_Transcendental_ApproximateTier
#[test]
fn kernel_transcendental_approximate_tier() {
    let expression = Sym::Fun("exp".to_string(), vec![Sym::sym("x")]);
    let kernel = compile_kernel(&expression, &params(&["x"])).expect("kernel");
    let expected = "#!mathir 2\n\
param x\n\
Parameter 0 0\n\
Exp 0 0\n";
    assert_eq!(kernel.ir_text(), expected);

    let _scope = Real::with_precision(40, 20);
    let result = kernel.evaluate(&[num(0)]).expect("evaluate");
    // NumReal is the approximate carrier even when its digits are exactly 1.
    assert!(
        matches!(result, Num::Real(_)),
        "expected the approximate tier, got {result:?}"
    );
    assert_same(&result, &num(1));
    assert_eq!(kernel.program().nodes().len(), 2);
    assert_eq!(kernel.program().root(), Some(1));
}

// The reference's Deserialize format-version gate (Ir.cs:48-89).
#[test]
fn deserialize_gates_the_format_version() {
    let error = IrProgram::deserialize("#!mathir 1\nparam x\n").expect_err("version gate");
    assert_eq!(
        error.message,
        "Unsupported MathIR format version '1' (expected 2)."
    );
    assert_eq!(error.code, "InvalidInput");
    assert_eq!(error.category, "ParseError");
    let error = IrProgram::deserialize("param x\nAdd 0 0\n").expect_err("arity");
    assert_eq!(
        error.message,
        "IR validation: node 0 (Add) has 1 operands; expected 2."
    );
}

// IrTyping.Validate's structural and type refusals (Typing.cs:44-93).
#[test]
fn validation_reports_pool_parameter_and_guard_diagnostics() {
    // The pool check runs after the operand-range check, so reaching it needs a node count
    // larger than the index (the reference's Validate order).
    let error = IrProgram::deserialize(
        "#!mathir 2\nparam x\nParameter 0 0\nParameter 0 0\nParameter 0 0\nParameter 0 0\nParameter 0 0\nParameter 0 0\nConstant 0 5",
    )
    .expect_err("pool");
    assert_eq!(
        error.message,
        "IR validation: Constant node 6 references out-of-range pool index 5."
    );
    let error = IrProgram::deserialize("#!mathir 2\nParameter 0 0").expect_err("parameter");
    assert_eq!(
        error.message,
        "IR validation: Parameter node 0 references out-of-range parameter index 0."
    );
    let error = IrProgram::deserialize("#!mathir 2\nparam x\nParameter 0 0\nSelect 0 0,0,0")
        .expect_err("guard");
    assert_eq!(
        error.message,
        "IR validation: Select node 1 requires a Bool guard, got unconstrained."
    );
    let error = IrProgram::deserialize(
        "#!mathir 2\nconst (cplx 1 0)\nConstant 0 0\nConstant 0 0\nLt 0 1,1",
    )
    .expect_err("ordered comparison");
    assert_eq!(
        error.message,
        "IR validation: ordered comparison Lt requires real operands, node 2 operand 1 is Complex."
    );
}

#[test]
fn lowering_refuses_what_the_reference_refuses() {
    let error = IrProgram::lower(
        &Sym::Fun("frobnicate".to_string(), vec![Sym::sym("x")]),
        &params(&["x"]),
    )
    .expect_err("unknown function");
    assert_eq!(
        error.message,
        "Function 'frobnicate' cannot be lowered to MathIR."
    );
    let error = IrProgram::lower(&Sym::sym("z"), &params(&["x"])).expect_err("free symbol");
    assert_eq!(
        error.message,
        "Node kind Symbol cannot be lowered to MathIR."
    );
    let error = IrProgram::lower(
        &Sym::relation(RelOp::Eq, Sym::sym("x"), Sym::num(0)),
        &params(&["x"]),
    )
    .expect_err("relation");
    assert_eq!(
        error.message,
        "Node kind Relation cannot be lowered to MathIR."
    );
}

#[test]
fn compile_requires_at_least_one_parameter() {
    let error = compile(&Sym::num(1), &[]).expect_err("no parameters");
    assert_eq!(
        error.message,
        "Compilation requires at least one parameter. (Parameter 'parameters')"
    );
    assert_eq!(error.code, "InvalidArgument");
    assert_eq!(error.category, "TypeMismatch");
}

// IrTyping.Infer: constants determine the tier, a parameter stays unconstrained.
#[test]
fn typing_infers_the_documented_tiers() {
    let with_rational = Sym::add(
        Sym::sym("x"),
        Sym::rational(&Int::from_i64(1), &Nat::from_u64(2)),
    );
    let compilation = compile(&with_rational, &params(&["x"])).expect("compile");
    let types = IrTyping::infer(compilation.program());
    assert_eq!(
        types,
        vec![Some(IrType::Rational), None, Some(IrType::Rational)]
    );
    assert_eq!(compilation.result_type(), Some(IrType::Rational));

    let with_integer = Sym::add(Sym::sym("x"), Sym::num(1));
    let compilation = compile(&with_integer, &params(&["x"])).expect("compile");
    assert_eq!(compilation.result_type(), Some(IrType::Integer));

    let bare_parameter = compile(&Sym::sym("x"), &params(&["x"])).expect("compile");
    assert_eq!(bare_parameter.result_type(), None);
}
