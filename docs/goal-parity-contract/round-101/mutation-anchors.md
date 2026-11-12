# Round 101 — re-anchor two stale mutations (GATE-003, GATE-008)

Status: **complete — 12/12 detected, 12 reverted trees pass**.

Run against HEAD `9e8f874` with a quiet tree: the desktop-lint lane had finished, its newest
`rust/` write was 18:58:55 local, and that timestamp did not move during the run. The manifest was
regenerated and green immediately before the run, and it was green again after it.

## 1. Why the gate read 9/12

| Mutation | Cause of the stale anchor | Evidence |
|---|---|---|
| GATE-003 | `line_col` was rewritten and moved out of `main.rs` into `ScriptSource::line_column` in `rust/lovelace-run/src/script_source.rs`; the old inline `upto` clamp no longer exists. | old literal occurred **0** times in `main.rs`; `main.rs` no longer contains `let upto`. |
| GATE-008 | The fault site still exists in `success_envelope`, but `main.rs` is checked out/stored with **CRLF** while the anchor was written with LF; a literal multi-line match therefore failed. | old LF block occurred **0** times; the same block with CRLF occurs **1** time. |
| GATE-011 | Its precondition is a green manifest, and the inventory was stale relative to the current `rust/` sources (the desktop-lint lane had edited 20 files). | `gate-manifest` reported `rust-tests.json is stale` before regeneration; `failures: []` after. |

## 2. Anchor changes

Both anchors were re-measured before writing, with a literal substring count **and** the same
regex-escaped count the harness uses. Both new anchors occur exactly once.

| ID | Old anchor | New anchor | Old count | New count |
|---|---|---|---|---|
| GATE-003 | `rust/lovelace-run/src/main.rs` :: `let upto = units.len().min(position);` → `...position + 1` | `rust/lovelace-run/src/script_source.rs` :: `        (line, (source_offset.saturating_sub(line_start)) + 1)` → `... + 2` | 0 | **1** |
| GATE-008 | `rust/lovelace-run/src/main.rs` :: `    let output = engine.take_output();\n    fields.push(("output", Json::str_array(output)));\n    fields.push((\n        "variables",` (LF block) | same block in `success_envelope` with **CRLF** separators; replacement adds `let mut output` + `output.extend(output.clone())` (CRLF) | 0 | **1** |

GATE-003's precise new mutation:

```json
"find": "        (line, (source_offset.saturating_sub(line_start)) + 1)",
"replace": "        (line, (source_offset.saturating_sub(line_start)) + 2)"
```

Calls `ScriptPositions::line_column(17)` (the `error_envelope` diagnostic) return column 18 today;
the mutation makes it 19 against the frozen golden's 18.

GATE-008's precise new mutation duplicates the captured vector between `take_output` and the
`"output"` field, exactly at the old site. The two `let output = engine.take_output();` lines
(`success_envelope`, `failure_envelope`) are individually non-unique, which is why the full
success-envelope block — followed by `"variables"` — is the anchor. **The JSON values contain
literal `\r\n` escapes because `main.rs` is CRLF**; do not re-normalise that anchor to LF.

## 3. GATE-003 label check

The authoritative label is already correct: `docs/parity/contract-v2.md` §12.6 says
“GATE-003: Break a Unicode source position”, and `parity/mutations.json` titles it “a Unicode-aware
source position is off by one column”. The re-anchored mutation edits the Unicode/UTF-16 column
computation, so the label matches the mutation. Nothing was relabelled.

The “truncation at a digit cap” characterisation in the task brief was inaccurate for GATE-003:
that fault class belongs to GATE-007 (fixed-decimal truncation) and GATE-010 (evalf decimal-place
cap), both still anchored to their own sites. GATE-003 was re-derived from its recorded fault class
(off-by-one Unicode column), not from a digit cap.

## 4. Inventory precondition (green before the run)

```
powershell -NoProfile -ExecutionPolicy Bypass -File parity/gen-rust-tests.ps1
  crates=11 tests=5667 suiteExit=0 passed=5666 failed=0
  sourcesSha256 = 7a8a2865ddc92b2b51455c1b701e39800d6810a77adf79f0f9131138dc686a77

node parity/gen-manifest.mjs
  rows 6209, ported 6090, excluded 119, coverage 98.08%, rustTests 5667

node parity/tools/gate-manifest.mjs
  failures: []
  PASS (structural): 6209 test cases dispositioned; 6090 claim Rust coverage (98.08%).
```

The manifest gate was then green again immediately after the mutation run.

## 5. Full run

Command (exit 0):

```
powershell -NoProfile -ExecutionPolicy Bypass -File parity/mutate.ps1
```

Summary line:

```
mutations: 12/12 detected; 12 reverted trees pass
```

Every per-mutation row, verbatim:

| ID | Detected | Restored | Detail |
|---|---|---|---|
| GATE-001 | yes | yes | `cargo-test:the_reference_truncates_a_period_that_closes_exactly_at_the_cap exit 101` |
| GATE-002 | yes | yes | `corpus:lang/020 exit 1` |
| GATE-003 | yes | yes | `fixtures:error_envelope exit 1` |
| GATE-004 | yes | yes | `fixtures:symbolic exit 1` |
| GATE-005 | yes | yes | `fixtures:error_envelope exit 1` |
| GATE-006 | yes | yes | `fixtures:symbolic exit 1` |
| GATE-007 | yes | yes | `corpus:sym/080 exit 1` |
| GATE-008 | yes | yes | `fixtures:print_purity exit 1` |
| GATE-009 | yes | yes | `cli:RUN-008 exit 1` |
| GATE-010 | yes | yes | `cargo-test:the_smallest_over_cap_request_is_marked_too exit 101` |
| GATE-011 | yes | yes | `manifest exit 1` |
| GATE-012 | yes | yes | `diff-empty exit 2` |

GATE-012 is an `invoke` mutation: the invocation is the fault and there is no tree to mutate or
restore, so its “restored” is the harness's vacuous count and is disclosed here rather than hidden.

## 6. Restored-tree verification

| Check | Result |
|---|---|
| All 10 source anchors re-counted after the run | each occurs exactly once (mutations reverted) |
| `git diff` over `rust/lovelace-run`, `lovelace-lang`, `lovelace-real`, `lovelace-symbolics` | empty |
| Newest `rust/lovelace-desktop/src` write during the run | unchanged at 18:58:55 (tree quiet) |
| `node parity/tools/gate-manifest.mjs` | PASS, `failures: []` |

One earlier full run was started before the desktop lane was quiesced and was killed at GATE-003;
the single applied mutation was reverted byte-precisely by hand (its line re-counts as 1), and that
run is **discarded**. Only the clean run above is reported.

## 7. Scope

Changed: `parity/mutations.json` (GATE-003 target + find/replace, GATE-008 find/replace) and the
generated `parity/rust-tests.json` / `parity/manifest.toml`. No `rust/` source, test, fixture,
`.cs` or documentation outside this report was edited.
