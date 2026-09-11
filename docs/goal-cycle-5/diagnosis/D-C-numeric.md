# D-C-numeric — Diagnosis of Tier-0 numeric defects (cycle 5)

**Round:** diagnosis only. **Method:** static source read + the audit's own recorded probe outputs.
**No build, test, `dotnet`, or `Lovelace.Run.exe` was executed in this round** (the orchestrator holds
the binary/locks). Every behavioural statement below is either **OBSERVED** (quoted from a recorded
probe output under `docs/goal-cycle-5/probes/pre2/`), **READ** (quoted source), or explicitly marked
**PREDICTED** / **OPEN**.

**Scope read:** `Lovelace.Real/Real.cs`, `Lovelace.Real/LReal64.cs`, `Lovelace.Real/LReal128.cs`,
`Lovelace.Real/RealField.cs`, `Lovelace.Rational/Rational.cs`, `Lovelace.Suite/NumericOps.cs`,
`Lovelace.Suite/Interpreter.cs`, `Lovelace.Suite/ValueFormatter.cs`,
`Lovelace.Symbolics/RationalReal.cs`, plus read-only the exact-flag producer
`Lovelace.Suite/StructuredProjection.cs`, the wire host `Lovelace.Run/Runner.cs`, the contract tests
in `Lovelace.Real.Tests/**` and `Lovelace.Suite.Tests/**`, and the recorded probes.

---

## 0. Evidence base

| id | what | evidence |
|----|------|----------|
| E1 | `2^-100000` → `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}`, top-level `ok:true` | OBSERVED `docs/goal-cycle-5/probes/pre2/a03-pow-neg100000.out.txt` |
| E2 | `2^-5000` → same zero + `exact:true` | OBSERVED `.../a05-pow-neg5000.out.txt` |
| E3 | `10^-1001` → same zero + `exact:true` | OBSERVED `.../a06-pow-ten-neg1001.out.txt` |
| E4 | `0^(-1.0)` → `0`, `exact:true` | OBSERVED `.../a07-zero-pow-realneg1.out.txt` |
| E5 | `0^-1` → `ok:false`, `code:"InvalidArgument"`, `category:"TypeMismatch"`, `message:"Base cannot be zero. (Parameter 'base')"`, `recoverable:true` | OBSERVED `.../a08-zero-pow-intneg1.out.txt` |
| E6 | `(1/17)*17` → `"0."` + 1000 fractional digits; measured: display length **1002**, fraction length **1000**, **998** of them `'9'`, tail `999999999984` | OBSERVED `.../a09-recip17-times17.out.txt` (digit counts measured from the recorded JSON) |
| E7 | `x = (1/17)*17; x == 1` → `{"kind":"Boolean","value":"false"}` | OBSERVED `.../a11-eq-one.out.txt` |
| E8 | `1/17` → `"0.(0588235294117647)"`, `exact:true`, `numerator:"1"`, `denominator:"17"` | OBSERVED `.../a12-recip17.out.txt` |
| E9 | `inspect((1/17)*17)` → field `exact` is `{"kind":"Null"}` for a Real | OBSERVED `.../a10-inspect-recip.out.txt` |

The numbers in E6 pin the mechanism exactly (see section 2): `1 − 16·10^-1000 = 0.` + 998 nines + `84`.

---

## 0.1 ALSO RECORD — what `Real` stores for a periodic value

**A digit string with a period marker, not a fraction — but the fraction is exactly recoverable.**

Base storage (`Integer`, inherited):

- `Lovelace.Integer/Integer.cs:38-39`
  `private readonly Nat _magnitude;`
  `private readonly bool _isNegative;`

Period metadata (`Real`):

- `Lovelace.Real/Real.cs:153` — `public long Exponent { get; set; }`
- `Lovelace.Real/Real.cs:160` — `public long PeriodStart { get; private set; }`
- `Lovelace.Real/Real.cs:166` — `public long PeriodLength { get; private set; }`
- `Lovelace.Real/Real.cs:172` — `public bool IsPeriodic => PeriodLength > 0;`

The magnitude stores **exactly one period block**:

- `Lovelace.Real/Real.cs:1782-1784` (parser): `// Layout: integerPart + nonRepeating + periodic (exactly one copy of the period)` / `string allDigits = integerPart.ToString() + nonRepeating.ToString() + periodic.ToString();`
- `Lovelace.Real/Real.cs:1796-1797`: `// Fractional digits stored = nonRepeating + one period block.` / `exponent = -(periodStart + periodLength);`
- `Lovelace.Real/Real.cs:645-657` (division) sets the same shape: `new Real(mag, actualNeg, resultExponent, foundPeriod ? periodStart : 0L, foundPeriod ? periodLength : 0L)`.

Consequences:

1. `1/17` is stored as magnitude `588235294117647`, `Exponent = -16`, `PeriodStart = 0`, `PeriodLength = 16` (READ: Real.cs:602-657) — E8 shows the round trip back out is exact (`1/17`, `exact:true`).
2. The exact rational is recoverable **today** without any storage change:
   `Lovelace.Symbolics/RationalReal.cs:58-61` — `if (value.IsPeriodic) return Rat.ParsePeriodic(value.ToString());   // period notation is never truncated`.
