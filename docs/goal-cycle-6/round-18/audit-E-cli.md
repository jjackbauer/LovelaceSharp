# Cycle-6 · round-18 · audit E — CLI-surface differential fuzzing

**Target**: the published Native AOT binary `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5 801 472 bytes, 2026-09-12 16:56:28 local).

**Method**: every probe is a real process launched through `System.Diagnostics.Process` with
stdout/stderr/stdin redirected; exit code, stdout bytes, stderr bytes and the parsed envelope are read
from the process, never from a shell transcript. Arguments are handed over as a Win32-quoted command
line, so scripts containing double quotes, embedded CR/LF and empty values arrive intact — this is the
route that avoids the `--eval "…"` quote-mangling trap round 13 recorded.

**Reproduction rule**: every finding below was run **at least twice**; the run counts are stated. Where
an observed envelope is quoted it is trimmed to the deciding fields and marked with `…`.

**Strategy**: 130+ invocations of flag × input combinations — `--eval` / `--file` / `--stdin` together
and separately, `--json` / `--text`, `--omit-functions` / `--omit-variables`, `--print-budget`
0/1/2/3/4/5/8/12/16/20/24/32/48/64/1000/2147483647, `--cancel-after` 0/1/100/1000000000/2147483648,
`--plot-dir` and `--plot-file` at 19 + 10 valid and hostile values, missing files, a directory and a
locked file where a script is expected, empty / whitespace-only / BOM / BOM-only / UTF-16 / non-UTF-8 /
CR-only / CRLF / 20 000-statement files, empty scripts, print-then-fail scripts, huge arguments
(30 000 chars), flags given twice, flags as each other's values, unknown flags, and no flags at all.

**Recorded-items check (done first, as required)**: the closed lists in
`docs\goal-cycle-6\evidence.md` (EVD-277…EVD-288), `docs\symbolics\a-plus-cycle-6-amendment.md`
(P-1…P-16 and the five P2 bounds at `:40`) and the four round-13 audits were read before probing. Every
match is named in the finding or in §"Matched a recorded item — not counted as new" below. Two of the
findings below are **cycle-5 P1s that are still live on this binary and appear in neither the closed
list nor the amendment's P.2 bounds**; they are reported as violations, explicitly **not** as new.

---

## F1 — **P1** — every runtime (non-parse) failure reports the source position **0 / line 1 / column 1**, whatever statement failed

### Exact command (2 runs, byte-identical apart from durations)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'print("hello"); nosuchfn(1)' --json --omit-functions
~~~

### Observed (verbatim, trimmed to the deciding fields)

~~~json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InvalidOperation","category":"DomainError","message":"Unknown function 'nosuchfn'.",
 "recoverable":true,
 "diagnostics":[{"message":"Unknown function 'nosuchfn'.","position":0,"line":1,"column":1}],
 "elapsed":"344 µs","elapsedTime":{"value":344,"unit":"µs"},
 "timings":[{"position":0,"elapsed":{"value":32.6,"unit":"µs"},"resultKind":"Void","hasOutput":true},
            {"position":16,"elapsed":{"value":44.6,"unit":"µs"},"resultKind":"Void","hasOutput":false}]}
~~~

The failing call is the **second** statement, at source offset **16** — the envelope's own `timings`
says so — and the diagnostic claims offset **0**. Exit 1. Four failing offsets were measured, twice
each, and every one reports `position 0, line 1, column 1`:

| script | failing statement at | `diagnostics[0]` | `timings[].position` |
|---|---|---|---|
| `nosuchfn(1)` | 0 | `{pos:0,line:1,col:1}` | `[0]` |
| `1+1; nosuchfn(1)` | 5 | `{pos:0,line:1,col:1}` | `[0,5]` |
| `1+1;⏎2+2;⏎nosuchfn(1)` (LF) | 10 | `{pos:0,line:1,col:1}` | `[0,5,10]` |
| `print("hello"); nosuchfn(1)` | 16 | `{pos:0,line:1,col:1}` | `[0,16]` |

Same for other runtime classes (1 run each, all `position 0`): `1+1; det(1)` →
`InvalidArgument` `"det(): argument 1 must be a square matrix; got Natural."`, `1+1; 1/0` →
`DivisionByZero`, `1+1; x[0]` → `InvalidOperation` `"Undefined variable 'x'."`.

### What the correct behaviour is, and how I know

* The protocol document calls the error envelope's `diagnostics` array **"the parser's source-position
  form (`message`/`position`/`line`/`column`)"** (`docs\symbolics\dsh-protocol.md:225-228`) and its own
  error example carries `"position": 0, "line": 1, "column": 1` for an error that really is at offset 0
  (`:189-199`). A source position that is 0 for a failure at offset 16 is not a source position.
* The same envelope already carries the true position in `timings[].position`, which the document
  defines as **"the zero-based source offset of the statement"** (`dsh-protocol.md:153-155`) — so the
  envelope contradicts itself. The failed statement is timed and its offset is known.
* The runner knows how to do this correctly: the deadline-overrun diagnostic is built from the last
  timing's position and from the **original** source (`Lovelace.Run\Runner.cs:400-415`, `:418-435`).
* **The repository's own golden fixture pins the wrong value.** `Lovelace.Run.Tests\fixtures\error_envelope.ls:1`
  is `x = symbol("x"); solve(2*x == 1, x, integer)` — the failing call starts at offset **17** — and
  `Lovelace.Run.Tests\fixtures\error_envelope.json:13-15` pins `"position": 0, "line": 1, "column": 1`.
  The pin is what makes this invisible to the suite.
* Located cause (observation, not a proposed fix): `SuiteEngine.ToDiagnostic` builds the position by
  scraping `at position (\d+)` out of the exception **message** and defaults to `0` when the message
  has no such phrase (`Lovelace.Suite\SuiteEngine.cs:359-370`); parse errors carry the phrase, runtime
  errors do not.

### New?

**NEW.** No cycle-3…6 audit files this: a docs-wide search for a "wrong/misleading/meaningless
position" finding returns nothing about diagnostics, the value appears only as incidental raw output in
older transcripts (`docs\goal-cycle-4\round-09\audit-P2P6-wire.md:190`,
`docs\goal-cycle-5\audit1\A1-numeric.md:335`, `A3-symbolic.md:626-638`), and round 13 verified only the
**parse** path (`docs\goal-cycle-5\audit1\A4-docs.md:56`, `A3-symbolic.md:479`). Not in EVD-277…288, not
in P-1…P-16, not in the amendment's five P2 bounds.

---

## F2 — **P1** — `line`/`column` are computed against the runner's rewritten source, so every error reports **line 1**

### Exact command (2 runs each, byte-identical)

~~~powershell
& out\aot\Lovelace.Run.exe --eval "1+1;<LF>2+2;<LF>)" --json --omit-functions
& out\aot\Lovelace.Run.exe --file crlfparse.ls --json --omit-functions   # file bytes 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29
~~~

### Observed (verbatim, trimmed)

~~~json
"diagnostics":[{"message":"Unexpected token ')' at position 10: expected a number, string, identifier, '[', or '('.",
                "position":10,"line":1,"column":11}]
~~~

The offending `)` is on **line 3** of both scripts. The reported `line` is **1** and `column` is
`position + 1` (11), i.e. a column far past the end of line 1 (`1+1;` is 4 characters). Both the LF and
the CRLF variant, and both the `--eval` and the `--file` route, report the same `line 1`.

### What the correct behaviour is, and how I know

* `dsh-protocol.md:225-228`: the error envelope's diagnostics are the source-position form; a line and
  column that do not exist in the caller's script are wrong.
* The runner hands the engine a **rewritten** script (`Lovelace.Run\Runner.cs:197` passes
  `ScriptSource.ToSemicolonStatements(source)`), which replaces every top-level newline with `;`
  (`Lovelace.Suite\ScriptSource.cs:78-89`), and the engine counts newlines in *that* text
  (`Lovelace.Suite\SuiteEngine.cs:372-388`, `_lastSource`) — where there are none.
* The file's own remark says the opposite is intended: **"the engine's `position` diagnostics still
  index into the input and hosts can recompute line/column from their original source"**
  (`ScriptSource.cs:19-24`) — the runner does recompute line/column from the original source for the
  overrun diagnostic (`Runner.cs:418-435`) but not for these.

### New?

**RECORDED, NOT CLOSED — not claimed as new.** The cycle-5 diagnosis wrote this down as an **open
question** with an explicit "that needs its own probe":
`docs\goal-cycle-5\diagnosis\D-D-robustness.md:515-519` ("diagnostic positions are computed against
the *rewritten* source … A multi-line script should therefore report line 1 for every error; that needs
its own probe"). The probe it asked for is above; the item was never filed as a finding, never fixed,
and is in neither P-1…P-16 nor the amendment's P.2 list. It is reported here because it is a live
violation of the position contract.

---

## F3 — **P1** — `timings[].position` is **not** a source offset when the script uses CRLF: the runner's CRLF→LF rewrite shortens the text by one character per line

### Exact command (2 runs each)

~~~powershell
# 13-byte file, hex 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29  (the ")" is at 0-based offset 12)
& out\aot\Lovelace.Run.exe --file crlfparse.ls --json --omit-functions
# three statements with CRLF separators: true offsets 0, 6, 12
& out\aot\Lovelace.Run.exe --eval "1+1;<CRLF>2+2;<CRLF>3+3" --json --omit-functions
~~~

### Observed (verbatim, trimmed)

| input | true start offsets | `timings[].position` reported |
|---|---|---|
| `1+1;⏎2+2;⏎3+3` (LF) | 0, 5, 10 | **0, 5, 10** ✔ |
| `1+1;⏎2+2;⏎3+3` (CR only) | 0, 5, 10 | **0, 5, 10** ✔ |
| `1+1;␍⏎2+2;␍⏎3+3` (CRLF) | 0, 6, 12 | **0, 5, 10** ✘ |
| 13-byte CRLF file, `)` at offset 12 | 12 | diagnostic `"position":10,"line":1,"column":11` ✘ |

The CRLF file's own bytes are `31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29`: the offending character is the
13th byte and the envelope reports offset 10 — off by one per preceding CRLF line break (two CRLFs →
off by two, measured).

### What the correct behaviour is, and how I know

* `dsh-protocol.md:153-155`: **"`timings[].position` is the zero-based source offset of the statement"**.
  For CRLF input it is not the offset into the file (or the `--eval` argument) the caller supplied.
* `ScriptSource.cs:19-24` states the rewrite is **"length-preserving: each newline becomes exactly one
  character … so the engine's `position` diagnostics still index into the input"**. That remark is false
  for CRLF, because the first statement of the method is
  `source.Replace("\r\n", "\n").Replace('\r', '\n')` (`Lovelace.Suite\ScriptSource.cs:27`) — two
  characters become one **before** the length-preserving pass described in the remark.
* Consequence for a consumer: a Windows-authored script (CRLF is the Windows default) has every timing
  offset after the first line pointing one character early, and every diagnostic offset likewise; the
  shift grows with the line number.
* **A second, related limb of the same line** (measured, 2 runs, but I do **not** claim it as a wrong
  value): a CRLF inside a string literal is rewritten too — the file whose bytes are
  `78 20 3D 20 22 41 0D 0A 42 22` (`x = "A␍␊B"`) crosses as `"structured":{"kind":"Text","value":"A\nB"}`,
  so the CR is gone. `ScriptSource.cs:15-17` declares "Normalizes CRLF/CR to LF" as the method's purpose,
  so this limb is arguably the documented normalisation rather than a defect; it is recorded here
  because it is the same line and because it means a script cannot carry a CR+LF inside a string literal
  at all.

### Severity note, stated so it can be argued

A reader applying the brief's P0 wording literally ("wrong value … on valid input") may grade F3 **P0**:
the input `1+1;␍⏎2+2;␍⏎3+3` is valid, and a documented machine field is wrong on it. I grade it **P1**
(a false claim in the machine API) because the wrong field is a source-span offset, not a computed
value, and nothing crashes; the parent can re-grade.

### New?

**NEW.** The cycle-5 diagnosis reasoned about the *newline→`;`* rewrite (which is length-preserving)
and did not measure CRLF; a search for the CRLF collapse / "length-preserving" over `docs\` finds no
finding or question about it. Not in EVD-277…288, P-1…P-16, or the P.2 bounds. (F2 and F3 share a
neighbourhood — both are about positions not describing the caller's source — but the mechanisms are
distinct lines of `ScriptSource.cs`/`SuiteEngine.cs` and the measurements are independent.)

---

## F4 — **P2** — `--omit-variables` is silently ignored on the cancelled path

### Exact command (2 runs each)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'x = symbol("x"); y = 42; print("hello"); sum(1..10000000)' --json --cancel-after 100
& out\aot\Lovelace.Run.exe --eval 'x = symbol("x"); y = 42; print("hello"); sum(1..10000000)' --json --cancel-after 100 --omit-variables
~~~

### Observed (verbatim, both runs; the two envelopes differ only in the timing digits)

Without the flag:

~~~json
"partialOutput":["hello"],
"partialVariables":[{"name":"x","kind":"Symbolic","display":"x","structured":{…}},
                   {"name":"y","kind":"Natural","display":"42","structured":{…}}],
"cancellation":{"budgetMs":100,"elapsedMs":111.508,"stopped":true,"exceeded":true,"excessMs":11.508}
~~~

With `--omit-variables`: **byte-identical deciding fields** — `"partialOutput":["hello"]`,
`"partialVariables"` still carrying `x` and `y` with their full `structured` projections, same
`cancellation` block. There is no `variables` key in either envelope (the error DTO has none).

### What the correct behaviour is, and how I know

* The flag's own help: **"`--omit-variables` omit the variables array from the envelope (agent loops)"**
  (`Lovelace.Run\Runner.cs:577`), and the code comment calls it "payload control for agent loops,
  mirroring `--omit-functions`: the variables array is state the agent usually already tracks"
  (`Runner.cs:88-91`).
* The repository's verification table states the intended scope as **any run**: "`--omit-variables` |
  any run | variable list empty, envelope otherwise unchanged"
  (`docs\symbolics\a-plus-cycle-3-alignment.md:374`). On a cancelled run the variable list is not
  empty — the whole point of the flag (payload reduction on the largest envelope the runner emits:
  3 036 bytes here, of which the variables are most) is not delivered.
* It is an oversight rather than a design carve-out: the **same** catch block honours the sibling knob —
  `--print-budget 3` on a cancelled run is honoured in `partialVariables`
  (`"truncated":true,"truncationReason":"node-budget","budget":3`, `prettyLen 49` of a 98-node value,
  1 run) — while `omitVariables` is never consulted on that path (`Runner.cs:273-280`).

### New?

**NEW.** No docs hit for an `--omit-variables`/cancelled interaction; the flag's tests exercise the
success path (`Lovelace.Run.Tests\VariableStructuredProjectionTests.cs:154-158`). Not in EVD-277…288,
P-1…P-16 or the P.2 bounds.

---

## F5 — **P1** — a failing run drops the script's print output entirely (invariant 1)

### Exact command (2 runs, byte-identical apart from durations)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'print("hello"); nosuchfn(1)' --json --omit-functions
~~~

### Observed (verbatim, trimmed)

~~~json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InvalidOperation","category":"DomainError","message":"Unknown function 'nosuchfn'.",
 "recoverable":true,"diagnostics":[…],"elapsed":"333.5 µs","elapsedTime":{…},
 "timings":[{"position":0,…,"resultKind":"Void","hasOutput":true},
            {"position":16,…,"resultKind":"Void","hasOutput":false}]}
~~~

The envelope's key set is exactly
`protocolVersion, symbolicFormatVersion, mathIrVersion, ok, code, category, message, recoverable,
diagnostics, elapsed, elapsedTime, timings` — **no `output`, no `partialOutput`** — and `"hello"` is
nowhere on stdout (stdout is that one JSON line) and nowhere on stderr (0 bytes). The same script
under cancellation *does* return it: `--cancel-after 100` → `"partialOutput":["hello"]` (2 runs).

### What the correct behaviour is, and how I know

* `dsh-protocol.md:9-10` (invariant 1): **"stdout carries the envelope and nothing else. Anything the
  script prints with `print(...)` is captured and returned in the top-level `output` array."** The
  capture demonstrably happened — `timings[0].hasOutput` is `true` — and the value is then dropped by
  the host, which is a loss of data the API says it carries.
* The behaviour is deliberate-looking (`Runner.cs:273`: partial output is published only for
  `code == "Cancelled"`) but it is not written down anywhere in the protocol document, and the
  document's only statement on the matter is unqualified.

### New?

**RECORDED, NOT CLOSED — not claimed as new.** Cycle-5 audit #1 filed exactly this: **F9 (P1) — "Print
output is lost on a script error (no `output` key) and moved to `partialOutput` on cancellation —
invariant 1"** (`docs\goal-cycle-5\audit1\A2-wire.md:30`, `:216-230`), re-recorded in audit #2
(`docs\goal-cycle-5\audit2\B3-surface.md:589`) and assigned to the "wire round" by the triage
(`docs\goal-cycle-5\audit1-triage.md:33`). The cycle-6 wire round closed the other limbs of that
cluster (P-7: trailing CR, `variables[]` structure, `divrem`, `assumptions()`, arity metadata) but not
this one, and the amendment's P.2 says **"No P0 and no P1 is outstanding"**
(`docs\symbolics\a-plus-cycle-6-amendment.md:40`). F5 is a live P1 that the closed list omits.

---

## F6 — **P1** — `ParseError` is attached to the wrong layer in both directions

### Exact commands (2 runs each)

~~~powershell
& out\aot\Lovelace.Run.exe --eval '1 +' --json --omit-functions
& out\aot\Lovelace.Run.exe --file 'C:\definitely\missing\x.ls' --json --omit-functions
~~~

### Observed (verbatim, trimmed)

~~~json
{"ok":false,"code":"InvalidOperation","category":"DomainError",
 "message":"Unexpected token '' at position 3: expected a number, string, identifier, '[', or '('."}
{"ok":false,"code":"FileReadError","category":"ParseError",
 "message":"Cannot read script file 'C:\definitely\missing\x.ls': Could not find a part of the path '…'."}
~~~

Both exit 1 with well-formed envelopes. The pattern is stable across the class (all measured twice):
`'#'`, `'~'`, `'?'`, `'@'`, `'\`', `'} '`, `'f(1'`, `'x ='`, `';1+1'`, `'1+1;;'`, `'1+1; ;2+2'` and the
non-UTF-8 file all cross as `InvalidOperation`/`DomainError`; only the unterminated string literal is
`InvalidInput`/`ParseError` (a `FormatException`); nothing was parsed for the missing/denied/locked file,
yet that is the one that carries `ParseError`.

