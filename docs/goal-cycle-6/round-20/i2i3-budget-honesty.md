# I-2 / I-3 (both P2) — evalf's digit clamp and a mid-token abbreviation are now caller-visible

Round-20 audit I fixer landing (budget honesty). Fix worktree
`.worktrees/r20-budgetdocs` (branch `r20-budgetdocs`), control tree
`.worktrees/r20-budgetdocs-ctl` (branch `r20-budgetdocs-ctl`), both created from the brief's HEAD
`9852a2f` and nothing else (the main tree has since moved on to `6f7e371`; this worktree was not
rebased and contains no other work).

Commit: **`cf99d48`** (`cf99d48a59e0ba0d15baa72a778ad095b6e7123f`, branch `r20-budgetdocs`, **not
pushed, not merged**). 9 files, +587/-26:

| file | what |
| --- | --- |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | the evalf cap is a named constant, a clamped value carries the clamp notice, the published summary states the real bound |
| `Lovelace.Real/Real.cs` | `Real.ClampNotice` (provenance, carried by the copy constructor) + `Real.AsClamped` |
| `Lovelace.Suite/StructuredProjection.cs` | publishes `truncated`/`truncationReason`/`budget` for a clamped Real/Complex value; the projection's own token-boundary cut |
| `Lovelace.Symbolics/Printing.cs` | `SafeCut` cuts at a token boundary in every case |
| `docs/symbolics/dsh-protocol.md` | invariant 4 documents the `digit-cap` marker and the never-mid-token rule |
| 4 new test files | 14 tests; 8 of them fail on `9852a2f` (see below) |

Nothing was removed, weakened, skipped or re-pinned: no existing test file was touched.

---

## I-2 — evalf's 1000-place cap was SILENT and its own summary denied it

### The contract that was broken (HEAD quotes)

* `Lovelace.Symbolics/SymbolicsPlugin.cs:1735` — `int places = Math.Min(digits, 1000);` (its twin at
  `:1618`). The cap is a hard-coded 1000 and does **not** follow the engine cap the same run raised
  (`setprecision(5000)` set `Real.MaxComputationDecimalPlaces` to 5000).
* `Lovelace.Symbolics/SymbolicsPlugin.cs:498-500` (HEAD summary, published by `help`/`capabilities`):
  "…and the published value is **bounded back to the N places that were asked for**" — false for
  N > 1000.
* The evalf body's own comment (`:475-477`, HEAD): "A digit count that cannot be honoured is an
  ARGUMENT problem … **never as a silently rounded/ignored count**". The clamp was exactly that
  silent count.
* `docs/symbolics/dsh-protocol.md:17-20` (HEAD): a bounded rendering reports `truncated: true`,
  `truncationReason` and `budget` — "never silently".
* EVD-276 (`docs/goal-cycle-6/evidence.md:48`) records the 1000-place bound itself as the operative,
  documented semantics. **Kept, as the brief requires.**

### Control (9852a2f) — verbatim

(Long digit runs are elided as `…` or `…(N)` in the pasted output; the raw logs are
`ctl-run3.txt`, `ctl-sym3.txt` and `ctl-sym4.txt` in the control tree.)

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\r20-budgetdocs-ctl
:: ev.ls contains exactly:  setprecision(5000); evalf(sqrt(2), 5000)
dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file ev.ls --json --omit-functions --omit-variables

CONTROL evalf: {"decimals":1000,"exact":false,"truncated":"ABSENT","truncationReason":"ABSENT",
                "budget":"ABSENT","structuredKeys":["kind","value","exact"]}

