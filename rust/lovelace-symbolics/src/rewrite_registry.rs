//! The reference rewrite-rule registry: the nine shipped rules, their patterns,
//! applicability predicates, classifications, declared conditions and ordered application.
//!
//! The reference's symbolic layer keeps the registry in `Simplify.BuildRegistry` and exposes it
//! only through test hooks (`RulesForTesting` / `ShippedRuleIds`). The language-layer simplifier
//! implements the same rewrites by hand, so nothing in this crate could observe the rule list
//! itself: its ids, its pattern shapes, which rule runs before which, what a rule declares as a
//! side condition, or what applying one named rule does. This module is that additive public
//! surface. It mirrors the reference engine's protocol exactly:
//!
//! * a rule is a stable id, a group, a [`Pattern`], an applicability predicate, a replacement
//!   builder, a [`RuleClassification`] and optional declared conditions;
//! * a mismatch is a VALUE ([`RuleApplicability::NotApplicable`] / [`RuleApplicability::Unknown`]),
//!   never exception-based control flow; only a declared
//!   [`RuleEvaluationException`] from an applicability probe is caught;
//! * the engine rewrites bottom-up in canonical child order and tries rules in registration order,
//!   first applicable rule wins, and the rewritten node is re-examined to a fixed point;
//! * the budget counts APPLIED rewrites (not matching attempts) and the condition builder is
//!   consulted even when tracing is disabled, because the conditions are semantics and the trace
//!   is provenance.
//!
//! The nine shipped rules are the reference's own ids, groups and pattern shapes. They are data:
//! constructing the registry and applying it does not touch the language-layer simplifier, does
//! not route through it, and changes no existing simplification result. [`transform`] is the
//! reference `Simplify.Transform` protocol over that registry (five phase groups in the order
//! trig, power, rat, logexp, abs), included so the budget/trace/condition behaviour can be
//! observed directly; the production simplification path remains the language engine's.
//!
//! The one deliberate representation difference is the error channel: C# throws a
//! [`RuleEvaluationException`] for the declared-undecidable state and lets every other exception
//! propagate. Rust has no exceptions, so an applicability probe returns
//! [`Result<RuleApplicability, RuleError>`](RuleError); the engine catches exactly the
//! [`RuleEvaluationException`] payload and returns every other error to the caller.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::assumptions::{Assumption, AssumptionSet, Domain, SymbolPredicate, Tristate};
use crate::rational_functions::non_zero_condition;
use crate::{Expr, Rat};

/// The failure channel of an applicability probe.
///
/// A probe returns `Err` only for one of two states, and the distinction is the whole protocol:
/// a [`RuleEvaluationException`] says the rule itself has declared that it cannot decide, which the
/// engine records as [`RuleApplicability::Unknown`] and keeps walking; any other error is a defect
/// in the rule, which must propagate instead of being folded into "not applicable".
pub type RuleError = Box<dyn Error + Send + Sync>;

/// The reference's explicit rule-applicability protocol. A mismatch is a value, never an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleApplicability {
    /// The pattern or precondition does not hold for this node.
    NotApplicable,
    /// The rule holds unconditionally for this node.
    Applicable,
    /// The rule holds only under the conditions its condition builder reports.
    ApplicableWithConditions,
    /// Applicability cannot be decided. Unknown never licenses a rewrite in any mode.
    Unknown,
}

impl RuleApplicability {
    /// The kernel spelling the reference enum publishes (`NotApplicable`, ...).
    pub fn name(self) -> &'static str {
        match self {
            RuleApplicability::NotApplicable => "NotApplicable",
            RuleApplicability::Applicable => "Applicable",
            RuleApplicability::ApplicableWithConditions => "ApplicableWithConditions",
            RuleApplicability::Unknown => "Unknown",
        }
    }
}

/// The reference's semantic classification of a rewrite rule.
///
/// `Universal` rules are valid over the represented domain with no side conditions;
/// `Conditional` rules fire only when their preconditions are proven; `DomainSpecific` rules
/// additionally require a declared domain assumption; `Approximate` rules change values within a
/// tolerance; `OptimizationOnly` rules may change evaluation order or floating-point error without
/// changing the intended algorithm class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleClassification {
    /// Mathematically valid with no side conditions.
    Universal,
    /// Valid only when its declared side conditions hold.
    Conditional,
    /// Valid only over the declared domain.
    DomainSpecific,
    /// Changes values within a tolerance and must be surfaced as approximate.
    Approximate,
    /// Changes evaluation order or floating-point error, not the algorithm class.
    OptimizationOnly,
}

impl RuleClassification {
    /// The kernel spelling the reference enum publishes (`Universal`, ...).
    pub fn name(self) -> &'static str {
        match self {
            RuleClassification::Universal => "Universal",
            RuleClassification::Conditional => "Conditional",
            RuleClassification::DomainSpecific => "DomainSpecific",
            RuleClassification::Approximate => "Approximate",
            RuleClassification::OptimizationOnly => "OptimizationOnly",
        }
    }
}

/// The one expected exceptional state of an applicability probe: the rule deliberately handles a
/// state as "cannot decide". The engine catches this type and only this type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleEvaluationException {
    /// The rule's explanation, recorded on the attempt diagnostic.
    pub message: String,
}

