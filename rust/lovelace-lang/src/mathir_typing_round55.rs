//! Round 55: the MathIrTypingTests census rows, mirrored over `lovelace_lang::mathir`.
//!
//! Sources: Lovelace.Symbolics.Tests/MathIrTypingTests.cs (lines 21-121) and
//! Lovelace.MathIR/Typing.cs. All seven rows are mirrored here; each drives the public surface
//! directly (not the language's `evalir` builtin) with the C# case's own program and asserts its
//! concrete result:
//!
//! * `Infer_TypesForLoweredExpressions` — the inferred tier of each lowered root: Integer for
//!   `x^2 + 1`, Rational for `x / 2`, Real for `exp(2)`, unconstrained for `exp(x)`, Bool at the
//!   `Gt` guard a piecewise emits, and Complex for `(1 + i) * x`.
//! * `Validate_AcceptsLoweredCorpus` — all five corpus expressions lower and validate, and the
//!   text that validates is exactly the text Deserialize accepts.
//! * the four `Validate_Rejects*` rows — the reference's first-refusal sentence, its code and
//!   category, and the typed fact that forces the refusal.
//! * `Deserialize_RejectsMalformedProgram` — `IrProgram::deserialize` refuses the C# text, and a
//!   line the parser cannot read is refused before validation with the parse sentence.
//!
//! One representation difference from the C# rows, none of which changes an expected value:
//! C# `IrProgram` exposes three mutable lists, so its Validate rows build a defective program node
//! by node; the Rust `IrProgram` keeps its constant pool, parameter list and node list private and
//! offers no in-memory builder. `IrTyping::validate` is therefore handed the same defective program
//! through the only constructor that can produce one — `IrProgram::deserialize`, whose documented
//! last step IS `IrTyping::validate` (see `mathir.rs`). These texts parse cleanly, so the refusals
//! below are validate's own first sentence, exactly what the C# `Assert.Throws<InvalidOperationException>`
//! pins; the parser's separate refusal is asserted on a genuinely unreadable line.

use lovelace_symbolics::{Expr as Sym, RelOp};

use crate::mathir::{IrOpKind, IrProgram, IrType, IrTyping};

fn params(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn exp_of(argument: Sym) -> Sym {
    Sym::Fun("exp".to_string(), vec![argument])
}

// MathIrTypingTests.cs:21 - Infer_TypesForLoweredExpressions
#[test]
fn infer_types_for_lowered_expressions() {
    let x = params(&["x"]);

    // x^2 + 1: the parameter is unconstrained (the reference's null) and the Integer constant
    // determines the sum's tier.
    let p1 = IrProgram::lower(
        &Sym::add(Sym::pow(Sym::sym("x"), Sym::num(2)), Sym::num(1)),
        &x,
    )
    .expect("lower x^2 + 1");
    assert_eq!(
        IrTyping::infer(&p1)[p1.root().expect("root")],
        Some(IrType::Integer)
    );

    // x / 2: the reference's Divide IS Multiply(x, Power(2, -1)), and Power folds that numeric
    // constant pair to (rat 1 2), so the quotient is a product over a Rational constant.
    // Sym::div is that same constructor (Mul over Pow); the expression type's explicit Div leaf,
    // which the reference does not have, is not what this case builds.
    let p2 = IrProgram::lower(&Sym::div(Sym::sym("x"), Sym::num(2)), &x).expect("lower x / 2");
    assert_eq!(
        IrTyping::infer(&p2)[p2.root().expect("root")],
        Some(IrType::Rational)
    );

    // exp of a typed (integer) argument is Real; exp of an unconstrained parameter is unknown.
    let p3 = IrProgram::lower(&exp_of(Sym::num(2)), &x).expect("lower exp(2)");
    assert_eq!(
        IrTyping::infer(&p3)[p3.root().expect("root")],
        Some(IrType::Real)
    );
    let p3u = IrProgram::lower(&exp_of(Sym::sym("x")), &x).expect("lower exp(x)");
    assert_eq!(IrTyping::infer(&p3u)[p3u.root().expect("root")], None);

    // A relation lowers as a piecewise guard: the Gt node types Bool, and the Select the
    // piecewise wraps around it promotes the taken branch (unconstrained x) and the otherwise
    // value (Integer 0) to Integer.
    let piecewise = Sym::piecewise(
        vec![(
            Sym::relation(RelOp::Gt, Sym::sym("x"), Sym::num(0)),
            Sym::sym("x"),
        )],
        Sym::num(0),
    );
    let p4 = IrProgram::lower(&piecewise, &x).expect("lower piecewise");
    let t4 = IrTyping::infer(&p4);
    let guard = p4
        .nodes()
        .iter()
        .position(|node| node.op() == IrOpKind::Gt)
        .expect("the piecewise lowers its guard as a Gt node");
    assert_eq!(t4[guard], Some(IrType::Bool));
    assert_eq!(t4[p4.root().expect("root")], Some(IrType::Integer));

    // (1 + i) * x: the named constant i is Complex, so the whole product is Complex.
    let complex = Sym::mul(
        Sym::add(Sym::num(1), Sym::Fun("i".to_string(), Vec::new())),
        Sym::sym("x"),
    );
    let p5 = IrProgram::lower(&complex, &x).expect("lower (1 + i) * x");
    assert_eq!(
        IrTyping::infer(&p5)[p5.root().expect("root")],
        Some(IrType::Complex)
    );
}

// MathIrTypingTests.cs:51 - Validate_AcceptsLoweredCorpus
#[test]
fn validate_accepts_lowered_corpus() {
    let parameters = params(&["x", "y"]);
    let corpus = [
        Sym::add(Sym::pow(Sym::sym("x"), Sym::num(2)), Sym::num(1)),
        Sym::mul(
            Sym::Fun("sin".to_string(), vec![Sym::sym("x")]),
            exp_of(Sym::sym("x")),
        ),
        Sym::piecewise(
            vec![(
                Sym::relation(RelOp::Gt, Sym::sym("x"), Sym::num(0)),
                Sym::sym("x"),
            )],
            Sym::neg(Sym::sym("x")),
        ),
        Sym::div(Sym::sym("x"), Sym::add(Sym::sym("x"), Sym::num(1))),
        Sym::pow(Sym::sym("x"), Sym::sym("y")),
    ];
    for expression in &corpus {
        let program = IrProgram::lower(expression, &parameters).expect("the corpus lowers");
        assert!(program.root().is_some());
        IrTyping::validate(&program).expect("the lowered corpus validates");
        // What validate accepts, Deserialize accepts too: the two entry points agree on the
        // serialized text.
        let text = program.serialize();
        assert_eq!(
            IrProgram::deserialize(&text)
                .expect("round trip")
                .serialize(),
            text
        );
    }
}

// MathIrTypingTests.cs:72 - Validate_RejectsOutOfRangeOperand
#[test]
fn validate_rejects_out_of_range_operand() {
    // The C# program: Parameters ["x"]; Constants ["(rat 1 1)"]; Parameter 0, Constant 0,
    // Add 0 42. Add's second operand names a node that does not exist.
    let error = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,42\n",
    )
    .expect_err("out-of-range operand");
    assert_eq!(
        error.message,
        "IR validation: node 2 (Add) references out-of-range operand 42."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");

    // The same program with the operand in range is exactly what validate accepts.
    let accepted = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,1\n",
    )
    .expect("in-range operand");
    IrTyping::validate(&accepted).expect("validated");
}

