# Round 32 — DOC-004 / DOC-005: recent files, session restore and the crash-recovery journal

Scope: `rust/lovelace-desktop/**` and `docs/goal-desktop-v4/round-32/`. No engine crate, `tools/`
or gate file was modified.

## 1. What existed before this round

Round 31 gave the workbench its native file lifecycle (`open`/`save`/`save_as`/`close_document`,
the Save/Discard/Cancel prompt, the File bar and a `Recent` menu) and a versioned settings value
(`settings.rs`, `%APPDATA%\Lovelace\desktop.json`). What was still missing:

- `settings.rs` stored **window geometry, layout fractions, precision and recent paths**, but the
  new window never loaded or wrote the file: `WorkbenchApp::new` constructed a bare
  `Workbench::new()` and `Recent` therefore only showed paths opened *in that session*. Nothing
  pruned a deleted path either.
- There was **no open-document list** in the settings, so a relaunch always started on the starter
  documents.
- There was **no journal, no recovery directory and no recovery prompt**; `shortcuts`/`main.rs`
  said nothing about restoring or recovering. `requirements.md:219` (DOC-005) was open.

## 2. What was added

### 2.1 Recent files (DOC-004.1)

`Settings` already had `recent_files: Vec<String>` and `add_recent` (deduplicated, capped at
`MAX_RECENT_FILES = 20`). This round adds the missing half of the lifecycle:

- `Settings::prune_missing_recent() -> usize` drops every path that no longer exists when the
  settings are loaded into the application, returning how many were dropped. The window logs
  `Pruned N recent file(s) that no longer exist.` at Info level.
- `Workbench::set_recent_files(Vec<String>)` adopts the persisted list at launch with the same
  rules as the workbench's own `add_recent`: blank paths dropped, missing paths pruned, duplicates
  dropped, existing entries keep their order, capped at 20.
- The File bar's `Recent` menu reads the workbench list, so after this round it offers the files
  from the previous session (minus missing ones) and `open` from it moves a file to the top.

Tests: `settings::tests::recent_files_move_a_reopened_file_to_the_top_without_duplicating_it`,
`settings::tests::recent_paths_that_no_longer_exist_are_pruned_on_load`,
`workbench::tests::adopting_the_saved_recent_list_dedupes_prunes_and_keeps_the_order`,
`workbench::tests::reopening_a_file_moves_it_to_the_top_of_the_recent_list`.

### 2.2 Session restore (DOC-004.2)

The settings schema (still version 1) gains one optional object; a v1 file without it restores
nothing and launches the starter documents:

```json
"session": {
  "active": 2,
  "documents": [
    { "document_id": 1 },
    { "path": "C:\\scripts\\a.lv", "document_id": 3 }
  ]
}
```

- A tab **with a path** is persisted as that path and restored by **reading the file again**
  (`document_io::read`), never by executing it and never by writing it. The restored tab is clean
  and keeps the file's own newline style and BOM.
- A tab **without a path** keeps only its document identity. It is restored only when a DOC-005
  recovery record with that identity still exists; the record's text becomes the buffer (dirty
  against disk) and the record is consumed. Without a record the entry is skipped with a
  `Session restore skipped a tab: …` diagnostic; a clean, empty pathless buffer has nothing worth
  restoring and simply does not come back.
- The **active tab** is restored by *identity through the entry index*, not by index into the
  restored list: if the active entry was skipped, the nearest following restored tab is selected.
  That is why a session of `[welcome, untitled, a.lv, b.lv]` with `b.lv` active relaunches as
  `[a.lv, b.lv]` with `b.lv` selected.
- `Workbench::session_state()` produces the value; `Workbench::restore_session(&SessionState,
  &[RecoveryRecord]) -> RestoreReport` consumes it and reports `restored`, `recovered` and
  `skipped`. The workbench logs one `Restored N of M document(s) from the last session; nothing was
  run and no file was written.` line plus one line per skip.

Persistence points: the window writes the session whenever the tab list, the active tab, the recent
list or the active precision changes (`persist_state(false)`, once per frame at most), and forcibly
at a clean exit (`persist_state(true)`). The file is therefore current from the first frame after a
launch, which is what makes a **crash** leave a current session rather than one from an earlier
clean quit.

