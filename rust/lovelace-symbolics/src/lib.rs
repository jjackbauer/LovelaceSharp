//! Symbolic expressions: canonical construction, term ordering, printing and differentiation.
//!
//! The rules that matter for parity are the ones a reader can see: an expression is rebuilt into
//! a canonical sum of terms, like terms are collected, and the printer reproduces the documented
//! forms exactly (`y + 2*x`, `x^2 - 2*x + 1`, `-1/2/(x + 1)`).

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

pub mod assumptions;
pub mod cancel;
pub mod groebner;
pub mod rational_functions;
pub mod real_literal;
pub mod series_kink;
pub mod symbolic_matrix;

/// An exact rational coefficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rat {
    num: Int,
    den: Nat,
}

impl Rat {
    /// Zero.
    pub fn zero() -> Rat {
        Rat {
            num: Int::zero(),
            den: Nat::from_u64(1),
        }
    }

    /// One.
    pub fn one() -> Rat {
        Rat {
            num: Int::from_i64(1),
            den: Nat::from_u64(1),
        }
    }

    /// Builds a rational from an i64.
    pub fn from_i64(v: i64) -> Rat {
        Rat {
            num: Int::from_i64(v),
            den: Nat::from_u64(1),
        }
    }

    /// Numerator.
    pub fn numerator(&self) -> &Int {
        &self.num
    }

    /// Denominator.
    pub fn denominator(&self) -> &Nat {
        &self.den
    }

    /// True when the value is zero.
    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    /// True when the value is one.
    pub fn is_one(&self) -> bool {
        !self.num.is_negative() && self.num.to_decimal() == "1" && self.den.to_decimal() == "1"
    }

    /// True when the value is a negative number.
    pub fn is_negative(&self) -> bool {
        self.num.is_negative()
    }

    /// Negation.
    pub fn neg(&self) -> Rat {
        Rat {
            num: self.num.negate(),
            den: self.den.clone(),
        }
    }

    /// Absolute value.
    pub fn abs(&self) -> Rat {
        if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }

    /// Addition.
    pub fn add(&self, other: &Rat) -> Rat {
        let left = self
            .num
            .mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
        let right = other
            .num
            .mul(&Int::from_decimal(&self.den.to_decimal()).unwrap_or_else(Int::zero));
        let den = self.den.mul(&other.den);
        Rat::new(left.add(&right), den)
    }

    /// Multiplication.
    pub fn mul(&self, other: &Rat) -> Rat {
        Rat::new(self.num.mul(&other.num), self.den.mul(&other.den))
    }

    /// Division; `None` when the divisor is zero.
    pub fn div(&self, other: &Rat) -> Option<Rat> {
        if other.is_zero() {
            return None;
        }
        let num = self
            .num
            .mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
        let den = self.den.mul(other.num.magnitude());
        let mut r = Rat::new(num, den);
        if other.is_negative() {
            r = r.neg();
        }
        Some(r)
    }

    /// Raises to an integer power.
    pub fn pow_i64(&self, exp: i64) -> Option<Rat> {
        if exp == 0 {
            return Some(Rat::one());
        }
        if exp < 0 {
            // Negating i64::MIN overflows in debug builds. The reciprocal of base^(2^63) is not
            // representable as a Rat here, so this is a refusal, not a panic.
            let positive = exp.checked_neg()?;
            let base = self.pow_i64(positive)?;
            return Rat::one().div(&base);
        }
        let mut acc = Rat::one();
        for _ in 0..exp {
            acc = acc.mul(self);
        }
        Some(acc)
    }

    fn new(num: Int, den: Nat) -> Rat {
        if num.is_zero() {
            return Rat::zero();
        }
        let mut n = num;
        let mut d = den;
        if let (Some(a), Some(b)) = (n.magnitude().to_u64(), d.to_u64()) {
            let g = gcd_u64(a, b);
            if g > 1 {
                let gn = Nat::from_u64(g);
                n = n.div_rem(&gn).map(|(q, _)| q).unwrap_or(n);
                d = d.div_rem(&gn).map(|(q, _)| q).unwrap_or(d);
            }
        }
        Rat { num: n, den: d }
    }

    /// Builds a rational from an arbitrary-precision numerator and denominator.
    pub fn from_parts(num: Int, den: Nat) -> Rat {
        Rat::new(num, den)
    }

    /// True when this is an integral value.
    pub fn is_integer(&self) -> bool {
        self.den.to_decimal() == "1"
    }

    /// The integer value, when integral.
    pub fn to_i64(&self) -> Option<i64> {
        if !self.is_integer() {
            return None;
        }
        self.num.to_decimal().parse::<i64>().ok()
    }
}

/// Numeric comparison via cross-multiplication. Two integral values compare their
/// numerators directly: the general path rebuilds both denominators as Ints through their
/// decimal rendering, which is the hot spot in the assumption engine's bound reasoning.
impl Ord for Rat {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.is_integer() && other.is_integer() {
            return self.num.cmp(&other.num);
        }
        let left = self
            .num
            .mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
        let right = other
            .num
            .mul(&Int::from_decimal(&self.den.to_decimal()).unwrap_or_else(Int::zero));
        left.cmp(&right)
    }
}

impl PartialOrd for Rat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl std::fmt::Display for Rat {
    /// Renders the coefficient as it appears in a product: `2`, `-1/2`, `3/4`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let one = Nat::from_u64(1);
        if self.den.cmp(&one) == Ordering::Equal {
            return f.write_str(&self.num.to_decimal());
        }
        f.write_str(&format!(
            "{}/{}",
            self.num.to_decimal(),
            self.den.to_decimal()
        ))
    }
}

/// A relation operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelOp {
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `>`
    Gt,
    /// `<`
    Lt,
    /// `>=`
    Ge,
    /// `<=`
    Le,
}

impl RelOp {
    fn text(self) -> &'static str {
        match self {
            RelOp::Eq => "==",
            RelOp::Ne => "!=",
            RelOp::Gt => ">",
            RelOp::Lt => "<",
            RelOp::Ge => ">=",
            RelOp::Le => "<=",
        }
    }
}

/// A symbolic expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    /// A rational constant.
    Num(Rat),
    /// A named symbol.
    Sym(String),
    /// A sum of terms.
    Add(Vec<Expr>),
    /// A product of factors.
    Mul(Vec<Expr>),
    /// A power.
    Pow(Box<Expr>, Box<Expr>),
    /// A function application.
    Fun(String, Vec<Expr>),
    /// An explicit quotient. Unlike a negative power, this node is never collapsed: the
    /// reference publishes forms such as 0/(x*y), which a canonical product would destroy.
    Div(Box<Expr>, Box<Expr>),
    /// A relation between two expressions.
    Rel(RelOp, Box<Expr>, Box<Expr>),
    /// A real constant carried as the DECIMAL rendering of the value it denotes. This is the
    /// reference's RealConstantExpr leaf: a value that crossed into the expression from the
    /// numeric tier, so it denotes a decimal approximation rather than an exact rational and is
    /// outside the exact basis by its kind (see is_exact). A decimal is stored as text because
    /// that is the whole of what the leaf means: its digits are its identity, its rendering and
    /// its ordering key.
    Real(String),
}

impl Expr {
    /// A numeric constant.
    pub fn num(v: i64) -> Expr {
        Expr::Num(Rat::from_i64(v))
    }

    /// A real constant carried as its decimal rendering (the RealConstant leaf).
    pub fn real(text: &str) -> Expr {
        Expr::Real(text.to_string())
    }

    /// A rational constant.
    pub fn rational(num: &Int, den: &Nat) -> Expr {
        Expr::Num(Rat::new(num.clone(), den.clone()))
    }

    /// A symbol.
    pub fn sym(name: &str) -> Expr {
        Expr::Sym(name.to_string())
    }

    /// Canonical sum.
    // The expression constructors below are the reference's `Expr.Add`/`Multiply`/`Negate`
    // operations: they canonicalise two owned operands and are called in associated form by the
    // evaluator. They are not the `std::ops` sugar (an `Add` impl would take `self`/`rhs` and
    // suggest a different contract), so the ported names stay.
    #[allow(clippy::should_implement_trait)]
    pub fn add(a: Expr, b: Expr) -> Expr {
        let mut terms: Vec<Expr> = Vec::new();
        let mut coefficient = Rat::zero();
        for t in [a, b] {
            match t {
                Expr::Add(inner) => terms.extend(inner),
                other => terms.push(other),
            }
        }
        // Collect like terms: strip the numeric coefficient, key on the rest.
        let mut collected: Vec<(Expr, Rat)> = Vec::new();
        for term in terms {
            let (rest, coef) = split_coefficient(term);
            if rest.is_none() {
                coefficient = coefficient.add(&coef);
                continue;
            }
            let rest = rest.unwrap();
            if let Some(slot) = collected.iter_mut().find(|(e, _)| *e == rest) {
                slot.1 = slot.1.add(&coef);
            } else {
                collected.push((rest, coef));
            }
        }
        // A zero coefficient over a remainder that is not a defined, finite value must not
        // vanish: dropping it would define 0*inf (and inf - inf, whose coefficient is zero over
        // the remainder inf) as the definite value 0. The reference's Constructors.Add keeps it
        // and prints the canonical indeterminate product (Constructors.HasUndefinedRisk).
        collected.retain(|(rest, c)| !c.is_zero() || rest.has_undefined_risk());
        let mut terms: Vec<Expr> = collected
            .into_iter()
            .map(|(rest, coef)| scale(rest, &coef))
            .collect();
        terms.sort_by(term_order);
        if !coefficient.is_zero() || terms.is_empty() {
            // Display order keeps the exact constant after the higher-degree symbolic terms
            // (x - 1, not -1 + x), but BEFORE a Real leaf: both are numeric constants and the
            // exact one comes first, which is what the reference prints for 1 + pi(30).
            let first_real = terms.iter().position(|t| matches!(t, Expr::Real(_)));
            match first_real {
                Some(index) => terms.insert(index, Expr::Num(coefficient)),
                None => terms.push(Expr::Num(coefficient)),
            }
        }
        if terms.len() == 1 {
            return terms.pop().unwrap();
        }
        Expr::Add(terms)
    }

    /// Canonical product.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(a: Expr, b: Expr) -> Expr {
        let mut factors: Vec<Expr> = Vec::new();
        for f in [a, b] {
            match f {
                Expr::Mul(inner) => factors.extend(inner),
                other => factors.push(other),
            }
        }
        // A zero factor over a remainder that is not a defined, finite value must stay visible:
        // 0*inf is indeterminate, and 0/x is undefined at 0 while 0 is defined. The reference's
        // Constructors.Multiply keeps the zero coefficient in exactly this case
        // (Constructors.HasUndefinedRisk) and folds to 0 otherwise.
        let any_undefined_risk = factors.iter().any(|f| f.has_undefined_risk());
        let mut coefficient = Rat::one();
        let mut rest: Vec<Expr> = Vec::new();
        for f in factors {
            match f {
                Expr::Num(n) => coefficient = coefficient.mul(&n),
                other => rest.push(other),
            }
        }
        if coefficient.is_zero() && !any_undefined_risk {
            return Expr::num(0);
        }
        // Combine powers of the same base with the reference's DEFINEDNESS-preserving split
        // (Constructors.MultiplyImpl): nonnegative and negative INTEGER exponents accumulate
        // separately per base, and a positive and a negative exponent merge ONLY when the
        // combined exponent stays negative (the pole set is unchanged). x^2 * x^-1 therefore
        // stays a product of two powers instead of cancelling to x, which would define the
        // product at 0 where x^-1 is undefined. A power with a non-integer exponent is a BARE
        // factor over that power (its own exponent counts 1).
        let mut powers: Vec<(Expr, Rat, Rat)> = Vec::new();
        for factor in rest {
            // A sqrt IS the half power (the reference's Function constructor maps sqrt to a
            // Power), so sqrt(g)*sqrt(g) still combines to g rather than staying a function
            // application; the half exponent is non-integer, so it is a bare factor here.
            let factor = match factor {
                Expr::Fun(name, args) if name == "sqrt" && args.len() == 1 => Expr::Pow(
                    Box::new(args.into_iter().next().expect("one argument")),
                    Box::new(Expr::Num(Rat::new(Int::from_i64(1), Nat::from_u64(2)))),
                ),
                other => other,
            };
            match factor {
                Expr::Pow(base, exponent) => match exponent.as_ref() {
                    Expr::Num(n) if n.is_integer() => {
                        let slot = match powers.iter_mut().find(|(b, _, _)| *b == *base) {
                            Some(slot) => slot,
                            None => {
                                powers.push(((*base).clone(), Rat::zero(), Rat::zero()));
                                powers.last_mut().expect("just pushed")
                            }
                        };
                        if n.is_negative() {
                            slot.2 = slot.2.add(n);
                        } else {
                            slot.1 = slot.1.add(n);
                        }
                    }
                    _ => {
                        let whole = Expr::Pow(base, exponent);
                        match powers.iter_mut().find(|(b, _, _)| *b == whole) {
                            Some(slot) => slot.1 = slot.1.add(&Rat::one()),
                            None => powers.push((whole, Rat::one(), Rat::zero())),
                        }
                    }
                },
                other => match powers.iter_mut().find(|(b, _, _)| *b == other) {
                    Some(slot) => slot.1 = slot.1.add(&Rat::one()),
                    None => powers.push((other, Rat::one(), Rat::zero())),
                },
            }
        }
        let mut combined: Vec<Expr> = Vec::new();
        for (base, positive, negative) in powers {
            if positive.is_zero() && negative.is_zero() {
                continue;
            }
            let sum = positive.add(&negative);
            if !positive.is_zero() && negative.is_negative() && sum.is_negative() {
                combined.push(Expr::pow(base, Expr::Num(sum)));
                continue;
            }
            if !positive.is_zero() {
                combined.push(if positive.is_one() {
                    base.clone()
                } else {
                    Expr::pow(base.clone(), Expr::Num(positive))
                });
            }
            if !negative.is_zero() {
                combined.push(Expr::pow(base, Expr::Num(negative)));
            }
        }
        // A combined power can fold to a NUMBER (i*i = -1, or a numeric power). Fold it into
        // the coefficient instead of emitting it as a factor: 2*i * 3*i is -6, not 6*-1.
        let mut folded: Vec<Expr> = Vec::with_capacity(combined.len());
        for e in combined.drain(..) {
            match e {
                Expr::Num(n) => coefficient = coefficient.mul(&n),
                other => folded.push(other),
            }
        }
        if coefficient.is_zero() && !any_undefined_risk {
            return Expr::num(0);
        }
        let mut combined = folded;
        combined.retain(|e| !matches!(e, Expr::Num(n) if n.is_one()));
        combined.sort_by(factor_order);
        if !coefficient.is_one() || combined.is_empty() {
            combined.insert(0, Expr::Num(coefficient));
        }
        if combined.len() == 1 {
            return combined.pop().unwrap();
        }
        Expr::Mul(combined)
    }

    /// Canonical power.
    pub fn pow(base: Expr, exp: Expr) -> Expr {
        if exp == Expr::num(0) {
            return Expr::num(1);
        }
        if exp == Expr::num(1) {
            return base;
        }
        match (&base, &exp) {
            (Expr::Num(b), Expr::Num(e)) => {
                if let Some(v) = rational_pow(b, e) {
                    return Expr::Num(v);
                }
            }
            (Expr::Pow(inner_base, inner_exp), _) => {
                // (x^a)^b = x^(a*b) only when BOTH exponents are numeric constants: a
                // symbolic inner exponent (x^y)^2 must never be replaced by x^(2*y), which
                // loses y. The reference kernel states the same rule on its Power constructor.
                if let (Expr::Num(e), Expr::Num(inner)) = (&exp, inner_exp.as_ref()) {
                    if e.denominator().to_decimal() == "1" {
                        let _ = inner;
                        let combined = Expr::mul((**inner_exp).clone(), exp.clone());
                        return Expr::pow((**inner_base).clone(), combined);
                    }
                }
            }
            _ => {}
        }
        // The imaginary unit with an INTEGER exponent folds exactly: i^0 = 1, i^1 = i,
        // i^2 = -1, i^3 = -i, and the cycle repeats for negative exponents. This is a named
        // constant (Fun("i", [])), never a user symbol of the same spelling.
        if let Expr::Fun(name, args) = &base {
            if name == "i" && args.is_empty() {
                if let Expr::Num(e) = &exp {
                    if e.is_integer() {
                        if let Some(k) = e.to_i64() {
                            return match k.rem_euclid(4) {
                                0 => Expr::num(1),
                                1 => base.clone(),
                                2 => Expr::num(-1),
                                _ => Expr::neg(imaginary_unit()),
                            };
                        }
                    }
                }
            }
        }
        // A NEGATIVE rational base under a half-integer exponent has an exact principal value
        // (the imaginary branch): (-4)^(1/2) = 2*i, (-4)^(-1/2) = -i/2. Integer exponents were
        // answered above; every other denominator stays unevaluated.
        if let (Expr::Num(b), Expr::Num(e)) = (&base, &exp) {
            if b.is_negative() {
                if let Some(v) = principal_half_power(b, e) {
                    return v;
                }
            }
        }
        // A rational base with a positive UNIT-fraction exponent folds when the root is exact:
        // 4^(1/2) = 2, 1/4^(1/2) = 1/2 and (-8)^(1/3) = -2 (the reference's odd-denominator
        // real-root shortcut). An inexact root (2^(1/2), (-8)^(2/3)) and every non-unit fraction
        // still stay a power. This runs AFTER the negative-base half power, so an even denominator
        // over a negative base is never answered by the real-root shortcut.
        if let (Expr::Num(b), Expr::Num(e)) = (&base, &exp) {
            if !e.is_negative()
                && e.denominator().to_decimal() != "1"
                && e.numerator().to_decimal() == "1"
            {
                if let Some(n) = e.denominator().to_u64() {
                    if let Some(root) = rational_root(b, n) {
                        return Expr::Num(root);
                    }
                }
            }
        }
        Expr::Pow(Box::new(base), Box::new(exp))
    }

    /// Negation.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(e: Expr) -> Expr {
        Expr::mul(Expr::num(-1), e)
    }

    /// Subtraction.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(a: Expr, b: Expr) -> Expr {
        Expr::add(a, Expr::neg(b))
    }

    /// Division.
    #[allow(clippy::should_implement_trait)]
    pub fn div(a: Expr, b: Expr) -> Expr {
        Expr::mul(a, Expr::pow(b, Expr::num(-1)))
    }

    /// A relation between two expressions.
    pub fn relation(op: RelOp, a: Expr, b: Expr) -> Expr {
        Expr::Rel(op, Box::new(a), Box::new(b))
    }

    /// A piecewise expression: one or more (guard, value) branches plus the otherwise value.
    /// The node is the reference's PiecewiseExpr - the calculus needs it because a derivative at
    /// a kink (abs, floor) is only a branch of a piecewise answer, never a smooth value.
    pub fn piecewise(branches: Vec<(Expr, Expr)>, otherwise: Expr) -> Expr {
        let mut args: Vec<Expr> = Vec::with_capacity(2 * branches.len() + 1);
        for (guard, value) in branches {
            args.push(guard);
            args.push(value);
        }
        args.push(otherwise);
        Expr::Fun("piecewise".to_string(), args)
    }

    /// An UNEVALUATED derivative of the body with respect to the named variables (the
    /// reference's DerivativeExpr). It is the honest answer where no closed form exists:
    /// d floor(x)/dx is never the smooth 0.
    pub fn derivative(body: Expr, vars: Vec<String>) -> Expr {
        let mut args: Vec<Expr> = Vec::with_capacity(vars.len() + 1);
        args.push(body);
        for v in vars {
            args.push(Expr::sym(&v));
        }
        Expr::Fun("derivative".to_string(), args)
    }

    /// The branches and otherwise value of a piecewise node.
    pub fn as_piecewise(&self) -> Option<(Vec<(Expr, Expr)>, Expr)> {
        let Expr::Fun(name, args) = self else {
            return None;
        };
        if name != "piecewise" || args.len() < 3 || args.len() % 2 == 0 {
            return None;
        }
        let mut branches = Vec::with_capacity(args.len() / 2);
        for pair in args[..args.len() - 1].chunks(2) {
            branches.push((pair[0].clone(), pair[1].clone()));
        }
        Some((branches, args[args.len() - 1].clone()))
    }

    /// The body and differentiation variables of a derivative node.
    pub fn as_derivative(&self) -> Option<(Expr, Vec<String>)> {
        let Expr::Fun(name, args) = self else {
            return None;
        };
        if name != "derivative" || args.len() < 2 {
            return None;
        }
        let mut vars = Vec::with_capacity(args.len() - 1);
        for a in &args[1..] {
            match a {
                Expr::Sym(s) => vars.push(s.clone()),
                _ => return None,
            }
        }
        Some((args[0].clone(), vars))
    }

    /// True when the expression is free of `name` - the reference's Calculus.FreeOf. It walks
    /// EVERY operand, including a piecewise's guards and an unevaluated derivative's body, so a
    /// piecewise is never mistaken for a constant.
    pub fn free_of(&self, name: &str) -> bool {
        !self.contains(name)
    }

    /// A square root. A negative exact value has an exact imaginary principal value
    /// (sqrt(-4) = 2*i, sqrt(-2) = i*sqrt(2)), never a real approximation and never a
    /// real sign flip: the real-domain contract lives on Real::sqrt, not on this constructor.
    pub fn sqrt(e: Expr) -> Expr {
        if let Expr::Num(n) = &e {
            if n.is_negative() {
                let radical = rational_sqrt(&n.abs());
                return Expr::mul(imaginary_unit(), radical);
            }
            return rational_sqrt(n);
        }
        Expr::Fun("sqrt".to_string(), vec![e])
    }

    /// True when the expression contains the given symbol.
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Expr::Num(_) | Expr::Real(_) => false,
            Expr::Sym(s) => s == name,
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|e| e.contains(name)),
            Expr::Pow(a, b) => a.contains(name) || b.contains(name),
            // A derivative's bound variables are not occurrences of the symbol: only its body
            // is (free_of(diff(y^2, x), x) is true).
            Expr::Fun(fname, _) if fname == "derivative" => match self.as_derivative() {
                Some((body, _)) => body.contains(name),
                None => false,
            },
            Expr::Fun(_, args) => args.iter().any(|e| e.contains(name)),
            Expr::Div(a, b) => a.contains(name) || b.contains(name),
            Expr::Rel(_, a, b) => a.contains(name) || b.contains(name),
        }
    }

    /// True when the expression contains no symbols.
    pub fn is_constant(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Real(_) => true,
            Expr::Sym(_) => false,
            Expr::Add(v) | Expr::Mul(v) => v.iter().all(|e| e.is_constant()),
            Expr::Pow(a, b) => a.is_constant() && b.is_constant(),
            Expr::Fun(_, args) => args.iter().all(|e| e.is_constant()),
            Expr::Div(a, b) => a.is_constant() && b.is_constant(),
            Expr::Rel(_, a, b) => a.is_constant() && b.is_constant(),
        }
    }

    /// True when a function with this name appears anywhere.
    pub fn contains_fun(&self, name: &str) -> bool {
        match self {
            Expr::Fun(n, args) => n == name || args.iter().any(|a| a.contains_fun(name)),
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|a| a.contains_fun(name)),
            Expr::Pow(a, b) => a.contains_fun(name) || b.contains_fun(name),
            Expr::Div(a, b) => a.contains_fun(name) || b.contains_fun(name),
            Expr::Rel(_, a, b) => a.contains_fun(name) || b.contains_fun(name),
            _ => false,
        }
    }

    /// True when the named symbol appears anywhere.
    pub fn contains_sym(&self, name: &str) -> bool {
        self.contains(name)
    }

    /// True when the value carries the imaginary unit: the NAMED CONSTANT `i` (the reference's
    /// node, and what this crate's constructors build) or a user symbol spelled `i` (the spelling
    /// the solver used before the named constant was adopted). The real-domain filter and the
    /// exactness classifier must see BOTH, or a complex root crosses as a real one.
    pub fn has_imaginary_unit(&self) -> bool {
        self.contains_fun("i") || self.contains_sym("i")
    }

    /// True when a power with a NON-INTEGER rational exponent appears anywhere: such a value is
    /// algebraic over the rationals (2^(1/3), sqrt(2)), so a solver solution carrying one is
    /// AlgebraicExact, never Exact. Only literal numeric exponents count; a symbolic exponent is
    /// not a claim about algebraicity.
    pub fn has_algebraic_power(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Real(_) | Expr::Sym(_) => false,
            Expr::Pow(base, exp) => {
                matches!(exp.as_ref(), Expr::Num(n) if !n.is_integer())
                    || base.has_algebraic_power()
                    || exp.has_algebraic_power()
            }
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|e| e.has_algebraic_power()),
            Expr::Fun(_, args) => args.iter().any(|a| a.has_algebraic_power()),
            Expr::Div(a, b) => a.has_algebraic_power() || b.has_algebraic_power(),
            Expr::Rel(_, a, b) => a.has_algebraic_power() || b.has_algebraic_power(),
        }
    }

    /// True when a power anywhere in the expression may be undefined because of a negative
    /// exponent (or an exponent that is not a provably nonnegative rational). The reference's
    /// `Constructors.HasPoleRisk`.
    pub fn has_pole_risk(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Real(_) | Expr::Sym(_) => false,
            Expr::Pow(base, exp) => match exp.as_ref() {
                Expr::Num(n) => n.is_negative() || base.has_pole_risk(),
                _ => true,
            },
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|e| e.has_pole_risk()),
            Expr::Fun(_, args) => args.iter().any(|a| a.has_pole_risk()),
            Expr::Div(a, b) => a.has_pole_risk() || b.has_pole_risk(),
            Expr::Rel(_, a, b) => a.has_pole_risk() || b.has_pole_risk(),
        }
    }

    /// True when the expression may fail to denote a defined, finite value: the symbolic constant
    /// infinity, an explicit quotient, or a pole risk. The identities `a - a = 0` and `0*a = 0`
    /// hold only for a defined, finite `a`, so the zero-coefficient and zero-factor folds must
    /// consult this guard: over infinity both forms are indeterminate (inf - inf, 0*inf) and folding
    /// them yields a definite wrong value. Pi and e are finite and are deliberately NOT hazards.
    /// This is the reference's `Constructors.HasUndefinedRisk`.
    pub fn has_undefined_risk(&self) -> bool {
        match self {
            Expr::Num(_) | Expr::Real(_) | Expr::Sym(_) => false,
            Expr::Fun(name, args) => {
                (name == "inf" && args.is_empty()) || args.iter().any(|a| a.has_undefined_risk())
            }
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|e| e.has_undefined_risk()),
            Expr::Pow(base, exp) => match exp.as_ref() {
                Expr::Num(n) => {
                    n.is_negative() || base.has_pole_risk() || base.has_undefined_risk()
                }
                _ => true,
            },
            // An explicit quotient is undefined wherever its denominator vanishes; the reference
            // has no Div node (a division is a negative power) and reaches the same verdict there.
            Expr::Div(..) => true,
            Expr::Rel(_, a, b) => a.has_undefined_risk() || b.has_undefined_risk(),
        }
    }

    /// Total degree in the named symbol (0 when absent).
    ///
    /// Every arithmetic step SATURATES: an exponent is a user-sized integer (a Natural can be
    /// larger than i64), so a polynomial-looking expression can ask for a degree outside i64
    /// (x^(2^63-1), (x^2 - 1)^(2^63-1)). The degree is a PRINT ORDER / dispatch hint, not a
    /// value, and no caller may turn the hint into a panic. A negative degree (a quotient) also
    /// saturates instead of wrapping; callers that need a real polynomial reject it.
    pub fn degree_in(&self, name: &str) -> i64 {
        match self {
            Expr::Num(_) | Expr::Real(_) => 0,
            Expr::Sym(s) => i64::from(s == name),
            Expr::Add(v) => v.iter().map(|e| e.degree_in(name)).max().unwrap_or(0),
            Expr::Mul(v) => v
                .iter()
                .map(|e| e.degree_in(name))
                .fold(0i64, |acc, d| acc.saturating_add(d)),
            Expr::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
                (inner, Expr::Num(n)) if n.denominator().to_decimal() == "1" => {
                    let exponent = n
                        .numerator()
                        .to_decimal()
                        .parse::<i64>()
                        .unwrap_or(if n.is_negative() { i64::MIN } else { i64::MAX });
                    inner.degree_in(name).saturating_mul(exponent)
                }
                (inner, _) => inner.degree_in(name),
            },
            Expr::Fun(_, args) => args.iter().map(|e| e.degree_in(name)).max().unwrap_or(0),
            Expr::Div(a, b) => a.degree_in(name).saturating_sub(b.degree_in(name)),
            Expr::Rel(_, a, b) => a.degree_in(name).max(b.degree_in(name)),
        }
    }

    /// Renders the expression in the documented form.
    fn render(&self) -> String {
        match self {
            Expr::Num(n) => n.to_string(),
            Expr::Real(text) => text.clone(),
            Expr::Sym(s) => s.clone(),
            Expr::Add(terms) => {
                // Printing.OrderTermsForDisplay: a sum carrying the series O(...) term renders in
                // ascending powers around the expansion point (the O-term last); a single-symbol
                // polynomial renders in descending degree; EVERY other sum renders in the kernel's
                // canonical order. The stored term order is a degree order, so it is only right for
                // the polynomial case - a quotient such as 5 - (4*x - 5)/(x^2 - 1) must not be
                // re-ordered to put the constant last.
                let ordered = display_term_order(terms);
                // A NEGATIVE leading constant renders last as a subtraction (x - 1, never -1 + x).
                if let Some(Expr::Num(lead)) = ordered.first() {
                    if lead.is_negative() {
                        let magnitude = lead.abs();
                        if ordered.len() == 1 {
                            return format!("-{magnitude}");
                        }
                        let mut out = String::new();
                        let (text, negative) = signed_text(&ordered[1]);
                        if negative {
                            out.push('-');
                        }
                        out.push_str(&text);
                        for t in &ordered[2..] {
                            let (text, negative) = signed_text(t);
                            if negative {
                                out.push_str(" - ");
                            } else {
                                out.push_str(" + ");
                            }
                            out.push_str(&text);
                        }
                        out.push_str(" - ");
                        out.push_str(&magnitude.to_string());
                        return out;
                    }
                }
                let mut out = String::new();
                for (i, t) in ordered.iter().enumerate() {
                    let (text, negative) = signed_text(t);
                    if i == 0 {
                        if negative {
                            out.push('-');
                        }
                        out.push_str(&text);
                    } else if negative {
                        out.push_str(" - ");
                        out.push_str(&text);
                    } else {
                        out.push_str(" + ");
                        out.push_str(&text);
                    }
                }
                out
            }
            Expr::Mul(factors) => {
                // The reference stores a product's factors in the kernel's canonical order and
                // its printer renders that order. Builders that append factors directly (factor()
                // is one) can leave the stored order different from the canonical one, so the
                // DISPLAY sorts exactly as canonical() does: x*(x - 1)*(x + 1), not (x - 1)*x*...
                let mut ordered = factors.clone();
                ordered.sort_by(canonical_order);
                let display = if ordered.len() == 1 {
                    ordered[0].clone()
                } else {
                    Expr::Mul(ordered.clone())
                };
                if let Some(Expr::Num(n)) = ordered.first() {
                    if n.is_negative() {
                        let mut rest = ordered.clone();
                        rest[0] = Expr::Num(n.abs());
                        let inner = if rest.len() == 1 {
                            rest[0].clone()
                        } else {
                            Expr::Mul(rest)
                        };
                        return format!("-{}", product_text(&inner));
                    }
                }
                product_text(&display)
            }
            Expr::Pow(_, _) => power_text(self),
            Expr::Fun(name, args) => {
                if args.is_empty() {
                    // Constants such as inf print as their name.
                    return name.clone();
                }
                if name == "and" || name == "or" {
                    let joiner = if name == "and" { " and " } else { " or " };
                    // Conjuncts render in a stable order even though the store keeps them as written.
                    let mut parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                    parts.sort();
                    return parts.join(joiner);
                }
                if name == "not" {
                    // The empty-argument case returned above (a Function with no arguments prints
                    // as its name), so args[0] is in range here even for canonical_parse input.
                    return format!("not {}", args[0]);
                }
                // The series tail: O(x^3) at zero, O((x - 1)^3) around a nonzero point, O(x) for a
                // degree-one tail (the reference's OrderShape rule).
                if name == "order" && args.len() == 3 {
                    if let Some((var, point, degree)) = series_kink::order_parts(self) {
                        let zero_point = matches!(point, Expr::Num(n) if n.is_zero());
                        let base = if zero_point {
                            var.to_string()
                        } else {
                            Expr::sub(var.clone(), point.clone()).to_string()
                        };
                        if degree == 1 {
                            return format!("O({base})");
                        }
                        let body = if zero_point {
                            base
                        } else {
                            format!("({base})")
                        };
                        return format!("O({body}^{degree})");
                    }
                }
                // An algebraic number: the root index is part of the printed value, and the
                // defining polynomial prints as a polynomial (rootof(x^4 - x^2 - 1, 0)).
                if name == "rootof" && args.len() == 2 {
                    return format!("rootof({}, {})", args[0], args[1]);
                }
                // A piecewise prints each branch as "value if guard" and the otherwise value
                // last: piecewise(sign(x) if x != 0, diff(abs(x), x)).
                if name == "piecewise" {
                    if let Some((branches, otherwise)) = self.as_piecewise() {
                        let mut parts: Vec<String> = branches
                            .iter()
                            .map(|(guard, value)| format!("{} if {}", value, guard))
                            .collect();
                        parts.push(otherwise.to_string());
                        return format!("piecewise({})", parts.join(", "));
                    }
                }
                // An unevaluated derivative prints as the diff call it denotes.
                if name == "derivative" {
                    if let Some((body, vars)) = self.as_derivative() {
                        return format!("diff({}, {})", body, vars.join(", "));
                    }
                }
                let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                format!("{}({})", name, parts.join(", "))
            }
            Expr::Div(a, b) => {
                let top = match a.as_ref() {
                    Expr::Add(_) | Expr::Mul(_) => format!("({a})"),
                    other => other.to_string(),
                };
                let bottom = match b.as_ref() {
                    Expr::Add(_) | Expr::Mul(_) => format!("({b})"),
                    other => other.to_string(),
                };
                format!("{}/{}", top, bottom)
            }
            Expr::Rel(op, a, b) => format!("{} {} {}", a, op.text(), b),
        }
    }
}