3. A periodic value's stored magnitude can hold fewer digits than `Exponent` names: `GetDecimalDigit` returns `0` when the computed index falls outside the stored digits (`Real.cs:1999-2012`, e.g. `if (idx < 0 || idx >= (long)digits.Length) return 0;`), which is how the leading `0` of `1/17 = 0.(0588…)` survives `Nat.TryParse` stripping it. Any new code must read digits through `GetDecimalDigit`/`FracDigitString` (`Real.cs:1993-2025`) and never by slicing the string.

**Fix-cost implication:** the fix for DEFECT 2 needs no new field and no new representation — the stored
`(magnitude, Exponent, PeriodStart, PeriodLength)` already denotes the exact rational; only the *arithmetic
branches* need to stop expanding it into a finite decimal first.

---

## 0.2 Where the wire's `exact` flag is decided (asked by DEFECT 1)

For a scalar Real the **only** producer is a shape heuristic on the finished value:

- `Lovelace.Suite/StructuredProjection.cs:110-113`
  `private static StructuredValueDto Real(Rl value)`
  `{`
  `    bool exact = RealExact(value);`
  `    var dto = new StructuredValueDto("Real", Value: value.ToString(), Exact: exact);`
- `Lovelace.Suite/StructuredProjection.cs:126-131`
  `/// <summary>A Real is exact when it is a finite/periodic decimal the kernel carries exactly;`
  `/// a truncated irrational is an approximation and must not advertise exactness.</summary>`
  `private static bool RealExact(Rl value) =>`
  `    // zero is exact however it was produced: 1/3 - 1/3 carries the negative exponent of the`
  `    // subtraction, and reading that as "approximate" told an agent that 0 was not exact`
  `    Rl.IsZero(value) || value.IsPeriodic || -value.Exponent <= 18;`

Wire path: `Lovelace.Run/Runner.cs:204` — `StructuredProjection.ToStructured(result, PrintBudgetOrNull(printBudget)));`
(and the IDE host `Lovelace.Studio/EngineHost.cs:116,142,291,301,337`). `Lovelace.Suite/ValueFormatter.cs`
never touches exactness (READ in full: only display text and type suffixes), so there is no second
scalar-Real producer.

**Answer to "computed from the operation, or hardcoded per type?": neither.** It is a *shape test on the
result* (zero ⇒ exact; periodic ⇒ exact; at most 18 stored fractional digits ⇒ exact). Nothing in
`Real` records provenance. Per-kind constants exist only one level up:

- `StructuredProjection.cs:57-58` — `ValueKind.Natural => … Exact: true`, `ValueKind.Integer => … Exact: true` (genuinely hardcoded per type);
- symbolic kernel: `Lovelace.Symbolics/Constructors.cs:47-54` `Rational(...)` sets `n._isExact = true;` while `Constructors.cs:60-67` `Real(RealLiteral value)` sets `n._isExact = false;` — again per node kind, not per operation.

Two further consequences worth recording:

- `Interpreter.ExactOf` returns **Void** for Real: `Lovelace.Suite/Interpreter.cs:1097-1102`
  `private static Value ExactOf(Value v) => v.Kind switch { ValueKind.Symbolic => new Value(v.AsSymbolic().IsExact), ValueKind.Natural or ValueKind.Integer => new Value(true), _ => Value.Void, };`
  hence E9: `inspect(...).exact` is `Null` for a Real, while the structured wire for the *same* value reports `exact:true`. The two surfaces disagree by construction.
- The same `-Exponent <= 18` shape test is duplicated in the Symbolics bridge:
  `Lovelace.Suite/NumericOps.cs:163-167` — `/// <summary>Exact Real literals become rationals (INV-09); truncated values stay approximate.</summary>` / `r.IsPeriodic || -r.Exponent <= 18 ? Exprs.Rational(RationalReal.FromReal(r)) : Exprs.Real(RealLiteral.FromRealExact(r));`
  So an 18-digit-truncated irrational would be promoted to an **exact** `RationalConstantExpr` (`_isExact = true`) on the symbolic path. (READ; not exercised by the recorded probes — see OPEN-4.)

---

## 1. DEFECT 1 (T0-3) — underflow to a zero labelled `"exact":true`, and `0^(-1.0)` returning 0

### 1.1 Symptom

- `2^-100000` → `0 (Real)`, structured `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` (E1). Same for `2^-5000` (E2) and `10^-1001` (E3).
- `0^(-1.0)` → `0 (Real)`, `exact:true` (E4), while `0^-1` is the typed refusal `InvalidArgument / TypeMismatch / "Base cannot be zero. (Parameter 'base')"` (E5).

### 1.2 Repro commands

`docs/goal-cycle-5/probes/pre2/a03-pow-neg100000.ls` = `2^-100000`;
`a05-pow-neg5000.ls` = `2^-5000`; `a06-pow-ten-neg1001.ls` = `10^-1001`;
`a07-zero-pow-realneg1.ls` = `0^(-1.0)`; `a08-zero-pow-intneg1.ls` = `0^-1`.
(Recorded outputs alongside each `.ls`; this round did not re-run them.)

