//! Exact reproduction of the seven generated corpora in
//! Lovelace.Symbolics.Tests/PropertyTests.cs.
//!
//! Each function below mirrors one C# test: the same literal seed, the same number of
//! iterations, the same recursive expression generator and -- critically -- the same order of
//! System.Random draws. The generator is [crate::dotnet_random::DotNetRandom].
//!
//! The C# tests do not assert literal corpus values; they assert properties over the corpus.
//! What is pinned here is the corpus itself: a transcript of every draw (method, arguments and
//! result) and the canonical text of every generated expression. Both are compared with values
//! produced by the .NET 10.0.3 runtime running the C# generator verbatim, so a later property
//! port is testing exactly the same inputs as the reference.

use lovelace_symbolics::{Expr, Rat};

use crate::dotnet_random::DotNetRandom;

/// A generated corpus: the expressions, their canonical text, and the draw transcript.
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    /// The generated expressions (empty for the solver corpus, which generates coefficients).
    pub expressions: Vec<Expr>,
    /// One canonical text per iteration. Expression corpora hold Expr::canonical(); the solver
    /// corpus holds "a,b,c" because the C# test builds the quadratic from three integers.
    pub items: Vec<String>,
    /// The exact draw transcript in the form "N:<v>;", "M:<max>:<v>;" and
    /// "R:<min>:<max>:<v>;", in call order.
    pub transcript: String,
}

impl Corpus {
    /// FNV-1a 64 of the draw transcript.
    pub fn transcript_fnv(&self) -> u64 {
        crate::fnv1a64(&self.transcript)
    }

    /// FNV-1a 64 of the canonical items joined with a newline.
    pub fn canonical_fnv(&self) -> u64 {
        crate::fnv1a64(&self.items.join("\n"))
    }

    /// The first canonical item, for diagnostics.
    pub fn first_item(&self) -> Option<&str> {
        self.items.first().map(String::as_str)
    }

    /// The last canonical item, for diagnostics.
    pub fn last_item(&self) -> Option<&str> {
        self.items.last().map(String::as_str)
    }

    fn push(&mut self, expression: Expr) {
        self.items.push(expression.canonical());
        self.expressions.push(expression);
    }
}

/// The C# tests drive Random only through these three calls; recording them makes the
/// transcript independent of the expression builder.
struct LoggingRandom {
    rng: DotNetRandom,
    transcript: String,
}

impl LoggingRandom {
    fn new(seed: i32) -> LoggingRandom {
        LoggingRandom {
            rng: DotNetRandom::new(seed),
            transcript: String::new(),
        }
    }

    fn next_max(&mut self, max_value: i32) -> i32 {
        let value = self.rng.next_max(max_value);
        self.transcript.push_str(&format!("M:{max_value}:{value};"));
        value
    }

    fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        let value = self.rng.next_range(min_value, max_value);
        self.transcript
            .push_str(&format!("R:{min_value}:{max_value}:{value};"));
        value
    }
}

/// Mirrors the part of the reference's Exprs.Function that the corpora can reach:
/// Constructors.Function calls Evaluation.FoldFunction (Lovelace.Symbolics/Evaluation.cs:729),
/// whose exact-rational table folds sin(0), exp(0) and abs(r) at construction time. The
/// generated functions are only sin, exp and abs over exact rationals, so those are the cases
/// this generator can hit; every other application stays a FunctionExpr, as it does in C#.
fn exact_function(name: &str, argument: Expr) -> Expr {
    if let Expr::Num(value) = &argument {
        let folded = match name {
            "exp" if value.is_zero() => Some(Rat::one()),
            "sin" if value.is_zero() => Some(Rat::zero()),
            "abs" => Some(value.abs()),
            _ => None,
        };
        if let Some(folded) = folded {
            return Expr::Num(folded);
        }
    }
    Expr::Fun(name.to_string(), vec![argument])
}

