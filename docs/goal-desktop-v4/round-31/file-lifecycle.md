# Round 31 — DOC-002 / DOC-003: the native file lifecycle and the unsaved-work prompt

Scope: `rust/lovelace-desktop/**` and `docs/goal-desktop-v4/round-31/`. No engine crate, `tools/`
or gate file was modified.

## 1. What existed before this round

The new eframe/egui workbench (`rust/lovelace-desktop/src/ui/app.rs` over
`Workbench` in `workbench.rs`) had **no file lifecycle of its own**:

- A document could only be filled programmatically: `Workbench::load_file(index, path)` used
  `std::fs::read_to_string`, which drops a UTF-8 BOM into the text, errors out for a non-UTF-8
  file with a raw `io::Error` text, records nothing about line endings, and replaces the whole
  buffer with LF-only text after the editor's `lines.join("\n")`.
- There was no New, Open, Save, Save As or Close action, and no menu or toolbar control for any
  of them. The window's only file action was the plot export (SVG/PNG), which already used the
  comdlg32 Save dialog through `win32/ffi.rs`.
- The workbench could show a `*` dirty marker in the tab strip
  (`format!("{}{}", title, if dirty {" *"} else {""})`) because `WorkbenchDocument::dirty()`
  compares `source` to `saved_source` — but `saved_source` was only ever set by `load_file`, so
  the marker could never be cleared by saving, and no code path closed a tab or asked about
  unsaved work.
- `Ctrl+Enter`/`Esc` were the only keyboard commands (`keyboard::WorkbenchCommand`).
- `win32/ffi.rs` had `open_file_dialog`/`save_file_dialog_with`, but `open_file_dialog` was fixed
  to the legacy `.ls`/`.lvl` filter and could not be parameterised; the workbench never called it.
- The legacy Win32 shell (`--legacy-win32`, `src/win32/shell.rs`) did have Open/Save and a
  Yes/No/Cancel `MessageBoxW` close flow, but it is not the shipped UI and its prompt is not
  Save/Discard/Cancel.

## 2. What was added

### 2.1 The on-disk codec — `src/document_io.rs` (new)

`decode(bytes) -> Result<LoadedFile, DecodeError>` is the only way a file enters a document:

- a UTF-8 BOM is detected, recorded and removed from the text;
- a UTF-16 BOM (`FF FE` / `FE FF`) is refused with its own message;
- invalid UTF-8 is refused with the byte and the offset (`byte 0xE9 at offset 10 is not valid
  UTF-8; re-save it as UTF-8`), and the file is never rewritten;
- the text is kept **exactly as stored** (no normalisation, `\r` characters stay in the string);
- `NewlineStyle::{Lf, CrLf, Mixed{first}, None}` is detected from the text;
- the file's own bytes are returned beside the text, so a clean save can write them verbatim.

`encode(text, bom)` prefixes the BOM and writes the text as it stands. `write` reports the byte
count it wrote. The module owns the policy:

| Situation | Policy |
|---|---|
| File has a UTF-8 BOM | The BOM is recorded and written back |
| File uses CRLF | New lines use CRLF; an untouched file is written back byte-for-byte |
| File mixes endings | New lines use the first ending; a clean save writes the file's own bytes |
| File is UTF-16 or not UTF-8 | Refused with a message naming the byte, the offset and the fix; nothing is written |
| Buffer has no file (New) | LF, no BOM; Save As writes exactly the buffer |

### 2.2 The document model — `src/workbench.rs`

`WorkbenchDocument` gained `newline`, `bom` and a private `original_bytes`, plus
`title_with_marker()` (the one producer of `"name.lv"` / `"name.lv *"`) and `encoding_label()`
(`"UTF-8 BOM · CRLF"`). New methods on `Workbench`:

- `open(path)` — decodes first, then opens in the active tab when it is an untouched empty
  document, else in a new tab. A refusal logs and returns the message and creates no tab.
- `new_document()` — appends an empty `untitled[-N].lv` tab, selects it, inherits the active
  precision.
- `save(index)` — writes to the document's own path, or returns `SaveOutcome::NeedsPath`.
- `save_as(index, path)` — writes, then updates the path, the title, the newline style, the
  BOM flag and the recent list.
