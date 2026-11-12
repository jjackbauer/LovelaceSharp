# Intent audit — Lovelace Desktop v4.0

**Auditor:** independent Intent Auditor; this is the first sight of this goal.
**Request:** the user's pasted v4.0 contract, transcribed as `docs/goal-desktop-v4/requirements.md`
(229 rows; 171 P0 / 56 P1 / 2 P2; 22 journeys — all verified by count from the file).
**Proposal:** `docs/goal-desktop-v4/goal.md`; its definition of done names
`tools/desktop-gate.ps1 -Verify`, which exists and is executable. At audit time `gate/gate.json`
reports `FAIL`, 0/171 P0 PASS, 0/22 journeys PASS, 171 P0 `NOT_TESTED`; this audit compares the
contract with what that DoD will accept, not with current progress.
**Sources inspected:** `Lovelace.Studio/README.md` (full), `wwwroot/{app.js,index.html,styles.css}`,
`IncrementalRunner.cs`, `EngineHost.cs`, `Program.cs`, `rust/lovelace-desktop/src/{main.rs,app.rs,
win32/shell.rs,engine/real.rs,svg.rs,selftest.rs,settings.rs}`, `rust/CONTRACT.md`,
`rust/lovelace-lang/src/lib.rs`, `.github/requirements/Lovelace.Rust.Desktop.md`,
`tools/desktop-gate.ps1`, `gate/{register,gate,journeys}.json`.

**Verdict semantics:** KEPT = the DoD requires the mechanism with evidence; REPLACED BY x = a
different mechanism stands in its place; REMOVED = absent from the DoD, including where the proposal
calls it out of scope.

**Headline findings**

1. `goal.md:41-61` marks all 19 mechanisms KEPT and states "Removed or replaced: none". The audit
   finds 3 REPLACED and 3 REMOVED.
2. The DoD cannot report success as written. It requires `p0_not_tested: 0` and `p0_blocked: 0`
   (`goal.md:73-78`), while `goal.md:131-136` commits TST-012 to `BLOCKED` and Linux/macOS to
   `NOT_TESTED`; `desktop-gate.ps1:214-220` rejects both statuses. Green therefore needs a CON-009
   matrix the proposal does not plan to publish and a user-approved TST-012 deviation; no waiver
   exists, and the DoD never mentions the deviation route.
3. The mechanism table is not "every mechanism the user named": the P0 groups LAY, LOG, A11Y and
   PERF appear only in dimensions D04/D13/D14/D16 (`goal.md:89,98,99,101`), not in
   `goal.md:34-59`.
4. The "real GUI evidence, not headless models" rule is asserted in three places but is not
   testable by the DoD's own gate (see section below).

## Mechanism-by-mechanism

