//! Kink-aware truncated power series (round 38).
//!
//! The published `series(f, x, x0, n)` contract is SymPy 1.14.0's RIGHT-HAND expansion. At a KINK
//! — a point where the function has no derivative, i.e. an `abs(g)` whose argument vanishes there —
//! the expansion has to be taken from one side, and SymPy resolves it before differentiating:
//! `Abs._eval_nseries` multiplies the argument's own series by `sign(arg.leadterm(x)[0])`, the sign
//! of the argument's leading COEFFICIENT.
//!
//! This module reproduces that rule on the Rust kernel and publishes the expansion with the
//! reference's `order` node, `(order var point degree)` — the node the canonical form carries and
//! the `O(...)` term the display carries. The rational-coefficient engine in `crate::series` is
//! still the one that expands the smooth shapes (`sin`, `cos`, `exp`, quotients); this module adds
//! the kink rewrite, symbolic coefficients (`|a*x|` is `a*x*sign(a)`) and the published form.

use crate::{Expr, substitute};

/// The series order node: `(order var point degree)`, the reference's `OrderExpr`.
pub fn order_term(var: &str, point: &Expr, degree: i64) -> Expr {
    Expr::Fun(
        "order".to_string(),
        vec![Expr::sym(var), point.clone(), Expr::num(degree)],
    )
}

/// True when the expression is the series order node.
pub fn is_order_term(e: &Expr) -> bool {
    matches!(e, Expr::Fun(name, args) if name == "order" && args.len() == 3)
}

/// The (variable, point, degree) of an order node.
pub fn order_parts(e: &Expr) -> Option<(&Expr, &Expr, i64)> {
    match e {
        Expr::Fun(name, args) if name == "order" && args.len() == 3 => {
            let Expr::Num(degree) = &args[2] else {
                return None;
            };
            Some((&args[0], &args[1], degree.to_i64()?))
        }
        _ => None,
    }
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Num(n) if n.is_zero())
}

fn is_zero_point(point: &Expr) -> bool {
    is_zero(point)
}

/// The sign of a numeric constant, or None when the expression is not one.
fn sign_of(e: &Expr) -> Option<i32> {
    match e {
        Expr::Num(n) if n.is_zero() => Some(0),
        Expr::Num(n) if n.is_negative() => Some(-1),
        Expr::Num(_) => Some(1),
        _ => None,
    }
}

/// Adds two coefficient vectors, truncating at `limit` coefficients.
fn add_vec(a: &[Expr], b: &[Expr], limit: usize) -> Vec<Expr> {
    let n = a.len().max(b.len()).min(limit.max(1));
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let left = a.get(k).cloned().unwrap_or_else(|| Expr::num(0));
        let right = b.get(k).cloned().unwrap_or_else(|| Expr::num(0));
        out.push(Expr::add(left, right));
    }
    out
}

/// Multiplies two coefficient vectors, truncating at `limit` coefficients.
fn mul_vec(a: &[Expr], b: &[Expr], limit: usize) -> Vec<Expr> {
    let n = (a.len() + b.len()).saturating_sub(1).min(limit.max(1));
    let mut out = vec![Expr::num(0); n];
    for (i, ca) in a.iter().enumerate() {
        if i >= n {
            break;
        }
        for (j, cb) in b.iter().enumerate() {
            if i + j >= n {
                break;
            }
            out[i + j] = Expr::add(out[i + j].clone(), Expr::mul(ca.clone(), cb.clone()));
        }
    }
    out
}

