# Lovelace — A+ Symbolic Runtime Convergence, Cycle 2: Alignment Addendum

> **Status:** Alignment proposal (awaiting approval). Cycle 2 is a **reproduction + plan** gate:
> no production code, test, or docs file was modified while producing this document. Scratch
> artifacts live outside the repository (`%TEMP%\probe_*.ls`, `%TEMP%\aot-smoke`, `out/aot`).
>
> **Predecessor contracts are law.** `docs/symbolics/a-plus-convergence-alignment-plan.md` §C–§J
> (frozen types, solver status semantics, domain contract, DSH schema, rewrite failure contract,
> assumption semantics, host formatting, intentional breaks) are **not redefined** here. Every
> change proposed in §5 is **additive** or explicitly listed as a break that the frozen doc already
> sanctioned (§J).

---

## 0. Baseline (recorded before any change)

| Item | Value |
|---|---|
| Branch / HEAD | `main` @ `f37f1ce61631cba5638584349f0934ea4f209681` ("docs: DX-cycle documentation pass") |
| Working tree | **dirty** — Cycle 1 implementation present and uncommitted (20 modified, 5 untracked: 3 test files + the 2 new docs) |
| Solution | `LovelaceSharp.slnx` (34 projects; `symbench` **not** a member) |
| Build | `dotnet build LovelaceSharp.slnx -c Release` → **0 errors, 0 warnings** |
| RID/AOT build | `dotnet build Lovelace.Run -c Release -r win-x64 -t:Rebuild` → 0 errors, 51 warnings (pre-existing CS8767 in `Lovelace.Integer` etc., plus **CS0162 unreachable code at `Lovelace.Run/Program.cs:34`**) |
| Symbolics tests | `Lovelace.Symbolics.Tests` → **338 passed / 0 failed / 0 skipped** (17.7 s) |
| Suite tests | `Lovelace.Suite.Tests` → **438 passed / 0 failed / 0 skipped** (9.4 s) |
| All suites | **1725 passed / 0 failed / 0 skipped** — Abstractions 20, Array 19, Complex 83, Dsp 61, Integer 148, Knowledge 28, Natural 195, Representation 91, Studio 19, Suite 438, Symbolics 338, precbench 13, Real (Category!=Heavy) 272 |
| Native AOT publish | `dotnet publish Lovelace.Run -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot` → **succeeded**, `out/aot/Lovelace.Run.exe` = 5 449 728 bytes |
| Published-runner smoke | **5/5 scenarios pass on the native binary** (see below) |
| symbench | builds and runs outside the solution; **no baseline file anywhere in the repo**; smoke row `CanonicalParse_SharedDAG` = 343.0 µs / 854.19 KB allocated (`--job short`, 3 iterations) |
| Machine | AMD Ryzen 9 5900X, 12 physical / 24 logical cores; .NET SDK 10.0.103 / .NET 10.0.3 |

**AOT smoke evidence (native `out/aot/Lovelace.Run.exe`, not the JIT build):**

```text
solve:     ok=True status=Solved complete=true domain=Domain/complex nsol=2
partial:   status=Partial complete=false unrep=2
purity:    output=[hello from the script] value=2
jacobian:  kind=Array shape=2,2
error:     ok=False code=InvalidOperation cat=DomainError msg=solve(): currently supports domains real and complex; got integer.
protocol:  v=1 sym=#!lovelace-sym 1 mathir=2
```

Cycle 1 is confirmed present and green. Nothing in §7 below needs re-doing.

---

## 1. Reproduction method

Every verdict below was produced by one of three executable paths, never by source reading alone:

1. **Published/JIT runner** — `dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <probe>.ls --omit-functions`
   (real envelope, real parsing, real kernel).
2. **Symbol inspection** — `file:line` read of the current working tree.
3. **Read-only recon of hosts** — the REPL and Studio surfaces (no builds), cross-checked against
   live runner behaviour.

Verdicts: **Confirmed** · **Already fixed** · **Not applicable** · **New adjacent issue**.

---

## 2. Reproduction matrix — §2 (P0 completion blockers)

### §2.1 Eight broad `catch (Exception)` sites remain in symbolic reasoning

**Verdict: Confirmed (8/8 present).** Cycle 1 narrowed the rewriter's guard to
`catch (RuleEvaluationException)` (`Rewriting.cs:339`) — that one is genuinely fixed. The other
eight are unchanged; note the audit's `Solve.cs:635` has drifted to **`:1011`** after Cycle 1's solver work.

