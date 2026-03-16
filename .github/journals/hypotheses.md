# Hypotheses

> **Journal file**: Testable claims derived from observations.
> **Schema**: See `.github/prompts/journal-schema.md` §4.2 for the HYP entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.
> Status updates (Proposed → Under review → Supported/Falsified/Superseded) are made in-place.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### HYP-{NNN}: {Short title}

- **Claim**: {Testable assertion about system behaviour or structure}
- **Supporting OBS**: {Comma-separated OBS IDs that motivate this hypothesis}
- **Why it matters**: {Impact on architecture, migration, or correctness if true/false}
- **Falsification strategy**: {Specific steps to disprove this claim — what to look for and where}
- **Status**: {Proposed | Under review | Supported | Falsified | Superseded}
- **Confidence**: {High | Medium | Low}
-->

---

### HYP-001: No code outside Lovelace.Representation directly accesses the byte[] backing store

- **Claim**: No C# project other than `Lovelace.Representation` reads or writes the private `_bytes` field or calls the `GetBitwise`/`SetBitwise` internal methods. All upper layers use only `GetDigit`/`SetDigit` or the snapshot APIs.
- **Supporting OBS**: OBS-001, OBS-003, OBS-006
- **Why it matters**: This is the fundamental encapsulation invariant of the architecture. If violated, BCD packing changes could break upper layers silently.
- **Falsification strategy**: Search all `.cs` files in `Lovelace.Natural/`, `Lovelace.Integer/`, `Lovelace.Real/`, and `Lovelace.Console/` for references to `_bytes`, `GetBitwise`, or `SetBitwise`. Also search for `List<byte>` field declarations outside DigitStore.cs. Any match is a counterexample.
- **Status**: Supported
- **Confidence**: High

---

### HYP-002: Integer is intentionally not sealed to allow Real inheritance

- **Claim**: The `Integer` class is declared as `public class` (not `sealed`) specifically because `Real` extends it via `public class Real : Int`. No other class in the solution inherits from Integer.
- **Supporting OBS**: OBS-008, OBS-011
- **Why it matters**: If Integer were sealed, the current Real implementation would not compile. If other classes also inherit from Integer, the design might have unintended extension points.
- **Falsification strategy**: Search all `.cs` files in the solution for classes that inherit from `Integer` (pattern `: Integer` or `: Int` where `Int` is the alias). Expect to find exactly one: `Real`. Any additional subclass falsifies the "only Real inherits" part.
- **Status**: Supported
- **Confidence**: High

---

### HYP-003: All arithmetic operators return new instances — no mutation of operands

- **Claim**: Every arithmetic operator (`+`, `-`, `*`, `/`, `%`) on Natural, Integer, and Real creates and returns a new instance rather than mutating the left or right operand.
- **Supporting OBS**: OBS-007, OBS-010, OBS-011
- **Why it matters**: If operators mutated operands, concurrent computations could produce incorrect results and repeated use of a value in expressions would be unsafe.
- **Falsification strategy**: In each arithmetic operator method body for Natural, Integer, and Real, check whether `left._store` or `right._store` (or `left._magnitude` / `right._magnitude`) is mutated. Look for `SetDigit` calls on the input parameters rather than a fresh local variable. Any mutation of an input operand is a counterexample.
- **Status**: Supported
- **Confidence**: High

---

### HYP-004: Generic type conversion methods are all stubs

- **Claim**: The six `TryConvertFrom*/TryConvertTo*` methods required by `INumberBase<T>` all return `false` or throw `NotImplementedException` in Natural, Integer, and Real — none performs an actual type conversion.
- **Supporting OBS**: OBS-019
- **Why it matters**: If any conversion actually works, cross-type generic math operations (e.g., `T.CreateChecked<int>(42)`) might succeed for some types and fail for others, creating inconsistent behavior.
- **Falsification strategy**: Read the bodies of all six `TryConvert*` methods in Natural.cs, Integer.cs, and Real.cs. Any method that returns `true` and populates the `out` parameter with a valid value is a counterexample.
- **Status**: Supported
- **Confidence**: High

---

### HYP-005: Natural subtraction does not auto-widen — it throws on underflow