impl RuleEvaluationException {
    /// Builds the declared-undecidable state with the given explanation.
    pub fn new(message: impl Into<String>) -> RuleEvaluationException {
        RuleEvaluationException {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuleEvaluationException {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuleEvaluationException {}

/// One pattern shape. Matching binds single nodes ([`Pattern::Wild`]) and 0..n contiguous siblings
/// inside an n-ary node ([`Pattern::SequenceWild`]); every other variant matches structurally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Matches exactly this node by structural equality.
    Literal(Expr),
    /// Matches any single node and binds it to the name (the first binding wins).
    Wild(String),
    /// Matches 0..n contiguous siblings inside an Add, Mul or function-argument list.
    SequenceWild(String),
    /// Matches an n-ary sum term by term.
    Add(Vec<Pattern>),
    /// Matches an n-ary product factor by factor.
    Mul(Vec<Pattern>),
    /// Matches a power: base and exponent.
    Pow(Box<Pattern>, Box<Pattern>),
    /// Matches a function application by name and arguments.
    Fun(String, Vec<Pattern>),
}

/// The bindings produced by a successful pattern match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Match {
    bindings: BTreeMap<String, Expr>,
    sequences: BTreeMap<String, Vec<Expr>>,
}

impl Match {
    /// An empty binding set.
    pub fn new() -> Match {
        Match::default()
    }

    /// The node bound to a pattern variable; panics when the name was not bound, exactly as the
    /// reference `Match.Get` does (a caller may only ask for a name its own pattern binds).
    pub fn get(&self, name: &str) -> Expr {
        self.bindings
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("Pattern variable '{name}' is not bound."))
    }

    /// The sibling sequence bound to a sequence variable; panics when the name was not bound.
    pub fn get_seq(&self, name: &str) -> &[Expr] {
        self.sequences
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_else(|| panic!("Sequence variable '{name}' is not bound."))
    }

    /// The node bound to a pattern variable, when it exists.
    pub fn try_get(&self, name: &str) -> Option<&Expr> {
        self.bindings.get(name)
    }

    fn bind(&mut self, name: &str, expression: &Expr) -> bool {
        match self.bindings.get(name) {
            Some(previous) => previous == expression,
            None => {
                self.bindings.insert(name.to_string(), expression.clone());
                true
            }
        }
    }

    fn bind_seq(&mut self, name: &str, expressions: &[Expr]) -> bool {
        match self.sequences.get(name) {
            Some(previous) => previous.as_slice() == expressions,
            None => {
                self.sequences
                    .insert(name.to_string(), expressions.to_vec());
                true
            }
        }
    }

    fn merge_from(&mut self, other: &Match) -> bool {
        let mut merged = self.clone();
        for (name, expression) in &other.bindings {
            if let Some(previous) = merged.bindings.get(name) {
                if previous != expression {
                    return false;
                }
            }
            merged.bindings.insert(name.clone(), expression.clone());
        }
        for (name, expressions) in &other.sequences {
            if let Some(previous) = merged.sequences.get(name) {
                if previous.as_slice() != expressions.as_slice() {
                    return false;
                }
            }
            merged.sequences.insert(name.clone(), expressions.clone());
        }
        *self = merged;
        true
    }
}

type ApplicabilityFn =
    dyn Fn(&Match, &AssumptionSet) -> Result<RuleApplicability, RuleError> + Send + Sync;
type ReplacementFn = dyn Fn(&Match, &AssumptionSet) -> Expr + Send + Sync;
type ConditionBuilderFn = dyn Fn(&Match, &AssumptionSet) -> AssumptionSet + Send + Sync;

/// One conditional rewrite rule with a stable id, a group, a pattern, an applicability predicate
/// and a replacement builder.
pub struct RewriteRule {
    /// The stable rule id (for example `rat.cancel-x-over-x`).
    pub id: String,
    /// The phase group the rule belongs to (`trig`, `power`, `rat`, `logexp`, `abs`).
    pub group: String,
    /// The pattern the rule matches.
    pub pattern: Pattern,
    /// The rule's semantic classification.
    pub classification: RuleClassification,
    /// Declarative side conditions the rule depends on, for traces and documentation.
    pub declared_conditions: AssumptionSet,
    applicability: Box<ApplicabilityFn>,
    replacement: Box<ReplacementFn>,
    condition_builder: Option<Box<ConditionBuilderFn>>,
}

impl RewriteRule {
    /// Builds a rule. The classification defaults to [`RuleClassification::Conditional`], exactly
    /// as the reference `RewriteRule` does: a rule must opt into `Universal` explicitly.
    pub fn new<A, R>(
        id: impl Into<String>,
        group: impl Into<String>,
        pattern: Pattern,
        applicability: A,
        replacement: R,
    ) -> RewriteRule
    where
        A: Fn(&Match, &AssumptionSet) -> Result<RuleApplicability, RuleError>
            + Send
            + Sync
            + 'static,
        R: Fn(&Match, &AssumptionSet) -> Expr + Send + Sync + 'static,
    {
        RewriteRule {
            id: id.into(),
            group: group.into(),
            pattern,
            classification: RuleClassification::Conditional,
            declared_conditions: AssumptionSet::empty(),
            applicability: Box::new(applicability),
            replacement: Box::new(replacement),
            condition_builder: None,
        }
    }

    /// Sets the semantic classification.
    pub fn with_classification(mut self, classification: RuleClassification) -> RewriteRule {
        self.classification = classification;
        self
    }

    /// Sets the declarative side conditions the rule depends on.
    pub fn with_declared_conditions(mut self, conditions: AssumptionSet) -> RewriteRule {
        self.declared_conditions = conditions;
        self
    }

    /// Sets the builder of the concrete side conditions for a specific match.
    pub fn with_condition_builder<C>(mut self, builder: C) -> RewriteRule
    where
        C: Fn(&Match, &AssumptionSet) -> AssumptionSet + Send + Sync + 'static,
    {
        self.condition_builder = Some(Box::new(builder));
        self
    }

    /// Runs the applicability predicate for a match.
    pub fn applicability(
        &self,
        bindings: &Match,
        assumptions: &AssumptionSet,
    ) -> Result<RuleApplicability, RuleError> {
        (self.applicability)(bindings, assumptions)
    }

    /// Builds the replacement for a match.
    pub fn replacement(&self, bindings: &Match, assumptions: &AssumptionSet) -> Expr {
        (self.replacement)(bindings, assumptions)
    }

    /// The conditions one application requires: the condition builder's answer when present,
    /// otherwise the declared conditions (the reference `ConditionBuilder ?? DeclaredConditions`).
    pub fn conditions_for(&self, bindings: &Match, assumptions: &AssumptionSet) -> AssumptionSet {
        match &self.condition_builder {
            Some(builder) => builder(bindings, assumptions),
            None => self.declared_conditions.clone(),
        }
    }

    /// True when the rule has a per-match condition builder (the reference `ConditionBuilder` is
    /// not null). A universal rule must not have one.
    pub fn has_condition_builder(&self) -> bool {
        self.condition_builder.is_some()
    }
}

/// A registration-ordered list of rules with a group lookup, the reference `RewriteRuleRegistry`.
#[derive(Default)]
pub struct RewriteRuleRegistry {
    rules: Vec<RewriteRule>,
}

impl RewriteRuleRegistry {
    /// An empty registry.
    pub fn new() -> RewriteRuleRegistry {
        RewriteRuleRegistry { rules: Vec::new() }
    }

    /// Appends a rule. Registration order is application order inside a group.
    pub fn register(&mut self, rule: RewriteRule) {
        self.rules.push(rule);
    }

    /// The rules of one group, in registration order.
    pub fn group(&self, group_id: &str) -> Vec<&RewriteRule> {
        self.rules
            .iter()
            .filter(|rule| rule.group == group_id)
            .collect()
    }

    /// Every rule, in registration order.
    pub fn all(&self) -> &[RewriteRule] {
        &self.rules
    }

    /// The rule with the given id, when it is registered. This is the additive lookup that lets a
    /// caller apply one named rule directly; the reference observes the same list through
    /// `RulesForTesting`.
    pub fn get(&self, id: &str) -> Option<&RewriteRule> {
        self.rules.iter().find(|rule| rule.id == id)
    }
}

/// The deterministic step budget of the rewrite engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    /// The maximum number of APPLIED rewrites.
    pub max_steps: usize,
    /// The number of applied rewrites so far.
    pub steps: usize,
    /// The machine-readable budget identity published in structured diagnostics.
    pub kind: &'static str,
}

