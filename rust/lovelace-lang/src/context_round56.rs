//! Round-56 mirrors of the eight census rows the round-50 re-sweep grouped under
//! "thread-scoped ExprContext/assumption scopes".
//!
//! The C# sources are `Lovelace.Symbolics.Tests/ConcurrencyTests.cs` and
//! `Lovelace.Symbolics.Tests/AssumptionScopeIsolationTests.cs`; every test names its exact census
//! id in its doc comment. The production surface is `crate::context`.

use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Barrier;
use std::thread;

use lovelace_symbolics::assumptions::{Assumption, AssumptionSet, Domain, Tristate};
use lovelace_symbolics::{Expr, RelOp};

use crate::context::Context;

fn relation(symbol: &str, op: RelOp) -> Assumption {
    Assumption::SymbolRelation(symbol.to_string(), op, Expr::num(0))
}

fn one(atom: Assumption) -> AssumptionSet {
    AssumptionSet::empty().add(atom).expect("consistent atom")
}

fn positive() -> AssumptionSet {
    one(relation("x", RelOp::Gt))
}

fn negative() -> AssumptionSet {
    one(relation("x", RelOp::Lt))
}

fn real_domain(symbol: &str) -> AssumptionSet {
    one(Assumption::SymbolDomain(symbol.to_string(), Domain::Real))
}

/// `Lovelace.Symbolics.Tests/.../AssumptionScopeIsolationTests/NestedScopes_RestoreTheEnclosingScopeOnDispose#0`
///
/// The C# row installs an outer `x > 0` scope, then an inner `x < 0` scope, and asserts that the
/// inner set is visible inside, that the outer set is visible again after the inner Dispose, and
/// that the base set is Empty after both scopes end.
#[test]
fn nested_scopes_restore_the_enclosing_scope_on_dispose() {
    let ctx = Context::new();
    let ctx = &ctx;
    {
        let outer = ctx.with_assumptions(positive());
        assert!(ctx.assumption_scope_is_active());
        assert_eq!(
            ctx.assumptions().ask(&relation("x", RelOp::Gt)),
            Tristate::True
        );
        {
            let _inner = ctx.with_assumptions(negative());
            assert_eq!(
                ctx.assumptions().ask(&relation("x", RelOp::Lt)),
                Tristate::True
            );
            // The inner set PROVES the negation of the outer query.
            assert_eq!(
                ctx.assumptions().ask(&relation("x", RelOp::Gt)),
                Tristate::False
            );
        }
        // The inner scope ended: the enclosing set is visible again.
        assert_eq!(
            ctx.assumptions().ask(&relation("x", RelOp::Gt)),
            Tristate::True
        );
        assert_eq!(ctx.assumptions().atoms().len(), 1);
        drop(outer);
    }
    assert_eq!(ctx.assumptions(), AssumptionSet::empty());
    assert!(!ctx.assumption_scope_is_active());
}

