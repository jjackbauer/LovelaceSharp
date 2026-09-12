# Wave 4 — fresh adversarial audit against the FINAL binary (dispatch-ready)

Wave 4 exists because of OBS-022/OBS-024: **a closure wave is not a substitute for a fresh audit.** The
per-round finding rate of new P0/P1s has been 1–2 per wave even against a tree whose previous wave had
been fully closed, so D1 is only claimable if a *new* strategy set, run against the binary published from
the final commit, comes back empty.

## Dispatch rules (learned, do not drop)

1. Every persona gets the PUBLISHED binary `out/aot/Lovelace.Run.exe` re-published from the final commit,
   plus its byte size and freshness, so a stale artifact cannot masquerade as the product (EVD-296).
2. Every persona is told about the known-open items and the two closures of wave 3, so a re-discovery is
   not counted as new.
3. Every persona is told the PowerShell trap: **PS 5.1 strips embedded double quotes from native
   arguments**, so `--eval 'symbol("x")'` becomes `symbol(x)` and fails for the wrong reason (EVD-299).
   Anything with quotes goes through `--file`/`--stdin`, or through a `.bat` run with `cmd /c`.
4. Personas may not edit anything except their own deliverable; findings need a command and verbatim
   output; P0 = wrong value or abort on valid input, P1 = false claim in the machine API / wrong refusal /
   data the API says it carries being lost or altered, P2 = cosmetic or documentation.

## Persona J — protocol conformance (the documents as the oracle)

Strategy: read `docs/symbolics/dsh-protocol.md` (Invariants 1–6, the value forms, the Diagnostic form,
the worked solve envelope, the error envelope, the CI section) and `--help`, then treat **every normative
sentence as a testable claim about the binary**. No CLAUDE-style assumption: if the document says
"stdout carries the envelope and nothing else", "an absent field is {kind:Null}, distinct from an empty
string", "a consumer never parses a unit suffix", "the statement that FAILED is still a timed statement
reported with resultKind Void", "exit codes 0/1/2", "the two structural durations are present on the error
path too, including for a parse error where timings is present and empty" — each is a probe with a
pass/fail. Also check the *examples*: every JSON block in the document must be reproducible against the
live binary (the document's own `solve` example already failed once — a recorded P2 — so examples are
fair game). Deliverable: one row per normative sentence, PASS/FAIL/UNTESTABLE, with the command.

## Persona K — library/embedding API (not the CLI)

Strategy: every earlier wave went through `Lovelace.Run`. This one writes a small C# program that
references the PRODUCT assemblies (`Lovelace.Symbolics`, `Lovelace.Real`, `Lovelace.Complex`,
`Lovelace.Suite`) and uses them the way an embedder would: create one engine, run many evaluations in one
process, share the engine across threads, reuse a cancelled evaluation, dispose and re-create, mix
precisions (`setprecision`) across calls, and check for state leaking between calls — a value that
depends on the process's call history is a P0 exactly like H-1 was. Also check thread-safety claims and
whether the public API's documented shapes hold outside the CLI envelope. Deliverable: the program, its
output, and each divergence as a finding.

## Persona M — composed programs and post-failure state

Strategy: all earlier waves probed single calls. This one builds *programs* out of parts that are each
individually verified (nested function definitions, recursion under budget, arrays and loops, a print
before a failure, a plot that fails, a solver after a cancellation, a second evaluation in the same
process after an error) and checks the GLOBAL invariants: the values that succeeded before a failure must
not change, a failure must not corrupt later statements in a later run, partial results must be labelled
partial, and repeating the same program in one process must equal running it once per process. Distinct
from the metamorphic wave because the unit of the test is the program, not the expression.

## Acceptance for D1

D1 is met only if the three personas together report **no open P0/P1** against the binary published from
the final commit, with their "could not check" lists read carefully (an untested area is not a clean
area). Any new P0/P1 restarts the loop: close it with a control-failing test, re-publish, and run another
fresh wave.