impl Budget {
    /// A budget of `max_steps` applied rewrites, identified as `rewrite_steps`.
    pub fn new(max_steps: usize) -> Budget {
        Budget {
            max_steps,
            steps: 0,
            kind: "rewrite_steps",
        }
    }

    /// True when the budget is exhausted (`steps >= max_steps`).
    pub fn exhausted(&self) -> bool {
        self.steps >= self.max_steps
    }
}

impl Default for Budget {
    fn default() -> Budget {
        Budget::new(400)
    }
}

/// One applied rewrite: machine-readable provenance for traces and falsification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteStep {
    /// The stable id of the rule that fired.
    pub rule_id: String,
    /// The rule's classification.
    pub classification: RuleClassification,
    /// The node before the rewrite.
    pub before: Expr,
    /// The node the rule produced.
    pub after: Expr,
    /// The conditions the rewrite requires.
    pub conditions: AssumptionSet,
}

impl fmt::Display for RewriteStep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} [{}]: {} -> {}",
            self.rule_id,
            self.classification.name(),
            self.before,
            self.after
        )?;
        if !self.conditions.atoms().is_empty() {
            let atoms: Vec<String> = self
                .conditions
                .atoms()
                .iter()
                .map(Assumption::describe)
                .collect();
            write!(formatter, " given {}", atoms.join("; "))?;
        }
        Ok(())
    }
}

/// One rule attempt, allocated only when diagnostics are requested.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleAttemptDiagnostic {
    /// The id of the rule that was tried.
    pub rule_id: String,
    /// True when the pattern matched (independently of applicability).
    pub pattern_matched: bool,
    /// The applicability decision, or `Unknown` for a caught `RuleEvaluationException`.
    pub precondition: RuleApplicability,
    /// Why the attempt did not fire, when there is a reason.
    pub rejection_reason: Option<String>,
}

/// The optional provenance and diagnostic sinks of one engine application. Conditions are
/// semantic bookkeeping and can be collected with or without the trace.
#[derive(Default)]
pub struct ApplySink<'a> {
    /// The applied-rewrite trace, when provenance is requested.
    pub trace: Option<&'a mut Vec<RewriteStep>>,
    /// The per-application conditions, when semantic bookkeeping is requested.
    pub applied_conditions: Option<&'a mut Vec<AssumptionSet>>,
    /// The per-attempt diagnostics, when diagnosis is requested.
    pub diagnostics: Option<&'a mut Vec<RuleAttemptDiagnostic>>,
}

/// The reference rewrite engine: deterministic, budgeted, bottom-up rewriting with ordered rule
/// application and a fixed point at every node.
pub struct RewriteEngine;

impl RewriteEngine {
    /// Applies `rules` to `expression` until no rule fires or the budget is exhausted, without
    /// collecting provenance or diagnostics.
    pub fn apply(
        expression: &Expr,
        assumptions: &AssumptionSet,
        rules: &[&RewriteRule],
        budget: &mut Budget,
    ) -> Result<Expr, RuleError> {
        RewriteEngine::apply_with(expression, assumptions, rules, budget, ApplySink::default())
    }

    /// Applies `rules` to `expression` with optional trace, condition and diagnostic sinks.
    ///
    /// Children are rewritten first in canonical order, then rules are tried at the rebuilt node in
    /// registration order. The first rule whose pattern matches and whose applicability is neither
    /// `NotApplicable` nor `Unknown` fires; an unchanged replacement does not count as a step and
    /// does not stop the scan. A node is re-examined until no rule fires or the budget is
    /// exhausted.
    pub fn apply_with(
        expression: &Expr,
        assumptions: &AssumptionSet,
        rules: &[&RewriteRule],
        budget: &mut Budget,
        mut sink: ApplySink<'_>,
    ) -> Result<Expr, RuleError> {
        RewriteEngine::walk(expression, assumptions, rules, budget, &mut sink)
    }

    /// Matches one pattern against one expression, filling `bindings` on success. This is the
    /// reference `RewriteEngine.TryMatch` entry point.
    pub fn try_match(pattern: &Pattern, expression: &Expr, bindings: &mut Match) -> bool {
        match_pattern(pattern, expression, bindings)
    }

    fn walk(
        expression: &Expr,
        assumptions: &AssumptionSet,
        rules: &[&RewriteRule],
        budget: &mut Budget,
        sink: &mut ApplySink<'_>,
    ) -> Result<Expr, RuleError> {
        let rebuilt = RewriteEngine::walk_children(expression, assumptions, rules, budget, sink)?;
        let mut current = rebuilt;
        while !budget.exhausted() {
            let mut fired = false;
            for rule in rules {
                if budget.exhausted() {
                    break;
                }
                let mut bindings = Match::new();
                if !match_pattern(&rule.pattern, &current, &mut bindings) {
                    push_diagnostic(
                        sink,
                        rule,
                        false,
                        RuleApplicability::NotApplicable,
                        Some("pattern did not match".to_string()),
                    );
                    continue;
                }
                let applicability = match rule.applicability(&bindings, assumptions) {
                    Ok(applicability) => applicability,
                    Err(error) => {
                        // The one caught state: the rule itself declared that it cannot decide.
                        // Every other error is a defect and propagates.
                        if let Some(undecidable) = error.downcast_ref::<RuleEvaluationException>() {
                            push_diagnostic(
                                sink,
                                rule,
                                true,
                                RuleApplicability::Unknown,
                                Some(undecidable.message.clone()),
                            );
                            continue;
                        }
                        return Err(error);
                    }
                };
                match applicability {
                    RuleApplicability::NotApplicable => {
                        push_diagnostic(
                            sink,
                            rule,
                            true,
                            applicability,
                            Some("precondition false".to_string()),
                        );
                        continue;
                    }
                    RuleApplicability::Unknown => {
                        push_diagnostic(
                            sink,
                            rule,
                            true,
                            applicability,
                            Some("applicability unknown".to_string()),
                        );
                        continue;
                    }
                    RuleApplicability::Applicable | RuleApplicability::ApplicableWithConditions => {
                    }
                }
                let next = rule.replacement(&bindings, assumptions);
                if next != current {
                    // The budget counts APPLIED rewrites, not matching attempts.
                    budget.steps += 1;
                    let conditions = rule.conditions_for(&bindings, assumptions);
                    if let Some(trace) = sink.trace.as_mut() {
                        trace.push(RewriteStep {
                            rule_id: rule.id.clone(),
                            classification: rule.classification,
                            before: current.clone(),
                            after: next.clone(),
                            conditions: conditions.clone(),
                        });
                    }
                    if let Some(applied) = sink.applied_conditions.as_mut() {
                        applied.push(conditions);
                    }
                    current = next;
                    fired = true;
                    break;
                }
            }
            if !fired {
                break;
            }
        }
        Ok(current)
    }

