# LovelaceSharp — advancement cycle 7: correctness, harnessability, verification

**Status:** planning only. No implementation has begun; nothing in the product tree was modified.
**Planning base:** `ce30fc3` (`bench(harnessability): add the harnessability benchmark and its gate evidence`), clean working tree.
**All numbers below were measured in this session** on that revision, with commands recorded inline. Library-level
measurements used an out-of-tree probe (`C:\Users\ricar\dev\.lovelace-adv-probe`) that references the built
assemblies; wire measurements used the Native AOT runner **re-published from HEAD** (5 815 808 bytes,
2026-09-16 15:01:46, `out/aot/Lovelace.Run.exe` — I republished it because the committed one predated HEAD, see F-5).

---

## A. Executive assessment

LovelaceSharp at HEAD is not a promising prototype; it is a **finished, unusually disciplined platform** with a
governed failure surface. What is already strong, verified rather than asserted:

* **Numerics.** `Natural` → `Integer` → `Real` (exact periodic fractions) → `Complex`, Lean-proved digit
  arithmetic, Native AOT throughout.
* **One engine, several projections.** `Lovelace.Suite` owns the language; REPL, Studio, `Lovelace.Run`/DSH are
  thin. 123 registered builtins in deterministic alphabetical order (verified: three runs produced byte-identical
  name orders, 914 characters each).
* **A real symbolic kernel.** Hash-consed `Expr` DAG, a rewrite registry with stable rule ids, a `RewriteProofs`
  obligation bridge that a test keeps in step with the registry, solvers, and MathIR (version 2) with an AOT-safe
  evaluator.
* **A versioned structured protocol** (`docs/symbolics/dsh-protocol.md`, 348 lines) with a frozen seven-member
  `ErrorCategory`, fine-grained stable codes, one diagnostic per failure, and a genuinely good name-resolution
  surface: an unresolved *function* crosses as `UnknownFunction`/`UnsupportedOperation` with
  `suggestions`/`hints`/`registrySize` (verified at HEAD: `determinant` → `Did you mean 'det'?`).
* **Verification machinery that already pays rent.** `Lovelace.Knowledge` (MGIR) discovers behaviour planes and
  fits guards by seeded Monte Carlo against the real binary, with a published confidence ladder; the symbolics
  suite already carries a **falsification battery over the rewrite registry** (`FalsificationTests.cs`,
  `FalsificationGateTests.cs`, 1098 lines of region-tagged probes: `near-pole`, `near-branch-cut`,
  `near-discontinuity`, `assumption-boundary`, `complex`).
* **Green baseline.** Full sweep on this machine at HEAD: **15 suites, 5 685 passed, 0 failed, 8 skipped, ~26 s**
  wall (`dotnet test LovelaceSharp.slnx -c Release`). CI (`.github/workflows/ci.yml`) pins 16 projects by name,
  runs the SymPy oracle fatally, and publishes **and executes** the AOT runner.

**Where the highest-leverage deficiencies actually are.** Not breadth. Three dishonesty classes, in priority order:

1. **The machine API still silently changes what the caller asked for.** `evalir(…, digits)` /
   `evalir_batch(…, digits)` **floor** a non-integral digit count (`2.5` → 2), **accept 0 and negative counts**
   and answer `0` with `ok: true`, and **overflow past Int64 as raw CLR text**. A precision argument is a claim
   about the answer, so a silent truncation is a wrong answer that looks like a right one. The whole class was
   closed for `series` and `setprecision` in round 23; the MathIR boundary was missed.
2. **A transcendental contract that is false inside its own declared request.** The known-open P-2 ("hundreds of
   digits of false precision") is real but **understated**: at the precision the existing property tests actually
   use (30), `Real.Sin(1, 30)` has 606 fractional digits of which **40** are sin(1); and `Real.Sin(10, 30)` has
   **29** correct digits — wrong *inside* the 30 the summary promises. Everything past the internal guard is the
   exact decimal expansion of a truncated Taylor rational, which is why it shows repeating blocks.
3. **Observation-driven verification exists but is not pointed at the rewrites.** The falsifier is a fixed probe
   list; `Lovelace.Knowledge`'s sampling lives on a different type universe (operand pairs over a fixed arithmetic
   domain) and cannot express a rewrite at all. There is no minimiser and no regression exporter.

The cycle should therefore be **narrow and adversarial**: make every integer-semantic argument honest, make the
transcendental boundary honest, and convert the existing falsification battery into a discovery→minimize→export
pipeline. Everything else is secondary.

---

## B. Current Truth

Legend: **CONFIRMED** · **ALREADY FIXED** · **PARTIALLY FIXED** · **SUPERSEDED** · **NOT REPRODUCED** ·
**NEEDS MORE EVIDENCE**.

| # | Claim in the brief | Verdict | Evidence |
|---|---|---|---|
| B1 | `series(sin(x), x, 0, 2.5)` behaves as order 2 | **CONFIRMED** | `SymbolicsPlugin.cs:2249` `Rl r => ParseInt64Argument(r.ToString().Split('.')[0])`; on the wire at HEAD: `ok:true, output:["x + O(x^2)"]`. `2.9999` → `O(x^2)` too. Recorded open as **Q-2** (`round-23/triage.md:224-232`). |
| B2 | It is a *class* of semantic defects, not one bug | **CONFIRMED, and wider than the brief states** | The truncating helper `SymbolicsPlugin.AsLong` has exactly **one** remaining caller (`series` order, `SymbolicsPlugin.cs:489`). The class lives in **two more places the brief does not name**: `Lovelace.MathIR/Evaluator.cs:592-599` (`evalir`/`evalir_batch` digits) and, more mildly, `Real.Pow`'s exponent path (F-3). Nine sites, three helpers, three different rules — table below. |
| B3 | `Real.Sin`/`Real.Cos` expose hundreds of digits where only a prefix is justified | **CONFIRMED, and the justified prefix is not `digits+10`** | `Real.Sin(1)` under `WithPrecision(30,30)`: 606 fractional digits, `IsExact=false`. Against mpmath at 4000 dps the correct counts are 40 for `Sin(1,30)` but **59** for `Sin(1,50)`, **29** for `Sin(10,30)`, **0** for `Sin(10^100,30)` — see F-2. |
| B4 | Previous truncation-based fixes were falsified | **CONFIRMED as recorded; both were *absolute* truncations, so the falsification does not generalise to a scale-aware cap** | `round-23/p2-preserved/RealTrigAccuracyTests.cs` (preserved outside the product tree) pins `FractionalDigits(Sin(1,30)) <= 30`. The real blocker is instructive: `RealTrigPiResolutionPropertyTests.cs:127-145` requires `Real.Cos(pi(100)/2, 30)` ≈ `4.11e-101` — an absolute cut at 30 places deletes the value. I could not reproduce the two attempts themselves (their diffs are not on disk), so "a scale-aware cap cannot work" is **NEEDS MORE EVIDENCE**; my own analysis says it can (D-2). |
| B5 | `CertifiedApproximation{Value,ErrorBound,CertifiedSignificantDigits}` may be the deeper contract | **NEEDS MORE EVIDENCE / likely the wrong shape** | The repository already marks exactness on the value itself (`Real.IsExact` + `MarkInexact` + `AsInexact` + `WithProvenanceOf`, `Real.cs:192-261`), threaded through the arithmetic and the transcendental kernels. A parallel certified-approximation ADT would be a second representation of the same fact. A **derived** accuracy query over the existing marker is the smaller coherent change. |
| B6 | The false precision is only reachable by embedders | **CONFIRMED for the audit's cases; a new counter-shape found** | CLI `evalf(sin(1),30)` = `0.84147098480789650665250232163` (30/30 correct) and `ComplexMath.Sin` answers 30, so the library tier is the outlier. But `print(dft([0.1,0.2,0.3,0.4,0.5]))` at `setprecision(60)` publishes a **63 996-byte** envelope whose first element alone carries ~700 fractional digits — over-delivery through `output[]`, which the protocol's 1000-place cap does not bound (the cap is on the structured projection, `dsh-protocol.md:24-28`). |
| B7 | Function-name/API discovery is a substantial source of wasted evaluations | **PARTIALLY FIXED — measurably reduced, not closed** | WS-1.1/WS-5 landed (`be16012`; verified at HEAD: `UnknownFunction`/`UnsupportedOperation` + `suggestions`). Gate run A: H_10 L1 0.715→0.860, L2 0.805→0.880, unknown-name failures 15→8, L3−L2 gap 0.175→0.070. The residual is one task shape (`transpose-purpose`: the agent wants `rows`, the repair needs `transpose`; `rows` now suggests `shape`) and one untouched class: **an unresolved *variable* is still `InvalidOperation`/`DomainError`** with no suggestions (F-4). |
| B8 | `--functions [pattern]` / `--function <name>` do not exist | **CONFIRMED** | `Lovelace.Run/Runner.cs` has only `--omit-functions` (line 96) and its help line (674). The only registry access is a full evaluation: 15 677 bytes for `--eval '0' --json` with 123 entries carrying only `name, parameters, minArity, parameterCount, variadic, builtin`. The benchmark's L3 `function_projection` also expects `plugin` (`bench-mcp/server.mjs:103-113`), which the runner never sends. |
| B9 | An arity failure should carry structured signature information | **CONFIRMED, and the type layer already has it** | `BuiltinArityException` (`Functions.cs:123-161`) already exposes `Builtin, ExpectedMin, ExpectedMax, Variadic, Actual`; `BuiltinShapeException` (178-203) exposes `Builtin, Position, Expected, Arrived`. `Runner.cs:327-330` passes **only `ex.Message`** to `WriteError`, so all of it is dropped at the wire. |
| B10 | A failure should reveal how far a multi-statement run progressed | **PARTIALLY FIXED** | `output[]` crosses on the error path (`Runner.cs:313`; invariant 1 is not scoped to success). `partialVariables` is populated **only** when `code == "Cancelled"` (`Runner.cs:317-323`) — still 0 % coverage on ordinary failures, exactly as the benchmark measured. `partialOutput` exists only on cancellation. |
| B11 | `plot(...)` followed by a failure leaves an artifact invisible to the envelope | **CONFIRMED** | `plot([1,2,3])` then a failing statement exits 1 with no `plot` block and no path while the SVG is on disk. Dispositioned as documented behaviour (**N-5**, `round-23/triage.md:62-67`); the brief asks to revisit that disposition, which is fair. |
| B12 | Operations can claim a deadline they do not honour | **CONFIRMED, and the ledger is already honest about it** | `solve(x^2-10^16, x)` under `--cancel-after 300`: **wall 7 028 ms**, `ok:true`, ledger `{budgetMs:300, elapsedMs:6871.492, stopped:false, exceeded:true, excessMs:6571.492}`. Contrast `series(…, 100000)` under the same budget: **wall 480 ms**, `Cancelled`, `stopped:true`. So the *claim* is honest and the *coverage* is partial — P-1 is a cost bound, not a false claim. |
| B13 | Budget dimensions beyond wall clock are missing | **PARTIALLY FIXED** | `ArrayAllocationBudget.cs`, `InputDepth.cs`, `AstDepth.cs`, `ValueDepth.cs`, `Abstractions/Cancellation.cs` exist, with 89 product touchpoints of cancellation/budget APIs. `Lovelace.Symbolics` contributes **13** of them across 8 files, and `Solvers/Solve.cs` (80 KB) has **2**. |
| B14 | Public library boundaries leak raw framework exceptions | **CONFIRMED, narrower than feared** | `Real.Pow(Real)` throws raw `NotImplementedException` for a non-integral exponent (`Real.cs:1225`) and for any negative exponent (1234). `Runner.Classify` maps `NotImplementedException`→`UnsupportedOperation` (428) and `ArgumentException`→`InvalidArgument`/`TypeMismatch` (425), so most of the wire is protected; what survives are *message*-level leaks (F-3). |
| B15 | The failure-state fields are consistent across CLI / SuiteEngine / Studio / DSH | **NEEDS MORE EVIDENCE** | Not measured this session. The three `SplitLines` copies and `ProjectValue` vs `StructuredProjection` were reconciled in earlier rounds (K-2, N-1), and audit R found Invariant 1 holds byte-wise over 37 runs; I did not re-derive it. |
| B16 | `Lovelace.Knowledge` provides observation maps / planes / boundaries / a confidence ladder | **CONFIRMED** | `Model.cs` (`Plane`, `Guard`, `BoundaryEdge`, `Frontier`, `Confidence{Hypothesized…Proven}`), `Proposal.cs`, `Observation.cs`, `Reducer.cs`, `ConvergeLoop.cs`; demo graph `knowledge-graph-demo.json` (217 668 bytes, 314 samples). Guards are `threshold`/`equality`/`composite` only. |
| B17 | Rewrite-rule falsification is *contemplated* by the symbolics plans | **SUPERSEDED** | It is not contemplated — it is **shipped**: `FalsificationTests.cs` + `FalsificationGateTests.cs` (1098 lines) with region-tagged probes and a mandatory rationale per probe, plus `RewriteProofs` obligations for all 9 shipped rules. What is missing is automated region discovery, minimisation and export. |
| B18 | Equality saturation may be the next step | **NOT READY — recommend declining this cycle** | No e-graph code exists, and the brief's own gates are unmet: coverage is a fixed probe battery (gate 1 weak), MathIR still carries F-1 at its argument boundary (gate 2 unmet), no budget/node-growth abstraction exists (gate 3 unmet). Scored in `D-8`. |
| B19 | `out/aot/Lovelace.Run.exe` is the artefact agents read | **CONFIRMED, and it was STALE** | At session start: 5 767 680 bytes / 2026-09-12 21:51:38 — **four commits behind HEAD**, missing `be16012` (name suggestions) and `ce30fc3`. Three of my own wire probes were made against it and discarded. `make runner` publishes elsewhere — to the path `harness/lovelace.host.js:94` actually spawns. See F-5. |

