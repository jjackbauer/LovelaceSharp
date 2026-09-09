# Lovelace.Symbolics — Usage Guide

> The symbolic-numeric kernel of LovelaceSharp: immutable, hash-consed expressions,
> assumption-aware canonicalization, algebra, calculus, solving, symbolic matrices,
> optimization, and a typed computational IR (MathIR) that runs on the exact
> arbitrary-precision numerics — all Native AOT-safe.

**Every example in this document is machine-verified.** The language examples are doctests
(`Lovelace.Symbolics.Tests/UsageDocumentationTests.cs` asserts each `lovelace`/`result` pair
exactly); every C# snippet is compiled and run by `Lovelace.Symbolics.Tests/UsageExamples.cs`,
and `DocsSyncTests.cs` fails the build if any C# snippet here drifts from that file.

---

## 1. Quick start

In the language (REPL, Studio, or the DSH `lovelace` tool), the symbolic builtins are
available because the hosts load `SymbolicsPlugin`. Create symbols, build expressions with the
ordinary operators, and apply operations:

```lovelace
x = symbol("x")
f = x^3 + 2*x^2 + 5*x + 7
diff(f, x)
```
```result
5 + 3*x^2 + 4*x (Symbolic)
```

Symbolic values are a **domain type** (like `Complex`): they do not widen to numeric kinds,
and exactness is preserved until you explicitly evaluate with `subs`/`evalf`.

## 2. Symbols, expressions, and canonical algebra

Expressions are built from symbols with `+ - * / ^` and the elementary functions.
Construction is **canonical**: numeric coefficients are collected, like terms merged, sums and
products sorted by a single deterministic term order, and division represented as negative
powers. Identities that hold unconditionally fold immediately; everything else stays
unevaluated (correctness before cleverness).

```lovelace
x = symbol("x")
x + 0
```
```result
x (Symbolic)
```

```lovelace
x = symbol("x")
x*0
```
```result
0 (Symbolic)
```

```lovelace
x = symbol("x")
x*2 + 3*x
```
```result
5*x (Symbolic)
```

```lovelace
x = symbol("x")
x^2*x^3
```
```result
x^5 (Symbolic)
```

```lovelace
x = symbol("x")
y = symbol("y")
(x+y)+x
```
```result
y + 2*x (Symbolic)
```

```lovelace
x = symbol("x")
x - x
```
```result
0 (Symbolic)
```

```lovelace
x = symbol("x")
x/x
```
```result
x/x (Symbolic)
```

Canonical construction preserves definedness: `x/x` is undefined at 0 while `1` is not,
so the kernel keeps the pole visible. `simplify(x/x)` cancels it and reports the side
condition `x != 0`:

```lovelace
x = symbol("x")
simplify(x/x)
```
```result
1 (Symbolic)
```

```lovelace
x = symbol("x")
x^2 - 2*x + 1
```
```result
1 + x^2 - 2*x (Symbolic)
```

```lovelace
x = symbol("x")
2^x
```
```result
2^x (Symbolic)
```

Numeric literals without symbols keep the existing numeric behavior — nothing changes for
existing scripts:

```lovelace
1/2 + 1/3
```
```result
0.8(3) (Real)
```

Comparisons on symbolic operands produce **symbolic relations** instead of booleans:

```lovelace
x = symbol("x")
x > 0
```
```result
x > 0 (Symbolic)
```


## 3. Assumptions

Assumptions are semantics, not annotations. They scope the transformations that are
licensed, and queries are tri-state: **True** (provable), **False** (provably negated), or
**Unknown** (never silently assumed).

```lovelace
x = symbol("x")
assume(x > 5)
```
```result
x > 5 (Symbolic)
```

```lovelace
x = symbol("x")
assume(x > 5)
assumptions()
```
```result
x > 5
```

Domain and property shortcuts: `assume_positive`, `assume_nonnegative`, `assume_negative`,
`assume_real`, `assume_integer`.

```lovelace
x = symbol("x")
assume_positive(x)
simplify(sqrt(x^2))
```
```result
x (Symbolic)
```

Without the assumption the same expression is **left unevaluated** — the safe behavior, since
`sqrt(x^2) = |x|` for reals and `x` only for nonnegative `x`:

