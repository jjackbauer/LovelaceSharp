# Cycle-4 commit plan — the 5-stage split (as committed, and as corrected)

Maintainer decision (this session): the Cycle-3 tree lands as **five staged commits**, with the P0
wire revisions revertible on their own (OQ-007).

## Why the brief's stage order is not used verbatim

The brief suggested: 1 docs → 2 runner test project + goldens + slnx/CI → 3 P0 wire revisions →
4 fixes → 5 remaining items. **Measured evidence makes stages 2→3 in that order impossible to keep
green:**

| Evidence | Consequence |
|---|---|
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:84` asserts `fields["status"]["kind"] == "Enum"` | The runner test project cannot pass until the wire revision exists |
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:39` pins a `capabilities` fixture | It also needs the `capabilities()` item |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj:24` adds a `Lovelace.Run` reference for the capabilities honesty test | The capabilities tests need the new `Lovelace.Run.Runner` seam |
| `Lovelace.Symbolics/SymbolicsPlugin.cs:223` calls `Printing.PrintMode.Latex`, which only exists in the modified `Printing.cs:325` | The wire commit's facade cannot compile before the printer change |
| `SymbolicsPlugin.cs`, `Interpreter.cs` and `DxStructuredResultsTests.cs` each carry concerns from two or three stages | No file-level partition separates them cleanly |

So the series is reordered so that every commit builds and its own suites pass. The wire revision
remains a single revertible commit.

**Assignment rule, applied mechanically:**

- An **implementation** file goes to the **earliest** stage that needs any part of it.
- A **test** file goes to the **latest** stage among the concerns its assertions touch, so it never
  runs before the code it asserts on exists.

**Honest limit of this split:** three files mix concerns at file granularity
(`SymbolicsPlugin.cs`, `Printing.cs`, `Interpreter.cs`). Their commit messages name every concern they
carry, so a reviewer is never misled about what a commit contains. Hunk-level splitting was rejected:
it would produce intermediate file states that were never compiled as such, trading buildability for
tidiness.

## The series as committed

| Stage | Commit | Contents |
|---|---|---|
| 1 | `b1fb67f` | Harness memory, cycle report, alignment addendum, Cycle-4 brief, long-run benchmark records. No product code. |
| 2 | `84a6a54` | The six correctness fixes (N13, N14, N15, N17, N18, N2) plus `Printing.cs`/`README.md`, which the facade needs. |
| 3 | `160ba4e` | **The P0 wire revisions** — `Enum` kind, structured `Diagnostic`, `bindings`/`Symbol` records, NoSolutions pairing, matrix records, protocol doc. |
| 4 | `3367447` | Remaining items 6–13, 17, the runner seam and analyzer hygiene. |
| 5 | `3992d80` | The runner test project, golden fixtures, slnx and CI wiring. |

## Per-stage verification (TODO-004) — measured in clean worktrees

| Stage | Build | Suites |
|---|---|---|
| 2 `84a6a54` | succeeded, 106 warnings | Symbolics 532/0, Suite 620/0, Integer 148/0, Natural 195/0 |
| 3 `160ba4e` | **FAILED — `CS0117`** | not runnable |
| 4 `3367447` | succeeded, **0 warnings / 0 errors** | Symbolics 791/0 (6 skipped), Suite 642/0 |

**Result: RISK-001 materialized and was caught by the check that exists for it.** Stage 3's facade
calls `RationalFunctions.CancelWithConditions`, and `RationalFunctions.cs` was committed one stage
later. Fix (DEC-006): move that one file into stage 3 and rebuild stages 3–5. The rewrite is
deliberately blocked until every in-flight implementer round has settled, so that an agent's
work-in-progress cannot be captured into a historical commit (RISK-003).

Stage 5 is the tree the full 15-project sweep was measured on, so it is verified by EVD-156 rather
than by a separate worktree: **2439 passed / 0 failed / 6 skipped**.