### What the correct behaviour is, and how I know

* `dsh-protocol.md:207-209`: categories are spelled from the one taxonomy `ErrorCategory`
  `{ParseError, DomainError, UnsupportedOperation, BudgetExceeded, NoSolution, TypeMismatch,
  InternalInvariantFailure}`. A lexer/parser refusal is the `ParseError` member; a file that could not be
  read is not a parse error at all. An agent branching on `category` to choose "fix my syntax" vs "fix my
  file path" is given the opposite of the truth — the wording used by the cycle-5 audits themselves
  (`docs\goal-cycle-5\audit2\B3-surface.md:567`, `docs\goal-cycle-5\audit1\A4-docs.md:369`).
* Located cause (observation): `Lovelace.Run\Runner.cs:334-337` maps `FormatException`→`ParseError` and
  `InvalidOperationException`→`DomainError`, while `Tokenizer`/`Parser` refuse with
  `InvalidOperationException` (`Lovelace.Suite\Tokenizer.cs:124`, `Parser.cs:71`, `:611`).

### New?

**RECORDED, NOT CLOSED — not claimed as new.** Cycle-5 audit #2 filed it as **F3 (P1) — "`ParseError` is
attached to the wrong layer in both directions"** (`docs\goal-cycle-5\audit2\B3-surface.md:30-31`,
`:563`, with rows `v01`–`v04`, `a23`, `a24`), audit #1 independently (`A3-symbolic.md:440`,
`A4-docs.md:364-369`), and the cycle-5 diagnosis proposed the mapping and a failing test
(`docs\goal-cycle-5\diagnosis\D-D-robustness.md:492-510`). It is in neither the cycle-5 O.1 closed list
nor cycle-6's P-1…P-16, and the amendment's P.2 does not list it among the remaining bounds — so the
"no P1 outstanding" sentence at `a-plus-cycle-6-amendment.md:40` does not cover it.

