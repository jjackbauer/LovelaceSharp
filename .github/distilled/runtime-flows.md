# Runtime Flows

> **Scope**: Key execution paths — parse→compute→format, division period detection, and other critical flows
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-005, OBS-011, OBS-012, OBS-013, OBS-014, OBS-015, OBS-016, OBS-017, OBS-021, OBS-022, OBS-023, OBS-024, OBS-025, OBS-028, VAL-006

---

## Purpose

This document describes the key execution paths through the LovelaceSharp system, including
how input is parsed into arbitrary-precision numbers, how arithmetic operations are computed,
and how results are formatted for output. It also covers specialised flows such as division
with periodic decimal detection. Content is sourced from OBS entries (call chain observations)
and VAL entries that confirm flow behaviour.

---

## Parse → Compute → Format

### String to Real: `Real.Parse("3.14")`

- ✅ `Real.Parse(string)` validates input is non-null, then delegates to `Real.TryParse(ReadOnlySpan<char>)` (Real.cs:964–970) (OBS-021, VAL-006).
- ✅ `TryParse` strips sign (`-` prefix), locates decimal point via `IndexOf('.')`, and splits the input into integer part, non-repeating fractional part, and optional periodic part delimited by parentheses (Real.cs:1002–1055) (OBS-021).
- ✅ All digit parts are concatenated into `allDigits` ("314" for "3.14") and exponent is computed as `-(long)nonRepeating.Length` (= -2 for "3.14") (Real.cs:1071–1081) (OBS-021).
- ✅ `Nat.TryParse(allDigits)` is called to parse the magnitude. Natural processes digits right-to-left (LSD first), calling `DigitStore.SetDigit(i, digit)` for each position (Natural.cs:828–866) (OBS-021).
- ✅ `SetDigit` acquires the per-instance `_syncRoot` lock, determines the target byte and nibble (even position → high nibble, odd → low nibble), and delegates to `SetBitwise` which packs with `(byte)((high << 4) | (low & 0x0F))` (DigitStore.cs:110–131, 276–300) (OBS-021, OBS-027).
- ✅ The result is constructed as `new Real(magnitude, isNegative, exponent, periodStart, periodLength)`, which calls the base `Integer(Nat, bool)` constructor (normalizes sign) and sets the exponent and period metadata (Real.cs:218–225) (OBS-021).

### Full call chain

```
Real.Parse("3.14")
  → Real.TryParse(span)         // strip sign, find '.', split parts, compute exponent=-2
    → Nat.TryParse("314")       // validate chars, iterate R-to-L
      → DigitStore.SetDigit(0, 4)  → SetBitwise(0, 4, 0x0F) → byte[0]=0x4F
      → DigitStore.SetDigit(1, 1)  → SetBitwise(0, 4, 1)    → byte[0]=0x41
      → DigitStore.SetDigit(2, 3)  → SetBitwise(1, 3, 0x0F) → byte[1]=0x3F
    → new Real(mag=314, isNeg=false, exp=-2)
```

**Result**: `Real { Magnitude=314, Exponent=-2, IsNegative=false }` → represents `3.14`.

---

## Division and Period Detection

### `1/3 = 0.(3)` — remainder-tracked period detection

- ✅ `Real.Divide(Real, Real)` computes the sign and exponent adjustment, then performs integer part division via `Nat.DivRem` (Real.cs:487–510) (OBS-022, VAL-006).
- ✅ Fractional digit loop (Real.cs:514–544): each iteration converts the remainder to a string key and checks a `Dictionary<string, long>` for a previously-seen remainder. If found, `periodStart = firstPos` and `periodLength = position - firstPos`; the loop breaks without adding the repeated digit (OBS-022).
- ✅ If no period is found, the remainder is multiplied by 10, `DivRem` produces the next fractional digit, and the digit is appended to a `List<char>`. The loop is capped at `MaxComputationDecimalPlaces` (default 1000) (OBS-022, OBS-012).
- ✅ The result is assembled by concatenating integer digits and fractional digits, parsing them as a Natural magnitude, computing the result exponent, and constructing a new Real with period metadata (Real.cs:546–560) (OBS-022).
- ⚠️ Periodic results **bypass `Normalize()`** to preserve the exact PeriodStart and PeriodLength values (OBS-022).

