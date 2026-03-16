# Observations

> **Journal file**: Grounded factual findings from source code.
> **Schema**: See `.github/prompts/journal-schema.md` §4.1 for the OBS entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### OBS-{NNN}: {Short title}

- **Source**: `{file}:{line}` (or `{file}:{startLine}-{endLine}`)
- **Fact**: {One or two sentence factual statement grounded in code}
- **Implications**: {What this fact means for the broader system understanding}
- **Confidence**: {High | Medium | Low}
- **Related**: {Comma-separated list of related entry IDs, e.g., HYP-003, OBS-012}
-->

---

### OBS-001: DigitStore is the sole BCD backing store

- **Source**: `Lovelace.Representation/DigitStore.cs:17-33`
- **Fact**: `DigitStore` is a public class with a private `List<byte> _bytes` field (line 22), a `long _digitCount` (line 23), a `bool _isZero` (line 24), and a per-instance `object _syncRoot` lock (line 27). Each instance receives a unique `_id` via `Interlocked.Increment` (line 33) for canonical lock ordering.
- **Implications**: DigitStore is the only type that directly accesses the raw byte array. All upper-layer classes must go through GetDigit/SetDigit or internal bitwise helpers.
- **Confidence**: High
- **Related**:

---

### OBS-002: BCD packing layout — two digits per byte

- **Source**: `Lovelace.Representation/DigitStore.cs:22` (field), internal GetBitwise/SetBitwise methods
- **Fact**: Each byte stores two decimal digits: the high nibble (bits 7–4) holds the even-indexed digit and the low nibble (bits 3–0) holds the odd-indexed digit. Position 0 is the least-significant digit.
- **Implications**: This mirrors the C++ `getBitwise`/`setBitwise` approach. Sentinel values 0x0C (allocated, not written) and 0x0F (freed low nibble) are used during grow/shrink operations.
- **Confidence**: High
- **Related**:

---

### OBS-003: InternalsVisibleTo grants access to Natural and Tests only

- **Source**: `Lovelace.Representation/DigitStore.cs:6-7`
- **Fact**: Two `[assembly: InternalsVisibleTo]` attributes expose internal members to `Lovelace.Representation.Tests` and `Lovelace.Natural` only. No other projects can access internal methods like `GetBitwise`, `SetBitwise`, `SnapshotDigits`, `RentDigitSnapshot`, etc.
- **Implications**: The boundary is enforced at compile time — Integer and Real cannot bypass the public GetDigit/SetDigit API.
- **Confidence**: High
- **Related**: OBS-001

---

### OBS-004: DigitStore thread-safety via Monitor with canonical lock ordering

- **Source**: `Lovelace.Representation/DigitStore.cs:27,33`
- **Fact**: All public and internal mutators acquire `_syncRoot` before reading/writing `_bytes`, `_digitCount`, or `_isZero`. `CopyDigitsFrom` acquires locks on both source and destination in canonical order (by `_id`) to prevent ABBA deadlocks. Lock-free `*Unsafe` private helpers exist for use within an already-held lock to avoid reentrant overhead.
- **Implications**: DigitStore is safe for concurrent access from multiple threads, though performance is bound by the single Monitor per instance.
- **Confidence**: High
- **Related**: OBS-001

---

### OBS-005: Natural class — sealed, implements INumber<Natural>

- **Source**: `Lovelace.Natural/Natural.cs:15-35`
- **Fact**: `Natural` is a `public sealed class` implementing `INumber<Natural>`, `IComparable<Natural>`, `IEquatable<Natural>`, `IParsable<Natural>`, `ISpanParsable<Natural>`, `ISpanFormattable`, and all arithmetic operator interfaces. It has a single private field `DigitStore _store` (line 35).
- **Implications**: Natural wraps DigitStore and provides the full .NET generic math interface for arbitrary-precision non-negative integers.
- **Confidence**: High
- **Related**: OBS-001

---

### OBS-006: Natural uses DigitStore public and internal APIs