- `close_document(id)` — cancels the tab's in-flight run, removes the tab, keeps a neighbour
  selected and leaves a fresh `untitled.lv` when the last tab closes.
- `dirty_documents()` and `document_index(id)` — what the prompt asks about, by stable
  `DocumentId`, not by position.
- `recent_files()` — newest first, deduplicated, capped at `settings::MAX_RECENT_FILES`.

**Byte identity for an unmodified file.** `write_document` chooses the bytes:
a clean buffer writes `original_bytes` (the file's own bytes); an edited buffer writes
`encode(source, bom)`, and `source` still carries the file's line endings because the editor's
`source()` joins with the recorded separator. A save also re-detects the newline style and clears
`dirty()` by setting `saved_source`. The log line states the size and the encoding:
`Saved C:\...\crlf-bom.lv (90 bytes, UTF-8 BOM · CRLF).`

**Non-UTF-8 is refused, not mangled.** `open`/`load_file` return
`Cannot open <path>: the file is not UTF-8 text: byte 0xE9 at offset 10 is not valid UTF-8;
re-save it as UTF-8 (a Lovelace script is UTF-8 text).` — logged at `diagnostic` level, with no
tab created and no byte written.

### 2.3 The editor — `src/ui/editor.rs`

`TextEditor` gained `newline: NewlineStyle`, `from_source_with_newline` and
`load_with_newline`. A file's CRLF is split into lines without the `\r` (so the pane has no
phantom cell and the caret cannot sit after it) and `source()` re-joins with the document's own
separator. Editing a CRLF document therefore writes CRLF; editing an LF document writes LF. This
also removed a pre-existing display defect: a CRLF file used to paint a stray `\r` cell at the end
of every line.

### 2.4 The window — `src/ui/app.rs`, `src/ui/file_dialogs.rs`

- A **File bar** above the toolbar: `New`, `Open...`, `Save`, `Save As...`, `Recent` (a menu of
  the recent paths), `Close`, `Quit`, the active document's encoding badge and its path (or
  `no file yet`).
- The **window title** is `Lovelace Desktop - <name><marker>` and is updated through
  `ViewportCommand::Title` only when it changes.
- Native dialogs are behind the `FileDialogs` trait (`ui/file_dialogs.rs`): the shipped
  `NativeFileDialogs` calls `open_file_dialog_with` / `save_file_dialog_with` with the workbench
  window as owner, a `.lv;.ls;.lvl` Open filter, a `*.lv` Save filter and `lv` as the default
  extension. A headless test supplies `ScriptedDialogs` and exercises Open and Save As without a
  dialog. `win32/ffi.rs` gained `open_file_dialog_with` (title, filters, initial directory); the
  legacy shell keeps its old wrapper.
- **Editors follow tabs by identity** (`editor_ids`): New/Open/Close reconcile the editor list by
  `DocumentId`, so a surviving tab keeps its caret, selection and undo history.

**The unsaved-changes prompt (DOC-003).** An `egui::Modal` in the foreground layer, painted
before the panes but above them:

```
Unsaved changes
Save changes to crlf-bom.lv before closing?
If you discard, what you changed since the last save is lost.
[ Save ] [ Discard ] [ Cancel ]
```

- A dirty tab closed with Ctrl+W / Close, the window closed with the title-bar X / Alt+F4, and
  `Quit` all start the same `CloseTarget::{Tab(Id), Window}` flow. The OS close request is
  answered with `ViewportCommand::CancelClose` and only proceeds once every dirty document has
  been saved or discarded (`window_close_approved`).
- Several dirty documents are asked about one at a time, in tab order, with `Document <n> of
  <m>` on screen.
- **Save** writes the document (asking the Save As dialog when it has no path) and continues;
  a failed write keeps the prompt open with the error in it; a cancelled Save As dialog leaves
  the prompt open so Discard/Cancel are still reachable.
