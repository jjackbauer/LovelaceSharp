# Cycle-6 · round-20 · audit I — budgets: does the binary say *which* limit, and are its numbers true?

**Claim under test.** "When the published binary hits a limit it says which limit, and the numbers it
reports about the limit are true."

**Verdict.** **Falsified** — one P1 (the `--cancel-after` ledger can publish a self-contradictory verdict) and two P2s
(an unnamed `evalf` digit cap whose own published description contradicts it; a truncation that splits an
identifier mid-token). **No P0**: no wrong computed value and no abort/crash was found on any limit path,
and every refusal I provoked was typed (code + category), named its budget and was stable across repeats.
Nothing in the repo was modified; this file is the only artifact written.

**Binary.** `out/aot/Lovelace.Run.exe`, **5 804 544 bytes**, last written **2026-09-12 18:00:55 local**.
Ground truth: SymPy 1.14.0 / mpmath 1.3.0 in `C:/Users/ricar/dev/.lovelace-tools/python`.
Shell: Windows PowerShell 5.1. Every probe was executed at least **twice** (ids ending -a/-b, or separate
sessions); the two observations agreed on every field quoted, apart from wall-clock digits.

**Tooling notes (my harness, NOT product defects — checked before writing anything down).**
1. PS 5.1 mangles an embedded double quote when a string is passed to a native exe, so scripts with string
   literals cannot go through `--eval` from this shell. Quoted scripts below were run through
   `--stdin` (pipeline) or `--file`; the same script text succeeds there.
2. My first process harness wrote stdin through .NET ProcessStartInfo, which prepends a BOM; the runner
   then correctly answered ParseError "Unexpected character" at position 0. That is my tooling, not the
   product: the shell pipeline form works ('1 + 1' piped to --stdin gives ok:true, value 2). Those BOM
   probes are excluded from every count below.

---

## I-1 — **P1** — the --cancel-after ledger can say the deadline stopped a run that "did not exceed" its budget

### Exact command (3 runs, PowerShell; no embedded quotes, so --eval is safe here)
    out/aot/Lovelace.Run.exe --eval 'sum(1..20000000)' --json --omit-functions --omit-variables --cancel-after 100

### Observed envelope (verbatim, trimmed to the deciding fields; wall clock measured around the call)
    RUN 1  exit=1  wall=141 ms  code=Cancelled  cancellation={"budgetMs":100,"elapsedMs":112.798,"stopped":true,"exceeded":true, "excessMs":12.798}   diagnostics: 1
    RUN 2  exit=1  wall=120 ms  code=Cancelled  cancellation={"budgetMs":100,"elapsedMs":99.559, "stopped":true,"exceeded":false,"excessMs":0}       diagnostics: 0
    RUN 3  exit=1  wall=136 ms  code=Cancelled  cancellation={"budgetMs":100,"elapsedMs":114.278,"stopped":true,"exceeded":true, "excessMs":14.278}   diagnostics: 1

**RUN 2 is the finding**: stopped:true (the deadline is what ended the run — code Cancelled,
category BudgetExceeded) published next to exceeded:false / excessMs:0 (the same envelope's verdict that
the budget was NOT consumed). Because exceeded:false suppresses the overrun diagnostic
(Lovelace.Run/Runner.cs:288-292), diagnostics is empty as well, so nothing in the envelope says the budget
was blown.

**Eleven instances, three sessions, six workloads** (all --cancel-after 100, all
--json --omit-functions --omit-variables; script text through --file, identical to the --eval above):