---

## F7 — **P2** — `--print-budget` is non-monotone: a **larger** budget returns **less** text

### Exact command (2 runs each, identical output)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'x = symbol("x"); expand((x+1)^10)' --json --print-budget 12
& out\aot\Lovelace.Run.exe --eval 'x = symbol("x"); expand((x+1)^10)' --json --print-budget 16
~~~

### Observed (both runs)

| budget | `pretty` length | `canonical` length | `nodeCount` |
|---|---|---|---|
| 12 | **73** | 73 | 48 |
| 16 | **31** | 97 | 48 |

### What the correct behaviour is, and how I know

The implementation's own contract states the intent — "The prefix is proportional to the fraction of the
expression the budget allows, so a larger budget still returns at least as much"
(`Lovelace.Suite\StructuredProjection.cs:135-137`) — while the kernel printer applies a character floor
of `max(48, 6 × budget)` (`Lovelace.Symbolics\Printing.cs:387`), so at budget 16 the projection's
proportional cut (31 chars) is applied where at 12 the printer's floor (73 chars) was. The protocol
document itself promises only "a ` …`-terminated prefix of the real rendering" (`dsh-protocol.md:17-20`),
which both budgets satisfy — the abbreviation is never a re-ordering.

### New?

**RECORDED — not claimed as new.** Round-13 audit D filed this exact pair as **F3 (P2)**
(`docs\goal-cycle-6\round-13\audit-D-workflow.md:157`, `:299`) and the cycle-6 amendment lists it as
one of the five remaining P2 bounds (`a-plus-cycle-6-amendment.md:40`). Reproduced here only to confirm
it is still live. **No P0/P1.**