    /// Rebuilds a node from its rewritten children, using the canonical constructors the
    /// reference uses (`Exprs.Add` / `Exprs.Multiply` / `Exprs.Power`). An explicit division node
    /// has no reference counterpart and is kept as a division rather than silently turned into a
    /// negative power.
    fn walk_children(
        expression: &Expr,
        assumptions: &AssumptionSet,
        rules: &[&RewriteRule],
        budget: &mut Budget,
        sink: &mut ApplySink<'_>,
    ) -> Result<Expr, RuleError> {
        match expression {
            Expr::Num(_) | Expr::Sym(_) | Expr::Real(_) => Ok(expression.clone()),
            Expr::Add(terms) => {
                let mut sum = Expr::num(0);
                for term in terms {
                    sum = Expr::add(
                        sum,
                        RewriteEngine::walk(term, assumptions, rules, budget, sink)?,
                    );
                }
                Ok(sum)
            }
            Expr::Mul(factors) => {
                let mut product = Expr::num(1);
                for factor in factors {
                    product = Expr::mul(
                        product,
                        RewriteEngine::walk(factor, assumptions, rules, budget, sink)?,
                    );
                }
                Ok(product)
            }
            Expr::Pow(base, exponent) => Ok(Expr::pow(
                RewriteEngine::walk(base, assumptions, rules, budget, sink)?,
                RewriteEngine::walk(exponent, assumptions, rules, budget, sink)?,
            )),
            Expr::Fun(name, arguments) => {
                let mut rebuilt = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    rebuilt.push(RewriteEngine::walk(
                        argument,
                        assumptions,
                        rules,
                        budget,
                        sink,
                    )?);
                }
                Ok(Expr::Fun(name.clone(), rebuilt))
            }
            Expr::Div(numerator, denominator) => Ok(Expr::Div(
                Box::new(RewriteEngine::walk(
                    numerator,
                    assumptions,
                    rules,
                    budget,
                    sink,
                )?),
                Box::new(RewriteEngine::walk(
                    denominator,
                    assumptions,
                    rules,
                    budget,
                    sink,
                )?),
            )),
            Expr::Rel(operator, left, right) => Ok(Expr::Rel(
                *operator,
                Box::new(RewriteEngine::walk(left, assumptions, rules, budget, sink)?),
                Box::new(RewriteEngine::walk(
                    right,
                    assumptions,
                    rules,
                    budget,
                    sink,
                )?),
            )),
        }
    }
}

fn push_diagnostic(
    sink: &mut ApplySink<'_>,
    rule: &RewriteRule,
    pattern_matched: bool,
    precondition: RuleApplicability,
    rejection_reason: Option<String>,
) {
    if let Some(diagnostics) = sink.diagnostics.as_mut() {
        diagnostics.push(RuleAttemptDiagnostic {
            rule_id: rule.id.clone(),
            pattern_matched,
            precondition,
            rejection_reason,
        });
    }
}

fn match_pattern(pattern: &Pattern, expression: &Expr, bindings: &mut Match) -> bool {
    match pattern {
        Pattern::Literal(literal) => literal == expression,
        Pattern::Wild(name) => bindings.bind(name, expression),
        Pattern::SequenceWild(name) => bindings.bind_seq(name, std::slice::from_ref(expression)),
        Pattern::Add(patterns) => match expression {
            Expr::Add(terms) => match_nary(patterns, terms, bindings),
            _ => false,
        },
        Pattern::Mul(patterns) => match expression {
            Expr::Mul(factors) => match_nary(patterns, factors, bindings),
            _ => false,
        },
        Pattern::Pow(base, exponent) => match expression {
            Expr::Pow(actual_base, actual_exponent) => {
                match_pattern(base, actual_base, bindings)
                    && match_pattern(exponent, actual_exponent, bindings)
            }
            _ => false,
        },
        Pattern::Fun(name, arguments) => match expression {
            Expr::Fun(actual_name, actual_arguments) => {
                actual_name == name && match_nary(arguments, actual_arguments, bindings)
            }
            _ => false,
        },
    }
}

fn match_nary(patterns: &[Pattern], items: &[Expr], bindings: &mut Match) -> bool {
    let mut sequence_index = None;
    for (index, pattern) in patterns.iter().enumerate() {
        if matches!(pattern, Pattern::SequenceWild(_)) {
            if sequence_index.is_some() {
                panic!("At most one sequence wildcard per pattern list.");
            }
            sequence_index = Some(index);
        }
    }
    let Some(index) = sequence_index else {
        if patterns.len() != items.len() {
            return false;
        }
        for (pattern, item) in patterns.iter().zip(items) {
            if !match_pattern(pattern, item, bindings) {
                return false;
            }
        }
        return true;
    };

    let fixed_left = index;
    let fixed_right = patterns.len() - index - 1;
    if items.len() < fixed_left + fixed_right {
        return false;
    }
    let sequence_len = items.len() - fixed_left - fixed_right;
    // Leftmost-greedy first, exactly as the reference's MatchNary: sequence length 0, then 1, ...
    for length in 0..=sequence_len {
        let mut trial = bindings.clone();
        let mut matched = true;
        for cursor in 0..fixed_left {
            if !match_pattern(&patterns[cursor], &items[cursor], &mut trial) {
                matched = false;
                break;
            }
        }
        if matched {
            let Pattern::SequenceWild(name) = &patterns[index] else {
                unreachable!("sequence_index names a sequence wildcard");
            };
            matched = trial.bind_seq(name, &items[index..index + length]);
        }
        for cursor in 0..fixed_right {
            if !matched {
                break;
            }
            matched = match_pattern(
                &patterns[patterns.len() - fixed_right + cursor],
                &items[items.len() - fixed_right + cursor],
                &mut trial,
            );
        }
        if matched && bindings.merge_from(&trial) {
            return true;
        }
    }
    false
}

