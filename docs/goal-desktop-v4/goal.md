# Goal — Lovelace Desktop v4.0: native Rust parity and modern UX

**Owner:** orchestrator (stretch-goal-harness). **Started:** 2026-11-12. **Round cap:** 40.
**Authoritative requirement catalog:** [`requirements.md`](requirements.md) — 229 requirements
(171 P0 / 56 P1 / 2 P2) plus 22 mandatory journeys.

## Verbatim request (anchor)

The user pasted the full *Lovelace Desktop — Native Rust Parity & Modern UX Requirements (v4.0)*
contract and invoked `/stretch-goal-harness`. Its own statement of the outcome, quoted verbatim:

> **Product goal:** A modern, genuinely pleasant, self-contained native scientific scripting workbench
> built in Rust that preserves every meaningful user-facing capability of the existing JavaScript Studio
> and meaningfully improves the desktop ergonomics. A Win32 demo shell that happens to execute scripts
> does **not** satisfy this goal.

and its release rule, quoted verbatim:

> A production-quality desktop parity release requires: **all P0 requirements** passing or
> human-waived; every Web-parity item explicitly accepted; all 22 mandatory journeys executed and
> green; native GUI launched on each advertised OS; no skipped proof counted as PASS; clean
> source-bound build and evidence hashes; and all severity-1 usability defects closed.

Every normative row of the source contract is transcribed verbatim in
[`requirements.md`](./requirements.md); that file is the only permitted source of requirement IDs,
priorities, types and acceptance criteria.

## The goal, in one sentence

Lovelace Desktop ships as a native Rust scientific scripting workbench that satisfies every P0 row of
the v4.0 contract with fresh, source-bound GUI evidence and passes all 22 mandatory journeys, judged
by the §5 release gate against the SHA-pinned web Studio reference.

## Named mechanisms — intent anchor table

Every mechanism the user named. A row may leave this table only by a user-approved `DEC` with
`Owner: user` and `Status: approved`.

| # | Mechanism named by the user | Verdict | Where the goal carries it |
|---|---|---|---|
| M1 | Code editor replacing the Win32 `EDIT`: completion, error decoration, caret/line/column | **KEPT** | EDT-001..021, LAY-001 |
| M2 | Asynchronous runtime with statement + sub-operation progress and cancel | **KEPT** | RUN-001..019, ARC-004..009 |
| M3 | Incremental execution: source hash + dependency tracking + per-session cache | **KEPT** | INC-001..011, RUN-015 |
| M4 | Session-scoped precision with a visible native control | **KEPT** | RUN-019, RUN-020, DOC-009 |
| M5 | Tabs with one engine per document and session isolation | **KEPT** | DOC-001, ARC-006/007, LAY-009 |
| M6 | Workspace: variable table, function table, delete variable, clear-variables semantics | **KEPT** | WKS-001..011 |
| M7 | Structured results and symbolic inspector (canonical, trace, assumptions, MathIR) | **KEPT** | RES-001..011, SYM-001..011 |
| M8 | Graph rendering the engine's own SVG (not a partial GDI interpretation) | **KEPT** | PLT-001..013 |
| M9 | Quick-eval console | **KEPT** | RUN-002, LOG-003 |
| M10 | Native files: open/save, tabs, recent files, settings, autosave/restore, conflict safety | **KEPT** | DOC-002..014, PLF-001/004/006 |
| M11 | Modern visual design: dark/light themes, tokens, typography, states | **KEPT** | VIS-001..018 |
| M12 | Automated tests: headless **plus real GUI**, screenshot regression, usability | **KEPT** (usability sessions human-waived, DEC-006; every automated part stays required) | TST-001..018, section 5 |
| M13 | Genuine native UI/editor framework spike before committing (ARC-001) | **KEPT** (Windows spike required; Linux comparison deferred by DEC-006) | ARC-001, section 3 |
| M14 | No webview / JavaScript / CDN / localhost HTTP / browser storage; offline-first | **KEPT** | ARC-002, PLF-002/003/005 |
| M15 | One authoritative evaluation engine; no math/parser/precision semantics re-implemented in UI | **KEPT** | CON-004, ARC-010 |
| M16 | Platform matrix (Windows initial release; Linux/macOS later ports) with built+run evidence only | **KEPT** | CON-009, PLF-009/010, `platform-matrix.md` |
| M17 | The 22 mandatory end-to-end journeys | **KEPT** | requirements.md §4, TST-001 |
| M18 | Data contracts: DocumentId/RunId/SourceRevision, typed commands and events | **KEPT** | ARC-003/004/008, section 3 |
| M19 | Stable user-visible capability of the web Studio preserved (W-type parity) | **KEPT** | CON-002/003/007, all W rows |
| M20 | All severity-1 usability defects closed at release | **KEPT** | `gate/defects.json`, see below |
| M21 | "Passing **or human-waived**" P0 route with an owner-approved record | **KEPT** | `gate/waivers.json`, see below |

