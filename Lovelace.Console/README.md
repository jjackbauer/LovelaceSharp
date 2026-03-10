# Lovelace.Console

An interactive REPL calculator built on `Lovelace.Natural`, `Lovelace.Integer`, and `Lovelace.Real`. It supports variable assignment, arithmetic expressions, built-in functions, and all three numeric types, choosing the narrowest type at parse time and widening automatically during operations.

---

## Architecture

```
Input line
    Tokenizer
List<Token>
    Parser
Expr AST
    Evaluator
Value
    ReplSession
Formatted output
```

Pure-logic components (`Value`, `Tokenizer`, `Parser`, `Evaluator`) are covered by 133 xUnit tests in `Lovelace.Console.Tests`.  
I/O-dependent components (`LineEditor`, `ReplSession`, `Program`) are verified by manual acceptance scenarios.

---

## Class: `Value`

**Namespace:** `Lovelace.Console.Repl`

A type-discriminated wrapper that holds one of `Natural`, `Integer`, `Real`, `bool`, or a pre-formatted `string`, together with a `ValueKind` tag.  
The three numeric kinds form a widening chain: `Natural  Integer  Real`.

### `ValueKind` enum

| Member | Description |
|---|---|
| `Natural` | Arbitrary-precision natural number ( 0) |
| `Integer` | Signed arbitrary-precision integer |
| `Real` | Arbitrary-precision fixed-point real |
| `Boolean` | Boolean result (from comparisons and `is_even`/`is_odd`) |
| `Text` | Pre-formatted string (from `divrem`) |

### `Value` public API

| Member | Signature | Description |
|---|---|---|
| Constructor | `Value(Natural)` | Wraps a `Natural`; sets `Kind = ValueKind.Natural`. |
| Constructor | `Value(Integer)` | Wraps an `Integer`; sets `Kind = ValueKind.Integer`. |
| Constructor | `Value(Real)` | Wraps a `Real`; sets `Kind = ValueKind.Real`. |
| Constructor | `Value(bool)` | Wraps a `bool`; sets `Kind = ValueKind.Boolean`. |
| Constructor | `Value(string)` | Wraps a pre-formatted text result; sets `Kind = ValueKind.Text`. |
| `Kind` | `ValueKind Kind { get; }` | The kind tag. |
| `AsNatural()` | `Natural AsNatural()` | Returns the inner `Natural` (throws if kind is wrong). |
| `AsInteger()` | `Integer AsInteger()` | Returns the inner `Integer`. |
| `AsReal()` | `Real AsReal()` | Returns the inner `Real`. |
| `AsBoolean()` | `bool AsBoolean()` | Returns the inner `bool`. |
| `AsText()` | `string AsText()` | Returns the inner `string`. |
| `Widen` | `Value Widen(ValueKind target)` | Promotes along `Natural  Integer  Real`. Same kind returns `this`. Narrowing throws. |
| `WidenPair` | `static (Value, Value) WidenPair(Value a, Value b)` | Brings both operands to `max(a.Kind, b.Kind)`. |
| `ToString()` | `string ToString()` | Returns `"Kind: value"` (e.g. `"Natural: 42"`). |

---

## Classes: `TokenKind` / `Token`

**Namespace:** `Lovelace.Console.Repl`  
**File:** `Repl/Token.cs`

`TokenKind` is an enum with 20 members: `NumberLiteral`, `Identifier`, `Plus`, `Minus`, `Star`, `Slash`, `Percent`, `Caret`, `Bang`, `Equals`, `DoubleEquals`, `BangEquals`, `Greater`, `Less`, `GreaterEquals`, `LessEquals`, `LParen`, `RParen`, `Comma`, `Eof`.

`Token` is a record: `TokenKind Kind`, `string Text`, `int Position`.

---

## Class: `Tokenizer`

**Namespace:** `Lovelace.Console.Repl`

