# Cycle-6 · round-22 · audit M — composed programs and the seams between them

**Claim under test:** a program built from individually-correct parts stays correct, and a failure is
contained.

**Verdict: FALSIFIED.** Three **P1** findings and one **P2**. Every earlier wave probed single calls; the
three P1s are only visible when parts are *composed*: a statement the deadline interrupts, an unrelated
statement placed before a recursive call, and an `assume` statement followed by a solver.

Severity ladder (as in the other audits of this cycle): **P0** = a wrong value or an abort/crash on valid
input; **P1** = a false claim in the machine API, a wrong refusal, or data the API says it carries being
lost or altered; **P2** = cosmetic/documentation.

---

## Binary used, and how it was driven

* **Binary under test — the AOT runner named by the brief**:
  `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`,
  **5 758 464 bytes**, LastWriteTime **2026-09-12 19:17:13**. `bin\Release` was **not** used anywhere in
  this audit.
  * The tree's HEAD when I started was `683a9cd8` ("docs(cycle-6): EVD-324 - the wave's binary matches
    the final product code exactly"); the brief names `b8fa1f1`. The size and mtime are exactly the ones
    the brief gives (5 758 464 bytes, 238 s newer than the newest source file), so I treated it as the
    same artifact and every claim below is about these bytes.