:: ctl.ls contains exactly:  setprecision(5000); sqrt(2)
CONTROL sqrt : {"decimals":5000,"truncated":"ABSENT"}
```

1000 decimals delivered, no `truncated`/`truncationReason`/`budget` (the keys are not even present),
no diagnostic — while the same binary's `sqrt(2)` carries 5000. This is the finding, reproduced here
from the control tree's own binary.

### The fix

1. **The value says it was clamped.** `Lovelace.Real.Real` gains
   `DigitClampNotice? ClampNotice` (a `(Reason, Budget)` pair) and `Real.AsClamped`, which attaches
   the notice to a **copy** (the `AsInexact` precedent: a producer never mutates a value under its
   owner's feet). The copy constructor carries the notice; equality, comparison, hashing and
   `ToString` ignore it (provenance, not value — the `IsExact` precedent). Arithmetic deliberately
   does not propagate it: a computed result is a new value.
2. **evalf sets it exactly when the cap bit.** `EvalfPayload` computes
   `places = Math.Min(digits, EvalfDecimalPlaceCap)` and marks a value when
   `digits > cap && !value.IsExact` — a request above the cap whose answer is a truncation, so the
   places past the cap are not there whatever the request said. The **cap itself is unchanged**
   (it is now a named constant used by both clamp sites) and the working precision is unchanged, so
   no digit moves. An **exact** answer to an over-cap request (`evalf(1/2, 5000)` = `0.5`) is NOT
   marked: nothing was dropped, so nothing is claimed.
3. **The wire publishes the protocol's own three fields.** `StructuredProjection` maps the notice to
   `truncated: true`, `truncationReason: "digit-cap"`, `budget: 1000` for Real values, and reports
   the same on a Complex value whose components were clamped. `display`, `typed`, the digits and
   `exact` are untouched.
4. **The summary states the real bound** (`help("evalf")`/`capabilities()` share this text): "…and the
   published count is bounded by the builtin's own 1000-place computation cap: a request above 1000
   answers a value carrying at most 1000 decimal places (a request the builtin CAN honour in full is
   never reduced, and an exact answer stays exact), so the value that was cut says so on the wire —
   truncated: true, truncationReason: digit-cap, budget: 1000 — instead of crossing as if the count
   had been honoured."
5. `docs/symbolics/dsh-protocol.md` invariant 4 documents the value-level `digit-cap` marker.

### After (same commands, fix worktree)

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\r20-budgetdocs
dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval "<script>" --json --omit-functions

evalf(sqrt(2),5000) under 5000: {"decimals":1000,"exact":false,"truncated":true,"reason":"digit-cap","budget":1000}
sqrt(2) under 5000          : {"decimals":5000,"exact":false,"truncated":"ABSENT","reason":"ABSENT","budget":"ABSENT"}
evalf(pi,1000)              : {"decimals":1000,"exact":false,"truncated":"ABSENT","reason":"ABSENT","budget":"ABSENT"}
evalf(1/2,5000)             : {"decimals":1,"exact":true,"truncated":"ABSENT","reason":"ABSENT","budget":"ABSENT"}
evalf(1/3,1001)             : {"decimals":1000,"exact":false,"truncated":true,"reason":"digit-cap","budget":1000}
```

### Test evidence

New: `Lovelace.Run.Tests/EvalfDigitCapHonestyTests.cs` (5 tests) and
`Lovelace.Symbolics.Tests/EvalfDigitCapDescriptionTests.cs` (2 tests).

Control, verbatim (the failing ones):

```
  Failed Lovelace.Run.Tests.EvalfDigitCapHonestyTests.ARequestBeyondTheCap_IsMarkedAsClamped [224 ms]
  Error Message:
   the 1000-place clamp crossed with no truncation flag

  Failed Lovelace.Run.Tests.EvalfDigitCapHonestyTests.TheSmallestOverCapRequest_IsMarked [5 ms]
  Error Message:
   a 1001-place request crossed as if the count had been honoured:
   {"kind":"Real","value":"0.3333…(1000 threes)","exact":false}

  Failed Lovelace.Symbolics.Tests.EvalfDigitCapDescriptionTests.TheSummary_NoLongerDeniesTheClamp
  Error Message:
   Assert.DoesNotContain() Failure: Sub-string found
   Found:  "bounded back to the N places that were as"

  Failed Lovelace.Symbolics.Tests.EvalfDigitCapDescriptionTests.TheSummary_NamesThePlaceCapAndTheMarkerThatReportsIt
  Error Message:
   Assert.Contains() Failure: Sub-string not found
   Not found: "1000"

Failed!  - Failed:     4, Passed:     4, Skipped:     0, Total:     8 - Lovelace.Run.Tests.dll (net10.0)
Failed!  - Failed:     4, Passed:     2, Skipped:     0, Total:     6 - Lovelace.Symbolics.Tests.dll (net10.0)
```