/// The reference's shipped registry: nine rules in registration order, with the reference's ids,
/// groups, pattern shapes, applicability predicates and conditions.
///
/// Two rules are `Universal` (the Pythagorean identity and the absolute-value sign rule); the
/// rest default to `Conditional` except the two `DomainSpecific` real-domain rules. No production path calls this
/// function; it is the observed surface and the tests are its only callers today.
pub fn shipped_registry() -> RewriteRuleRegistry {
    let mut registry = RewriteRuleRegistry::new();
    let two = Expr::num(2);
    let half = Expr::Num(Rat::from_parts(
        lovelace_int::Int::from_i64(1),
        lovelace_nat::Nat::from_u64(2),
    ));
    let minus_one = Expr::num(-1);

    // trig: sin(x)^2 + cos(x)^2 -> 1. The canonical sum sorts the cos term first.
    let cos_x = Pattern::Wild("x".to_string());
    let sin_y = Pattern::Wild("y".to_string());
    registry.register(
        RewriteRule::new(
            "trig.pythagorean-sin2-cos2",
            "trig",
            Pattern::Add(vec![
                Pattern::Pow(
                    Box::new(Pattern::Fun("cos".to_string(), vec![cos_x])),
                    Box::new(Pattern::Literal(two.clone())),
                ),
                Pattern::Pow(
                    Box::new(Pattern::Fun("sin".to_string(), vec![sin_y])),
                    Box::new(Pattern::Literal(two.clone())),
                ),
            ]),
            |bindings, _assumptions| {
                Ok(if bindings.get("x") == bindings.get("y") {
                    RuleApplicability::Applicable
                } else {
                    RuleApplicability::NotApplicable
                })
            },
            |_bindings, _assumptions| Expr::num(1),
        )
        .with_classification(RuleClassification::Universal),
    );

    // power: sqrt(w^2) -> w when w is provably nonnegative.
    let w = Pattern::Wild("w".to_string());
    registry.register(
        RewriteRule::new(
            "pow.sqrt-square-nonnegative",
            "power",
            Pattern::Pow(
                Box::new(Pattern::Pow(
                    Box::new(w),
                    Box::new(Pattern::Literal(two.clone())),
                )),
                Box::new(Pattern::Literal(half.clone())),
            ),
            |bindings, assumptions| {
                let applicable = match bindings.get("w") {
                    Expr::Sym(symbol) => {
                        assumptions.ask(&Assumption::SymbolProperty(
                            symbol,
                            SymbolPredicate::NonNegative,
                        )) == Tristate::True
                    }
                    _ => false,
                };
                Ok(if applicable {
                    RuleApplicability::Applicable
                } else {
                    RuleApplicability::NotApplicable
                })
            },
            |bindings, _assumptions| bindings.get("w"),
        )
        .with_condition_builder(|bindings, _assumptions| {
            let w = bindings.get("w");
            AssumptionSet::from_atoms([match w {
                Expr::Sym(symbol) => {
                    Assumption::SymbolProperty(symbol, SymbolPredicate::NonNegative)
                }
                expression => {
                    Assumption::ExpressionProperty(expression, SymbolPredicate::NonNegative)
                }
            }])
        }),
    );

    // power: sqrt(w^2) -> |w| when w is provably real but the nonnegative premise is not known.
    let w2 = Pattern::Wild("w2".to_string());
    registry.register(
        RewriteRule::new(
            "pow.sqrt-square-real",
            "power",
            Pattern::Pow(
                Box::new(Pattern::Pow(
                    Box::new(w2),
                    Box::new(Pattern::Literal(two.clone())),
                )),
                Box::new(Pattern::Literal(half.clone())),
            ),
            |bindings, assumptions| {
                let applicable = match bindings.get("w2") {
                    Expr::Sym(symbol) => {
                        assumptions.ask(&Assumption::SymbolProperty(
                            symbol.clone(),
                            SymbolPredicate::NonNegative,
                        )) != Tristate::True
                            && assumptions.ask(&Assumption::SymbolDomain(symbol, Domain::Real))
                                == Tristate::True
                    }
                    _ => false,
                };
                Ok(if applicable {
                    RuleApplicability::Applicable
                } else {
                    RuleApplicability::NotApplicable
                })
            },
            |bindings, _assumptions| Expr::Fun("abs".to_string(), vec![bindings.get("w2")]),
        )
        .with_classification(RuleClassification::DomainSpecific)
        .with_condition_builder(|bindings, _assumptions| match bindings.get("w2") {
            Expr::Sym(symbol) => {
                AssumptionSet::from_atoms([Assumption::SymbolDomain(symbol, Domain::Real)])
            }
            _ => AssumptionSet::empty(),
        }),
    );

    // rat: x * x^-1 -> 1, with the removed pole carried as x != 0.
    let cancel_u = Pattern::Wild("cu".to_string());
    let cancel_v = Pattern::Wild("cv".to_string());
    registry.register(
        RewriteRule::new(
            "rat.cancel-x-over-x",
            "rat",
            Pattern::Mul(vec![
                cancel_u,
                Pattern::Pow(
                    Box::new(cancel_v),
                    Box::new(Pattern::Literal(minus_one.clone())),
                ),
            ]),
            |bindings, _assumptions| {
                Ok(if bindings.get("cu") == bindings.get("cv") {
                    RuleApplicability::Applicable
                } else {
                    RuleApplicability::NotApplicable
                })
            },
            |_bindings, _assumptions| Expr::num(1),
        )
        .with_condition_builder(|bindings, _assumptions| non_zero_condition(&bindings.get("cu"))),
    );

    // rat: 0 * x^-1 -> 0, with the same removed pole condition.
    let zero_v = Pattern::Wild("cv2".to_string());
    registry.register(
        RewriteRule::new(
            "rat.cancel-zero-over-x",
            "rat",
            Pattern::Mul(vec![
                Pattern::Literal(Expr::num(0)),
                Pattern::Pow(
                    Box::new(zero_v),
                    Box::new(Pattern::Literal(minus_one.clone())),
                ),
            ]),
            |_bindings, _assumptions| Ok(RuleApplicability::Applicable),
            |_bindings, _assumptions| Expr::num(0),
        )
        .with_condition_builder(|bindings, _assumptions| non_zero_condition(&bindings.get("cv2"))),
    );

    // logexp: exp(log(x)) -> x, conditional on log(x) being finite.
    let exp_log_x = Pattern::Wild("lx".to_string());
    registry.register(
        RewriteRule::new(
            "logexp.exp-log",
            "logexp",
            Pattern::Fun(
                "exp".to_string(),
                vec![Pattern::Fun("log".to_string(), vec![exp_log_x])],
            ),
            |_bindings, _assumptions| Ok(RuleApplicability::Applicable),
            |bindings, _assumptions| bindings.get("lx"),
        )
        .with_condition_builder(|bindings, _assumptions| {
            AssumptionSet::from_atoms([Assumption::ExpressionProperty(
                Expr::Fun("log".to_string(), vec![bindings.get("lx")]),
                SymbolPredicate::Finite,
            )])
        }),
    );

    // logexp: log(exp(z)) -> z only when z lies in the principal strip Im z in (-pi, pi].
    let log_exp_z = Pattern::Wild("lx2".to_string());
    registry.register(RewriteRule::new(
        "logexp.log-exp",
        "logexp",
        Pattern::Fun(
            "log".to_string(),
            vec![Pattern::Fun("exp".to_string(), vec![log_exp_z])],
        ),
        |bindings, assumptions| {
            let z = bindings.get("lx2");
            let imaginary_part = Expr::Fun("im".to_string(), vec![z]);
            let pi = Expr::Fun("pi".to_string(), Vec::new());
            let minus_pi = Expr::neg(pi.clone());
            let strip = Assumption::Interval(imaginary_part, Some(minus_pi), true, Some(pi), false);
            Ok(if assumptions.ask(&strip) == Tristate::True {
                RuleApplicability::Applicable
            } else {
                RuleApplicability::NotApplicable
            })
        },
        |bindings, _assumptions| bindings.get("lx2"),
    ));

    // abs: |x|^2 -> x^2 only for a provably real x (over C, |z|^2 is not z^2).
    let abs_square = Pattern::Wild("aw".to_string());
    registry.register(
        RewriteRule::new(
            "abs.abs-square",
            "abs",
            Pattern::Pow(
                Box::new(Pattern::Fun("abs".to_string(), vec![abs_square])),
                Box::new(Pattern::Literal(two.clone())),
            ),
            |bindings, assumptions| {
                let applicable = match bindings.get("aw") {
                    Expr::Sym(symbol) => {
                        assumptions.ask(&Assumption::SymbolDomain(symbol, Domain::Real))
                            == Tristate::True
                    }
                    _ => false,
                };
                Ok(if applicable {
                    RuleApplicability::Applicable
                } else {
                    RuleApplicability::NotApplicable
                })
            },
            |bindings, _assumptions| Expr::pow(bindings.get("aw"), Expr::num(2)),
        )
        .with_classification(RuleClassification::DomainSpecific)
        .with_condition_builder(|bindings, _assumptions| match bindings.get("aw") {
            Expr::Sym(symbol) => {
                AssumptionSet::from_atoms([Assumption::SymbolDomain(symbol, Domain::Real)])
            }
            _ => AssumptionSet::empty(),
        }),
    );

    // abs: |-x| -> |x|, universal.
    let abs_neg = Pattern::Wild("aw2".to_string());
    registry.register(
        RewriteRule::new(
            "abs.abs-neg",
            "abs",
            Pattern::Fun(
                "abs".to_string(),
                vec![Pattern::Mul(vec![Pattern::Literal(minus_one), abs_neg])],
            ),
            |_bindings, _assumptions| Ok(RuleApplicability::Applicable),
            |bindings, _assumptions| Expr::Fun("abs".to_string(), vec![bindings.get("aw2")]),
        )
        .with_classification(RuleClassification::Universal),
    );

    registry
}

