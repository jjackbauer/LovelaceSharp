//! The assumption engine: an immutable set of assumed facts that answers queries in a
//! tri-state (True/False/Unknown, plus Unsatisfiable for a set with no model).
//!
//! This is the kernel the language's assume / assume_* / assume_clear / assumptions() and
//! simplify / simplify_full surfaces read. It is a port of the reference kernel's AssumptionSet:
//! the same atoms, the same implication lattice, the same constant-bound reasoning, the same
//! contradiction detection (including the integer-domain emptiness that no pairwise negation
//! test can see) and the same canonical, rendering-keyed order.
//!
//! A query answers True only when the set PROVES it and False only when the set proves its
//! negation; everything else is Unknown. There is no closed-world assumption, and an
//! unsatisfiable set answers Unsatisfiable rather than Unknown so a consumer can tell "this
//! branch has no model" from "no information".

use crate::{Expr, Rat, RelOp};
use lovelace_int::Int;

/// A tri-state answer, plus the "no model" sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tristate {
    /// The queried condition is provable from the set.
    True,
    /// Its negation is provable from the set.
    False,
    /// The set says nothing either way.
    Unknown,
    /// The set has no model: nothing is provable because nothing holds.
    Unsatisfiable,
}

/// The domain lattice: Integer < Rational < Real < Complex.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Domain {
    /// Whole numbers.
    Integer,
    /// Exact ratios of integers.
    Rational,
    /// Real numbers.
    Real,
    /// Complex numbers.
    Complex,
}

impl Domain {
    /// The spelling the wire and the human rendering use (real, integer, ...).
    pub fn name(self) -> &'static str {
        match self {
            Domain::Integer => "integer",
            Domain::Rational => "rational",
            Domain::Real => "real",
            Domain::Complex => "complex",
        }
    }

    /// The kernel spelling used by the human rendering (Real, Integer, ...).
    pub fn kernel_name(self) -> &'static str {
        match self {
            Domain::Integer => "Integer",
            Domain::Rational => "Rational",
            Domain::Real => "Real",
            Domain::Complex => "Complex",
        }
    }

    /// The narrowest of two domains: several domain atoms are NOT contradictory, and the
    /// narrowest one is the best structural answer.
    pub fn narrowest(self, other: Domain) -> Domain {
        if (self as u8) <= (other as u8) {
            self
        } else {
            other
        }
    }
}

/// A property a symbol (or expression) is assumed to have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolPredicate {
    /// Strictly greater than zero.
    Positive,
    /// Strictly less than zero.
    Negative,
    /// Greater than or equal to zero.
    NonNegative,
    /// Less than or equal to zero.
    NonPositive,
    /// Different from zero.
    NonZero,
    /// An even integer.
    Even,
    /// An odd integer.
    Odd,
    /// Finite (defined and bounded).
    Finite,
}

impl SymbolPredicate {
    /// The kernel spelling (Positive, ...), as the reference's enum prints it.
    pub fn name(self) -> &'static str {
        match self {
            SymbolPredicate::Positive => "Positive",
            SymbolPredicate::Negative => "Negative",
            SymbolPredicate::NonNegative => "NonNegative",
            SymbolPredicate::NonPositive => "NonPositive",
            SymbolPredicate::NonZero => "NonZero",
            SymbolPredicate::Even => "Even",
            SymbolPredicate::Odd => "Odd",
            SymbolPredicate::Finite => "Finite",
        }
    }
}

/// One assumed fact.
// The variant payload mirrors the reference's assumption forms, and `Interval` carries its two
// optional bounds inline. Boxing them would change the published enum shape (and allocate for
// every interval) to save stack space; this is a representation choice, not a defect.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Assumption {
    /// A symbol ranges over a domain.
    SymbolDomain(String, Domain),
    /// A symbol has a property.
    SymbolProperty(String, SymbolPredicate),
    /// A symbol stands in a relation to a bound expression.
    SymbolRelation(String, RelOp, Expr),
    /// An expression has a property.
    ExpressionProperty(Expr, SymbolPredicate),
    /// An expression lies in an interval: lower < e < upper (open ends optional).
    Interval(Expr, Option<Expr>, bool, Option<Expr>, bool),
}

impl Assumption {
    /// The relation operator equivalent to a predicate when one exists (NonZero => != 0);
    /// predicates with no single relation form (Even, Odd, Finite) answer None.
    pub fn relation_op(p: SymbolPredicate) -> Option<RelOp> {
        match p {
            SymbolPredicate::NonZero => Some(RelOp::Ne),
            SymbolPredicate::Positive => Some(RelOp::Gt),
            SymbolPredicate::Negative => Some(RelOp::Lt),
            SymbolPredicate::NonNegative => Some(RelOp::Ge),
            SymbolPredicate::NonPositive => Some(RelOp::Le),
            _ => None,
        }
    }

    /// The relation atom equivalent to a property atom, or None when the predicate has no
    /// relation form.
    pub fn relation_of(symbol: &str, p: SymbolPredicate) -> Option<Assumption> {
        Assumption::relation_op(p)
            .map(|op| Assumption::SymbolRelation(symbol.to_string(), op, Expr::num(0)))
    }

    /// The same relation read from the other side: a < b is b > a.
    pub fn flipped(op: RelOp) -> RelOp {
        match op {
            RelOp::Lt => RelOp::Gt,
            RelOp::Gt => RelOp::Lt,
            RelOp::Le => RelOp::Ge,
            RelOp::Ge => RelOp::Le,
            RelOp::Eq => RelOp::Eq,
            RelOp::Ne => RelOp::Ne,
        }
    }