### B2 detail — the integer-argument surface, enumerated

| Site | Helper | Behaviour on a non-integral `Real` | Bounds enforced | Failure classification |
|---|---|---|---|---|
| `SymbolicsPlugin.cs:2247` `AsLong(object?)` | text-split floor | **silently floors** | Int64, message only | `ArgumentException` |
| `SymbolicsPlugin.cs:2245` `AsInt` | `(int)AsLong` | silently floors | none (unchecked cast) | `ArgumentException` |
| `SymbolicsPlugin.cs:2219` `DigitCount` (`evalf`) | strict | refuses (correct) | `[1, int.MaxValue]` | `ArgumentException` |
| `Interpreter.cs:1148` `ToLong(Value)` | strict | refuses (correct) | Int64 | `ArgumentException` |
| `Interpreter.cs:1141` `ToInteger` | strict | refuses (correct) | — | `InvalidOperationException` |
| `Interpreter.cs:1952` digit counts (`pi`/`e`) | strict | refuses (correct) | `[1, engine cap]` | `InvalidOperationException` |
| `Interpreter.cs:2129` `ParseShape` + `2136` | via `ToLong` | refuses (correct) | non-negative + allocation budget | `ArgumentException` |
| `DspPlugin.cs:345` `ToLong` | strict | refuses (correct) | Int64, non-negative, Int32 seed | `InvalidOperationException` |
| **`MathIR/Evaluator.cs:592` `AsLong`** | **text-split floor** | **silently floors** | **none** | raw `OverflowException` past Int64 |

Nine sites, three helpers, three different semantic rules, and two of them silently floor. The brief's instinct —
one canonical contract — is right; this table is the argument for it.

---

## C. Findings

Severity ladder in use by this repository (from `round-23/audit-P-crossimpl.md:9-11`): **P0** a wrong value or
abort on valid input; **P1** a false claim in the machine API, a wrong refusal, or data the API says it carries
being lost or altered; **P2** cosmetic or documentation. New findings are marked **NEW**; previously recorded ones
name their id.

### F-1 — P1 — NEW (same class as Q-2, different boundary): `evalir`/`evalir_batch` silently floor, unbind and overflow the `digits` argument

* Root cause: `Lovelace.MathIR/Evaluator.cs:592-599`
  ```csharp
  private static long AsLong(object? o) => o switch {
      global::Lovelace.Real.Real r => long.Parse(r.ToString().Split('.')[0]),   // floors
      global::Lovelace.Integer.Integer i => long.Parse(i.ToString()),
      global::Lovelace.Natural.Natural n => long.Parse(n.ToString()),
      long l => l, _ => throw new InvalidOperationException("Expected an integer.") };
  ```
  and its two call sites, `Evaluator.cs:440` and `:476`: `var digits = (int)AsLong(args[2]!);`.
* Reproduction (AOT published from HEAD, through `--stdin`, base64-encoded to survive PowerShell 5.1's
  double-quote stripping — EVD-299):
  * `x = symbol("x"); print(evalir(compile(x*1/3, [x]), [1], 2.9))` → `ok:true, output:["0.33"]` — 2.9 became 2.
  * `…, [1], 0)` → `ok:true, output:["0"]`; `…, [1], -5)` → `ok:true, output:["0"]` — a nonsense request answered
    with a plausible wrong number and `ok:true`.
  * `…, [1], 10^30)` → `FAIL ArithmeticError/DomainError: Value was either too large or too small for an Int64.`
    — raw CLR text, naming no number, in the same file whose `evalf` sibling answers a numbered refusal
    (`SymbolicsPlugin.cs:2243`).
  * Controls: `…, [1], 40)` and `print(evalf(1/3, 40))` both answer `0.3333…` to 40 places — the integer path
    works; only the argument contract is missing.
* Why P1: a precision argument is a claim about the answer. Flooring it changes the answer while `ok:true` and the
  value's own rendering stay silent. This is the class round 23 closed for `setprecision`, `series`, `zeros` and
  indexing.

### F-2 — P1 — `Real.Sin`/`Real.Cos` are wrong **inside** the requested precision, not merely over-delivering past it (extends the open P-2)

* Method: an out-of-tree probe emitted the fractional-digit strings of `Real.Sin/Cos` at several precisions; a
  Python/mpmath 1.3.0 comparison at 4000 dps counted the correct prefix.
* Result:

  | case | fractional digits emitted | correct digits | `digits+10` |
  |---|---:|---:|---:|
  | `Sin(1, 30)` | 606 | 40 | 40 |
  | `Sin(1, 50)` | 1378 | **59** | 60 |
  | `Sin(1, 100)` | 4068 | **109** | 110 |
  | `Sin(1, 200)` | 12605 | **209** | 210 |
  | `Sin(10, 30)` | 1453 | **29** | 40 |
  | `Sin(10, 50)` | 3133 | **49** | 60 |
  | `Sin(10, 100)` | 10310 | **99** | 110 |
  | `Sin(10, 200)` | 33516 | **199** | 210 |
  | `Sin(10^100, 30)` | 1754 | **0** | 40 |

  `Cos` behaves the same way (641 digits / 40 correct at 30; 1381 / 60 at 50). `Exp` and `Sqrt` are clean
  (audit P §3.1 rows 8 and 10).
* **This is the part the previous cycle did not have.** The preserved failed attempt and the surviving property
  tests are all about *how many digits to cut*. But `Sin(10, 30)` has no 30 correct digits to cut to: the working
  precision is short by one place for `Sin(1)`, by eleven for `Sin(10, 30)`, and by the whole reduction cost for
  `Sin(10^100, 30)`. A truncation-only fix, even a correct one, would publish a *shorter wrong value* and keep
  `ok:true`.
* The tail's repeating blocks (`…178861788617886178861788617886…`, `…962962962962962996296…`) are the tell: past
  the guard the value is the exact decimal expansion of a truncated Taylor rational, not a digit of sin.
* Reachability: `Real.Sin`/`Real.Cos` are public `Lovelace.Real` API. The CLI's `evalf` path is correct and
  `ComplexMath.Sin` is correct, so the product ships **two implementations of sin and only the public library one
  is wrong** — the same "second implementation" shape audit P reported.

### F-3 — P2 — the `Real.Pow` exponent surface has an exact-answer hole and a raw-message leak (extends P-4)

* `Real.cs:1225` throws `NotImplementedException("Non-integer exponents are not yet supported.")`;
  `Real.cs:1234` throws `NotImplementedException("Negative exponents are not yet supported.")`.
* Wire at HEAD: `8 ^ (1/3)` and `4 ^ 0.5` → `UnsupportedOperation/UnsupportedOperation` (typed, good);
  `(1/2) ^ (-1)` → the same, **although `(1/2)^-1 = 2` is an exact rational answer the engine can already
  represent**; `0 ^ (-1)` → `InvalidArgument/TypeMismatch: Base cannot be zero. (Parameter 'base')`, a raw
  framework message on the wire.
* The brief's three options (rational-exponent subset / route through complex / typed unsupported) are all
  available; `D-1`-adjacent comparison in ADV-06.

### F-4 — P2 — NEW: an unresolved **variable** is still `InvalidOperation`/`DomainError`, with no registry answer

* Wire at HEAD: `print(zzz)` → `code:"InvalidOperation", category:"DomainError", message:"Undefined variable 'zzz'."`
  — the exact pair F2/WS-5 removed for *functions*, because a consumer cannot branch on "you used a name I do not
  have" versus "your mathematics is undefined".
* `Interpreter.cs:814` throws a bare `InvalidOperationException`; the fix pattern already exists one file away
  (`UnknownFunctionException` in `EngineExceptions.cs:68`, whose `WireSuggestions`/`WireHints`/`RegistrySize`
  are already threaded through `WriteError`).
* Cheapest remaining structural win on the harnessability axis, and **not in the brief**.

### F-5 — P1 for agents (process), not a product defect — the documented agent artefact was four commits stale

* `out/aot/Lovelace.Run.exe` at session start: 5 767 680 bytes, 2026-09-12 21:51:38. HEAD's product code is
  `ce30fc3` (2026-09-16) and includes `be16012` (name suggestions) — an agent reading that binary saw an engine
  without `UnknownFunction`, without `suggestions`, and without the six round-23 refusal repairs.