Scans a string into a `List<Token>`. Supports:
- **Number literals**: digits, optional `.`, optional decimal digits, optional `(digits)` periodic suffix (e.g. `0.(3)`, `1.2(345)`).
- **Identifiers**: `[a-zA-Z_][a-zA-Z0-9_]*`.
- **Operators**: two-character variants (`==`, `!=`, `>=`, `<=`) matched before single-character.
- Skips whitespace; throws `InvalidOperationException` with position for unknown characters.

| Member | Signature | Description |
|---|---|---|
| `Tokenize` | `List<Token> Tokenize(string input)` | Produces a token list ending with an `Eof` token. |

---

## Classes: AST nodes

**Namespace:** `Lovelace.Console.Repl`  
**File:** `Repl/Ast.cs`

| Type | Kind | Properties |
|---|---|---|
| `Expr` | abstract base |  |
| `LiteralExpr` | record | `string RawText` |
| `VariableExpr` | record | `string Name` |
| `AssignExpr` | record | `string Name`, `Expr Value` |
| `BinaryExpr` | record | `Expr Left`, `BinaryOp Op`, `Expr Right` |
| `UnaryExpr` | record | `UnaryOp Op`, `Expr Operand` |
| `PostfixExpr` | record | `Expr Operand`, `PostfixOp Op` |
| `CallExpr` | record | `string FunctionName`, `List<Expr> Arguments` |

**`BinaryOp`**: `Add`, `Subtract`, `Multiply`, `Divide`, `Modulo`, `Power`, `Equal`, `NotEqual`, `Greater`, `Less`, `GreaterEqual`, `LessEqual`.  
**`UnaryOp`**: `Plus`, `Negate`.  
**`PostfixOp`**: `Factorial`.

---

## Class: `Parser`

**Namespace:** `Lovelace.Console.Repl`

Recursive-descent parser. Precedence (low  high):

| Level | Operators | Associativity |
|---|---|---|
| Assignment | `=` | Right |
| Comparison | `== != > < >= <=` | Left |
| Additive | `+ -` | Left |
| Multiplicative | `* / %` | Left |
| Power | `^` | **Right** |
| Unary | `- +` (prefix) | Right |
| Postfix | `!` | Left |
| Primary | literals, identifiers, `(expr)`, calls |  |

| Member | Signature | Description |
|---|---|---|
| `Parse` | `Expr Parse(List<Token> tokens)` | Parses the token list into an `Expr` AST. Throws `InvalidOperationException` with position on syntax error. |

---

## Class: `Evaluator`

**Namespace:** `Lovelace.Console.Repl`

AST walker with a variable store and built-in function registry.

### Variable store

| Member | Signature | Description |
|---|---|---|
| `Variables` | `IReadOnlyDictionary<string, Value> Variables { get; }` | Read-only view of all assigned variables. |
| `Clear()` | `void Clear()` | Removes all variables. |
| `Remove(name)` | `bool Remove(string name)` | Removes one variable; returns `true` if it existed. |

### Evaluation

| Member | Signature | Description |
|---|---|---|
| `Evaluate` | `Value Evaluate(Expr expr)` | Walks the AST and produces a `Value`. |

### Type inference and widening

- Whole-number literals  `Natural`. Literals containing `.` or `(`  `Real`.
- Binary operators widen both operands to `max(left.Kind, right.Kind)` before operating.
- `Natural` subtraction that would produce a negative result auto-widens to `Integer` and retries.
- Unary `-` on `Natural` widens to `Integer` first.
- Comparisons widen the pair and return `Boolean`.

### Built-in functions