Tests: `settings::tests::session_tab_order_and_active_tab_round_trip_through_disk`,
`settings::tests::a_malformed_session_is_ignored_with_a_warning`,
`workbench::tests::the_session_lists_the_tabs_in_order_with_their_paths_and_the_active_tab`,
`workbench::tests::restoring_the_last_session_reopens_the_same_tabs_in_order_without_running_or_writing`,
`workbench::tests::restoring_a_session_never_constructs_an_engine`,
`workbench::tests::a_pathless_session_tab_restores_only_when_the_journal_covers_it`,
`ui::app::tests::the_session_is_written_at_a_clean_quit_and_restored_on_the_next_launch`.

### 2.3 The recovery record (DOC-005.4)

New module `src/recovery.rs`. One record per dirty document lives under the application-scoped
recovery directory, `settings::default_recovery_dir()` = `%APPDATA%\Lovelace\recovery` (the sibling
of `desktop.json`), named `doc-<document id>.lvrec`. The format is a fixed ASCII header, one blank
line, then the buffer bytes exactly as the editor holds them:

```
LOVELACE-RECOVERY 1
id: 3
revision: 5
title: recovery-fixture.lv
path: C:\...\round-32\recovery-fixture.lv
bom: 0
caret: 1:9
mtime_unix_nanos: 1794517436761189100
saved_unix_nanos: 1794517442805875200
bytes: 16

a = 1
b = 2 = 1
```

| Key | Meaning |
|---|---|
| `LOVELACE-RECOVERY 1` | magic + record schema number (a newer number is refused: `record schema 99 is newer than supported 1`) |
| `id` | the document identity the file name is keyed by |
| `revision` | the source revision the body belongs to |
| `title` | the tab title to restore |
| `path` | the document's file, when it has one (the line is absent for an unsaved buffer) |
| `bom` | `0`/`1`, whether the file began with a UTF-8 BOM |
| `caret` | 0-based `row:col` |
| `selection` | anchor and caret end, `row:col-row:col`; **absent when there is no selection** |
| `mtime_unix_nanos` | the source file's modification time when the document was opened/last saved |
| `saved_unix_nanos` | when the record was written |
| `bytes` | the body length; validated on load, so a truncated file is refused instead of recovered short |

`RecoveryRecord::parse` never panics: a missing magic, a missing id/title/byte count, a byte count
that disagrees with the body, a header that is not UTF-8 or a body that is not UTF-8 each produce a
message naming the problem. Unknown keys are ignored for forward compatibility.

**Atomicity.** `RecoveryRecord::write_atomic(path)` writes a sibling temp file
(`doc-3.lvrec.tmp-<pid>-<n>`), `sync_all()`s it, and only then renames it over the record. On
Windows `std::fs::rename` is `MoveFileEx(MOVEFILE_REPLACE_EXISTING)`, so the visible record is
always a whole file: a kill leaves either the previous record or the new one. On failure the temp
file is removed and the error is logged.

**Throttle.** `RecoveryJournal` (one per window) tracks per document when it last wrote and which
revision is on disk. `sync(documents, now)` writes a dirty document's **first** revision
immediately (leading edge), then at most one write per `DEFAULT_DEBOUNCE` (1 s) while edits keep
arriving. `pending_delay(documents, now)` reports the remaining time, and the window calls
`request_repaint_after` so the **trailing** write happens even if the user stops typing. A forced
kill therefore loses at most one debounce window; the window proof measured the newest revision on
disk 909 ms after the last keystroke.

**When records go away:** a document that becomes clean (saved) deletes its record on the next
pass; closing a tab deletes its record; a clean exit deletes the records of every open document
(`RecoveryJournal::clean_exit`). Records that belong to a recovery prompt the user kept for later
are *not* deleted at exit: quitting must not destroy recoverable work.

Tests: `recovery::tests::a_record_round_trips_and_its_body_is_the_buffer_bytes`,
`recovery::tests::an_atomic_write_replaces_the_previous_record_and_leaves_no_temporary_file`,
`recovery::tests::scanning_skips_corrupt_and_truncated_records_with_a_warning`,
`workbench::tests::a_journaled_dirty_buffer_produces_a_record_immediately_and_is_throttled_for_a_second`,
`workbench::tests::a_journaled_record_names_the_sources_mtime_at_open`,
`workbench::tests::a_crash_leaves_the_record_and_the_next_launch_restores_the_exact_text_and_original_path`,
`workbench::tests::a_clean_exit_deletes_only_the_records_the_session_owns`,
`ui::app::tests::an_edit_journals_the_buffer_and_the_save_retires_the_record`.

### 2.4 Recovery on launch and the prompt (DOC-005.5)

`WorkbenchApp::launch` scans the recovery directory **before** anything else. `recovery::scan`
reads every `*.lvrec`, skips a file that cannot be read or parsed with a warning naming it
(`skipped a corrupt recovery record: <file>: <reason>`), and returns the usable records sorted by
document identity. `*.lvrec.tmp-*` files from an interrupted write are ignored.

