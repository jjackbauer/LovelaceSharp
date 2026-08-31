# LovelaceSharp

> Arbitrary-precision arithmetic, end to end: a .NET library, an interactive calculator, and a
> Lean proof that the digits are actually right.

Named after [Ada Lovelace](https://en.wikipedia.org/wiki/Ada_Lovelace), the first programmer,
LovelaceSharp computes with numbers of *any* size. No `long`, no `double`, no fixed precision —
unless you ask for it.

---

## Try it — the REPL

The fastest way to meet the library is the interactive calculator:

```bash
dotnet run --project Lovelace.Console
```

```
LovelaceSharp REPL v1.0.0
Arbitrary-precision arithmetic calculator.
Type 'help' for a list of operators, functions, and commands.

> 42
= 42 (Natural)
> x = 3.14
= 3.14 (Real)
> x * 2
= 6.28 (Real)
> 1 / 3
= 0.(3) (Real)        # exact periodic fraction, not 0.3333333...
> sqrt(2)
= 1.4142135623… (Real) # as many digits as you want
> pi(100)
= 3.1415926535… (Real)
> 5!
= 120 (Natural)
> divrem(17, 5)
= quotient = 3, remainder = 2
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

It is a full scripting engine, not just a calculator: variables (`x = 3.14`, and `_` always
holds the last result), operators `+ - * / % ^ ! == != > < >= <=`, statements (`if`, `while`,
`for … in …`, `return`), user-defined functions (`func`), vectors and ranges (`1..10`, `[1, 2, 3]`
with element-wise arithmetic and 0-based indexing), string interpolation (`$"… {expr} …"`),
`print`, and 2D plotting (`plot(x, y)` → SVG). Built-ins include `abs`, `inv`, `divrem`,
`is_even`, `is_odd`, `sign`, `sqrt`, `pi`, `len`. Full details in
[`Lovelace.Suite/README.md`](Lovelace.Suite/README.md); the complete, machine-checked syntax
reference is [`Lovelace.Suite/docs/Language.md`](Lovelace.Suite/docs/Language.md).

---

## Try it — the web IDE

A browser IDE (script editor, variables/functions workspace, inline SVG plots, and a logs bar)
over the same engine:

```bash
make studio            # or: dotnet run --project Lovelace.Studio
```

Open the printed localhost URL (default `http://localhost:5000`). It is a local, single-user
tool that intentionally runs arbitrary scripts. See
[`Lovelace.Studio/README.md`](Lovelace.Studio/README.md).

---

## Try it — the DSH harness

A DeepSeek Harness (DSH) `lovelace` tool over the same engine, for authoring scripts from
an agent conversation (results, variables, and plots come back as JSON):

```bash
make runner     # publish Lovelace.Run first
```

Then load the dynamic plugin in [`harness/lovelace.host.js`](harness/lovelace.host.js) — full
steps in [`harness/README.md`](harness/README.md).

---

## The numbers

Three arbitrary-precision types, each implementing the relevant `System.Numerics` generic-math
interfaces:

| Type | Domain | Highlights |
|---|---|---|
| `Natural` | ℕ₀ (≥ 0) | digit-by-digit `+ − × ÷`, `DivRem`, binary `Pow`, parallel `Factorial` |
| `Integer` | ℤ | sign + magnitude, signed `DivRem`, `Pow`, `Factorial` |
| `Real` | ℝ | arbitrary precision + **exact periodic fractions**, `Sqrt`, `Pi` |

**Why that is fun:**

- `1 / 3` is *exactly* `0.(3)`. Division detects the repeating block and stores it compactly
  instead of rounding.
- `sqrt(2)` and `π` go to any number of digits — Newton–Raphson and the Chudnovsky algorithm,
  both parallelized.
- `100000!` works. It just takes a moment.

---

## Architecture

Four numeric library projects, each built on the one below it, plus the script engine and two front-ends (a REPL and a web IDE):

```
Lovelace.Representation ← Lovelace.Natural ← Lovelace.Integer ← Lovelace.Real
                                                                        ↑
                                                               Lovelace.Suite (script engine)
                                                                        ↑
                                                               ┌────────┴────────┐
                                                               ↓                 ↓
                                                     Lovelace.Console      Lovelace.Studio
                                                        (REPL)             (web IDE)
```

| Project | Responsibility |
|---|---|
| `Lovelace.Representation` | `DigitStore` — the only project that touches the raw BCD `byte[]` (two decimal digits per byte). |
| `Lovelace.Natural` | Arbitrary-precision naturals. |
| `Lovelace.Integer` | Signed integers on top of `Natural`. |
| `Lovelace.Real` | Reals on top of `Integer` (decimal exponent + period metadata). |
| `Lovelace.Suite` | The scripting engine: tokenizer → parser → interpreter, the `SuiteEngine` introspection API, vectors, and SVG plotting. |
| `Lovelace.Console` | The interactive REPL front-end over `Lovelace.Suite`. |
| `Lovelace.Studio` | A browser IDE over `Lovelace.Suite`: editor, variables/functions workspace, inline SVG plots, and a logs bar. |
| `Lovelace.Run` | A non-interactive script runner over `Lovelace.Suite` that emits a JSON envelope; the engine behind the DSH `lovelace` tool in [`harness/`](harness/README.md). |

Each library project has a matching `*.Tests` project (xUnit).

---

## Formal proofs (Lean)

The digit-by-digit algorithms are not just tested — they are **proved**.
[`Lovelace.Proofs/`](Lovelace.Proofs/) is a Lean 4 formalization (core-only, no Mathlib) of the
schoolbook base-`b` arithmetic in `White Paper.pdf`, stated over `Nat`:

- **Representation** — digit expansion, round-trip, and uniqueness.
- **Addition** — carry-propagating digit addition.
- **Subtraction** — borrow-propagating digit subtraction.
- **Multiplication** — convolution (Cauchy product) plus carry.
- **Division** — digit-by-digit long division with a bounded running remainder.

The named theorems are in [`Lovelace.Proofs/README.md`](Lovelace.Proofs/README.md).

---

## Requirements & Status

| Project | Requirements doc | Status |
|---|---|---|
| `Lovelace.Representation` | [`.github/requirements/Lovelace.Representation.md`](.github/requirements/Lovelace.Representation.md) | ✅ Complete |
| `Lovelace.Natural` | [`.github/requirements/Lovelace.Natural.md`](.github/requirements/Lovelace.Natural.md) | ✅ Complete |
| `Lovelace.Integer` | [`.github/requirements/Lovelace.Integer.md`](.github/requirements/Lovelace.Integer.md) | ✅ Complete |
| `Lovelace.Real` | [`.github/requirements/Lovelace.Real.md`](.github/requirements/Lovelace.Real.md) | ✅ Complete |
| `Lovelace.Real` — Sqrt | [`.github/requirements/Lovelace.Real.Sqrt.md`](.github/requirements/Lovelace.Real.Sqrt.md) | ✅ Complete |
| `Lovelace.Real` — Pi | [`.github/requirements/Lovelace.Real.Pi.md`](.github/requirements/Lovelace.Real.Pi.md) | ✅ Complete |
| `Lovelace.Real` — Sqrt Redesign | [`.github/requirements/Lovelace.Real.Sqrt-Redesign.md`](.github/requirements/Lovelace.Real.Sqrt-Redesign.md) | ✅ Complete |
| `Lovelace.Console` | [`.github/requirements/Lovelace.Console.md`](.github/requirements/Lovelace.Console.md) | ✅ Complete |
| `Lovelace.Suite` | [`.github/requirements/Lovelace.Suite.md`](.github/requirements/Lovelace.Suite.md) | ✅ Complete |
| `Lovelace.Proofs` | [`.github/requirements/Lovelace.Proofs.md`](.github/requirements/Lovelace.Proofs.md) | ✅ Complete |
| `Lovelace.Proofs` — Division | [`.github/requirements/Lovelace.Proofs.Division.md`](.github/requirements/Lovelace.Proofs.Division.md) | ✅ Complete |
| `Lovelace.Studio` | [`.github/requirements/Lovelace.Studio.md`](.github/requirements/Lovelace.Studio.md) | ✅ Complete |

Engineering notes (parallelization, investigations):
[Representation audit](.github/requirements/Lovelace.Representation-parallelization-audit.md) ·
[Natural audit](.github/requirements/Lovelace.Natural-parallelization-audit.md) ·
[Real parallelism](.github/requirements/Lovelace.Real.Parallelism.md) ·
[Sqrt investigation](.github/requirements/Lovelace.Real.Sqrt.investigation.md).

---

## Project structure

```
LovelaceSharp.slnx
├── Legacy/                              # Original C++ source (reference only)
│   ├── Lovelace.hpp / .cpp              # BCD store + natural arithmetic
│   ├── InteiroLovelace.hpp / .cpp       # Signed integers
│   ├── RealLovelace.hpp / .cpp          # Real numbers
│   ├── VetorLovelace.hpp / .cpp         # Arbitrary-precision vector (not yet migrated)
│   └── VetorMultidimensionalLovelace.*  # Multi-dimensional array (not yet migrated)
│
├── Lovelace.Representation/             # BCD digit store (DigitStore)
├── Lovelace.Representation.Tests/
│
├── Lovelace.Natural/                    # Natural numbers (Natural)
├── Lovelace.Natural.Tests/
│
├── Lovelace.Integer/                    # Signed integers (Integer)
├── Lovelace.Integer.Tests/
│
├── Lovelace.Suite/                      # Scripting engine: interpreter, introspection API, vectors, plotting
├── Lovelace.Suite.Tests/
│
├── Lovelace.Console/                    # Interactive REPL front-end
│
├── Lovelace.Studio/                     # Browser IDE (ASP.NET Core minimal API + wwwroot)
├── Lovelace.Studio.Tests/
│
├── Lovelace.Run/                        # Non-interactive JSON script runner
│
├── harness/                             # DSH plugin source + docs (the `lovelace` tool)
│
├── Lovelace.Real/                       # Real numbers (Real)
├── Lovelace.Real.Tests/
│
└── Lovelace.Proofs/                     # Lean formal proofs (representation → division)
```

---

## Building

Requires the [.NET 10 SDK](https://dotnet.microsoft.com/download/dotnet/10.0).

```bash
dotnet build          # build the whole solution
make studio           # build + run the Lovelace.Studio web IDE
make runner           # publish the non-interactive Lovelace.Run script runner
make test             # run the fast test suites
```

A [`Makefile`](Makefile) wraps the common commands: `make build` (publish the console app),
`make run` (run the published console binary), `make runner` (publish the script runner),
`make studio` (build + run the web IDE), `make test` (fast test suites), `make clean`, and `make help`.

The Lean proofs are a separate toolchain (Lean 4.33.1, core-only):

```bash
cd Lovelace.Proofs
lake build
```

---

## Testing

```bash
# Run all tests
dotnet test

# Run a specific project
dotnet test Lovelace.Representation.Tests/
dotnet test Lovelace.Natural.Tests/
```

Test naming convention: `MethodName_GivenScenario_ExpectedResult`.

---

## Legacy migration

The C# codebase is a class-by-class migration of the C++ `Legacy/` source (originally written in
Portuguese; identifiers are English here, using .NET conventions). Key reference documents live in
[`.github/`](.github/):

| File | Purpose |
|---|---|
| [`.github/prompts/legacy-knowledge-map.md`](.github/prompts/legacy-knowledge-map.md) | Portuguese → English method-name mapping and representation contract |
| [`.github/prompts/skill-impl-completeness.prompt.md`](.github/prompts/skill-impl-completeness.prompt.md) | Audit a C++ class against its C# counterpart |
| [`.github/prompts/skill-test-standards.prompt.md`](.github/prompts/skill-test-standards.prompt.md) | Generate an xUnit test plan for a method |
| [`.github/prompts/skill-falsify-claims.prompt.md`](.github/prompts/skill-falsify-claims.prompt.md) | Verify or refute claims against the legacy source |
| [`.github/prompts/workflow-requirements-gathering.prompt.md`](.github/prompts/workflow-requirements-gathering.prompt.md) | Produce a checklist and test plan for a whole class |
| [`.github/prompts/workflow-iterative-implementation.prompt.md`](.github/prompts/workflow-iterative-implementation.prompt.md) | Implement one checklist item end-to-end |

---

## License

See [LICENSE](LICENSE) if present, or contact the repository owner.
