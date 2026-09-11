# LovelaceSharp — Cycle 4: Unblock and Converge (fresh-session brief)

You are starting fresh. You know nothing about prior cycles. Everything you need is on disk in the
repository; this brief tells you where. Read it fully before acting.

---

## 1. Situation

Cycle 3 closed all nineteen items of its brief and fixed nine defects. It is **not** claimed A+
because three categories remain, and **all three are blocked on resources an agent in this
environment does not have**. Your job is to unblock them and finish.

**Repository**: `C:\Users\ricar\dev\LovelaceSharp`
**HEAD**: `35e2609` — and **the entire cycle is uncommitted**. That is blocker 1.

### Verified state (measured, not reported)

| Fact | Value | How it was established |
|---|---|---|
| Last full suite sweep | **2386 passed / 0 failed / 6 skipped** across 15 projects | orchestrator ran all 15 suites; `docs/goal-cycle-3/final/suite-counts.txt` |
| Forced full rebuild | **0 warnings / 0 errors** | `dotnet build LovelaceSharp.slnx -c Release --nologo --no-incremental` |
| Native AOT | published and executed; smoke passes | `out/aot/Lovelace.Run.exe` |
| Cycle start baseline | 1976 passed | recorded in the cycle-3 alignment addendum |

**Re-measure the sweep yourself.** Rounds 36–37 added tests after that 2386 figure was taken
(Symbolics alone went from 740 to 791 in the last round), so the current total is higher and no one
has measured it end to end. Do not quote 2386 as current.

### Where the memory is

The previous session ran the stretch-goal-harness and left its full memory on disk. **Read these
before doing anything** — they are the difference between a fresh session that resumes and one that
re-derives everything:

- `docs/symbolics/a-plus-cycle-3-report.md` — the cycle report: defects fixed, contract changes, the
  explicit below-A+ list, and what the audit found.
- `docs/symbolics/a-plus-cycle-3-alignment.md` — the approved plan: reproduction matrix,
  decisions D1–D8, section 9.1 wire resolutions, section 13 (the `--omit-display` decision).
- `docs/symbolics/dsh-protocol.md` — the machine protocol.
- `docs/goal-cycle-3/state.md` — round ledger, gate status, **remaining work**.
- `docs/goal-cycle-3/journal.md` — 36 validations, 10 decisions, open risks (RISK-005) and
  questions (OQ-007, OQ-002).
- `docs/goal-cycle-3/evidence.md` — **EVD-001…EVD-147**, every fact the previous orchestrator
  personally verified. Trust this file over any prose summary, including this one.
- `docs/goal-cycle-3/round-*/` — per-round artifacts, logs and probe scripts.

---

## 2. Mandatory protocol

1. **Read the memory above before touching anything.** Confirm with `git rev-parse HEAD`,
   `git status --porcelain`, and by opening `docs/goal-cycle-3/state.md`.
2. **Re-measure before trusting**: full 15-suite sweep, a forced full rebuild, and one published-binary
   probe. The cycle's numbers must be re-established on your tree, not inherited.
3. **Create your own harness directory** — `docs/goal-cycle-4/` with `goal.md`, `journal.md`,
   `evidence.md`, `state.md`, `deliverables.md`. Continue EVD numbering from 148 so the two runs
   never collide. The Cycle-3 memory stays read-only history except for explicit superseding entries.
4. **Resolve the blockers in the order below.** Blocker 1 first: everything else is worthless if an
   accident destroys 37 rounds of uncommitted work.

---

## 3. The three blockers

### Blocker 1 — commit the cycle (needs a human decision)

**State**: HEAD is `35e2609`; ~40 modified or new paths sit in the working tree, including two
kernel correctness fixes, a printer correctness fix, a cancellation fix, a 53× performance fix, a
LaTeX surface, a new test project, a clean build, and two wire revisions.