- **Source**: `Lovelace.Natural/Natural.cs:97,103,403-404,491-492`
- **Fact**: Natural constructs `DigitStore` instances (lines 97, 103, 109, 128), calls `GetDigit`/`SetDigit` for arithmetic, and uses internal methods `RentDigitSnapshot()` (line 403–404) and `ReturnDigitSnapshot()` (lines 491–492) with `ArrayPool<byte>` for lock-free multiplication. It also calls `TrimLeadingZeros()` and `ToString(char)`.
- **Implications**: Natural is the only arithmetic layer that directly uses DigitStore's internal snapshot APIs, which is consistent with the InternalsVisibleTo grant.
- **Confidence**: High
- **Related**: OBS-003, OBS-005

---

### OBS-007: Natural arithmetic — grade-school algorithms with parallel paths

- **Source**: `Lovelace.Natural/Natural.cs:312,344,383,497`
- **Fact**: Addition (line 312) uses carry-propagation O(max digits). Subtraction (line 344) uses borrow-propagation with underflow guard. Multiplication (line 383) uses grade-school algorithm with parallel `Parallel.For` for large operands. Division (line 497) delegates to `DivRem` which implements trial-division long division.
- **Implications**: All algorithms are digit-level operations on the BCD representation. Parallelism is conditional on operand size vs ProcessorCount.
- **Confidence**: High
- **Related**: OBS-005, OBS-006

---

### OBS-008: Integer class — wraps Natural magnitude + sign flag

- **Source**: `Lovelace.Integer/Integer.cs:15,38-39`
- **Fact**: `Integer` is a `public class` (not sealed) implementing `ISignedNumber<Integer>`, `INumber<Integer>`, and all arithmetic operator interfaces. It has two private readonly fields: `Nat _magnitude` (line 38) and `bool _isNegative` (line 39). Uses `using Nat = global::Lovelace.Natural.Natural;` alias (line 6).
- **Implications**: Integer adds sign semantics on top of Natural. It is not sealed, allowing Real to inherit from it.
- **Confidence**: High
- **Related**: OBS-005

---

### OBS-009: Integer sign normalization — no negative zero

- **Source**: `Lovelace.Integer/Integer.cs:78-82`
- **Fact**: The constructor `Integer(Nat magnitude, bool isNegative)` normalizes the sign: `_isNegative = isNegative && !Nat.IsZero(magnitude)` (line 82). This ensures zero is always stored with positive sign.
- **Implications**: Negative zero is impossible in the Integer representation, simplifying equality and comparison logic.
- **Confidence**: High
- **Related**: OBS-008

---

### OBS-010: Integer delegates all arithmetic to Natural operators

- **Source**: `Lovelace.Integer/Integer.cs:152`
- **Fact**: `ToNatural()` (line 152) returns the private `_magnitude` field directly. All arithmetic (add, subtract, multiply, divide) operates on Natural magnitudes and applies sign rules separately.
- **Implications**: Integer contains no digit-level arithmetic — it is purely a sign-managing wrapper around Natural.
- **Confidence**: High
- **Related**: OBS-008, OBS-007

---

### OBS-011: Real class — extends Integer with decimal exponent and period metadata

- **Source**: `Lovelace.Real/Real.cs:21,104,111,117,123`
- **Fact**: `Real` is a `public class` that extends `Int` (aliased Integer). It adds `long Exponent` (line 104), `long PeriodStart` (line 111), `long PeriodLength` (line 117), and computed `bool IsPeriodic` (line 123). Example: value 3.14 is stored as magnitude=314, exponent=-2.
- **Implications**: Real extends the Integer chain rather than wrapping it, maintaining the Representation ← Natural ← Integer ← Real dependency chain.
- **Confidence**: High
- **Related**: OBS-008

---

### OBS-012: Real uses AsyncLocal for thread-safe precision scoping