// MathIrTypingTests.cs:84 - Validate_RejectsNonBoolSelectGuard
#[test]
fn validate_rejects_non_bool_select_guard() {
    // Parameters ["x"]; Constant (rat 1 1); Parameter 0, Constant 0, Add 0 0,1, Select 2 0 1.
    let error = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,1\nSelect 0 2,0,1\n",
    )
    .expect_err("non-Bool Select guard");
    assert_eq!(
        error.message,
        "IR validation: Select node 3 requires a Bool guard, got Integer."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");

    // The guard's inferred type is why: Add of the unconstrained parameter and the Integer
    // constant is Integer, never Bool. The same prefix without the Select validates.
    let prefix = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,1\n",
    )
    .expect("guard prefix");
    IrTyping::validate(&prefix).expect("prefix validates");
    assert_eq!(IrTyping::infer(&prefix)[2], Some(IrType::Integer));
}

// MathIrTypingTests.cs:97 - Validate_RejectsOrderedComparisonOfComplex
#[test]
fn validate_rejects_ordered_comparison_of_complex() {
    // Parameters ["x"]; Constant (cplx 1 1 1 1); Constant 0, Parameter 0, Lt 0 0,1.
    let error = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (cplx 1 1 1 1)\nConstant 0 0\nParameter 0 0\nLt 0 0,1\n",
    )
    .expect_err("ordered comparison of Complex");
    assert_eq!(
        error.message,
        "IR validation: ordered comparison Lt requires real operands, node 2 operand 0 is Complex."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");

    // The pool entry is what types as Complex; without the comparison the program validates.
    let prefix = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (cplx 1 1 1 1)\nConstant 0 0\nParameter 0 0\n",
    )
    .expect("complex prefix");
    IrTyping::validate(&prefix).expect("prefix validates");
    assert_eq!(IrTyping::infer(&prefix)[0], Some(IrType::Complex));
}

// MathIrTypingTests.cs:109 - Validate_RejectsWrongArity
#[test]
fn validate_rejects_wrong_arity() {
    // Parameters ["x"]; Parameter 0, Add 0 0 0 — an Add with three operands.
    let error = IrProgram::deserialize("#!mathir 2\nparam x\nParameter 0 0\nAdd 0 0,0,0\n")
        .expect_err("wrong arity");
    assert_eq!(
        error.message,
        "IR validation: node 1 (Add) has 3 operands; expected 2."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");
    assert_eq!(IrOpKind::Add.arity(), 2);

    // The same program with the binary Add validates.
    let accepted = IrProgram::deserialize("#!mathir 2\nparam x\nParameter 0 0\nAdd 0 0,0\n")
        .expect("binary Add");
    IrTyping::validate(&accepted).expect("validated");
}

// MathIrTypingTests.cs:119 - Deserialize_RejectsMalformedProgram
#[test]
fn deserialize_rejects_malformed_program() {
    // The C# text is syntactically well formed; its Add names an operand that does not exist, so
    // Deserialize's final validation step is what refuses it.
    let error = IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,42\n",
    )
    .expect_err("malformed program");
    assert_eq!(
        error.message,
        "IR validation: node 2 (Add) references out-of-range operand 42."
    );
    assert_eq!(error.code, "InvalidOperation");
    assert_eq!(error.category, "DomainError");

    // A line the parser cannot read is refused before validation, with the parse sentence.
    let error =
        IrProgram::deserialize("#!mathir 2\nparam x\nAddish 0 0\n").expect_err("unknown op");
    assert_eq!(error.message, "Malformed IR line: Addish 0 0");
    assert_eq!(error.code, "InvalidInput");
    assert_eq!(error.category, "ParseError");

    // And the well-formed program deserializes.
    IrProgram::deserialize(
        "#!mathir 2\nparam x\nconst (rat 1 1)\nParameter 0 0\nConstant 0 0\nAdd 0 0,1\n",
    )
    .expect("well-formed program");
}
