# Codebase Patterns Reference

> **Usage**: Include this file in any prompt via `#file:.github/prompts/codebase-patterns.md`.
> This file is a reference-only document — it is not a runnable prompt.
>
> **Purpose**: Codifies implementation and test-writing patterns discovered in the
> LovelaceSharp codebase, structured as generalizable .NET/xUnit conventions annotated
> with project-specific examples. AI sessions should produce idiomatic code without
> re-discovering conventions.

---

## §1 Project Structure Patterns

| Convention | LovelaceSharp Rule |
|---|---|
| Layout | Flat layout — one primary class per project; no nested folders |
| Naming | Library: `Lovelace.<Layer>` → test: `Lovelace.<Layer>.Tests` |
| Solution format | `.slnx` (new XML format) |
| Target | `net10.0` |
| Implicit usings | `<ImplicitUsings>enable</ImplicitUsings>` |
| Nullable | `<Nullable>enable</Nullable>` |
| Test SDK | `Microsoft.NET.Test.Sdk`, `xunit`, `xunit.runner.visualstudio`, `coverlet.collector` |
| Global using | Test `.csproj` files include `<Using Include="Xunit" />` — **no `using Xunit;` in test source files** |

**Generalizable rule**: *Each bounded-context layer gets its own project; test projects mirror library projects 1:1 with an identical namespace suffixed by `.Tests`.*

---

## §2 Dependency Chain & Composition Model

```
Lovelace.Representation  ←  Lovelace.Natural  ←  Lovelace.Integer  ←  Lovelace.Real
```

| Rule | Detail |
|---|---|
| Direction | Lower layers know nothing about upper layers |
| Representation boundary | Only `Lovelace.Representation` reads/writes the backing `byte[]` |
| Composition | `Natural` **composes** `DigitStore`; `Integer` **composes** `Natural` (magnitude + sign) |
| Inheritance exception | `Real` **inherits** from `Integer` (the only inheritance in the chain) |
| Type aliases | `using Nat = global::Lovelace.Natural.Natural;` in Integer; both `Int` and `Nat` aliases in Real |
| `InternalsVisibleTo` | Declared only in `DigitStore.cs` — grants to `Lovelace.Representation.Tests` and `Lovelace.Natural` |

**Generalizable rule**: *Lower layers know nothing about upper layers; cross-layer type references use `using` aliases to avoid namespace collisions.*

---

## §3 Class Layout Template

All four production classes follow a consistent section-banner ordering.
Sections are delimited by a three-line ASCII comment banner:

```csharp
// -------------------------------------------------------------------------
// Section Name
// -------------------------------------------------------------------------
```

**Banner format**: `//` + space + **73 hyphens** (`-`), yielding a 77-character line.

### Canonical section ordering (superset — omit sections that don't apply)

| # | Section | Present in |
|---|---|---|
| 1 | Backing fields / store | All four classes |
| 2 | Static configuration properties | Natural, Real |
| 3 | INumberBase\<T\> — required static properties | Natural, Integer, Real |
| 4 | Instance properties | Real |
| 5 | Constructors | All four classes |
| 6 | Assign | Real |
| 7 | INumberBase\<T\> — classification predicates | Natural, Real |
| 8 | Static predicates / sign property | Integer, Real |
| 9 | Magnitude / conversion helpers | Natural, Integer, Real |
| 10 | IEquatable\<T\> / IComparable\<T\> | Natural, Integer, Real |
| 11 | Comparison operators | Natural, Real |
| 12 | Unary operators (negate, plus) | Natural, Integer, Real |
| 13 | Arithmetic operators | Natural, Integer, Real |
| 14 | Increment / Decrement operators | Natural, Integer |
| 15 | Domain-specific operations (Pow, Factorial, Invert) | Natural, Integer, Real |
| 16 | ISpanFormattable / Formatting | All four classes |
| 17 | IParsable\<T\> / Parsing | Natural, Integer, Real |
| 18 | INumberBase\<T\> — generic conversion stubs | Natural, Integer, Real |

### DigitStore-specific sections

DigitStore uses the same hyphen banner but has its own ordering:

1. Backing fields → 2. Constructors → 3. Public properties → 4. Public digit access
→ 5. Pooled snapshot helpers → 6. Internal BCD infrastructure → 7. Private lock-free "Unsafe" helpers → 8. Formatting

---

## §4 Constructor Patterns

### Mandatory constructor set (all numerical classes)

| Constructor | Pattern |
|---|---|
| Default / zero | `new T()` — always produces zero |
| Copy | `new T(T other)` — deep copy (independent mutability) |
| `string` | Delegates to `Parse(s, null)` then copies backing store |
| `ReadOnlySpan<char>` | Same delegation as `string` |
| Primitive | `ulong` / `int` / `long` / `double` depending on type |

### Delegation conventions

- **String/span constructors** always delegate to `Parse`; they do not implement their own parsing.
- **`int` → `long`** delegation in Integer: `: this((long)value)`.
- **`double` → string → Parse** in Real: `: this(Parse(value.ToString("R", CultureInfo.InvariantCulture), null))`.
- **Copy constructor** performs deep copy — modifying the copy never affects the original.

### `ulong` decomposition loop (Natural)

```
digit = value % 10;  store.SetDigit(pos, digit);
value /= 10;  pos++;
```

Repeats while `value > 0`, building digits from least-significant to most-significant.

---

## §5 Static Property Patterns

### Thread-safe statics

All mutable static configuration properties use `Interlocked.Read` / `Interlocked.Exchange`:

```csharp
private static long _displayDigits = -1L;

public static long DisplayDigits
{
    get => Interlocked.Read(ref _displayDigits);
    set => Interlocked.Exchange(ref _displayDigits, value);
}
```

**Generalizable rule**: *Never use `volatile` for 64-bit fields; use `Interlocked` to guarantee atomicity on 32-bit runtimes.*

### INumberBase\<T\> constants

Every numerical class exposes these five expression-bodied properties:

| Property | Natural | Integer | Real |
|---|---|---|---|
| `One` | `new(1UL)` | `new(Nat.One, false)` | `new(Nat.One, false, 0L)` |
| `Zero` | `new()` | `new()` | `new()` |
| `Radix` | `10` | `10` | `10` |
| `AdditiveIdentity` | `Zero` | `Zero` | `Zero` |
| `MultiplicativeIdentity` | `One` | `One` | `One` |

Integer adds `NegativeOne`; Real shadows all inherited properties with `static new`.

**Pattern**: Always allocate a **new instance** per access (no caching). Derived types use the **`new`** keyword to shadow the base type's property.

---

## §6 Interface Implementation Patterns

### Interface list on class declaration

All numerical classes list interfaces on the type declaration, one per line:

```csharp
public sealed class Natural :
    INumber<Natural>,
    IComparable<Natural>,
    IEquatable<Natural>,
    IParsable<Natural>,
    ISpanParsable<Natural>,
    ISpanFormattable,
    IAdditionOperators<Natural, Natural, Natural>,
    // ... remaining interfaces
```

### Trivial predicate stubs (INumberBase\<T\>)

| Predicate | Return value | Reason |
|---|---|---|
| `IsCanonical(T)` | `true` | BCD representation is always canonical |
| `IsComplexNumber(T)` | `false` | Not a complex number system |
| `IsFinite(T)` | `true` | Always finite |
| `IsImaginaryNumber(T)` | `false` | No imaginary part |
| `IsInfinity(T)` | `false` | No infinity representation |
| `IsNaN(T)` | `false` | No NaN representation |
| `IsNegativeInfinity(T)` | `false` | No negative infinity |
| `IsNormal(T)` | `!IsZero(value)` | Non-zero values are "normal" |
| `IsPositiveInfinity(T)` | `false` | No positive infinity |
| `IsRealNumber(T)` | `true` | All values are real |
| `IsSubnormal(T)` | `false` | No subnormal representation |

Non-trivial predicates (`IsZero`, `IsPositive`, `IsNegative`, `IsEvenInteger`, `IsOddInteger`) contain real logic.

### Conversion stubs