- **Source**: `Lovelace.Real/Real.cs:44-48,70-92`
- **Fact**: `MaxComputationDecimalPlaces` defaults to 1000 (line 44). An `AsyncLocal<long?>` field (line 48) allows per-call-stack precision overrides via `WithLocalPrecision()` (line 80+). The getter prefers the AsyncLocal value over the global default (line 73). This is used by Sqrt and Pi to avoid clobbering global precision in parallel tests.
- **Implications**: Test parallelism is safe because each async/thread context can have its own precision without global mutation.
- **Confidence**: High
- **Related**: OBS-011

---

### OBS-013: Real division implements remainder-tracked period detection

- **Source**: `Lovelace.Real/Real.cs` (Divide method)
- **Fact**: The `Divide` method performs long division while tracking remainder values in a dictionary. When a previously-seen remainder recurs, the period (start position, length) is recorded exactly. Division is capped at `MaxComputationDecimalPlaces` for irrationals.
- **Implications**: Rational numbers get exact periodic representation; irrationals are approximated to configurable precision.
- **Confidence**: High
- **Related**: OBS-011, OBS-012

---

### OBS-014: Real implements Chudnovsky algorithm for Pi with binary splitting

- **Source**: `Lovelace.Real/Real.cs:51` (s_chudnovskyC3 constant = 640320³)
- **Fact**: `Pi(long digits)` computes π using the Chudnovsky series with binary splitting. The `PiSegment(long termStart, long termEnd)` internal method computes (P, Q, T) triples recursively. Parallel dispatch via `Task.WhenAll` splits work across up to `ProcessorCount` (capped at 64) tasks. Guard digits (+10) are added beyond requested precision.
- **Implications**: Pi computation avoids intermediate precision loss by accumulating as exact rational (Integer numerator / Natural denominator) before a single final division.
- **Confidence**: High
- **Related**: OBS-011, OBS-012

---

### OBS-015: Real Sqrt uses Newton-Raphson (Heron's method) with progressive precision

- **Source**: `Lovelace.Real/Real.cs` (Sqrt method)
- **Fact**: `Sqrt(Real value)` iterates `x_{n+1} = (x_n + value/x_n) / 2` seeded from a `double` approximation. Working precision is doubled each iteration from 16 up to the target. Guard digits (+50) absorb truncation drift. Convergence is detected when two consecutive results agree to full precision.
- **Implications**: The progressive precision doubling is an optimization that avoids expensive full-precision divisions in early iterations.
- **Confidence**: High
- **Related**: OBS-011, OBS-012

---

### OBS-016: Console project — REPL with tokenizer, parser, evaluator pipeline

- **Source**: `Lovelace.Console/Repl/ReplSession.cs`, `Lovelace.Console/Repl/Tokenizer.cs`, `Lovelace.Console/Repl/Parser.cs`, `Lovelace.Console/Repl/Evaluator.cs`
- **Fact**: The Console project implements an interactive REPL with the pipeline: LineEditor → Tokenizer → Parser (recursive descent, 8 precedence levels) → Evaluator → Value output. It references all three numeric libraries (Natural, Integer, Real). Built-in functions include `abs`, `inv`, `divrem`, `is_even`, `is_odd`, `sign`, `sqrt`, and `pi`.
- **Implications**: The Console project serves as both a user-facing calculator and an integration test surface for the numeric library.
- **Confidence**: High
- **Related**: OBS-005, OBS-008, OBS-011

---

### OBS-017: Value type system with automatic widening

- **Source**: `Lovelace.Console/Repl/Value.cs`
- **Fact**: The `Value` class uses a `ValueKind` enum (Natural=0, Integer=1, Real=2, Boolean=3, Text=4) to discriminate stored types. `WidenPair` promotes both operands to the wider numeric type (Natural → Integer → Real). Literal type inference is deferred: text containing `.` or `(` becomes Real, otherwise Natural.
- **Implications**: The widening chain mirrors the dependency chain. Subtraction of Naturals auto-widens to Integer on underflow.
- **Confidence**: High
- **Related**: OBS-016

---

### OBS-018: Dependency chain matches architectural spec