/// `Lovelace.Symbolics.Tests/.../AssumptionScopeIsolationTests/TwoConcurrentScopes_AreIsolated#0`
///
/// Both scopes are open simultaneously. Each thread blocks until the other has entered its scope,
/// then asks the same question; the positive flow proves it, the negative flow refutes it. The
/// base set is untouched because a scope is not a mutation of shared state.
#[test]
fn two_concurrent_scopes_are_isolated() {
    let ctx = Context::new();
    let ctx = &ctx;
    let barrier = Barrier::new(2);
    let (positive_result, negative_result) = thread::scope(|scope| {
        let positive_thread = scope.spawn(|| {
            let _scope = ctx.with_assumptions(positive());
            assert!(ctx.assumption_scope_is_active());
            barrier.wait();
            ctx.assumptions().ask(&relation("x", RelOp::Gt))
        });
        let negative_thread = scope.spawn(|| {
            let _scope = ctx.with_assumptions(negative());
            assert!(ctx.assumption_scope_is_active());
            barrier.wait();
            ctx.assumptions().ask(&relation("x", RelOp::Gt))
        });
        (
            positive_thread.join().expect("positive scope thread"),
            negative_thread.join().expect("negative scope thread"),
        )
    });
    assert_eq!(positive_result, Tristate::True);
    // Under the old shared-field implementation one flow saw the other's set.
    assert_eq!(negative_result, Tristate::False);
    assert_eq!(ctx.assumptions(), AssumptionSet::empty());
    assert!(!ctx.assumption_scope_is_active());
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/AssumptionScopes_RestoreOnDispose#0`
///
/// The C# row opens a scope on a worker task, simplifies `sqrt(x^2)` under `x in Real`, sees
/// `abs(x)`, and then asserts that work after the scope sees the baseline and an empty set again.
#[test]
fn assumption_scopes_restore_on_dispose() {
    let ctx = Context::new();
    let ctx = &ctx;
    let x = ctx.symbol("x");
    let expression = Expr::Fun(
        "sqrt".to_string(),
        vec![Expr::pow(Expr::sym(x.name()), Expr::num(2))],
    );
    let baseline = ctx.simplify(expression.clone()).canonical();
    // The reference builds this expression as Power(Power(x, 2), 1/2); the Rust symbolic
    // constructor represents the principal square root as the sqrt function, and its canonical
    // printer emits the same half-power form the reference prints.
    assert_eq!(baseline, "(pow (pow (sym x) (rat 2 1)) (rat 1 2))");

    let scoped = thread::scope(|scope| {
        scope
            .spawn(|| {
                let _scope = ctx.with_assumptions(real_domain("x"));
                ctx.simplify(expression.clone()).canonical()
            })
            .join()
            .expect("scope thread")
    });
    assert_eq!(scoped, "(fn abs (sym x))");

    // After dispose the scope is gone: unrelated work sees the original assumptions again.
    assert_eq!(ctx.simplify(expression).canonical(), baseline);
    assert!(ctx.assumptions().atoms().is_empty());
    assert_eq!(
        ctx.symbol_by_id(x.id())
            .map(|symbol| symbol.name().to_string()),
        Some("x".to_string())
    );
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/ParallelCompilation_AgreesWithSerial#0`
///
/// The C# row lowers one polynomial over `x` on 16 parallel tasks and asserts every serialized
/// program equals the serial one.
#[test]
fn parallel_compilation_agrees_with_serial() {
    let ctx = Context::new();
    let ctx = &ctx;
    let x = Expr::sym("x");
    let expression = Expr::add(
        Expr::pow(x.clone(), Expr::num(5)),
        Expr::add(
            Expr::mul(Expr::num(2), Expr::pow(x.clone(), Expr::num(4))),
            Expr::mul(Expr::num(3), Expr::pow(x, Expr::num(3))),
        ),
    );
    let parameters = vec!["x".to_string()];
    let serial = ctx
        .lower(&expression, &parameters)
        .expect("serial lowering")
        .serialize();
    let parallel: Vec<String> = thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..16 {
            handles.push(scope.spawn(|| {
                ctx.lower(&expression, &parameters)
                    .expect("parallel lowering")
                    .serialize()
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("lowering thread"))
            .collect()
    });
    for program in &parallel {
        assert_eq!(*program, serial);
    }
    assert_eq!(parallel.len(), 16);
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/ParallelFunctionCreation_NoDuplicateIds#0`
///
/// Eight tasks intern 250 distinct function names each; every returned id is unique and every id
/// resolves back to its name.
#[test]
fn parallel_function_creation_no_duplicate_ids() {
    let ctx = Context::new();
    let ctx = &ctx;
    let all = thread::scope(|scope| {
        let mut handles = Vec::new();
        for task in 0..8 {
            handles.push(scope.spawn(move || {
                let names: Vec<String> = (0..250).map(|i| format!("fn{task}_{i}")).collect();
                for name in &names {
                    ctx.function(name);
                }
                names
                    .iter()
                    .map(|name| ctx.function(name))
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("function thread"))
            .collect::<Vec<_>>()
    });
    let ids: HashSet<usize> = all.iter().map(|function| function.id()).collect();
    assert_eq!(all.len(), 8 * 250);
    assert_eq!(ids.len(), all.len());
    for function in &all {
        assert_eq!(
            ctx.function_by_id(function.id())
                .expect("function id round-trips")
                .name(),
            function.name()
        );
    }
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/ParallelParse_AgreesWithSerial#0`
///
/// The C# row builds 40 canonical texts, parses them serially and then in parallel on the same
/// context, and asserts the canonical results are equal. The Rust mirror also checks that parsing
/// interned the shared `x` identity into the context.
#[test]
fn parallel_parse_agrees_with_serial() {
    let ctx = Context::new();
    let ctx = &ctx;
    let texts: Vec<String> = (0..40)
        .map(|i| {
            Expr::add(
                Expr::pow(Expr::sym("x"), Expr::num(i % 9 + 1)),
                Expr::num(i + 1),
            )
            .canonical()
        })
        .collect();
    let serial: Vec<String> = texts
        .iter()
        .map(|text| {
            ctx.canonical_parse(text)
                .expect("serial canonical parse")
                .canonical()
        })
        .collect();
    let parallel: Vec<String> = thread::scope(|scope| {
        let mut handles = Vec::new();
        for text in &texts {
            let text = text.clone();
            handles.push(scope.spawn(move || {
                ctx.canonical_parse(&text)
                    .expect("parallel canonical parse")
                    .canonical()
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("parse thread"))
            .collect()
    });
    assert_eq!(serial, parallel);
    assert_eq!(ctx.symbol("x").name(), "x");
    assert!(ctx.symbol_by_id(ctx.symbol("x").id()).is_some());
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/ParallelSimplifyDiff_AgreeWithSerial#0`
///
/// The C# row differentiates and simplifies 40 expressions serially and then in parallel on the
/// same context, asserting the canonical renderings agree.
#[test]
fn parallel_simplify_diff_agree_with_serial() {
    let ctx = Context::new();
    let ctx = &ctx;
    let x = Expr::sym("x");
    let expressions: Vec<Expr> = (0..40)
        .map(|i| {
            Expr::pow(
                Expr::Fun("sin".to_string(), vec![Expr::pow(x.clone(), Expr::num(2))]),
                Expr::num(i % 7 + 1),
            )
        })
        .collect();
    let serial_diff: Vec<String> = expressions
        .iter()
        .map(|expression| ctx.diff(expression, "x").canonical())
        .collect();
    let serial_simplify: Vec<String> = expressions
        .iter()
        .map(|expression| ctx.simplify(expression.clone()).canonical())
        .collect();
    let (parallel_diff, parallel_simplify) = thread::scope(|scope| {
        let mut diff_handles = Vec::new();
        let mut simplify_handles = Vec::new();
        for expression in &expressions {
            let expression = expression.clone();
            diff_handles.push(scope.spawn({
                let expression = expression.clone();
                move || ctx.diff(&expression, "x").canonical()
            }));
            simplify_handles.push(scope.spawn(move || ctx.simplify(expression).canonical()));
        }
        let diffs = diff_handles
            .into_iter()
            .map(|handle| handle.join().expect("diff thread"))
            .collect::<Vec<_>>();
        let simplifies = simplify_handles
            .into_iter()
            .map(|handle| handle.join().expect("simplify thread"))
            .collect::<Vec<_>>();
        (diffs, simplifies)
    });
    assert_eq!(serial_diff, parallel_diff);
    assert_eq!(serial_simplify, parallel_simplify);
}

/// `Lovelace.Symbolics.Tests/.../ConcurrencyTests/ParallelSymbolCreation_NoDuplicateIds#0`
///
/// Eight tasks intern 250 distinct symbol names each; every returned id is unique, every id
/// resolves back to its name, and equality remains by canonical name.
#[test]
fn parallel_symbol_creation_no_duplicate_ids() {
    let ctx = Context::new();
    let ctx = &ctx;
    let all = thread::scope(|scope| {
        let mut handles = Vec::new();
        for task in 0..8 {
            handles.push(scope.spawn(move || {
                let names: Vec<String> = (0..250).map(|i| format!("x{task}_{i}")).collect();
                for name in &names {
                    ctx.symbol(name);
                }
                names
                    .iter()
                    .map(|name| ctx.symbol(name))
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("symbol thread"))
            .collect::<Vec<_>>()
    });
    let ids: HashSet<usize> = all.iter().map(|symbol| symbol.id()).collect();
    assert_eq!(all.len(), 8 * 250);
    assert_eq!(ids.len(), all.len());
    for symbol in &all {
        assert_eq!(
            ctx.symbol_by_id(symbol.id())
                .expect("symbol id round-trips")
                .name(),
            symbol.name()
        );
    }
    let first = ctx.symbol("x0_0");
    let second = ctx.symbol("x0_0");
    assert_eq!(first, second);
    assert_eq!(first.id(), second.id());
}

/// The reference's `Assumptions` setter updates the BASE set; an active flow-local scope still
/// overrides it until that scope ends, at which point the new base set is visible.
#[test]
fn the_base_setter_does_not_override_an_active_scope() {
    let ctx = Context::new();
    let ctx = &ctx;
    let scope = ctx.with_assumptions(positive());
    ctx.set_assumptions(negative());
    assert_eq!(
        ctx.assumptions().ask(&relation("x", RelOp::Gt)),
        Tristate::True
    );
    drop(scope);
    // The scope ended, so the setter's base set is now the active one.
    assert_eq!(
        ctx.assumptions().ask(&relation("x", RelOp::Lt)),
        Tristate::True
    );
    assert_eq!(
        ctx.assumptions().ask(&relation("x", RelOp::Gt)),
        Tristate::False
    );
    assert!(!ctx.assumption_scope_is_active());
}

/// Objective requirement the C# rows do not spell out: a scope is a using statement in C#, so its
/// Dispose runs while an exception unwinds; Rust's RAII guard must do the same during a panic.
#[test]
fn a_scope_restores_the_base_set_when_the_thread_unwinds() {
    let ctx = Context::new();
    let ctx = &ctx;
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _scope = ctx.with_assumptions(positive());
        assert_eq!(
            ctx.assumptions().ask(&relation("x", RelOp::Gt)),
            Tristate::True
        );
        panic!("unwind through the scope guard");
    }));
    assert!(result.is_err());
    assert_eq!(ctx.assumptions(), AssumptionSet::empty());
    assert!(!ctx.assumption_scope_is_active());
}

/// A scope is visible only on the thread that opened it: an idle thread reads the base set while
/// the opener is inside its scope.
#[test]
fn a_scope_is_invisible_to_another_thread() {
    let ctx = Context::new();
    let ctx = &ctx;
    let entered = Barrier::new(2);
    thread::scope(|scope| {
        let opener = scope.spawn(|| {
            let _scope = ctx.with_assumptions(positive());
            entered.wait();
            assert_eq!(
                ctx.assumptions().ask(&relation("x", RelOp::Gt)),
                Tristate::True
            );
        });
        let observer = scope.spawn(|| {
            entered.wait();
            assert_eq!(ctx.assumptions(), AssumptionSet::empty());
            assert!(!ctx.assumption_scope_is_active());
        });
        opener.join().expect("opener thread");
        observer.join().expect("observer thread");
    });
    assert_eq!(ctx.assumptions(), AssumptionSet::empty());
}