---

## F8 — **P2** — `--text` publishes no envelope at all: on failure, exit 1 with **0 bytes** on stdout

### Exact command (2 runs, identical)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'nosuchfn(1)' --text
& out\aot\Lovelace.Run.exe --eval 'print("hi"); 1+1' --text
~~~

### Observed (verbatim)

* failure: exit **1**, stdout **0 bytes**, stderr `Error [InvalidOperation\DomainError]: Unknown function 'nosuchfn'.`
* success: exit 0, stdout 28 bytes, `= 2 (Natural) / hi /   _ = 2` — human text, not JSON.

### What the correct behaviour is, and how I know

* `dsh-protocol.md:9-10` (invariant 1) — "stdout carries the envelope and nothing else" — and `:26`
  (invariant 6, "Errors are structural: `code`, `category`, `message`, `recoverable`, `diagnostics`")
  have **no carve-out for `--text`**. In this mode a failure produces no code and no category anywhere on
  stdout; the `code`/`category` exist only inside a human sentence on stderr.
* **Stated plainly so this is not over-counted:** `--text` is a documented CLI option of the runner
  itself ("`--text` emit a human-readable summary", `Lovelace.Run\Runner.cs:582`); it is only the
  *protocol document* that never mentions the mode, and round-13 audit D explicitly declared `--text`
  out of scope (`docs\goal-cycle-6\round-13\audit-D-workflow.md:311`). This is a documentation gap in
  `dsh-protocol.md`, not a hidden defect, hence P2.