The 6 that pass on control are the boundary controls that must pass on both sides: a plain
`setprecision(5000); sqrt(2)` (5000 decimals, unmarked), `evalf(pi, 1000)` (at the cap), an exact
`evalf(1/2, 5000)`, and the two `evalf`-free printer controls listed under I-3.

Fix, same filters:

```
Passed!  - Failed:     0, Passed:     8, Skipped:     0, Total:     8 - Lovelace.Run.Tests.dll (net10.0)
Passed!  - Failed:     0, Passed:     6, Skipped:     0, Total:     6 - Lovelace.Symbolics.Tests.dll (net10.0)
```

The two existing suites the brief named are untouched and still pass:
`Lovelace.Run.Tests/StructuredRealPrecisionTests.cs` (including
`Assert.Null(structured["truncated"])` for `setprecision(1100); pi(1100)` and the 1000-place
`evalf(pi, 1000)` case) and `Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs` (the typed
`InvalidArgument`/`TypeMismatch` refusal for non-positive / out-of-range counts — unchanged, because
a count above the cap is still **accepted and clamped**, not refused).

---

## I-3 — a budgeted truncation split an identifier mid-token

### The contract that was broken (HEAD quotes)

* `Lovelace.Symbolics/Printing.cs:340-345` (HEAD) — "A rendering that exceeds the budget is
  abbreviated (**never silently truncated mid-token**)".
* `Lovelace.Symbolics/Printing.cs:402-409` (HEAD) — `SafeCut`: "Cuts at a token boundary so a number
  or identifier is never split mid-token", whose body ended `return i > 0 ? i : cut;` — the raw cut,
  i.e. exactly the split it promises never to make, whenever the character allowance
  `Math.Max(48, limit × CharsPerNode)` falls inside the rendering's FIRST token.
* `Lovelace.Suite/StructuredProjection.cs:158-166` (HEAD) makes the same promise for the projection's
  own proportional cut, and had the same fallback.

### Control (9852a2f) — verbatim

```
:: long.ls:  a*54 = symbol("a*54"); a*54 + 1     (the audit's repro)
dotnet Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --file long.ls --json --omit-functions --omit-variables --print-budget 1

CONTROL long : {"pretty":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa …","bodyLen":48,
                "canonical":"(add (rat 1 1) (sym  …","truncated":true,
                "reason":"node-budget","budget":1}
CONTROL long (no budget): {"pretty":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1","truncated":"ABSENT"}

:: proj.ls:  x = symbol("x"); b*40 = symbol("b*40"); b*40 + x     (the projection's own cut)
CONTROL proj : {"pretty":"bbbbbbbbbbbbbb …","bodyLen":14}
```

The pretty body is **48** characters of a **54**-character identifier; the identifier it names does
not exist in the value. The projection's cut splits a **40**-character identifier after 14.

### The fix