- **Source**: `Lovelace.Natural/Lovelace.Natural.csproj`, `Lovelace.Integer/Lovelace.Integer.csproj`, `Lovelace.Real/Lovelace.Real.csproj`, `Lovelace.Console/Lovelace.Console.csproj`
- **Fact**: Project references form the chain: `Lovelace.Representation` ← `Lovelace.Natural` ← `Lovelace.Integer` ← `Lovelace.Real`. `Lovelace.Console` references Natural, Integer, and Real directly.
- **Implications**: The implemented dependency chain matches the documented architecture exactly.
- **Confidence**: High
- **Related**: OBS-005, OBS-008, OBS-011, OBS-016

---

### OBS-019: All numeric types implement .NET generic math interfaces

- **Source**: `Lovelace.Natural/Natural.cs:15-30`, `Lovelace.Integer/Integer.cs:15-36`, `Lovelace.Real/Real.cs:21-38`
- **Fact**: Natural implements `INumber<Natural>` and related interfaces. Integer implements `ISignedNumber<Integer>` and `INumber<Integer>`. Real implements `INumber<Real>` and `ISignedNumber<Real>`. All three include `IParsable<T>`, `ISpanParsable<T>`, and `ISpanFormattable`. Generic conversion methods (`TryConvertFrom/To*`) all throw `NotImplementedException` or return `false`.
- **Implications**: The types are usable with .NET generic math constraints (`where T : INumber<T>`), though cross-type conversion is not yet implemented.
- **Confidence**: High
- **Related**: OBS-005, OBS-008, OBS-011

---

### OBS-020: Real.InternalsVisibleTo exposes only to Real.Tests

- **Source**: `Lovelace.Real/Real.cs:11`
- **Fact**: `[assembly: InternalsVisibleTo("Lovelace.Real.Tests")]` is the only InternalsVisibleTo attribute in Real.cs. This grants test access to internal members like `PiSegment` and `WithLocalPrecision`.
- **Implications**: The Console project cannot access Real's internal methods — it uses only the public API.
- **Confidence**: High
- **Related**: OBS-003, OBS-011

---

### OBS-021: Parse flow — Real.Parse("3.14") calls Natural.Parse("314") with exponent=-2

- **Source**: `Lovelace.Real/Real.cs:964-970,1002-1102`, `Lovelace.Natural/Natural.cs:828-866`, `Lovelace.Representation/DigitStore.cs:110-131,276-300`
- **Fact**: `Real.Parse(string)` delegates to `Real.TryParse(ReadOnlySpan<char>)` which: (1) strips sign, (2) locates the decimal point via `IndexOf('.')`, (3) splits into integer and fractional parts, (4) concatenates into `allDigits`, (5) computes `exponent = -(long)nonRepeating.Length`, (6) calls `Nat.TryParse(allDigits)`, (7) constructs `new Real(magnitude, isNeg, exponent)`. `Natural.TryParse` validates all characters are digits, processes right-to-left (LSD first) calling `DigitStore.SetDigit(i, digit)` for each. SetDigit packs into BCD bytes via `SetBitwise((byte)(high << 4) | (low & 0x0F))`. Supports periodic notation `"0.(3)"` via parenthesized syntax.
- **Implications**: The parse flow is end-to-end: string → Real.TryParse → Nat.TryParse → DigitStore.SetDigit → SetBitwise (BCD packing). Exponent is derived from the number of fractional digits. No allocation beyond the DigitStore's backing list.
- **Confidence**: High
- **Related**: OBS-005, OBS-011

---

### OBS-022: Divide flow — Real division uses Dictionary<string,long> remainder tracking for period detection

- **Source**: `Lovelace.Real/Real.cs:487-564`
- **Fact**: `Real.Divide(Real, Real)` computes integer part via `Nat.DivRem`, then enters a fractional-digit loop where each iteration: (1) calls `remainder.ToString()` as dictionary key, (2) checks `remainderHistory.TryGetValue` for a previously-seen remainder, (3) if found, sets `periodStart=firstPos`, `periodLength=position-firstPos`, breaks; (4) otherwise, multiplies remainder by 10, calls `DivRem`, appends the digit. Loop caps at `MaxComputationDecimalPlaces`. The result uses the internal constructor `new Real(mag, isNeg, exponent, periodStart, periodLength)`. Periodic results bypass `Normalize()` to preserve period metadata.
- **Implications**: Period detection is exact for all rational numbers — the loop will always find a repeated remainder within `denominator` iterations. Irrational numbers (from other flows) hit the precision cap.
- **Confidence**: High
- **Related**: OBS-013, OBS-011