`TryConvertFromChecked`, `TryConvertFromSaturating`, `TryConvertFromTruncating` and the three `TryConvertTo*` variants: all `throw new NotImplementedException(...)` or `return false` as provisional stubs. These are the **only** acceptable surviving `NotImplementedException`s.

---

## §7 Operator Patterns

### General rules

1. **Zero/identity shortcut** at the top of every operator: check for zero or identity inputs and return early.
2. **Immutability**: operators always return a **new instance** — never mutate an operand.
3. **No `#region`**: use ASCII comment banners to group operators by category.

### Style by class

| Class | Operator style |
|---|---|
| Natural | **Inline implementation** — operator body contains the full algorithm (digit loops, carry, etc.) |
| Integer | **Operator-delegates-to-method** — `operator+` calls `left.Add(right)`, etc. |
| Real | **Operator-delegates-to-static-method** — `operator+` calls `Add(left, right)` |

### `new` keyword in Real

Real uses `static new` to shadow all inherited Integer operators so the return type is `Real`:

```csharp
public static new Real operator +(Real left, Real right) => Add(left, right);
```

---

## §8 Error Handling Patterns

| Exception type | When to throw |
|---|---|
| `ArgumentOutOfRangeException` | Invalid argument values (digit out of 0–9, negative int for Natural, negative exponent for Pow) |
| `InvalidOperationException` | Mathematically impossible operation (Natural subtraction would underflow, factorial of negative) |
| `DivideByZeroException` | Division/modulo/invert by zero |
| `FormatException` | Parse receives unparseable input |
| `NotImplementedException` | Conversion stubs, unimplemented operator overloads (only for INumberBase stubs) |
| `ArgumentException` | `CompareTo(object?)` receives wrong type |

**Pattern**: Guard clauses go at the **top** of the method, before any logic. Use `nameof(param)` for the parameter argument in exceptions.

---

## §9 Thread Safety & Parallelism Patterns

| Mechanism | Where | Detail |
|---|---|---|
| `lock (_syncRoot)` | `DigitStore` | Every public/internal mutating and reading method acquires this monitor |
| `*Unsafe()` private variants | `DigitStore` | Same logic without lock; called when caller already holds `_syncRoot` |
| `Interlocked.Increment` | `DigitStore._idCounter` | Monotonic ID for canonical lock ordering in `CopyDigitsFrom` |
| `Interlocked.Read`/`Exchange` | Natural, Real static props | Atomic 64-bit access for mutable configuration |
| `ArrayPool<byte>.Shared` | `DigitStore` | `RentDigitSnapshot` / `ReturnDigitSnapshot` for zero-GC hot paths |
| `Parallel.For` | Natural (`operator*`, `Factorial`), `DigitStore.ToString` | Used only when work exceeds a processor-count threshold |
| Canonical lock ordering | `DigitStore.CopyDigitsFrom` | Acquires locks on `_id` order to prevent ABBA deadlocks |
| `CollectionsMarshal.AsSpan` | `DigitStore` | Zero-allocation span-based bulk copies |

**Anti-pattern**: Never use `volatile` on 64-bit fields — `Interlocked` is the correct tool.

---

## §10 Test File Structure

### File naming

`<ClassName><Topic>Tests.cs` — examples:

- `NaturalAddTests.cs`, `NaturalConstructorTests.cs`, `NaturalStaticPropertyTests.cs`
- `IntegerAddTests.cs`, `IntegerEqualityTests.cs`, `IntegerFactorialTests.cs`
- `RealDivideTests.cs`, `RealParseTests.cs`, `RealToStringTests.cs`
- `DigitStoreTests.cs`, `DigitStoreMutatorTests.cs`, `DigitStoreThreadSafetyTests.cs`

### File layout

```csharp
using Lovelace.<Layer>;                    // Single project reference (+ alias if needed)

namespace Lovelace.<Layer>.Tests;          // File-scoped namespace

/// <summary>
/// Functional tests for <see cref="ClassName"/> <Topic>.
/// Checklist item: "..."
/// </summary>
public class <ClassName><Topic>Tests
{
    // -------------------------------------------------------------------------
    // Section Title
    // -------------------------------------------------------------------------

    [Fact]
    public void MethodName_GivenScenario_ExpectedResult()
    { ... }
}
```