| Mechanism | Verdict (KEPT \| REPLACED BY <x> \| REMOVED) | Proposal location | Citation | Consequence |
|---|---|---|---|---|
| Native workbench replacing the Win32 demo shell (the product bar) | **KEPT** | M1/M8/M11/M13 `goal.md:41,48,51,53`; D02/D03/D05/D12 `goal.md:87-90,97` | `requirements.md:13-16,50-53`; `main.rs:5-9`; `shell.rs:5-16` | The gate must replace the shell's editor, renderer and design, not extend them. |
| Code editor replacing the Win32 `EDIT`: completion, gutter, error decoration, caret/line/column, IME | **KEPT** | M1 `goal.md:41`; D05 `goal.md:90` | EDT-001..015 `requirements.md:189-203`; baseline `requirements.md:37`; `shell.rs:134` | A plain EDIT control cannot be the release editor. |
| Asynchronous execution off the UI thread; statement and sub-operation progress; real cancel | **KEPT** | M2 `goal.md:42`; D07 `goal.md:92`; D02 ARC-005/009 `goal.md:87` | RUN-001..014 `requirements.md:234-247`; ARC-005/009 `requirements.md:136,140`; `shell.rs:14-16,375-388` | Current inline `eval_full` on the UI thread is the defect to be replaced. |
| Incremental execution: statement slicing, source hashes, read-set dependency tracking, per-session cache | **KEPT** | M3 `goal.md:43`; D08 `goal.md:93` | INC-001..010 `requirements.md:259-268`; `IncrementalRunner.cs:173-258,358-372`; `README.md:56-62` | Metamorphic test must prove incremental equals fresh (`INC-010`). |
| Session-scoped precision with a visibly labelled control and `setprecision` synchronisation | **KEPT** | M4 `goal.md:44`; D07 `goal.md:92`; D06 `goal.md:91` | RUN-019/020 `requirements.md:252-253`; DOC-009 `requirements.md:223`; `app.rs:186-189,827-833`; `README.md:47-52` | Two tabs may hold different precision; cache keys must include it. |
| Tabs with one engine per document, isolation, concurrent documents | **KEPT** | M5 `goal.md:45`; D06 `goal.md:91`; D02 `goal.md:87`; D18 J-04/05 `goal.md:103` | DOC-001 `requirements.md:215`; ARC-006/007 `requirements.md:137-138`; `app.rs:204-245`; `README.md:38-43` | No process-wide engine lock; no cross-tab leak. |
| Workspace: variable table, function table, delete one variable, Clear Variables preserving functions | **KEPT** | M6 `goal.md:46`; D09 `goal.md:94` | WKS-001..004 `requirements.md:275-278`; `real.rs:10-12`; `rust/CONTRACT.md:84-100` | Claimed but currently unevidenceable: the frozen engine exposes no variable removal or selective clear (see below). |
| Structured results and symbolic inspector: canonical, tree, assumptions, trace, MathIR | **KEPT** | M7 `goal.md:47`; D10 `goal.md:95`; D11 `goal.md:96` | RES-001..006 `requirements.md:291-296`; SYM-001..007 `requirements.md:307-313`; `README.md:3-8`; `app.js:322-369`; `EngineHost.cs:104-139` | The desktop adapter must carry typed kernel values; it currently returns display strings (`real.rs:41-47,61-67`). |
| Graph renders the engine's own SVG, including text, polyline, path, clipping | **KEPT** | M8 `goal.md:48`; D12 `goal.md:97` | PLT-001/002 `requirements.md:323-324`; baseline `requirements.md:44`; `svg.rs:1-10`; `shell.rs:11-13` | The tiny polyline/text GDI parser is explicitly rejected. |
| Quick-eval console | **KEPT** | M9 `goal.md:49`; D07 D13 `goal.md:92,98` | RUN-002 `requirements.md:235`; LOG-003 `requirements.md:343`; `app.rs:194`; `index.html:103` | The command already exists headlessly; the shell input must be built. |
| Native file lifecycle: open/save, recent files, autosave, external-change and overwrite safety, settings | **KEPT** | M10 `goal.md:50`; D06 `goal.md:91`; D15 `goal.md:100` | DOC-002..006,008..010 `requirements.md:216-224`; PLF-001/004/006 `requirements.md:375,378,380`; `settings.rs:260-300`; `main.rs:74-76` | Recovery and conflict semantics are gated, not optional. |
| Visual design system: dark/light tokens, typography, control states, empty states, contrast | **KEPT** | M11 `goal.md:51`; D03 `goal.md:88` | VIS-001..010,016,018 `requirements.md:149-158,164,166`; `goal.md:91-109`; `styles.css:2-12` | Both themes must cover every control; no unthemed native child. |
| Automated acceptance: headless suites, real GUI driver, screenshot regression | **KEPT** | M12 `goal.md:52`; D17 `goal.md:102` | TST-001..011,014..017 `requirements.md:408-418,421-424`; `selftest.rs:84-495` (15 checks) | The 15 headless checks alone cannot satisfy any GUI row. |
| ARC-001 framework spike before committing to a native stack | **REPLACED BY a Windows-only spike record: no Windows/Linux comparison, no stated scenario set or selection criteria** | M13 `goal.md:53`; D02 `goal.md:87`; C6 `goal.md:118`; preference `goal.md:123` | ARC-001 `requirements.md:132`; §3 `requirements.md:465-470`; prior decision `.github/requirements/Lovelace.Rust.Desktop.md:75-111,318-322` | The recorded choice can rest only on Windows evidence; editor, IME, plotting and docking acceptance scenarios are not in the DoD. |
| No embedded webview, JavaScript engine, CDN, browser storage or mandatory localhost service; offline-first | **KEPT** | M14 `goal.md:54`; C1 `goal.md:113`; D15 `goal.md:100`; D18 J-21 `goal.md:103` | ARC-002 `requirements.md:133`; PLF-002/003/005 `requirements.md:376-379`; `requirements.md:27-29,496`; `main.rs:8-9`; `shell.rs:8-13` | The web Studio's REST/localStorage transport is not the mechanism; behaviour is. |
| One authoritative evaluation engine; no math/parser/precision semantics re-implemented in UI | **KEPT** | M15 `goal.md:55`; C2 `goal.md:114`; D01 CON-004 `goal.md:86` | CON-004 `requirements.md:120`; ARC-010 `requirements.md:141`; `rust/CONTRACT.md:84-105`; `real.rs:1-14` | ARC-010 is the only P0 ID absent from all 19 dimensions; it is carried only by RES-001/C2. |
| Platform matrix: Windows initial, Linux/macOS later ports, no platform claim without a built and run GUI test | **REPLACED BY a Windows-only release declaration in which Linux/macOS are "explicitly unsupported/NOT_TESTED", not later ports** | M16 `goal.md:56`; C6 `goal.md:118`; known limits `goal.md:134-136`; stop `goal.md:140-142` | CON-009 `requirements.md:125`; release rule `requirements.md:515-517` | No platform-matrix artifact is named by D01; C6 ("later ports") contradicts the known limit ("explicitly unsupported"); CON-009 cannot be PASS while `NOT_TESTED`. |
| The 22 mandatory end-to-end journeys | **KEPT** | M17 `goal.md:57`; D18 `goal.md:103`; gate fields `goal.md:74-75` | §4 `requirements.md:472-497`; `tools/desktop-gate.ps1:225-228` | 22/22 required; each needs a hash-bound artifact; the gate checks status/hash, not the journey's semantic outcome. |
| Typed data contracts: DocumentId/RunId/SourceRevision, typed commands and revision-bound events | **KEPT** | M18 `goal.md:58`; D02 ARC-003/004/008 `goal.md:87` | ARC-003/004/008 `requirements.md:134-139`; §3 `requirements.md:431-463` | Stale-revision events must never overwrite current pane state. |
| Web behavioural-parity register, SHA-pinned reference surfaces, no renames-as-equivalence | **KEPT** | M19 `goal.md:59`; D01 CON-001..003/007 `goal.md:86`; D18 `goal.md:103` | CON-001..003 `requirements.md:117-119`; CON-007 `requirements.md:123`; `README.md:3-11` | Register omissions fail the audit (`CON-002`). |
| Human usability sessions: at least 5 target users, 8 tasks, >=90% success, qualitative friction | **REMOVED** | M12 claim `goal.md:52`; known limits `goal.md:131-133`; stop `goal.md:140-142` | TST-012 `requirements.md:419`; release rule `requirements.md:515` | No human usability evidence will exist; the gate can only go green if the row becomes an approved deviation, which no one has approved. |
| Cross-platform screen-reader and keyboard test: one Windows UIA path and one other supported platform path | **REPLACED BY Windows-UIA-only accessibility evidence** | D14 `goal.md:99`; D17 `goal.md:102`; known limits `goal.md:134-136` | TST-011 `requirements.md:418`; A11Y-002 `requirements.md:357` | TST-011's "one other supported platform accessibility path" cannot exist; D17 cannot be MET. |
| All severity-1 usability defects closed at release | **REMOVED** | Release-rule quote `goal.md:19-22`; DoD `goal.md:84-107` has no such field | Release rule `requirements.md:518`; TST-013 is P1 `requirements.md:420` | A green gate can coexist with unresolved severity-1 usability defects; the only related row is report-only. |
| P0 human-waiver / approved-deviation path | **REMOVED** | DoD `goal.md:73-78` has no waiver field; gate proxy `desktop-gate.ps1:218` | Release rule `requirements.md:515`; CON-006 `requirements.md:122` | The contract's "or human-waived" route has no user-approval, expiry or evidence record; the script's free-text `APPROVED_DEVIATION` is a proxy, not the named mechanism. |
| "Real GUI evidence, not headless models" rule | **KEPT** | DoD `goal.md:80-82`; C4 `goal.md:116`; evidence rules `goal.md:147-153` | §0 `requirements.md:25-26`; TST-002 `requirements.md:409` | Stated but not enforceable: register rows carry no evidence class (`desktop-gate.ps1:77-88`) and freshness checks only path/hash/HEAD (`desktop-gate.ps1:151-170`). |
| Layout, panes, draggable splitters, status strip, narrow-window behaviour | **KEPT** | D04 `goal.md:89`; absent from the M-table `goal.md:34-59` | LAY-001..004,008,009,011 `requirements.md:172-182`; `index.html:37-110` | P0 layout is gated even though the mechanism table omits it. |
| Console/logs: exact-once print, typed event lines, actionable errors, bounded storage | **KEPT** | D13 `goal.md:98`; absent from the M-table `goal.md:34-59` | LOG-001..006,010 `requirements.md:341-350` | LOG-001's byte-sensitive exactly-once rule is the anti-duplication gate. |
| Accessibility: full keyboard, UIA roles/names, focus, text scaling, Unicode glyphs | **KEPT** | D14 `goal.md:99`; absent from the M-table `goal.md:34-59` | A11Y-001..008,012,014 `requirements.md:356-369` | Gated, but the cross-platform half of TST-011 is not. |
| Performance budgets: cold start, typing p95, scroll, run-start, cancel, memory, paint failure | **KEPT** | D16 `goal.md:101`; absent from the M-table `goal.md:34-59` | PERF-001..005,009..012 `requirements.md:391-402` | Budgets must be measured on this machine; P1 budget rows are report-only. |