/// The degree of a SINGLE-SYMBOL monomial, or None when the term is not one. This is the
/// reference's Printing.TryMonomial: a symbol, a power of a symbol with a rational exponent, or a
/// product of those and numeric constants (a negative exponent counts, so x^-1 is a monomial).
fn monomial_degree(e: &Expr) -> Option<(String, Rat)> {
    match e {
        Expr::Sym(s) => Some((s.clone(), Rat::one())),
        Expr::Pow(base, exp) => match (base.as_ref(), exp.as_ref()) {
            (Expr::Sym(s), Expr::Num(n)) => Some((s.clone(), n.clone())),
            _ => None,
        },
        Expr::Mul(factors) => {
            let mut symbol: Option<String> = None;
            let mut total = Rat::zero();
            for factor in factors {
                match factor {
                    Expr::Num(_) | Expr::Real(_) => {}
                    Expr::Sym(s) => {
                        if let Some(found) = &symbol {
                            if found != s {
                                return None;
                            }
                        } else {
                            symbol = Some(s.clone());
                        }
                        total = total.add(&Rat::one());
                    }
                    Expr::Pow(base, exp) => {
                        let (Expr::Sym(s), Expr::Num(n)) = (base.as_ref(), exp.as_ref()) else {
                            return None;
                        };
                        if let Some(found) = &symbol {
                            if found != s {
                                return None;
                            }
                        } else {
                            symbol = Some(s.clone());
                        }
                        total = total.add(n);
                    }
                    _ => return None,
                }
            }
            symbol.map(|s| (s, total))
        }
        _ => None,
    }
}

/// Printing.OrderTermsForDisplay, exactly: series order first, then the single-symbol polynomial
/// order (descending degree, canonical tie-break), then canonical order for everything else.
fn display_term_order(terms: &[Expr]) -> Vec<Expr> {
    let canonical = || {
        let mut ordered = terms.to_vec();
        ordered.sort_by(canonical_order);
        ordered
    };
    if terms.is_empty() || terms.len() > 64 {
        return terms.to_vec();
    }
    if terms
        .iter()
        .any(|t| matches!(t, Expr::Fun(name, args) if name == "order" && args.len() == 3))
    {
        return series_kink::series_display_order(terms);
    }
    let mut symbol: Option<String> = None;
    for term in terms {
        if matches!(term, Expr::Num(_) | Expr::Real(_)) {
            continue;
        }
        let Some((found, _)) = monomial_degree(term) else {
            return canonical();
        };
        match &symbol {
            None => symbol = Some(found),
            Some(existing) if existing != &found => return canonical(),
            _ => {}
        }
    }
    let Some(symbol) = symbol else {
        return canonical();
    };
    let degree = |term: &Expr| {
        monomial_degree(term)
            .filter(|(s, _)| s == &symbol)
            .map(|(_, d)| d)
            .unwrap_or_else(Rat::zero)
    };
    let mut ordered = terms.to_vec();
    ordered.sort_by(|a, b| {
        degree(b)
            .cmp(&degree(a))
            .then_with(|| canonical_order(a, b))
    });
    ordered
}

fn signed_text(e: &Expr) -> (String, bool) {
    match e {
        Expr::Num(n) if n.is_negative() => (n.abs().to_string(), true),
        Expr::Mul(factors) => {
            // The sign-carrying coefficient is the canonically FIRST factor (the display sorts a
            // product's factors); reading the stored order could miss a coefficient that a direct
            // builder appended last and print " + -2*x" instead of " - 2*x".
            let mut ordered = factors.clone();
            ordered.sort_by(canonical_order);
            let factors = &ordered;
            if let Some(Expr::Num(n)) = factors.first() {
                if n.is_negative() {
                    let mut rest: Vec<Expr> = Vec::with_capacity(factors.len());
                    rest.push(Expr::Num(n.abs()));
                    rest.extend_from_slice(&factors[1..]);
                    // A bare coefficient of one is not printed: -y, not -1*y.
                    if rest.len() > 1 && matches!(rest.first(), Some(Expr::Num(x)) if x.is_one()) {
                        rest.remove(0);
                    }
                    let inner = match rest.len() {
                        0 => Expr::Num(n.abs()),
                        1 => rest[0].clone(),
                        _ => Expr::Mul(rest),
                    };
                    // The right operand of a minus binds tighter than a sum, so a magnitude that
                    // IS a sum keeps its parentheses: x - (y - a) must never render as x - y - a,
                    // which denotes a different expression.
                    let text = inner.to_string();
                    let text = match &inner {
                        Expr::Add(_) | Expr::Rel(..) => format!("({text})"),
                        _ => text,
                    };
                    return (text, true);
                }
            }
            (e.to_string(), false)
        }
        _ => (e.to_string(), false),
    }
}

fn product_text(e: &Expr) -> String {
    // The reference's ONE product decomposition (Printing.ShapeProduct): an optional leading
    // rational coefficient with its sign split off, the factors that stay in the numerator, and
    // the bases of the factors raised to EXACTLY -1 (the denominators). A factor raised to -2 is
    // a numerator power (x*y^-2), not a second division.
    let factors = match e {
        Expr::Mul(f) => f.clone(),
        other => vec![other.clone()],
    };
    let mut coefficient = Rat::one();
    let mut negative = false;
    let mut first = 0;
    if let Some(Expr::Num(n)) = factors.first() {
        coefficient = n.clone();
        first = 1;
        if coefficient.is_negative() {
            negative = true;
            coefficient = coefficient.abs();
        }
    }
    let minus_one = Rat::from_i64(-1);
    let mut numerators: Vec<Expr> = Vec::new();
    let mut denominators: Vec<Expr> = Vec::new();
    for f in &factors[first..] {
        match f {
            Expr::Pow(base, exp) if matches!(exp.as_ref(), Expr::Num(n) if *n == minus_one) => {
                denominators.push((**base).clone())
            }
            other => numerators.push(other.clone()),
        }
    }
    // The numerator side, with a sum delimited: (x + y)*z, never x + y*z.
    let mut numerator_parts: Vec<String> = Vec::new();
    for f in &numerators {
        match f {
            Expr::Add(_) => numerator_parts.push(format!("({})", f)),
            other => numerator_parts.push(other.to_string()),
        }
    }
    // With no divisor the coefficient stays ONE text, its own denominator included:
    // (1/2)*i renders 1/2*i, not 1*i/2.
    if denominators.is_empty() {
        let body = numerator_parts.join("*");
        let mut text = if coefficient.is_one() {
            if body.is_empty() {
                "1".to_string()
            } else {
                body
            }
        } else if body.is_empty() {
            coefficient.to_string()
        } else {
            format!("{}*{}", coefficient, body)
        };
        if negative {
            text = format!("-{text}");
        }
        return text;
    }
    // A rational coefficient folds into the numerator side instead of producing a num*coef/den
    // chain; its denominator is one of the divisors (-1/2/(x + 1)).
    let mut parts: Vec<String> = Vec::new();
    let mut divisor_texts: Vec<String> = Vec::new();
    if !coefficient.is_one() {
        parts.push(coefficient.numerator().to_decimal());
        if !coefficient.is_integer() {
            divisor_texts.push(coefficient.denominator().to_decimal());
        }
    }
    parts.extend(numerator_parts);
    // The reference's factors are stored in the kernel's canonical order; this model's product
    // keeps a DISPLAY order, so the denominator bases are put back in canonical order before they
    // are rendered (y before (z/w) in x/y/(z/w); w, y, z in x/w/y/z).
    denominators.sort_by(canonical_order);
    for b in &denominators {
        // The ONE operand rule the reference applies to a divisor: a divisor binds at the
        // division's precedence, so a product (x/(y*z)), a sum and a quotient are delimited while a
        // power (x/y^2) and a symbol are not.
        let text = b.to_string();
        let needs_delimiter = matches!(b, Expr::Add(_) | Expr::Mul(_) | Expr::Div(_, _))
            || matches!(b, Expr::Num(n) if n.is_negative() || !n.is_integer());
        divisor_texts.push(if needs_delimiter {
            format!("({text})")
        } else {
            text
        });
    }
    // ONE division per denominator factor: a value with y and z as two separate denominator
    // factors divides twice (x/y/z); a single divisor that IS the product y*z divides once and
    // carries the pair (x/(y*z)). Writing both as x/(y*z) is what let the two values share one
    // text, and the text meant the other one.
    let mut text = if parts.is_empty() {
        "1".to_string()
    } else {
        parts.join("*")
    };
    for divisor in divisor_texts {
        text.push('/');
        text.push_str(&divisor);
    }
    if negative {
        text = format!("-{text}");
    }
    text
}

fn power_text(e: &Expr) -> String {
    match e {
        Expr::Pow(base, exp) => {
            // Radical rendering (human form only): x^(1/2) is sqrt(x) and x^(-1/2) is
            // 1/sqrt(x). A parens pair around the call's argument already delimits the base.
            if let Expr::Num(n) = exp.as_ref() {
                if n.denominator().to_decimal() == "2" {
                    if n.numerator().to_decimal() == "1" {
                        return format!("sqrt({})", base);
                    }
                    if n.numerator().to_decimal() == "-1" {
                        return format!("1/sqrt({})", base);
                    }
                }
            }
            // A power used as a BASE must be parenthesised: x^y^2 right-associates to
            // x^(y^2), so Pow(Pow(x, y), 2) needs (x^y)^2 to re-parse as itself. A negative or
            // non-integer numeric base is textually looser than the superscript for the same
            // reason: -2^x is -(2^x) and 1/2^x is 1/(2^x), not (-2)^x and (1/2)^x; a quotient
            // base (x/y)^2 is a division at the power's level too.
            let base_text = match base.as_ref() {
                Expr::Add(_) | Expr::Mul(_) | Expr::Pow(_, _) | Expr::Div(_, _) => {
                    format!("({})", base)
                }
                Expr::Num(n) if n.is_negative() || !n.is_integer() => {
                    format!("({})", n)
                }
                other => other.to_string(),
            };
            // The exponent's own grouping: a power exponent keeps its parentheses (x^(y^2)) and a
            // NON-INTEGER rational would otherwise be read as a division at the power's level
            // (x^(1/2)). A NEGATIVE INTEGER exponent needs none: the parser's power tail accepts
            // a signed exponent, so x^-2 is exactly x^(-2).
            let exp_text = match exp.as_ref() {
                Expr::Add(_) | Expr::Mul(_) | Expr::Pow(_, _) => {
                    format!("({})", exp)
                }
                Expr::Num(n) if !n.is_integer() => format!("({})", n),
                other => other.to_string(),
            };
            format!("{}^{}", base_text, exp_text)
        }
        other => other.to_string(),
    }
}

fn split_coefficient(e: Expr) -> (Option<Expr>, Rat) {
    match e {
        Expr::Num(n) => (None, n),
        Expr::Mul(factors) => {
            let mut coef = Rat::one();
            let mut rest = Vec::new();
            for f in factors {
                match f {
                    Expr::Num(n) => coef = coef.mul(&n),
                    other => rest.push(other),
                }
            }
            if rest.is_empty() {
                return (None, coef);
            }
            let expr = if rest.len() == 1 {
                rest.pop().unwrap()
            } else {
                Expr::Mul(rest)
            };
            (Some(expr), coef)
        }
        other => (Some(other), Rat::one()),
    }
}

fn scale(e: Expr, coef: &Rat) -> Expr {
    if coef.is_one() {
        return e;
    }
    match e {
        // A coefficient over a product JOINS the product's factors: nesting a second Mul inside
        // the first makes the inner product an atomic base, so the product algebra never combines
        // it ((i*sqrt(3))^2 stays a power of a product instead of folding to -3, and
        // expand(subs(x^3 - 1, x, cardano_root)) never reaches 0). The reference's Add scales a
        // term through its Multiply constructor, which is flat for the same reason.
        Expr::Mul(mut factors) => {
            factors.insert(0, Expr::Num(coef.clone()));
            Expr::Mul(factors)
        }
        other => Expr::Mul(vec![Expr::Num(coef.clone()), other]),
    }
}

/// Orders the factors inside a product: constants, then symbols, then functions and powers;
/// the degree plays no part here (`x*exp(x)` is not reordered by depth).
pub fn factor_order(a: &Expr, b: &Expr) -> Ordering {
    let ra = kind_rank(a);
    let rb = kind_rank(b);
    if ra != rb {
        return ra.cmp(&rb);
    }
    a.to_string().cmp(&b.to_string())
}

/// Orders terms for printing: higher degree first, then by kind, then lexicographically.
pub fn term_order(a: &Expr, b: &Expr) -> Ordering {
    let da: i64 = a.degree_in_total();
    let db: i64 = b.degree_in_total();
    if da != db {
        return db.cmp(&da);
    }
    let ra = kind_rank(a);
    let rb = kind_rank(b);
    if ra != rb {
        return ra.cmp(&rb);
    }
    a.to_string().cmp(&b.to_string())
}

fn kind_rank(e: &Expr) -> u8 {
    match e {
        // The exact numeric constants come first, the Real leaf next (it is a numeric constant
        // too, but outside the exact basis), then the symbolic nodes.
        Expr::Num(_) => 0,
        Expr::Real(_) => 1,
        Expr::Sym(_) => 2,
        Expr::Fun(_, _) => 3,
        Expr::Pow(_, _) => 4,
        Expr::Mul(_) => 5,
        Expr::Add(_) => 6,
        Expr::Div(_, _) => 4,
        Expr::Rel(_, _, _) => 7,
    }
}

impl Expr {
    fn degree_in_total(&self) -> i64 {
        match self {
            Expr::Num(_) | Expr::Real(_) => 0,
            Expr::Sym(_) => 1,
            Expr::Add(v) => v.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            // A product is as deep as its deepest factor: x*exp(x) ranks with exp(x), which is
            // what the documented ordering shows.
            Expr::Mul(v) => v.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            Expr::Pow(b, e) => match e.as_ref() {
                Expr::Num(n) if n.denominator().to_decimal() == "1" => {
                    // Same user-sized exponent contract as degree_in: the depth decides term
                    // ORDER only, so saturation is the correct failure mode, never a panic.
                    let exponent = n
                        .numerator()
                        .to_decimal()
                        .parse::<i64>()
                        .unwrap_or(if n.is_negative() { i64::MIN } else { i64::MAX });
                    b.degree_in_total().saturating_mul(exponent)
                }
                _ => b.degree_in_total(),
            },
            Expr::Fun(_, args) => args.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            Expr::Div(a, b) => a.degree_in_total().saturating_sub(b.degree_in_total()),
            Expr::Rel(_, a, b) => a.degree_in_total().max(b.degree_in_total()),
        }
    }
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a.max(1)
}

