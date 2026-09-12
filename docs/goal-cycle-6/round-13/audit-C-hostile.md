# Cycle-6 · round-13 · audit C — hostile input shapes

**Target**: the published Native AOT binary `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5 776 896 bytes, 2026-09-12 15:06; SHA-256 `CCE441898FA6E824C761148DB97A43287CA58893230998F752CB918FF562959E`).
**Method**: every probe is a real process launched through `System.Diagnostics.Process` with stdout/stderr/stdin
redirected; exit code, stdout bytes, stderr bytes and the envelope's `code`/`category` are read from the process,
never from a shell transcript. Scripts with string literals were fed **through `--stdin`** because
`--eval "<script>"` loses inner double quotes on the way through the shell (confirmed here: a `--eval` probe that
contained `symbol("x")` arrived mangled and produced a bogus refusal — see the correction below).
**Reproduction rule**: every finding below was run at least twice on this binary; the run counts are stated.
**Scope**: my own probe list (degenerate shapes, zeros, ranges, depth, magnitudes, mixed domains, unicode,
parser junk, CLI values). The cycle-5 differential corpus, the cycle-6 bounds re-probe and the sibling
round-13 audits A (metamorphic) and D (workflow) are **not** repeated; where I rediscovered one of their items
it is listed under "Already recorded — not counted".
**Result: 4 new findings — 1 P0, 3 P1.**

---

## F1 — **P0** — `--plot-dir` naming an existing file kills the process: exit `0xC0000409`, **0 bytes on stdout**, no envelope

### Exact command (run twice, byte-identical stderr)

~~~powershell
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval 1 --json --omit-functions --omit-variables --plot-dir 'C:\Windows\notepad.exe'
~~~

### Observed (verbatim)

  exit = -1073740791 (`0xC0000409`), **stdout 0 bytes**, stderr 527 bytes:

~~~
Unhandled exception. System.IO.IOException: Cannot create 'C:\Windows\notepad.exe' because a file or directory with the same name already exists.
   at System.IO.FileSystem.CreateDirectory(String, Byte[]) + 0x448
   at System.IO.Directory.CreateDirectory(String) + 0x2a
   at Lovelace.Run.Runner.<RunAsync>d__2.MoveNext() + 0x70d
--- End of stack trace from previous location ---
   at Program.<<Main>$>d__0.MoveNext() + 0x4f
--- End of stack trace from previous location ---
   at Program.<Main>(String[] args) + 0x28
~~~

Same outcome, same code, 8 runs in total across four values (2 runs each):

| `--plot-dir` value | exit | stdout | stderr | stderr headline |
|---|---|---|---|---|
| `C:\Windows\notepad.exe` | `0xC0000409` | 0 B | 527 B | `Cannot create '…\notepad.exe' because a file or directory with the same name already exists.` |
| `C:\Windows\System32\kernel32.dll` | `0xC0000409` | 0 B | 537 B | same shape |
| `C:\Users\ricar\dev\LovelaceSharp\README.md` | `0xC0000409` | 0 B | 547 B | same shape |
| `""` (empty argument) | `0xC0000409` | 0 B | 497 B | `System.ArgumentException: The value cannot be an empty string. (Parameter 'path')` |

The empty-string row was produced through the process API (PowerShell 5.1 silently drops an empty argument to a
native command, so that row is not reproducible from `pwsh` alone — the other three are).

### What the correct behaviour is, and how I know

* The machine contract is: **"stdout carries the envelope and nothing else"** and **"Exit codes: 0 success,
  1 script/diagnostic error, 2 usage error"** — `docs\symbolics\dsh-protocol.md:9-10` and `:207-209`, repeated in
  `Lovelace.Run\Program.cs:18` and `Lovelace.Run\Runner.cs:27`. Every failure is structural
  (`dsh-protocol.md:26`). A plot directory that cannot be created is a caller-level failure; the accepted
  answers are a structured envelope with exit 1 or a usage error with exit 2. `0xC0000409` with an empty stdout
  is neither, and no code or category reaches the agent at all.
* Location: `Directory.CreateDirectory(engine.PlotOutputDirectory)` (`Lovelace.Run\Runner.cs:168`) sits **before**
  the `try` that starts at `Lovelace.Run\Runner.cs:177`; `Program.cs:24` calls `RunAsync` and catches nothing.
  The stack trace names exactly that frame.
* The argument parser accepts the value: it refuses only a *missing* argument (exit 2, measured:
  `--plot-dir` with no value → exit 2, stderr "requires a directory argument"). Nothing in the documentation
  requires the plot directory to exist or to be a directory — cycle 5 measured the opposite direction as
  working (`docs\goal-cycle-5\audit2\B3-surface.md:111`: a missing `--plot-dir` is created, exit 0).
* Severity note, stated so it can be argued: a reader may call the value "invalid input" and downgrade this to
  P1. What is not arguable is that the process dies outside the documented exit-code contract with zero bytes on
  stdout — the class the round brief forbids (`docs\goal-cycle-6\journal.md:353-357`).

### New?

**NEW.** A docs-wide search for "Unhandled exception" finds only an unrelated `dotnet.exe` trace
(`docs\goal-cycle-6\probes\help-solve-pre.txt:5`); the `plot-dir` search finds the help text, the successful
`a41`/`a42` probes (`docs\goal-cycle-5\audit2\B3-surface.md:111-112`) and the cycle-4 `plot()` finding, none of
which is this. Nothing in `docs\goal-cycle-6` records it.

---

## F2 — **P1** — a huge but well-formed request crosses as `InternalError/InternalInvariantFailure` (out of memory), `recoverable:false`

### Exact command (4 runs)

~~~powershell
& 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --eval 'zeros(1000000000)' --json --omit-functions --omit-variables
~~~

### Observed (verbatim, trimmed to the deciding fields)

~~~json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InternalError","category":"InternalInvariantFailure",
 "message":"Insufficient memory to continue the execution of the program.","recoverable":false,
 "elapsed":"3.51 s","elapsedTime":{"value":3.51,"unit":"s"},
 "timings":[{"position":0,"elapsed":{"value":3.51,"unit":"s"},"resultKind":"Vector","hasOutput":false}]}
~~~

Exit code **1**; stdout 427–538 bytes (the envelope is well formed on this path); stderr 0 bytes. Four runs:
36.3 s to envelope and exit 36.316 s (envelope `elapsed:"3.25 s"`); 0.263 s / 0.285 s (envelope
`elapsed:"2.62 ms"`, `resultKind:"Void"`); 38.1 s; and one run whose envelope was on stdout but whose process
had still not exited after 60 s (that one was killed). Peak working set 415–532 MB while the machine had
**39.3 GB free**, so this is a per-request allocation refusal, not machine exhaustion.

### What the correct behaviour is, and how I know

* `Runner.Classify` (`Lovelace.Run\Runner.cs:273-296`) enumerates the exception types that may cross the wire;
  `OutOfMemoryException` is not one of them, so it falls to `_ => ("InternalError", "InternalInvariantFailure",
  false)` (`Runner.cs:295`). `recoverable:false` tells an agent "do not retry" — a false instruction for a
  request that is merely too large, and the classification the round brief lists as forbidden
  (`docs\goal-cycle-6\journal.md:353-357`: "hunt the classes the project forbids: `InternalError` /
  `InternalInvariantFailure`, unhandled exceptions, empty stdout, non-zero exits without a code").
* No size limit is documented: `zeros(2,3)` is a documented call (`Lovelace.Suite\docs\Language.md:931`) and the
  runner does answer for 10^8 elements — `len(1..100000000)` → `100000000` in 44.5 s (measured; 461-byte
  envelope), and `x = 1..100000000` builds the vector. The boundary is the single allocation, not a bound the
  documentation states.
* Message provenance: `Insufficient memory to continue the execution of the program.` is the CLR's
  `OutOfMemoryException` text; nothing in the product sources or docs mentions it (search over `docs` and
  `*.cs`: no hits).

### New?

**NEW, and distinct from the recorded sibling.** EVD-277 (`docs\goal-cycle-6\evidence.md:49`) and audit D's F1
record an `InternalError` family that is a **wrong-shaped argument** reaching an `InvalidCastException`
("Specified cast is not valid.", 52 calls / 26 builtins). F2's trigger is a *valid* shape, its exception is
`OutOfMemoryException`, and its message is different; the sweep in EVD-277 could not have produced it.

### Bound I did not push to an envelope

`1..1..3000000000` (a 3-billion-element range) was killed at 25 s with 0 bytes on stdout; the same class is
demonstrated above with a case that answers in seconds. Listed under "could not check".

---

## F3 — **P1** — a failed plot *write* is an `InternalError/InternalInvariantFailure`, while the same class of failure on the input side is a recoverable `FileReadError`

### Exact command (2 runs; variant also 2 runs)

~~~powershell
'plot([1,2,3])' | & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables --plot-dir $env:TEMP\lv13audit3 --plot-file 'nope/x.svg'
~~~

### Observed (verbatim, trimmed)

~~~json
{"ok":false,"code":"InternalError","category":"InternalInvariantFailure",
 "message":"Could not find a part of the path 'C:\Users\ricar\AppData\Local\Temp\lv13audit3\nope\x.svg'.",
 "recoverable":false,
 "diagnostics":[{"message":"Could not find a part of the path …","position":0,"line":1,"column":1}]}
~~~

Exit 1; stdout 634–636 bytes; stderr 0. Variant `--plot-file ""` (process API) → the same code and category with
`"Access to the path '…\lv13audit2' is denied."` (600 bytes), twice. Control with a valid name
(`--plot-file ok.svg`) → exit 0, SVG written, result `Text` holding the absolute path.

### What the correct behaviour is, and how I know

* The protocol says a wrong *argument* crosses as a recoverable `InvalidArgument/TypeMismatch` and "never as an
  internal invariant failure" (`docs\symbolics\dsh-protocol.md:211-214`), and every error is structural (`:26`). An
  unwritable output path is a caller-level error, like a script file that cannot be read.
* Measured on the same binary, the read side of the same class is exactly that: `--file C:\Windows` → exit 1,
  361 bytes, `FileReadError/ParseError`, `recoverable:true`, "Cannot read script file … Access to the path … is
  denied."; `--file C:\definitely\missing\x.ls` → exit 1, 402 bytes, `FileReadError/ParseError`. The write side
  answers the non-recoverable internal-invariant class instead.

### New?

**NEW.** The `plot-file` search over `docs` finds only the help text and the *successful* probe
(`docs\goal-cycle-5\audit2\B3-surface.md:112`); "Access to the path" finds only the `--file` cases. No record of a failed
plot write.

---

## F4 — **P1** — `len()` refuses every array that has a zero dimension, contradicting its own descriptor and the same array's `shape`/`numel`

### Exact command (3 runs of the first form, byte-identical apart from durations)

~~~powershell
'len([[],[]])' | & 'C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe' --stdin --json --omit-functions --omit-variables
~~~

### Observed (verbatim, trimmed)

~~~json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"Array dimensions must be positive, but got 0. (Parameter 'shape')","recoverable":true,
 "diagnostics":[{"message":"Array dimensions must be positive, but got 0. (Parameter 'shape')","position":0,"line":1,"column":1}]}
~~~

Same refusal for `len([[]])`, `len(zeros(2,0))`, `len(zeros(0,3))`, `len(zeros(0,0))`; accepted and correct in
the neighbouring cases: `len(zeros(2,3))` → `2`, `len(zeros(2,3,4))` → `2`, `len([1,2,3])` → `3`,
`len([])` → `0`, `len(1..0)` → `0`.

### What the correct behaviour is, and how I know

* The builtin's own metadata says it answers **"Length of a vector (first dimension of an array)."**
  (`Lovelace.Suite\CoreBuiltinMetadata.cs:78`); the language documents `len([5, 6, 7, 8])` → `4`
  (`Lovelace.Suite\docs\Language.md:458-462`). So `len([[],[]])` must be `2` and `len(zeros(0,3))` must be `0`.
* The refused value is a value the product itself builds and describes structurally, so the refusal contradicts
  the product, not just the documentation: `[[],[]]` → display `[[], []]`; `shape([[],[]])` → `[2, 0]`;
  `numel(zeros(2,0))` → `0`; `zeros(2,0)` → `[[], []]`; `[[]] + [[]]` → `[[]]`.
* Observation of the mechanism (not a proposed fix): the Array branch answers `Natural(arg.AsArray().Shape[0])`
  (`Lovelace.Suite\Interpreter.cs:1795`), and the conversion behind `AsArray()` rejects a zero dimension — the
  message is the `ArgumentException` at `Lovelace.Array\NdArray.cs:34` (`nameof(shape)`). The `Vector` branch
  (`Interpreter.cs:1794`) is unaffected, which is why `len([])` works and `len(zeros(0,0))` does not.

### New?

**NEW.** A search for "Array dimensions must be positive" over `docs` → **no hits**; the only zero-dimension item on
record is cycle 5's `eye(0,0)` refusal (`docs\goal-cycle-5\audit2\B3-surface.md:400`), a different builtin whose
refusal is intentional.

---

## Already recorded — reproduced here, NOT counted as findings

| Item | Where it is recorded | What I measured on the published binary |
|---|---|---|
| A wrong **shaped** argument crosses as an internal invariant failure | `docs\goal-cycle-6\evidence.md:49` (EVD-277), `docs\goal-cycle-6\round-13\audit-D-workflow.md:297` (F1) | `det(1)`, `transpose(1)`, `matmul(2,3)` → exit 1, `InternalError/InternalInvariantFailure`, "Specified cast is not valid.", `recoverable:false` — the same class; not re-filed |
| `reshape(1,1,1,1,1,1,1,1,1)` → `InternalError` | `docs\goal-cycle-6\round-5\bounds-reprobe.md:150` ("one `InternalError/InternalInvariantFailure` remains") | reproduced verbatim, exit 1, "Specified cast is not valid."; still open, not re-filed |
| Depth budgets are enforced and no deep shape kills the process | `docs\goal-cycle-6\evidence.md:34` (EVD-262), amendment P-10 | expression nesting 256 / expression tree 512 / statement nesting 256 / symbolic 256 / value depth 1024 all refuse with `DepthExceeded/BudgetExceeded` at 200–200 000 source nesting and at 500–20 000 statement-built wraps; a value at exactly depth 1024 renders (107 554 B envelope, exit 0); recursion `f(86)` refused — no crash, no empty stdout |
| A documented zero-step range error | `Lovelace.Suite\docs\Language.md:412-419` ("A zero step is an error." / "Range step must not be zero.") | `1..0..5` → exit 1, `InvalidOperation/DomainError`, "Range step must not be zero." — documented bound, holds |

## Correction to my own first reading (so no one re-chases it)

An early `--eval`-based batch reported `symbol("")`, `symbol("1")` and `symbol("+")` as
`InternalError/InternalInvariantFailure`. **That was a harness artifact**: the shell mangled the inner quotes and
the runner saw a different script. Re-run through `--stdin`, `symbol("")` and `symbol("1")` are accepted
(exit 0, `Symbolic`); `symbol("x")` → `x`. No finding; the artifact is recorded here because the same trap
produced a "wrong refusal" that does not exist.

## Coverage that held (no finding)

Ranges: empty/descending/huge-step/negative-step all well formed (`1..5`, `1..2..7`, `-2..2`, `5..-1..1` match
`Language.md:378-410`); `1..10^20..5` → `[1]`; a Real or Boolean step is refused with a typed error.
Zero-size and degenerate matrices: `zeros(0)`, `zeros(0,0)`, `zeros(2,0)`, `zeros(0,3)`, `zeros(2,3,0)`,
`zeros(1,0)`, `eye(0)`, `det(zeros(0,0))` → `1`, `inv(zeros(0,0))` → `[]`,
`matmul(zeros(1,0),zeros(0,2))` → `[[0, 0]]`, `matmul(zeros(0,2),zeros(2,0))` → `[]`, rectangular/singular
refusals typed (`det() requires a square matrix`, `Matrix is singular`, `inner dimensions must match`).
Empty reductions: `sum([])` → 0, `prod([])` → 1, `dot([],[])` → 0, `mean/min/max/norm([])` refused by name,
`conv/fft/dft([])` → `[]`. Indexing: out-of-range and negative indexes typed
(`Index 5 is out of range for vector of length 3`). Ragged literals (`[[1,2],[3]]`, `[[],[1]]`) refused at the
literal with one message; `[[1,2],[3,[4,5]]]` accepted and printed. Digit counts: `evalf(f, -1|0|1.5|2^31)`
typed `InvalidArgument/TypeMismatch`; `evalf(sin(1), 2147483647)` is accepted and does not finish in 60 s **but
honours `--cancel-after 1000`** (Cancelled/BudgetExceeded at 1.25–1.33 s, ledger `budgetMs:1000`) — the deadline
contract holds. Magnitudes: `10^1000`, `10^-1000`, `10^1000*10^1000`, `10^(-1000)/10^1000`, `2^100000` all
answer; `1e999999` is a structured parse error (the lexer has no exponent literal). Unicode: non-ASCII inside a
string literal is refused by the lexer as `Unexpected character 'é' at position 9` — a structured parse error,
no crash; a 1 MB symbol name through `--stdin` answers in 63 ms. Parser junk: empty stdin, whitespace-only,
unterminated string (`InvalidInput/ParseError`), unterminated comment, stray `}`/`)`, `x =`, `;;;`, a UTF-8 BOM,
CR-only line endings, an embedded NUL and invalid UTF-8 bytes (`FF FE 80 C0`) all produce structured envelopes
on stdout; a 100 000-element list literal answers in 224 ms (6 169 139-byte envelope); a 20 000-term chain is
refused by the expression-tree budget. CLI values: `--print-budget 2147483647` ok, `2147483648` → usage exit 2;
`--cancel-after 0/-5/abc` → usage exit 2; `--file` missing / a directory → `FileReadError/ParseError`, exit 1;
no script → usage exit 2. `plot(sin(x))` (the cycle-4 `InternalError`) is now
`InvalidOperation/DomainError "plot() argument 1 must be a vector"`.

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| F1 | **P0** | NEW | `& $exe --eval 1 --json --omit-functions --omit-variables --plot-dir 'C:\Windows\notepad.exe'` → exit `-1073740791` (`0xC0000409`), **0 B stdout**, "Unhandled exception … Directory.CreateDirectory … Runner.&lt;RunAsync&gt;" |
| F2 | P1 | NEW | `& $exe --eval 'zeros(1000000000)' --json --omit-functions --omit-variables` → exit 1, `InternalError/InternalInvariantFailure` "Insufficient memory to continue the execution of the program." `recoverable:false` |
| F3 | P1 | NEW | `$exe --stdin --json --omit-functions --omit-variables --plot-dir $env:TEMP\x --plot-file 'nope/x.svg'` with stdin `plot([1,2,3])` → exit 1, `InternalError/InternalInvariantFailure` "Could not find a part of the path …" (also `--plot-file ""` → "Access … is denied.") |
| F4 | P1 | NEW | `$exe --stdin --json --omit-functions --omit-variables` with stdin `len([[],[]])` → exit 1, `InvalidArgument/TypeMismatch` "Array dimensions must be positive, but got 0. (Parameter 'shape')" (`len(zeros(2,0))`, `len(zeros(0,3))`, `len(zeros(0,0))` identical) |
| — | recorded | — | `det(1)` / `transpose(1)` / `matmul(2,3)` → `InternalError` "Specified cast is not valid." — EVD-277, audit-D F1 |

## Could not check

| Item | Reason |
|---|---|
| The envelope of `1..1..3000000000` (3-billion-element range) | killed at 25 s with 0 bytes on stdout; reaching the allocation refusal needs minutes and tens of GB. The same class is demonstrated by `zeros(1000000000)`. |
| Whether the F2 class also covers every allocating builtin and every size | only `zeros(10^9)`, `zeros(10^8)` and `1..10^8` were measured; a full size sweep per builtin was out of budget. |
| `--file CON` (a console device path) | would block on a console read and hang the audit; not attempted. |
| `--stdin` with an absent or closed stdin handle (no console, no redirection) | not constructible from this shell without a service/host context. |
| A consumer that closes stdout early (broken pipe) | the harness always drains both streams; not attempted. |
| Interactive surfaces — `Lovelace.Studio`, the REPL, the DSH plugin host | outside the published runner named as the target. |
| Whether every zero-dimension shape is refused by `len` (full shape lattice) | swept `[1,0]`, `[2,0]`, `[0,0]`, `[0,3]` plus the `Vector` neighbours; a shape-generator sweep was not run. |

## Disclosure

Running the probe `plot([1,2,3])` without `--plot-dir` made the binary write its default
`C:\Users\ricar\dev\LovelaceSharp\plot.svg` (15 116 B, git-ignored). I deleted it in the same session; every later
plot probe wrote inside `$env:TEMP`. No product, test, fixture or document was edited; this report is the only
file I wrote.
