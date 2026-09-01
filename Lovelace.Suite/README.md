# Lovelace.Suite

The scripting engine behind the LovelaceSharp REPL: a tokenizer → parser → interpreter that
compiles and executes a MATLAB/Scilab-style math scripting language, plus a public introspection
API and 2D SVG plotting. `Lovelace.Console` is a thin front-end over this library.

> **Language reference:** the complete, machine-checked syntax reference is
> [`docs/Language.md`](docs/Language.md). Every example there is verified by the
> `LanguageDocumentationTests` doctest.

---

## Architecture

```
Source text
    Tokenizer          Token.cs, Tokenizer.cs
    Parser             Ast.cs, Parser.cs   (expressions + statements)
    Interpreter        Interpreter.cs      (tree-walking backend)
    SuiteEngine        SuiteEngine.cs      (public façade + introspection API)
```

The AST (`Ast.cs`) is a stable intermediate representation: the interpreter is the first backend,
and a future bytecode/AOT compiler can reuse the same front-end.

---

## Language

### Values

| Kind | Payload |
|---|---|
| `Natural` / `Integer` / `Real` | arbitrary-precision numerics; widen `Natural → Integer → Real` |
| `Boolean` | from comparisons and predicates |
| `Text` | strings and interpolated strings |
| `Vector` | numeric list (ranges, list literals) — a rank-1 array |
| `Array` | N-dimensional array (rank ≥ 2): matrices and tensors, from nested list literals |
| `Function` | first-class function reference |
| `Void` | result of statements that produce no value |

### Statements

`expr`, `name = expr`, `{ … }`, `if (c) { … } else { … }`, `while (c) { … }`,
`for i in range { … }`, `return [expr]`, `break`, `continue`, and
`func name(a, b) { … }` (or `func f(x) = expr`).

### Vectors & N-D arrays

- `a..b` and `a..step..b` ranges; `[a, b, c]` list literals (a rank-1 **vector**).
- Nested rectangular lists build higher ranks: `[[1, 2], [3, 4]]` is a **matrix** (rank 2),
  `[[[...]]]` an N-D **array**. A ragged nested list is an error.
- 0-based indexing `v[i]` and multi-index `m[i, j]`; a partial index returns a sub-array.
- Element-wise `+ - * / % ^` between same-shape arrays, or with a scalar broadcast.
- Built-ins: reductions (`sum` `prod` `min` `max` `mean` `norm`, with optional `axis`),
  linear algebra (`dot` `cross` `matmul` `det` `inv` `trace`), construction
  (`zeros` `ones` `eye` `reshape`), introspection (`shape` `rank` `numel` `len`), and
  manipulation (`flatten` `transpose` `squeeze` `concat` `append`).
- A range binds tighter than arithmetic operators, so `1..10 ^ 2` is `(1..10) ^ 2`,
  `2 * 1..5` is `2 * (1..5)`, and `1..n + 1` is `(1..n) + 1` (parenthesize an endpoint to
  change that, e.g. `1..(n + 1)`).
- The array type and algorithms live in [`Lovelace.Array`](../Lovelace.Array/), consumed here as
  `NdArray<Value>`; the full reference is [`docs/Language.md`](docs/Language.md) §14.

### Strings and output

- `"plain"` and `$"interpolated {expr}"` (with `{{` / `}}` escapes).
- `print(expr)` writes the rendered value to the engine's `Output` writer.

### Plotting

`plot(y)`, `plot(x, y)`, or `plot(x, y, "title")` builds a `PlotModel` and renders it to a
deterministic SVG file via `SvgPlotRenderer`; the returned `Text` value is the output path.
Series of three or more points are drawn as a natural cubic spline through the data by default,
sampled densely into a `<polyline>` so a coarse sample renders as a curve (not an angular polygon)
without the overshoot/kinks a pixel-space Catmull-Rom spline can add. Set
`PlotSeries.Interpolation = PlotInterpolation.Linear` for straight segments (also used
automatically for fewer than three points).

---

## Public API

The façade class is [`SuiteEngine`](SuiteEngine.cs):

| Member | Description |
|---|---|
| `Evaluate(string)` / `EvaluateAsync(string)` | compile + execute a script/expression |
| `Parse(string)` / `ParseExpression(string)` | expose the front-end result (the IR) |
| `Variables` | live `name → Value` view of global variables |
| `Functions` | live `name → FunctionDefinition` view (user + built-in) |
| `SetVariable` / `TryGetVariable` / `RemoveVariable` / `Clear` | state mutation |
| `DefineFunction` / `RegisterBuiltin` | register functions |
| `CaptureState()` | immutable `StateSnapshot` (variables + functions + revision) |
| `VariableChanged` / `FunctionDefined` | change notifications |
| `Diagnostics` | position-carrying diagnostics from the last failure |
| `Output` / `PlotOutputDirectory` / `PlotFileName` | host settings |

---

## Usage

```csharp
using Lovelace.Suite;

var engine = new SuiteEngine();
await engine.EvaluateAsync("func f(x) = x ^ 2");
await engine.EvaluateAsync("y = f(5)");
Console.WriteLine(engine.Variables["y"]);   // Natural: 25

var snapshot = engine.CaptureState();        // feed a future GUI variables panel
```

See also: [`.github/requirements/Lovelace.Suite.md`](../.github/requirements/Lovelace.Suite.md),
[`.github/requirements/Lovelace.Suite.Arrays.md`](../.github/requirements/Lovelace.Suite.Arrays.md),
[`Lovelace.Array`](../.github/requirements/Lovelace.Array.md).