- A record the session restore consumed is tracked by the journal (its file already holds exactly
  the text now in the tab) and is deleted when that document is saved, closed or the app exits.
- Every other record is offered by a foreground `egui::Modal` at launch: the title **Recover
  unsaved work**, the explanation, one row per record with its tab title, the **original path**,
  its byte count, and `Recover` / `Discard` buttons. The keyboard is stripped from every frame
  while it is open and the modal's layer blocks the panes, so **the text is offered before any
  save can overwrite a file**.
- `Recover` puts the record's text, caret and selection into the tab that already represents the
  document (usually the tab the session just restored from disk) or into a new tab; nothing is
  executed and nothing is written. `Discard` deletes the record. `Escape` keeps the records for the
  next launch (they are not deleted).
- While the prompt is open a window close is refused (`CancelClose`), so a quit cannot race the
  question.

Tests: `ui::app::tests::the_recovery_prompt_offers_a_crashed_buffer_and_recovering_restores_it_before_any_save`,
`ui::app::tests::discarding_a_recovery_record_deletes_it_and_leaves_the_file_and_buffer_alone`,
`ui::app::tests::a_corrupt_recovery_record_is_skipped_with_a_warning_and_the_launch_continues`,
`ui::app::tests::discarding_at_the_window_close_does_not_recreate_the_record`.

### 2.5 Wiring and visibility

`WorkbenchApp` gained `settings: Settings`, `storage: AppStorage { settings_path, recovery_root }`
and `recovery: RecoveryJournal`. `WorkbenchApp::new` resolves both from the application directory;
`WorkbenchApp::with_storage` is the test seam, so no test touches the user profile and no product
code writes beside the executable or below the working directory (PLF-006). A settings write
failure, a journal write failure and a skipped record are all `Diagnostic` lines in the Logs pane.

## 3. Tests and observed output

`cargo test -p lovelace-desktop` (log: [`cargo-test.log`](cargo-test.log)):

```
test result: ok. 347 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.84s
```

347 = the 318 tests that passed on the same tree before this round + 29 new tests:

| Module | New tests | What they prove |
|---|---|---|
| `settings` | 5 | tab order and active tab round-trip through save/load on disk; a re-opened recent file moves to the top exactly once; missing recents are pruned on load; a malformed session is ignored with a warning; the recovery directory is the settings file's sibling |
| `recovery` | 8 | a record round-trips and its body is byte-identical to the buffer; no path/selection variant; atomic write replaces a record and leaves no temp file; corrupt/truncated/garbage/empty records are skipped with warnings naming each file; scanning a missing directory is empty; discard deletes only its file; a newer schema is refused; a disabled journal touches nothing |
| `workbench` | 10 | recent list adoption (dedup/prune/order); re-open moves to top; session lists tabs/paths/active; restore reopens tabs in order without running or writing; restore constructs no engine; pathless restore needs the journal; first edit journaled immediately then throttled for a second with `pending_delay`; the record names the source mtime; a crash leaves the record and the next launch recovers the exact text and path without overwriting; clean exit deletes only the session's records |
| `ui::app` | 6 | the prompt lists a crashed buffer and Recover restores the text/caret before any save and keeps the record until the save; Discard deletes it and changes no buffer; a corrupt record is skipped with a log warning and the launch continues; the session is written at a clean quit and restored on the next launch (order + active tab); an edit journals the buffer and the save retires the record; discarding at the window close does not recreate the record |

Verbatim assertions worth quoting:

- Byte identity: `assert_eq!(first.source.as_bytes(), b"a = 1\n", "byte-identical to the buffer")`
  and, for the crash test, `assert_eq!(record.source, "a = 1\nb = 3\nc = 4\n", …)` while the file
  on disk is still `b"a = 1\nb = 2\n"`.
- Throttle: `assert!(report.written.is_empty(), "typing must not rewrite per keystroke")`,
  `assert_eq!(journal.pending_delay(…, start + 500ms), Some(Duration::from_millis(500)))`, then
  `assert_eq!(report.written, vec![document_id])` at +1000 ms.
- Restore does not run: a factory that counts its own invocations,
  `assert_eq!(calls.load(Ordering::SeqCst), 0, "no engine was constructed")`, every document
  `RunState::Idle`, and no log line starts with `Run queued`; every file's bytes are compared
  before and after.