**Removed or replaced: none** after DEC-006. The two waived items are recorded as user-approved
deviations, not dropped: TST-012 (whole row) and the Linux half of the ARC-001 / TST-011 platform
scope. Everything else stays required.

## Definition of done (falsifiable)

Two verdicts are defined. Only the second is a release.

```
powershell -NoProfile -ExecutionPolicy Bypass -File tools/desktop-gate.ps1 -Readiness
powershell -NoProfile -ExecutionPolicy Bypass -File tools/desktop-gate.ps1 -Verify
```

**READY_FOR_WAIVER** (`-Readiness`, exit 0): no P0 row is `FAIL` or `NOT_TESTED`; every row is either
`PASS` with fresh, class-correct, hash-bound evidence, or `BLOCKED` with a named missing artifact and
a user-owned waiver in `gate/waivers.json`. This is the strongest verdict the machine can produce
alone.

**RELEASE** (`-Verify`, exit 0): additionally, every P0 row is `PASS` or covered by a user-approved
waiver, all 22 journeys are `PASS` with fresh evidence, `gate/defects.json` contains zero open
severity-1 usability defects, the working tree is clean, and `gate/gate.json` records the same HEAD
that produced every evidence hash.

Any non-zero exit, any `FAIL`/`NOT_TESTED` P0 row, any journey not `PASS`, any stale evidence hash,
any evidence whose class cannot satisfy the row, or any open severity-1 defect is a **gate failure**,
not a partial pass.

### Evidence classes

Headless AppState output can never certify a GUI row. Every requirement row carries a required
evidence class; a `PASS` row must attach at least one artifact of that class:

| Class | Accepted artifact |
|---|---|
| `unit` | a test binary/log with concrete input→output assertions |
| `gui` | a real-window artifact: screenshot PNG, UIA tree dump, or GUI-driver log naming the window |
| `perf` | a measured numeric result with the command and machine recorded |
| `file` | an on-disk artifact proving bytes (round-trip, recovery, atomic save) |
| `static` | a source-level audit with citations, for pure absence/shape requirements |

### The 19 dimensions

Each dimension is `MET` only when all of its P0 rows are `PASS` (or waived) with the class that row
requires.

| # | Dimension | Requirements |
|---|---|---|
| D01 | CON — contract, scope, traceability | CON-001..009 |
| D02 | ARC — native architecture, concurrency | ARC-001..009 |
| D03 | VIS — visual design system | VIS-001..010, 016, 018 |
| D04 | LAY — layout, panes, navigation | LAY-001..004, 008, 009, 011 |
| D05 | EDT — editor | EDT-001..010, 012..017 |
| D06 | DOC — documents and files | DOC-001..006, 008..010, 014 |
| D07 | RUN — async execution, progress, cancel | RUN-001..014, 016, 017, 019, 020 |
| D08 | INC — incremental correctness | INC-001..008, 010 |
| D09 | WKS — workspace | WKS-001..004, 006, 009 |
| D10 | RES — structured results | RES-001..006, 010 |
| D11 | SYM — symbolic inspector and MathIR | SYM-001..007, 009, 011 |
| D12 | PLT — plots | PLT-001..006, 008, 009, 011, 012 |
| D13 | LOG — console and logs | LOG-001..006, 010 |
| D14 | A11Y — accessibility | A11Y-001..008, 012, 014 |
| D15 | PLF — OS integration, security, packaging | PLF-001..006, 009, 010 |
| D16 | PERF — budgets | PERF-001..005, 009..012 |
| D17 | TST — acceptance infrastructure | TST-001..012, 014..017 |
| D18 | Journeys | J-01..J-22 |
| D19 | Release evidence | TST-016, CON-005/008 |

