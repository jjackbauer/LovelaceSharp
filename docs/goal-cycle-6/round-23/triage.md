# Round 23 — wave 5 triage (the fixed rule: P0/P1 → control-failing test, land, re-publish, another wave; P2 → dispositioned in writing)

Wave 5 ran three personas with strategies none of the six earlier waves had used, against
`out\aot\Lovelace.Run.exe` **5 764 608 bytes / 2026-09-12 21:05:23 / 4 349 s newer than the newest
non-generated source / `git diff --name-only HEAD -- '*.cs'` = 0** (EVD-334).

| persona | strategy | result |
|---|---|---|
| **N** | state-machine / protocol fuzzing against the documented grammar (778 audited runs, every envelope checked against the protocol's own invariants) | **1 P1 (N-1) + 4 P2** |
| **O** | fresh-eyes regression hunt re-attacking every fix landed this cycle | (see below) |
| **P** | second-implementation cross-check (Symbolics vs Complex vs Dsp vs Real, AOT vs JIT) + locale/encoding/resource variation | (see below) |

---

## Audit N — `round-23/audit-N-protocol-fuzz.md`

### N-1 — P1 — the trailing line of captured print output was dropped while `hasOutput` said it was written

**Verdict: CONFIRMED by my own reproduction, twice, with controls; root-caused; fix written and verified
in worktrees; landing and re-publish are held only until O and P stop reading the published binary.**

* My transcript (`round-23/n1-reverify.txt`, two identical passes): `print("a"); print()` →
  `output:["a"]` with `hasOutput:(true,true)`; `print("a"); print(); print()` → `["a"]` with three
  trues; the failing path `print("a"); print(); det(1)` → exit 1 with `["a"]` and `(true,true,false)`.
* **Controls that make it a defect rather than a convention**: an interior blank line is kept
  (`print("a"); print(""); print("b")` → `["a","","b"]`) and a single blank print is kept
  (`print()` → `[""]`).
* **Cause**: `Runner.cs:604` removed EVERY trailing terminator; the capture writer appends one per
  printed line. Both Studio hosts already drop exactly one (`EngineHost.cs:319-326`,
  `IncrementalRunner.cs:418-425`) — the CLI was the only copy that removed more.
* **Fix**: strip exactly one terminator (CRLF as a unit, else one LF/CR), keep the per-element CR trim.
* **Control/fix evidence**: control tree at `dfae21b` + the test file only → **4 of 7 fail**; fix tree →
  **7 of 7 pass**; `Run.Tests` **352/0**, Console 15/0, Studio 22/0; JIT wire `output:["a",""]`
  (EVD-341, `round-23/n1-implementation.md`).
* **One control of the audit's I could not reproduce, recorded**: its `print("a\n")` row came from a real
  LF in an argv string; through `--file` the language does not process the escape and the element is
  `"a\\n"`. The finding does not rest on that row.

### N-2 — P2 — `--omit-variables` changes the `--text` display

**CONFIRMED by my own probe** (`x = 42; x --text` prints `  _ = 42` / `  x = 42`; with
`--omit-variables` neither line). **Disposition: documentation corrected, behaviour kept** (DEC-009's
disposition shape — the flag's whole point is that the caller does not want the variable table, and
`--help` already scopes both flags to "an EMPTY variables array"). `dsh-protocol.md`'s payload-control
paragraph now says the JSON envelope is unchanged and names `--text` as the one display that differs.

### N-3 — P2 — the documented `Function` value kind is unreachable

**CONFIRMED**: `func square(x) = x ^ 2; square`, `print(square)`, `f = square`, `type(square)` all
answer `InvalidOperation` / "Undefined variable 'square'." **Disposition: documentation corrected** —
`Lovelace.Suite/docs/Language.md`'s value-kind table now states that a `func` name is not a value in the
language today (and the doctests still pass, 106/106 in the worktree). Whether function values should
become reachable is a scope decision, not a defect fix, and is NOT silently granted here.

### N-4 — P2 — the document's closed lists omit what `cancel_full` publishes

**CONFIRMED**: `cancel_full((x^2 - 1)/(x - 1))` publishes enum type **`CancelStatus`** and record
**`CancelResult`**, which has **no `diagnostics` field** (`status`, `original`, `expression`,
`changed`, `conditions`). **Disposition: documentation corrected in both places** — `CancelStatus` joins
the enum-type list and `CancelResult` is named as the one rich record without `diagnostics`.

### N-5 — P2 — a plot written before a later failure is on disk but not in the envelope

**CONFIRMED**: `plot([1, 2, 3]); det(1)` exits 1 with an error envelope carrying no `plot` block and no
path, while `plot.svg` (14 636 bytes) is written. **Disposition: documented** next to the existing
"one plot per evaluation" paragraph (M-4's disposition shape); whether the error envelope should carry
the path is a design question, not a correctness fix.

### Audit N's own "could not check" list (read it as part of the verdict)

`Diagnostic.location` as a `DiagnosticLocation` record (all samples `Null`), `recoverable:false`,
`transform.unsatisfiable-conditions`, `transform.budget-exceeded`, `RuleClassification`,
`SolveStatus.BudgetExceeded`, `classification` fields, two evaluations in one invocation, concurrency,
non-UTF-8 encodings, `--print-budget` monotonicity. **An untested area is not a clean area.**

---

## Audit O — `round-23/audit-O-regression-hunt.md`

The persona that re-attacked every fix landed this cycle from one step outside its pinned case. It
also produced the most useful **negative** evidence of the wave: 13 fix rows where the fix survived
(20 kink-series probes, 25 pole/branch probes with hand-checked Laurent coefficients, BOM across
`--file`/a real-process `--stdin`/`--eval` plus UTF-16/32, a 24-run cancel-ledger battery with the
invariant holding 24/24, both prose fixes probed sentence by sentence, the M-2 depth number stable at
513 across eight compositions with no abort at 50k nesting, the evalf clamp honest).

### O-1 — P1 — `setprecision(<wider than Int64>)` escaped as the raw CLR message

**CONFIRMED by me, twice** (EVD-343): `ArithmeticError`/`DomainError`, message "Value was either too
large or too small for an Int64.", naming no numbers — while the control `pi(<the same count>)` on the
same binary answers `InvalidArgument`/`TypeMismatch` naming the count and the cap, four lines above it
in the same file. **CLOSED** by routing the count through the same argument-refusal grammar
(`long.TryParse` + `ArgumentException`); control tree **6 of 10 fail**, fixed tree **10/10**, wire
verified twice (EVD-344).

### O-2 — P1 **re-graded to P2** — `series(..., order)` refused with the raw framework text

**CONFIRMED by me** (EVD-343): `InvalidArgument`/`TypeMismatch` with "Specified argument was out of the
range of valid values. (Parameter 'order')". **The code and category were already the documented ones**,
so by the precedent that re-graded K-1 from P1 to P2 (EVD-316/EVD-320) the residue is the MESSAGE, and
that is a P2. **CLOSED anyway** in the same landing (it is the same class and the message named no
numbers): the refusal now reads "a series expansion order must be at least 1 (the number of terms to
carry, with the O-term); got -1." The wider-than-Int64 form is closed with it through
`SymbolicsPlugin.AsLong`.

### O-3 — P2 — on a BOM-prefixed file the parser MESSAGE names the engine offset, not the caller's

**CONFIRMED, and my measurement corrects the roles the persona's row implied.** Three shapes, one
binary: a file with a BOM only in the MIDDLE reports message "at position 7" and
`diagnostics.position = 7` (they agree); the same file with a LEADING BOM reports message "at position
7" while `diagnostics.position = 8` — and Python (not PowerShell) shows the offending U+FEFF really is
at caller index **8**. So the structured field, which is what a machine consumer reads, is **right**,
and the human-readable **message** carries the engine offset. **OPEN — P2, disposition recorded**: the
machine API is correct, so this is a message-prose defect, and it is not fixed in this round. (My first
probe read the indices through PowerShell's `[string]` indexing and printed 6/7 instead of 7/8; the
Python dump is the one cited.)

### O-4 — P1 (a) and P1 (b) — two more raw CLR refusals, outside the 13 fixes

**CONFIRMED by me, twice** (EVD-343): `[1, 2, 3][10^30]` → the raw Int64 conversion text;
`zeros(-1)` → "Arithmetic operation resulted in an overflow." (from `FillArrayValue`'s `checked`
arithmetic after a negative dimension was accepted). **CLOSED** with the same argument grammar:
`ToLong` parses with `TryParse` and names the value and the bound, and `ParseShape` refuses a negative
dimension by name and position (EVD-344). Controls pinned: `zeros(0)` still answers `[]`,
`[1, 2, 3][-1]` keeps its documented message with its numbers, `setprecision(-5)` keeps its naming
refusal.

---

## Audit P — `round-23/audit-P-crossimpl.md`

The cross-implementation persona: the same value through every independent implementation in the
product (Real tier, Complex tier, Symbolics `evalf`, the CLI, AOT vs JIT) and against mpmath/SymPy.
Its clean rows are as valuable as its findings: **AOT vs JIT 76/76 byte-identical** after stripping
duration fields; **`SuiteEngine.ProjectValue` vs the Runner projection 42/42 identical** (K-2 stays
fixed); exact `det`/`inv`/`matmul`/`linsolve` match SymPy; **every `solve`/`solve_full` root
re-substitutes to residual 0**; `fft == dft == mpmath` on lengths 4, 8 and a padded 3→8; conv/filter/
moving-average match hand/mpmath ground truth; the CLI's `evalf(sin/cos/exp/log)` is correct to every
requested digit at 30/60/100/200. **The binary under test produced no wrong value anywhere in its
numeric battery.**

### P-1 — P1 **re-graded to NO SEVERITY CLASS (open cost bound)** — `solve()` cost cliff

**CONFIRMED by me on the published binary**: `solve(x^2 - 10^12, x)` **210 ms**, `10^16` **17 208 ms**
with a 15 s budget, and `10^30` no answer inside the window; the persona's 150 s run of `10^17` did
finish. **Not a wrong value and not a false claim**: every root it returns re-substitutes to residual 0,
and the deadline ledger is HONEST about not stopping it — my own probe with `--cancel-after 100` on the
`10^16` case answered `{"budgetMs":100,"elapsedMs":10351.82,"stopped":false,"exceeded":true,
"excessMs":10251.82}`, i.e. it reports that the deadline did not stop the solve rather than claiming a
stop that never happened (which is exactly what I-1 was about). **Recorded in §P.2 as an OPEN cost
bound of the same class as "evalf above 1000 places is minutes-slow"** — the honest fix is a solver
cost decision, not a severity fix.

### P-2 — P1 — `Real.Sin`/`Real.Cos` returned hundreds of digits of FALSE PRECISION

**CONFIRMED by me against mpmath** (EVD-345): `Real.Sin(1)` at precision 30 printed **607 fractional
digits of which only the leading 40** — exactly the internal `digits + 10` guard — matched mpmath's
`sin(1)` at 1900 dps; the tail is the exact decimal expansion of the truncated Taylor RATIONAL, which
is why it shows repeating blocks. `ComplexMath.Sin` answered 30 digits, the CLI answered 30, so the
library tier was the outlier and a host reading `Real.Sin` directly received invented digits with
`Truncated: null`. **NOT CLOSED — and the two attempts are recorded as falsified rather than hidden
(the EVD-314 pattern).** Attempt 1 truncated the series result to the caller's `digits` fractional
places; the project's own tests falsified it: **12 failures** (`RealTrigPiResolutionPropertyTests`
×9 — `cos(π/2 − 10^-100)` is ~10^-100 and an absolute cut at 100 places deletes the value itself —
plus 3 `Dsp.Tests`). Attempt 2 was scale-aware (keep the digits the computation actually determined,
measured from the value's own first significant digit); it still failed **6 of those property tests**.
Both were reverted, the tree re-measured green, and the attempt's tests are preserved OUTSIDE the
product tree at `round-23/p2-preserved/RealTrigAccuracyTests.cs`. **P-2 therefore stands OPEN as a
P1 at the library boundary**, with the finding, the measurement and the two falsified fixes all on
disk. It is reachable only by an embedder calling `Lovelace.Real.Real.Sin`/`Cos` directly; no CLI or
AOT surface publishes the false digits.

### P-3 — P2 — the last requested digit is truncated, not rounded

**Accepted as recorded (no change).** It is the product's own convention on every surface (the CLI's
`evalf(cos(1), 30)` truncates the same way), so it is a documented precision convention rather than a
defect. Recorded in §P.2.

### P-4 — P2 — `Real.Pow(Real)` throws a raw `NotImplementedException` for non-integer exponents

**Accepted as recorded (no change this round).** `ComplexMath.Pow` computes the value and the CLI maps
the same message to a typed `UnsupportedOperation` refusal, so the language surface holds; what remains
is a library-level raw exception. Re-measured and carried into §P.2 as open.

### Audit P's own untested list (read it as part of the verdict)

Locale (no per-process culture override on Windows, so the `InvariantGlobalization` AOT/JIT asymmetry
is untested), `chcp` 437/65001, `LANG`/`LC_ALL`, non-BMP text, path shapes, file shapes, `fft`
lengths above 8, and the true cost of the `k >= 18` solve cases (killed at 15 s). **An untested area is
not a clean area.**
