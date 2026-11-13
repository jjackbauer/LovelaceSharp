# Round 103 — the interactive REPL (CLI-001..007)

Date: 2026-11-12 (America/Sao_Paulo) · Repository HEAD: `ef17854` · Scope:
`rust/lovelace-run/src/main.rs`, `rust/lovelace-run/src/repl.rs`,
`rust/lovelace-run/src/round35_run_batch.rs`, `rust/lovelace-run/tests/repl.rs` and this report.
No C# source, fixture, parity file, desktop file or other document was touched.

## 1. Reference behaviour read from the frozen C#

| Source | Behaviour |
|---|---|
| `Lovelace.Console/Program.cs` | Banner: `LovelaceSharp REPL v1.0.0`, `Arbitrary-precision math scripting, vector math, and plotting.`, `Type 'help' for a list of statements, operators, functions, and commands.`, blank line. |
| `Lovelace.Console/Repl/ReplSession.cs` | `» ` prompt; first line `Trim()`med; empty line skipped; braces counted literally (`{` +1, `}` -1, balanced when depth <= 0) and continuation read with the `… ` prompt until balanced; source = trimmed first line + `\n` + each raw continuation; EOF in the outer loop or a continuation prints `Bye!`. |
| `ReplSession.cs` commands | `exit`/`quit`, `help`, `help <arg>`, `vars`, `funcs`, `funcs <cat>`, `set pretty unicode\|ascii`, `clear`, `delete <name>`, `set precision <n>`, `set display <n>`, `run <file>`. Exact messages: `Pretty output: unicode.`, `Display digits set to <n>.`, `Computation precision set to <n> decimal places.`, `Invalid argument '<arg>': expected a positive integer.`, `Variable '<name>' deleted.` / `... is not defined.`, `All variables cleared.`, `(no variables defined)`. |
| `ReplSession.cs` rendering | Non-void result: `= {FormatValueTyped(result)}   [{LastElapsedDisplay}]` (three spaces). Error: when the message contains `at position (\d+)`, echo the accumulated source, a line of `position` spaces followed by `^`, then `Error: {message}   [{elapsed}]`; otherwise only the error line. The session continues after an error. |
| `ReplSession.cs` state | One engine per session; `_` is maintained by the engine after each successful non-void evaluation; `vars` lists variables sorted by name; `clear` removes variables but keeps functions; `run` evaluates a file with the current precision. |
| `Lovelace.Console/Repl/LineEditor.cs` | Printable insert at cursor, Backspace/Delete, Left/Right, Home/End, Up/Down history, Ctrl+C returns null (exit intent), Enter submits; non-blank submitted lines are appended to history. |
| `Lovelace.Suite/Timing.cs` | Elapsed is the largest unit worth at least one whole unit (ns/µs/ms/s/min/h), two decimals at most with zeros trimmed; nanoseconds are whole. |

**Observed fact that shaped the test strategy.** The frozen console app builds `new ReplSession()`,
whose `LineEditor` calls `Console.ReadKey`; with redirected stdin it aborts before reading a
command (`docs/goal-cycle-6/probes/help-solve-pre.txt`, observed
`InvalidOperationException: Cannot read keys when either application does not have a console...`).
Its own 15 transcript tests therefore drive the injectable
`ReplSession(StringReader, StringWriter)` seam. The Rust REPL keeps the same transcript contract
but also serves the piped flow directly, which is what this round's tests drive.

## 2. What was implemented

`lovelace-run --repl` starts a session over `Engine::new()` (no envelope, no second engine):

- the banner, `» `/`… ` prompts, brace-balanced accumulation, EOF/exit handling and the full
  command set above;
- engine state persists between commands (variables, functions, assumptions, precision, `_`);
- `set precision n` moves both the computation bound and the display override (so it also
  replaces an earlier `set display`); `set display n` pins only the display;