### Key rules

- **One `using` directive** — only the project namespace under test. No `using Xunit;` (SDK global using handles it).
- **File-scoped namespace** — always, never block-scoped.
- **XML `<summary>`** on the class — references the method under test via `<see cref="..."/>` and names the corresponding checklist item.
- **Section banners** — same 73-char hyphen format as production code. Separate conceptual groups of tests.

### Known inconsistencies (document, don't replicate)

- `RealParseTests.cs` and `NaturalStaticPropertyTests.cs` include `using Xunit;` — this is unnecessary and should not be replicated.
- DigitStore Representation tests use `=` banners instead of `-` — the established standard is `-`.

---

## §11 Test Naming Convention

**Format**: `MethodName_GivenScenario_ExpectedResult`

Three underscore-separated parts:

| Part | Content | Examples |
|---|---|---|
| MethodName | The method, constructor, operator, or property being tested | `Add`, `Constructor`, `OperatorPlus`, `Parse`, `ToString`, `TryFormat`, `DisplayDigits` |
| GivenScenario | Describes the input condition, prefixed with `Given` | `GivenTwoPositiveNumbers`, `GivenZeroAndN`, `GivenCarryPropagation`, `GivenDivisorZero` |
| ExpectedResult | The observable outcome | `ReturnsCorrectSum`, `ProducesZero`, `ThrowsDivideByZeroException`, `IsTrue`, `PreservesPeriod` |

#### Examples by category

| Category | Name |
|---|---|
| Happy path | `Add_GivenTwoPositiveNumbers_ReturnsCorrectSum` |
| Identity | `Add_GivenZeroAndN_ReturnsN` |
| Boundary | `Constructor_GivenUlongMaxValue_StoresAllDigits` |
| Exception | `Divide_GivenDivisorZero_ThrowsDivideByZeroException` |
| Round-trip | `Parse_GivenToStringOutput_ProducesEqualValue` |
| Algebraic property | `Add_IsCommutative_GivenAnyTwoValues` |
| Large number | `Multiply_GivenBeyondUlongRange_ReturnsCorrectProduct` |

---

## §12 Test Method Anatomy

### Attribute usage

- **`[Fact]`** for single-case tests — the vast majority of tests. Zero blank lines between attribute and method.
- **`[Theory]` + `[InlineData]`** only for parameterized tests with the same assertion pattern across multiple inputs.

```csharp
[Theory]
[InlineData(9UL,   1UL,   "10")]
[InlineData(99UL,  1UL,   "100")]
[InlineData(999UL, 1UL,   "1000")]
public void Add_GivenCarryPropagation_ProducesCorrectResult(ulong a, ulong b, string expected)
```

- `[InlineData]` values are **column-aligned** with spaces.
- No blank line between `[Theory]`/`[InlineData]` block and the method signature.

### Spacing

- **One blank line** between methods.
- **No blank line** between attribute and method signature.
- **Implicit AAA** — no `// Arrange / Act / Assert` comments. Short tests may have no internal blank lines; longer tests use a blank line between logical sections.

### Variable naming preferences

| Context | Names |
|---|---|
| Binary ops (Natural, Real) | `left`, `right`, `result` (or `sum`, `product`, `quotient`) |
| Binary ops (Integer) | `a`, `b`, `result` |
| Single subject | `n`, `r`, `store` |
| Copy tests | `original`, `copy` |
| Expected string | `expected` |
| Boolean result | `ok` |
| Saved state | `saved` |

### Column alignment

Natural and Real tests **column-align** multi-variable declarations:

```csharp
var left  = new Natural(123UL);
var right = new Natural(456UL);
var sum   = left + right;
```

Integer tests do **not** column-align. Follow the style of the project you're working in.

### Literal suffixes

| Type | Suffix | Example |
|---|---|---|
| `ulong` | `UL` (uppercase) | `123UL`, `0UL` |
| `long` | `L` (uppercase) | `3L`, `-7L` |
| `double` | `.0` | `3.14`, `0.0` |
| Real values | string-constructed | `new Real("3.14")`, `Real.Parse("0.(3)")` |

