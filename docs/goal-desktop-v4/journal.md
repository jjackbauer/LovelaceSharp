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

### VAL-001: The intent audit falsified the contract's "no replacement" claim

- **Target**: `goal.md` v1 mechanism table (M1–M19, "Removed or replaced: none") and DEC-001..005
- **Method**: Read `intent-audit.md`; re-derived each of its six REPLACED/REMOVED rows against
  `goal.md` and `requirements.md`; opened the engine source for the WKS claim; read the gate script
  for the waiver and evidence-class claims.
- **Evidence examined**: `intent-audit.md` rows for ARC-001, CON-009, TST-011, TST-012, severity-1
  closure, human-waiver path; `requirements.md` §2 CON/ARC/TST rows; `rust/lovelace-lang/src/lib.rs`
  (no public remove/clear); `tools/desktop-gate.ps1` v1 (free-text `APPROVED_DEVIATION`, no class
  check, no dimension tally)
- **Result**: **Supported** for all six. Additionally: the DoD as written could never exit 0 because
  it required `p0_not_tested = p0_blocked = 0` while TST-012 was committed to BLOCKED and Linux to
  NOT_TESTED; and an agent-inferred constraint (C7, freeze the engine) excluded WKS-003/004, a
  user-named mechanism.
- **Conclusion**: Contract repaired in `goal.md` v2 — two verdicts (`-Readiness`, `-Verify`),
  per-row evidence classes, `gate/waivers.json` with `owner: user`, `gate/defects.json`, C7 amended to
  allow tested engine extensions, `platform-matrix.md` published, M20/M21 added. DEC-006 records the
  user's scope decision. The gate was rebuilt and re-verified (EVD-028).
- **Related**: EVD-026, EVD-028, DEC-006, RISK-005

### DEC-006: User-approved Windows-only v1 with the TST-012 usability study waived

- **Decision**: v1 is released on Windows only. TST-012 (>=5 target users, 8 tasks) is waived for v1
  and stays `BLOCKED`, never `PASS`. The Linux half of ARC-001 and the "other supported platform"
  clause of TST-011 are deferred with the platform scope. The waiver is recorded in
  `gate/waivers.json` and the platform matrix in `platform-matrix.md`.
- **Owner**: user — **Status**: approved
- **Rationale**: the user answered the scope question with "Waive both now: approve a Windows-only v1
  without the 5-user study". The v4 release rule explicitly allows a P0 to be "passing **or
  human-waived**", so this is a contract-compliant deviation, not a narrowing by the agent.
- **Alternatives considered**: leaving both `BLOCKED` with a READY_FOR_WAIVER verdict (rejected by the
  user, who wants the release route open); keeping the release permanently blocked (also rejected).
- **Consequences**: the final verdict may be RELEASE without human usability evidence; every report
  must say so explicitly, and TST-013 (SUS >= 80) is not claimed.
- **Related**: TST-012, TST-013, ARC-001, TST-011, CON-009, EVD-028

### OBS-003: The headless core is synchronous and projects strings, not structured values

- **Source**: `rust/lovelace-desktop/src/app.rs:145-167,592-610`;
  `rust/lovelace-desktop/src/engine.rs:45-81`
- **Fact**: `Command::Run` only snapshots a `RunRequest`; `run_blocking` evaluates inline on the
  caller's thread; `RunOutput` carries `Vec<(String, String)>` variables and
  `Vec<(String, Vec<String>)>` functions, so the UI can never see a value's structure. `Evaluator`
  has no cancellation token.
- **Implications**: ARC-005/008/010, RUN-004/005/006 and RES-001 all require replacing the execution
  and projection layer, not decorating it.
- **Confidence**: High — **Agent**: Observer "core" (census-core.md, 238 OBS) — **Verified by**:
  EVD-025, EVD-006
- **Related**: EVD-025, RUN-006, ARC-010

### OBS-004: The web reference delivers plot SVG and structured results inside the run payload

- **Source**: `Lovelace.Studio/wwwroot/app.js:185-192,212-231,263-339`;
  `Lovelace.Studio/EngineHost.cs:270-272`
- **Fact**: the poll response carries `plot: {svg, title}` and a `result.structured` tree; the client
  sets the SVG into the DOM and renders records/arrays/symbolic fields from the structured tree. It
  never calls `/api/symbolic/inspect`; the inspector is fed only from the run result.
