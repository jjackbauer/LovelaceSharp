# Goal Journal — desktop-v4

> Append-only. See the stretch-goal-harness evidence protocol for entry format.

---

<!-- New entries go below this line -->

### DEC-001: Goal directory and authoritative catalog

- **Decision**: This run's memory lives in `docs/goal-desktop-v4/`. `requirements.md` is the verbatim
  transcription of the user's pasted 229-requirement contract; `goal.md` is the contract, definition
  of done and constraint record. Requirement IDs may not be renumbered or re-worded.
- **Owner**: agent (process) — **Status**: approved
- **Rationale**: EVD-002 — the catalog parses to exactly 229 rows (171 P0 / 56 P1 / 2 P2) and 22
  journeys, so the transcription is complete and machine-usable for the gate.
- **Alternatives considered**: pointing at the pasted chat text (not durable, not machine-readable);
  using `.github/requirements/Lovelace.Rust.Desktop.md` (older, weaker doc that the v4 contract
  explicitly supersedes and whose egui decision the v4 doc reopens).
- **Related**: EVD-002, CON-001, CON-002

### DEC-002: Frozen baseline is `fad305c` plus a recorded dirty set

- **Decision**: Every parity claim in this run names commit
  `fad305c1852be469f2bc365f7ae5948b76b97bcb`. The 9 uncommitted paths present at start (from a
  concurrent prior session) are excluded from this run's baseline and treated as read-only.
- **Owner**: agent — **Status**: approved (records a fact; does not change the goal)
- **Rationale**: EVD-001 (HEAD and dirty list) and EVD-012 (the dirty set is another live session's
  work, not a stable reference).
- **Alternatives considered**: pinning the working tree as-is (unreproducible); waiting for the other
  session to finish before starting (no guarantee it will, and the reference surfaces are unmodified).
- **Related**: CON-001, RISK-001

### DEC-003: The gate is a fail-closed script, not a document

- **Decision**: Acceptance is declared only by `tools/desktop-gate.ps1 -Verify` over
  `docs/goal-desktop-v4/gate/register.json` (229 rows) and `gate/journeys.json` (22 rows). Status
  vocabulary is exactly `PASS | FAIL | BLOCKED | NOT_TESTED | APPROVED_DEVIATION`. A `PASS` row needs
  hash-bound evidence; a stale hash or a dirty tree fails the gate.
- **Owner**: agent — **Status**: approved
- **Rationale**: EVD-003 — on an unevidenced register the gate exits 1 with 194 blocking problems
  rather than defaulting to green. That is the CON-005/TST-017 behaviour the contract demands.
- **Alternatives considered**: trusting a markdown scorecard (cannot detect stale or missing proof);
  computing the verdict inside a subagent (violates the orchestrator-writes-truth rule).
- **Related**: EVD-002, EVD-003, CON-005, TST-016, TST-017

### DEC-004: Windows is the initial release target; Linux/macOS are later ports

- **Decision**: Certification in this run is Windows-only. The release matrix marks Linux and macOS
  as later ports, unsupported until a built and run GUI test exists there.
- **Owner**: user (CON-009 explicitly permits "marking initial release and later ports") — **Status**:
  approved by the contract text, not by an agent preference
- **Rationale**: EVD-008 and EVD-010 are Windows measurements; C6 forbids claiming an untested
  platform. No Linux or macOS host is available to this run.
- **Alternatives considered**: claiming cross-platform support from cross-compilation alone — rejected
  because CON-009 requires a *run* GUI test and PLF-010 requires platform behaviour tests.
- **Related**: CON-009, PLF-009, PLF-010, C6

### DEC-005: Build isolation from the concurrent session

- **Decision**: Every cargo invocation in this run uses
  `CARGO_TARGET_DIR=rust/target-desktop` and `TMP/TEMP=rust/target-desktop/tmp`, with
  `RUSTUP_HOME`/`CARGO_HOME` pointed at the repo-local `.toolchain`.
- **Owner**: agent — **Status**: approved
- **Rationale**: EVD-012 (a second session runs `cargo test --workspace` against the shared target
  directory) and a first link attempt that failed `LNK1104` on a file under `%LOCALAPPDATA%\Temp`.
- **Alternatives considered**: sharing `rust/target` (blocks on the other session's lock and risks
  cross-contamination of artifacts); waiting for the other session (unbounded).
- **Related**: EVD-012, RISK-001

### RISK-001: A concurrent session is mutating the same repository

- **Risk**: Another DSH session is running `cargo test --workspace` and has modified
  `rust/lovelace-lang/src/builtins.rs` and `rust/lovelace-run/*`. Engine behaviour could change
  mid-round, builds can invalidate each other, and edits can collide.
- **Likelihood**: High — **Impact**: Medium
- **Evidence**: EVD-001, EVD-012
- **Mitigation**: isolated target dir; treat its paths as read-only (C7); record HEAD at the start and
  end of every round; gate binds PASS rows to the HEAD they were produced against; never revert its
  work.
- **Gate**: —

### RISK-002: TST-012 (human usability sessions) cannot be produced by this run

- **Risk**: TST-012 requires at least five target users completing eight scripted tasks. No human
  subjects are available to an autonomous agent run, so this P0 row will remain BLOCKED and the
  release gate cannot reach a full green verdict without a human waiver.