| # | Site | Encloses | Defect |
|---|---|---|---|
| 1 | `Lovelace.Symbolics/Calculus/Limits.cs:57` | direct-substitution probe | falls through to series on *any* failure |
| 2 | `Lovelace.Symbolics/Calculus/Limits.cs:66` | `SeriesLimit` | `return LimitResult.Uneval(ex.Message)` — **any** exception becomes a mathematical non-answer (§76) |
| 3 | `Lovelace.Symbolics/Calculus/Limits.cs:126` | leading-coefficient evaluation | any failure ⇒ `Uneval("leading coefficient is symbolic")` |
| 4 | `Lovelace.Symbolics/Evaluation.cs:499` | `NumOps.Compare` on a guard | unordered **and internal failure** both ⇒ `Unknown` |
| 5 | `Lovelace.Symbolics/Evaluation.cs:605` | plugin numeric evaluator fold | any failure silently degrades to a symbolic node |
| 6 | `Lovelace.Symbolics/Evaluation.cs:686` | approximate fold | any failure ⇒ `null` |
| 7 | `Lovelace.Symbolics/Calculus/Integrate.cs:104` | `Algebra.Expand` equivalence check | defect masked as "verification fell through" |
| 8 | `Lovelace.Symbolics/Solvers/Solve.cs:1011` | residual numeric verification | defect masked as "cannot verify: refuse" |

Live consequence of #2 (the worst of the eight) — an internal failure is presented as a limit verdict:

```text
limit_full(sin(x), x, inf)  ->  LimitResult(status: Unevaluated, exists: False, value: ,
                                diagnostics: coefficient does not evaluate at the point)
```

There is **no test** anywhere asserting that an *unexpected* exception propagates out of
limit/integration/evaluation/solve. (The rewriter has one — `RewriteProtocolTests`.)

### §2.2 Condition arrays still carry strings

**Verdict: Confirmed — and broader than described.** `SymbolicsPlugin.ConditionExprs`
(`SymbolicsPlugin.cs:671-701`) emits three distinct string classes into a machine array:

- `:674` `return new object?[] { "unsatisfiable" };` (the whole array collapses to one string)
- `:684` `"finite(" + sf.S.Name + ")"` and `:687` `"finite(" + PrettyPrint(ef.E) + ")"`
- `:690/:693/:696` `a.ToString()` fallback — **raw C# record syntax crosses the wire**:

```json
x = symbol("x", real); simplify_full(sqrt(x^2))
"conditions": { "kind": "Array", "shape": [1], "elements": [
    { "kind": "Text", "value": "SymbolDomainAssumption { S = x, D = Real }" } ] }
"steps"[0].fields[required_conditions] -> the same Text leaf
```

The same `ToString()` leak appears in user-facing contradiction errors (§2.14).

### §2.3 Studio exposes no structured results

**Verdict: Confirmed.** `Lovelace.Studio/Dtos.cs:35` `ValueResult(string Kind, string Display, string Typed)` —
no payload. Built at `EngineHost.cs:273-275`:

```csharp
new ValueResult(result.Kind.ToString(), engine.FormatValue(result), engine.FormatValueTyped(result))
```

- SolveResult/TransformResult/LimitResult fields: **not in any DTO**; they exist only inside the
  flattened display string produced by `ValueFormatter.FormatRecord` (`ValueFormatter.cs:61-62`).
  The record *type name* never crosses either — `Kind` is just `"Record"`.