    /// The logical negation of an atom, when one is expressible as an atom.
    pub fn negate(&self) -> Option<Assumption> {
        match self {
            Assumption::SymbolProperty(s, p) => match p {
                SymbolPredicate::Positive => Some(Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::NonPositive,
                )),
                SymbolPredicate::NonPositive => Some(Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Positive,
                )),
                SymbolPredicate::NonNegative => Some(Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Negative,
                )),
                SymbolPredicate::Negative => Some(Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::NonNegative,
                )),
                SymbolPredicate::Even => {
                    Some(Assumption::SymbolProperty(s.clone(), SymbolPredicate::Odd))
                }
                SymbolPredicate::Odd => {
                    Some(Assumption::SymbolProperty(s.clone(), SymbolPredicate::Even))
                }
                _ => None,
            },
            Assumption::SymbolRelation(s, op, bound) => {
                let negated = match op {
                    RelOp::Eq => RelOp::Ne,
                    RelOp::Ne => RelOp::Eq,
                    RelOp::Gt => RelOp::Le,
                    RelOp::Le => RelOp::Gt,
                    RelOp::Lt => RelOp::Ge,
                    RelOp::Ge => RelOp::Lt,
                };
                Some(Assumption::SymbolRelation(
                    s.clone(),
                    negated,
                    bound.clone(),
                ))
            }
            _ => None,
        }
    }

    /// True when the atom constrains the named symbol.
    pub fn mentions(&self, name: &str) -> bool {
        match self {
            Assumption::SymbolDomain(s, _)
            | Assumption::SymbolProperty(s, _)
            | Assumption::SymbolRelation(s, _, _) => s == name,
            Assumption::ExpressionProperty(e, _) => {
                e.free_symbols().iter().any(|symbol| symbol == name)
            }
            Assumption::Interval(e, lo, _, hi, _) => {
                let mut names = e.free_symbols();
                if let Some(lo) = lo {
                    names.extend(lo.free_symbols());
                }
                if let Some(hi) = hi {
                    names.extend(hi.free_symbols());
                }
                names.iter().any(|symbol| symbol == name)
            }
        }
    }

    /// The human rendering of one atom: relations as relations, domains as membership,
    /// predicates as words. Never Rust debug syntax, which would leak an implementation detail
    /// into a user-facing message.
    pub fn describe(&self) -> String {
        match self {
            Assumption::SymbolRelation(s, op, bound) => {
                Expr::relation(*op, Expr::sym(s), bound.clone()).to_string()
            }
            Assumption::SymbolDomain(s, d) => format!("{s} in {}", d.kernel_name()),
            Assumption::SymbolProperty(s, p) => format!("{s} is {}", p.name()),
            Assumption::ExpressionProperty(e, p) => format!("{e} is {}", p.name()),
            Assumption::Interval(e, lo, lo_open, hi, hi_open) => {
                let left = match lo {
                    None => "(-inf".to_string(),
                    Some(bound) => {
                        let bracket = if *lo_open { "(" } else { "[" };
                        format!("{bracket}{bound}")
                    }
                };
                let right = match hi {
                    None => "inf)".to_string(),
                    Some(bound) => {
                        let bracket = if *hi_open { ")" } else { "]" };
                        format!("{bound}{bracket}")
                    }
                };
                format!("{e} in {left}, {right}")
            }
        }
    }

    /// The ordering key: the atom's own record rendering, ordinal-compared. The set is kept
    /// sorted by this key, which is the reference's observable order (a domain atom renders
    /// before a property atom, which renders before a relation atom; relations order by
    /// operator then bound).
    pub fn order_key(&self) -> String {
        match self {
            Assumption::SymbolDomain(s, d) => {
                format!(
                    "SymbolDomainAssumption {{ S = {s}, D = {} }}",
                    d.kernel_name()
                )
            }
            Assumption::SymbolProperty(s, p) => {
                format!("SymbolPropertyAssumption {{ S = {s}, P = {} }}", p.name())
            }
            Assumption::SymbolRelation(s, op, bound) => format!(
                "SymbolRelationAssumption {{ S = {s}, Op = {}, Bound = {} }}",
                rel_op_name(*op),
                node_type_name(bound)
            ),
            Assumption::ExpressionProperty(e, p) => format!(
                "ExpressionPropertyAssumption {{ E = {}, P = {} }}",
                node_type_name(e),
                p.name()
            ),
            Assumption::Interval(e, lo, lo_open, hi, hi_open) => format!(
                "IntervalAssumption {{ E = {}, Lower = {}, LowerOpen = {}, Upper = {}, UpperOpen = {} }}",
                node_type_name(e),
                lo.as_ref().map(node_type_name).unwrap_or_default(),
                cs_bool(*lo_open),
                hi.as_ref().map(node_type_name).unwrap_or_default(),
                cs_bool(*hi_open)
            ),
        }
    }
}

fn rel_op_name(op: RelOp) -> &'static str {
    match op {
        RelOp::Eq => "Eq",
        RelOp::Ne => "Ne",
        RelOp::Gt => "Gt",
        RelOp::Lt => "Lt",
        RelOp::Ge => "Ge",
        RelOp::Le => "Le",
    }
}

fn cs_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

/// The ordering key's rendering of an expression: the reference orders atoms by the record's
/// generated rendering, and an expression contributes the NAME OF ITS NODE KIND there (the kernel
/// types do not override ToString). Two relations that differ only in their rational bound
/// therefore tie, and the insertion is stable among equal keys.
fn node_type_name(e: &Expr) -> &'static str {
    match e {
        Expr::Num(_) => "Lovelace.Symbolics.RationalConstantExpr",
        Expr::Sym(_) => "Lovelace.Symbolics.SymbolExpr",
        Expr::Add(_) => "Lovelace.Symbolics.AddExpr",
        Expr::Mul(_) => "Lovelace.Symbolics.MultiplyExpr",
        Expr::Pow(_, _) => "Lovelace.Symbolics.PowerExpr",
        Expr::Fun(_, _) => "Lovelace.Symbolics.FunctionExpr",
        Expr::Div(_, _) => "Lovelace.Symbolics.MultiplyExpr",
        Expr::Rel(_, _, _) => "Lovelace.Symbolics.RelationExpr",
        Expr::Real(_) => "Lovelace.Symbolics.RealConstantExpr",
        Expr::Complex(_, _) => "Lovelace.Symbolics.ComplexConstantExpr",
    }
}

/// Why an assumption was refused: its message is the sentence the language publishes, so the
/// wording is part of this type, not of a caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contradiction {
    /// The refusal sentence.
    pub message: String,
}

/// An immutable set of assumptions, ordered by [Assumption::order_key].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssumptionSet {
    atoms: Vec<Assumption>,
    /// The ordering keys, parallel to `atoms` (same length, same order). The rendering of an
    /// atom is computed ONCE, when it enters the set, instead of once per comparison — the
    /// reference stores the same parallel key array for the same reason.
    keys: Vec<String>,
    unsatisfiable: bool,
}

impl Default for AssumptionSet {
    fn default() -> Self {
        AssumptionSet::empty()
    }
}

impl AssumptionSet {
    /// The empty set: no condition holds.
    pub fn empty() -> AssumptionSet {
        AssumptionSet {
            atoms: Vec::new(),
            keys: Vec::new(),
            unsatisfiable: false,
        }
    }

    /// The sentinel for a provably contradictory set (no model exists). Distinct from empty: an
    /// unsatisfiable branch is not the absence of conditions, it is a contradiction that must be
    /// pruned or reported.
    pub fn unsatisfiable() -> AssumptionSet {
        AssumptionSet {
            atoms: Vec::new(),
            keys: Vec::new(),
            unsatisfiable: true,
        }
    }

    /// True when this is the unsatisfiable sentinel.
    pub fn is_unsatisfiable(&self) -> bool {
        self.unsatisfiable
    }

    /// The atoms, in canonical order.
    pub fn atoms(&self) -> &[Assumption] {
        &self.atoms
    }

    /// True when the exact atom is stored.
    pub fn contains(&self, a: &Assumption) -> bool {
        self.atoms.iter().any(|atom| atom == a)
    }

    /// Builds a set from an existing (already consistent) atom sequence, preserving the
    /// canonical order. Never fails: use [AssumptionSet::add] when contradiction detection is
    /// wanted.
    pub fn from_atoms<I: IntoIterator<Item = Assumption>>(atoms: I) -> AssumptionSet {
        let mut pairs: Vec<(String, Assumption)> = Vec::new();
        for atom in atoms {
            if pairs.iter().any(|(_, existing)| *existing == atom) {
                continue;
            }
            pairs.push((atom.order_key(), atom));
        }
        // A stable sort: equal keys keep the order they were given in, which is what the
        // reference's ordered insertion does for a tied key.
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let (keys, atoms) = pairs.into_iter().unzip();
        AssumptionSet {
            atoms,
            keys,
            unsatisfiable: false,
        }
    }

    /// Returns a new set including `a`; fails on contradiction. Adding to the unsatisfiable
    /// sentinel stays unsatisfiable.
    pub fn add(&self, a: Assumption) -> Result<AssumptionSet, Contradiction> {
        self.clone()
            .add_owned(a)
            .map_err(|(contradiction, _)| contradiction)
    }

