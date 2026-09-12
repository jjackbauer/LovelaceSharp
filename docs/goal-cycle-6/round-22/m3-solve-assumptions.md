# M-3 (P1) — `solve()` ignored a session `assume()` that `simplify()` honours

**Round 22 · fixer report · worktree `.worktrees/r22-m3` (branch `r22-m3`)**

| | |
|---|---|
| Base | `19ae61e` (`main` at the time of the fix) |
| Fix commit | **`4f441aa39b41484f68d4f20d3a54f3231abc6405`** |
| Control worktree | `.worktrees/r22-m3-ctl` (detached at `19ae61e`), new tests only, **not committed** |
| Files changed | `Lovelace.Symbolics/SymbolicsPlugin.cs`, `docs/symbolics/dsh-protocol.md`, new `Lovelace.Run.Tests/SolveSessionAssumptionTests.cs` |
| Build | `dotnet build LovelaceSharp.slnx -c Release` → **0 Warning(s), 0 Error(s)** |
| Tests | `dotnet test LovelaceSharp.slnx -c Release` → **15/15 projects passed, 0 failed** (5606 passed, 8 skipped) |

---

## 1. The decision: **(a) — `solve` must respect the session assumptions.** Pinned.

The store was already installed and already live when the solver ran; the solver simply never read it.
Three independent documents decide the question, and they all point the same way.

**1. `assume*` is documented as session state with no exemption.** The builtin descriptors say so in
so many words — `Lovelace.Symbolics/SymbolicsPlugin.cs:129-139`:

> `"assume_positive"` … *"Assumes x > 0 **for the rest of the session**."*
> `"assume_real"` … *"Assumes x is real-valued **for the rest of the session**."*

Nothing in the descriptor surface excepts the solver, and the sentence "the rest of the session" is a
claim about every later statement in the session — including `solve`. Option (b) would leave that
sentence false for one builtin.

**2. The protocol already defines a `NoSolutions` answer produced by a domain condition that removed
every candidate.** `docs/symbolics/dsh-protocol.md:207-211`:

> *"`NoSolutions` means the solution set over the **requested domain is provably empty** … over the
> requested domain, **including the case where a denominator or domain condition removed every
> candidate**. A provably empty set is a complete answer, so it reports `complete: true` /
> `completeness: Complete`."*

A session `assume(x > 5)` **is** a domain condition, and the kernel's own `AcceptedSolutions` already
drops candidates its own conditions removed. The assumption-driven case is the same case with the
condition coming from the other side of the seam.

**3. The solver's own completeness invariant makes the observed record false.** `Solvers/Solve.cs:11-24`
and `:94-95`: *"`Status == Solved` implies the represented set is complete over `Domain`."* With
`assume(x > 5)`, the set `{3}` is **not** the complete solution set of `x == 3` in that session, so
`Solved`/`complete: true` is a false claim in the machine API — which is exactly the P1 the audit
graded. Option (b) would not repair that; it would only *document* a contract that contradicts the
completeness invariant. And the store is not merely "a store `simplify` happens to read": `EngineSession.Run`
(`SymbolicsPlugin.cs:209-233`) installs it **flow-locally around every registered builtin**, `solve`
included, so the solver was reading from a context that already held the assumptions and ignoring them.

Documentation basis in one line: the store is documented as session-wide, the protocol counts a
domain-condition-emptied set as `NoSolutions`/`complete: true`, and the solver's own `Solved`
invariant forbids publishing a set the session's domain excludes.

### Why not (b)
Under (b) the solver would still publish `complete: true` for a set that is not complete in the
session it was called in, so the machine API would still be lying; (b) only moves the lie into prose.
It would also force a change to the `solve` descriptor text — see §5, which is pinned verbatim by an
existing test that does **not** pin the bug.

---

## 2. The fix

