# D-B — Printer round-trip dossier (T0-2 + the reported Tier-2 round-trip failures)

**Round**: goal-cycle-5, round 1 (diagnosis only)
**Scope read**: `Lovelace.Symbolics/Printing.cs`, `Lovelace.Symbolics/Expr.cs`,
`Lovelace.Symbolics/Constructors.cs`, `Lovelace.Suite/Parser.cs`, `Lovelace.Suite/Tokenizer.cs`,
`Lovelace.Symbolics/SymbolicsPlugin.cs` (print/parse + `evalf` entry points). Supporting reads
outside that list are named explicitly where used (they were needed for the `evalf` chain and for the
blast-radius inventory).
**Not done this round (deliberate)**: no `dotnet`, no test run, no `out/aot` invocation, no writes
outside this file. Every claim below is one of:
* **[READ]** — I read this code this round; the quoted text is verbatim.
* **[AUDIT]** — an observation reported in `docs/goal-cycle-4/round-09/audit-P4-symbolic.md` (cycle 4);
  not re-executed here.
* **[PREDICTED]** — a falsifiable prediction I did **not** run. Marked as such at each point.

---

## 0. The one mechanism T0-2 must reuse (found first, so the fix is not a second rule)

The subtraction analogue was fixed in **cycle 3, round 24** ("Minimal parentheses in the pretty
printer"), and it is documented in-tree:

* `Lovelace.Symbolics/Printing.cs:519-522`
  ```csharp
  /// <summary>Precedence demanded of the magnitude of a subtracted term: strictly tighter than
  /// additive, so a sum on the right of a minus keeps its parentheses — <c>x - (y - a)</c> must
  /// never render as <c>x - y - a</c>, which denotes a different expression. Round 24.</summary>
  private const int SubtrahendPrec = MulPrec;
  ```
* `Lovelace.Symbolics/Printing.cs:1018-1032` — `AppendSigned`, whose comment is the exact analogue of
  the T0-2 failure: *"the right operand of a minus binds tighter than a sum: x - (y - a) must not
  render as x - y - a, which reparses to a different expression. Round 24."* [READ]
* The detection helper that round introduced is `IsNegativeConstant` (**Printing.cs:1034-1048**), used
  by the sum arm at **Printing.cs:544** (and its LaTeX twin at **:787**):

  ```csharp
  private static bool IsNegativeConstant(Expr t, out Expr magnitude)
  {
      if (t is RationalConstantExpr r && r.Value.IsNegative) { ... return true; }
      if (t is IntegerConstantExpr i && Int.IsNegative(i.Value)) { ... return true; }
      magnitude = t;
      return false;
  }
  ```
* Provenance of that fix: `docs/goal-cycle-3/round-24/implementation.md:59-67` (the pre-fix failure
  `x - (y - a)` → `x - y - a`) and `:87` (the single changed line: `Pretty(inner, 1, …)` →
  `Pretty(inner, 2, …)`, i.e. `SubtrahendPrec`). [READ]

**The reusable mechanism is therefore: "a node whose rendering *starts with a unary minus* does not
have the precedence its node kind suggests; the printer must say so explicitly, in the one place that
owns the decision."** For a power base that one place already exists —
`NeedsPowerBaseDelimiter` (**Printing.cs:509**) — and it is already shared by the ASCII and LaTeX
arms (**Printing.cs:627-628** and **:876-877**). T0-2 is a gap *inside* that rule, not a missing rule.
The file's own constraint is explicit (**Printing.cs:461-465**: *"ONE table, ONE parenthesis decision,
shared by every infix rendering … there is no second precedence table anywhere in this file"* and
**:491-496**), so the fix must be an extension of `NeedsPowerBaseDelimiter`, never a parallel rule.

Note the shape that **already works**, which is why the defect looks inconsistent:
`-x` is `(mul (rat -1 1) (sym x))` (**Constructors.cs:719** `Negate(a) => Multiply(Rational(Rat.MinusOne), a)`),
so `Prec` returns `MulPrec = 2 <= PowPrec = 3` and `(-x)^2` is delimited correctly. Only the
*bare* negative constant, whose `Prec` is `AtomPrec`, escapes. [READ]

---

## 1. T0-2 — `(-1)^x` renders as `-1^x` and re-parses to a different value

### Symptom
The canonical form is `(pow (rat -1 1) (sym x))` but the pretty form is `-1^x`. Re-entering the
printed text gives `-(1^x)`, so at `x = 2` the expression is `1` and the text is `-1`; for
`(-2)^x` the pair is `4` vs `-4`. [AUDIT: `audit-P4-symbolic.md:67` (P2.8, FINDING F1),
`:138-139`, `:146-147`, `:165-171`; restated at `docs/symbolics/a-plus-cycle-4-report.md:208`
(A7) and `docs/goal-cycle-5/state.md:40-42`.]

### Repro command
Script (`--file` form used by the audit; **not run this round**):
```
x = symbol("x"); e = (-1)^x; e; subs(e, x, 2); subs(-1^x, x, 2); (-2)^x; subs((-2)^x, x, 2)
```
(the orchestrator's probe already exists at `docs/goal-cycle-5/probes/pre/t02-printer-negbase.ls`
with output in `.../t02-printer-negbase.out.txt`).
C#-level repro is the same in one line:
`Printing.PrettyPrint(Exprs.Power(Exprs.Rational(Rat.MinusOne), Exprs.Symbol("x"))) == "-1^x"`.