    /// [AssumptionSet::add] for a caller that already owns the set: the atoms and their keys are
    /// moved into the new set instead of being copied, and a contradiction hands the unchanged
    /// set back so the caller can keep serving queries from it.
    pub fn add_owned(self, a: Assumption) -> Result<AssumptionSet, (Contradiction, AssumptionSet)> {
        if self.unsatisfiable {
            return Ok(self);
        }
        if self.contains(&a) {
            return Ok(self);
        }
        let mut probe = self;
        let key = a.order_key();
        // Ordered insertion: the key is rendered ONCE and the atom goes straight to its ordinal
        // position, so an insertion never re-renders nor re-sorts the rest of the collection.
        // Equal keys keep insertion order (the reference falls back to append-and-sort there).
        let index = probe.keys.partition_point(|existing| *existing <= key);
        probe.keys.insert(index, key);
        probe.atoms.insert(index, a.clone());

        if let Some(negation) = a.negate() {
            if probe.ask(&negation) == Tristate::True {
                return Err((
                    Contradiction {
                        message: format!(
                            "Assumption {} contradicts the existing assumptions (its negation {} is provable).",
                            a.describe(),
                            negation.describe()
                        ),
                    },
                    probe,
                ));
            }
        }
        // An integer domain with fractional bounds can be empty even though every atom is
        // individually consistent: no integer lies strictly between 1/2 and 1. The negation test
        // above cannot see this, because the emptiness comes from the domain, not from a clash.
        if probe.integer_interval_is_empty(&a) {
            return Err((
                Contradiction {
                    message: format!(
                        "Assumption {} leaves no integer satisfying the existing assumptions.",
                        a.describe()
                    ),
                },
                probe,
            ));
        }
        Ok(probe)
    }

    /// True when the symbol constrained by `added` is known to be an integer and the accumulated
    /// bounds admit no integer at all.
    fn integer_interval_is_empty(&self, added: &Assumption) -> bool {
        let Assumption::SymbolRelation(symbol, _, _) = added else {
            return false;
        };
        if !self
            .atoms
            .iter()
            .any(|atom| matches!(atom, Assumption::SymbolDomain(s, Domain::Integer) if s == symbol))
        {
            return false;
        }
        let mut lower: Option<(Rat, bool)> = None;
        let mut upper: Option<(Rat, bool)> = None;
        for atom in &self.atoms {
            let Assumption::SymbolRelation(s, op, bound) = atom else {
                continue;
            };
            if s != symbol {
                continue;
            }
            let Some(b) = numeric_to_rational(bound) else {
                continue;
            };
            match op {
                RelOp::Gt => match &lower {
                    Some((existing, _)) => match existing.cmp(&b) {
                        std::cmp::Ordering::Less => lower = Some((b, true)),
                        std::cmp::Ordering::Equal => lower = Some((existing.clone(), true)),
                        std::cmp::Ordering::Greater => {}
                    },
                    None => lower = Some((b, true)),
                },
                RelOp::Ge => match &lower {
                    Some((existing, _)) if existing.cmp(&b) != std::cmp::Ordering::Less => {}
                    _ => lower = Some((b, false)),
                },
                RelOp::Lt => match &upper {
                    Some((existing, _)) => match existing.cmp(&b) {
                        std::cmp::Ordering::Greater => upper = Some((b, true)),
                        std::cmp::Ordering::Equal => upper = Some((existing.clone(), true)),
                        std::cmp::Ordering::Less => {}
                    },
                    None => upper = Some((b, true)),
                },
                RelOp::Le => match &upper {
                    Some((existing, _)) if existing.cmp(&b) != std::cmp::Ordering::Greater => {}
                    _ => upper = Some((b, false)),
                },
                _ => {}
            }
        }
        let (Some((lower, lower_open)), Some((upper, upper_open))) = (lower, upper) else {
            return false;
        };
        // The smallest admissible integer is the ceiling (one past an integral open bound); the
        // largest is the floor (one below an integral open bound). Empty when the former
        // exceeds the latter. Integers only: over the reals 1/2 < x < 1 is satisfiable.
        let one = Int::from_i64(1);
        let lo = if lower.is_integer() && !lower_open {
            integer_of(&lower)
        } else {
            floor(&lower).add(&one)
        };
        let hi = if upper.is_integer() && upper_open {
            integer_of(&upper).sub(&one)
        } else {
            floor(&upper)
        };
        lo.cmp(&hi) == std::cmp::Ordering::Greater
    }