`SymbolicsPlugin` is where the session store meets the solver, and it is where the kernel's answer is
turned into the published record. The kernel (`Solvers.Solve`, `SystemSolvers.Solve`) is left a pure
function of *(equation, variable, domain)*; the plugin intersects its answer with the **active**
assumptions (which, inside a builtin call, are this engine's store — `EngineSession.Run`).

* `AcceptedSolutions` (`SolveResultRecord`) now judges every root against every active atom with a
  three-valued verdict:
  * **refuted** → the root is dropped. A set all of whose roots are dropped becomes the
    `NoSolutions`/`complete: true` record the builder already derives from an empty accepted list.
  * **undecided** → the root is kept and the atoms that could not be decided ride in its per-solution
    `conditions` (`AssumptionSet.FromAtoms`, never `Add`, so a union can never throw).
  * **satisfied** → **nothing changes**: an assumption that admits every root publishes byte-for-byte
    what an assumption-free session publishes.
* The same verdict is applied to `solve_system`/`solve_system_full` per assignment
  (`AcceptedSystemSolutions`: one refuted binding drops the whole assignment; undecided atoms become
  its conditions; an assignment set emptied by the store reports `NoSolutions`/`complete: true`).
* A parametric family cannot be tested one value at a time, so the active atoms that constrain the
  solved variable are projected onto the family's `conditions` (`SolutionResult` family projection).
* An assumption about **another** symbol is not a condition on this solve
  (`AmbientAtomsOn` filters by the solved variable's name) — pinned by a test.
* Deciding one atom at one value (`HoldsAt`): the atom's relation form is substituted and evaluated
  with the kernel's own `Evaluation.EvaluateCondition` (which consults the same lattice first);
  a domain atom is decided on the domain lattice (`Domains.DomainOf` ≤ the assumed domain), with two
  proofs of exclusion and **no guesses** — a concrete complex value against a real-or-narrower domain,
  and an exact non-integral rational against `Integer`. `ProvablyNonReal` is deliberately narrow
  (the imaginary unit, a complex literal, and a product with exactly one such factor): `i*i = -1` and
  `exp(i*pi) = -1` are real, so "mentions i" is not a test, and anything unrecognised stays undecided
  and becomes a condition instead of a rejection.
* No new refusal, no new error code, no `catch (Exception)`: the change only decides what a record
  publishes; every path that returned a record still returns a record.

---

## 3. Test-first: the new tests FAIL on `19ae61e` (control) and PASS on `4f441aa`

New test file `Lovelace.Run.Tests/SolveSessionAssumptionTests.cs` (10 tests) was written in
`.worktrees/r22-m3-ctl` **first** and run there. Nothing else was changed in the control tree.

### 3.1 Control (19ae61e) — `.worktrees/r22-m3-ctl`

    PS> dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release `
          --filter FullyQualifiedName~SolveSessionAssumptionTests `
          --logger "console;verbosity=detailed" --nologo

    Failed  ...Solve_DoesNotPublishARootTheSessionAssumptionExcludes
      Expected: "NoSolutions"
      Actual:   "Solved"
    Failed  ...Solve_KeepsOnlyTheRootsTheAssumptionAdmits
      Expected: ["2"]
      Actual:   ["-2", "2"]
    Failed  ...Solve_HonoursAnEqualityAssumption
      Expected: ["2"]
      Actual:   ["-2", "2"]
    Failed  ...Solve_HonoursADomainAssumption
      Expected: "NoSolutions"
      Actual:   "Solved"
    Failed  ...OneSession_OneAssumptionStore_ForSimplifyAndSolve
      Expected: ["2"]
      Actual:   ["-2", "2"]
    Failed  ...AResponseThatCannotDecideIsCarriedAsACondition
      Expected: ["x > 5"]
      Actual:   []
    Failed  ...SystemSolve_HonoursTheSameStore
      Expected: "NoSolutions"
      Actual:   "Solved"
    Passed  ...AnAssumptionThatAdmitsEveryRootPublishesTheSameSet
    Passed  ...AnAssumptionOnAnotherSymbolChangesNothing
    Passed  ...WithoutAssumptionsTheSolverPublishesExactlyWhatItDidBefore

    Total tests: 10
         Passed: 3
         Failed: 7

The three that pass on the control are the **controls** (an assumption that admits every root, an
assumption on another symbol, and no assumption at all): they are what proves the fix does not
over-reach.

The whole `Lovelace.Run.Tests` project on the control tree fails **only** on those 7 new tests —
nothing pre-existing was red, and nothing pre-existing was hidden:

    PS> dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo     [r22-m3-ctl]
    Failed!  - Failed: 7, Passed: 334, Skipped: 0, Total: 341 ... Lovelace.Run.Tests.dll

(341 − 10 new = the 331 tests the project had at `19ae61e`; on the fix tree the same project is
341 passed / 0 failed.)

### 3.2 The audit's own scripts, same commands, control vs fix

Built runner `Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe` in each worktree (the scripts go
through `--file`, per the PS 5.1 quoting rule), command:

    PS> & <worktree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe `
          --file <scratch>\z2_contra_solve.ls --json --omit-functions --omit-variables

| script (audit M's text) | control `19ae61e` | fix `4f441aa` |
|---|---|---|
| `x = symbol("x"); assume(x > 5); solve(x == 3, x)` | `Solved, complete: True, solutions: [3], conditions: []` | **`NoSolutions, complete: True, completeness: Complete, solutions: []`** |
| `x = symbol("x"); assume_positive(x); solve(x^2 == 4, x)` | `[-2, 2]` | **`[2]`** |
| `x = symbol("x"); assume(x < 0); solve(x^2 == 4, x)` | `[-2, 2]` | **`[-2]`** |
| `x = symbol("x"); assume(x > 0); simplify(sqrt(x^2))` | `x` (unchanged) | `x` (unchanged — the shared store still decided here) |
| `x = symbol("x"); assume(x > 5); solve_system([x == 3], [x])` | `Solved, solutions: [Binding(x, 3)]` | **`NoSolutions, complete: True, solutions: []`** |
| `x = symbol("x"); solve(x^2 == 4, x)` (no assumption) | `[-2, 2]` | `[-2, 2]` (unchanged) |

Exact fixed envelopes (the deciding fields):

    z2  exit=0 ok=True
    SolveResult(status: NoSolutions, variable: x, domain: complex, complete: True, completeness: Complete,
      solutions: [], families: [], common_conditions: [], represented_count: 0, unrepresented_count: 0,
      unrepresented_reason: , diagnostics: [])
    z1  SolveResult(status: Solved, ..., solutions: [Solution(value: 2, conditions: [], multiplicity: 1, exactness: Exact)], ...)
    z3  SolveResult(status: Solved, ..., solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact)], ...)
    z5  SystemSolveResult(status: NoSolutions, domain: complex, complete: True, completeness: Complete, solutions: [], diagnostics: [])

### 3.3 Fix tree — the same 10 tests

    PS> dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release `
          --filter FullyQualifiedName~SolveSessionAssumptionTests --logger "console;verbosity=detailed" --nologo

    Passed  ...OneSession_OneAssumptionStore_ForSimplifyAndSolve
    Passed  ...SystemSolve_HonoursTheSameStore
    Passed  ...AResponseThatCannotDecideIsCarriedAsACondition
    Passed  ...Solve_DoesNotPublishARootTheSessionAssumptionExcludes
    Passed  ...Solve_HonoursADomainAssumption
    Passed  ...WithoutAssumptionsTheSolverPublishesExactlyWhatItDidBefore
    Passed  ...Solve_HonoursAnEqualityAssumption
    Passed  ...Solve_KeepsOnlyTheRootsTheAssumptionAdmits
    Passed  ...AnAssumptionThatAdmitsEveryRootPublishesTheSameSet
    Passed  ...AnAssumptionOnAnotherSymbolChangesNothing

    Total tests: 10
         Passed: 10

---

## 4. Full-project totals (fix tree `.worktrees/r22-m3`)

`dotnet build LovelaceSharp.slnx -c Release --nologo`

    Build succeeded.
        0 Warning(s)
        0 Error(s)

`dotnet test LovelaceSharp.slnx -c Release --nologo`

| project | passed | failed | skipped | total |
|---|---|---|---|---|
| Lovelace.Abstractions.Tests | 20 | 0 | 0 | 20 |
| Lovelace.Representation.Tests | 91 | 0 | 0 | 91 |
| Lovelace.Array.Tests | 19 | 0 | 0 | 19 |
| Lovelace.Natural.Tests | 195 | 0 | 0 | 195 |
| Lovelace.Integer.Tests | 148 | 0 | 0 | 148 |
| precbench.Tests | 13 | 0 | 0 | 13 |
| Lovelace.Knowledge.Tests | 28 | 0 | 0 | 28 |
| Lovelace.Console.Tests | 15 | 0 | 0 | 15 |
| Lovelace.Complex.Tests | 121 | 0 | 0 | 121 |
| Lovelace.Real.Tests | 2495 | 0 | 0 | 2495 |
| Lovelace.Dsp.Tests | 61 | 0 | 0 | 61 |
| Lovelace.Suite.Tests | 852 | 0 | 0 | 852 |
| Lovelace.Run.Tests | 341 | 0 | 0 | 341 |
| Lovelace.Symbolics.Tests | 1185 | 0 | 8 | 1193 |
| Lovelace.Studio.Tests | 22 | 0 | 0 | 22 |
| **total** | **5606** | **0** | **8** | **5614** |

The 8 skips are the SymPy differential-oracle tests, which `[SKIP]` themselves when the oracle is not
configured; they are skipped on `19ae61e` too and are unrelated to this change.
`Lovelace.Run.Tests` grew from 331 to 341 by exactly the 10 new tests.

---

## 5. Existing tests: does any pin the current behaviour?

**No test pins "solve ignores the assumptions".** `assume`/solver co-occurrence was swept across
every test project: `Lovelace.Symbolics.Tests` (13 files) and `Lovelace.Run.Tests` (3 files) mention
assumptions only in simplify/inspect/isolation contexts; `Lovelace.Suite.Tests` uses them for
cross-engine isolation (`CrossEngineIsolationTests`, `SharedPluginInstanceAssumptionIsolationTests`)
and `assume(x == 0); simplify_full(x/x)`; `Lovelace.Studio.Tests` uses `assume_positive` for the
inspection panel; `Lovelace.Console.Tests` and `Lovelace.Complex.Tests` have none. No test, and no
document, states that the solver ignores the store — so nothing was weakened, skipped, deleted or
re-pinned.

**One test is worth naming loudly, because it is what makes option (b) expensive.**
`Lovelace.Console.Tests/ReplOutputTests.cs:57-78` (`Help_Solve_ShowsSignatureAndSummary`) pins the
**entire `solve` descriptor sentence verbatim** with `AssertHasLine`:

    "Solves an equation for x and returns the SAME SolveResult record solve_full returns: "
    + "status, domain (a Domain value), complete/completeness, per-solution "
    + "conditions/multiplicity/exactness, parametric families and diagnostics. The default "
    + "domain is Complex; pass real for real solutions only."

That test does **not** pin the bug — it pins help text. Option (b) would have required editing the
descriptor and therefore **re-pinning this passing test**, which the round's hard rules forbid. Under
(a) the descriptor needs no change at all: "assumes x > 0 for the rest of the session" becomes true,
which is the point. (The protocol paragraph added in `docs/symbolics/dsh-protocol.md` is prose with no
json fence, so the doc-vs-binary test `BuiltinSurfaceContractTests.ProtocolDocumentExamples_MatchTheBinary`
— which reads the first `json` fence after two specific headings — is unaffected; it passes in the
full run above.)

---

## 6. Not done, and why

1. **The published AOT runner was not re-published.** `out/aot/Lovelace.Run.exe` in the main tree
   still has the pre-fix behaviour; the main tree must not be edited from this worktree, and the
   re-publish belongs to the wave's own step. Every "after" number above comes from the runner built
   from `4f441aa` (`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.exe`).
2. **An assumption-emptied set carries an empty `diagnostics` array.** `assume(x > 5); solve(x == 3, x)`
   now answers `NoSolutions` / `complete: true` / `diagnostics: []`. That is a *true* answer (the set
   is provably empty and complete) and it matches the record's existing convention that a proved-empty
   set carries no note — but there is no kernel note explaining *why* it is empty, because the kernel
   never saw the assumptions. A dedicated diagnostic code/message would be new contract surface and
   was deliberately not invented in this round; the machine-readable answer (`status` + `completeness`)
   is already exact.
3. **`ProvablyNonReal` is intentionally incomplete**, so `assume(x > 0); solve(x^2 + 1 == 0, x)` drops
   the canonical `i`/`-i` shapes (the multiply-with-one-complex-factor arm) but a contrived
   real-valued expression of a complex argument would stay undecided and be carried as a condition
   rather than rejected. Under-rejecting never produces a false claim; over-rejecting would.
4. **The audit's EVD-329 wording "M-3 (P1)" is now stale** for `solve`/`solve_full`/`solve_system`;
   `docs/goal-cycle-6/evidence.md` was not edited from this worktree.

## 7. Reproduction

    git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/r22-m3     -b r22-m3 19ae61e
    git -C C:\Users\ricar\dev\LovelaceSharp worktree add .worktrees/r22-m3-ctl          19ae61e
    # control: copy Lovelace.Run.Tests/SolveSessionAssumptionTests.cs into r22-m3-ctl and run the
    #          filtered test command of §3.1 (7 failed / 3 passed)
    # fix:     the same file arrives with commit 4f441aa (10 passed)