| probe id | script | wall ms | elapsedMs | stopped | exceeded | excessMs |
|---|---|---|---|---|---|---|
| (RUN 2 above) | sum(1..20000000) | 120 | 99.559 | true | **false** | **0** |
| cx-c2-100 | prod(1..200000) | 140 | 97.730 | true | **false** | **0** |
| cx-c7-100 | sum(1..10000000); nosuchfn(1) | 157 | 99.748 | true | **false** | **0** |
| rep1-sumfail-100 | sum(1..10000000); nosuchfn(1) | 141 | 95.728 | true | **false** | **0** |
| rep1-sum-100 | sum(1..10000000) | 140 | 96.165 | true | **false** | **0** |
| rep1-sum2-100 | sum(1..20000000) | 125 | 98.834 | true | **false** | **0** |
| rep1-mat-100 | matmul(eye(400), eye(400)) | 123 | 98.581 | true | **false** | **0** |
| rep2-prod-100 | prod(1..200000) | 141 | 97.175 | true | **false** | **0** |
| rep2-sum2-100 | sum(1..20000000) | 142 | 97.514 | true | **false** | **0** |
| rep2-mat2-100 | matmul(eye(600), eye(600)) | 141 | 99.252 | true | **false** | **0** |
| rep3-sum2-100 | sum(1..20000000) | 140 | 96.070 | true | **false** | **0** |

sum(1..20000000) alone produced the contradiction in **4 of 5** runs and in **3 of 3** runs inside one
batch; the opposite outcome (stopped:true, exceeded:true) is more common, which is why it is easy to miss.

### Correct behaviour, and how I know
Lovelace.Run/RunProtocol.cs:53-73 is the contract for these five fields, in the runner's own words:
* stopped — "the deadline actually stopped this evaluation: the envelope is a failure with code Cancelled
  and category BudgetExceeded". No second condition.
