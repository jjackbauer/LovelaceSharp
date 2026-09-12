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
3*x^2 + 4*x + 5 (Symbolic)
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
so the kernel keeps the pole visible. `simplify` is **safe by default**: it applies only
rules whose side conditions are already provable from the active assumptions, so the bare
convenience form never silently discards a condition:

```lovelace
x = symbol("x")
simplify(x/x)
```
```result
x/x (Symbolic)
```

```lovelace
x = symbol("x")
assume(x != 0)
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
x^2 - 2*x + 1 (Symbolic)
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
AssumptionSet(display: x > 5, assumptions: [x > 5]) (AssumptionSet)
```

`assumptions()` is a structured record, not a display string: `display` carries the one-line
rendering above and `assumptions` carries the atoms themselves (a relation is a symbolic value, a
domain restriction a `DomainCondition` record), which is also what `inspect(f).assumptions`
projects.

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
sqrt(y^2) (Symbolic)
```

```lovelace
simplify(sqrt(4))
```
```result
2 (Symbolic)
```


## 4. Simplification

`simplify` runs budgeted, assumption-aware rewrite groups. The v1 groups cover the
Pythagorean identity, power rules, and exp/log inverses. Conditional rules — cancellation
and the exp/log inverses — fire only when their side conditions are provable; otherwise the
expression is left untouched:

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
exp(log(x)) (Symbolic)
```

```lovelace
x = symbol("x")
assume(x > 0)
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
x^3 + 3*x^2 + 3*x + 1 (Symbolic)
```

```lovelace
x = symbol("x")
y = symbol("y")
collect(x*y + x + 2, x)
```
```result
2 + x*(y + 1) (Symbolic)
```

`factor` works over Q[x] (square-free decomposition + rational-root linear factors).

```lovelace
x = symbol("x")
factor(x^4 - 5*x^2 + 4)
```
```result
(x - 2)*(x - 1)*(x + 1)*(x + 2) (Symbolic)
```

```lovelace
x = symbol("x")
cancel((x^2 - 1)/(x - 1))
```
```result
x + 1 (Symbolic)
```

```lovelace
x = symbol("x")
apart(1/(x^2 - 1), x)
```
```result
-1/2/(x + 1) + 1/2/(x - 1) (Symbolic)
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
error: diff(): argument 2 must be a symbolic variable; got Natural.
```


## 7. Vector calculus: jacobian, hessian

```lovelace
x = symbol("x")
y = symbol("y")
jacobian([x*y, x+y], [x, y])
```
```result
[[y, x], [1, 1]] (Array)
```

```lovelace
x = symbol("x")
y = symbol("y")
hessian(x*y, [x, y])
```
```result
[[0, 1], [1, 0]] (Array)
```


## 8. Series

```lovelace
x = symbol("x")
series(sin(x)/x, x, 0, 8)
```
```result
1 - 1/6*x^2 + 1/120*x^4 - 1/5040*x^6 + O(x^8) (Symbolic)
```

```lovelace
x = symbol("x")
series(exp(x), x, 0, 5)
```
```result
1 + x + 1/2*x^2 + 1/6*x^3 + 1/24*x^4 + O(x^5) (Symbolic)
```

The trailing `O(x^n)` marks the truncation order explicitly.


## 9. Limits

`limit`, `limit_left` and `limit_right` all return the SAME `LimitResult` record `limit_full`
returns — `status`, `exists`, `value`, the one-sided `left`/`right` values each with the constraint
it holds under, `exactness` and `diagnostics` — so a caller reads the answer off the structure
instead of parsing prose out of the result:

```lovelace
x = symbol("x")
limit(sin(x)/x, x, 0)
```
```result
LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

```lovelace
x = symbol("x")
limit((1 - cos(x))/x^2, x, 0)
```
```result
LimitResult(status: Value, exists: True, value: 1/2, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

Two-sided limits report disagreement between the one-sided limits instead of forcing a
value:

```lovelace
x = symbol("x")
limit(1/x, x, 0)
```
```result
LimitResult(status: DoesNotExist, exists: False, value: , left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

One-sided limits are first-class (`limit_left`, `limit_right`):

```lovelace
x = symbol("x")
limit_left(1/x, x, 0)
```
```result
LimitResult(status: MinusInfinity, exists: True, value: -inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

```lovelace
x = symbol("x")
limit_right(1/x, x, 0)
```
```result
LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

```lovelace
x = symbol("x")
limit(1/x^2, x, 0)
```
```result
LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

Rational functions at infinity use exact degree comparison:

```lovelace
x = symbol("x")
limit(1/(x+1), x, inf)
```
```result
LimitResult(status: Value, exists: True, value: 0, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

```lovelace
x = symbol("x")
limit(x^2 + 1, x, inf)
```
```result
LimitResult(status: PlusInfinity, exists: True, value: inf, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

```lovelace
x = symbol("x")
limit((2*x^2 + 1)/(x^2 - 1), x, inf)
```
```result
LimitResult(status: Value, exists: True, value: 2, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)
```

When no safe closed form is found the limit is reported unevaluated — never guessed:

```lovelace
x = symbol("x")
limit(exp(-(x^2)), x, inf)
```
```result
LimitResult(status: Unevaluated, exists: , value: , left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: None, diagnostics: [Diagnostic(code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, recoverable: True, location: , details: [])]) (LimitResult)
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
-1/2*log(x + 1) + 1/2*log(x - 1) (Symbolic)
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
integrate(exp(-x^2), x) (Symbolic)
```


### 10b. Assumed bounds decide relations

An assumption with a constant bound is a real decision procedure, not a remark: relations the
assumptions decide fold to Booleans, and relations they do not decide stay symbolic (never a
guess).

```lovelace
x = symbol("x")
assume(x >= 5)
simplify(x < 5)
```
```result
False (Boolean)
```

```lovelace
x = symbol("x")
assume(x >= 5)
simplify(x > 4)
```
```result
True (Boolean)
```

Contradictory assumptions are rejected outright rather than silently kept:

```lovelace
x = symbol("x")
assume(x > 0)
assume(x <= 0)
```
```result
error: Assumption x <= 0 contradicts the existing assumptions (its negation x > 0 is provable).
```

## 11. Solving equations

Dispatch is by mathematical structure: linear, polynomial (formulas to degree 3, then
`RootOf`), rational, and invertible elementary compositions — always with explicit
conditions. Every answer, complete or not, is the same `SolveResult` record `solve_full` returns:
the status is an enum, the solutions are records, and a parametric family is a record too, so
nothing has to be recovered from a display string.

```lovelace
x = symbol("x")
solve(2*x - 4 == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

```lovelace
x = symbol("x")
solve(x^2 - 4 == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

```lovelace
x = symbol("x")
solve(x^4 - 5*x^2 + 4 == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: -1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 4, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

```lovelace
x = symbol("x")
solve(x^3 - 1 == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: 1/2*(i*sqrt(3) - 1), conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: 1/2*(-i*sqrt(3) - 1), conditions: [], multiplicity: 1, exactness: AlgebraicExact), Solution(value: 1, conditions: [], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 3, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

(In the cubic roots above the principal complex square root is now written out in closed form:
`sqrt(-3)` is exactly `i*sqrt(3)`, so the two non-real roots are the complex cube roots of unity.
Round 03 made that value computable instead of leaving the radical unevaluated.)

```lovelace
x = symbol("x")
solve(exp(x) == 5, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [SolutionFamily(template: log(5) + 2*k*pi*i, parameter: k, period: 2*pi*i, parameter_domain: integer, conditions: [], exactness: ParametricExact)], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

Periodic inverses return parametric families rather than a single principal branch — the family is a
`SolutionFamily` record, so the parameter, its domain and the period are all readable from structure
(`solutions` stays empty because a family is not a finite list of values):

```lovelace
x = symbol("x")
solve(sin(x) == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [SolutionFamily(template: k*pi, parameter: k, period: pi, parameter_domain: integer, conditions: [], exactness: ParametricExact)], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

```lovelace
x = symbol("x")
y = symbol("y")
solve(x + y == 0, x)
```
```result
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -y, conditions: [1 != 0], multiplicity: 1, exactness: Exact)], families: [], common_conditions: [1 != 0], represented_count: 1, unrepresented_count: 0, unrepresented_reason: , diagnostics: []) (SolveResult)
```

Polynomial systems solve via Gröbner-basis elimination and back-substitution
(`solve_system`). Like `solve_system_full`, it publishes the `SystemSolveResult` record — the
status is an enum, each solution's bindings are records, and `complete`/`completeness` are read off
the one mapping the protocol documents:

```lovelace
x = symbol("x")
y = symbol("y")
solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])
```
```result
SystemSolveResult(status: Solved, domain: complex, complete: True, completeness: Complete, solutions: [SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: -1)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: -1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 1), Binding(name: y, value: 0)], conditions: [], exactness: Exact), SystemSolution(bindings: [Binding(name: x, value: 0), Binding(name: y, value: 1)], conditions: [], exactness: Exact)], diagnostics: []) (SystemSolveResult)
```

An inconsistent equation is a structured `NoSolutions` answer (the reason rides in `diagnostics`);
an unsupported structure is `Unevaluated`:

```lovelace
x = symbol("x")
solve(x - x + 1 == 0, x)
```
```result
SolveResult(status: NoSolutions, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: the equation reduces to a nonzero constant., recoverable: True, location: , details: [])]) (SolveResult)
```

```lovelace
x = symbol("x")
solve(exp(x) + x == 0, x)
```
```result
SolveResult(status: Unevaluated, variable: x, domain: complex, complete: False, completeness: Unknown, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.unevaluated, category: UnsupportedOperation, message: No solver for this structure., recoverable: True, location: , details: [])]) (SolveResult)
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
[[y/(x*y), -1/(x*y)], [0/(x*y), x/(x*y)]] (Array)
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
x^2*(1 + x*(3 + x*(x + 2))) (Symbolic)
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
AssumptionSet(display: x > 0; x < 5; x != 2, assumptions: [x > 0, x < 5, x != 2]) (AssumptionSet)
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
[-x/x/(x*y), x/(x*y)] (Vector)
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

The digit count holds for an argument that arrives **already numeric** too. `sqrt(2)` is evaluated
at the ambient precision before `evalf` runs, so the value is truncated to the requested count
instead of being passed through at 100 digits:

```lovelace
evalf(sqrt(2), 5)
```
```result
1.41421 (Real)
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


## 18. Determinism

The kernel never iterates in hash order, never uses culture-dependent formatting for
identity, and never relies on reflection. The same script, assumptions, and seed produce
byte-identical output on every run and platform — including under Native AOT
(`make runner` ships the symbolic engine in the published binary).

---

## 18b. Structured results (the `*_full` APIs)

The `*_full` builtins return first-class structured records, and the short forms that have one
(`solve`, `solve_system`) publish the SAME record rather than a second, looser projection — so no
answer has to be recovered from prose. Property access (`r.solutions`) reads the fields, and the
same records serialize structurally through the DSH runner.

```lovelace
x = symbol("x")
solve_full(x^2 - 4 == 0, x).status
```
```result
Solved
```

```lovelace
x = symbol("x")
solve_full((x^2 - 1)/(x - 1) == 0, x).solutions
```
```result
[Solution(value: -1, conditions: [x - 1 != 0], multiplicity: 1, exactness: Exact)] (Vector)
```

```lovelace
x = symbol("x")
solve_full((x^2 - 1)/(x - 1) == 0, x).common_conditions
```
```result
[x - 1 != 0] (Vector)
```

Every solution carries its own conditions — there is no cross-branch union. A result is
`Solved` only when it is the complete solution set over the requested domain:

```lovelace
x = symbol("x")
solve_full(x^4 - x^2 - 1 == 0, x).status
```
```result
Partial
```

```lovelace
x = symbol("x")
solve_full(x^4 - x^2 - 1 == 0, x).complete
```
```result
False (Boolean)
```

```lovelace
x = symbol("x")
solve_full(x^4 - x^2 - 1 == 0, x).domain
```
```result
complex (Domain)
```

```lovelace
x = symbol("x")
solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count
```
```result
2 (Integer)
```

```lovelace
x = symbol("x")
simplify_full(x/x).expression
```
```result
1 (Symbolic)
```

```lovelace
x = symbol("x")
simplify_full(x/x).conditions
```
```result
[x != 0] (Vector)
```

```lovelace
x = symbol("x")
simplify_full(x/x).steps[0].rule_id
```
```result
rat.cancel-x-over-x
```

```lovelace
x = symbol("x")
limit_full(1/x, x, 0).exists
```
```result
False (Boolean)
```

```lovelace
x = symbol("x")
limit_full(1/x, x, 0).left
```
```result
-inf (Symbolic)
```

```lovelace
x = symbol("x")
integrate_full(x^2, x).status
```
```result
SolvedExact
```

```lovelace
x = symbol("x")
integrate_full(x^2, x).verified
```
```result
True (Boolean)
```

```lovelace
x = symbol("x")
compile_full(x^2 + 1, [x]).mathir_version
```
```result
2 (Integer)
```

```lovelace
x = symbol("x")
compile_full(x^2 + 1, [x]).parameters
```
```result
[ParameterInfo(name: x, domain: complex)] (Vector)
```

```lovelace
x = symbol("x")
compile_full(x^2 + 1, [x]).result_domain
```
```result
complex (Domain)
```

```lovelace
x = symbol("x")
y = symbol("y")
linsolve_full([[x, 1], [0, y]], [0, 1]).conditions
```
```result
[x*y != 0] (Vector)
```

```lovelace
x = symbol("x")
type(solve_full(x^2 - 4 == 0, x))
```
```result
SolveResult
```

```lovelace
x = symbol("x")
inspect(x^2 + 1).free_symbols
```
```result
[x] (Vector)
```

Solver domains are first-class values (never magic strings); the default is Complex, and an empty
result is a structured `NoSolutions` answer rather than prose:

```lovelace
x = symbol("x")
solve(x^2 + 1 == 0, x, real)
```
```result
SolveResult(status: NoSolutions, variable: x, domain: real, complete: True, completeness: Complete, solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [Diagnostic(code: solve.no-solutions, category: NoSolution, message: no real solutions, recoverable: True, location: , details: [])]) (SolveResult)
```

## 19. Library usage (C#)

The same kernel is a plain .NET library. The snippets below are compiled and executed by
`Lovelace.Symbolics.Tests/UsageExamples.cs`; `DocsSyncTests.cs` keeps them in sync with this
document.

### 16.1 Context, symbols, and canonical construction
```csharp
var ctx = new ExprContext();
Exprs.Current = ctx;
var x = ctx.Symbol("x");
Expr f = Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(2, x), 1);
Assert.Equal("x^2 + 2*x + 1", Printing.PrettyPrint(f));
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
Assert.True(Exprs.Power(Exprs.Rational(2L), Exprs.Rational(1, 2)).IsExact);  // sqrt(2): a radical is exact
Assert.True(Exprs.Function(ctx.Function("sin"), Exprs.Symbol("x")).IsExact); // an exact closed form is exact
Assert.False(Exprs.Real(RealLiteral.FromRationalExact(Rat.From(3, 2))).IsExact); // the Real leaf is the carrier
```

---

## 20. Limitations (v1, by design)

- `factor` is univariate over Q; multivariate factorization is deferred. Gröbner bases
  (`Groebner.Basis/Reduce/Eliminate` over lex/grlex/grevlex) and polynomial-systems
  solving (`solve_system`, via lex elimination + back-substitution) are available in the
  C# API; a standalone language `groebner` builtin is not exposed yet.
- Cubic roots use the branch-coupled Cardano form (`u·v = −P/3`), verified against their
  polynomial; quartic and higher-degree factors yield `RootOf` roots over the exact Sturm
  count of REAL roots (complex algebraic numbers are deferred), numerically evaluated on
  request. The solver's domain argument is explicit and never widened: the default is Complex,
  and `solve(…, integer)` / `solve(…, rational)` is REJECTED rather than silently answered over
  the complexes. The refusal is scoped to the SOLVER's domain argument — `symbol(name, integer)`,
  `assume_integer` and a solution family's integer parameter domain stay supported (see §2), so
  `capabilities()` reports the pair as `solve_domains_accepted` / `solve_domains_refused` rather
  than as a claim about the runtime's domains as a whole.
- **A result is reported as a complete solution set only when it is one.** A degree ≥ 4
  factor with real roots *and* non-real ones is reported `Partial`
  (`complete: False`, with `unrepresented_count` and `unrepresented_reason`); with no real
  roots at all it is `Unevaluated`. `solve` and `solve_full` publish the same `SolveResult`
  record, so a partial set is never shown as a complete vector and never described in prose:
  the status, the counts and the reason are all fields. Real roots come back in ascending
  order; other roots follow a deterministic canonical order.
- `solve` handles linear, polynomial (univariate), rational, and invertible elementary
  compositions (with parametric families for periodic inverses); inequality solving is not
  yet implemented.
- The simplifier's rule groups are deliberately small and condition-carrying; it never
  invents identities.
- Series expansion points must be numeric constants; truncation is explicit (`O(x^n)` terms).
- The e-graph optimizer, an agent CLI, and Lean proofs remain deferred; the Monte-Carlo
  falsification harness, seeded property suites, and the SymPy differential oracle ship in
  `Lovelace.Symbolics.Tests`.

---

## 21. How this document is verified

| Check | Mechanism |
|---|---|
| Every `lovelace`/`result` pair | `UsageDocumentationTests` runs each script in a fresh `SuiteEngine` with `SymbolicsPlugin`/`MathIRPlugin` loaded and asserts the exact rendered result, print output, or error message |
| Every C# snippet | `UsageExamples.cs` contains the identical code in compiling, asserting `[Fact]`s; `DocsSyncTests` fails if any snippet here does not appear (whitespace-normalized) in that file |
| The whole kernel | `Lovelace.Symbolics.Tests` (acceptance + invariants + DX contracts), `Lovelace.Suite.Tests` (438 incl. the language-reference doctests and the help/record DX contracts), and the full solution suites run green in CI; `make runner` publishes the engine as a Native AOT binary |