### New?

**NEW** as a filed item (round 13 left `--text` unchecked; the protocol document does not mention the
mode anywhere).

---

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| F1 | **P1** | NEW | `--eval 'print("hello"); nosuchfn(1)' --json --omit-functions` → `"diagnostics":[{"position":0,"line":1,"column":1}]` while `timings[1].position` is `16`; pinned as 0 by `Lovelace.Run.Tests\fixtures\error_envelope.json:13-15` for a failure at offset 17 |
| F2 | **P1** | no — recorded open question, never probed (`D-D-robustness.md:515-519`) | `--eval $'1+1;\n2+2;\n)'` → `"position":10,"line":1,"column":11` — the `)` is on line 3 |
| F3 | **P1** (P0 argued) | NEW | 13-byte CRLF file whose `)` is at offset **12** → `"position":10`; 3-statement CRLF script → `timings` positions `[0,5,10]` where the true offsets are `[0,6,12]` (LF and CR controls are correct) |
| F4 | P2 | NEW | `--eval 'x = symbol("x"); y = 42; print("hello"); sum(1..10000000)' --json --cancel-after 100 [--omit-variables]` → `partialVariables` carries `x` and `y` with **and** without the flag |
| F5 | **P1** | no — recorded, not closed (cycle-5 A2-wire **F9**) | `--eval 'print("hello"); nosuchfn(1)'` → exit 1, no `output`/`partialOutput` key, `"hello"` nowhere; the same script cancelled → `"partialOutput":["hello"]` |
| F6 | **P1** | no — recorded, not closed (cycle-5 B3-surface **F3**) | `--eval '1 +'` → `InvalidOperation`/`DomainError`; `--file C:\definitely\missing\x.ls` → `FileReadError`/`ParseError` |
| F7 | P2 | no — recorded, still live (audit-D **F3**) | `--print-budget 12` → 73 chars; `--print-budget 16` → 31 chars, same expression |
| F8 | P2 | NEW | `--eval 'nosuchfn(1)' --text` → exit 1, stdout 0 bytes, `Error [InvalidOperation\DomainError]: …` on stderr |

