# Lovelace — A+ Symbolic Runtime Convergence, Cycle 3: Alignment Addendum

> **Status:** Alignment proposal (**awaiting approval**). This cycle is a **reproduction + plan**
> gate: no production code, test, or document other than this addendum and the harness records
> under `docs/goal-cycle-3/` was modified while producing it.
>
> **Predecessor contracts are law.** `docs/symbolics/a-plus-convergence-alignment-plan.md`
> §C–§J (frozen types, solver completeness semantics, domain contract, DSH schema, rewrite failure
> contract, assumption semantics, host formatting, intentional breaks) are **not redefined** here.
> Where this document proposes a change to a frozen surface it is called out as an **intentional
> break** with the §J sanction that permits it, or as a **decision requested** (§8) — never applied
> silently.

---

## 0. Baseline (recorded before any change)

| Item | Value |
|---|---|
| Branch / HEAD | `main` @ `35e2609f541c14d3933987657ec27909e7424d11` ("docs(symbolics): Cycle-2 final report") |
| Working tree | **clean** (`git status --porcelain` empty) |
| Shell / SDK | Windows PowerShell 5.1 (Desktop) · .NET SDK 10.0.103 |
| Build | `dotnet build LovelaceSharp.slnx -c Release` → **0 errors, 51 warnings** |
| Warnings (unique) | 34×`CS0109` (`Lovelace.Real/Real.cs`), 8×`CS8767` (`Integer.cs:462,464`; `Real.cs:470,472`), 7×`CS8600` (`Real.cs:635,862,910,1056,1116,1189,1979` — all `out` parameters of `Nat.TryParse` declared `[MaybeNullWhen(false)]`), 2×`CS8602` (`Representation/DigitStore.cs:613`, `Real.Tests/RealParseTests.cs:88`). `CS0162` at `Lovelace.Run/Program.cs:34` no longer appears |
| All suites | **1976 passed / 0 failed / 0 skipped** across 14 projects (see below) |
| Native AOT publish | `dotnet publish Lovelace.Run -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot` → **succeeded**; `out/aot/Lovelace.Run.exe` = 5 611 520 bytes |
| Freshness gate | exe mtime `2026-09-10T23:21:57` vs newest first-party source `2026-09-10T20:20:43` → `STALE=False` (the Cycle-2 audit mistake is guarded mechanically) |
| Published-runner smoke | **5/5 pass** on the native binary (solve, partial, jacobian, stdout purity, domain error) |
| Benchmarks | `benchmarks/symbench-baseline.md` (15 878 B) and `benchmarks/raw-baseline.txt` (62 868 B) present; `symbench` builds (exit 0) |
| Python | `python`/`python3` resolve to Microsoft Store **alias stubs** (version 0.0.0.0) — the SymPy oracle cannot execute here |
| Evidence | `docs/goal-cycle-3/round-1/` — `baseline.ps1` + logs, `probes.ps1`…`probes5.ps1` + 60 recorded envelopes |

**Per-suite counts** (all `Failed: 0, Skipped: 0`): Abstractions 20 · Array 19 · Complex 83 ·
**Console 15** · Dsp 61 · Integer 148 · Knowledge 28 · Natural 195 · Representation 91 · Real
(`Category!=Heavy`) 272 · Studio 22 · Suite 620 · Symbolics 389 · precbench 13.

> **Correction carried forward.** The Cycle-2 report states "all suites 1971". That figure is not
> the sum of its own per-suite list, which is 1976. The measured total today is **1976**. New
> adjacent issue **N9**.

---

## 1. Reproduction method

Every verdict below was produced by executing the **freshly published Native AOT binary**
(`out/aot/Lovelace.Run.exe --file <probe>.ls --omit-functions`, envelope parsed as JSON) or
by reading the current working tree at `35e2609`. No verdict rests on source reading alone.
Verdict vocabulary: **Confirmed** · **Already fixed** · **Not applicable** · **New adjacent issue**.

Three probe errors were made and corrected during this reproduction; they are recorded because the
correction is the evidence:

| Probe error | What it looked like | Correction |
|---|---|---|
| `--print-budget` appeared inert | budget 6 produced byte-identical output | the probed expression had `nodeCount 5 << 6`. On `expand((x+1)^12)` (nodeCount 58) the budget works: `truncated=true, reason=node-budget, budget=6`, rendering ends `" …"`. **Not a defect** |
| `compile_full`/`optimize_full` failed | `"Index was out of range"` | both take `(f, params)`; my call had one argument. Re-probed correctly — but the wrong-arity call exposed **N1** |
| `CancellationToken` appeared absent from the kernel | a first regex (`\bCancel\b`) matched only `RationalFunctions.Cancel` | corrected regex finds 9 sites across 7 kernel files. Cancellation is plumbed; see OQ-2 |

---

## 2. Reproduction matrix — §2 (P0 contract conformance)

### §2.1 `solve_system_full` emits prose, not structure — **Confirmed**

`solve_system_full([x + y == 1, x - y == 3], [x, y])` →

```text
Record(SystemSolveResult){
  status: Text("Solved"),
  solutions: Array[1] < Record(SystemSolution){
      assignment: Array[2] < Text("x = 2") > x2,
      conditions: Array[0] } >,
  diagnostics: Text("") }
```

`assignment` is a **Vector of Text**. The frozen §C requires
`SystemSolution(Bindings, Conditions, Exactness)` with `Binding(Name, Value)`, and §J
already sanctions the break («`solve_system_full` `assignment` (strings) →
`bindings` (records)»).

Source, verified by reading: `Lovelace.Symbolics/SymbolicsPlugin.cs:335-337` builds each
entry as `v.Name + " = " + Printing.PrettyPrint(...)`. The structural type **already
exists and is unused**: `Solve.cs:69` declares `public sealed record Binding(string Name,
Expr Value)` with the comment "The one structural binding contract for solver output … (never
an 'x = ...' string)".