`SafeCut` (and the projection's `CutAtTokenBoundary`, which documents itself as "the same rule the
kernel printer applies") now cuts at the last token boundary **at or before** the allowance and,
when the allowance falls inside the **first** token (no boundary before it), at the boundary **after**
that token — the whole token is kept. Keeping one token may exceed the character allowance by that
token's width; splitting it publishes an identifier the value does not contain. When the rendering
**is** one token longer than the allowance there is no honest prefix, so the abbreviation is the
ellipsis alone (0 retained) rather than a text that drops nothing yet claims to be an abbreviation.
Every result is still a strict prefix of the real rendering and still reports
`truncated`/`node-budget`/`budget`; `display`, `typed` and every unbudgeted rendering are unchanged.
The rule is now stated in both source comments and in `docs/symbolics/dsh-protocol.md`.

### After (same commands, fix worktree)

```
long-name pretty-budget 1: display  = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1"   (54 chars, real name)
                           pretty   = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa …"        (bodyLen 54)
                           canonical= "(add (rat 1 1) (sym  …"   truncated:true node-budget budget:1
40-char projection       : pretty   = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb …"                    (bodyLen 40)
                           canonical= "(add (sym  …"             truncated:true node-budget budget:1
long-name no budget      : pretty   = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1"   (unchanged)
```

### Test evidence

New: `Lovelace.Symbolics.Tests/PrintBudgetTokenBoundaryTests.cs` (4 tests, the kernel printer) and
`Lovelace.Run.Tests/PrintBudgetTokenBoundaryTests.cs` (3 tests, both wire seams).

Control, verbatim (`ctl-run3.txt`, `ctl-sym4.txt`):

```
  Failed Lovelace.Run.Tests.PrintBudgetTokenBoundaryTests.AnIdentifierLongerThanTheKernelAllowance_IsNeverSplit
  Error Message:
   the pretty body is 48 characters of a 54-character identifier: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

  Failed Lovelace.Run.Tests.PrintBudgetTokenBoundaryTests.AnIdentifierLongerThanTheProjectionsAllowance_IsNeverSplit
  Error Message:
   pretty: the cut falls between 'b' and 'b', both token characters, so 'bbbbbbbbbbbbbb …' ends inside
   the token …'bbbbbbbbbbbbbbbb'

  Failed Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests.AnIdentifierLongerThanTheAllowance_IsKeptWhole
  Error Message:
   the pretty body is 48 characters of a 54-character identifier: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'

  Failed Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests.ARenderingThatIsOneLongToken_RetainsNothing
  Error Message:
   Assert.Equal() Failure: Strings differ
   Expected: "…"
   Actual:   "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"…

Failed!  - Failed:     2, Passed:     2, Skipped:     0, Total:     4 - Lovelace.Symbolics.Tests.dll (kernel filter)
Failed!  - Failed:     4, Passed:     4, Skipped:     0, Total:     8 - Lovelace.Run.Tests.dll (both new classes)
```

Fix: both filters `Passed! - Failed: 0, Passed: 8 / 6` (the same commands as in I-2).
`Lovelace.Symbolics.Tests/PrintBudgetTests.cs` and `Lovelace.Run.Tests/PrintBudgetTotalityTests.cs`
(the tests that already pin "never inside a token", the strict-prefix rule and the truncation
fields) are untouched and still pass, as do the `--print-budget` rows of the round-20 audit.

---

## Full test run (fix tree, `cf99d48`)

```
cd C:\Users\ricar\dev\LovelaceSharp\.worktrees\r20-budgetdocs
dotnet build LovelaceSharp.slnx -c Release      ->  Build succeeded.  0 Warning(s)  0 Error(s)
dotnet test LovelaceSharp.slnx -c Release --no-build
```

| test project | failed | passed | skipped | total |
| --- | --- | --- | --- | --- |
| Lovelace.Abstractions.Tests | 0 | 20 | 0 | 20 |
| Lovelace.Array.Tests | 0 | 19 | 0 | 19 |
| Lovelace.Integer.Tests | 0 | 148 | 0 | 148 |
| Lovelace.Natural.Tests | 0 | 195 | 0 | 195 |
| precbench.Tests | 0 | 13 | 0 | 13 |
| Lovelace.Representation.Tests | 0 | 91 | 0 | 91 |
| Lovelace.Knowledge.Tests | 0 | 28 | 0 | 28 |
| Lovelace.Studio.Tests | 0 | 22 | 0 | 22 |
| Lovelace.Console.Tests | 0 | 15 | 0 | 15 |
| Lovelace.Complex.Tests | 0 | 121 | 0 | 121 |
| Lovelace.Dsp.Tests | 0 | 61 | 0 | 61 |
| **Lovelace.Real.Tests** | 0 | **2495** | 0 | 2495 |
| **Lovelace.Suite.Tests** | 0 | **848** | 0 | 848 |
| **Lovelace.Symbolics.Tests** | 0 | **1179** | 8 | 1187 |
| **Lovelace.Run.Tests** | 0 | **299** | 0 | 299 |
| **total** | **0** | **5554** | **8** | **5562** |

(The four bold projects are the ones this landing touches: `Lovelace.Real`, `Lovelace.Suite`,
`Lovelace.Symbolics` and the runner; the whole solution is green so nothing else moved either.)

---

## Rules and honesty notes

* **Test-first, proven in a control tree.** The four new test files were written before the fix and
  copied into `.worktrees/r20-budgetdocs-ctl`, whose product sources are byte-identical to
  `9852a2f` (`git diff --stat` there is empty; the only source additions are the four test files).
  8 of the 14 new tests fail on `9852a2f`; the failing output is
  pasted above, and the 6 that pass there are the boundary controls that must pass on both sides.
* **No existing test was weakened, skipped, deleted or re-pinned** — no existing test file is in the
  commit. Two existing tests were read for contradiction before the fix:
  `StructuredRealPrecisionTests` (the structured payload carries the value's own stored digits, and
  `setprecision(1100); pi(1100)` must NOT be flagged) and
  `EvalfDigitCountContractTests` (the typed refusal and its exact message) — both still pass, and
  the fix was designed around them: the marker appears only when `evalf`'s own cap dropped digits,
  and a count above the cap is still accepted and clamped rather than refused, so the refusal's
  advertised range (`1..2147483647`) does not move.
* **No broad catch**, no new refusal, no new exception type: the only product behaviour added is a
  provenance field, its projection and a boundary rule.
* `ExactOf` (a private helper in `StructuredProjection`) became unused when the Complex arm was
  given the clamp notice and was removed; its rule now lives in the new `Complex` helper. No test
  referenced it.
* The bound itself is unchanged (1000 places, `Math.Min(digits, 1000)`), the arithmetic is
  unchanged, no digit moved, and the delivered value of every recorded repro is byte-identical to
  `9852a2f` apart from the new marker fields.

## What I could not do (and deliberately did not)

* **The cost of an over-cap request is untouched.** `evalf(sin(1), 5000)` still computes at
  `digits + guard` places and publishes at most 1000, so the caller still pays for digits it never
  receives (audit I-2's cost note, "recorded, not charged"). Capping the WORKING precision at the cap
  was rejected: it would change the delivered digits (the brief requires the arithmetic and all
  1000 delivered digits to stay as they are) and the brief scopes the fix to the contract, not the
  cost.
* **The clamp does not follow a raised engine cap** (`Real.MaxComputationDecimalPlaces`): the brief
  says the 1000-place bound is documented and must not be removed, so `EvalfDecimalPlaceCap` stays a
  hard 1000 and every clamp is reported instead. Consequence to be explicit about: `setprecision(5000);
  evalf(sqrt(2), 5000)` answers 1000 places WITH a marker, while `setprecision(5000); sqrt(2)`
  answers 5000 — two different bounds, each now stated.
* **The notice does not propagate through arithmetic** (documented in `Real.ClampNotice`):
  `evalf(sqrt(2), 5000) + 1` is a new value and is not flagged. Propagating it would claim
  "truncated" about values whose digits were never bounded by any request.
* **Not pushed and not merged** — the brief asked for a commit inside the worktree and the sha; the
  branch `r20-budgetdocs` and the control branch `r20-budgetdocs-ctl` are local only. The report
  file (this file) is written in the main tree but NOT committed there.
