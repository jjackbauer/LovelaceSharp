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

## Language (v1)

### Values

| Kind | Payload |
|---|---|
| `Natural` / `Integer` / `Real` | arbitrary-precision numerics; widen `Natural → Integer → Real` |
| `Boolean` | from comparisons and predicates |
| `Text` | strings and interpolated strings |
| `Vector` | numeric list (ranges, list literals); seeds the vector/matrix layer |
| `Function` | first-class function reference |
| `Void` | result of statements that produce no value |

### Statements

`expr`, `name = expr`, `{ … }`, `if (c) { … } else { … }`, `while (c) { … }`,
`for i in range { … }`, `return [expr]`, `break`, `continue`, and
`func name(a, b) { … }` (or `func f(x) = expr`).

### Vectors

- `a..b` and `a..step..b` ranges; `[a, b, c]` list literals.
- `len(v)`, 0-based indexing `v[i]`, and element-wise `+ - * /` (vector∘vector or scalar broadcast).
- A range binds tighter than arithmetic operators, so `1..10 ^ 2` is `(1..10) ^ 2`,
  `2 * 1..5` is `2 * (1..5)`, and `1..n + 1` is `(1..n) + 1` (parenthesize an endpoint to
  change that, e.g. `1..(n + 1)`).

### Strings and output

- `"plain"` and `$"interpolated {expr}"` (with `{{` / `}}` escapes).
- `print(expr)` writes the rendered value to the engine's `Output` writer.

### Plotting

`plot(y)`, `plot(x, y)`, or `plot(x, y, "title")` builds a `PlotModel` and renders it to a
deterministic SVG file via `SvgPlotRenderer`; the returned `Text` value is the output path.

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

See also: [`.github/requirements/Lovelace.Suite.md`](../.github/requirements/Lovelace.Suite.md).
