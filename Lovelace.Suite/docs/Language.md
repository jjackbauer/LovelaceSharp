# Lovelace Language Reference

This is the authoritative reference for the Lovelace scripting language implemented by
`Lovelace.Suite` — the tokenizer → parser → interpreter engine behind the REPL
(`Lovelace.Console`), the web IDE (`Lovelace.Studio`), and the headless runner
(`Lovelace.Run`).

Every code example in this document is **executable and machine-checked**. A doctest
(`LanguageDocumentationTests` in `Lovelace.Suite.Tests`) reads this file, evaluates each
`lovelace` block in a fresh engine, and asserts that the following `result` block matches the
actual engine output exactly. If the language changes, the test fails until this document is
updated — so this reference cannot silently drift out of date.

### The example format

Each example is a `lovelace` fenced code block immediately followed by a `result` fenced code
block. A `result` block asserts one of four things:

| `result` content | Meaning |
|---|---|
| `3 (Natural)` | The final value renders to exactly `3 (Natural)`. |
| `error: Undefined variable 'x'.` | Evaluation throws, with exactly that message. |
| `prints: hi [1, 2, 3]` | `print` output equals the text (and the result is void). |
| `plot: 1/x^2` | A plot was rendered whose title is `1/x^2`. |

Scripts are evaluated the same way every host evaluates them: newlines are rewritten to `;`
via `ScriptSource.ToSemicolonStatements`, then the program runs in a fresh `SuiteEngine`.

---

## 1. Values and types

A value has exactly one of these kinds:

| Kind | Domain / meaning | Example rendering |
|---|---|---|
| `Natural` | Non-negative arbitrary-precision integers | `42 (Natural)` |
| `Integer` | Signed arbitrary-precision integers | `-5 (Integer)` |
| `Real` | Arbitrary-precision reals, exact periodic fractions | `0.(3) (Real)` |
| `Boolean` | `True` / `False` | `True (Boolean)` |
| `Text` | Strings and interpolated strings | `hello` |
| `Vector` | Numeric list (ranges, list literals) | `[1, 2, 3] (Vector)` |
| `Function` | First-class function reference | `Function: square (Function)` |
| `Void` | Result of statements that produce no value | `(void)` |