Counts: **2 NEW P1s (F1, F3); 2 recorded-but-not-closed P1s (F5, F6); 1 P1 that cycle 5 recorded only
as an unprobed open question and that is measured here for the first time (F2); 3 P2 (F4 and F8 NEW,
F7 recorded and still live).** No P0 was reproduced: every invocation
returned a documented exit code (0/1/2), stdout never carried anything but the single well-formed JSON
envelope in `--json` mode, and no probe crashed, aborted or wrote an unhandled exception. No item was
found where the exit code and the envelope disagree.

---

## Matched a recorded item — **not counted as new** (checked first, as required)

| item | where recorded | what I measured on this binary |
|---|---|---|
| `--plot-dir` naming an existing file aborts the process | EVD-282; audit-C **F1 (P0)**, **closed** by `d9f2a7c` (EVD-283) | **CLOSED, confirmed**: `--plot-dir` `C:\Windows\notepad.exe` / `kernel32.dll` / `README.md` / `""` / `notepad.exe\sub` / `NUL` / `CON` / `\\.\NUL` / `C:\bad<name>` / `C:\x|y` / `C:\a:b` / `*` all → exit 1, `PlotDirectoryError`/`TypeMismatch`, `recoverable:true`, well-formed envelope, 0 stderr bytes (12 values × 1 run; `pd_unc` and a valid directory exit 0) |
| a failed plot **write** crosses as an internal failure | EVD-282; audit-C **F3**, **closed** by `1e6fd72` (EVD-288) | **CLOSED, confirmed**: `--plot-file` `''` / `nope/x.svg` / `.` / `C:\Windows\notepad.exe` / `x`*300+`.svg` → exit 1 `PlotFileError`/`TypeMismatch`, `recoverable:true`; a valid name and an absolute path in `%TEMP%` exit 0 with a `plot` object |
| `--print-budget` does not bound a value already over budget | amendment P-7, closed | **CLOSED, confirmed**: 88 (expression × budget) cases at budgets 1–1000, every truncated rendering ends `" …"`, its text is a **prefix** of the unbudgeted rendering, and `truncated`/`truncationReason:"node-budget"`/`budget` are present and correct — **0 violations** |
| usage errors exit 2 with no envelope | cycle-5 A2-wire **F23** (P2); reproduced in audit-D `:275-276` | reproduced exactly: `--bogus`, no args, `--`, `-`, `--eval` (no value), `--file`/`--plot-dir`/`--plot-file`/`--cancel-after`/`--print-budget` missing their value, `--cancel-after 0|-5|abc|2147483648`, `--print-budget 0|-1|abc`, a second bare file, `<file> --bogus` → exit 2, **stdout 0 bytes**, usage text on stderr (21 invocations). Recorded, not counted |
| `Void` statement omits `result`; `truncationReason:"depth-limit"` unreachable from the CLI | cycle-5 A3-F17/A4-F10; audit-D `:272-281` | reproduced: an empty script, a whitespace-only script, a BOM-only file and `--file NUL` all exit 0 with no `result` key; no CLI input sets `MaxDepth` (`Runner.cs:442-443`). Recorded, not counted |
| CLI values: `--file` missing/dir, non-UTF-8 bytes, parser junk, huge `--print-budget` | audit-C "Coverage that held" `:254-256` | re-confirmed on top of it: `--file` a directory `C:\Windows` / `.`, a locked file (`FileShare.None`), a non-existent pipe → `FileReadError`/`ParseError`, exit 1; empty / whitespace / BOM-only / BOM / UTF-16 / CR-only / non-UTF-8 files and a 20 000-statement file (1 924 774-byte envelope, 20 000 timings) all structured; `--eval` with 30 000 characters → exit 0, 150 549-byte envelope |
| audit-C's four findings (plot-dir abort, OOM class, plot write, `len([[],[]])`) and audit-D/A/B's findings | EVD-283/EVD-288, P-1…P-16 | none re-observed; not re-filed |