    /// Tri-state query over the set. Derived answers (bound reasoning, the implication lattice)
    /// dominate the literal negation-containment check: on a contradictory set both a query and
    /// its negation can be provable, and the positive derivation is the one that matters for
    /// contradiction detection in [AssumptionSet::add].
    pub fn ask(&self, predicate: &Assumption) -> Tristate {
        if self.unsatisfiable {
            return Tristate::Unsatisfiable;
        }
        if self.contains(predicate) {
            return Tristate::True;
        }
        let derived = match predicate {
            // ---- symbol properties: implication lattice ----
            Assumption::SymbolProperty(s, SymbolPredicate::Positive) => self.ask(
                &Assumption::SymbolRelation(s.clone(), RelOp::Gt, Expr::num(0)),
            ),
            Assumption::SymbolProperty(s, SymbolPredicate::NonNegative) => or(&[
                self.ask(&Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Positive,
                )),
                self.ask(&Assumption::SymbolRelation(
                    s.clone(),
                    RelOp::Ge,
                    Expr::num(0),
                )),
            ]),
            Assumption::SymbolProperty(s, SymbolPredicate::Negative) => self.ask(
                &Assumption::SymbolRelation(s.clone(), RelOp::Lt, Expr::num(0)),
            ),
            Assumption::SymbolProperty(s, SymbolPredicate::NonPositive) => or(&[
                self.ask(&Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Negative,
                )),
                self.ask(&Assumption::SymbolRelation(
                    s.clone(),
                    RelOp::Le,
                    Expr::num(0),
                )),
            ]),
            Assumption::SymbolProperty(s, SymbolPredicate::NonZero) => or(&[
                self.ask(&Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Positive,
                )),
                self.ask(&Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Negative,
                )),
                self.ask(&Assumption::SymbolProperty(s.clone(), SymbolPredicate::Odd)),
                self.ask(&Assumption::SymbolRelation(
                    s.clone(),
                    RelOp::Ne,
                    Expr::num(0),
                )),
            ]),
            Assumption::SymbolProperty(s, SymbolPredicate::Finite) => or(&[
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Real)),
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Rational)),
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Integer)),
            ]),

            // ---- domain lattice (only narrower domains infer wider ones) ----
            // Even/Odd imply Integer (never the reverse)
            Assumption::SymbolDomain(s, Domain::Integer) => or(&[
                self.ask(&Assumption::SymbolProperty(
                    s.clone(),
                    SymbolPredicate::Even,
                )),
                self.ask(&Assumption::SymbolProperty(s.clone(), SymbolPredicate::Odd)),
            ]),
            Assumption::SymbolDomain(s, Domain::Complex) => or(&[
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Real)),
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Rational)),
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Integer)),
            ]),
            Assumption::SymbolDomain(s, Domain::Real) => or(&[
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Integer)),
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Rational)),
            ]),
            Assumption::SymbolDomain(s, Domain::Rational) => {
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Integer))
            }

            // ---- expression properties (function-level inference) ----
            Assumption::ExpressionProperty(_, p @ SymbolPredicate::NonNegative)
            | Assumption::ExpressionProperty(_, p @ SymbolPredicate::Positive)
            | Assumption::ExpressionProperty(_, p @ SymbolPredicate::NonZero)
            | Assumption::ExpressionProperty(_, p @ SymbolPredicate::Finite) => {
                self.ask_expr_predicate(predicate, *p)
            }
            Assumption::Interval(..) => self.ask_interval(predicate),

            // ---- symbol relations: constant-bound reasoning ----
            Assumption::SymbolRelation(s, op, bound) => self.ask_relation(s, *op, bound),

            _ => Tristate::Unknown,
        };
        if derived == Tristate::True {
            return Tristate::True;
        }
        if let Some(negation) = predicate.negate() {
            if self.contains(&negation) {
                return Tristate::False;
            }
        }
        derived
    }

    /// Bound reasoning over assumed symbol relations: a relation query is True when some
    /// assumed constant-bound atom on the same symbol implies it, False when every comparable
    /// assumed atom implies its negation, and Unknown otherwise (x > 5 proves x > 0 and refutes
    /// x < 5, while x > 5 says nothing about x < 7).
    fn ask_relation(&self, symbol: &str, op: RelOp, bound: &Expr) -> Tristate {
        let Some(query_bound) = numeric_to_rational(bound) else {
            return Tristate::Unknown;
        };
        let mut any_implies = false;
        let mut all_refute = true;
        let mut any = false;
        // The atoms are walked in place: cloning every bound expression per query made the
        // N-atom scaling quadratic in allocations, not just in comparisons.
        for atom in &self.atoms {
            let (atom_op, atom_bound) = match atom {
                Assumption::SymbolRelation(s, o, b) if s == symbol => (*o, numeric_to_rational(b)),
                Assumption::SymbolProperty(s, p) if s == symbol => {
                    match Assumption::relation_op(*p) {
                        Some(o) => (o, Some(Rat::zero())),
                        None => continue,
                    }
                }
                _ => continue,
            };
            let Some(assumed) = atom_bound else {
                continue;
            };
            any = true;
            let (implies, refutes) = compare_bounds(atom_op, &assumed, op, &query_bound);
            any_implies |= implies;
            all_refute &= refutes;
        }
        if !any {
            return Tristate::Unknown;
        }
        if any_implies {
            return Tristate::True;
        }
        if all_refute {
            Tristate::False
        } else {
            Tristate::Unknown
        }
    }

    /// True when the expression is provably real-valued under this set.
    pub fn provably_real(&self, e: &Expr) -> bool {
        match e {
            Expr::Num(_) => true,
            Expr::Fun(name, args) if args.is_empty() => name != "i",
            Expr::Sym(s) => {
                self.ask(&Assumption::SymbolDomain(s.clone(), Domain::Real)) == Tristate::True
            }
            Expr::Add(terms) => terms.iter().all(|t| self.provably_real(t)),
            Expr::Mul(factors) => factors.iter().all(|f| self.provably_real(f)),
            Expr::Pow(base, exp) => match exp.as_ref() {
                Expr::Num(n) if n.is_integer() => self.provably_real(base),
                _ => false,
            },
            Expr::Fun(name, args) => {
                matches!(
                    name.as_str(),
                    "re" | "im" | "abs" | "sign" | "floor" | "ceil"
                ) && !args.is_empty()
                    && self.provably_real(&args[0])
            }
            _ => false,
        }
    }

    fn ask_expr_predicate(&self, predicate: &Assumption, p: SymbolPredicate) -> Tristate {
        let Assumption::ExpressionProperty(e, _) = predicate else {
            return Tristate::Unknown;
        };
        // a bare symbol delegates to its property atom
        if let Expr::Sym(s) = e {
            return self.ask(&Assumption::SymbolProperty(s.clone(), p));
        }
        // abs(x) is nonnegative; positive iff x != 0, valid over C as well
        if let Expr::Fun(name, args) = e {
            if name == "abs" && args.len() == 1 {
                if p == SymbolPredicate::NonNegative {
                    return Tristate::True;
                }
                if p == SymbolPredicate::Positive || p == SymbolPredicate::NonZero {
                    return match &args[0] {
                        Expr::Sym(s) => self.ask(&Assumption::SymbolProperty(
                            s.clone(),
                            SymbolPredicate::NonZero,
                        )),
                        _ => Tristate::Unknown,
                    };
                }
            }
            // exp(x) is positive (and nonzero) only when x is provably real: over C, exp(z) is
            // not an ordered real (exp(i*pi) = -1)
            if name == "exp"
                && args.len() == 1
                && (p == SymbolPredicate::Positive || p == SymbolPredicate::NonZero)
            {
                return if self.provably_real(&args[0]) {
                    Tristate::True
                } else {
                    Tristate::Unknown
                };
            }
            // log(f) is finite (defined) where f > 0 (principal branch)
            if name == "log" && args.len() == 1 && p == SymbolPredicate::Finite {
                return self.ask(&Assumption::ExpressionProperty(
                    args[0].clone(),
                    SymbolPredicate::Positive,
                ));
            }
        }
        // sqrt(x) is nonnegative (principal branch) only under provable real nonnegativity
        let half_power_base = match e {
            Expr::Pow(base, exp) => match exp.as_ref() {
                Expr::Num(r) if r.to_string() == "1/2" => Some(base.as_ref()),
                _ => None,
            },
            Expr::Fun(name, args) if name == "sqrt" && args.len() == 1 => Some(&args[0]),
            _ => None,
        };
        if let Some(base) = half_power_base {
            if p == SymbolPredicate::NonNegative {
                if !self.provably_real(base) {
                    return Tristate::Unknown;
                }
                if let Expr::Sym(s) = base {
                    return self.ask(&Assumption::SymbolProperty(
                        s.clone(),
                        SymbolPredicate::NonNegative,
                    ));
                }
                if let Some(value) = numeric_to_rational(base) {
                    return if value.is_negative() {
                        Tristate::False
                    } else {
                        Tristate::True
                    };
                }
                return Tristate::Unknown;
            }
        }
        // x^even is nonnegative only for provably real bases (over C, z^2 is not ordered)
        if let Expr::Pow(base, exp) = e {
            if let Expr::Num(r) = exp.as_ref() {
                if r.is_integer() && p == SymbolPredicate::NonNegative {
                    let even = integer_of(r)
                        .div_rem(&lovelace_nat::Nat::from_u64(2))
                        .map(|(_, rem)| rem.is_zero())
                        .unwrap_or(false);
                    if even {
                        return if self.provably_real(base) {
                            Tristate::True
                        } else {
                            Tristate::Unknown
                        };
                    }
                }
            }
        }
        Tristate::Unknown
    }

    fn ask_interval(&self, predicate: &Assumption) -> Tristate {
        let Assumption::Interval(e, lower, lower_open, upper, upper_open) = predicate else {
            return Tristate::Unknown;
        };
        // Im(z) = 0 when z is provably real: membership reduces to a constant check
        if let Expr::Fun(name, args) = e {
            if name == "im" && args.len() == 1 && self.provably_real(&args[0]) {
                if let Some(lo) = lower {
                    let answer = compare_to_zero(lo, true, *lower_open);
                    if answer == Tristate::Unknown {
                        return Tristate::Unknown; // an unevaluable bound is not a satisfied bound
                    }
                    if answer == Tristate::False {
                        return Tristate::False;
                    }
                }
                if let Some(hi) = upper {
                    let answer = compare_to_zero(hi, false, *upper_open);
                    if answer == Tristate::Unknown {
                        return Tristate::Unknown;
                    }
                    if answer == Tristate::False {
                        return Tristate::False;
                    }
                }
                return Tristate::True;
            }
        }
        Tristate::Unknown
    }

    /// The narrowest domain the set gives `symbol`; an unconstrained symbol ranges over the
    /// complex field.
    pub fn domain_of_symbol(&self, symbol: &str) -> Domain {
        let mut best: Option<Domain> = None;
        for atom in &self.atoms {
            if let Assumption::SymbolDomain(s, d) = atom {
                if s == symbol {
                    best = Some(match best {
                        Some(current) => current.narrowest(*d),
                        None => *d,
                    });
                }
            }
        }
        best.unwrap_or(Domain::Complex)
    }
}