* exceeded — "the evaluation consumed more than its budget, with excessMs saying by how much. This is the
  fact an agent must never have to infer". The only combinations the document explains are
  stopped=false/exceeded=true ("a DROPPED deadline") and stopped=true/exceeded=true ("a deadline that was
  observed LATE"). **stopped=true with exceeded=false has no documented meaning** — it reads as "the
  deadline fired inside the budget", which a timer cannot do.
* elapsedMs — "the SAME wall time the envelope publishes as elapsedTime, so budget and elapsed are
  directly comparable: excessMs = max(0, elapsedMs - budgetMs)".

The two clocks do not share an origin; that is the located cause:
* the deadline starts at Lovelace.Run/Runner.cs:185-187 (new CancellationTokenSource(budget));
* Directory.CreateDirectory(engine.PlotOutputDirectory) at Runner.cs:199 runs **before**
  engine.EvaluateAsync(...) at Runner.cs:211;
* the elapsed clock starts inside that call, at Lovelace.Suite/SuiteEngine.cs:241
  (var stopwatch = Stopwatch.StartNew();).

So budgetMs is measured from T and elapsedMs from T + delta (delta = the pre-evaluation setup), which is
visible above as the 0.44-4.27 ms by which elapsedMs falls short, and as the ~20-40 ms by which the
process wall clock exceeds elapsedMs in every run. A CancellationTokenSource(ms) timer cannot fire before
ms have elapsed on its own clock, so stopped:true proves at least 100 ms passed while the same envelope
reports 99.559 ms consumed. Either elapsedMs must be measured from the deadline's origin, or the ledger
must not answer "the budget was respected" when the deadline is what ended the run.

**Severity P1** — a false claim in the machine API. Not P0: no computed value is wrong and nothing aborts.
**NEW** — not in round-13/, not in round-18/audit-E-cli.md, not in evidence.md or
a-plus-cycle-6-amendment.md: EVD-258 recorded only the exceeded:true case
("stopped":true,"exceeded":true,"excessMs":48.331), audit-G-consistency.md:250 only exceeded:true, and
audit-H-determinism.md:208 only the never-fired stopped:false/exceeded:false.

---

## I-2 — **P2** — evalf's 1000-place cap is silent, and the builtin's published description contradicts it

### Exact commands
    :: ev.ls contains exactly:  setprecision(5000); evalf(sqrt(2), 5000)
    out/aot/Lovelace.Run.exe --file ev.ls --json --omit-functions --omit-variables

    :: control, one statement:   setprecision(5000); sqrt(2)
    out/aot/Lovelace.Run.exe --file ctl.ls --json --omit-functions --omit-variables

    :: and the script file containing exactly   evalf(sin(1), 1001)   (plus the 1000 variant)

(The equivalent --eval 'evalf(sin(1), 1001)' form was also tried directly and exceeded a 180 s fence when
re-run under load — see the cost note; the --file runs below are the recorded ones.)

### Observed envelope (verbatim, trimmed to the deciding fields)
    setprecision(5000); evalf(sqrt(2), 5000)   exit=0  ok=true
      result.structured = {"kind":"Real","value":"1.4142135623730950488016887242096980785696718753769...","exact":false}
      decimals=1000  valueLen=1002  displayDecimals=1000
      truncated / truncationReason / budget = ABSENT      diagnostics = none       (identical twice)

    setprecision(5000); sqrt(2)                exit=0  ok=true
      result.structured = {"kind":"Real","value":"1.4142...","exact":false}   decimals=5000  valueLen=5002

    evalf(sin(1), 1000) and evalf(sin(1), 1001)   exit=0  ok=true
      both = {"kind":"Real","value":"0.8414709848078965066525023216302989996225...85150793983830395678167948","exact":false}
      decimals=1000 each (byte-identical values); truncated/budget = ABSENT     (identical twice)

    Ground truth: mpmath 1200 dps, first 1000 decimals of sin(1) vs the delivered value -> firstMismatchAtDecimal = None

### What is wrong
A request above 1000 is silently reduced to 1000: ok:true, exit 0, **no** truncated/truncationReason/budget
(the protocol's own vocabulary for a bounded rendering, docs/symbolics/dsh-protocol.md:17-20), no
diagnostic, and a value byte-identical to the 1000-place request. The cap that bites is **not** the
engine's cap: in the same binary sqrt(2) carries 5000 decimals under setprecision(5000) while
evalf(sqrt(2), 5000) carries 1000. The clamp is a hard-coded 1000 inside evalf
(Lovelace.Symbolics/SymbolicsPlugin.cs:1735, int places = Math.Min(digits, 1000); and its twin at :1618),
not Real.MaxComputationDecimalPlaces, which that run had raised to 5000.

The builtin's own published description says the opposite — SymbolicsPlugin.cs:498-500: "Numerically
evaluates a symbolic expression to the given number of decimal places. The count is honoured for an
already-numeric argument too ... **the published value is bounded back to the N places that were asked
for**". Neither capabilities() nor the pi/e/setprecision descriptors
(Lovelace.Suite/CoreBuiltinMetadata.cs:41-46) announce any cap either.

**Not charged (documented bound, said out loud as the brief requires): the existence of a 1000-place
bound.** EVD-276 records it as the operative cap, and audit-B-lattice.md's "could not check" #3 names the
evalf clamp explicitly (SymbolicsPlugin.cs:1554-1563, Math.Min(digits, 1000)) and deliberately did not
probe past it. What this finding charges is the silence on the wire and the descriptor sentence that
denies the clamp.

**Severity P2** (documentation / machine-API honesty; no value is numerically wrong — all 1000 delivered
digits match mpmath). **NEW? Partly**: the clamp is acknowledged in audit B's could-not-check list; the
contradicting descriptor text and the unmarked delivery are not recorded anywhere.

**Cost note — recorded, not charged** (no severity class covers cost): evalf(sin(1), 5000) did not finish
in **12.4 minutes** and was killed, because the working precision is digits + 20 places
(SymbolicsPlugin.cs:1657) while the payload is clamped to 1000 (:1735) — the caller pays for digits it
never receives. audit-B-lattice.md's could-not-check #2 already recorded 151 953 ms for the 1000-place
case.

**Match — already recorded, not counted here:** pi(1001) and e(1001) (default precision) refuse with
code InvalidArgument / category TypeMismatch, message "Specified argument was out of the range of valid
values. (Parameter 'digits')" — no builtin, no range, no cap named. That is audit B's **L5** (P2, open,
audit-B-lattice.md:322), reproduced twice; my trigger (the hard cap at 1001 at default precision) is a
variant of the recorded setprecision(20); pi(30).

---

## I-3 — **P2** — a budgeted truncation can split an identifier mid-token, contradicting the printer's own promise

### Exact command (PowerShell; stdin so the embedded quotes survive PS 5.1)
    $nm = 'a' * 54
    $s  = $nm + ' = symbol("' + $nm + '"); ' + $nm + ' + 1'
    $s | out/aot/Lovelace.Run.exe --stdin --json --omit-functions --omit-variables --print-budget 1

### Observed envelope (verbatim, trimmed; identical twice)
    "ok":true, "result":{"kind":"Symbolic","structured":{
       "kind":"Symbolic",
       "pretty":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ...",
       "canonical":"(add (rat 1 1) (sym  ...",
       "exact":true, "nodeCount":3, "truncated":true, "truncationReason":"node-budget", "budget":1}}
    "display":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1"   (58 chars, the real name)

The pretty body is **48** characters, all of them identifier characters, taken from a symbol whose real
name is **54** characters. The published prefix therefore ends in an identifier (a x48) that does not
exist in the value, and is not cut at a token boundary.

### Correct behaviour, and how I know
Lovelace.Symbolics/Printing.cs:340-345: "A rendering that exceeds the budget is abbreviated (**never
silently truncated mid-token**)"; Printing.cs:402-409: "Cuts at a token boundary so a number or
identifier is never split mid-token". The implementation's fallback defeats that promise exactly when the
first token is longer than the character allowance Math.Max(48, limit * CharsPerNode) (Printing.cs:387):
the walk in SafeCut reaches position 0 and "return i > 0 ? i : cut;" (Printing.cs:408) returns the raw cut.
The canonical form is unaffected (it starts with an open paren).