Broader than the brief, same defect:

| # | Observed | Contract (§C) | Source |
|---|---|---|---|
| a | `assignment: Array<Text>` | `IReadOnlyList<Binding>` | `SymbolicsPlugin.cs:335-337` |
| b | no `exactness` on `SystemSolution` | `SolutionExactness Exactness` | `Solve.cs:837` |
| c | no `domain`, no `complete` on `SystemSolveResult` | `Domain Domain, Completeness Complete` | `Solve.cs:851, 866` compute both and neither is emitted |

### §2.2 `SolutionFamily.parameter` is Text — **Confirmed**

`solve_full(sin(x) == 0, x)` →

```text
Record(SolutionFamily){
  template: Symbolic[k*pi],  parameter: Text("k"),  period: Symbolic[pi],
  parameter_domain: Domain(integer), conditions: Array[0], exactness: Text("ParametricExact") }
```

Half a migration exactly as described: the domain is a Domain value, the parameter is still a name.
The kernel is already structural — `Solve.cs:78` declares `Symbol Parameter` and all
five construction sites pass a real symbol. The flattening happens in the plugin at
`SymbolicsPlugin.cs:395` (`f.Parameter.Name`).

**New adjacent issue N5:** the very next line, `SymbolicsPlugin.cs:397`, calls
`ParameterDomainOf(f.Domain)` whose entire body is
`private static MathDomain ParameterDomainOf(ParameterDomain p) => MathDomain.Integer;`
(`:612`) — the parameter is **ignored**, so the `NonNegativeIntegers` case declared
at `Solve.cs:72` cannot ever reach the wire.

### §2.3 `diagnostics` is Text, not `Diagnostic[]` — **Confirmed, and broader**

Executed shape of the field on every record that has one:

| Record | Emitted `diagnostics` | Populated from | Source |
|---|---|---|---|
| `SolveResult` | `Text` | `set.Note ?? ""` | `SymbolicsPlugin.cs:413` |
| `SystemSolveResult` | `Text` | `result.Note ?? ""` | `SymbolicsPlugin.cs:341` |
| `LimitResult` | `Text` | `FailureReason` | `Limits.cs:15, 307` |
| `IntegrationResult` | `Text` | `Note` (never assigned) | `Integrate.cs:15, 220` |
| `TransformResult` | **absent** | — (has `status` Text + `budget_exceeded` Boolean instead) | — |
| `OptimizationResult` | **absent** | — | — |
| `CompilationResult` | **absent** | — | — |

Observed: `solve_full(x^4 - x^2 - 1 == 0, x)` emits
`diagnostics: Text("")`; `solve_full((x^2-1)/(x^2-1) == 0, x)` emits
`diagnostics: Text("every root violates the denominator condition.")`.
`compile_full(x^2 + 1, [x])` and `optimize_full(x^2 + 2*x + 1, [x])` emit **no**
`diagnostics` field at all.

No type anywhere in the repository carries `IReadOnlyList<Diagnostic><`; there is not even a
type named `SolveResult` (0 declarations repo-wide — the name exists only as a projection
string). **The freeze declares a record that was never built.**

The same "structure destroyed at the Suite boundary" defect reaches every enum-typed member, which
the brief's item 3 lists only as "completeness/status spelling":

```text
status:       Text("Partial")          §C: SolveStatus enum      §F: {"kind":"Enum","type":"SolveStatus"}
completeness: Text("Partial")          §C: Completeness enum      §F: {"kind":"Enum","type":"Completeness"}
complete:     Boolean(false)           §C: (no such member)
variable:     Text("x")                §C: Symbol Variable        §F: (not shown)
exactness:    Text("AlgebraicExact")   §C: SolutionExactness enum §F: {"kind":"Enum", ...}
classification: Text("Universal")      §C: RuleClassification     §F: (not shown)
```

`complete` and `completeness` therefore spell the same fact twice, in two types,
with no invariant tying them together.

### §2.4 `NoSolutions` with `complete:false / completeness:Unknown` — **Confirmed**

Five independently chosen inputs, all producing `status NoSolutions`:

| Probe | `complete` | `completeness` | `diagnostics` |
|---|---|---|---|
| `solve_full((x^2-1)/(x^2-1) == 0, x)` | `false` | `Unknown` | "every root violates the denominator condition." |
| `solve_full(x/x == 0, x)` | `false` | `Unknown` | "every root violates the denominator condition." |
| `solve_full(x^2 + 1 == 0, x, real)` | `false` | `Unknown` | "no real solutions" |
| `solve_full(exp(x) == 0, x)` | `false` | `Unknown` | "exp(u) = 0 has no solution." |
| `solve_full(sqrt(x) == -2, x)` | — | — | (returns `Solved` with one root; recorded) |

Source: `Solve.cs:96-101` — `Completeness Complete => Status switch { Solved => Complete,
Partial => Partial, _ => Unknown }`. An empty-and-proven-empty set falls into the `_` arm.

§D defines `NoSolutions` as "the solution set over the requested domain is **provably
empty**". A provably empty set is a *complete* answer, so the coherent pairing is
`complete: true / completeness: Complete` (decision **D4**).