/// The polynomial coefficients of `e` in `var` (index = degree), SYMBOLIC coefficients allowed.
///
/// This is the shape the kink rewrite needs: after `|a*x|` is resolved to `sign(a)*a*x` the
/// coefficients carry a function of `a`, which the rational engine cannot represent. None when the
/// expression is not a polynomial in `var`.
fn poly_coefficients(e: &Expr, var: &str, limit: usize) -> Option<Vec<Expr>> {
    // A subexpression that does not mention the expansion variable is a CONSTANT coefficient,
    // whatever its shape: sign(a) in |a*x|, exp(b), a quotient of two other symbols.
    if !e.contains(var) {
        return Some(vec![e.clone()]);
    }
    match e {
        Expr::Num(n) => Some(vec![Expr::Num(n.clone())]),
        Expr::Real(text) => Some(vec![Expr::Real(text.clone())]),
        Expr::Sym(name) if name == var => Some(vec![Expr::num(0), Expr::num(1)]),
        Expr::Sym(_) => Some(vec![e.clone()]),
        Expr::Add(terms) => {
            let mut acc = vec![Expr::num(0)];
            for term in terms {
                acc = add_vec(&acc, &poly_coefficients(term, var, limit)?, limit);
            }
            Some(acc)
        }
        Expr::Mul(factors) => {
            let mut acc = vec![Expr::num(1)];
            for factor in factors {
                acc = mul_vec(&acc, &poly_coefficients(factor, var, limit)?, limit);
            }
            Some(acc)
        }
        Expr::Pow(base, exp) => {
            let Expr::Num(n) = exp.as_ref() else {
                return None;
            };
            if !n.is_integer() || n.is_negative() {
                return None;
            }
            let copies = n.to_i64()?;
            if copies > 64 {
                return None;
            }
            let base = poly_coefficients(base, var, limit)?;
            let mut acc = vec![Expr::num(1)];
            for _ in 0..copies {
                acc = mul_vec(&acc, &base, limit);
            }
            Some(acc)
        }
        Expr::Div(..) | Expr::Fun(..) | Expr::Rel(..) => None,
    }
}

/// The Maclaurin coefficients of `e` around zero, exactly `order` of them.
///
/// Two engines, in the reference's order: the polynomial view (which carries symbolic coefficients)
/// and the rational power-series engine (which carries `exp`/`sin`/`cos` and quotients).
fn maclaurin(e: &Expr, var: &str, order: i64) -> Option<Vec<Expr>> {
    let count = order.max(1) as usize;
    if let Some(poly) = poly_coefficients(e, var, count) {
        let mut out = vec![Expr::num(0); count];
        for (k, coefficient) in poly.into_iter().enumerate() {
            if k < count {
                out[k] = coefficient;
            }
        }
        return Some(out);
    }
    let rational = crate::series(e, var, order)?;
    let mut out = vec![Expr::num(0); count];
    for (exponent, coefficient) in rational {
        if exponent >= 0 {
            let k = exponent as usize;
            if k < count {
                out[k] = Expr::Num(coefficient);
            }
        }
    }
    Some(out)
}

/// The leading (first nonzero) coefficient of `g`'s own series at the point.
///
/// `None` means the series cannot be computed here (the kink stays unresolved); `Some(None)` means
/// the series is zero to the requested order, so `|g|` is zero there (the reference returns zero).
fn leading_coefficient(g: &Expr, var: &str, order: i64) -> Option<Option<Expr>> {
    let coefficients = maclaurin(g, var, order.max(2))?;
    Some(coefficients.into_iter().find(|c| !is_zero(c)))
}

/// Rewrites every kink the expansion crosses: an `abs(g)` becomes the branch the RIGHT-HAND
/// expansion follows, so no derivative is ever taken OF the kink. That is SymPy's rule — the
/// argument's series times the sign of its leading coefficient — and it is why
/// `series(abs(x), x, 0, 3)` is `x + O(x^3)`, never a refusal and never the value at the point.
fn resolve_kinks(e: &Expr, var: &str, order: i64) -> Expr {
    match e {
        Expr::Fun(name, args) if name == "abs" && args.len() == 1 => {
            let g = resolve_kinks(&args[0], var, order);
            match leading_coefficient(&g, var, order) {
                // the shape is past what this expansion can see; leaving it is never a wrong value
                None => Expr::Fun("abs".to_string(), vec![g]),
                Some(None) => Expr::num(0),
                Some(Some(coefficient)) => match sign_of(&coefficient) {
                    Some(sign) if sign > 0 => g,
                    Some(sign) if sign < 0 => Expr::neg(g),
                    Some(_) => Expr::num(0),
                    // |a*x| at 0: which side the expansion follows depends on sign(a), so the
                    // branch is COMPUTED — sign(a)*a*x, the branch SymPy takes — never guessed.
                    None => Expr::mul(Expr::Fun("sign".to_string(), vec![coefficient]), g),
                },
            }
        }
        Expr::Add(terms) => {
            let mut acc = Expr::num(0);
            for term in terms {
                acc = Expr::add(acc, resolve_kinks(term, var, order));
            }
            acc
        }
        Expr::Mul(factors) => {
            let mut acc = Expr::num(1);
            for factor in factors {
                acc = Expr::mul(acc, resolve_kinks(factor, var, order));
            }
            acc
        }
        Expr::Pow(base, exp) => Expr::pow(
            resolve_kinks(base, var, order),
            resolve_kinks(exp, var, order),
        ),
        Expr::Fun(name, args) => Expr::Fun(
            name.clone(),
            args.iter().map(|a| resolve_kinks(a, var, order)).collect(),
        ),
        Expr::Div(a, b) => Expr::div(resolve_kinks(a, var, order), resolve_kinks(b, var, order)),
        Expr::Rel(op, a, b) => Expr::relation(
            *op,
            resolve_kinks(a, var, order),
            resolve_kinks(b, var, order),
        ),
        other => other.clone(),
    }
}