**Severity P2** — cosmetic: the value is still marked truncated:true, still names
truncationReason:node-budget and budget:1, and the text is still a verbatim prefix of the real rendering,
so nothing is passed off as complete. **NEW** — not in round-13/round-18.

---

## Coverage that held — driven to the edge, no finding

All run with --json --omit-functions, most in -a/-b pairs; the two observations were byte-identical apart
from durations.

### --print-budget (the truncation report is honest)
Expressions x^2 + 1 (5 nodes), expand((x+1)^10) (48), diff(sin(x)*cos(x)*exp(x), x) (23),
(x+1)*(x+2)*...*(x+8) (25), x^1234...890 (3), expand((x+1)^30) (148) at budgets 0/1/2/12/16/100/1000:
* **every** truncated pretty/canonical ends in the " ..." marker, carries truncated:true,
  truncationReason:"node-budget" and budget = the requested budget, and is a **verbatim prefix** of the
  budget-free rendering (checked mechanically over all 115 probes plus a dense 1..64 sweep);
* nodeCount always reports the **real** node count (5/48/23/25/3/148), never the truncated one;
* no untruncated value ends in the marker, i.e. no complete value looks truncated and no truncated value
  looks complete;
* variables[] carries the same budget and marker (y = expand((x+1)^10) at 1/12/16), and nesting is
  correct ([x^2+1, (x+1)^10] at budget 1 -> both elements truncated:true, budget:1; x at 1 node correctly
  untruncated);
* result.display/typed are **not** budgeted (58/92/418 chars at budget 1) — the documented split
  (Runner.cs:529-539: the display bound is a display bound).