- **Implications**: behavioural parity needs the desktop to consume the engine's SVG string and the
  structured projection directly — both already exist in `lovelace-lang` (`last_plot`,
  `variable_values`, `structured::project_value`). The desktop's temp-file plot path is an
  implementation regression, not a web behaviour.
- **Confidence**: High — **Agent**: Observer "web" (census-web.md, 107 OBS, 23 capabilities) —
  **Verified by**: EVD-023, EVD-024
- **Related**: EVD-023, EVD-024, PLT-002, RES-001, SYM-001

### OBS-005: The desktop SVG parser covers two of the elements the engine emits

- **Source**: `rust/lovelace-desktop/src/svg.rs:111-200`
- **Fact**: only `<text>` and `<polyline>` are parsed; `<rect>`, `<line>`, `<path>`, transforms,
  `fill`, `stroke-width` and `clipPath` are not read. `svg.rs` contains no painter; GDI drawing lives
  elsewhere.
- **Implications**: PLT-001/PLT-002 require a real renderer (resvg/tiny-skia), not an extension of
  this parser.
- **Confidence**: High — **Agent**: Observer "render-engine" (census-render-engine.md, 117 OBS) —
  **Verified by**: EVD-027
- **Related**: EVD-027, PLT-001, PLT-002

### OBS-006: The current window exposes no accessibility tree, no DPI handling and no theme integration

- **Source**: `rust/lovelace-desktop/src/win32/shell.rs`, `ffi.rs` (whole files)
- **Fact**: no `WM_GETOBJECT`, `IAccessible`, UIA provider, DPI-awareness call, `WM_CTLCOLOR`,
  `SetWindowTheme` or `DwmSetWindowAttribute` appears in either file; colours come from the
  `COLOR_WINDOW` system index; the status bar writes only part 0.
- **Implications**: A11Y-002/003/004/006 and VIS-002 require a stack with an accessibility tree and
  theming. The eframe probe already demonstrates such a tree (EVD-021).
- **Confidence**: High — **Agent**: Observer "gui" (census-gui.md, 322 OBS) — **Verified by**: grep
  of both files for those tokens (no matches) plus EVD-021
- **Related**: EVD-021, A11Y-002, VIS-002

### OBS-007: The web Studio has no tabs; its session is the browser tab

- **Source**: `Lovelace.Studio/wwwroot/app.js:5-8,31-63`; `index.html`
- **Fact**: the client stores one `sessionId` in `sessionStorage` and has no tab strip; "one engine per
  browser tab" is the session boundary. Pane minimize/maximize exist and are not persisted.
- **Implications**: DOC-001 desktop tabs are a deliberate desktop improvement, and LAY-005/006 must
  preserve the web's minimize/maximize semantics while adding persistence.
- **Confidence**: High — **Agent**: Observer "web" — **Verified by**: census-web capability table and
  the absence of tab code in app.js
- **Related**: DOC-001, LAY-005, LAY-006

### RISK-005: HEAD moves under this run and the engine is being edited concurrently

- **Risk**: the other session committed `420000f` mid-run and still holds uncommitted edits in
  `rust/lovelace-lang/src/{builtins.rs,syntax.rs}`. Evidence recorded at one HEAD can be invalidated
  by a later engine change, and a strict clean-tree release gate cannot pass while those edits are
  outstanding.
- **Likelihood**: High — **Impact**: High
- **Evidence**: EVD-019, `git status` showing the other session's modified engine files
- **Mitigation**: gate provenance is ancestor-bound (proof produced at an ancestor commit stays
  valid); each round records HEAD at start and end; this run commits only its own paths; the final
  release run must happen at a commit where the other session is clean and the tracked evidence
  hashes still match. If that cannot be achieved, the verdict is READY_FOR_WAIVER, not RELEASE.
- **Gate**: G1

### OBS-008: The parity register is real but 42 IDs still have no owning feature row

- **Source**: `round-1/parity-register.md`
- **Fact**: 49 web capability rows (7 PRESENT / 10 PARTIAL / 20 MISSING / 12 DIFFERENT) and 24
  desktop-only rows, each with a web citation, the desktop as-built code, owning requirement IDs and
  a planned GUI test. 42 IDs are UNMAPPED: CON-001/004/005/007/008/010, ARC-001, EDT-016/017,
  RUN-016, INC-009/011, PLF-007/010, 11 PERF rows and all 18 TST rows.
- **Implications**: CON-002 cannot pass while any ID lacks an owner. The unmapped groups are mostly
  process/quality requirements whose owner is the gate itself rather than a feature row; the register
  must say so explicitly (a `gate-owned` owner) before CON-002 is claimable.