**Latent three-way contradiction (claim Falsified, risk retained).** Observer A read
`SymbolicsPlugin.cs:384-406` and claimed the emitted pair for this case is
`status "NoSolutions" + completeness "Complete"`. That is **falsified at runtime** by all
five probes above. The reading is still worth recording: when `:384` rewrites the *local*
`status` to `NoSolutions` while `set.Status` stays `Solved`,
`:406` evaluates `set.Complete` → `Complete`. I could not construct an input
reaching that branch (5 attempts, all took the kernel's own `NoSolutions` path). The item-4
fix must **single-source the derivation** so the branch cannot disagree, rather than patch the
symptom — recorded as **RISK-003**.

### §2.5 No golden fixtures, no runner test project — **Confirmed**

Confirmed by my own searches, not by report:

- `glob **/*golden*` → **0 matches**; no checked-in JSON fixture exists anywhere outside
  `obj/`, `bin/` and the harness's own probe records.
- `glob **/*Lovelace.Run.Tests*` → **0 matches**; there is no test project for the runner.
- 14 test projects exist on disk. The runner's envelope is asserted **only** by inline `python3`
  heredocs inside `.github/workflows/ci.yml:150-250`, which cannot run on this machine and is
  not unit-testable.
- The §113/§114 corpus the alignment plan specifies (Symbolic, Vector, Array, Record, nested Record,
  SolveResult, TransformResult, LimitResult, SystemSolveResult, CompilationResult, Complex, absent
  field, error envelope, `print()<<B>-plus-result) does not exist in any form.

### §2.6 **New adjacent issue N1** — a two-argument builtin called with one argument leaks a framework exception

`compile_full(x^2 + 1)` (missing `[x]`) → exit 1,
`code=InvalidArgument category=TypeMismatch`,
`message="Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')"`.

This is the same defect class as Cycle-2's consequential find (a framework exception where the
kernel's typed error is required). Cycle-2's A12 shipped descriptor-driven arity validation for
`diff`; it does not cover positional index reads, so a *user* mistake is reported as an
internal index failure. P0-class: it is an acceptance-gate item ("an internal error surfacing" as a
user-facing code).

---

## 3. Reproduction matrix — §3 (P1 capability and verification gaps)

### §3.6 Real-only limitations surface as arithmetic errors — **Confirmed**, with two corrections

| Probe | `code` | `category` | message | throw site |
|---|---|---|---|---|
| `sqrt(-1)` | `ArithmeticError` | `DomainError` | "Square root is not defined for negative numbers." | `Real.cs:818` (`ArithmeticException`) |
| `sqrt(-4)` | `ArithmeticError` | `DomainError` | same | same |
| `(-1)^(1/2)` | `UnsupportedOperation` | `UnsupportedOperation` | "Non-integer exponents are not yet supported." | `Real.cs:730` (`NotImplementedException`) |
| `(-8)^(1/3)` | `UnsupportedOperation` | `UnsupportedOperation` | same | same |
| `2^-1` | `InvalidArgument` | `TypeMismatch` | "Exponent must be positive. (Parameter 'exponent')" | `Integer.cs:378` |
| `0^(-1)` | `InvalidArgument` | `TypeMismatch` | "**Base** cannot be zero. (Parameter '**exponent**')" | `Integer.cs:378` |
| `atanh(0.5)` | `InvalidOperation` | `DomainError` | "Unknown function 'atanh'." | dispatch |
| `acosh(2)` | `InvalidOperation` | `DomainError` | "Unknown function 'acosh'." | dispatch |
| `asinh(1)` | `InvalidOperation` | `DomainError` | "Unknown function 'asinh'." | dispatch |

**Correction 1 — the last three are *not* unimplemented.** `asinh`/`acosh`/`atanh`
**are** registered kernel functions with evaluators and a domain rule
(`Functions.cs:93/95/97`, `Evaluation.cs:341-345`, `Assumptions.cs:723-725`);
they are simply not exported as builtins (`SymbolicsPlugin.cs:25-26` excludes them from
`ElementaryFunctions`; `CoreBuiltinMetadata.cs:22-82` has no hyperbolic entry).
They are one registration line from working — this is an **exposure** gap, not missing mathematics.
The same correction extends the brief's list: `asinh` is in the same state as
`atanh`/`acosh`.

**Correction 2 — the failure classes are not all "arithmetic errors".** Four distinct codes appear
(`ArithmeticError`, `UnsupportedOperation`, `InvalidArgument`,
`InvalidOperation`). The problem is not that the code is wrong; it is that **the *capability*
is never stated**: an agent must call a function and catch an error to discover that the kernel is
real-only for roots, that negative exponents are unsupported, and which transcendentals exist.

**New adjacent issue N2:** `0^(-1)` reports `(Parameter 'exponent')` for a base-domain
failure — the framework parameter name is wrong on the wire.

### §3.7 `abs(x)` cannot be written although the engine produces it — **Confirmed**

| Probe | Result |
|---|---|
| `abs(-3)` | `Integer(3)` — numeric `abs` works and is a builtin (`CoreBuiltinMetadata.cs:32`, `Interpreter.cs:1224`) and a kernel function (`Functions.cs:107`) |
| `x = symbol("x"); abs(x)` | `InvalidOperation/DomainError "abs() is not supported for values of kind 'Symbolic'."` |
| `x = symbol("x"); diff(abs(x), x)` | same message |
| `x = symbol("x", real); simplify_full(sqrt(x^2))` | **succeeds**: `expression: Symbolic[abs(x)]`, one step `pow.sqrt-square-real` carrying a `DomainCondition` |

A value the engine creates cannot be read back into it — exactly as the brief states. There is no
`Absolute` node kind (`Expr.cs:8-13`), which is why the payload kind check rejects
`Symbolic`.

### §3.8 Redundant parentheses in the pretty printer — **Confirmed, and wider**

| Probe | Rendering |
|---|---|
| `integrate_full(exp(-x^2), x)` | `integrate(exp((-x^2)), x)` — the double parens the brief names |
| `(x + 1)^12` | `((x + 1))^12` — doubled parens on an input echo |
| `solve((x+1)^3 == 8, x)` | third root renders `-(1 - 2*(-1/2*i*sqrt(3) - 1/2))` — not merely extra parens: an un-simplified nesting where every CAS prints `-2 - i*sqrt(3)` |

The parse is correct (the sign/precedence defect is fixed — see §5). The rendering is not. Source:
`Printing.cs:596-597` wraps a power base when `Prec(p.Base) <= 3`, plus a function
application arm at `:606-609` that never unwraps. `PrintMode` is exactly
`{Canonical, Pretty, Debug}` (`Printing.cs:325`) — there is no second printer to extend,
which is what makes decision D6 safe. A printer fix must not be a second
renderer: the same `Printing``/``PrintOptions` path feeds
`pretty`, `canonical` and `display<<B>.

### §3.9 `AssumptionSet.Add<<B> is quadratic in atoms — **Confirmed at source; measurement carried forward**

`Assumptions.cs:92-113`: a `Sort` per non-duplicate insertion, with
`ToString()` inside the comparator (`:98`) — the ordering key is re-rendered on every
comparison instead of being precomputed per atom. The Cycle-2 measurement stands as the recorded
baseline (100 atoms 8.18 ms → 200 → 26.91 ms → 400 → 91.13 ms, exponent ≈ 1.8); it was **not**
re-measured this round, deliberately: no code changed, and item 26's rule is measure-before-restructure.

### §3.10 The metamorphic set (§116) — **Already fixed (5 of 6); the brief's premise is falsified**

The brief states the set "is not implemented". Five of the six §116 properties exist as property
tests, verified by reading the cited test bodies:

| §116 property | Present | Citation | Note |
|---|---|---|---|
| `d/dx ∫f = f` | **yes** | `PropertyTests.cs:146 Integrate_DiffRoundTrips` | 6 integrands, seeded RNG |
| `substitute(solution) = 0` | **yes** | `PropertyTests.cs:177 Solve_QuadraticRoots_SatisfyPolynomial` | 40 random quadratics |
| `evalIR(compiled) = eval(symbolic)` | **yes** | `PropertyTests.cs:236 MathIR_EvaluatesLikeSymbolic` | 60 random exprs, precision scope |
| `expand(factor(p)) = p` | **yes** | `SmokeTests.cs:59 Factor_SquaresDifference_FullyFactors` | asserts `Expand(Factor(p)) == p` |
| `A·A⁻¹ = I` **under conditions** | **partial** | `SmokeTests.cs:157` | numeric only: `:164` comments "canonical form does not factor common denominators", so the identity is checked at one substituted point, never symbolically under the `det ≠ 0` condition the kernel computes (`SymbolicMatrix.cs:235-255`) |
| `cancel preserves value off poles` | **absent** | — | `RationalFunctions.Cancel` exists (`Algebra/RationalFunctions.cs:98`, exported at `SymbolicsPlugin.cs:273`) with no property test |

So item 10 is **not** "implement the metamorphic set" but "close two named gaps", one of which is a
real strengthening (symbolic `A·A⁻¹` under the emitted conditions) and one a new property.

### §3.11 Falsification breadth (§117) — **Confirmed**

`Lovelace.Symbolics.Tests/FalsificationTests.cs`: the point set is 13 fixed rationals
(`:17-22`), reused by subsets at `:87`/`:107`/`:165`/`:175`,
with a single complex point at `:141-142` that is not in the main identity sweep. No
occurrence anywhere in the file of a pole, branch cut, discontinuity or assumption-boundary
strategy. The rule set is hard-coded per `[Fact]` (9 ids) even though registry accessors
exist and are unused there (`Simplify.cs:182/188`) — so a newly added rule is **not**
automatically sampled.

### §3.12 The SymPy differential oracle — **Confirmed, and worse than described**

- Coverage: differentiation (`:80-82`, 6 expressions) and one solve corpus (`:105-107`,
  4 equations). No roots, no factorization, no limits, no matrix operations.
- The skip is `if (SympyProbe() is null) return;` inside a **plain `[Fact]<<B]>** at
  `:64-65` and `:91-92`. There is no skip attribute and no assertion — **the test
  reports pass when the oracle is absent**. An oracle that never executes is worse than no oracle:
  it reports green.
- `ci.yml:57-83` runs the project with no `pip install sympy` step, so the oracle is
  absent in CI too. It is green *everywhere*, and has never compared anything.
- On this machine `python`/`python3` are Store alias stubs.

---

## 4. Reproduction matrix — §4 (P2 polish, breadth, hygiene)

| Item | Verdict | Evidence |
|---|---|---|
| **13 LaTeX printer (§56)** | **Confirmed absent** | `grep -i "latex\|\\\\frac"` over `*.cs` → **0 hits**. No LaTeX output exists in any form |
| **14 Record schema registry gaps** | **Confirmed** | `Lovelace.Suite/RecordSchemas.cs:63-113` registers **17** schemas. The interpreter emits `MatrixInverseResult` (`Interpreter.cs:1273, 1278`) and `MatrixSolveResult` (`Interpreter.cs:1332`); neither is registered, so the drift test cannot see them. The drift corpus itself
  (`RecordSchemaTests.cs:47-59`) never calls `inv_full` or `linsolve_full`, so the
  registry and the corpus are blind in the same direction: 19 emitted record names, 17 schemas **Sequencing hazard:** the registry *pins the current text shapes* — `("diagnostics","Text")`, `("parameter","Text")`, `("status","Text")` — so items 2–3 must update the registry in the same change or the drift test goes red |
| **15 Proof bridge coverage** | **Confirmed complete** | `RewriteProofs.cs:46-66` — 9 obligations, one per shipped rule, all `ProofStatus.Unproven`; a registry-equality invariant test keeps the map in step. Nothing to do unless rules are added |
| **16 Test-project wiring** | **Confirmed defect** | `Lovelace.Console.Tests` is **absent from `LovelaceSharp.slnx` and from the `ci.yml` suite list** — its 15 tests pass locally and **cannot fail CI**. `symbench` **is** in the slnx and CI builds it (`ci.yml:51-55`, build-only by design); it contains no test framework, so "add to the suite list" is **not applicable** to it |
| **17 Pre-existing warnings** | **Confirmed, understated** | 51 unique warnings on a plain `Release` build (not RID-only): 34×`CS0109`, 8×`CS8767`, 7×`CS8600`, 2×`CS8602`. The brief names only `CS8767`/`CS8600`; the dominant class is `CS0109` ("`new` keyword is not required") on 34 members of `Lovelace.Real/Real.cs`. `CS0162` no longer appears |
| **18 Quotable benchmark run** | **Not re-run** | Baseline and raw transcript present. A longer job on an idle machine is required for a defensible *improvement* claim; allocations remain the trustworthy signal |
| **19 `--omit-display`** | **Confirmed absent, and loudly rejected** | `out/aot/Lovelace.Run.exe --omit-display` → exit **2**, `Error: Unknown argument '--omit-display'.` The runner validates unknown arguments, so this is a clean refusal rather than a silently ignored flag |

---

## 5. Do-not-regress — verified live on the freshly published binary

Every row below was executed this session against `out/aot/Lovelace.Run.exe` (mtime newer
than all source). This is the regression snapshot the cycle must keep green.

| Behaviour | Probe | Observed |
|---|---|---|
| Unary minus vs exponent | `-2^2` | `Integer(-4)` |
| Substitution precedence | `subs(-x^2, x, 3)` | `Symbolic[-9]` |
| Range from a negative literal | `-1..2` | `Array[4]` from `Integer(-1)` = [-1,0,1,2] |
| Range binds tighter than `^` | `1..3^2` | `Array[3]` from `Natural(1)` = (1..3)² = [1,4,9] |
| Incomplete solve is honest | `solve_full(x^4 - x^2 - 1 == 0, x)` | `Partial / complete=false / unrepresented_count=2` |
| Shifted quadratic | `solve((x+1)^2 - 4 == 0, x)` | `Array[2]` starting `-3` |
| Cubic returns 3 roots | `solve((x+1)^3 == 8, x)` | `Array[3]` |
| Ascending real ordering | `solve(x^2 - 4 == 0, x)` | `Array[2]` starting `-2` |
| Unsupported domain rejected | `solve(2*x == 1, x, integer)` | `DomainError` "solve(): currently supports domains real and complex; got integer." |
| Unevaluated, not a bad subset | `solve(x^4 + 1 == 0, x)` | `Text("unevaluated: complex algebraic roots not supported (RootOf is real-only in v1).")` |
| Pole exclusion ⇒ NoSolutions | `solve_full((x^2-1)/(x^2-1) == 0, x)` | `status NoSolutions` |
| Multiplicity survives | `solve_full(x^2 - 2*x + 1 == 0, x)` | one `Solution` with `multiplicity Integer(2)` |
| `domain` is a Domain value | any `SolveResult` | `{"kind":"Domain","domain":"complex"}` |
| Assumption folding | `assume(x >= 5); simplify(x < 5)` | `Boolean(false)` |
| Integer-interval contradiction rejected | `symbol("x", integer); assume(x > 1/2); assume(x < 1)` | `UnsatisfiableAssumptions` "leaves no integer satisfying the existing assumptions." |
| Real-domain control still accepted | `symbol("x"); assume(x > 1/2); assume(x < 1)` | `Symbolic[x < 1]` |
| Mirrored spelling accepted | `assume(1/2 < x)` | accepted, `Symbolic[1/2 < x]` |
| Exact rational payload | `1/3` | `Real(0.(3), exact=true, numerator=1, denominator=3)` |
| Zero is exact | `1/3 - 1/3` | `Real(0, exact=true, 0/1)` |
| Approximations carry no rational form | `evalf(1/3, 50)` | `exact=false`, no numerator/denominator |
| `type()` vocabulary | `type(x)` | `Text("Symbolic")` |
| `inspect` is uniform | `inspect(x^2 + 1)` | `Inspection` with `domain: Domain(complex)` |
| Array shape row-major | `jacobian([x*y, x+y], [x, y])` | `Array[2,2]`, first element `Symbolic[y]` |
| stdout purity | `print("hello from the script"); 1 + 1` | `output=["hello from the script"]`, result `Natural(2)` |
| Three protocol versions | any success | `protocolVersion 1`, `#!lovelace-sym 1`, `mathIrVersion 2` |
| Structured condition leaves | `symbol("x", real); simplify_full(sqrt(x^2))` | `DomainCondition{variable: Symbolic[x], domain: Domain(real)}` |
| Print budget honoured | `expand((x+1)^12) --print-budget 6` | `truncated=true, truncationReason=node-budget, budget=6, nodeCount=58`, rendering ends `" …"` |
| `--omit-variables` | any run | variable list empty, envelope otherwise unchanged |
| Unknown flag refused | `--omit-display` | exit 2, structural usage error |

**Not verified this round (carried as OQs, not silently dropped):** cancellation actually firing,
reentrancy diagnostics, cross-engine isolation, Studio API round-trip, REPL output text, and the 132
descriptor examples. These are claimed green by Cycle 2 and are re-verified in the implementation
phase rather than re-litigated here.

---

## 6. New adjacent issues found this cycle

| ID | Issue | Evidence | Class |
|---|---|---|---|
| **N1** | A two-argument builtin called with one argument leaks a framework `ArgumentOutOfRangeException`: `compile_full(x^2 + 1)` → `InvalidArgument/TypeMismatch "Index was out of range…"` | executed | P0-class (internal error on the user surface) |
| **N2** | `0^(-1)` reports `(Parameter 'exponent')` for a base-domain failure | executed | P1 (message quality) |
| **N3** | `SystemSolveResult` emits neither `domain` nor `complete` although the kernel computes both | `Solve.cs:851, 866` + executed envelope | P0 (frozen-contract) |
| **N4** | `SystemSolution` emits no `exactness`; `Binding` is declared and unused | `Solve.cs:69, 837` | P0 (frozen-contract) |
| **N5** | `ParameterDomainOf` ignores its argument and always returns `Integer` | `SymbolicsPlugin.cs:612` | P1 (latent wrong value) |
| **N6** | `simplify_full((x+1)^40)` reports `changed: true` with `steps: []` and identical `original`/`expression` renderings | executed | **Unresolved → OQ-1** |
| **N7** | The pretty printer emits un-simplified nested radicals and doubled parens on three independent probes | executed (§3.8) | P1 (same code path as item 8) |
| **N8** | Latent: `SymbolicsPlugin.cs:384-406` can emit `status NoSolutions` with `completeness Complete`; unreachable in 5 attempts | observer claim, **Falsified at runtime**, source risk retained | RISK-003 |
| **N9** | Cycle-2's suite total (1971) contradicts its own per-suite list (1976) | arithmetic over recorded counts | Documentation honesty |
| **N10** | All 117 builtins have metadata (still true), but three kernel functions (`asinh/acosh/atanh`) have evaluators and **no** descriptor — the metadata-completeness test can only see registered builtins, so this class of gap is invisible to it | `Functions.cs:93/95/97` vs `CoreBuiltinMetadata.cs:22-82` | P1 (verification blind spot) |

---

## 7. Explicitly already fixed / not reproducible — do not redo

1. **The metamorphic set (§116) is 5/6 implemented** (§3.10). Do not rebuild it; close the two gaps.
2. **`--print-budget` works** — it abbreviates, reports `truncated`,
   `truncationReason`, `budget` and the true `nodeCount`, and the abbreviation
   is a prefix of the true rendering. Probe error, not a defect.
3. **The runner refuses unknown arguments** (`--omit-display` → exit 2) rather than ignoring
   them.
4. **`--omit-variables``, `--cancel-after` and `--print-budget` are all
   wired** (`Program.cs:66-80, 159-161, 192`), and a `CancellationTokenSource(budget)`
   is constructed and passed to `EvaluateAsync`.
5. **Structured condition leaves, Domain values, exact rationals, array shape, stdout purity,
   `inspect`, `type()`, timings and the three versions** — all verified live (§5).
6. **The sign/operator precedence defect is fixed** and stays fixed (§5).
7. **`symbench` is already in the solution and in CI** — the remaining item-18 work is a
   longer measurement, not wiring.
8. **The proof bridge is complete for the shipped rule set** (9/9).

---

## 8. Decisions requested before implementation

| ID | Decision | Recommendation |
|---|---|---|
| **D1** | `diagnostics` shape: add `IReadOnlyList<Diagnostic> Diagnostics` to **all seven** rich records (adding the member to `TransformResult`, `OptimizationResult`, `CompilationResult` where it is absent), and decide the fate of the parallel free-text members (`SolveResult.Note`, `SystemSolveResult.Note`, `LimitResult.FailureReason`, `IntegrationResult.Note`) | **Add the structured member everywhere.** Keep `unrepresented_reason` (already its own §C field) as the human projection; **drop** the free-text `Note`/`FailureReason` from the emitted record so no semantic string survives on the wire. A caller that wants prose reads `display` |
| **D2** | **Status/completeness spelling — a protocol revision.** §C declares enums; alignment-plan §F shows `{"kind":"Enum","type":"SolveStatus","value":"Partial"}`; `dsh-protocol.md:57` shows `{"kind":"Text","value":"Solved"}` and its "Value forms" list has **no Enum kind**. The two frozen documents disagree | **Follow §F: emit an `Enum` value kind** for `status`, `completeness`, `exactness` and `classification`, keep the human spelling in `display`/`typed`, and **amend `dsh-protocol.md`** (it is the protocol doc; the alignment plan is the contract). This is an intentional, documented protocol revision — flagged because item 19's bar is "not as a convenience flag" |
| **D3** | Keep the Boolean `complete` beside the `Completeness` enum, or drop it? §C declares only the enum, but the published CI smoke and the Cycle-2 fixtures assert `complete` | **Keep it, derived.** Both must come from **one** expression so they cannot disagree, and the redundancy is documented as a convenience field. Dropping it is a breaking change to the published smoke for no gain |
| **D4** | Correct pairing for a provably empty solution set | `status: NoSolutions` ⇒ `completeness: Complete` and `complete: true`. §D defines the status as "provably empty over the requested domain" — that is a complete answer, and today's <<B>Unknown/false` is the self-contradiction item 4 names. The derivation is single-sourced so RISK-003 cannot occur |
| **D5** | Item 6 shape: implement complex roots / rational powers / negative exponents, **or** publish a machine-readable capability statement | **Both, bounded.** (a) Register `asinh`/`acosh`/`atanh` as builtins (they already exist in the kernel — one descriptor each); (b) implement negative **integer** exponents (`2^-1 = 1/2`) and numeric negative-base roots through the existing `Lovelace.Complex` assembly; (c) add a structured `capabilities()` builtin returning a record (supported domains, unsupported operation classes) **and** keep every per-call typed error. Symbolic complex roots stay out of scope per §D's root-representation decision |
| **D6** | Item 13 LaTeX | **Implement as a `PrintMode.Latex` arm of the existing printer** once P0/P1 are green — structurally it *cannot* be "a second printer that can disagree". If it competes with §2–§3 for the round budget it is recorded as below-A+ rather than half-shipped |
| **D7** | Item 12 oracle: how to stop a never-executing oracle reporting green | The local skip becomes **visible and conditional**: the tests fail when `LOVELACE_REQUIRE_SYMPY=1` and sympy is missing, and the new CI job sets it after installing sympy. Extend the corpus to roots, factorization, limits and matrix ops under explicitly matched domains (§115). This is part of the deliverable, not follow-up |
| **D8** | Item 14 registry | Extend `RecordSchemas` with every interpreter-emitted record (starting with `MatrixInverseResult`, `MatrixSolveResult`) **and** derive the emitted-type set from a test that walks a live corpus, so a new record type cannot be emitted unregistered |

---

## 9. Contract changes proposed (all additive or already sanctioned)

| Change | Kind | Sanction |
|---|---|---|
| `SystemSolution.assignment: Array<Text>` → `bindings: Array<Record(Binding)>` | **Intentional break** | §J row "`solve_system_full` `assignment` (strings) → `bindings` (records)" |
| `SolveResult.variable: Text` → `Symbol`; `SolutionFamily.parameter: Text` → `Symbol` | **Intentional break** | §C declares both as `Symbol`; §J sanctions the family-parameter migration |
| `status`/`completeness`/`exactness`/`classification`: `Text` → `Enum` | **Protocol revision** | §F (frozen) already shows this form — **decision D2** |
| `Diagnostic[] Diagnostics` on all seven rich records | **Contract completion** | §C declares it on all seven |
| `SystemSolveResult.domain`, `.complete`; `SystemSolution.exactness` | **Contract completion** | §C declares them (N3, N4) |
| `NoSolutions` ⇒ `completeness: Complete, complete: true` | **Semantic correction** | §D — the status describes the requested domain — **decision D4** |
| New `capabilities()<<B> structured builtin | **Additive** | §E's "the supported-domain set is metadata, not duplicated magic checks" |
| `RecordSchemas` entries for matrix results; registry updated with every shape change above | **Additive** | §80 registry rule |


### 9.1 Wire-field resolutions (recorded before implementation)

Two field-name conflicts inside the frozen material are resolved here so the implementation has one
answer, per "amend the alignment doc rather than reducing scope silently":

1. **`Binding` field names.** §C declares `Binding(string Name, Expr Value)`; §F's prose says
   `system solve (bindings:[{variable, value}])`. The DSH protocol doc states that record field names
   come from the kernel records and are part of the contract, and §C **is** the kernel record
   contract. **Resolution: emit `{"kind":"Record","type":"Binding","fields":[{"name":"name",...},{"name":"value",...}]}`**
   — §C wins, §F's `variable` is read as shorthand. This is recorded because it is a wire field name
   and therefore cannot be changed later without a protocol revision.
2. **The `Enum` structured kind (decision D2).** §F shows
   `{"kind":"Enum","type":"SolveStatus","value":"Partial"}`; `dsh-protocol.md` documents Text and its
   value-form list has no `Enum`. **Resolution: implement the §F form.** The mechanism is additive and
   AOT-safe: a small `EnumValue(TypeName, Name)` carrier in `Lovelace.Abstractions`, one new
   `ValueKind.Enum`, one `PayloadMap` case, one projection arm, one formatter arm, and the schema
   kinds updated — no change to any existing value kind, and no reflection. `dsh-protocol.md` is
   amended in the same round. Recorded here because D2 changes the protocol for
   `status<<B>/`completeness<<B>/`exactness<<B>/`classification` and must not be discovered later as a surprise.

**No change is proposed to**: the status taxonomy names, the domain contract, the rewrite failure
contract, the assumption semantics, the envelope's version fields, or stdout purity.

---

## 10. Sequencing and what "done" means

```text
Phase A  P0 contract conformance (items 1-5 + N1, N3, N4)
           A1  structural bindings + SystemSolution.exactness + SystemSolveResult.domain/complete
           A2  SolutionFamily.parameter as Symbol; ParameterDomainOf stops ignoring its argument
           A3  Diagnostic[] on all seven records; enum kinds for status/completeness/exactness
           A4  NoSolutions pairing, derivation single-sourced
           A5  golden fixtures + Lovelace.Run.Tests, wired into the slnx AND ci.yml
           A6  RecordSchemas updated in the same change (drift test must stay green)
Phase B  P1 capability and verification (items 6-12 + N2, N5, N7, N10)
Phase C  P2 polish and hygiene (items 13-19 + N9), incl. Console.Tests in the slnx and ci.yml
Phase D  §5 do-not-regress re-run, benchmark deltas, §7 report, §143 audit on a fresh AOT publish
```

**Acceptance gates** (unchanged from the brief §6, with two additions this cycle found): no test
project outside CI; no contract deviation recorded in §2 still open; **no framework exception on the
user surface (N1)**; **no path that emits a status and a completeness derived from different
expressions (RISK-003)**.

**Every change lands with its test in the same round**, one bounded change per round, and the
freshly-published binary is the only thing audited.

---

## 11. Risks

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| RISK-001 | The A3 enum-kind change is a **wire-format revision** touching `StructuredProjection`, the Studio DTO, the CI smoke and every golden file at once | High | High | Land A5 (fixtures + runner tests) **first**, then change the wire with the fixtures as the red-to-green signal; update `dsh-protocol.md` in the same round |
| RISK-002 | `RecordSchemas` pins the *old* text shapes; A1–A3 will turn the drift test red | High | Medium | Update the registry inside each of A1–A3, never in a later round |
| RISK-003 | `SymbolicsPlugin.cs:384-406` can emit `status NoSolutions` with `completeness Complete` (unreachable in 5 attempts) | Low | High | A4 single-sources the derivation; add a targeted test that constructs the `Solved-but-all-rejected` state directly rather than hoping an input reaches it |
| RISK-004 | `Assumptions.cs`` changes are forbidden without the full suite (324-query + 324-pair matrices) | Medium | High | Item 9 is touched **only** if a precomputed ordering key can be added without altering the comparator's semantics; the scaling test alternative is preferred if the measurement is re-run |
| RISK-005 | Extending the oracle (D7) cannot be verified on this machine | Certain | Medium | The CI job that installs sympy **is** the verification; the local skip becomes loud. The cycle report must state plainly that the extended oracle was executed only in CI |
| RISK-006 | The `--print-budget` probe error (nodeCount 5 < budget 6) is easy to repeat when writing budget tests | Medium | Low | Budget fixtures must assert `nodeCount > budget` explicitly |

---

## 12. Requested approval

Approve (a) the reproduction verdicts in §2–§4, (b) decisions **D1–D8**, and (c) the Phase A–D
sequencing, which puts the five P0 contract obligations and their golden fixtures first.

On approval I will start with **Phase A1** (structural bindings) with **A5** (golden fixtures and the
runner test project) landing alongside it, so the wire contract is pinned as it changes.

**No production code, test, or document other than this addendum and the harness records under
`docs/goal-cycle-3/` has been modified in this session.** The working tree contains only
those additions; the five probe scripts and 60 recorded envelopes are the reproduction evidence.

---

## 13. Item 19 — `--omit-display` is **DECLINED** (recorded 2026-09-11, Cycle-3 round 30)

> Cycle 2 took this decision and Cycle 3's reproduction reconfirmed it. It is recorded here so it
> stops living as an implicit absence and becomes a decision with a reason, a revisit condition and
> a verification. **No flag was implemented and the runner was not changed.**

### 13.1 Decision

**DECLINED — `--omit-display` will not be implemented.** The runner keeps emitting the human
projection of every value alongside its structure, and keeps refusing the flag as an unknown
argument (§4 item 19, §7 item 3).

### 13.2 Reason — what the flag would strip, and where the protocol documents it

The flag's only coherent reading is "drop the display-formatting fields from the envelope". Those
fields are the result-level pair built in `Lovelace.Run/Runner.cs:203`
(`new ResultDto(result.Kind.ToString(), ValueFormatter.Format(result), ValueFormatter.FormatTyped(result), …)`)
and declared at `Lovelace.Run/RunProtocol.cs:27`
(`ResultDto(string Kind, string Display, string Typed, StructuredValueDto Structured)`) — i.e.
`result.display` and `result.typed` — plus the per-variable `variables[].display`
(`RunProtocol.cs:17`, populated at `Runner.cs:185`).

They are not padding; they are documented parts of the value the envelope carries:

* `docs/symbolics/dsh-protocol.md:107-108` — the protocol's own "A real solve envelope (abridged)"
  example shows both fields **on the result object itself**, next to `structured`:
  `"display": "SolveResult(status: Solved, ...)"` and `"typed": "SolveResult(...) (SolveResult)"`.
  This is the reference shape every consumer codes against, and the document states the envelope is
  **versioned** (`dsh-protocol.md:16`, invariant 4) — a shape change here is a protocol revision,
  not a host preference.
* `docs/symbolics/dsh-protocol.md:3-5` — the document's opening contract sentence: the envelope is
  "designed so that an agent never has to parse a display string to recover mathematical meaning".
  The display string is *defined by* that sentence as the thing that rides **alongside** structure;
  the structure is what the agent reads, and the display is what a human — or a log, or a diff in a
  golden fixture — reads. A flag that deletes it removes the human half of a two-audience contract
  to save bytes nobody has shown to be a bottleneck.
* The display fields are also what the runner's `--text` mode prints (`Runner.cs:311`/315) and what
  the recorded golden fixtures and Cycle-2 Studio round-trips compare.

The cost side of the ledger is empty: the envelope's dominant bytes are `structured` (recursively
projected) and `output`; `display`/`typed` are two short strings per statement. There is no backlog
item, probe or measurement showing envelope size is a problem — so the flag trades a documented
contract field for an unmeasured saving. That is the definition of a convenience flag, and §8's bar
(recorded in decision **D2**) is that protocol surface moves only by an intentional, documented
revision.

### 13.3 Revisit condition

This decision is revisited **only** if `docs/symbolics/dsh-protocol.md` is revised — as an explicit,
reviewed protocol revision that (a) removes `display`/`typed` from the documented envelope, (b)
states the replacement for the human projection (the `--text` summary, a `format` subcommand, or
nothing), and (c) states the compatibility story for existing consumers and the golden fixtures.
**Never** as a convenience flag, and never in a host-only change that leaves the protocol document
describing fields the runner no longer emits. Until such a revision exists, "the flag is refused"
is the correct, tested behaviour.

### 13.4 Verification — the refusal is observed, not assumed

`out/aot/Lovelace.Run.exe --omit-display`, executed 2026-09-11 (observed output follows verbatim;
stderr and stdout merged, exit code captured by the shell):

```text
Error: Unknown argument '--omit-display'.
Lovelace.Run — evaluate a Lovelace script and emit a JSON envelope.

Usage:
  Lovelace.Run --eval "<script>" [options]
  Lovelace.Run <file.ls> [options]
  Lovelace.Run --stdin [options]

Options:
  --eval <script>      evaluate the given script text
  --file <path>        read the script from a file
  --stdin              read the script from standard input
  --plot-dir <dir>     directory for plot() SVG output
  --plot-file <name>   filename for plot() SVG output (default: plot.svg)
  --omit-functions     omit the builtin registry from the envelope (agent loops)
  --omit-variables     omit the variables array from the envelope (agent loops)
  --print-budget <n>   abbreviate structured renderings beyond n nodes, reporting the truncation
  --cancel-after <ms>  cancel the evaluation after the given time, returning the partial result
  --json               emit JSON (default)
  --text               emit a human-readable summary
  --help, -h           show this help
```

```text
EXIT=2
```

Exit **2**, message `Unknown argument '--omit-display'.` — the structural usage refusal at
`Lovelace.Run/Runner.cs:125` (`return Usage(stderr, $"Unknown argument '{args[i]}'.")`), with the
usage text at `Runner.cs:345-346` listing only `--omit-functions` and `--omit-variables`. The flag
is therefore neither implemented nor silently ignored: it fails loudly, which is the behaviour §7
item 3 asks the cycle to preserve.

### 13.5 Cross-reference — the item-18 measurement

The quotable benchmark run that this round produced (item 18) is recorded in
[`benchmarks/symbench-longrun.md`](../../benchmarks/symbench-longrun.md): method, job kind, the rows
measured and skipped, per-row mean with its error bar and allocation bytes, and an explicit statement
of what is and is not defensible from it. It is linked here so the item-18 evidence and this item-19
decision read as one record.