- **Discard** closes the tab / window without writing.
- **Cancel** drops the prompt and does nothing else: it never calls into an editor, so the
  buffer, the caret and the selection are byte-for-byte what they were. While the prompt is open
  every key and text event is stripped from the frame before the panes run, and the modal's layer
  blocks the editor, so a keystroke cannot reach the buffer. The focus that was taken when the
  modal appeared is re-requested on the passes after it closes, because egui only makes the
  editor interactable again once the modal layer is gone.
- Keyboard: `Escape` = Cancel, `Enter` = Save, the three buttons are UIA `Button` nodes named
  `Save`, `Discard`, `Cancel`.

### 2.5 Keyboard — `src/keyboard.rs`

New `FileCommand::{New, Open, Save, SaveAs, CloseTab}` and
`file_command_for_shortcut(letter, modifier, shift)`:

| Shortcut | Command |
|---|---|
| Ctrl+N | New |
| Ctrl+O | Open |
| Ctrl+S | Save |
| Ctrl+Shift+S | Save As |
| Ctrl+W | Close tab |

The window reads the same function; `--help` and the module docs list the table.

### 2.6 Headless acceptance — `src/gui_selftest.rs`

Two checks were added to the shipped acceptance mode (17 checks total, was 15):
`file-bom-crlf` and `file-refuse`. They drive `Workbench::open/save/set_source/close_document`,
so the headless proof goes through the same entry points as the window.

## 3. Tests and observed output

`cargo test -p lovelace-desktop` (log: [`cargo-test.log`](cargo-test.log)):

```
test result: ok. 318 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
```

318 = the 277 tests that passed on the same tree before this round + 41 new tests:

| Module | New tests | What they prove |
|---|---|---|
| `document_io` | 9 | BOM detected/stripped/written back; every newline style classified; mixed file keeps its own bytes; invalid UTF-8 refused with byte+offset; UTF-16 BOM refused; truncated sequence refused; on-disk BOM round-trip; on-disk non-UTF-8 refused with its path |
| `keyboard` | 3 | Ctrl+N/O/S, Ctrl+Shift+S and Ctrl+W map to the file commands; modifiers required; labels |
| `workbench` | 11 | BOM+CRLF recorded on open; unmodified CRLF save byte-identical; BOM kept on save; mixed-ending save byte-identical; dirty→save clears the marker and keeps CRLF; Save As updates path/title/recent; recent deduplicated; non-UTF-8 refused with no tab and no write; Open reuses an empty tab; New is untitled/selected/clean; closing a tab selects a neighbour and the last tab leaves a fresh document |
| `ui::editor` | 3 | A CRLF buffer round-trips and new lines use CRLF; an LF buffer stays LF; a CRLF line has no phantom column |
| `ui::file_dialogs` | 2 | The suggested name is a `.lv` name; both dialogs offer `.lv` |
| `ui::app` | 13 | Prompt names the document and offers Save/Discard/Cancel; dirty→cancel keeps buffer, caret, tab and focus and writes nothing; discard closes without writing; save-in-prompt writes then closes; a clean tab closes silently; window close refused with dirty work; window close allowed when clean; Quit from the File bar prompts and Discard approves the window close; Save on a pathless document asks for a path then closes; Ctrl+N opens, Ctrl+W closes a clean tab; Ctrl+Shift+S adopts the path and Ctrl+S writes to it (dirty marker clears); Ctrl+O opens the dialog's file with its CRLF preserved and the name in the tab; a non-UTF-8 file through the dialog is refused with no tab |

Notable assertions, verbatim from the new tests:

- BOM round-trip: `assert_eq!(encode(&loaded.text, loaded.bom), bytes)` and, through the
  workbench, `std::fs::read(&path) == b"\xEF\xBB\xBFa = 1\r\nb = 2\r\n"` after a save.
- CRLF byte identity: `assert_eq!(std::fs::read(&path), bytes)` for
  `b"a = 1\r\nb = 2\r\n"` after `save`, plus `keeps_every_line_crlf()` on the edited save.
- Dirty marker: `title_with_marker() == format!("{} *", title)` after an edit,
  `!dirty()` and the plain title after Save.
- Cancel: buffer `"a = 1\nb = 3\n"` unchanged, `caret == (1, 3)` unchanged, tab count unchanged,
  file bytes unchanged, and `memory.focused() == Some(surface_id())` after the modal closes.