### 1.3 Root cause

**Stage 1 — the negative exponent is handled only in NumericOps, not in `Real.Pow`.**

- `Lovelace.Suite/Parser.cs:361` (`//     x^-2   ==  x^(-2)      (the exponent may carry its own sign)`) and `Parser.cs:388-389` (`return new BinaryExpr(left, BinaryOp.Power, ParsePower());`) ⇒ `2^-100000` parses as `2^(-100000)`.
- `Lovelace.Suite/Interpreter.cs:571-573` evaluates the unary minus to an **Integer**: `ValueKind.Natural => new Value(-operand.Widen(ValueKind.Integer).AsInteger()),`.
- `Lovelace.Suite/NumericOps.cs:38` `(left, right) = Value.WidenPair(left, right);` then `NumericOps.cs:56` `(BinaryOp.Power, ValueKind.Integer) => IntegerPower(left.AsInteger(), right.AsInteger()),`.
- **The negative-exponent power implementation is `NumericOps.IntegerPower`, `Lovelace.Suite/NumericOps.cs:258-267`:**

      private static Value IntegerPower(Int baseValue, Int exponent)
      {
          if (!Int.IsNegative(exponent) || Int.IsZero(baseValue))
              return new Value(baseValue.Pow(exponent));

          var magnitude = baseValue.Pow(exponent.Negate());
          return new Value(Rl.Divide(
              new Value(Int.One).Widen(ValueKind.Real).AsReal(),
              new Value(magnitude).Widen(ValueKind.Real).AsReal()));
      }

  So `2^-100000` = `Real.Divide(1, 2^100000)` (lines 264-266). `Real.Pow` is **not** on this path at all; it refuses negative exponents (`Real.cs:754-755`, see stage 3), so all negative exponents flow through this reciprocal construction.

**Stage 2 — the branch that returns zero: the capped fractional-digit loop in `Real.Divide`.**

- Cap default: `Lovelace.Real/Real.cs:44` — `private static long _maxComputationDecimalPlaces = 1000L;`
- **The truncation seam — `Lovelace.Real/Real.cs:613`:**

      while (!Nat.IsZero(remainder) && position < MaxComputationDecimalPlaces)

  The loop stops after `MaxComputationDecimalPlaces` (1000) fractional digits even though `remainder` is still non-zero; nothing records that it stopped early (contrast the period exit, which sets `foundPeriod` at lines 617-623).
- The result is assembled at `Real.cs:649-658` (`long resultExponent = -fracLen;` … `return foundPeriod ? divResult : Normalize(divResult);`) and `Normalize` deliberately refuses to touch a zero — `Real.cs:2121-2123`:

      if (r.IsPeriodic || r.Exponent >= 0 || Int.IsZero(r))
          return r;

  So the truncation leaves a **non-canonical zero with `Exponent = -1000`**.
- Arithmetic of the three repros (the first significant digit sits past the 1000-digit budget, so every emitted digit is `0`):
  `2^-100000 = 10^(-100000·log10 2) = 10^-30102.9996` → leading digit at fractional index 30102;
  `2^-5000 ≈ 10^-1505.1` → index 1505; `10^-1001` → index **1000**, exactly one past the last digit the loop can emit. Consistent with E1-E3 all being zero — and with `10^-1000` (index 999) still being representable.