- **Confidence**: High — **Agent**: Synthesizer "parity register" — **Verified by**: EVD-039, EVD-040
- **Related**: EVD-039, EVD-040, CON-002, CON-007

### TODO-003: Make the register account for every requirement ID

- **Task**: Give each of the 42 UNMAPPED IDs an explicit owner — either a feature row or the gate row
  that certifies it (for example TST-016 owns the release manifest; PERF rows are owned by the
  performance gate under CON-008) — so CON-002 can be audited without omissions.
- **Priority**: P0
- **Depends on**: the first GUI rows existing, so gate-owned rows can point at real tests
- **Status**: **Done** (round 7) — `parity-register.md` section 5 resolves all 43 IDs as feature-owned
  or gate-owned, with a count check: 43 + 186 = 229.
- **Related**: CON-002, EVD-040

### OBS-009: The execution core is written but red

- **Source**: `rust/lovelace-desktop/src/statements.rs:102,198`; `rust/lovelace-desktop/src/execution.rs`;
  `cargo test -p lovelace-desktop`
- **Fact**: the modules exist (7 297 B and 40 519 B) but the test run reports 67 passed / 14 failed.
  Every slicer test fails because `split()` returns an empty vector for a normal three-statement
  source, and the nine execution tests fail downstream of it.
- **Implications**: round 3 has produced code but no verified capability. ARC-005 (no evaluation on
  the UI thread), RUN-001/006 and the typed event stream remain NOT_TESTED. The round is a gate
  failure on G4 until the tests are green and re-run by the orchestrator.
- **Confidence**: High — **Agent**: none; orchestrator's own run — **Verified by**: EVD-047
- **Related**: EVD-047, ARC-005, RUN-001, RUN-006

### RISK-006: A round can consume its whole budget without a verified result

- **Risk**: the round-3 implementer wrote the modules and then went quiet while the tests were red;
  the round nearly ended with code that could not be trusted and no report.
- **Likelihood**: Medium — **Impact**: High (a red artifact presented as delivered would be phantom
  completion)
- **Evidence**: EVD-047
- **Mitigation**: the orchestrator re-runs every test command before recording a row; a red test run
  is recorded as FAIL/NOT_TESTED, never as delivery; the round's objective is re-dispatched with the
  observed failure output rather than the same prompt.
- **Gate**: G4

### OBS-010: The execution core is green but not yet driving any window

- **Source**: `rust/lovelace-desktop/src/execution.rs`, `statements.rs`; `cargo test -p lovelace-desktop`
- **Fact**: 81 tests pass (0 failed). The core spawns a worker per run, emits typed sequenced events
  with document/run/revision identity, cancels through a token checked between statements, gates
  stale revisions in the sink, isolates panics and preserves the committed snapshot. Submit returns in
  154 µs and a cancel reaches a terminal event in 20.21 ms. Neither module references a window handle.
- **Implications**: the architectural invariants (ARC-006/007/008) and the cooperative cancel are now
  demonstrable headlessly, but no P0 row may be marked PASS on that basis: ARC-005 and RUN-001/009 are
  about what the *shipped window* does. The gap is integration, not correctness.
- **Confidence**: High — **Agent**: repair implementer — **Verified by**: EVD-052, EVD-053
- **Related**: EVD-052, EVD-053, ARC-005, ARC-006, ARC-008, RUN-001

### TODO-004: Wire the core into the product window

- **Task**: make `lovelace-desktop` open the new egui workbench by default, run the real engine through
  `ExecutionHost` on a worker, and consume the typed event stream; keep the Win32 shell behind an
  explicit flag until parity.
- **Priority**: P0
- **Depends on**: the green execution core (done) and the spike modules
- **Status**: Open — round 5 objective
- **Related**: OBS-010, ARC-003, ARC-005, RUN-001, RUN-004, RUN-009

### RISK-007: The product writes plot files into the current working directory

- **Risk**: the new shell sets the engine's plot directory to `<cwd>\lovelace-plots`, so running the
  app from the repository root creates an untracked directory there and, on an installed system,
  would attempt to write next to the binary (PLF-006 forbids that by default).
- **Likelihood**: Certain (observed) — **Impact**: Medium
- **Evidence**: EVD-055, `round-5/uia-tree.txt:51`, `round-5/product-typed.png`
- **Mitigation**: add `lovelace-plots/` to `.gitignore` now; round 6 must move the plot directory to
  an app-scoped location (`%APPDATA%\Lovelace\plots`), create it explicitly, surface write failures,
  and delete the repository copy.