fn or(states: &[Tristate]) -> Tristate {
    let mut any_unknown = false;
    for state in states {
        if *state == Tristate::True {
            return Tristate::True;
        }
        if *state == Tristate::Unknown {
            any_unknown = true;
        }
    }
    if any_unknown {
        Tristate::Unknown
    } else {
        Tristate::False
    }
}

fn satisfies(value: &Rat, op: RelOp, bound: &Rat) -> bool {
    let ordering = value.cmp(bound);
    match op {
        RelOp::Eq => ordering == std::cmp::Ordering::Equal,
        RelOp::Ne => ordering != std::cmp::Ordering::Equal,
        RelOp::Gt => ordering == std::cmp::Ordering::Greater,
        RelOp::Ge => ordering != std::cmp::Ordering::Less,
        RelOp::Lt => ordering == std::cmp::Ordering::Less,
        RelOp::Le => ordering != std::cmp::Ordering::Greater,
    }
}

/// Does the value set of the assumed relation imply / refute the query relation?
fn compare_bounds(assumed_op: RelOp, assumed: &Rat, query_op: RelOp, query: &Rat) -> (bool, bool) {
    match assumed_op {
        RelOp::Eq => (
            satisfies(assumed, query_op, query),
            !satisfies(assumed, query_op, query),
        ),
        RelOp::Ne => (
            query_op == RelOp::Ne && assumed == query,
            query_op == RelOp::Eq && assumed == query,
        ),
        RelOp::Gt => (
            implies_lower(assumed, true, query_op, query),
            refutes_lower(assumed, true, query_op, query),
        ),
        RelOp::Ge => (
            implies_lower(assumed, false, query_op, query),
            refutes_lower(assumed, false, query_op, query),
        ),
        RelOp::Lt => (
            implies_upper(assumed, true, query_op, query),
            refutes_upper(assumed, true, query_op, query),
        ),
        RelOp::Le => (
            implies_upper(assumed, false, query_op, query),
            refutes_upper(assumed, false, query_op, query),
        ),
    }
}

/// Does a lower bound (x > ab if strict, else x >= ab) imply the query?
fn implies_lower(ab: &Rat, strict: bool, query: RelOp, qb: &Rat) -> bool {
    let ordering = ab.cmp(qb);
    match query {
        RelOp::Gt => {
            if strict {
                ordering != std::cmp::Ordering::Less
            } else {
                ordering == std::cmp::Ordering::Greater
            }
        }
        RelOp::Ge => ordering != std::cmp::Ordering::Less,
        RelOp::Ne => {
            if strict {
                ordering != std::cmp::Ordering::Less
            } else {
                ordering == std::cmp::Ordering::Greater
            }
        }
        // a lower bound never implies an upper bound or an equality
        _ => false,
    }
}

/// Does a lower bound refute the query (every value above ab violates it)?
fn refutes_lower(ab: &Rat, strict: bool, query: RelOp, qb: &Rat) -> bool {
    let ordering = ab.cmp(qb);
    match query {
        // x >= ab refutes x < qb as soon as ab >= qb (the boundary itself already violates it)
        RelOp::Lt => ordering != std::cmp::Ordering::Less,
        RelOp::Le | RelOp::Eq => {
            if strict {
                ordering != std::cmp::Ordering::Less
            } else {
                ordering == std::cmp::Ordering::Greater
            }
        }
        _ => false,
    }
}

/// Does an upper bound (x < ab if strict, else x <= ab) imply the query?
fn implies_upper(ab: &Rat, strict: bool, query: RelOp, qb: &Rat) -> bool {
    let ordering = ab.cmp(qb);
    match query {
        RelOp::Lt => {
            if strict {
                ordering != std::cmp::Ordering::Greater
            } else {
                ordering == std::cmp::Ordering::Less
            }
        }
        RelOp::Le => ordering != std::cmp::Ordering::Greater,
        RelOp::Ne => {
            if strict {
                ordering != std::cmp::Ordering::Greater
            } else {
                ordering == std::cmp::Ordering::Less
            }
        }
        // an upper bound never implies a lower bound or an equality
        _ => false,
    }
}

/// Does an upper bound refute the query (every value below ab violates it)?
fn refutes_upper(ab: &Rat, strict: bool, query: RelOp, qb: &Rat) -> bool {
    let ordering = ab.cmp(qb);
    match query {
        RelOp::Gt => ordering != std::cmp::Ordering::Greater,
        RelOp::Ge | RelOp::Eq => {
            if strict {
                ordering != std::cmp::Ordering::Greater
            } else {
                ordering == std::cmp::Ordering::Less
            }
        }
        _ => false,
    }
}

/// Evaluates "0 satisfies this half-bound" from the bound's SIGN. The reference falls back to a
/// numeric evaluation; a sign oracle decides every bound that has one (pi, -pi, 2*pi, 1/2) and
/// answers Unknown for a bound that cannot be decided at all (a symbol). An unevaluable bound
/// must NEVER be treated as satisfied.
fn compare_to_zero(bound: &Expr, is_lower: bool, open: bool) -> Tristate {
    if let Some(value) = numeric_to_rational(bound) {
        let ok = if is_lower {
            if open {
                value.is_negative()
            } else {
                !value.is_negative()
            }
        } else if open {
            !value.is_negative() && !value.is_zero()
        } else {
            !value.is_negative()
        };
        return if ok { Tristate::True } else { Tristate::False };
    }
    let Some(sign) = sign_of(bound) else {
        return Tristate::Unknown;
    };
    let ok = if is_lower {
        if open { sign < 0 } else { sign <= 0 }
    } else if open {
        sign > 0
    } else {
        sign >= 0
    };
    if ok { Tristate::True } else { Tristate::False }
}