---

### OBS-023: REPL pi(100) evaluation — full pipeline: Tokenizer→Parser→Evaluator→Real.Pi

- **Source**: `Lovelace.Console/Repl/ReplSession.cs:99-138`, `Lovelace.Console/Repl/Tokenizer.cs:20-110`, `Lovelace.Console/Repl/Parser.cs:34-220`, `Lovelace.Console/Repl/Evaluator.cs:47-478`
- **Fact**: For input `pi(100)`: (1) Tokenizer produces `[Identifier("pi"), LParen, NumberLiteral("100"), RParen, Eof]`; (2) Parser's `ParsePrimary` matches Identifier followed by LParen and produces `CallExpr("pi", [LiteralExpr("100")])`; (3) Evaluator's `EvaluateCall` dispatches to `BuiltinPi`, which evaluates the argument as `Nat.Parse("100")`, converts to `long`, and calls `Real.Pi(100)`; (4) Result is wrapped in `Value(Real)` and printed via `ReplSession.PrintResult` as `= {value} (Real)`. The REPL stores the result in variable `_` for subsequent use.
- **Implications**: The REPL is a complete integration layer: Tokenizer→Parser→Evaluator→NumericLibrary→Formatter. All 8 builtins follow the same `EvaluateCall` dispatch pattern.
- **Confidence**: High
- **Related**: OBS-016, OBS-017, OBS-014

---

### OBS-024: Real.ToString formats periodic numbers as "intPart.nonRepeating(period)"

- **Source**: `Lovelace.Real/Real.cs:1139-1198`
- **Fact**: When `IsPeriodic` is true, `ToString()` computes: (1) `fracLen = -Exponent`, (2) pads digits to fractional length, (3) splits into `intPart` (left of decimal) and `allFrac` (right of decimal), (4) extracts `nonRepeating = allFrac[..PeriodStart]` and `period = allFrac[PeriodStart..(PeriodStart+PeriodLength)]`, (5) formats as `sign + intPart + "." + nonRepeating + "(" + period + ")"`. Example: `1/3` → magnitude=3, exponent=-1, PeriodStart=0, PeriodLength=1 → `"0.(3)"`. For `1/6` → `"0.1(6)"`.
- **Implications**: The periodic formatting is the inverse of parse — `Real.Parse("0.(3)")` round-trips correctly because TryParse handles the parenthesized syntax.
- **Confidence**: High
- **Related**: OBS-022, OBS-011

---

### OBS-025: Value type system — WidenPair promotes both operands to max(Kind)

- **Source**: `Lovelace.Console/Repl/Value.cs:12-20,82-127`
- **Fact**: `ValueKind` enum assigns Natural=0, Integer=1, Real=2, Boolean=3, Text=4. `WidenPair(a, b)` computes `target = (ValueKind)Math.Max((int)a.Kind, (int)b.Kind)` and widens both operands. `Widen` promotes: Natural→Integer via `new Int(asNatural())`; Natural→Real via `new Real(new Int(asNatural()))`; Integer→Real via `new Real(asInteger())`. Literal type inference: text containing `.` or `(` → Real; otherwise → Natural.
- **Implications**: The widening chain Natural⊆Integer⊆Real mirrors the dependency chain. The Evaluator's subtraction handler catches Natural's `InvalidOperationException` on underflow and retries with Integer operands — this is the only widening not handled by WidenPair.
- **Confidence**: High
- **Related**: OBS-017, OBS-016

---

### OBS-026: Legacy C++ vocabulary — all 12 glossary ❓ terms verified in source