**Stage 3 — `0^(-1.0)`: the zero-base return precedes the negative-exponent refusal in `Real.Pow`.**
`Lovelace.Real/Real.cs:729-755`:

    public Real Pow(Real exponent)
    {
        // x^0 = 1 for any base (including zero).
        if (Real.IsZero(exponent)) return Real.One;

        // 0^n = 0 for any positive exponent.
        if (Real.IsZero(this)) return Real.Zero;          // line 735 — fires for ANY exponent
        …
        if (n < 0)
            throw new NotImplementedException("Negative exponents are not yet supported.");   // lines 754-755

The `n` sign is only known after the string parse at lines 739-752, so a zero base with a negative Real exponent returns `0` before the refusal can run. The Integer path has no such hole because the zero base is delegated to `Int.Pow`, and that throws:
`Lovelace.Suite/NumericOps.cs:260-261` (`if (!Int.IsNegative(exponent) || Int.IsZero(baseValue)) return new Value(baseValue.Pow(exponent));`) →
`Lovelace.Integer/Integer.cs:375-382`:

    if (IsZero(this))
        …
        throw new ArgumentOutOfRangeException("base", "Base cannot be zero.");
    if (exponent <= Zero)
        throw new ArgumentOutOfRangeException(nameof(exponent), "Exponent must be positive.");

That is exactly E5, and `NumericOps.cs:252-256` already states the intended rule: *"A zero base is delegated to `Int.Pow`: `0^-n` has no value in any field … so there is exactly one spelling of that failure in the codebase."* `PowerReal` (`NumericOps.cs:87-94`) adds no guard of its own.

**Stage 4 — the `"exact":true` label.** With the value being a zero, `RealExact` short-circuits on its first clause: `Rl.IsZero(value) || value.IsPeriodic || -value.Exponent <= 18` (`StructuredProjection.cs:131`). Note the *same* value would be `exact:false` if that clause were absent (`IsPeriodic` is false — no period was detected; `-Exponent = 1000 > 18`). The zero carve-out exists for a legitimate case (`1/3 - 1/3` also yields a non-canonical zero; comment at `StructuredProjection.cs:129-130`), which is why a shape test cannot separate the two.

### 1.4 Minimal fix proposal

1. **`Real.Pow` ordering (reorder + 1 line) — `Real.cs:729-755`.** Parse `n` first, then:
   `if (n < 0) { if (Real.IsZero(this)) throw new ArgumentOutOfRangeException("base", "Base cannot be zero."); throw new NotImplementedException("Negative exponents are not yet supported."); }`
   and keep `if (Real.IsZero(this)) return Real.Zero;` only for `n > 0` (this preserves `RealPowTests.Pow_GivenBaseZeroExponentPositive_ReturnsZero`, `RealPowTests.cs:35-42`). Optionally implement `n < 0` as `Real.One / Pow(-n)`, which would also close the gap that `2.0^-1` currently throws `NotImplementedException` while `2^-1 = 0.5`.
2. **`NumericOps.PowerReal` guard — `NumericOps.cs:87-94`:** refuse a zero base with a negative exponent before calling `Pow`, with the same typed error as the Integer path, so the Suite has one spelling of the refusal.
3. **Underflow honesty — `Real.Divide` (`Real.cs:613`).** Choose one and state it:
   - (3a) **refuse**: when the loop exits with `remainder != 0`, `!foundPeriod`, and the accumulated magnitude is zero, throw a typed "result below the active computation precision" error (a value the representation cannot hold must not be silently returned as 0);
   - (3b) **widen the range**: not viable at fixed cost — `2^-100000` needs ~30 103 digits;
   - (3c) **approximate**: return 0 but *labelled* approximate. Impossible today without (4).
4. **Give exactness provenance (the real fix for the labelling half).** Add a truncation bit to `Real` (e.g. `internal bool IsTruncated`, set by `Divide` when the cap is hit at line 613 and by `Sqrt`/`Pi`/`Exp`/trig where they truncate) and make `RealExact` read it: `!value.IsTruncated && (value.IsPeriodic || -value.Exponent <= 18)`, keeping the `IsZero` carve-out only for non-truncated zeros. Note the carve-out **cannot** be replaced by an exponent test, because `1/3 - 1/3` legitimately produces a zero carrying `Exponent = -1000` (comment at `StructuredProjection.cs:129-130`; READ: `Add`'s periodic branch at `Real.cs:2318-2325` computes `resultExp = Math.Min(exA, exB)` at line 2300 and `Normalize` leaves the zero alone). There is no value-level signal that separates the two zeros.

**Primary files:** `Lovelace.Real/Real.cs` (Pow 729-768, Divide 613/649-658), `Lovelace.Suite/NumericOps.cs` (256-267, 87-94), `Lovelace.Suite/StructuredProjection.cs` (110-131). **Not** `ValueFormatter.cs` (no exactness logic).

### 1.5 Test that would FAIL on the current tree

- `Lovelace.Suite.Tests` (new file or extend `ExactnessPreservationTests2`): `Assert.False(Structured("2^-100000").Exact);` — today the probe shows `exact:true` (E1) ⇒ **FAILS**. Same for `2^-5000`, `10^-1001`. A value assertion `Assert.NotEqual("0", sv.Value)` is the stronger statement (it pins 3a/3b over 3c).
- `Lovelace.Suite.Tests`, mirroring `HyperbolicBuiltinsAndNegativeIntegerPowersTests.ZeroToTheMinusOne_IsATypedRecoverableErrorThatNamesTheBase` (`Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:181-191`):
  `var ex = Assert.Throws<ArgumentOutOfRangeException>(() => engine.Evaluate("0^(-1.0)")); Assert.Equal("base", ex.ParamName);` — today it returns `0` (E4) ⇒ **FAILS**.
- `Lovelace.Real.Tests` (`RealPowTests.cs`): `Assert.Throws<ArgumentOutOfRangeException>(() => new Real("0").Pow(new Real("-1.0")));` ⇒ **FAILS** (returns `Real.Zero`).
- `Lovelace.Real.Tests` (`RealDivideTests.cs`): `Assert.False(Real.IsZero(Real.One / new Real(Int.Pow(new Int(2), new Int(100000)))));` ⇒ **FAILS** (returns a zero with `Exponent = -1000`). If (3c) is chosen instead, the correct test is `Assert.False(StructuredProjection.ToStructured(v).Exact)`.

### 1.6 Blast radius

- **Lovelace.Real.Tests** — `RealPowTests.cs` (zero-base positive-exponent case at lines 35-42 must stay green), `RealDivideTests.cs` (cap/period exit), `RealToStringTests.cs` (a non-canonical zero prints `"0"`; `Real.cs:1859` `if (Int.IsZero(this)) return "0";`), `RealParseTests.cs`, `RealPropertiesTests.cs` (defaults), `LReal64Tests.cs`/`LReal128Tests.cs` if the cap semantics change for the fast tiers.
- **Lovelace.Suite.Tests** — `ExactnessPreservationTests2.cs` (its case list at lines 24-33 is the natural home for the new theory rows), `ExactRationalPayloadTests2.cs` (the `Exact`/`Numerator`/`Denominator` payload contract, lines 26-53), `InterpreterBinaryArithmeticTests.cs` (power at lines 130-140, non-exact division widening at 208-230), `LRealDispatchTests.cs` (fast path must keep matching class `Real`).
- **Lovelace.Symbolics.Tests** — `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs` (the negative-power contract: `2^-1`, `3^-2`, `0^-1`, `4^-1 * 4^2`, `x^-1`, lines 113-191), `NegativePowerEvaluationTests.cs` (pinned digits), `CapabilitiesBuiltinTests.cs:227` (`"0^-1;"` is listed as a live refusal — a new Real-base refusal spelling may need adding), `FalsificationGateTests.cs` (uses `x^-1` poles).
- **Lovelace.Run.Tests / Lovelace.Studio.Tests** — any golden payload containing `"exact":true` for a Real; `Lovelace.Studio.Tests/StructuredPayloadTests.cs` reads structured fields.

### 1.7 Open questions

- **OPEN-1 (oracle number in the ticket).** The ticket expects "SymPy: 1.2e-9062" for `2^-100000`, but `2^-100000 = 10^(-100000·log10 2) = 10^-30102.9996 ≈ 1.0010e-30103`. `1.2e-9062` corresponds to about `2^-30103`. Either the quoted oracle belongs to a different expression or it is off; the defect (non-zero → `0`) stands either way. Worth confirming with the audit owner before writing the expected value into a test.
- **OPEN-2.** Is `MaxComputationDecimalPlaces = 1000` the published floor, and should exceeding it be a refusal or an approximation? `docs/Language.md:808-810` says *"exact operations never lose digits; transcendentals truncate at the active budget"* — this case is neither: an exact rational operation (`2^-100000` via `Int.Pow` + `Divide`) silently loses every digit.
- **OPEN-3.** Should `inspect(x).exact` agree with the structured wire for Real? Today it is `Null` (`Interpreter.cs:1097-1102`), and `RecordSchemas.cs:122` allows `Boolean|Null`, so it may be deliberate — but a diagnosis tool that cannot show the flag the wire publishes is a trap for the next auditor.
- **OPEN-4 (adjacent, not yet reproduced).** The `-r.Exponent <= 18` test at `NumericOps.cs:165` promotes a Real to `Exprs.Rational` (exact) purely by digit count, so a ≤18-digit *truncation* would become an exact rational in the kernel. No probe covers it; flagged for a separate check.

---

## 2. DEFECT 2 (T0-4) — `(1/17)*17 ≠ 1`

### 2.1 Symptom

`(1/17)*17` yields a 1000-fractional-digit decimal, 998 nines then `…9984` (E6), and
`x = (1/17)*17; x == 1` is `False` (E7). `1/17` on its own is exactly right: `0.(0588235294117647)`,
`exact:true`, `1/17` (E8).

### 2.2 Repro commands

`docs/goal-cycle-5/probes/pre2/a09-recip17-times17.ls` = `(1/17)*17`;
`a11-eq-one.ls` = `x = (1/17)*17; x == 1`; `a12-recip17.ls` = `1/17`.

### 2.3 Root cause

**Step 1 — `1/17` is exact.** `NumericOps.DivideNatural` (`NumericOps.cs:269-278`) routes a non-exact
quotient to `Rl.Divide`, whose remainder-tracked loop is exact:
`Lovelace.Real/Real.cs:606-636` — `var remainderHistory = new Dictionary<string, long>();` (line 607),
`if (remainderHistory.TryGetValue(remKey, out long firstPos)) { periodStart = firstPos; periodLength = position - firstPos; foundPeriod = true; break; }` (617-623).
Remainder `1` recurs at position 16, so the stored value is `Real(mag=588235294117647, Exponent=-16, PeriodStart=0, PeriodLength=16)` (lines 655-657) — exactly one period block (section 0.1).

**Step 2 — the multiply path expands the periodic operand into a finite decimal.**
`Lovelace.Real/Real.cs:506-531` (the whole periodic branch):

    public static Real Multiply(Real left, Real right)
    {
        bool eitherPeriodic = left.IsPeriodic || right.IsPeriodic;
        long resultExp = left.Exponent + right.Exponent;

        if (!eitherPeriodic)
        {
            …
            return Normalize(new Real(product.ToNatural(), Int.IsNegative(product), resultExp));
        }
        else
        {
            // Periodic path: expand both operands to MaxComputationDecimalPlaces fractional
            // digits (resolving any period), multiply via the non-periodic path, then detect
            // a repeating suffix in the result and normalise (including 0.999… → 1).
            long workingFrac   = MaxComputationDecimalPlaces;
            Real expandedLeft  = ExpandToNonPeriodic(left,  workingFrac);
            Real expandedRight = ExpandToNonPeriodic(right, workingFrac);
            Real rawProduct    = Multiply(expandedLeft, expandedRight); // non-periodic path
            return DetectAndNormalizePeriod(rawProduct);
        }
    }

**The truncation constant and its location: `Real.cs:525` — `long workingFrac = MaxComputationDecimalPlaces;`,
defaulting to `1000L` at `Real.cs:44` (`private static long _maxComputationDecimalPlaces = 1000L;`).**

`ExpandToNonPeriodic` materialises that finite decimal and throws the period away:
`Lovelace.Real/Real.cs:2053-2068` — `string fracPart = r.FracDigitString(fracDigits);` (2061) …
`return new Real(mag, isNeg, -fracDigits);` (2068). (It expands the *non-periodic* operand too — `17` becomes `17×10^1000` with `Exponent = -1000` — harmless for the value, but it doubles the exponent bookkeeping.)

**Step 3 — arithmetic of the failure.** With `X = floor(10^1000/17)` (the truncated 1000-digit expansion
of `1/17`), `10^1000 = 17X + 16`, hence

    17X/10^1000 = 1 − 16·10^-1000 = 0.(998 nines)84

which is digit-for-digit what E6 recorded (measured: 1000 fractional digits, 998 nines, tail `84`). The
product is therefore **off by exactly `16·10^-1000`**, and the `84` is the residue, not a rendering
artifact. (Congruence check: `ord_17(10) = 16`, `1000 = 62·16 + 8`, `10^8 ≡ 16 (mod 17)`.)

**Step 4 — the repair cannot fire.** `DetectAndNormalizePeriod` (`Real.cs:2212-2279`) searches for a
repeating suffix with `Lovelace.Real/Real.cs:2083-2106` (`FindSmallestPeriod`), retries once with a
one-digit slack, and only then gives up:

- `Real.cs:2229` — `(long pStart, long pLen) = FindSmallestPeriod(fracPart);`
- `Real.cs:2230-2234`
  `// A single-digit truncation error can prevent exact period detection.`
  `// If no period was found, retry ignoring the last character (slack = 1).`
  `if (pLen == 0 && fracPart.Length > 2) (pStart, pLen) = FindSmallestPeriod(fracPart, 1);`
  `if (pLen == 0) return r; // no period detected`
- the all-nines carry needs **every** detected period digit to be `'9'`: `Real.cs:2238-2242`
  `// Check for all-nines period (0.999… = 1.0 etc.)` / `bool allNines = true;` / `foreach (char c in periodStr) { if (c != '9') { allNines = false; break; } }`

For `0.99…984` no `p` works (the trailing `8` and `4` break every candidate), and the slack-1 retry
still includes the `'8'` at index 998 (`effectiveLen = 999`, `Real.cs:2086`), so `pLen == 0` and the
function returns the truncated decimal unchanged. **The comment's stated bound — "a single-digit
truncation error" — is not the bound of the operation:** the error here is `16·10^-1000`, which costs
two trailing digits, and for a general `1/q` it costs `digits(10^1000 mod q)` digits.

**Step 5 — why the existing tests pass anyway.** For `1/3 * 3` the residue is `10^1000 mod 3 = 1`, so the
product is exactly `10^1000 − 1` = 1000 nines; `FindSmallestPeriod` finds `p=1, s=0` at slack **0**, the
all-nines branch carries into the integer part, and the result is exactly `1` (`Real.cs:2242-2252`). That
is precisely the case pinned in `Lovelace.Suite.Tests/ExactnessPreservationTests2.cs:24-33` (`"1/3 * 3"`)
and `:45-50`. The test set covers the one residue that happens to be 1.

**Step 6 — why `x == 1` is `False`.** `Lovelace.Real/Real.cs:393-404`
(`if (IsPeriodic != other.IsPeriodic) return false;` … `return CompareTo(other) == 0;`) → `CompareTo`
compares integer-part lengths first (`Real.cs:434-440`: `long intLenA = (long)digitsA.Length + Exponent;` …
`if (intLenA != intLenB) return (intLenA > intLenB ? 1 : -1) * signMul;`). For `x`: `1000 + (-1000) = 0`
versus `1 + 0 = 1` ⇒ `−1` ⇒ not equal. (Even a one-digit deficit would fail at the digit loop,
`Real.cs:449-455`.)

### 2.4 Minimal fix proposal — and what the file's own comments claim

**The correct fix is (ii) round-trip through the stored fraction — make the periodic path operate on the
exact rational that the stored `(magnitude, Exponent, PeriodStart, PeriodLength)` already denotes — not
(i) replacing `Real`'s internal representation with a `Rational`.** The representation is not the
problem: the truncation is introduced *by this operation*, and the exact rational is recoverable today
(`RationalReal.FromReal`, `RationalReal.cs:58-61`, verified by E8).

What the file's own comments claim is the (i)-style semantics — the periodic value *is* the exact rational:

- `Real.cs:15-19` class doc: `/// Arbitrary-precision real number (ℝ as fixed-point decimal with optional period).` … `/// support exact rational representation via periodic decimal notation (e.g. <c>"0.(3)"</c>).`
- `Real.cs:536-546` `Divide` doc: *"…when the same remainder recurs, `PeriodStart` and `PeriodLength` are set and the loop terminates immediately — yielding an exact rational result."*
- `docs/Language.md:220-224`: *"### Division is exact (never truncates) … preserving the result exactly."*; `docs/Language.md:810`: *"exact operations never lose digits; transcendentals truncate at the active budget."*
- The periodic branches themselves (the only place that admits the expansion) claim the repair: `Real.cs:522-524` *"then detect a repeating suffix in the result and normalise (including 0.999… → 1)"*, mirrored at `Real.cs:2318-2320` for `Add`.

So the comments claim exact-rational semantics while the implementation re-derives a finite decimal and
pattern-matches for a period. The 1/17 repro falsifies the specific claim in parentheses.

**Recommended fix (structural, keeps `Real`'s public API consistent):** in the periodic branch of
`Multiply` (`Real.cs:520-530`), `Add` (`Real.cs:2316-2326`) and `Divide` (`Real.cs:565-574`), do not call
`ExpandToNonPeriodic`. Compute with exact integer fraction arithmetic on the stored triple (recover
numerator/denominator using `Int`/`Nat` only — the same machinery `DivideNonPeriodic`/`ToInteger` already
use), then reconstruct through a factored-out copy of the remainder-tracked division at `Real.cs:576-658`,
which already produces the canonical periodic form.
Constraint to respect: **`Lovelace.Real` cannot use `Lovelace.Rational`** — `Lovelace.Real/Lovelace.Real.csproj`
references only `Lovelace.Abstractions` and `Lovelace.Integer` (which is exactly why `RationalReal` lives in
`Lovelace.Symbolics`, `RationalReal.cs:9-12`). A fix inside `Real.cs` must therefore use `Int`/`Nat`
directly, or the layering must change. The alternative — patching the Suite only
(`NumericOps.ApplyRealBinary`, `NumericOps.cs:101-148`, which can already see `RationalReal`) — leaves
`RealField.Multiply` (`Lovelace.Real/RealField.cs:23` `public Rl Multiply(Rl a, Rl b) => a * b;`) and every
direct `Real` operator user wrong; **recommend against it.**

**Rejected mitigation:** raising the fixed slack at `Real.cs:2232-2233` from 1 to `k`. The required `k` is
data-dependent (`k ≈ digits(10^N mod q)`, unbounded up to `N`); `k=2` repairs `1/17`, `k=3` is needed for
`1/7`/`1/97`, and any constant large enough for the worst case starts reporting periods for genuinely
non-periodic truncations — swapping one wrong value for another. A defensible companion change is to
*verify* a candidate period by re-dividing the stored fraction instead of trusting the suffix pattern.

**Tier parity is part of the fix:** the low-precision fast path duplicates the same design —
`LReal64.cs:226-241` (`Multiply`, with `WorkingFractionalDigits` = 18 at `LReal64.cs:29`) and
`LReal128.cs:269-281` (`WorkingFractionalDigits` = 37 at `LReal128.cs:120`) — and
`Lovelace.Suite.Tests/LRealDispatchTests.cs:46-55` requires the fast path to produce results *identical*
to class `Real`. Fixing only `Real.cs` would make the tiers disagree (or require the fast path to be
disabled for periodic operands).

**Adjacent evidence that the periodic path is known-fragile:** `RationalReal.cs:15-21` — *"Period detection is deliberately NOT used: the exact periodic Real would change the value class … and elementary-function paths mishandle periodic operands."* That reverse conversion truncates on purpose, the same asymmetry seen here.

### 2.5 Test that would FAIL on the current tree

- `Lovelace.Real.Tests` (`RealMultiplyTests.cs`):
  `Assert.Equal(Real.One, new Real(1) / new Real(17) * new Real(17));`
  Today the product is `1 − 16·10^-1000`, so `Equals(One)` is false (E6) ⇒ **FAILS**.
- `Lovelace.Suite.Tests` (`ExactnessPreservationTests2.cs:24-33`): add `[InlineData("(1/17) * 17")]` to
  `Arithmetic_DoesNotDegradeExactness`. Today `sv.Exact` is `false` (`RealExact`: not zero, not periodic,
  `−Exponent = 1000 > 18`) ⇒ **FAILS**; `Assert.Equal("1", sv.Value)` also fails.
- **PREDICTED extra failures (arithmetic, not observed — confirm by running before/after):**
  `(1/7)*7` → residue `10^1000 mod 7 = 4` → `1 − 4·10^-1000 = 0.99…996` → the slack-1 retry still
  sees the `'6'` ⇒ also wrong today; `(1/97)*97` → residue up to 96 ⇒ up to three corrupt trailing digits.
  `1/3*3` and `1/9*9` **pass** today (residues 1) — do not use them as regression evidence for this fix.

### 2.6 Blast radius

- **Lovelace.Real.Tests** — `RealMultiplyTests.cs`, `RealAddTests.cs` (`Add` has the identical
  expand-then-detect branch, `Real.cs:2316-2326`), `RealDivideTests.cs`, `RealToStringTests.cs`
  (period rendering), `RealSqrtPeriodicTests.cs` (`Real.cs:843-849` re-expands periodic inputs before
  Newton-Raphson), `RealAsyncLocalTests.cs` / `RealPiTests.cs` (results change only if the expansion
  disappears from low-precision paths), `LReal64Tests.cs` + `LReal128Tests.cs` (parallel implementations,
  `LReal64.cs:355-386`, `LReal128.cs:366` ff., plus the `Add_Periodic_MatchesReal` theories at
  `LReal64Tests.cs:96-103`, `LReal128Tests.cs:53-59`).
- **Lovelace.Suite.Tests** — `ExactnessPreservationTests2.cs`, `ExactRationalPayloadTests2.cs`,
  `InterpreterBinaryArithmeticTests.cs` (lines 208-230, non-exact division widening),
  `LRealDispatchTests.cs:57-62` (default precision must stay exact).
- **Lovelace.Symbolics.Tests** — `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs` (its
  `2^-1 == 1/2`, `3^-2 == 1/9` assertions run through `Divide`'s periodic path),
  `NegativePowerEvaluationTests.cs` (pinned digits computed via periodic arithmetic),
  `DxStructuredResultsTests.cs` (structured projection of Real values).
- **Lovelace.Run.Tests / Lovelace.Studio.Tests** — golden JSON containing a Real `display`/`structured`
  payload for a periodic product.

### 2.7 Open questions

- **OPEN-5.** What exactly does the audit's "largest single cluster" enumerate? If it is `(1/q)*q` over
  many `q`, the fix must be exact for *all* residues, which is what section 2.4 proposes; the predicted
  `(1/7)*7` failure makes a good first confirmation (it distinguishes "slack too small" from "only q=17").
- **OPEN-6.** Should the periodic path stay exact when the true period exceeds
  `MaxComputationDecimalPlaces` (e.g. `1/999983` has period 999 982)? The stored form can hold it, but the
  reconstruction loop's cap (`Real.cs:613`) would truncate. Decide the contract before coding the fix.
- **OPEN-7.** Do `Add`/`Subtract`/`Modulo`/`Sqrt`-of-periodic and the LReal tiers get the same treatment in
  this cycle, or is the fix scoped to `Multiply` + `Divide`? Partial fixes will leave
  `LRealDispatchTests` tier-parity red.
- **OPEN-8.** `RationalReal.ToReal` (`RationalReal.cs:22-27`) truncates on purpose. If the periodic path is
  made exact, the two conversion directions will disagree about whether periodic values are first-class
  exact values; that inconsistency needs an explicit decision.

---

## 3. Ruled out / could not locate

- **Ruled out for E1-E3:** a hard-coded zero return, an overflow/saturation branch, or a `double` cast in
  the power path. There is no such branch: `IntegerPower` (`NumericOps.cs:258-267`) delegates to exact
  integer `Int.Pow` and one `Real.Divide`; the zero is produced solely by the loop cap at `Real.cs:613`
  plus `Normalize`'s zero short-circuit at `Real.cs:2122`. The value-level evidence for the cap being 1000
  is arithmetic (indices 1000/1505/30102 all ≥ 1000) and is confirmed by E6, where the same constant
  produced exactly 1000 fractional digits.
- **Ruled out for E6:** a display/`ToString` truncation (the recorded `display` itself carries the 1000
  digits and the `…84` residue at full stored precision; `ToString`'s non-periodic branch only truncates
  the *stored* fraction at `DisplayDecimalPlaces`, `Real.cs:1868-1888`), and a `Multiply` exponent bug
  (`resultExp = left.Exponent + right.Exponent`, `Real.cs:509`, is consistent with the observed exponent
  of −1000 after `Normalize` strips the 1000 zeros contributed by the expanded right operand).
- **Could not locate:** any place that records *why* a Real is exact. There is no provenance bit anywhere
  in `Real` (fields are exactly `_magnitude`, `_isNegative`, `Exponent`, `PeriodStart`, `PeriodLength`); this is
  the structural cause of DEFECT 1's mislabelling and is the single change that would make both defects
  checkable rather than argued.
- **Not verified this round (no runs allowed):** the PREDICTED `(1/7)*7`, `(1/97)*97`, and `10^-1000`
  boundary case. The first two are arithmetic predictions from the same formula that E6 confirms exactly.

## 4. Suggested order of work

1. Give `Real` a truncation/exactness bit and make `RealExact` read it (kills the "exact lie" class for
   every operation at once; section 1.4(4)).
2. Reorder `Real.Pow` + guard `NumericOps.PowerReal` (small, self-contained; closes E4).
3. Make the periodic arithmetic path exact through the stored fraction, starting with `Multiply`, and
   apply the same to `LReal64`/`LReal128` or gate the fast path off for periodic operands (section 2.4).
4. Then decide the underflow contract (refuse vs approximate) and fix `Divide`'s cap exit (section 1.4(3)).