| Function | Description |
|---|---|
| `abs(x)` | `Natural.Abs` / `Integer.Abs` / `Real.Abs` depending on kind. |
| `inv(x)` | Widens to `Real`, calls `Real.Invert()` (1/x). Throws `DivideByZeroException` for zero. |
| `divrem(a, b)` | `Natural.DivRem` or `Integer.DivRem`; returns `Text` value `"quotient = Q, remainder = R"`. Not supported for `Real`. |
| `is_even(x)` | Calls `IsEvenInteger` on the appropriate type; returns `Boolean`. |
| `is_odd(x)` | Calls `IsOddInteger` on the appropriate type; returns `Boolean`. |
| `sign(x)` | Widens to at least `Integer`, reads `.Sign`, returns `Integer(-1 / 0 / 1)`. Not supported for `Real`. |
| `sqrt(x)` | Widens the single argument to `Real` and delegates to `Real.Sqrt`. Precision is `Real.MaxComputationDecimalPlaces`. Throws `ArithmeticException` for negative input; `InvalidOperationException` for wrong arity. |
| `pi()` / `pi(digits)` | Computes π via `Real.Pi`. With 0 arguments uses `Real.DisplayDecimalPlaces` as the digit count; with 1 `Natural` or `Integer` argument uses its value. Throws `InvalidOperationException` for `Real` argument or arity > 1. |

---

## Class: `LineEditor`

**Namespace:** `Lovelace.Console.Repl`

Console line editor using `Console.ReadKey(intercept: true)` for full cursor control and command history.

| Member | Signature | Description |
|---|---|---|
| `ReadLine` | `string? ReadLine(string prompt)` | Interactive line reading. Returns the submitted string, or `null` on Ctrl+C. Non-empty lines are appended to history. |

**Key bindings:**

| Key | Action |
|---|---|
| Printable char | Insert at cursor |
| Backspace | Delete character before cursor |
| Delete | Delete character at cursor |
| Left / Right | Move cursor |
| Home / End | Jump to start / end |
| Up / Down | Navigate history |
| Ctrl+C | Return `null` (exit signal) |
| Enter | Submit line |

---

## Class: `ReplSession`

**Namespace:** `Lovelace.Console.Repl`

Orchestrates the REPL: reads a line via `LineEditor`  tokenizes  parses  evaluates  prints the result.

| Member | Signature | Description |
|---|---|---|
| `Run()` | `void Run()` | Starts the REPL loop. Exits on `exit`, `quit`, or Ctrl+C. |

### Result display format

```
= <value> (<Type>)
```

Examples: `= 42 (Natural)`, `= -3 (Integer)`, `= 3.14 (Real)`, `= True (Boolean)`.

### Special commands

| Command | Description |
|---|---|
| `help` | Print available operators, functions, and commands. |
| `vars` | List all assigned variables with types and values. |
| `clear` | Delete all variables (including `_`). |
| `delete <name>` | Remove one variable. |
| `set precision <n>` | Set `Real.MaxComputationDecimalPlaces` to `n`. |
| `set display <n>` | Set `Real.DisplayDecimalPlaces` and `Natural.DisplayDigits` to `n`. |
| `exit` / `quit` | Terminate the REPL. |

### Underscore variable `_`

After each successful evaluation, the result is stored in the variable `_`, so the next expression can reference the previous result.

### Error display

Parse and evaluation errors print the input line, a `^` caret under the error position (when extractable), and the error message.

---

## Usage

```bash
dotnet run --project Lovelace.Console
```

Sample session:

```
LovelaceSharp REPL v1.0.0
Arbitrary-precision arithmetic calculator.
Type ''help'' for a list of operators, functions, and commands.

� 42
= 42 (Natural)
� x = 3.14
= 3.14 (Real)
� x * 2
= 6.28 (Real)
� abs(-5)
= 5 (Integer)
� 5!
= 120 (Natural)
� divrem(17, 5)
= quotient = 3, remainder = 2
� 3 == 3
= True (Boolean)
� vars
  _ = 2 (Natural)
  x = 3.14 (Real)
� exit
Bye!
```

---

## See also

- Requirements: [`.github/requirements/Lovelace.Console.md`](../.github/requirements/Lovelace.Console.md)
- Libraries: [`Lovelace.Natural`](../Lovelace.Natural/README.md), [`Lovelace.Integer`](../Lovelace.Integer/README.md), [`Lovelace.Real`](../Lovelace.Real/README.md)
- Tests: `Lovelace.Console.Tests/`