fn rational(numerator: i32, denominator: i32) -> Expr {
    let value = Rat::from_i64(i64::from(numerator))
        .div(&Rat::from_i64(i64::from(denominator)))
        .expect("the generator never draws a zero denominator");
    Expr::Num(value)
}

/// The C# RandomExpr(rng, ctx, depth, name), with the same pre-order/post-order draw order.
fn random_expr(rng: &mut LoggingRandom, depth: i32, name: &str) -> Expr {
    let x = Expr::sym(name);
    if depth <= 0 {
        return match rng.next_max(4) {
            0 => Expr::num(i64::from(rng.next_range(-5, 6))),
            1 => {
                let numerator = rng.next_range(-5, 6);
                let denominator = rng.next_range(1, 5).max(1);
                rational(numerator, denominator)
            }
            _ => x,
        };
    }

    let a = random_expr(rng, depth - 1, name);
    let b = random_expr(rng, depth - 1, name);
    match rng.next_max(7) {
        0 => Expr::add(a, b),
        1 => Expr::mul(a, b),
        2 => {
            let exponent = rng.next_range(0, 5);
            Expr::pow(a, Expr::num(i64::from(exponent)))
        }
        3 => exact_function("sin", a),
        4 => exact_function("exp", a),
        5 => exact_function("abs", a),
        _ => Expr::sub(a, b),
    }
}

fn finish(rng: LoggingRandom, corpus: Corpus) -> Corpus {
    Corpus {
        transcript: rng.transcript,
        ..corpus
    }
}

/// CanonicalText_RoundTripsAndReinterns (seed 20260909, 200 iterations).
pub fn canonical_text_roundtrip(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    for _ in 0..200 {
        let depth = rng.next_range(1, 5);
        corpus.push(random_expr(&mut rng, depth, "x"));
    }
    finish(rng, corpus)
}

/// Simplify_PreservesValues_UnderRealAssumption (seed 20260910, 150 iterations).
pub fn simplify_preserves_values(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    for _ in 0..150 {
        let depth = rng.next_range(1, 4);
        corpus.push(random_expr(&mut rng, depth, "x"));
    }
    finish(rng, corpus)
}

/// Diff_MatchesNumericalDerivative (seed 20260911, 60 iterations).
pub fn diff_matches_numerical_derivative(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    for _ in 0..60 {
        let depth = rng.next_range(1, 4);
        corpus.push(random_expr(&mut rng, depth, "x"));
    }
    finish(rng, corpus)
}

/// Integrate_DiffRoundTrips (seed 20260912, six integrands).
pub fn integrate_diff_roundtrips(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    let x = Expr::sym("x");

    corpus.push(Expr::pow(
        x.clone(),
        Expr::num(i64::from(rng.next_range(0, 9))),
    ));
    corpus.push(Expr::add(
        Expr::add(
            Expr::pow(x.clone(), Expr::num(3)),
            Expr::mul(Expr::num(2), x.clone()),
        ),
        Expr::num(1),
    ));
    corpus.push(Expr::Fun(
        "sin".to_string(),
        vec![Expr::mul(Expr::num(2), x.clone())],
    ));
    corpus.push(Expr::Fun("exp".to_string(), vec![x.clone()]));
    corpus.push(Expr::div(Expr::num(1), x.clone()));
    corpus.push(Expr::mul(
        Expr::pow(x.clone(), Expr::num(2)),
        Expr::Fun("sin".to_string(), vec![Expr::pow(x, Expr::num(3))]),
    ));

    finish(rng, corpus)
}

/// Solve_QuadraticRoots_SatisfyPolynomial (seed 20260913, 40 iterations).
///
/// The C# test draws a in [1,6), b in [-7,7) and c in [-7,7) per iteration; the expressions are
/// built from those integers, so the corpus item is the coefficient triple.
pub fn solve_quadratic_roots_satisfy_polynomial(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    for _ in 0..40 {
        let a = rng.next_range(1, 7);
        let b = rng.next_range(-7, 8);
        let c = rng.next_range(-7, 8);
        corpus.items.push(format!("{a},{b},{c}"));
    }
    finish(rng, corpus)
}