- **Likelihood**: Certain — **Impact**: Medium (it blocks the final release claim, not the build-out)
- **Evidence**: `requirements.md` TST-012; goal.md "Known resource limits"
- **Mitigation**: keep the delivery path green for every row that machine evidence can carry; report
  TST-012 as BLOCKED with the missing evidence named; never relabel an agent review as a user session.
- **Gate**: G3 (coverage) at convergence

### RISK-003: 171 P0 rows against a 40-round cap

- **Risk**: The contract cannot fully converge within the round cap; a late "done" claim would be
  phantom completion.
- **Likelihood**: High — **Impact**: Medium
- **Evidence**: EVD-002 (171 P0 rows); the desktop baseline is a Win32 EDIT shell (EVD-004..006).
- **Mitigation**: order rounds by gate dependency (spike → core → shell → semantics → inspection →
  certification); report coverage after every round; stop at the cap with an honest tally rather than
  declaring convergence.
- **Gate**: G3

### OQ-001: Who waives TST-012?

- **Question**: Does the product owner waive TST-012 (five target users, >=90% task success) for this
  release, or must the release remain blocked until human usability sessions are run?
- **Needed evidence**: an explicit user decision recorded as a `DEC` with `Owner: user`,
  `Status: approved`, or the results of five real usability sessions.
- **Priority**: P0 blocking (release-gate only; development continues)
- **Raised by**: Phase 0 (orchestrator)
- **Related**: RISK-002, TST-012

### TODO-001: Synthesize the round-1 census into the parity register

- **Task**: Merge the five observation artifacts under `round-1/` into a current-state column in
  `gate/register.json` (status stays NOT_TESTED until a test exists) and a `parity-register.md` that
  satisfies CON-002's owner/source/status/test/waiver columns.
- **Priority**: P0
- **Depends on**: round-1 observer artifacts verified by the orchestrator
- **Status**: Open
- **Related**: CON-002, CON-003, CON-007

### TODO-002: Run the ARC-001 framework/editor spike

- **Task**: Build and measure a native editor prototype against real editor, IME, accessibility
  (UIA), SVG and docking scenarios, then record the framework decision with its measurements.
- **Priority**: P0
- **Depends on**: TODO-001 (baseline census) and the gate skeleton
- **Status**: Open
- **Related**: ARC-001, EDT-001, ARC-002, A11Y-002

### OBS-001: The desktop has no GUI, screenshot or integration test infrastructure

- **Source**: `.github/workflows/ci.yml:1-414`; `rust/lovelace-desktop/src/selftest.rs:26`;
  `rust/lovelace-desktop/tests` (absent)
- **Fact**: The only workflow has three `ubuntu-latest` .NET jobs and no Rust or desktop job. The
  desktop's only automated acceptance surface is a fixed 15-check in-process `--selftest`; the crate
  has no `tests/` directory and no `[dev-dependencies]`.
- **Implications**: TST-002 (native GUI e2e), TST-003 (screenshot regression), TST-004 (real
  cancellation), TST-005 (concurrent tabs), TST-011 (screen reader) and TST-014 (DPI) have no
  infrastructure at all. They are not "not yet run"; they are unbuilt.
- **Confidence**: High — **Agent**: Observer "tests" (census-tests.md) — **Verified by**: EVD-018
- **Related**: EVD-018, TST-002, TST-003

### OBS-002: The `cancel` selftest is a state-machine test, not a cancellation test

- **Source**: `rust/lovelace-desktop/src/selftest.rs:207-242`
- **Fact**: `check_cancel` inserts `1 + 1`, runs, cancels, then hand-builds a stale `RunOutput` and
  asserts it is ignored. No worker thread and no engine loop is started or interrupted.
- **Implications**: RUN-006 ("cancelling a real long-running script ends work cooperatively") and
  TST-004 are currently unproven; a green `--selftest` says nothing about cancellation latency.
- **Confidence**: High — **Agent**: Observer "tests" — **Verified by**: EVD-016
- **Related**: EVD-016, RUN-006, TST-004

### RISK-004: The default temp directory is not writable for child processes in this environment

- **Risk**: A plain Rust process cannot create a directory under `%LOCALAPPDATA%\Temp` (os error 5),
  although PowerShell can. The engine writes plots under `%TEMP%\lovelace-plots` and swallows the
  `create_dir_all` error, so the visible symptom is "the engine produced no plot" rather than a
  permission error.
- **Likelihood**: High (reproduced) — **Impact**: Medium
- **Evidence**: EVD-013, EVD-014, EVD-015, EVD-017
- **Mitigation**: every build/test harness in this run sets `TMP`/`TEMP` to
  `rust\target-desktop\tmp`; the product should move plot delivery off ambient temp files to an
  app-scoped directory (PLF-006) and surface write failures (PLT-008).
- **Gate**: —

### OQ-002: Why does this environment deny temp-directory creation to child processes?

- **Question**: Is the denial caused by the DSH sandbox, by an ACL/mandatory-label on
  `%LOCALAPPDATA%\Temp`, or by an image-specific policy? The workaround is known; the cause is not.
- **Needed evidence**: sandbox policy documentation, or an ACL/mandatory-integrity dump of the temp
  directory, or a reproduction outside the harness.
- **Priority**: P2 curiosity (workaround is in place)
- **Raised by**: Round 0 (orchestrator)
- **Related**: EVD-013, RISK-004
