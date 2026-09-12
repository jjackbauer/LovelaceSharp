# Round 14 — F2 (P1, NEW): the SHORT limit builtins publish the same `LimitResult` record `limit_full` does

**Objective.** A status must never have to be parsed out of prose. Audit D's F2: `limit`,
`limit_left` and `limit_right` answered a refusal (and a proven non-existence) as a bare **Text**
value under `ok: true` — no code, no category, nothing structured — while `limit_full` on the same
kernel result already published a record carrying every value the prose spelled out.

**Trees.** Nothing was committed, staged or pushed. `git status` in the scratch tree lists exactly
the files below.

| Tree | Path | What it is |
| --- | --- | --- |
| Scratch (the work) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f2` | `git worktree add .worktrees/c6-f2 HEAD` — detached at `141f34e` |
| Control (requirement 5) | `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-f2-ctl` | `git worktree add .worktrees/c6-f2-ctl HEAD` — pristine `141f34e`, ONLY the 4 new/changed test files copied in |
| Deliverable | `C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-14\` | this file plus the raw logs/diffs it cites |

`git -C .worktrees/c6-f2-ctl status --porcelain` — the control tree's whole diff:

```
 M Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs
 M Lovelace.Symbolics.Tests/IndeterminateExponentialLimitTests.cs
 M Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs
?? Lovelace.Run.Tests/LimitShortFormRecordTests.cs
```

`git -C .worktrees/c6-f2-ctl diff --stat -- Lovelace.Symbolics Lovelace.Suite Lovelace.Run` prints
nothing: the control tree's **product code is pristine**. SHA-256 of the four copied files matches
the scratch tree's byte for byte.

**Files changed (scratch tree)** — `git diff --numstat`:

```
1	1	Lovelace.Suite/docs/Language.md
15	2	Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs
22	7	Lovelace.Symbolics.Tests/IndeterminateExponentialLimitTests.cs
23	4	Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs
15	10	Lovelace.Symbolics/README.md
78	49	Lovelace.Symbolics/SymbolicsPlugin.cs
```
plus the new `Lovelace.Run.Tests/LimitShortFormRecordTests.cs`.

---

## 1. Pre-fix behaviour (observed on the PRISTINE product, HEAD `141f34e`)

Re-observed by me on the unfixed tree, and independently re-run at the end against the **control
tree's** own build of the pristine product (`wire-prefix.txt`):

```
$exe = <tree>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe
& $exe --file <case>.ls --json --omit-functions --omit-variables
```

| Case | `result.kind` | `result.display` |
| --- | --- | --- |
| `x = symbol("x"); limit(1/x, x, 0)` | **Text** | `does not exist (left: -inf, right: +inf)` |
| `x = symbol("x"); limit_left(1/x, x, 0)` | **Symbolic** | `-inf` |
| `x = symbol("x"); limit_right(1/x, x, 0)` | **Symbolic** | `inf` |
| `x = symbol("x"); limit(sin(x), x, inf)` | **Text** | `unevaluated: coefficient does not evaluate at the point` |
| `x = symbol("x"); limit_left(sin(x), x, inf)` | **Text** | `unevaluated: coefficient does not evaluate at the point` |
| `x = symbol("x"); limit_right(sin(x), x, inf)` | **Text** | `unevaluated: coefficient does not evaluate at the point` |
| `x = symbol("x"); limit(sin(x)/x, x, 0)` | Symbolic | `1` |
| `x = symbol("x"); limit_left(sin(x)/x, x, 0)` | Symbolic | `1` |
| `x = symbol("x"); limit_right(sin(x)/x, x, 0)` | Symbolic | `1` |

The structured payloads, byte for byte:

```json
limit(1/x, x, 0)              structured = {"kind":"Text","value":"does not exist (left: -inf, right: +inf)"}
limit(sin(x), x, inf)         structured = {"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}
limit_left(sin(x), x, inf)    structured = {"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}
limit_right(sin(x), x, inf)   structured = {"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}
limit_left(1/x, x, 0)         structured = {"kind":"Symbolic","pretty":"-inf", ...}
limit(sin(x)/x, x, 0)         structured = {"kind":"Symbolic","pretty":"1", ...}
```

`ok: true` on every one of them (the exit code is 0 and the envelope carries no `code`/`category` at
all). Source at HEAD: `Lovelace.Symbolics/SymbolicsPlugin.cs:1575-1591` — `LimitToExpr` maps
`DoesNotExist` to the interpolated sentence and everything else (`Unevaluated`/`Failed`) to
`"unevaluated: " + reason`; `SideText` renders each side into English inside that sentence. The
short forms were the only callers.

For contrast, the SAME kernel result through `limit_full` (`SymbolicsPlugin.cs:392-421` at HEAD):

```
limit_full(1/x, x, 0)      -> {"kind":"Record","type":"LimitResult","fields":[
                                status=Enum(LimitStatus:DoesNotExist), exists=Boolean(false), value=Null,
                                left=Symbolic(-inf), left_conditions=[x < 0],
                                right=Symbolic(inf), right_conditions=[x > 0],
                                conditions=[], exactness=Enum(SolutionExactness:Exact), diagnostics=[]]}
limit_full(sin(x), x, inf) -> status=Enum(LimitStatus:Unevaluated), exists=Null, exactness=None,
                              diagnostics=[{code:"limit.unevaluated", category:Enum(ErrorCategory:UnsupportedOperation),
                                            message:"coefficient does not evaluate at the point", recoverable:true, ...}]
```

Every value the prose encoded was therefore **already computed** — only the short forms threw the
structure away.

---

## 2. Failing tests first (requirement 2)

Written and RUN before any product edit.

**(a) New wire test — `Lovelace.Run.Tests/LimitShortFormRecordTests.cs`** (21 cases). It drives the
PUBLISHED envelope and pins: every shape is a `LimitResult` record (`structured.kind == "Record"`);
`status`/`exists` are the record's own claim; the two-sided short form is `AssertSameJson`-identical
to `limit_full`; the DNE case keeps `left`/`right` and their conditions; the refusal carries
`code=limit.unevaluated`, `category=ErrorCategory:UnsupportedOperation`, the kernel message and
`recoverable: true`. It is the limit twin of the cycle-5 precedent
`Lovelace.Run.Tests/BuiltinSurfaceContractTests.cs:183`
(`Solve_ShortForm_PublishesTheSameSolveResultRecord_AsTheFullForm`), which is what
`SymbolicsPlugin.cs:449` (HEAD: `445-455`) records as the decision for `solve`.

**(b) Three stale assertions updated** — each keeps the value it already asserted and moves it onto
the record field, adding the status beside it (strictly larger claims, nothing loosened):

| File:line (HEAD) | Was | Now |
| --- | --- | --- |
| `Lovelace.Symbolics.Tests/DxStructuredResultsTests.cs:170` | `limit(1/x,x,0).AsText() == "does not exist (left: -inf, right: +inf)"` | `AsRecord()`, then `exists=false`, `left="-inf"`, `right="inf"`, `left_conditions="[x < 0] (Vector)"`, `right_conditions="[x > 0] (Vector)"`, `exactness=Exact` — 7 assertions instead of 1 string |
| `Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs:110-113` | `StartsWith("unevaluated", limit_left(x*sin(1/x),x,0).AsText())` (×2), `Format(limit_left(1/x,x,0)) == "-inf"`, `Format(limit_right(...)) == "inf"` | the same four values read off `status`/`value` fields (`Unevaluated` + `value` is the protocol Null; `MinusInfinity`/`-inf` and `PlusInfinity`/`inf` with `exists=true`) |
| `Lovelace.Symbolics.Tests/IndeterminateExponentialLimitTests.cs:249-256` | `ValueFormatter.Format(engine.Evaluate("limit(...)"))` == `"5"`, `"1"`, `"8"`, `"1"`, `"e"`, `"e"`, `"e^2"` | the same seven values via the new helper `AssertShortFormValue`, which reads the record's `value` field and additionally asserts `status=LimitStatus:Value` and `exists=true` |

### Observed failure on the unfixed scratch tree

```
dotnet test .worktrees/c6-f2/Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~LimitShortFormRecordTests"
Failed!  - Failed:    21, Passed:     0, Skipped:     0, Total:    21

dotnet test .worktrees/c6-f2/Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~LimitExistenceTriStateTests|FullyQualifiedName~DxStructuredResultsTests|FullyQualifiedName~IndeterminateExponentialLimitTests"
Failed!  - Failed:     3, Passed:    50, Skipped:     0, Total:    53
```

The failure message (full transcript `failing-first-run-tests.txt`, 25 KB):

```
  Failed Lovelace.Run.Tests.LimitShortFormRecordTests.ShortForm_PublishesALimitResultRecord_NeverProse
      (script: "x = symbol(\"x\"); limit(1/x, x, 0)", status: "DoesNotExist", ...)
  Error Message:
   Assert.Equal() Failure: Strings differ
           ↓ (pos 0)
Expected: "Record"
Actual:   "Text"
           ↑ (pos 0)
```

Cases whose old answer was a bare Symbolic (`limit_left(1/x, x, 0)`, the three success shapes) fail as
`Actual: "Symbolic"`; the unevaluated one-sided shapes fail as `Actual: "Text"` like the two-sided one. The three updated files fail with the kind
cast that names the old carrier exactly:

```
System.InvalidCastException : Unable to cast object of type 'System.String'
                              to type 'Lovelace.Abstractions.RecordValue'.
   at Lovelace.Suite.Value.AsRecord()
   at ...DxStructuredResultsTests.LimitFull_ExposesExistenceAndOneSidedValues() ...:line 172
   at ...LimitExistenceTriStateTests.UndeterminedOneSidedLimits_ReportNoValue() ...:line 114
   at ...IndeterminateExponentialLimitTests.AssertShortFormValue(...) ...:line 267
```

---

## 3. The fix

Complete product diff: `f2-plugin.diff` (also inlined at the end of this file). Three mechanisms,
all inside `Lovelace.Symbolics/SymbolicsPlugin.cs`:

**(a) One projection.** The record builder that lived inline in `limit_full`'s lambda is extracted
verbatim into `private static RecordValue LimitRecord(LimitResult r, Symbol x, Expr point)`
(`SymbolicsPlugin.cs:1494`). The N19 three-valued-`exists` comment moved with it, unchanged.

**(b) All four builtins call it.** `limit` (`:398`), `limit_full` (`:408`), `limit_left` (`:421`) and
`limit_right` (`:427`) now each run their own kernel query and pass the result through
`LimitRecord`. `limit` and `limit_full` are now literally the same expression — the `solve`/`solve_full`
arrangement. `limit_full`'s own behaviour is byte-identical (same kernel call, same projection).

**(c) The prose path is deleted.** `LimitToExpr` and `SideText` had no other caller, so they are
removed rather than left as a dead path back to Text.

**Descriptors** (`SymbolicsPlugin.cs:132-137`, `:398-407`): `ReturnKind` `"Symbolic | Text"` →
`"LimitResult"` for all three, and each summary now states that the short form returns the SAME
`LimitResult` record `limit_full` returns — the help text (`Lovelace.Suite/HelpService.cs`) is
generated from these strings, so the advertised surface follows the behaviour.

**What a one-sided record means.** The kernel's one-sided query returns its answer as the record's
PRIMARY result, so it lands in `value` with its own `status`/`exists`/`exactness`; `left`/`right`
stay Null because the kernel splits a result into the two sides only for a TWO-SIDED query whose
sides disagree (`Calculus/Limits.cs:454-461` is the only place `LimitResult.Dne` is constructed).
This is documented on `LimitRecord` itself. No information the old answer carried is lost: the
one-sided value is still there, now with the status vocabulary beside it.

---

## 4. After (same command, same cases) — `wire-postfix.txt`

| Case | before | after |
| --- | --- | --- |
| `limit(1/x, x, 0)` | Text `does not exist (left: -inf, right: +inf)` | **Record/LimitResult** `status=DoesNotExist, exists=false, value=Null, left=-inf, left_conditions=[x < 0], right=inf, right_conditions=[x > 0], exactness=Exact` |
| `limit_left(1/x, x, 0)` | Symbolic `-inf` | **Record/LimitResult** `status=MinusInfinity, exists=true, value=-inf, exactness=Exact` |
| `limit_right(1/x, x, 0)` | Symbolic `inf` | **Record/LimitResult** `status=PlusInfinity, exists=true, value=inf, exactness=Exact` |
| `limit(sin(x), x, inf)` | Text `unevaluated: coefficient does not evaluate at the point` | **Record/LimitResult** `status=Unevaluated, exists=Null, exactness=None, diagnostics=[Diagnostic(code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, recoverable: True)]` |
| `limit_left(sin(x), x, inf)` | Text (same prose) | same record as above |
| `limit_right(sin(x), x, inf)` | Text (same prose) | same record as above |
| `limit(sin(x)/x, x, 0)` | Symbolic `1` | **Record/LimitResult** `status=Value, exists=true, value=1 (exact, rational), exactness=Exact, diagnostics=[]` |
| `limit_left(sin(x)/x, x, 0)` | Symbolic `1` | same record |
| `limit_right(sin(x)/x, x, 0)` | Symbolic `1` | same record |

Full envelope for the F2 case:

```json
{"kind":"Record","type":"LimitResult","fields":[
  {"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"DoesNotExist"}},
  {"name":"exists","value":{"kind":"Boolean","value":"false"}},
  {"name":"value","value":{"kind":"Null"}},
  {"name":"left","value":{"kind":"Symbolic","pretty":"-inf","canonical":"(mul (rat -1 1) (inf))","domain":"real","exact":false,...}},
  {"name":"left_conditions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Symbolic","pretty":"x < 0",...}]}},
  {"name":"right","value":{"kind":"Symbolic","pretty":"inf","canonical":"(inf)","domain":"real","exact":false,...}},
  {"name":"right_conditions","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Symbolic","pretty":"x > 0",...}]}},
  {"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
  {"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Exact"}},
  {"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}
```

`limit(1/x, x, 0)` and `limit_full(1/x, x, 0)` are now `AssertSameJson`-equal field for field
(`LimitShortFormRecordTests.TwoSidedShortForm_IsTheSameRecordAsTheFullForm`, three shapes: DNE,
determined value, unevaluated).

### Golden fixtures — NOT stale

`Lovelace.Run.Tests/fixtures/limit_result.ls` is `limit_full(sin(x)/x, x, 0)`, and `limit_full` is
unchanged, so `limit_result.json` is untouched: `git status` lists no fixture, and
`GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture(limit_result)` passes inside the green full-suite
run. The only `.ls` fixture that touches limits at all is `limit_result.ls`. The `capabilities.json`
golden embeds the two advertised limit TRIGGER scripts — `limit_full(sin(x), x, inf)` (:295) and
`limit(sin(x), 1, 0)` (:679) — but neither behaviour moved (the second is the argument-coercion
error, still `InvalidOperation`/`DomainError`), and that golden's test passes unchanged.

---

## 5. Scope ripple: the executable documentation examples

The first full-suite run exposed 11 more callers of the old shape — and they are **executable doc
tests**, i.e. exactly the "documentation example that shows the old shape" the scope clause covers,
and the file the repository's rule says to regenerate from the product:

* `Lovelace.Symbolics.Tests/UsageDocumentationTests.cs:16,51` — evaluates every `lovelace`/`result`
  pair in `Lovelace.Symbolics/README.md` and asserts `Assert.Equal(expected,
  ValueFormatter.FormatTyped(value))`.
* `Lovelace.Suite.Tests/LanguageDocumentationTests.cs:16,72` — the same for
  `Lovelace.Suite/docs/Language.md`.

First full-suite run (before the docs were regenerated) — 1 failed in `Lovelace.Suite.Tests`, 10 in
`Lovelace.Symbolics.Tests`, every one a `limit`/`limit_left`/`limit_right` example, e.g.:

```
Failed ...UsageDocumentationTests.DocumentedExample_MatchesEngine(script: "x = symbol(\"x\")\nlimit(1/x, x, 0)",
      expected: "does not exist (left: -inf, right: +inf)")
  Expected: "does not exist (left: -inf, right: +inf)"
  Actual:   "LimitResult(status: DoesNotExist, exists:"···
Failed ...LanguageDocumentationTests.DocumentedExample_MatchesEngine(script: "x = symbol(\"x\")\nlimit(sin(x)/x, x, 0)",
      expected: "1 (Symbolic)")
```

Each affected `result` block was **regenerated from the product**: the same script was run through the
fixed binary and the block replaced with `result.typed`, which is the wire form of
`ValueFormatter.FormatTyped` (confirmed against the untouched blocks: e.g. `1 (Symbolic)`).

| File | `result` block lines (after edit) | old → new |
| --- | --- | --- |
| `Lovelace.Symbolics/README.md` | 394 | `1 (Symbolic)` → `LimitResult(status: Value, ..., exactness: Exact, diagnostics: []) (LimitResult)` |
| `Lovelace.Symbolics/README.md` | 402 | `1/2 (Symbolic)` → `LimitResult(status: Value, exists: True, value: 1/2, ...)` |
| `Lovelace.Symbolics/README.md` | 413 | `does not exist (left: -inf, right: +inf)` → `LimitResult(status: DoesNotExist, ..., left: -inf, left_conditions: [x < 0], right: inf, right_conditions: [x > 0], ...)` |
| `Lovelace.Symbolics/README.md` | 423 | `-inf (Symbolic)` → `LimitResult(status: MinusInfinity, exists: True, value: -inf, ...)` |
| `Lovelace.Symbolics/README.md` | 431 | `inf (Symbolic)` → `LimitResult(status: PlusInfinity, exists: True, value: inf, ...)` |
| `Lovelace.Symbolics/README.md` | 439 | `inf (Symbolic)` → `LimitResult(status: PlusInfinity, exists: True, value: inf, ...)` |
| `Lovelace.Symbolics/README.md` | 449 | `0 (Symbolic)` → `LimitResult(status: Value, exists: True, value: 0, ...)` |
| `Lovelace.Symbolics/README.md` | 457 | `inf (Symbolic)` → `LimitResult(status: PlusInfinity, exists: True, value: inf, ...)` |
| `Lovelace.Symbolics/README.md` | 465 | `2 (Symbolic)` → `LimitResult(status: Value, exists: True, value: 2, ...)` |
| `Lovelace.Symbolics/README.md` | 475 | `unevaluated: coefficient does not evaluate at the point` → `LimitResult(status: Unevaluated, ..., diagnostics: [Diagnostic(code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, recoverable: True, ...)])` |
| `Lovelace.Suite/docs/Language.md` | 1098 | `1 (Symbolic)` → the same `LimitResult(status: Value, ...)` line |

One narrative sentence was added to the README's Limits section
(`Lovelace.Symbolics/README.md:384-387`) stating the record contract, so a reader is not left
guessing why the transcripts changed.

**Protocol documents that do NOT need changing** (checked by grep, stated as required):
`docs/symbolics/dsh-protocol.md:81` names `limit.unevaluated` as a diagnostic code — still true, and
the file contains no `limit(...)` example at all. The stale prose descriptions that remain are in
**historical cycle-3 planning documents**, out of scope and deliberately not rewritten:
`docs/symbolics/dx-convergence-alignment-plan.md:80` and `:164` (`DNE becomes the prose string …`),
`:182-184`; `docs/symbolics/a-plus-convergence-alignment-plan.md:50`.

---

## 6. Capability honesty (requirement 3)

**The statement did not need updating.** The only entry that names these builtins' refusals is
`SymbolicsPlugin.cs:876-879`:

```csharp
UnsupportedCapability("limit.unevaluated-in-record-diagnostics", "limit.unevaluated",
    ErrorCategory.UnsupportedOperation,
    "coefficient does not evaluate at the point",
    "x = symbol(\"x\"); limit_full(sin(x), x, inf)"),
```

Its `operation_class` marker (`-in-record-diagnostics`) and its `limit_full` trigger were already
true and `limit_full` is out of scope and unchanged. After the fix the SHORT forms surface the very
same class in a record's `diagnostics`, so the statement became more complete, not less: there is no
reachable refusal class here that is unadvertised. The descriptors' `ReturnKind` and summaries were
updated instead (§3), and no test or fixture pins a `capabilities()` field to a limit return kind
(`CapabilitiesBuiltinTests` pins the 19 `unsupported_operations` classes, all unchanged).

**Re-run.** `docs/goal-cycle-5/verify-capabilities.ps1` is hard-wired to the MAIN tree
(`Set-Location 'C:\Users\ricar\dev\LovelaceSharp'`, line 6) and to its AOT binary
(`...\out\aot\Lovelace.Run.exe`, line 11), i.e. to code that does NOT contain this fix. I therefore
ran a copy of it with **exactly two substituted lines** — `Set-Location` → the scratch worktree and
`$exe` → `<scratch>\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.exe` — and nothing else
(`Compare-Object` output is quoted in the transcript; the copy is
`%TEMP%\c6-f2-probe\verify-capabilities-fixedtree.ps1`). This is the one adaptation in this round
that a reader should weigh.

```
advertised entries: 19
... 19 rows, every one MATCH ...
MATCH=19  MISMATCH=0
```

Full output: `capability-honesty-fixedtree.txt`. The script wrote its report into the scratch tree at
`docs/goal-cycle-5/baseline/capability-honesty-verification.txt`; that file's git blob is
`ecce2d7d14cb0542c0e374e2b18ec9024da8cfbc`, **identical** to `HEAD:` the same path — the re-run
reproduced the committed baseline byte for byte (git's ` M` marker on it is only the stat cache /
`autocrlf` notice; `git diff --quiet` exits 0).

---

## 7. Control (requirement 5)

Pristine `141f34e` + ONLY the four new/changed test files (hashes verified identical to the scratch
tree's). The product code there is untouched, so all four files must fail:

```
dotnet test .worktrees/c6-f2-ctl/Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~LimitShortFormRecordTests"
Failed!  - Failed:    21, Passed:     0, Skipped:     0, Total:    21     [control-run-tests.txt]

dotnet test .worktrees/c6-f2-ctl/Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo \
  --filter "FullyQualifiedName~LimitExistenceTriStateTests|FullyQualifiedName~DxStructuredResultsTests|FullyQualifiedName~IndeterminateExponentialLimitTests"
Failed!  - Failed:     3, Passed:    50, Skipped:     0, Total:    53    [control-symbolics-tests.txt]
```

All 21 new cases fail; all three updated files fail
(`DxStructuredResultsTests.LimitFull_ExposesExistenceAndOneSidedValues`,
`LimitExistenceTriStateTests.UndeterminedOneSidedLimits_ReportNoValue`,
`IndeterminateExponentialLimitTests.SubstitutionShapedLimits_StillAnswerImmediately`). On the fixed
tree the same four files are 21/0 and 53/0 (`postfix-run-tests.txt`, `postfix-symbolics-tests.txt`).

---

## 8. Whole solution and Timing subsets (requirement 4)

`dotnet test LovelaceSharp.slnx --configuration Release --nologo` on the final scratch tree, with
`PATH` prefixed by `C:\Users\ricar\dev\.lovelace-tools\python` and `LOVELACE_REQUIRE_SYMPY=1`
(the oracle is live: the differential limit tests run) → **exit code 0**. Full transcript
`suite-whole-solution.txt`.

| Project | Failed | Passed | Skipped | Total |
| --- | --- | --- | --- | --- |
| Lovelace.Abstractions.Tests | 0 | 20 | 0 | 20 |
| Lovelace.Array.Tests | 0 | 19 | 0 | 19 |
| Lovelace.Complex.Tests | 0 | 115 | 0 | 115 |
| Lovelace.Console.Tests | 0 | 15 | 0 | 15 |
| Lovelace.Dsp.Tests | 0 | 61 | 0 | 61 |
| Lovelace.Integer.Tests | 0 | 148 | 0 | 148 |
| Lovelace.Knowledge.Tests | 0 | 28 | 0 | 28 |
| Lovelace.Natural.Tests | 0 | 195 | 0 | 195 |
| Lovelace.Real.Tests | 0 | 2495 | 0 | 2495 |
| Lovelace.Representation.Tests | 0 | 91 | 0 | 91 |
| Lovelace.Run.Tests | 0 | 227 | 0 | 227 |
| Lovelace.Studio.Tests | 0 | 22 | 0 | 22 |
| Lovelace.Suite.Tests | 0 | 825 | 0 | 825 |
| Lovelace.Symbolics.Tests | 0 | 1127 | 0 | 1127 |
| precbench.Tests | 0 | 13 | 0 | 13 |
| **total** | **0** | **5401** | **0** | **5401** |

`Lovelace.Run.Tests` 227 = 206 + the 21 new cases; `Lovelace.Symbolics.Tests` 1127 unchanged
(no case added or removed, three re-pointed).

Timing subsets, same configuration:

| Command | Result |
| --- | --- |
| `Lovelace.Suite.Tests --filter "Category=Timing"` | Failed 0, Passed 8, Total 8 (`timing-Lovelace.Suite.Tests.txt`) |
| `Lovelace.Run.Tests --filter "Category=Timing"` | Failed 0, Passed 5, Total 5 (`timing-Lovelace.Run.Tests.txt`) |
| `Lovelace.Real.Tests --filter "Category=Timing"` | Failed 0, Passed 1, Total 1 (`timing-Lovelace.Real.Tests.txt`) |
| `Lovelace.Symbolics.Tests --filter "Category=Timing"` | Failed 0, Passed 1, Total 1 (`timing-Lovelace.Symbolics.Tests.txt`) |

**One flake, investigated and explained (not hidden).** An intermediate full-solution run reported
`Lovelace.Suite.Tests` 1 failed:

```
Failed Lovelace.Suite.Tests.CancellationObservationTests.Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly [26 s]
  Error Message:
   cancellation was observed but only after 26630 ms; the budget was 1500 ms.
```

That test measures WALL-CLOCK cancellation latency and is unrelated to limits (it does not mention
one). Run alone on the same fixed tree: `Passed! - Failed: 0, Passed: 2, Total: 2`; the whole
`Lovelace.Suite.Tests` project alone on the same fixed tree: `Passed! - Failed: 0, Passed: 825,
Total: 825` (`suite-project-alone.txt`); and the final 15-project run — the one tabulated above —
is 0 failed. The pristine control tree at HEAD contains the same test, so this is a machine-load
artifact of running 15 test projects concurrently, not a property of the change.

---

## 9. Could not verify / not done

1. **The capability-honesty script was run with two substituted lines** (tree + binary path; §6). I
   did NOT run the literal file from the main tree against the fix, because it is hard-wired to the
   main tree's `out/aot` AOT binary, which is built from unfixed sources; pointing it at my fix would
   have required overwriting a gitignored artifact of the main tree or rewriting the tracked
   `docs/goal-cycle-5/baseline` report from a different tree. The script's logic is otherwise
   untouched and its verdict is 19/0.
2. **No AOT publish was made for this round.** The honesty script and the wire probes ran the
   framework-dependent `Lovelace.Run.exe` from the Release build of the scratch tree. The fix is in
   `Lovelace.Symbolics`, which both publish modes load identically, but I did not re-run the AOT
   smoke suite.
3. **The one-sided record's `left`/`right` fields stay Null.** For `limit_left`/`limit_right` the
   kernel's one-sided answer is published in `value` (with `status`/`exists`/`exactness`), not
   duplicated into `left`/`right`, because the kernel produces a two-sided split only for a
   two-sided query. This is a design decision, documented on `LimitRecord`
   (`SymbolicsPlugin.cs:1476-1493`) and in the descriptors; a consumer that wants "the left limit"
   of an arbitrary call must still call `limit_full` (or `limit_left`). It is not something I could
   "verify" independently — it is the shape I chose, and the alternative (filling `left` while
   `status`/`value` describe a one-sided primary result) would publish a record whose `status` and
   `value` disagree with their `limit_full` meaning.
4. **Historical planning docs still describe the old prose shape** —
   `docs/symbolics/dx-convergence-alignment-plan.md:80,164,182-184` and
   `docs/symbolics/a-plus-convergence-alignment-plan.md:50`. They are cycle-3/cycle-5 planning
   records, outside this round's SCOPE, and were left as written. No executable doctest reads them.
5. **Nothing was run on the main tree's sources.** The only files written outside the two worktrees
   are this document and its evidence siblings under `docs/goal-cycle-6/round-14/`.
6. The `docs/goal-cycle-5/baseline/capability-honesty-verification.txt` inside the SCRATCH worktree
   was rewritten by the honesty script. Content is byte-identical to HEAD (same blob), so the
   scratch diff carries no change there.

---

## Appendix — product diff (`f2-plugin.diff`)

```diff
@@ -130,9 +130,11 @@
             ["limit_left"] = new("limit_left", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
-                "One-sided limit from the left as x approaches x0.", ["limit_left(1/x, x, 0)"], "Symbolic | Text", ["limit", "limit_full"]),
+                "One-sided limit from the left as x approaches x0, returned as the SAME LimitResult record limit_full returns: status, exists, the one-sided value, exactness and diagnostics.",
+                ["limit_left(1/x, x, 0)"], "LimitResult", ["limit", "limit_full"]),
             ["limit_right"] = new("limit_right", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
-                "One-sided limit from the right as x approaches x0.", ["limit_right(1/x, x, 0)"], "Symbolic | Text", ["limit", "limit_full"]),
+                "One-sided limit from the right as x approaches x0, returned as the SAME LimitResult record limit_full returns: status, exists, the one-sided value, exactness and diagnostics.",
+                ["limit_right(1/x, x, 0)"], "LimitResult", ["limit", "limit_full"]),
@@ -383,49 +385,51 @@
+        // audit D F2 (cycle 6): the short forms and the _full form publish the SAME LimitResult
+        // record, produced by the ONE projection below. Before this, limit() answered a bare Text
+        // for every shape the kernel did not determine ... [full text in f2-plugin.diff]
         Add("limit", new[] { "f", "x", "x0" }, args =>
-            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.TwoSided, Context)),
+        {
+            var x = AsSymbol(args[1]);
+            var point = AsExpr(args[2]);
+            return LimitRecord(Limits.Limit(AsExpr(args[0]), x, point, LimitDirection.TwoSided, Context), x, point);
+        },
         new BuiltinDescriptor("limit", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
-            "Two-sided limit of f as x → x0. Use limit_full for the structured result (existence, left/right values).",
-            ["limit(sin(x)/x, x, 0)", "limit(1/x, x, 0)"], "Symbolic | Text",
+            "Two-sided limit of f as x → x0, returned as the SAME LimitResult record limit_full returns: ... Nothing has to be parsed out of prose.",
+            ["limit(sin(x)/x, x, 0)", "limit(1/x, x, 0)"], "LimitResult",
@@ -1466,6 +1470,48 @@
+    private static RecordValue LimitRecord(LimitResult r, Symbol x, Expr point)   // :1494
@@ -1572,24 +1618,6 @@
-    private static object LimitToExpr(LimitResult r) => ...   // deleted
-    private static string SideText(LimitResult? r) => ...     // deleted
```

The complete, unnarrated diff is `f2-plugin.diff` (12,405 bytes) and the whole scratch-tree diff is
`f2-full.diff` (24,539 bytes); both were produced by `git -C .worktrees/c6-f2 diff`.
