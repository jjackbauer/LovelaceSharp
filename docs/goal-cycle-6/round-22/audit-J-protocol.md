# Audit J — document conformance of the DSH structured protocol (round 22)

**Oracle:** `docs/symbolics/dsh-protocol.md` (279 lines, read cover to cover) plus the binary's `--help`.
**Binary under test:** `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
— used IT, **not** `bin\Release`. Verified: 5 758 464 bytes, LastWriteTime 2026-09-12 19:17:13.
**Method:** every normative sentence was turned into a probe driven from Python 3.12 `subprocess` with a
literal argv (no shell), so quoting is exact; scripts containing quotes went through `--file`/`--stdin`
bytes, never through the PS 5.1 `--eval` quote-stripping trap.
Every FAIL was reproduced at least twice; the confirmation runs are in "FAIL evidence" below.
Nothing in the repo was modified; this file is the only artifact written.

Command spelling below is the literal argv, with `BIN` = the binary above.

---

## 1. Invariants

| id | normative sentence (quoted / paraphrased) | verdict | command that decided it | observed |
|---|---|---|---|---|
| I1-a | "stdout carries the envelope and nothing else." | PASS | `BIN --eval 'print("hello"); 1+1' --omit-functions` | stdout is one JSON object (`json.loads` succeeds, no trailing bytes), `stderr=''`; `"hello"` occurs nowhere outside `output` |
| I1-b | "Anything the script prints with `print(...)` is captured and returned in the top-level `output` array." | PASS | `BIN --eval 'print("hello"); 1+1'` | `"output":["hello"]`; `print("a","b")`→`["a b"]`, `print()`→`[""]`, `print([1,2,3])`→`["[1, 2, 3]"]` |
| I1-c | (scope) Invariant 1 for the `--text` surface | UNTESTABLE | `BIN --eval '1+1' --text` | the doc never mentions `--text`; help does ("emit a human-readable summary"), and that mode writes `= 2 (Natural)` to stdout. Known-open, see §7 |
| I2-a | "Every field is one of `scalar \| symbolic \| array \| record \| domain \| enum \| null`; nothing degrades to text because the serializer lacked a case." | PASS | 10 record-producing scripts scanned structurally | only these `kind`s occur: Natural, Real, Complex, Integer, Text, Boolean, Symbolic, Array, Record, Domain, Enum, Null — all inside the declared set |
| I2-b | "An enum-valued field is never `Text`: it carries the declared enum type" | PASS | same scan | every `status`/`completeness`/`exactness`/`classification`/`category` is `{"kind":"Enum","type":…,"value":…}`; no Text in those positions |
| I2-c | "`SolveStatus.Partial` and `Completeness.Partial` can never be confused" | PASS | `BIN --eval 'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)'` | `status={"kind":"Enum","type":"SolveStatus","value":"Partial"}`, `completeness={"kind":"Enum","type":"Completeness","value":"Partial"}` — same member name, different `type` |
| I3 | "An absent field is `{"kind":"Null"}`, distinct from an empty string." | PASS | `solve(x^2-4==0,x)` / `limit_full(a/x, x, 0)` | `unrepresented_reason={"kind":"Text","value":""}` **and** `Diagnostic.location={"kind":"Null"}` **and** Unevaluated `LimitResult.{exists,value,left,right}` all `{"kind":"Null"}`; empty arrays cross as `{"kind":"Array",…,"elements":[]}` |
| I4-a | "`protocolVersion`, `symbolicFormatVersion` and `mathIrVersion` are always present." | PASS | success, runtime error, parse error, unreadable `--file`, `--cancel-after` envelopes | all three present with `1` / `"#!lovelace-sym 1"` / `2` on every path. Usage errors (rc=2) emit no envelope at all — the doc is silent about that path |
| I4-b | "`--print-budget <nodes>` bounds structured renderings: beyond it `pretty`/`canonical` carry a ` …`-terminated prefix of the real rendering (never a re-ordered one) and the value reports `truncated: true`, `truncationReason`, `budget`." | PASS | `BIN --eval 'x = symbol("x"); expand((x+1)^10)' --print-budget N`, N=1…59 | for every N with nodeCount>N: `truncated:true`, `truncationReason:"node-budget"`, `budget:N`; **0 prefix violations in 118 (pretty,canonical) pairs** (prefix equality against the unbudgeted rendering); when nothing is dropped the three keys are absent |
| I4-c | "An abbreviation NEVER ends inside a token: the cut lands on the last token boundary at or before the allowance, and when the allowance falls inside the rendering's first token that whole token is kept." | PASS | `BIN --eval 's = symbol("<140×d>"); s^2' --print-budget 1` | `pretty='dddd…(140) …'` — the whole 140-char first token kept although the allowance is 48; `pretty='aaaa…(60) …'` for the 60-char case. The degenerate "first token *is* the rendering" case needs `--print-budget 0`, which is a usage error (rc=2), so it is unreachable from this surface |
| I4-d | "`evalf(f, digits)` publishes at most **1000** decimal places … reports `truncated: true`, `truncationReason: "digit-cap"` and `budget: 1000` — never a silent clamp." | PASS | `BIN --eval 'evalf(pi(), 1001)'` (also 1500 and 5000) | each: `truncated=true`, `truncationReason="digit-cap"`, `budget=1000`, value has exactly 1000 decimals; the truncation keys also appear on the `variables[]` copy |
| I4-e | "A value the cap did not cut (an exact answer, or a count at or below 1000) carries none of the three fields." | PASS | `evalf(pi(),999)`, `evalf(pi(),1000)`, `evalf(sqrt(2),30)`, `1+1` | key sets are exactly `["exact","kind","value"]`; no `truncated`/`truncationReason`/`budget` |
| I4-f | "Durations are structural: `elapsedTime` is `{value, unit}` next to the human `elapsed` string" | PASS | `1+1`, `evalf(pi(),200)`, `a = 1\nb = 2\ndet(a)` | `"elapsed":"74.2 µs"` ↔ `{"value":74.2,"unit":"µs"}`; `"2.92 ms"` ↔ `{2.92,"ms"}`; `"688.3 µs"` ↔ `{688.3,"µs"}`; `"0 ns"` ↔ `{0,"ns"}` — same unit selector |
| I4-g | "`timings` carries one entry per top-level statement" | PASS | `1+1; 2+2; 3+3; 4+4; 5+5`; `print("a"); 1+1; x = symbol("x")` | 5 entries / 3 entries, positions 0,6,12,18,24 and 0,13,30 |
| I4-h | "an agent never parses a unit suffix" | PASS | every envelope above | `unit` is a separate JSON string (`ns`/`µs`/`ms`); the human string is separate |
| I5 | "camelCase JSON keys; snake_case record field names" | PASS | 10 record-producing scripts, all keys walked | **0** underscores in non-record keys (protocolVersion, mathIrVersion, elapsedTime, minArity, parameterCount, resultKind, hasOutput, …); **0** uppercase letters in the 100 record field names of the 17 record types observed (`unrepresented_reason`, `common_conditions`, `mathir_version`, `estimated_cost_before`, `verification_method`, `operation_class`, …) |
| I6 | "Errors are structural: `code`, `category`, `message`, `recoverable`, `diagnostics`." | PASS | runtime error, parse error `[1,2;3,4]`, unreadable `--file` | all five keys present on all three paths |

## 2. Payload control

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| PC-a | "each flag replaces its array with an EMPTY one — the key stays present" | PASS | `BIN --eval '1+1' --omit-functions --omit-variables` | `"functions":[]`, `"variables":[]`, both keys present (123 functions / 2 variables without the flags) |
| PC-b | "They change no value, no display and no `output[]` entry." | PASS | full vs omitted, with `elapsed`/`elapsedTime`/`timings` removed (they are wall-clock and differ between *any* two runs) | `full==full:True` and `full==omit:True` for 4 scripts incl. `print("hi"); x = symbol("x"); solve(…)` — `result`, `output`, `revision`, `ok` byte-identical |

## 3. Value forms

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| VF-a | Symbolic `{kind,pretty,canonical,domain,exact,nodeCount,freeSymbols}` | PASS | `BIN --eval 'x = symbol("x"); diff(x^2, x)'` | `{"kind":"Symbolic","pretty":"2*x","canonical":"(mul (rat 2 1) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}` |
| VF-b | Array `{kind:"Array",type:"Array",shape:[2,2],elements:[/* row-major */]}` | PASS | `BIN --eval 'x = symbol("x"); y = symbol("y"); jacobian([x*y, x+y],[x,y])'` | `kind=Array type=Array shape=[2,2]`, elements `['(sym y)','(sym x)','(rat 1 1)','(rat 1 1)']` = row-major ✓. A rank-1 array reports `"type":"Vector"` (e.g. `[1,2,3]`, `[]`) — the block does not mention that variant |
| VF-c | Complex `{kind,re,im,exact}` | PASS | `BIN --eval 'dft([1,2,3,4])'` | elements `{"kind":"Complex","exact":true,"re":"10","im":"0"}` (member set identical; the doc's *member order* differs — JSON member order is not significant) |
| VF-d | Integer `{kind,value,exact}` | PASS | `solve(x^2-4==0,x)` | `{"kind":"Integer","value":"2","exact":true}` (`represented_count`), `{"kind":"Integer","value":"1","exact":true}` (`multiplicity`) |
| VF-e | Domain `{kind,domain}` | PASS | `BIN --eval 'real()'`, solve domain field | `{"kind":"Domain","domain":"real"}`; also `integer`/`rational`/`complex` |
| VF-f | Boolean `{kind,value}` (string payload) | PASS | `solve(x^2-4==0,x)` | `{"kind":"Boolean","value":"true"}` — a **string**, as the block shows (CI relies on it) |
| VF-g | Enum `{kind,type,value}` | PASS | `solve(x^2-4==0,x)` | `{"kind":"Enum","type":"SolveStatus","value":"Solved"}` |
| VF-h | Null `{kind:"Null"}` | PASS | `limit_full(a/x,x,0)` | `{"kind":"Null"}` for `exists`;`value`;`left`;`right` |
| VF-i | Record `{kind,type,fields:[{name,value}]}` | PASS | `solve(x^2-4==0,x)` | `{"kind":"Record","type":"SolveResult","fields":[{"name":"status","value":…},…]}` |

## 4. Enum-valued fields

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| EVF-a | "`status`, `completeness`, `exactness`, `classification` and a diagnostic's `category` are **enums**, never text." | PASS | `type(r.status)`, `r.solutions[0].exactness`, `simplify_full(x/x).steps[0].classification`, `solve_full(x^4-…).diagnostics[0].category` | each is `{"kind":"Enum","type":…,"value":…}` |
| EVF-b | "The declared enum type is part of the value: `type` names it (`SolveStatus`, `Completeness`, `SolutionExactness`, `LimitStatus`, `IntegrationStatus`, `TransformStatus`, `RuleClassification`, `ErrorCategory`)" | PASS | `type(…)` on each, plus the raw records | all eight names observed on the wire: SolveStatus, Completeness, SolutionExactness, LimitStatus, IntegrationStatus, TransformStatus, RuleClassification, ErrorCategory |
| EVF-c | "`type()` answers the same declared name: `type(r.status)` is `SolveStatus`, not `Enum`." | PASS | `BIN --eval 'x = symbol("x"); r = solve(x^2 - 4 == 0, x); type(r.status)'` | `{"kind":"Text","value":"SolveStatus"}` (also `Completeness`, `LimitStatus`, `IntegrationStatus`, `RuleClassification`) |
| EVF-d | "`complete` … is `true` exactly when `completeness` is `Complete` … the two fields cannot disagree" | PASS | 8 solve scripts incl. `solve_system`, plus 3 independent runs | invariant held in 100% of runs: Solved→true/Complete, NoSolutions→true/Complete, Partial→false/Partial, Unevaluated→false/Unknown |

## 5. The Diagnostic form

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| DF-a | "Every rich result record (`SolveResult`, `SystemSolveResult`, `TransformResult`, `LimitResult`, `IntegrationResult`, `OptimizationResult`, `CompilationResult`) **ends with** a `diagnostics` field that is an Array of `Diagnostic` records — never a text value." | PASS | `solve`, `solve_system_full`, `simplify_full`, `limit_full`, `integrate_full`, `optimize_full`, `compile_full` | all 7 present; `diagnostics` is the **last** field of every one; always `kind:"Array"` |
| DF-b | "When there is nothing to report the array is **empty** (`{"kind":"Array","shape":[0],"elements":[]}`): never `Null` and never `""`." | **FAIL (literal shape)** | `BIN --eval 'x = symbol("x"); solve(x^2 - 4 == 0, x)' --omit-functions --omit-variables` | live: `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}` — the doc's literal block omits `"type"`, and the doc contradicts itself: line 153 of the same document shows the solve-result `diagnostics` **with** `"type":"Vector"`. Substance (never Null, never "") PASSes. See J-2 |
| DF-c | "The kernel's human-readable note is no longer a field of its own; it rides inside a diagnostic's `message`." | PASS | `solve_full(x == x, x)` | SolveResult field list has no note/text field; the note is `diagnostics[0].fields[message].value = "0 = 0: every value is a solution."` |
| DF-d | "A `Diagnostic` is a Record named `Diagnostic` with exactly these six fields, in this order: `code`, `category`, `message`, `recoverable`, `location`, `details`." | PASS | `solve_full(x^4 - x^2 - 1 == 0, x)` | `type:"Diagnostic"`, exactly the six names in exactly that order |
| DF-e | "`code` is **stable text a consumer matches**" | PASS | partial / unevaluated / limit / transform probes | `solve.unrepresented-roots`, `solve.unevaluated`, `limit.unevaluated`, `transform.budget-exceeded`, `transform.unsatisfiable-conditions` all observed as Text |
| DF-f | "`category` is an **Enum of type `ErrorCategory`**: ParseError, DomainError, UnsupportedOperation, BudgetExceeded, NoSolution, TypeMismatch, InternalInvariantFailure" | PASS | diagnostic probes + enum declaration read | observed UnsupportedOperation / BudgetExceeded / DomainError with `type:"ErrorCategory"`; the C# `ErrorCategory` enum declares exactly those seven members in that order |
| DF-g | "`recoverable` says whether the caller can continue: a `transform.unsatisfiable-conditions` diagnostic is `false`" | PASS | `assume(x == 0); simplify_full(x/x)` | `status=Unsatisfiable`, diagnostic `code=transform.unsatisfiable-conditions`, `category=DomainError`, `recoverable={"kind":"Boolean","value":"false"}` |
| DF-h | "`location` is `{"kind":"Null"}` for a kernel-level diagnostic … and otherwise a `DiagnosticLocation` record with `start_line`, `start_column`, `end_line`, `end_column`." | PASS (Null branch) / UNTESTABLE (the "otherwise" branch) | 5 diagnostic-producing scripts | `location={"kind":"Null"}` every time. No shipped producer constructs a `DiagnosticLocation` (only `Diagnostic.Of` exists, which passes `null`), so the second half cannot be reached |
| DF-i | "`details` is always an Array (empty when there are none), never `Null`." | PASS (substance) | partial / budget / unsat probes | `details` is `kind:"Array"` with 0 elements; the literal shape carries the same extra `"type":"Vector"` as DF-b (J-2) |
| DF-j | "An agent can … answer … from `diagnostics.elements[].fields[category].type` + `value`, without reading prose." | PASS | partial-solve envelope | `type:"ErrorCategory"`, `value:"UnsupportedOperation"` addressable by name without touching `message` |

## 6. The worked solve envelope (doc lines 110–201)

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| SE-a | "The script is `x = symbol("x"); solve(x^2 - 4 == 0, x)`" with `protocolVersion:1`, `symbolicFormatVersion:"#!lovelace-sym 1"`, `mathIrVersion:2`, `ok:true` | PASS | `BIN --eval 'x = symbol("x"); solve(x^2 - 4 == 0, x)'` | exactly those four values |
| SE-b | "`solve` and `solve_full` publish this SAME record" | PASS | same script with `solve` and `solve_full`, timings/elapsed normalised | `structured`, `display`, `typed` byte-identical; only the wall-clock fields differ |
| SE-c | `"revision": 83` | **FAIL** | `BIN --eval 'x = symbol("x"); solve(x^2 - 4 == 0, x)'` | `"revision":82` — deterministic, 5/5 runs (with and without the omit flags). See J-1 |
| SE-d | `result` is `{kind,display,typed,structured}` with `display`/`typed` as shown | PASS | same | `"display":"SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete, solutions: [Solution(value: -2, conditions: [], multiplicity: 1, exactness: Exact), Solution(value: 2, …)], families: [], common_conditions: [], represented_count: 2, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])"` and `typed` = same + `" (SolveResult)"` — character for character as the abridged block shows |
| SE-e | `SolveResult` field list and order (`status`, `variable`, `domain`, `complete`, `completeness`, `solutions`, `families`, `common_conditions`, `represented_count`, `unrepresented_count`, `unrepresented_reason`, `diagnostics`) | PASS | same | exact match, in order |
| SE-f | `solutions` shown as `{"kind":"Array","shape":[2],"elements":[…]}` | **FAIL (literal member set)** | same | live `solutions={"kind":"Array","type":"Vector","shape":[2],"elements":[…]}`; the doc shows `"type":"Vector"` on `families`, `common_conditions` and `diagnostics` in the very same block but omits it on `solutions`, and the stated abridgement scope is "only in the nested symbolic renderings". See J-3 |
| SE-g | `families`/`common_conditions`/`diagnostics` = `{"kind":"Array","type":"Vector","shape":[0],"elements":[]}` | PASS | same | all three exactly as written |
| SE-h | `unrepresented_reason` = `{"kind":"Text","value":""}` | PASS | same | exact match (and distinct from the `{"kind":"Null"}` fields elsewhere — Invariant 3) |
| SE-i | solution `value` canonical `(rat -2 1)`/`(rat 2 1)`, `conditions:[]`, `multiplicity:1`, `exactness: SolutionExactness/Exact` | PASS | same | exact match for both elements |
| SE-j | "Abridged below only in the nested symbolic renderings, which keep `pretty`/`canonical` and drop `domain`/`exact`/`nodeCount`/`freeSymbols`." | PASS (as an abridgement declaration) | same | the live nested symbolics carry the full member set (`"domain":"complex","exact":true,"nodeCount":1,"freeSymbols":["x"]`); the doc declares those members dropped, so nothing is falsified — but note the abridgement is *applied* only to the nested values, which is exactly what SE-f contradicts |
| SE-k | "`elapsedTime` and `timings[].elapsed` are produced by the same unit selector as the human `elapsed` string, so the two forms can never disagree." | PASS (structure) / sample numbers non-reproducible | same, 20+ runs | `elapsed`/`elapsedTime` always agree (`"739 µs"`↔`{739,"µs"}`); the block's `1.2 ms`/`1.1 ms` are wall-clock samples, not checkable |
| SE-l | "`timings[].position` is the zero-based offset of the statement in the text the caller supplied" | PASS | `BIN --eval 'x = symbol("x"); solve(x^2 - 4 == 0, x)'` | `[{"position":0,…},{"position":17,…}]`; `script[17:] == 'solve(x^2 - 4 == 0, x)'` |
| SE-m | "`resultKind` is the value kind it produced (`Void` for a statement with no value)" | PASS | `print("a"); 1+1; x = symbol("x")` | kinds `["Void","Natural","Symbolic"]` |
| SE-n | "`hasOutput` says whether it wrote anything with `print`" | PASS | same | `[true,false,false]` |
| SE-o | the "an agent can answer …" enumeration (Was it solved? over what domain? is it complete? how many solutions? what is each solution? what conditions? is it exact?) | PASS | same envelope | `status`/`domain`/`completeness`/`complete`/`solutions.shape`/`solutions.elements[].fields[value].canonical`/`…[conditions]`/`…[exactness]` all resolvable by field name |

### 6b. The complete/completeness mapping table

| id | row | verdict | command | observed |
|---|---|---|---|---|
| MT-a | `Solved` → `true` / `Complete` | PASS | `solve(x^2 - 4 == 0, x)` | true / Complete |
| MT-b | `NoSolutions` → `true` / `Complete` | PASS | `solve(x^2 + 1 == 0, x, real)` | NoSolutions / true / Complete |
| MT-c | `Partial` → `false` / `Partial` | PASS | `solve_full(x^4 - x^2 - 1 == 0, x)` | Partial / false / Partial |
| MT-d | `Unevaluated` → `false` / `Unknown` | PASS | `solve(x == x, x)`, `solve(exp(x) == x, x)` | Unevaluated / false / Unknown |
| MT-e | `BudgetExceeded` → `false` / the kernel's `Completeness` for the subset it stopped on | UNTESTABLE | exhaustive search of the shipped surfaces | nothing in the shipped kernel produces `SolveStatus.BudgetExceeded` (the repo's own mapping tests say the status cannot be produced), so this row is not exercisable from the binary |
| MT-f | "`SolveCompletenessMapping.Of` … so the two fields cannot disagree and the two records cannot drift apart" | PASS | 8 solve scripts incl. `solve_system` | 8/8 held the invariant; `SolveResult` and `SystemSolveResult` both carry `status`/`complete`/`completeness`/`diagnostics` consistently |

### 6c. The NoSolutions / Partial / Unevaluated paragraphs

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| NS-a | "`NoSolutions` means the solution set over the **requested domain is provably empty** … including the case where a denominator or domain condition removed every candidate … reports `complete: true` / `completeness: Complete`" | PASS | `solve(x^2+1==0,x,real)`, `solve(cos(x)==2,x,real)`, `solve(1/(x-1)==0,x,real)` | all three: `NoSolutions`, `complete:true`, `completeness:Complete` |
| NS-b | "Only `Partial`, `Unevaluated` and `BudgetExceeded` report `complete: false`." | PASS | the mapping sweep | every `complete:false` observed was Partial or Unevaluated; every Solved/NoSolutions was `true` |
| PU-a | "When the solver cannot represent the whole set, `status` is `Partial` and `unrepresented_count`/`unrepresented_reason` say how much is missing and why." | PASS | `solve_full(x^4 - x^2 - 1 == 0, x)` | `unrepresented_count={"kind":"Integer","value":"2","exact":true}`, `unrepresented_reason="complex algebraic roots not supported (RootOf is real-only in v1)."` |
| PU-b | "When nothing is representable the status is `Unevaluated` — never a complete-looking subset." | PASS | `solve(x == x, x)`, `solve(exp(x) == x, x)`, `solve(sqrt(x) == -1, x, real)` | all three `Unevaluated` with `complete:false`/`Unknown` |

## 7. Positions and locations

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| PL-a | "Every position … is a **zero-based offset into the characters** of the text the caller supplied, exactly as that surface received it" | **FAIL for a literal "characters" reading on astral text** | `BIN --eval 'print("😀"); det(1)'` | reported `position = 13`; a Unicode-scalar slice gives `'et(1)'`, a UTF-16 slice gives `'det(1)'`. All BMP text (incl. BOM/CRLF/CR) is exact. See J-4 |
| PL-b | `--eval <script>` → "the argument string, character for character" | PASS | `--eval 'a = 1\nb = 2\ndet(a)'` | timings positions `[0,6,12]`; diagnostic `position 12` |
| PL-c | `--file <path>` → "the file's decoded characters; a leading byte-order mark is the character U+FEFF at offset 0 and is **not** removed" | PASS | `--file pos_BOM_CRLF.ls` (file bytes `EF BB BF` + CRLF script) | positions `[1,8,15]`, diagnostic `position 15, line 3, column 1` |
| PL-d | `--stdin` → "the characters read from standard input" | PASS | `--stdin` with the same bytes written raw to the pipe | LF `[0,6,12]`, CRLF `[0,7,14]`, CR `[0,6,12]`, BOM+CRLF `[1,8,15]` — identical to `--eval`/`--file` |
| PL-e | "Byte-identical input therefore reports identical positions on every surface." | PASS | same bytes through `--eval`, `--file`, `--stdin` (and the bare-path form), incl. a script with `é`/`π` | all three surfaces: `positions=[0,13]`, `position 13, line 1, column 14`; bare path == `--file` |
| PL-f | "the text it handed over … sliced at the reported offset starts at the statement … `text.Substring(position)` names it, on every surface, for the same bytes" | PASS under UTF-16 `Substring` / **FAIL under Unicode-scalar slicing** | `BIN --eval 'print("😀"); det(1)'` | UTF-16 `Substring(13)` = `'det(1)'` ✓; Python/JS/Rust codepoint slice = `'et(1)'` ✗. Same root cause as PL-a; J-4 |
| PL-g | "`a = 1` / `b = 2` / `det(a)` on three lines reports its third statement at 12 with LF separators, at 14 with CRLF separators, and at 15 when that CRLF file is saved as 'UTF-8 with BOM'" | PASS | the three scripts on all three surfaces | `[0,6,12]` / `[0,7,14]` / `[1,8,15]` — the third statement is at exactly 12, 14, 15 |
| PL-h | "The engine itself drops a leading U+FEFF … which is why a 'UTF-8 with BOM' file still evaluates" | PASS | `--file` on the BOM file | `ok=false` but only because `det(a)` is a type error — the three statements parsed and ran (timings has 3 entries); a BOM file with valid statements returns `ok:true` |
| PL-i | "`line` and `column` are 1-based and read off that same text, with CRLF, CR and LF each ending a line once — so a Windows script, a Unix script and a classic-Mac script … report the same line and column." | PASS | same three separators | LF/CRLF/CR all report `line 3, column 1` for the third statement, while `position` is 12/14/12 |
| PL-j | "Its `position`/`line`/`column` index the caller's text by the same rule as `timings[].position` … so the two forms of one envelope can never disagree about where a failure is." | PASS | the error envelopes above | every case: `diagnostics[0].position == timings[last].position` (12/14/12/15) and `column == position-in-line + 1` |

## 8. Error envelope

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| EE-a | the worked error envelope (`ok:false`, `code:"InvalidOperation"`, `category:"DomainError"`, the exact `solve(): currently supports domains real and complex; got integer.` message, `recoverable:true`, `diagnostics[0] = {message,position:17,line:1,column:18}`, `timings` = Symbolic@0 + Void@17) | PASS | `BIN --eval 'x = symbol("x"); solve(x^2 - 4 == 0, x, integer())'` | reproduced **exactly**, including position 17 / line 1 / column 18 and the two `timings` entries with `resultKind` `Symbolic` then `Void`; only the wall-clock numbers differ (`579.7 µs` vs the block's `46.22 ms`), which are samples. The live envelope also carries `"output":[]`, which the block does not show — the doc does not claim the block is exhaustive |
| EE-b | "Invariant 4 is **not scoped to success**: the two structural durations are present on the error path too, including for an error raised before any engine exists (a parse error, an unreadable script file), where `timings` is present and empty." | PASS | `--eval '[1,2;3,4]'`; `--file does-not-exist.ls` | parse error: `elapsed:"365.9 µs"`, `elapsedTime:{365.9,"µs"}`, `timings:[]`; unreadable file: `elapsed:"0 ns"`, `elapsedTime:{0,"ns"}`, `timings:[]` |
| EE-c | "A consumer never parses a unit suffix on either path." | PASS | same two envelopes | `unit` present as its own field on the error path |
| EE-d | "The statement that FAILED is still a timed statement: it is reported with `resultKind` `Void`." | PASS | `det(a)` and the domain refusal | the failing statement appears in `timings` with its own position and `"resultKind":"Void"` |
| EE-e | "Categories are spelled from the one taxonomy, `ErrorCategory`: ParseError, DomainError, UnsupportedOperation, BudgetExceeded, NoSolution, TypeMismatch, InternalInvariantFailure." | PASS | 20 error envelopes | observed envelope categories: `ParseError`, `DomainError`, `TypeMismatch` — all members of the declared seven-member enum (read from source); no out-of-taxonomy spelling, no `InternalInvariantFailure` |
| EE-f | "Exit codes: 0 success, 1 script/diagnostic error, 2 usage error." | PASS | ok run; parse error; runtime error; unreadable file; no args; `--bogus`; `--print-budget 0` | 0 / 1 / 1 / 1 / 2 / 2 / 2 |
| EE-g | "A call with the wrong number of arguments is rejected at the call site … It crosses as a recoverable argument error (`code: InvalidArgument`, `category: TypeMismatch`) naming the builtin and both counts, never as an internal invariant failure." | PASS | 15 zero/one-arg calls (`compile_full(1)`, `dft()`, `symbol()`, `zeros()`, `subs(1)`, `powerseries(1,2,3,4)`, …) | 15/15 `InvalidArgument`/`TypeMismatch`/`recoverable:true`, message names builtin + counts; no `InternalInvariantFailure` |
| EE-h | the example `compile_full(): expected 2 arguments; got 1.` | PASS | `BIN --eval 'compile_full(1)'` | `{"code":"InvalidArgument","category":"TypeMismatch","recoverable":true,"message":"compile_full(): expected 2 arguments; got 1."}` — exact |
| EE-i | "The counts come from the builtin's declared metadata (`Parameters`, `MinArity`, `Variadic`), so a builtin that legitimately accepts a shorter form declares it (`symbol(name [, domain])`, `solve(f, x [, domain])`, `dft(x [, n])`)" | PASS | registry scan + short-form calls | `symbol` parameters `["name","domain"]` minArity 1; `solve` `["f","x","domain"]` minArity 2; `dft` `["x","n"]` minArity 1; `symbol("x")`, `symbol("x", real())`, `solve(f,x)`, `solve(f,x,real)`, `dft(x)`, `dft(x,2)` all evaluate |
| EE-j | "The error envelope's `diagnostics` array is the **parser's source-position form** (`message`/`position`/`line`/`column`) — a different record from the `Diagnostic` above, and also an array." | PASS | parse + runtime + file errors | every error `diagnostics` entry has exactly `message, position, line, column` and no `code`/`category`; always an array |
| EE-k | "The two are never mixed: a result record carries `Diagnostic` records, the error envelope carries parse-site diagnostics." | PASS | `solve_full(x^4-…)` (ok) vs `[1,2;3,4]` (error) | ok envelope: `Diagnostic` records present, no parse-site array; error envelope: parse-site array present, zero `Diagnostic` records anywhere |

## 9. CI

| id | sentence | verdict | command | observed |
|---|---|---|---|---|
| CI-a | "`.github/workflows/ci.yml` publishes the runner as a Native AOT binary and then **executes it** against the scenarios above, asserting the envelope's structure." | PASS (static) | read `.github/workflows/ci.yml` (393 lines); CI not executed here | job `aot-smoke` runs `dotnet publish Lovelace.Run/Lovelace.Run.csproj -p:PublishAot=true -o out/aot`, then runs 5 python-asserted scenarios against `out/aot/Lovelace.Run` (solve Record/Enum, Partial completeness, rank-2 row-major array, stdout purity with `print`, structural error code+category) |

---

## FAIL evidence (command + observed output, twice each)

**J-1 — the worked solve envelope's `revision` (doc line 123)**
```
BIN --eval "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)" --omit-functions --omit-variables
run1 rc=0 revision=82 protocolVersion=1 symbolicFormatVersion='#!lovelace-sym 1' mathIrVersion=2
run2 rc=0 revision=82 protocolVersion=1 symbolicFormatVersion='#!lovelace-sym 1' mathIrVersion=2
(without the omit flags, 3 further runs: revision=82 each)
```
The document prints `"revision": 83`. `revision` is deterministic per script (it comes from the engine
snapshot), so this is a stale literal, not a race: `1+1`→81, `0`→81, `print("a"); print("b")`→80,
`x = symbol("x"); x`→82.

**J-2 — the Diagnostic form's empty array (doc lines 78 and 91)**
```
BIN --eval "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)" --omit-functions --omit-variables
run1 rc=0 solve.diagnostics={"kind": "Array", "type": "Vector", "shape": [0], "elements": []}
run2 rc=0 solve.diagnostics={"kind": "Array", "type": "Vector", "shape": [0], "elements": []}

BIN --eval "x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)" --omit-functions --omit-variables
run1 rc=0 diagnostic.details={"kind": "Array", "type": "Vector", "shape": [0], "elements": []}
run2 rc=0 diagnostic.details={"kind": "Array", "type": "Vector", "shape": [0], "elements": []}
```
The doc's literal block is `{"kind":"Array","shape":[0],"elements":[]}` — no `"type"`. The same document
(line 153) shows the solve result's `diagnostics` **with** `"type":"Vector"`, so the document contradicts
itself; the binary matches line 153. The substance the paragraph is arguing for (never `Null`, never `""`)
holds.

**J-3 — the `solutions` array in the worked envelope (doc line 137)**
```
BIN --eval "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)" --omit-functions --omit-variables
run1 rc=0 solutions={"kind":"Array","type":"Vector","shape":[2]} | families={...,"type":"Vector","shape":[0]}
             | common_conditions={...,"type":"Vector","shape":[0]} | diagnostics={...,"type":"Vector","shape":[0]}
run2 (identical)
```
The doc shows `"type"` on `families`, `common_conditions` and `diagnostics` in the same block but omits it on
`solutions`, and the only announced abridgement is "in the nested symbolic renderings".

**J-4 — "offset into the characters" / `text.Substring(position)` (doc lines 205 and 216)**
```
BIN --eval "print(\"\U0001F600\"); det(1)" --omit-functions --omit-variables
run1 rc=1 position=13 scalar-slice='et(1)' utf16-slice='det(1)'
run2 rc=1 position=13 scalar-slice='et(1)' utf16-slice='det(1)'
```
The script is `print("\U0001F600"); det(1)` (25 UTF-16 units, 24 Unicode scalars). `det` starts at Unicode-scalar
index 12 / UTF-16 index 13. The binary reports 13, i.e. **UTF-16 code units**, matching .NET's
`text.Substring(position)` (which the doc names) but not "characters" as a Python/JS/Rust consumer reads
the word. All-BMP text — including the doc's own BOM/CRLF/CR examples — is exact under both readings.

---

## Findings

| id | severity | the sentence | one-line repro |
|---|---|---|---|
| J-1 | P2 | `"revision": 83` in "A real solve envelope" (doc line 123) | `Lovelace.Run.exe --eval "x = symbol(\"x\"); solve(x^2 - 4 == 0, x)"` → `"revision":82` (5/5 runs) |
| J-2 | P2 | "the array is **empty** (`{"kind":"Array","shape":[0],"elements":[]}`)" — Diagnostic form, and `details` in the same block (doc lines 78, 91); also Invariant 2's value forms | `solve(x^2-4==0,x)` → `"diagnostics":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}` |
| J-3 | P2 | the worked envelope's `solutions` member list (doc line 137), read with "Abridged below only in the nested symbolic renderings" (line 114) | `solve(x^2-4==0,x)` → `solutions={"kind":"Array","type":"Vector","shape":[2],…}` |
| J-4 | P2 | "a zero-based offset into the **characters**" and "`text.Substring(position)` names it, on every surface" (doc lines 205, 216) | `--eval 'print("\U0001F600"); det(1)'` → position 13; UTF-16 slice `det(1)`, Unicode-scalar slice `et(1)` |

No P0 or P1 finding was reproduced: every value the document's machine API promises was present and
correct on every path probed.

### Mentions of the known-open list (not new findings)

| known-open item | reproduced? | evidence |
|---|---|---|
| `--print-budget` non-monotonicity | yes, matches | `expand((x+1)^10)` at budget 30 → 57-char `pretty`; at budget 20 → 37-char `pretty` (smaller budget, longer rendering), 2/2 runs |
| `--omit-variables` ignored on the cancelled path | yes, matches | `--eval 'x = symbol("x"); solve(x^4 - x^2 - 1 == 0, x)' --cancel-after 1 --omit-variables` → `"variables":[]` — the *empty* array is honoured here |
| `--text` writes a failure to stderr with empty stdout | yes, matches | `--eval '[1,2;3,4]' --text` → rc=1, `stdout=''`, `stderr="Error [ParseError/ParseError]: Expected 'RBracket' but found ';' at position 4."` |
| `evalf` cost above 1000 places, the `PrecisionExplicitlySet` one-way latch, the special-angle exactness bound | not re-probed (out of scope for a document audit) | — |

---

## Could not check

1. **`truncationReason: "depth-limit"`** — unreachable from this surface: the runner only ever builds
   `PrintBudget(MaxNodes: n)`; the depth branch needs a `MaxDepth` the CLI never sets.
2. **The non-null `location` branch of a Diagnostic** (`DiagnosticLocation` with `start_line`/`start_column`/
   `end_line`/`end_column`) — no shipped producer constructs one; all 5 diagnostic-producing scripts give
   `{"kind":"Null"}`.
3. **The `BudgetExceeded` row of the solve-completeness table** — nothing in the shipped kernel produces
   that solve status, so the row cannot be exercised against the binary.
4. **The literal elapsed samples** in both worked envelopes (`1.2 ms`, `1.1 ms`, `8.89 ms`, `16.41 ms`,
   `46.22 ms`) — wall-clock values; only the structure and the `elapsed`↔`elapsedTime` agreement are checkable.
5. **The `--text` surface** — the document never mentions it, so neither Invariant 1 nor Invariant 4 is
   asserted for it. On the failure path it matches the known-open list (rc=1, empty stdout, error on
   stderr); on the success path it writes a human summarising string to stdout, which is a mode the
   document never describes. Recorded as a scope gap, not as a document finding.
6. **The CI claim** — verified only by reading `.github/workflows/ci.yml`; no CI run was performed here.
7. **The degenerate abbreviation** (the rendering *is* its first token → the wire's `"…"` with nothing kept) —
   needs `--print-budget 0`, which exits 2 as a usage error; not reachable, and the document is silent on it.
8. **"never a re-ordered one"** — verified as prefix equality over one expression family
   (`expand((x+1)^10)`, 59 budgets, 118 renderings); not exhaustively over every value shape.
9. **Non-BMP/encoding corners** beyond those probed — UTF-16-encoded script files, lone surrogates,
   and surfaces other than `--eval`/`--file`/`--stdin`/`bare path` were not exercised.
10. **The `--help` text itself** — read and used (it documents `--eval`, `--file`, `--stdin`,
    `--omit-functions`, `--omit-variables`, `--print-budget`, `--cancel-after`, `--json`, `--text`,
    `--plot-dir`, `--plot-file`, `--help/-h`); the document describes none of these options except the
    protocol-relevant ones, so cross-checking the rest is out of scope.
11. **Doc-silent observations recorded but not scored** (no sentence to fail):
    `print("a\nb")` returns **two** `output[]` entries for one `print` call;
    `pi()`/`e()` publish 1000 decimals with no truncation keys (the cap is documented only for `evalf`);
    a rank-1 array reports `"type":"Vector"` where the value-forms block shows `"type":"Array"`;
    `plot(sin(x), x, -1, 1)` without `--plot-dir` returns rc=1;
    `symbol("x", "bogus")` is accepted and creates no assumption while `symbol("x", real())` correctly
    records `x in Real`.