fn rational_pow(base: &Rat, exp: &Rat) -> Option<Rat> {
    if exp.denominator().to_decimal() != "1" {
        return None;
    }
    let e = exp.numerator().to_decimal().parse::<i64>().ok()?;
    base.pow_i64(e)
}

/// The imaginary unit as the reference represents it: a named constant, not a user symbol, so
/// i^2 folds and a user symbol literally named i stays an ordinary symbol.
fn imaginary_unit() -> Expr {
    Expr::Fun("i".to_string(), Vec::new())
}

/// The exact square root of a NON-NEGATIVE rational: a rational when both numerator and
/// denominator are perfect squares, otherwise the radical expression `sqrt(q)`.
fn rational_sqrt(q: &Rat) -> Expr {
    if let (Some(n), Some(d)) = (
        perfect_square(q.numerator().magnitude()),
        perfect_square(q.denominator()),
    ) {
        if let Some(n_int) = Int::from_decimal(&n.to_string()) {
            return Expr::Num(Rat::new(n_int, Nat::from_u64(d)));
        }
    }
    Expr::Fun("sqrt".to_string(), vec![Expr::Num(q.clone())])
}

/// An exact n-th root of a non-negative u128, when one exists: the integer root test the
/// reference's RationalRoot performs before its unit-fraction power folds to a rational.
fn exact_root_u128(value: u128, n: u32) -> Option<u128> {
    if n == 0 {
        return None;
    }
    if value <= 1 {
        return Some(value);
    }
    // Binary search on [1, value]; checked_pow stops the search at the first overflow, which is
    // already past the root.
    let (mut low, mut high) = (1u128, value);
    while low <= high {
        let mid = low + (high - low) / 2;
        match mid.checked_pow(n) {
            Some(power) if power == value => return Some(mid),
            Some(power) if power < value => low = mid + 1,
            _ => high = mid - 1,
        }
    }
    None
}

/// The exact rational n-th root of q, when q is a perfect n-th power: 4 -> 2, 1/4 -> 1/2,
/// -8 -> -2 (an odd n only; an even n of a negative value has no real root and is left alone).
fn rational_root(q: &Rat, n: u64) -> Option<Rat> {
    if !(2..=1024).contains(&n) {
        return None;
    }
    let numerator = q.numerator().magnitude().to_u64()? as u128;
    let denominator = q.denominator().to_u64()? as u128;
    let root_numerator = exact_root_u128(numerator, n as u32)?;
    let root_denominator = exact_root_u128(denominator, n as u32)?;
    if root_denominator == 0 {
        return None;
    }
    let signed_numerator = if q.is_negative() {
        if n % 2 == 0 {
            return None;
        }
        format!("-{root_numerator}")
    } else {
        root_numerator.to_string()
    };
    let num = Int::from_decimal(&signed_numerator)?;
    let den = Nat::from_decimal(&root_denominator.to_string())?;
    Some(Rat::new(num, den))
}

/// The principal value of a NEGATIVE rational base raised to a half-integer exponent:
/// (-a)^(m/2) = a^(m/2)*i^m for a > 0, an exact imaginary multiple of a radical. Mirrors the
/// reference kernel's PrincipalSquareRootPower: the scale is the exact integer power of a, the
/// radical folds to a rational for a perfect square, and i^m is i or -i by the parity of (m-1)/2.
fn principal_half_power(base: &Rat, exponent: &Rat) -> Option<Expr> {
    if exponent.denominator().to_decimal() != "2" {
        return None;
    }
    let q = base.abs();
    let m = exponent.numerator();
    let (k, _) = m.sub(&Int::from_i64(1)).div_rem_signed(&Int::from_i64(2))?;
    let k_i64 = k.to_decimal().parse::<i64>().ok()?;
    // A conservative bound: a scale this large is not a documented case and must not hang the
    // evaluator; the caller then keeps the power unevaluated.
    if !(-100_000..=100_000).contains(&k_i64) {
        return None;
    }
    let scale = q.pow_i64(k_i64)?;
    let radical = rational_sqrt(&q);
    let even = k
        .div_rem_signed(&Int::from_i64(2))
        .map(|(_, r)| r.is_zero())
        .unwrap_or(false);
    let unit = if even {
        imaginary_unit()
    } else {
        Expr::neg(imaginary_unit())
    };
    Some(Expr::mul(Expr::mul(Expr::Num(scale), radical), unit))
}