- **Gate**: G1 (PLF-006 evidence must come from the fixed build)

### TODO-005: Move the engine plot directory out of the working directory

- **Task**: resolve the plot directory from the app-scoped data directory, create it with error
  reporting, use it for `Engine::set_plot_dir`, and remove the accidental `<repo>\lovelace-plots`.
- **Priority**: P0 (PLF-006 is P0)
- **Depends on**: nothing
- **Status**: **Done** (round 7) — the shell resolves `%APPDATA%\Lovelace\plots`, logs it, refuses
  with an explicit message when unwritable, and the repository copy is deleted. PLF-006 is PASS with
  a scope note: this session's child-process sandbox denies the runtime write (EVD-058).
- **Related**: EVD-055, EVD-058, RISK-007, RISK-008, PLF-006

### OBS-011: Round 7 delivered the run surface, with two honest gaps

- **Source**: `round-7/hardening.md`; `--gui-selftest` (EVD-056, EVD-057)
- **Fact**: 105 tests pass; the headless acceptance run reports Ctrl+Enter → `run Running` with Run
  disabled and Cancel visible, Esc → `run Cancelled` in 823 ms, a long run whose submit returns in
  153 µs while the event loop processed 183 frames in 300 ms, and a cancel terminal in 492 ms after
  2 of 13 statements. The plot checks fail only because `%APPDATA%` writes are denied to child
  processes in this session.
- **Gaps**: (a) cancellation is only observed at statement boundaries — the implementer measured a
  single `sqrt(2)` at precision 200 000 taking 29 721 ms with no sub-2 s cancellation, so RUN-006 and
  ARC-009 are **not** met for kernel-bound work; (b) no real-window artifact yet proves VIS-010 or
  RUN-009, because UIA `ValuePattern` is unsupported on the editor and foreground ownership is lost
  between separate tool calls, so the keystroke path could not be driven reliably in this session.
- **Implications**: the row set stays open for VIS-010/RUN-009 until a single-process driver holds the
  foreground for the whole gesture, and RUN-006/ARC-009 need a real cancellation token inside kernels
  (the engine already polls a thread-local deadline — the live flag is the missing piece).
- **Confidence**: High for the headless numbers and the APPDATA denial; Medium for the 29.7 s figure
  (agent-reported, not re-measured by the orchestrator)
- **Related**: EVD-056, EVD-057, EVD-058, RUN-006, ARC-009, VIS-010, RISK-008

### RISK-008: This session cannot certify app-scoped file writes from the product binary

- **Risk**: the sandbox denies child processes writes outside the workspace (EVD-013 for `%TEMP%`,
  EVD-058 for `%APPDATA%`), so requirements whose acceptance is a successful app-scoped write
  (PLF-006 runtime half, PLT-009 plot persistence, DOC-005 recovery files) can only be certified
  statically here, and a naive run makes plot scripts fail with a visible error.
- **Likelihood**: Certain — **Impact**: Medium
- **Evidence**: EVD-013, EVD-058
- **Mitigation**: record the environment condition in every affected row's `scope_note`; keep the
  code path app-scoped and explicitly error-reporting; capture the runtime half on a host where the
  denial does not apply, or via a single-process launcher that owns the foreground/env.
- **Gate**: G1 for the affected rows

### OBS-012: The single-process gesture runner did not produce evidence (round 8 attempt)

- **Source**: `tools/gui-harness.ps1 -Action scenario`; `round-8/scenario.log`
- **Fact**: the scenario action launched the product window, typed 134 characters, sent Ctrl+Enter,
  then sampled UI Automation for 9 s and recorded **0 status states**; the final capture was a
  **136-byte PNG** (effectively empty). No run was observed and no screenshot of a running state was
  produced. The window was closed. Artifacts: `round-8/scenario.log`, `round-8/scenario-final.png`.
- **Implications**: VIS-010 and RUN-009 remain NOT_TESTED. The two candidate causes are (a) a stale
  UIA root taken before the window finished creating its tree, and (b) a zero-size window rect making
  the screen capture empty. Neither is proven; the next attempt must assert the window rect is
  non-zero and re-acquire the UIA root after a state change before sampling.
- **Confidence**: High for the observed failure; Low for either cause
- **Related**: VIS-010, RUN-009, RISK-008
