# A+ Cycle 4 — report (DRAFT, in progress)

> Companion documents: **a-plus-cycle-3-report.md** (the previous cycle), **a-plus-cycle-3-alignment.md**
> and **a-plus-convergence-alignment-plan.md** (sections C–J, treated as law), **dsh-protocol.md**
> (the machine protocol), and the harness records under `docs/goal-cycle-4/`
> (`goal.md`, `journal.md`, `evidence.md`, `state.md`, `deliverables.md`, `commit-plan.md`,
> per-round artifacts).

## Headline

Cycle 3 ended with three categories below A+ and all three blocked on resources it did not have.
**All three are now closed**, and the highest-value verification in the project — the SymPy
differential oracle — **executed for the first time in its history**. It immediately found a defect
in itself, and once that was fixed it found a wrong answer in the numeric core.

## 1. The three blockers

### 1.1 The cycle is committed (was RISK-005 / OQ-007)

The uncommitted tree was first protected by a **non-destructive safety net**: `git add -A` →
`git write-tree` → `git commit-tree -p HEAD` → tag `cycle-3-safety-net`
(`8539cf4e46aa760cbd574664a6a152349f64b3ba`, tree `ab26e1388d2900b18c2f6b76b6ca692e3c39c1ca`), then
`git reset --mixed HEAD`. No file was ever moved, so no failure mode of the net itself could lose
work. The dirty count was 78 before and after (EVD-149, EVD-150).

The maintainer chose the five-commit split with the P0 wire revisions revertible on their own. The
brief's suggested *order* could not be kept green, and the reordering is documented with the
measurement that forced it (`commit-plan.md`, DEC-003, VAL-001, EVD-158):

| Stage | Commit | Contents |
|---|---|---|
| 1 | `b1fb67f` | Harness memory, cycle report, alignment addendum, Cycle-4 brief, benchmark records |
| 2 | `84a6a54` | The six correctness fixes (N13, N14, N15, N17, N18, N2) |
| 3 | `160ba4e` | **The P0 wire revisions** (Enum kind, structured Diagnostic, bindings/Symbol records, NoSolutions pairing, matrix records) |
| 4 | `3367447` | Remaining items 6–13, 17, the runner seam, analyzer hygiene |
| 5 | `3992d80` | The runner test project, golden fixtures, slnx and CI wiring |

Per-commit verification in clean `git worktree`s found that **stage 3 did not compile on its own** —
`SymbolicsPlugin.cs:359` calls `RationalFunctions.CancelWithConditions`, committed one stage later
(EVD-173, VAL-003). RISK-001 predicted exactly this. The fix moves that one file into stage 3 and
rebuilds stages 3–5 (DEC-006); the rewrite is deliberately serialized behind the in-flight rounds so
agent work-in-progress cannot be captured into a historical commit (RISK-003).

### 1.2 The SymPy differential oracle has now executed

A self-contained CPython 3.12.14 was fetched into `C:\Users\ricar\dev\.lovelace-tools\` (no system
install, no PATH change) with sympy 1.14.0, and `python3.exe` was provided because `SympyOracle.cs:109`
spawns the literal name `python3`. The machine had working network egress all along (EVD-151, EVD-152).

**First execution ever: `Failed 2 / Passed 4 / Skipped 0 / Total 6`** (EVD-160). Both failures were
one root cause, and it was in the oracle, not the kernel:

- **N20** — the generated SymPy program's prelude was `import sympy` only, so every name the corpora
  use unqualified (`sin`, `cos`, …) raised `NameError`. The derivative and limit corpora were
  therefore **comparing nothing at all**, and the failure surfaced as "the SymPy side did not
  evaluate". Fixed at `SympyOracle.cs` by exposing sympy's namespace in the prelude (DEC-004). This
  widens what the oracle can compare and weakens no assertion.

With that fixed the oracle compares for real — **57 corpus cases across 6 families** (6 derivatives,
4 solve, 8 roots, 9 factorisations, 17 limits, 13 matrix operations), each sampled at multiple
rational points — and it immediately disagreed with the kernel on one case (EVD-161):

```
diff(tan(x**3)) at x = 1/3
  kernel 3*x^2*cos(x^3)^(-2) = 0
  sympy                      = 0.333790999179746492481090015847
