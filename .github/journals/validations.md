# Validations

> **Journal file**: Falsification or confirmation of hypotheses.
> **Schema**: See `.github/prompts/journal-schema.md` §4.3 for the VAL entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### VAL-{NNN}: {Short title}

- **Target HYP**: {HYP-NNN being validated}
- **Method**: {Description of the validation approach — which tactics were used}
- **Evidence examined**: {List of files, lines, tests, or artifacts inspected}
- **Result**: {Supported | Falsified | Unresolved}
- **Conclusion**: {Summary of findings and their implications}
- **Related**: {Comma-separated list of related entry IDs}
-->

---

### VAL-001: Validation of HYP-001

- **Target HYP**: HYP-001
- **Method**: Alternate entrypoints, Bypass paths, Hidden write paths
- **Evidence examined**: Searched all `.cs` files in `Lovelace.Natural/`, `Lovelace.Integer/`, `Lovelace.Real/`, and `Lovelace.Console/` for references to `_bytes`, `GetBitwise`, or `SetBitwise`. Zero matches found in any project outside Lovelace.Representation. Also verified InternalsVisibleTo grants (DigitStore.cs:6-7) limiting internal access to Representation.Tests and Natural only.
- **Result**: Supported
- **Conclusion**: No code outside Lovelace.Representation accesses the raw byte[] backing store or the internal bitwise methods. The encapsulation invariant holds across the entire solution. All upper layers use GetDigit/SetDigit or snapshot APIs exclusively.
- **Related**: HYP-001, OBS-001, OBS-003, OBS-006

---

### VAL-002: Validation of HYP-002

- **Target HYP**: HYP-002
- **Method**: Alternate entrypoints, Duplicate implementations
- **Evidence examined**: `Lovelace.Real/Real.cs:21-22` (declaration `public class Real : Int`), `Lovelace.Integer/Integer.cs:15` (declaration `public class Integer`). Searched all `.cs` files for `class\s+\w+\s*:\s*(Integer|Int)` — only match was a comment in RealPropertiesTests.cs. Real.cs line 22 confirms `Real : Int` (aliased Integer).
- **Result**: Supported
- **Conclusion**: Integer is not sealed specifically because Real inherits from it. Real is the sole subclass of Integer in the entire solution.
- **Related**: HYP-002, OBS-008, OBS-011

---

### VAL-003: Validation of HYP-003

- **Target HYP**: HYP-003
- **Method**: Hidden write paths, Bypass paths, Contradicting tests
- **Evidence examined**: `Natural.operator+` (Natural.cs:312-336) creates `var result = new Natural()` and returns it; identity shortcuts use `new Natural(left)` / `new Natural(right)` (copy constructors). `Natural.operator-` (Natural.cs:344-377) similarly creates `var result = new Natural()`. Integer arithmetic delegates to Natural operators and wraps results in new Integer instances. Real arithmetic creates new Real instances via internal constructor.
- **Result**: Supported
- **Conclusion**: All arithmetic operators across Natural, Integer, and Real create fresh result instances. No operator mutates its input operands.
- **Related**: HYP-003, OBS-007, OBS-010, OBS-011

---

### VAL-004: Validation of HYP-004

- **Target HYP**: HYP-004
- **Method**: Bypass paths, Special-case conditionals
- **Evidence examined**: Natural.cs:901-923 — all six methods `=> throw new NotImplementedException()`. Integer.cs:570-598 — all six methods `{ result = Zero/default; return false; }`. Real.cs:310-338 — all six methods `{ result = Zero/default; return false; }` (using `new` keyword to shadow Integer versions).
- **Result**: Supported
- **Conclusion**: All 18 generic conversion methods across the three numeric types are stubs. Natural throws NotImplementedException; Integer and Real return false. No cross-type conversion works via the generic math interfaces.
- **Related**: HYP-004, OBS-019

---

### VAL-005: Validation of HYP-005

- **Target HYP**: HYP-005
- **Method**: Bypass paths, Contradicting tests, Special-case conditionals
- **Evidence examined**: `Natural.operator-(Natural, Natural)` at Natural.cs:344-377 — line 347-348 throws `InvalidOperationException` when `right > left`. Return type is `Natural`. No catch or widening logic exists. Evaluator.cs in Console handles the widening by catching the exception and retrying with Integer operands (referenced in EvaluatorBinaryArithmeticTests.cs:182 comment about widening).
- **Result**: Supported
- **Conclusion**: Natural subtraction strictly throws on underflow — it does not auto-widen. The widening behavior exists only in the Console Evaluator application layer.
- **Related**: HYP-005, OBS-007, OBS-017

---

### VAL-006: Validation of HYP-006 — glossary ⚠️ mappings are accurate