- result, print-output, caret and elapsed rendering as the reference;
- `--cancel-after <ms>` is honoured per command through the engine's existing deadline, so a
  cancelled statement is rendered as a normal `Cancelled` error and the session continues;
- `run <file>` reads the same BOM-aware encodings, strips a BOM as `File.ReadAllText` does, and
  reports a missing file with the reference's CLR sentence
  `Could not find file '<absolute path>'.`;
- dependency-free line input: when stdin/stdout are pipes it reads and echoes lines (exactly the
  headless transcript path); on a terminal it uses a key-by-key editor (Windows virtual-terminal
  input via kernel32 FFI, Unix `stty raw -echo`, ANSI navigation sequences), with history,
  Ctrl+C interrupt, and the reference's key semantics.

## 3. Before / after

### 3.1 Before — no interactive Rust surface

The pre-change `parse_args` rejected `--repl` through its unknown-option arm:

```
$ rust\target\release\lovelace-run.exe --repl
error: unknown option '--repl'

lovelace-run - evaluate a Lovelace script
...
[exit 2]
```

The only interactive front-end was C# `Lovelace.Console`, which cannot read a pipe (above).

### 3.2 Reference session (C# `ReplSession`, headless injection; elapsed normalised to `[T]`)

```text
» a = 5
= 5 (Natural)   [T]
» func f(x) = x^2
» _ * 3
= 15 (Natural)   [T]
» sqrt(2)
= 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727 (Real)   [T]
» set display 5
Display digits set to 5.
» sqrt(2)
= 1.41421 (Real)   [T]
» [1,2;3,4]
[1,2;3,4]
    ^
Error: Expected 'RBracket' but found ';' at position 4.   [T]
» set pretty unicode
Pretty output: unicode.
» inf
= ∞ (Symbolic)   [T]
» vars
  _ = ∞ (Symbolic)
  a = 5 (Natural)
» exit
Bye!
```

### 3.3 After — the same script through `lovelace-run --repl` (pipe, elapsed normalised)

```text
LovelaceSharp REPL v1.0.0
Arbitrary-precision math scripting, vector math, and plotting.
Type 'help' for a list of statements, operators, functions, and commands.

» a = 5
= 5 (Natural)   [T]
» func f(x) = x^2
» _ * 3
= 15 (Natural)   [T]
» sqrt(2)
= 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727 (Real)   [T]
» set display 5
Display digits set to 5.
» sqrt(2)
= 1.41421 (Real)   [T]
» [1,2;3,4]
[1,2;3,4]
    ^
Error: Expected 'RBracket' but found ';' at position 4.   [T]
» set pretty unicode
Pretty output: unicode.
» inf
= ∞ (Symbolic)   [T]
» vars
  _ = ∞ (Symbolic)
  a = 5 (Natural)
» exit
Bye!
```

The session lines are byte-identical to §3.2: a line-by-line comparison after replacing the
elapsed bracket with `[T]` reports no difference; the four leading lines are the reference
`Program.cs` banner, which the Rust binary prints before the session.

### 3.4 Cancellation (Rust, `--repl --cancel-after 200`)

```text
» sum(1..1000000000)
Error: the evaluation was cancelled by the caller.   [T]
» 1+1
= 2 (Natural)   [T]
» 
Bye!
```

The cancellation is the engine's structured `Cancelled/BudgetExceeded` failure rendered through
the ordinary error path; the session, its variables and the engine all survive it.

## 4. Tests and CLI ids

All tests run with `cargo test -p lovelace-run`: **437 unit tests + 1 recursion + 11 REPL
transcript tests, 0 failed**. The transcript tests spawn the built binary, pipe the script on
stdin, and compare the whole stdout after normalising only the volatile elapsed bracket.