### Depth budgets — five kinds plus evaluation, all typed, named and stable
| input | boundary | refusal |
|---|---|---|
| nested parens | 200 ok / 260 refused | DepthExceeded/BudgetExceeded — "expression nesting depth 257 exceeds the maximum supported nesting depth of 256" |
| nested sin( | 200 ok / 260 refused | same message |
| nested if blocks | 300 refused | "statement nesting depth 257 exceeds ... 256" |
| flat 1+1+... chain | **510 ok / 511 refused** | "expression tree depth 513 exceeds ... 512" |
| runtime-built symbolic e = sin(e) | **255 ok / 256 refused** | "symbolic expression depth 257 exceeds ... 256" |
| x = [x] repetition | **1023 ok / 1024 refused** | "value depth 1025 exceeds ... 1024" |
| recursive f(n) | **f(85) ok / f(86) refused**, f(432) refused | "evaluation depth 513 exceeds the maximum supported evaluation depth of 512" |

Every refusal names the stage, the depth reached, the budget and the measure, exits 1 with **0 bytes on
stderr**, and repeats identically. The reported depth is the level at which the walk stopped (513 for
every chain >= 511) — consistent, not the input's total depth.

### Allocation budget
zeros(1000000000), zeros(268435457), zeros(2, 200000000), eye(16385) -> all
AllocationRefused/BudgetExceeded, exit 1, message names the builtin, the exact shape, the exact element
count **and** the budget: "... which holds 268435457 element(s) — more than the maximum single-allocation
budget of 268435456 element(s). The request is refused BEFORE the allocation is attempted."
(Lovelace.Suite/ArrayAllocationBudget.cs:36,114-122). len(zeros(1000000)) -> 1000000;
len(1..1000000) -> 1000000.

### --cancel-after — the rest of the ledger is truthful
* in-budget control 1 + 1 at 1/100/10^9 ms -> ok:true, stopped:false, exceeded:false, excessMs:0;
* --cancel-after 1000000000 on sum(1..10000000) -> exit 0 after 3486.8 ms, ledger present,
  stopped:false/exceeded:false, no bogus stop;
* exceeded = (excessMs > 0) and excessMs = max(0, round(elapsedMs,3) - budgetMs) held in all 68 probe
  envelopes I checked, including the late-observed stops (112.798 -> 12.798);
* a run that fails after blowing the deadline keeps both facts (stopped:true, exceeded:true, plus the
  overrun diagnostic);
* --cancel-after 0, -1, 2147483648, 9999999999 -> exit **2**, "Error: --cancel-after requires a positive
  millisecond count." on stderr, 0 bytes on stdout (the documented usage-error exit code,
  dsh-protocol.md:207-209); 2147483647 is accepted and honoured.

### Huge inputs inside the limits — nothing dropped
* **2000 statements** -> timings has exactly **2000** entries, result 1999; **5000 statements** -> **5000**
  timings (timings really is one entry per top-level statement);
* **1000 prints** -> output has 1000 lines, first 0, last 999; **5000 prints** -> 5000 lines, first 0,
  last 4999;
* **a 50 000-character string** -> result.structured.value length **50 000**; print(s) -> one output line
  of **50 000** characters;
* **20 000-element array** -> elements has all 20 000 (first 1, last 20 000, shape [1,20000]);
* **5000 variables** -> variables has **5001** entries (the 5000 plus _), 5000 timings;
* wide expression len(1..5000) -> 5000; expand((x+1)^30) (148 nodes, 418-char pretty) intact.

---

## Matched already-recorded items — reproduced, **not** counted as findings

| item | recorded at | what I measured |
|---|---|---|
| --print-budget is non-monotone | audit-D **F3**, audit-E **F7** (P2, still open) | expand((x+1)^10): budget 12 -> 73 chars, budget **16 -> 31 chars**; two more families: diff(sin*cos*exp) 2 -> 50 chars, **12 -> 28**; (x+1)...(x+8) 2 -> 50, **12 -> 29**. A dense 1..64 sweep puts the cliff at **15 -> 16** (92 -> 31 chars) and shows the text is not even containment-monotone. audit-G-consistency.md:397 reports "not reproduced" for this item: its budget grid (1/2/3/5/10/40/1000) steps over the transition — the pair to use is 12/16. |
| --omit-variables ignored on the cancelled path | audit-E **F4** (P2, open) | --cancel-after 100 with --omit-variables: partialVariables still carries x and y (2 entries), 4 runs. Its sibling --print-budget **is** honoured on that path (truncation markers present). |
| pi(n)/e(n) over the cap -> raw .NET message | audit-B **L5** (P2, open) | pi(1001), e(1001) -> InvalidArgument/TypeMismatch, "Specified argument was out of the range of valid values. (Parameter 'digits')", no builtin/range/cap named. |
| evalf digit-count misbehaviour | audit-B **L4** (over-delivery, open) + could-not-check #3 (the clamp) | I measured the **under**-delivery side (I-2). |
| 100-decimal structured truncation | **closed** b3b74b8 (EVD-287) | re-checked and clean: setprecision(5000); sqrt(2) carries 5000 decimals in structured.value with no marker. |