impl From<Rat> for Expr {
    fn from(r: Rat) -> Expr {
        Expr::Num(r)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

// ---------------------------------------------------------------------------
// LaTeX rendering — the SAME expression model and the SAME precedence table as
// the infix form, a different notation. Every precedence decision goes through
// latex_prec/latex_delimit and every structural decision (what a fraction is,
// what a subtracted term is, which powers are radicals) through the helpers the
// infix arm calls; only the notation is new.
// ---------------------------------------------------------------------------

const LATEX_OR_PREC: i32 = -1;
const LATEX_AND_PREC: i32 = 0;
const LATEX_RELATION_PREC: i32 = 0;
const LATEX_ADD_PREC: i32 = 1;
const LATEX_MUL_PREC: i32 = 2;
const LATEX_POW_PREC: i32 = 3;
const LATEX_ATOM_PREC: i32 = 4;
/// The loosest rendering context: a child that LaTeX's own braces already group.
const LATEX_ROOT_PREC: i32 = 0;
/// Function arguments are already delimited by a call's parentheses and commas.
const LATEX_ARG_PREC: i32 = 1;
/// The magnitude of a subtracted term binds strictly tighter than an additive sum.
const LATEX_SUBTRAHEND_PREC: i32 = 2;

impl Expr {
    /// Renders the expression as LaTeX source: the same model, the same precedence table and the
    /// same structural decisions as the infix form, only the notation differs.
    pub fn latex(&self) -> String {
        latex_render(self, LATEX_ROOT_PREC)
    }
}

/// The precedence of a node in the ONE table both infix renderings share.
fn latex_prec(e: &Expr) -> i32 {
    match e {
        Expr::Fun(name, _) if name == "or" => LATEX_OR_PREC,
        Expr::Fun(name, _) if name == "and" => LATEX_AND_PREC,
        Expr::Rel(_, _, _) => LATEX_RELATION_PREC,
        Expr::Add(_) => LATEX_ADD_PREC,
        Expr::Mul(_) | Expr::Div(_, _) => LATEX_MUL_PREC,
        Expr::Pow(_, _) => LATEX_POW_PREC,
        _ => LATEX_ATOM_PREC,
    }
}

/// The ONE parenthesis decision, applied in LaTeX notation: a node whose own precedence is
/// `node_prec` is wrapped exactly when the context demands something that binds tighter.
fn latex_delimit(text: String, parent_prec: i32, node_prec: i32) -> String {
    if parent_prec > node_prec {
        format!("\\left({}\\right)", text)
    } else {
        text
    }
}

/// The precedence of rendered text AS WRITTEN, in the same table the model is classified in.
/// Only the text's own top level is read: a grouping construct hides everything inside it, a
/// control word is a single token, and a top-level space is the loosest operator of all.
fn latex_text_prec(text: &str) -> i32 {
    let chars: Vec<char> = text.chars().collect();
    let mut prec = LATEX_ATOM_PREC;
    let mut depth: i32 = 0;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let mut j = i + 1;
            if j < chars.len() && chars[j].is_alphabetic() {
                let start = j;
                while j < chars.len() && chars[j].is_alphabetic() {
                    j += 1;
                }
                let word: String = chars[start..j].iter().collect();
                if word == "left" || word == "right" {
                    if j >= chars.len() {
                        return LATEX_ADD_PREC;
                    }
                    if word == "left" {
                        depth += 1;
                    } else if depth == 0 {
                        return LATEX_ADD_PREC;
                    } else {
                        depth -= 1;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            // a control symbol is one literal character (\{ \} \# \% \_ \&), never a group
            i = j + 1;
            continue;
        }
        if depth == 0 {
            match c {
                '+' | '-' | '=' | '<' | '>' | '!' | ',' | ';' | '&' | '|' => {
                    prec = prec.min(LATEX_ADD_PREC)
                }
                '*' | '/' => prec = prec.min(LATEX_MUL_PREC),
                '^' => prec = prec.min(LATEX_POW_PREC),
                _ if c.is_whitespace() => prec = prec.min(LATEX_ADD_PREC),
                _ => {}
            }
        }
        if c == '(' || c == '{' || c == '[' {
            depth += 1;
        } else if c == ')' || c == '}' || c == ']' {
            if depth == 0 {
                return LATEX_ADD_PREC;
            }
            depth -= 1;
        }
        i += 1;
    }
    if depth == 0 { prec } else { LATEX_ADD_PREC }
}

/// The ONE operand rule asked of a rendered text: a text used as an operand of the operator that
/// binds at `operator_prec` is delimited unless it binds strictly tighter. The text matters as
/// well as the node, because a constant's spelling need not be an atom.
fn latex_needs_operand_delimiter(operator_prec: i32, operand: &Expr, rendered: &str) -> bool {
    latex_prec(operand) <= operator_prec || latex_text_prec(rendered) <= operator_prec
}

fn latex_rational(value: &Rat) -> String {
    if value.is_integer() {
        return value.to_string();
    }
    let abs = value.abs();
    let body = format!(
        "\\frac{{{}}}{{{}}}",
        abs.numerator().to_decimal(),
        abs.denominator().to_decimal()
    );
    if value.is_negative() {
        format!("-{}", body)
    } else {
        body
    }
}

/// The LaTeX spelling of a symbol NAME. A name is opaque: it is rendered as literal characters,
/// never as structure (x_1 is x\_1, not a subscript), and all ten characters LaTeX gives a meaning
/// of its own are escaped with the LaTeX2e spellings.
fn latex_name(name: &str) -> String {
    if !name.chars().any(|c| {
        matches!(
            c,
            '#' | '$' | '%' | '&' | '_' | '{' | '}' | '~' | '^' | '\\'
        )
    }) {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len() + 8);
    for ch in name.chars() {
        match ch {
            '#' => out.push_str("\\#"),
            '$' => out.push_str("\\$"),
            '%' => out.push_str("\\%"),
            '&' => out.push_str("\\&"),
            '_' => out.push_str("\\_"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '~' => out.push_str("\\textasciitilde{}"),
            '^' => out.push_str("\\textasciicircum{}"),
            '\\' => out.push_str("\\textbackslash{}"),
            other => out.push(other),
        }
    }
    out
}

/// The LaTeX spelling of a function name: the control sequences TeX already defines keep their
/// spelling, everything else is an \operatorname.
fn latex_call_name(name: &str) -> String {
    match name {
        "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "exp" | "log" | "min" | "max"
        | "det" | "lim" => format!("\\{}", name),
        _ => format!("\\operatorname{{{}}}", name),
    }
}

/// The product decomposition both infix renderings use: an optional leading rational coefficient
/// with its sign split off, the factors that stay in the numerator, and the bases of the factors
/// raised to -1 (the denominator).
struct LatexProductShape {
    coefficient: Option<Rat>,
    negative: bool,
    numerators: Vec<Expr>,
    denominators: Vec<Expr>,
}

/// The kind rank of the reference kernel's TermOrder, used to put the factors of a product in
/// the order the reference printer sees: Symbol before NamedConstant before Function before
/// Power before Multiply before Add.
fn latex_term_rank(e: &Expr) -> u8 {
    match e {
        Expr::Num(_) | Expr::Real(_) => 0,
        Expr::Sym(_) => 1,
        Expr::Fun(name, args)
            if args.is_empty() && matches!(name.as_str(), "pi" | "e" | "i" | "inf") =>
        {
            2
        }
        Expr::Fun(_, _) => 3,
        Expr::Pow(_, _) => 4,
        Expr::Mul(_) | Expr::Div(_, _) => 5,
        Expr::Add(_) => 6,
        Expr::Rel(_, _, _) => 7,
    }
}

fn latex_seq_cmp(a: &[Expr], b: &[Expr]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let c = latex_term_cmp(x, y);
        if c != Ordering::Equal {
            return c;
        }
    }
    a.len().cmp(&b.len())
}

/// The reference kernel's total order over canonical expressions, to the depth the LaTeX
/// printer needs: a Power compares its BASE before its exponent, which is what puts y^-1 before
/// (z*w^-1)^-1 in a denominator sequence.
fn latex_term_cmp(a: &Expr, b: &Expr) -> Ordering {
    let ra = latex_term_rank(a);
    let rb = latex_term_rank(b);
    if ra != rb {
        return ra.cmp(&rb);
    }
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => x.cmp(y),
        (Expr::Sym(x), Expr::Sym(y)) => x.cmp(y),
        (Expr::Fun(nx, ax), Expr::Fun(ny, ay)) => nx.cmp(ny).then_with(|| latex_seq_cmp(ax, ay)),
        (Expr::Pow(bx, ex), Expr::Pow(by, ey)) => {
            latex_term_cmp(bx, by).then_with(|| latex_term_cmp(ex, ey))
        }
        (Expr::Mul(xs), Expr::Mul(ys)) => latex_seq_cmp(xs, ys),
        (Expr::Div(xn, xd), Expr::Div(yn, yd)) => {
            latex_term_cmp(xn, yn).then_with(|| latex_term_cmp(xd, yd))
        }
        (Expr::Add(xs), Expr::Add(ys)) => latex_seq_cmp(xs, ys),
        (Expr::Rel(ox, lx, rx), Expr::Rel(oy, ly, ry)) => (*ox as u8)
            .cmp(&(*oy as u8))
            .then_with(|| latex_term_cmp(lx, ly))
            .then_with(|| latex_term_cmp(rx, ry)),
        _ => Ordering::Equal,
    }
}

fn latex_shape_product(factors: &[Expr]) -> LatexProductShape {
    // The reference printer sees the reference kernel's canonical factor order. The Rust product
    // keeps the same factors, so sorting them by the same total order before shaping makes the
    // fraction nesting independent of the crate's own factor_order helper.
    let mut sorted: Vec<Expr> = factors.to_vec();
    sorted.sort_by(latex_term_cmp);
    let factors = &sorted[..];
    let mut coefficient = None;
    let mut negative = false;
    let mut first = 0;
    if let Some(Expr::Num(rc0)) = factors.first() {
        coefficient = Some(rc0.clone());
        first = 1;
        if rc0.is_negative() {
            negative = true;
            coefficient = Some(rc0.abs());
        }
    }
    let mut numerators = Vec::new();
    let mut denominators = Vec::new();
    for f in &factors[first..] {
        match f {
            Expr::Pow(base, exponent) if matches!(exponent.as_ref(), Expr::Num(n) if *n == Rat::from_i64(-1)) => {
                denominators.push((**base).clone())
            }
            other => numerators.push(other.clone()),
        }
    }
    LatexProductShape {
        coefficient,
        negative,
        numerators,
        denominators,
    }
}

/// A negative constant term, as its positive magnitude.
fn latex_negative_constant(t: &Expr) -> Option<Expr> {
    match t {
        Expr::Num(n) if n.is_negative() => Some(Expr::Num(n.abs())),
        _ => None,
    }
}

/// A term that renders as " - <positive-magnitude term>", as that magnitude.
fn latex_negative_term(t: &Expr) -> Option<Expr> {
    if let Some(mag) = latex_negative_constant(t) {
        return Some(mag);
    }
    if let Expr::Mul(factors) = t {
        if let Some(Expr::Num(rc)) = factors.first() {
            if rc.is_negative() {
                let rest = &factors[1..];
                let pos = rc.abs();
                if pos.is_one() {
                    return Some(match rest.len() {
                        0 => Expr::Num(pos),
                        1 => rest[0].clone(),
                        _ => Expr::Mul(rest.to_vec()),
                    });
                }
                let mut inner = Vec::with_capacity(rest.len() + 1);
                inner.push(Expr::Num(pos));
                inner.extend_from_slice(rest);
                return Some(Expr::Mul(inner));
            }
        }
    }
    None
}

fn latex_append_signed(out: &mut String, t: &Expr) {
    if let Some(inner) = latex_negative_term(t) {
        out.push_str(" - ");
        out.push_str(&latex_render(&inner, LATEX_SUBTRAHEND_PREC));
    } else {
        out.push_str(" + ");
        out.push_str(&latex_render(t, LATEX_ADD_PREC));
    }
}

/// LaTeX source for an expression of the same model. Where LaTeX already groups a child by
/// construction (the braces of \frac, \sqrt and ^{}) the child is rendered at the loosest
/// precedence and carries no delimiter of its own.
fn latex_render(e: &Expr, parent_prec: i32) -> String {
    match e {
        Expr::Num(n) => latex_rational(n),
        // A real constant renders its decimal digits; they are the value it denotes.
        Expr::Real(text) => text.clone(),
        Expr::Sym(s) => latex_name(s),
        Expr::Add(terms) => {
            if terms.is_empty() {
                return String::new();
            }
            let mut out = String::new();
            if let Some(lead_mag) = latex_negative_constant(&terms[0]) {
                if terms.len() == 1 {
                    return latex_delimit(
                        format!("-{}", latex_render(&lead_mag, LATEX_ADD_PREC)),
                        parent_prec,
                        LATEX_ADD_PREC,
                    );
                }
                out.push_str(&latex_render(&terms[1], LATEX_ADD_PREC));
                for t in &terms[2..] {
                    latex_append_signed(&mut out, t);
                }
                out.push_str(" - ");
                out.push_str(&latex_render(&lead_mag, LATEX_ADD_PREC));
            } else {
                out.push_str(&latex_render(&terms[0], LATEX_ADD_PREC));
                for t in &terms[1..] {
                    latex_append_signed(&mut out, t);
                }
            }
            latex_delimit(out, parent_prec, LATEX_ADD_PREC)
        }
        Expr::Mul(factors) => {
            let shape = latex_shape_product(factors);
            let mut text;
            if shape.denominators.is_empty() {
                let body = shape
                    .numerators
                    .iter()
                    .map(|f| latex_render(f, LATEX_MUL_PREC))
                    .collect::<Vec<_>>()
                    .join(" \\cdot ");
                text = match &shape.coefficient {
                    Some(c0) if !c0.is_one() => {
                        if shape.numerators.is_empty() {
                            latex_rational(c0)
                        } else {
                            format!("{} \\cdot {}", latex_rational(c0), body)
                        }
                    }
                    _ => {
                        if shape.numerators.is_empty() {
                            "1".to_string()
                        } else {
                            body
                        }
                    }
                };
            } else {
                enum Part {
                    Literal(String),
                    Node(Expr),
                }
                let mut num_parts: Vec<Part> = Vec::new();
                let mut den_parts: Vec<Part> = Vec::new();
                if let Some(c1) = &shape.coefficient {
                    if !c1.is_one() {
                        if c1.is_integer() {
                            num_parts.push(Part::Literal(c1.to_string()));
                        } else {
                            num_parts.push(Part::Literal(c1.numerator().to_decimal()));
                            den_parts.push(Part::Literal(c1.denominator().to_decimal()));
                        }
                    }
                }
                for f in &shape.numerators {
                    num_parts.push(Part::Node(f.clone()));
                }
                for b in &shape.denominators {
                    den_parts.push(Part::Node(b.clone()));
                }
                // The braces of \frac ARE the delimiter of each side, so a side that is a single
                // part is rendered at the loosest precedence and carries no \left...\right of its
                // own; a side of several parts is a text-level \cdot product whose Add factors do
                // need them.
                let side = |parts: &[Part], when_empty: &str| -> String {
                    if parts.is_empty() {
                        return when_empty.to_string();
                    }
                    let prec = if parts.len() > 1 {
                        LATEX_MUL_PREC
                    } else {
                        LATEX_ROOT_PREC
                    };
                    parts
                        .iter()
                        .map(|p| match p {
                            Part::Literal(s) => s.clone(),
                            Part::Node(e) => latex_render(e, prec),
                        })
                        .collect::<Vec<_>>()
                        .join(" \\cdot ")
                };
                let top = side(&num_parts, "1");
                let bottoms: Vec<String> = den_parts
                    .iter()
                    .map(|p| match p {
                        Part::Literal(s) => s.clone(),
                        Part::Node(e) => latex_render(e, LATEX_ROOT_PREC),
                    })
                    .collect();
                // ONE \frac per division, exactly as the infix arm emits one "/" per denominator
                // factor: a value with two separate denominator factors nests.
                text = format!("\\frac{{{}}}{{{}}}", top, bottoms[0]);
                for d in &bottoms[1..] {
                    text = format!("\\frac{{{}}}{{{}}}", text, d);
                }
            }
            if shape.negative {
                text = format!("-{}", text);
            }
            latex_delimit(text, parent_prec, LATEX_MUL_PREC)
        }
        Expr::Pow(base, exponent) => {
            let half = Rat::new(Int::from_i64(1), Nat::from_u64(2));
            let minus_half = Rat::new(Int::from_i64(-1), Nat::from_u64(2));
            if let Expr::Num(n) = exponent.as_ref() {
                if *n == half {
                    return latex_delimit(
                        format!("\\sqrt{{{}}}", latex_render(base, LATEX_ROOT_PREC)),
                        parent_prec,
                        LATEX_POW_PREC,
                    );
                }
                if *n == minus_half {
                    return latex_delimit(
                        format!(
                            "\\frac{{1}}{{\\sqrt{{{}}}}}",
                            latex_render(base, LATEX_ROOT_PREC)
                        ),
                        parent_prec,
                        LATEX_POW_PREC,
                    );
                }
            }
            let mut b = latex_render(base, LATEX_ROOT_PREC);
            if latex_needs_operand_delimiter(LATEX_POW_PREC, base, &b) {
                b = format!("\\left({}\\right)", b);
            }
            let ex = latex_render(exponent, LATEX_ROOT_PREC);
            latex_delimit(format!("{}^{{{}}}", b, ex), parent_prec, LATEX_POW_PREC)
        }
        Expr::Fun(name, args) => match name.as_str() {
            "sqrt" if args.len() == 1 => latex_delimit(
                format!("\\sqrt{{{}}}", latex_render(&args[0], LATEX_ROOT_PREC)),
                parent_prec,
                LATEX_POW_PREC,
            ),
            "abs" if args.len() == 1 => {
                format!("\\left|{}\\right|", latex_render(&args[0], LATEX_ROOT_PREC))
            }
            "and" => latex_delimit(
                args.iter()
                    .map(|a| latex_render(a, LATEX_AND_PREC))
                    .collect::<Vec<_>>()
                    .join(" \\land "),
                parent_prec,
                LATEX_AND_PREC,
            ),
            "or" => latex_delimit(
                args.iter()
                    .map(|a| latex_render(a, LATEX_OR_PREC))
                    .collect::<Vec<_>>()
                    .join(" \\lor "),
                parent_prec,
                LATEX_OR_PREC,
            ),
            "not" if args.len() == 1 => latex_delimit(
                format!("\\neg {}", latex_render(&args[0], LATEX_ATOM_PREC)),
                parent_prec,
                LATEX_ATOM_PREC,
            ),
            "rootof" if args.len() == 2 => format!(
                "\\operatorname{{rootof}}({}, {})",
                latex_render(&args[0], LATEX_ARG_PREC),
                latex_render(&args[1], LATEX_ARG_PREC)
            ),
            _ if args.is_empty() => match name.as_str() {
                "pi" => "\\pi".to_string(),
                "e" => "e".to_string(),
                "i" => "i".to_string(),
                "inf" => "\\infty".to_string(),
                _ => format!("{}()", latex_call_name(name)),
            },
            _ => format!(
                "{}({})",
                latex_call_name(name),
                args.iter()
                    .map(|a| latex_render(a, LATEX_ARG_PREC))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        Expr::Div(a, b) => latex_delimit(
            format!(
                "\\frac{{{}}}{{{}}}",
                latex_render(a, LATEX_ROOT_PREC),
                latex_render(b, LATEX_ROOT_PREC)
            ),
            parent_prec,
            LATEX_MUL_PREC,
        ),
        Expr::Rel(op, a, b) => {
            let symbol = match op {
                RelOp::Eq => " = ",
                RelOp::Ne => " \\ne ",
                RelOp::Lt => " < ",
                RelOp::Le => " \\le ",
                RelOp::Gt => " > ",
                RelOp::Ge => " \\ge ",
            };
            format!(
                "{}{}{}",
                latex_render(a, LATEX_RELATION_PREC),
                symbol,
                latex_render(b, LATEX_RELATION_PREC)
            )
        }
    }
}

/// A truncated series: exponents (which may be negative) mapped to rational coefficients.
pub type Series = Vec<(i64, Rat)>;

fn series_add(a: &Series, b: &Series) -> Series {
    if cancel::check() {
        return a.clone();
    }
    let mut out = a.clone();
    for (e, c) in b {
        match out.iter_mut().find(|(k, _)| k == e) {
            Some(slot) => slot.1 = slot.1.add(c),
            None => out.push((*e, c.clone())),
        }
    }
    out.retain(|(_, c)| !c.is_zero());
    out.sort_by_key(|(e, _)| *e);
    out
}

fn series_mul(a: &Series, b: &Series, order: i64) -> Series {
    if cancel::check() {
        return Vec::new();
    }
    let mut out: Series = Vec::new();
    for (ea, ca) in a {
        for (eb, cb) in b {
            let e = ea + eb;
            if e > order + 8 {
                continue;
            }
            let c = ca.mul(cb);
            match out.iter_mut().find(|(k, _)| *k == e) {
                Some(slot) => slot.1 = slot.1.add(&c),
                None => out.push((e, c)),
            }
        }
    }
    out.retain(|(_, c)| !c.is_zero());
    out.sort_by_key(|(e, _)| *e);
    out
}

fn series_inv(a: &Series, order: i64) -> Option<Series> {
    if cancel::check() || a.is_empty() {
        return None;
    }
    let (e0, c0) = a[0].clone();
    if c0.is_zero() {
        return None;
    }
    // 1/(c0 x^e0) * 1/(1 + u) where u has positive exponents.
    let inv_c0 = Rat::one().div(&c0)?;
    let mut u: Series = a[1..]
        .iter()
        .map(|(e, c)| (e - e0, c.mul(&inv_c0)))
        .collect();
    let mut acc: Series = vec![(0, Rat::one())];
    let mut term: Series = vec![(0, Rat::one())];
    for _ in 0..(order + 8).max(8) {
        term = series_mul(&term, &u, order + 8);
        if term.is_empty() {
            break;
        }
        for (e, c) in &term {
            let target = e;
            match acc.iter_mut().find(|(k, _)| k == target) {
                Some(slot) => slot.1 = slot.1.add(&c.neg()),
                None => acc.push((*target, c.neg())),
            }
        }
        u = term.clone();
    }
    // 1/(c0 x^e0) * sum: the exponent shifts DOWN by e0.
    acc = acc
        .into_iter()
        .map(|(e, c)| (e - e0, c.mul(&inv_c0)))
        .filter(|(_, c)| !c.is_zero())
        .collect();
    acc.sort_by_key(|(e, _)| *e);
    Some(acc)
}

fn series_pow(a: &Series, k: i64, order: i64) -> Option<Series> {
    if k == 0 {
        return Some(vec![(0, Rat::one())]);
    }
    if k < 0 {
        let positive = series_pow(a, -k, order)?;
        return series_inv(&positive, order);
    }
    let mut acc: Series = vec![(0, Rat::one())];
    for _ in 0..k {
        acc = series_mul(&acc, a, order);
    }
    Some(acc)
}

/// Expands an expression in powers of one variable around zero.
pub fn series(e: &Expr, var: &str, order: i64) -> Option<Series> {
    if cancel::check() {
        return None;
    }
    let normalised = strip_div(e);
    series_inner(&normalised, var, order)
}

fn series_inner(e: &Expr, var: &str, order: i64) -> Option<Series> {
    match e {
        Expr::Num(r) => Some(vec![(0, r.clone())]),
        Expr::Sym(s) => {
            if s == var {
                Some(vec![(1, Rat::one())])
            } else {
                None
            }
        }
        Expr::Add(terms) => {
            let mut acc: Series = Vec::new();
            for t in terms {
                acc = series_add(&acc, &series(t, var, order)?);
            }
            Some(acc)
        }
        Expr::Mul(factors) => {
            let mut acc: Series = vec![(0, Rat::one())];
            for f in factors {
                acc = series_mul(&acc, &series(f, var, order)?, order);
            }
            Some(acc)
        }
        Expr::Pow(base, exp) => {
            if let Expr::Num(n) = exp.as_ref() {
                if let Some(k) = n.to_i64() {
                    let inner = series(base, var, order)?;
                    return series_pow(&inner, k, order);
                }
            }
            None
        }
        Expr::Fun(name, args) if args.len() == 1 => {
            if args[0] != Expr::sym(var) {
                return None;
            }
            let mut out: Series = Vec::new();
            match name.as_str() {
                "exp" => {
                    let mut fact = Rat::one();
                    for k in 0..order {
                        if k > 0 {
                            fact = fact.div(&Rat::from_i64(k))?;
                        }
                        out.push((k, fact.clone()));
                    }
                }
                "sin" | "cos" => {
                    let want_odd = name == "sin";
                    let mut fact = Rat::one();
                    for k in 0..order {
                        if k > 0 {
                            fact = fact.div(&Rat::from_i64(k))?;
                        }
                        let odd = k % 2 == 1;
                        if odd != want_odd {
                            continue;
                        }
                        let half = if want_odd { (k - 1) / 2 } else { k / 2 };
                        let sign = if half % 2 == 0 { 1 } else { -1 };
                        out.push((k, if sign > 0 { fact.clone() } else { fact.neg() }));
                    }
                }
                _ => return None,
            }
            out.retain(|(_, c)| !c.is_zero());
            Some(out)
        }
        _ => None,
    }
}

/// Substitutes a symbol with an expression, leaving everything else alone.
pub fn substitute(e: &Expr, var: &str, replacement: &Expr) -> Expr {
    if cancel::check() {
        return e.clone();
    }
    match e {
        Expr::Sym(s) if s == var => replacement.clone(),
        Expr::Sym(_) | Expr::Num(_) | Expr::Real(_) => e.clone(),
        Expr::Add(v) => {
            let mut acc = Expr::num(0);
            for t in v {
                acc = Expr::add(acc, substitute(t, var, replacement));
            }
            acc
        }
        Expr::Mul(v) => {
            let mut acc = Expr::num(1);
            for t in v {
                acc = Expr::mul(acc, substitute(t, var, replacement));
            }
            acc
        }
        Expr::Pow(a, b) => Expr::pow(
            substitute(a, var, replacement),
            substitute(b, var, replacement),
        ),
        Expr::Fun(n, args) => Expr::Fun(
            n.clone(),
            args.iter()
                .map(|a| substitute(a, var, replacement))
                .collect(),
        ),
        Expr::Div(a, b) => Expr::Div(
            Box::new(substitute(a, var, replacement)),
            Box::new(substitute(b, var, replacement)),
        ),
        Expr::Rel(op, a, b) => Expr::Rel(
            *op,
            Box::new(substitute(a, var, replacement)),
            Box::new(substitute(b, var, replacement)),
        ),
    }
}

/// Exact polynomial division of `num` by `den` (both coefficient vectors, index = degree).
pub fn split_quotient(e: &Expr) -> Option<(Expr, Expr)> {
    // A bare reciprocal is a quotient with numerator one.
    if let Expr::Pow(base, exp) = e {
        if let Expr::Num(n) = exp.as_ref() {
            if n.to_string() == "-1" {
                return Some((Expr::num(1), (**base).clone()));
            }
        }
    }
    let factors = match e {
        Expr::Mul(f) => f.clone(),
        _ => return None,
    };
    let mut numerator: Vec<Expr> = Vec::new();
    let mut denominator: Option<Expr> = None;
    for f in factors {
        if let Expr::Pow(base, exp) = &f {
            if let Expr::Num(n) = exp.as_ref() {
                if n.to_string() == "-1" {
                    denominator = Some((**base).clone());
                    continue;
                }
            }
        }
        numerator.push(f);
    }
    let den = denominator?;
    let num = match numerator.len() {
        0 => Expr::num(1),
        1 => numerator.pop().unwrap(),
        _ => Expr::Mul(numerator),
    };
    Some((num, den))
}

/// Partial-fraction decomposition of a rational expression in `var` over simple roots.
///
/// Returns `None` when the expression is not a quotient of univariate polynomials, when the
/// denominator has repeated poles, or when cancellation is disabled.
pub fn apart(e: &Expr, var: &str) -> Option<Expr> {
    if cancel::check() {
        return None;
    }
    let (num_e, den_e) = split_quotient(e)?;
    let nc = poly_coeffs(&num_e, var)?;
    let dc = poly_coeffs(&den_e, var)?;
    if dc.len() < 2 {
        return None;
    }
    let roots = match solve_polynomial(&dc) {
        // Partial fractions need SIMPLE poles: a repeated root would require every power of the
        // factor, which this decomposition does not produce.
        SolveOutcome::Roots(r) if r.iter().all(|root| root.multiplicity == 1) => r,
        _ => return None,
    };
    if roots.len() + 1 != dc.len() {
        return None;
    }
    let deriv: Vec<Rat> = (1..dc.len())
        .map(|i| dc[i].mul(&Rat::from_i64(i as i64)))
        .collect();
    let mut sorted: Vec<Rat> = roots.into_iter().map(|root| root.value).collect();
    sorted.sort();
    let mut terms: Vec<Expr> = Vec::new();
    for r in sorted {
        let num_at = eval_poly(&nc, &r)?;
        let den_at = eval_poly(&deriv, &r)?;
        let residue = num_at.div(&den_at)?;
        let factor = Expr::add(Expr::sym(var), Expr::Num(r.neg()));
        terms.push(Expr::mul(
            Expr::Num(residue),
            Expr::pow(factor, Expr::num(-1)),
        ));
    }
    match terms.len() {
        0 => None,
        1 => terms.pop(),
        _ => Some(Expr::Add(terms)),
    }
}

/// Evaluates a univariate polynomial expression in `var` at the rational point `x`.
///
/// Returns `None` when the expression is not a polynomial in `var` or cancellation is disabled.
pub fn eval_at(e: &Expr, var: &str, x: &Rat) -> Option<Rat> {
    if cancel::check() {
        return None;
    }
    let coeffs = poly_coeffs(e, var)?;
    let mut acc = Rat::zero();
    for c in coeffs.iter().rev() {
        acc = acc.mul(x).add(c);
    }
    Some(acc)
}

/// Exact polynomial division, ascending coefficients: the quotient when `den` divides `num`.
///
/// The divisor is trimmed of high zero coefficients; a zero or empty divisor answers `None`.
pub fn poly_div(num: &[Rat], den: &[Rat]) -> Option<Vec<Rat>> {
    if cancel::check() {
        return None;
    }
    let mut den = den.to_vec();
    while den.len() > 1 && den.last().map(|c| c.is_zero()).unwrap_or(false) {
        den.pop();
    }
    if den.is_empty() || den.last().map(|c| c.is_zero()).unwrap_or(true) {
        return None;
    }
    let mut rem = num.to_vec();
    let ddeg = den.len() - 1;
    if rem.len() < den.len() {
        return Some(vec![Rat::zero()]);
    }
    let mut quot = vec![Rat::zero(); rem.len() - ddeg];
    for i in (ddeg..rem.len()).rev() {
        let factor = rem[i].div(&den[ddeg])?;
        quot[i - ddeg] = factor.clone();
        if factor.is_zero() {
            continue;
        }
        for j in 0..=ddeg {
            let sub = factor.mul(&den[j]);
            rem[i - ddeg + j] = rem[i - ddeg + j].add(&sub.neg());
        }
    }
    while quot.len() > 1 && quot.last().map(|c| c.is_zero()).unwrap_or(false) {
        quot.pop();
    }
    Some(quot)
}

/// Cancels a quotient of univariate polynomials when the division is exact.
pub fn cancel(e: &Expr) -> Option<Expr> {
    if cancel::check() {
        return None;
    }
    let mut symbols: Vec<String> = Vec::new();
    collect_all(&e.clone(), &mut symbols);
    if symbols.len() != 1 {
        return None;
    }
    let var = symbols.first().cloned()?;
    // Recognise num * den^-1.
    let factors = match e {
        Expr::Mul(f) => f.clone(),
        _ => return None,
    };
    let mut numerator: Vec<Expr> = Vec::new();
    let mut denominator: Option<Expr> = None;
    for f in factors {
        match f {
            Expr::Pow(base, exp) => {
                if let Expr::Num(n) = exp.as_ref() {
                    if n.to_string() == "-1" {
                        denominator = Some((*base).clone());
                        continue;
                    }
                }
                numerator.push(Expr::Pow(base, exp));
            }
            other => numerator.push(other),
        }
    }
    let den = denominator?;
    let num = match numerator.len() {
        0 => Expr::num(1),
        1 => numerator.pop().unwrap(),
        _ => Expr::Mul(numerator),
    };
    let nc = poly_coeffs(&num, &var)?;
    let dc = poly_coeffs(&den, &var)?;
    let q = poly_div(&nc, &dc)?;
    Some(poly_to_expr(&q, &var))
}

fn collect_all(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Sym(s) => {
            if !out.iter().any(|x| x == s) {
                out.push(s.clone());
            }
        }
        Expr::Add(v) | Expr::Mul(v) => {
            for x in v {
                collect_all(x, out);
            }
        }
        Expr::Pow(a, b) => {
            collect_all(a, out);
            collect_all(b, out);
        }
        Expr::Fun(_, args) => {
            for a in args {
                collect_all(a, out);
            }
        }
        Expr::Div(a, b) => {
            collect_all(a, out);
            collect_all(b, out);
        }
        Expr::Rel(_, a, b) => {
            collect_all(a, out);
            collect_all(b, out);
        }
        Expr::Real(_) => {}
        Expr::Num(_) => {}
    }
}

/// Rebuilds a polynomial expression from coefficients.
pub fn poly_to_expr(coeffs: &[Rat], var: &str) -> Expr {
    let mut acc = Expr::num(0);
    for (i, c) in coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let term = if i == 0 {
            Expr::Num(c.clone())
        } else if i == 1 {
            Expr::mul(Expr::Num(c.clone()), Expr::sym(var))
        } else {
            Expr::mul(
                Expr::Num(c.clone()),
                Expr::pow(Expr::sym(var), Expr::num(i as i64)),
            )
        };
        acc = Expr::add(acc, term);
    }
    acc
}

/// Distributes products over sums until the expression is a flat sum of products.
pub fn expand(e: &Expr) -> Expr {
    expand_inner(&strip_div(e))
}

fn expand_inner(e: &Expr) -> Expr {
    // A kernel that observed the caller's deadline returns its input and unwinds: the evaluation
    // discards the value and reports the structured Cancelled refusal, so a partial expansion is
    // never observable (Lovelace.Symbolics/Algebra/Expand.cs polls the same way).
    if cancel::check() {
        return e.clone();
    }
    match e {
        Expr::Num(_) | Expr::Real(_) | Expr::Sym(_) => e.clone(),
        Expr::Add(terms) => {
            let mut acc = Expr::num(0);
            for t in terms {
                acc = Expr::add(acc, expand_inner(t));
            }
            acc
        }
        Expr::Mul(factors) => {
            let mut acc = Expr::num(1);
            for f in factors {
                acc = distributive_mul(&acc, &expand_inner(f));
            }
            acc
        }
        Expr::Pow(base, exp) => {
            let b = expand_inner(base);
            if let Expr::Num(n) = exp.as_ref() {
                if n.denominator().to_decimal() == "1" {
                    if let Ok(k) = n.numerator().to_decimal().parse::<u32>() {
                        let mut acc = Expr::num(1);
                        for step in 0..k {
                            if cancel::poll(step as u64) {
                                return acc;
                            }
                            acc = distributive_mul(&acc, &b);
                        }
                        return acc;
                    }
                }
            }
            Expr::pow(b, expand_inner(exp))
        }
        Expr::Fun(name, args) => Expr::Fun(name.clone(), args.iter().map(expand_inner).collect()),
        Expr::Div(a, b) => Expr::Div(Box::new(expand_inner(a)), Box::new(expand_inner(b))),
        Expr::Rel(op, a, b) => Expr::Rel(*op, Box::new(expand_inner(a)), Box::new(expand_inner(b))),
    }
}

fn distributive_mul(a: &Expr, b: &Expr) -> Expr {
    if cancel::check() {
        return a.clone();
    }
    match (a, b) {
        (Expr::Add(xs), _) => {
            let mut acc = Expr::num(0);
            for x in xs {
                acc = Expr::add(acc, distributive_mul(x, b));
            }
            acc
        }
        (_, Expr::Add(ys)) => {
            let mut acc = Expr::num(0);
            for y in ys {
                acc = Expr::add(acc, distributive_mul(a, y));
            }
            acc
        }
        _ => Expr::mul(a.clone(), b.clone()),
    }
}

/// Groups a sum into powers of one variable: `x*y + x + 2` collected in x is `2 + x*(y + 1)`.
pub fn collect(e: &Expr, var: &str) -> Expr {
    if cancel::check() {
        return e.clone();
    }
    let expanded = expand(e);
    let mut coefficients: Vec<(i64, Expr)> = Vec::new();
    let terms = match &expanded {
        Expr::Add(ts) => ts.clone(),
        single => vec![single.clone()],
    };
    for term in terms {
        let degree = term.degree_in(var);
        // A term whose degree does not fit the materialization bound is not something collect
        // can key by a coefficient; publish the expanded expression unchanged (the reference's
        // collect leaves an unexpandable power alone) instead of synthesizing x^<saturated>.
        if !degree_within_materialization(degree) {
            return expanded;
        }
        let coefficient = divide_out(&term, var, degree);
        if let Some(slot) = coefficients.iter_mut().find(|(d, _)| *d == degree) {
            slot.1 = Expr::add(slot.1.clone(), coefficient);
        } else {
            coefficients.push((degree, coefficient));
        }
    }
    coefficients.retain(|(_, c)| *c != Expr::num(0));
    coefficients.sort_by_key(|a| std::cmp::Reverse(a.0));
    // Built directly rather than through the canonical adder: collect publishes the constant
    // first, followed by descending powers.
    let mut terms: Vec<Expr> = Vec::new();
    for (degree, coefficient) in coefficients {
        if degree == 0 {
            terms.insert(0, coefficient);
            continue;
        }
        let power = if degree == 1 {
            Expr::sym(var)
        } else {
            Expr::pow(Expr::sym(var), Expr::num(degree))
        };
        terms.push(Expr::mul(coefficient, power));
    }
    match terms.len() {
        0 => Expr::num(0),
        1 => terms.pop().unwrap(),
        _ => Expr::Add(terms),
    }
}

fn divide_out(term: &Expr, var: &str, degree: i64) -> Expr {
    if degree == 0 {
        return term.clone();
    }
    match term {
        Expr::Mul(factors) => {
            let mut rest = Vec::new();
            let mut removed = 0i64;
            for f in factors {
                // Only the variable itself and an integer power of it are removable. A factor
                // that merely MENTIONS the variable (exp(x), sin(x), log(x)) is not a power of it:
                // stripping it would make x*exp(x) look like x^2 and the solver would answer
                // +-sqrt(c) for x*exp(x) = c. The reference's Polynomial.TryFromExpr refuses such
                // a term; this is that refusal stated on the factor.
                let removable = match f {
                    Expr::Sym(s) => s == var,
                    Expr::Pow(base, exp) => {
                        base.as_ref() == &Expr::sym(var)
                            && matches!(exp.as_ref(), Expr::Num(n) if n.denominator().to_decimal() == "1")
                    }
                    _ => false,
                };
                let d = if removable { f.degree_in(var) } else { 0 };
                if removed < degree && d > 0 && d <= degree - removed {
                    removed += d;
                    continue;
                }
                rest.push(f.clone());
            }
            if removed != degree {
                return Expr::Fun("__opaque".to_string(), Vec::new());
            }
            match rest.len() {
                0 => Expr::num(1),
                1 => rest.pop().unwrap(),
                _ => Expr::Mul(rest),
            }
        }
        Expr::Sym(s) if s == var => Expr::num(1),
        Expr::Pow(base, exp) if base.as_ref() == &Expr::sym(var) => match exp.as_ref() {
            Expr::Num(n) if n.denominator().to_decimal() == "1" => {
                let k = n.to_i64().unwrap_or(0);
                if k == degree {
                    Expr::num(1)
                } else if k > degree {
                    let remaining = k - degree;
                    if remaining == 1 {
                        Expr::sym(var)
                    } else {
                        Expr::pow(Expr::sym(var), Expr::num(remaining))
                    }
                } else {
                    Expr::Fun("__opaque".to_string(), Vec::new())
                }
            }
            _ => Expr::Fun("__opaque".to_string(), Vec::new()),
        },
        other => {
            let _ = other;
            Expr::Fun("__opaque".to_string(), Vec::new())
        }
    }
}

/// The greatest coefficient count `poly_coeffs` will materialize for ONE polynomial. The
/// degree is user-controlled (it comes from an exponent written in the script), so the vector
/// length must be bounded before `resize`: 2^16 coefficients is already a 65 537-entry exact
/// vector, far beyond the degrees the exact solver's Sturm/factorization paths can answer in
/// bounded time, and a larger request is REFUSED as non-polynomial instead of aborting.
const MAX_MATERIALIZED_POLYNOMIAL_DEGREE: usize = 1 << 16;

/// True when `degree` fits the materialization bound in either direction. `collect` uses a
/// signed degree as its key; `poly_coeffs` only accepts non-negative ones.
fn degree_within_materialization(degree: i64) -> bool {
    degree.unsigned_abs() <= MAX_MATERIALIZED_POLYNOMIAL_DEGREE as u64
}

/// The coefficient index for a non-negative degree, or None when the vector would exceed the
/// materialization bound. Never narrows through `as`: a negative or wider-than-usize degree is
/// rejected rather than wrapped.
fn degree_index(degree: i64) -> Option<usize> {
    if degree < 0 {
        return None;
    }
    let index = usize::try_from(degree).ok()?;
    if index > MAX_MATERIALIZED_POLYNOMIAL_DEGREE {
        return None;
    }
    Some(index)
}

/// Extracts polynomial coefficients (index = degree) when the expression is a polynomial in
/// `var` with rational coefficients.
pub fn poly_coeffs(e: &Expr, var: &str) -> Option<Vec<Rat>> {
    if cancel::check() {
        return None;
    }
    let expanded = expand(&strip_div(e));
    let terms = match &expanded {
        Expr::Add(ts) => ts.clone(),
        single => vec![single.clone()],
    };
    let mut coeffs: Vec<Rat> = Vec::new();
    for term in terms {
        let degree = term.degree_in(var);
        if degree < 0 {
            return None;
        }
        // The coefficient vector is indexed by the degree, so a user-sized exponent must be
        // measured against a bound BEFORE the resize: x^(2^63-1) is not a polynomial this kernel
        // will materialize, and asking for it must answer "not polynomial here" rather than
        // aborting on a capacity/allocator failure. None sends the solver to its structured
        // Unevaluated path.
        let index = degree_index(degree)?;
        let coefficient = divide_out(&term, var, degree);
        // A coefficient of a polynomial in \`var\` is free of \`var\` by definition. A factor such as
        // exp(x) or sin(x) counts as one degree (its argument does), so divide_out can strip it as
        // if it were a power of x and leave a NUMERIC coefficient behind — x*exp(x) then looked
        // like x^2 and the solver answered +-sqrt(c). The reference's Polynomial.TryFromExpr
        // refuses a term whose variable-carrying factor is not a power of the variable; this is
        // that refusal stated on the coefficient.
        if coefficient.contains(var) {
            return None;
        }
        let value = match coefficient {
            Expr::Num(n) => n,
            _ => return None,
        };
        if coeffs.len() <= index {
            coeffs.resize(index + 1, Rat::zero());
        }
        coeffs[index] = coeffs[index].add(&value);
    }
    while coeffs.len() > 1 && coeffs.last().map(|c| c.is_zero()).unwrap_or(false) {
        coeffs.pop();
    }
    Some(coeffs)
}

/// One exact rational root of a polynomial, with the multiplicity of that root in the
/// factorization (1 for a simple root). The reference preserves multiplicity from the square-free
/// factorization instead of collapsing a repeated root to a single count-1 entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    /// The root value.
    pub value: Rat,
    /// How many times the factor `(x - value)` divides the polynomial.
    pub multiplicity: u64,
}

impl Root {
    /// A simple root (multiplicity 1).
    pub fn simple(value: Rat) -> Root {
        Root {
            value,
            multiplicity: 1,
        }
    }
}

/// The outcome of solving a polynomial equation.
#[derive(Clone, Debug)]
pub enum SolveOutcome {
    /// Exact rational roots, ascending; a repeated root appears ONCE with its multiplicity.
    Roots(Vec<Root>),
    /// The equation reduces to a false constant.
    NoSolutions,
    /// The equation has roots, but none inside the requested real domain.
    NoRealSolutions,
    /// Exact roots given as expressions (complex or algebraic).
    RootExpressions(Vec<Expr>),
    /// Expression roots plus a count of roots that exist but are not represented.
    RootExpressionsCount(Vec<Expr>, u64),
    /// Some roots were found but not all of them.
    Partial(Vec<Rat>),
    /// No exact method applied.
    Unevaluated,
}

/// Trims a coefficient list to its true degree (a constant zero is [0]).
fn trim_poly(mut p: Vec<Rat>) -> Vec<Rat> {
    while p.len() > 1 && p.last().map(|c| c.is_zero()).unwrap_or(false) {
        p.pop();
    }
    if p.is_empty() {
        p.push(Rat::zero());
    }
    p
}

/// The formal derivative of an ascending-coefficient polynomial.
fn poly_derivative(p: &[Rat]) -> Vec<Rat> {
    if p.len() <= 1 {
        return vec![Rat::zero()];
    }
    let mut out = Vec::with_capacity(p.len() - 1);
    for (i, c) in p.iter().enumerate().skip(1) {
        out.push(c.mul(&Rat::from_i64(i as i64)));
    }
    trim_poly(out)
}

/// The remainder of polynomial long division, ascending coefficients.
fn poly_remainder(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let mut r = trim_poly(a.to_vec());
    let divisor = trim_poly(b.to_vec());
    if divisor.len() == 1 && divisor[0].is_zero() {
        return r;
    }
    let db = divisor.len() - 1;
    let lead = divisor[db].clone();
    // The step runs while the remainder's degree is STILL at least the divisor's: a remainder of
    // EQUAL degree has to have its leading term cancelled too, or the Euclid loop in poly_gcd
    // swaps two same-degree polynomials forever (x^3 - x^2 against x^2 is exactly that shape).
    // A CONSTANT divisor (db = 0) would keep the loop alive after the remainder reached zero,
    // since a trimmed vector is never empty, so the zero remainder ends it explicitly.
    while r.len() > db {
        if r.len() == 1 && r[0].is_zero() {
            break;
        }
        let dr = r.len() - 1;
        let factor = match r[dr].div(&lead) {
            Some(f) => f,
            None => break,
        };
        let shift = dr - db;
        for i in 0..=db {
            let subtracted = divisor[i].mul(&factor);
            r[i + shift] = r[i + shift].add(&subtracted.neg());
        }
        r = trim_poly(r);
    }
    trim_poly(r)
}

/// The number of DISTINCT real roots of a rational-coefficient polynomial: Sturm's theorem, so
/// the count is exact rather than sampled. Used by the degree >= 4 solver to publish RootOf
/// values for the representable (real) roots and count the rest.
pub fn real_root_count(coeffs: &[Rat]) -> usize {
    if cancel::check() {
        return 0;
    }
    let p = trim_poly(coeffs.to_vec());
    if p.len() <= 1 {
        return 0;
    }
    let mut sequence: Vec<Vec<Rat>> = vec![p.clone(), poly_derivative(&p)];
    while sequence.len() < 256 {
        let n = sequence.len();
        let remainder = poly_remainder(&sequence[n - 2], &sequence[n - 1]);
        let zero_remainder = remainder.len() == 1 && remainder[0].is_zero();
        if zero_remainder {
            break;
        }
        let next: Vec<Rat> = remainder.iter().map(|c| c.neg()).collect();
        let constant = next.len() == 1;
        sequence.push(poly_normalize_positive(&next));
        if constant {
            break;
        }
    }
    // Sign variations of the (nonzero) leading coefficients at +infinity and -infinity; the
    // leading term of p at -infinity flips by (-1)^degree.
    let variations = |at_negative_infinity: bool| -> usize {
        let mut last: Option<bool> = None; // Some(true) = positive
        let mut changes = 0usize;
        for poly in &sequence {
            if poly.len() == 1 && poly[0].is_zero() {
                continue;
            }
            let lead = poly.last().cloned().unwrap_or_else(Rat::zero);
            let mut positive = !lead.is_negative();
            if at_negative_infinity && (poly.len() - 1) % 2 == 1 {
                positive = !positive;
            }
            if let Some(previous) = last {
                if previous != positive {
                    changes += 1;
                }
            }
            last = Some(positive);
        }
        changes
    };
    variations(true).saturating_sub(variations(false))
}

// ---------------------------------------------------------------------------
// Yun's square-free decomposition and the numeric evaluation of a RootOf.
//
// Both are kernel surfaces the reference publishes and this crate lacked: a square-free
// factorization with multiplicities, and `Roots.N(RootOf(p, i), digits)` - the i-th REAL root
// of a defining polynomial, isolated by Sturm counts and refined by bisection in EXACT rational
// arithmetic (so the digits are the requested ones, never a float cast of them).
// ---------------------------------------------------------------------------

/// The reduced form of a rational. `Rat::new` only cancels a common factor when both parts fit
/// u64, and an exact polynomial remainder chain routinely leaves a common factor much wider than
/// that: without this the square-free part of a degree-3 polynomial carries 70-digit numerators
/// and denominators whose value is 1.
fn rat_reduce(r: &Rat) -> Rat {
    if r.numerator().is_zero() {
        return Rat::zero();
    }
    let numerator = r.numerator().magnitude();
    let denominator = r.denominator();
    let (common, _) = match gcd_int(numerator, denominator) {
        Some(pair) => pair,
        None => return r.clone(),
    };
    if common.is_zero() || common.to_decimal() == "1" {
        return r.clone();
    }
    let (num, _) = match r.numerator().div_rem(&common) {
        Some(pair) => pair,
        None => return r.clone(),
    };
    let (den, _) = match denominator.div_rem(&common) {
        Some(pair) => pair,
        None => return r.clone(),
    };
    Rat::from_parts(num, den)
}

/// The monic normalization of a coefficient vector (the leading coefficient becomes one).
fn poly_monic(p: &[Rat]) -> Vec<Rat> {
    let trimmed = trim_poly(p.to_vec());
    let lead = match trimmed.last() {
        Some(c) if !c.is_zero() => c.clone(),
        _ => return trimmed,
    };
    let mut out: Vec<Rat> = Vec::with_capacity(trimmed.len());
    for c in &trimmed {
        match c.div(&lead) {
            Some(v) => out.push(rat_reduce(&v)),
            None => return trimmed,
        }
    }
    out
}

/// The greatest common divisor of two rational-coefficient polynomials, monic. A zero operand
/// gives the other (monic); both zero gives one.
fn poly_gcd(a: &[Rat], b: &[Rat]) -> Vec<Rat> {
    let mut a = trim_poly(a.to_vec());
    let mut b = trim_poly(b.to_vec());
    for _ in 0..256 {
        if b.len() == 1 && b[0].is_zero() {
            return poly_monic(&a);
        }
        let remainder = poly_normalize_positive(&poly_remainder(&a, &b));
        a = b;
        b = remainder;
    }
    poly_monic(&a)
}

/// Exact division: None when the divisor does not divide the dividend.
fn poly_div_exact(num: &[Rat], den: &[Rat]) -> Option<Vec<Rat>> {
    let quotient = poly_div(num, den)?;
    let remainder = poly_remainder(num, den);
    if remainder.len() == 1 && remainder[0].is_zero() {
        Some(trim_poly(quotient))
    } else {
        None
    }
}

/// Yun's square-free decomposition of a univariate polynomial with rational coefficients.
///
/// Each returned pair is (factor coefficient vector, multiplicity), with the factors pairwise
/// coprime and their product equal to the input. `x^3 - x^2 = x^2*(x - 1)` decomposes into
/// `(x - 1, 1)` then `(x, 2)` - the order the reference's factor list is built in.
pub fn square_free_univariate(coeffs: &[Rat]) -> Vec<(Vec<Rat>, u64)> {
    if cancel::check() {
        return Vec::new();
    }
    let p = trim_poly(coeffs.to_vec());
    if p.len() <= 1 {
        return Vec::new();
    }
    let derivative = poly_derivative(&p);
    let mut g = poly_gcd(&p, &derivative);
    let mut w = match poly_div_exact(&p, &g) {
        Some(v) => v,
        None => return Vec::new(),
    };
    let mut out: Vec<(Vec<Rat>, u64)> = Vec::new();
    let mut multiplicity: u64 = 1;
    while w.len() > 1 {
        if cancel::check() {
            return out;
        }
        let y = poly_gcd(&w, &g);
        let z = match poly_div_exact(&w, &y) {
            Some(v) => v,
            None => return out,
        };
        if z.len() > 1 {
            out.push((z, multiplicity));
        }
        if y.len() > 1 {
            g = match poly_div_exact(&g, &y) {
                Some(v) => v,
                None => return out,
            };
        } else {
            g = vec![Rat::one()];
        }
        w = y;
        multiplicity += 1;
        if multiplicity > 64 {
            break;
        }
    }
    out
}

/// Why a RootOf index has no value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootOfError {
    /// The defining polynomial has no real root at all.
    NoRealRoots,
    /// The requested index is past the number of DISTINCT real roots.
    IndexOutOfRange {
        /// The index the caller asked for.
        index: usize,
        /// The number of distinct real roots the polynomial has.
        count: usize,
    },
    /// The isolator could not separate the roots; a measured refusal, never an approximation.
    NotIsolated,
}

