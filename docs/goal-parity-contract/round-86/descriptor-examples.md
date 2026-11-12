# Round 86 — descriptor-example family: six rows already closed, nine weak mappings found

**Status: classification complete; no production change.** The six rows the original family named
(DescriptorExampleTests.Example_Executes #21, #22, #53, #86, #107, #108) were already ported in
round 52 and their examples match the oracle exactly today (§1). The follow-up sweep of the
**whole** published corpus found **nine other examples** that diverge; every one of their census
rows is also already ported (§3). Therefore the deferred-row fix list is empty (§2): under the
redirect, no production source was changed and no test expectation was touched.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll and
rust/target/release/lovelace-run.exe, with the differential's normalisation
(revision, elapsed, elapsedTime, timings replaced at any depth).

## 1. The six original rows — already closed (short note)

The corpus index of every row below was re-enumerated from the C# test's own
DescriptorExampleTests.Corpus() logic (141 examples today; see §6). Each example was run with the
documented prelude x = symbol("x"); y = symbol("y"); t = symbol("t"). All six envelopes are
identical to the oracle after normalisation; the oracle-observed typed results are:

| Row | Function | Published example | Oracle-observed typed result | Fresh probe |
|---|---|---|---|---|
| #21 | cancel_full | cancel_full(x/x) | CancelResult(status: Conditional, original: x/x, expression: 1, changed: True, conditions: [x != 0]) (CancelResult) | MATCH |
| #22 | cancel_full | cancel_full((x^2 - 1)/(x - 1)) | CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x + 1, changed: True, conditions: [x - 1 != 0]) (CancelResult) | MATCH |
| #53 | filter | filter([1], [1], [1, 2, 3]) | [1, 2, 3] (Vector) | MATCH |
| #86 | ndims | ndims([[1, 2], [3, 4]]) | 2 (Natural) | MATCH |
| #107 | scale | scale([1, 2, 3], 2) | [2, 4, 6] (Vector) | MATCH |
| #108 | scale | scale([1, 2, 3], 0.5) | [0.5, 1, 1.5] (Vector) | MATCH |

All six are ported in parity/manifest.toml to
lovelace-lang::dispatcher_guard_round51::the_four_gap_builtins_run_their_published_examples, which
asserts exactly the six typed strings above. The round-50 divergence (exit 1, unknown function, no
dispatcher arm) was removed by the round-51 arms; round 52 mapped the rows. No production fix is
needed today.

## 2. Deferred-row divergences fixed — 0

None of the nine diverging examples below belongs to a deferred census row: every one is
disposition = "ported" in parity/manifest.toml (§3 table), and the curated
parity/manifest-map.json also has all 142 DescriptorExampleTests rows ported. There was therefore
no deferred-row production cause to fix under the redirect, no test expectation was changed, and
no new test was added. **Fixed ids: none. Tests added: none.**

## 3. Ported-row divergences found — 9

All nine are theory rows of
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes; the #N suffix
is the census case index. The nine full ids are:
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#4, #10, #24,
#25, #46, #89, #94, #97 and #98.

| # | Example | Disposition | Mapped Rust test | First differing field | Oracle | Rust |
|---:|---|---|---|---|---|---|
| 4 | assume(x > 0); and(x > 0, x < 1) | ported | lovelace-lang::suite_parity_batch::descriptor_example_004_and | $.result.structured.canonical | (and (lt (sym x) (rat 1 1)) (gt (sym x) (rat 0 1))) | (fn and (gt (sym x) (rat 0 1)) (lt (sym x) (rat 1 1))) |
| 10 | assume(and(x > 0, x < 10)) | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.result | present Symbolic, canonical (and (lt (sym x) (rat 10 1)) (gt (sym x) (rat 0 1))) | absent |
| 24 | collect(x*y + x + y + 1, x) | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.result.display | 1 + y + x*(y + 1) | x*(y + 1) + y + 1 |
| 25 | compile(x^2 + 1, [x]) | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.result.kind | Text (the IR source) | Record (CompilationResult) |
| 46 | k = compile(x^2 + 1, [x]); evalir_batch(k, [1, 2, 3], 40) | ported | lovelace-lang::suite_parity_batch::descriptor_example_046_evalir_batch | $.variables[1].kind (the k binding) | Text (the IR source) | Record (CompilationResult) |
| 89 | not(x > 0) | ported | lovelace-lang::suite_parity_batch::descriptor_example_089_not | $.result.structured.canonical | (not (gt (sym x) (rat 0 1))) | (fn not (gt (sym x) (rat 0 1))) |
| 94 | or(x > 0, x < -1) | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.result.display | (x < -1 or x > 0) | x < -1 or x > 0 |
| 97 | plot([1, 2, 3]) | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.artifacts[0].bytes | 14636 | 326 |
| 98 | plot([1, 2, 3], [1, 4, 9], "title") | ported | lovelace-lang::suite_engine_batch::descriptor_examples_execute | $.artifacts[0].bytes | 14595 | 331 |

Exit code and ok are 0/true on both sides for all nine; every divergence is a result-payload
difference, not a refusal. The exact full field sets follow.

### 3.1 #4 — and canonical form

assume(x > 0); and(x > 0, x < 1)

- $.result.structured.canonical and $.variables[0].structured.canonical:
  - oracle (and (lt (sym x) (rat 1 1)) (gt (sym x) (rat 0 1)))
  - Rust   (fn and (gt (sym x) (rat 0 1)) (lt (sym x) (rat 1 1)))
- display, pretty and typed agree (x < 1 and x > 0, x < 1 and x > 0 (Symbolic)), so the divergence
  is only the canonical rendering (and the canonical operand order).

### 3.2 #10 — assume of a conjunction drops the result

assume(and(x > 0, x < 10))

- $.result is absent in Rust; the oracle publishes
  {"kind":"Symbolic","display":"x < 10 and x > 0","typed":"x < 10 and x > 0 (Symbolic)",...,"canonical":"(and (lt (sym x) (rat 10 1)) (gt (sym x) (rat 0 1)))","nodeCount":7,...}.
- $.variables.length: oracle 4 (_, t, x, y); Rust 3 (t, x, y). The missing entry is the _
  result variable.
- Both exit 0 with ok: true.

### 3.3 #24 — collect leaves the constant term nested

collect(x*y + x + y + 1, x)

- $.result.display and $.result.typed:
  - oracle 1 + y + x*(y + 1) and 1 + y + x*(y + 1) (Symbolic)
  - Rust   x*(y + 1) + y + 1 and x*(y + 1) + y + 1 (Symbolic)
- $.result.structured.canonical (same under $.variables[0].structured):
  - oracle (add (rat 1 1) (sym y) (mul (sym x) (add (rat 1 1) (sym y))))
  - Rust   (add (mul (sym x) (add (rat 1 1) (sym y))) (add (rat 1 1) (sym y)))
- $.result.structured.nodeCount: oracle 8, Rust 9.

### 3.4 #25 — compile publishes the record instead of the IR text

compile(x^2 + 1, [x])

- $.result.kind: oracle Text, Rust Record.
- $.result.structured: oracle {"kind":"Text","value":"<IR>"}; Rust a CompilationResult record
  with fields ir, parameters, result_domain, result_type, precision_policy, optimization_policy,
  target, mathir_version, exact, diagnostics.
- $.result.typed and $.variables[0].display follow the same shape: the oracle is the bare IR
  text, Rust is CompilationResult(ir: ..., parameters: ..., result_type: Integer, ...)
  (CompilationResult).
- The IR text itself is identical on both sides:

    #!mathir 2
    param x
    const (rat 1 1)
    const (rat 2 1)
    Constant 0 0
    Parameter 0 0
    Constant 0 1
    PowInt 0 1,2
    Add 0 0,3

### 3.5 #46 — the k binding of compile is a record instead of text

k = compile(x^2 + 1, [x]); evalir_batch(k, [1, 2, 3], 40)

- $.result matches: [2, 5, 10] (Vector) on both sides.
- $.variables[1] (name k): kind oracle Text, Rust Record; display oracle is the IR text above,
  Rust is the same CompilationResult(...) rendering as #25; structured.value vs
  structured.fields/structured.type follow from the kind. This is the same production cause as #25.

### 3.6 #89 — not canonical form

not(x > 0)

- $.result.structured.canonical and $.variables[0].structured.canonical:
  - oracle (not (gt (sym x) (rat 0 1)))
  - Rust   (fn not (gt (sym x) (rat 0 1)))
- display/typed agree (not x > 0, not x > 0 (Symbolic)).

### 3.7 #94 — or display parentheses and canonical form

or(x > 0, x < -1)

- $.result.display, $.result.typed, $.result.structured.pretty and the same fields under
  $.variables[0]:
  - oracle (x < -1 or x > 0) / (x < -1 or x > 0) (Symbolic)
  - Rust   x < -1 or x > 0 / x < -1 or x > 0 (Symbolic)
- $.result.structured.canonical:
  - oracle (or (lt (sym x) (rat -1 1)) (gt (sym x) (rat 0 1)))
  - Rust   (fn or (gt (sym x) (rat 0 1)) (lt (sym x) (rat -1 1)))

### 3.8 #97 — plot([1, 2, 3]) SVG renderer

- With the same --plot-dir <dir> --plot-file plot.svg on both runners (the C# doctest sets
  PlotOutputDirectory and PlotFileName the same way), the returned path is identical and the
  remaining divergence is:
  - $.artifacts[0].bytes: oracle 14636, Rust 326.
  - $.plot.svg: oracle an 800x600 chart (svg width="800" height="600", white rect, frame, x/y
    axis lines, numeric tick labels, and a ~700-point red polyline); Rust a 640x400 minimal
    svg (width="640" height="400") with a white rect and a 3-point blue polyline.
- The default plot directory also differs (oracle <cwd>/plot.svg, Rust
  <cwd>/lovelace-plots/plot.svg), so $.result.display, $.artifacts[0].path and $.plot.path differ
  unless --plot-dir is passed to both. That path part is invocation, not renderer, and is not
  counted as a content divergence under the doctest's configured engine.

### 3.9 #98 — plot([1, 2, 3], [1, 4, 9], "title") SVG renderer

- Same shape as #97 with the title rendered in both:
  - $.artifacts[0].bytes: oracle 14595, Rust 331.
  - $.plot.svg: oracle the 800x600 axis/tick chart with a title text element; Rust the 640x400
    minimal chart with the same title text.
- The default-path caveat from #97 applies identically; with --plot-dir aligned the paths are
  equal and only bytes/SVG content differ.

## 4. Why these are weak mappings

The C# row body (DescriptorExampleTests.cs:102-136) only requires the example to execute
(EvaluateAsync does not assert the result); it does not assert the returned value. A row can
therefore be legitimately ported by a Rust test that executes the same snippet even when the two
runners publish different results. The nine mappings split into three grades:

- Execution-only (6 rows): #10, #24, #25, #94, #97, #98 map to
  lovelace-lang::suite_engine_batch::descriptor_examples_execute, which asserts only that each
  script evaluates. Every divergence above is invisible to that test.
- Display-only (2 rows): #4 maps to
  lovelace-lang::suite_parity_batch::descriptor_example_004_and, which asserts
  x < 1 and x > 0 (Symbolic) but not the canonical text; #89 maps to
  descriptor_example_089_not, which asserts not x > 0 (Symbolic) but not the canonical text.
- Result-only (1 row): #46 maps to
  lovelace-lang::suite_parity_batch::descriptor_example_046_evalir_batch, which asserts the batch
  result [2, 5, 10] (Vector) but not the k binding that carries the compile result.

The same pattern also appears on the ported non-descriptor row
RelationsAndFamiliesTests/Assume_ConjunctionAndNegation_ThroughEngine#0, whose mapped test asserts
the display x < 1 and x > 0 while the canonical and form differs; it is outside the nine-example
sweep and is recorded here only as supporting evidence that the and/or/not canonical gap is not
confined to one mapping.

## 5. Root causes (named, not fixed under the redirect)

1. **and / or / not are Expr::Fun nodes in Rust.** The dispatcher builds
   Sym::Fun("and"|"or"|"not", args) and Expr::canonical falls through to the generic
   (fn NAME ARGS) arm (rust/lovelace-symbolics/src/lib.rs:4242-4319), while the reference has
   dedicated And/Or/Not nodes whose canonical tags are (and ...), (or ...), (not ...) and whose
   canonicaliser orders the operands ((lt ...) before (gt ...)). The or pretty printer also omits
   the enclosing parentheses at top level that the reference emits. This is the cause of #4, #89
   and #94.
2. **assume(and(...)) returns Value::Void.** The assume dispatcher arm special-cases a
   Sym::Fun("and", parts), asserts each conjunct, then returns Value::Void
   (rust/lovelace-lang/src/builtins.rs:2992-3001). The reference returns the assumption expression
   as a Symbolic value, which is why #10 has no $.result and no _ variable in Rust.
3. **collect builds Expr::Add directly.** lovelace_symbolics::collect assembles Expr::Add(terms)
   rather than folding through the canonical Expr::add
   (rust/lovelace-symbolics/src/lib.rs:2708-2727), so a degree-0 coefficient that is itself a sum
   (y + 1) stays nested. The reference flattens it into the outer n-ary Add, giving
   (add 1 y (mul x (add 1 y))) and nodeCount 8 instead of Rust's 9. This is #24.
4. **compile and compile_full share one record-producing arm.** The arm "compile" | "compile_full"
   always returns the CompilationResult record
   (rust/lovelace-lang/src/builtins.rs:1703-1786), even though the Rust registry itself declares
   compile with return_kind Text and compile_full with return_kind CompilationResult
   (rust/lovelace-lang/src/registry.rs:331-359). The reference declares compile with return kind
   Text (Lovelace.MathIR/Evaluator.cs:368) and publishes the IR source; only compile_full publishes
   the record. This is #25 and #46.
5. **The Rust SVG plot renderer is a minimal 640x400 polyline.** The oracle's SvgPlotRenderer
   emits an 800x600 chart with frame, axes, tick labels and the full sampled polyline; Rust emits
   a 640x400 rect plus a 3-point polyline. This is #97 and #98. The reference's configured plot
   directory is also different by default, but the doctest configures it, so that part is not a
   content divergence.

## 6. Probe method and corpus result

The C# corpus was enumerated by replicating DescriptorExampleTests.Corpus() exactly (the same
sorted builtin names, plugin descriptors and core metadata) against the built assemblies: **141
examples**. Each example was run as prelude; example through both binaries with --file <temp.ls>
--omit-functions, the envelopes were normalised with the differential's four volatile keys at any
depth, and the result compared. **132/141 matched; 9/141 differed** — exactly the nine rows above.
The two plot rows were re-run with the same --plot-dir <dir> --plot-file plot.svg to separate the
renderer difference from the default-directory difference.