**Counts:** 23 KEPT, 3 REPLACED, 3 REMOVED (29 mechanisms).

## Mechanisms the definition of done cannot evidence

- **TST-012 — human usability sessions.** `goal.md:131-133` says the row "will stay `BLOCKED`" because
  the run cannot recruit five users; `desktop-gate.ps1:214-220` fails every P0 row that is not `PASS`
  or `APPROVED_DEVIATION`. The stop criterion's carve-out (`goal.md:140-142`) is prose, not a gate
  status, so it does not change that. No waiver record exists, so the DoD's own expected observable
  (`p0_blocked: 0`) is unreachable.
- **CON-009 / Linux and macOS.** `goal.md:134-136` commits the non-Windows rows to `NOT_TESTED`,
  while `goal.md:73-78` requires `p0_not_tested: 0`. No dimension names a published Windows/Linux/
  macOS matrix as an artifact; `goal.md:118` calls Linux/macOS "later ports" and `goal.md:134-136`
  calls them "explicitly unsupported", which are different commitments.
- **TST-011's second platform.** The acceptance criterion requires "one Windows UIA screen reader and
  one other supported platform accessibility path" (`requirements.md:418`). D14 (`goal.md:99`) can
  only supply Windows UIA. D17 therefore cannot be MET.
- **ARC-001's Linux half.** §3 requires the framework spike "on Windows and Linux"
  (`requirements.md:465-470`), and ARC-001 requires the named editor/accessibility/text-IME/
  plotting/docking scenarios (`requirements.md:132`). D02 asks only for "spike record"
  (`goal.md:87`). The Windows-only machine cannot produce the comparison the contract specifies.