- **Source**: `Legacy/Lovelace.cpp:4,317,321,325,476,638`, `Legacy/Lovelace.hpp`, `Legacy/RealLovelace.cpp:23,164`, `Legacy/RealLovelace.hpp`, `Legacy/VetorLovelace.hpp`, `Legacy/VetorMultidimensionalLovelace.hpp`
- **Fact**: All glossary terms marked ❓ have been traced to their source: (1) `VetorLovelace` / `VetorMultidimensionalLovelace` exist in Legacy headers — not yet migrated; (2) `multiplicar_burro` (Lovelace.cpp:476) and `dividir_burro` (Lovelace.cpp:638) are naïve algorithms not exposed in C#; (3) `inverter` (RealLovelace.cpp:164) has an empty implementation; (4) `imprimirInfo` (Lovelace.cpp:317) prints diagnostic state; (5) `expandirAlgarismos`→`GrowDigits` and `reduzirAlgarismos`→`ShrinkDigits` confirmed in DigitStore.cs with sentinel values 0x0C and 0x0F respectively; (6) `TabelaDeConversao` (Lovelace.cpp:4) is a digit-to-char lookup table replaced by inline arithmetic in C#; (7) `toInteiroLovelace` (RealLovelace.cpp:23) exists in Legacy but has no direct `ToInteger()` equivalent in C# — the pattern uses `ToNatural()` + constructor instead.
- **Implications**: All glossary ❓ entries are now grounded in source code. The `toInteiroLovelace` mapping needs correction: C# uses composition (`ToNatural` + constructor) rather than a dedicated conversion method.
- **Confidence**: High
- **Related**: OBS-018

---

### OBS-027: Sentinel values 0x0C and 0x0F confirmed in DigitStore.cs

- **Source**: `Lovelace.Representation/DigitStore.cs:296,438`
- **Fact**: `GrowDigits()` (line 296) appends a byte with the sentinel value `0x0C` (12 decimal) to mark a newly allocated but unused digit slot. `ShrinkDigits()` (line 438) sets the low nibble of the last byte to `0x0F` (15 decimal) when shrinking an even-count store, marking the vacated half-byte as "no digit."
- **Implications**: The sentinel values documented in the glossary and legacy knowledge map are implemented exactly as described. These are the only two sentinel values in the BCD encoding scheme.
- **Confidence**: High
- **Related**: OBS-002, OBS-001

---

### OBS-028: Real.Sqrt progressive precision doubling — Newton-Raphson seeded from double

- **Source**: `Lovelace.Real/Real.cs` (Sqrt method)
- **Fact**: `Sqrt(Real value)` seeds from a `double` approximation, then iterates Newton-Raphson (`x_{n+1} = (x_n + value/x_n) / 2`). Working precision starts at 16 digits and doubles each iteration up to the target. Guard digits (+50) absorb truncation drift. Convergence is detected when two consecutive results agree to full precision. `WithLocalPrecision` scopes the division precision per-iteration to avoid clobbering global state.
- **Implications**: Progressive precision doubling is an optimization — early iterations use cheap low-precision divisions, and only the final iterations use full precision. This makes Sqrt logarithmic in the number of full-precision operations.
- **Confidence**: High
- **Related**: OBS-015, OBS-012

---

### OBS-029: C++ Lovelace class split into DigitStore + Natural in C#

- **Source**: `Legacy/Lovelace.hpp:17-35`, `Lovelace.Representation/DigitStore.cs:17-33`, `Lovelace.Natural/Natural.cs:15-35`
- **Fact**: The C++ `Lovelace` class combined raw digit storage (protected `vector<char> algarismos`, private `long long tamanho`, `quantidadeAlgarismos`, `bool zero`) and all arithmetic in one class hierarchy. In C#, storage is extracted into `DigitStore` (in `Lovelace.Representation`) and arithmetic is placed in `Natural` (in `Lovelace.Natural`), forming two distinct projects with an explicit dependency boundary.
- **Implications**: The split enforces the encapsulation invariant that arithmetic code cannot access raw bytes directly. This is a deliberate structural deviation from the C++ single-class design.
- **Confidence**: High
- **Related**: OBS-001, OBS-005, OBS-003