- Crash: the record is scanned after the "process" (workbench + journal) is dropped without the
  clean-exit path; `assert_eq!(record.path, Some(file.clone()), "the record reports the original
  path")`.
- Corrupt: `scan.records.len() == 1`, `scan.warnings.len() == 3`, every warning contains
  `corrupt recovery record` and one names `doc-8.lvrec`.
- Discard: `assert!(!record_path.exists(), "discarding deletes the record")` and the file and
  buffer are unchanged.

### 3.1 Two defects the window proof found (and fixed)

The first capture run was not accepted as evidence. It exposed two real defects in the new code,
both fixed before the run below:

1. **The trailing journal write waited for the next input event.** The leading edge wrote the first
   keystroke at once, but after the user stopped typing no frame was requested, so the newest
   revision could stay unjournaled indefinitely (observed: a 13-byte record for a 16-byte buffer).
   `RecoveryJournal::pending_delay` + `request_repaint_after` now guarantee the write when the
   window elapses.
2. **A discard at the window close re-journaled the buffer after the clean exit.** `Discard` at the
   *window* prompt leaves the buffer dirty, and the same frame's journal pass rewrote the record
   that `finish_clean_exit` had just deleted (observed: `1 record(s) left`). The journal and
   settings passes are now skipped once `window_close_approved` is set; regression test
   `discarding_at_the_window_close_does_not_recreate_the_record`.

## 4. Window proof — captures and method

The console session was **locked** for the whole capture: the foreground window was
`Windows Default Lock Screen` (`SESSION foreground='Windows Default Lock Screen' locked=True` in
[`capture-run.log`](capture-run.log)). [`capture-session-recovery.ps1`](capture-session-recovery.ps1)
therefore uses only:

- **UIA Invoke** (`InvokePattern`) for the recovery prompt's `Recover` / `Discard`, the File bar's
  buttons and the close prompt's `Discard`;
- **PrintWindow with `PW_RENDERFULLCONTENT`** into a `System.Drawing.Bitmap` of `GetWindowRect`, so
  an occluded/locked-desktop window still renders;
- **posted `WM_KEYDOWN`** messages for the edit (winit resolves the character from the virtual key
  and hands egui a real key/text event; a posted `WM_CHAR` would deliver the character twice);
- `SetWindowPos` to 1600x1000 for legible captures.

`APPDATA` was redirected to [`appdata`](appdata) and the process working directory was
[`work`](work), so the settings file, the recovery directory and the plot directory all resolve
inside `round-32` and never below the working directory.

The fixture is [`recovery-fixture.lv`](recovery-fixture.lv): 12 bytes, `a = 1\nb = 2\n`, SHA-256
`FE9C2DADB34BEE2AE03FC8FE25E26C64CE2F572A53A127DB5D6E73899B64FC11`.

Sequence and result (**46 checks, 0 failed** — [`window-proof.json`](window-proof.json),
[`capture-run.log`](capture-run.log)):

1. Launch with the fixture path. Title `Lovelace Desktop - recovery-fixture.lv`; the tab reads
   `recovery-fixture.lv`; the File bar shows the path; the editor reads `3 lines / 12 bytes`; the
   log line is `[info] Opened …\recovery-fixture.lv (12 bytes, UTF-8 · LF).`; `desktop.json`
   exists and names the open file.
   → [`launched.png`](launched.png) (1600x1000, 182 020 bytes) and
   [`launched-uia.txt`](launched-uia.txt).
2. Posted `VK_DOWN`, `VK_END`, then ` = 1`. Caret `Ln 2, Col 6` before typing; the tab becomes
   `recovery-fixture.lv *`, the buffer `3 lines / 16 bytes`, and the file's SHA-256 is unchanged.
   → [`dirty-uia.txt`](dirty-uia.txt).
3. Within the debounce window the journal wrote `recovery\doc-3.lvrec`. Its header is
   `bom=0; bytes=16; caret=1:9; id=3; mtime_unix_nanos=1794517436761189100; path=…\recovery-fixture.lv;
   revision=5; saved_unix_nanos=1794517442805875200; title=recovery-fixture.lv`; its body is exactly
   `a = 1\nb = 2 = 1\n` (16 bytes); the newest revision reached disk **909 ms** after the last
   keystroke. The record is copied to [`record-after-crash.lvrec`](record-after-crash.lvrec).
4. Simulated crash: `Stop-Process -Force` (no `WM_CLOSE`, no clean-exit path; exit code -1). The
   record still exists and is byte-identical afterwards.
