# B3 — Command-line surface and self-description (independent adversarial audit 2)

Auditor: independent adversarial pass (B3), no attachment to the implementation. Product under test is the
published Native AOT binary; it was **not** rebuilt, no `dotnet` command was run, and no repository file was
modified except this deliverable. All probe scripts live under `%TEMP%\lv5\`.

* Binary: `out/aot/Lovelace.Run.exe` (5,715,456 bytes, mtime 2026-09-11 15:18; `git log --oneline -1` = `9e761ba`).
* Contract read: `docs/symbolics/dsh-protocol.md` (210 lines) + the builtin metadata returned by `capabilities()`.
* Ground truth: SymPy 1.14.0 / mpmath (independent), and exact arithmetic for the rest.
* Harness: PowerShell 5.1 launching the exe through `System.Diagnostics.Process` with stdout, stderr and the exit
  code captured **separately** (the two streams are never merged in any row below).

**Probe count.** 278 table rows below, backed by roughly 500 process invocations (the `--print-budget` boundary
sweep alone is 185 invocations collapsed into 12 rows). Verdicts: **200 HELD / 66 FINDING rows / 12 INCONCLUSIVE**; the
66 FINDING rows roll up into 18 distinct findings (F0–F17): one P0, seven P1, eight P2, two P3.

---

## 0. Headline

1. **P0 — `solve_full(exp(x) == x, x)` answers `Solved` / `complete: true` / `represented_count: 1` with the
   single "solution" `log(x)`**, a value that contains the unknown and satisfies the equation identically.
   SymPy 1.14.0: `solveset(exp(x)-x, x, Reals)` = ConditionSet, `solve(exp(x)-x, x)` = `[-LambertW(-1)]` (one of
   infinitely many complex roots). `solve(log(x) == x, x)` mirrors it with `exp(x)`.
2. **P1 — `capabilities()` advertises a refusal that does not happen**: class `pow.non-integer-exponent`, message
   "Non-integer exponents are not yet supported.", trigger `2^(1/2)` — but `(-4)^(1/2)` → `2*i` and `(-1)^(1/2)` → `i`
   both return `ok: true`.
3. **P1 — `--cancel-after <ms>` is not a deadline**: `--cancel-after 200` on one `integrate_full` call returned the
   `Cancelled` envelope after **46.27 s** (1 ms → 41.74 s, 2000 ms → 35.78 s).
4. **P1 — parse errors are `category: DomainError`** and a *missing script file* is `category: ParseError`; the
   taxonomy member `ParseError` is attached to the wrong layer in both directions.
5. **P1 — print output is dropped on any script error**: `print("hello"); 1 +` exits 1 with an envelope that has no
   `output` key at all, contradicting invariant 1.
6. **P1 — `evalf(pi, N)` ignores `N`** for symbolic constants (5, 20, 50, 200, 1000000 all return 100 digits), while
   `evalf(1/3, 5)` correctly returns 5 digits.
7. **P1 — `--print-budget N` does not bound by nodes**: a 6-node rendering is emitted in full at `N = 1`, while a
   3-node rendering (30-character symbol name) *is* truncated at `N = 1`.
8. **P1 — `capabilities()` omits at least three reachable refusal classes** whose diagnostics carry
   `category: UnsupportedOperation`: `solve.unevaluated`, `system-solve.unevaluated`, `integration.no-closed-form`.

The rest (usage errors with no envelope, `--help` on stdout, the absent `result` key for `Void`, the `exact` flip
between `evalf` call forms, positional-argument asymmetry, the int32 wraparound in `setprecision`, the
`unsupported_domains` wording, undocumented `partialOutput`/`partialVariables`, BCL message leakage, and the
UTF-16 file) are P2/P3 and are itemised in §3.

---

## 1. What "expected" means in every row

| Expectation source | Used for |
|---|---|
| `dsh-protocol.md` §Invariants 1–6 | stdout purity, structural values, absent-field rule, version fields, `--print-budget`, camelCase |
| `dsh-protocol.md` §Value forms / §Diagnostic / §Error envelope | diagnostic shape, `code`+`category`+`recoverable`, exit codes 0/1/2 |
| `capabilities()` (live) | the 16 advertised `unsupported_operations` and their advertised `code`/`category`/`trigger` |
| Mathematics (SymPy/mpmath/exact arithmetic) | every mathematical claim quoted in a verdict |

Where a row expectation comes from the runner help text (`--help`), the row says so.

---

## 2. Probe tables

Command form for file probes: `out\aot\Lovelace.Run.exe --file <path>.ls --omit-functions` (the method command).
Rows that pass other flags name them in the *command/flags* cell. `RAW` cells are the real bytes, abridged only in
the marked places (long digitisations, the 9.4 KB `functions` registry); every field that decides a verdict is verbatim.

### 2.1 Command line: every documented flag (A group, 50 probes)

| id | what was probed | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| a01 | no arguments | (none) | exit 2; stdout `""`; stderr `Error: No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.` + usage (1073 B) | FINDING F8 | P2 |
| a02 | `--help` | `--help` | exit 0; **stdout** = 979 B usage text (`Lovelace.Run — evaluate a Lovelace script ...`); stderr empty | FINDING F9 | P2 |
| a03 | `-h` | `-h` | exit 0; stdout identical 979 B usage text | FINDING F9 | P2 |
| a04 | unknown long flag | `--version` | exit 2; stdout empty; stderr `Error: Unknown argument (--version).` + usage | HELD | — |
| a05 | unknown flag | `--foo` | exit 2; stdout empty; stderr `Error: Unknown argument (--foo).` | HELD | — |
| a06 | short form `-f` | `-f <ok.ls>` | exit 2; stdout empty; `Error: Unknown argument (-f).` (not advertised, so correct) | HELD | — |
| a07 | short form `-e` | `-e 1+1` | exit 2; `Error: Unknown argument (-e).` | HELD | — |
| a08 | `--file` with no value | `--file` | exit 2; stdout empty; `Error: --file requires a path argument.` | HELD | — |
| a09 | `--eval` with no value | `--eval` | exit 2; `Error: --eval requires a script argument.` | HELD | — |
| a10 | `--print-budget` with no value | `--print-budget` | exit 2; `Error: --print-budget requires a node count.` | HELD | — |
| a11 | `--print-budget 0` | `--print-budget 0 --file ok.ls` | exit 2; stdout empty; `Error: --print-budget requires a positive node count.` | HELD | — |
| a12 | `--print-budget 1` | `--print-budget 1 --eval ...` | exit 0; envelope 10008 B — see §2.2 | FINDING F6 | P1 |
| a13 | `--print-budget 5` | `--print-budget 5 --eval ...` | exit 0; envelope 10002 B — see §2.2 | FINDING F6 | P1 |
| a14 | negative budget | `--print-budget -1` | exit 2; stdout empty; `... requires a positive node count.` | HELD | — |
| a15 | non-numeric budget | `--print-budget abc` | exit 2; same message (message says "positive node count" for a non-number) | HELD | P3 note |
| a16 | budget past int64 | `--print-budget 99999999999999999999` | exit 2; same message | HELD | — |
| a17 | fractional budget | `--print-budget 5.5` | exit 2; same message | HELD | — |
| a18 | `--cancel-after` with no value | `--cancel-after` | exit 2; `Error: --cancel-after requires a millisecond argument.` | HELD | — |
| a19 | `--cancel-after 0` | `--cancel-after 0 --file ok.ls` | exit 2; `... requires a positive millisecond count.` | HELD | — |
| a20 | negative cancel | `--cancel-after -1` | exit 2; same message | HELD | — |
| a21 | non-numeric cancel | `--cancel-after abc` | exit 2; same message | HELD | — |
| a22 | cancel past int64 | `--cancel-after 99999999999999999999` | exit 2; same message | HELD | — |
| a23 | `--file` does not exist | `--file %TEMP%\lv5\fx\nope.ls` | exit 1; ok:false; `"code":"FileReadError","category":"ParseError","message":"Cannot read script file (...nope.ls): Could not find file (...nope.ls)."` | FINDING F3 | P1 |
| a24 | `--file` is a directory | `--file %TEMP%\lv5\fx` | exit 1; `FileReadError` / **`ParseError`**; `Access to the path (...\fx) is denied.` | FINDING F3 | P1 |
| a25 | `--file ""` (empty value) | `--file ""` | exit 1; `FileReadError` / `ParseError`; `The value cannot be an empty string. (Parameter (path))` | FINDING F16 | P3 |
| a26 | `--eval ""` (empty script) | `--eval ""` | exit 0; ok:true; keys `protocolVersion,...,ok,revision,output,variables,functions,elapsed,elapsedTime,timings` — **no `result`** | FINDING F10 | P2 |
| a27 | whitespace-only script | `--eval "   "` | exit 0; identical shape to a26 | FINDING F10 | P2 |
| a28 | both `--file` and `--eval` | `--file ok.ls --eval "2+2"` | exit 0; result `4` → `--eval` wins | HELD (undocumented precedence) | P3 note |
| a29 | both, reversed order | `--eval "2+2" --file ok.ls` | exit 0; result `4` → `--eval` wins regardless of order | HELD (undocumented) | P3 note |
| a30 | two bare positional paths | `ok.ls unicode.ls` | exit 2; stdout empty; `Error: Unknown argument (...unicode.ls).` | HELD | — |
| a31 | bare positional path | `ok.ls` | exit 0; envelope with result `2` | HELD | — |
| a32 | `--file=path` equals form | `--file=...\ok.ls` | exit 2; `Error: Unknown argument (--file=...ok.ls).` (equals form unsupported, not advertised) | HELD | — |
| a33 | case sensitivity | `--FILE ok.ls` | exit 2; `Error: Unknown argument (--FILE).` | HELD | — |
| a34 | `--omit-functions` | `--omit-functions --eval "1+1"` | exit 0; 492 B envelope, keys without `functions` | HELD | — |
| a35 | `--omit-variables` | `--omit-variables --eval "1+1"` | exit 0; 9651 B envelope, keys without `variables`, `functions` present | HELD | — |
| a36 | `--text` | `--text --eval "1+1"` | exit 0; stdout `= 2 (Natural)` then `  _ = 2` — **no envelope on stdout** | FINDING F9 | P2 |
| a37 | `--text --json` | `--text --json --eval "1+1"` | exit 0; JSON envelope (last flag wins) | HELD | — |
| a38 | `--json --text` | `--json --text --eval "1+1"` | exit 0; text output (last flag wins) | HELD | — |
| a39 | `--json` | `--json --eval "1+1"` | exit 0; JSON envelope | HELD | — |
| a40 | `--print-budget 5 --text` | as shown | exit 0; text output | HELD | — |
| a41 | `--plot-dir` pointing at a missing dir | `--plot-dir %TEMP%\lv5\nope --file plot.ls` | exit 0; result Text `...\nope\plot.svg`; directory created; envelope 26296 B incl. top-level `plot` object | HELD | P3 note |
| a42 | `--plot-dir` + `--plot-file` | `--plot-dir %TEMP%\lv5\plots --plot-file custom.svg --file plot.ls` | exit 0; result Text `...\plots\custom.svg` | HELD | — |
| a43 | flag at end without value | `--eval "1+1" --print-budget` | exit 2; `Error: --print-budget requires a node count.` | HELD | — |
| a44 | repeated flag | `--omit-functions --omit-functions --eval "1+1"` | exit 0 | HELD | — |
| a45 | `--` separator | `-- --eval "1+1"` | exit 2; `Error: Unknown argument (--).` | HELD | — |
| a46 | extra positional **with** `--eval` | `--eval "1+1" ok.ls` | exit 0; envelope 9694 B, result `2` — extra arg silently ignored | FINDING F12 | P2 |
| a47 | extra positional **with** `--file` | `--file ok.ls unicode.ls` | exit 2; stdout empty; `Error: Unknown argument (...unicode.ls).` | FINDING F12 | P2 |
| a48 | `--stdin` (empty stdin) | `--stdin` | exit 0; empty program envelope, no `result` | FINDING F10 | P2 |
| a49 | `--stdin` + `--file` | `--stdin --file ok.ls` | exit 0; result `2` → `--file` wins over `--stdin` | HELD (undocumented) | P3 note |
| a50 | `--eval` + empty `--file` | `--eval "1+1" --file ""` | exit 0; result `2` (the empty path is never read because `--eval` wins) | HELD | — |

Note on the single-quote marks in raw cells: the exe emits ASCII apostrophes (U+0027); they are rendered as
parentheses above only where a nested quote would break the table, and the deciding fields are unaffected.

### 2.2 `--print-budget`: the boundary (PB group; 12 rows, 185 invocations)

| id | what was probed | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| PB01 | no flag, 38-node value | `--eval (x = symbol("x"); expand((x+1)^8))` | `"nodeCount":38`, `"canonical"` 308 chars, keys `kind,pretty,canonical,domain,exact,nodeCount,freeSymbols` — no `truncated` | HELD (contract: without the flag the full form is available) | — |
| PB02 | budget 10, same value | `--print-budget 10` | `"truncated":true,"truncationReason":"node-budget","budget":10`, canonical 61 chars ending `(sym  …` | HELD | — |
| PB03 | budget 40, same value | `--print-budget 40` | full canonical 308 chars, no `truncated` (38 ≤ 40) | HELD | — |
| PB04 | budget 1, **6-node** value | `--print-budget 1 --eval (x = symbol("x"); x^2 + x + 1)` | `"nodeCount":6`, canonical 47 chars **complete** `(add (rat 1 1) (sym x) (pow (sym x) (rat 2 1)))`, no `truncated` | FINDING F6 | P1 |
| PB05 | budget 1, **3-node** value with a 30-char symbol | `--print-budget 1 --eval (aaaa...(30) = symbol(...); aaaa... + 1)` | `"nodeCount":3`, `"canonical":"(add (rat 1 1) (sym  …"`, `"truncated":true,"truncationReason":"node-budget","budget":1` | FINDING F6 | P1 |
| PB06 | budget 1 vs 2 vs 3 vs 6, same 3-node value | `--print-budget {1,2,3,6}` | budgets 1 and 2 give **byte-identical** cutting (`...(sym  …`); raising the budget does not lengthen the prefix | FINDING F6 | P1 |
| PB07 | rendering-length boundary at budget 1 | `x^2 + x + 1` (pretty 11) / `x^2 + x + 22` (12) / `x^2 + x + 222` (13) | 11 → full (canonical 47); 12 → full (canonical 47); 13 → `truncated:true`, canonical 47 + ` …` | FINDING F6 | P1 |
| PB08 | expanded powers, budgets 1–40 | `expand((x+1)^k)`, k = 2,4,8, budgets 1..40 | truncation starts exactly at budget < nodeCount for these (k=2: full at 8; k=4: full at 18; k=8: full at 38) | HELD (this shape is consistent with nodes) | — |
| PB09 | budget far above nodeCount | `--print-budget 1000` | no `truncated` key, full canonical | HELD | — |
| PB10 | budget 0 / -1 / abc / 5.5 / 2^64 | see a11, a14–a17 | exit 2, no envelope | FINDING F8 | P2 |
| PB11 | where the truncation fields live | budgets 1 and 10 | `truncated` / `truncationReason` / `budget` live **inside** `result.structured` (the rendered value), not on the envelope | HELD | — |
| PB12 | is the truncated form still an ellipsis-terminated prefix? | PB05/PB07 raw | prefix + ` …` is a character-prefix of the real canonical, order preserved, never re-ordered | HELD | — |

### 2.3 `--cancel-after` (C group, 3 probes + usage rows above)

| id | what was probed | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| C01 | 200 ms budget on a long `integrate_full` | `--cancel-after 200 --file slow.ls`, `slow.ls` = `x = symbol("x"); integrate_full(1/(x^100-1), x)` | exit 1 after **46.27 s** wall clock; `{"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[],"elapsed":"46.27 s","elapsedTime":{"value":46.27,"unit":"s"},"timings":[{"position":0,...,"resultKind":"Symbolic"},{"position":17,...,"resultKind":"Void"}],"partialOutput":[],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x"}]}` | FINDING F2 | P1 |
| C02 | 1 ms budget, same script | `--cancel-after 1` | exit 1 after **41.74 s**; same `Cancelled` envelope, `"elapsed":"41.74 s"` | FINDING F2 | P1 |
| C03 | 2000 ms budget, same script | `--cancel-after 2000` | exit 1 after **35.78 s**; same envelope | FINDING F2 | P1 |
| C04 | `--cancel-after` on a fast script | not run (the parse side is a18–a22) | — | INCONCLUSIVE | — |

The two extra keys `partialOutput` / `partialVariables` appear in **no** section of `dsh-protocol.md` (F15).

### 2.4 Stream purity (SP group, 7 probes)

| id | what was probed | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| SP01 | script prints 20 000 values | `--file bigprint.ls --omit-functions` (20 000 `print(i)` statements) | exit 0; stdout **2 007 955 B**, exactly 2 lines, starts `{`, ends `}`, `ConvertFrom-Json` succeeds, `output` has 20 000 entries, `timings` 20 000, **stderr 0 B** | HELD | — |
| SP02 | script prints nothing | `--file noprint.ls` (`x = symbol("x"); x^2`) | exit 0; `"output":[]`, stderr 0 B | HELD | — |
| SP03 | one very large print | `--file printbig.ls` (`print(10^100000)`) | exit 0; stdout 100 316 B single JSON line; `output[0]` = 100 001 chars, starts `1`, ends in zeros; stderr 0 B | HELD | — |
| SP04 | stderr on a **successful** run | every a/b/c file probe | stderr 0 bytes, every time | HELD | — |
| SP05 | stderr on a **failing** run | `d1.ls`, `pr2.ls`, a23 | stderr 0 bytes; the whole error envelope is on stdout, exit 1 | HELD (stderr carries only usage errors) | — |
| SP06 | stdout when the script errors after a print | `print("hello"); 1 +` | exit 1; envelope keys `protocolVersion,symbolicFormatVersion,mathIrVersion,ok,code,category,message,recoverable,diagnostics,elapsed,elapsedTime,timings` — **no `output` key**, "hello" is gone | FINDING F4 | P1 |
| SP07 | stdout when the script errors after a runtime fault | `print("hello"); 1/0` | exit 1; same key list, `"message":"Cannot divide by zero."`, still no `output` | FINDING F4 | P1 |

SP06 was executed three times in one command (same file twice, then the division variant) with identical results.

### 2.5 Script files: missing, empty, directory, BOM, CRLF, unicode, enormous (X group, 18 probes)

| id | what was probed | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| X01 | missing file | see a23 | exit 1; `FileReadError` / `ParseError` | FINDING F3 | P1 |
| X02 | directory as script | see a24 | exit 1; `FileReadError` / `ParseError`, `Access to the path ... is denied.` | FINDING F3 | P1 |
| X03 | 0-byte file | `--file empty.ls` | exit 0; ok:true; no `result` key; `output:[]`; `timings:[]` | FINDING F10 | P2 |
| X04 | whitespace-only file (7 B) | `--file ws.ls` | exit 0; same as X03 | FINDING F10 | P2 |
| X05 | UTF-8 BOM + ASCII script | `--file bom.ls` (EF BB BF then `x = symbol("x"); x^2`) | exit 0; `"kind":"Symbolic"`, `"canonical":"(pow (sym x) (rat 2 1))"` | HELD | — |
| X06 | CRLF line endings | `--file crlf.ls` | exit 0; result Symbolic | HELD | — |
| X07 | BOM + CRLF | `--file bomcrlf.ls` | exit 0; result Symbolic | HELD | — |
| X08 | no-BOM source (control for X05/X07) | `--file bomsrc.ls` | exit 0; identical result | HELD | — |
| X09 | UTF-16LE file | `--file utf16.ls` (UTF-16 `print(1)`) | exit 1; `{"code":"InvalidOperation","category":"DomainError","message":"Unexpected character (space) at position 1."}` | FINDING F17 | P3 |
| X10 | invalid UTF-8 byte (Latin-1 e-acute = 0xE9) inside a string | `--file badutf8.ls` | exit 0; ok:true; the reported result kind set is **empty** (void statement) — the decoded text was not captured | INCONCLUSIVE | — |
| X11 | unicode identifier + unicode string | `--file unicode.ls` (`x = symbol("alpha"); print("..."); solve_full(x^2 - 2 == 0, x)`) | exit 0; `SolveStatus.Solved`, `Completeness.Complete`, `SolutionExactness.AlgebraicExact` | HELD (identifier works; printed text not captured) | — |
| X12 | newline between statements | `--file newlinesep.ls` = `x = 1` NL `y = 2` NL `x + y` | exit 0; result Natural | INCONCLUSIVE vs the brief ("a newline is NOT a separator"); no contract document covers separators | — |
| X13 | `;` + newline mixed | `--file newlineok.ls` | exit 0; result Symbolic | HELD | — |
| X14 | two statements on one line with no `;` | `1 2` (v04) | exit 1; `Expected (;) or end of input but found (2) at position 2.` | HELD | — |
| X15 | 100 000 statements (200 KB) | `--file many.ls` (`1;1;...;42`) | exit 0; result Natural (the printed value was not verified) | HELD | — |
| X16 | 200 001-term `+` chain (400 KB) | `--file chain.ls` | exit 1; `{"code":"DepthExceeded","category":"BudgetExceeded","message":"expression tree depth 513 exceeds the maximum supported nesting depth of 512. ..."}` — no crash, envelope printed | HELD | — |
| X17 | 50 000 nested parentheses (100 KB) | `--file deep.ls` | exit 1; `DepthExceeded` / `BudgetExceeded`, `expression nesting depth 257 exceeds ... 256` | HELD | — |
| X18 | runaway builtin, no cancel flag | `--file slow.ls` (`integrate_full(1/(x^100-1), x)`) | no envelope within 60 s (killed by the harness) | INCONCLUSIVE (no documented time limit) | — |

### 2.6 Exit codes and every error class the protocol names (V group, 60 probes)

All 60 run with `--file <id>.ls --omit-functions`. `code/cat` is the envelope `code`/`category` when `ok:false`,
otherwise the `Diagnostic` records found anywhere in the envelope (`code/ErrorCategory.member`).

| id | script | exit | ok | code / category (or diagnostics) | notes | verdict |
|---|---|---|---|---|---|---|
| v01 | 1 + | 1 | false | InvalidOperation / DomainError (msg: Unexpected token at position 3) | diagnostics carry position 3, line 1, column 4 | FINDING F3 |
| v02 | x = symbol("x" | 1 | false | InvalidOperation / DomainError (Expected RParen ... position 14) | diagnostics carry position 14, line 1, column 15 | FINDING F3 |
| v03 | ) | 1 | false | InvalidOperation / DomainError | position 0, column 1 | FINDING F3 |
| v04 | 1 2 | 1 | false | InvalidOperation / DomainError (Expected ; or end of input) | position 2 | FINDING F3 |
| v05 | x = 1 \n y = 2 | 0 | true | - | newline separated the statements | HELD* |
| v06 | x = 1 \n y = 2 \n x + y | 0 | true | - | result Natural | HELD* |
| v07 | print("a"); print("b"); 7 | 0 | true | - | output [a,b], result 7 | HELD |
| v08 | compile_full(x^2+1,[x]); evalir(c,[1,2],5) | 1 | false | InvalidOperation / DomainError (kernel expects 1 parameters; got 2 values) |  | HELD |
| v09 | ... evalir(c,[],5) | 1 | false | InvalidOperation / DomainError (got 0 values) |  | HELD |
| v10 | ... evalir(c,[1],0) | 0 | true | - | 0 digits accepted here while setprecision(0) is refused | P3 note |
| v11 | ... evalir(c,[1],-3) | 0 | true | - | negative digits accepted by evalir | P3 note |
| v12 | ... evalir_batch(c,[[1],[2]],5) | 0 | true | - |  | HELD |
| v13 | ... lower(c,[1]) | 1 | false | InvalidOperation / DomainError (Expected a symbolic expression) |  | HELD |
| v14 | ... evalir(c,[y],5) | 1 | false | InvalidOperation / DomainError (Expected a numeric value) |  | HELD |
| v15 | evalir([[1,2]],[1],5) | 1 | false | InvalidOperation / DomainError (expect the IR from compile) |  | HELD |
| v16 | compile(x^2+1,[x]) with x undeclared | 1 | false | InvalidOperation / DomainError (Undefined variable x) |  | HELD |
| v17 | compile_full(1,[]) | 1 | false | InvalidOperation / DomainError (Expected a symbolic expression) |  | HELD |
| v18 | compile_full(x^2+1, x) | 1 | false | InvalidOperation / DomainError (argument 2 must be a list of parameter symbols) |  | HELD |
| v19 | linsolve_full([[a,2],[2,4]],[[1],[2]]) | 0 | true | - | MatrixSolveResult Solved/Complete, conditions [4*a - 4 != 0] | HELD |
| v20 | linsolve([[a,2],[2,4]],[[1],[2]]) | 0 | true | - | Vector result | HELD |
| v21 | solve_system_full([x==1, x==2],[x]) | 0 | true | - | SolveStatus.NoSolutions, Completeness.Complete | HELD (correct) |
| v22 | solve_system_full([],[x]) | 0 | true | system-solve.unevaluated/ErrorCategory.UnsupportedOperation | status Unevaluated | FINDING F7 |
| v23 | solve_system_full([x==1],[]) | 0 | true | system-solve.unevaluated/ErrorCategory.UnsupportedOperation | status Unevaluated | FINDING F7 |
| v24 | det([[1,2,3],[4,5,6]]) | 1 | false | InvalidArgument / TypeMismatch (requires a square matrix) |  | HELD |
| v25 | transpose(A,[0,0]) | 1 | false | InvalidArgument / TypeMismatch (valid reordering of the axes) |  | HELD |
| v26 | transpose(A,[1,2]) | 1 | false | InvalidArgument / TypeMismatch |  | HELD |
| v27 | concat([1,2],[3,4],5) | 1 | false | InvalidArgument / TypeMismatch (Axis 5 is out of range for rank 1) |  | HELD |
| v28 | dot(A,A) | 1 | false | InvalidArgument / TypeMismatch (operands must be rank-1 vectors) |  | HELD |
| v29 | zeros([2,-1]) | 1 | false | InvalidOperation / DomainError (Index must be Natural or Integer, but got Vector) | message names the vector, not the -1 element | P3 note |
| v30 | ones(1.5) | 1 | false | InvalidOperation / DomainError (.... but got Real) |  | P3 note |
| v31 | eye(2,3) | 0 | true | - |  | HELD |
| v32 | assume(x>0); assume(x<0); solve(x^2==4,x) | 1 | false | UnsatisfiableAssumptions / DomainError |  | HELD |
| v33 | assume(1 > 2) | 1 | false | InvalidOperation / DomainError (assume() accepts relations ...) |  | HELD |
| v34 | assume_clear(); assumptions() | 0 | true | - | Text/Symbolic | HELD |
| v35 | print(x); 1 | 0 | true | - | output [x], result 1 | HELD |
| v36 | a = ones(100000); print(len(a)) | 0 | true | - | Vector | HELD |
| v37 | plot([x]) | 1 | false | InvalidOperation / DomainError | = advertised plot.symbolic-element class | HELD |
| v38 | type(1) | 0 | true | - | Text | HELD |
| v39 | 1 == 1 | 0 | true | - | Boolean | HELD |
| v40 | series(sin(x),x,0,0) | 1 | false | InvalidArgument / TypeMismatch (BCL range message, Parameter order) |  | P3 note |
| v41 | limit_full(sin(x)/x,x,0) | 0 | true | - | LimitStatus.Value, SolutionExactness.Exact | HELD |
| v42 | integrate_full(1/x,x) | 0 | true | - | IntegrationStatus.SolvedConditional, SolutionExactness.Approximate | HELD |
| v43 | setprecision(1000000000000); pi(5) | 1 | false | InvalidArgument / TypeMismatch; message length (-727379968) must be a non-negative value. (Parameter length) | int32 wraparound | FINDING F13 |
| v44 | evalf(pi,1000000) | 0 | true | - | Real, 100 digits (see E group) | FINDING F5 |
| v45 | 2^0.5 | 1 | false | UnsupportedOperation / UnsupportedOperation (Non-integer exponents ...) | = advertised pow.non-integer-exponent | HELD |
| v46 | 0.1 + 0.2 | 0 | true | - | Real value 0.3, exact true, numerator 3 denominator 10 | HELD (exact decimal arithmetic) |
| v47 | 1/3 | 0 | true | - | Real value 0.(3), exact true, numerator 1 denominator 3 | HELD |
| v48 | print(1/3) | 0 | true | - | void statement | HELD* |
| v49 | x = symbol("x"); x = 5 | 0 | true | - | rebinding allowed | HELD |
| v50 | pi = 5 | 0 | true | - | a builtin name is silently shadowed by an assignment | P3 note |
| v51 | zzz | 1 | false | InvalidOperation / DomainError (Undefined variable zzz) |  | HELD |
| v52 | symbol("x", real); solve(x^2-2==0,x,real) | 0 | true | - | Vector | HELD |
| v53 | solve_full(exp(x)==0,x) | 0 | true | solve.no-solutions/ErrorCategory.NoSolution | NoSolutions/Complete (correct) | HELD |
| v54 | solve_full(abs(x)==-1,x) | 0 | true | solve.unevaluated/ErrorCategory.UnsupportedOperation | Unevaluated | FINDING F7 |
| v55 | solve_full(x^3-2==0,x) | 0 | true | - | Solved/Complete/AlgebraicExact | HELD |
| v56 | solve_full(x^2==2,x,complex) | 0 | true | - | Solved/Complete/AlgebraicExact | HELD |
| v57 | complex() | 0 | true | - | Domain | HELD |
| v58 | real() | 0 | true | - | Domain | HELD |
| v59 | integer() | 0 | true | - | Domain | HELD |
| v60 | rational() | 0 | true | - | Domain | HELD |

\* HELD* marks rows whose *shape* held but whose mathematical value was not captured; they are not counted as
findings and are re-listed in §4 where the missing value matters.

Second, clean reproduction of the parse-error classification (fresh file `d1.ls` = `x = symbol("x"`, minimal command):

```
$e=out\aot\Lovelace.Run.exe; & $e --omit-functions --file d1.ls
exit=1   {"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
"code":"InvalidOperation","category":"DomainError","message":"Expected 'RParen' but found '' at position 14.",
"recoverable":true,"diagnostics":[{"message":"Expected 'RParen' but found '' at position 14.","position":14,
"line":1,"column":15}],"elapsed":"240.6 µs", ...}
```

Second, clean reproduction of F12 (extra positional argument, two runs each):

```
& $e --omit-functions --file ok.ls c3.ls   -> A_exit=2   stdout 0 bytes
& $e --omit-functions --eval "1+1" c3.ls   -> B_exit=0   stdout 990 bytes (JSON envelope, result 2)
```

Second, clean reproduction of F13 (`d2.ls` = `x = symbol("x"); setprecision(1000000000000); pi(5)`):

```
& $e --omit-functions --file d2.ls
exit=1   {"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"length ('-727379968') must be a
non-negative value. (Parameter 'length')\r\nActual value was -727379968.","recoverable":true,"diagnostics":[],...}
```

### 2.7 `capabilities()`: every advertised entry driven by its own trigger (T group, 16 probes)

Advertised metadata (16 entries, all fields taken from the structured record, not from `display`):

| # | advertised operation_class | advertised code | advertised category | advertised trigger |
|---|---|---|---|---|
| T0 | pow.non-integer-exponent | UnsupportedOperation | UnsupportedOperation | 2^(1/2) |
| T1 | pow.negative-base-unrepresentable-exponent | UnsupportedOperation | UnsupportedOperation | (-8)^(1/3) |
| T2 | solve.unsupported-domain | InvalidOperation | DomainError | x = symbol("x"); solve(x^2 - 2 == 0, x, integer) |
| T3 | rootof.complex-algebraic | solve.unrepresented-roots | UnsupportedOperation | x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x) |
| T4 | limit.unevaluated-in-record-diagnostics | limit.unevaluated | UnsupportedOperation | x = symbol("x"); limit_full(sin(x), x, inf) |
| T5 | integration.unevaluated-in-record-diagnostics | integration.unevaluated | UnsupportedOperation | x = symbol("x"); integrate_full(exp(x^2), x) |
| T6 | plot.symbolic-expression | InvalidOperation | DomainError | x = symbol("x"); plot(sin(x)) |
| T7 | plot.symbolic-element | InvalidOperation | DomainError | x = symbol("x"); plot([x, 1, 2]) |
| T8 | solve.non-symbolic-variable | InvalidOperation | DomainError | x = symbol("x"); solve(x^2 - 2 == 0, 1) |
| T9 | solve.non-symbolic-expression | InvalidOperation | DomainError | x = symbol("x"); solve([x == 1], x) |
| T10 | diff.non-symbolic-variable | InvalidOperation | DomainError | x = symbol("x"); diff(x^2, 1) |
| T11 | integrate.non-symbolic-variable | InvalidOperation | DomainError | x = symbol("x"); integrate(x^2, 1) |
| T12 | limit.non-symbolic-variable | InvalidOperation | DomainError | x = symbol("x"); limit(sin(x), 1, 0) |
| T13 | dsp.symbolic-element | InvalidOperation | DomainError | x = symbol("x"); dft([x, 1, 2, 3]) |
| T14 | linsolve.non-symbolic-matrix | InvalidOperation | DomainError | A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b) |
| T15 | fft.non-power-of-two-length | InvalidArgument | TypeMismatch | x = symbol("x"); fft([1,2,3]) |

Live result of executing each advertised trigger verbatim (`--file <Tn>.ls --omit-functions`):

| id | exit | ok | live code | live category | live evidence | advertised vs live | verdict |
|---|---|---|---|---|---|---|---|
| T0 | 1 | false | UnsupportedOperation | UnsupportedOperation | msg Non-integer exponents are not yet supported. | identical | HELD |
| T1 | 1 | false | UnsupportedOperation | UnsupportedOperation | msg Non-integer exponents are not yet supported. | identical | HELD |
| T2 | 1 | false | InvalidOperation | DomainError | solve(): currently supports domains real and complex; got integer. | identical | HELD |
| T3 | 0 | true | solve.unrepresented-roots (Diagnostic) | UnsupportedOperation | SolveStatus.Partial, Completeness.Partial, SolutionExactness.AlgebraicExact | identical | HELD |
| T4 | 0 | true | limit.unevaluated (Diagnostic) | UnsupportedOperation | LimitStatus.Unevaluated, SolutionExactness.Exact | identical | HELD |
| T5 | 0 | true | integration.unevaluated + **integration.no-closed-form** | UnsupportedOperation | IntegrationStatus.Unevaluated, SolutionExactness.**Approximate** | advertised code present; a second diagnostic code is not advertised | HELD for the advertised code / FINDING F7 for the extra |
| T6 | 1 | false | InvalidOperation | DomainError | plot() argument 1 must be a vector, but got Symbolic. | identical | HELD |
| T7 | 1 | false | InvalidOperation | DomainError | Cannot convert value of kind Symbolic to a number for plotting. | identical | HELD |
| T8 | 1 | false | InvalidOperation | DomainError | solve(): argument 2 must be a symbolic variable; got Natural. | identical | HELD |
| T9 | 1 | false | InvalidOperation | DomainError | solve(): argument 1 must be a symbolic expression; got Vector. | identical | HELD |
| T10 | 1 | false | InvalidOperation | DomainError | diff(): argument 2 must be a symbolic variable; got Natural. | identical | HELD |
| T11 | 1 | false | InvalidOperation | DomainError | integrate(): argument 2 must be a symbolic variable; got Natural. | identical | HELD |
| T12 | 1 | false | InvalidOperation | DomainError | limit(): argument 2 must be a symbolic variable; got Natural. | identical | HELD |
| T13 | 1 | false | InvalidOperation | DomainError | DSP builtins expect numeric/complex elements, but got SymbolExpr. | identical | HELD |
| T14 | 1 | false | InvalidOperation | DomainError | linsolve() requires a symbolic matrix A. | identical | HELD |
| T15 | 1 | false | InvalidArgument | TypeMismatch | FFT length must be a power of two, but got 3. (Parameter x) | identical | HELD |

**16/16 advertised code+category pairs reproduce exactly.** The advertisements are honest *for their own
triggers*; what fails is the generalisation of the class (F1) and the completeness of the list (F7).

### 2.8 `capabilities()`: hunting refusals the statement does not list (U group, 90 probes)

A refusal = an envelope with `ok:false`, or a result record whose diagnostics/status say
unevaluated/unsupported/partial/budget. Every row below ran as `--file u<nn>.ls --omit-functions`.

| id | script | exit | ok | live code / category (or status) | listed in capabilities()? | verdict |
|---|---|---|---|---|---|---|
| u01 | symbol("x", integer) | 0 | true | Symbolic (domain complex, see F14) | integer is listed as an **unsupported** domain | FINDING F14 |
| u02 | symbol("x", rational) | 0 | true | Symbolic | rational is listed as an unsupported domain | FINDING F14 |
| u03 | symbol("x", natural) | 1 | false | InvalidOperation / DomainError (Undefined variable natural) | n/a | HELD |
| u04 | symbol("x", bogus) | 1 | false | InvalidOperation / DomainError (Undefined variable bogus) | n/a | HELD |
| u05 | solve(x^2-2==0, x, rational) | 1 | false | InvalidOperation / DomainError (got rational) | = advertised solve.unsupported-domain | HELD |
| u06 | solve(x^2-2==0, x, natural) | 1 | false | InvalidOperation / DomainError (Undefined variable natural) | n/a | HELD |
| u07 | solve_full(x^5-x+1==0, x) | 0 | true | solve.unrepresented-roots; Partial/Partial/AlgebraicExact | yes (T3) | HELD |
| u08 | solve_full(sin(x)==0.5, x) | 0 | true | Solved/Complete/ParametricExact | - | HELD |
| u09 | solve_full(exp(x)==x, x) | 0 | true | **Solved/Complete/AlgebraicExact, solution log(x), represented_count 1** | - | FINDING F0 (P0) |
| u10 | solve_full(x==x, x) | 0 | true | solve.unevaluated / UnsupportedOperation; Unevaluated/Unknown | **no** | FINDING F7 |
| u11 | solve_full(1/x==0, x) | 0 | true | solve.no-solutions / NoSolution; NoSolutions/Complete | - | HELD (correct) |
| u12 | solve_system_full([x+y==1],[x,y]) | 0 | true | system-solve.unevaluated / UnsupportedOperation | **no** | FINDING F7 |
| u13 | solve_system_full([x^2+y^2==1, x+y==3],[x,y]) | 0 | true | Solved/Complete/AlgebraicExact (complex roots) | - | HELD |
| u14 | solve_system_full([sin(x)==y, x*y==1],[x,y]) | 0 | true | system-solve.unevaluated / UnsupportedOperation | **no** | FINDING F7 |
| u15 | linsolve([[1,2],[2,4]],[[1],[2]]) | 1 | false | InvalidOperation / DomainError (requires a symbolic matrix A) | yes (T14) | HELD |
| u16 | linsolve_full([[1,2],[2,4]],[[1],[3]]) | 1 | false | InvalidOperation / DomainError (requires a symbolic matrix A) | yes (T14 class) | HELD |
| u17 | linsolve_full([[1,2],[3,4]],[[1],[2]]) | 1 | false | InvalidOperation / DomainError | yes (T14 class) | HELD |
| u18 | integrate_full(sin(x)/x, x) | 0 | true | integration.unevaluated + integration.no-closed-form; Unevaluated, **Approximate** | partly (unevaluated listed) | FINDING F7 |
| u19 | integrate_full(exp(-x^2), x) | 0 | true | same two diagnostics; Unevaluated, Approximate | partly | FINDING F7 |
| u20 | integrate_full(1/log(x), x) | 0 | true | same; Unevaluated, Approximate | partly | FINDING F7 |
| u21 | integrate_full(x^x, x) | 0 | true | same; Unevaluated, Approximate | partly | FINDING F7 |
| u22 | integrate_full(1/(x^5+1), x) | 0 | true | same two diagnostics; Unevaluated, **Exact** | partly | FINDING F7 |
| u23 | limit_full(1/x, x, 0) | 0 | true | LimitStatus.DoesNotExist, Exact | - | HELD |
| u24 | limit_full(abs(x)/x, x, 0) | 0 | true | limit.unevaluated / UnsupportedOperation | yes (T4 class) | HELD |
| u25 | limit_full(x*sin(1/x), x, 0) | 0 | true | limit.unevaluated (true limit is 0; conservative refusal) | yes (T4 class) | HELD |
| u26 | limit_full((-1)^x, x, inf) | 0 | true | limit.unevaluated | yes (T4 class) | HELD |
| u27 | limit_left(abs(x)/x, x, 0) | 0 | true | Text/Symbolic result | - | HELD |
| u28 | series(1/x, x, 0, 3) | 0 | true | Symbolic | - | HELD |
| u29 | series(sin(x), x, 0, 500) | 0 | true | Symbolic, fast | - | HELD |
| u30 | series(exp(x), x, 0, -1) | 1 | false | InvalidArgument / TypeMismatch (BCL range message, Parameter order) | - | P3 note |
| u31 | diff(abs(x), x) | 0 | true | Symbolic | - | HELD |
| u32 | diff(x^x, x) | 0 | true | Symbolic | - | HELD |
| u33 | simplify(sin(x)^2+cos(x)^2) | 0 | true | Symbolic | - | HELD |
| u34 | factor(x^2+1) | 0 | true | Symbolic | - | HELD |
| u35 | apart(1/(x^2+1), x) | 0 | true | Symbolic | - | HELD |
| u36 | expand((x+1)^50) | 0 | true | Symbolic | - | HELD |
| u37 | solve_full(x^2-2==0, x, real) | 0 | true | Solved/Complete/AlgebraicExact | - | HELD |
| u38 | optimize_full(x^2-2*x, [x]) | 0 | true | Record | - | HELD |
| u39 | compile_full(x^2+1, [x]) | 0 | true | Record | - | HELD |
| u40 | compile_full(sin(x)^2+cos(x)^2, [x]) | 0 | true | Record | - | HELD |
| u41 | compile_full then evalir(c,[3],20) | 0 | true | Integer result | - | HELD |
| u42 | evalir(42,[1],5) | 1 | false | InvalidOperation / DomainError | - | HELD |
| u43 | evalir_batch(42,[[1]],5) | 1 | false | InvalidOperation / DomainError | - | HELD |
| u44 | lower(42,[1]) | 1 | false | InvalidOperation / DomainError (Expected a symbolic expression) | - | HELD |
| u45 | setprecision(0); pi(5) | 1 | false | InvalidOperation / DomainError (expects a positive digit count, but got 0) | - | HELD |
| u46 | setprecision(-5); pi(5) | 1 | false | InvalidOperation / DomainError (but got -5) | - | HELD |
| u47 | setprecision(1000000000); pi(5) | 0 | true | Real | - | HELD |
| u48 | pi(0) | 1 | false | InvalidArgument / TypeMismatch (BCL range message, Parameter digits) | - | P3 note |
| u49 | pi(-1) | 1 | false | InvalidArgument / TypeMismatch | - | P3 note |
| u50 | e(0) | 1 | false | InvalidArgument / TypeMismatch | - | P3 note |
| u51 | evalf(pi, 0) | 0 | true | Real, 100 digits, exact **true** | - | FINDING F5/F11 |
| u52 | evalf(sin(x), 10) | 1 | false | EvaluationError / DomainError (No value for symbol x) | **no** (code not advertised) | P2 note |
| u53 | 1/0 | 1 | false | DivisionByZero / DomainError | - | HELD |
| u54 | 0^0 | 0 | true | Natural (value not captured) | - | INCONCLUSIVE |
| u55 | sqrt(-1) | 0 | true | Symbolic (value not captured) | - | INCONCLUSIVE |
| u56 | log(0) | 0 | true | Symbolic (value not captured) | - | INCONCLUSIVE |
| u57 | log(-1) | 0 | true | Symbolic (value not captured) | - | INCONCLUSIVE |
| u58 | frobnicate(1) | 1 | false | InvalidOperation / DomainError (Unknown function) | - | HELD |
| u59 | ones(0) | 0 | true | Vector | - | HELD |
| u60 | zeros(-1) | 1 | false | ArithmeticError / DomainError (Arithmetic operation resulted in an overflow) | - | P3 note |
| u61 | eye(0,0) | 1 | false | InvalidArgument / TypeMismatch (dimensions must be positive) | - | HELD |
| u62 | reshape([1,2,3],[2,2]) | 1 | false | InvalidOperation / DomainError (Index must be Natural or Integer, but got Vector) | - | P3 note |
| u63 | max([]) | 1 | false | InvalidOperation / DomainError (cannot reduce an empty array) | - | HELD |
| u64 | sum([]) | 0 | true | Natural 0 | - | HELD |
| u65 | det([[1,2],[2,4]]) | 0 | true | Natural 0 (correct) | - | HELD |
| u66 | inv_full([[1,2],[2,4]]) | 1 | false | InvalidOperation / DomainError (requires a symbolic matrix) | - | P2 note |
| u67 | inv([[1,2],[2,4]]) | 1 | false | InvalidOperation / DomainError (Matrix is singular) | - | HELD |
| u68 | matrix_rank([[1,2],[2,4]]) | 1 | false | InvalidOperation / DomainError (requires a symbolic matrix) | - | P2 note |
| u69 | dot([1,2],[1,2,3]) | 1 | false | InvalidArgument / TypeMismatch (same length 2 vs 3) | - | HELD |
| u70 | matmul([[1,2]],[[1,2]]) | 1 | false | InvalidArgument / TypeMismatch (inner dimensions must match) | - | HELD |
| u71 | print() | 0 | true | no result key; output [""] | - | FINDING F10 |
| u72 | print(1) | 0 | true | no result key; output ["1"] | - | FINDING F10 |
| u73 | compile_full(x^2) | 1 | false | InvalidArgument / TypeMismatch (expected 2 arguments; got 1) | - | HELD (matches the documented example exactly) |
| u74 | plot([1,2,3]) | 0 | true | Text absolute path + top-level plot object; writes plot.svg into the process CWD | - | P3 note |
| u75 | noise(1,1,4,0) | 0 | true | Vector | - | HELD |
| u76 | fft([1,2,3,4]) | 0 | true | Vector/Complex | - | HELD |
| u77 | dft([1,2,3,4]) | 0 | true | Vector/Complex | - | HELD |
| u78 | subs(x^2, x, 3) | 0 | true | Symbolic | - | HELD |
| u79 | subs(x^2, x, [1,2]) | 1 | false | InvalidOperation / DomainError (argument 3 must be a symbolic expression) | - | HELD |
| u80 | hessian(x^3+y^3, [x]) | 0 | true | Array/Symbolic | - | HELD |
| u81 | assume(x>5); solve(x^2==4, x) | 0 | true | Vector (value not captured) | - | INCONCLUSIVE |
| u82 | assume(x>0); solve_full(x^2==-4, x) | 0 | true | Solved/Complete/AlgebraicExact (complex domain) | - | HELD |
| u83 | assume(x>0); solve(x^2==4, x) | 0 | true | Vector (value not captured) | - | INCONCLUSIVE |
| u84 | integrate_full(1/(x^100-1), x) | - | - | **no envelope within 60 s** | - | INCONCLUSIVE (see X18) |
| u85 | simplify((x^100-1)/(x-1)) | 0 | true | Symbolic | - | HELD |
| u86 | apart(1/(x^20-1), x) | 0 | true | Symbolic | - | HELD |
| u87 | series(tan(x), x, 0, 200) | - | - | **no envelope within 25 s** | - | INCONCLUSIVE |
| u88 | factor(x^20-1) | 0 | true | Symbolic | - | HELD |
| u89 | 9999999999999999999999999999999999999999^2 | 0 | true | Natural (exact) | - | HELD |
| u90 | 10^100000 | 0 | true | Natural (exact, 100001 digits printable) | - | HELD |

**Unlisted refusal classes found (category `UnsupportedOperation`, absent from `unsupported_operations`):**
`solve.unevaluated` (u10, v54), `system-solve.unevaluated` (u12, u14, v22, v23),
`integration.no-closed-form` (u18–u22, and T5 emits it next to the advertised `integration.unevaluated`).
The protocol doc itself names `transform.budget-exceeded` and `transform.unsatisfiable-conditions` as diagnostic
codes; neither was reachable in this pass (`UnsatisfiableAssumptions` appears only as an *envelope* code, v32).

### 2.9 `capabilities()`: statement against binary (D group, 7 probes)

| id | probe | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| D01 | `2^(1/2)` (advertised trigger of pow.non-integer-exponent) | `--eval 2^(1/2)` | exit 1; `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported."}` | HELD | — |
| D02 | `(-4)^(1/2)` (same advertised class) | `--eval (-4)^(1/2)` | exit 0; `{"kind":"Symbolic","pretty":"2*i","canonical":"(mul (rat 2 1) (i))","domain":"complex","exact":true,"nodeCount":3}` | FINDING F1 | P1 |
| D03 | `(-1)^(1/2)` (same advertised class) | `--eval (-1)^(1/2)` | exit 0; `{"kind":"Symbolic","pretty":"i","canonical":"(i)","domain":"complex","exact":true,"nodeCount":1}` | FINDING F1 | P1 |
| D04 | `(-8)^(1/3)` (advertised trigger of T1) | `--eval (-8)^(1/3)` | exit 1; UnsupportedOperation (unchanged) | HELD | — |
| D05 | `(-8)^(1/2)`, `(-9)^(1/2)`, `(-12)^(1/2)`, `(-0.5)^(1/2)` (file form, 4 probes) | `--file p01/p02/p04/p07.ls` | exit 0; `i*sqrt(8)` (exact false), `3*i` (exact true), `i*sqrt(12)` (exact false), `i*sqrt(1/2)` (exact false) | FINDING F1 | P1 |
| D06 | `4^(1/2)`, `9^(1/2)` | `--eval 4^(1/2)` | exit 1; UnsupportedOperation — the *positive* perfect square is refused while the negative one works | FINDING F1 | P1 |
| D07 | `unsupported_domains: [integer, rational]` vs `symbol(name, domain)` | `--file u01.ls`, `u02.ls` | exit 0; Symbolic for both, while `solve(..., x, integer/rational)` is refused | FINDING F14 | P2 |

Second, clean reproduction of F1 (fresh temp files, file form): `p01.ls` = `(-8)^(1/2)` and `p05.ls` = `(-4)^(1/2)`
and `p02.ls` = `(-9)^(1/2)`, all `exit=0 ok=True`; and `--eval 2^(1/2)` / `--eval 4^(1/2)` / `--eval 9^(1/2)` all
`exit=1 UnsupportedOperation` in the same session. The negative-base cases were also re-run three times each through
`--eval` with `exit=0` every time.

### 2.10 Precision and exactness (E group, 9 probes)

| id | probe | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| E01 | `evalf(pi, 0)` | `--file e0.ls` | exit 0; Real; `value` 102 chars `3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170679`; `exact:true`; `numerator` 1001 digits, `denominator` 1001 digits | FINDING F5/F11 | P1 |
| E02 | `evalf(pi, 5)` | `--file e5.ls` | exit 0; Real; **same 102-char value**; `exact:false`; no `numerator` | FINDING F5 | P1 |
| E03 | `evalf(pi, 20)` | `--file e20.ls` | exit 0; same 102-char value; `exact:false` | FINDING F5 | P1 |
| E04 | `evalf(pi, 50)` | `--file e50.ls` | exit 0; same 102-char value | FINDING F5 | P1 |
| E05 | `evalf(pi, 200)` | `--file e200.ls` | exit 0; same 102-char value (asked for 200 digits) | FINDING F5 | P1 |
| E06 | `evalf(pi, 1000000)` | `--file ebig.ls` | exit 0; same 102-char value (asked for a million digits) | FINDING F5 | P1 |
| E07 | `setprecision(30); evalf(pi, 0)` | `--file sp30.ls` | exit 0; value 32 chars (30 digits), `exact:true`, `numerator` 31 digits | HELD (session precision is honoured) |
| E08 | `setprecision(30); evalf(pi, 5)` | `--file sp30b.ls` | exit 0; value 32 chars, `exact:false` | HELD |
| E09 | `evalf(1/3, 5)` (control: rational input) | `--file third.ls` | exit 0; Real `0.33333` (5 digits) — the `digits` argument **is** honoured here | FINDING F5 (asymmetry) |

Ground truth for the value itself (mpmath, 101 significant digits):
`3.1415926535897932384626433832795028841971693993751058209749445923078164062862089986280348253421170680` —
the tool returns the **truncated** 100-digit form (`...1170679`), i.e. a rational approximation, not pi; the
`exact:true` / `exact:false` split between E01 and E02–E06 is decided by the call form, not by the value.

### 2.11 Timings and elapsed structure (G group, 1 probe)

| id | probe | command / flags | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| G01 | `print(1); print(2); 3` | `--eval ... --omit-functions` | `"timings":[{"position":0,...,"resultKind":"Void","hasOutput":true},{"position":10,...,"resultKind":"Void","hasOutput":true},{"position":20,...,"resultKind":"Natural","hasOutput":false}]`; `"elapsed":"77.9 µs","elapsedTime":{"value":77.9,"unit":"µs"}` | HELD (positions 0/10/20 are the exact source offsets; unit selector matches the string) | — |

### 2.12 Mathematics spot-checks against SymPy 1.14.0 / mpmath (M group, 4 probes)

| id | probe | ground truth | tool answer | verdict | sev |
|---|---|---|---|---|---|
| M01 | `x = symbol("x"); solve_full(exp(x) == x, x)` | `solveset(exp(x)-x, x, S.Reals)` = ConditionSet; `solveset(..., S.Complexes)` = ConditionSet; `solve(exp(x)-x, x)` = `[-LambertW(-1)]`; e^x - x > 0 at x = -0.5, 0, 1 (1.1065, 1.0, 1.7183) | `Solved`, `complete: True`, `completeness: Complete`, `represented_count: 1`, solution value `log(x)` with the vacuous condition `1 != 0` | FINDING F0 | P0 |
| M02 | `evalf(pi, N)` | mpmath: the first 101 significant digits of pi end `...21170680` | `...21170679` (truncation), reported `exact:true` for N = 0 and `exact:false` for N > 0 | FINDING F11 | P2 |
| M03 | `0.1 + 0.2`, `1/3` | exact decimal arithmetic: 0.3; the exact rational 1/3 | Real `0.3` (numerator 3 / denominator 10, exact); Real `0.(3)` (numerator 1 / denominator 3, exact) | HELD | — |
| M04 | `(-4)^(1/2)`, `(-9)^(1/2)` | 2i and 3i exactly | `2*i` (exact true) and `3*i` (exact true) | HELD as a value; the *capability statement* is contradicted (F1) | P1 |

---

## 3. FINDINGS

Each finding is one sentence, then the exact reproduction and the second, clean run.

### F0 — P0 — `solve_full` reports a circular expression as a complete solution set

**One sentence.** For `exp(x) == x` the solver returns `status: Solved`, `complete: true`, `completeness: Complete`,
`represented_count: 1`, `unrepresented_count: 0` with the single "solution" `log(x)` — an expression containing the
unknown that satisfies the equation identically — although the true solution set has no closed form and is infinite
(SymPy: `solve(exp(x)-x, x)` = `[-LambertW(-1)]`, `solveset` over both Reals and Complexes = ConditionSet).

Reproduction (run 1 and run 2 are two identical invocations of two identically-written files `z1.ls` / `z1b.ls`):

```
$e=out\aot\Lovelace.Run.exe;  # z1.ls = x = symbol("x"); solve_full(exp(x) == x, x)
& $e --omit-functions --file z1.ls
exit=0
SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
  solutions: [Solution(value: log(x), conditions: [1 != 0], multiplicity: 1, exactness: AlgebraicExact)],
  families: [], common_conditions: [1 != 0], represented_count: 1, unrepresented_count: 0,
  unrepresented_reason: , diagnostics: [])
& $e --omit-functions --file z1b.ls   # identical output, byte for byte
```

Neighbourhood (same session, fresh files): `solve_full(log(x) == x, x)` -> solution `exp(x)`;
`solve(exp(x) == x, x)` -> the vector `[log(x)]`; while `solve_full(sin(x) == x, x)`, `solve_full(cos(x) == x, x)`,
`solve_full(exp(x) == x + 1, x)` and `solve_full(2^x == x, x)` all answer `Unevaluated` (honest).
The legitimate sibling `solve_full(exp(x) == 2, x)` correctly answers `log(2)`.

### F1 — P1 — `capabilities()` advertises a refusal that the binary does not perform

**One sentence.** The advertised class `pow.non-integer-exponent` carries the message "Non-integer exponents are
not yet supported.", yet `(-4)^(1/2)` returns `2*i`, `(-1)^(1/2)` returns `i`, and `(-9)^(1/2)` returns `3*i` with
`ok: true` and `exact: true` (while `4^(1/2)` and `9^(1/2)` are refused).

```
& $e --omit-functions --eval "2^(1/2)"
exit=1  {"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation",
         "message":"Non-integer exponents are not yet supported."}
& $e --omit-functions --eval "(-4)^(1/2)"
exit=0  {"ok":true,...,"result":{"kind":"Symbolic","display":"2*i","structured":
         {"kind":"Symbolic","pretty":"2*i","canonical":"(mul (rat 2 1) (i))","domain":"complex","exact":true,"nodeCount":3}}}
& $e --omit-functions --eval "(-1)^(1/2)"
exit=0  {"ok":true,...,"pretty":"i","canonical":"(i)","exact":true}
& $e --omit-functions --eval "4^(1/2)"   -> exit=1 UnsupportedOperation   (positive perfect square refused)
& $e --omit-functions --eval "9^(1/2)"   -> exit=1 UnsupportedOperation
```

Second run: `p05.ls`, `p02.ls`, `p01.ls`, `p04.ls`, `p07.ls` (file form) returned `exit=0`, `2*i` / `3*i` /
`i*sqrt(8)` / `i*sqrt(12)` / `i*sqrt(1/2)`; `--eval` repeats of `(-8)^(1/2)`, `(-2)^(1/2)`, `(-0.5)^(1/2)` ran
three times each with `exit=0` every time.

### F2 — P1 — `--cancel-after <ms>` is not a deadline

**One sentence.** `--cancel-after 200` on a single long `integrate_full` call returned the `Cancelled` envelope
after 46.27 s of wall clock (230x the budget), and the two other budgets overshot the same way, so the flag does
not bound an evaluation.

```
# slow.ls = x = symbol("x"); integrate_full(1/(x^100-1), x)
& $e --omit-functions --cancel-after 200 --file slow.ls
exit=1   {"ok":false,"code":"Cancelled","category":"BudgetExceeded",
          "message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[],
          "elapsed":"46.27 s","elapsedTime":{"value":46.27,"unit":"s"},
          "timings":[{"position":0,...,"resultKind":"Symbolic"},{"position":17,...,"resultKind":"Void"}],
          "partialOutput":[],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x"}]}
& $e --omit-functions --cancel-after 1    --file slow.ls   -> exit 1, "elapsed":"41.74 s"
& $e --omit-functions --cancel-after 2000 --file slow.ls   -> exit 1, "elapsed":"35.78 s"
```

Second and third runs are the 1 ms and 2000 ms rows above (the elapsed values differ by seconds, never by
milliseconds).

### F3 — P1 — `ParseError` is attached to the wrong layer in both directions

**One sentence.** Genuine syntax errors carry `category: DomainError` with the generic code `InvalidOperation` (the
parser diagnostics are present, proving the layer), while a *missing or unreadable script file* carries
`category: ParseError`, so an agent that branches on `category` sees the opposite of the truth.

```
& $e --omit-functions --file v01.ls     # v01.ls = 1 +
exit=1  {"ok":false,"code":"InvalidOperation","category":"DomainError",
         "message":"Unexpected token '' at position 3: ...",
         "diagnostics":[{"message":"...","position":3,"line":1,"column":4}]}
& $e --omit-functions --file d1.ls      # d1.ls = x = symbol("x"   (second, clean run)
exit=1  {"ok":false,"code":"InvalidOperation","category":"DomainError",
         "message":"Expected 'RParen' but found '' at position 14.",
         "diagnostics":[{"message":"...","position":14,"line":1,"column":15}]}
& $e --omit-functions --file nope.ls    # missing file
exit=1  {"ok":false,"code":"FileReadError","category":"ParseError",
         "message":"Cannot read script file '...nope.ls': Could not find file '...nope.ls'."}
```

Reproduced on four different syntax errors (v01 `1 +`, v02 unterminated call, v03 `)`, v04 `1 2`) and twice on
`d1.ls`; the file case was reproduced with a missing path, a directory and an empty path (a23, a24, a25).

### F4 — P1 — print output is dropped whenever the script fails

**One sentence.** `print("hello"); 1 +` exits 1 with an envelope that has no `output` key at all, so everything the
script printed is lost, contradicting invariant 1 ("Anything the script prints with print(...) is captured and
returned in the top-level output array").

```
& $e --omit-functions --file pr1.ls     # pr1.ls = print("hello"); 1 +   (run twice)
exit=1  keys=protocolVersion,symbolicFormatVersion,mathIrVersion,ok,code,category,message,
             recoverable,diagnostics,elapsed,elapsedTime,timings   output=<ABSENT>
& $e --omit-functions --file pr2.ls     # pr2.ls = print("hello"); 1/0
exit=1  keys=...same list...                                       output=<ABSENT>
```

### F5 — P1 — `evalf(f, digits)` ignores `digits` for symbolic constants

**One sentence.** `evalf(pi, N)` returns the same 100-digit value for N = 0, 5, 20, 50, 200 and 1000000 (only
`setprecision` changes it), while `evalf(1/3, 5)` honours the request and returns 5 digits.

```
for N in 0 5 20 50 200 1000000:  & $e --omit-functions --file e<N>.ls
  EVAL|e0|exit=0|kind=Real|valueLen=102|exact=True |numLen=1001
  EVAL|e5|exit=0|kind=Real|valueLen=102|exact=False|numLen=0
  EVAL|e20|exit=0|kind=Real|valueLen=102|exact=False|numLen=0
  EVAL|e50|exit=0|kind=Real|valueLen=102|exact=False|numLen=0
  EVAL|e200|exit=0|kind=Real|valueLen=102|exact=False|numLen=0
  EVAL|ebig|exit=0|kind=Real|valueLen=102|exact=False|numLen=0
  EVAL|sp30|exit=0|kind=Real|valueLen=32 |exact=True |numLen=31     # setprecision(30); evalf(pi,0)
  EVAL|third|exit=0|kind=Real|valueLen=7 |exact=False|numLen=0      # evalf(1/3, 5) -> 0.33333
```

Second run: the whole sweep was executed twice (once through `--eval`, once through `--file`) with identical
lengths; `evalf(pi, 1000000)` also appeared as `v44` with the same 100-digit answer.

### F6 — P1 — `--print-budget N` does not bound a rendering by N nodes

**One sentence.** A 6-node rendering is emitted complete under `--print-budget 1` while a 3-node rendering is
truncated under the same budget, because truncation is driven by the rendered length rather than by the node count
the flag, the field name `truncationReason: "node-budget"` and the help text ("abbreviate structured renderings
beyond n nodes") all name.

```
& $e --omit-functions --print-budget 1 --eval 'x = symbol("x"); x^2 + x + 1'
  "nodeCount":6, "canonical":"(add (rat 1 1) (sym x) (pow (sym x) (rat 2 1)))", no "truncated" key
& $e --omit-functions --print-budget 1 --eval 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa = symbol("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"); aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa + 1'
  "nodeCount":3, "canonical":"(add (rat 1 1) (sym  …", "truncated":true,
  "truncationReason":"node-budget", "budget":1
& $e --omit-functions --print-budget 2 ... (same 3-node value)  -> byte-identical cut
& $e --omit-functions --print-budget 1 --eval 'x^2 + x + 22'   -> full, canonical 47 chars
& $e --omit-functions --print-budget 1 --eval 'x^2 + x + 222'  -> truncated
```

Second run: the sweep was repeated for `expand((x+1)^k)`, k = 2/4/8, over budgets 1..40 (185 invocations), and the
same two boundary values (pretty length 12 -> full, 13 -> truncated) were reproduced for three different
expressions.

### F7 — P1 — `capabilities()` omits reachable refusal classes

**One sentence.** Three refusal classes whose diagnostics carry `category: UnsupportedOperation` are absent from
`unsupported_operations`: `solve.unevaluated`, `system-solve.unevaluated` and `integration.no-closed-form`
(the last one is emitted *alongside* the advertised `integration.unevaluated` for the advertised trigger itself).

```
& $e --omit-functions --file u10.ls   # x = symbol("x"); solve_full(x == x, x)
  SolveStatus.Unevaluated + Diagnostic(solve.unevaluated / ErrorCategory.UnsupportedOperation / rec=true)
& $e --omit-functions --file u12.ls   # solve_system_full([x + y == 1], [x, y])
  SolveStatus.Unevaluated + Diagnostic(system-solve.unevaluated / UnsupportedOperation)
& $e --omit-functions --file T5.ls    # the advertised integration trigger, verbatim
  Diagnostics: integration.unevaluated AND integration.no-closed-form (both UnsupportedOperation)
& $e --omit-functions --file v54.ls   # solve_full(abs(x) == -1, x)
  Diagnostic(solve.unevaluated / UnsupportedOperation)
```

Second runs: `solve.unevaluated` from u10 and v54, `system-solve.unevaluated` from u12/u14 and v22/v23,
`integration.no-closed-form` from u18–u22 and from T5 — six independent scripts across two separate corpus runs.
`docs/symbolics/a-plus-convergence-alignment-plan.md:654` calls this list **exhaustive**; under that claim the
severity is P1, under `dsh-protocol.md` alone it is P2.

### F8 — P2 — usage errors produce no envelope

**One sentence.** Every usage error (no script, unknown flag, empty/malformed flag value) exits 2 with an empty
stdout and human prose on stderr, so an agent that reads only stdout has nothing structural to branch on, although
§6 says errors are structural.

Reproduction: a01 (`exit 2`, stdout `""`, stderr `Error: No script provided...`), a04, a05, a08–a11, a14–a22, a43,
a45 — 17 rows in §2.1, all `exit 2` with a 0-byte stdout.

### F9 — P2 — `--help` and `--text` put non-envelope text on stdout

**One sentence.** `--help` exits 0 with 979 bytes of usage text on **stdout** and `--text` replaces the envelope on
stdout entirely, so invariant 1 ("stdout carries the envelope and nothing else") is only true in the default mode,
and the help/exit-code pair is asymmetric (help 0, usage error 2).

Reproduction: a02/a03 (`exit 0`, stdout 979 B usage, stderr empty) and a36 (`exit 0`, stdout `= 2 (Natural)`).
Second run: a02 and a03 are two separate invocations with byte-identical stdout (979 B each).

### F10 — P2 — a `Void` statement omits the `result` key entirely

**One sentence.** For a statement with no value the envelope has no `result` member at all (not `{"kind":"Null"}`
and not a `Void` value form, although `timings[].resultKind` says `"Void"`), so the top-level shape varies with the
script.

Reproduction: `print(1)` -> keys `protocolVersion,symbolicFormatVersion,mathIrVersion,ok,revision,output,variables,
functions,elapsed,elapsedTime,timings`; same for `print()`, `--eval ""`, the 0-byte file and the 20 000-print script.

Second run: SP01/SP02/SP03 (three further scripts) and a26/a27/a48 all reproduce the missing key.

### F11 — P2 — `exact` flips between two call forms of the same value

**One sentence.** The identical 100-digit decimal for pi is `exact: true` (with a 1001-digit numerator and
denominator) from `evalf(pi, 0)` and `exact: false` (no numerator) from `evalf(pi, 5)`/`evalf(pi, 20)`, so a
machine-visible field depends on the call form rather than on the value.

Reproduction: E01 vs E02/E03 raw cells above. Under the reading "`exact` states mathematical exactness" the
`exact:true` case is a wrong exactness claim about pi and would be P0; see §4 for why that reading is not settled.

### F12 — P2 — an extra positional argument is ignored with `--eval` but fatal with `--file`

**One sentence.** `--eval "1+1" foo.ls` succeeds and ignores `foo.ls`, while `--file ok.ls foo.ls` and a second bare
path are usage errors, so the same extra token is silently dropped or rejected depending on which source flag is
present.

Reproduction (second, clean run):
```
& $e --omit-functions --file ok.ls c3.ls   -> exit 2, stdout 0 bytes,  Error: Unknown argument (c3.ls)
& $e --omit-functions --eval "1+1" c3.ls   -> exit 0, stdout 990 bytes JSON envelope, detail field "result" present
```

### F13 — P2 — `setprecision` silently wraps a large digit count to int32

**One sentence.** `setprecision(1000000000000)` reaches the string constructor as `-727379968` and surfaces the BCL
message `length ('-727379968') must be a non-negative value. (Parameter 'length')` under `code: InvalidArgument`,
`category: TypeMismatch`, exposing both an unchecked conversion and framework text.

Reproduction (second, clean run, fresh file `d2.ls`):
```
& $e --omit-functions --file d2.ls
exit=1  {"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"length ('-727379968') must be a
         non-negative value. (Parameter 'length')\r\nActual value was -727379968.","recoverable":true,
         "diagnostics":[],"elapsed":"359.4 µs",...}
```

### F14 — P2 — `unsupported_domains` says `integer`/`rational` while `symbol()` accepts them

**One sentence.** `capabilities()` advertises `unsupported_domains: [integer, rational]`, but `symbol("x", integer)`
and `symbol("x", rational)` both return a Symbolic value with `ok: true` — the statement is only true of `solve`,
and it does not say so.

Reproduction: u01/u02 raw rows in §2.8, versus u05 (`solve(..., rational)` -> `InvalidOperation`/`DomainError`).
Second run: the same two scripts were the first two entries of the corpus run and are reproduced in D07; `natural`
is not a domain keyword at all (`Undefined variable 'natural'`, u03/u06).

### F15 — P2 — the cancellation envelope introduces undocumented top-level keys

**One sentence.** `partialOutput` and `partialVariables` appear in the `Cancelled` envelope but in no section of
`dsh-protocol.md`, which documents `output`/`variables` and states that print output lands in `output`.

Reproduction: C01–C03 raw envelopes above; `partialOutput: []` and `partialVariables: [{name: x, ...}]` are
present in all three runs.

### F16 — P3 — `--file ""` is treated as a script error, not a usage error, with framework text

**One sentence.** An empty `--file` value exits 1 with `code: FileReadError`, `category: ParseError` and the message
`The value cannot be an empty string. (Parameter 'path')`, where exit 2 (usage) is the documented class for a
malformed flag value.

Reproduction: a25 plus the second clean run quoted after the §2.1 table.

### F17 — P3 — a UTF-16 script is reported as a domain error

**One sentence.** A UTF-16LE file with the single statement `print(1)` is rejected with
`code: InvalidOperation`, `category: DomainError`, `message: "Unexpected character ' ' at position 1."` — a decode
failure wearing a mathematical category, with no statement about encodings anywhere in the contract.

Reproduction: X09 raw cell (`--file utf16.ls`).

---

## 4. COULD NOT DECIDE

1. **`exact: true` on `evalf(pi, 0)` is either P0 or correct.** I could not settle whether `Real.exact` means
   "the stored rational is exact" (in which case the field is honest: 314159...679/10^1000 *is* that rational) or
   "the answer is mathematically exact" (in which case it is a wrong exactness claim about pi). Evidence for the
   first reading: `evalf(1/3, 5)` returns `0.33333` with `exact:false` — the flag tracks the rational backing, not
   the digit string. Evidence against: an agent reading `exact: true` next to an `evalf` of pi has no structural
   hint that the value is a truncation. The inconsistency itself (F11) is not in doubt.
2. **Two harness process crashes, not attributable to the product.** Two `run_code` batches ended with the Windows
   job runner exiting `3221226505` (0xC0000409) and `134`, and one PowerShell then reported
   `Starting the CLR failed with HRESULT 80004005`. Every suspect input (`(-8)^(1/2)`, `(-2)^(1/2)`, `(-0.5)^(1/2)`,
   `evalf(pi, ...)`, the conf corpus) was re-run afterwards and the binary produced `exit=0`/`exit=1` with a full
   envelope each time, so the failures are on the harness side; I did **not** observe the product die without an
   envelope.
3. **Invalid-UTF-8 handling (X10).** A script containing the Latin-1 byte 0xE9 exits 0, and the decoded string was
   not captured, so whether the byte becomes U+FFFD, mojibake, or is dropped is unknown.
4. **The printed text of the unicode probe (X11) was not captured**, only that the `alpha` identifier parsed and
   that `solve_full` on it succeeded; round-tripping of non-ASCII `print` output is unverified.
5. **The exact `--print-budget` trigger.** Two data points pin it to "pretty length 12 -> full, 13 -> truncated"
   and one more to "3-node long-symbol -> truncated", but I did not read the implementation, so the precise
   predicate (F6) is inferred, not proven.
6. **Newline as a statement separator (X12).** The binary treats a newline as a separator; the brief for this
   audit says it does not. No contract document covers separators, so no verdict is asserted.
7. **Whether "exhaustive" binds (F7).** `dsh-protocol.md` never says the list is exhaustive; a repository plan doc
   does. The severity of F7 therefore depends on which document is treated as the contract.
8. **Source precedence is undocumented.** `--eval` beats `--file` beats `--stdin` in both orders tested (a28/a29/a49);
   nothing states this, so it is recorded, not scored.
9. **Long-running inputs (X18, u84, u87).** `integrate_full(1/(x^100-1), x)` produced no envelope in 60 s and
   `series(tan(x), x, 0, 200)` none in 25 s. No document promises a time limit, and the flag meant to bound it
   overshoots (F2), so I record them as undecided rather than as defects.
10. **Mathematical values I did not capture** for `0^0`, `sqrt(-1)`, `log(0)`, `log(-1)`, `assume(...)+solve(...)`
    (u54–u57, u81, u83): the rows hold structurally (exit 0, envelope) but the values are unverified.

---

## 5. Summary

I attacked the command-line surface (all 13 documented options plus 6 undocumented forms, each with valid, missing,
empty, negative, huge and malformed values), the file surface (missing, empty, directory, BOM, CRLF, UTF-16,
invalid UTF-8, unicode, 200 KB and 400 KB inputs), the exit-code and error-class surface, stream purity under
20 000 prints and 100 000-digit values, the `--print-budget` and `--cancel-after` semantics at their boundaries, and
`capabilities()` — every one of its 16 advertised entries driven by its own verbatim trigger (16/16 advertised
code+category pairs reproduce) and then 90 further scripts hunting refusals the statement does not list.
**278 probe rows / about 500 process invocations: 200 HELD, 66 FINDING rows, 12 INCONCLUSIVE**, rolling up into
**18 findings: one P0, seven P1, eight P2, two P3**. The P0 is F0 (`solve_full(exp(x) == x, x)` answers
`Solved`/`Complete` with the circular "solution" `log(x)`); the P1s are F1 (capabilities advertises a refusal that
`(-4)^(1/2)` does not perform), F2 (`--cancel-after 200` overshoots to 46.27 s), F3 (parse errors are `DomainError`,
file errors are `ParseError`), F4 (print output dropped on error), F5 (`evalf(pi, N)` ignores N), F6
(`--print-budget` is not node-bounded) and F7 (three unlisted refusal classes).
What held is worth as much: the envelope is genuinely the only thing on stdout in every default-mode run, stderr is
never used for anything an agent needs, the three exit codes behave as documented, BOM/CRLF/unicode/large files are
handled, deep inputs are refused with a clean envelope instead of a crash, `timings[].position` carries true source
offsets, and the structural Diagnostic shape (six ordered fields, enum `category`, array `details`) was correct in
every one of the ~250 envelopes inspected.