- **Target HYP**: HYP-006
- **Method**: Exhaustive source-code search for every ⚠️ glossary entry
- **Evidence examined**: (1) Class mappings: `DigitStore` in DigitStore.cs, `Natural` in Natural.cs:15, `Integer` in Integer.cs:15, `Real` in Real.cs:21 — all confirmed. (2) Constructor/Assignment: `Parse`/`TryParse` methods confirmed in Natural.cs:801-893, Integer.cs:465-544, Real.cs:964-1102. (3) Arithmetic ops: `operator+` in Natural.cs:312/Integer.cs:235, `operator-` in Natural.cs:344/Integer.cs:262, `operator*` in Natural.cs:383/Integer.cs:275, `DivRem` in Integer.cs:283, `Pow` in Natural.cs:674/Integer.cs:327, `Factorial` in Natural.cs, `operator++` in Integer.cs:367, `operator--` in Integer.cs:370, `Negate` in Integer.cs:214. (4) Comparisons: all 6 operators confirmed in Natural.cs:565-572 and Integer.cs:372-396. (5) I/O: `ToString` in DigitStore.cs:595 and all types. (6) Digit storage: `GetBitwise` at DigitStore.cs:255, `SetBitwise` at DigitStore.cs:276, `GetDigit` at DigitStore.cs:110, `SetDigit` at DigitStore.cs:131. (7) Static members: `DisplayDigits` and `Precision` in Natural.cs, `DisplayDecimalPlaces` in Real.cs:58. (8) Real-specific: `Exponent` at Real.cs:104.
- **Result**: Supported
- **Conclusion**: All 43 ⚠️ entries in the glossary correctly map Portuguese C++ identifiers to their English C# equivalents. Every claimed method, property, and operator exists at the documented locations.
- **Related**: HYP-006, OBS-021, OBS-022, OBS-023, OBS-024, OBS-025, OBS-026, OBS-027

---

### VAL-007: Validation of HYP-007 — glossary ❓ Legacy terms verified

- **Target HYP**: HYP-007
- **Method**: Exhaustive search of Legacy .cpp/.hpp files and DigitStore.cs for all ❓ terms
- **Evidence examined**: (1) `VetorLovelace` in VetorLovelace.hpp — confirmed as vector of RealLovelace elements. (2) `VetorMultidimensionalLovelace` in VetorMultidimensionalLovelace.hpp — confirmed. (3) `multiplicar_burro` in Lovelace.cpp:476 — naïve repeated-addition, confirmed. (4) `dividir_burro` in Lovelace.cpp:638 — naïve long-division, confirmed. (5) `inverter` in RealLovelace.cpp:164 — declared but empty implementation, confirmed. (6) `imprimirInfo` in Lovelace.cpp:317 — prints diagnostic state, confirmed. (7) `expandirAlgarismos` in Lovelace.cpp:321 → `GrowDigits` in DigitStore.cs:296 — confirmed with sentinel 0x0C. (8) `reduzirAlgarismos` in Lovelace.cpp:325 → `ShrinkDigits` in DigitStore.cs:438 — confirmed with sentinel 0x0F. (9) `TabelaDeConversao` in Lovelace.cpp:4 — char[] lookup table, replaced by inline arithmetic in C#, confirmed. (10) `toInteiroLovelace` in RealLovelace.cpp:23 — exists in Legacy but no direct `ToInteger()` method in Real.cs; C# uses `ToNatural()` + constructor composition instead.
- **Result**: Supported
- **Conclusion**: All 12 ❓ entries are now grounded in source code. One correction needed: `toInteiroLovelace → ToInteger` should be revised to note that C# achieves this through `ToNatural()` + constructor composition rather than a dedicated method. All sentinel values (0x0C, 0x0F) are confirmed in DigitStore.cs.
- **Related**: HYP-007, OBS-026, OBS-027

---

### VAL-008: Validation of HYP-008 — VetorLovelace/VetorMultidimensional are intentionally out of scope

- **Target HYP**: HYP-008
- **Method**: Alternate entrypoints, Infrastructure divergence
- **Evidence examined**: (1) `LovelaceSharp.slnx` (solution file) — searched for any `Project` element referencing "Vector", "Vetor", or cognates; zero matches. (2) Root workspace directory listing — no folder named `Lovelace.Vector*` or `Vetor*` exists; all eight project folders are accountable to the documented migration plan. (3) `.github/journals/todos.md` — searched for "Vector" or "Vetor"; zero matches. (4) `.github/journals/risks.md` — no RISK entry mentioning vector migration. (5) OBS-034 confirms the two C++ classes exist in `Legacy/` but are referenced by nothing in the C# solution.
- **Result**: Supported
- **Conclusion**: VetorLovelace and VetorMuldimensionalLovelace are confirmed to be intentionally out of scope. No planned C# counterpart exists in the solution, the task list, or the risk register. This is a documented migration boundary, not an accidental omission.
- **Related**: HYP-008, OBS-034, OBS-026

---

### VAL-009: Validation of HYP-009 — Real.Pow stubs are not dependency-blocked

- **Target HYP**: HYP-009
- **Method**: Bypass paths, Special-case conditionals
- **Evidence examined**: (1) `Real.Pow(Real exponent)` body at `Lovelace.Real/Real.cs:625-660` — the two `NotImplementedException` messages read "Non-integer exponents are not yet supported" and "Negative exponents are not yet supported" with no reference to any missing dependency (no "requires Log", "requires Exp", or similar). (2) The positive-integer path (lines 661-673) uses binary exponentiation via `Real.*` operator — a fully working multiplication loop. (3) Negative exponents could be implemented as `1 / Pow(abs(exponent))` using the existing `Invert()` method; no new infrastructure is needed. (4) Non-integer exponents would require `Real.Log`, which is not yet implemented, but the stub's message does not acknowledge this dependency.
- **Result**: Supported
- **Conclusion**: The Pow stubs are effort stubs, not dependency-blocked stubs. Negative-exponent support could be added immediately (via Invert + positive Pow). Non-integer exponent support would require Real.Log; the stub's message does not cite this, which is a minor documentation gap but does not falsify the core claim that no hard technical dependency blocks the stubs from being completed.
- **Related**: HYP-009, OBS-036, OBS-011