## Invariants that held under the fuzz (no finding)

* **stdout purity / one envelope**: every `--json` run produced exactly one JSON document, no extra
  lines, in every flag combination (including `print()` scripts, a print-then-fail script, a cancelled
  script with partial output, a plot script, `--print-budget`, and both omit flags).
* **Version triple**: `protocolVersion:1`, `symbolicFormatVersion:"#!lovelace-sym 1"`, `mathIrVersion:2`
  on every envelope, success and error alike (including the zero-engine paths: unreadable file, bad plot
  directory).
* **Structural failures**: every `ok:false` envelope carried `code`, `category`, `message`,
  `recoverable`, `diagnostics`, `elapsed`, `elapsedTime` and `timings` (the last empty only where the
  document says so — a failure before any statement ran), and every `category` was a member of the
  published taxonomy.
* **Exit codes**: exit 0 ⇔ `ok:true`; exit 1 ⇔ `ok:false`; exit 2 only on the usage paths (which publish
  no envelope). **No disagreement between the exit code and the envelope was found.**
* **Flag semantics that held**: `--eval` wins over `--file` and `--stdin` in either order; a repeated
  flag takes its last value; `--json` after `--text` restores JSON; `--omit-functions` empties
  `functions` (and the key is absent on the error DTO, which has no such member); `--cancel-after`
  1/100/1000000000 produce a ledger with `exceeded:false` on fast scripts and
  `exceeded:true,stopped:true` on a cancelled one, with `excessMs = max(0, elapsedMs − budgetMs)` and
  `elapsedMs` matching `elapsedTime` to the published rounding (3 runs); `--plot-file`/`--plot-dir`
  accept a valid directory, a missing directory (created), a relative path and an absolute plot file.