- Non-UTF-8: `message.contains("not UTF-8") && message.contains("re-save it as UTF-8")`,
  `documents().len()` unchanged, file bytes unchanged, a `Diagnostic` line in the log.

## 4. Window proof — captures and method

The console session was **locked** for the whole capture (the foreground window was
`Windows Default Lock Screen`), so no hardware input was available. [`capture-file-lifecycle.ps1`](capture-file-lifecycle.ps1)
uses only:

- **UIA Invoke** (`InvokePattern`) to press the File bar's and the prompt's buttons;
- **PrintWindow with `PW_RENDERFULLCONTENT`** into a `System.Drawing.Bitmap` of
  `GetWindowRect`, so an occluded/locked-desktop window still renders;
- **posted `WM_KEYDOWN`** messages for the edit (winit resolves the character from the virtual
  key with the current layout and hands egui a real key/text event). A posted `WM_CHAR` would
  deliver the same character a second time — measured: one `WM_KEYDOWN`+`WM_CHAR` pair inserted
  2 bytes, one `WM_KEYDOWN` alone inserted 1 — so the script posts `WM_KEYDOWN` only;
- `SetWindowPos` to 1600x1000 for a legible capture.

`APPDATA` was redirected to [`appdata`](appdata) and the process working directory was
[`work`](work), outside the repository's working-directory check, so the resolved plot directory
(`round-31/appdata/Lovelace/plots`) is writable and not under the working directory. This is
required: without the redirect every plot statement fails with access denied (observed in the
first headless run, documented in the log).

The fixture is [`crlf-bom.lv`](crlf-bom.lv): 90 bytes, SHA-256
`D67CE20FD0B3E541CAEFD106307AA4848265F6C8FB49D9F0B88750DA37C72963`, a UTF-8 BOM and four CRLF
pairs. [`lf.lv`](lf.lv), [`mixed-newlines.lv`](mixed-newlines.lv) and [`non-utf8.lv`](non-utf8.lv)
are the other codec fixtures.

Sequence and result (29 checks, 0 failed — [`window-proof.json`](window-proof.json),
[`capture-run.log`](capture-run.log)):

1. Launch with the fixture path. Title `Lovelace Desktop - crlf-bom.lv`; the tab reads
   `crlf-bom.lv`; the File bar reads `UTF-8 BOM · CRLF` and the full path; the log line is
   `[info] Opened C:\...\crlf-bom.lv (87 bytes, UTF-8 BOM · CRLF).`
   → [`opened-file.png`](opened-file.png) (1600x1000, 199 680 bytes) and
   [`opened-file-uia.txt`](opened-file-uia.txt).
2. UIA-invoke `Save`. The SHA-256 of the file is **identical** before and after
   (`D67CE2…2963`), the log says `Saved … (90 bytes, UTF-8 BOM · CRLF).`, and the tab has no
   dirty marker. → [`after-unmodified-save-uia.txt`](after-unmodified-save-uia.txt).
3. Posted `VK_DOWN` keys (the status strip is parsed after each one, so the caret is on line 4
   before typing: `Ln 4, Col 2`), `VK_END`, then ` = 1`. The tab becomes `crlf-bom.lv *`, the
   window title `Lovelace Desktop - crlf-bom.lv *`, and the header grows from 83 to 87 bytes.
   → [`dirty-uia.txt`](dirty-uia.txt).
4. UIA-invoke the File bar's `Close`. The prompt appears, names the document and offers the three
   answers. → **[`unsaved-prompt.png`](unsaved-prompt.png) (1600x1000, 174 189 bytes)** and
   [`unsaved-prompt-uia.txt`](unsaved-prompt-uia.txt) (`Save changes to crlf-bom.lv before
   closing?`, `Button | Save`, `Button | Discard`, `Button | Cancel`).
5. UIA-invoke the prompt's `Cancel`. The tab is still `crlf-bom.lv *`, the buffer is still
   `5 lines / 87 bytes`, and the file's SHA-256 is still `D67CE2…2963`.
   → [`after-cancel.png`](after-cancel.png) and [`after-cancel-uia.txt`](after-cancel-uia.txt).
