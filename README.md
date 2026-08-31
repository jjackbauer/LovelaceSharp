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
> exit
Bye!
```

It is a full expression evaluator: variables (`x = 3.14`, and `_` always holds the last result),
operators `+ - * / % ^ ! == != > < >= <=`, and built-ins `abs`, `inv`, `divrem`, `is_even`,
`is_odd`, `sign`, `sqrt`, `pi`. Full details in
[`Lovelace.Console/README.md`](Lovelace.Console/README.md).

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

Four focused projects, each built on the one below it, plus an interactive front-end:

```
Lovelace.Representation ← Lovelace.Natural ← Lovelace.Integer ← Lovelace.Real
                                                                        ↑
                                                             Lovelace.Console (REPL)
```

| Project | Responsibility |
|---|---|
| `Lovelace.Representation` | `DigitStore` — the only project that touches the raw BCD `byte[]` (two decimal digits per byte). |
| `Lovelace.Natural` | Arbitrary-precision naturals. |
| `Lovelace.Integer` | Signed integers on top of `Natural`. |
| `Lovelace.Real` | Reals on top of `Integer` (decimal exponent + period metadata). |
| `Lovelace.Console` | The interactive REPL (tokenizer → parser → evaluator). |

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
| `Lovelace.Proofs` | [`.github/requirements/Lovelace.Proofs.md`](.github/requirements/Lovelace.Proofs.md) | ✅ Complete |
| `Lovelace.Proofs` — Division | [`.github/requirements/Lovelace.Proofs.Division.md`](.github/requirements/Lovelace.Proofs.Division.md) | ✅ Complete |

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
├── Lovelace.Console/                    # Interactive REPL calculator
├── Lovelace.Console.Tests/
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
dotnet build
```

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
