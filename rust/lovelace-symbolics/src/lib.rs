//! Symbolic expressions: canonical construction, term ordering, printing and differentiation.
//!
//! The rules that matter for parity are the ones a reader can see: an expression is rebuilt into
//! a canonical sum of terms, like terms are collected, and the printer reproduces the documented
//! forms exactly (`y + 2*x`, `x^2 - 2*x + 1`, `-1/2/(x + 1)`).

use std::cmp::Ordering;

use lovelace_int::Int;
use lovelace_nat::Nat;

/// An exact rational coefficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rat {
    num: Int,
    den: Nat,
}

impl Rat {
    /// Zero.
    pub fn zero() -> Rat {
        Rat { num: Int::zero(), den: Nat::from_u64(1) }
    }

    /// One.
    pub fn one() -> Rat {
        Rat { num: Int::from_i64(1), den: Nat::from_u64(1) }
    }

    /// Builds a rational from an i64.
    pub fn from_i64(v: i64) -> Rat {
        Rat { num: Int::from_i64(v), den: Nat::from_u64(1) }
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
        Rat { num: self.num.negate(), den: self.den.clone() }
    }

    /// Absolute value.
    pub fn abs(&self) -> Rat {
        if self.is_negative() { self.neg() } else { self.clone() }
    }