6. UIA-invoke `Close` again, then the prompt's `Discard`. The tab disappears, the app stays open,
   and the file's SHA-256 is unchanged. → [`after-discard.png`](after-discard.png),
   [`after-discard-uia.txt`](after-discard-uia.txt).
7. Type `z = 1` into the remaining `untitled.lv` tab (posted keys), post `WM_CLOSE`: the window
   stays open and the prompt names `untitled.lv`. → **[`window-close-prompt.png`](window-close-prompt.png)**
   (1600x1000, 139 073 bytes) and [`window-close-prompt-uia.txt`](window-close-prompt-uia.txt).
   UIA-invoke `Discard`; the process exits.

All five PNGs are 1600x1000 `PrintWindow` captures of the whole window (title bar included); each
was opened and read back for this report. Note for a future UIA driver: the window's system Close
button is the **first** UIA `Button` named `Close` (the File bar's is the second); the script
presses the last match on purpose, because the first one sends `WM_CLOSE` (the window-close path).

### Headless acceptance output

`lovelace-desktop --gui-selftest` with `APPDATA` redirected ([`gui-selftest.log`](gui-selftest.log)):

```
GUI_SELFTEST ok file-bom-crlf: UTF-8 BOM · CRLF opened in tab 2 (CRLF); unmodified save byte-identical (17 bytes); dirty marker cleared by save; edited save kept BOM and CRLF; tab closed
GUI_SELFTEST ok file-refuse: refused with "Cannot open ...\non-utf8.lv: the file is not UTF-8 text: byte 0xE9 at offset 10 is not valid UTF-8; re-save it as UTF-8 (a Lovelace script is UTF-8 text)."; tab count kept at 2; file bytes untouched (12 bytes)
GUI_SELFTEST summary checks=17 failed=0
```

## 5. Suite summary

- `cargo test -p lovelace-desktop`: **318 passed, 0 failed** (277 before this round, 41 added).
- `--gui-selftest`: **17 checks, 0 failed** (15 before, 2 added).
- Window proof: **29 checks, 0 failed**, five 1600x1000 `PrintWindow` captures.
- `cargo check --all-targets` is warning-free for `lovelace-desktop`.

## 6. Scope, and one shared-tree note

All product changes are inside `rust/lovelace-desktop/**`; all evidence is inside
`docs/goal-desktop-v4/round-31/`.

While this round was in progress another agent committed an engine change (`cde88f9`) that adds
`Expr::Complex(Rat, Rat)` to `lovelace-symbolics`. The desktop crate's symbolic inspector matches
that enum exhaustively, so `rust/lovelace-desktop/src/engine/inspection.rs` gained the leaf arm
`Expr::Complex(_, _) => ("Complex", None, Vec::new())` — the minimal in-scope adaptation, no
engine file touched. The parent was notified when the shared tree was briefly unbuildable.

## 7. Files

| File | Change |
|---|---|
| `src/document_io.rs` | **new** — UTF-8/BOM/newline codec and its 9 tests |
| `src/ui/file_dialogs.rs` | **new** — `FileDialogs` trait, native implementation, `.lv` filters |
| `src/workbench.rs` | document `newline`/`bom`/`original_bytes`, `open`, `new_document`, `save`, `save_as`, `close_document`, `dirty_documents`, recent list, title marker; 11 tests |
| `src/ui/editor.rs` | per-document newline, CRLF-aware load/serialise; 3 tests |
| `src/ui/app.rs` | File bar, Recent menu, window title, prompt + close flow, editor reconciliation, file shortcuts; 13 tests |
| `src/ui/mod.rs` | registers `file_dialogs` |
| `src/keyboard.rs` | `FileCommand` and the Ctrl+N/O/S, Ctrl+Shift+S, Ctrl+W mapping; 3 tests |
| `src/win32/ffi.rs` | `open_file_dialog_with`; the old open dialog delegates to it |
| `src/main.rs` | `--help` shortcut table |
| `src/gui_selftest.rs` | `file-bom-crlf` and `file-refuse` checks |
| `src/engine/inspection.rs` | one match arm for the engine's `Expr::Complex` leaf |