## Documented bounds measured — flag honest, **not** charged

* --print-budget 0 and --cancel-after 0 are **usage errors**: exit **2**, one line on stderr, 0 bytes on
  stdout, no envelope. The protocol documents exit 2 for usage errors and does not promise an envelope
  there (dsh-protocol.md:207-209; Runner.cs:83-96 rejects a non-positive count).
* The **1000-place computation cap** (Real.MaxComputationDecimalPlaces, EVD-276) — a documented engine
  bound; see I-2 for what is charged instead.
* The **allocation cap** of 268 435 456 elements, the **five nesting budgets** (256/512/256/256/1024,
  Lovelace.Abstractions/InputDepth.cs:33-94) and the **evaluation budget** of 512 units
  (InputDepth.cs:53-69, f(85)/f(86)) are intended bounds; every one names itself when it bites.
* Trailing-zero drops (setprecision(1001); sqrt(2) -> 1000 decimals because the 1001st digit of sqrt(2)
  is **0**, verified against mpmath) — numerically harmless and already recorded by audit B.
* display/typed are not bounded by --print-budget — documented in Runner.cs:529-539.

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| I-1 | **P1** | **NEW** | out/aot/Lovelace.Run.exe --eval 'sum(1..20000000)' --json --omit-functions --omit-variables --cancel-after 100 -> code Cancelled, cancellation {"budgetMs":100,"elapsedMs":99.559,"stopped":true,"exceeded":false,"excessMs":0} (11 instances) |
| I-2 | P2 | partly (clamp acknowledged in audit B's could-not-check #3; the contradicting descriptor and the silence are new) | setprecision(5000); evalf(sqrt(2), 5000) -> 1000 decimals, truncated/budget absent, while setprecision(5000); sqrt(2) in the same binary carries 5000 |
| I-3 | P2 | **NEW** | 54-char symbol name + --print-budget 1 -> pretty "aaa...(48 a's) ...", truncated:true, budget:1 — the identifier is split mid-token |

Counts: **P0 = 0, P1 = 1, P2 = 2.**

## Could not check

1. **The accept side of the allocation boundary** — zeros(268435456) (exactly 2^28 elements, a ~2 GiB
   single allocation). A JSON envelope for that array would be gigabytes of text, so I did not start it;
   only +1 (refused, above) and 10^6 (accepted) were measured.
2. **evalf(f, N) for N > 1001 completing.** evalf(sin(1), 5000) did not finish in **12.4 min** (killed).
   The claim that it also delivers 1000 decimals is an inference from the N = 1001 probe plus the single
   clamp site, not an observation.
3. **setprecision(100000); pi(1000)** — never ran (its batch was killed). setprecision(1000/1001/2000/5000)
   were measured: sqrt(2) carries 999/1000/2000/5000 decimals respectively.
4. **--stdin for the huge inputs.** The 2000-statement, 5000-print and 50 000-character scripts were
   driven through --file; --stdin was verified only for '1 + 1' and the I-3 script (my process harness's
   stdin writes a BOM — a tooling artifact, see the notes).
5. **--text with a budget refusal / a cancelled run** (recorded as open elsewhere); not probed.
6. **High-precision cost as a defect**: evalf above 1000 places is minutes-slow, recorded as a cost note
   only (no severity class covers cost).
7. **Other hosts** (Studio, REPL) and **non-default locales**: only the published AOT binary on this
   machine's default culture was exercised.