/// The ids of every shipped rewrite rule, in registration order (the reference
/// `Simplify.ShippedRuleIds`).
pub fn shipped_rule_ids() -> Vec<String> {
    shipped_registry()
        .all()
        .iter()
        .map(|rule| rule.id.clone())
        .collect()
}

/// The disposition of one transformation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformStatus {
    /// The rewrite completed under its collected side conditions.
    Satisfied,
    /// Rewriting stopped at the step budget with a partial result.
    BudgetExceeded,
    /// The applied rules carried contradictory requirements, so the branch has no model.
    Unsatisfiable,
}

impl TransformStatus {
    /// The kernel spelling the reference enum publishes (`Satisfied`, ...).
    pub fn name(self) -> &'static str {
        match self {
            TransformStatus::Satisfied => "Satisfied",
            TransformStatus::BudgetExceeded => "BudgetExceeded",
            TransformStatus::Unsatisfiable => "Unsatisfiable",
        }
    }
}

/// The options of [`transform`], mirroring the reference `Simplify.Options`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransformOptions {
    /// Collect the applied-rewrite trace. Conditions are collected regardless.
    pub trace: bool,
    /// The maximum number of applied rewrites.
    pub max_steps: usize,
    /// Collect per-attempt diagnostics.
    pub diagnose: bool,
}

impl Default for TransformOptions {
    fn default() -> TransformOptions {
        TransformOptions {
            trace: false,
            max_steps: 400,
            diagnose: false,
        }
    }
}

/// A transformation outcome: the transformed expression, the side conditions the result requires,
/// the provenance trace and the budget state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformResult {
    /// The expression the transformation started from.
    pub original: Expr,
    /// The transformed expression.
    pub expression: Expr,
    /// The side conditions under which `expression` equals `original` pointwise.
    pub conditions: AssumptionSet,
    /// The applied-rewrite trace; empty when tracing is disabled.
    pub steps: Vec<RewriteStep>,
    /// True when rewriting stopped at the step budget.
    pub budget_exceeded: bool,
    /// The machine-readable budget identity when `budget_exceeded` is set.
    pub budget_kind: Option<&'static str>,
    /// Per-attempt diagnostics; empty unless requested.
    pub attempts: Vec<RuleAttemptDiagnostic>,
}

impl TransformResult {
    /// True when the transformation changed the expression.
    pub fn changed(&self) -> bool {
        self.expression != self.original
    }

    /// The structured status: `Unsatisfiable` when the conditions have no model,
    /// `BudgetExceeded` when the budget stopped the rewrite, `Satisfied` otherwise.
    pub fn status(&self) -> TransformStatus {
        if self.conditions.is_unsatisfiable() {
            TransformStatus::Unsatisfiable
        } else if self.budget_exceeded {
            TransformStatus::BudgetExceeded
        } else {
            TransformStatus::Satisfied
        }
    }
}

/// Runs the reference `Simplify.Transform` protocol over [`shipped_registry`]: five phase groups
/// in the fixed order trig, power, rat, logexp, abs; a budget shared across the passes; conditions
/// collected whether or not tracing is enabled.
///
/// The shipped rules are total (no applicability probe returns an error), so this function cannot
/// fail. It is an additive observation surface: the language-layer simplifier is the production
/// path and is unchanged.
pub fn transform(
    expression: &Expr,
    assumptions: &AssumptionSet,
    options: TransformOptions,
) -> TransformResult {
    let registry = shipped_registry();
    let mut budget = Budget::new(options.max_steps);
    let mut trace = if options.trace {
        Some(Vec::new())
    } else {
        None
    };
    let mut applied_conditions = Vec::new();
    let mut attempts = if options.diagnose {
        Some(Vec::new())
    } else {
        None
    };
    let mut current = expression.clone();
    for group in ["trig", "power", "rat", "logexp", "abs"] {
        // Once the budget is exhausted there is nothing left to gain from another pass.
        if budget.exhausted() {
            break;
        }
        let rules = registry.group(group);
        let sink = ApplySink {
            trace: trace.as_mut(),
            applied_conditions: Some(&mut applied_conditions),
            diagnostics: attempts.as_mut(),
        };
        current = RewriteEngine::apply_with(&current, assumptions, &rules, &mut budget, sink)
            .expect("the shipped rewrite rules never return an applicability error");
    }
    let conditions = collect_conditions(&applied_conditions, assumptions);
    TransformResult {
        original: expression.clone(),
        expression: current,
        conditions,
        steps: trace.unwrap_or_default(),
        budget_exceeded: budget.exhausted(),
        budget_kind: if budget.exhausted() {
            Some(budget.kind)
        } else {
            None
        },
        attempts: attempts.unwrap_or_default(),
    }
}