5. Relaunch with no arguments. The window title is already `Lovelace Desktop - recovery-fixture.lv`
   (the session restored the tab by reading the file; its SHA-256 is still `FE9C2D…FC11`), and the
   recovery prompt lists the record with its original path and `16 bytes`.
   → **[`recovery-prompt.png`](recovery-prompt.png) (1600x1000, 169 772 bytes)**, read back for this
   report, and [`recovery-prompt-uia.txt`](recovery-prompt-uia.txt) (`Recover unsaved work`,
   `The last session did not close cleanly. …`, `…\recovery-fixture.lv  ·  16 bytes`,
   `Button | Recover`, `Button | Discard`).
6. UIA-invoke `Recover`. The tab is `recovery-fixture.lv *`, the editor `3 lines / 16 bytes`, the
   caret `Ln 2, Col 10` (the journaled `1:9`), the log reads
   `[info] Recovered recovery-fixture.lv from the recovery journal (16 bytes); nothing was written.`,
   the prompt is gone, the file's SHA-256 is still `FE9C2D…FC11`, and the record's body is
   byte-identical to the pre-crash body.
   → **[`restored-buffer.png`](restored-buffer.png) (1600x1000, 201 302 bytes)**, read back for this
   report, and [`restored-buffer-uia.txt`](restored-buffer-uia.txt). The record copy is
   [`record-after-recovery.lvrec`](record-after-recovery.lvrec) (identical to the crash copy).
7. `WM_CLOSE` → the unsaved-changes prompt names `recovery-fixture.lv`; UIA-invoke `Discard`; the
   process exits and the recovery directory is empty (`0 record(s) left`), while `desktop.json`
   still carries the session.
   → [`close-prompt-uia.txt`](close-prompt-uia.txt).

All three PNGs are 1600x1000 `PrintWindow` captures of the whole window (title bar included); each
was opened and read back for this report.

### Headless acceptance output

`lovelace-desktop --gui-selftest` with `APPDATA` redirected ([`gui-selftest.log`](gui-selftest.log)):

```
GUI_SELFTEST ok session-recovery: journaled "a = 1\nb = 3\n" (12 bytes) for the original path; the forced kill left it; the next launch restored the exact text and path without running or writing; discarding deleted it
GUI_SELFTEST ok recovery-corrupt: scan skipped 2 corrupt record(s) with a warning and recovered the valid one (12 bytes)
GUI_SELFTEST summary checks=19 failed=0
```

## 5. Suite summary

- `cargo test -p lovelace-desktop`: **347 passed, 0 failed** (318 before this round, 29 added).
- `--gui-selftest`: **19 checks, 0 failed** (17 before, 2 added).
- Window proof: **46 checks, 0 failed**, three 1600x1000 `PrintWindow` captures (the two required
  ones plus the launched state).
- `cargo check --all-targets` is warning-free for `lovelace-desktop`.

## 6. Decisions worth recording

- **Recent files:** a path is pruned only when it no longer exists; an unreadable-but-existing file
  stays in the list so Open can report the real failure.
- **Session:** a clean pathless buffer is not restored (it has no content to restore and no
  journal); a dirty pathless buffer is restored only through the journal. A file-backed tab is
  always restored from disk, and the journal's newer text is offered by the prompt.
- **Active tab:** mapping by entry identity, not by list index, so skipped entries cannot silently
  move the selection.
- **Journal ownership:** only records the running window wrote or adopted are deleted on a clean
  exit. A record the user kept for later survives the quit, so it is offered again next launch.
- **Recovery ordering:** while the recovery prompt is open, the window refuses close requests and
  strips every key/text event, so the journaled text is offered before any save can overwrite a
  file.

## 7. Files

| File | Change |
|---|---|
| `src/recovery.rs` | **new** — record format, atomic write, scan/skip, debounced journal with `pending_delay`, clean exit; 8 tests |
| `src/settings.rs` | session types and JSON, `prune_missing_recent`, `set_session`, `default_recovery_dir`; 5 tests |
| `src/workbench.rs` | `source_mtime`, `caret`, `selection`, `set_caret`, `set_recent_files`, `session_state`, `restore_session`, `recover`, `log_active`; 10 tests |
| `src/ui/app.rs` | `AppStorage`, launch restore, journal wiring and repaint scheduling, settings persistence, recovery prompt, clean-exit handling; 6 tests |
| `src/gui_selftest.rs` | `session-recovery` and `recovery-corrupt` checks (17 → 19) |
| `src/main.rs` | registers `recovery`; `--help` mentions restore and recovery |
| `docs/goal-desktop-v4/round-32/*` | capture script, logs, UIA dumps, PNGs, record copies |