### Step-by-step for `1 / 3`

```
Sign: false, Exponent adjustment: 0
Integer division: DivRem(1, 3) → quotient=0, remainder=1

Fractional loop:
  Position 0: remainder "1" → not in history → history["1"]=0
              remainder×10=10 → DivRem(10,3) → digit=3, remainder=1
  Position 1: remainder "1" → FOUND at pos 0!
              periodStart=0, periodLength=1, break

Result: magnitude=03→3, exponent=-1, periodStart=0, periodLength=1
Output: "0.(3)"
```

---

## Square Root Computation

### Newton-Raphson with progressive precision doubling

- ✅ `Sqrt(Real value)` seeds from a `double` approximation of the input. Working precision starts at 16 digits and doubles each iteration until reaching the target precision + 50 guard digits (OBS-028, OBS-015).
- ✅ Each iteration computes `x_{n+1} = (x_n + value/x_n) / 2` using `WithLocalPrecision` to scope the division precision to the current working precision, avoiding clobbering global state (OBS-028, OBS-012).
- ✅ Convergence is detected when two consecutive results agree to full precision. The progressive doubling is an optimization: early iterations use cheap low-precision divisions, making the overall flow logarithmic in the number of full-precision operations (OBS-028).

### Pi via Chudnovsky with binary splitting

- ✅ `Pi(long digits)` adds 10 guard digits beyond the requested precision, then computes the number of Chudnovsky terms as `ceil(guardDigits / 14.0) + 2` (OBS-014).
- ✅ Binary splitting partitions the term range across `ProcessorCount` (capped at 64) parallel tasks, each computing `(P, Q, T)` triples via `PiSegment(start, end)`. Results are merged: `T(a,b) = T(a,m)·Q(m,b) + P(a,m)·T(m,b)`, `Q(a,b) = Q(a,m)·Q(m,b)`, `P(a,b) = P(a,m)·P(m,b)` (OBS-014).
- ⚠️ Final computation: `π ≈ 426880 · √10005 · Q / T`. The result is truncated to exactly the requested number of fractional digits (OBS-014).

---

## REPL Evaluation Loop

### `pi(100)` — full pipeline trace

- ✅ `ReplSession.Run()` reads input via `LineEditor.ReadLine("» ")` and feeds it through three stages (OBS-023):

**Stage 1 — Tokenization** (Tokenizer.cs):
- ✅ `Tokenizer.Tokenize("pi(100)")` produces: `[Identifier("pi"), LParen, NumberLiteral("100"), RParen, Eof]` (OBS-023).

**Stage 2 — Parsing** (Parser.cs, recursive descent, 8 precedence levels):
- ✅ `ParsePrimary` matches `Identifier("pi")` followed by `LParen`, entering the function-call path. Builds `CallExpr("pi", [LiteralExpr("100")])` with the argument parsed recursively via `ParseAssignment` (OBS-023).

**Stage 3 — Evaluation** (Evaluator.cs):
- ✅ `Evaluate(CallExpr)` dispatches to `EvaluateCall`, which matches `"pi"` against the 8 built-in functions and invokes `BuiltinPi`. The argument is evaluated as `Nat.Parse("100")`, converted to `long`, and passed to `Real.Pi(100)` (OBS-023).

**Output**:
- ⚠️ The result `Value(Real)` is stored in the `_` variable and printed via `PrintResult` as `= {value.ToString()} (Real)` (OBS-023).

### Built-in functions

- ✅ The Evaluator supports 8 built-in functions: `abs`, `inv`, `divrem`, `is_even`, `is_odd`, `sign`, `sqrt`, `pi` (OBS-016, OBS-023).

### Value type system

- ✅ `ValueKind` enum: Natural(0), Integer(1), Real(2), Boolean(3), Text(4). `WidenPair(a, b)` promotes both operands to `max(a.Kind, b.Kind)`. Literal inference: text containing `.` or `(` → Real; otherwise → Natural (OBS-025).
- ✅ Natural subtraction underflow is caught by the Evaluator, which retries with Integer operands — the only widening not handled by WidenPair (OBS-025, OBS-017).