/// The sign of a constant expression, when it has one: -1, 0 or 1.
fn sign_of(e: &Expr) -> Option<i8> {
    match e {
        Expr::Real(text) => {
            let value = decimal_to_rational(text)?;
            Some(if value.is_zero() {
                0
            } else if value.is_negative() {
                -1
            } else {
                1
            })
        }
        Expr::Num(n) => Some(if n.is_zero() {
            0
        } else if n.is_negative() {
            -1
        } else {
            1
        }),
        Expr::Fun(name, args) if args.is_empty() => match name.as_str() {
            "pi" | "e" | "inf" => Some(1),
            _ => None,
        },
        Expr::Mul(factors) => {
            let mut sign = 1i8;
            for factor in factors {
                sign *= sign_of(factor)?;
            }
            Some(sign)
        }
        Expr::Div(a, b) => {
            let denominator = sign_of(b)?;
            if denominator == 0 {
                return None;
            }
            Some(sign_of(a)? * denominator)
        }
        Expr::Add(terms) => {
            let first = sign_of(terms.first()?)?;
            if terms.iter().all(|t| sign_of(t) == Some(first)) {
                Some(first)
            } else {
                None
            }
        }
        Expr::Pow(base, exp) => {
            let exponent = numeric_to_rational(exp)?;
            let base_sign = sign_of(base)?;
            if base_sign == 0 {
                return Some(0);
            }
            if exponent.is_integer() {
                let even = integer_of(&exponent)
                    .div_rem(&lovelace_nat::Nat::from_u64(2))
                    .map(|(_, rem)| rem.is_zero())
                    .unwrap_or(false);
                if base_sign < 0 && !even {
                    return Some(-1);
                }
                return Some(1);
            }
            if base_sign > 0 {
                return Some(1);
            }
            None
        }
        Expr::Fun(name, args) if name == "abs" && args.len() == 1 => match sign_of(&args[0])? {
            0 => Some(0),
            _ => Some(1),
        },
        _ => None,
    }
}

/// The exact rational an expression denotes, when it is a numeric constant. A real constant is
/// a finite decimal, so the rational it denotes is exact even though the LEAF is outside the
/// exact basis: that is why real-valued bounds still participate in bound reasoning.
pub fn numeric_to_rational(e: &Expr) -> Option<Rat> {
    match e {
        Expr::Num(n) => Some(n.clone()),
        Expr::Real(text) => decimal_to_rational(text),
        _ => None,
    }
}

/// The exact rational a decimal rendering denotes: an integer, a finite decimal, a periodic
/// decimal (`0.(3)`) or the compact `digits`e`exp` form. None when the text is not a decimal.
pub fn decimal_to_rational(text: &str) -> Option<Rat> {
    let (sign, body) = match text.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1i64, text),
    };
    // split a trailing exponent
    let (mantissa, exponent) = match body.split_once('e') {
        Some((m, e)) => (m, e.parse::<i64>().ok()?),
        None => (body, 0),
    };
    let (integer_part, fraction) = match mantissa.split_once('.') {
        Some((i, f)) => (i, f),
        None => (mantissa, ""),
    };
    if integer_part.is_empty() && fraction.is_empty() {
        return None;
    }
    if !integer_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let ten = lovelace_nat::Nat::from_u64(10);
    let mut numerator = if integer_part.is_empty() {
        Int::zero()
    } else {
        Int::from_decimal(integer_part)?
    };
    let mut denominator = lovelace_nat::Nat::from_u64(1);
    if !fraction.is_empty() {
        // a periodic block (p) is n1/10^p + r/(10^p * (10^L - 1))
        let (fixed, repeat) = match fraction.split_once('(') {
            Some((f, r)) => (f, Some(r.strip_suffix(')')?)),
            None => (fraction, None),
        };
        if !fixed.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        if !fixed.is_empty() {
            let scale = ten.pow(fixed.len() as u64);
            let digits = Int::from_decimal(fixed)?;
            numerator = numerator.mul(&int_of_nat(&scale)).add(&digits);
            denominator = scale;
        }
        if let Some(block) = repeat {
            if block.is_empty() || !block.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let repeat_scale = ten.pow(block.len() as u64);
            let repeat_minus_one = int_of_nat(&repeat_scale).sub(&Int::from_i64(1));
            let rep = Int::from_decimal(block)?;
            // value = fixed + rep / (10^p * (10^L - 1)) with the fixed part already over 10^p
            numerator = numerator.mul(&repeat_minus_one).add(&rep);
            denominator = denominator.mul(&nat_of_int(&repeat_minus_one));
        }
    }
    // apply the decimal exponent
    if exponent > 0 {
        numerator = numerator.mul(&int_of_nat(&ten.pow(exponent as u64)));
    } else if exponent < 0 {
        denominator = denominator.mul(&ten.pow((-exponent) as u64));
    }
    let numerator = if sign < 0 {
        numerator.negate()
    } else {
        numerator
    };
    Some(Rat::from_parts(numerator, denominator))
}

fn int_of_nat(value: &lovelace_nat::Nat) -> Int {
    Int::from_decimal(&value.to_decimal()).unwrap_or_else(Int::zero)
}

fn nat_of_int(value: &Int) -> lovelace_nat::Nat {
    lovelace_nat::Nat::from_decimal(&value.to_decimal()).unwrap_or_else(lovelace_nat::Nat::zero)
}

fn integer_of(r: &Rat) -> Int {
    Int::from_decimal(&r.numerator().to_decimal()).unwrap_or_else(Int::zero)
}