impl RootOfError {
    /// The refusal text the caller publishes.
    pub fn message(&self) -> String {
        match self {
            RootOfError::NoRealRoots => {
                "rootof: the defining polynomial has no real root.".to_string()
            }
            RootOfError::IndexOutOfRange { index, count } => format!(
                "rootof: index {index} is out of range; the polynomial has {count} real root(s)."
            ),
            RootOfError::NotIsolated => "rootof: the real roots could not be isolated.".to_string(),
        }
    }
}

impl std::fmt::Display for RootOfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

/// Normalizes a polynomial by the ABSOLUTE VALUE of its leading coefficient. Multiplication by a
/// positive constant leaves every sign variation (and therefore every Sturm count) unchanged,
/// while keeping the coefficients bounded: without it the remainder chain of a degree-3
/// polynomial reaches thousand-digit coefficients after a few steps.
fn poly_normalize_positive(p: &[Rat]) -> Vec<Rat> {
    let trimmed = trim_poly(p.to_vec());
    let lead = match trimmed.last() {
        Some(c) if !c.is_zero() => c.abs(),
        _ => return trimmed,
    };
    let mut out: Vec<Rat> = Vec::with_capacity(trimmed.len());
    for c in &trimmed {
        match c.div(&lead) {
            Some(v) => out.push(rat_reduce(&v)),
            None => return trimmed,
        }
    }
    out
}

/// The Sturm sequence of a square-free polynomial: p, p', -rem(p, p'), ... The sequence ends at
/// the last nonzero remainder, so its sign variations count the DISTINCT real roots.
fn sturm_sequence(p: &[Rat]) -> Vec<Vec<Rat>> {
    let square_free = trim_poly(p.to_vec());
    let mut sequence: Vec<Vec<Rat>> = vec![
        poly_normalize_positive(&square_free),
        poly_normalize_positive(&poly_derivative(&square_free)),
    ];
    while sequence.len() < 256 {
        let n = sequence.len();
        let remainder = poly_remainder(&sequence[n - 2], &sequence[n - 1]);
        if remainder.len() == 1 && remainder[0].is_zero() {
            break;
        }
        let next: Vec<Rat> = remainder.iter().map(|c| c.neg()).collect();
        let constant = next.len() == 1;
        sequence.push(poly_normalize_positive(&next));
        if constant {
            break;
        }
    }
    sequence
}

/// A polynomial with integer coefficients: the coefficient vector cleared of denominators by a
/// positive common multiple, plus that multiple. None when the multiple is wider than the
/// materialization bound, so a pathological input is a refusal rather than a hang.
fn integer_coefficients(p: &[Rat]) -> Option<(Vec<Int>, Int)> {
    let mut multiple = Int::from_i64(1);
    for c in p {
        let denominator = Int::from_decimal(&c.denominator().to_decimal())?;
        multiple = lcm_int(&multiple, &denominator)?;
        if multiple.to_decimal().len() > 40 {
            return None;
        }
    }
    let mut out: Vec<Int> = Vec::with_capacity(p.len());
    for c in p {
        let numerator = c.numerator().mul(&multiple);
        let (quotient, _) = numerator.div_rem(c.denominator())?;
        out.push(quotient);
    }
    Some((out, multiple))
}

/// The least common multiple of two positive integers, or None when it does not fit the bound.
fn lcm_int(a: &Int, b: &Int) -> Option<Int> {
    if a.is_zero() || b.is_zero() {
        return Some(Int::from_i64(1));
    }
    let (g, _) = gcd_int(a.magnitude(), b.magnitude())?;
    let (divided, _) = a.div_rem(&g)?;
    Some(divided.mul(b))
}

/// The greatest common divisor of two magnitudes.
fn gcd_int(a: &Nat, b: &Nat) -> Option<(Nat, Nat)> {
    let mut left = a.clone();
    let mut right = b.clone();
    for _ in 0..4096 {
        if right.is_zero() {
            return Some((left, Nat::from_u64(0)));
        }
        let (_, remainder) = left.div_rem(&right)?;
        left = right;
        right = remainder;
    }
    None
}

/// The sign of `poly(X / scale)` in EXACT integer arithmetic: every coefficient is cleared of its
/// denominator, the substitution multiplies through by `scale^degree`, and the resulting integer
/// is only ever compared against zero. This keeps the isolation free of the rational
/// denominator growth that would make a 40-digit bisection quadratic in the digit count.
fn scaled_sign(poly: &[Rat], x: &Int, scale: &Int) -> Option<Ordering> {
    let (coefficients, _multiple) = integer_coefficients(poly)?;
    let degree = coefficients.len() - 1;
    let mut powers: Vec<Int> = Vec::with_capacity(degree + 1);
    powers.push(Int::from_i64(1));
    for _ in 0..degree {
        let last = powers.last()?.mul(scale);
        powers.push(last);
    }
    let mut acc = coefficients[degree].clone();
    for i in (0..degree).rev() {
        acc = acc.mul(x).add(&coefficients[i].mul(&powers[degree - i]));
    }
    Some(acc.cmp(&Int::zero()))
}

/// The number of sign variations of the Sturm sequence evaluated at `X / scale`. A zero
/// evaluation is skipped, which is the standard convention at a root of the polynomial.
fn scaled_variations(sequence: &[Vec<Rat>], x: &Int, scale: &Int) -> Option<usize> {
    let mut last: Option<Ordering> = None;
    let mut changes = 0usize;
    for poly in sequence {
        let sign = scaled_sign(poly, x, scale)?;
        if sign == Ordering::Equal {
            continue;
        }
        if let Some(previous) = last {
            if previous != sign {
                changes += 1;
            }
        }
        last = Some(sign);
    }
    Some(changes)
}

/// Separates every real root of a square-free polynomial into an interval of width at most one
/// SCALED unit, in ascending order. All arithmetic is integer: a bisection point is the integer
/// midpoint of the scaled endpoints, so the work is linear in the digit count, never quadratic.
fn isolate_scaled(
    sequence: &[Vec<Rat>],
    lo: &Int,
    hi: &Int,
    scale: &Int,
    depth: usize,
    out: &mut Vec<(Int, Int)>,
) -> Option<()> {
    if out.len() > 64 || depth > 1200 {
        return Some(());
    }
    let below = scaled_variations(sequence, lo, scale)?;
    let above = scaled_variations(sequence, hi, scale)?;
    let inside = below.saturating_sub(above);
    if inside == 0 {
        return Some(());
    }
    // The interval is narrower than one scaled unit: it isolates exactly one root.
    if inside == 1 && hi.sub(lo).cmp(&Int::from_i64(1)) != Ordering::Greater {
        out.push((lo.clone(), hi.clone()));
        return Some(());
    }
    let (mid, _) = lo.add(hi).div_rem(&Nat::from_u64(2))?;
    if mid.cmp(lo) != Ordering::Greater || mid.cmp(hi) != Ordering::Less {
        if inside == 1 {
            out.push((lo.clone(), hi.clone()));
        }
        return Some(());
    }
    isolate_scaled(sequence, lo, &mid, scale, depth + 1, out)?;
    isolate_scaled(sequence, &mid, hi, scale, depth + 1, out)?;
    Some(())
}

/// The `index`-th real root (ascending, 0-based, distinct roots) of a rational-coefficient
/// polynomial, as an exact rational accurate to about 40 digits: the reference's
/// `Roots.N(RootOf(p, i), digits)` surface.
pub fn nth_real_root(coeffs: &[Rat], index: usize) -> Result<Rat, RootOfError> {
    nth_real_root_digits(coeffs, index, 40)
}

/// `nth_real_root` with an explicit digit target.
pub fn nth_real_root_digits(
    coeffs: &[Rat],
    index: usize,
    digits: usize,
) -> Result<Rat, RootOfError> {
    if cancel::check() {
        return Err(RootOfError::NotIsolated);
    }
    let p = trim_poly(coeffs.to_vec());
    if p.len() <= 1 {
        return Err(RootOfError::NoRealRoots);
    }
    // A repeated root is ONE real root of the value: isolate the square-free part, or the count
    // and the index would both be about the multiplicity rather than the root.
    let derivative = poly_derivative(&p);
    let common = poly_gcd(&p, &derivative);
    // Divide out the common factor and NORMALIZE: the exact quotient of two polynomials whose
    // gcd carries a large constant content has large coefficients, and the isolation below is
    // only polynomial in their width. A positive normalization leaves every sign unchanged.
    let quotient = poly_div_exact(&p, &common).unwrap_or_else(|| p.clone());
    let reduced: Vec<Rat> = quotient.iter().map(rat_reduce).collect();
    let square_free = poly_normalize_positive(&reduced);
    let count = real_root_count(&square_free);
    if count == 0 {
        return Err(RootOfError::NoRealRoots);
    }
    if index >= count {
        return Err(RootOfError::IndexOutOfRange { index, count });
    }
    let sequence = sturm_sequence(&square_free);
    // Cauchy's bound: every real root lies inside [-B, B] with B = 1 + max |a_i / a_n|, which
    // the scaled interval below rounds up to the next whole unit.
    let lead = square_free.last().map(|c| c.abs()).unwrap_or_else(Rat::one);
    let mut bound = Rat::one();
    for c in &square_free[..square_free.len() - 1] {
        if lead.is_zero() {
            break;
        }
        if let Some(candidate) = c.abs().div(&lead).map(|v| v.add(&Rat::one())) {
            if candidate.cmp(&bound) == Ordering::Greater {
                bound = candidate;
            }
        }
    }
    let mut scale_text = String::from("1");
    for _ in 0..(digits.min(60) + 5) {
        scale_text.push('0');
    }
    let scale = Int::from_decimal(&scale_text).ok_or(RootOfError::NotIsolated)?;
    let (bound_number, _) = bound
        .numerator()
        .mul(&scale)
        .div_rem(bound.denominator())
        .ok_or(RootOfError::NotIsolated)?;
    let magnitude = Int::from_decimal(&bound_number.magnitude().to_decimal())
        .ok_or(RootOfError::NotIsolated)?;
    let radius = magnitude.add(&Int::from_i64(1));
    let mut intervals: Vec<(Int, Int)> = Vec::new();
    isolate_scaled(
        &sequence,
        &radius.negate(),
        &radius,
        &scale,
        0,
        &mut intervals,
    )
    .ok_or(RootOfError::NotIsolated)?;
    if intervals.len() != count {
        return Err(RootOfError::NotIsolated);
    }
    let (lo, hi) = intervals.swap_remove(index);
    let (middle, _) = lo
        .add(&hi)
        .div_rem(&Nat::from_u64(2))
        .ok_or(RootOfError::NotIsolated)?;
    let denominator = Nat::from_decimal(&scale_text).ok_or(RootOfError::NotIsolated)?;
    Ok(Rat::from_parts(middle, denominator))
}

/// The numeric value of a `rootof(p, index)` node to the requested number of digits - the
/// reference's `Roots.N` over the node the solver publishes.
pub fn evalf_rootof(e: &Expr, digits: usize) -> Result<Rat, RootOfError> {
    let Expr::Fun(name, args) = e else {
        return Err(RootOfError::NotIsolated);
    };
    if name != "rootof" || args.len() != 2 {
        return Err(RootOfError::NotIsolated);
    }
    let var = args[0]
        .free_symbols()
        .into_iter()
        .next()
        .ok_or(RootOfError::NotIsolated)?;
    let coeffs = poly_coeffs(&args[0], &var).ok_or(RootOfError::NotIsolated)?;
    let index = match &args[1] {
        Expr::Num(n) => match n.to_i64() {
            Some(v) if v >= 0 => v as usize,
            _ => return Err(RootOfError::NotIsolated),
        },
        _ => return Err(RootOfError::NotIsolated),
    };
    nth_real_root_digits(&coeffs, index, digits)
}

/// The polynomial expression c0 + c1*x + ... that a RootOf's defining polynomial prints as.
fn poly_expr(coeffs: &[Rat], var: &str) -> Expr {
    let mut acc = Expr::num(0);
    for (i, c) in coeffs.iter().enumerate() {
        if c.is_zero() {
            continue;
        }
        let term = if i == 0 {
            Expr::Num(c.clone())
        } else {
            Expr::mul(
                Expr::Num(c.clone()),
                Expr::pow(Expr::sym(var), Expr::num(i as i64)),
            )
        };
        acc = Expr::add(acc, term);
    }
    acc
}

/// Solves a polynomial equation for the conventional variable x.
pub fn solve_polynomial(coeffs: &[Rat]) -> SolveOutcome {
    solve_polynomial_named(coeffs, "x")
}

/// Solves a polynomial equation with rational coefficients by exact root extraction.
///
/// The variable name is only used to spell a RootOf's defining polynomial; the roots themselves
/// are a property of the coefficients.
pub fn solve_polynomial_named(coeffs: &[Rat], var: &str) -> SolveOutcome {
    // A cancelled search has no answer to publish. The evaluation discards this outcome and
    // reports the structured Cancelled refusal (Lovelace.Symbolics/Solvers/Solve.cs polls the
    // same point), so "Unevaluated" is never observable from a cancellation.
    if cancel::check() {
        return SolveOutcome::Unevaluated;
    }
    if coeffs.is_empty() {
        return SolveOutcome::Unevaluated;
    }
    if coeffs.len() == 1 {
        return if coeffs[0].is_zero() {
            SolveOutcome::Unevaluated
        } else {
            SolveOutcome::NoSolutions
        };
    }
    let mut working = coeffs.to_vec();
    let mut roots: Vec<Root> = Vec::new();
    // Find rational roots until the polynomial is linear or has none. Each root is divided out
    // completely and its MULTIPLICITY is recorded: the reference preserves the square-free
    // factorization's multiplicity instead of collapsing x^2 - 2x + 1 to one count-1 entry.
    loop {
        if working.len() <= 2 {
            break;
        }
        if cancel::check() {
            return SolveOutcome::Unevaluated;
        }
        match find_rational_root(&working) {
            Some(root) => {
                let mut multiplicity = 0u64;
                while working.len() > 1
                    && eval_poly(&working, &root)
                        .map(|v| v.is_zero())
                        .unwrap_or(false)
                {
                    working = divide_by_root(&working, &root);
                    multiplicity += 1;
                }
                if multiplicity == 0 {
                    break;
                }
                add_root(&mut roots, root, multiplicity);
            }
            None => {
                // A quadratic remainder can still be solved exactly when its discriminant is a
                // perfect square.
                if working.len() == 3 {
                    if let Some((a, b)) = quadratic_roots(&working) {
                        if a.cmp(&b) == Ordering::Equal {
                            add_root(&mut roots, a, 2);
                        } else {
                            add_root(&mut roots, a, 1);
                            add_root(&mut roots, b, 1);
                        }
                        working = vec![Rat::one()];
                    } else if let Some(complex) = quadratic_complex_roots(&working) {
                        // Complex roots are published first, then the exact ones already found.
                        let mut exprs = complex;
                        roots.sort_by(|a, b| a.value.cmp(&b.value));
                        exprs.extend(roots.iter().map(|r| expr_from_rat(&r.value)));
                        return SolveOutcome::RootExpressions(exprs);
                    } else if let Some(radical) = quadratic_radical_roots(&working) {
                        // A positive, non-square discriminant is an exact algebraic value the
                        // reference publishes as a radical (x^2 - 2 -> -1/2*sqrt(8) and
                        // 1/2*sqrt(8)); the kernel used to answer Unevaluated here, which left
                        // every nonlinear system that eliminated to such a quadratic unsolved.
                        let mut exprs = radical;
                        roots.sort_by(|a, b| a.value.cmp(&b.value));
                        exprs.extend(roots.iter().map(|r| expr_from_rat(&r.value)));
                        // The reference orders non-rational solutions by canonical print, which
                        // is the kernel's own total order: -1/2*sqrt(20), then 1/2*sqrt(20),
                        // then the exact 0 (the display order would put 0 between them).
                        exprs.sort_by_key(|e| e.canonical());
                        return SolveOutcome::RootExpressions(exprs);
                    }
                }
                break;
            }
        }
    }
    // A linear remainder still has an exact root.
    if working.len() == 2 && !working[1].is_zero() {
        if let Some(root) = Rat::zero().add(&working[0].neg()).div(&working[1]) {
            add_root(&mut roots, root, 1);
        }
    }
    roots.sort_by(|a, b| a.value.cmp(&b.value));
    let represented: u64 = roots.iter().map(|r| r.multiplicity).sum();
    let degree = (coeffs.len() - 1) as u64;
    // A remaining CUBIC factor with no rational root has an exact radical form (Cardano). The
    // reference publishes it; refusing silently would lose roots the oracle represents. Domain
    // filtering happens in the caller, which drops expressions carrying i for a real request.
    if working.len() == 4 && roots.is_empty() {
        if let Some(exprs) = cubic_root_expressions(&working) {
            return SolveOutcome::RootExpressions(exprs);
        }
    }
    // A remaining factor of degree >= 4 has no radical formula here. The reference represents its
    // real roots as algebraic numbers: RootOf(defining_polynomial, index), one per real root in
    // ascending order, with the non-real roots counted as unrepresented.
    if working.len() >= 5 {
        let remaining_degree = working.len() - 1;
        let real = real_root_count(&working);
        if real > 0 || !roots.is_empty() {
            let defining = poly_expr(&working, var);
            let mut exprs: Vec<Expr> = roots.iter().map(|r| expr_from_rat(&r.value)).collect();
            for index in 0..real {
                exprs.push(Expr::Fun(
                    "rootof".to_string(),
                    vec![defining.clone(), Expr::num(index as i64)],
                ));
            }
            return SolveOutcome::RootExpressionsCount(exprs, (remaining_degree - real) as u64);
        }
    }
    if represented < degree {
        // The polynomial is not fully factored with exact methods.
        if roots.is_empty() {
            // A biquadratic may still have exact roots through y = x^2.
            if let Some(outcome) = solve_biquadratic(coeffs) {
                return outcome;
            }
            return if coeffs.len() > 3 {
                SolveOutcome::Partial(Vec::new())
            } else {
                SolveOutcome::Unevaluated
            };
        }
        return SolveOutcome::Partial(roots.into_iter().map(|r| r.value).collect());
    }
    if roots.is_empty() {
        if let Some(outcome) = solve_biquadratic(coeffs) {
            return outcome;
        }
        return SolveOutcome::Unevaluated;
    }
    SolveOutcome::Roots(roots)
}