    /// Addition.
    pub fn add(&self, other: &Rat) -> Rat {
        let left = self.num.mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
        let right = other.num.mul(&Int::from_decimal(&self.den.to_decimal()).unwrap_or_else(Int::zero));
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
        let num = self.num.mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
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
            let base = self.pow_i64(-exp)?;
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

    /// Numeric comparison via cross-multiplication.
    pub fn cmp(&self, other: &Rat) -> Ordering {
        let left = self.num.mul(&Int::from_decimal(&other.den.to_decimal()).unwrap_or_else(Int::zero));
        let right = other.num.mul(&Int::from_decimal(&self.den.to_decimal()).unwrap_or_else(Int::zero));
        left.cmp(&right)
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

    /// Renders the coefficient as it appears in a product: `2`, `-1/2`, `3/4`.
    pub fn to_string(&self) -> String {
        let one = Nat::from_u64(1);
        if self.den.cmp(&one) == Ordering::Equal {
            return self.num.to_decimal();
        }
        format!("{}/{}", self.num.to_decimal(), self.den.to_decimal())
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
}

impl Expr {
    /// A numeric constant.
    pub fn num(v: i64) -> Expr {
        Expr::Num(Rat::from_i64(v))
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
        collected.retain(|(_, c)| !c.is_zero());
        let mut terms: Vec<Expr> = collected
            .into_iter()
            .map(|(rest, coef)| scale(rest, &coef))
            .collect();
        terms.sort_by(term_order);
        if !coefficient.is_zero() || terms.is_empty() {
            terms.push(Expr::Num(coefficient));
        }
        if terms.len() == 1 {
            return terms.pop().unwrap();
        }
        Expr::Add(terms)
    }

    /// Canonical product.
    pub fn mul(a: Expr, b: Expr) -> Expr {
        let mut factors: Vec<Expr> = Vec::new();
        for f in [a, b] {
            match f {
                Expr::Mul(inner) => factors.extend(inner),
                other => factors.push(other),
            }
        }
        let mut coefficient = Rat::one();
        let mut rest: Vec<Expr> = Vec::new();
        for f in factors {
            match f {
                Expr::Num(n) => coefficient = coefficient.mul(&n),
                other => rest.push(other),
            }
        }
        if coefficient.is_zero() {
            return Expr::num(0);
        }
        // Combine powers of the same base: x^2 * x^3 = x^5.
        let mut combined: Vec<Expr> = Vec::new();
        for factor in rest {
            let (base, exp) = as_power(factor);
            if let Some(slot) = combined.iter_mut().find(|e| as_power((*e).clone()).0 == base) {
                let (_, slot_exp) = as_power(slot.clone());
                let summed = Expr::add(slot_exp.clone(), exp.clone());
                // x * x^-1 is left alone: cancelling to 1 needs a nonzero assumption.
                if summed == Expr::num(0) {
                    combined.push(Expr::pow(base, exp));
                    continue;
                }
                *slot = Expr::pow(base.clone(), summed);
                let _ = slot_exp;
            } else {
                combined.push(Expr::pow(base, exp));
            }
        }
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
                // (x^a)^b = x^(a*b) when b is an integer.
                if let Expr::Num(e) = &exp {
                    if e.denominator().to_decimal() == "1" {
                        let combined = Expr::mul((**inner_exp).clone(), exp.clone());
                        return Expr::pow((**inner_base).clone(), combined);
                    }
                }
            }
            _ => {}
        }
        Expr::Pow(Box::new(base), Box::new(exp))
    }

    /// Negation.
    pub fn neg(e: Expr) -> Expr {
        Expr::mul(Expr::num(-1), e)
    }

    /// Subtraction.
    pub fn sub(a: Expr, b: Expr) -> Expr {
        Expr::add(a, Expr::neg(b))
    }

    /// Division.
    pub fn div(a: Expr, b: Expr) -> Expr {
        Expr::mul(a, Expr::pow(b, Expr::num(-1)))
    }

    /// A relation between two expressions.
    pub fn relation(op: RelOp, a: Expr, b: Expr) -> Expr {
        Expr::Rel(op, Box::new(a), Box::new(b))
    }

    /// A square root.
    pub fn sqrt(e: Expr) -> Expr {
        if let Expr::Num(n) = &e {
            if n.denominator().to_decimal() == "1" {
                if let Some(root) = exact_sqrt(n.numerator().magnitude()) {
                    let sign = if n.is_negative() { -1 } else { 1 };
                    return Expr::num(sign * root as i64);
                }
            }
        }
        Expr::Fun("sqrt".to_string(), vec![e])
    }

    /// True when the expression contains the given symbol.
    pub fn contains(&self, name: &str) -> bool {
        match self {
            Expr::Num(_) => false,
            Expr::Sym(s) => s == name,
            Expr::Add(v) | Expr::Mul(v) => v.iter().any(|e| e.contains(name)),
            Expr::Pow(a, b) => a.contains(name) || b.contains(name),
            Expr::Fun(_, args) => args.iter().any(|e| e.contains(name)),
            Expr::Div(a, b) => a.contains(name) || b.contains(name),
            Expr::Rel(_, a, b) => a.contains(name) || b.contains(name),
        }
    }

    /// True when the expression contains no symbols.
    pub fn is_constant(&self) -> bool {
        match self {
            Expr::Num(_) => true,
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

    /// Total degree in the named symbol (0 when absent).
    pub fn degree_in(&self, name: &str) -> i64 {
        match self {
            Expr::Num(_) => 0,
            Expr::Sym(s) => i64::from(s == name),
            Expr::Add(v) => v.iter().map(|e| e.degree_in(name)).max().unwrap_or(0),
            Expr::Mul(v) => v.iter().map(|e| e.degree_in(name)).sum(),
            Expr::Pow(b, e) => match (b.as_ref(), e.as_ref()) {
                (inner, Expr::Num(n)) if n.denominator().to_decimal() == "1" => {
                    inner.degree_in(name) * n.numerator().to_decimal().parse::<i64>().unwrap_or(1)
                }
                (inner, _) => inner.degree_in(name),
            },
            Expr::Fun(_, args) => args.iter().map(|e| e.degree_in(name)).max().unwrap_or(0),
            Expr::Div(a, b) => a.degree_in(name) - b.degree_in(name),
            Expr::Rel(_, a, b) => a.degree_in(name).max(b.degree_in(name)),
        }
    }

    /// Renders the expression in the documented form.
    pub fn to_string(&self) -> String {
        match self {
            Expr::Num(n) => n.to_string(),
            Expr::Sym(s) => s.clone(),
            Expr::Add(terms) => {
                let mut out = String::new();
                for (i, t) in terms.iter().enumerate() {
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
                if let Some(Expr::Num(n)) = factors.first() {
                    if n.is_negative() {
                        let mut rest = factors.clone();
                        rest[0] = Expr::Num(n.abs());
                        let inner = if rest.len() == 1 { rest[0].clone() } else { Expr::Mul(rest) };
                        return format!("-{}", product_text(&inner));
                    }
                }
                product_text(self)
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
                    return format!("not {}", args[0].to_string());
                }
                let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                format!("{}({})", name, parts.join(", "))
            }
            Expr::Div(a, b) => {
                let top = match a.as_ref() {
                    Expr::Add(_) | Expr::Mul(_) => format!("({})", a.to_string()),
                    other => other.to_string(),
                };
                let bottom = match b.as_ref() {
                    Expr::Add(_) | Expr::Mul(_) => format!("({})", b.to_string()),
                    other => other.to_string(),
                };
                format!("{}/{}", top, bottom)
            }
            Expr::Rel(op, a, b) => format!("{} {} {}", a.to_string(), op.text(), b.to_string()),
        }
    }
}

fn signed_text(e: &Expr) -> (String, bool) {
    match e {
        Expr::Num(n) if n.is_negative() => (n.abs().to_string(), true),
        Expr::Mul(factors) => {
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
                    return (inner.to_string(), true);
                }
            }
            (e.to_string(), false)
        }
        _ => (e.to_string(), false),
    }
}

fn product_text(e: &Expr) -> String {
    let factors = match e {
        Expr::Mul(f) => f.clone(),
        other => vec![other.clone()],
    };
    let mut numerator: Vec<String> = Vec::new();
    let mut denominator: Vec<String> = Vec::new();
    for f in &factors {
        match f {
            Expr::Pow(base, exp) => match exp.as_ref() {
                Expr::Num(n) if n.is_negative() => {
                    let positive = Expr::pow((**base).clone(), Expr::Num(n.abs()));
                    let needs_parens = matches!(positive, Expr::Mul(_) | Expr::Add(_));
                    let text = positive.to_string();
                    denominator.push(if needs_parens { format!("({})", text) } else { text });
                }
                _ => numerator.push(f.to_string()),
            },
            Expr::Num(n) if n.is_one() && factors.len() > 1 => {}
            Expr::Add(_) => numerator.push(format!("({})", f.to_string())),
            other => numerator.push(other.to_string()),
        }
    }
    let top = if numerator.is_empty() { "1".to_string() } else { numerator.join("*") };
    if denominator.is_empty() {
        return top;
    }
    let bottom = if denominator.len() == 1 {
        denominator[0].clone()
    } else {
        format!("({})", denominator.join("*"))
    };
    format!("{}/{}", top, bottom)
}

fn power_text(e: &Expr) -> String {
    match e {
        Expr::Pow(base, exp) => {
            let base_text = match base.as_ref() {
                Expr::Add(_) | Expr::Mul(_) => format!("({})", base.to_string()),
                other => other.to_string(),
            };
            let exp_text = match exp.as_ref() {
                Expr::Num(n) if n.is_negative() => format!("({})", n.to_string()),
                Expr::Add(_) | Expr::Mul(_) => format!("({})", exp.to_string()),
                other => other.to_string(),
            };
            format!("{}^{}", base_text, exp_text)
        }
        other => other.to_string(),
    }
}

fn as_power(e: Expr) -> (Expr, Expr) {
    match e {
        Expr::Pow(b, x) => (*b, *x),
        other => (other, Expr::num(1)),
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
            let expr = if rest.len() == 1 { rest.pop().unwrap() } else { Expr::Mul(rest) };
            (Some(expr), coef)
        }
        other => (Some(other), Rat::one()),
    }
}