P1 rows are reported but do not gate convergence; they are required only for the words
"polished 1.0". P2 rows are optional.

## Human-waiver record

`gate/waivers.json` is the only way a P0 row may be non-`PASS` at RELEASE. Each entry is
`{ id, owner: "user", approved: true, quote, date, scope }`, where `quote` is the user's own words.
An entry with `owner` other than `user`, `approved: false`, or no quote does not count.

DEC-006 records the user's approval of a Windows-only v1 and the waiver of TST-012.

## Hard constraints (gates)

| # | Constraint | Provenance |
|---|---|---|
| C1 | No embedded webview, JavaScript engine, CDN asset, browser storage or mandatory localhost HTTP service; the app evaluates with all network interfaces disabled | user (ARC-002, PLF-002/003/005, §0) |
| C2 | One authoritative engine: desktop commands call the same Rust kernel as the Rust runner; no math/parser/precision semantics re-implemented in widgets, renderer or tests | user (CON-004, ARC-010) |
| C3 | Do not change language semantics, the C# oracle, fixture expectations or web Studio behaviour to make desktop tests pass | user (§0) |
| C4 | P0 is blocking; an absent, skipped, stale or unrun acceptance test is a failure; headless proof cannot stand in for GUI proof | user (CON-005, TST-002, §0) |
| C5 | Unsupported or unverified features are shown as `BLOCKED`/`NOT_TESTED`, never silently skipped or renamed PASS | user (TST-017) |
| C6 | A platform is claimed only after a built and run GUI test on it; Windows is the initially released platform, Linux/macOS are later ports | user (CON-009) + DEC-006 |
| C7 | The engine is the contract's engine: extending `rust/lovelace-lang` with a required capability (for example variable removal) is allowed and must be tested, but language semantics, the C# oracle, fixtures and web Studio behaviour must not change to make desktop tests pass. Concurrent edits from another session must never be reverted | agent, corrected after VAL-001 |
| C8 | A P0 row may leave the release gate only through `gate/waivers.json` with `owner: user` | user (release rule) + DEC-006 |

## Preferences (tie-breakers only, never gates)

- `egui`/`eframe` is the leading framework candidate because the repo's own prior requirements doc
  chose it; the ARC-001 spike decides on measured evidence, and a different framework may win.
- Dark theme is the default; 1280×800 is the default window; the §1 token table is a starting point,
  not a validated palette.
- Core semantics move to a framework-independent layer before any renderer is committed.

## Resolved scope limits (DEC-006)

- **TST-012** (≥5 target users, 8 tasks, ≥90% success) is waived by the user for v1. It stays in the
  register as `BLOCKED` with the missing evidence named and the waiver attached; it is never renamed
  PASS.
- **TST-013** (SUS ≥ 80) is P1 and is not claimed.
- **CON-009**: `platform-matrix.md` declares Windows 11 x64 as the initial release and Linux/macOS as
  later ports with no support claim; PLF-009 requires the unsupported-platform error to be explicit.
- **ARC-001**: the spike is executed on Windows with the full scenario set; the Linux comparison is
  deferred under the same decision and recorded as such in the spike record.
- **TST-011**: the "one other supported platform accessibility path" clause has no referent while
  Windows is the only supported platform; the Windows UIA path is still required and tested.

## Stop criteria

1. **Converged:** `-Verify` exits 0 with all 19 dimensions MET (or waived per the waiver record).
2. **Capped:** round 40 reached. Stop and report coverage honestly; a cap is not convergence.
3. **Blocked:** the same concrete blocking condition persists for three consecutive rounds. Name the
   condition, never "it is hard".

## Non-negotiable evidence rules for this goal

- Agent chat answers are claims. Only the orchestrator's own tool calls create `EVD` rows.
- Any GUI claim requires an artifact: a screenshot PNG, a UIA tree dump, or a driver log naming the
  window that produced it — headless `--selftest` output is not GUI evidence.
- Any performance claim requires a measured number from this machine with the command recorded.
- Any parity claim names the baseline SHA and the reference path.