| ID | Requirement | Evidence |
|---|---|---|
| CLI-001 | Implement the interactive Lovelace REPL. | `--repl` in `main.rs`; `repl::*`; all 11 tests in `tests/repl.rs`; `cargo test -p lovelace-run` exit 0. |
| CLI-002 | Preserve multiline statement accumulation. | `repl::multiline_accumulation_runs_the_block_after_braces_balance`, `repl::continuation_end_of_input_still_prints_bye_once`, `lovelace-run::repl::tests::braces_must_balance_for_continuation`. |
| CLI-003 | Support editing, history, and keyboard navigation. | `lovelace-run::repl::tests::line_buffer_edits_at_the_cursor`, `history_navigation_restores_the_draft`, `ansi_navigation_sequences_decode_to_keys`, `ctrl_c_interrupts_the_line`; terminal driver in `repl.rs` (Windows VT / Unix stty). |
| CLI-004 | Preserve interactive commands and help. | `repl::interactive_commands_and_help_match_the_reference_rendering` (help overview, `help solve`, unknown help, `funcs`, pretty, display, clear, vars, delete), `repl::run_executes_a_script_file_and_prints_its_output_before_the_result`, `repl::a_missing_run_file_reports_the_reference_clr_sentence`. |
| CLI-005 | Maintain engine state between commands. | `repl::state_persists_between_commands_and_underscore_tracks_the_last_result` (variable, function, `_`, `vars`), `repl::precision_knobs_apply_to_later_commands`. |
| CLI-006 | Match precision handling and `_` result behavior. | `repl::precision_knobs_apply_to_later_commands` (100-digit default, `set display 5`, then `set precision 7` replacing the override) and the `_` assertions in the state test. |
| CLI-007 | Match errors, cancellation, and output formatting. | `repl::errors_render_the_caret_and_the_session_keeps_accepting_input` (parse error with caret, runtime error without, recovery), `repl::cancellation_is_rendered_and_the_session_continues`, `repl::run_executes_...` (print output before result), `repl::exit_and_end_of_input_terminate_with_bye`. |

## 5. Gates

| Command | Observed |
|---|---|
| `cargo fmt -p lovelace-run -- --check` | exit 0 |
| `cargo clippy -p lovelace-run --all-targets -- -D warnings` | exit 0 |
| `cargo test -p lovelace-run` | exit 0 — 437 unit + 1 recursion + 11 REPL = 449 passed, 0 failed |
| `cargo test --workspace` | RED in the shared working tree on two crates modified by other in-flight work outside this scope: `lovelace-desktop::workbench::tests::stale_events_from_an_abandoned_run_are_discarded` (Stale != Cancelled) and `lovelace-know::convergence_tests::frozen_reference_metrics_match` (c1NewPlaneRate). Neither file is in this round's scope. |
| `cargo test --workspace` on pristine HEAD `ef17854` + this patch | PENDING_RESULT |
| `lovelace-corpus.exe` (release) | exit 0 — `corpus: 214 passed, 0 failed, 214 total` |
| `node parity/tools/gate-fixtures.mjs` | exit 0 — `oracle: 20/20`, `rust: 20/20` |
| `node parity/tools/gate-cli.mjs` | exit 0 — `CLI matrix: 16/16` (non-interactive RUN rows unchanged) |
| `node parity/diff/run.mjs --cases 200 --seed 1` | exit 0 — generated 200 match / 0 mismatch / 0 error; replayed 1,758 match / 0 mismatch / 0 error |

## 6. Remaining / caveats

- The only workspace-test failures observed belong to the two out-of-scope crates shown above and
  were present in files this round did not touch; the clean-HEAD verification row separates them
  from this patch.
- The interactive terminal path could not be driven end-to-end in this environment (no console).
  Its pure editor state machine and ANSI decoder are unit-tested; the terminal layer is a thin
  read/redraw loop over them. The pipe path is fully transcript-tested.
- Cancellation during an evaluation is expressed through `--cancel-after`; Ctrl+C at the prompt
  is exit intent, exactly as the reference `LineEditor` defines it.