* **`--print-budget` totality**: 88 cases, 0 violations of the prefix/reason/budget triple (see the
  recorded-items table).

## Could not check

| item | reason |
|---|---|
| A consumer that closes stdout early (broken pipe) | the harness always drains both streams; not attempted (audit C recorded the same gap). An unhandled `IOException` from the final `WriteLine` is the theoretical failure mode and is **not claimed either way**. |
| `--file CON` / `--file COM1` (device paths that block on a console read) | would hang the probe; `--file NUL` was checked instead (exit 0, empty script). |
| A NUL byte inside `--plot-file` | not deliverable through argv: the Win32 command line is NUL-terminated, so the process receives a truncated argument (the probe measured the truncation, not the product). Audit C hit the same wall for the empty argument. |
| Arguments beyond the Windows command-line limit (~32 767 chars) | the largest probe was 30 000 characters; beyond the limit `CreateProcess` fails, which is an OS boundary, not the runner's. |
| `--stdin` with no console and no redirection (service/host context) | not constructible from this shell. |
| "A script with only comments" | **the language has no comment syntax**: `Tokenizer.Tokenize` has no comment arm (`Lovelace.Suite\Tokenizer.cs:22-199`) and `Lovelace.Suite\docs\Language.md` has no occurrence of "comment". `#`-prefixed text is refused as `Unexpected character '#'` (`InvalidOperation`/`DomainError`); the nearest constructible inputs (empty, whitespace-only, BOM-only) were checked. |
| The `transform.unsatisfiable-conditions` diagnostic's `recoverable:false` claim (`dsh-protocol.md:86-88`) | no input in this budget produced a `TransformResult` carrying that diagnostic; the exception form (`UnsatisfiableAssumptions`) was not exercised either. Not claimed either way. |
| Exhaustiveness of the `--plot-dir`/`--plot-file` value space (19 + 10 values sampled) and of the flag grammar (flags-as-values) | sampled, not enumerated. `--plot-dir --json` silently consumes the flag as the directory value, creates a directory literally named `--json` and exits 0; no invariant in `dsh-protocol.md` states how an option value that looks like a flag must be treated, so it is recorded as an observation, not a finding. |
| Studio / REPL / DSH-plugin surfaces, and whether the F3 drift affects them | outside the published runner named as the target. |

## Disclosure

* Scratch fixtures (script files, a locked file, a non-UTF-8 file, a CRLF file) live under
  `%TEMP%\auditE\` and nowhere else.
* One probe (`--plot-dir --json`) made the runner create a directory named `--json` in the repository
  root; I removed it in the same session and `git status --porcelain` is clean (`--plot-dir *` and the
  other relative-directory probes ran with the process working directory set to `%TEMP%`; a
  `..\..\auditE2` directory created under `C:\Users\ricar\AppData` was removed as well).
* No product, test, fixture or document was edited; this report is the only file written in the
  repository.
