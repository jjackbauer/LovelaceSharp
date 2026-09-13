# Wave 6 — the closure wave after wave 5's P1, dispatched against the RE-PUBLISHED binary

Wave 6 exists because wave 5 (round 23) found **N-1 (P1)** with a strategy no earlier wave had used,
and the cycle's rule is that **a fix restarts the gate**: the new tests fail on a control tree, the fix
lands, the tree is re-published, and a **fresh** wave runs against that artefact. Nothing here may
re-run a persona's own probe list from wave 5.

## The artefact every persona gets (fill in from the publish, do not narrate it)

`C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe` — size, write time, age against the
newest non-generated source file, and `git diff --name-only HEAD -- '*.cs'` = 0 files. If any of those
four numbers is missing from a persona's report, the persona was not told which artefact it audited.

## Dispatch rules (unchanged from wave 4/5)

1. Published binary re-published from the final commit, with size + freshness quoted.
2. Known-open items and every closure this cycle are named up front, so a re-discovery is not counted new.
3. PowerShell 5.1 strips embedded double quotes from native argv (EVD-299): scripts with quotes go
   through `--file`/`--stdin`, or a `.bat` run with `cmd /c`.
4. A persona edits nothing but its deliverable; every finding needs two reproductions with the exact
   command and verbatim output; no fixes proposed; a "could not check" list is mandatory.
5. **Time-boxed**: each persona writes its deliverable as it goes and stops at the box. A partial report
   on disk beats a complete one in a stopped agent.
6. Do not re-publish, rebuild, or run the repo's suites while a wave is reading the binary.

## Persona Q — attack the N-1 repair (the fix is now itself a surface)

Strategy: the repair changed `Runner.SplitLines` from "remove every trailing terminator" to "remove
exactly one". Attack **that** decision where its own seven tests do not look:
* every other consumer of the captured text — `--text` (the `= result` line, the variable lines, the
  cancellation ledger line), `output[]` vs `partialOutput[]` on the cancelled and failing paths, the
  envelope's `hasOutput` ledger, Studio's two `SplitLines` copies (do the three implementations still
  agree on the same capture?);
* the boundary the fix creates: a print whose display form already ends in whitespace, a print of an
  empty string as the ONLY statement, `print()` twice, a statement that prints and then throws, a
  function whose body prints, a loop that prints on every iteration, prints interleaved with errors in
  a block, a script whose last statement is a `print` inside an `if` that does not run;
* the line-terminator shapes the fix distinguishes: a capture written with LF only, CRLF, a bare CR, a
  string containing a real newline, a script read from a CRLF file, a BOM file, `--stdin` from a pipe
  vs a file;
* and the invariant the defect was: for EVERY script in a generated corpus, does
  `count(timings.hasOutput == true)` agree with `output[].length` when each printing statement prints
  exactly one line? A single disagreement is a finding.
Deliverable: `docs/goal-cycle-6/round-23/audit-Q-n1-attack.md`.

## Persona R — the envelope as a consumer's state machine (reconstruction, not inspection)

Strategy: act as the CONSUMER the protocol is written for, and never read the product's source.
* Rebuild the print stream from `output[]` (+ `partialOutput[]` where it exists) and compare it with
  the raw bytes the process wrote to stdout and stderr, for a corpus that mixes `print`, `--text`,
  failures, cancellations and plots. Invariant 1 says stdout carries the envelope and nothing else: is
  every byte the process wrote accounted for by the envelope, and vice versa?
* Replay a session as a state machine: feed the envelope's own fields back into the next call
  (`revision`, `variables[].name`, a `SolveResult`'s `solutions[].value` re-substituted into the
  equation, a `classification`/diagnostic code driving a retry) and check that the document's promises
  hold for a consumer that trusts them — including the truncation flags, the `Truncated` field, and the
  claim that a consumer never parses a unit suffix.
* Check the document's own examples byte-for-byte against the binary again (they have drifted twice).
Deliverable: `docs/goal-cycle-6/round-23/audit-R-consumer.md`.

## Acceptance for D1

D1 is met only if the wave reports **no open P0/P1**, with the "could not check" lists read carefully
(an untested area is not a clean area). Any new P0/P1 restarts the loop: control-failing test, land,
re-publish, another fresh wave. **D3 remains the maintainer's**: it is open whatever this wave finds,
so A+ is not claimable this round unless the maintainer's words arrive.