```lovelace
y = symbol("y")
simplify(sqrt(y^2))
```
```result
(y^2)^(1/2) (Symbolic)
```

```lovelace
simplify(sqrt(4))
```
```result
2 (Symbolic)
```


## 4. Simplification

`simplify` runs budgeted, assumption-aware rewrite groups. The v1 groups cover the
Pythagorean identity, power rules, and exp/log inverses.

```lovelace
x = symbol("x")
simplify(sin(x)^2 + cos(x)^2)
```
```result
1 (Symbolic)
```

```lovelace
x = symbol("x")
simplify(exp(log(x)))
```
```result
x (Symbolic)
```


## 5. Algebra: expand, collect, factor, cancel, apart

```lovelace
x = symbol("x")
expand((x+1)^3)
```
```result
1 + x^3 + 3*x + 3*x^2 (Symbolic)
```

```lovelace
x = symbol("x")
y = symbol("y")
collect(x*y + x + 2, x)
```
```result
2 + x*(1 + y) (Symbolic)
```

`factor` works over Q[x] (square-free decomposition + rational-root linear factors).

```lovelace
x = symbol("x")
factor(x^4 - 5*x^2 + 4)
```
```result
(-2 + x)*(-1 + x)*(1 + x)*(2 + x) (Symbolic)
```

```lovelace
x = symbol("x")
cancel((x^2 - 1)/(x - 1))
```
```result
1 + x (Symbolic)
```

```lovelace
x = symbol("x")
apart(1/(x^2 - 1), x)
```
```result
-1/2/((1 + x)) + 1/2/((-1 + x)) (Symbolic)
```

Non-polynomial input passes through unchanged (honesty: nothing is forced):

```lovelace
x = symbol("x")
factor(sin(x))
```
```result
sin(x) (Symbolic)
```


## 6. Differentiation

```lovelace
x = symbol("x")
diff(x*exp(x), x)
```
```result
exp(x) + x*exp(x) (Symbolic)
```

```lovelace
x = symbol("x")
diff(diff(x^4, x), x)
```
```result
12*x^2 (Symbolic)
```

```lovelace
x = symbol("x")
diff(cos(x^2), x)
```
```result
-2*x*sin(x^2) (Symbolic)
```

```lovelace
x = symbol("x")
diff(x^2, 3)
```
```result
error: Expected a symbol.
```


## 7. Vector calculus: jacobian, hessian

```lovelace
x = symbol("x")
y = symbol("y")
jacobian([x*y, x+y], [x, y])
```
```result
[y, x, 1, 1] (Vector)
```

```lovelace
x = symbol("x")
y = symbol("y")
hessian(x*y, [x, y])
```
```result
[0, 1, 1, 0] (Vector)
```


## 8. Series

```lovelace
x = symbol("x")
series(sin(x)/x, x, 0, 8)
```
```result
1 - 1/6*x^2 - 1/5040*x^6 + 1/120*x^4 + O(x^8) (Symbolic)
```

```lovelace
x = symbol("x")
series(exp(x), x, 0, 5)
```
```result
1 + x + 1/24*x^4 + 1/6*x^3 + 1/2*x^2 + O(x^5) (Symbolic)
```

The trailing `O(x^n)` marks the truncation order explicitly.


## 9. Limits

```lovelace
x = symbol("x")
limit(sin(x)/x, x, 0)
```
```result
1 (Symbolic)
```

```lovelace
x = symbol("x")
limit((1 - cos(x))/x^2, x, 0)
```
```result
1/2 (Symbolic)
```

Two-sided limits report disagreement between the one-sided limits instead of forcing a
value:

```lovelace
x = symbol("x")
limit(1/x, x, 0)
```
```result
does not exist (left: -inf, right: +inf)
```

One-sided limits are first-class (`limit_left`, `limit_right`):

```lovelace
x = symbol("x")
limit_left(1/x, x, 0)
```
```result
-inf (Symbolic)
```

```lovelace
x = symbol("x")
limit_right(1/x, x, 0)
```
```result
inf (Symbolic)
```

```lovelace
x = symbol("x")
limit(1/x^2, x, 0)
```
```result
inf (Symbolic)
```

Rational functions at infinity use exact degree comparison:

```lovelace
x = symbol("x")
limit(1/(x+1), x, inf)
```
```result
0 (Symbolic)
```

```lovelace
x = symbol("x")
limit(x^2 + 1, x, inf)
```
```result
inf (Symbolic)
```

```lovelace
x = symbol("x")
limit((2*x^2 + 1)/(x^2 - 1), x, inf)
```
```result
2 (Symbolic)
```

When no safe closed form is found the limit is reported unevaluated — never guessed:

```lovelace
x = symbol("x")
limit(exp(-(x^2)), x, inf)
```
```result
unevaluated: coefficient does not evaluate at the point
```


## 10. Integration

The integrator is tiered (linearity, tables, substitution, parts, partial fractions) and
**self-verifying**: every closed form is differentiated and checked against the integrand
before it is returned.

```lovelace
x = symbol("x")
integrate(x^5, x)
```
```result
1/6*x^6 (Symbolic)
```

```lovelace
x = symbol("x")
integrate(1/x, x)
```
```result
log(x) (Symbolic)
```

```lovelace
x = symbol("x")
integrate(1/(x^2 - 1), x)
```
```result
-1/2*log((1 + x)) + 1/2*log((-1 + x)) (Symbolic)
```

```lovelace
x = symbol("x")
integrate(2*x*cos(x^2), x)
```
```result
sin(x^2) (Symbolic)
```

Failure preserves the integral as a first-class, unevaluated node:

```lovelace
x = symbol("x")
integrate(exp(-(x^2)), x)
```
```result
integrate(exp((-x^2)), x) (Symbolic)
```


## 11. Solving equations

Dispatch is by mathematical structure: linear, polynomial (formulas to degree 3, then
`RootOf`), rational, and invertible elementary compositions — always with explicit
conditions.

```lovelace
x = symbol("x")
solve(2*x - 4 == 0, x)
```
```result
[2] (Vector)
```

```lovelace
x = symbol("x")
solve(x^2 - 4 == 0, x)
```
```result
[-2, 2] (Vector)
```

```lovelace
x = symbol("x")
solve(x^4 - 5*x^2 + 4 == 0, x)
```
```result
[-2, -1, 1, 2] (Vector)
```

```lovelace
x = symbol("x")
solve(x^3 - 1 == 0, x)
```
```result
[1, 1/2*(-1 + -3^(1/2)), 1/2*(-1 - -3^(1/2))] (Vector)
```

(In the cubic roots above, `-3^(1/2)` denotes the principal complex square root, i.e.
`i*sqrt(3)` — the roots are the two complex cube roots of unity.)

```lovelace
x = symbol("x")
solve(exp(x) == 5, x)
```
```result
[log(5)] (Vector)
```

Periodic inverses return parametric families rather than a single principal branch:

```lovelace
x = symbol("x")
solve(sin(x) == 0, x)
```
```result
k*pi for integer k
```

```lovelace
x = symbol("x")
y = symbol("y")
solve(x + y == 0, x)
```
```result
[-y] (Vector)
```

An inconsistent equation returns the equation itself (no solutions); an unsupported
structure is reported unevaluated:

```lovelace
x = symbol("x")
solve(x - x + 1 == 0, x)
```
```result
no solutions: the equation reduces to a nonzero constant.
```

```lovelace
x = symbol("x")
solve(exp(x) + x == 0, x)
```
```result
x + exp(x) = 0
```


## 12. Symbolic matrices

Matrices are the ordinary array literals with symbolic elements; `det` uses fraction-free
Bareiss elimination, `inv` the adjugate over the determinant.

```lovelace
x = symbol("x")
y = symbol("y")
det([[x, 1], [y, x]])
```
```result
x^2 - y (Symbolic)
```

```lovelace
x = symbol("x")
y = symbol("y")
inv([[x, 1], [0, y]])
```
```result
[[y/(x*y), -1/(x*y)], [0/(x*y), x/(x*y)]] (Vector)
```

The zero entry stays `0/(x*y)`: the inverse exists only where `det = x*y` is nonzero, and
the kernel does not silently define `0/det` at `det = 0`.

```lovelace
x = symbol("x")
y = symbol("y")
matmul([[x, 1], [0, x]], [[y], [1]])
```
```result
[[1 + x*y], [x]] (Array)
```