/// Records a root, merging the multiplicity when the same value is found again.
fn add_root(roots: &mut Vec<Root>, value: Rat, multiplicity: u64) {
    if let Some(existing) = roots
        .iter_mut()
        .find(|r| r.value.cmp(&value) == Ordering::Equal)
    {
        existing.multiplicity += multiplicity;
    } else {
        roots.push(Root {
            value,
            multiplicity,
        });
    }
}

fn find_rational_root(coeffs: &[Rat]) -> Option<Rat> {
    if cancel::check() {
        return None;
    }
    let degree = coeffs.len() - 1;
    // Scale to integers so the rational root theorem applies. Every step is checked: a
    // user-sized coefficient (an exact Int of any width) must not wrap through an `as i64`, and
    // i64::MIN.abs() panics. A coefficient that does not fit this enumeration's i64 arithmetic is
    // simply not enumerated here; the quadratic/cubic exact branches still get their chance.
    let mut scale: u64 = 1;
    for c in coeffs {
        let d = c.denominator().to_u64()?;
        scale = lcm(scale, d.max(1))?;
    }
    let mut ints: Vec<i64> = Vec::with_capacity(coeffs.len());
    for c in coeffs {
        let numerator: i128 = c.numerator().to_decimal().parse().ok()?;
        let denominator: i128 = c.denominator().to_decimal().parse().ok()?;
        let scaled = numerator
            .checked_mul(i128::from(scale))?
            .checked_div(denominator)?;
        ints.push(i64::try_from(scaled).ok()?);
    }
    // unsigned_abs is defined for i64::MIN, where abs() would panic.
    let constant = ints[0].unsigned_abs();
    let leading = ints[degree].unsigned_abs();
    if leading == 0 {
        return None;
    }
    // A zero constant term means x divides the polynomial, so 0 is a root. The rational-root
    // theorem cannot see it: its p/q enumeration runs over the divisors of a0 = 0. Extracting the
    // zero root HERE is what turns x^4 - 5*x^2 into the exact pair +-1/2*sqrt(20) (the remaining
    // quadratic) instead of three RootOf values.
    if constant == 0 {
        return Some(Rat::zero());
    }
    for p in divisors(constant) {
        if cancel::check() {
            return None;
        }
        // A divisor beyond i64::MAX has no signed numerator form; skip it rather than wrap it.
        let Ok(p) = i64::try_from(p) else {
            continue;
        };
        for q in divisors(leading) {
            for sign in [1i64, -1i64] {
                let candidate = Rat::new(Int::from_i64(sign * p), Nat::from_u64(q));
                if eval_poly(coeffs, &candidate)
                    .map(|v| v.is_zero())
                    .unwrap_or(false)
                {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return vec![1];
    }
    let mut out = Vec::new();
    let mut d = 1u64;
    // d is capped at 100 000, so d * d cannot overflow u64.
    while d * d <= n && d < 100_000 {
        if cancel::check() {
            return out;
        }
        if n % d == 0 {
            out.push(d);
            out.push(n / d);
        }
        d += 1;
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The least common multiple, or None when it does not fit u64. The caller turns that into a
/// refusal to enumerate rather than a wrapped scale factor.
fn lcm(a: u64, b: u64) -> Option<u64> {
    if a == 0 || b == 0 {
        return Some(1);
    }
    (a / gcd_u64(a, b)).checked_mul(b)
}

fn eval_poly(coeffs: &[Rat], x: &Rat) -> Option<Rat> {
    let mut acc = Rat::zero();
    for c in coeffs.iter().rev() {
        acc = acc.mul(x).add(c);
    }
    Some(acc)
}

fn divide_by_root(coeffs: &[Rat], root: &Rat) -> Vec<Rat> {
    let degree = coeffs.len() - 1;
    let mut out = vec![Rat::zero(); degree];
    let mut carry = Rat::zero();
    for i in (1..coeffs.len()).rev() {
        let value = coeffs[i].add(&carry);
        out[i - 1] = value.clone();
        carry = value.mul(root);
    }
    while out.len() > 1 && out.last().map(|c| c.is_zero()).unwrap_or(false) {
        out.pop();
    }
    out
}

/// Quadratic roots over the complex numbers, as expressions when the discriminant is negative.
fn quadratic_complex_roots(coeffs: &[Rat]) -> Option<Vec<Expr>> {
    let a = coeffs[2].clone();
    let b = coeffs[1].clone();
    let c = coeffs[0].clone();
    let disc = b.mul(&b).add(&c.mul(&a).mul(&Rat::from_i64(-4)));
    if !disc.is_negative() {
        return None;
    }
    let mag = disc.neg();
    // sqrt(|disc|) as an exact integer when possible, otherwise a radical expression.
    let root_expr = match (
        perfect_square(mag.numerator().magnitude()),
        perfect_square(mag.denominator()),
    ) {
        (Some(n), Some(1)) => Expr::num(n as i64),
        _ => Expr::sqrt(Expr::Num(mag.clone())),
    };
    let i_root = Expr::mul(imaginary_unit(), root_expr);
    let two_a = a.mul(&Rat::from_i64(2));
    // Roots are (i*sqrt(|d|) - b) / (2a) and (-i*sqrt(|d|) - b) / (2a).
    let factor = Rat::one().div(&two_a)?;
    let minus_b: Expr = Expr::Num(b.neg());
    let plus = Expr::mul(
        Expr::Num(factor.clone()),
        Expr::add(i_root.clone(), minus_b.clone()),
    );
    let minus = Expr::mul(Expr::Num(factor), Expr::add(Expr::neg(i_root), minus_b));
    Some(vec![plus, minus])
}

/// Quadratic roots when the discriminant is positive and not a perfect square: the exact
/// radical pair the reference publishes. Coefficients are canonical (the Groebner basis and the
/// factor path hand this function a monic quadratic), so `1/2*sqrt(8)` keeps the reference's
/// own form instead of being simplified to `sqrt(2)`.
fn quadratic_radical_roots(coeffs: &[Rat]) -> Option<Vec<Expr>> {
    if coeffs.len() != 3 {
        return None;
    }
    let a = coeffs[2].clone();
    let b = coeffs[1].clone();
    let c = coeffs[0].clone();
    if a.is_zero() {
        return None;
    }
    let disc = b.mul(&b).add(&c.mul(&a).mul(&Rat::from_i64(-4)));
    if disc.is_negative() || disc.is_zero() {
        return None;
    }
    let sqrt_disc = Expr::sqrt(Expr::Num(disc));
    let two_a = Expr::Num(a.mul(&Rat::from_i64(2)));
    let neg_b: Expr = Expr::Num(b.neg());
    Some(vec![
        Expr::div(Expr::add(neg_b.clone(), sqrt_disc.clone()), two_a.clone()),
        Expr::div(Expr::sub(neg_b, sqrt_disc), two_a),
    ])
}

fn quadratic_roots(coeffs: &[Rat]) -> Option<(Rat, Rat)> {
    // a x^2 + b x + c
    let a = coeffs[2].clone();
    let b = coeffs[1].clone();
    let c = coeffs[0].clone();
    let disc = b.mul(&b).add(&c.mul(&a).mul(&Rat::from_i64(-4)));
    if disc.is_negative() {
        return None;
    }
    let num = disc.numerator().clone();
    let den = disc.denominator().clone();
    // discriminant must be a perfect square of a rational
    let rn = perfect_square(num.magnitude())?;
    let rd = perfect_square(&den)?;
    let root = Rat::new(Int::from_i64(rn as i64), Nat::from_u64(rd));
    let two_a = a.mul(&Rat::from_i64(2));
    let r1 = Rat::zero().add(&b.neg()).add(&root).div(&two_a)?;
    let r2 = Rat::zero().add(&b.neg()).add(&root.neg()).div(&two_a)?;
    Some((r1, r2))
}

/// Exact Cardano roots of a depressed cubic `a x^3 + b x^2 + c x + d`.
///
/// The reference's `CubicRoots` is the contract: normalize to monic, substitute
/// `x = t - A/3` to reach `t^3 + P t + Q`, then
/// - `disc = Q^2/4 + P^3/27 >= 0`: one real branch plus the two omega rotations,
/// - `disc < 0`: three real roots through the cosine form.
///   The returned expressions are exact; the CALLER applies the domain filter (a real request
///   drops the branches carrying `i`).
fn cubic_root_expressions(coeffs: &[Rat]) -> Option<Vec<Expr>> {
    if cancel::check() {
        return None;
    }
    if coeffs.len() != 4 {
        return None;
    }
    let a = coeffs[3].clone();
    if a.is_zero() {
        return None;
    }
    let inv_a = Rat::one().div(&a)?;
    let aa = coeffs[2].mul(&inv_a);
    let bb = coeffs[1].mul(&inv_a);
    let cc = coeffs[0].mul(&inv_a);
    let three = Rat::from_i64(3);
    let a3 = aa.div(&three)?;
    let p = bb.add(&aa.mul(&aa).div(&three)?.neg());
    let q = Rat::from_i64(2)
        .mul(&aa)
        .mul(&aa)
        .mul(&aa)
        .div(&Rat::from_i64(27))?
        .add(&aa.mul(&bb).div(&three)?.neg())
        .add(&cc);
    let disc = q
        .mul(&q)
        .div(&Rat::from_i64(4))?
        .add(&p.mul(&p).mul(&p).div(&Rat::from_i64(27))?);
    let one_third = Rat::new(Int::from_i64(1), Nat::from_u64(3));
    let minus_one_third = Rat::new(Int::from_i64(-1), Nat::from_u64(3));
    let mut ts: Vec<Expr> = Vec::new();
    if !disc.is_negative() {
        // One real root through Cardano; the other two are the omega rotations. u and v are
        // branch-coupled by u*v = -P/3, exactly as the reference insists.
        let half_q = q.neg().div(&Rat::from_i64(2))?;
        // sqrt, not a half power: the reference printer renders a 1/2 exponent as sqrt(...).
        let sqrt_d = Expr::sqrt(Expr::Num(disc.clone()));
        let r1 = Expr::add(Expr::Num(half_q), sqrt_d);
        let u = Expr::pow(r1.clone(), Expr::Num(one_third));
        let v = if p.is_zero() {
            Expr::num(0)
        } else {
            Expr::mul(
                Expr::Num(p.neg().div(&three)?),
                Expr::pow(r1, Expr::Num(minus_one_third)),
            )
        };
        let omega = Expr::add(
            Expr::Num(Rat::new(Int::from_i64(-1), Nat::from_u64(2))),
            Expr::mul(
                Expr::div(Expr::sqrt(Expr::num(3)), Expr::num(2)),
                imaginary_unit(),
            ),
        );
        let omega2 = Expr::sub(
            Expr::Num(Rat::new(Int::from_i64(-1), Nat::from_u64(2))),
            Expr::mul(
                Expr::div(Expr::sqrt(Expr::num(3)), Expr::num(2)),
                imaginary_unit(),
            ),
        );
        ts.push(Expr::add(u.clone(), v.clone()));
        ts.push(Expr::add(
            Expr::mul(omega.clone(), u.clone()),
            Expr::mul(omega2.clone(), v.clone()),
        ));
        ts.push(Expr::add(Expr::mul(omega2, u), Expr::mul(omega, v)));
    } else {
        // Three real roots via the trigonometric form.
        let m = Expr::mul(Expr::num(2), Expr::sqrt(Expr::Num(p.neg().div(&three)?)));
        let inner = Expr::mul(
            Expr::Num(Rat::from_i64(3).mul(&q).div(&Rat::from_i64(2).mul(&p))?),
            Expr::sqrt(Expr::Num(Rat::from_i64(-3).div(&p)?)),
        );
        let theta = Expr::div(Expr::Fun("acos".to_string(), vec![inner]), Expr::num(3));
        let pi = Expr::Fun("pi".to_string(), Vec::new());
        for k in 0..3i64 {
            let angle = Expr::add(
                theta.clone(),
                Expr::div(Expr::mul(Expr::num(2 * k), pi.clone()), Expr::num(3)),
            );
            ts.push(Expr::mul(
                m.clone(),
                Expr::Fun("cos".to_string(), vec![angle]),
            ));
        }
    }
    let shift = Expr::Num(a3.neg());
    Some(
        ts.into_iter()
            .map(|t| Expr::add(t, shift.clone()))
            .collect(),
    )
}

fn solve_biquadratic(coeffs: &[Rat]) -> Option<SolveOutcome> {
    if cancel::check() {
        return None;
    }
    // The biquadratic substitution y = x^2 only exists for a quartic. The caller may hand this
    // function any degree it could not factor (the crash was a cubic: x^3 - 2 reached
    // coeffs[4]); refusing a non-quartic is part of the fix, not an optimization.
    if coeffs.len() != 5 {
        return None;
    }
    let a = coeffs[4].clone();
    let b = coeffs[2].clone();
    let c = coeffs[0].clone();
    let disc = b.mul(&b).add(&c.mul(&a).mul(&Rat::from_i64(-4)));
    if disc.is_negative() {
        return None;
    }
    let sqrt_disc = match (
        perfect_square(disc.numerator().magnitude()),
        perfect_square(disc.denominator()),
    ) {
        (Some(n), Some(1)) => Expr::num(n as i64),
        _ => Expr::sqrt(Expr::Num(disc.clone())),
    };
    let two_a = a.mul(&Rat::from_i64(2));
    let minus_b: Expr = Expr::Num(b.neg());
    let mut roots: Vec<Expr> = Vec::new();
    let mut unrepresented: u64 = 0;
    for sign in [1i64, -1i64] {
        let numerator = if sign > 0 {
            Expr::add(minus_b.clone(), sqrt_disc.clone())
        } else {
            Expr::add(minus_b.clone(), Expr::neg(sqrt_disc.clone()))
        };
        let y = Expr::Div(Box::new(numerator), Box::new(Expr::Num(two_a.clone())));
        let approx = approximate(&y);
        if approx > 0.0 {
            let root = Expr::sqrt(y);
            roots.push(root.clone());
            roots.push(Expr::neg(root));
        } else {
            unrepresented += 2;
        }
    }
    if roots.is_empty() {
        return None;
    }
    Some(SolveOutcome::RootExpressionsCount(roots, unrepresented))
}

fn approximate(e: &Expr) -> f64 {
    match e {
        Expr::Fun(name, args) if name == "sqrt" && args.len() == 1 => approximate(&args[0]).sqrt(),
        Expr::Div(a, b) => {
            let denominator = approximate(b);
            if denominator == 0.0 {
                0.0
            } else {
                approximate(a) / denominator
            }
        }
        Expr::Add(v) => v.iter().map(approximate).sum(),
        Expr::Mul(v) => v.iter().map(approximate).product(),
        Expr::Num(n) => n.to_string().parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn perfect_square(n: &Nat) -> Option<u64> {
    let v = n.to_u64()?;
    let mut r = 0u64;
    while r * r <= v {
        if r * r == v {
            return Some(r);
        }
        r += 1;
        if r > 200_000 {
            return None;
        }
    }
    None
}

/// Builds an expression from a rational.
pub fn expr_from_rat(r: &Rat) -> Expr {
    Expr::Num(r.clone())
}

/// Differentiates an expression with respect to a symbol.
pub fn diff(e: &Expr, var: &str) -> Expr {
    if cancel::check() {
        return e.clone();
    }
    match e {
        Expr::Num(_) | Expr::Real(_) => Expr::num(0),
        Expr::Sym(s) => Expr::num(if s == var { 1 } else { 0 }),
        Expr::Add(terms) => {
            let mut acc = Expr::num(0);
            for t in terms {
                acc = Expr::add(acc, diff(t, var));
            }
            acc
        }
        Expr::Mul(factors) => {
            // Product rule, one factor at a time.
            let mut acc = Expr::num(0);
            for i in 0..factors.len() {
                let d = diff(&factors[i], var);
                if d == Expr::num(0) {
                    continue;
                }
                let mut product = d;
                for (j, f) in factors.iter().enumerate() {
                    if j != i {
                        product = Expr::mul(product, f.clone());
                    }
                }
                acc = Expr::add(acc, product);
            }
            acc
        }
        Expr::Pow(base, exp) => {
            // d(u^n) = n*u^(n-1)*u'
            if let Expr::Num(n) = exp.as_ref() {
                if n.denominator().to_decimal() == "1" {
                    let k = n.numerator().to_decimal().parse::<i64>().unwrap_or(0);
                    let inner = Expr::pow((**base).clone(), Expr::num(k - 1));
                    let du = diff(base, var);
                    return Expr::mul(Expr::mul(Expr::num(k), inner), du);
                }
            }
            Expr::num(0)
        }
        Expr::Fun(name, args) => {
            // A piecewise differentiates BRANCH BY BRANCH; the guards are conditions, not
            // functions of the differentiation variable, so they are carried unchanged.
            if name == "piecewise" {
                if let Some((branches, otherwise)) = e.as_piecewise() {
                    let differentiated: Vec<(Expr, Expr)> = branches
                        .into_iter()
                        .map(|(guard, value)| (guard, diff(&value, var)))
                        .collect();
                    return Expr::piecewise(differentiated, diff(&otherwise, var));
                }
                return Expr::num(0);
            }
            // d abs(u)/dx carries the NONZERO condition and, at the kink, the unevaluated
            // derivative: piecewise(sign(u)*u' if u != 0, diff(abs(u), x)). Stating it as 0
            // would claim abs is smooth at the origin.
            if name == "abs" && args.len() == 1 {
                let u = &args[0];
                let guard = Expr::relation(RelOp::Ne, u.clone(), Expr::num(0));
                let slope = Expr::mul(Expr::Fun("sign".to_string(), vec![u.clone()]), diff(u, var));
                return Expr::piecewise(
                    vec![(guard, slope)],
                    Expr::derivative(e.clone(), vec![var.to_string()]),
                );
            }
            // d floor(u)/dx has NO closed form: it is an unevaluated derivative, never 0.
            if name == "floor" && args.len() == 1 {
                return Expr::derivative(e.clone(), vec![var.to_string()]);
            }
            // Differentiating an unevaluated derivative raises its order.
            if let Some((body, vars)) = e.as_derivative() {
                let mut vars = vars;
                vars.push(var.to_string());
                return Expr::derivative(body, vars);
            }
            if args.len() != 1 {
                return Expr::num(0);
            }
            let u = &args[0];
            let du = diff(u, var);
            let outer = match name.as_str() {
                "sin" => Expr::Fun("cos".to_string(), vec![u.clone()]),
                "cos" => Expr::neg(Expr::Fun("sin".to_string(), vec![u.clone()])),
                "exp" => Expr::Fun("exp".to_string(), vec![u.clone()]),
                "log" => Expr::div(Expr::num(1), u.clone()),
                "sqrt" => Expr::div(Expr::num(1), Expr::mul(Expr::num(2), Expr::sqrt(u.clone()))),
                _ => return Expr::num(0),
            };
            Expr::mul(outer, du)
        }
        Expr::Div(u, v) => {
            // (u'v - uv') / v^2
            let du = diff(u, var);
            let dv = diff(v, var);
            let numerator = Expr::sub(Expr::mul(du, (**v).clone()), Expr::mul((**u).clone(), dv));
            Expr::div(numerator, Expr::pow((**v).clone(), Expr::num(2)))
        }
        Expr::Rel(_, _, _) => Expr::num(0),
    }
}

/// Rewrites explicit quotients as products with negative powers, for the analyses that need
/// the polynomial or series view. The Div node itself is preserved for printing.
pub fn strip_div(e: &Expr) -> Expr {
    match e {
        Expr::Div(a, b) => Expr::div(strip_div(a), strip_div(b)),
        Expr::Add(v) => {
            let mut acc = Expr::num(0);
            for t in v {
                acc = Expr::add(acc, strip_div(t));
            }
            acc
        }
        Expr::Mul(v) => {
            let mut acc = Expr::num(1);
            for t in v {
                acc = Expr::mul(acc, strip_div(t));
            }
            acc
        }
        Expr::Pow(a, b) => Expr::pow(strip_div(a), strip_div(b)),
        Expr::Fun(n, args) => Expr::Fun(n.clone(), args.iter().map(strip_div).collect()),
        Expr::Rel(op, a, b) => Expr::Rel(*op, Box::new(strip_div(a)), Box::new(strip_div(b))),
        other => other.clone(),
    }
}

// ---------------------------------------------------------------------------
// Wire introspection: canonical form, size, domain, exactness and free symbols.
// ---------------------------------------------------------------------------

/// The numeric domains, ordered Integer < Rational < Real < Complex so a sum or product
/// takes the widest operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dom {
    /// Whole numbers.
    Integer,
    /// Exact quotients.
    Rational,
    /// Real-valued (irrational or transcendental) expressions.
    Real,
    /// Complex-valued expressions.
    Complex,
}

impl Dom {
    /// The lower-case wire name.
    pub fn name(self) -> &'static str {
        match self {
            Dom::Integer => "integer",
            Dom::Rational => "rational",
            Dom::Real => "real",
            Dom::Complex => "complex",
        }
    }
}

impl Expr {
    /// The canonical S-expression form (the versioned, round-trippable wire encoding).
    pub fn canonical(&self) -> String {
        match self {
            Expr::Num(n) => format!(
                "(rat {} {})",
                n.numerator().to_decimal(),
                n.denominator().to_decimal()
            ),
            Expr::Real(text) => format!("(real {text})"),
            Expr::Sym(s) => format!("(sym {})", s),
            Expr::Add(terms) => {
                // The canonical order is the kernel's total term order (constants first, then by
                // node kind), which is NOT the degree order the pretty printer uses.
                let mut ordered = terms.clone();
                ordered.sort_by(canonical_order);
                let parts: Vec<String> = ordered.iter().map(|t| t.canonical()).collect();
                format!("(add {})", parts.join(" "))
            }
            Expr::Mul(factors) => {
                let mut ordered = factors.clone();
                ordered.sort_by(canonical_order);
                let parts: Vec<String> = ordered.iter().map(|f| f.canonical()).collect();
                format!("(mul {})", parts.join(" "))
            }
            Expr::Pow(base, exp) => format!("(pow {} {})", base.canonical(), exp.canonical()),
            Expr::Div(a, b) => {
                // A quotient is a product with the reciprocal, which is the canonical shape. The
                // two factors are emitted in the kernel's canonical order so that parsing the
                // text back gives a Multiply that re-prints the SAME text: a canonical form whose
                // operand order depends on which constructor built the value would not round-trip.
                let reciprocal = Expr::Pow(
                    Box::new((**b).clone()),
                    Box::new(Expr::Num(Rat::from_i64(-1))),
                );
                if canonical_order(a, &reciprocal) == Ordering::Greater {
                    format!("(mul {} {})", reciprocal.canonical(), a.canonical())
                } else {
                    format!("(mul {} {})", a.canonical(), reciprocal.canonical())
                }
            }
            Expr::Fun(name, args) => {
                if name == "sqrt" && args.len() == 1 {
                    return format!("(pow {} (rat 1 2))", args[0].canonical());
                }
                // An unevaluated integral is a BINDING node in the reference's canonical
                // vocabulary: (integ (x) operand), with the bound variables as bare names and
                // NOT counted as operand nodes.
                if name == "integrate" && args.len() >= 2 {
                    let variables: Option<Vec<String>> = args[1..]
                        .iter()
                        .map(|a| match a {
                            Expr::Sym(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect();
                    if let Some(variables) = variables {
                        return format!(
                            "(integ ({}) {})",
                            variables.join(" "),
                            args[0].canonical()
                        );
                    }
                }
                match (name.as_str(), args.len()) {
                    ("pi", 0) => "(pi)".to_string(),
                    ("e", 0) => "(e)".to_string(),
                    ("i", 0) => "(i)".to_string(),
                    ("inf", 0) => "(inf)".to_string(),
                    // A root of a defining polynomial: the polynomial is a bound-variable form,
                    // not a function argument, so it is printed directly rather than as (fn ...).
                    // The series tail is a BINDING node in the reference's canonical vocabulary:
                    // its variable, point and degree are operands, not a function call.
                    ("order", 3) => format!(
                        "(order {} {} {})",
                        args[0].canonical(),
                        args[1].canonical(),
                        args[2].canonical()
                    ),
                    ("rootof", 2) => {
                        // The root index is an integer position, printed as one (0, not (rat 0 1)).
                        let index = match &args[1] {
                            Expr::Num(n) => n.numerator().to_decimal(),
                            other => other.canonical(),
                        };
                        format!("(rootof {} {})", args[0].canonical(), index)
                    }
                    // A piecewise is a BINDING node: its guards and values are operands, not a
                    // function call, and the reference publishes (pw ((guard value)) otherwise).
                    ("piecewise", _) => {
                        if let Some((branches, otherwise)) = self.as_piecewise() {
                            let pairs: Vec<String> = branches
                                .iter()
                                .map(|(guard, value)| {
                                    format!("({} {})", guard.canonical(), value.canonical())
                                })
                                .collect();
                            // The reference emits each branch as its own (guard value) operand:
                            // (pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) ...)).
                            format!("(pw {} {})", pairs.join(" "), otherwise.canonical())
                        } else {
                            format!("(fn {})", name)
                        }
                    }
                    // An unevaluated derivative binds its variables: (der (x) (fn abs (sym x))).
                    ("derivative", _) => {
                        if let Some((body, vars)) = self.as_derivative() {
                            format!("(der ({}) {})", vars.join(" "), body.canonical())
                        } else {
                            format!("(fn {})", name)
                        }
                    }
                    _ => {
                        let parts: Vec<String> = args.iter().map(|a| a.canonical()).collect();
                        if parts.is_empty() {
                            format!("(fn {})", name)
                        } else {
                            format!("(fn {} {})", name, parts.join(" "))
                        }
                    }
                }
            }
            Expr::Rel(op, a, b) => {
                let tag = match op {
                    RelOp::Eq => "eq",
                    RelOp::Ne => "ne",
                    RelOp::Lt => "lt",
                    RelOp::Le => "le",
                    RelOp::Gt => "gt",
                    RelOp::Ge => "ge",
                };
                format!("({} {} {})", tag, a.canonical(), b.canonical())
            }
        }
    }

    /// The number of nodes in the expression tree (a rational or a symbol is one node).
    pub fn node_count(&self) -> usize {
        match self {
            Expr::Num(_) | Expr::Real(_) | Expr::Sym(_) => 1,
            Expr::Add(items) | Expr::Mul(items) => {
                1 + items.iter().map(|i| i.node_count()).sum::<usize>()
            }
            Expr::Pow(a, b) => 1 + a.node_count() + b.node_count(),
            Expr::Div(a, b) => 1 + a.node_count() + b.node_count(),
            Expr::Fun(name, args) => {
                // sqrt is canonically a power, so it carries the power's node count.
                if name == "sqrt" && args.len() == 1 {
                    return 1 + args[0].node_count() + 1;
                }
                // A RootOf abstracts its defining polynomial into one term per coefficient:
                // the reference counts it as 1 + the polynomial's term count (x^4-x^2-1 -> 4).
                if name == "rootof" && args.len() == 2 {
                    let terms = match &args[0] {
                        Expr::Add(items) => items.len(),
                        _ => 1,
                    };
                    return 1 + terms;
                }
                // An integral counts its operand only; the bound variables are binding syntax,
                // not operand nodes (the reference's Exprs.Integral: 1 + operand.NodeCount).
                if name == "integrate"
                    && args.len() >= 2
                    && args[1..].iter().all(|a| matches!(a, Expr::Sym(_)))
                {
                    return 1 + args[0].node_count();
                }
                // A piecewise counts one node per branch pair plus the otherwise value; a
                // derivative counts its body and one node per bound variable.
                if name == "piecewise" {
                    if let Some((branches, otherwise)) = self.as_piecewise() {
                        let branch_nodes: usize = branches
                            .iter()
                            .map(|(guard, value)| guard.node_count() + value.node_count())
                            .sum();
                        return 1 + branch_nodes + otherwise.node_count();
                    }
                }
                if name == "derivative" {
                    if let Some((body, vars)) = self.as_derivative() {
                        return 1 + body.node_count() + vars.len();
                    }
                }
                1 + args.iter().map(|a| a.node_count()).sum::<usize>()
            }
            Expr::Rel(_, a, b) => 1 + a.node_count() + b.node_count(),
        }
    }

    /// Every free symbol name, sorted.
    pub fn free_symbols(&self) -> Vec<String> {
        let mut names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        fn walk(e: &Expr, out: &mut std::collections::BTreeSet<String>) {
            match e {
                Expr::Sym(s) => {
                    out.insert(s.clone());
                }
                Expr::Add(items) | Expr::Mul(items) => items.iter().for_each(|i| walk(i, out)),
                Expr::Pow(a, b) | Expr::Div(a, b) | Expr::Rel(_, a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                // rootof's defining polynomial binds its own variable: no free symbols.
                Expr::Fun(name, _) if name == "rootof" => {}
                // A derivative binds the variables it differentiates by; its body is free.
                Expr::Fun(name, _) if name == "derivative" => {
                    if let Some((body, _)) = e.as_derivative() {
                        walk(&body, out);
                    }
                }
                Expr::Fun(_, args) => args.iter().for_each(|a| walk(a, out)),
                Expr::Num(_) | Expr::Real(_) => {}
            }
        }
        walk(self, &mut names);
        names.into_iter().collect()
    }

    /// True when no leaf denotes an approximation or a transcendental constant.
    pub fn is_exact(&self) -> bool {
        match self {
            Expr::Real(_) => false,
            Expr::Num(_) | Expr::Sym(_) => true,
            Expr::Add(items) | Expr::Mul(items) => items.iter().all(|i| i.is_exact()),
            Expr::Pow(a, b) | Expr::Div(a, b) | Expr::Rel(_, a, b) => a.is_exact() && b.is_exact(),
            Expr::Fun(name, args) => match (name.as_str(), args.len()) {
                // The imaginary unit denotes an exact value; pi, e and infinity do not.
                ("i", 0) => true,
                ("rootof", 2) => true,
                ("pi", 0) | ("e", 0) | ("inf", 0) => false,
                _ => args.iter().all(|a| a.is_exact()),
            },
        }
    }

    /// The structural best-known domain of the expression.
    pub fn domain(&self) -> Dom {
        match self {
            Expr::Num(_) => Dom::Rational,
            // A real constant is a decimal value the numeric tier handed over.
            Expr::Real(_) => Dom::Real,
            // An unconstrained symbol ranges over the complex field.
            Expr::Sym(_) => Dom::Complex,
            Expr::Add(items) | Expr::Mul(items) => items
                .iter()
                .map(|i| i.domain())
                .max()
                .unwrap_or(Dom::Integer),
            Expr::Pow(base, exp) => {
                let bd = base.domain();
                if let Expr::Num(n) = exp.as_ref() {
                    if n.denominator().to_decimal() == "1" {
                        if n.is_negative() {
                            // A negative integer power is rational-valued where defined.
                            return match bd {
                                Dom::Integer | Dom::Rational => Dom::Rational,
                                other => other,
                            };
                        }
                        return bd;
                    }
                }
                // A non-integer exponent is real only under a provable non-negative base.
                if bd != Dom::Complex {
                    if let Some(v) = numeric_rational(base) {
                        if !v.is_negative() {
                            return Dom::Real;
                        }
                    }
                }
                Dom::Complex
            }
            Expr::Div(a, b) => a.domain().max(b.domain()),
            Expr::Rel(..) => Dom::Complex,
            Expr::Fun(name, args) => match (name.as_str(), args.len()) {
                ("pi", 0) | ("e", 0) | ("inf", 0) => Dom::Real,
                ("i", 0) => Dom::Complex,
                // A RootOf is the i-th REAL root of its defining polynomial.
                ("rootof", 2) => Dom::Real,
                // A piecewise takes the widest domain over its values, guards included: the
                // branch values are the expression's possible values.
                ("piecewise", _) => match self.as_piecewise() {
                    Some((branches, otherwise)) => branches
                        .iter()
                        .map(|(guard, value)| guard.domain().max(value.domain()))
                        .fold(otherwise.domain(), |acc: Dom, d| acc.max(d)),
                    None => Dom::Complex,
                },
                // An unevaluated derivative of a real-valued body is real.
                ("derivative", _) => match self.as_derivative() {
                    Some((body, _)) => body.domain(),
                    None => Dom::Complex,
                },
                ("sqrt", 1) => {
                    let ad = args[0].domain();
                    if ad == Dom::Complex {
                        return Dom::Complex;
                    }
                    if let Some(v) = numeric_rational(&args[0]) {
                        return if v.is_negative() {
                            Dom::Complex
                        } else {
                            Dom::Real
                        };
                    }
                    Dom::Real
                }
                _ => {
                    let ad = if args.is_empty() {
                        Dom::Real
                    } else {
                        args[0].domain()
                    };
                    if name == "abs" || name == "re" || name == "im" {
                        return Dom::Real;
                    }
                    if ad == Dom::Complex {
                        Dom::Complex
                    } else {
                        Dom::Real
                    }
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Print modes, the debug rendering and the canonical-text parser (round 16).
//
// The reference surface is "Lovelace.Symbolics.Printing": PrettyPrint(expr, options) dispatches
// Canonical/Pretty/Debug/Latex, DebugPrint is the structural form agents read, and CanonicalParse
// turns the S-expression form back into an expression. This section gives the Rust model the same
// surface, one printer per mode and one parser, so a canonical rendering is round-trippable
// without going through the infix grammar.
// ---------------------------------------------------------------------------

/// The four printer modes of the reference "Printing.PrintMode": pretty (the infix display),
/// canonical (the S-expression wire form), LaTeX, and debug (the kind-annotated structural form).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrintMode {
    /// The human-oriented infix form.
    Pretty,
    /// The canonical S-expression form.
    Canonical,
    /// The LaTeX rendering.
    Latex,
    /// The kind-annotated structural form.
    Debug,
}

impl PrintMode {
    /// Every accepted mode name, in the order a refusal lists them.
    pub const NAMES: [&'static str; 4] = ["pretty", "canonical", "latex", "debug"];

    /// Resolves a mode name. The comparison is ASCII case-insensitive, so "Debug" and "debug"
    /// name the same mode. There is no fallback: an unknown name is an error, never Pretty.
    pub fn from_name(name: &str) -> Result<PrintMode, UnknownPrintMode> {
        for mode in [
            PrintMode::Pretty,
            PrintMode::Canonical,
            PrintMode::Latex,
            PrintMode::Debug,
        ] {
            if name.eq_ignore_ascii_case(mode.name()) {
                return Ok(mode);
            }
        }
        Err(UnknownPrintMode {
            name: name.to_string(),
        })
    }

    /// The mode's documented lower-case name.
    pub fn name(self) -> &'static str {
        match self {
            PrintMode::Pretty => "pretty",
            PrintMode::Canonical => "canonical",
            PrintMode::Latex => "latex",
            PrintMode::Debug => "debug",
        }
    }
}

/// A mode name that PrintMode::from_name does not know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownPrintMode {
    /// The name exactly as supplied.
    pub name: String,
}

impl std::fmt::Display for UnknownPrintMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown print mode '{}'; expected one of: {}",
            self.name,
            PrintMode::NAMES.join(", ")
        )
    }
}

impl std::error::Error for UnknownPrintMode {}

impl Expr {
    /// Renders the expression through the printer arm named by "mode". The four arms are the
    /// reference's Printing.PrettyPrint dispatch: Canonical is canonical(), Debug is debug_print(),
    /// Latex is latex() and Pretty is the display form.
    pub fn print(&self, mode: PrintMode) -> String {
        match mode {
            PrintMode::Pretty => self.to_string(),
            PrintMode::Canonical => self.canonical(),
            PrintMode::Latex => self.latex(),
            PrintMode::Debug => self.debug_print(),
        }
    }

    /// print by mode NAME, refusing a name outside PrintMode::NAMES.
    pub fn print_named(&self, mode: &str) -> Result<String, UnknownPrintMode> {
        PrintMode::from_name(mode).map(|m| self.print(m))
    }

    /// The structural, kind-annotated rendering of the reference Printing.DebugPrint: every node
    /// carries its kind and only call arguments are parenthesised, so a reader never has to infer
    /// structure from the infix form.
    ///
    /// Add and multiply operands are visited in the kernel's canonical order, which is the order
    /// the reference expression is stored in. The Rust model's own term order is a DISPLAY order
    /// (degree first); visiting that order would print a shape the reference expression does not
    /// have.
    pub fn debug_print(&self) -> String {
        match self {
            Expr::Num(n) => format!("rat({})", n),
            Expr::Real(text) => format!("real({text})"),
            Expr::Sym(s) => format!("sym({})", s),
            Expr::Add(terms) => {
                let mut ordered = terms.clone();
                ordered.sort_by(canonical_order);
                format!(
                    "add[{}]",
                    ordered
                        .iter()
                        .map(Expr::debug_print)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Expr::Mul(factors) => {
                let mut ordered = factors.clone();
                ordered.sort_by(canonical_order);
                format!(
                    "mul[{}]",
                    ordered
                        .iter()
                        .map(Expr::debug_print)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Expr::Pow(base, exp) => {
                format!("pow[{}, {}]", base.debug_print(), exp.debug_print())
            }
            Expr::Fun(name, args) => {
                // The reference's named constants are their own node kind; the Rust model carries
                // them as zero-argument functions with the documented spellings.
                if args.is_empty() {
                    match name.as_str() {
                        "pi" => return "named(Pi)".to_string(),
                        "e" => return "named(E)".to_string(),
                        "i" => return "named(I)".to_string(),
                        "inf" => return "named(Infinity)".to_string(),
                        _ => {}
                    }
                }
                // sqrt is a POWER in the canonical form (the reference constructor builds one),
                // so its debug shape is the power's, not a function call.
                if name == "sqrt" && args.len() == 1 {
                    return format!("pow[{}, rat(1/2)]", args[0].debug_print());
                }
                // A RootOf's index is an integer POSITION, printed as one, exactly as the
                // reference prints the RootIndex field.
                if name == "rootof" && args.len() == 2 {
                    let index = match &args[1] {
                        Expr::Num(n) if n.is_integer() => n.numerator().to_decimal(),
                        other => other.debug_print(),
                    };
                    return format!("rootof[{}, {}]", args[0].debug_print(), index);
                }
                // The reference has dedicated logic nodes; the Rust model keeps them as
                // functions, but the debug shape is the reference's.
                if matches!(name.as_str(), "and" | "or" | "not") {
                    let body = args
                        .iter()
                        .map(Expr::debug_print)
                        .collect::<Vec<_>>()
                        .join(", ");
                    return format!("{}[{}]", name, body);
                }
                format!(
                    "fn:{}({})",
                    name,
                    args.iter()
                        .map(Expr::debug_print)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            // An explicit quotient is the reference's multiply-by-reciprocal shape; its debug form
            // is the same structure the canonical form prints.
            Expr::Div(a, b) => {
                // The same multiply-by-reciprocal shape and the same canonical operand order the
                // canonical arm uses, so Debug and Canonical never describe different trees.
                let mut parts = [
                    (**a).clone(),
                    Expr::Pow(
                        Box::new((**b).clone()),
                        Box::new(Expr::Num(Rat::from_i64(-1))),
                    ),
                ];
                parts.sort_by(canonical_order);
                format!(
                    "mul[{}]",
                    parts
                        .iter()
                        .map(Expr::debug_print)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Expr::Rel(op, a, b) => format!(
                "rel:{}({}, {})",
                rel_op_name(*op),
                a.debug_print(),
                b.debug_print()
            ),
        }
    }
}

/// The reference enum spelling of a relation operator, used by the debug form.
fn rel_op_name(op: RelOp) -> &'static str {
    match op {
        RelOp::Eq => "Eq",
        RelOp::Ne => "Ne",
        RelOp::Lt => "Lt",
        RelOp::Le => "Le",
        RelOp::Gt => "Gt",
        RelOp::Ge => "Ge",
    }
}

/// A canonical-text parse failure: the byte offset and the expectation that failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalParseError {
    /// Byte offset into the input text.
    pub position: usize,
    /// What the parser expected and what it found.
    pub message: String,
}

impl std::fmt::Display for CanonicalParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at position {}", self.message, self.position)
    }
}

impl std::error::Error for CanonicalParseError {}

/// Parses the canonical S-expression form (the wire encoding Expr::canonical emits) back into an
/// expression.
///
/// The grammar is the tagged prefix language: (rat N D), (sym NAME), (add ...), (mul ...),
/// (pow A B), (fn NAME ARG...), the relation tags, (rootof POLY INDEX) and the named constants
/// (pi)/(e)/(i)/(inf). The oracle's (int N) alias is accepted for compatibility; the Rust model
/// has ONE numeric leaf, so it canonicalises to (rat N 1). Tags the model has no node for
/// (real/complex literals, derivative, integral, piecewise, logic, order) are refused rather than
/// guessed.
///
/// The round-trip guarantee is over THIS crate's canonical output: for every expression e,
/// canonical_parse(e.canonical()).unwrap().canonical() == e.canonical().
pub fn canonical_parse(text: &str) -> Result<Expr, CanonicalParseError> {
    let tokens = tokenize_canonical(text);
    let mut parser = CanonicalParser {
        tokens: &tokens,
        pos: 0,
        text_len: text.len(),
    };
    let expr = parser.parse_node()?;
    if parser.pos != parser.tokens.len() {
        return Err(parser.error_at_current("trailing tokens after the expression"));
    }
    Ok(expr)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CanonicalToken {
    Open(usize),
    Close(usize),
    Atom(String, usize),
}

fn tokenize_canonical(text: &str) -> Vec<CanonicalToken> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '(' {
            tokens.push(CanonicalToken::Open(i));
            continue;
        }
        if c == ')' {
            tokens.push(CanonicalToken::Close(i));
            continue;
        }
        let start = i;
        let mut end = i + c.len_utf8();
        while let Some(&(j, next)) = chars.peek() {
            if next.is_whitespace() || next == '(' || next == ')' {
                break;
            }
            chars.next();
            end = j + next.len_utf8();
        }
        tokens.push(CanonicalToken::Atom(text[start..end].to_string(), start));
    }
    tokens
}

struct CanonicalParser<'a> {
    tokens: &'a [CanonicalToken],
    pos: usize,
    text_len: usize,
}

impl CanonicalParser<'_> {
    fn error(&self, position: usize, message: impl Into<String>) -> CanonicalParseError {
        CanonicalParseError {
            position,
            message: message.into(),
        }
    }

    fn error_at_current(&self, message: impl Into<String>) -> CanonicalParseError {
        let position = match self.tokens.get(self.pos) {
            Some(CanonicalToken::Open(p))
            | Some(CanonicalToken::Close(p))
            | Some(CanonicalToken::Atom(_, p)) => *p,
            None => self.text_len,
        };
        self.error(position, message)
    }

    fn next(&mut self) -> Result<CanonicalToken, CanonicalParseError> {
        let token = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or_else(|| self.error(self.text_len, "unexpected end of input"))?;
        self.pos += 1;
        Ok(token)
    }

    fn next_atom(&mut self, expectation: &str) -> Result<String, CanonicalParseError> {
        match self.next()? {
            CanonicalToken::Atom(text, _) => Ok(text),
            CanonicalToken::Open(p) => {
                Err(self.error(p, format!("expected {expectation}, got '('")))
            }
            CanonicalToken::Close(p) => {
                Err(self.error(p, format!("expected {expectation}, got ')'")))
            }
        }
    }

    fn expect_close(&mut self) -> Result<(), CanonicalParseError> {
        match self.next()? {
            CanonicalToken::Close(_) => Ok(()),
            CanonicalToken::Open(p) => Err(self.error(p, "expected ')', got '('")),
            CanonicalToken::Atom(text, p) => {
                Err(self.error(p, format!("expected ')', got '{text}'")))
            }
        }
    }

    fn parse_nodes_until_close(&mut self) -> Result<Vec<Expr>, CanonicalParseError> {
        let mut nodes = Vec::new();
        loop {
            match self.tokens.get(self.pos) {
                None => {
                    return Err(self.error(self.text_len, "unexpected end of input; expected ')'"));
                }
                Some(CanonicalToken::Close(_)) => {
                    self.pos += 1;
                    return Ok(nodes);
                }
                Some(_) => nodes.push(self.parse_node()?),
            }
        }
    }

    fn parse_relation(&mut self, op: RelOp, tag: &str) -> Result<Expr, CanonicalParseError> {
        let left = self.parse_node()?;
        let right = self.parse_node()?;
        self.expect_close().map_err(|mut e| {
            e.message = format!("({tag}) requires exactly two operands: {}", e.message);
            e
        })?;
        Ok(Expr::Rel(op, Box::new(left), Box::new(right)))
    }

    fn parse_node(&mut self) -> Result<Expr, CanonicalParseError> {
        match self.next()? {
            CanonicalToken::Open(_) => {}
            CanonicalToken::Close(p) => return Err(self.error(p, "expected '(', got ')'")),
            CanonicalToken::Atom(text, p) => {
                return Err(self.error(p, format!("expected '(', got '{text}'")));
            }
        }
        let tag_position = match self.tokens.get(self.pos) {
            Some(CanonicalToken::Atom(_, p)) => *p,
            _ => return Err(self.error_at_current("expected a tag after '('")),
        };
        let tag = self.next_atom("a tag after '('")?;
        match tag.as_str() {
            "int" => {
                let text = self.next_atom("an integer")?;
                let value = Int::from_decimal(&text).ok_or_else(|| {
                    self.error(tag_position, format!("(int) needs an integer, got '{text}'"))
                })?;
                self.expect_close()?;
                Ok(Expr::Num(Rat::new(value, Nat::from_u64(1))))
            }
            "rat" => {
                let numerator_text = self.next_atom("a numerator")?;
                let denominator_text = self.next_atom("a denominator")?;
                let numerator = Int::from_decimal(&numerator_text).ok_or_else(|| {
                    self.error(
                        tag_position,
                        format!("(rat) needs an integer numerator, got '{numerator_text}'"),
                    )
                })?;
                let denominator = Nat::from_decimal(&denominator_text).ok_or_else(|| {
                    self.error(
                        tag_position,
                        format!(
                            "(rat) needs a non-negative integer denominator, got '{denominator_text}'"
                        ),
                    )
                })?;
                if denominator.is_zero() {
                    return Err(self.error(tag_position, "(rat) denominator must not be zero"));
                }
                self.expect_close()?;
                Ok(Expr::Num(Rat::new(numerator, denominator)))
            }
            "sym" => {
                let name = self.next_atom("a symbol name")?;
                self.expect_close()?;
                Ok(Expr::Sym(name))
            }
            "pi" | "e" | "i" | "inf" => {
                self.expect_close()?;
                Ok(Expr::Fun(tag, Vec::new()))
            }
            "add" => Ok(Expr::Add(self.parse_nodes_until_close()?)),
            "mul" => Ok(Expr::Mul(self.parse_nodes_until_close()?)),
            "pow" => {
                let base = self.parse_node()?;
                let exponent = self.parse_node()?;
                self.expect_close()?;
                Ok(Expr::Pow(Box::new(base), Box::new(exponent)))
            }
            "fn" => {
                let name = self.next_atom("a function name")?;
                let args = self.parse_nodes_until_close()?;
                Ok(Expr::Fun(name, args))
            }
            "eq" => self.parse_relation(RelOp::Eq, "eq"),
            "ne" => self.parse_relation(RelOp::Ne, "ne"),
            "lt" => self.parse_relation(RelOp::Lt, "lt"),
            "le" => self.parse_relation(RelOp::Le, "le"),
            "gt" => self.parse_relation(RelOp::Gt, "gt"),
            "ge" => self.parse_relation(RelOp::Ge, "ge"),
            "rootof" => {
                let polynomial = self.parse_node()?;
                let index_text = self.next_atom("a root index")?;
                let index = Int::from_decimal(&index_text).ok_or_else(|| {
                    self.error(
                        tag_position,
                        format!("(rootof) needs an integer index, got '{index_text}'"),
                    )
                })?;
                self.expect_close()?;
                Ok(Expr::Fun(
                    "rootof".to_string(),
                    vec![polynomial, Expr::Num(Rat::new(index, Nat::from_u64(1)))],
                ))
            }
            "integ" => {
                // (integ (x) OPERAND): bound variables are bare names in one parenthesised list.
                match self.next()? {
                    CanonicalToken::Open(_) => {}
                    CanonicalToken::Close(p) => {
                        return Err(self.error(p, "expected '(' before the bound variables"))
                    }
                    CanonicalToken::Atom(text, p) => {
                        return Err(self.error(
                            p,
                            format!("expected '(' before the bound variables, got '{text}'"),
                        ))
                    }
                }
                let mut variables: Vec<Expr> = Vec::new();
                loop {
                    match self.next()? {
                        CanonicalToken::Atom(name, _) => variables.push(Expr::sym(&name)),
                        CanonicalToken::Close(_) => break,
                        CanonicalToken::Open(p) => {
                            return Err(self.error(p, "expected a bound variable name, got '('"))
                        }
                    }
                }
                if variables.is_empty() {
                    return Err(self.error(
                        tag_position,
                        "(integ) needs at least one bound variable",
                    ));
                }
                let operand = self.parse_node()?;
                self.expect_close()?;
                let mut args = vec![operand];
                args.extend(variables);
                Ok(Expr::Fun("integrate".to_string(), args))
            }
            "real" | "cplx" => Err(self.error(
                tag_position,
                format!(
                    "the tag '{tag}' denotes a numeric literal this expression model has no leaf for"
                ),
            )),
            "der" | "pw" | "and" | "or" | "not" | "order" => Err(self.error(
                tag_position,
                format!("the tag '{tag}' is not part of this crate's canonical vocabulary"),
            )),
            other => Err(self.error(tag_position, format!("unknown tag '{other}'"))),
        }
    }
}

/// The node kind's rank in the kernel's total term order: a numeric constant sorts first, then a
/// symbol, a named constant, a function application, a power, a product, a sum and a relation.
fn canonical_rank(e: &Expr) -> u8 {
    match e {
        Expr::Num(_) | Expr::Real(_) => 0,
        Expr::Sym(_) => 1,
        Expr::Fun(name, args)
            if args.is_empty() && matches!(name.as_str(), "pi" | "e" | "inf" | "i") =>
        {
            2
        }
        // The series tail sorts after every computed term: (add (rat 1 1) (sym x) (mul ...) (order ...)).
        Expr::Fun(name, args) if name == "order" && args.len() == 3 => 8,
        Expr::Fun(..) => 3,
        Expr::Pow(..) => 4,
        Expr::Mul(_) | Expr::Div(..) => 5,
        Expr::Add(_) => 6,
        Expr::Rel(..) => 7,
    }
}

/// The reference's NamedConstant ordinal (Pi, E, I, Infinity): the total order over the four
/// named constants, which the canonical form of a product follows.
fn named_constant_rank(name: &str) -> Option<u8> {
    match name {
        "pi" => Some(0),
        "e" => Some(1),
        "i" => Some(2),
        "inf" => Some(3),
        _ => None,
    }
}

/// The kernel's total order over canonical expressions, used for Add and Multiply operand order.
pub fn canonical_order(a: &Expr, b: &Expr) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (ra, rb) = (canonical_rank(a), canonical_rank(b));
    if ra != rb {
        return ra.cmp(&rb);
    }
    fn seq(a: &[Expr], b: &[Expr]) -> Ordering {
        for (x, y) in a.iter().zip(b.iter()) {
            let c = canonical_order(x, y);
            if c != Ordering::Equal {
                return c;
            }
        }
        a.len().cmp(&b.len())
    }
    match (a, b) {
        (Expr::Num(x), Expr::Num(y)) => x.cmp(y),
        (Expr::Sym(x), Expr::Sym(y)) => x.cmp(y),
        (Expr::Fun(n1, a1), Expr::Fun(n2, a2)) => {
            // The reference's NamedConstant enum ordinal (Pi, E, I, Infinity) orders the four
            // named constants, so a product's canonical form is (pi) (i), not (i) (pi).
            if a1.is_empty() && a2.is_empty() {
                if let (Some(r1), Some(r2)) = (named_constant_rank(n1), named_constant_rank(n2)) {
                    return r1.cmp(&r2);
                }
            }
            n1.cmp(n2).then_with(|| seq(a1, a2))
        }
        (Expr::Pow(b1, e1), Expr::Pow(b2, e2)) => {
            canonical_order(b1, b2).then_with(|| canonical_order(e1, e2))
        }
        (Expr::Mul(f1), Expr::Mul(f2)) => seq(f1, f2),
        (Expr::Div(a1, b1), Expr::Div(a2, b2)) => {
            canonical_order(a1, a2).then_with(|| canonical_order(b1, b2))
        }
        (Expr::Add(t1), Expr::Add(t2)) => seq(t1, t2),
        (Expr::Rel(o1, l1, r1), Expr::Rel(o2, l2, r2)) => {
            let rank = |o: &RelOp| match o {
                RelOp::Eq => 0,
                RelOp::Ne => 1,
                RelOp::Lt => 2,
                RelOp::Le => 3,
                RelOp::Gt => 4,
                RelOp::Ge => 5,
            };
            rank(o1)
                .cmp(&rank(o2))
                .then_with(|| canonical_order(l1, l2))
                .then_with(|| canonical_order(r1, r2))
        }
        _ => Ordering::Equal,
    }
}

/// The exact rational value of a numeric expression, when it has one.
fn numeric_rational(e: &Expr) -> Option<Rat> {
    match e {
        Expr::Num(n) => Some(n.clone()),
        Expr::Mul(items) => {
            let mut acc = Rat::one();
            for i in items {
                acc = acc.mul(&numeric_rational(i)?);
            }
            Some(acc)
        }
        Expr::Add(items) => {
            let mut acc = Rat::zero();
            for i in items {
                acc = acc.add(&numeric_rational(i)?);
            }
            Some(acc)
        }
        Expr::Pow(b, e) => {
            let base = numeric_rational(b)?;
            if let Expr::Num(n) = e.as_ref() {
                if n.denominator().to_decimal() == "1" {
                    let k = n.to_i64()?;
                    return base.pow_i64(k);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod panic_sweep_round23;

#[cfg(test)]
mod complex_closed_forms_round30;

#[cfg(test)]
mod indeterminate_forms_round31;

#[cfg(test)]
mod round36_exactness;
#[cfg(test)]
mod tail_round34;

#[cfg(test)]
mod round39_residue;

#[cfg(test)]
mod printing_round16;

#[cfg(test)]
mod tests {
    #![allow(non_snake_case)]
    use super::*;

    fn s(e: Expr) -> String {
        e.to_string()
    }

    #[test]
    fn Print_GivenPolynomial_UsesDescendingDegree() {
        let x = Expr::sym("x");
        let e = Expr::add(
            Expr::add(Expr::pow(x.clone(), Expr::num(2)), Expr::num(1)),
            Expr::mul(Expr::num(-2), x),
        );
        assert_eq!(s(e), "x^2 - 2*x + 1");
    }

    #[test]
    fn Print_GivenLikeTerms_Collects() {
        let x = Expr::sym("x");
        let e = Expr::add(
            Expr::mul(Expr::num(2), x.clone()),
            Expr::mul(Expr::num(3), x),
        );
        assert_eq!(s(e), "5*x");
    }

    #[test]
    fn Print_GivenTwoKinds_OrdersSymbolBeforeProduct() {
        let x = Expr::sym("x");
        let y = Expr::sym("y");
        let e = Expr::add(Expr::add(x, y), Expr::sym("x"));
        assert_eq!(s(e), "y + 2*x");
    }

    #[test]
    fn Mul_GivenSameBase_PowersCombine() {
        let x = Expr::sym("x");
        let e = Expr::mul(
            Expr::pow(x.clone(), Expr::num(2)),
            Expr::pow(x, Expr::num(3)),
        );
        assert_eq!(s(e), "x^5");
    }

    #[test]
    fn Add_GivenZeroTerm_Disappears() {
        let x = Expr::sym("x");
        assert_eq!(s(Expr::add(x.clone(), Expr::num(0))), "x");
        assert_eq!(s(Expr::mul(x, Expr::num(0))), "0");
    }

    #[test]
    fn Diff_GivenCubic_MatchesDocumentedDerivative() {
        let x = Expr::sym("x");
        // x^3 + 2*x^2 + 5*x + 7
        let f = Expr::add(
            Expr::add(
                Expr::pow(x.clone(), Expr::num(3)),
                Expr::mul(Expr::num(2), Expr::pow(x.clone(), Expr::num(2))),
            ),
            Expr::add(Expr::mul(Expr::num(5), x), Expr::num(7)),
        );
        assert_eq!(s(f.clone()), "x^3 + 2*x^2 + 5*x + 7");
        assert_eq!(s(diff(&f, "x")), "3*x^2 + 4*x + 5");
    }

    #[test]
    fn Diff_GivenProduct_AppliesProductRule() {
        let x = Expr::sym("x");
        let f = Expr::mul(x.clone(), Expr::Fun("exp".to_string(), vec![x]));
        assert_eq!(s(diff(&f, "x")), "exp(x) + x*exp(x)");
    }

    #[test]
    fn Series_GivenSinOverX_MatchesDocumentedExpansion() {
        let x = Expr::sym("x");
        let f = Expr::div(Expr::Fun("sin".to_string(), vec![x.clone()]), x.clone());
        let ser = series(&f, "x", 8).expect("series");
        let rendered: Vec<String> = ser.iter().map(|(e, c)| format!("{}*x^{}", c, e)).collect();
        assert_eq!(rendered.join(" "), "1*x^0 -1/6*x^2 1/120*x^4 -1/5040*x^6");
    }

    #[test]
    fn Series_GivenExp_MatchesFactorialCoefficients() {
        let x = Expr::sym("x");
        let f = Expr::Fun("exp".to_string(), vec![x]);
        let ser = series(&f, "x", 5).expect("series");
        let coeffs: Vec<String> = ser.iter().map(|(_, c)| c.to_string()).collect();
        assert_eq!(coeffs.join(" "), "1 1 1/2 1/6 1/24");
    }

    #[test]
    fn Solve_GivenLinear_ReturnsSingleRoot() {
        // 2x - 4 = 0
        let coeffs = vec![Rat::from_i64(-4), Rat::from_i64(2)];
        match solve_polynomial(&coeffs) {
            SolveOutcome::Roots(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].value.to_string(), "2");
                assert_eq!(r[0].multiplicity, 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenQuarticWithRationalRoots_ReturnsAllFour() {
        // x^4 - 5x^2 + 4
        let coeffs = vec![
            Rat::from_i64(4),
            Rat::zero(),
            Rat::from_i64(-5),
            Rat::zero(),
            Rat::from_i64(1),
        ];
        match solve_polynomial(&coeffs) {
            SolveOutcome::Roots(r) => {
                let rendered: Vec<String> = r.iter().map(|x| x.value.to_string()).collect();
                assert_eq!(rendered.join(" "), "-2 -1 1 2");
                assert!(r.iter().all(|root| root.multiplicity == 1));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenRepeatedRoot_PreservesMultiplicity() {
        // x^2 - 2x + 1 = (x - 1)^2: ONE root, multiplicity 2 (the oracle's --eval value).
        let coeffs = vec![Rat::from_i64(1), Rat::from_i64(-2), Rat::from_i64(1)];
        match solve_polynomial(&coeffs) {
            SolveOutcome::Roots(r) => {
                assert_eq!(
                    r.len(),
                    1,
                    "repeated root must not be duplicated nor collapsed in count"
                );
                assert_eq!(r[0].value.to_string(), "1");
                assert_eq!(r[0].multiplicity, 2);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenTripleRoot_PreservesMultiplicityThree() {
        // x^3 - 3x^2 + 3x - 1 = (x - 1)^3.
        let coeffs = vec![
            Rat::from_i64(-1),
            Rat::from_i64(3),
            Rat::from_i64(-3),
            Rat::from_i64(1),
        ];
        match solve_polynomial(&coeffs) {
            SolveOutcome::Roots(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].multiplicity, 3);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenCubicWithoutRationalRoot_ReturnsCardanoRoots() {
        // x^3 - 2: the regression for the index-out-of-bounds panic (the old code reached
        // coeffs[4] in solve_biquadratic). The reference publishes three exact roots, one of
        // which is 2^(1/3).
        let coeffs = vec![Rat::from_i64(-2), Rat::zero(), Rat::zero(), Rat::one()];
        match solve_polynomial(&coeffs) {
            SolveOutcome::RootExpressions(exprs) => {
                assert_eq!(exprs.len(), 3, "a cubic has three roots");
                assert!(
                    exprs.iter().any(|e| e.to_string() == "2^(1/3)"),
                    "no 2^(1/3) among {:?}",
                    exprs.iter().map(|e| e.to_string()).collect::<Vec<_>>()
                );
                assert!(exprs.iter().all(|e| e.is_exact()));
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenCubicWithoutRationalRoot_DoesNotPanicForEitherDiscriminantSign() {
        // x^3 + x + 1 has disc > 0 (one real root); x^3 - 3x + 1 has disc < 0 (three real roots,
        // the trigonometric branch). Both used to reach coeffs[4] and panic.
        let one_real = vec![Rat::from_i64(1), Rat::from_i64(1), Rat::zero(), Rat::one()];
        let three_real = vec![Rat::from_i64(1), Rat::from_i64(-3), Rat::zero(), Rat::one()];
        for coeffs in [one_real, three_real] {
            match solve_polynomial(&coeffs) {
                SolveOutcome::RootExpressions(exprs) => assert_eq!(exprs.len(), 3),
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn Solve_GivenNonzeroConstant_IsNoSolutions() {
        let coeffs = vec![Rat::from_i64(1)];
        assert!(matches!(
            solve_polynomial(&coeffs),
            SolveOutcome::NoSolutions
        ));
    }

    #[test]
    fn Apart_GivenOneOverDifferenceOfSquares_SplitsWithResidues() {
        let x = Expr::sym("x");
        let denominator = Expr::sub(Expr::pow(x, Expr::num(2)), Expr::num(1));
        let f = Expr::div(Expr::num(1), denominator);
        let parts = apart(&f, "x").expect("apart");
        assert_eq!(parts.to_string(), "-1/2/(x + 1) + 1/2/(x - 1)");
    }

    #[test]
    fn PolyDiv_GivenExactDivision_ReturnsQuotient() {
        // (x^2 - 1) / (x - 1) = x + 1
        let num = vec![Rat::from_i64(-1), Rat::zero(), Rat::one()];
        let den = vec![Rat::from_i64(-1), Rat::one()];
        let q = poly_div(&num, &den).expect("division");
        assert_eq!(poly_to_expr(&q, "x").to_string(), "x + 1");
    }

    #[test]
    fn EvalAt_GivenPolynomial_ComputesExactly() {
        let x = Expr::sym("x");
        let f = Expr::add(Expr::pow(x, Expr::num(2)), Expr::num(1));
        assert_eq!(
            eval_at(&f, "x", &Rat::from_i64(3)).map(|r| r.to_string()),
            Some("10".into())
        );
    }

    #[test]
    fn Substitute_GivenSymbol_ReplacesEveryOccurrence() {
        let x = Expr::sym("x");
        let f = Expr::add(Expr::pow(x, Expr::num(2)), Expr::num(1));
        let replaced = substitute(&f, "x", &Expr::num(3));
        assert_eq!(replaced.to_string(), "10");
    }

    #[test]
    fn Series_GivenCosine_AlternatesSigns() {
        let x = Expr::sym("x");
        let f = Expr::Fun("cos".to_string(), vec![x]);
        let ser = series(&f, "x", 6).expect("series");
        let coeffs: Vec<String> = ser.iter().map(|(_, c)| c.to_string()).collect();
        assert_eq!(coeffs.join(" "), "1 -1/2 1/24");
    }

    #[test]
    fn Series_GivenReciprocal_ShiftsExponentsNegative() {
        // 1/x has a single term at exponent -1.
        let x = Expr::sym("x");
        let f = Expr::pow(x, Expr::num(-1));
        let ser = series(&f, "x", 4).expect("series");
        assert_eq!(ser.len(), 1);
        assert_eq!(ser[0].0, -1);
    }

    #[test]
    fn Collect_GivenMixedTerms_GroupsByPower() {
        let x = Expr::sym("x");
        let y = Expr::sym("y");
        let f = Expr::add(Expr::add(Expr::mul(x.clone(), y.clone()), x), Expr::num(2));
        assert_eq!(collect(&f, "x").to_string(), "2 + x*(y + 1)");
    }

    #[test]
    fn SplitQuotient_GivenBareReciprocal_ReturnsNumeratorOne() {
        let x = Expr::sym("x");
        let f = Expr::pow(Expr::add(x, Expr::num(1)), Expr::num(-1));
        let (num, den) = split_quotient(&f).expect("quotient");
        assert_eq!(num.to_string(), "1");
        assert_eq!(den.to_string(), "x + 1");
    }

    #[test]
    fn Rat_GivenSixthReducesToLowestTerms() {
        let r = Rat::new(Int::from_i64(2), Nat::from_u64(4));
        assert_eq!(r.to_string(), "1/2");
    }

    #[test]
    fn Print_GivenNegativeUnitCoefficient_OmitsTheOne() {
        let y = Expr::sym("y");
        assert_eq!(Expr::neg(y).to_string(), "-y");
    }

    #[test]
    fn Expand_GivenBinomialCubed_MatchesDocumentedForm() {
        let x = Expr::sym("x");
        let f = Expr::pow(Expr::add(x, Expr::num(1)), Expr::num(3));
        assert_eq!(expand(&f).to_string(), "x^3 + 3*x^2 + 3*x + 1");
    }

    #[test]
    fn Diff_GivenChainRule_ProducesProductForm() {
        let x = Expr::sym("x");
        // d/dx 2*x*cos(x^2) = 2cos(x^2) - 4x^2 sin(x^2): product rule plus chain rule.
        let f = Expr::mul(
            Expr::num(2),
            Expr::mul(
                x.clone(),
                Expr::Fun("cos".to_string(), vec![Expr::pow(x.clone(), Expr::num(2))]),
            ),
        );
        let d = diff(&f, "x");
        assert_eq!(d.to_string(), "-4*sin(x^2)*x^2 + 2*cos(x^2)");
    }

    #[test]
    fn Biquadratic_GivenQuarticWithIrrationalRoots_ReportsTwoUnrepresented() {
        // x^4 - x^2 - 1: two real roots through y = x^2, two imaginary ones.
        let coeffs = vec![
            Rat::from_i64(-1),
            Rat::zero(),
            Rat::from_i64(-1),
            Rat::zero(),
            Rat::one(),
        ];
        match solve_polynomial(&coeffs) {
            SolveOutcome::RootExpressionsCount(roots, unrepresented) => {
                assert_eq!(roots.len(), 2);
                assert_eq!(unrepresented, 2);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Sqrt_GivenPerfectSquare_IsExact() {
        assert_eq!(s(Expr::sqrt(Expr::num(4))), "2");
    }

    #[test]
    fn Rational_GivenHalf_PrintsFraction() {
        let half = Expr::Num(Rat::new(Int::from_i64(1), Nat::from_u64(2)));
        assert_eq!(s(Expr::neg(half)), "-1/2");
    }

    #[test]
    fn Latex_GivenRationalConstant_IsAFraction() {
        let half = Expr::Num(Rat::new(Int::from_i64(1), Nat::from_u64(2)));
        assert_eq!(half.latex(), "\\frac{1}{2}");
        assert_eq!(Expr::neg(half).latex(), "-\\frac{1}{2}");
        assert_eq!(Expr::num(7).latex(), "7");
    }

    #[test]
    fn Latex_GivenPowerAndProduct_UsesTheSharedGrouping() {
        let x = Expr::sym("x");
        let y = Expr::sym("y");
        let z = Expr::sym("z");
        assert_eq!(
            Expr::pow(Expr::add(x.clone(), Expr::num(1)), Expr::num(2)).latex(),
            "\\left(x + 1\\right)^{2}"
        );
        assert_eq!(
            Expr::pow(Expr::pow(x.clone(), y.clone()), Expr::num(2)).latex(),
            "\\left(x^{y}\\right)^{2}"
        );
        assert_eq!(
            Expr::div(Expr::div(x.clone(), y.clone()), z.clone()).latex(),
            "\\frac{\\frac{x}{y}}{z}"
        );
        assert_eq!(
            Expr::div(x, Expr::mul(y, z)).latex(),
            "\\frac{x}{y \\cdot z}"
        );
    }

    #[test]
    fn Latex_GivenSymbolName_EscapesTheTenSpecials() {
        assert_eq!(Expr::sym("x_1").latex(), "x\\_1");
        assert_eq!(Expr::sym("a%b").latex(), "a\\%b");
        assert_eq!(
            Expr::sym("x_1%&#'{}~^\\").latex(),
            "x\\_1\\%\\&\\#'\\{\\}\\textasciitilde{}\\textasciicircum{}\\textbackslash{}"
        );
    }

    #[test]
    fn Sqrt_GivenNegativeValue_IsTheExactImaginaryMultiple() {
        assert_eq!(s(Expr::sqrt(Expr::num(-4))), "2*i");
        assert_eq!(s(Expr::sqrt(Expr::num(-2))), "i*sqrt(2)");
        assert_eq!(
            s(Expr::sqrt(Expr::Num(Rat::new(
                Int::from_i64(-1),
                Nat::from_u64(4)
            )))),
            "1/2*i"
        );
    }

    #[test]
    fn Pow_GivenNegativeHalfInteger_IsThePrincipalImaginaryValue() {
        let three_halves = Rat::new(Int::from_i64(3), Nat::from_u64(2));
        let minus_half = Rat::new(Int::from_i64(-1), Nat::from_u64(2));
        assert_eq!(s(Expr::pow(Expr::num(-4), Expr::Num(three_halves))), "-8*i");
        assert_eq!(s(Expr::pow(Expr::num(-4), Expr::Num(minus_half))), "-1/2*i");
    }

    #[test]
    fn Pow_GivenImaginaryUnit_IntegerPowersFold() {
        let i = imaginary_unit();
        assert_eq!(s(Expr::pow(i.clone(), Expr::num(2))), "-1");
        assert_eq!(s(Expr::pow(i.clone(), Expr::num(3))), "-i");
        assert_eq!(s(Expr::mul(i.clone(), i)), "-1");
        // A user symbol spelled i is an ordinary symbol, not the named constant.
        assert_eq!(s(Expr::pow(Expr::sym("i"), Expr::num(2))), "i^2");
    }
}