---

### OBS-030: C++ Lovelace methods not migrated or replaced in C#

- **Source**: `Legacy/Lovelace.hpp:42-70`, `Legacy/Lovelace.cpp:8-14`
- **Fact**: The following C++ `Lovelace` methods have no direct C# equivalent: (1) `multiplicar_burro` and `dividir_burro` — naïve O(n²) algorithms, absent from Natural.cs; (2) `errorMessage(string)` — replaced by .NET typed exceptions (`DivideByZeroException`, `InvalidOperationException`, etc.); (3) `imprimirInfo(int)` — diagnostic print helper, removed with no C# counterpart; (4) `inverteNumero(Lovelace &saida)` — private number-reversal helper, absorbed into DigitStore internals; (5) `TabelaDeConversao` (char[10] digit-to-char table) — replaced by inline `digit + '0'` arithmetic; (6) `operator<<`/`operator>>` C++ stream operators — replaced by `ToString()`/`Parse()`; (7) `operator=` C++ assignment — not applicable in C# (copy constructor used explicitly).
- **Implications**: The C# design eliminates C++ idioms that have no .NET equivalent and replaces fatal `errorMessage`+`exit(1)` with catchable typed exceptions.
- **Confidence**: High
- **Related**: OBS-029, OBS-035

---

### OBS-031: C++ getBitwise/setBitwise visibility changed from public to internal in C#

- **Source**: `Legacy/Lovelace.hpp:52-53`, `Lovelace.Representation/DigitStore.cs:6-7`
- **Fact**: In C++, both `getBitwise`/`setBitwise` (raw BCD byte access) and `getDigito`/`setDigito` (digit-level access) were declared `public`. In C#, `GetBitwise`/`SetBitwise` are `internal` (accessible only to `Lovelace.Natural` via `InternalsVisibleTo`) while `GetDigit`/`SetDigit` are `public`. This intentionally prevents `Integer` and `Real` from calling raw BCD accessors.
- **Implications**: The visibility narrowing is a deliberate architectural enforcement: upper layers must use the digit-level API, not the byte-level API. Integer and Real cannot bypass this boundary at compile time.
- **Confidence**: High
- **Related**: OBS-001, OBS-003, OBS-029

---

### OBS-032: InteiroLovelace → Integer migration: five structural decisions

- **Source**: `Legacy/InteiroLovelace.hpp:10-12,29-35`, `Lovelace.Integer/Integer.cs:38-39,78-82,152`
- **Fact**: Five structural changes in the C++ → C# migration for the signed integer type: (1) C++ `sinal` (bool, positive=true/negative=false) → C# `_isNegative` (bool inverted semantics: true=negative); (2) `getSinal()`/`setSinal()` → replaced by `IsNegative(value)` static read-only predicate (no public setter, sign is set only at construction); (3) `toLovelace(Lovelace &saida)` out-parameter → C# `ToNatural()` returning a `Natural` value directly; (4) `inverterSinal()` → `operator-`; (5) `multiplicar_burro` was already commented out in the C++ header (`//InteiroLovelace multiplicar_burro`) — confirmed not migrated.
- **Implications**: The sign-semantics inversion (positive=true vs isNegative=false) could cause confusion when reading legacy code alongside C#. The out-parameter `toLovelace` pattern is replaced by idiomatic C# return-value style.
- **Confidence**: High
- **Related**: OBS-008, OBS-009

---

### OBS-033: RealLovelace → Real migration: seven structural decisions