/// Publishes `series(e, var, point, order)`: the kink-resolved expansion with its `O(...)` tail.
///
/// When no power series exists at the point (a branch point: `sqrt(x)` at zero differentiates to
/// `1/(2*sqrt(0))`) the SOURCE function is published unchanged — what SymPy publishes for those
/// inputs and the same answer the previous engine gave.
pub fn publish_series(e: &Expr, var: &str, point: &Expr, order: i64) -> Expr {
    if order < 1 {
        return e.clone();
    }
    let shifted = if is_zero_point(point) {
        e.clone()
    } else {
        substitute(e, var, &Expr::add(Expr::sym(var), point.clone()))
    };
    let resolved = resolve_kinks(&shifted, var, order);
    let Some(coefficients) = maclaurin(&resolved, var, order) else {
        return e.clone();
    };
    let dx = if is_zero_point(point) {
        Expr::sym(var)
    } else {
        Expr::sub(Expr::sym(var), point.clone())
    };
    let mut acc = Expr::num(0);
    for (k, coefficient) in coefficients.iter().enumerate() {
        if is_zero(coefficient) {
            continue;
        }
        let term = if k == 0 {
            coefficient.clone()
        } else {
            Expr::mul(
                coefficient.clone(),
                Expr::pow(dx.clone(), Expr::num(k as i64)),
            )
        };
        acc = Expr::add(acc, term);
    }
    if is_zero(&acc) {
        return Expr::num(0);
    }
    let mut terms = match acc {
        Expr::Add(terms) => terms,
        single => vec![single],
    };
    terms.push(order_term(var, point, order));
    Expr::Add(terms)
}

/// The degree of a series term around `offset`, or None when the term is not a monomial there.
pub fn series_term_degree(term: &Expr, offset: &Expr) -> Option<i64> {
    match term {
        Expr::Num(_) | Expr::Real(_) => Some(0),
        Expr::Fun(name, args) if name == "order" && args.len() == 3 => Some(i64::MAX),
        _ if term == offset => Some(1),
        Expr::Pow(base, exp) => match exp.as_ref() {
            Expr::Num(n) if base.as_ref() == offset && n.is_integer() => n.to_i64(),
            _ => None,
        },
        Expr::Mul(factors) => {
            let mut total: i64 = 0;
            let mut any = false;
            for factor in factors {
                match series_term_degree(factor, offset)? {
                    0 => {}
                    degree => {
                        total = total.checked_add(degree)?;
                        any = true;
                    }
                }
            }
            Some(if any { total } else { 0 })
        }
        _ => None,
    }
}

/// The reference's display order for a sum that carries an `O(...)` term: ascending powers around
/// the expansion point, the O-term always last (`Printing.OrderTermsForDisplay`).
pub fn series_display_order(terms: &[Expr]) -> Vec<Expr> {
    if terms.is_empty() || terms.len() > 64 {
        return terms.to_vec();
    }
    let Some((var, point, _)) = terms.iter().find_map(order_parts) else {
        return terms.to_vec();
    };
    let offset = if is_zero_point(point) {
        var.clone()
    } else {
        Expr::sub(var.clone(), point.clone())
    };
    let mut ordered = terms.to_vec();
    ordered.sort_by(|a, b| compare_series_terms(a, b, &offset));
    ordered
}

fn compare_series_terms(a: &Expr, b: &Expr, offset: &Expr) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (a_order, b_order) = (is_order_term(a), is_order_term(b));
    if a_order || b_order {
        return if a_order == b_order {
            Ordering::Equal
        } else if a_order {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }
    match (series_term_degree(a, offset), series_term_degree(b, offset)) {
        (Some(da), Some(db)) if da != db => da.cmp(&db),
        _ => crate::term_order(a, b),
    }
}