```lovelace
x = symbol("x")
y = symbol("y")
trace([[x, y], [y, x]])
```
```result
2*x (Symbolic)
```

```lovelace
x = symbol("x")
y = symbol("y")
dot([x, y], [1, 2])
```
```result
x + 2*y (Symbolic)
```


## 13. Optimization and MathIR

`optimize` performs CSE accounting and Hornerization over the given parameters:

```lovelace
x = symbol("x")
optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])
```
```result
x^2*(1 + x*(3 + x*(2 + x))) (Symbolic)
```

`lower` compiles the optimized expression to MathIR (a typed DAG with a constant pool,
parameter slots, and power chains), and `evalir` executes it at a chosen precision:

```lovelace
x = symbol("x")
k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])
lower(k, [x])
```
```result
#!mathir 2
param x
const (rat 2 1)
const (rat 1 1)
const (rat 3 1)
Parameter 0 0
Constant 0 0
PowInt 0 0,1
Constant 0 1
Constant 0 2
Constant 0 0
Add 0 5,0
Mul 0 0,6
Add 0 4,7
Mul 0 0,8
Add 0 3,9
Mul 0 2,10
```

```lovelace
x = symbol("x")
k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x])
ir = lower(k, [x])
evalir(ir, [2], 40)
```
```result
92 (Integer)
```


## 14. Relations, conditions, and parametric solutions

Comparisons build first-class relations, and the logical combinators compose them:

```lovelace
x = symbol("x")
cond = and(x > 0, x < 1)
```
```result
x < 1 and x > 0 (Symbolic)
```

`assume` accepts conjunctions and negations of relations:

```lovelace
x = symbol("x")
assume(and(x > 0, x < 5))
assume(not(x == 2))
assumptions()
```
```result
x > 0; x < 5; x != 2
```

## 15. Symbolic matrices: rank and linear systems

```lovelace
x = symbol("x")
y = symbol("y")
matrix_rank([[x, 1], [y, x]])
```
```result
2 (Integer)
```

```lovelace
x = symbol("x")
y = symbol("y")
linsolve([[x, 1], [0, y]], [0, 1])
```
```result
[-x/x*(x*y), x/(x*y)] (Vector)
```

Entries keep their definedness-preserving form (`x/(x*y)` is `1/y` away from `x*y = 0`,
which is exactly the condition the solution requires).

## 16. Compilation and batch evaluation

`compile` lowers to MathIR; `evalir_batch` evaluates many points in one call:

```lovelace
x = symbol("x")
k = compile(x^2 + 1, [x])
evalir_batch(k, [1, 2, 3], 40)
```
```result
[2, 5, 10] (Vector)
```

## 17. Substitution and evaluation

```lovelace
x = symbol("x")
subs(x^2 + 1, x, 3)
```
```result
10 (Symbolic)
```

`evalf` evaluates at arbitrary precision through the exact `Real` engine — exact rationals
stay exact (periodic), and irrationals come back to the requested digits:

```lovelace
evalf(1/3, 25)
```
```result
0.3333333333333333333333333 (Real)
```

```lovelace
setprecision(20)
evalf(sqrt(2), 20)
```
```result
1.4142135623730950488 (Real)
```

Unbound symbols are an error, not a silent guess:

```lovelace
x = symbol("x")
evalf(x, 20)
```
```result
error: No value for symbol 'x'.
```


## 15. Determinism

The kernel never iterates in hash order, never uses culture-dependent formatting for
identity, and never relies on reflection. The same script, assumptions, and seed produce
byte-identical output on every run and platform — including under Native AOT
(`make runner` ships the symbolic engine in the published binary).

---

## 16. Library usage (C#)

The same kernel is a plain .NET library. The snippets below are compiled and executed by
`Lovelace.Symbolics.Tests/UsageExamples.cs`; `DocsSyncTests.cs` keeps them in sync with this
document.

### 16.1 Context, symbols, and canonical construction
```csharp
var ctx = new ExprContext();
Exprs.Current = ctx;
var x = ctx.Symbol("x");
Expr f = Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1);
Assert.Equal("1 + x^2 + 2*x", Printing.PrettyPrint(f));
```