/// The floor of a rational (division truncates toward zero, so a negative remainder needs the
/// one-step adjustment).
fn floor(r: &Rat) -> Int {
    let numerator = r.numerator();
    let (quotient, remainder) = numerator
        .div_rem(r.denominator())
        .unwrap_or_else(|| (Int::zero(), Int::zero()));
    if remainder.is_zero() || !numerator.is_negative() {
        quotient
    } else {
        quotient.sub(&Int::from_i64(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn x() -> String {
        "x".to_string()
    }

    fn relation(op: RelOp, bound: i64) -> Assumption {
        Assumption::SymbolRelation(x(), op, Expr::num(bound))
    }

    fn half() -> Rat {
        Rat::from_i64(1)
            .div(&Rat::from_i64(2))
            .expect("nonzero divisor")
    }

    fn rational_expr(value: Rat) -> Expr {
        Expr::Num(value)
    }

    /// The reference truth table's witness sample: integers plus the two half-points that make
    /// the strict-bound pairs satisfiable over the reals.
    fn reference_witnesses() -> Vec<Rat> {
        let mut out: Vec<Rat> = [-100i64, 0, 3, 4, 5, 6, 100]
            .iter()
            .map(|v| Rat::from_i64(*v))
            .collect();
        out.push(Rat::from_i64(9).div(&Rat::from_i64(2)).expect("nonzero"));
        out.push(Rat::from_i64(11).div(&Rat::from_i64(2)).expect("nonzero"));
        out
    }

    #[test]
    fn empty_set_answers_unknown_everywhere() {
        let set = AssumptionSet::empty();
        assert_eq!(set.ask(&relation(RelOp::Gt, 0)), Tristate::Unknown);
        assert_eq!(
            set.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::Positive)),
            Tristate::Unknown
        );
        assert_eq!(set.atoms().len(), 0);
    }

    #[test]
    fn bound_reasoning_proves_and_refutes() {
        let set = AssumptionSet::empty()
            .add(relation(RelOp::Gt, 5))
            .expect("consistent");
        assert_eq!(set.ask(&relation(RelOp::Gt, 0)), Tristate::True);
        assert_eq!(set.ask(&relation(RelOp::Ge, 0)), Tristate::True);
        assert_eq!(set.ask(&relation(RelOp::Lt, 5)), Tristate::False);
        assert_eq!(set.ask(&relation(RelOp::Le, 5)), Tristate::False);
        assert_eq!(set.ask(&relation(RelOp::Eq, 5)), Tristate::False);
        // x > 5 says nothing about x < 7
        assert_eq!(set.ask(&relation(RelOp::Lt, 7)), Tristate::Unknown);
        assert_eq!(set.ask(&relation(RelOp::Gt, 6)), Tristate::Unknown);
    }

    #[test]
    fn bound_truth_table_matches_the_reference_over_witnesses() {
        let ops = [
            RelOp::Eq,
            RelOp::Ne,
            RelOp::Gt,
            RelOp::Ge,
            RelOp::Lt,
            RelOp::Le,
        ];
        let bounds = [4i64, 5, 6];
        let witnesses = reference_witnesses();
        for assumed_op in ops {
            for assumed_bound in bounds {
                let set = AssumptionSet::empty()
                    .add(relation(assumed_op, assumed_bound))
                    .expect("consistent");
                for query_op in ops {
                    for query_bound in bounds {
                        let got = set.ask(&relation(query_op, query_bound));
                        let in_set: Vec<Rat> = witnesses
                            .iter()
                            .filter(|&w| satisfies(w, assumed_op, &Rat::from_i64(assumed_bound)))
                            .cloned()
                            .collect();
                        let all = !in_set.is_empty()
                            && in_set
                                .iter()
                                .all(|w| satisfies(w, query_op, &Rat::from_i64(query_bound)));
                        let none = in_set
                            .iter()
                            .all(|w| !satisfies(w, query_op, &Rat::from_i64(query_bound)));
                        let expected = if all {
                            Tristate::True
                        } else if none {
                            Tristate::False
                        } else {
                            Tristate::Unknown
                        };
                        assert_eq!(
                            got, expected,
                            "assume(x {assumed_op:?} {assumed_bound}), ask(x {query_op:?} {query_bound})"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn property_atoms_follow_the_implication_lattice() {
        let positive = AssumptionSet::empty()
            .add(Assumption::SymbolProperty(x(), SymbolPredicate::Positive))
            .expect("consistent");
        assert_eq!(positive.ask(&relation(RelOp::Gt, 0)), Tristate::True);
        assert_eq!(
            positive.ask(&Assumption::SymbolProperty(
                x(),
                SymbolPredicate::NonNegative
            )),
            Tristate::True
        );
        assert_eq!(
            positive.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::NonZero)),
            Tristate::True
        );
        // Positive does not imply Odd, Even, or a domain
        assert_eq!(
            positive.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::Odd)),
            Tristate::Unknown
        );
        assert_eq!(
            positive.ask(&Assumption::SymbolDomain(x(), Domain::Integer)),
            Tristate::Unknown
        );

        let odd = AssumptionSet::empty()
            .add(Assumption::SymbolProperty(x(), SymbolPredicate::Odd))
            .expect("consistent");
        assert_eq!(
            odd.ask(&Assumption::SymbolDomain(x(), Domain::Integer)),
            Tristate::True
        );
        assert_eq!(
            odd.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::NonZero)),
            Tristate::True
        );

        // Integer does not imply Even/Odd/NonZero, but does imply Real and Rational
        let integer = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Integer))
            .expect("consistent");
        assert_eq!(
            integer.ask(&Assumption::SymbolDomain(x(), Domain::Real)),
            Tristate::True
        );
        assert_eq!(
            integer.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::Odd)),
            Tristate::Unknown
        );
        assert_eq!(
            integer.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::NonZero)),
            Tristate::Unknown
        );
    }

    #[test]
    fn contradiction_detection_is_neither_missed_nor_invented() {
        let ops = [
            RelOp::Eq,
            RelOp::Ne,
            RelOp::Gt,
            RelOp::Ge,
            RelOp::Lt,
            RelOp::Le,
        ];
        let bounds = [4i64, 5, 6];
        let witnesses = reference_witnesses();
        for o1 in ops {
            for b1 in bounds {
                for o2 in ops {
                    for b2 in bounds {
                        let first = AssumptionSet::empty().add(relation(o1, b1));
                        let Ok(first) = first else { continue };
                        let second = first.add(relation(o2, b2));
                        let has_model = witnesses.iter().any(|w| {
                            satisfies(w, o1, &Rat::from_i64(b1))
                                && satisfies(w, o2, &Rat::from_i64(b2))
                        });
                        assert_eq!(
                            second.is_err(),
                            !has_model,
                            "({o1:?} {b1}) & ({o2:?} {b2}) should {}",
                            if has_model {
                                "be accepted"
                            } else {
                                "be refused"
                            }
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn property_and_relation_atoms_contradict_in_both_orders() {
        let property_first = AssumptionSet::empty()
            .add(Assumption::SymbolProperty(x(), SymbolPredicate::Positive))
            .expect("consistent")
            .add(relation(RelOp::Le, 0));
        assert!(property_first.is_err());
        let relation_first = AssumptionSet::empty()
            .add(relation(RelOp::Le, 0))
            .expect("consistent")
            .add(Assumption::SymbolProperty(x(), SymbolPredicate::Positive));
        let refusal = relation_first.expect_err("contradiction");
        assert_eq!(
            refusal.message,
            "Assumption x is Positive contradicts the existing assumptions (its negation x is NonPositive is provable)."
        );
    }

    #[test]
    fn unsatisfiable_set_answers_unsatisfiable_not_unknown() {
        let set = AssumptionSet::unsatisfiable();
        assert_eq!(set.ask(&relation(RelOp::Gt, 0)), Tristate::Unsatisfiable);
        assert_eq!(
            set.ask(&Assumption::SymbolProperty(x(), SymbolPredicate::Positive)),
            Tristate::Unsatisfiable
        );
        // the sentinel is sticky
        assert!(
            set.add(relation(RelOp::Gt, 0))
                .expect("sticky")
                .is_unsatisfiable()
        );
    }

    #[test]
    fn integer_interval_emptiness_is_detected_but_reals_stay_satisfiable() {
        let integer = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Integer))
            .expect("consistent");
        // no integer lies strictly between 1/2 and 1
        let bounded = integer
            .add(Assumption::SymbolRelation(
                x(),
                RelOp::Gt,
                rational_expr(half()),
            ))
            .expect("1/2 < x");
        let refusal = bounded
            .add(Assumption::SymbolRelation(x(), RelOp::Lt, Expr::num(1)))
            .expect_err("no integer in (1/2, 1)");
        assert_eq!(
            refusal.message,
            "Assumption x < 1 leaves no integer satisfying the existing assumptions."
        );

        // the same fractional bounds over the reals are satisfiable
        let reals = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Real))
            .expect("consistent");
        let bounded = reals
            .add(Assumption::SymbolRelation(
                x(),
                RelOp::Gt,
                rational_expr(half()),
            ))
            .expect("1/2 < x");
        assert!(
            bounded
                .add(Assumption::SymbolRelation(x(), RelOp::Lt, Expr::num(1)))
                .is_ok()
        );

        // an open integral boundary still leaves its neighbour reachable
        let above = integer
            .add(Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(1)))
            .expect("x > 1");
        assert!(
            above
                .add(Assumption::SymbolRelation(x(), RelOp::Le, Expr::num(2)))
                .is_ok()
        );
        // and a unit interval is contradictory
        let unit = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Integer))
            .expect("consistent")
            .add(Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(0)))
            .expect("x > 0");
        assert!(
            unit.add(Assumption::SymbolRelation(x(), RelOp::Lt, Expr::num(1)))
                .is_err()
        );
    }

    #[test]
    fn interval_bounds_with_unevaluable_bound_are_unknown() {
        let set = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Real))
            .expect("consistent");
        let im = Expr::Fun("im".to_string(), vec![Expr::sym("x")]);
        let pi = Expr::Fun("pi".to_string(), Vec::new());
        let minus_pi = Expr::mul(Expr::num(-1), pi.clone());
        let two_pi = Expr::mul(Expr::num(2), pi.clone());
        let inside = Assumption::Interval(im.clone(), Some(minus_pi), true, Some(pi), false);
        assert_eq!(set.ask(&inside), Tristate::True);
        let outside = Assumption::Interval(im.clone(), Some(two_pi), false, None, false);
        assert_eq!(set.ask(&outside), Tristate::False);
        let symbolic =
            Assumption::Interval(im, Some(Expr::sym("a")), true, Some(Expr::sym("b")), false);
        assert_eq!(set.ask(&symbolic), Tristate::Unknown);
    }

    #[test]
    fn interval_condition_im_zero_requires_provably_real() {
        let im = Expr::Fun("im".to_string(), vec![Expr::sym("x")]);
        let interval = Assumption::Interval(im, None, false, Some(Expr::num(1)), false);
        let complex_symbol = AssumptionSet::empty();
        assert_eq!(complex_symbol.ask(&interval), Tristate::Unknown);
        let real = complex_symbol
            .add(Assumption::SymbolDomain(x(), Domain::Real))
            .expect("consistent");
        assert_eq!(real.ask(&interval), Tristate::True);
    }

    #[test]
    fn domain_atoms_merge_to_the_narrowest() {
        let set = AssumptionSet::empty()
            .add(Assumption::SymbolDomain(x(), Domain::Complex))
            .expect("consistent")
            .add(Assumption::SymbolDomain(x(), Domain::Integer))
            .expect("consistent");
        assert_eq!(set.domain_of_symbol("x"), Domain::Integer);
        assert_eq!(
            set.ask(&Assumption::SymbolDomain(x(), Domain::Real)),
            Tristate::True
        );
        assert_eq!(set.domain_of_symbol("unconstrained"), Domain::Complex);
    }

    #[test]
    fn atoms_are_stored_in_canonical_order_and_deduplicated() {
        let set = AssumptionSet::empty()
            .add(Assumption::SymbolRelation(x(), RelOp::Ne, Expr::num(2)))
            .expect("consistent")
            .add(Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(5)))
            .expect("consistent")
            .add(Assumption::SymbolDomain(x(), Domain::Real))
            .expect("consistent")
            .add(Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(0)))
            .expect("consistent")
            .add(Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(0)))
            .expect("consistent");
        let rendered: Vec<String> = set.atoms().iter().map(Assumption::describe).collect();
        assert_eq!(rendered, vec!["x in Real", "x > 5", "x > 0", "x != 2"]);
        assert_eq!(set.atoms().len(), 4);
    }

    #[test]
    fn from_atoms_preserves_the_canonical_order() {
        let set = AssumptionSet::from_atoms(vec![
            Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(5)),
            Assumption::SymbolDomain(x(), Domain::Real),
            Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(0)),
        ]);
        // the two relations tie on the reference's key (their bounds render as the same node
        // kind), so the sort keeps the given order: domain first, then the tied relations.
        let rendered: Vec<String> = set.atoms().iter().map(Assumption::describe).collect();
        assert_eq!(rendered, vec!["x in Real", "x > 5", "x > 0"]);
        // distinct operators order by the operator name, whatever the insertion order
        let by_operator = AssumptionSet::from_atoms(vec![
            Assumption::SymbolRelation(x(), RelOp::Ne, Expr::num(2)),
            Assumption::SymbolRelation(x(), RelOp::Lt, Expr::num(1)),
            Assumption::SymbolRelation(x(), RelOp::Gt, Expr::num(9)),
        ]);
        let rendered: Vec<String> = by_operator
            .atoms()
            .iter()
            .map(Assumption::describe)
            .collect();
        assert_eq!(rendered, vec!["x > 9", "x < 1", "x != 2"]);
    }

    #[test]
    fn real_valued_bounds_participate_in_reasoning() {
        // C# AssumptionSoundnessTests.RealValuedBounds_ParticipateInReasoning: a bound carried
        // as a RealConstant leaf is still an exact decimal, so the bound lattice reads it.
        let set = AssumptionSet::empty()
            .add(Assumption::SymbolRelation(x(), RelOp::Ge, Expr::real("5")))
            .expect("consistent");
        assert_eq!(set.ask(&relation(RelOp::Lt, 5)), Tristate::False);
        assert_eq!(set.ask(&relation(RelOp::Gt, 4)), Tristate::True);
    }

    #[test]
    fn decimal_renderings_parse_to_their_exact_rational_value() {
        assert_eq!(
            decimal_to_rational("3.14")
                .map(|r| r.to_string())
                .as_deref(),
            Some("157/50")
        );
        assert_eq!(
            decimal_to_rational("5").map(|r| r.to_string()).as_deref(),
            Some("5")
        );
        assert_eq!(
            decimal_to_rational("-0.5")
                .map(|r| r.to_string())
                .as_deref(),
            Some("-1/2")
        );
        // a periodic rendering denotes its rational exactly: 0.(3) is 1/3
        assert_eq!(
            decimal_to_rational("0.(3)")
                .map(|r| r.to_string())
                .as_deref(),
            Some("1/3")
        );
        assert!(decimal_to_rational("not a decimal").is_none());
    }

    #[test]
    fn expression_properties_are_proved_from_their_shape() {
        let set = AssumptionSet::empty()
            .add(Assumption::SymbolProperty(x(), SymbolPredicate::NonZero))
            .expect("consistent");
        let abs_x = Expr::Fun("abs".to_string(), vec![Expr::sym("x")]);
        assert_eq!(
            set.ask(&Assumption::ExpressionProperty(
                abs_x,
                SymbolPredicate::NonNegative
            )),
            Tristate::True
        );
        // abs(x) is positive because x is nonzero
        let abs_x = Expr::Fun("abs".to_string(), vec![Expr::sym("x")]);
        assert_eq!(
            set.ask(&Assumption::ExpressionProperty(
                abs_x,
                SymbolPredicate::Positive
            )),
            Tristate::True
        );
        // an expression with no such shape is unknown
        assert_eq!(
            set.ask(&Assumption::ExpressionProperty(
                Expr::num(0),
                SymbolPredicate::NonNegative
            )),
            Tristate::Unknown
        );
    }

    #[test]
    fn provably_real_follows_the_domain_atoms() {
        let empty = AssumptionSet::empty();
        assert!(empty.provably_real(&Expr::num(1)));
        assert!(empty.provably_real(&Expr::Fun("pi".to_string(), Vec::new())));
        assert!(!empty.provably_real(&Expr::Fun("i".to_string(), Vec::new())));
        assert!(!empty.provably_real(&Expr::sym("x")));
        let real = empty
            .add(Assumption::SymbolDomain(x(), Domain::Real))
            .expect("consistent");
        assert!(real.provably_real(&Expr::sym("x")));
        assert!(real.provably_real(&Expr::add(Expr::sym("x"), Expr::num(1))));
        assert!(real.provably_real(&Expr::Fun("abs".to_string(), vec![Expr::sym("x")])));
        assert!(!real.provably_real(&Expr::Fun("sin".to_string(), vec![Expr::sym("x")])));
    }
}