The three numeric kinds form a widening chain `Natural → Integer → Real`. Arithmetic widens
operands to the wider kind and returns that kind (see [§4 Arithmetic](#4-arithmetic)).

---

## 2. Literals

### Numbers

A literal is a `Natural` when it has no decimal point or periodic suffix, and a `Real` when it
does. There is no negative literal: `-5` is unary negation applied to the `Natural` `5`, which
produces an `Integer`.

```lovelace
42
```
```result
42 (Natural)
```

```lovelace
3.14
```
```result
3.14 (Real)
```

```lovelace
-5
```
```result
-5 (Integer)
```

A `Real` can be written with an explicit periodic part, `a.(b)`, meaning the digit block `b`
repeats forever. This is the exact representation the engine itself produces for non-terminating
fractions.

```lovelace
0.(3)
```
```result
0.(3) (Real)
```

### Strings

Plain strings use double quotes. They render without a type suffix.

```lovelace
"hello"
```
```result
hello
```

### Interpolated strings

`$"…"` interpolates `{expr}` parts using the value's display rendering. See
[§8 Strings and interpolation](#8-strings-and-interpolation).

```lovelace
$"x = {3 + 4}"
```
```result
x = 7
```

### List literals

```lovelace
[1, 2, 3]
```
```result
[1, 2, 3] (Vector)
```

### Ranges

`a..b` and `a..step..b` build vectors; see [§7 Vectors and ranges](#7-vectors-and-ranges).

```lovelace
1..5
```
```result
[1, 2, 3, 4, 5] (Vector)
```

---

## 3. Operators and precedence

Operators, tightest first:

| Precedence | Operator(s) | Associativity |
|---|---|---|
| 1 (highest) | `!` postfix factorial, `[i]` index | left |
| 2 | `-` `+` unary | right |
| 3 | `..` range | — |
| 4 | `^` power | right |
| 5 | `*` `/` `%` | left |
| 6 | `+` `-` | left |
| 7 | `==` `!=` `>` `<` `>=` `<=` | left |
| 8 (lowest) | `=` assignment | right |

Two points to internalise:

- **`^` is right-associative.** `2 ^ 3 ^ 2` is `2 ^ (3 ^ 2)`, not `(2 ^ 3) ^ 2`.

```lovelace
2 ^ 3 ^ 2
```
```result
512 (Natural)
```

- **`..` binds tighter than every arithmetic operator.** A range reads like an atomic value:
  `1..10 ^ 2` is `(1..10) ^ 2`, `2 * 1..5` is `2 * (1..5)`, and `1..5 + 1` is `(1..5) + 1`.
  To put an arithmetic expression *inside* a range endpoint, parenthesise it: `1..(n + 1)`.

```lovelace
1..10 ^ 2
```
```result
[1, 4, 9, 16, 25, 36, 49, 64, 81, 100] (Vector)
```

```lovelace
2 * 1..5
```
```result
[2, 4, 6, 8, 10] (Vector)
```

---

## 4. Arithmetic

Operators `+ - * / % ^ !` and unary `-`. The exact result kind follows the widening chain.

### Addition and multiplication

```lovelace
1 + 2
```
```result
3 (Natural)
```

```lovelace
2 * 3
```
```result
6 (Natural)
```

### Subtraction widens on underflow

Subtracting a larger `Natural` from a smaller one does not error — it widens to `Integer`.

```lovelace
7 - 3
```
```result
4 (Natural)
```

```lovelace
3 - 5
```
```result
-2 (Integer)
```

### Division is exact (never truncates)

Division always yields a `Real` when it is not exact, preserving the result exactly. A
non-terminating decimal is stored with its repeating block: `1 / 3` is exactly `0.(3)`, not a
rounded `double`.

```lovelace
1 / 3
```
```result
0.(3) (Real)
```

```lovelace
1 / 4
```
```result
0.25 (Real)
```

```lovelace
1 / 2
```
```result
0.5 (Real)
```

Division by zero is an error.

```lovelace
1 / 0
```
```result
error: Cannot divide by zero.
```

### Widening to Real

Mixing an integer and a `Real` widens to `Real`.

```lovelace
2 + 0.5
```
```result
2.5 (Real)
```

```lovelace
-7 / 2
```
```result
-3.5 (Real)
```

### Power

```lovelace
2 ^ 10
```
```result
1024 (Natural)
```

### Factorial

Postfix `!`, defined for `Natural` and `Integer`.

```lovelace
5!
```
```result
120 (Natural)
```

### Modulo

`a % b` has the sign of the dividend (C-style), and widens like the other operators.

```lovelace
10 % 3
```
```result
1 (Natural)
```

```lovelace
-7 % 3
```
```result
-1 (Integer)
```

---

## 5. Comparison and Booleans

Comparison operators return a `Boolean`. They compare only numeric values.

```lovelace
2 > 1
```
```result
True (Boolean)
```

```lovelace
1 == 1
```
```result
True (Boolean)
```

```lovelace
3 <= 3
```
```result
True (Boolean)
```

```lovelace
2 != 2
```
```result
False (Boolean)
```

---

## 6. Variables and assignment

`name = value` evaluates `value`, stores it, and **returns the value** (so it can be used
inline or chained). Assignment is right-associative.

```lovelace
x = 42
```
```result
42 (Natural)
```

```lovelace
x = 42; x + 1
```
```result
43 (Natural)
```

> **The `_` variable.** In the REPL, after each successful non-void evaluation the result is
> also stored in `_`. This is per-evaluation state (the REPL sets it between prompts), so a
> fresh script does not see a previous `_`. It is a convenience for interactive use, not a
> script feature.

---

## 7. Vectors and ranges

### Ranges

`a..b` builds the inclusive range from `a` to `b` with step `1`. `a..step..b` uses an explicit
step. With no explicit step and both endpoints `Natural`, the elements are `Natural`; otherwise
they are `Integer`.

```lovelace
1..5
```
```result
[1, 2, 3, 4, 5] (Vector)
```

```lovelace
1..2..7
```
```result
[1, 3, 5, 7] (Vector)
```

```lovelace
-2..2
```
```result
[-2, -1, 0, 1, 2] (Vector)
```

A negative step descends.

```lovelace
5..-1..1
```
```result
[5, 4, 3, 2, 1] (Vector)
```

A zero step is an error.

```lovelace
1..0..5
```
```result
error: Range step must not be zero.
```

### List literals

```lovelace
[1, 2, 3]
```
```result
[1, 2, 3] (Vector)
```

### Indexing

0-based. Out-of-range (including negative) indexes are errors; there is no negative indexing
from the end.

```lovelace
[10, 20, 30][0]
```
```result
10 (Natural)
```

```lovelace
[10, 20, 30][3]
```
```result
error: Index 3 is out of range for vector of length 3.
```

```lovelace
[1, 2, 3][-1]
```
```result
error: Index -1 is out of range for vector of length 3.
```

### Length

```lovelace
len([5, 6, 7, 8])
```
```result
4 (Natural)
```

### Element-wise arithmetic and broadcast

`+ - * /` (and the other numeric operators) apply element-wise between arrays, with
right-aligned broadcasting (dimensions are equal, or one of them is 1), or broadcast a
scalar across an array. Incompatible shapes are an error.

```lovelace
[1, 2] + [10, 20]
```
```result
[11, 22] (Vector)
```

```lovelace
[1, 2, 3] * 10
```
```result
[10, 20, 30] (Vector)
```

```lovelace
[1, 2] + [[1, 2], [3, 4]]
```
```result
[[2, 4], [4, 6]] (Array)
```

```lovelace
[1, 2] + [1, 2, 3]
```
```result
error: Operands could not be broadcast together with shapes [2] and [3].
```

### Slicing

`a[start:stop:step]` returns a strided view; the step is optional and defaults to 1.

```lovelace
[0, 1, 2, 3, 4][1:4]
```
```result
[1, 2, 3] (Vector)
```

```lovelace
[[1, 2, 3], [4, 5, 6]][:, 1]
```
```result
[2, 5] (Vector)
```

---

## 8. Strings and interpolation

Plain strings are literal. Interpolated strings embed `{expr}` parts, rendered with the value's
display form (no type suffix). Escape a literal brace as `{{` / `}}`.

```lovelace
"hello"
```
```result
hello
```

```lovelace
$"x = {3 + 4}"
```
```result
x = 7
```

```lovelace
$"v = {1..3}"
```
```result
v = [1, 2, 3]
```

```lovelace
$"a {{ b }}"
```
```result
a { b }
```

---

## 9. Statements

A program is a `;`-separated sequence of statements; the value of the program is the value of
its **last** statement. Statements are: an expression, an assignment, a block `{ … }`,
`if (c) … else …`, `while (c) …`, `for i in range …`, `return`, `break`, `continue`, and a
`func` definition.

> **Newlines are not statement separators inside blocks.** Hosts rewrite *top-level* newlines to
> `;` before evaluation, but newlines inside `{ … }` are just whitespace. Separate statements in
> a block with `;`.

### Blocks

```lovelace
{ 1; 2; 3 }
```
```result
3 (Natural)
```

### Conditionals

The condition must be a `Boolean`.

```lovelace
if (2 > 1) { 10 } else { 20 }
```
```result
10 (Natural)
```

```lovelace
if (1) { 2 }
```
```result
error: if condition must be Boolean, but got 'Natural'.
```

### Loops

`for i in range` iterates the vector in order; `while (c)` loops while the condition is true.
`break` exits the nearest loop; `continue` skips to the next iteration. Both are only valid
inside a loop.

```lovelace
sum = 0; for i in 1..4 { sum = sum + i }; sum
```
```result
10 (Natural)
```

```lovelace
n = 0; while (n < 3) { n = n + 1 }; n
```
```result
3 (Natural)
```

```lovelace
break
```
```result
error: 'break' is only valid inside a loop.
```

---

## 10. Functions

Define a function with a block body or an expression body. Parameters are function-local and
shadow globals; assignments inside a function do not leak to the global scope.

```lovelace
func add(a, b) { a + b }; add(2, 3)
```
```result
5 (Natural)
```

```lovelace
func square(x) = x ^ 2; square(5)
```
```result
25 (Natural)
```

> The `func f(x) = expr` shorthand accepts a single **expression** only. Bodies that need
> statements (`if`, `while`, `for`, `return`, or multiple statements) must use the `{ … }` form.

`return` exits a function with an optional value.

```lovelace
func inc(x) { return x + 1 }; inc(5)
```
```result
6 (Natural)
```

Functions are recursive.

```lovelace
func fact(n) { if (n == 0) { 1 } else { n * fact(n - 1) } }; fact(5)
```
```result
120 (Natural)
```

---

## 11. Built-in functions

### `abs(x)`

Absolute value; preserves the numeric kind.

```lovelace
abs(-5)
```
```result
5 (Integer)
```

### `inv(x)`

Multiplicative inverse, returned as a `Real`.

```lovelace
inv(4)
```
```result
0.25 (Real)
```

### `divrem(a, b)`

Integer quotient and remainder, returned as text.

```lovelace
divrem(17, 5)
```
```result
quotient = 3, remainder = 2
```

### `is_even(x)` / `is_odd(x)`

```lovelace
is_even(4)
```
```result
True (Boolean)
```

```lovelace
is_odd(4)
```
```result
False (Boolean)
```

### `sign(x)`

`-1`, `0`, or `1` as an `Integer`.

```lovelace
sign(-7)
```
```result
-1 (Integer)
```

### `sqrt(x)`

Square root, returned as a `Real`. Computed to `Real.MaxComputationDecimalPlaces` fractional
digits (default `1000`); perfect squares converge immediately.

```lovelace
sqrt(4)
```
```result
2 (Real)
```

### `pi()` / `pi(digits)`

`π` via the Chudnovsky algorithm. With no argument it uses `Real.DisplayDecimalPlaces` (default
`100`); with an argument it uses that many digits.

```lovelace
pi(5)
```
```result
3.14159 (Real)
```

### `setprecision(n)`

Raises `Real.MaxComputationDecimalPlaces` (the hard cap on generated digits, default `1000`) and
`Real.DisplayDecimalPlaces` (how many fractional digits `ToString()` emits for non-periodic values,
default `100`) to the given positive integer `n`. Returns `void`. Use it before `pi(n)` (or other
irrational work) to compute and display more than the default precision.

```lovelace
setprecision(50)
pi(50)
```
```result
3.14159265358979323846264338327950288419716939937510 (Real)
```

### `len(v)`

Vector length.

```lovelace
len([5, 6, 7, 8])
```
```result
4 (Natural)
```

### `print(values…)`

Writes the display form of each argument (space-separated, followed by a newline) to the
engine's output, and returns void.

```lovelace
print("hi", 1..3)
```
```result
prints: hi [1, 2, 3]
```

### DSP functions (loaded by the CLI hosts)

The REPL, web IDE, and headless runner opt into the DSP extension via
`engine.RegisterDspBuiltins()`. These return complex vectors; `re`, `im`, `conj`, and `abs` bridge
a complex value back to the `Real` lattice. `conv`, `dft`, `fft`, `filter`, `movingavg`,
`impulse`, `step`, `cosine`, `exponential`, `powerseries`, `noise`, `delay`, and `scale` complete
the surface.

```lovelace
fft([1, 0, 0, 0])
```
```result
[1, 1, 1, 1] (Vector)
```

```lovelace
conv([1, 1], [1, 1])
```
```result
[1, 2, 1] (Vector)
```

```lovelace
z = fft([0, 1, 0, 0])[1]; [re(z), im(z), abs(z)]
```
```result
[0, -1, 1] (Vector)
```

---

## 12. Plotting

`plot(y)`, `plot(x, y)`, and `plot(x, y, "title")` render a 2D line plot to an SVG file and
return the output path as a `Text` value. The path is host-dependent, so the doctest asserts the
plot was produced and its title, not the path.

- `plot(y)` uses `1..len(y)` as the x-axis.
- `plot(x, y)` requires two equal-length vectors.
- The optional third argument is the title, rendered into the SVG.
- A series of three or more points is connected with a smooth cubic spline through the data
  (sampled densely, so a coarse sample draws as a curve, not an angular polygon); fewer than three
  points fall back to straight segments.

```lovelace
plot(1..10, 1 / (1..10 ^ 2), "1/x^2")
```
```result
plot: 1/x^2
```

---

## 13. Errors and diagnostics

Errors are `InvalidOperationException`s carrying a message, and the `SuiteEngine` attaches a
`Diagnostic` with a source line/column. Common failure modes:

```lovelace
nope + 1
```
```result
error: Undefined variable 'nope'.
```

```lovelace
2 + "x"
```
```result
error: Cannot widen from Natural to Text: only numeric kinds (Natural, Integer, Real) support widening.
```

---

## 14. N-Dimensional arrays

A **vector** is a rank-1 array, a **matrix** a rank-2 array, and a nested list literal of depth `k`
builds a rank-`k` array. Every row must be rectangular.

### Literals

```lovelace
[[1, 2], [3, 4]]
```
```result
[[1, 2], [3, 4]] (Array)
```

```lovelace
[[[1, 2], [3, 4]], [[5, 6], [7, 8]]]
```
```result
[[[1, 2], [3, 4]], [[5, 6], [7, 8]]] (Array)
```

A ragged nested list is an error.

```lovelace
[[1, 2], [3]]
```
```result
error: Ragged nested list literal: every row must have the same shape.
```

### Indexing

`a[i, j, …]` indexes with one coordinate per dimension (0-based); `a[i, …]` with fewer coordinates
returns a lower-rank sub-array.

```lovelace
[[1, 2], [3, 4]][1, 0]
```
```result
3 (Natural)
```

```lovelace
[[[1, 2], [3, 4]], [[5, 6], [7, 8]]][0]
```
```result
[[1, 2], [3, 4]] (Array)
```

### Construction and shape

```lovelace
zeros(2, 3)
```
```result
[[0, 0, 0], [0, 0, 0]] (Array)
```

```lovelace
reshape(1..6, 2, 3)
```
```result
[[1, 2, 3], [4, 5, 6]] (Array)
```

```lovelace
shape(zeros(2, 3))
```
```result
[2, 3] (Vector)
```

### Reductions

`sum`/`prod`/`min`/`max`/`mean`/`norm` collapse all elements, or reduce along one `axis`.

```lovelace
sum([[1, 2], [3, 4]])
```
```result
10 (Natural)
```

```lovelace
sum([[1, 2], [3, 4]], 0)
```
```result
[4, 6] (Vector)
```

```lovelace
mean([1, 2])
```
```result
1.5 (Real)
```

```lovelace
norm([3, 4])
```
```result
5 (Real)
```

### Linear algebra

```lovelace
matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])
```
```result
[[19, 22], [43, 50]] (Array)
```

```lovelace
dot([1, 2], [3, 4])
```
```result
11 (Natural)
```

```lovelace
cross([1, 0, 0], [0, 1, 0])
```
```result
[0, 0, 1] (Vector)
```

```lovelace
det([[1, 2], [3, 4]])
```
```result
-2 (Integer)
```

```lovelace
inv([[1, 2], [3, 4]])
```
```result
[[-2, 1], [1.5, -0.5]] (Array)
```

```lovelace
trace([[1, 2], [3, 4]])
```
```result
5 (Natural)
```

### Shape manipulation and concatenation

```lovelace
transpose([[1, 2], [3, 4]])
```
```result
[[1, 3], [2, 4]] (Array)
```

```lovelace
flatten([[1, 2], [3, 4]])
```
```result
[1, 2, 3, 4] (Vector)
```

```lovelace
append([1, 2], [3, 4])
```
```result
[1, 2, 3, 4] (Vector)
```