### 16.2 Equality and hash-consing
```csharp
var a = Exprs.Add(Exprs.Power(x, 2), x);
var b = Exprs.Add(x, Exprs.Power(x, 2));
Assert.Same(a, b);   // canonical construction: equal structure is the same reference
```

### 16.3 Assumptions drive simplification
```csharp
var e = Exprs.Power(Exprs.Power(x, 2), Exprs.Rational(1, 2));
Assert.Equal(e, Simplify.SimplifyExpr(e, ctx));   // unconstrained: unchanged
ctx.Assumptions = ctx.Assumptions.Add(new SymbolPropertyAssumption(x, SymbolPredicate.NonNegative));
Assert.Equal(x, Simplify.SimplifyExpr(e, ctx));   // x >= 0: sqrt(x^2) = x
```

### 16.4 Canonical text round-trip
```csharp
var text = Printing.CanonicalPrint(f);
var back = Printing.CanonicalParse(text, ctx);
Assert.Same(f, back);   // versioned, lossless, hash-consed round-trip
```

### 16.5 Numeric evaluation at precision
```csharp
using var scope = global::Lovelace.Real.Real.WithPrecision(50, 15);
var g = Exprs.Add(Exprs.Power(x, 2), 1);
var value = Evaluation.EvaluateToNum(g, ctx, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(3L)) });
Assert.Equal(0, NumOps.Compare(value, NumOps.FromLong(10)));
```

### 16.6 MathIR: lower and evaluate
```csharp
var prog = MathIR.Lowering.Lower(Exprs.Add(Exprs.Power(x, 2), 1), ctx, new[] { x });
var irValue = MathIR.IrEvaluator.Evaluate(prog, new Dictionary<Symbol, Num> { [x] = new NumRat(Rat.FromLong(4L)) }, ctx);
Assert.Equal(0, NumOps.Compare(irValue, NumOps.FromLong(17)));
```

### 16.7 Symbolic matrices
```csharp
var y = ctx.Symbol("y");
var m = SymbolicMatrix.From(new[] { new Expr[] { x, 1 }, new Expr[] { y, x } });
Assert.Equal(Exprs.Subtract(Exprs.Power(x, 2), y), m.Det(ctx));
```

### 16.8 Exactness discipline
```csharp
Assert.True(Exprs.Rational(Rat.From(1, 3)).IsExact);            // exact rational
Assert.False(Exprs.Power(Exprs.Rational(2L), Exprs.Rational(1, 2)).IsExact);  // sqrt(2) is not exact
```

---

## 17. Limitations (v1, by design)

- `factor` is univariate over Q (multivariate factorization and Gröbner bases are the
  next layer); non-univariate input passes through unchanged.
- Cubic roots use the principal complex form (`sqrt(-3)` denotes `i*sqrt(3)`); quartic and
  higher-degree factors yield unevaluated `RootOf` roots, numerically evaluated on request.
- `solve` handles linear, polynomial (univariate), rational, and invertible elementary
  compositions; polynomial systems and inequality solving are not yet implemented.
- The simplifier's rule groups are deliberately small; it never invents identities.
- Series expansion points must be numeric constants; one-sided limits are exposed as
  `limit_left`/`limit_right` and two-sided limits report side disagreement explicitly.
- The e-graph optimizer, Monte-Carlo falsification harness, agent CLI, and Lean proofs are
  tracked as the next work packages (`docs/symbolics/implementation-plan.md`, SYM-24..48).

---

## 18. How this document is verified

| Check | Mechanism |
|---|---|
| Every `lovelace`/`result` pair | `UsageDocumentationTests` runs each script in a fresh `SuiteEngine` with `SymbolicsPlugin`/`MathIRPlugin` loaded and asserts the exact rendered result, print output, or error message |
| Every C# snippet | `UsageExamples.cs` contains the identical code in compiling, asserting `[Fact]`s; `DocsSyncTests` fails if any snippet here does not appear (whitespace-normalized) in that file |
| The whole kernel | `Lovelace.Symbolics.Tests` (acceptance + invariants), `Lovelace.Suite.Tests` (412 incl. the language reference doctests), and the full solution suites run green in CI; `make runner` publishes the engine as a Native AOT binary |