- **The GUI-evidence rule and the 19 dimensions.** `goal.md:80-82` says each dimension is MET only
  when its P0 rows are PASS with the required evidence class, but the register schema has no
  evidence-class field (`desktop-gate.ps1:77-88`), `Test-FreshEvidence` checks only existence, hash
  and HEAD (`desktop-gate.ps1:151-170`), and `-Verify` computes no dimensions at all
  (`desktop-gate.ps1:206-253`; `gate.json` has no dimension field). A `--selftest` text log can
  therefore certify EDT-001, RUN-001 or PLT-001 as "GUI evidence".
- **WKS-003 / WKS-004 — delete one variable and Clear Variables preserving functions.** The engine
  contract exposes no variable-removal or selective-clear operation (`rust/CONTRACT.md:84-100`);
  `Engine::set_var` is crate-private (`rust/lovelace-lang/src/lib.rs:1606`), and `real.rs:10-12`
  documents that `clear()` rebuilds the engine and discards user functions — exactly the workaround
  WKS-004's acceptance criterion says must fail (`requirements.md:278`). C7 (`goal.md:119`) leaves
  `lovelace-lang` outside this run's ownership, so D09 cannot produce this evidence without an engine
  change or the rejected workaround.
- **P1 "polished 1.0" set.** Installer/release artifacts (PLF-007), SUS >=80 (TST-013), high-contrast
  mode (A11Y-009) and the rest of the 56 P1 rows are report-only (`goal.md:106-107`), and `-Verify`
  filters to P0 only (`desktop-gate.ps1:214`). The goal defers all P1 without the "explicit product
  decision" the contract's priority semantics require (`requirements.md:23-24`), and without a
  documented quality impact.

## Not judged

- Whether the delivered product is "genuinely pleasant" or "meaningfully improves desktop
  ergonomics". The contract gives no operational test outside TST-012/TST-013, which are removed or
  non-gating; that is a product judgement, not an intent-conformance question.
- Whether `requirements.md` matches the user's original paste word-for-word. I verified only internal
  consistency: 229 rows, 171/56/2 priorities, 17 groups, 22 journeys. The original paste is not in
  the repo to diff against.
- Whether deferring all 56 P1 rows is acceptable to the user. The contract permits deferral only by
  explicit product decision; the proposal defers them by policy, and only the user can decide whether
  that still satisfies "genuinely pleasant".

## Question the user must answer

**Do you approve the two P0 carve-outs this proposal depends on — TST-012 (at least five target users
in human usability sessions) as a human-waived deviation, and a Windows-only initial-release platform
matrix with Linux/macOS deferred — or must the release remain blocked until both are delivered?**