- **Source**: `Legacy/RealLovelace.hpp:9-10,16-46`, `Legacy/RealLovelace.cpp:1-2,23`, `Lovelace.Real/Real.cs:44,104,111,117,123,166,604-609`
- **Fact**: Seven structural changes in the C++ → C# migration for the real number type: (1) C++ `expoente` (private long field) → C# `Exponent` public property with public setter (RISK-002); (2) `casasDecimaisExibicao` → `DisplayDecimalPlaces` (static atomic property, same default of 100); (3) `toInteiroLovelace(zeros)` private BCD re-encoding helper → no equivalent — DigitStore handles BCD packing directly; (4) C++ `inverter()` was an empty stub (RealLovelace.cpp:164) → C# `Invert()` (line 609) fully implements `1 / this` via Real division; (5) C++ `RealLovelace(const double A)` → C# `Real(double value)` (line 166) implemented via `double.ToString("R")` parse chain; (6) C++ `ler()` (interactive console input) → not migrated, REPL LineEditor handles input separately; (7) C# adds `PeriodStart`, `PeriodLength`, `IsPeriodic` (lines 111, 117, 123) with no C++ counterpart — enabling exact rational periodic decimal representation.
- **Implications**: The largest additive change over C++ is the period metadata, enabling exact rational representation that did not exist in the original C++ design. The `toInteiroLovelace` private BCD conversion helper is eliminated because DigitStore handles encoding natively.
- **Confidence**: High
- **Related**: OBS-011, OBS-022, OBS-024

---

### OBS-034: VetorLovelace and VetorMultidimensionalLovelace are unmigrated C++ classes

- **Source**: `Legacy/VetorLovelace.hpp:1-26`, `Legacy/VetorMultidimensionalLovelace.hpp:1-23`
- **Fact**: `VetorLovelace` (a dynamic array of `RealLovelace` elements with 9 public methods: `imprimir`, `getElemento`, `setElemento`, `getDimensionalidade`, `setDimensionalidade`, `somar`, `subtrair`, `produtoInterno`, `multiplicar`) and `VetorMuldimensionalLovelace` (multidimensional vector wrapper, 8 public methods) exist in the C++ Legacy codebase but have no C# counterpart in any project or folder in the solution.
- **Implications**: These classes represent an unmigrated vector-algebra feature layer. Neither class is referenced by existing C# project files, confirming they are a known migration gap rather than an accidental omission.
- **Confidence**: High
- **Related**: OBS-026

---

### OBS-035: Natural exception model — typed .NET exceptions replace C++ errorMessage

- **Source**: `Lovelace.Natural/Natural.cs:126,305,348,566`
- **Fact**: Natural uses four distinct exception types for domain edge cases: (1) `ArgumentOutOfRangeException` for a negative `int` constructor argument (line 126); (2) `InvalidOperationException` for unary negation of a Natural — "Cannot negate a Natural number; the result would be negative" (line 305); (3) `InvalidOperationException` when subtraction underflows: right > left (line 348: "Natural subtraction would produce a negative result"); (4) `DivideByZeroException` when the divisor is zero (line 566: "Cannot divide by zero"). The C++ equivalent used `errorMessage(string)` followed by `exit(1)` — a process-terminating pattern with no typed hierarchy.
- **Implications**: The C# exception model is a significant improvement: callers can catch specific exceptions and handle them gracefully. The C++ pattern made error recovery impossible.
- **Confidence**: High
- **Related**: OBS-030, OBS-007

---

### OBS-036: Real exception model — typed exceptions for domain boundary errors

- **Source**: `Lovelace.Real/Real.cs:490,642,651,730,855`
- **Fact**: Real uses typed exceptions for domain errors: (1) `DivideByZeroException` when the divisor Real is zero (line 490); (2) `NotImplementedException` when the exponent in `Pow` is non-integer (line 642: non-integer not yet supported); (3) `NotImplementedException` when the exponent in `Pow` is negative (line 651: negative exponents not yet supported); (4) `ArithmeticException` when `Sqrt` receives a negative argument (line 730: "Square root is not defined for negative numbers"); (5) `ArgumentOutOfRangeException` when `Pi` receives `digits ≤ 0` (line 855). All errors are catchable — unlike C++ `errorMessage`+`exit(1)`.
- **Implications**: `Pow` has two known stub branches (`NotImplementedException`) that are acknowledged implementation gaps. These are migration stubs that will need to be completed to support fractional or negative exponents.
- **Confidence**: High
- **Related**: OBS-033, OBS-035