```

The kernel's *derivative is correct*; the **numeric evaluation** was wrong (EVD-162). Six probes
outside the repo isolated it to `Lovelace.Real` division (EVD-163, EVD-164):

| Expression | Kernel | SymPy truth |
|---|---|---|
| `10/9` | 1.1111… ✅ | 1.1111… |
| `1/0.9` | **1** ❌ | 1.1111… |
| `1/0.99` | **1** ❌ | 1.0101… |
| `1/0.9993142073433579945` | **1** ❌ | 1.000686… |
| `1/0.99862888499828389309965043538706203025` | **0** ❌ | 1.001373… |

**N21** (wrong quotients for divisors just below 1) and **N22** (`Real.ToString()` throwing
`ArgumentOutOfRangeException` at `Real.cs:1764` on a 2-digit repeating expansion, **user-visible**:
the published runner returns an error envelope for the script `1 / 0.99`) are both **pre-existing**:
the identical probe against a clean worktree at `35e2609` prints identical results (EVD-165). Also
recorded: **N23**, `cos(1/2)` costing 44 s at the default 1000-place precision (EVD-168) — which is
what made two earlier probes look like hangs.

> Both directions matter here. The oracle is only worth its runtime if it is allowed to disagree, and
> this one disagreement is the most consequential finding of the cycle: 2439 tests were green while
> `1/0.9` was wrong in the numeric core.

### 1.3 The Studio UI is browser-verified

The UI had never been loaded in a browser from this environment in two cycles. Chrome 152 and Edge
were installed all along (EVD-152). Driving headless Chrome over the DevTools Protocol against a live
Studio host (EVD-169):

| Check | Observation |
|---|---|
| Browser | Chrome/152.0.7977.77, headless |
| Module + framework init | CodeMirror mounted, all 4 panes visible, stylesheet applied |
| Quick-eval round trip | typed `2^10 + 1` + Enter → `Natural = 1025 (Natural)` … `done: 59.09 ms` |
| Symbolic round trip | `x = symbol("x"); factor(x^2 - 1)` → `(x - 1)*(x + 1)`, canonical, domain complex, exact, 7 nodes |
| Workspace table | rendered rows for `_` and `x` |
| Page exceptions | **0** |
| Console errors | 1 — a network `404` for `/favicon.ico` (EVD-170), not an application fault |

The host log independently shows the page polling `/api/completions`, `/api/state` and
`/api/run/{runId}`, so the browser really drove the application rather than merely rendering it.

## 2. Measurements re-established on this tree

| Measurement | Result | Evidence |
|---|---|---|
| Forced full rebuild (`--no-incremental`) | **0 warnings / 0 errors** (12.08 s) | EVD-155 |
| Full 15-project sweep, as received | **2439 passed / 0 failed / 6 skipped** (99.4 s) | EVD-156 |
| Cycle-3 report's 2386 | **stale** — Symbolics 740 → 791, Suite 640 → 642 | EVD-157 |
| Pre-cycle tree warnings | ~30 `CS0109` + `CS8767`/`CS8600`, removed by 43 annotation-only lines | EVD-171 |

## 3. Maintainer decisions this session

1. Five staged commits, wire revisions revertible alone — delivered, with the order corrected by
   measurement.
2. Fetch Python + sympy locally and run the oracle — done.
3. Browser-verify and keep the claim — done, and now a verified fact rather than a caveat.
4. Section-4 scope, in favour of the ambitious option on all five items:
   exhaustive capability list (§4.1); complex treatment for all nine falsification rules (§4.2);
   reopen `sqrt(-1)`/`(-1)^(1/2)` (§4.3); error bars on the ShortRun baseline (§4.4); fix the
   high-precision `cos` path (N23).
   Two of these reinterpret the literal choice on mathematical grounds, disclosed rather than
   silently applied: §4.2 is implemented as the sound maximum (5 rules complex-sampled + 4 rules with
   a negative control proving the exclusion necessary, DEC-007) because a complex value cannot
   satisfy an order predicate and manufacturing such a counterexample would break the gate; and §4.3
   is bounded to exactly-representable complex values (DEC-008) because the implementer established
   that `2^(1/2)` fails for the same reason as `(-1)^(1/2)`, making the literal item "general
   rational exponents" — a much larger feature than the §4.3 wording.

## 4. PENDING (filled in as the remaining rounds land)

- §4.1/§4.3 kernel round result and the amended scope boundary in the alignment document.
- §4.2 falsification partition (nine rules, with dispositions).
- N21/N22 fix verified by the oracle at `Failed 0 / Passed 6 / Skipped 0`.
- N23 high-precision `cos` fix with before/after timings.
- Re-measured sweep and forced rebuild on the final tree.
- ShortRun benchmark error bars (§4.4), with the non-idle caveat stated.
- Re-published AOT binary, freshness guard, and the five CI smoke scenarios run locally.
- The adversarial audit (seven personas) against the re-published binary.