fn scale(e: Expr, coef: &Rat) -> Expr {
    if coef.is_one() {
        return e;
    }
    Expr::Mul(vec![Expr::Num(coef.clone()), e])
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
        Expr::Num(_) => 0,
        Expr::Sym(_) => 1,
        Expr::Fun(_, _) => 2,
        Expr::Pow(_, _) => 3,
        Expr::Mul(_) => 4,
        Expr::Add(_) => 5,
        Expr::Div(_, _) => 3,
        Expr::Rel(_, _, _) => 6,
    }
}

impl Expr {
    fn degree_in_total(&self) -> i64 {
        match self {
            Expr::Num(_) => 0,
            Expr::Sym(_) => 1,
            Expr::Add(v) => v.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            // A product is as deep as its deepest factor: x*exp(x) ranks with exp(x), which is
            // what the documented ordering shows.
            Expr::Mul(v) => v.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            Expr::Pow(b, e) => match e.as_ref() {
                Expr::Num(n) if n.denominator().to_decimal() == "1" => {
                    b.degree_in_total() * n.numerator().to_decimal().parse::<i64>().unwrap_or(1)
                }
                _ => b.degree_in_total(),
            },
            Expr::Fun(_, args) => args.iter().map(|e| e.degree_in_total()).max().unwrap_or(0),
            Expr::Div(a, b) => a.degree_in_total() - b.degree_in_total(),
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

fn exact_sqrt(n: &Nat) -> Option<u64> {
    let v = n.to_u64()?;
    let mut r = 0u64;
    while r * r <= v {
        if r * r == v {
            return Some(r);
        }
        r += 1;
        if r > 1_000_000 {
            return None;
        }
    }
    None
}

impl From<Rat> for Expr {
    fn from(r: Rat) -> Expr {
        Expr::Num(r)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&Expr::to_string(self))
    }
}

/// A truncated series: exponents (which may be negative) mapped to rational coefficients.
pub type Series = Vec<(i64, Rat)>;

fn series_add(a: &Series, b: &Series) -> Series {
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
    if a.is_empty() {
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
    match e {
        Expr::Sym(s) if s == var => replacement.clone(),
        Expr::Sym(_) | Expr::Num(_) => e.clone(),
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
        Expr::Pow(a, b) => Expr::pow(substitute(a, var, replacement), substitute(b, var, replacement)),
        Expr::Fun(n, args) => {
            Expr::Fun(n.clone(), args.iter().map(|a| substitute(a, var, replacement)).collect())
        }
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

pub fn apart(e: &Expr, var: &str) -> Option<Expr> {
    let (num_e, den_e) = split_quotient(e)?;
    let nc = poly_coeffs(&num_e, var)?;
    let dc = poly_coeffs(&den_e, var)?;
    if dc.len() < 2 {
        return None;
    }
    let roots = match solve_polynomial(&dc) {
        SolveOutcome::Roots(r) => r,
        _ => return None,
    };
    if roots.len() + 1 != dc.len() {
        return None;
    }
    let deriv: Vec<Rat> = (1..dc.len())
        .map(|i| dc[i].mul(&Rat::from_i64(i as i64)))
        .collect();
    let mut sorted = roots;
    sorted.sort_by(|a, b| a.cmp(b));
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

pub fn eval_at(e: &Expr, var: &str, x: &Rat) -> Option<Rat> {
    let coeffs = poly_coeffs(e, var)?;
    let mut acc = Rat::zero();
    for c in coeffs.iter().rev() {
        acc = acc.mul(x).add(c);
    }
    Some(acc)
}

pub fn poly_div(num: &[Rat], den: &[Rat]) -> Option<Vec<Rat>> {
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
    let mut var_name: Option<String> = None;
    let mut symbols: Vec<String> = Vec::new();
    collect_all(&e.clone(), &mut symbols);
    if symbols.len() != 1 {
        return None;
    }
    var_name = symbols.first().cloned();
    let var = var_name?;
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
            Expr::mul(Expr::Num(c.clone()), Expr::pow(Expr::sym(var), Expr::num(i as i64)))
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
    match e {
        Expr::Num(_) | Expr::Sym(_) => e.clone(),
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
                        for _ in 0..k {
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
    let expanded = expand(e);
    let mut coefficients: Vec<(i64, Expr)> = Vec::new();
    let terms = match &expanded {
        Expr::Add(ts) => ts.clone(),
        single => vec![single.clone()],
    };
    for term in terms {
        let degree = term.degree_in(var);
        let coefficient = divide_out(&term, var, degree);
        if let Some(slot) = coefficients.iter_mut().find(|(d, _)| *d == degree) {
            slot.1 = Expr::add(slot.1.clone(), coefficient);
        } else {
            coefficients.push((degree, coefficient));
        }
    }
    coefficients.retain(|(_, c)| *c != Expr::num(0));
    coefficients.sort_by(|a, b| b.0.cmp(&a.0));
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
                let d = f.degree_in(var);
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
        Expr::Pow(base, exp) if base.as_ref() == &Expr::sym(var) => {
            match exp.as_ref() {
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
            }
        }
        other => {
            let _ = other;
            Expr::Fun("__opaque".to_string(), Vec::new())
        }
    }
}

/// Extracts polynomial coefficients (index = degree) when the expression is a polynomial in
/// `var` with rational coefficients.
pub fn poly_coeffs(e: &Expr, var: &str) -> Option<Vec<Rat>> {
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
        let coefficient = divide_out(&term, var, degree);
        let value = match coefficient {
            Expr::Num(n) => n,
            _ => return None,
        };
        let index = degree as usize;
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

/// The outcome of solving a polynomial equation.
#[derive(Clone, Debug)]
pub enum SolveOutcome {
    /// Exact rational roots, ascending.
    Roots(Vec<Rat>),
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

/// Solves a polynomial equation with rational coefficients by exact root extraction.
pub fn solve_polynomial(coeffs: &[Rat]) -> SolveOutcome {
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
    let mut roots: Vec<Rat> = Vec::new();
    // Find rational roots until the polynomial is linear or has none.
    loop {
        if working.len() <= 2 {
            break;
        }
        match find_rational_root(&working) {
            Some(root) => {
                working = divide_by_root(&working, &root);
                if !roots.iter().any(|r| r.cmp(&root) == Ordering::Equal) {
                    roots.push(root);
                }
            }
            None => {
                // A quadratic remainder can still be solved exactly when its discriminant is a
                // perfect square.
                if working.len() == 3 {
                    if let Some((a, b)) = quadratic_roots(&working) {
                        for r in [a, b] {
                            if !roots.iter().any(|x| x.cmp(&r) == Ordering::Equal) {
                                roots.push(r);
                            }
                        }
                        working = vec![Rat::one()];
                    } else if let Some(complex) = quadratic_complex_roots(&working) {
                        // Complex roots are published first, then the exact ones already found.
                        let mut exprs = complex;
                        roots.sort_by(|a, b| a.cmp(b));
                        exprs.extend(roots.iter().map(expr_from_rat));
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
            if !roots.iter().any(|r| r.cmp(&root) == Ordering::Equal) {
                roots.push(root);
            }
        }
    }
    roots.sort_by(|a, b| a.cmp(b));
    if roots.len() < coeffs.len() - 1 {
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
        return SolveOutcome::Partial(roots);
    }
    if roots.is_empty() {
        if let Some(outcome) = solve_biquadratic(coeffs) {
            return outcome;
        }
        return SolveOutcome::Unevaluated;
    }
    SolveOutcome::Roots(roots)
}

fn find_rational_root(coeffs: &[Rat]) -> Option<Rat> {
    let degree = coeffs.len() - 1;
    // Scale to integers so the rational root theorem applies.
    let mut scale = 1u64;
    for c in coeffs {
        if let Some(d) = c.denominator().to_u64() {
            scale = lcm(scale, d.max(1));
        }
    }
    let ints: Vec<i64> = coeffs
        .iter()
        .map(|c| {
            let n = c.numerator().to_decimal().parse::<i64>().unwrap_or(0);
            let d = c.denominator().to_decimal().parse::<i64>().unwrap_or(1);
            n * (scale as i64) / d
        })
        .collect();
    let constant = ints[0];
    let leading = ints[degree];
    if leading == 0 {
        return None;
    }
    for p in divisors(constant.abs()) {
        for q in divisors(leading.abs()) {
            for sign in [1i64, -1i64] {
                let candidate = Rat::new(
                    Int::from_i64(sign * p),
                    Nat::from_u64(q as u64),
                );
                if eval_poly(coeffs, &candidate).map(|v| v.is_zero()).unwrap_or(false) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn divisors(n: i64) -> Vec<i64> {
    if n == 0 {
        return vec![1];
    }
    let mut out = Vec::new();
    let mut d = 1i64;
    while d * d <= n && d < 100_000 {
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

fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 1;
    }
    (a / gcd_u64(a, b)) * b
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
    let root_expr = match (perfect_square(mag.numerator().magnitude()), perfect_square(mag.denominator())) {
        (Some(n), Some(d)) if d == 1 => Expr::num(n as i64),
        _ => Expr::sqrt(Expr::Num(mag.clone())),
    };
    let mag = disc.neg();
    let root_expr = match (
        perfect_square(mag.numerator().magnitude()),
        perfect_square(mag.denominator()),
    ) {
        (Some(n), Some(d)) if d == 1 => Expr::num(n as i64),
        _ => Expr::sqrt(Expr::Num(mag.clone())),
    };
    let i_root = Expr::mul(Expr::sym("i"), root_expr);
    let two_a = a.mul(&Rat::from_i64(2));
    // Roots are (i*sqrt(|d|) - b) / (2a) and (-i*sqrt(|d|) - b) / (2a).
    let factor = Rat::one().div(&two_a)?;
    let minus_b: Expr = Expr::Num(b.neg());
    let plus = Expr::mul(
        Expr::Num(factor.clone()),
        Expr::add(i_root.clone(), minus_b.clone()),
    );
    let minus = Expr::mul(
        Expr::Num(factor),
        Expr::add(Expr::neg(i_root), minus_b),
    );
    Some(vec![plus, minus])
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

fn solve_biquadratic(coeffs: &[Rat]) -> Option<SolveOutcome> {
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
        (Some(n), Some(d)) if d == 1 => Expr::num(n as i64),
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
            if denominator == 0.0 { 0.0 } else { approximate(a) / denominator }
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
    match e {
        Expr::Num(_) => Expr::num(0),
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
        let e = Expr::add(Expr::mul(Expr::num(2), x.clone()), Expr::mul(Expr::num(3), x));
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
        let rendered: Vec<String> = ser
            .iter()
            .map(|(e, c)| format!("{}*x^{}", c.to_string(), e))
            .collect();
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
                assert_eq!(r[0].to_string(), "2");
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
                let rendered: Vec<String> = r.iter().map(|x| x.to_string()).collect();
                assert_eq!(rendered.join(" "), "-2 -1 1 2");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn Solve_GivenNonzeroConstant_IsNoSolutions() {
        let coeffs = vec![Rat::from_i64(1)];
        assert!(matches!(solve_polynomial(&coeffs), SolveOutcome::NoSolutions));
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
        assert_eq!(eval_at(&f, "x", &Rat::from_i64(3)).map(|r| r.to_string()), Some("10".into()));
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
}
