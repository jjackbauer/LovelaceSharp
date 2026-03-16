# LovelaceSharp — Workspace Context for GitHub Copilot

## Project Purpose

LovelaceSharp is an arbitrary-precision number library being migrated from a C++ implementation (`Legacy/`) to idiomatic .NET 10 C# with xUnit tests. The migration is class-by-class; each C# project corresponds to exactly one C++ class.

## C# Project Responsibilities

| C# Project | Maps from C++ | Role |
|---|---|---|
| `Lovelace.Representation` | `Lovelace` (digit storage layer) | **Internal bitwise digit store.** Packs two decimal digits per `byte` (BCD). Exposes only `GetDigit(long position)` and `SetDigit(long position, byte digit)`. No other project accesses the raw `byte[]` directly. |
| `Lovelace.Natural` | `Lovelace` (arithmetic layer) | Arbitrary-precision natural numbers (≥ 0). Depends on `Lovelace.Representation`. |
| `Lovelace.Integer` | `InteiroLovelace` | Signed arbitrary-precision integers. Adds a sign bit on top of `Lovelace.Natural`. |
| `Lovelace.Real` | `RealLovelace` | Arbitrary-precision fixed-point/floating-point real numbers. Adds a decimal exponent on top of `Lovelace.Integer`. |

## Dependency Chain

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
```

## Key Architectural Rules

1. **`Lovelace.Representation` is the only project that may read or write the backing `byte[]`.**  
   All upper-layer classes call `GetDigit`/`SetDigit` and never touch raw bytes.

2. **BCD packing**: two decimal digits per byte — high nibble = even-indexed digit, low nibble = odd-indexed digit (mirrors `getBitwise`/`setBitwise` in C++).

3. **Naming**: all C# identifiers use English, following .NET conventions (`PascalCase` for public members).  
   See `.github/prompts/legacy-knowledge-map.md` for the full Portuguese → English translation table.

4. **Testing**: xUnit only; test naming convention is `MethodName_GivenScenario_ExpectedResult`.

5. **Interfaces**: C# types should implement the appropriate `System.Numerics` generic math interfaces (`INumber<T>`, `IComparable<T>`, `IEquatable<T>`, `IParsable<T>`, `ISpanFormattable`, etc.) where they apply.

## When Working on Migration Tasks

Always load the legacy knowledge map and codebase patterns reference before writing code or tests:

```
#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/codebase-patterns.md
```

## Available Prompts and Workflows

| File | Purpose |
|---|---|
| `.github/prompts/legacy-knowledge-map.md` | Reference: class/method mapping and representation contract |
| `.github/prompts/skill-falsify-claims.prompt.md` | Skill: verify or refute specific claims against source |
| `.github/prompts/skill-test-standards.prompt.md` | Skill: generate xUnit test plan for a method |
| `.github/prompts/skill-impl-completeness.prompt.md` | Skill: audit C++ class against C# counterpart |
| `.github/prompts/workflow-requirements-gathering.prompt.md` | Workflow: generic — produce a requirements checklist and xUnit test plan for any C# project, driven by a pluggable analysis source and mandatory-item rules |
| `.github/prompts/rule-migration.prompt.md` | Rule: migration-specific pre-configuration of the requirements-gathering workflow for C++ → C# class migration |
| `.github/prompts/workflow-iterative-implementation.prompt.md` | Workflow: implement one checklist item end-to-end |
| `.github/prompts/codebase-patterns.md` | Reference: implementation and test-writing conventions distilled from the codebase |
| `.github/prompts/skill-codebase-patterns.prompt.md` | Skill: regenerate/update the codebase patterns reference by re-analyzing source files |
| `.github/prompts/skill-plan-format-gate.prompt.md` | Skill: validate plan document structure; auto-invoked by the requirements-gathering workflow and self-heals violations autonomously |

## Journal-Driven Codebase Distillation System

An incremental, evidence-grounded system for exploring the codebase, recording findings, validating
hypotheses, and synthesising trustworthy downstream artifacts (architecture maps, migration plans,
risk assessments). All files fit the existing `.github/prompts/` skill/workflow/rule/reference taxonomy.

### Entry Points

| Usage | How to invoke |
|---|---|
| **Architecture documentation** | Open `#file:.github/prompts/rule-architecture-analysis.prompt.md` and supply an optional `Scope`. |
| **Migration gap analysis** | Open `#file:.github/prompts/rule-migration-analysis.prompt.md` and supply an optional `CppClass`. |
| **Single observation session** | Open `#file:.github/prompts/skill-journal-observe.prompt.md` with a narrow `Objective`. |
| **Manual exploration cycle** | Open `#file:.github/prompts/workflow-codebase-exploration.prompt.md` with an `Objective`. |

