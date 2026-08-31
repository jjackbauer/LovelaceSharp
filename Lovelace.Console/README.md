# Lovelace.Console

The interactive REPL front-end for LovelaceSharp. All language logic (tokenizer, parser,
interpreter, vectors, plotting) lives in [`Lovelace.Suite`](../Lovelace.Suite/README.md); this
project only handles interactive I/O and command dispatch over the `SuiteEngine` façade.

---

## Architecture

```
LineEditor  (Console.ReadKey line editing + history)
     │
ReplSession  (multi-line accumulation → SuiteEngine.EvaluateAsync → print)
     │
SuiteEngine  (Lovelace.Suite — the scripting engine)
     │
Program  (entry point)
```

`ReplSession` and `LineEditor` are I/O-dependent and verified by manual acceptance scenarios.

---

## Class: `LineEditor`

**Namespace:** `Lovelace.Console.Repl`

Console line editor using `Console.ReadKey(intercept: true)` for full cursor control and command
history.

| Key | Action |
|---|---|
| Printable char | Insert at cursor |
| Backspace / Delete | Delete before / at cursor |
| Left / Right | Move cursor |
| Home / End | Jump to start / end |
| Up / Down | Navigate history |
| Ctrl+C | Return `null` (exit signal) |
| Enter | Submit line |

---

## Class: `ReplSession`

**Namespace:** `Lovelace.Console.Repl`

Orchestrates the REPL over a `SuiteEngine`: reads input, accumulates continuation lines while
braces are unbalanced (for multi-line functions and blocks), dispatches special commands, and
prints results via `ValueFormatter.FormatTyped`.

### Special commands

| Command | Description |
|---|---|
| `help` | Print statements, operators, functions, and commands. |
| `vars` | List all variables with types and values. |
| `funcs` | List all functions (user + built-in). |
| `clear` | Delete all variables (functions remain). |
| `delete <name>` | Remove one variable. |
| `run <file>` | Execute a script file. |
| `set precision <n>` | Set `Real.MaxComputationDecimalPlaces`. |
| `set display <n>` | Set `Real.DisplayDecimalPlaces` and `Natural.DisplayDigits`. |
| `exit` / `quit` | Terminate the REPL. |

The `_` (last result) variable is maintained by the engine: after each successful non-void
evaluation the result is stored in `_`.

---

## Usage

```bash
dotnet run --project Lovelace.Console
```

Sample session:

```
LovelaceSharp REPL v1.0.0
Arbitrary-precision math scripting, vector math, and plotting.
Type 'help' for a list of statements, operators, functions, and commands.

> func square(x) = x ^ 2
> square(5)
= 25 (Natural)
> 1..5
= [1, 2, 3, 4, 5] (Vector)
> plot(1..5, [1, 4, 9, 16, 25], "squares")
= C:\…\plot.svg (Text)
> exit
Bye!
```

---

## See also

- Language & engine: [`Lovelace.Suite/README.md`](../Lovelace.Suite/README.md)
- Requirements: [`.github/requirements/Lovelace.Suite.md`](../.github/requirements/Lovelace.Suite.md)
- Numeric libraries: [`Lovelace.Natural`](../Lovelace.Natural/README.md),
  [`Lovelace.Integer`](../Lovelace.Integer/README.md), [`Lovelace.Real`](../Lovelace.Real/README.md)