* **Scratch area (outside the repo)**: `C:\Users\ricar\dev\.lovelace-auditM\` — the `.ls` programs,
  `sweep.ps1`/`batch.ps1`/`cancelinspect.ps1` drivers, `gen\` and `plots\` subdirectories. **Nothing in
  the repo was modified**; this report is the only file written under it.
* **Every run is a fresh process** (`--file <script> --json --omit-functions [--omit-variables]`,
  plus `--cancel-after` / `--plot-dir` where noted). No state crosses an invocation.
* **Every finding below was reproduced at least twice**, in fresh processes; the reproduction counts are
  given per finding.
* **Ground truth** is the protocol contract (`docs/symbolics/dsh-protocol.md`), the runner's own
  envelope DTOs (`Lovelace.Run\RunProtocol.cs`), the builtin descriptors
  (`Lovelace.Symbolics\SymbolicsPlugin.cs`) and **contrast runs of the same script text** — a script
  that is identical except that a failure replaces a cancellation, so the two envelopes can be compared
  line for line.

---

## M-1 — **P1** — everything the interrupted statement printed is dropped from the cancelled envelope, and its `timings` entry says `hasOutput:false`

**New?** **NEW.** Cycle-5's cancellation audit (`docs/goal-cycle-5/audit2/B2-temporal.md`, CB4/CB10)
checked only whether `partialOutput` is a *prefix of the statements' prints*; it never put a `print`
**inside** the statement the deadline cuts. `--omit-variables` on the cancelled path is the known item;
this is a different key and a different trigger.

### Exact program (`v1_cancel.ls`, verbatim)

    print("head")
    for i in 1..100000000 { print(i); s = 0; while (s < 200000) { s = s + 1 } }

Every iteration prints its index **immediately**, then spends ~0.3 s in the inner `while`; at
`--cancel-after 300` the deadline lands inside iteration 1 or 2, i.e. **long after at least one print has
executed**.

### Exact command

    & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --file v1_cancel.ls --json --omit-functions --cancel-after 300

### Observed envelope (deciding fields, two runs)

    run A: exit=1  ok=false  code=Cancelled  category=BudgetExceeded
      message   = "the evaluation was cancelled by the caller."
      output         = ["head"]                     (count 1)
      partialOutput  = ["head"]                     (equals output)
      timings        = [ {position:0, resultKind:"Void", hasOutput:true,  elapsed:35µs},
                         {position:14,resultKind:"Void", hasOutput:FALSE, elapsed:303.72ms} ]
      cancellation   = {"budgetMs":300,"elapsedMs":303.941,"stopped":true,"exceeded":true,"excessMs":4.271}
      result         = ABSENT
    run B: identical apart from durations (elapsedMs 294.47, exceeded:false)

    with --cancel-after 200: output=["head"], loop timing hasOutput:false
    with --cancel-after 150: output=["head"], loop timing hasOutput:false

The loop at offset 14 is the statement that ran and printed; the envelope says it printed nothing.

### The contrast run — the same text, failing instead of cancelling (`v2_error.ls`)

    print("head")
    for i in 1..100000000 { print(i); s = 0; while (s < 200000) { s = s + 1 }; if (i == 2) { nosuch() } }

    exit=1  ok=false  code=InvalidOperation  category=DomainError  message="Unknown function 'nosuch'."
      output  = ["head","1","2"]
      timings = [ {position:0, Void, hasOutput:true}, {position:14, Void, hasOutput:TRUE} ]

The prints executed by the *same loop body* are carried when the statement ends by **failing**, and are
gone when the same statement ends by **cancelling**. The high-volume variant makes the loss countable:
`k06` = `print("head"); for i in 1..100000000 { if (i < 1000) { print(i) } ; s = i }` cancelled at
300 ms (two runs), 200 ms and 150 ms returns **output count 1** every time, while the identical loop with
`if (i == 1000) { nosuch() }` added (`k07`) returns **output count 1000** (`head`+1..999). 999 lines
that ran are absent and the consumer has no way to learn they existed.

### Correct behaviour, and how I know

* `dsh-protocol.md:9-10` (Invariant 1): "stdout carries the envelope and nothing else. Anything the
  script prints with `print(...)` is captured and returned in the top-level `output` array." The lines in
  question were printed by the script; they are neither on stdout nor in `output`.
* `Lovelace.Run\RunProtocol.cs:110-121`: the error envelope's `output` holds "everything the script
  printed before the run ended: all of it on success, the lines committed before the failure here", and
  `partialOutput` is "everything the engine had already committed … the lines are the same ones Output
  carries, so the two cannot disagree". `output` and `partialOutput` do agree with each other — they
  **both under-report**, and the run's own `elapsed` (303.7 ms on the loop) shows the statement ran long
  enough to print many times.
* The engine *can* commit loop-body prints (the `v2`/`k07` contrast, and the uncancelled control
  `for i in 1..6 { print(i) }` → `output` head,1..6 with `hasOutput:true`). So the information exists
  at the moment the deadline fires and is discarded on the cancellation path.
* Observed rule (evidence, not a repair proposal): the captured output of the statement that is
  interrupted is discarded; prints from statements that completed *before* it survive (in
  `func h() { print("in"); k = 0; while (k < 100000000) { k = k + 1 }; return k }; h()` the `in` survives
  although the top-level statement `h()` is the one cancelled).
* Machine-API honesty: `timings[].hasOutput` is documented as "whether it wrote print output"
  (`dsh-protocol.md:170`); here it answers `false` for a statement that printed — a false claim a
  consumer branches on.

**Severity P1** — data the API says it carries is lost, plus a false machine-readable flag. No numeric
value is wrong and nothing aborts.

---

## M-2 — **P1** — the recursion budget is spent by *unrelated preceding statements*, so the documented `f(85)` is refused inside ordinary programs, and the refusal names "nesting" with a depth number that is not monotone

**New?** **NEW.** The known-open items are the `--print-budget` non-monotonicity and the
`PrecisionExplicitlySet` latch; audit-I measured the boundary `f(85)` ok / `f(86)` refused at top level
and did not vary the surrounding statement. K-1 (round-21) is leftover *precision* state in an embedded
engine — unrelated.

### Exact programs (verbatim; `DEF` = `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` + newline)

    m01_block_call_first.ls :  DEF  { f(85); 1 }
    m02_block_call_last.ls  :  DEF  { 1; f(85) }

### Exact command

    & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --file <script> --json --omit-functions --omit-variables

### Observed envelopes (verbatim, trimmed; three runs each, identical apart from durations)

    m01  exit=0
    {"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,
     "result":{"kind":"Natural","display":"1","typed":"1 (Natural)","structured":{"kind":"Natural","value":"1","exact":true}},
     "output":[],"variables":[],"functions":[],"elapsed":"671.4 µs","elapsedTime":{"value":671.4,"unit":"µs"},
     "timings":[{"position":0,"elapsed":{"value":3,"unit":"µs"},"resultKind":"Void","hasOutput":false},
                {"position":56,"elapsed":{"value":593.9,"unit":"µs"},"resultKind":"Natural","hasOutput":false}]}

    m02  exit=1
    {"protocolVersion":1,...,"ok":false,"code":"DepthExceeded","category":"BudgetExceeded",
     "message":"evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less
                nesting (the engine refuses it before a recursive walk can exhaust the process stack).","recoverable":true,
     "diagnostics":[{"message":"(same text)","position":56,"line":2,"column":1}],
     "elapsed":"3.83 ms","elapsedTime":{"value":3.83,"unit":"ms"},
     "timings":[{"position":0,...,"resultKind":"Void","hasOutput":false},
                {"position":56,...,"resultKind":"Void","hasOutput":false}],"output":[]}

The two programs differ **only in the order of two statements**, both at the same nesting depth:
`{ f(85); 1 }` answers, `{ 1; f(85) }` is refused. A *following* constant statement is free; a
*preceding* one costs the recursive call its headroom.

### The boundary moves with the context (max N accepted for `f(N)`, each cell a fresh process)

| context of the call | max N ok | N+1 refused with |
|---|---|---|
| top level (documented) | **85** | depth 513 |
| `{ f(N) }` (block, single statement) | 85 | depth 513 |
| `{ f(N); 1 }` (trailing sibling) | 85 | depth 513 |
| `{ 1; f(N) }` (one leading sibling) | **84** | depth 513 |
| `{ 1 ×k; f(N) }` (leading siblings, k = 1,2,3,4,5,6,8,10,15,20) | **84** for every k | k=1,2 → **513**; k=3,4 → **514**; k=5 → **515**; k=6,8,10,15,20 → **513** (not monotone in k) |
| `print(f(N))` | 84 | depth 513 |
| `f(N) + 0` | 84 | depth 513 |
| `ident(f(N))` (`func ident(x) { return x }`) | 84 | depth 513 |
| `if (1 == 1) { f(N) }` | 84 | depth 513 |
| `for i in 1..M { f(N) }` (M = 1,2,3,5,10,20,40) | 84 for every M | depth 513 |
| `i = 0; while (i < M) { f(N); i = i + 1 }` (M = 1..20) | 84 for every M | depth 513 |
| `func g() { return f(N) }` then `g()` | 84 | depth 513 |
| `func g() { return f(N) }` + `func h() { return g() }` then `h()` | 83 | depth 513 |
| `{ { … { f(N) } … } }` (2..7 nested blocks, no sibling) | 85 | depth 513 |
| k leading **top-level** statements (k = 1..1000) | 85 | depth 513 |

Two facts in that table are the finding:

1. **Order dependence.** `{ f(85); 1 }` is accepted and `{ 1; f(85) }` is refused. A statement that
   has already *returned* (the constant `1`) still costs the later call enough budget to cross 512.
2. **The refusal names the wrong cause and reports a number that is not a depth.** The input
   `{ 1; f(85) }` has one block and one constant; its textual/expression nesting is trivial, yet the
   message says "evaluation depth 513 exceeds the maximum supported evaluation depth of 512 … Rewrite the
   input with less nesting". The reported number is also non-monotone in the only thing that changes:
   with the *same* failing call `f(86)` inside a block, k = 1,3,8,12 leading siblings report **513**,
   k = 5 reports **515**, k = 4 reported **514** (from the max-N scan) — a consumer that reads the
   number as a depth gets a different "depth" for the same call.

### Correct behaviour, and how I know

* The product's own post-fix statement is that **`f(85)` answers**: `docs/goal-cycle-6/journal.md:515`
  ("`f(85)` answers (exit 0, `ok:true`), `f(86)` is refused"), `docs/goal-cycle-6/state.md:78`
  ("the evaluation walk is bounded at 512 units, `f(85)` answers"), and
  `docs/symbolics/a-plus-cycle-6-report.md:51-53` ("`f(85)` still answers"). A program that puts one
  constant statement before the call is refused, so the published capability is not a property of the
  call but of its surroundings.
* The refusals are *typed* correctly (`DepthExceeded`/`BudgetExceeded`, exit 1, stderr empty), and a
  budget that nesting consumes is defensible in principle; what is not defensible is (a) a **preceding**
  statement costing budget while a **following** one does not — the two programs have the same shape —
  and (b) a diagnostic whose message calls the counter a *depth* and prescribes "less nesting" for a
  program whose nesting has not changed. The machine-readable `code` is right; the human-facing claim
  and its number are not.
* Nothing in this table produces a wrong *value* — only refusals — so P1, not P0.

---

## M-3 — **P1** — a session `assume` is quietly ignored by `solve`/`solve_full`, which publish the contradicting root as `Solved` / `complete: true` with `conditions: []`

**New?** **NEW.** Not in the known-open list. I grepped the audit corpus for the composition
(`docs/goal-cycle-6/round-13`, `round-20`, `round-21`, `docs/goal-cycle-5`): assumptions appear only in
arity/type sweeps (`audit-D-workflow.md`, `f1-implementation.md`) and in the *simplify* rows
(`docs/goal-cycle-5/audit2/A3-symbolic.md:301-302`), never with a solver.

### Exact programs (verbatim)

    z2_contra_solve.ls :  x = symbol("x"); assume(x > 5); solve(x == 3, x)
    z1_pos_pred.ls     :  x = symbol("x"); assume_positive(x); solve(x^2 == 4, x)
    z3_assume_neg.ls   :  x = symbol("x"); assume(x < 0); solve(x^2 == 4, x)
    z4_simplify_uses.ls:  x = symbol("x"); assume(x > 0); simplify(sqrt(x^2))     <- control, same session store

### Exact command

    & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --file <script> --json --omit-functions --omit-variables

### Observed (three consecutive fresh processes; identical)

    z2: exit=0  ok=true
        result.display = "SolveResult(status: Solved, variable: x, domain: complex, complete: True,
        completeness: Complete, solutions: [Solution(value: 3, conditions: [], multiplicity: 1,
        exactness: Exact)], families: [], common_conditions: [], represented_count: 1,
        unrepresented_count: 0, unrepresented_reason: , diagnostics: [])"
        result.structured.status = {"kind":"Enum","type":"SolveStatus","value":"Solved"}
        result.structured.complete = {"kind":"Boolean","value":"true"}
        result.structured.solutions.elements[0].fields[conditions] = {"kind":"Array","shape":[0],"elements":[]}

    z1: solutions = [-2, 2], conditions [], complete: True
    z3: solutions = [-2, 2], conditions [], complete: True
    z4: exit=0, result = x     (the SAME assumption store is honoured by simplify)

### Correct behaviour, and how I know

* The `assume` builtins' own published descriptors are session-wide:
  `SymbolicsPlugin.cs:129-136` — "Assumes x > 0 **for the rest of the session**." /
  "Assumes x is real-valued for the rest of the session." etc.; `assume` itself is registered with
  `["assume(x > 5)", "assume(and(x > 0, x < 10))"]` (`SymbolicsPlugin.cs:350-360`).
* The store is live and contradiction-checked in the same process: `assume(x > 0); assume(x < 0)`
  refuses with `UnsatisfiableAssumptions`, "Assumption x < 0 contradicts the existing assumptions
  (its negation x >= 0 is provable)", and `assumptions()` returns the set
  (`AssumptionSet(display: x > 0, assumptions: [x > 0])`).
* The contract has a place to carry exactly this: per-solution `conditions`
  (`Lovelace.Symbolics\Solvers\Solve.cs:61`, `Solution(Expr Value, AssumptionSet Conditions)`;
  `RunProtocol`/`dsh-protocol.md:128-153` publish `conditions` as a per-solution array). Yet the root
  the assumption excludes comes back with **no condition, `complete: true`, and an empty
  `diagnostics` array** — the envelope asserts an unconditional, complete solution set that the
  session's own assumption set contradicts.
* The engine does consume the store for symbolic evaluation: with the *same* `assume(x > 0)` in the
  *same* run, `simplify(sqrt(x^2))` answers `x` (z4), matching the recorded HELD rows
  `A3-symbolic.md:301`. So the two statements are individually correct and the seam between them is
  where the assumption is lost.
* Grep of the solver sources: `Solvers\Solve.cs` contains no reference to the session assumption set —
  its `AssumptionSet` values are locally built from denominators/non-zero conditions
  (`Solve.cs:227,275,929`) or `AssumptionSet.Empty` (`Solve.cs:442`). This is consistent with the
  observation; it is cited as corroboration, not as the contract.

**Severity P1** (false claim in the machine API: `Solved` / `complete:true` / empty `conditions` for a
root the session's assumptions exclude). Under the stronger reading — that a session assumption is
meant to constrain solving, as it constrains `simplify` — the returned root `3` under `x > 5` is a
**wrong value (P0)**; I grade the observed artifact P1 because I could not find a sentence that says in
so many words "solve honours the assumption store", only a sentence that says the assumption holds for
the rest of the session.

---

## M-4 — **P2** — a program that plots twice publishes only the last plot, and the second SVG overwrites the first, so the first plot is unrecoverable

**New?** **NEW** (no prior audit runs two plots in one program). **P2** because the envelope's `plot`
field is a single object by construction (`RunProtocol.cs:43`, `PlotDto(Path, Title, Svg)`), so nothing
is *claimed* about the first plot — the loss is simply silent.

### Exact program (verbatim, `p05_two_plots.ls`)

    plot([1, 2, 3], [1, 2, 3], "first")
    plot([1, 2, 3], [3, 2, 1], "second")

### Exact command

    & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --file p05_two_plots.ls --json --omit-functions --omit-variables --plot-dir <scratch>\plots

### Observed (two runs, identical)

    plot.path="<scratch>\plots\plot.svg"  plot.title="second"  plot.svg.Length=14734
    envelope SVG contains "second": True   contains "first": False
    plot.svg on disk contains "second": True   contains "first": False
    result.display = "<scratch>\plots\plot.svg"

Both statements succeed (exit 0, no diagnostic, `output` empty). The only artifact a consumer can
recover is the second plot; the first is neither in the envelope nor on disk.

---

## Held under composition (driven to the edge, **no** finding charged)

* **Prints before a mid-program failure survive, on every failure path I could reach.** `print("a");
  print("b"); 1/0` → `output:["a","b"]`; an error inside a loop body → `["1","2"]`; an error inside a
  function called twice → `["enter","exit","1","enter"]` (both calls' prints, in order, plus the first
  call's result); the 999-print loop with the error added → 1000 lines (M-1's contrast run).
* **A plot that fails does not corrupt what printed before it.** `print("before"); plot([1,2],[1,2,3],"t")`
  → `InvalidOperation` "plot() vectors must have the same length (2 vs 3).", `output:["before"]`,
  diagnostic at the plot statement. A plot **write** failure (`--plot-file a:b.svg`) →
  `code:PlotFileError`, `category:TypeMismatch`, "Cannot write the plot file 'a:b.svg': …",
  `output:["before"]`, diagnostic position 17 = the plot statement's own offset.
* **Positions.** `diagnostics[].position` and `timings[].position` agreed (both the failing statement's
  start) in every program I ran: single-line, CRLF (offsets 0/15/30 on lines 1-3), semicolons *inside*
  string literals (`print("a;b;c"); 1/0` → 16), `µ` (1 unit) and an emoji (2 UTF-16 units) before the
  failure. The UTF-16-unit accounting for a multi-byte prefix is the same family as the recorded
  **G-2** (leading BOM shifts positions) — reproduced, **not** charged, and no *new* trigger produced a
  wrong position (my first reading of a "+1" offset was my own line:column/position mix-up and did not
  reproduce under a controlled matrix).
* **Precision changed mid-program does not damage values.** `setprecision(60); a = sqrt(2); setprecision(15); a`
  → `result.display` is 15 digits but `result.structured.value` and `variables[a].structured.value`
  still carry all 60; `evalf(a, 50)` after the lowering answers 50 digits; `b = a + 0` computed at 15
  then read back at 60 still carries 60. Only the human `display` follows the ambient precision at
  publish time. Lowering precision then asking for too many digits is refused with the *current*
  precision named: "pi(): argument 1 (digits) must be a digit count between 1 and 20 (the engine's
  computation precision); got 30." **The success envelope has no precision field at all** (checked the
  full top-level key list and `RunProtocol.cs`), so the brief's "the ENVELOPE's reported precision"
  has no corresponding key to test; see *could not check*.
* **Repeated and looped work near the budget does not accumulate.** `f(85); f(85)` and `f(80); f(85)`
  both answer; `for i in 1..M { f(84) }` answers for every M I tried (1, 2, 3, 5, 10, 20, 40, 50); `f(80)` ×200 in a for answers;
  `func h() { return f(84) }; h(); h(); h()` answers; the max N inside a for/while body is 84 for
  every M I tried (1..40), i.e. the deficit is a per-context constant, not a leak that grows with
  iterations. Top-level statements before the call (up to 1000) cost nothing.
* **Mutual recursion** works and is budgeted: `even(20)`, `even(45)`, `even(80)` answer;
  `even(90)` is refused with the same typed `DepthExceeded/BudgetExceeded`.
* **Value-depth budget composed with itself.** Two sequential 1023-deep builds in one program both
  succeed; a function that builds a 1023-deep value answers on both calls; the 1024th nesting is refused
  with `DepthExceeded`, "value depth 1025 exceeds the maximum supported nesting depth of 1024". The
  1023-deep value projects completely (the flat `shape` list, 2144 bytes of JSON) and prints.
* **Cancellation keeps the useful partial state and labels it.** `x = 5; y = [1,2,3]; print("a");
  for i in 1..1e8 { s = i }; z = 9` cancelled → `code:Cancelled`, `category:BudgetExceeded`,
  `output == partialOutput`, `partialVariables = [x=5, y=[1, 2, 3]]`, one `timings` entry per started
  statement, and (when the stop is late) a diagnostic naming the overrun with
  `excessMs = round(elapsedMs − budgetMs)` (311.73/300 → 12.028; 209.755/200 → 10.041;
  154.841/150 → 5.164). A completed `solve` result survives a later cancellation **in full**:
  `partialVariables` carries `s=Record:SolveResult(status: Solved, …, solutions: [Solution(value: -2 …),
  Solution(value: 2 …)], …)`.
* **"Same program twice in one call" vs "split across two invocations".** Repeating a program's
  statements in one runner call gave byte-identical answers to running it once (values, `revision` 81,
  `output` order); I found **no** contamination between the two copies. The only split-dependence I
  could produce is the intended session state: `setprecision(15); 1; pi(50)` in one call is refused
  ("between 1 and 15 (the engine's computation precision)"), while `pi(50)` in a second, fresh
  invocation answers 50 digits — and the refusal names the precision it was refused under, so the
  cause is not hidden.
* Ranges are not subject to the array allocation budget: `len(1..268435457)` → 268435457 (one past the
  documented single-allocation bound), while arrays over the bound are refused before allocation
  (recorded in audit I).

## Matched already-recorded items — reproduced, **not** counted

| item | recorded at | what I measured on this binary |
|---|---|---|
| `--cancel-after` ledger can say the deadline stopped a run that did not exceed its budget | audit-I **I-1** | `--cancel-after 300` on the 999-print loop: `elapsedMs 290.1/292.4/294.5/297.8` all with `stopped:true, exceeded:false, excessMs:0` (4 of 5 runs). Not re-charged. |
| positions count a leading BOM / non-ASCII as characters | audit-G **G-2** | reproduced as a *family*: positions count UTF-16 code units (emoji = 2). No new trigger produced a wrong position. |
| `--print-budget` non-monotone, `--omit-variables` on the cancelled path, `--text` failure on stderr, evalf >1000 places, the precision latch, the special-angle exactness bound | brief's known-open list | touched only incidentally; none is re-reported. |

## Could not check

1. **A solver *after* a cancellation.** The CLI performs exactly one evaluation per process and
   cancellation terminates it, so no program text can reach a solver after a cancel. The nearest
   reachable composition — a solver *before* the cancel — was checked and the result survives intact in
   `partialVariables` (above). Testing the literal item needs the embedded engine (round-21's harness).
2. **The allocation budget boundary itself (268435456 elements).** An allocation *at* the documented
   bound needs tens of GB on this box (cycle-5 measured ≈122 bytes/element for a 10 M-element vector),
   so I did not attempt it; I checked only the recorded >bound refusal shape and cheap lazy ranges
   (`len(1..268435457)`).
3. **An envelope-reported precision field.** No such field exists in the success or error DTOs
   (`RunProtocol.cs` has no `precision` key, and a raw envelope's top-level key list contains none), so
   the brief's "effect on the ENVELOPE's reported precision" could only be tested through
   `display`/`structured` and through refusal messages — both did hold.
4. **Concurrency.** Every probe here is one process per run; the multi-caller, shared-engine cases are
   round-21's scope, not reachable from the CLI.
5. **`--text` and `--stdin` surfaces for the M-1 loss.** All M-1 measurements are `--json --file`; the
   `--text` failure path is already known to drop captured output, so I did not add a surface variant.
6. **Narrowing the mutual-recursion boundary to a single N** (I bracketed it: `even(80)` ok,
   `even(90)` refused) and the exact per-statement budget cost of a preceding sibling (bracketed to
   "1 to 8 units" by the max-N scan).

---

## Findings table

| id | severity | one line | new? |
|---|---|---|---|
| **M-1** | **P1** | Prints made inside the statement the deadline interrupts are absent from `output` **and** `partialOutput`, and that statement's `timings` entry reports `hasOutput:false`; the same prints are carried when the same statement fails instead of being cancelled. | NEW |
| **M-2** | **P1** | The evaluation budget is spent by *preceding* statements, so the documented `f(85)` is refused in `{ 1; f(85) }` while `{ f(85); 1 }` answers; the refusal calls it "evaluation depth 513/514/515" and prescribes "less nesting" for input whose nesting did not change. | NEW |
| **M-3** | **P1** | A session `assume` is ignored by `solve`/`solve_full`: `assume(x > 5); solve(x == 3, x)` → `Solved`, `complete: true`, `conditions: []`; the same store makes `simplify(sqrt(x^2))` answer `x` in the same run. | NEW |
| **M-4** | **P2** | Two `plot()` calls in one program: only the last is published, the second SVG overwrites the first on disk; the first plot is unrecoverable (the single `plot` field is by construction). | NEW |