/// Optimize_PreservesValues (seed 20260914, 40 iterations).
pub fn optimize_preserves_values(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    let x = Expr::sym("x");
    for _ in 0..40 {
        let first_coefficient = rng.next_range(1, 6);
        let first_exponent = rng.next_range(1, 6);
        let second_coefficient = rng.next_range(1, 6);
        let second_exponent = rng.next_range(1, 6);
        let constant = rng.next_range(-5, 6);
        corpus.push(Expr::add(
            Expr::add(
                Expr::mul(
                    Expr::num(i64::from(first_coefficient)),
                    Expr::pow(x.clone(), Expr::num(i64::from(first_exponent))),
                ),
                Expr::mul(
                    Expr::num(i64::from(second_coefficient)),
                    Expr::pow(x.clone(), Expr::num(i64::from(second_exponent))),
                ),
            ),
            Expr::num(i64::from(constant)),
        ));
    }
    finish(rng, corpus)
}

/// MathIR_EvaluatesLikeSymbolic (seed 20260915, 60 iterations).
pub fn mathir_evaluates_like_symbolic(seed: i32) -> Corpus {
    let mut rng = LoggingRandom::new(seed);
    let mut corpus = Corpus::default();
    for _ in 0..60 {
        let depth = rng.next_range(1, 4);
        corpus.push(random_expr(&mut rng, depth, "x"));
    }
    finish(rng, corpus)
}
#[cfg(test)]
mod tests {
    use super::{
        Corpus, canonical_text_roundtrip, diff_matches_numerical_derivative,
        integrate_diff_roundtrips, mathir_evaluates_like_symbolic, optimize_preserves_values,
        simplify_preserves_values, solve_quadratic_roots_satisfy_polynomial,
    };

    /// Assert the corpus against the values the .NET 10.0.3 runtime produced while running
    /// the C# generator verbatim. The hashes cover every draw and every canonical item.
    fn assert_golden(
        corpus: &Corpus,
        items: usize,
        transcript_len: usize,
        transcript_fnv: u64,
        canonical_fnv: u64,
        first: &str,
        last: &str,
    ) {
        assert_eq!(corpus.items.len(), items, "corpus item count");
        assert_eq!(corpus.transcript.len(), transcript_len, "transcript length");
        assert_eq!(corpus.transcript_fnv(), transcript_fnv, "draw transcript");
        assert_eq!(corpus.canonical_fnv(), canonical_fnv, "canonical text");
        assert_eq!(corpus.first_item(), Some(first), "first item");
        assert_eq!(corpus.last_item(), Some(last), "last item");
    }

    #[test]
    fn canonical_text_corpus_matches_the_reference() {
        // CanonicalText_RoundTripsAndReinterns, seed 20260909, 200 iterations.
        let corpus = canonical_text_roundtrip(20_260_909);
        assert_eq!(corpus.items.len(), 200);
        assert_eq!(corpus.transcript.len(), 29_885);
        assert_eq!(corpus.transcript_fnv(), 7_322_240_257_790_247_404);
        assert_eq!(
            corpus.first_item(),
            Some("(fn sin (pow (sym x) (rat 3 1)))")
        );
        assert_eq!(corpus.last_item(), Some("(fn exp (fn exp (rat -1 1)))"));
        // All 200 canonical items are byte-identical to the C# oracle. Item 49 is the product
        // whose two commutative sum factors the round-62 report recorded as ordered the other way;
        // the canonical order now compares the canonical operand sequences, so it is the oracle's
        // order (the C# string is recorded in that report).
        assert_eq!(corpus.canonical_fnv(), 6_083_718_521_819_931_755);
        assert_eq!(
            corpus.items[49],
            "(mul (add (rat -5 1) (mul (rat -1 1) (fn sin (mul (rat 2 1) (sym x))))) (add (fn exp (fn abs (sym x))) (fn exp (add (rat -5 1) (sym x)))))"
        );
    }