### Root cause  **[READ]**
1. The delimiter decision ignores sign — `Lovelace.Symbolics/Printing.cs:509`:
   ```csharp
   private static bool NeedsPowerBaseDelimiter(Expr b) => Prec(b) <= PowPrec;
   ```
   with `PowPrec = 3` (**Printing.cs:472**) and every constant falling to `AtomPrec = 4`
   (**Printing.cs:473** and `Prec`'s default arm at **:488**, `_ => AtomPrec`).
2. The base text itself carries the unary minus — `Printing.cs:529`:
   ```csharp
   case RationalConstantExpr r: return r.Value.ToString();
   ```
   `Rat.MinusOne.ToString()` is `"-1"`.
3. The Pow arm consumes the rule and therefore emits no parentheses — `Printing.cs:625-628`:
   ```csharp
   var b = Pretty(p.Base, RootPrec, false, unicode);
   // a power base must be parenthesized: x^y^2 is ambiguous ((x^y)^2 vs x^(y^2))
   if (NeedsPowerBaseDelimiter(p.Base))
       b = "(" + b + ")";
   ```
   For base `(rat -1 1)`: `Prec = AtomPrec = 4`, `4 <= 3` is **false** → no delimiter → `-1^x`.
   (Same rule, same gap, LaTeX arm: **Printing.cs:875-877** → `-1^{x}`.)
4. The other half of the wrong *value* is the language's documented convention, which must **not**
   change: `Lovelace.Suite/Parser.cs:13-16` and `:358-362` — *"A leading sign binds looser than ^
   (the mathematical convention: -x^2 is -(x^2), so -2^2 is −4)"*, implemented at **Parser.cs:363-377**
   (`Current.Kind == TokenKind.Caret ? new UnaryExpr(op, ParsePowerTail(operand)) : …`).
   This contract is pinned by `Lovelace.Suite.Tests/SignPrecedenceTests.cs:95-107`
   (`[InlineData("-2^2", "-4")]`) and `:30-50`. [READ]

So: **the printer is the faulty side; the parser is behaving as documented.** The fix must not touch
`Parser.cs`.

### Minimal fix proposal  (one predicate, both notations)
Extend the ONE power-base rule at **Printing.cs:509** to ask the same sign question the sum arm asks,
reusing the existing helper (`IsNegativeConstant`, **Printing.cs:1034-1048**) rather than adding a
second decision:

```csharp
private static bool NeedsPowerBaseDelimiter(Expr b) =>
    Prec(b) <= PowPrec || IsNegativeConstant(b, out _);
```

Because both arms already call this rule (`:627-628` ASCII, `:876-877` LaTeX), the single edit fixes
both renderings and cannot desynchronise them. Recommended companion (still inside the same helper,
not a new rule): also treat a negative `RealConstantExpr` as sign-prefixed —
`RealLiteral.ToString()` prefixes `"-"` (`Lovelace.Symbolics/Expr.cs:179-180`) and
`Printing.cs:530` renders it verbatim, so `(pow (real -1.5…) (sym x))` has the same defect even
though language-derived `-1.5` currently arrives as the rational `(rat -3 2)`
(`SymbolicsPlugin.cs:1466-1471`, `RealToExpr`, keeps ≤18-fractional-digit Reals as exact rationals).
A general predicate "this base's rendering starts with a unary minus" is the honest form of the rule;
`IsNegativeConstant` is its existing, tested core. [READ]

What I would **not** do: add a second `Prec`-like table, or special-case `PowPrec` inside the arm
(both are exactly what `Printing.cs:461-465`/**:491-496** forbid, and what Round 24 removed).

### Test that would FAIL on the current tree
Extend the existing corpus-based round-trip theory — it already asserts exactly the right property
(`parse(pretty(e))` canonicalises to the same canonical form) and needs no new harness:

* `Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:29-71` (`Corpus`) — add `"(-1)^x"`,
  `"(-2)^x"`, `"(-1/2)^x"`, `"(-1)^(x + 1)"`. (The corpus already carries `"(-x)^2"`, `"(-x)^3"`,
  `"(-x - 1)^2"`, `"(-x)^(1/2)"` — i.e. every negative-base shape *except* the numeric one.)
* `PrettyParenthesesTests.cs:104-112` `RoundTrip_PrettyRenderingReparsesToTheSameCanonicalForm`
  then fails pre-fix with, verbatim, the audit's mismatch:
  `canonical (pow (rat -1 1) (sym x))` vs reparsed `(rat -1 1)` [AUDIT: `audit-P4-symbolic.md:67`].
* Same rows in the LaTeX twin corpus: `Lovelace.Symbolics.Tests/LatexPrinterTests.cs:44-78`
  (plus an exact pair next to `:226-238`, e.g. `{ "(-1)^x", "\\left(-1\\right)^{x}" }`).

### Blast radius  (what encodes the current rendering)
* **No test asserts the wrong string.** A repo-wide grep for `-1^` (and `-2^`/`-3^` inside
  `Lovelace.Symbolics.Tests`) matches only `docs/**` prose and the audit — no test, no golden
  fixture, no doc example. The fix is expected to break no existing assertion. [READ]
* **LaTeX changes with it (by design, one rule):** `LatexPrinterTests.cs` corpus (`:44-78`) and
  exact-pair table (`:226-238`; `{ "-x^2", "-x^{2}" }` is the neighbouring case that must stay
  unchanged). Any new row must be added to both arms in the same commit.
* **Golden envelopes:** `Lovelace.Run.Tests/fixtures/*.json`, pinned by
  `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:6-15` — none contains a negative numeric base
  (grepped), so no regeneration is expected; but the golden harness is the thing to re-run because it
  compares whole envelopes byte-for-byte.
* **Executable docs:** `Lovelace.Symbolics/README.md` examples are executed and asserted exactly by
  `Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:21-52`, and the C# snippets are checked against
  `Lovelace.Symbolics.Tests/UsageExamples.cs` by `DocsSyncTests.cs:15-32`; the Suite language guide is
  checked by `Lovelace.Suite.Tests/LanguageDocumentationTests.cs` against `Lovelace.Suite/docs/Language.md`.
  None of them contains `-1^x` today → no doc churn expected *for this defect*.
* `Lovelace.Symbolics.Tests/PrintBudgetTests.cs` compares against `Printing.PrettyPrint(...)`
  dynamically (`:29`, `:38`, `:69`) → unaffected.
* Minimality is safe: `PrettyParenthesesTests.cs:118-128` forbids only *doubled* pairs; the fix adds a
  single required pair.

### Open questions
1. Should the predicate be "renders with a leading minus" (covers a negative `RealConstantExpr` and
   any future signed atom) or stay `IsNegativeConstant` (rational/integer only)? The former is
   provably total for the current node set; the latter is smaller. **Decision needed before coding.**
2. Is `ComplexConstantExpr` ever a power base that starts with `"-"`? `ComplexToText`
   (`Printing.cs:1215-1222`) always emits `"re + …i"` / `"re - …i"` — it never starts with a sign, so
   no. (Recorded so the next agent does not re-open it.)
3. Does any *external* consumer parse pretty text (Studio, Console REPL history, docs)? If a consumer
   keys on the old strings, the change is a visible output change even though it is a bug fix.

---

## 2. Item (a) — complex closed forms print `i`, which the parser rejects

### Symptom
`sqrt(-1)` renders `i`, `sqrt(-4)` renders `2*i`, and re-entering that text fails with
`DomainError: Undefined variable 'i'.` [AUDIT: `audit-P4-symbolic.md:68` (P2.9, FINDING F2),
`:102-105`, `:209`; `docs/symbolics/a-plus-cycle-4-report.md:210` (A9).]

### Repro command
```
sqrt(-1)                     # pretty "i"
i                            # -> Undefined variable 'i'.
```
C#-level: `ValueFormatter.Format(engine.Evaluate("sqrt(-1)"))` is `"i"`
(asserted at `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:220`).

### Root cause  **[READ]** — a print/parse vocabulary asymmetry, not a printer bug
* Printer emits the bare glyph: `Lovelace.Symbolics/Printing.cs:537`
  ```csharp
  NamedConstant.I => "i",
  ```
  (LaTeX twin `Printing.cs:780`, same glyph). The canonical form is symmetric and **does** round-trip:
  printed `"(i)"` at `Printing.cs:34`, parsed back at `Printing.cs:159`
  (`case "i": ExpectClose(tokens, ref pos); return Exprs.I;`).
* The Suite lexes `i` as an ordinary identifier and the parser builds a *variable*:
  `Lovelace.Suite/Parser.cs:532-556` — `return new VariableExpr(name);`.
* The interpreter has a constant table but `i` is not in it:
  `Lovelace.Suite/Interpreter.cs:309-334` — `pi` (**:316-317**), `e` (**:318-319**), `inf`
  (**:320-321**), `real`/`complex`/`integer`/`rational` (**:324-331**), then
  ```csharp
  throw new InvalidOperationException($"Undefined variable '{var.Name}'.");   // Interpreter.cs:333
  ```
* Cross-check: every other constant the printer can emit is readable — `pi` (Printing.cs:535 ↔
  Interpreter.cs:316), `e` (:536 ↔ :318), `inf` (:538 ↔ :320). `i` is the **only** asymmetric one,
  which is what makes the one-line fix below the right shape.

### Minimal fix proposal
Add the same kind of entry the other constants already have, at `Interpreter.cs:316-331`:
```csharp
if (var.Name == "i")
    return new Value(Lovelace.Symbolics.Exprs.I);
```
It is symmetric with `pi`, it keeps the human notation the printer chose (and that 92 audit rows and
the LaTeX arm rely on), and `scope.TryGet` still wins (**:311-312**), so a user variable `i` shadows
it exactly as `pi` is shadowed. The alternatives (print `sqrt(-1)` instead; add an `i()` builtin) are
worse: they change the *rendering* to satisfy the parser instead of making the language read its own
notation, and would churn the golden fixtures below.

### Test that would FAIL on the current tree
Add `"sqrt(-1)"` and `"sqrt(-4)"` to the `PrettyParenthesesTests.Corpus`
(`Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:29-71`): the existing theory
(`:104-112`) prints `i` / `2*i`, re-evaluates the text, and pre-fix throws
`InvalidOperationException: Undefined variable 'i'.` — a genuine pre-fix-failing test with no new
harness.

### Blast radius
* `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:219-222` asserts the exact strings `"i"`,
  `"i"`, `"2*i"` — unaffected by an interpreter-side fix (printer untouched).
* `Lovelace.Symbolics.Tests/ComplexClosedFormTests.cs:60-69` (`AssertRoundTrips`) exercises
  `CanonicalPrint`/`CanonicalParse` only — **this is why the defect escaped**: the canonical form
  `(i)` round-trips (Printing.cs:159), so a canonical-only round-trip test cannot see it.
* Golden envelopes `Lovelace.Run.Tests/fixtures/*.json`: grepping the fixture directory for a bare
  `i` token matched nothing (only `rootof(...)` in `solve_result.json`) → no fixture encodes the
  `i` rendering today.
* Executable docs (`UsageDocumentationTests.cs`, `DocsSyncTests.cs`) — any README example whose
  output contains `i` would already be passing; adding a readable constant changes only scripts that
  *read* an unbound `i`, which today is an error.
* **Behavioural widening to watch**: a script that today errors on a stray `i` will now silently get
  the imaginary unit. `Lovelace.Suite/docs/Language.md` (asserted by
  `Lovelace.Suite.Tests/LanguageDocumentationTests.cs`) documents `error: Undefined variable 'x'.`
  with `x` as the example — check it does not use `i`.

### Open questions
1. Is `i` reserved *before* the fix is applied to scripts using `i` as a loop/counter variable?
   `scope.TryGet` wins, so existing assignments are safe; an *unbound read* changes from error to
   value. Is that acceptable, or should `i` require an explicit opt-in?
2. Should the same treatment apply to `I` (SymPy spelling) and to `oo`/`∞`? The printer never emits
   them, so today they are out of scope — recorded so this does not grow.

---

## 3. Item (b) — `solve_full` prints `2^(1/3)` and `rootof(...)`; neither re-parses

### Symptom
`solve_full(x^3 - 2, x)` solution values render `2^(1/3)`; re-entering that text fails with
`UnsupportedOperation: Non-integer exponents are not yet supported.`
`solve_full(x^4 - 2*x^2 - 3, x)` solution values render `rootof(x^4 - 2*x^2 - 3, 0)`; re-entering it
fails with `DomainError: Unknown function 'rootof'.`
[AUDIT: `audit-P4-symbolic.md:69-70` (P2.10/P2.11, FINDING F3), `:218-228`, `:247-248`.]

### Repro command
```
x = symbol("x"); solve_full(x^3 - 2, x)        # values print 2^(1/3), ...
2^(1/3)                                        # -> UnsupportedOperation
x = symbol("x"); solve_full(x^4 - 2*x^2 - 3, x)
rootof(x^4 - 2*x^2 - 3, 0)                     # -> Unknown function 'rootof'.
```

### Root cause  **[READ]** — two *different* gaps, neither of them in the printer
**The printer is faithful.** `2^(1/3)` is exactly `(pow (rat 2 1) (rat 1 3))` rendered by the Pow arm
(`Printing.cs:612-637`; the exponent is parenthesised because it is a non-integer rational at
`:633-634`), and `rootof(...)` is exactly `RootOfExpr` at `Printing.cs:668-669`:
```csharp
case RootOfExpr r:
    return "rootof(" + Pretty(r.DefiningPolynomial.ToExpr(), ArgPrec, false, unicode) + ", " + r.RootIndex + ")";
```

**(b1) `2^(1/3)` parses and then fails to evaluate.** The text is a valid Suite expression
(`Parser.cs:382-390` for `^`, `:559-565` for the parenthesised exponent), but a *numeric* base with
a non-integer exponent has no route to the symbolic model:
* `Lovelace.Suite/NumericOps.cs:64` — `(BinaryOp.Power, ValueKind.Real) => PowerReal(left.AsReal(), right.AsReal()),`
* `NumericOps.cs:87-94` — `PowerReal`: only the exact `(-a)^(1/2)` case is routed to the symbolic
  complex path; everything else is `return new Value(baseValue.Pow(exponent));`
* `Lovelace.Real/Real.cs:739-746` —
  ```csharp
  string fracPart = expStr[(dotIdx + 1)..];
  if (fracPart.Any(c => c != '0'))
      throw new NotImplementedException("Non-integer exponents are not yet supported.");
  ```
  (A symbolic base is fine: `NumericOps.cs:35-36` routes any Symbolic operand to
  `ApplySymbolic`, whose `BinaryOp.Power` arm is `NumericOps.cs:179`: `Exprs.Power(l, r)`.)

**(b2) `rootof` has no reader at all.** `Lovelace.Suite/Interpreter.cs:619`:
`throw new InvalidOperationException($"Unknown function '{call.FunctionName}'.");` — and a grep over
the whole tree for `Add("rootof"` / `Register("rootof"` finds **no registration** in either the Suite
core or the Symbolics plugin. The inverse operation already exists *inside* the canonical grammar:
`Printing.cs:241-250` (`CanonicalParse`'s `case "rootof"`) collects symbols, runs
`Polynomial.TryFromExpr`, and constructs `Exprs.RootOf(poly, index)` — i.e. the semantics are
already implemented; only the human surface is missing.

### Minimal fix proposal
* **(b2) first — smallest, fully specified by existing code:** register a Symbolics builtin
  `rootof(poly, index)` that mirrors `Printing.cs:241-250` exactly
  (`CollectSymbols` → `Polynomial.TryFromExpr` → `Exprs.RootOf`), with the same failure text
  (`"RootOf operand is not a polynomial."`). This makes the printed `rootof(...)` re-readable
  without touching the printer or the canonical form.
* **(b1) is a language/kernel decision, not a printer fix.** Either (i) route a numeric base with a
  non-integer exponent into the exact symbolic `Power` when `Real.Pow` refuses (the same "exact
  representation exists, so use it" pattern as the `(-a)^(1/2)` fallback at `NumericOps.cs:87-94`),
  which makes `2^(1/3)` re-readable and is the only option that satisfies "rendering must be
  re-readable" without lying; or (ii) keep the refusal and make the *capability advertisement* honest
  that this rendering is not re-entrant. Option (ii) costs nothing but leaves the round-trip property
  false; option (i) touches `NumericOps` and the advertised capability message, and interacts with
  `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:224-234` which pins the current refusal.

### Test that would FAIL on the current tree
* **(b2)** a round-trip test through the real parser: build/print the value from
  `solve_full(x^4 - 2*x^2 - 3, x)` (or construct `Exprs.RootOf(...)` directly), assert
  `CanonicalPrint(reparse(PrettyPrint(e))) == CanonicalPrint(e)`; pre-fix the reparse throws
  `Unknown function 'rootof'.`
* **(b1)** the same test with `e = Exprs.Power(Exprs.Integer(2), Exprs.Rational(1, 3))`
  (`PrettyPrint` → `2^(1/3)`); pre-fix the reparse throws
  `NotImplementedException("Non-integer exponents are not yet supported.")`.
  Neither belongs in `PrettyParenthesesTests.Corpus` as-is, because its `BuildAsync` *evaluates* the
  source and would throw before the assertion — the corpus harness needs a pre-built-expression entry
  point (`PrettyParenthesesTests.cs:73-98`) or the row must be added after the language fix.

### Blast radius
* **Golden fixture that encodes the current rootof rendering:**
  `Lovelace.Run.Tests/fixtures/solve_result.json` — `display`/`typed` at lines **9, 10, 278** and
  `structured…pretty` at lines **76, 121** all contain `rootof(x^4 - x^2 - 1, 0)` / `(…, 1)`;
  the fixture is compared by `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:6-15`. Regeneration is
  required if the rendering changes (it should not, for (b2)). Historical copies exist under
  `docs/goal-cycle-3/round-5/fixtures-before/solve_result.json` (not a test).
* `Lovelace.Symbolics.Tests/ComplexClosedFormTests.cs:286-288` uses canonical print/parse → unaffected.
* **Capability advertisement drift (for (b1)):** `SymbolicsPlugin.cs:843` advertises
  `pow.non-integer-exponent` / `pow.negative-base-unrepresentable-exponent` with trigger `2^(1/2)` /
  `(-8)^(1/3)`; the advertised text is pinned in `docs/goal-cycle-4/round-10/implementation.md:287,295-296`
  and asserted as behaviour by `HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs:224-234`.
  `CapabilitiesBuiltinTests` also reads it.
* Docs: no README/language example prints `2^(1/3)` (grepped) → no doc churn for (b2).

### Open questions
1. (b1) Exact symbolic power vs honest refusal — **unresolved by design; needs an owner decision.**
   I did not find any comment in the printer/parser claiming the exact-radical rendering is
   re-readable, so no in-tree contract is violated by refusing; the *audit's* contract
   ("rendering must be re-readable") is what fails.
2. (b2) Does the plugin or the Suite core own `rootof`? The RootOf node lives in
   `Lovelace.Symbolics`, so the Symbolics plugin is the natural home (and `GuardName` at
   `ModusHost.cs:134-139` prevents shadowing an existing Suite builtin).
3. Are there other printer-emitted *function* spellings with no reader (the audit's N5 `ln` is the
   numeric one)? A one-off grep of the printer's literal spellings vs the builtin registry is not in
   this round's scope and is **OPEN**.

---

## 4. Item (c) — `(x/y)/z` renders `x/(y*z)`; the reparse gives a different canonical form

### Symptom
```
(x/y)/z    canonical (mul (sym x) (pow (sym y) (rat -1 1)) (pow (sym z) (rat -1 1)))
           pretty    x/(y*z)
x/(y*z)    canonical (mul (sym x) (pow (mul (sym y) (sym z)) (rat -1 1)))
```
Same class for `x/y/a`, `2/x/y`, `x^(-1)/y`. The *values* agree, but the tool's own machinery cannot
see it: `simplify((x/y)/a - x/(a*y))` → `-x/(a*y) + x/(a*y)`, not `0`.
[AUDIT: `audit-P4-symbolic.md:62` (P2.3, FINDING F5), `:271-288`.]

### Repro command
```
x = symbol("x"); y = symbol("y"); z = symbol("z");
(x/y)/z                       # pretty x/(y*z)
x/(y*z)                       # different canonical form
simplify((x/y)/z - x/(y*z))   # not 0
```

### Root cause  **[READ]** — the printer's flat denominator is the lossy step
* **Printer:** `Printing.cs:732-739` puts *every* factor of the form `(pow · -1)` into a flat list
  `dens`, one entry each:
  ```csharp
  if (f is PowerExpr p && p.Exponent is RationalConstantExpr pe && pe.Value.IsMinusOne)
      dens.Add(p.Base);
  ```
  and `:606` then joins them into a single textual denominator:
  ```csharp
  var denText = dens.Count == 1 ? dens[0] : "(" + string.Join("*", dens) + ")";
  ```
  Two structurally distinct inverse factors (`(pow y -1)`, `(pow z -1)`) therefore become one
  denominator product `(y*z)`. The same decomposition is at `Printing.cs:715-741` (`ShapeProduct`).
* **Parser/constructors:** re-reading `x/(y*z)` builds `Divide(x, Multiply(y,z))` →
  `Multiply(x, Power(Multiply(y,z), Rational(-1)))` (`Constructors.cs:716`
  `Divide(a, b) => Multiply(a, Power(b, Rational(Rat.MinusOne)))`), and `Power`
  (`Constructors.cs:448-567`) contains **no fold that distributes an exact integer exponent over a
  product** — the fold list is `:454-534` and the only exponent merging is per *same base*
  (`:416-431`). So the node is `(pow (mul y z) (rat -1 1))`, not two inverse factors.
* **Which side is at fault?** The printer's flat rendering: it is the step that discards the split.
  The parser is consistent — `a/(b*c)` *means* `a·(bc)⁻¹` — and the canonical model deliberately
  does not rewrite that into two inverses (the neighbouring pole-set reasoning is documented at
  `Constructors.cs:413-415`: *"keep them separate so that x·x⁻¹ can never collapse to 1 (that would
  define the product at 0)"*). The fraction form is invertible **only when `dens.Count == 1`**, which
  is exactly the audit's split: `x/(a*y)` (single denominator that happens to be a product) HELD,
  `x/(y*z)` (two denominators) FINDING. [AUDIT `:61-62`]

### Minimal fix proposal
Printer-side (preferred; local, no canonical-model change): keep the `num/den` fraction rendering only
when `shape.Denominators.Count <= 1`; with two or more, render each inverse factor as its own negative
power in the product (`x*y^-1*z^-1`), which re-parses to the split canonical form. The single-
denominator case — including `x/(a*y)` and the coefficient fold that produces
`-1/(2*(x + 1))` (`Printing.cs:589-607`) — is untouched.

Alternative (constructors-side, **not** preferred first): distribute an exact integer exponent over a
product in `Power` (`(b*c)^n → b^n·c^n`), so the parse of `x/(y*z)` produces the split form. It is
mathematically sound over a field (b·c ≠ 0 ⟺ b ≠ 0 ∧ c ≠ 0, no zero divisors) and would also make
`simplify((x/y)/a - x/(a*y)) = 0` reachable, but it changes canonical forms globally (hash/equality,
every golden) and is a much larger blast radius. **Test the printer-side fix first**, then decide
whether the normalisation is wanted for its own sake.

### Test that would FAIL on the current tree
Add `"(x/y)/z"`, `"x/y/a"`, `"2/x/y"` to `PrettyParenthesesTests.Corpus`
(`Lovelace.Symbolics.Tests/PrettyParenthesesTests.cs:29-71`) — the existing theory
(`:104-112`) then fails with the audit's exact canonical mismatch. The corpus currently contains only
single-denominator rows (`"x/(y + 1)"`, `"(x + 1)/y"`, `"1/(2*(x + 1))"`), which is why Round 24
passed.

### Blast radius
* Golden fixtures: I grepped `Lovelace.Run.Tests/fixtures/` for a denominator containing `(` — the
  only hit is `solve_nosolutions.ls` (a script, not a golden string); no fixture golden contains a
  two-factor textual denominator, so no regeneration is expected. Re-run
  `GoldenEnvelopeTests` anyway.
* Executable docs: `Lovelace.Symbolics/README.md:674` contains `y/(x*y)` — a **single** denominator
  (a product) → the proposed rule leaves it unchanged; `UsageDocumentationTests.cs:21-52` asserts it
  exactly, so this is the one to watch.
* `Lovelace.Symbolics.Tests/PrintBudgetTests.cs` compares against `PrettyPrint` dynamically → unaffected.
* Note `x/(y*z)` typed by a user keeps rendering `x/(y*z)` (its canonical node is a single
  pow-of-product, so the Multiply arm is not even used) while `(x/y)/z` starts rendering
  `x*y^-1*z^-1`: the two expressions finally render *differently*, which is correct but is a visible
  change in the pretty look.

### Open questions
1. Is the aesthetic cost of `x*y^-1*z^-1` acceptable, or is the canonical normalisation the real
   prize (it is what `simplify` needs to reach 0 — audit `:284-287`)? **Decision needed.**
2. Do the LaTeX and ASCII arms stay in one decision? `LatexRender`'s product arm
   (`Printing.cs:805-864`) has the same flat `\frac{...}{...}` shape and must be changed in the same
   commit; `LatexPrinterTests.cs` has to gain the same rows.

---

## 5. Item (d) — `evalf(sqrt(2), 5)` returns 100 decimals instead of 5 digits

### Symptom
`evalf(sqrt(2), 5)` → a Real with 100 decimal places; the count is **the same for every requested
digit count** (5, 10, 15, 20, 40, 60, 100, 120, 300), while `evalf(1/3, 5)` → `0.33333` (5) and
`evalf(log(-2), 5)` → `0.69314 + 3.14159i` (5) obey.
[AUDIT: `audit-P4-symbolic.md:115` (P4.14, FINDING F7) and `:311-326`; the README documents the
contract as *"irrationals come back to the requested digits"* (`Lovelace.Symbolics/README.md:829-830`).]

### Repro command
```
evalf(sqrt(2), 5)     # 100 decimals  (audit); expected N(sqrt(2),5) = 1.4142
evalf(pi, 5)          # 100 decimals
evalf(1/3, 5)         # 0.33333  (obeys)
evalf(log(-2), 5)     # 0.69314 + 3.14159i  (obeys)
```

### Root cause  **[READ]** — the requested digit count never reaches the *value* or the *formatter*
`evalf` itself (`Lovelace.Symbolics/SymbolicsPlugin.cs:546-555`):
```csharp
Add("evalf", new[] { "f", "digits" }, args =>
{
    var f = AsExpr(args[0]);
    var digits = (int)AsLong(args[1]);
    using (Rl.WithPrecision(digits, Math.Min(digits, 50)))
    {
        var num = Evaluation.EvaluateToNum(f, Context, new Dictionary<Symbol, Num>());
        return NumToPayload(num);
    }
},
```
and the payload conversion (`SymbolicsPlugin.cs:1349-1357`):
```csharp
NumRat r => r.V.IsInteger ? r.V.ToInteger() : RationalReal.ToReal(r.V, (int)Math.Min(Rl.MaxComputationDecimalPlaces, 1000)),
NumReal rl => rl.V,
NumComplex c => c.V,
```

Three facts compose into the symptom:

1. **The argument is already numeric before `evalf` runs.** `sqrt` and `pi` are *Suite* builtins
   that return Reals computed at the ambient engine precision:
   `Lovelace.Suite/Interpreter.cs:1451-1468` (`return new Value(await Rl.SqrtAsync(real, SubProgress("sqrt")));`)
   and `:1472-1488` (`return new Value(Rl.Pi);`); `Rl.SqrtAsync(value, progress) => Task.Run(() =>
   Sqrt(value, MaxComputationDecimalPlaces, progress))` (`Lovelace.Real/Real.cs:1597`) and
   `Real.Pi` upgrades its cache on demand to `MaxComputationDecimalPlaces` (`Real.cs:1168-1191`).
   At the engine defaults that is **computation 1000 / display 100**
   (`Interpreter.cs:83-84` `_computationDecimalPlaces = 1000L; _displayDecimalPlaces = 100L;`).
   The plugin then receives that value as an `Rl` and converts it to a **real literal** carrying all
   of its digits — `SymbolicsPlugin.cs:1455-1471`:
   ```csharp
   private static Expr RealToExpr(Rl r)
   {
       if (r.IsPeriodic || -r.Exponent <= 18)
           return Exprs.Rational(RationalReal.FromReal(r));
       return Exprs.Real(RealLiteral.FromRealExact(r));
   }
   ```
   so `EvaluateToNum` sees a `RealConstantExpr` and passes it straight through
   (`Lovelace.Symbolics/Evaluation.cs:415`: `case RealConstantExpr rl: return new NumReal(rl.Value.ToReal());`).
   **`digits` cannot lower digits that are already in the input.**
2. **Only the exact-rational arm of `NumToPayload` applies the digit bound** (`:1353`, via the scoped
   `Rl.MaxComputationDecimalPlaces`); the Real and Complex arms pass the value through unchanged
   (`:1354-1355`). That is precisely why `evalf(1/3, 5)` obeys and `evalf(sqrt(2), 5)` cannot.
3. **The printed count is fixed at format time, outside the scope.** The payload is rendered after
   the plugin returns — `Lovelace.Suite/ValueFormatter.cs:18` `ValueKind.Real => value.AsReal().ToString(),`
   — and `Real.ToString()` truncates the fractional part at the *ambient* `DisplayDecimalPlaces`
   (`Real.cs:1885-1886`), i.e. 100 by default (`Real.cs:43`). `evalf`'s display half
   (`Math.Min(digits, 50)`, line 550) is **dead for a payload return**: it is restored on `Dispose`
   long before anything is printed.

The complex case obeys for a different, already-correct reason: `ComplexMath` truncates complex results
to the *ambient* digit count — `Lovelace.Complex/ComplexMath.cs:204-207`
(`long digits = AmbientDigits(); … return TruncateComplex(…, digits);`) with
`AmbientDigits() => Rl.MaxComputationDecimalPlaces` (`:295-299`) — which *is* the scoped `digits`.
Conversely the real algorithms deliberately add guard digits: `Real.Sqrt` sets
`const long guard = 50L; long targetPrecision = precision + guard;` (`Real.cs:840-841`) and
`ComplexMath` uses `GuardDigits = 20` (`ComplexMath.cs:19`). So even a value computed *inside* the
scope would print `digits + guard` digits, not `digits`.

**Two candidates, and which I would test first.**
* **D1 (primary)**: the argument is pre-evaluated at the ambient precision (fact 1) and nothing
  quantizes the result afterwards (facts 2-3). Reasoned evidence: the observed count is exactly the
  ambient display precision (100) and **invariant** to `digits`; had the scope governed the value,
  the count would have been `digits + 50` and would have varied with `digits` (5, 55, 70, …).
* **D2 (contributing)**: the real numeric path is not truncated to the scoped digit count the way the
  complex path is (guard digits, `Real.cs:840-841` vs `ComplexMath.cs:204-207`).
  [PREDICTED] **Discriminating probe to run first**: `setprecision(3); evalf(sqrt(2), 5)`. Under D1
  the output is governed by the ambient precision (≈3 decimals), proving `digits` is inert for an
  already-numeric argument; a D2-only world would print a scope-derived count instead. Cheap control:
  `evalf(1/3, 5)` after the same `setprecision(3)` should still print 5 (the rational arm reads the
  scoped value). I did **not** run either.

### Minimal fix proposal
Quantize at the payload boundary, inside the scope, reusing the conversion the rational arm already
uses: in `NumToPayload` (`SymbolicsPlugin.cs:1349-1357`) convert `NumReal`/`NumComplex` through the
same digit-bounded path as `:1353` (`RationalReal.ToReal(RationalReal.FromReal(v), digits)` /
`RationalReal.ToReal`, `RationalReal.cs:22,58`) with `digits = (int)Math.Min(Rl.MaxComputationDecimalPlaces, 1000)`
— still in scope at that point. This makes the *output* obey the requested count for every argument
shape, regardless of D1/D2. Whether `digits` means **significant digits** (SymPy's `N`, the audit's
expectation) or **decimal places** (today's `evalf(1/3,5) = 0.33333` and the README's
`evalf(1/3,25) = 0.3333333333333333333333333`) is a contract decision that must be made first: the
README asserts a *decimal-place* reading, the audit expects a *significant-digit* reading, and the
two differ only for values whose integer part has more than one digit (e.g. `evalf(sqrt(2),5)`:
decimal-place → `1.41421`, significant-digit → `1.4142`).

### Test that would FAIL on the current tree
`Assert.Equal("1.4142", ValueFormatter.Format(engine.Evaluate("evalf(sqrt(2), 5)")))` (a new case in
an evalf/README-style test). Pre-fix it returns the 100-decimal real. **There is currently no unit
test for `evalf` anywhere in `Lovelace.Symbolics.Tests`** (grepped: no `evalf` match) — the only
executable coverage is the README example below, which passes *by coincidence* (see blast radius).

### Blast radius
* **`Lovelace.Symbolics/README.md:839-845` is the fixture that encodes the current behaviour**:
  ```
  setprecision(20)
  evalf(sqrt(2), 20)
  ---
  1.4142135623730950488 (Real)
  ```
  asserted exactly by `Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:21-52`. Note *why* it
  passes today: `setprecision(20)` sets the ambient precision to 20, which is also the `digits`
  argument, so the ambient-precision leak is invisible. Any fix must keep this pair passing (it does,
  under either digit reading — 20 significant digits of √2 is exactly the documented string) and must
  not be "fixed" by editing the doc silently.
* `Lovelace.Symbolics/README.md:832-837` (`evalf(1/3, 25)` → 25 threes) pins the rational arm —
  unchanged if the fix keeps the decimal-place reading.
* `Lovelace.Symbolics.Tests/UsageDocumentationTests.cs` + `DocsSyncTests.cs` (README ↔
  `UsageExamples.cs`) are the doc-sync instruments; `Lovelace.Symbolics.Tests/CompilationTests.cs:88,119`
  and `Lovelace.MathIR/Evaluator.cs:453,483` use the same
  `Rl.WithPrecision(digits, Math.Min(digits, 50))` idiom and may share the defect — **they are out of
  this dossier's scope and marked OPEN.**
* The wire: the payload stays a Real, so `kind`/`exact` are unaffected; only the digit string changes,
  which affects `Lovelace.Run.Tests` goldens *if* any golden prints an irrational Real. (Grep of the
  fixtures for evalf/irrational output is **OPEN** — not completed this round.)

### Open questions
1. **Significant digits or decimal places?** The README and the audit disagree; the fix cannot satisfy
   both spellings of `evalf(x, n)` without picking one. Owner decision required.
2. Should `digits` also bound the *computation* (currently it lowers the cap, which is right) or only
   the output? The audit's SymPy expectation is an output contract.
3. Do `CompilationTests.cs` / `MathIR/Evaluator.cs` (same precision idiom) exhibit the same symptom?
   **OPEN** — out of scope this round.

---

## 6. Inventory summary

| # | Defect | Root cause located at | Status |
|---|---|---|---|
| T0-2 | `(-1)^x` → `-1^x` | **`Printing.cs:509`** (`NeedsPowerBaseDelimiter`), consumed at `Printing.cs:625-628`; `Prec` default `AtomPrec` at `:488`; base text at `:529`; LaTeX twin `:875-877`; parser convention that must not change: `Parser.cs:13-16,358-362` | **LOCATED** |
| (a) | `i` prints but does not re-parse | printer `Printing.cs:537`; parser `Parser.cs:532-556`; missing constant `Interpreter.cs:309-333` (throw at `:333`) | **LOCATED** |
| (b1) | `2^(1/3)` prints but does not re-parse | printer faithful `Printing.cs:612-637`; language gap `NumericOps.cs:64,87-94` → `Real.cs:739-746` | **LOCATED (fix is a language decision)** |
| (b2) | `rootof(...)` prints but does not re-parse | printer `Printing.cs:668-669`; no builtin registered → `Interpreter.cs:619`; inverse already exists `Printing.cs:241-250` | **LOCATED** |
| (c) | `(x/y)/z` → `x/(y*z)` reassociation | printer `Printing.cs:606` with `ShapeProduct` `:715-741` (flat denominators); parser/constructors consistent `Constructors.cs:716,448-567` (no product distribution) | **LOCATED (printer side is the lossy step)** |
| (d) | `evalf(sqrt(2),5)` → 100 decimals | `SymbolicsPlugin.cs:546-555` + `1349-1357` (`:1354` passes Real through), argument pre-evaluated at ambient precision (`Interpreter.cs:1451-1468,1472-1488`, `SymbolicsPlugin.cs:1455-1471`), formatted outside the scope (`ValueFormatter.cs:18`, `Real.cs:1885-1886`, `Real.cs:43`/`Interpreter.cs:83-84`) | **LOCATED (D1 primary, D2 contributing; probe predicted)** |

**Could not locate / explicitly OPEN:** the exact digit reading the `evalf` contract must adopt
(significant vs decimal places); whether `MathIR/Evaluator.cs:453,483` and
`CompilationTests.cs:88,119` share (d); whether any `Lovelace.Run.Tests` golden prints an irrational
Real that (d)'s fix would move; whether any other printer-emitted spelling lacks a reader besides
`i`/`rootof` (a systematic printer-spelling ↔ builtin-registry sweep was out of scope).