* `make runner` publishes to `Lovelace.Run/bin/Release/net10.0/publish` (Makefile:15, 48-53), exactly the path
  `harness/lovelace.host.js:94` spawns; `out/aot` is the CI publish-verification path (`ci.yml:263`). Two paths,
  no freshness assertion anywhere; every audit in cycle 6 hand-measured "N seconds newer than the newest
  non-generated source".
* Consequence for this cycle: **Gate 0 must publish and fingerprint both paths, and every claim about the agent
  surface must name the binary it measured.**

### F-6 — P2 — small, previously unfiled odds and ends

* `--text` writes its human lines to stderr on the failure path (recorded as **R-3**); the JSON path is unaffected.
* On the cancelled path `output[]` and `partialOutput[]` duplicate the same entries with nothing saying which is a
  superset (**R-3**).
* `excessMs` is computed from a stopwatch started *with* the deadline while `elapsedMs` is the engine's own
  elapsed (**R-1**) — two clocks, documented nowhere.
* `--omit-functions`/`--omit-variables` deliberately emit `[]` rather than omitting the key; audit N recorded the
  opposite expectation (**N-2**), dispositioned "behaviour kept, documentation corrected". Not a defect.

---

## D. Architecture decisions

### D-1 — Exact-integer argument conversion

**Requirement.** One rule, one implementation, one wire shape: *an argument that requires an integer must either
represent an exact integer or be refused explicitly; it is never silently rounded or truncated.*

| Option | Assessment |
|---|---|
| **A. Keep per-layer helpers, fix each** | Nine sites, three rules today; the tenth site added by the next plugin will copy whichever neighbour is closest. Rejected: this is what produced F-1. |
| **B. One static contract type in `Lovelace.Abstractions`** | *Chosen.* `Abstractions` depends only on `Representation`, and its `RegisterBuiltin` payload contract already documents that `Natural`/`Integer`/`Real` arrive as their public types (`Modus.cs:33-41`), so this is the layer that already owns "what a builtin's argument is". |
| **C. Put it in `Lovelace.Suite`** | Rejected on the dependency graph: the engine *loads* `Lovelace.Symbolics` and `Lovelace.MathIR`, so those plugins cannot reference `Suite`. |
| **D. A method on each numeric type (`Real.TryGetInt64`)** | Rejected as the *owner*: it makes each type responsible for a *language argument* policy (bounds, naming, error class), which must be identical across plugins. Accepted as the *predicate* the contract calls. |

**Shape.**
```csharp
// Lovelace.Abstractions/ExactIntegerArgument.cs
public readonly record struct Int64Range(long Min, long Max);        // declared per call site
public readonly record struct Int32Range(int  Min, int  Max);

public static class ExactIntegerArgument
{
    public static long Require(object? payload, string parameter, Int64Range bounds, string? builtin = null);
    public static int  RequireInt32(object? payload, string parameter, Int32Range bounds, string? builtin = null);
    public static bool TryGet(object? payload, out long value);      // non-throwing probe
}
```

Two design points carry the semantics:

1. **Exactness, not rounding.** `Natural`/`Integer` are exact by construction. A `Real` is accepted only when it
   *is* an integer: non-periodic, `Exponent >= 0`, no non-zero fractional digit, magnitude inside the requested
   range. A non-integral `Real` is refused. A `Real` that is exactly integral but whose *kind* the product already
   refuses (digit counts: `InterpreterBuiltinPiTests.cs:86` pins "digit counts must be Natural or Integer") is
   refused **with a different message** that tells the caller how to write it (`write it as 2, not 2.0`). This keeps
   every pinned contract intact, makes the refusal actionable, and lets `series(…, 2.5)` and `series(…, 2.0)` be
   distinguished by a consumer **without an eighth `ErrorCategory`**.
2. **Bounds are mandatory and declared at the call site.** `series` order `[1, cap]`, `evalf`/`evalir` digits
   `[1, int.MaxValue]`, `pi`/`e` `[1, engine cap]`, dimensions `[0, Int64.MaxValue]`, indices the full Int64
   range. The message names builtin, 1-based position, parameter, bound and received value — the grammar
   `pi(digits)` already uses.

**Error class.** A new `ArgumentContractException : ArgumentException` in `Abstractions`, matching the case
`Runner.Classify` already maps to `InvalidArgument`/`TypeMismatch` (`Runner.cs:425`). A new `ErrorCategory` is
**rejected**: the taxonomy is frozen at seven members and pinned by
`Lovelace.Symbolics.Tests/DiagnosticContractTests`.

**Residual risk.** Call sites that today cross as `InvalidOperationException`/`DomainError` for a *type* mistake
(`zeros(2.5)`, `[1,2,3][1.5]`) would become `InvalidArgument`/`TypeMismatch` — which is what audit R-2 asked for,
but it *moves pinned messages* in `Lovelace.Run.Tests` and `Lovelace.Suite.Tests`. Mitigation: migrate per site with
a control-tree test each; where a message is pinned, the site keeps its current exception type and gains only the
contract (exactness + bounds).

### D-2 — Certified approximation and error tracking

| Option | Assessment |
|---|---|
| **A. `CertifiedApproximation{Value, ErrorBound, CertifiedDigits}` as a new public numeric type** | Rejected. `Real` already carries exactness (`IsExact`, `MarkInexact`, `AsInexact`, `WithProvenanceOf` — `Real.cs:192-261`) threaded through `+ − × ÷`, `Sqrt`, `PiTo`, `Exp`, `Sin`, `Cos`. A second representation of the same fact guarantees drift, and a full error-interval redesign is out of proportion to a defect whose only reachable surface is a direct library call. |
| **B. Truncate the emitted digits** | **Falsified twice already** (`p2-preserved/RealTrigAccuracyTests.cs`; the surviving property tests ×9 then ×6). Rejected as a *fix*; retained as a *component* — see below. |
| **C. Certified-prefix contract: fix the working precision, then derive the publishable digit count from the value plus the certified bound** | *Chosen.* |
| **D. Route `Real.Sin`/`Cos` through `ComplexMath`** | Rejected as the primary fix: it would replace the exact special-angle table (`Sin(π/6) = 1/2` exactly, `Sin(π) = 0`) with an approximation, and `RealTrigPiResolutionPropertyTests.cs:202-230` pins those values **and** their `IsExact` flags. Useful as an **oracle** for ADV-05, not as an implementation. |

**The chosen contract (C), stated precisely.** For `Sin`/`Cos(value, digits)` with `guard = digits + 10`:

1. **Working precision must exceed the request by the cancellation the argument actually causes.** The argument's
   integer-digit count `i` plus the range-reduction tail must be paid for: `work = max(guard, i + digits + guard)`,
   and π must be resolved to at least `i + guard` places (`ResolvedPiDigits` already exists at
   `Real.cs:1854-1871` — this is where `Sin(10^100, 30)` fails). Calibration is *measured*, not hand-derived:
   ADV-05 sweeps `i` upward, finds the smallest work that renders the requested digits correct against mpmath, and
   the test asserts the shipped formula is at least that.
2. **The result is then capped to the digits the computation actually determined**, scale-aware and never
   value-deleting:
   ```
   certified_fractional_places = max(digits, -exponent(value)) + guard
   ```
   For `Sin(1,30)`: exponent 0 → `max(30,0)+10 = 40` places, matching the measured 40 correct digits. For
   `Cos(π/2 − 10^-100, 30)`: the value's exponent is ≈ −100, so the cap is far below the value's own scale — the
   value survives **and** its invented tail is cut. This is the rule that unifies what B4 calls two incompatible
   requirements; it is a hypothesis until ADV-05 falsifies or confirms it, and it is written that way.
3. **The value keeps `IsExact = false`** and gains no new flag; the certified count is available through an
   additive query, so the exact representation does not change.

**Explicit non-goal.** No interval arithmetic, no error propagation through `Real` arithmetic, no change to
`+`, `−`, `×`, `÷`. The cycle targets the transcendental kernels and their range reduction only.

### D-3 — Execution budgets

| Option | Assessment |
|---|---|
| **A. A new general `ExecutionBudget` (wall clock + iterations + allocation + depth + node growth)** | Rejected this cycle. Four of those five dimensions already exist as separate mechanisms (`ArrayAllocationBudget`, `InputDepth`, `AstDepth`, `ValueDepth`, `Abstractions/Cancellation` + `Poll`). Replacing them is a constitutional change with no measured defect behind it. |
| **B. A cooperative-checkpoint discipline in the one measured non-interruptible region (`solve`), plus preflight cost guards at the two known pathological entry points (`series` order, `setprecision`)** | *Chosen.* Smallest change that makes an honest claim true, and falsifiable by measurement. |
| **C. Do nothing; the ledger is already honest (`stopped:false, exceeded:true`)** | Rejected as *sufficient*, accepted as *fallback*. Honesty about not stopping is necessary and already achieved; the brief's invariant ("if the runtime claims a supported budget can stop an operation, the algorithm must periodically cooperate") is still violated for `solve` and for every symbolic loop with no `Poll`. |

**Shape.** A `WorkBudget` in `Lovelace.Abstractions` wrapping the ambient `Cancellation.Token` plus a monotone
work counter, with a single `Poll(units)` entry point that (a) cancels on the token and (b) throws the existing
`BudgetExceededException` on exhaustion — so call sites get *one* call, not a new vocabulary. Deployment is
incremental and measurement-gated: `Solve.cs` first (2 polls today, 80 KB of code, the measured 23× overrun), then
the Gröbner/expand/polynomial loops. **Algorithms that cannot be made interruptible are named as
non-interruptible in the protocol document rather than described as stoppable.**

**Preflight cost guards.** Two, both measured rather than guessed:
* `series(…, order)`: measured cost — order 500 ≈ 579 ms, order 5 000 ≈ 3.0 s, order 100 000 > 3 s at a 3 s
  budget. Refuse above a declared cap (or require an explicit opt-in) **and** surface the cap in `capabilities()`,
  so the brief's `10^11` case refuses before the process starts working instead of cancelling at 6 s.
* `setprecision(n)`: currently free to set and unbounded (Q-3). Because `pi`/`e` cap at 1000 and `evalf` at
  `int.MaxValue`, the honest bound is a declared maximum computation precision whose value is *measured* from a
  cost sweep, with the refusal naming it. Leaving it unbounded and documenting that the next precision-consuming
  computation is unbounded is the status quo that makes `--cancel-after` the only guard.

### D-4 — Failure-state transaction semantics

**Statement-level commit.** Chosen semantics match what the engine already does rather than inventing a new model:

* Statements execute in order; a statement's effect (variable bindings, `print` output, plot capture) becomes
  **committed** when the statement returns normally.
* A failure aborts the remaining statements and leaves the committed prefix intact — already true for variables and
  output (`Runner.cs:313-323`). What is *not* true is that the envelope always **says** so.