### Lightweight vs. Heavyweight Usage

- **Lightweight** (quick exploration, no distillation needed): invoke `skill-journal-observe` only.
  Produces OBS entries. Skip Steps 4–7 of the workflow. Useful when you need a fast factual snapshot
  of one file or method without updating distilled documents.

- **Heavyweight** (full cycle, distillation + metrics): invoke `workflow-codebase-exploration` (or a
  rule that wraps it). Runs all 7 steps: observe → hypothesize → validate → assess → distill → metrics.
  Required before generating downstream artifacts (architecture reports, migration plans).

### Prompts

| File | Type | Purpose |
|---|---|---|
| `.github/prompts/journal-schema.md` | Reference | Entry templates, ID format, confidence levels for all journal files |
| `.github/prompts/distilled-knowledge-schema.md` | Reference | Header template, uncertainty markers (✅⚠️❓), update criteria for distilled docs |
| `.github/prompts/skill-journal-observe.prompt.md` | Skill | Explorer — read source, produce grounded OBS entries |
| `.github/prompts/skill-journal-hypothesize.prompt.md` | Skill | Theorist — derive testable HYP entries with falsification strategies |
| `.github/prompts/skill-journal-validate.prompt.md` | Skill | Skeptic — falsify HYP entries, produce VAL entries, update HYP status |
| `.github/prompts/skill-journal-distill.prompt.md` | Skill | Synthesizer — promote validated findings to distilled knowledge documents |
| `.github/prompts/skill-completeness-review.prompt.md` | Skill | Reviewer — audit 12 coverage dimensions, produce OQ and TODO entries |
| `.github/prompts/skill-convergence-metrics.prompt.md` | Skill | Metrics — compute all convergence metrics and evaluate stopping criteria |
| `.github/prompts/workflow-codebase-exploration.prompt.md` | Workflow | Orchestrates one full 7-step exploration cycle |
| `.github/prompts/rule-architecture-analysis.prompt.md` | Rule | Pre-configures the workflow for architecture/design documentation |
| `.github/prompts/rule-migration-analysis.prompt.md` | Rule | Pre-configures the workflow for C++ → C# migration analysis |

### Journals (`.github/journals/`)

Append-only evidence store. Never edit past entries.

| File | Entry type | Purpose |
|---|---|---|
| `observations.md` | OBS | Grounded factual findings from source code |
| `hypotheses.md` | HYP | Testable claims with falsification strategies |
| `validations.md` | VAL | Falsification results — Supported, Falsified, or Unresolved |
| `decisions.md` | DEC | Architectural and synthesis decisions with rationale |
| `todos.md` | TODO | Actionable exploration and implementation tasks |
| `risks.md` | RISK | Identified risks with likelihood, impact, and mitigation |
| `open-questions.md` | OQ | Unresolved questions requiring further investigation |
| `artifact-index.md` | ART | Registry of downstream artifacts linked to their evidence |

### Distilled Knowledge (`.github/distilled/`)

Synthesised, curated documents derived from journal entries. Updated by `skill-journal-distill`.

| File | Scope |
|---|---|
| `system-overview.md` | High-level architecture and project structure |
| `module-map.md` | Per-project responsibilities and inter-module boundaries |
| `domain-concepts.md` | BCD packing, periodic decimals, exponent model |
| `runtime-flows.md` | Key execution paths (parse→compute→format, etc.) |
| `dependencies.md` | Inter-project and external dependencies |
| `invariants-and-risks.md` | Architectural invariants and known risks |
| `migration-findings.md` | C++ → C# migration decisions and lessons |
| `trusted-facts.md` | High-confidence claims (✅ markers only) |
| `unresolved-areas.md` | Gaps, weak evidence, and open questions |
| `glossary.md` | Domain terms (Portuguese → English, BCD terminology) |

### State (`.github/state/`)

| File | Purpose |
|---|---|
| `convergence-metrics.md` | Quantitative snapshot of exploration depth and stopping-criteria status |

### Artifacts (`.github/artifacts/`)

Downstream generated documents (architecture reports, migration plans, risk assessments) produced
from distilled knowledge when confidence is sufficient.