- **Claim**: `Natural.operator-(Natural left, Natural right)` throws `InvalidOperationException` when `right > left`, rather than returning an Integer or any other type. The auto-widening behavior described in OBS-017 is implemented only in the Console Evaluator, not in the Natural type itself.
- **Supporting OBS**: OBS-007, OBS-017
- **Why it matters**: Clarifies the boundary between library-level behavior (strict type, throws) and application-level behavior (REPL widens). Misunderstanding this could lead to incorrect assumptions about Natural's contract.
- **Falsification strategy**: Read the body of `operator-(Natural left, Natural right)` in Natural.cs. If it catches the underflow and returns an Integer or any non-Natural type, the claim is falsified. Also check if the return type is anything other than `Natural`.
- **Status**: Supported
- **Confidence**: High

---

### HYP-006: All glossary ⚠️ class and method mappings are accurate

- **Claim**: Every glossary entry marked ⚠️ (Tentative) — class-level names, method-level names, BCD vocabulary, static members, and dependency-chain vocabulary — correctly maps the Portuguese C++ identifier to the English C# equivalent, and the C# method/class exists at the documented location.
- **Supporting OBS**: OBS-021, OBS-022, OBS-023, OBS-024, OBS-025, OBS-026, OBS-027
- **Why it matters**: If any mapping is incorrect, developers consulting the glossary would write code with wrong method names or misunderstand the architecture.
- **Falsification strategy**: For each ⚠️ entry, search the corresponding C# file for the claimed English name (class, method, property, or operator). Any missing match or name mismatch is a counterexample.
- **Status**: Supported
- **Confidence**: High

---

### HYP-007: All glossary ❓ Legacy-only terms exist in C++ source as described

- **Claim**: Every glossary entry marked ❓ (Unverified) — `VetorLovelace`, `VetorMultidimensionalLovelace`, `multiplicar_burro`, `dividir_burro`, `inverter`, `imprimirInfo`, `expandirAlgarismos`, `reduzirAlgarismos`, `TabelaDeConversao`, `toInteiroLovelace`, and sentinels 0x0C/0x0F — exists in the C++ Legacy source with the described semantics. The C# equivalents (where they exist) match the documented names.
- **Supporting OBS**: OBS-026, OBS-027
- **Why it matters**: If Legacy terms are incorrectly described, future migration work would be based on wrong assumptions.
- **Falsification strategy**: Read each Legacy .cpp/.hpp file for the claimed method/variable. Check that: (a) the name exists, (b) the described behavior matches the implementation. Any mismatch or missing declaration is a counterexample.
- **Status**: Supported
- **Confidence**: High

---

### HYP-008: VetorLovelace and VetorMultidimensionalLovelace are intentionally out of scope for the current migration

- **Claim**: The unmigrated `VetorLovelace` and `VetorMuldimensionalLovelace` C++ classes are a known, intentional scope exclusion — no C# project for either class is planned in the current solution.
- **Supporting OBS**: OBS-034, OBS-026
- **Why it matters**: If the vector classes are in scope, the migration is incomplete and requires additional C# projects. If they are intentionally excluded, the solution structure is considered complete for the planned migration.
- **Falsification strategy**: Search the `.slnx` solution file for any project named `Lovelace.Vector*` or similar. Search for any folder under the workspace root matching `Vector*` or `Vetor*`. Check `todos.md` and `risks.md` for any entry explicitly mentioning vector class migration. Any such evidence falsifies "intentionally excluded."
- **Status**: Supported
- **Confidence**: Medium

---

### HYP-009: Real.Pow non-integer/negative stubs are unblocked — missing implementation effort only

- **Claim**: The two `NotImplementedException` branches in `Real.Pow` (non-integer exponents, negative exponents) are not blocked by any missing technical dependency — they are implement-on-demand stubs that could be completed without upstream changes to Natural, Integer, or DigitStore.
- **Supporting OBS**: OBS-036, OBS-011, OBS-013
- **Why it matters**: If there is a hidden technical dependency (e.g., Real logarithm is needed), the stubs cannot be closed without more design work. If they are pure effort stubs, they can be prioritised as implementation tasks.
- **Falsification strategy**: Read the full body of `Real.Pow` in Real.cs. If the `NotImplementedException` messages or surrounding comments call out a specific missing component (e.g., "requires Real.Log"), the claim is falsified. Also check whether the `Natural.Pow` path (integer exponents) delegates in a way that would naturally extend to real exponents.
- **Status**: Supported
- **Confidence**: Medium