fn collect_conditions(applied: &[AssumptionSet], ambient: &AssumptionSet) -> AssumptionSet {
    let mut conditions = AssumptionSet::empty();
    for set in applied {
        for atom in set.atoms() {
            match conditions.add(atom.clone()) {
                Ok(next) => conditions = next,
                Err(_) => return AssumptionSet::unsatisfiable(),
            }
        }
    }
    // A requirement the ACTIVE assumptions already refute makes this branch unsatisfiable,
    // instead of reporting "satisfied, provided x != 0" on a set that proves x == 0.
    if !conditions.is_unsatisfiable() && refuted_by_assumptions(&conditions, ambient) {
        AssumptionSet::unsatisfiable()
    } else {
        conditions
    }
}

fn refuted_by_assumptions(collected: &AssumptionSet, ambient: &AssumptionSet) -> bool {
    if ambient.is_unsatisfiable() {
        return true;
    }
    for atom in collected.atoms() {
        if let Assumption::SymbolProperty(symbol, predicate) = atom {
            if let Some(relation) = Assumption::relation_of(symbol, *predicate) {
                if ambient.ask(&relation) == Tristate::False {
                    return true;
                }
                continue;
            }
        }
        if let Some(negation) = atom.negate() {
            if ambient.ask(&negation) == Tristate::True {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use lovelace_int::Int;
    use lovelace_nat::Nat;

    use super::*;

    fn x() -> Expr {
        Expr::sym("x")
    }

    fn one() -> Expr {
        Expr::num(1)
    }

    fn two() -> Expr {
        Expr::num(2)
    }

    fn half() -> Expr {
        Expr::Num(Rat::from_parts(Int::from_i64(1), Nat::from_u64(2)))
    }

    fn sqrt_square(inner: Expr) -> Expr {
        Expr::pow(Expr::pow(inner, two()), half())
    }

    fn x_over_x() -> Expr {
        Expr::div(x(), x())
    }

    fn real_symbol(name: &str) -> AssumptionSet {
        AssumptionSet::empty()
            .add(Assumption::SymbolDomain(name.to_string(), Domain::Real))
            .expect("x is real is consistent")
    }

    fn non_zero_x() -> Assumption {
        Assumption::SymbolProperty("x".to_string(), SymbolPredicate::NonZero)
    }

    #[test]
    fn every_rule_has_a_stable_unique_id_and_universal_rules_are_unconditional() {
        let registry = shipped_registry();
        let rules = registry.all();
        assert_eq!(rules.len(), 9, "the reference registry ships nine rules");
        let ids: Vec<&str> = rules.iter().map(|rule| rule.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "trig.pythagorean-sin2-cos2",
                "pow.sqrt-square-nonnegative",
                "pow.sqrt-square-real",
                "rat.cancel-x-over-x",
                "rat.cancel-zero-over-x",
                "logexp.exp-log",
                "logexp.log-exp",
                "abs.abs-square",
                "abs.abs-neg",
            ]
        );
        let unique: HashSet<&str> = ids.iter().copied().collect();
        assert_eq!(unique.len(), ids.len(), "rule ids must be unique");

        let universal: Vec<&str> = rules
            .iter()
            .filter(|rule| rule.classification == RuleClassification::Universal)
            .map(|rule| rule.id.as_str())
            .collect();
        assert_eq!(universal, vec!["trig.pythagorean-sin2-cos2", "abs.abs-neg"]);
        for rule in rules
            .iter()
            .filter(|rule| rule.classification == RuleClassification::Universal)
        {
            assert!(
                rule.declared_conditions.atoms().is_empty(),
                "universal rule {} must not declare conditions",
                rule.id
            );
            assert!(
                !rule.has_condition_builder(),
                "universal rule {} must not have a condition builder",
                rule.id
            );
        }
    }

    #[test]
    fn domain_specific_rule_declares_its_condition() {
        let assumptions = real_symbol("x");
        let result = transform(
            &sqrt_square(x()),
            &assumptions,
            TransformOptions {
                trace: true,
                ..TransformOptions::default()
            },
        );
        assert_eq!(result.expression, Expr::Fun("abs".to_string(), vec![x()]));
        let step = result
            .steps
            .first()
            .expect("the real-domain square-root rule fires");
        assert_eq!(result.steps.len(), 1);
        assert_eq!(step.rule_id, "pow.sqrt-square-real");
        assert_eq!(step.classification, RuleClassification::DomainSpecific);
        assert!(
            !step.conditions.atoms().is_empty(),
            "the domain condition is provenance, not a hidden lambda"
        );
        assert!(
            step.conditions
                .contains(&Assumption::SymbolDomain("x".to_string(), Domain::Real))
        );
        assert_eq!(result.status(), TransformStatus::Satisfied);
    }

    #[test]
    fn budget_counts_applied_rewrites_and_stops_further_groups() {
        let result = transform(
            &x_over_x(),
            &AssumptionSet::empty(),
            TransformOptions {
                max_steps: 0,
                ..TransformOptions::default()
            },
        );
        assert!(result.budget_exceeded);
        assert_eq!(result.status(), TransformStatus::BudgetExceeded);
        assert_eq!(result.status().name(), "BudgetExceeded");
        assert_eq!(result.budget_kind, Some("rewrite_steps"));
        assert!(result.steps.is_empty());
        assert_eq!(result.expression, x_over_x());
    }

    #[test]
    fn trace_disabled_keeps_conditions_and_allocates_no_steps() {
        let result = transform(
            &x_over_x(),
            &AssumptionSet::empty(),
            TransformOptions::default(),
        );
        assert!(result.steps.is_empty());
        assert!(!result.conditions.atoms().is_empty());
        assert!(result.conditions.contains(&non_zero_x()));
        assert_eq!(result.expression, one());
    }

    #[test]
    fn rule_evaluation_exception_is_the_only_caught_state() {
        let rule = RewriteRule::new(
            "test.undecidable",
            "test",
            Pattern::Fun(
                "undecidable".to_string(),
                vec![Pattern::Wild("a".to_string())],
            ),
            |_bindings, _assumptions| {
                Err(Box::new(RuleEvaluationException::new("cannot decide here")))
            },
            |_bindings, _assumptions| Expr::num(0),
        );
        let expression = Expr::Fun("undecidable".to_string(), vec![x()]);
        let mut diagnostics = Vec::new();
        let mut budget = Budget::new(400);
        let result = RewriteEngine::apply_with(
            &expression,
            &AssumptionSet::empty(),
            &[&rule],
            &mut budget,
            ApplySink {
                diagnostics: Some(&mut diagnostics),
                ..ApplySink::default()
            },
        )
        .expect("the declared undecidable state is caught");
        assert_eq!(
            result, expression,
            "the caught state left the node unchanged"
        );
        let attempt = diagnostics
            .iter()
            .find(|attempt| attempt.pattern_matched)
            .expect("the matched attempt is diagnosed");
        assert_eq!(attempt.precondition, RuleApplicability::Unknown);
        assert_eq!(
            attempt.rejection_reason.as_deref(),
            Some("cannot decide here")
        );
    }

    #[test]
    fn throwing_precondition_propagates() {
        #[derive(Debug)]
        struct BoomError;

        impl fmt::Display for BoomError {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("boom precondition")
            }
        }

        impl Error for BoomError {}

        let rule = RewriteRule::new(
            "test.boom-precondition",
            "test",
            Pattern::Fun("boom".to_string(), vec![Pattern::Wild("a".to_string())]),
            |_bindings, _assumptions| Err(Box::new(BoomError)),
            |_bindings, _assumptions| Expr::num(0),
        );
        let expression = Expr::Fun("boom".to_string(), vec![x()]);
        let error = RewriteEngine::apply(
            &expression,
            &AssumptionSet::empty(),
            &[&rule],
            &mut Budget::new(400),
        )
        .expect_err("a state the reference calls a defect must NOT be reported as not applicable");
        assert_eq!(error.to_string(), "boom precondition");
        assert!(error.downcast_ref::<BoomError>().is_some());
    }

    #[test]
    fn unknown_applicability_never_fires() {
        let rule = RewriteRule::new(
            "test.unknown",
            "test",
            Pattern::Fun("maybe".to_string(), vec![Pattern::Wild("a".to_string())]),
            |_bindings, _assumptions| Ok(RuleApplicability::Unknown),
            |_bindings, _assumptions| Expr::num(0),
        );
        let expression = Expr::Fun("maybe".to_string(), vec![x()]);
        let result = RewriteEngine::apply(
            &expression,
            &AssumptionSet::empty(),
            &[&rule],
            &mut Budget::new(400),
        )
        .expect("Unknown is a value, not an error");
        assert_eq!(result, expression);
    }

    #[test]
    fn registry_lists_the_nine_reference_rules_in_registration_order() {
        let registry = shipped_registry();
        assert_eq!(registry.group("trig").len(), 1);
        assert_eq!(registry.group("power").len(), 2);
        assert_eq!(registry.group("rat").len(), 2);
        assert_eq!(registry.group("logexp").len(), 2);
        assert_eq!(registry.group("abs").len(), 2);
        assert_eq!(
            registry
                .group("power")
                .iter()
                .map(|rule| rule.id.as_str())
                .collect::<Vec<_>>(),
            vec!["pow.sqrt-square-nonnegative", "pow.sqrt-square-real"]
        );
        assert_eq!(
            shipped_rule_ids(),
            registry
                .all()
                .iter()
                .map(|rule| rule.id.clone())
                .collect::<Vec<_>>()
        );
        assert!(registry.get("rat.cancel-x-over-x").is_some());
        assert!(registry.get("not.a.rule").is_none());
    }

    #[test]
    fn named_rule_application_rewrites_with_its_condition() {
        let registry = shipped_registry();
        let cancel = registry
            .get("rat.cancel-x-over-x")
            .expect("the cancellation rule is registered");
        let mut applied = Vec::new();
        let result = RewriteEngine::apply_with(
            &x_over_x(),
            &AssumptionSet::empty(),
            &[cancel],
            &mut Budget::new(400),
            ApplySink {
                applied_conditions: Some(&mut applied),
                ..ApplySink::default()
            },
        )
        .expect("the cancellation rule is total");
        assert_eq!(result, one());
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0], non_zero_condition(&x()));

        let abs_neg = registry
            .get("abs.abs-neg")
            .expect("the abs-neg rule is registered");
        let negated = Expr::Fun("abs".to_string(), vec![Expr::neg(x())]);
        assert_eq!(
            RewriteEngine::apply(
                &negated,
                &AssumptionSet::empty(),
                &[abs_neg],
                &mut Budget::new(400)
            )
            .expect("the universal abs rule is total"),
            Expr::Fun("abs".to_string(), vec![x()])
        );
    }

    #[test]
    fn shipped_patterns_match_the_reference_shapes_and_first_match_wins() {
        let registry = shipped_registry();
        let cases = [
            (
                "trig.pythagorean-sin2-cos2",
                Expr::add(
                    Expr::pow(Expr::Fun("sin".to_string(), vec![x()]), two()),
                    Expr::pow(Expr::Fun("cos".to_string(), vec![x()]), two()),
                ),
            ),
            ("pow.sqrt-square-nonnegative", sqrt_square(x())),
            ("pow.sqrt-square-real", sqrt_square(x())),
            ("rat.cancel-x-over-x", x_over_x()),
            (
                "rat.cancel-zero-over-x",
                Expr::mul(Expr::num(0), Expr::pow(x(), Expr::num(-1))),
            ),
            (
                "logexp.exp-log",
                Expr::Fun(
                    "exp".to_string(),
                    vec![Expr::Fun("log".to_string(), vec![x()])],
                ),
            ),
            (
                "logexp.log-exp",
                Expr::Fun(
                    "log".to_string(),
                    vec![Expr::Fun("exp".to_string(), vec![x()])],
                ),
            ),
            (
                "abs.abs-square",
                Expr::pow(Expr::Fun("abs".to_string(), vec![x()]), Expr::num(2)),
            ),
            (
                "abs.abs-neg",
                Expr::Fun("abs".to_string(), vec![Expr::neg(x())]),
            ),
        ];
        for (id, expression) in cases {
            let rule = registry.get(id).expect("the listed rule exists");
            let mut bindings = Match::new();
            assert!(
                RewriteEngine::try_match(&rule.pattern, &expression, &mut bindings),
                "the pattern of {id} matches its reference shape"
            );
        }

        // Registration order is application order: both rules match, the first one wins.
        let first = RewriteRule::new(
            "test.first",
            "test",
            Pattern::Fun("pick".to_string(), vec![Pattern::Wild("a".to_string())]),
            |_bindings, _assumptions| Ok(RuleApplicability::Applicable),
            |_bindings, _assumptions| Expr::num(1),
        );
        let second = RewriteRule::new(
            "test.second",
            "test",
            Pattern::Fun("pick".to_string(), vec![Pattern::Wild("a".to_string())]),
            |_bindings, _assumptions| Ok(RuleApplicability::Applicable),
            |_bindings, _assumptions| Expr::num(2),
        );
        let picked = Expr::Fun("pick".to_string(), vec![x()]);
        assert_eq!(
            RewriteEngine::apply(
                &picked,
                &AssumptionSet::empty(),
                &[&first, &second],
                &mut Budget::new(400)
            )
            .expect("both test rules are total"),
            one()
        );
    }
}