- Canonical form: only on `/api/symbolic/inspect` (`EngineHost.cs:134`), never on evaluate.
- Provenance: `trace.Steps.Select(s => s.ToString())` → `string[]` (`EngineHost.cs:138`).
- MathIR: string only (`EngineHost.cs:126`). Per-step results: `engine.FormatValue` strings (`EngineHost.cs:282`).
- Studio **never sets** `UnicodeOutput` — grep over the project (C# **and** `wwwroot`): **0 hits**.
  It inherits the ASCII default (`Interpreter.cs:131`), so the engine's Unicode mode is unreachable from Studio.
- Studio does **not** duplicate `ToStructuredValue` (0 hits); reuse is blocked because
  `StructuredValueDto` is `internal` inside the `Lovelace.Run` executable's top-level
  `Program.cs:343-362` and `Lovelace.Studio.csproj` does not reference `Lovelace.Run`.
- UI reconstruction from display text (all in `wwwroot/app.js`): result content is
  `"= " + data.result.typed` (`:220`); variables are `v.display` (`:157`);
  the inspect panel picks its subject by **regex-splitting the editor buffer** (`:237-240`) and
  re-evaluates it server-side (`EngineHost.cs:112`) — re-running side effects on every finished run;
  `traceSteps.join(" | ")` and a stringified tree (`:242-274`).
- Server-side prose parsing is used for diagnostics: `IncrementalRunner.cs:407-411`
  `Regex.Match(ex.Message, @"at position (\d+)")`.

### §2.4 Cancellation is absent from the entire stack

**Verdict: Confirmed.** `CancellationToken` occurrences: **0** in `Lovelace.Symbolics`, **0** in
`Lovelace.Suite`, **0** in `Lovelace.Abstractions`. `SuiteEngine.EvaluateAsync(string, TextWriter?)`
(`SuiteEngine.cs:198`) has no token; `RegisterBuiltin` takes a bare
`Func<IReadOnlyList<object?>, object?>`; the Gröbner/simplify/root-isolation/differentiation/
integration/compilation kernels take no token. The only token in the tree is Studio's own
`RunState.Cts` for its incremental runner (`RunState.cs:13`), which cannot reach the engine.

### §2.5 Reentrancy is undefined

**Verdict: Confirmed.** `SuiteEngine._evaluationGate = new SemaphoreSlim(1, 1)` (`SuiteEngine.cs:187`);
`EvaluateAsync` awaits `WaitAsync()` (`:205`) with no owner tracking and no reentrancy check. A
builtin that calls `EvaluateAsync` on the same engine blocks forever: there is no diagnostic, no
timeout, and no documented contract. Secondary race: `_diagnostics.Clear()`, `ClearOperationTimings()`
and `_lastSource = source` (`:200-202`) run **before** the gate, so concurrent callers clobber each
other's diagnostics/timings even though the evaluation itself is serialised.

### §2.6 `ExprContext.WithAssumptions` is not flow-local

**Verdict: Confirmed.** `Context.cs:95-119`: the method saves `Assumptions` into a closure, assigns
the new set to the **shared** field, and restores on `Dispose`. Two overlapping scopes interleave and
the last `Dispose` wins. The XML doc on the same method claims "Composable and **flow-local**".
Only a single-scope test exists (`ConcurrencyTests.cs:99-115`); no two-concurrent-scopes test.

---

## 3. Reproduction matrix — §3 (P1 protocol & introspection)

| # | Item | Verdict | Evidence |
|---|---|---|---|
| 7 | `1/3` crosses as `{"kind":"Real","display":"0.(3)"}` | **Confirmed, with a second defect** | Live: `1/3` → `{"kind":"Real","value":"0.(3)","exact":true}`. There is **no Rational ValueKind** (`Value.cs:20-35`: Natural, Integer, Real, Boolean, Text, Vector, Function, Void, Array, Complex, Symbolic, Record, Domain). Worse: **`exact:true` is hardcoded for every Real** (`Program.cs:321-323`) — `evalf(1/3, 50)` returns 50 truncated digits stamped `exact:true`, and `sqrt(2)` returns 100 digits stamped `exact:true`. The flag currently means "carried in a Real" not "mathematically exact", and `type(1/2)` → `"Real"` |
| 8 | `inspect()` incomplete | **Confirmed** | Live `inspect(x^2+1)` → `type: Text "Symbolic"`, `domain: Text "complex"`, `exact`, `free_symbols: Text[]`, `node_count`. **No** `canonical`, `pretty`, `shape`, `rank`, element domains, or assumptions. `/api/symbolic/inspect` does emit canonical+pretty (`EngineHost.cs:134-138`), so the vocabularies really do diverge: `solve_full` emits a **Domain** value, `inspect` emits lowercased **Text** |
| 9 | `type()` naming inconsistent | **Confirmed (worse than reported)** | Live: `type(1)`→`Natural`, `type(complex)`→`complex`, `type(1.5)`→`Real`, `type([1,2])`→`Vector`, `type(symbol("x"))`→`Symbolic`, `type(1/2)`→`Real`, `type(symbol("x", real))`→`Symbolic`, `type(rational())`→`rational`. Three casings on one surface, and the declared domain of a symbol is invisible |
| 10 | MathIR structured API | **Confirmed (each sub-item)** | `compile_full(x^2+1,[x])` structured fields = `ir(Text), parameters(Text["x"]), result_type(Text "Integer"), target(Text "mathir"), mathir_version(Integer), exact(Boolean)`. **No** parameter domains/types, result domain, precision policy, optimization policy. `CompilationTarget` remains dead. `evalir_batch` **does** accept `[[1,2],[3,4]]`, but **without validation**: with a 2-parameter kernel, `[[1,2,5],[3,4,6]]` silently misaligns to rows `[1,2],[5,3],[4,6]` → `[3, 16, 25]`. The documented message `evalir_batch(): kernel expects N parameters per row; row R contains M.` does not exist — a ragged literal fails at the **parser** with `"Ragged nested list literal: every row must have the same shape."` and `[1,2,3]` fails with the old `"Values must be a multiple of 2 (the parameter count)."`. No serialize→deserialize→evaluate differential test exists |
| 11 | Rich-result field completion | **Confirmed** | `limit_full(1/x,x,0)` → `status, exists, value, left, right, diagnostics(Text)`; **no `conditions`, no `exactness`**, and a non-existent limit yields `value: {"kind":"Null"}` with no condition set. `integrate_full` → `status, expression, conditions, verified, diagnostics`; **no `method`/`verification_method`** even though the integrator verifies by differentiation. `optimize_full(f,[x])` → `original, optimized, estimated_cost_before, estimated_cost_after, shared_subtrees, horner_rewrites, target`; **no `transformations`, no `policy`** |
| 12 | Structured durations | **Confirmed** | Envelope carries only `"elapsed":"16.94 ms"` (a unit-scaled string from `engine.LastElapsedDisplay`). `SuiteEngine.OperationTimings` (`SuiteEngine.cs:129`) exists and is asserted by `SuiteEngineTests.cs:169-174`, but is **never emitted** by any host — grep of `Lovelace.Run`: 0 hits |
| 13 | Integrality reasoning | **Confirmed** | Live: `x = symbol("x", integer); assume(x > 1/2); assume(x < 1)` is **accepted** → `x in Integer; x > 1/2; x < 1`, an empty set. Adjacent discovery: the *same* constraint written `assume(1/2 < x)` is **rejected** with `"assume() accepts relations and their conjunctions/negations with constant bounds."` — the analyser is shape-sensitive (symbol-on-the-left only), so the gap is both a missed contradiction **and** an inconsistency |
| 14 | Error message quality | **Confirmed** | `diff(x, 3)` → `"Expected a symbolic symbol, got Natural."` — no function name, no argument index, no expected/actual names. `optimize_full(f, x)` (wrong arg type) → **`InternalError / InternalInvariantFailure`** `"Unable to cast object of type 'SymbolExpr' to type 'IReadOnlyList<object>'."` — a raw .NET cast message. Contradiction errors print raw records: `"Assumption SymbolRelationAssumption { S = x, Op = Le, Bound = Lovelace.Symbolics.RationalConstantExpr } contradicts…"`. `BuiltinDescriptor.Variadic` is **read once** (`HelpService.cs:176`, dead `...` branch) and **never set**; `MinArity` is set 4× and read once, only for signature text — runtime arity is a separate positional `RequireArity` (`Interpreter.cs:1103`), so descriptor arity can disagree with runtime arity unnoticed |
| 15 | Descriptor examples must be executable | **Confirmed** | No runner executes `BuiltinDescriptor.Examples`: the only reads are `HelpService.cs:135,139`, which print them. Live: `assume(x > 0 and x < 10)` (the advertised example) → `ParseError` `"Expected 'RParen' but found 'and'"`. README doctests do run (`UsageDocumentationTests`), but they assert with the **static** `ValueFormatter.FormatTyped` (`UsageDocumentationTests.cs:51`, `LanguageDocumentationTests.cs:72`), bypassing the engine format context entirely |
| 16 | REPL is untestable | **Confirmed** | `LineEditor.cs:50` `var key = System.Console.ReadKey(intercept: true);`; `ReplSession` and `LineEditor` are `sealed`, parameterless, no interface, no virtuals; all output via `System.Console.WriteLine`. **No `Lovelace.Console.Tests` project exists** and a repo-wide grep finds zero test references. (The engine's own `Output` writer is only for `print()`.) **Correction to the audit:** in the *current main tree* `set pretty unicode` (`ReplSession.cs:144`), `set pretty ascii` (`:151`), `set precision` (`:178-179`) and `set display` (`:194`) are **effective**, because results render through `_engine.FormatValueTyped` (`:232`, `:244`). Remaining gaps: exact/case-sensitive line matching, no clamp on `set precision`, and `Lovelace.Console/README.md:66` claiming `set display` also sets `Natural.DisplayDigits` (it does not) |
| 17 | Print budget | **Confirmed absent** | Repo-wide grep for `MaxDepth|MaxNodes|PrintBudget|Abbreviat`: **0 hits**. No abbreviation mode, no blow-up diagnostic, no partial result on print overflow |

**Stale rows in the frozen doc (must be annotated, not silently kept):**
`a-plus-convergence-alignment-plan.md` §B.5 rows for §45 ("`set pretty unicode` … **Confirmed broken**"),
§47, §61 ("no alias table") and §62 ("`funcs sol` → No category") describe the pre-Cycle-1 tree. The
alias table (`HelpService.cs:185-212`) and case-insensitive **substring** search (`:78-80`) now exist,
and the REPL honours Unicode/display. Those rows are **Already fixed**.

---

## 4. Reproduction matrix — §4 (P2) and §5 (regressions)

### §4 P2 items

| # | Item | Verdict | Evidence |
|---|---|---|---|
| 18 | Benchmarks | **Confirmed — the largest untouched gate** | `symbench` has **no baseline file** (only `docs/architecture/typed-array-benchmark-baseline.md` exists, unrelated); it is **not in `LovelaceSharp.slnx`** (only `dspbench`, `precbench`); **not in `ci.yml`**; and `Makefile test` omits Symbolics, Array, Complex, Dsp, Abstractions, Real and precbench. It builds and runs standalone (smoke row recorded in §0). None of the required rows exist: construction, canonicalization, `simplify` safe, `simplify` full (trace off/on), `diff`, `factor`, `solve`, pretty/canonical print, `RecordValue` construction, member access, JSON structured serialization, MathIR scalar/batch, help catalog generation, record field lookup |
| 19 | Assumption cost | **Confirmed** | `AssumptionSet.Add` (`Assumptions.cs:91-104`): `_atoms.Add(a).Sort(static (x, y) => string.CompareOrdinal(x.ToString(), y.ToString()))` then a full `Ask` probe — **every insertion re-renders every atom to a string and re-sorts**, so building an n-atom set is O(n²) renderings. `Ask` (`:149-…`) is `_atoms.Contains(predicate)` (O(n)) plus recursive `Or(Ask(...), Ask(...), …)` expansion with **no memoisation**, although its doc comment at `:144` says "memoized per set by the caller". No scaling test |
| 20 | Verification breadth | **Confirmed** | `FalsificationTests.cs:46-49` `catch (Exception) { continue; }` — a rule that is wrong *by throwing* is never falsified. Sampling is a fixed 13-point rational set (`:17-22`) in interior/near-zero/large magnitude only: **no near-pole, near-branch-cut, near-discontinuity or assumption-boundary sampling**, and no randomized sampling. The rules are hand-written per test rather than driven from the registry. SymPy oracle exists but covers only `diff` and one solve corpus, and **silently returns** when the probe fails (`DifferentialOracleTests.cs:64-65`) — on this machine `python3` is the Microsoft Store alias and sympy is **not installed**, so those tests pass without comparing anything. No metamorphic set (`d/dx ∫f = f`, substitute-solution, `A·A⁻¹ = I`, `evalIR(compiled) = eval(symbolic)`, `expand(factor(p)) = p`) |
| 21 | Unsatisfiable propagation into Piecewise | **Confirmed** | `Evaluation.cs:410-422`: branches are decided by numeric guards only; `Tristate.Unknown` → `throw new EvaluationException("Piecewise guard cannot be decided numerically.")` — an undecidable guard is a **hard error**, not a pruned/reported branch. `Assumptions.cs:528-534` computes a domain for Piecewise but never prunes branches with an unsatisfiable assumption set |
| 22 | Record schema registry | **Confirmed absent** | No schema/registry type in `Lovelace.Suite`; the only hits for "schema" are two doc comments (`HelpService.cs:8`, `CoreBuiltinMetadata.cs:9`) referring to a DSH tool-schema generator that **does not exist** (repo-wide grep `ToolSchema`: 0 hits). No deep structural-equality helper exists (`StructuralEqual|AssertDeepEqual|RecordEquals`: 0 hits) |
| 23 | Rule→Lean proof bridge | **Confirmed absent** | No `RuleId → theorem` map anywhere. Only incidental mentions of "theorem" in maths comments (`Solve.cs:1082` Sturm, `Polynomial.cs:409` rational-root, `Groebner.cs:58` elimination) |
| 24 | LaTeX printer | **Confirmed absent** | Repo-wide grep `latex|Latex|LaTeX` in `*.cs`: **0 hits**. Recommend keeping it deferred unless items 1–3 and 18 are all closed |
| 25 | `--omit-variables` / `--omit-display` | **Confirmed absent** | Live: both are rejected as `Unknown argument '<opt>'.` on stderr (exit 2). `--omit-functions` and `--text` do exist and work |
| 26 | Sub-`Ask` hot paths | **Not measurable yet** | No benchmark row exists to measure against, so no restructuring is proposed this cycle. The doc/implementation mismatch in item 19 is fixed regardless |

### §5 Do-not-regress — verified live this session

| Required behaviour | Verified | Result |
|---|---|---|
| `solve_full(x^4-x^2-1==0,x)` → Partial + `complete:false` + `unrepresented_count:2` | ✔ live + AOT | `Partial / False / 2`; reason `complex algebraic roots not supported (RootOf is real-only in v1).` |
| `solve((x+1)^2-4==0,x)` → `[-3, 1]` | ✔ live | `[-3, 1]` |
| `solve((x+1)^3==8,x)` → 3 roots | ✔ live | 3 represented roots |
| `solve(x^2-4==0,x)` → ascending `[-2, 2]` | ✔ live | `[-2, 2]` |
| `solve(2*x==1,x,integer)` rejected with the documented message | ✔ live + AOT | `solve(): currently supports domains real and complex; got integer.` |
| `solve(x^4+1==0,x)` → Unevaluated | ✔ live | `Unevaluated`, `unrepresented_count: 4` |
| `solve_full((x^2-1)/(x^2-1)==0,x).status` → NoSolutions | ✔ live | `NoSolutions` (note: `complete:false`, `completeness:Unknown` — see N9) |
| multiplicity preserved (`x²-2x+1`) | ✔ live | `Solution(value: 1, multiplicity: 2)` |
| `solve_full(x^2-4==0,x).domain` is a `Domain` value | ✔ live + AOT | `{"kind":"Domain","domain":"complex"}` |
| `assume(x>=5); simplify(x<5)` → False | ✔ live | `False`; `simplify(x>4)` → `True` |
| 324-query bound matrix + 324 pair-contradiction matrix exact | ✔ suite | `AssumptionSoundnessTests.BoundReasoning_MatchesReferenceTruthTable` + pair matrix pass in the 338 |
| throwing rewrite precondition **propagates** | ✔ suite + code | `Rewriting.cs:339` narrow catch; `RewriteProtocolTests` passes |
| stdout carries only the envelope, `print()` captured into `output` | ✔ live + AOT | `output:["hello from the script"]`, result `2` |
| protocol versions present | ✔ live + AOT | `protocolVersion:1`, `symbolicFormatVersion:"#!lovelace-sym 1"`, `mathIrVersion:2` |
| `jacobian(...)` flat row-major + `shape [2,2]` | ✔ live + AOT | `{"kind":"Array","shape":[2,2]}`, 4 elements |
| every one of the 117 builtins has help metadata | ✔ suite + live | `HelpMetadataCompletenessTests` passes; registry enumerates 117 |
| README doctests + `DocsSyncTests` pass | ✔ suite | inside the 338/438 |

**Not reproducible from the runner surface (needs a C#-level probe):** the audit's
`Im(real x) ∈ [π,2π] → False` case — the script language cannot spell `in` (`ParseError`). Its
replacement test lives in the suite and passes; Cycle 2 must keep it there.

---

## 5. New adjacent issues found this cycle

| ID | Severity | Issue | Evidence |
|---|---|---|---|
| **N1** | **P0 (wrong mathematics)** | **Unary minus binds tighter than exponentiation.** `-2^2` → **4** (should be −4); `subs(-x^2, x, 3)` → **9** while `subs(0-x^2, x, 3)` → **−9**; canonical of `-x^2` is `(pow (mul (rat -1 1) (sym x)) (rat 2 1))` = `(−x)²`. Every CAS (SymPy, Mathematica, Maple, Maxima) parses `-x^2` as `−(x²)`. This also silently changes `exp(-x^2)` to `exp((−x)²)` — visible in the `integrate_full` probe | live runner |
| **N2** | **P0 (protocol)** | **An argument-type error surfaces as `InternalInvariantFailure`.** `optimize_full(f, x)` (parameter list given as a symbol) throws `InvalidCastException`; the envelope reports `code:"InternalError", category:"InternalInvariantFailure", recoverable:false` with a raw .NET cast message. Descriptor-driven validation would classify it as a `TypeMismatch` naming argument 2 | live runner |
| **N3** | P1 | **`TransformResult.changed` is `true` when nothing changed.** `simplify_full(x^2+1)` → `original: x^2 + 1, expression: x^2 + 1, changed: True, steps: []`. Built as `!r.Expression.Equals(e)` (`SymbolicsPlugin.cs:225`) — the comparison uses the *argument*, not `r.Original`, and disagrees with the identical printed forms | live runner |
| **N4** | **P0 (silent misalignment)** | **`evalir_batch` accepts row-oriented input but never validates the row width.** A 2-parameter kernel given `[[1,2,5],[3,4,6]]` returns `[3, 16, 25]` — three results from data the caller meant as two rows. Item 10's documented error message must accompany a real check | live runner |
| **N5** | P1 | **`exact:true` is hardcoded for every Real and every Complex** (`Program.cs:315,321-323`), so `evalf(1/3,50)` and `evalf(sqrt(2),40)` — truncations — advertise exactness. An agent that trusts `exact` is misled | live runner |
| **N6** | P1 | **`SolveResult.diagnostics` and the other rich records ship `diagnostics` as `Text`,** not as the `Diagnostic[]` the frozen §C contract declares (`SymbolicsPlugin.cs:387`). `SolveResult` also carries `completeness` as a **Text** next to a Boolean `complete` | live runner |
| **N7** | P1 | **`solve_system_full` still emits `assignment` as Text** `["x = 2", "y = -1"]` — the `Binding` migration listed in the frozen §J break table was not performed | live runner |
| **N8** | P2 | **`SolutionFamily.parameter` is Text `"k"`** while `parameter_domain` is now correctly a `Domain` — half of §36 landed | live runner |
| **N9** | P2 | **`NoSolutions` reported with `complete:false, completeness:Unknown`.** For a *provably empty* solution set the completeness contract should not read "Unknown" | live runner |
| **N10** | P1 | **`assume()` is shape-sensitive:** `assume(1/2 < x)` is rejected, `assume(x > 1/2)` is accepted. The bound lattice and the surface accept different spellings of the same fact | live runner |
| **N11** | P2 | **`abs` is numeric-only in the language:** `simplify(abs(x))` → `"abs() is not supported for values of kind 'Symbolic'."` — yet the rewriter *produces* `abs(x)` as a result. A value the engine creates cannot be fed back in | live runner |
| **N12** | P2 | **Doctests assert with the static formatter** (`UsageDocumentationTests.cs:51`, `LanguageDocumentationTests.cs:72`), so engine-format changes are invisible to the doctest gate; and the SymPy oracle silently skips when Python is absent (it is absent here) | recon + live |
| **N13** | P3 | **`warning CS0162: Unreachable code detected` at `Lovelace.Run/Program.cs:34`** — the entry statement is `return await ProgramMain(args);` at line 9, so the const declarations after it are unreachable (harmless, but it is the only warning the AOT job emits that the incremental build hides) | RID rebuild |
| **N14** | P3 | **Record member introspection reports `Available members: type, members`** for `Inspection`, i.e. the record's member surface differs from the `RecordField` names it actually carries | live runner |
| **N15** | P2 | **`OperationTimings` is collected but never surfaced** (also item 12); `_diagnostics/_lastSource/OperationTimings` are reset **outside** the evaluation gate (`SuiteEngine.cs:200-202`), so concurrent callers clobber each other's diagnostics | code |

---

## 6. Contract additions proposed for Cycle 2 (additive; §C–§J unchanged)

All new types are immutable records, AOT-safe, no reflection, and are **extensions** of the frozen
contracts rather than redefinitions.

| ID | Addition | Replaces |
|---|---|---|
| **A1** | `ConditionLeaf` payload union: `Finite(Expr)`, `DomainPredicate(Symbol, Domain)`, `PropertyPredicate(Expr, Predicate)`, and the sentinel `UnsatisfiableConditions` — serialized structurally, rendered to text **only** by the pretty printer | `"unsatisfiable"`, `"finite(x)"`, and every `a.ToString()` fallback in `ConditionExprs` |
| **A2** | Exact-rational payload form on the projection: `{"kind":"Rational","numerator":…,"denominator":…,"exact":true}` **plus** an honest `exact` flag on `Real`/`Complex` (approximation ⇒ `exact:false`). No new `ValueKind` is strictly required — see §8 decision D1 | misleading `Real`+`exact:true` for `1/3`, `evalf`, `sqrt` |
| **A3** | `CancellationToken` on `EvaluateAsync`, on the builtin registration contract, and on the long-running kernels; cancellation returns `BudgetExceeded`/`Cancelled` **with the partial result** | silent absence |
| **A4** | Explicit `ReentrancyNotSupported` diagnostic (code + category) instead of blocking on the gate; diagnostics/timings reset moved **inside** the gate | deadlock |
| **A5** | `AsyncLocal`-based `WithAssumptions` (or an explicit context parameter) + a two-concurrent-scopes test | the shared-field save/restore |
| **A6** | `PrintBudget { MaxDepth, MaxNodes, Abbreviate }` with a structured blow-up diagnostic `{reason, nodeCount, budget, partialResult}`; full canonical output stays explicitly requestable | unbounded printing |
| **A7** | `{value, unit}` alongside `elapsed`, plus the per-statement `OperationTimings` in the envelope | unit-scaled string only |
| **A8** | `ParameterInfo(Name, Domain)`, `ResultDomain`, `PrecisionPolicy`, `OptimizationPolicy` on `compile_full`; row-oriented `evalir_batch` **validated** against the parameter count with the documented message | Text parameter names, no policies, silent misalignment |
| **A9** | `LimitResult.conditions` + structured one-sided values; `IntegrationResult.method`/`verification_method`; `OptimizationResult.transformations`/`policy` | missing fields |
| **A10** | `Diagnostic[]` on every rich record (replacing `Text`), with record-syntax rendering removed from **all** messages | `diagnostics: Text` + raw C# record dumps |
| **A11** | The structured projection lifted into `Lovelace.Suite` as a public, AOT-safe serializer consumed by **both** `Lovelace.Run` and `Lovelace.Studio` | the `internal` copy inside the Run executable |
| **A12** | Descriptor-driven arity/type validation (`Variadic`, `MinArity`, parameter types) producing `diff(): argument 2 must be a symbolic variable; got Integer.` | duplicated positional checks + cast exceptions |
| **A13** | Doctest runner over `BuiltinDescriptor.Examples`, executing them through the **engine** formatting path with the documented prelude contract (`x = symbol("x")` etc.) | unexecuted examples |

**Not proposed:** redefining `SolveResult`/`Solution`/`Domain`/status taxonomies, the DSH envelope
shape, or the rewrite applicability protocol. Items 24 (LaTeX) and 25 (`--omit-variables`/
`--omit-display`) stay **deferred**: LaTeX adds a third printer surface, and the omit flags must not
fragment the single-sourced protocol.

---

## 7. Explicitly already fixed — do not redo

1. Rewriter applicability protocol, `RuleApplicability`, declared conditions, `RuleEvaluationException` propagation, stable rule ids (`Rewriting.cs`, `RewriteProtocolTests`).
2. Solver completeness/status/domain work: `Partial`/`NoSolutions`/`Unevaluated` derivation, domain rejection for integer/rational, multiplicity, per-solution conditions, ascending real ordering, families with a `Domain` parameter domain.
3. Assumption soundness: interval-bound fix, property↔relation contradictions, domain-lattice conflicts, relation folding to Booleans through `simplify`, 324-query + 324-pair matrices.
4. Structured protocol: envelope versions, stdout purity, `output` capture, Domain values, Array `shape`, Symbolic canonical/pretty/domain/exact/nodeCount/freeSymbols, error `code`+`category`+`recoverable`, `Unwrap(Value.Void) → null`.
5. Help metadata: all 117 builtins documented, `HelpMetadataCompletenessTests`, alias table, substring `funcs` search.
6. CI: AOT job publishing `Lovelace.Run` + executing it; `--text` emits text.
7. REPL: `set pretty unicode|ascii`, `set precision`, `set display` **are** honoured through the engine formatter (the frozen doc's §45/§47 rows are stale).

---

## 8. Decisions requested before implementation

| ID | Decision | Recommendation |
|---|---|---|
| **D1** | Rational representation: add a `Rational` **ValueKind** (a payload-vocabulary change; touches `ModusHost`, `PayloadMap`, every host switch) **or** keep the vocabulary and add `{numerator, denominator}` to the structured projection of exact rationals | **Keep the vocabulary, add the fields.** It preserves the frozen §C Modus payload set, is AOT-trivial, and still lets an agent recover exactness without parsing `0.(3)`. The frozen doc's §F already anticipates `{"kind":"Rational"}` as a *wire* form, so this is compatible either way |
| **D2** | N1 (unary minus precedence) is a **mathematical bug outside the brief's item list**. Fixing it changes expression canonical forms for existing inputs | **Fix it**, with a dedicated regression test, and document the canonical-form change. A CAS that evaluates `-2^2` as 4 is not A+ regardless of any other item |
| **D3** | Item 18 requires recording a baseline **before** touching hot paths, which serialises with items 1–3 | Baselines first (a single `--job short` sweep, artifacts committed under `benchmarks/`), then P0, then the new rows |
| **D4** | SymPy oracle cannot be exercised on this machine (no Python/sympy) | Extend the oracle **and** add a local-run script + a CI job that installs sympy, so the extension is actually verified somewhere; keep the silent-skip behaviour only when CI explicitly declares the oracle optional |

---

## 9. Sequencing (P0 → P1 → P2), and what "done" means

```text
Phase A  Baseline + Benchmarks            (D3)  symbench in slnx, baseline file, new rows
Phase B  P0: typed catches (§2.1) + condition leaves (§2.2) + N1 precedence + N2/N4 validation
Phase C  P0: shared structured serializer (§2.3/A11) -> Studio panels + Unicode/format context
Phase D  P0: cancellation (§2.4) + reentrancy (§2.5) + AsyncLocal assumptions (§2.6)
Phase E  P1: rational/exactness (7), inspect (8), type() (9), MathIR (10), rich records (11),
              durations (12), integrality (13), error quality (14), doctests (15), REPL seam (16),
              print budget (17)
Phase F  P2: verification breadth (20) + Piecewise propagation (21) + schema registry (22)
              + proof-bridge metadata (23)
Phase G  Adversarial audit (§143) on the published binaries
```

**Gate discipline.** No acceptance claim without: `dotnet build LovelaceSharp.slnx -c Release` clean,
all 1725 tests still passing **plus** the new ones, a fresh Native AOT publish, the published-runner
smoke re-executed, the benchmark delta table against the Phase-A baseline, and every item in the
cycle brief's §6 acceptance list either closed or explicitly reported as still below the bar.

---

## 10. Risks

| Risk | Mitigation |
|---|---|
| The frozen alignment doc's reproduction matrix has stale rows (§45/§47/§61/§62) | Annotate those rows in place as part of this cycle; the *contracts* (§C–§J) stay untouched |
| Fixing N1 changes canonical forms and may move benchmark numbers | Land it early (Phase B), before the baseline-sensitive rows in Phase A are re-measured; dedicated regression tests |
| Cancellation + `AsyncLocal` assumptions touch every kernel | Land as one phase with the concurrency tests first; keep `EvaluateAsync` overloads source-compatible |
| Studio work is front-end heavy and cannot be verified by the existing suites | Verify through the shared serializer's tests, the `/api/evaluate` DTO golden fixture, and a Studio-level test asserting `structured` is non-null for every value kind |
| AOT publish on this machine takes minutes and must be re-run per phase | Batch AOT verification at phase boundaries, not per commit |

---

## 11. Requested approval

Approve (a) the plan above, (b) decisions **D1–D4**, and (c) the intent to treat item 18 (benchmarks)
as a **required deliverable** rather than a stretch goal. On approval I will start with Phase A
(baseline + benchmark harness) and Phase B (the three true completion blockers: typed catches,
condition leaves, and the N1 precedence defect).

**No production code, test, or document other than this addendum has been modified in this session.**