The six original rows are inside the 132 matching examples; no other descriptor example is a
matching-but-silent divergence on the fields measured here.

## 7. Gates (observed 2026-11-12)

| Step | Command | Observed line | Exit |
|---|---|---|---:|
| Workspace suite | cargo test --workspace (in rust/) | **5,345 passed, 0 failed, 1 ignored** over 21 binaries (the task snapshot's 5,326 predates 19 concurrent desktop-lane tests; this round adds none) | 0 |
| Documentation corpus | cargo run --release -p lovelace-corpus | corpus: 214 passed, 0 failed, 214 total | 0 |
| Protocol fixtures | node parity/tools/gate-fixtures.mjs | oracle: 20/20, rust: 20/20, PASS: oracle 20/20, rust 20/20 | 0 |
| CLI matrix | node parity/tools/gate-cli.mjs | CLI matrix: 16/16, PASS: every RUN-001..RUN-014 row behaves like the oracle | 0 |
| Differential (generated + replay) | node parity/diff/run.mjs --cases 200 --seed 1 | generated 200 match / 0 mismatch / 0 error; replayed 1758 match / 0 mismatch / 0 error (115 duplicate skipped; 1958 executions) | 0 |
| Clippy (own crates) | cargo clippy -p lovelace-lang -p lovelace-run --all-targets -- -D warnings | clean (no warnings; `Finished dev profile` only) | 0 |
| Format (own crates) | cargo fmt -p lovelace-lang -- --check and cargo fmt -p lovelace-run -- --check | both clean (no output) | 0 |

The workspace count is the measured one; the only difference from the task's 5,326 baseline is the
concurrent rust/lovelace-desktop lane's 19 tests, which are already in the shared tree and are not
touched by this round.

## 8. Scope

No Rust source, .cs file, fixture, corpus file, parity/ file, rust/lovelace-desktop file or other
document was edited. The only artifact added by this round is this report. Probe scripts,
enumerators and raw envelopes live under rust/target/tmp/ (git-ignored). The divergences in §3 are
reported, not fixed: every affected census row is already ported, so the redirect's deferred-row
fix condition was not met.
