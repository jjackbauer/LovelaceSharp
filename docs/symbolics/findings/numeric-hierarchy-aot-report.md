# LovelaceSharp — Numeric Hierarchy & AOT/Build Report
*Evidence-based inspection of the LovelaceSharp repository for planning `Lovelace.Symbolics`.*
*All line numbers are from the current working tree (`C:\Users\ricar\dev\LovelaceSharp`). Generated `obj\` files and the `.worktrees\binary` mirror are excluded.*

> **POST-REMEDIATION NOTE (2026-09-08).** This report predates the DSP plugin remediation
> (`docs/architecture/dsp-plugin-remediation-plan.md`) that landed the same day. Superseded
> facts in this file: (1) §7's `IArrayKernel<T> where T : unmanaged` is gone — the kernel seam
> is now `IFieldKernel<T>` (field injected, no `unmanaged` constraint); (2) `IField<T>` moved
> from `Lovelace.Array` to `Lovelace.Abstractions`, and production `NaturalField`/`IntegerField`/
> `RealField` implementations now exist (`Lovelace.Natural/NaturalField.cs` etc.);
> (3) `ScalarResult` landed in `Lovelace.Abstractions/Modus.cs` with duplicate-registration
> guards in `ModusHost`; (4) plugin builtins run under a default `Real.WithPrecision(30, 15)`
> scope when the session precision knob is untouched. See
> [../architecture.md](../architecture.md) §1.3 (correction 13) and
> [../implementation-plan.md](../implementation-plan.md) §1.1 for the authoritative
> post-remediation baseline.

---

## 1. Lovelace.Representation

**Files:** `Lovelace.Representation\DigitStore.cs` (only top-level `.cs` file) + `Lovelace.Representation\README.md`.
There are **no MGIR or AST types** in this project (grep for `MGIR` returns only a comment in `Lovelace.Knowledge.Run\Program.cs`; the language AST lives in `Lovelace.Suite\Ast.cs`).

### 1a. Public API of `DigitStore`
`public class DigitStore` — `DigitStore.cs:17`.

| Member | Line | Signature / behaviour |
|---|---|---|
| Default ctor | 40 | `DigitStore()` → empty zero store (`DigitCount=0`, `IsZero=true`, `ByteCount=0`) |
| Copy ctor | 47 | `DigitStore(DigitStore other)` — deep copy under `other._syncRoot` |
| `ByteCount` | 65 | `long` (read-only) — backing byte count |
| `DigitCount` | 68 | `long` (getter only; mutated via `internal SetDigitCount`) |
| `IsZero` | 71 | `bool` (getter only; mutated via `internal SetIsZero`) |
| `GetDigit` | 108 | `byte GetDigit(long position)` — LSD-first; returns 0 when `IsZero`/out of range |
| `SetDigit` | 132 | `void SetDigit(long position, byte digit)` — sequential writes only (`position ≤ DigitCount`), else `ArgumentOutOfRangeException` |
| `TrimLeadingZeros` | 437 | `public void TrimLeadingZeros()` |
| `ToString()` | 571 | decimal, MSD-first |
| `ToString(char)` | 579 | thousands separator every 3 digits; uses `Parallel.For` for interior bytes (638) |
| `Dump(bool)` | 671 | debug helper |

Internal (friend) members: `SetDigitCount` (78), `SetIsZero` (91), `SnapshotDigits` (180), `RentDigitSnapshot` (214) / `ReturnDigitSnapshot` (241), `GetBitwise` (255), `SetBitwise` (276), `GrowDigits` (296), `ShrinkDigits` (415), `ClearDigits` (454), `CopyDigitsFrom` (470), `Initialize` (499), `Reset` (516), `SetDigitsBulk` (534).

### 1b. BCD packing
- Two decimal digits per byte (high nibble = even index, low nibble = odd index) — `DigitStore.cs:11-16`, unpack/pack at `GetBitwise`/`SetBitwise` (255/276).
- Position 0 = least-significant digit (little-endian digit order).
- Sentinel values: `0x0C` = allocated-but-empty slot (appended by `GrowDigits`, 296/319); `0x0F` = freed low nibble (written by `ShrinkDigits`, 415/426). Backing store is `List<byte> _bytes` (22).
- `ByteCount == ⌈DigitCount/2⌉` (README constraint 5).

### 1c. Thread-safety mechanism
- Per-instance `private readonly object _syncRoot` monitor; every read/write path locks it (27, 110, 134, 182, 216, 278, 298, 417, 455, 499, 518, 536).
- Deadlock avoidance in `CopyDigitsFrom`: both instance locks acquired in canonical order by a monotonic construction ID (`_id` from `Interlocked.Increment`, 32-33; 479-484).
- "Unsafe" lock-free helpers (`*Unsafe`, 317-407) eliminate reentrant monitor acquisitions for callers already holding the lock (`TrimLeadingZeros`, `Reset`).
- Zero-GC path: `CollectionsMarshal.AsSpan`/`SetCount` in `CopyDigitsFrom` (491-494); `ArrayPool<byte>.Shared` in snapshot/`ToString` (222, 244, 596, 666).

### 1d. InternalsVisibleTo grants
`DigitStore.cs:6-7`:
```
[assembly: InternalsVisibleTo("Lovelace.Representation.Tests")]
[assembly: InternalsVisibleTo("Lovelace.Natural")]
```
Only **tests + Lovelace.Natural** are granted. **Lovelace.Integer and Lovelace.Real are NOT granted**, despite the README claiming all upper layers use the API.

### 1e. Assessment (symbolic relevance)
- **Vestigial / not in the numeric hot path.** `DigitStore` is referenced by *zero* production projects — `Natural` (which holds the `InternalsVisibleTo` grant) actually stores binary `ulong[]` limbs and never calls `DigitStore` (verified by grep: only `Lovelace.Representation.Tests` and `DigitStore.cs` itself reference the type). The BCD layer is dead code for the current arithmetic stack.
- **Liability**: mutable, lock-heavy, `class` (reference equality default; no `Equals`/`GetHashCode` override) — unsuitable as a symbolic coefficient container.
- **Conclusion**: not reusable for `Lovelace.Symbolics`; consider deprecating/removing.

---

## 2. Lovelace.Natural

**File:** `Lovelace.Natural\Natural.cs` (1516 lines).

### 2a. Declaration
`public sealed class Natural : INumber<Natural>, IComparable<Natural>, IEquatable<Natural>, IParsable<Natural>, ISpanParsable<Natural>, ISpanFormattable, IAdditionOperators<…>, ISubtractionOperators<…>, IMultiplyOperators<…>, IDivisionOperators<…>, IModulusOperators<…>, IIncrementOperators<…>, IDecrementOperators<…>, IComparisonOperators<…>` — `Natural.cs:14-28`.

### 2b. Representation & immutability
- Backing store: **little-endian 64-bit binary limbs (base 2⁶⁴)**, *not* BCD — `Natural.cs:11-13, 31-38, 41`.
- Dual lazy cache: `_limbs` (binary) and `_decimal` (canonical string), materialized once, cached via `Interlocked.CompareExchange` (151-168). Instances are immutable; operators return new instances.

### 2c. Key arithmetic API
- Static properties: `One` (79), `Zero` (85), `AdditiveIdentity` (88), `MultiplicativeIdentity` (91), `Radix = 10` (82).
- Config: `DisplayDigits` (61), `Precision` (**stub** — 67-72, "C++ body was absent").
- Operators: `+` (309), `-` throws when result negative (340-346), `*` (366), `/` (376), `%` (379), `++`/`--` (390/393), unary `+` (297), unary `-` **always throws** `InvalidOperationException` (301), comparison operators (400-420).
- `static Natural DivRem(left, right, out remainder)` (431); instance overload `DivRem(divisor, out rem)` (493).
- `ShiftLeftDecimal(long k)` = ×10ᵏ (485).
- `Pow(Natural exponent)` — repeated squaring (499).
- `Factorial(IProgress<double>?)` — parallel partition across `Environment.ProcessorCount` via `Parallel.For` (523-566).
- **No `GCD`/`LCM`.** Confirmed by grep — no `Gcd`/`Lcm`/`GreatestCommonDivisor`/`LeastCommonMultiple` in `Natural` or `Integer`.
- Formatting/parsing: `ToString()` (573), `ToString(format, provider)` supports `"N"` grouping (609), `TryFormat` (617), `Parse`/`TryParse` incl. `NumberStyles` (635-691).

### 2d. Parallelism
- Karatsuba multiplication: top `ParallelKaratsubaDepth=2` levels fork 3 sub-products via `Parallel.Invoke` (780-833); threshold 40 limbs (769).
- NTT (number-theoretic transform) multiplication over two primes, `Parallel.For` per butterfly stage when ≥8 groups (1011-1043); NTT threshold 100000 combined limbs (993).
- Newton-reciprocal division above 4096 combined limbs (777, 1405); Knuth Algorithm D below (1147).
- `ToStringRecursive` fork-join via `Task.Run` above 2048 limbs (576, 596).
- Unbalanced multiply via `Parallel.For` over chunks (863).

### 2e. Generic math conversions
**Stubbed.** `INumberBase<Natural>.TryConvertFrom*/TryConvertTo*` all `throw new NotImplementedException()` — `Natural.cs:697-713`.

### 2f. Assessment (symbolic relevance)
- **REUSE candidate**: `Natural` is a correct, parallel, immutable arbitrary-precision unsigned integer with `DivRem`, `Pow`, `Factorial`. Its `ulong[]` limbs + `DivRem` are the natural substrate for an exact **unsigned** coefficient type.
- **Gaps**: no `GCD` (mandatory for rational normalization / polynomial content), no `LCM`, no `IsPowerOfTwo`/bit tests, `Precision` is a stub, and `TryConvertFrom/To` throw rather than converting to/from `BigInteger`, `long`, etc.

---

## 3. Lovelace.Integer

**File:** `Lovelace.Integer\Integer.cs` (598 lines).

### 3a. Declaration
`public class Integer : ISignedNumber<Integer>, INumber<Integer>, IComparable<Integer>, IEquatable<Integer>, IParsable<Integer>, ISpanParsable<Integer>, ISpanFormattable, IUnaryNegationOperators<…>, IUnaryPlusOperators<…>, IAdditionOperators<…>, ISubtractionOperators<…>, IMultiplyOperators<…>, IDivisionOperators<…>, IModulusOperators<…>, IIncrementOperators<…>, IDecrementOperators<…>, IComparisonOperators<…>` — `Integer.cs:15-32`.
Note: **not `sealed`** (unlike `Natural`), a deliberate open point for `Real` to subclass.

### 3b. Representation & sign normalization
- `private readonly Nat _magnitude` + `private readonly bool _isNegative` (38-39). Immutable fields; the class is effectively immutable.
- Sign normalization: **zero is always positive.** `Integer(Nat magnitude, bool isNegative)` forces `_isNegative = isNegative && !Nat.IsZero(magnitude)` (78-83); `Parse` normalizes `-0` → `+0` (497). `Negate()` preserves positive zero (214-218).

### 3c. API surface
- Static: `One` (46), `Zero` (52), `NegativeOne` (61), `Radix=10` (49), `AdditiveIdentity`/`MultiplicativeIdentity` (55/58).
- `ToNatural()` → magnitude (152).
- `Sign` property (-1/0/+1) (204).
- Operators: `+` (255), `-` (269), `*` (287), `/` (308), `%` (311), `++`/`--` (367/370), unary `-`/`+` (221/224), `==`/`!=`/`<`/`>`/`<=`/`>=` (416-426).
- Instance methods: `Add` (235), `Subtract` (266), `Multiply` (279), `DivRem(divisor, out remainder)` (299), `Negate` (214), `Increment`/`Decrement` (361/364), `Pow(Integer exponent)` (327), `Factorial(IProgress<double>?)` (349).
- Formatting/parsing incl. `NumberStyles` overloads (433-546).
- Magnitude helpers: `Abs` (553), `MaxMagnitude`/`MinMagnitude` (556/563).

### 3d. `ToNatural` semantics
`public Nat ToNatural() => _magnitude;` — `Integer.cs:152`. Returns the **absolute magnitude** (the `Natural` field directly; `Natural` is immutable so the reference is safe).

### 3e. GCD/LCM
**Absent** (same grep result as §2c).

### 3f. Generic math conversions
**Stubbed but non-throwing**: all six `TryConvert*` return `false` with `result = Zero/default` — `Integer.cs:570-597` (contrast with `Natural`, which throws).

### 3g. Assessment (symbolic relevance)
- **Primary REUSE candidate for exact integer polynomial coefficients.** `Integer` is the signed, arbitrary-precision coefficient type. It already feeds `Real`'s exact-rational Pi/E accumulation (`Integer` numerator / `Natural` denominator — see §4).
- **Liabilities**: (a) not sealed + reference type → structural equality is overloaded but `GetHashCode()` hashes `ToString()` (413), so `Integer`/`Real` values do **not** hash equal to their string; (b) no `GCD`, so a rational/polynomial content step cannot be done in-domain; (c) `TryConvertFrom` returns `false` instead of actually converting from `BigInteger`/`long` — symbolic code importing coefficients must hand-roll bridges.

---

## 4. Lovelace.Real

**File:** `Lovelace.Real\Real.cs` (2048 lines) + `LReal64.cs` / `LReal128.cs` (fixed-width fast paths).

### 4a. Declaration & inheritance
`public class Real : Int, INumber<Real>, ISignedNumber<Real>, IComparable<Real>, IEquatable<Real>, IParsable<Real>, ISpanParsable<Real>, ISpanFormattable, IAdditionOperators<…>, ISubtractionOperators<…>, IMultiplyOperators<…>, IDivisionOperators<…>, IUnaryNegationOperators<…>, IIncrementOperators<…>, IDecrementOperators<…>, IComparisonOperators<…>` — `Real.cs:21-37`.
`InternalsVisibleTo("Lovelace.Real.Tests")` at 11.
**Inherits `Integer`** (the magnitude is stored in the inherited `Natural` via `Integer`'s fields; `Real` adds an exponent + period metadata).

### 4b. Exponent / period model
- `public long Exponent { get; set; }` — **mutable public setter** (153). Value = significand × 10^Exponent (e.g. exp=-2 → 2 fractional digits).
- `public long PeriodStart { get; private set; }` (160); `public long PeriodLength { get; private set; }` (166); `public bool IsPeriodic => PeriodLength > 0` (172).
- Periodic value stores *one* period block only (e.g. `0.(3)` stored as `Nat=3, Exponent=-1`); expansion on demand via `GetDecimalDigit` (1790) / `ExpandToNonPeriodic` (1845).

### 4c. Periodic-decimal detection in division
`Divide` (548-643) uses remainder-tracked long division: a `Dictionary<string,long>` maps each remainder string → first fractional position; on recurrence it sets `PeriodStart`/`PeriodLength` and stops (599-622). No period within `MaxComputationDecimalPlaces` ⇒ truncated irrational approximation. `DetectAndNormalizePeriod` (1947) post-processes (incl. 0.999… → 1.0). `DivideNonPeriodic` (656) is the single fixed-point division fast path (no detection) used by `PiTo`/`Sqrt`/Taylor series.

### 4d. Precision machinery
- Globals: `_displayDecimalPlaces = 100` (43), `_maxComputationDecimalPlaces = 1000` (44).
- `private static readonly AsyncLocal<long?> _localMaxComputationDecimalPlaces` / `_localDisplayDecimalPlaces` (48-49).
- `DisplayDecimalPlaces` (59) and `MaxComputationDecimalPlaces` (71) prefer the AsyncLocal override over the `Interlocked` global.
- `internal static PrecisionScope WithLocalPrecision(long)` (82) — call-stack-scoped override used by `Sqrt`.
- `public static IDisposable WithPrecision(computation, display)` (94) and `public static void SetLocalPrecision(...)` (107) — public session/evaluation-local precision used by `setprecision`.

### 4e. Sqrt / Pi / E / transcendentals
- `Sqrt` (780) → Newton–Raphson (Heron) seeded from a `double`; progressive precision doubling; exact-square early-exit (890). Batch `Sqrt(IReadOnlyList<Real>)` via `Task.Run` (794-806). Internal `Sqrt(value, precision, progress)` (815) is used by `PiTo`.
- `PiTo(long digits)` (940) — Chudnovsky via Binary Splitting (`PiSegment`, 1422), parallelized, accumulated as an **exact rational** (`Integer numS / Natural denS`) and only the final step divides at guard precision (1035-1038). `Pi` cached via `Lazy<Real>` (1137).
- `ETo(long digits)` (1073) — Taylor sum accumulated as exact rational `Integer/Natural`, single final `DivideNonPeriodic`. `E` cached (1138).
- `Sin`/`Cos` (1159/1168) with 16-entry special-angle table (1236-1279) + Taylor series (1281/1301); `Exp` (1327) with argument reduction + Taylor (1356).
- `Pow` (713) — **integer exponents only**; throws `NotImplementedException` for negative/non-integer exponents (729/739).

### 4f. Exact rational capability & `ToRational`-like APIs
- There is **no public `Rational` type** and **no `ToRational`/`GetNumerator`/`GetDenominator`** anywhere in the codebase (grep = 0 hits).
- The exact-rational machinery (`Integer` numerator / `Natural` denominator) exists **only as private locals** inside `PiTo` (975-1038) and `ETo` (1092-1100). It is **not exposed as a reusable type**.
- Periodic `Real` *is* an exact rational representation in disguise (a terminating/periodic decimal), but equality is string-based (see §4g) and there is no reduction-to-`p/q` API.

### 4g. Comparison / equality semantics
- `Equals` (393): periodic vs periodic ⇒ compare `ToString()` strings; non-periodic ⇒ `CompareTo == 0`.
- `CompareTo` (407-457): exponent-aligned digit-by-digit; for periodic values the comparison window extends by `DisplayDecimalPlaces` (446).
- `GetHashCode()` = `ToString().GetHashCode()` (463) — **string-dependent**, not value-canonical.
- `operator %` (1465) = truncated remainder (`left - Truncate(left/right)*right`); unary `+` (1469).

### 4h. Assessment (symbolic relevance)
- **REUSE (careful)**: `Real` is the only exact/truncated arbitrary-precision real, and its periodic form can represent exact rationals (`1/3` as `0.(3)`). For a symbolic kernel you want *exact* rational coefficients, and `Real` is only exact for numbers with finite/periodic decimal expansions — `sqrt(2)`, `pi` etc. are truncations. Using `Real` as a coefficient silently loses exactness.
- **Liabilities**:
  - **`Exponent` has a public setter** (153) — a mutable value-semantic field on an otherwise immutable type; a symbolic kernel that shares `Real` instances could be corrupted.
  - Equality/`GetHashCode` are **string-based** and precision-dependent for periodic values (compare window = `DisplayDecimalPlaces`), so `Real` is a poor dictionary/hash key for coefficients.
  - No exact `Rational` type is exposed; the `Integer/Natural` pair used internally is the obvious thing to hoist into `Lovelace.Symbolics`.
  - `Pow` non-integer/negative exponents unimplemented.

---

## 5. Lovelace.Complex

**Files:** `Lovelace.Complex\Complex.cs` (236 lines) + fixed-width `LComplex64.cs` / `LComplex128.cs`.

### 5a. Kind
- `public sealed class Complex : IEquatable<Complex>` — `Complex.cs:13`. **A class, not a struct; NOT generic**; represents a pair of two arbitrary-precision `Real` components.
- `public Rl Re { get; }` / `public Rl Im { get; }` (16-19).
- **Does not implement `INumber<Complex>` or any `System.Numerics` operator interface** — only `IEquatable<Complex>`. (The fixed-width forms `LComplex64`/`LComplex128` are `readonly struct : IEquatable<…>` — `LComplex64.cs:14`, `LComplex128.cs:14`.)

### 5b. Internal representation
Two `Real` components (`Re`, `Im`). `I` (imaginary unit) is `0 + 1i`: `public static Complex I => new(new Rl("0"), new Rl("1"))` (38). `Zero`/`One`/`Pi`/`E` are fresh allocations per access (32-44).

### 5c. API surface & functions
- Operators: `+`/`-` (51/58), unary `-` (65), `*` (68), `/` (81), scalar `Complex * Rl` / `Rl * Complex` (75/78), `Complex / Rl` (88). **No `+`/`-`/`==` with raw `Real` scalars beyond `*`/`/`.**
- Properties: `Conjugate` (132), `MagnitudeSquared` (135, exact, no sqrt), `Magnitude` (141), `Reciprocal` (144 = `Conjugate / |z|²`).
- Functions: **`Exp()` / `Exp(digits)`** only (147/150). **No `Log`, no `Sin`/`Cos`, no `Sqrt`, no `Pow` on `Complex`** — those exist only on `Real`. (DSP provides `re`/`im`/`conj` builtins; `Complex` itself has only `Exp`.)

### 5d. Precision handling
- `Exp(digits)` calls `Rl.Exp`/`Rl.Cos`/`Rl.Sin` at `digits` (150-154). Default `Exp()` uses `Rl.MaxComputationDecimalPlaces` (147).
- The `Binary` helper (102-125) attempts a **fixed-width fast path**: if `Rl.MaxComputationDecimalPlaces ≤ 37` and both operands fit `LComplex64` then `LComplex128`, use the `readonly struct` arithmetic; any `LRealPromoteException` falls back to the arbitrary-precision `slow` path (exactness preserved — fast path never rounds).

### 5e. Assessment (symbolic relevance)
- **REUSE**: `Complex` over `Real` is a usable arbitrary-precision complex pair for `ℂ[x]` coefficients, but it inherits `Real`'s truncation semantics.
- **Gaps/liabilities**: not an `INumber<Complex>` (so no generic-math reuse); reference type with fresh-allocation constants; only `Exp` implemented (no `log`/`sqrt`/`pow`/`sin`/`cos`), and no mixed `Complex + Real`/`Complex == Real` operators (only scalar `*`/`/`).

---

## 6. Lovelace.Abstractions

**Files:** `Precision.cs`, `Slice.cs`, `ArrayValue.cs`, `DenseArray.cs`, `DType.cs`, `Modus.cs`.

### Public types (exact signatures)
- `public readonly record struct Precision(int SignificantDigits)` — `Precision.cs:9`; `ToString()` → `"{n} sig"`.
- `public readonly record struct Slice(long? Start, long? Stop, long? Step)` with `static Slice All` — `Slice.cs:4-8`.
- `public readonly record struct IndexSpec(long? Index, Slice? Slice)` with `Scalar`/`Range` factories — `Slice.cs:11-14`.
- `public abstract class ArrayValue` — `ArrayValue.cs:10`; abstract members: `DType DType`, `Precision Precision`, `int Rank`, `ReadOnlyMemory<long> Shape/Strides`, `long Offset`, `long Numel`, `Type ElementType`, `bool IsContiguous`, `ArrayValue AsContiguous()`, `Transpose(long[]?)`, `Reshape(long[])`, `Slice(IReadOnlyList<IndexSpec>)`, `GetElement(long)`, `GetElement(ReadOnlySpan<long>)`.
- `public sealed class DenseArray<T> : ArrayValue` — `DenseArray.cs:9`; two ctors (packed 19, view 40), `AsSpan()` (98), plus overrides.
- `public enum DType { Natural, Integer, Real, Complex }` — `DType.cs:10-26` (ordered narrow→wide for widening lattice).
- `public enum ArrayOp { Add, Subtract, Multiply, Divide }` — `Modus.cs:4-10`.
- `public interface IArrayKernel<T> where T : unmanaged { DType DType { get; } bool TryElementwise(ArrayOp, ReadOnlySpan<T>, ReadOnlySpan<T>, Span<T>); }` — `Modus.cs:17-22`.
- `public interface IModusContext { void RegisterArrayBuiltin(string, Func<ArrayValue,ArrayValue>); void RegisterBuiltin(string, IReadOnlyList<string>, Func<IReadOnlyList<object?>, object?>); void RegisterKernel<T>(IArrayKernel<T>) where T : unmanaged; }` — `Modus.cs:25-44`.
- `public interface IModusPlugin { string Name { get; } void Register(IModusContext context); }` — `Modus.cs:47-52`.

### Notable absences (searched for, not found)
There is **no `IField<T>` in Abstractions** (it lives in `Lovelace.Array`, §7), and **no `IRing<T>`, `INumeric`, or `IEvaluator`** anywhere in the repository (grep = 0 hits). The only field abstraction is `IField<T>`; the only evaluation surface is `SuiteEngine`/`Interpreter` in `Lovelace.Suite`.

### Assessment
Abstractions is currently **array/language-shaped, not scalar-algebra-shaped**. For `Lovelace.Symbolics` you would add a ring/field/symbolic-expression interface layer here (or in a new `Lovelace.Symbolics.Abstractions`).

---

## 7. Cross-cutting

### 7a. `System.Numerics` generic-math interfaces by type
| Type | Interfaces implemented | Evidence |
|---|---|---|
| `Natural` | `INumber<Natural>` (+ 9 operator interfaces, `IParsable`, `ISpanParsable`, `ISpanFormattable`) | `Natural.cs:14-28` |
| `Integer` | `ISignedNumber<Integer>`, `INumber<Integer>` (+ operator/parsing interfaces) | `Integer.cs:15-32` |
| `Real` | `INumber<Real>`, `ISignedNumber<Real>` (+ operator/parsing interfaces); **implements `%` (1465) and unary `+` (1469) even though not listed in the header list** | `Real.cs:21-37` |
| `Complex` | **None** (`IEquatable<Complex>` only) | `Complex.cs:13` |
| `LReal64`/`LReal128` | `IComparable<…>`, `IEquatable<…>` only (not `INumber`) | `LReal64.cs:24`, `LReal128.cs:117` |
| `LComplex64`/`LComplex128` | `IEquatable<…>` only | `LComplex64.cs:14`, `LComplex128.cs:14` |

### 7b. Mixed-type operators
- **No implicit/explicit conversion operators exist anywhere** (grep = 0 hits).
- `Real` reaches `Integer`/`Natural` via *constructors* (`Real(Int)` 239; internal `Real(Nat, bool, long, …)` 267) and inheritance (so `Real` *is-a* `Integer`), not operators. No `Integer + Real` / `Integer / Real` mixed operators.
- `Complex` has only scalar **`*`** and **`/`** with `Real` (`Complex.cs:75/78/88`); no `Complex + Real`, no `Complex == Real`, no `Real`→`Complex` implicit.
- Fixed-width ↔ arbitrary conversion is explicit: `LReal64.TryFromReal`/`ToReal` (`LReal64.cs:60/71`), `LComplex64.TryFromComplex`/`ToComplex` (`LComplex64.cs:74/84`), same for 128.

### 7c. Conversion APIs
- `Integer.ToNatural()` (152); `Real` inherits it (returns stored magnitude).
- `Value.AsNatural/AsInteger/AsReal/AsComplex` (`Value.cs:147-155`) + `Value.Widen(target)` / `WidenPair` (184/223) — the widening lattice `Natural → Integer → Real` (`Complex` excluded by design, 189-196).
- `ExactNumber.Parse(string)` / `ToLovelaceLiteral()` (`ExactNumber.cs:84/173`) — literal ↔ rational.
- **`TryConvert` stubs**: `Natural` → throws `NotImplementedException` (`Natural.cs:697-713`); `Integer` → returns `false` (`Integer.cs:570-597`); `Real` → returns `false` (`Real.cs:359-386`). **No real numeric conversions between these types and `BigInteger`/`long`/`double`.**

### 7d. String parsing
`Natural`, `Integer`, `Real` all implement `IParsable`/`ISpanParsable` (+ `NumberStyles` overloads); `Real` additionally accepts periodic notation `"0.(3)"` (`Real.cs:1515-1621`); `Complex`/`LComplex*` provide `Parse` (not `IParsable`). `Radix = 10` for all three numeric types.

---

## 8. AOT / build

### 8a. Per-project build properties
All main projects target **`net10.0`**, `ImplicitUsings` + `Nullable` enable, and set **`<IsAotCompatible>true</IsAotCompatible>`**:

| Project | SDK | Extra |
|---|---|---|
| Abstractions / Representation / Natural / Integer / Real / Complex / Array / Suite / Knowledge / Dsp / Statistics / Console / Run | `Microsoft.NET.Sdk` | `IsAotCompatible=true`; none declare `PublishAot`, `IsTrimmable`, `TrimMode`, `PublishTrimmed`, `InvariantGlobalization`, `AllowUnsafeBlocks`, or `RuntimeHostConfigurationOption` |
| Studio | `Microsoft.NET.Sdk.Web` | `IsAotCompatible=true` + `EnableRequestDelegateGenerator=true` |
| Console / Run | `Microsoft.NET.Sdk` (`Exe`) | `IsAotCompatible=true` |

Project reference graph: `Integer→Natural`; `Real→Integer`; `Complex→Real`; `Suite→{Abstractions, Array, Complex, Natural, Integer, Real}`; `Dsp→{Abstractions, Complex, Integer, Natural, Real}`; `Statistics→Abstractions`; `Studio/Run/Console→{Dsp, Suite}`. (All from the `.csproj` files read directly.)

### 8b. JSON source generation
`JsonSerializerContext` source-gen is used in three places (no `System.Text.Json` reflection fallback):
- `Lovelace.Knowledge\KnowledgeJsonContext.cs:9,28` — `[JsonSourceGenerationOptions(...)] partial class KnowledgeJsonContext : JsonSerializerContext` with 11 `[JsonSerializable]` entries (10-20).
- `Lovelace.Studio\StudioJsonContext.cs:12,28` — `partial class StudioJsonContext`; wired as `TypeInfoResolver = StudioJsonContext.Default` in `Program.cs:16`.
- `Lovelace.Run\Program.cs:224-239` — `RunJsonContext` / `RunJsonPrettyContext` with `[JsonSerializable]` DTOs.

### 8c. Trimming-hostile reflection
- **No `[DynamicallyAccessedMembers]`, `[RequiresUnreferencedCode]`, or `[UnconditionalSuppressMessage]`** anywhere (grep = 0).
- **No `Activator`, no `Assembly.Load`/`LoadFromAssemblyPath`/`GetTypes`/`MakeGenericType`/`Expression.*`** in non-test source (grep = 0).
- `GetType()` appears 5×, all diagnostic `Name` formatting only (`ModusHost.cs:102,116`, `Interpreter.cs:244,821`, `DspPlugin.cs:197`).
- `Assembly` appears once: `Console\Program.cs:7` reads its own version string (`typeof(Program).Assembly.GetName().Version`).
- Plugin loading is **explicit** (no scanning): hosts call `engine.LoadPlugin(new DspPlugin())` (`ReplSession.cs:23`, `Run\Program.cs:106`, `Studio\Session.cs:39`) and `LoadPlugin(new StatisticsPlugin())` in tests.
- No `unsafe` blocks and no `AllowUnsafeBlocks` (grep = 0).

### 8d. AOT assessment
The codebase is **deliberately AOT/trim-friendly**: `IsAotCompatible=true` everywhere, source-gen JSON contexts, explicit plugin wiring, no reflection, no unsafe. **Gap**: no project actually sets `PublishAot`/`IsTrimmable`/`PublishTrimmed`, so the AOT path is not exercised by any build profile; a `Lovelace.Symbolics` project should keep the same discipline (no `Type.GetType`, no `Activator`, prefer explicit registration).

---

## 9. NuGet dependencies

### 9a. Runtime (main) projects
**Zero external NuGet packages** in every main project — only `ProjectReference`s (verified by reading each `.csproj`). Studio uses only framework ASP.NET Core (Web SDK); Console/Run use only framework.

### 9b. Test projects
All 11 test projects use the **same** stack (xUnit):
- `Microsoft.NET.Test.Sdk` **17.14.1**
- `xunit` **2.9.3**
- `xunit.runner.visualstudio` **3.1.4**
- `coverlet.collector` **6.0.4**
- `IsPackable=false`, `net10.0`, `<Using Include="Xunit" />`.

**No property-testing library** (`FsCheck`, `Hedgehog`) in any project (grep = 0). **No BenchmarkDotNet** in any test project.

### 9c. Benchmark projects
`BenchmarkDotNet` **0.15.8** is referenced only in the standalone `precbench` and `dspbench` executables (`precbench.csproj`, `dspbench.csproj`). Other bench tools (`bench`, `mulbench`, `realbench`, `arraybench`) are hand-rolled timing loops with no external packages.

---

## 10. Performance conventions

- **`ArrayPool<T>.Shared`** — used only in `DigitStore` (snapshot rent/return and `ToString` buffer) — `DigitStore.cs:222,244,596,666`. Not used in `Natural`/`Integer`/`Real` (they allocate `ulong[]`/`string`).
- **`Parallel.For`** — `DigitStore.ToString` interior bytes (638); `Natural.Factorial` (544), `Natural.UnbalancedMultiply` (863), `Natural.Ntt` butterfly stages (1039).
- **`Parallel.Invoke`** — Karatsuba 3-way sub-product fork (`Natural.cs:827`).
- **`Task.Run`** — `Natural.ToStringRecursive` fork-join (596); `Real.Sqrt` batch (803), `Real.PiTo` concurrent √10005 + BSP (963, 1008), `Real` async wrappers (1131, 1387, 1403); `Studio.EngineHost` (59).
- **`Interlocked`** — lazy cache publish in `Natural` (156, 166); global precision get/set in `Real` (61-76); `DigitStore` ID counter (33).
- **`CollectionsMarshal.AsSpan`/`SetCount`** — zero-allocation copy in `DigitStore.CopyDigitsFrom` (491-494) and `ToString` snapshot (597).
- **Struct vs class choice**:
  - Classes: `DigitStore`, `Natural` (sealed), `Integer`, `Real`, `Complex` (sealed), `NdArray<T>` (sealed), `DenseArray<T>` (sealed), `Value`.
  - Structs (fast fixed-width exact-decimal path): `LReal64`/`LReal128` (`readonly struct`, `MaxSignificantDigits` 19/38, throw `LRealPromoteException` on overflow — never round), `LComplex64`/`LComplex128` (`readonly struct`), `Precision`/`Slice`/`IndexSpec` (`readonly record struct`), `UInt256` (`internal readonly struct`, `LReal128.cs:10`), `ExactNumber` (`readonly struct`).
- **Zero-GC hot paths**: `Natural` schoolbook/Karatsuba/NTT operate on pooled/stack `ulong[]` with canonical trim (`Make`, `TrimCopy`); fixed-width `LReal*`/`LComplex*` are value types avoiding per-op allocation when precision ≤ 37 (fast-path dispatch in `Complex.Binary`, `Complex.cs:109-123`, and `Suite\NumericOps.ApplyRealBinary`, `NumericOps.cs:73-98`).
- **Source generators**: only the System.Text.Json source generator (three contexts) and the Studio request-delegate generator. **No custom source generator** exists.
- **GC tuning** is set only in bench executables (`ServerGarbageCollection=false`, `TieredPGO=true`, `Optimize=true` in `bench`, `precbench`, `dspbench`) — not in library projects.

---

## 11. The 10 most important architectural facts for planning `Lovelace.Symbolics`

1. **`Integer` is the natural exact coefficient type** — immutable, signed, arbitrary-precision, `ISignedNumber<Integer>`/`INumber<Integer>`, with `DivRem`/`Pow`/`Factorial` (`Integer.cs:15-32,299,327`). It is **not `sealed`** so it can be reused/composed, and `Real` already subclasses it.
2. **There is no public exact `Rational` type.** Exact rational arithmetic exists only as private `Integer`/`Natural` locals inside `PiTo`/`ETo` (`Real.cs:975-1038,1092-1100`); the reusable `readonly struct ExactNumber` (`Knowledge\ExactNumber.cs:18`) is `BigInteger`-based and supports **only Add/Subtract** (no Multiply/Divide). A `Lovelace.Symbolics` rational coefficient type must be built on `Integer` and needs `*`,`/`, and `GCD`.
3. **`GCD`/`LCM` are missing from `Natural` and `Integer`** (grep = 0) — mandatory to add for rational normalization, polynomial content/GCD, and `ExactNumber`-style reduction.
4. **`Natural` is a strong unsigned substrate** (parallel Karatsuba + NTT multiply, Newton/Knuth division, `DivRem`, `Pow`, `Factorial`) but its `TryConvertFrom/To` **throw `NotImplementedException`** (`Natural.cs:697-713`); `Integer`/`Real` versions return `false`. Symbolic import/export must hand-roll bridges to `BigInteger`/`long`.
5. **`Real` is exact only for terminating/periodic decimals** (periodic form = exact rational), and is otherwise a truncated approximation. Its **`Exponent` has a public setter** (`Real.cs:153`) and its equality/`GetHashCode` are **string- and precision-dependent** (`Real.cs:393-463`) — both liabilities if `Real` is used as a shared coefficient or hash key.
6. **`Complex` is not `INumber<Complex>`** (`Complex.cs:13`) and implements only `Exp` among transcendental functions (no `log`/`sqrt`/`pow`/`sin`/`cos`); it has scalar `*`/`/` with `Real` but no `+`/`-`/`==` mixed operators. `ℂ[x]` coefficients would need these filled in.
7. **`DigitStore` (BCD) is dead code** — no production project references it; `Natural` uses binary `ulong[]` limbs. Its `InternalsVisibleTo` grants are only `Lovelace.Representation.Tests` + `Lovelace.Natural`. Exclude from symbolic planning.
8. **The language layer widens `Natural → Integer → Real`** via `Value.Widen`/`WidenPair` (`Suite\Value.cs:184-227`, `DType.cs:10-26`), with `Complex` as an explicit non-widening domain type. Symbolic expressions will need an analogous (but exact) promotion rule, ideally mapping literals to `Integer`/rational rather than `Real`.
9. **AOT discipline is already in place and must be preserved**: `IsAotCompatible=true` on every project, System.Text.Json source-gen contexts, explicit (non-reflective) plugin wiring (`LoadPlugin(new DspPlugin())`), zero `unsafe`, zero `Activator`/assembly scanning, and **no** `[DynamicallyAccessedMembers]`/`[RequiresUnreferencedCode]`/`[UnconditionalSuppressMessage]` anywhere. No project yet sets `PublishAot`/`IsTrimmable`, so a publish profile is the open follow-up.
10. **Tests are xUnit 2.9.3 (SDK 17.14.1) with no property-testing library and no BenchmarkDotNet in test projects** (BenchmarkDotNet 0.15.8 only in `precbench`/`dspbench`). For symbolic correctness work (normalization invariants, term ordering, GCD), adding a property-testing library and co-located benchmarks would be new infrastructure.

---

## Quick reference — file map
- `Lovelace.Representation\DigitStore.cs` — BCD `DigitStore` (legacy/unused).
- `Lovelace.Natural\Natural.cs` — `Natural` (unsigned big-int, binary limbs).
- `Lovelace.Integer\Integer.cs` — `Integer` (signed, magnitude+sign).
- `Lovelace.Real\Real.cs` — `Real : Integer` (exponent + period); `LReal64.cs`/`LReal128.cs` fixed-width.
- `Lovelace.Complex\Complex.cs` — `Complex` (two `Real`s); `LComplex64.cs`/`LComplex128.cs` fixed-width.
- `Lovelace.Abstractions\*.cs` — `ArrayValue`/`DenseArray<T>`/`DType`/`Precision`/`Slice`/`IndexSpec`/`IModus*`.
- `Lovelace.Array\IField.cs` — `IField<T>` (the only field abstraction); `NdArray.cs`, `ArrayMath.cs`.
- `Lovelace.Suite\Value.cs`, `ValueField.cs`, `NumericOps.cs` — language `Value` union + widening + fast-path arithmetic.
- `Lovelace.Knowledge\ExactNumber.cs` — exact rational (`BigInteger`-backed) with Lovelace-literal round-trip.