* So the change is a **projection** change, not an engine change: publish the committed variable table on **every**
  recoverable failure, not only on `Cancelled`; keep `output[]` always (already); and resolve the
  `partialOutput`/`partialVariables` duplication on the cancelled path by naming which is authoritative (chosen in
  ADV-09, with audit R-3's complaint as the acceptance test).
* **Artifacts.** A file written by a statement that *did* commit is a committed artifact and must appear in the
  envelope. The minimal coherent change is an **`artifacts` array** `[{kind, path, title, bytes}]`, populated from
  the engine's plot captures and any future file-producing builtin, printed on **both** the success and the error
  path. `plot` on the success path keeps its exact current shape (frozen); the error path stops being silent about
  a file it wrote.
* **Explicitly not transactional:** a partial `print` capture from a statement that threw mid-statement; a plot
  overwritten by a later successful call (already documented, M-4); any side effect of a cancelled statement beyond
  what it committed before the stop.
* **Consistency target:** CLI, `SuiteEngine`, Studio and DSH **project** one engine result. The engine owns the
  commit rule; the four surfaces own only rendering. Tests assert the projections agree on a shared fixture set —
  audit P's `ProjectValue`-vs-Runner differential (42/42) widened to the failure path.

### D-5 — Function discovery

| Option | Assessment |
|---|---|
| **A. `--functions [pattern]` + `--function <name>` CLI flags** | *Chosen as the wire form*: no new transport, AOT-trivial, mirrors the CLI's existing flag style. |
| **B. Separate `search-functions` / `describe-function` MCP tools** | *Chosen as the DSH form*, layered on the same flags. The two are not alternatives: the MCP server is a projection, and the brief is right that the *semantics* should be two operations — one filtered list, one exact lookup — rather than one overloaded syntax. |
| **C. A `functions(pattern)` builtin in the language** | Rejected: it would make discovery require an evaluation (the cost this workstream exists to remove) and pollute the language namespace with tooling. |

**Payload.** Reuse `BuiltinDescriptor` — already the single source of truth (`BuiltinDescriptor.cs:3-6`) and already
carrying `Parameters`, `Category`, `Summary`, `Examples`, `ReturnKind`, `Related`, `Variadic`, `MinArity`.
The runner's `FunctionDto` forwards six of those; the work is to forward the rest **plus** a `plugin` provenance
field (the benchmark's L3 projection already expects it, `bench-mcp/server.mjs:103-113`) while keeping the existing
six field names and meanings byte-stable (they are pinned by recorded trajectories and by
`bench.py::function_projection`). Deterministic alphabetical order is already true (verified over three runs);
keep it and assert it. `--functions [pattern]` returns names + arity only, bounded, with an explicit `truncated`
flag; `--function <name>` returns one full descriptor. Both are pure registry reads: **no evaluation, no `_`, no
timing ledger.**

### D-6 — Behavioural verification integration

| Option | Assessment |
|---|---|
| **A. Extend `Lovelace.Knowledge` to sample arbitrary scripts** | Rejected this cycle. `SampleSpec` is built from `(Operation, Operand Left, Operand Right)` and `Proposal` fabricates script text from that pair algebra (`Proposal.cs:159-190`); generalising it means replacing the sample type, the proposal distribution, the reducer's sweep machinery and the guard fitter at once — a rewrite of a working system for a use case it was not built for. |
| **B. A purpose-built falsifier reusing the *ideas* and the *in-process evaluator*** | *Chosen.* The symbolics suite already evaluates expressions in-process at a scoped precision (≈ms per sample versus ≈10-20 ms per process spawn), already has region-tagged probes with a mandatory rationale (`RealProbe`/`ComplexProbe`), and already has a registry of rules with stable ids and proof obligations. |
| **C. Generic black-box fuzzing** | Rejected, as the brief says: it wastes the mathematical structure that makes the search cheap. |

**Shape.** Three separable pieces, each useful alone:
1. **Region model (ADV-11).** Promote `RealProbe`'s `Region`/`Label`/`Why` into a first-class, serialisable record
   the falsifier can *report*, and add a boundary-biased proposal that draws new **exact rationals** *between* the
   two observations that straddle a disagreement (deterministic, seeded, no new randomness source).
2. **Minimiser (ADV-12).** Deterministic shrink over a stated dimension order — variables, node count, constant
   magnitude, degree, precision, distance-to-boundary — greedy, total-order, one candidate per step, re-checked
   against the same predicate, terminating at a fixpoint.
3. **Exporter (ADV-13).** Emit a deterministic `CounterexampleRecord` (rule id, canonical LHS/RHS, input,
   assumptions, observation pair, classified disagreement, precision, seed, generator version, runtime revision) and
   render it into an xUnit theory row appended to a checked-in corpus.

**Counterexample representation.** Chosen: a **record plus a rendered test**, reusing the repository's own
vocabulary (`RuleId`, `AssumptionSet`, `RewriteStep`, `Confidence`) rather than a new ontology. The observation
classifier reuses the brief's five classes because they map 1:1 onto distinctions the engine already makes:
`agree`/`disagree` from the numeric comparison; `undefined-LHS`/`undefined-RHS` from `EvaluationException`
(which `FalsificationTests.cs:53` already catches *specifically*, so an unexpected exception type still fails the
test rather than being skipped); `budget/refusal` from the typed budget exceptions.

### D-7 — Boundary exploration (derivatives and discontinuities)

Chosen: **categorical-first, scalar-second.** The model is not allowed to assume smoothness, so the primary signal
is *observation change under perturbation*: for probe `x` and a step `h` derived from the observed scale (not a
global epsilon), compare the canonical observation at `x`, `x±h`, `x±2h`. A change in the **class** (kind, domain,
definedness, exactness flag) is the transition; only when the class is stable *and* the observation is scalar does a
symmetric quotient `(f(x+h) − f(x−h))/2h` become meaningful — and then only to **rank** candidates for a boundary
search, never as a value. Exact rational arithmetic means `h` can be exact, so probes carry no floating-point
epsilon. Higher-dimensional probes are directional: perturb along the axis with the largest one-step change, then
along the discovered gradient direction. This is a heuristic *ranking* over a deterministic *search*; the graph's
determinism never depends on it.

### D-8 — Equality-saturation readiness

**Recommendation: do not implement, and do not carry it as a conditional deliverable.** The brief's five gates,
scored against the repository:

| Gate | Status | Evidence |
|---|---|---|
| 1. Rewrite rules have strong falsification coverage | **Partially met** | A fixed battery over 9 rules with region tags exists; it is not discovery-driven, has no minimiser, and did not find the `Series.cs` nondeterminism (H-1) or the pole/branch-point cluster — audits did. |
| 2. MathIR validation is mature enough | **Not met** | `evalir`'s integer boundary is broken today (F-1), and the digits argument *is* a MathIR evaluation parameter. Fixing an IR's argument contract is a precondition to trusting extracted IR. |
| 3. Budget/node-growth controls are explicit | **Not met** | No e-node, iteration or node-growth budget exists; `Poll` coverage in the symbolic layer is 13 touchpoints across 8 files, 2 of them in an 80 KB solver. |
| 4. Extraction can be deterministic | **Not met (untested)** | A property of a system that does not exist. The *preconditions* are good: the expression DAG is hash-consed and canonical printing round-trips and re-interns (`PropertyTests.CanonicalText_RoundTripsAndReinterns`). |
| 5. Equivalence of extracted results is independently checkable | **Not met** | No equivalence checker exists; the falsifier would be the first one, which is why ADV-11..13 come first. |

**What to do instead, and what would unlock it:** land ADV-11..13, then re-score these five gates with measured
coverage (how many registry rules carry a discovered counterexample or a boundary record; samples to first
counterexample; boundary bias versus uniform). Equality saturation deserves a planning round when gate 1 moves from
"fixed battery" to "discovery + minimisation + regression per rule" **and** gates 2 and 3 are green. Recording that
explicitly is more useful than a conditional deliverable that arrives half-built.

---

## E. Work packages

Field order follows the brief. `Files` names the *owning* layer first. Every package's first task is a **control
tree** (`.worktrees/<id>-ctl`) carrying only its new tests, run to prove they fail for the right reason — the
repository's own rule, which round 23's records show caught real mis-attributions.

### ADV-01 — Exact-integer argument contract (foundation)

* **Purpose.** Introduce the one owner of "this argument must be an exact integer".
* **Motivation/evidence.** F-1; the §B2 table (9 sites, 3 rules, 2 flooring); Q-2's residual (`SymbolicsPlugin.AsLong`).
* **Dependencies.** None. Blocks ADV-02, ADV-03, ADV-06, ADV-07, ADV-10.
* **Files/projects.** NEW `Lovelace.Abstractions/ExactIntegerArgument.cs` (+ `Int64Range`, `Int32Range`,
  `ArgumentContractException`); additive read-only integrality predicates in `Lovelace.Natural`, `Lovelace.Integer`,
  `Lovelace.Real`; `Lovelace.Abstractions.Tests`.
* **Public contracts.** Additive only: a new public type and an exception deriving from `ArgumentException`. No
  `ErrorCategory` change, no change to `Real`'s representation or to `IsExact`.
* **Implementation approach.** `Require(payload, parameter, bounds, builtin?)`: Natural/Integer → `TryParse` (never
  `Parse`) plus range check; `Real` → accepted only when exactly integral **and** the call site's policy allows a
  Real-that-is-integral, else refused with one of two distinct messages (non-integral vs integral-but-wrong-kind);
  any other payload → type refusal. Never `ToString().Split('.')[0]`; never an unchecked `(int)` cast.
* **Tests.** Unit: the exactness predicate over an enumerated boundary matrix (integer Reals, `x.0`, `x.000`,
  periodic integers such as `2.(0)`, `1e3`, negative zero, Int64.MaxValue, Int64.MaxValue+1, and each numeric kind).
  Refusal-message tests assert the grammar (builtin, position, parameter, bound, received value).
* **Property tests.** For each `long` in a swept set with random digit suffixes: `Require(v) == v`, and
  `Require(v + 0.5)` throws — no input yields a value other than the exact one.
* **Differential/oracle tests.** None (no external oracle for argument typing).
* **Harness tests.** `code == "InvalidArgument" && category == "TypeMismatch"` on every refusing shape.
* **Acceptance criteria.** All nine §B2 sites are expressible through the one helper; `ArgumentContractException`
  crosses as `InvalidArgument`/`TypeMismatch` with **no** change to `Runner.Classify`; no existing public signature
  changes; Native AOT publish stays green.
* **Falsifiers.** A call site that still needs its own parsing; an exactly-integral `Real` misjudged by the predicate
  (the property test above is the specific falsifier).
* **Risk.** Low-medium; the highest risk is the integrality predicate over periodic/exponent forms — hence the
  enumerated matrix before any call site migrates.
* **Parallelization.** Solo (it is the contract everything else consumes).

### ADV-02 — Integer-argument call-site migration

* **Purpose.** Make every enumerated site use the contract, and close F-1.
* **Motivation/evidence.** F-1 (confirmed on the wire); Q-2; the O-1/O-4 refusal grammar.
* **Dependencies.** ADV-01.
* **Files/projects.** `Lovelace.Suite/Interpreter.cs` (`ToLong`, `ToInteger`, `ParseShape`, axis, digit counts);
  `Lovelace.Symbolics/SymbolicsPlugin.cs` (`AsLong`/`AsInt`/`DigitCount`, the `series` order call site);
  **`Lovelace.MathIR/Evaluator.cs` (`AsLong` and the two `evalir` digit call sites)**;
  `Lovelace.Dsp/DspPlugin.cs` (`ToLong`, `ToNonNegative`, `ToSeed`); the matching test projects.
* **Public contracts.** Wire fields unchanged. Some message texts move (documented per site); a `code`/`category`
  pair may move from `InvalidOperation`/`DomainError` to `InvalidArgument`/`TypeMismatch` **only** where no
  existing test pins it — audit R-2's requested unification, applied conservatively.
* **Implementation approach.** One commit per layer (Suite → Symbolics → MathIR → Dsp), each with its own
  control-failing test. `evalir` gains the same `[1, int.MaxValue]` bound and message grammar as `evalf`.
* **Tests.** Per site: a control tree proving the new test fails on unmodified HEAD, plus a positive control proving
  valid integral input still answers.
* **Property tests.** For `digits`-shaped arguments, for `n ∈ {0, 1, 2, INT_MAX-1, INT_MAX, INT_MAX+1, 10^30}` and
  each of `{n−ε, n, n+ε}` written as a decimal literal: the answer is either the exact-`n` behaviour or a named
  refusal — never a different `n`.
* **Differential/oracle tests.** `evalir(…, d)` and `evalf(…, d)` agree digit-for-digit on the same expression for
  `d ∈ {1, 2, 30, 100, 1000}`.
* **Harness tests.** The five wire shapes from F-1 asserted structurally.
* **Acceptance criteria.** `series(sin(x), x, 0, 2.5)` refuses and names the value; `evalir(…, 2.9)` refuses;
  `evalir(…, 0)` and `evalir(…, -5)` refuse; `evalir(…, 10^30)` answers `InvalidArgument`/`TypeMismatch` with a
  numbered message; positive controls (`order = 2`, `digits = 30`) are unchanged.
* **Falsifiers.** A site where the contract's refusal would break a *legitimate* program (a builtin that
  intentionally truncates a real). The audit must name such a site or the migration is complete.
* **Risk.** Medium: message-contract churn across four projects. Mitigated by per-layer commits and by the pinned
  tests being the arbiter.
* **Parallelization.** Must follow ADV-01. The four layers can be four worktrees, but they must **merge** in the
  order Suite → Symbolics → MathIR → Dsp, because MathIR's evaluator is registered by the Symbolics plugin host.

### ADV-03 — Integer-argument property and boundary matrix

* **Purpose.** Make the contract's boundary a permanent generated gate rather than a curated list.
* **Motivation/evidence.** The brief's `n / n±ε / 0 / negative / very large / overflow / exact rational / non-integral
  rational` matrix, and the repository's rule that a control-failing test is the price of a fix.
* **Dependencies.** ADV-01, ADV-02.
* **Files/projects.** NEW `Lovelace.Run.Tests/IntegerArgumentMatrixTests.cs` (wire-level, spawning the runner) and
  in-process variants in `Lovelace.Suite.Tests` / `Lovelace.Symbolics.Tests`; one shared case generator.
* **Public contracts.** None.
* **Implementation approach.** One case table drives every integer-semantic builtin: `{zeros, ones, eye, reshape,
  reductions with axis, setprecision, pi, e, evalf, evalir, evalir_batch, series, movingavg, delay, fft n, vector
  index, range bound}` × `{exact integer, exact integral Real, non-integral Real, 0, negative, ±1, Int64.Max,
  Int64.Max+1, 10^30, periodic integer}`.
* **Tests.** The matrix itself; each cell asserts `(outcomeClass, code, category, messageGrammar)`.
* **Property tests.** Random decimal literals whose integrality is decided by an independent Python/SymPy oracle
  (the differential-oracle job already provisions it).
* **Differential/oracle tests.** SymPy decides integrality for generated literals; the runner must agree.
* **Harness tests.** Structured-field assertions only (no message matching), except where the grammar *is* the
  contract.
* **Acceptance criteria.** The matrix is exhaustive over the enumerated builtins and green; no accepting cell alters
  its argument (asserted by making the accepted value observable, e.g. comparing `evalir(…, d)` output length
  across `d`).
* **Falsifiers.** An integer-semantic builtin missing from the table — caught by a census test over
  `InterpreterBuiltinDescriptors` parameter names (no reflection, AOT-safe).
* **Risk.** Low but broad; it will surface sites the §B2 grep missed, which is the point.
* **Parallelization.** Independent of ADV-04 and later.

### ADV-04 — Certified transcendental precision at the `Real` boundary

* **Purpose.** Close P-2 **and** F-2: make `Real.Sin/Cos` deliver the digits they claim, and stop publishing digits
  they did not compute.
* **Motivation/evidence.** F-2's measured table; `p2-preserved/RealTrigAccuracyTests.cs`;
  `RealTrigPiResolutionPropertyTests.cs`; `Real.cs:1756-1848`, `:2041-2100`, `:1854-1871`.
* **Dependencies.** None technically; **sequence it after ADV-01/02** only to avoid concurrent edits to
  `SymbolicsPlugin.cs` (this package touches `Lovelace.Real`, which ADV-02 does not).
* **Files/projects.** `Lovelace.Real/Real.cs` (`Sin`, `Cos`, the Taylor helpers, the reduction and precision
  resolution), `Lovelace.Real.Tests`.
* **Public contracts.** `Real.Sin/Cos(Real)` and `(Real, long)` keep their signatures. The **behaviour** of the
  returned value changes (fewer digits, and correct ones). Additive: a documented accuracy query. `IsExact`
  semantics unchanged.
* **Implementation approach.** (a) Make the working precision a function of the argument's magnitude and the
  reduction cost rather than a constant guard. (b) Cap the published value at `max(digits, -exponent) + guard`
  (D-2). (c) Run the same audit over `Exp`, `Log`, `Sqrt`, `Tan` and the complex kernels, fixing only what the
  measurement falsifies.
* **Tests.** The preserved `RealTrigAccuracyTests.cs` re-enters the tree **as a whole** (it is already written
  against the correct contract), with the one expectation that contradicts the surviving property tests
  (`Sin_AtATenthOfTheAmbientPrecision_StopsAtTen`) reconciled explicitly rather than deleted.
* **Property tests.** The existing `RealTrigPiResolutionPropertyTests` (9 + 6 + 9 cases) plus a new battery: for
  `i ∈ 0..40` and several `k`, `Sin(10^i + k, 30)`, `Cos`, and near-`π/2` residues — asserting **correctness of
  the requested digits** against mpmath, not a digit count.
* **Differential/oracle tests.** mpmath at ≥ 2000 dps as ground truth; `ComplexMath.Sin` as a second implementation
  to cross-check the *value*; the CLI `evalf` path as a third.
* **Harness tests.** `evalf(sin(10), 30)` unchanged; `setprecision` + `print(sin(10))` unchanged; library-and-CLI
  agreement asserted on a shared fixture battery.
* **Acceptance criteria.** For every case in ADV-05's battery, the published value's first `digits` fractional
  digits match mpmath and no digit beyond the certified cap is published. `Sin(10, 30)` has 30 correct digits.
  `Cos(π/2 − 10^-100, 30)` still returns its ~10^-100 value, positive, with no invented tail. All 2 495 existing
  `Lovelace.Real.Tests` and 61 `Lovelace.Dsp.Tests` stay green.
* **Falsifiers.** Any case where the cap deletes a value (the `cos(π/2 − 10^-100)` family is the named falsifier);
  any case where raising the guard is still insufficient for the requested digits; a runtime regression beyond a
  stated factor on the benchmark's transcendental rows.
* **Risk.** **High** — this package has already been falsified twice. It must land behind (1) a measured battery
  *before* the change, (2) the control tree, and (3) an explicit statement that a digit-count-only fix is
  insufficient (F-2).
* **Parallelization.** Solo. Do not run concurrently with any other edit to `Lovelace.Real`.

### ADV-05 — Precision oracle battery and transcendental differential harness

* **Purpose.** Give ADV-04, and every future numeric change, an external oracle that measures *correct digits*.
* **Motivation/evidence.** F-2 was discoverable only with a hand-built probe; audit P built one outside the repo
  (`C:\Users\ricar\dev\.lovelace-audit-P`) and it is not part of the product's evidence trail.
* **Dependencies.** None — start it first: it measures the *unfixed* state, which is its baseline.
* **Files/projects.** NEW `Lovelace.Real.Tests/PrecisionOracleTests.cs` + `tools/precision-oracle/` (Python,
  mpmath), following the existing `SympyOracle`/`OracleCorpus` pattern and `LOVELACE_REQUIRE_SYMPY` semantics
  (an unavailable oracle fails CI rather than skipping).
* **Public contracts.** None.
* **Implementation approach.** The C# side emits `(case, fractional-digit-string)`; Python compares against mpmath
  and returns the correct-prefix length; the test asserts the **contract** (requested digits correct) rather than a
  golden string, so the battery survives legitimate reformatting.
* **Tests.** The battery, plus a self-test that the oracle *fails* on a deliberately truncated value.
* **Property tests.** Generated magnitudes `10^i`, near-pole residues, periodic arguments, negative arguments.
* **Differential/oracle tests.** This package *is* the oracle.
* **Harness tests.** The CLI path is included as a third implementation on the same cases.
* **Acceptance criteria.** The battery reproduces ADV-04's before/after numbers; CI runs a cheap subset per push and
  the full battery in the SymPy-oracle job.
* **Falsifiers.** An oracle that agrees with a known-wrong value (the self-test).
* **Risk.** Low. **Parallelization.** Fully independent — start first.

### ADV-06 — `Real.Pow` exponent semantics and the raw-exception class

* **Purpose.** Decide and implement the non-integral and negative exponent contract, and close the raw-message leaks.
* **Motivation/evidence.** F-3; P-4 recorded open; `Real.cs:1212, 1225, 1234`.
* **Dependencies.** ADV-01 (the refusal grammar).
* **Files/projects.** `Lovelace.Real/Real.cs`, `Lovelace.Complex/ComplexMath.cs` (the working implementation),
  `Lovelace.Suite/Interpreter.cs` (exponent dispatch), `Lovelace.Symbolics` (exact power rules).
* **Public contracts.** `Real.Pow` must stop throwing a framework `NotImplementedException`. Chosen contract: a
  **typed** refusal for exponents the kernel cannot represent exactly, plus a **principled rational subset** —
  `x^(p/q)` for exact rational `p/q` with `q ∈ {1, 2}` (square roots are already exact machinery), and negative
  integer exponents of non-zero exact rationals, which are already computable (`(1/2)^-1 = 2`). Everything else is
  refused, never approximated.
* **Implementation approach.** Route representable cases through the existing exact paths; classify the rest through
  `ArgumentContractException` or a typed unsupported-operation type; `0 ^ -1` becomes a named domain refusal rather
  than `(Parameter 'base')`.
* **Tests.** `2^-1`, `2^-3`, `(1/2)^-1`, `0^-1`, `8^(1/3)`, `4^0.5`, `(-8)^(1/3)`, `2^(1/2)` vs `sqrt(2)`.
* **Property tests.** For exact rational `b ≠ 0` and `n ∈ ℤ`: `b^n * b^-n == 1` exactly.
* **Differential/oracle tests.** SymPy for the rational subset.
* **Harness tests.** No `InternalError` for any exponent shape; `(Parameter '…')` never appears in a message.
* **Acceptance criteria.** No public API throws a raw `NotImplementedException`/`ArgumentOutOfRangeException` for a
  mathematical request; the rational subset is exact (asserted via `IsExact`); everything else is a typed refusal
  naming the operation.
* **Falsifiers.** A case where the rational subset silently approximates.
* **Risk.** Medium — it touches exact-power semantics used by expand/factor. **Parallelization.** Independent of
  ADV-04 (different files), but must not overlap ADV-02's `SymbolicsPlugin` edits.

### ADV-07 — Structured signature and shape diagnostics

* **Purpose.** Close WS-2: the arity/shape facts the exception types already carry must reach the wire.
* **Motivation/evidence.** B9; the benchmark's `arity-dot` task costs 2 evaluations in every mode.
* **Dependencies.** ADV-01 (shares the refusal grammar).
* **Files/projects.** `Lovelace.Run/RunProtocol.cs` (`RunErrorDto`), `Lovelace.Run/Runner.cs` (`WriteError`,
  `Classify`), `Lovelace.Suite/Functions.cs` only if a field is missing.
* **Public contracts.** **Additive**: new optional fields on the error object — `function`, `parameters`,
  `expectedArity`, `actualArity`, `variadic`; and for shape failures `parameter`, `position`, `expectedShape`,
  `actualKind`, `actualRank`. `protocolVersion` stays 1; fields are omitted (never `null`) on failures that do not
  have them, matching the `suggestions`/`hints` precedent.
* **Implementation approach.** `WriteError` already receives the exception; add a projection from
  `BuiltinArityException`/`BuiltinShapeException`/`ArgumentContractException` to those fields, exactly as
  `UnknownFunctionException` is projected today.
* **Tests.** Golden-fixture tests (the repository pins envelopes in `Lovelace.Run.Tests/fixtures`).
* **Property tests.** For every registered builtin's declared arity, calls with `min-1`, `min` and `max+1`
  arguments produce the right structured counts — a generated census over all 123 builtins.
* **Differential/oracle tests.** None.
* **Harness tests.** `arity-dot` and `shape-matmul` solved in ≤ 2 evaluations in the re-run.
* **Acceptance criteria.** Every `InvalidArgument` caused by arity or shape carries the structured fields, and the
  counts agree with the registry's own `minArity`/`parameterCount`/`variadic` for all 123 builtins.
* **Falsifiers.** A builtin whose declared metadata disagrees with what its body accepts — the census will find it.
* **Risk.** Low. **Parallelization.** Independent, but shares `RunProtocol.cs` with ADV-08 → merge order.

### ADV-08 — Registry query surface

* **Purpose.** Close WS-3: make the capability surface cheap, deterministic and bounded.
* **Motivation/evidence.** B7/B8: 185 queries over 20 runs in L3; 15 677 bytes per registry read today; `plugin`
  missing from the payload the benchmark's own projection expects.
* **Dependencies.** ADV-07 (shares `RunProtocol.cs`).
* **Files/projects.** `Lovelace.Run/Runner.cs` (flags), `RunProtocol.cs` (`FunctionDto`), `Lovelace.Suite`
  (expose `Category`/`Summary`/`Examples`/`ReturnKind`/`Related` and plugin provenance through
  `InterpreterBuiltinDescriptors`), `harness/lovelace.host.js`, `harness/README.md`, the protocol document.
* **Public contracts.** `--functions [pattern]` (no script; names + arity; `truncated` + `total` + `returned`)
  and `--function <name>` (one full entry). All six existing `FunctionDto` fields keep their names and meanings
  (recorded trajectories depend on them); new fields are additive.
* **Implementation approach.** A pure registry read — no engine evaluation, no revision bump. Deterministic
  alphabetical order (already true; assert it). Bounded default page with an explicit `truncated` flag.
* **Tests.** Flag parsing, case-insensitive substring filter, unknown name, empty result, truncation boundary at the
  page size.
* **Property tests.** For every registered name, `--function <name>` returns exactly one entry whose `name`
  matches and whose `parameters` equal the registry's.
* **Differential/oracle tests.** For all 123 names, `--function <name>` equals the corresponding entry of the full
  `functions` array, field for field, on the fields both carry.
* **Harness tests.** Measured bytes: the L3 lookup for `det` must drop from ~15.7 KB to ≤ 2 KB; the benchmark's
  `function_projection` must be satisfiable by the runner's payload without a translation layer.
* **Acceptance criteria.** A registry query requires no evaluation (asserted: it answers even when the supplied
  script would fail); deterministic across runs; bounded with an honest `truncated`.
* **Falsifiers.** A query that changes engine state (`revision`) — that would mean it evaluated.
* **Risk.** Low. **Parallelization.** After ADV-07.

### ADV-09 — Failure-state projection: committed state and artifacts

* **Purpose.** Close WS-4 and N-5: a failure says how far it got and what it produced.
* **Motivation/evidence.** B10/B11; `partialVariables` at 0 % on non-cancel failures; the plot-on-disk case.
* **Dependencies.** ADV-08 (same DTO/writer path).
* **Files/projects.** `Lovelace.Run/Runner.cs`, `RunProtocol.cs`, `Lovelace.Suite/SuiteEngine.cs` (plot capture
  inventory), `Lovelace.Studio`'s two hosts (projection parity), the protocol document.
* **Public contracts.** Additive: `artifacts: [{kind, path, title, bytes}]` on success and failure;
  `partialVariables` populated on every recoverable failure (a superset of today); and a documented statement that
  `partialOutput` is authoritative on cancellation while `output` is always the committed prefix.
* **Implementation approach.** The engine already owns plot captures; the Runner projects them into `artifacts` and
  stops special-casing `plot` on the error path. `partialVariables` is filled from `engine.CaptureState()` on every
  recoverable failure — the cancellation path already proves the mechanism.
* **Tests.** Multi-statement fixtures: success; failure after 2 commits; failure at statement 1; cancellation; parse
  error (nothing committed); plot-then-failure; plot-then-success; two plots.
* **Property tests.** For a generated program with `k` committed statements then a failure, `partialVariables`
  contains exactly those `k` names, for `k ∈ 0..5`.
* **Differential/oracle tests.** CLI vs `SuiteEngine` vs Studio host projections on the same fixtures — audit P's
  42/42 `ProjectValue`-`Runner` agreement extended to the failure path.
* **Harness tests.** The L2 projection carries non-zero `partialVariables` (the benchmark's own 0 % metric moves).
* **Acceptance criteria.** No failure envelope can be silent about a file it wrote; `checks/measure_surface.py`
  reports non-zero partial-variable coverage.
* **Falsifiers.** A shape where the engine committed something the envelope cannot name (found by the `k` sweep).
* **Risk.** Medium: four surfaces, one of which (Studio) has its own `SplitLines`. **Parallelization.** After
  ADV-07/ADV-08 merge.

### ADV-10 — Cooperative budget: `solve` checkpoints and preflight guards

* **Purpose.** Make the budget claim true where it is currently false, and refuse the pathological before working.
* **Motivation/evidence.** B12 (7 028 ms at a 300 ms budget, `stopped:false`); Q-2 (`series` order 10^11);
  Q-3 (`setprecision` unbounded); 2 polls in an 80 KB solver.
* **Dependencies.** ADV-01, ADV-02.
* **Files/projects.** NEW `Lovelace.Abstractions/WorkBudget.cs`; `Lovelace.Symbolics/Solvers/Solve.cs`,
  `Algebra/Groebner.cs`, `Algebra/Expand.cs`, `Calculus/Series.cs`, the simplifier; `Lovelace.Suite` (preflight
  guards for `series` order and `setprecision`); the protocol document.
* **Public contracts.** No new `ErrorCategory`. A budget stop remains `BudgetExceeded`/`BudgetExceeded` (already
  frozen); a preflight refusal is `InvalidArgument`/`TypeMismatch` naming the bound. Documentation gains an explicit
  **non-interruptible list**.
* **Implementation approach.** `WorkBudget` wraps the ambient token plus a work counter; `Poll(units)` is the one
  call. Instrument `solve`'s candidate enumeration and polynomial loops first, measure, then extend. Preflight
  guards are declared constants with their measured justification recorded next to them.
* **Tests.** Timing-tagged tests (the repository's `Category=Timing` pattern) asserting an overrun bound: the same
  `solve(x^2-10^16, x)` under a 300 ms budget must stop within a stated factor — or the case must be declared
  non-interruptible and the test asserts that the ledger says so.
* **Property tests.** For a generated family of solver inputs: when `stopped:true`, `excessMs` is bounded by a
  stated constant; when `stopped:false`, the ledger must say so (already true — assert it stays true).
* **Differential/oracle tests.** The same answer with and without a budget large enough not to bind: a budget must
  not change results.
* **Harness tests.** The `cancellation` structured fields on both outcomes.
* **Acceptance criteria.** The measured overrun for the named cases drops from 23× to a stated tested bound, or the
  case is documented non-interruptible; `series(…, 10^30)` refuses **before** doing work; `setprecision` has a
  measured declared cap.
* **Falsifiers.** A checkpoint that changes the answer; a cap that refuses a request the engine serves in reasonable
  time (measured, not guessed).
* **Risk.** **Medium-high** — the solver's control flow is large and the budget must not perturb results.
  **Parallelization.** `Solve.cs` is one hotspot: one worktree. The preflight guards are separable and can run in
  parallel with the solver instrumentation.

### ADV-11 — Falsifier substrate: regions, boundary bias, observation classes

* **Purpose.** Turn the existing fixed probe battery into a discovery loop that exploits mathematical structure.
* **Motivation/evidence.** B17: `FalsificationGateTests.cs` is a curated list; the round-23 audits found the defects
  a discovery loop would have found (the `Series` nondeterminism, the pole/branch cluster).
* **Dependencies.** None for the substrate; ADV-02 makes "undefined" cleanly distinguishable from "refused".
* **Files/projects.** NEW `Lovelace.Symbolics.Tests/Falsification/RegionModel.cs`, `BoundaryBias.cs`,
  `ObservationClass.cs`; the existing `FalsificationRegions` becomes a consumer rather than the owner.
* **Public contracts.** Test-side only; no product contract changes. The region record is serialisable so a report can
  name the region a counterexample came from.
* **Implementation approach.** Reuse `RealProbe`/`ComplexProbe` as the probe type; add (a) a deterministic
  boundary-bias generator that bisects between the two observed points straddling a value/class change using **exact
  rationals**, (b) the five-class classifier, and (c) a ranked candidate list by symmetric quotient (D-7) used to
  order, never to decide.
* **Tests.** Seed determinism (same seed → byte-identical report); the classifier over hand-built cases; bisection
  converging to a known boundary (`a - b` widening Natural→Integer at `b > a`) as a control.
* **Property tests.** For every shipped rule, the fixed legacy probe set is a **subset** of what the new generator
  produces — the repository's own superset rule applied to itself.
* **Differential/oracle tests.** Boundary-biased versus uniform search on a planted defect: samples-to-first-
  counterexample must be lower for boundary bias.
* **Harness tests.** A report projection for the DSH harness (optional this cycle).
* **Acceptance criteria.** Boundary bias discovers a planted counterexample in fewer samples than uniform sampling on
  a fixed budget, with the measurement recorded; the report is deterministic under a fixed seed.
* **Falsifiers.** Boundary bias **not** beating uniform — the brief's own test, and it is a hypothesis, not a fact.
  If falsified, the substrate keeps uniform sampling and the boundary work is re-scoped to *classification* only.
* **Risk.** Medium: heuristic quality. Bounded by keeping the deterministic part (generation, classification,
  reporting) independent of the heuristic part (ranking).
* **Parallelization.** Independent of every product package.

### ADV-12 — Deterministic counterexample minimiser

* **Purpose.** Turn a large failure into a readable one.
* **Motivation/evidence.** The brief's `sqrt(x^2) -> x` example; nothing in the repository shrinks anything.
* **Dependencies.** ADV-11.
* **Files/projects.** NEW `Lovelace.Symbolics.Tests/Falsification/Minimizer.cs` + tests.
* **Public contracts.** Test-side only.
* **Implementation approach.** A stated total shrink order (variables → node count → constant magnitude → degree →
  precision → boundary distance), greedy single-step, re-checking the *same* predicate, terminating at a fixpoint.
  Deterministic by construction: no randomness, total order, one deterministic candidate per step.
* **Tests.** Known cases shrink to their expected minimal form (a `sqrt(x^2)`-shaped violation without an `x >= 0`
  assumption shrinks toward a negative rational); idempotence (minimise twice = minimise once); determinism across
  runs.
* **Property tests.** The minimiser never returns a candidate that does not still falsify — the one invariant that
  makes it safe — and never returns something larger under the stated order.
* **Differential/oracle tests.** For a case with a known exact answer, the minimised input is semantically equivalent
  (same observation class).
* **Harness tests.** Report size before/after (a metric, not a gate).
* **Acceptance criteria.** Every discovered counterexample is minimised to a fixpoint within a stated step budget;
  the output is byte-stable across runs and processes.
* **Falsifiers.** A shrink step that changes the *reason* for the failure (the property test catches it).
* **Risk.** Low. **Parallelization.** Independent once ADV-11's probe/observation types exist.

### ADV-13 — Regression exporter

* **Purpose.** Monte Carlo discovers; the repository permanently pins.
* **Motivation/evidence.** Every cycle-6 landing was justified by a control-failing test; the falsifier currently
  ends in a report, not a test.
* **Dependencies.** ADV-11, ADV-12.
* **Files/projects.** NEW `Lovelace.Symbolics.Tests/Falsification/CounterexampleRecord.cs`,
  `FalsificationCorpus.cs` (a checked-in generated corpus), `tools/falsify/` (a CLI that emits the record).
* **Public contracts.** Test-side only; the record shape is versioned so a generator change does not invalidate the
  corpus.
* **Implementation approach.** Emit a record with exactly the brief's fields (seed, generator version, rule id,
  input, assumptions, expected relation, observed disagreement, precision, boundary context, runtime revision) and
  render it into a deterministic xUnit theory row. The generated test must **not** re-run the stochastic search.
* **Tests.** Round-trip: record → rendered test → the test fails on the unfixed kernel and passes on the fixed one —
  the control-tree discipline, automated.
* **Property tests.** Rendering is injective on the record fields (two different records never render identically).
* **Differential/oracle tests.** A corpus regenerated from the same seed is byte-identical.
* **Harness tests.** The falsifier report carries the record, so an agent can consume it.
* **Acceptance criteria.** At least one **real** counterexample from ADV-11 is minimised and exported as a permanent
  test in this cycle; re-running the falsifier on the fixed kernel reports zero disagreements.
* **Falsifiers.** An exported test that passes on the unfixed kernel — the record lost the discriminating detail.
* **Risk.** Low. **Parallelization.** After ADV-12.

### ADV-14 — Harnessability default surface and benchmark re-run

* **Purpose.** Make the *production* DSH surface the rich one, without contaminating the frozen L0-L3 projections.
* **Motivation/evidence.** WS-6; `harness/lovelace.host.js:95` already sends `--omit-functions` and has no
  capability-discovery path; the benchmark's L3 advantage has collapsed to 0.070, exactly the predicted consequence
  of moving discovery into the failure itself.
* **Dependencies.** ADV-07, ADV-08, ADV-09.
* **Files/projects.** `harness/lovelace.host.js`, `harness/README.md`, `harnessability-bench/` (re-run artefacts
  only — `bench.py`, `tasks.json`, `run.schema.json` and the L0-L3 projections **do not change**).
* **Public contracts.** The *benchmark's* four modes stay frozen; the *harness* tool's default output is a new,
  documented surface. Both facts are stated in the same document so the next cycle cannot conflate them.
* **Implementation approach.** The DSH tool defaults to the rich envelope (structured result, code, category,
  recoverability, position, committed state, artifacts) with capability discovery on demand; the runner flags that
  build the frozen projections remain available and unchanged.
* **Tests.** Harness-level: the tool's default payload contains the structured fields; the frozen projections still
  produce byte-identical shapes on the recorded fixtures.
* **Property tests.** None.
* **Differential/oracle tests.** Re-run the 80-cell matrix and compare against the frozen baseline and gate run A.
* **Harness tests.** The matrix itself, with **≥ 3 replicates per cell** — gate run A's H6 falsification showed a
  single L0 replicate carries ≈ ±0.075, larger than several effects being claimed. No claim from a 1-replicate arm.
* **Acceptance criteria.** The production surface is documented and tested; the re-run is executed and reported with
  its replicate count; the L0-L3 projections are unchanged (asserted by diffing the projection sources).
* **Falsifiers.** A re-run showing no movement, or movement only in the control arm.
* **Risk.** Medium: cost (~40 min + tokens per arm set) and contamination. Mitigation: projections frozen by test;
  the harness surface is a separate code path.
* **Parallelization.** Last.

### Out of scope for this cycle (recorded, not planned)

Equality saturation (D-8); special-function breadth; a Risch integrator; a parser rewrite; alias tables as a
substitute for discovery; GPU/distributed execution; a generalized `ExecutionBudget`; extending
`Lovelace.Knowledge`'s sample type. Each is deferred for a stated reason, not by omission.

---

## F. Dependency DAG

```text
ADV-05 precision oracle battery ──────────────────────────────┐   (independent; measures the baseline)
                                                              │
ADV-01 exact-integer contract                                 │
  ├──> ADV-02 call-site migration  ──> F-1 closed             │
  │       ├──> ADV-03 boundary/property matrix                │
  │       ├──> ADV-06 Real.Pow exponent semantics             │
  │       └──> ADV-10 budget checkpoints + preflight guards   │
  └──> ADV-07 structured signature/shape diagnostics          │
          ├──> ADV-08 registry query surface                  │
          │       └──> ADV-09 committed state + artifacts     │
          │               └──> ADV-14 harness default + bench re-run
          └──> (ADV-09 also needs ADV-08's DTO freeze)

ADV-04 certified transcendental precision   <── verified by ── ADV-05

ADV-11 falsifier substrate ──> ADV-12 minimiser ──> ADV-13 regression exporter
   (independent of the product packages; ADV-02 only improves "undefined vs refused")

ADV-04 ──> (future) equality-saturation gate scoring   [NOT a deliverable this cycle]
```

**Critical path:** ADV-01 → ADV-02 → ADV-07 → ADV-08 → ADV-09 → ADV-14.
**Second path:** ADV-05 → ADV-04.
**Third path:** ADV-11 → ADV-12 → ADV-13.

Merge order is the topological order above. ADV-04 and ADV-11..13 live on independent branches and can land at any
point after their own dependencies.

---

## G. Parallel implementation strategy (DSH worktrees)

**Constitutional files — one worktree each, never two at once:**

| File / area | Owner package(s) | Rule |
|---|---|---|
| `Lovelace.Real/Real.cs` | ADV-04, ADV-06 | serialize; if both are scheduled, ADV-04 first (higher risk, needs the battery) |
| `Lovelace.Suite/Interpreter.cs` | ADV-02, ADV-10 (preflight) | serialize |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | ADV-02, ADV-10 (preflight) | serialize |
| `Lovelace.Run/RunProtocol.cs`, `Lovelace.Run/Runner.cs` | ADV-07, ADV-08, ADV-09 | strict order 07 → 08 → 09 |
| `Lovelace.Abstractions/*` | ADV-01 (+ ADV-10's `WorkBudget`) | ADV-01 solo, then frozen; ADV-10's addition is additive and may follow |
| `Lovelace.MathIR/Evaluator.cs` | ADV-02 | solo |

**Safe concurrency (≤ 5 worktrees at once):**

| Wave | Worktrees | Why safe |
|---|---|---|
| 0 | `adv-05-oracle`, `adv-11-falsifier`, `adv-01-contract` (+ `adv-01-ctl`) | three disjoint trees; the control tree is separate again |
| 1 | `adv-02-suite`, `adv-02-symbolics`, `adv-02-mathir`, `adv-02-dsp`, `adv-12-minimiser` | four different projects with no shared file; serialized **merges** |
| 2 | `adv-04-real`, `adv-06-pow`, `adv-03-matrix`, `adv-13-export` | `Real.cs` is ADV-04's alone; ADV-06 waits if ADV-04 is in flight |
| 3 | `adv-07-diag` → `adv-08-registry` → `adv-09-state` | same three files, strict order |
| 4 | `adv-10-budget` | solver hotspot, solo |

**Merge order.** ADV-01 → ADV-02 (Suite, Symbolics, MathIR, Dsp) → ADV-03 → (ADV-05, ADV-04) → ADV-06 →
ADV-07 → ADV-08 → ADV-09 → ADV-10 → ADV-11 → ADV-12 → ADV-13 → ADV-14.

**Practice this repository already requires and that must be repeated:** every fixer worktree gets a **control
tree** (same tests, unmodified product code), and the fix is not accepted until the control fails for the right
reason and the fixed tree passes. Round 23's records show this caught real mis-attributions.

---

## H. Validation gates

### Gate 0 — baseline (re-run before any change)

* `dotnet build LovelaceSharp.slnx -c Release` → 0 warnings / 0 errors (measured: clean, 9.8 s).
* Full sweep → **5 685 passed / 0 failed / 8 skipped** (measured at HEAD; record the number for the final tree).
* `dotnet publish Lovelace.Run -c Release -p:PublishAot=true` → **two** artefacts, each fingerprinted by
  (bytes, write time, git revision, seconds newer than the newest non-generated source):
  `out/aot/Lovelace.Run.exe` **and** `Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe` (F-5).
* Known reproductions **pinned as failing tests** (they must fail at Gate 0 and pass at the gate that fixes them):
  `series(sin(x),x,0,2.5)`; `evalir(…,2.5)`; `evalir(…,0)`; `evalir(…,10^30)`; `Real.Sin(10,30)` digit
  correctness; `Real.Sin(1,30)` digit count; plot-then-failure artifact; `solve` overrun; undefined-variable
  classification.
* `harnessability-bench` baseline confirmed reproducible (`checks/verify_invariants.py` 6/6,
  `bench.py validate` golden and probe counts) **with the binary fingerprint recorded**.

### Gate 1 — correctness

* The integer-coercion class is eliminated: every cell of ADV-03's matrix behaves as specified, and the §B2
  enumeration has no flooring site left.
* `evalir` and `evalf` agree digit-for-digit at 1 / 2 / 30 / 100 / 1000 digits.
* ADV-05's battery is green: every transcendental case's requested digits are correct against mpmath, and no digit
  beyond the certified cap is published.
* Library / `ComplexMath` / CLI agreement within the requested digits.
* 2 495 `Lovelace.Real.Tests` + 61 `Lovelace.Dsp.Tests` + 367 `Lovelace.Run.Tests` green.
* AOT publish from the final commit on both paths, smoke green.

### Gate 2 — harnessability

* `--functions` / `--function` exist, are evaluation-free, deterministic, bounded, and carry the full descriptor;
  a single-name lookup measures ≤ 2 KB.
* Every arity/shape failure carries the structured fields, and the counts agree with the registry for all 123
  builtins (generated census).
* `partialVariables` is non-zero in `checks/measure_surface.py`; `artifacts` is present for a plot-then-failure run.
* An undefined variable is structurally distinguishable from a domain refusal and receives the same kinds of
  resolution hints a bad function name gets (F-4).
* The harnessability benchmark is re-run with **≥ 3 replicates** and compared against the frozen baseline and gate
  run A; the production default surface is documented as **separate** from the frozen L0-L3 modes.

### Gate 3 — behavioural verification

* Known-bad transformations are detected (a planted defect per rule class: a wrong rule, a rule with a missing
  precondition, a rule that changes the domain).
* Fixed seed → byte-identical report, across runs and across processes.
* **Boundary-biased search beats uniform search** on samples-to-first-counterexample, with the measurement recorded;
  if it does not, the hypothesis is reported falsified and the substrate keeps uniform sampling.
* A real counterexample is minimised to a fixpoint and exported as a permanent regression that fails on the pre-fix
  kernel and passes on the post-fix one.

### Gate 4 — compiler/optimizer readiness

Scored, not implemented: re-evaluate D-8's five gates with measured falsification coverage. Expected outcome this
cycle: gates 2 and 3 become green once ADV-02 and ADV-10 land; gates 1, 4 and 5 stay unmet, because coverage is
still rule-by-rule and no e-graph machinery exists. **Equality saturation is not a deliverable of this cycle.**

### Gate 5 — final adversarial audit

Fresh personas that did **not** implement the fixes, against the binary published from the final commit, with a
mandatory "could not check" list. The cycle's own rule from round 23 holds: *a gate is not passed because the
implementing agent's tests pass.* Minimum personas: (a) attack the new argument contract at its boundaries;
(b) attack the transcendental fix with an independent oracle at magnitudes the implementer did not sweep;
(c) drive the registry/diagnostics surface as a consumer that never reads the source.

---

## I. Benchmark plan (measure before implementing)

| Metric | Definition | Instrument | Baseline today |
|---|---|---|---|
| correct digits / requested digits | matching prefix vs mpmath at ≥ 2 000 dps | ADV-05 | `Sin(1,30)`: 606 published, 40 right; `Sin(10,30)`: **29 of 30 requested** |
| transcendental runtime vs precision | wall time per (kernel, digits) | ADV-05 CLI side | not measured |
| cancellation overrun | `wall − budget` and `excessMs` per operation | ADV-10 timing tests | `solve(10^16)`: 7 028 ms on a 300 ms budget |
| structured diagnostic completeness | fraction of failures carrying code + category + position + signature | ADV-07/09 | arity/shape fields 0 %; `partialVariables` 0 % |
| registry-query bytes | bytes for one name lookup, and for a filtered list | ADV-08 | 15 677 B for the whole registry |
| agent evaluations to solution | mean evaluations per task | benchmark | L0 6.60 · L1 5.55 · L2 5.15 · L3 4.55 → gate A 5.80 · 4.55 · 3.95 · 4.20 |
| agent false convergence | runs claiming completion that fail the hidden probes | benchmark | 4 (baseline) → 2 (gate A) |
| H_B, H_10 | harnessability curves | benchmark | L0 .675→.750 · L1 .715→.860 · L2 .805→.880 · L3 .980→.950 |
| falsifier discovery rate | counterexamples per 1 000 samples per rule | ADV-11 | no discovery loop exists (fixed battery only) |
| samples to first counterexample | seeded | ADV-11 | not measured |
| boundary-search advantage | samples-to-first, boundary-biased ÷ uniform | ADV-11 | not measured (the hypothesis under test) |
| counterexample size before/after shrinking | node count, variables, constant magnitude | ADV-12 | not applicable |
| falsifier determinism | report hash across runs and processes | ADV-11/13 | not applicable |
| MathIR equivalence rate | `evalir` vs `evalf` agreement across a case corpus | ADV-02 | 4/4 spot checks today; the battery is new |

Every metric names the artefact that produced it (binary path + bytes + revision), because F-5 shows how easily a
stale binary invalidates a measurement.

---

## J. Scope recommendation

**Must land — the cycle is not complete without these:**

| Id | Why |
|---|---|
| ADV-01 exact-integer contract | the brief's central rule; blocks the whole correctness axis |
| ADV-02 call-site migration | closes F-1 and Q-2's residual on the wire |
| ADV-03 boundary/property matrix | makes the class permanently closed rather than reported closed |
| ADV-05 precision oracle battery | without it ADV-04 cannot be verified, and F-2 stayed invisible for want of it |
| ADV-04 certified transcendental precision | the only open P1 at a public library boundary |
| ADV-07 structured diagnostics | cheapest measurable harnessability win; the data already exists and is dropped |
| ADV-08 registry query | removes the last evaluation-priced discovery path |
| ADV-09 committed state + artifacts | closes the 0 %-coverage gap the benchmark itself publishes |

**Should land:**

| Id | Why |
|---|---|
| ADV-06 `Real.Pow` semantics | closes a raw-exception leak and a wrong refusal on an exactly-representable case |
| ADV-10 `solve` checkpoints + preflight guards | makes the budget claim true where it is currently false |
| ADV-11 falsifier substrate | converts an existing curated battery into a discovery loop |
| ADV-12 minimiser | makes a discovered failure readable |
| ADV-13 regression exporter | makes a discovered failure permanent |
| ADV-14 harness default + benchmark re-run | the measurement that tells the next cycle whether any of it helped |

**Experimental:** boundary-bias ranking (D-7) beyond classification; `Real.Pow`'s rational subset beyond
`q ∈ {1, 2}`; a stateful DSH session (the harness is stateless today by design).

**Deferred:** equality saturation (D-8 — gates unmet, scored rather than half-built); a generalized
`ExecutionBudget`; extending `Lovelace.Knowledge`'s sample type; everything in the brief's §17 non-goals.

---

## K. Risks

| Risk | Where it bites | Mitigation |
|---|---|---|
| **Semantic regression from the argument contract** | `ArgumentContractException : ArgumentException` changes `Assert.Throws<ArgumentException>`-style tests and some pinned messages | per-layer commits, control trees, and the pinned tests as the arbiter; where a message is pinned the site keeps its exception type and gains only bounds/exactness |
| **Precision-contract migration** | ADV-04 changes what `Real.Sin/Cos` return; an embedder may depend on the old (wrong) width | the change is *narrowing* and *correcting*; ADV-05 plus a documented before/after table; the preserved test file re-enters the tree intact |
| **ADV-04 falsified a third time** | it is the highest-risk package by history | read the two prior attempts first; mandatory control tree; the digit-count-only hypothesis is explicitly rejected with F-2's evidence; the scale-aware cap has a named falsifier |
| **Performance regression** | extra precision in the trig guard; solver polls | ADV-05 / ADV-10 measure; the benchmark's transcendental rows and the timing-tagged suites gate it |
| **Protocol compatibility** | every new field is additive, but a strict consumer schema could break | `protocolVersion` stays 1; new fields optional and omitted (never `null`); golden fixtures updated; `DocsSyncTests` extended, since the protocol document is machine-checked against the implementation |
| **Test-oracle weakness** | mpmath comparisons could be written to agree with the implementation | ADV-05's oracle self-test (a deliberately truncated value must fail); the correct-prefix contract replaces golden strings |
| **Monte Carlo false confidence** | a green falsifier run means "nothing found", never "nothing there" | the report states samples, seed, coverage and the untested region list; Gate 3 requires a planted defect to be found, not merely a clean run |
| **Boundary overfitting** | boundary bias could be tuned to one planted defect | the hypothesis is stated with its falsifier, and Gate 3 requires more than one defect |
| **Expression explosion** | minimisation and boundary search both grow expressions | explicit step budgets; `WorkBudget` (ADV-10) makes them interruptible |
| **AOT regression** | new types must stay trim/AOT-safe | no reflection, no dynamic serialization; the CI `aot-smoke` job is the gate; source-generated JSON contexts only |
| **Agent-benchmark contamination** | ADV-14 must not leak the rich surface into the frozen L0-L3 projections | the projections are frozen by test; the controller files are not edited; the harness surface is a separate, documented code path |
| **Stale-binary measurement** | F-5 already poisoned one round of my own probes | Gate 0 fingerprints **both** publish paths; every measurement names its artefact |

---

## L. Definition of Done

1. **Every integer-semantic argument in the product obeys one rule**, implemented once, with the §B2 enumeration
   reduced to a single owner and no site that floors, truncates, wraps or silently bounds.
2. **`Real.Sin/Cos` deliver the digits they claim**: for every case in the oracle battery the requested fractional
   digits are correct against mpmath, no digit beyond the certified cap is published, and the near-π/2 tiny-value
   family still returns its value.
3. **The failure envelope is a complete description of what happened**: code, category, recoverability, position, the
   offending argument's identity and bounds, the builtin's signature, the committed variable state, the committed
   output stream, and every artifact written — on the CLI, through `SuiteEngine`, in Studio and through DSH, from
   one engine result.
4. **The capability surface is queryable without evaluating anything**, deterministically, with bounded output and an
   honest truncation flag, and the production DSH surface exposes it.
5. **A budget claim is a true claim**: operations that advertise cooperation demonstrably stop within a stated bound,
   and operations that do not can no longer be described as stoppable.
6. **Behavioural discovery is an adversarial verifier**: seeded, deterministic, boundary-biased where it measurably
   helps, with a minimiser and an exporter, and at least one real counterexample converted into a permanent test
   that fails on the pre-fix kernel.
7. **Everything is measured on a named artefact** published from the final commit, and the harnessability benchmark
   is re-run with enough replicates to distinguish signal from noise.
8. **A fresh adversarial wave, run by agents that did not implement the fixes, cannot falsify the above** — or its
   findings are triaged and either closed or explicitly carried, with A+ not claimed while any P0/P1 is open.

---

## Decision requested

This plan declines three things the brief leaned toward — an equality-saturation build, a public
`CertifiedApproximation` type, and a generalized execution-budget abstraction — and it adds one finding class the
brief did not name (`evalir`'s digit contract, F-1) to **Must land**. Implementation has not started.
**Approval is requested before any code changes.**