---

## §13 Assertion Patterns

### Primary pattern — string comparison

```csharp
Assert.Equal("12345", n.ToString());
```

Used when equality is via `ToString()` (especially in Natural).

### Structural equality

```csharp
Assert.Equal(new Integer(7L), result);
Assert.Equal(new Real("3.8"), result);
Assert.Equal(Real.Parse("0.(3)"), result);
```

Used when the type has a reliable `Equals` implementation.

### Predicate assertions

```csharp
Assert.True(Natural.IsZero(n));
Assert.False(Integer.IsNegative(value));
Assert.True(result.IsPeriodic);
```

Always use the **static** predicate form for `INumber<T>` predicates.

### Exception assertions

```csharp
Assert.Throws<DivideByZeroException>(() => a / b);
Assert.Throws<FormatException>(() => Real.Parse("abc"));
Assert.Throws<InvalidOperationException>(() => a - b);
```

Never use `Record.Exception` — always use `Assert.Throws<T>`.

### DivRem output assertions

```csharp
Natural.DivRem(a, b, out var remainder);
Assert.Equal("quotient", result.ToString());
Assert.Equal("remainder", remainder.ToString());
```

### Zero in assertions

Use `new T()` (parameterless constructor) for zero values:

```csharp
Assert.Equal(new Natural(), result);
```

---

## §14 Test Coverage Checklist

Every method group must cover these categories:

| Category | Description |
|---|---|
| Zero inputs | At least one operand is zero |
| Identity elements | Operation with identity returns the other operand unchanged |
| Carry/borrow propagation | Inputs that trigger cascading carries or borrows (e.g. 999 + 1) |
| Beyond native range | Values exceeding `ulong.MaxValue` (20+ digits) |
| Algebraic properties | Commutativity, associativity where applicable |
| Exception cases | All documented exception paths (divide-by-zero, negative underflow, parse errors) |
| Parse/format round-trips | `Parse(x.ToString()) == x` |
| Leading-zero trimming | Results should never have spurious leading zeros |
| Sign combinations | For signed types: +/+, +/−, −/+, −/− |
| Periodic values | For Real: operations involving periodic decimals, period detection, period preservation |

### Static property test pattern

Use `try`/`finally` to save and restore mutable static properties:

```csharp
[Fact]
public void DisplayDigits_AfterSet_ReturnsSetValue()
{
    long saved = Natural.DisplayDigits;
    try
    {
        Natural.DisplayDigits = 42L;
        Assert.Equal(42L, Natural.DisplayDigits);
    }
    finally
    {
        Natural.DisplayDigits = saved;
    }
}
```

---

## §15 Anti-Patterns (Do NOT)

| Anti-pattern | Correct alternative |
|---|---|
| **Scaffolding tests** — `typeof(T).IsAssignableFrom(...)`, type-name checks | Always test behaviour with concrete inputs and assertions |
| **Weakened assertions** — replacing `Assert.Equal("12345", ...)` with `Assert.NotNull(n)` | Fix the implementation, not the test (unless Falsify Claims confirms the expectation was wrong) |
| **`NotImplementedException` surviving past implementation** | Only acceptable in `TryConvertFrom/To` stubs |
| **Raw `byte[]` access outside Representation** | Use `GetDigit` / `SetDigit` through `DigitStore` |
| **`#region` blocks** | Use ASCII comment banners with 73 hyphens |
| **`volatile` on 64-bit fields** | Use `Interlocked.Read` / `Interlocked.Exchange` |
| **`using Xunit;`** in test files | Rely on the global `<Using Include="Xunit" />` in the `.csproj` |
| **`Record.Exception`** | Use `Assert.Throws<T>` |
| **Caching static `One`/`Zero` instances** | Always allocate new (expression-bodied property `=> new(...)`) |
| **Mutating operands in operators** | Always return a new instance |
| **Block-scoped namespaces** | Use file-scoped namespaces (`;` terminated) |