**What is needed**: `OQ-007<<B> asks whether the P0 wire revisions should land as **their own revertible
commit** before the rest, or as one commit. Ask the user; do not choose for them. The Cycle-2
handoff named "an accident, not a defect" as the largest risk to a finished-but-uncommitted cycle,
and this is the second cycle in a row to end that way.

**Suggested split, if the user wants staging** (verify each stage builds and its suites pass before
committing it):
1. Harness and cycle docs (`docs/goal-cycle-3/**`, `docs/symbolics/a-plus-cycle-3-*.md`).
2. The runner test project and golden fixtures + slnx/CI wiring.
3. The P0 wire revisions (enum kind, Diagnostic vocabulary, bindings, parameter/variable, NoSolutions
   pairing, matrix records) with the regenerated goldens.
4. The correctness fixes (N13/N14 factoring, N15 printer, N17 cancellation, N18 indeterminate forms,
   N19 limit existence) — these are the ones a reviewer will most want isolated.
5. The remaining items (items 6–13, 17) and their tests.

### Blocker 2 — the SymPy differential oracle has never executed anywhere

**State**: it no longer reports a false pass. It **SKIPs** with a reason when sympy is absent, and
**FAILS** when `LOVELACE_REQUIRE_SYMPY=1` is set and the oracle is missing. A CI job
(`.github/workflows/ci.yml`, job `sympy-oracle`) installs sympy and runs it — but **the job has never
run**, and there is no working Python in this environment (`python`/`python3` are Microsoft Store alias
stubs that exit 9009).

**What is needed**: a Python with sympy, or a GitHub runner.

- If the user can install Python/sympy locally: run
  `dotnet test Lovelace.Symbolics.Tests -c Release --filter "FullyQualifiedName~DifferentialOracle"`
  with `LOVELACE_REQUIRE_SYMPY=1` and record the observed result. **This is the single highest-value
  verification left in the project**: the oracle's corpora (differentiation, solve, roots,
  factorisation, limits, matrix ops) have never once been compared against SymPy.
- Otherwise the user must push and you must read the CI result.
- **Do not describe the corpus as verified if no comparison ran.** A locally green run proves only the
  skip mechanism.

### Blocker 3 — the Studio UI has never been loaded in a browser

**State**: the Studio API surface is covered by tests and was exercised in the Cycle-3 audit; the
JS/CSS panels have never been rendered in a browser from this environment. This has been carried as a
below-A+ caveat since Cycle 2.

**What is needed**: either a browser check of the panels against a running Studio host
(`dotnet run --project Lovelace.Studio --urls http://127.0.0.1:<port>`), or an explicit decision to
**retire the claim** rather than carry it forever. Ask which. Do not silently drop it and do not
pretend to have checked it.

---

## 4. Remaining A+ gaps beyond the blockers

These are honest sub-limits recorded in the cycle-3 report. For each, either close it or get a
maintainer decision to accept it — do not leave it ambiguous:

1. `capabilities()` reports `exactness = BestEffort`, not exhaustive. Decide whether that is
   acceptable for A+, and if not, enumerate the rest.
2. The **complex** falsification sweep reaches 5 of the 9 rewrite rules; the **log/exp** sweeps exercise
   reals only. Extend or state the bound.
3. `sqrt(-1)` and `(-1)^(1/2)` remain unsupported. They are **declared structurally** through
   `capabilities()` rather than only discoverable by error, and the frozen root-representation
   decision puts them out of scope. Confirm that is still the right call.
4. Benchmark **means are directional** (non-idle machine). The MediumRun at
   `benchmarks/symbench-longrun.md` has error bars; the ShortRun baseline does not. A quotable
   improvement claim needs an idle machine.

---

## 5. Hard constraints (carry over — these are gates)

1. **Native AOT must keep publishing and running.** `-p:PublishAot=true`.
2. `docs/symbolics/a-plus-convergence-alignment-plan.md` sections C–J are **frozen law**. Amend the
   alignment document rather than reducing scope silently.
3. **No semantically meaningful string in a full machine API.**
4. **No broad `catch (Exception)` in `Lovelace.Symbolics<<B>** (currently 0 — verified). The single
   broad catch in `Lovelace.Suite/SuiteEngine.cs:282` records a diagnostic and rethrows; leave it.
5. `Assumptions.cs` carries a 324-query bound matrix and a 324-pair contradiction matrix. Any change
   there requires the **full** suite, not just Symbolics.
6. **No test weakened, skipped or deleted to reach green.**
7. **Re-publish the AOT binary before auditing it.** Cycle 2 produced two false defects from a stale
   binary.
8. **One bounded change per round, verified in the same round.**

---

## 6. Process lessons from Cycle 3 — each of these cost real time

These are the most transferable findings of the cycle. Ignore them and you will re-pay for them.

1. **Diagnose in source BEFORE dispatching a fix.** Five defects (N13, N14, N17, N18, N19) were fixed
   on the **first dispatch** because the prompt carried a located root cause with file:line. Two
   dispatches that were told to *search* burned roughly ninety minutes and produced nothing. If you
   do not know where the defect lives, either read the source yourself or dispatch a
   *diagnosis-only* round — never a fix round that also has to find the cause.
2. **Attack the system; do not only work the item list.** Fifteen rounds of closing items reported
   green while four defects were live, including a documented feature (cancellation) that did not
   work at all. The adversarial audit found them in minutes. **Schedule the audit early, not last.**
3. **Choose acceptance criteria that are PROPERTIES, not strings.** The printer round was specified
   as `parse(pretty(e))` must canonicalise identically — which caught a rendering that parsed to a
   *different value*, while the item itself was only a complaint about doubled parentheses.
4. **Demand negative controls.** A test that cannot fail is scaffolding. The strongest tests in this
   codebase prove they can fail: an inverse that passes the old numeric check and fails the new
   symbolic one; a deliberately throwing rewrite rule; a removed delimiter that must break the parse.
5. **Beware the probe.** Five separate probe errors cost time this cycle: a budget below the node
   count, a reserved PowerShell parameter name, an undeclared symbol, member access on a Symbolic
   value, and a regex that could not match the token it was looking for. **When a probe fails, suspect
   the probe first.**
6. **A long-enough workload matters.** OQ-002 (does cancellation work?) stayed open for thirty rounds
   because every probe used a workload that finished in about 2 ms.

---

## 7. Environment traps (Windows, this harness)

- **PowerShell 5.1**: no `-Encoding utf8NoBOM` — use `[System.IO.File]::WriteAllText` with
  `UTF8Encoding(false)`. No `ProcessStartInfo.ArgumentList` (use `.Arguments`). Never name a
  function parameter after the automatic arguments variable `$args` — it silently breaks every
  `dotnet` invocation with exit code -2147450751.
- **The code runtime's TypeScript parser** rejects a <<B>$` character inside a template literal, a nested
  template literal inside a call argument, and backticks inside a prompt string. Build long documents
  from arrays of quoted lines, or use a sentinel token and substitute it afterwards.
- **The `read` tool truncates lines at 2000 characters.** Chunk long single-line JSON before reading it.
- **No Python**: `python`/`python3` are Store alias stubs.
- **The Lovelace scripting language** separates statements with `;` — a newline is NOT a separator.
  Declare symbols before use (`x = symbol("x")`) or you get "Undefined variable".
- **A bare `new SuiteEngine()` carries only core builtins.** Symbolics/MathIR/Dsp functions need the
  plugins loaded exactly as `Lovelace.Run/Program.cs` does.

---

## 8. Done, and stop criteria

**Done** means: the three blockers resolved; every item in section 4 either closed or explicitly
accepted by the maintainer in writing; a re-measured full sweep green; a forced full rebuild at zero
warnings; the AOT binary re-published from the final tree and executed; and the adversarial audit
re-run against **those** binaries.

**Stop criteria**: all of the above at high confidence, zero falsified rows, zero P0 open questions.
Round cap: 30.

**Do not self-award A+ while knowingly leaving a category below the bar.** Cycle 3 did not, and that
is why its report is trustworthy. If a category stays open, say so in the final report and say why.

---

## 9. One thing to keep

Cycle 3's most useful habit was cheap and mechanical: **every claim in the final report traced to a
numbered EVD row that the orchestrator had personally reproduced.** Keep that. When a subagent reports
success, that is a hypothesis; when you have run the command yourself and read the output, it is a
fact. The distinction is the entire value of this harness, and it is what turned three
green-but-unverified rounds into two kernel correctness fixes.