    #[test]
    fn simplify_corpus_matches_the_reference() {
        // Simplify_PreservesValues_UnderRealAssumption, seed 20260910, 150 iterations.
        let corpus = simplify_preserves_values(20_260_910);
        assert_golden(
            &corpus,
            150,
            14_616,
            14_940_808_270_984_512_727,
            1_633_150_268_426_884_249,
            "(rat 0 1)",
            "(fn exp (sym x))",
        );
    }

    #[test]
    fn diff_corpus_matches_the_reference() {
        // Diff_MatchesNumericalDerivative, seed 20260911, 60 iterations.
        let corpus = diff_matches_numerical_derivative(20_260_911);
        assert_golden(
            &corpus,
            60,
            5_339,
            6_917_419_283_826_314_171,
            317_453_029_287_146_900,
            "(mul (rat -3 4) (sym x))",
            "(rat 0 1)",
        );
    }

    #[test]
    fn integrate_corpus_matches_the_reference() {
        // Integrate_DiffRoundTrips, seed 20260912, six integrands.
        let corpus = integrate_diff_roundtrips(20_260_912);
        assert_golden(
            &corpus,
            6,
            8,
            12_676_276_517_006_698_977,
            17_347_720_087_512_824_430,
            "(rat 1 1)",
            "(mul (fn sin (pow (sym x) (rat 3 1))) (pow (sym x) (rat 2 1)))",
        );
    }

    #[test]
    fn solve_corpus_matches_the_reference() {
        // Solve_QuadraticRoots_SatisfyPolynomial, seed 20260913, 40 iterations.
        let corpus = solve_quadratic_roots_satisfy_polynomial(20_260_913);
        assert_golden(
            &corpus,
            40,
            1_085,
            9_944_149_293_831_397_347,
            14_005_185_850_700_946_657,
            "4,6,2",
            "5,-2,-7",
        );
    }

    #[test]
    fn optimize_corpus_matches_the_reference() {
        // Optimize_PreservesValues, seed 20260914, 40 iterations.
        let corpus = optimize_preserves_values(20_260_914);
        assert_golden(
            &corpus,
            40,
            1_661,
            9_501_032_669_161_794_695,
            9_211_877_804_060_214_730,
            "(add (rat 5 1) (sym x) (mul (rat 2 1) (pow (sym x) (rat 3 1))))",
            "(add (rat -4 1) (mul (rat 4 1) (pow (sym x) (rat 2 1))) (mul (rat 4 1) (pow (sym x) (rat 3 1))))",
        );
    }

    #[test]
    fn mathir_corpus_matches_the_reference() {
        // MathIR_EvaluatesLikeSymbolic, seed 20260915, 60 iterations.
        let corpus = mathir_evaluates_like_symbolic(20_260_915);
        assert_eq!(corpus.items.len(), 60);
        assert_eq!(corpus.transcript.len(), 5_434);
        assert_eq!(corpus.transcript_fnv(), 9_870_694_799_073_714_841);
        assert_eq!(corpus.first_item(), Some("(fn sin (fn sin (rat -5 3)))"));
        assert_eq!(corpus.last_item(), Some("(pow (sym x) (rat 3 1))"));
        // All 60 canonical items are byte-identical to the C# oracle; item 35 is the second
        // product the round-62 report recorded with its commutative sum factors the other way.
        assert_eq!(corpus.canonical_fnv(), 6_946_605_504_391_048_442);
        assert_eq!(
            corpus.items[35],
            "(mul (add (rat -1 1) (mul (rat -4 1) (sym x))) (add (fn exp (sym x)) (mul (rat -1 1) (sym x))))"
        );
    }
}
