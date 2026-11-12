# Round 1 — GUI census of the current native Win32 window

Scope read in full: [shell.rs](../../../rust/lovelace-desktop/src/win32/shell.rs) (961 lines),
[ffi.rs](../../../rust/lovelace-desktop/src/win32/ffi.rs) (1210 lines),
[mod.rs](../../../rust/lovelace-desktop/src/win32/mod.rs) (8 lines). No other file was opened; the
window was not executed or inspected at runtime. Quotes reproduce the exact line with leading
indentation trimmed; `\|` inside a quote is a Markdown escape and the source has a single `|`.
Absence statements are valid for these three files only, not for the whole process.

Subject order in the OBS table: FFI surface, window classes, top-level window, child controls,
layout, painting, commands/menus, message routing, engine/threading, status bar, settings, tests.

## 1. OBS table

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-F-01 | mod.rs:7 | The module tree exposes exactly two submodules; `pub mod ffi;` declares the FFI module. | High |
| OBS-F-02 | mod.rs:8 | `pub mod shell;` declares the shell module. | High |
| OBS-F-03 | mod.rs:5 | The module doc states: `//! of those wrappers. Nothing outside this directory contains unsafe code.` | High (as written) |
| OBS-F-04 | shell.rs:23 | The shell module opts into unsafe code: `#![allow(unsafe_code)]`. | High |
| OBS-F-05 | ffi.rs:18 | The FFI module opts into unsafe code: `#![allow(unsafe_code)]`. | High |
| OBS-F-06 | ffi.rs:23 | Unused declarations are permitted: `#![allow(dead_code)]`. | High |
| OBS-F-07 | ffi.rs:3 | The FFI doc states the crate has no external dependencies: `//! The workspace builds with no external crates, so there is no windows-sys here: the module`. | High (as written) |
| OBS-F-08 | ffi.rs:442 | The first linked library is user32: `#[link(name = "user32")]`. | High |
| OBS-F-09 | ffi.rs:530 | The second linked library is gdi32: `#[link(name = "gdi32")]`. | High |
| OBS-F-10 | ffi.rs:574 | The third linked library is comctl32: `#[link(name = "comctl32")]`. | High |
| OBS-F-11 | ffi.rs:581 | The fourth linked library is comdlg32: `#[link(name = "comdlg32")]`. | High |
| OBS-F-12 | ffi.rs:590 | The fifth linked library is kernel32: `#[link(name = "kernel32")]`. | High |
| OBS-F-13 | ffi.rs:29 | Handles are pointer-sized integers: `pub type HWND = isize;`. | High |
| OBS-F-14 | ffi.rs:47 | `pub type WPARAM = usize;`. | High |
| OBS-F-15 | ffi.rs:49 | `pub type LPARAM = isize;`. | High |
| OBS-F-16 | ffi.rs:51 | `pub type LRESULT = isize;`. | High |
| OBS-F-17 | ffi.rs:53 | The window-procedure type is `Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>`. | High |
| OBS-F-18 | shell.rs:25 | The shell imports `Cell` and `RefCell` only: `use std::cell::{Cell, RefCell};`; no thread-spawn, thread-handle or channel API appears in either scoped file. | High |
| OBS-F-19 | ffi.rs:601 | All strings handed to Win32 are converted to NUL-terminated UTF-16: `pub fn wide(text: &str) -> Vec<u16> {`. | High |
| OBS-F-20 | ffi.rs:608 | Strings read back from Win32 are decoded lossily: `pub fn from_wide(buffer: &[u16]) -> String {`. | High |
| OBS-F-21 | ffi.rs:654 | The generic send helper ignores a zero window: `pub fn send(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {` (zero check at line 655). | High |
| OBS-F-22 | ffi.rs:675 | Client size is measured by the Win32 client-rect call: `pub fn client_size(hwnd: HWND) -> (i32, i32) {`. | High |
| OBS-F-23 | ffi.rs:689 | Window size is measured by the Win32 window-rect call: `pub fn window_size(hwnd: HWND) -> (i32, i32) {`. | High |
| OBS-F-24 | ffi.rs:709 | Every move repaints: `MoveWindow(hwnd, x, y, width.max(0), height.max(0), 1);`. | High |
| OBS-F-25 | ffi.rs:715 | Fonts are applied by message, with redraw enabled: `send(hwnd, WM_SETFONT, font as WPARAM, 1);`. | High |
| OBS-F-26 | ffi.rs:725 | Invalidation covers the whole client area and does not erase: `InvalidateRect(hwnd, std::ptr::null(), 0);`. | High |
| OBS-F-27 | ffi.rs:738 | The status bar is written through part 0: `SendMessageW(hwnd, SB_SETTEXTW, 0, text.as_ptr() as LPARAM);`. | High |
| OBS-F-28 | ffi.rs:747 | The caret query masks the low word: `(result as usize) & 0xFFFF`. | High |
| OBS-F-29 | ffi.rs:760 | Scrolling a read-only edit control is done by moving its selection to the end: `send(hwnd, EM_SETSEL, length, length as isize);`. | High |
| OBS-F-30 | ffi.rs:768 | The timer is a message timer with no callback: `SetTimer(hwnd, id, milliseconds, 0);`. | High |
| OBS-F-31 | ffi.rs:790 | The class cursor is loaded as the standard arrow (IDC_ARROW, 32512): `LoadCursorW(0, 32512usize as *const u16)`. | High |
| OBS-F-32 | ffi.rs:797 | Common controls are initialised for bars only: `dwICC: ICC_BAR_CLASSES,`. | High |
| OBS-F-33 | ffi.rs:150 | The bar control group is `pub const ICC_BAR_CLASSES: u32 = 0x0000_0004;`. | High |
| OBS-F-34 | ffi.rs:820 | Class registration reports success as a non-zero atom: `unsafe { RegisterClassW(&class) != 0 }`. | High |
| OBS-F-35 | ffi.rs:845 | Window creation is centralised in `CreateWindowExW(`; the creation parameter is null at line 857. | High |
| OBS-F-36 | ffi.rs:927 | Fonts are created by `CreateFontW` with the face name and weight supplied by the caller: `CreateFontW(` (call body lines 927-942). | High |
| OBS-F-37 | ffi.rs:955 | Pens are solid: `unsafe { CreatePen(0, width, color) }` (comment at line 954 records PS_SOLID = 0). | High |
| OBS-F-38 | ffi.rs:1140 | The Open dialog title is `let title = wide("Open Lovelace script");`. | High |
| OBS-F-39 | ffi.rs:1137 | The Open dialog filter is `"Lovelace scripts (*.ls)\0*.ls;*.lvl\0All files (*.*)\0*.*\0\0"`. | High |
| OBS-F-40 | ffi.rs:1141 | The Open dialog file buffer is `let mut file = vec![0u16; 2048];`. | High |
| OBS-F-41 | ffi.rs:1175 | The Save dialog title is `let title = wide("Save Lovelace script");`. | High |
| OBS-F-42 | ffi.rs:1176 | The Save dialog default extension is `let extension = wide("ls");`. | High |
| OBS-F-43 | ffi.rs:1191 | The Save dialog flags are `Flags: OFN_OVERWRITEPROMPT \| OFN_PATHMUSTEXIST \| OFN_EXPLORER \| OFN_NOCHANGEDIR,`. | High |
| OBS-WC-01 | shell.rs:34 | The main window class name is `const WINDOW_CLASS: &str = "LovelaceDesktopMainWindow";`. | High |
| OBS-WC-02 | shell.rs:36 | The plot window class name is `const PLOT_CLASS: &str = "LovelaceDesktopPlotWindow";`. | High |
| OBS-WC-03 | shell.rs:465 | The main class is registered with `main_window_proc` and the system window brush: `if !register_class(WINDOW_CLASS, Some(main_window_proc), COLOR_WINDOW_BRUSH)`. | High |
| OBS-WC-04 | shell.rs:466 | The plot class is registered with `plot_window_proc` and the same brush: `\|\| !register_class(PLOT_CLASS, Some(plot_window_proc), COLOR_WINDOW_BRUSH)`. | High |
| OBS-WC-05 | ffi.rs:807 | Class style is `style: CS_HREDRAW \| CS_VREDRAW,`. | High |
| OBS-WC-06 | ffi.rs:812 | The class icon is null: `hIcon: 0,`. | High |
| OBS-WC-07 | ffi.rs:813 | The class cursor is the standard arrow: `hCursor: load_arrow_cursor(),`. | High |
| OBS-WC-08 | ffi.rs:814 | The class background is the caller's brush: `hbrBackground: background,`. | High |
| OBS-WC-09 | ffi.rs:815 | No menu resource is named on the class: `lpszMenuName: std::ptr::null(),`. | High |
| OBS-WC-10 | ffi.rs:214 | `COLOR_WINDOW_BRUSH` is the index-plus-one for the system window colour: `pub const COLOR_WINDOW_BRUSH: isize = 6;`. | High |
| OBS-WC-11 | shell.rs:474 | A failed class registration exits with code 1: `return 1;` (inside the check at lines 465-475). | High |
| OBS-TL-01 | shell.rs:477 | The menu is built before the main window: `let menu = build_menu();`. | High |
| OBS-TL-02 | shell.rs:478 | Initial size comes from settings: `let geometry = app.settings.window;`. | High |
| OBS-TL-03 | shell.rs:479 | Initial title comes from the application: `let title = app.window_title();`. | High |
| OBS-TL-04 | shell.rs:480 | The main window is created with `create_window`: `let hwnd = create_window(`. | High |
| OBS-TL-05 | shell.rs:481 | The main window has a taskbar button: `WS_EX_APPWINDOW,`. | High |
| OBS-TL-06 | shell.rs:484 | The main style is `WS_OVERLAPPEDWINDOW \| WS_CLIPCHILDREN,`. | High |
| OBS-TL-07 | ffi.rs:60 | `WS_OVERLAPPEDWINDOW` is `0x00CF_0000`: caption, system menu, thick frame, minimise and maximise boxes. | High |
| OBS-TL-08 | ffi.rs:66 | `WS_CLIPCHILDREN` is `0x0200_0000`. | High |
| OBS-TL-09 | ffi.rs:78 | `WS_EX_APPWINDOW` is `0x0004_0000`. | High |
| OBS-TL-10 | shell.rs:485 | The x position is `CW_USEDEFAULT,`. | High |
| OBS-TL-11 | shell.rs:486 | The y position is `CW_USEDEFAULT,`. | High |
| OBS-TL-12 | ffi.rs:101 | `pub const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;`. | High |
| OBS-TL-13 | shell.rs:487 | The initial width is `geometry.width,`. | High |
| OBS-TL-14 | shell.rs:488 | The initial height is `geometry.height,`. | High |
| OBS-TL-15 | shell.rs:489 | The parent handle is `0,`, making the window top-level. | High |
| OBS-TL-16 | shell.rs:490 | The menu bar is attached in the create call: `menu,`. | High |
| OBS-TL-17 | shell.rs:491 | The instance handle is `module_handle(),`. | High |
| OBS-TL-18 | shell.rs:496 | The create-failure message is `"Could not create the main window.",`. | High |
| OBS-TL-19 | shell.rs:503 | The shell state is stored in the thread-local before children exist: `SHELL.with(\|cell\| *cell.borrow_mut() = Some(Shell::new(hwnd, app, settings_path)));`. | High |
| OBS-TL-20 | shell.rs:505 | Startup order inside one borrow is children, accelerators, layout, refresh: `shell.create_children();`. | High |
| OBS-TL-21 | shell.rs:508 | The final startup step is `shell.refresh_all();`. | High |
| OBS-TL-22 | shell.rs:512 | If shell setup fails the window is destroyed: `destroy_window(hwnd);`. | High |
| OBS-TL-23 | shell.rs:516 | The window is shown only after children and layout: `show_window(hwnd);`. | High |
| OBS-TL-24 | shell.rs:517 | Pending paints are flushed with `update_window(hwnd);`. | High |
| OBS-TL-25 | shell.rs:518 | A 250 ms timer is started: `set_timer(hwnd, TIMER_STATUS, 250);`. | High |
| OBS-TL-26 | shell.rs:61 | The timer id is `const TIMER_STATUS: usize = 1;`. | High |
| OBS-TL-27 | shell.rs:666 | Minimum tracking size is 900×600: `(*info).ptMinTrackSize = POINT { x: 900, y: 600 };`. | High |
| OBS-TL-28 | shell.rs:671 | `WM_SETFOCUS` is handled: `WM_SETFOCUS => {`. | High |
| OBS-TL-29 | shell.rs:673 | Gaining focus moves focus to the editor: `set_focus(editor);`. | High |
| OBS-TL-30 | shell.rs:678 | Unhandled messages go to the default procedure: `def_window_proc(hwnd, message, wparam, lparam)`. | High |
| OBS-TL-31 | shell.rs:676 | No other message has an arm in the main procedure: `_ => {}` (arms are only WM_SIZE, WM_COMMAND, WM_TIMER, WM_CLOSE, WM_DESTROY, WM_GETMINMAXINFO, WM_SETFOCUS). | High |
| OBS-TL-32 | shell.rs:295 | The title is refreshed from `self.app.window_title()`: `let title = self.app.window_title();`. | High |
| OBS-TL-33 | shell.rs:296 | The frame text is only rewritten when it differs: `if window_text(self.hwnd) != title {` (set_text at line 297). | High |
| OBS-CH-01 | shell.rs:54 | `const ID_EDITOR: u16 = 1000;`. | High |
| OBS-CH-02 | shell.rs:55 | `const ID_VARIABLES: u16 = 1001;`. | High |
| OBS-CH-03 | shell.rs:56 | `const ID_LOGS: u16 = 1002;`. | High |
| OBS-CH-04 | shell.rs:57 | `const ID_PLOT: u16 = 1003;`. | High |
| OBS-CH-05 | shell.rs:58 | `const ID_STATUS: u16 = 1004;`. | High |
| OBS-CH-06 | shell.rs:108 | All children are created by `fn create_children(&mut self) {`. | High |
| OBS-CH-07 | shell.rs:121 | The editor style starts `let editor_style = WS_CHILD`. | High |
| OBS-CH-08 | shell.rs:123 | The editor has a vertical scroll bar: `\| WS_VSCROLL`. | High |
| OBS-CH-09 | shell.rs:124 | The editor has a horizontal scroll bar: `\| WS_HSCROLL`. | High |
| OBS-CH-10 | shell.rs:125 | The editor participates in tab navigation: `\| WS_TABSTOP` (constant doc at ffi.rs:72). | High |
| OBS-CH-11 | shell.rs:127 | The editor is multi-line: `\| ES_MULTILINE`. | High |
| OBS-CH-12 | shell.rs:129 | The editor scrolls horizontally with the caret: `\| ES_AUTOHSCROLL`. | High |
| OBS-CH-13 | shell.rs:130 | Enter inserts a carriage return: `\| ES_WANTRETURN`. | High |
| OBS-CH-14 | shell.rs:131 | The selection stays visible without focus: `\| ES_NOHIDESEL`. | High |
| OBS-CH-15 | shell.rs:133 | The editor has a client edge: `WS_EX_CLIENTEDGE,`. | High |
| OBS-CH-16 | shell.rs:134 | The editor class is the system edit control: `"EDIT",`. | High |
| OBS-CH-17 | shell.rs:142 | The editor id is passed at creation: `ID_EDITOR as isize,`. | High |
| OBS-CH-18 | shell.rs:206 | The editor text limit is 0x7FFFFFFE: `send(self.editor, EM_SETLIMITTEXT, 0x7FFF_FFFE, 0);`. | High |
| OBS-CH-19 | shell.rs:207 | The editor font is the monospace font: `set_font(self.editor, self.font_mono);`. | High |
| OBS-CH-20 | shell.rs:208 | The variables pane font is the UI font: `set_font(self.variables, self.font_ui);`. | High |
| OBS-CH-21 | shell.rs:209 | The logs pane font is the UI font: `set_font(self.logs, self.font_ui);`. | High |
| OBS-CH-22 | shell.rs:146 | The pane style starts `let pane_style = WS_CHILD`. | High |
| OBS-CH-23 | shell.rs:148 | Read-only panes have only a vertical scroll bar: `\| WS_VSCROLL`. | High |
| OBS-CH-24 | shell.rs:150 | Read-only panes are multi-line: `\| ES_MULTILINE`. | High |
| OBS-CH-25 | shell.rs:152 | Read-only panes are read-only: `\| ES_READONLY`. | High |
| OBS-CH-26 | shell.rs:155 | The variables pane class is `"EDIT",`. | High |
| OBS-CH-27 | shell.rs:163 | The variables pane id is `ID_VARIABLES as isize,`. | High |
| OBS-CH-28 | shell.rs:168 | The logs pane class is `"EDIT",`. | High |
| OBS-CH-29 | shell.rs:176 | The logs pane id is `ID_LOGS as isize,`. | High |
| OBS-CH-30 | shell.rs:181 | The plot pane class is the custom class: `PLOT_CLASS,`. | High |
| OBS-CH-31 | shell.rs:183 | The plot style is `WS_CHILD \| WS_VISIBLE \| WS_BORDER,`. | High |
| OBS-CH-32 | shell.rs:189 | The plot id is `ID_PLOT as isize,`. | High |
| OBS-CH-33 | shell.rs:193 | The status bar has no extended style: `0,`. | High |
| OBS-CH-34 | shell.rs:194 | The status bar uses the common-controls class: `STATUSCLASSNAME,`. | High |
| OBS-CH-35 | shell.rs:196 | The status style is `WS_CHILD \| WS_VISIBLE \| SBARS_SIZEGRIP,`. | High |
| OBS-CH-36 | shell.rs:202 | The status id is `ID_STATUS as isize,`. | High |
| OBS-CH-37 | ffi.rs:148 | `pub const STATUSCLASSNAME: &str = "msctls_statusbar32";`. | High |
| OBS-CH-38 | ffi.rs:146 | `pub const SBARS_SIZEGRIP: u32 = 0x0100;`. | High |
| OBS-CH-39 | shell.rs:110 | The monospace font is `create_font(-16, FW_NORMAL, "Consolas")`. | High |
| OBS-CH-40 | shell.rs:111 | The UI font is `create_font(-14, FW_NORMAL, "Segoe UI")`. | High |
| OBS-CH-41 | shell.rs:112 | The title font is `create_font(-18, FW_SEMIBOLD, "Segoe UI")`. | High |
| OBS-CH-42 | ffi.rs:207 | `FW_NORMAL` is 400: `pub const FW_NORMAL: i32 = 400;` (FW_SEMIBOLD at line 209 is 600). | High |
| OBS-CH-43 | ffi.rs:211 | Fonts use `DEFAULT_CHARSET` = 1. | High |
| OBS-CH-44 | shell.rs:113 | The plot background brush is white: `self.brush_white = create_brush(0x00FF_FFFF);`. | High |
| OBS-CH-45 | shell.rs:115 | Pen 0 is `create_pen(2, 0x001F_4E79),`. | High |
| OBS-CH-46 | shell.rs:116 | Pen 1 is `create_pen(2, 0x00C0_5034),`. | High |
| OBS-CH-47 | shell.rs:117 | Pen 2 is `create_pen(2, 0x0038_7638),`. | High |
| OBS-CH-48 | shell.rs:118 | Pen 3 is `create_pen(2, 0x0080_60A0),`. | High |
| OBS-CH-49 | shell.rs:207 | Fonts are set on editor, variables and logs only: `set_font(self.editor, self.font_mono);`; no set_font call targets the status bar, plot or frame window. | High |
| OBS-CH-50 | shell.rs:108 | All children are created in `fn create_children(&mut self) {`; exactly five child windows exist and no toolbar class or creation call appears in either scoped file. | High |
| OBS-LAY-01 | shell.rs:227 | Layout is computed in `fn layout(&mut self) {`. | High |
| OBS-LAY-02 | shell.rs:228 | Layout is skipped until the status bar exists: `if self.status == 0 {`. | High |
| OBS-LAY-03 | shell.rs:231 | Layout starts from the client area: `let (width, height) = client_size(self.hwnd);`. | High |
| OBS-LAY-04 | shell.rs:232 | Zero-size clients skip layout: `if width <= 0 \|\| height <= 0 {`. | High |
| OBS-LAY-05 | shell.rs:237 | The status bar is first told to size itself: `send(self.status, WM_SIZE, 0, 0);`. | High |
| OBS-LAY-06 | shell.rs:236 | Code comment states the status bar moves itself to the bottom on WM_SIZE: `// WM_SIZE, so ask it to do that first and then measure the strip.` | High (as written) |
| OBS-LAY-07 | shell.rs:238 | Status strip height is measured by outer window size, minimum 22: `let status_height = window_size(self.status).1.max(22);`. | High |
| OBS-LAY-08 | shell.rs:239 | Content height is the client height minus the status strip: `let content_height = (height - status_height).max(0);`. | High |
| OBS-LAY-09 | shell.rs:240 | The application is told the layout input: `self.app.apply_layout(width, content_height);`. | High |
| OBS-LAY-10 | shell.rs:241 | Pane sizes come from `compute_layout` using settings: `let sizes = compute_layout(width, content_height, &self.app.settings.layout);`. | High |
| OBS-LAY-11 | shell.rs:242 | The computed sizes are stored: `self.app.layout = sizes;`. | High |
| OBS-LAY-12 | shell.rs:244 | The editor is moved to the top-left with the editor size: `move_window(self.editor, 0, 0, sizes.editor_w, sizes.editor_h);`. | High |
| OBS-LAY-13 | shell.rs:247 | Plot x is 0: `0,`. | High |
| OBS-LAY-14 | shell.rs:248 | Plot y is below the editor plus one gap: `sizes.editor_h + PANE_GAP,`. | High |
| OBS-LAY-15 | shell.rs:249 | Plot width equals the editor width: `sizes.editor_w,`. | High |
| OBS-LAY-16 | shell.rs:250 | Plot height is `sizes.plot_h,`. | High |
| OBS-LAY-17 | shell.rs:254 | Variables x is right of the editor plus one gap: `sizes.editor_w + PANE_GAP,`. | High |
| OBS-LAY-18 | shell.rs:255 | Variables y is 0: `0,`. | High |
| OBS-LAY-19 | shell.rs:256 | Variables width is `sizes.right_w,`. | High |
| OBS-LAY-20 | shell.rs:257 | Variables height spans editor, gap and plot: `sizes.editor_h + PANE_GAP + sizes.plot_h,`. | High |
| OBS-LAY-21 | shell.rs:261 | Logs x is 0: `0,`. | High |
| OBS-LAY-22 | shell.rs:262 | Logs y is below editor, plot and two gaps: `sizes.editor_h + PANE_GAP + sizes.plot_h + PANE_GAP,`. | High |
| OBS-LAY-23 | shell.rs:263 | Logs width is the full client width: `width,`. | High |
| OBS-LAY-24 | shell.rs:264 | Logs height is `sizes.logs_h,`. | High |
| OBS-LAY-25 | shell.rs:266 | The plot is invalidated after every layout: `invalidate(self.plot);`. | High |
| OBS-LAY-26 | shell.rs:613 | A resize re-runs layout: `with_shell(\|shell\| shell.layout());` under the WM_SIZE arm. | High |
| OBS-LAY-27 | shell.rs:28 | The gap is imported from outside scope and its value is not in these files: `use crate::app::{AppState, Command, Effect, LogLevel, PANE_GAP, compute_layout};`. | High |
| OBS-PNT-01 | shell.rs:692 | The plot handles background erase: `WM_ERASEBKGND => {`. | High |
| OBS-PNT-02 | shell.rs:695 | Erase fills the whole client area with the white brush: `fill_rect(` with a rect spanning (0,0)-(width,height) (lines 697-702). | High |
| OBS-PNT-03 | shell.rs:705 | The plot reports the background as erased: `return 1;`. | High |
| OBS-PNT-04 | shell.rs:707 | Paint is handled by `WM_PAINT => {`. | High |
| OBS-PNT-05 | shell.rs:708 | Paint delegates to `paint_plot(hwnd);`. | High |
| OBS-PNT-06 | shell.rs:717 | Painting begins with: `let (hdc, paint) = paint_begin(hwnd);`. | High |
| OBS-PNT-07 | shell.rs:735 | Paint re-fills the area white: `fill_rect(hdc, &area, brush);`. | High |
| OBS-PNT-08 | shell.rs:737 | Only a non-empty plot is drawn: `Some(plot) if !plot.is_empty() => {`. | High |
| OBS-PNT-09 | shell.rs:741 | Placeholder text uses a transparent background: `set_bk_mode(hdc, TRANSPARENT);`. | High |
| OBS-PNT-10 | shell.rs:742 | Placeholder text is grey 0x00808080: `set_text_color(hdc, 0x0080_8080);`. | High |
| OBS-PNT-11 | shell.rs:743 | Placeholder text is `"no plot yet"` at (16,16): `text_out(hdc, 16, 16, "no plot yet");`. | High |
| OBS-PNT-12 | shell.rs:746 | Painting ends with `paint_end(hwnd, &paint);`. | High |
| OBS-PNT-13 | shell.rs:759 | Drawing is skipped for tiny or invalid plot sizes: `if width < 4 \|\| height < 4 \|\| plot.width <= 0 \|\| plot.height <= 0 {`. | High |
| OBS-PNT-14 | shell.rs:762 | X is scaled by client width over plot width: `let to_x = \|x: i64\| -> i32 { ((x * width as i64) / plot.width) as i32 };`. | High |
| OBS-PNT-15 | shell.rs:763 | Y is scaled by client height over plot height: `let to_y = \|y: i64\| -> i32 { ((y * height as i64) / plot.height) as i32 };`. | High |
| OBS-PNT-16 | shell.rs:765 | Polylines are drawn in document order: `for (index, line) in plot.polylines.iter().enumerate() {`. | High |
| OBS-PNT-17 | shell.rs:766 | A polyline needs at least two points: `if line.points.len() < 2 {`. | High |
| OBS-PNT-18 | shell.rs:780 | Pen selection cycles through the four pens by index: `let previous = select_object(hdc, pens[index % pens.len()]);`. | High |
| OBS-PNT-19 | shell.rs:786 | Text background is transparent: `set_bk_mode(hdc, TRANSPARENT);`. | High |
| OBS-PNT-20 | shell.rs:787 | Text is dark grey 0x00202020: `set_text_color(hdc, 0x0020_2020);`. | High |
| OBS-PNT-21 | shell.rs:789 | Text height is scaled and clamped to 9-28 px: `let size = ((item.size * height as i64) / plot.height).clamp(9, 28) as i32;`. | High |
| OBS-PNT-22 | shell.rs:790 | Bold text uses the title font, other text the UI font: `let font = if item.bold { font_title } else { font_ui };`. | High |
| OBS-PNT-23 | shell.rs:797 | Middle-anchored text is shifted by half its measured width: `TextAnchor::Middle => x -= text_width(hdc, &item.text) / 2,`. | High |
| OBS-PNT-24 | shell.rs:798 | End-anchored text is shifted by its full measured width: `TextAnchor::End => x -= text_width(hdc, &item.text),`. | High |
| OBS-PNT-25 | shell.rs:801 | Text baseline offset is 4/5 of the scaled size: `let top = to_y(item.y) - (size * 4) / 5;`. | High |
| OBS-PNT-26 | shell.rs:802 | Text coordinates are clamped to zero: `text_out(hdc, x.max(0), top.max(0), &item.text);`. | High |
| OBS-PNT-27 | shell.rs:807 | A title already present as text is not drawn twice: `let already_drawn = plot.texts.iter().any(\|item\| &item.text == title);`. | High |
| OBS-PNT-28 | shell.rs:810 | The title colour is 0x00101010: `set_text_color(hdc, 0x0010_1010);`. | High |
| OBS-PNT-29 | shell.rs:812 | The title is centred at y = 4: `text_out(hdc, ((width - text) / 2).max(0), 4, title);`. | High |
| OBS-PNT-30 | ffi.rs:205 | `TRANSPARENT` is 1: `pub const TRANSPARENT: i32 = 1;`. | High |
| OBS-PNT-31 | shell.rs:676 | The message match ends with `_ => {}`; no WM_CTLCOLOR message arm exists, so edit/status colours are not set by this file. | High |
| OBS-CMD-01 | shell.rs:40 | `const CMD_NEW: u16 = 101;`. | High |
| OBS-CMD-02 | shell.rs:41 | `const CMD_OPEN: u16 = 102;`. | High |
| OBS-CMD-03 | shell.rs:42 | `const CMD_SAVE: u16 = 103;`. | High |
| OBS-CMD-04 | shell.rs:43 | `const CMD_SAVE_AS: u16 = 104;`. | High |
| OBS-CMD-05 | shell.rs:44 | `const CMD_CLOSE: u16 = 105;`. | High |
| OBS-CMD-06 | shell.rs:45 | `const CMD_EXIT: u16 = 109;`. | High |
| OBS-CMD-07 | shell.rs:46 | `const CMD_UNDO: u16 = 111;`. | High |
| OBS-CMD-08 | shell.rs:47 | `const CMD_REDO: u16 = 112;`. | High |
| OBS-CMD-09 | shell.rs:48 | `const CMD_CLEAR: u16 = 115;`. | High |
| OBS-CMD-10 | shell.rs:49 | `const CMD_RUN: u16 = 121;`. | High |
| OBS-CMD-11 | shell.rs:50 | `const CMD_CANCEL: u16 = 122;`. | High |
| OBS-CMD-12 | shell.rs:51 | `const CMD_ABOUT: u16 = 131;`. | High |
| OBS-CMD-13 | shell.rs:530 | The menu bar is built by `fn build_menu() -> HMENU {`. | High |
| OBS-CMD-14 | shell.rs:532 | File → New: `append_menu(file, MF_STRING, CMD_NEW as usize, "&New\tCtrl+N");`. | High |
| OBS-CMD-15 | shell.rs:533 | File → Open: `append_menu(file, MF_STRING, CMD_OPEN as usize, "&Open...\tCtrl+O");`. | High |
| OBS-CMD-16 | shell.rs:534 | File → Save: `append_menu(file, MF_STRING, CMD_SAVE as usize, "&Save\tCtrl+S");`. | High |
| OBS-CMD-17 | shell.rs:539 | File → Save As: `"Save &As...\tCtrl+Shift+S",`. | High |
| OBS-CMD-18 | shell.rs:541 | A separator follows Save As: `append_menu(file, MF_SEPARATOR, 0, "");`. | High |
| OBS-CMD-19 | shell.rs:542 | File → Close Tab: `append_menu(file, MF_STRING, CMD_CLOSE as usize, "&Close Tab\tCtrl+W");`. | High |
| OBS-CMD-20 | shell.rs:543 | A second separator follows Close Tab: `append_menu(file, MF_SEPARATOR, 0, "");`. | High |
| OBS-CMD-21 | shell.rs:544 | File → Exit: `append_menu(file, MF_STRING, CMD_EXIT as usize, "E&xit");`. | High |
| OBS-CMD-22 | shell.rs:547 | Edit → Undo: `append_menu(edit, MF_STRING, CMD_UNDO as usize, "&Undo\tCtrl+Z");`. | High |
| OBS-CMD-23 | shell.rs:548 | Edit → Redo: `append_menu(edit, MF_STRING, CMD_REDO as usize, "&Redo\tCtrl+Y");`. | High |
| OBS-CMD-24 | shell.rs:549 | A separator follows Redo: `append_menu(edit, MF_SEPARATOR, 0, "");`. | High |
| OBS-CMD-25 | shell.rs:550 | Edit → Clear Variables: `append_menu(edit, MF_STRING, CMD_CLEAR as usize, "Clear &Variables");`. | High |
| OBS-CMD-26 | shell.rs:553 | Run → Run: `append_menu(run, MF_STRING, CMD_RUN as usize, "&Run\tCtrl+Enter");`. | High |
| OBS-CMD-27 | shell.rs:554 | Run → Cancel: `append_menu(run, MF_STRING, CMD_CANCEL as usize, "&Cancel\tEsc");`. | High |
| OBS-CMD-28 | shell.rs:561 | Help → About: `"&About Lovelace Desktop",`. | High |
| OBS-CMD-29 | shell.rs:564 | The menu bar itself is created: `let menu = create_menu();`. | High |
| OBS-CMD-30 | shell.rs:565 | File is attached as a popup: `append_menu(menu, MF_POPUP, file as usize, "&File");`. | High |
| OBS-CMD-31 | shell.rs:566 | Edit is attached as a popup: `append_menu(menu, MF_POPUP, edit as usize, "&Edit");`. | High |
| OBS-CMD-32 | shell.rs:567 | Run is attached as a popup: `append_menu(menu, MF_POPUP, run as usize, "&Run");`. | High |
| OBS-CMD-33 | shell.rs:568 | Help is attached as a popup: `append_menu(menu, MF_POPUP, help as usize, "&Help");`. | High |
| OBS-CMD-34 | shell.rs:331 | Editor changes are handled by: `if id == ID_EDITOR && code == EN_CHANGE {`. | High |
| OBS-CMD-35 | shell.rs:332 | Programmatic echoes are ignored through the sync flag: `if SYNCING.with(Cell::get) {`. | High |
| OBS-CMD-36 | shell.rs:335 | The edit control's full text is read: `let text = window_text(self.editor);`. | High |
| OBS-CMD-37 | shell.rs:336 | The text is handed to the application: `let effects = self.app.set_active_text(&text);`. | High |
| OBS-CMD-38 | shell.rs:342 | Only notifications with code 0 or 1 are pursued: `if code != 0 && code != 1 {`. | High |
| OBS-CMD-39 | shell.rs:346 | New maps to `CMD_NEW => Command::NewDocument,`. | High |
| OBS-CMD-40 | shell.rs:347 | Close maps to `CMD_CLOSE => Command::CloseDocument,`. | High |
| OBS-CMD-41 | shell.rs:348 | Save maps to `CMD_SAVE => Command::Save,`. | High |
| OBS-CMD-42 | shell.rs:349 | Run maps to `CMD_RUN => Command::Run,`. | High |
| OBS-CMD-43 | shell.rs:350 | Cancel maps to `CMD_CANCEL => Command::Cancel,`. | High |
| OBS-CMD-44 | shell.rs:351 | Undo maps to `CMD_UNDO => Command::Undo,`. | High |
| OBS-CMD-45 | shell.rs:352 | Redo maps to `CMD_REDO => Command::Redo,`. | High |
| OBS-CMD-46 | shell.rs:353 | Clear Variables maps to `CMD_CLEAR => Command::ClearVariables,`. | High |
| OBS-CMD-47 | shell.rs:354 | About opens a message box instead of applying a command: `CMD_ABOUT => {`. | High |
| OBS-CMD-48 | shell.rs:359 | About text names the shortcuts: `Ctrl+Enter runs the document, Ctrl+S saves it, Esc cancels.",`. | High |
| OBS-CMD-49 | shell.rs:360 | About caption is `"About Lovelace Desktop",`. | High |
| OBS-CMD-50 | shell.rs:363 | About returns without dispatch: `return;`. | High |
| OBS-CMD-51 | shell.rs:366 | Exit posts a close: `post(self.hwnd, WM_CLOSE, 0, 0);`. | High |
| OBS-CMD-52 | shell.rs:369 | Unknown ids do nothing: `_ => return,`. | High |
| OBS-CMD-53 | shell.rs:371 | Commands are applied on the UI thread: `let effects = self.app.apply(command);`. | High |
| OBS-CMD-54 | shell.rs:372 | Effects are dispatched immediately: `self.dispatch(effects);`. | High |
| OBS-CMD-55 | shell.rs:617 | The command message takes the id from the low word: `let id = (wparam & 0xFFFF) as u16;`. | High |
| OBS-CMD-56 | shell.rs:618 | It takes the notification code from the high word: `let code = ((wparam >> 16) & 0xFFFF) as u16;`. | High |
| OBS-CMD-57 | shell.rs:620 | Open is handled before the generic command handler: `CMD_OPEN if code == 0 \|\| code == 1 => {`. | High |
| OBS-CMD-58 | shell.rs:621 | Open asks the common dialog for a path: `if let Some(path) = open_file_dialog(hwnd) {`. | High |
| OBS-CMD-59 | shell.rs:623 | A chosen path becomes an Open command: `let effects = shell.app.apply(Command::Open(path));`. | High |
| OBS-CMD-60 | shell.rs:628 | Save As is handled before the generic command handler: `CMD_SAVE_AS if code == 0 \|\| code == 1 => {`. | High |
| OBS-CMD-61 | shell.rs:629 | Save As is pre-filled from the document name: `if let Some(path) = save_file_dialog(hwnd, &suggested_file_name()) {`. | High |
| OBS-CMD-62 | shell.rs:631 | A chosen path becomes SaveAs: `let effects = shell.app.apply(Command::SaveAs(path));`. | High |
| OBS-CMD-63 | shell.rs:637 | All other commands go through `on_command`: `with_shell(\|shell\| shell.on_command(id, code));`. | High |
| OBS-CMD-64 | shell.rs:331 | EN_CHANGE is the only control notification handled; all other control notifications are discarded by the code filter at line 342. | High |
| OBS-CMD-65 | shell.rs:530 | The menu is built in `fn build_menu() -> HMENU {`; no EnableMenuItem or CheckMenuItem entry point is declared in ffi.rs and no call appears in shell.rs, and items are appended without checked/disabled flags. | High |
| OBS-MSG-01 | shell.rs:520 | The message loop is the last startup step: `let exit_code = message_loop(hwnd);`. | High |
| OBS-MSG-02 | shell.rs:575 | Messages are read with `let result = get_message(&mut message);`. | High |
| OBS-MSG-03 | shell.rs:577 | A zero GetMessage result (WM_QUIT) returns its wParam as the exit code: `break message.wParam as i32;`. | High |
| OBS-MSG-04 | shell.rs:580 | A negative GetMessage result exits with 2: `break 2;`. | High |
| OBS-MSG-05 | shell.rs:582 | The accelerator table is fetched each iteration: `let table = with_shell(\|shell\| shell.accelerators).unwrap_or(0);`. | High |
| OBS-MSG-06 | shell.rs:583 | Accelerators are tried first: `if !translate_accelerator(hwnd, table, &mut message) {`. | High |
| OBS-MSG-07 | shell.rs:584 | Only untranslated messages get `translate_message(&message);`. | High |
| OBS-MSG-08 | shell.rs:585 | Only untranslated messages are dispatched: `dispatch_message(&message);`. | High |
| OBS-MSG-09 | shell.rs:643 | The 250 ms timer refreshes the status bar: `with_shell(\|shell\| shell.refresh_status());`. | High |
| OBS-MSG-10 | shell.rs:647 | Close is gated on `if close_flow(hwnd) {`. | High |
| OBS-MSG-11 | shell.rs:648 | Close destroys the frame: `destroy_window(hwnd);`. | High |
| OBS-MSG-12 | shell.rs:654 | Destroy saves settings: `shell.save_settings();`. | High |
| OBS-MSG-13 | shell.rs:655 | Destroy frees GDI/menu resources: `shell.release_resources();`. | High |
| OBS-MSG-14 | shell.rs:657 | Destroy posts quit with code 0: `post_quit(0);`. | High |
| OBS-MSG-15 | shell.rs:594 | Shell access never panics on re-entry: `cell.try_borrow_mut()` inside the shell helper. | High |
| OBS-MSG-16 | shell.rs:855 | The close flow returns immediately when the document is clean: `if !dirty {`. | High |
| OBS-MSG-17 | shell.rs:858 | A dirty document prompts: `let answer = message_box(`. | High |
| OBS-MSG-18 | shell.rs:860 | The close prompt text is `"Save changes to {name} before closing?"`. | High |
| OBS-MSG-19 | shell.rs:862 | The prompt has Yes/No/Cancel and a warning icon: `MB_YESNOCANCEL \| MB_ICONWARNING,`. | High |
| OBS-MSG-20 | shell.rs:864 | Cancel aborts the close: `if answer == IDCANCEL {`. | High |
| OBS-MSG-21 | shell.rs:867 | No abandons changes and closes: `if answer == IDNO {`. | High |
| OBS-MSG-22 | shell.rs:872 | An untitled document gets a Save dialog named after the tab: `None => match save_file_dialog(hwnd, &format!("{name}.ls")) {`. | High |
| OBS-MSG-23 | shell.rs:877 | Yes saves through `save_active_to(target)`. | High |
| OBS-MSG-24 | shell.rs:840 | The close flow's result is whether the document is dirty after saving: `.map(\|document\| document.dirty)`. | High |
| OBS-ENG-01 | shell.rs:14 | The module doc states runs execute on the UI thread: `//! - Runs execute on the UI thread. The controller models a run as "request in flight", so the`. | High (as written) |
| OBS-ENG-02 | shell.rs:377 | Effects are consumed by `fn dispatch(&mut self, effects: Vec<Effect>) {`. | High |
| OBS-ENG-03 | shell.rs:381 | A run is an effect: `Effect::RunRequested(request) => {`. | High |
| OBS-ENG-04 | shell.rs:382 | Execution is timed from before the engine call: `let started = std::time::Instant::now();`. | High |
| OBS-ENG-05 | shell.rs:384 | The engine is called inline with the request source: `Some(document) => document.engine_mut().eval_full(&request.source),`. | High |
| OBS-ENG-06 | shell.rs:385 | A missing document index skips the effect silently: `None => continue,`. | High |
| OBS-ENG-07 | shell.rs:387 | Elapsed time is recorded in whole milliseconds: `let elapsed = started.elapsed().as_millis() as u64;`. | High |
| OBS-ENG-08 | shell.rs:388 | The result is handed back to the application: `let follow_up = self.app.finish_run(request.document, outcome, elapsed);`. | High |
| OBS-ENG-09 | shell.rs:389 | The comment asserts finish_run only yields Repaint: `// finish_run only ever returns Repaint (plus Error, already logged).`. | High (as written) |
| OBS-ENG-10 | shell.rs:392 | Saved and Error both request a repaint: `Effect::Saved(_) \| Effect::Error(_) => repaint = true,`. | High |
| OBS-ENG-11 | shell.rs:397 | Repaint effects call `self.refresh_all();`. | High |
| OBS-ENG-12 | shell.rs:349 | The shell run path starts at `CMD_RUN => Command::Run,`; no cancellation token, flag or interrupt call exists in either scoped file. | High |
| OBS-STB-01 | shell.rs:314 | The caret position is read from the editor: `let position = editor_selection(self.editor);`. | High |
| OBS-STB-02 | shell.rs:321 | It is converted to 1-based line/column: `let (line, column) = line_col_utf16(buffer, position);`. | High |
| OBS-STB-03 | shell.rs:322 | The same position is converted to a byte offset: `(line, column, byte_offset_utf16(buffer, position))`. | High |
| OBS-STB-04 | shell.rs:325 | The document cursor is updated from the control: `document.set_cursor(byte_offset);`. | High |
| OBS-STB-05 | shell.rs:327 | The status text is written: `set_status_text(self.status, &self.app.status_text(line, column));`. | High |
| OBS-STB-06 | ffi.rs:144 | `pub const SB_SETTEXTW: u32 = WM_USER + 1;`. | High |
| OBS-STB-07 | ffi.rs:730 | The wrapper is `pub fn set_status_text(hwnd: HWND, text: &str) {`. | High |
| OBS-STB-08 | ffi.rs:738 | Only part 0 is set: `SendMessageW(hwnd, SB_SETTEXTW, 0, text.as_ptr() as LPARAM);`; no SB_SETPARTS import exists, so no part boundaries are configured in scope. | High |
| OBS-STB-09 | shell.rs:307 | Every full refresh ends with `self.refresh_status();`. | High |
| OBS-STB-10 | shell.rs:896 | Line/column conversion is `fn line_col_utf16(text: &str, units: usize) -> (usize, usize) {`. | High |
| OBS-STB-11 | shell.rs:920 | Offset conversion is `fn byte_offset_utf16(text: &str, units: usize) -> usize {`. | High |
| OBS-SET-01 | shell.rs:401 | Settings persistence is `fn save_settings(&mut self) {`. | High |
| OBS-SET-02 | shell.rs:405 | The frame's outer size is read: `let (width, height) = window_size(self.hwnd);`. | High |
| OBS-SET-03 | shell.rs:407 | It is stored as the next window geometry: `self.app.settings.window = WindowGeometry { width, height };`. | High |
| OBS-SET-04 | shell.rs:414 | Precision is persisted: `self.app.settings.active_precision = precision;`. | High |
| OBS-SET-05 | shell.rs:416 | Save failures are logged at Warn: `self.app.push_log(` with LogLevel::Warn at line 417. | High |
| OBS-SET-06 | shell.rs:443 | The config location is `let settings_path = default_config_path();`. | High |
| OBS-SET-07 | shell.rs:444 | Settings are loaded from that path: `let settings = match &settings_path {`. | High |
| OBS-SET-08 | shell.rs:446 | With no path, defaults are used: `None => Settings::new(),`. | High |
| OBS-SET-09 | shell.rs:452 | A script path from the command line is opened before the window exists: `app.apply(Command::Open(PathBuf::from(script)));`. | High |
| OBS-SET-10 | shell.rs:654 | `shell.save_settings();` runs in the WM_DESTROY handler, and save plus release run again after the loop at lines 522-523. | High |
| OBS-TST-01 | shell.rs:931 | The file has a test module: `#[cfg(test)]`. | High |
| OBS-TST-02 | shell.rs:936 | Tests cover line/column mapping only: `fn maps_utf16_offsets_to_line_and_column() {`. | High |
| OBS-TST-03 | shell.rs:946 | Tests cover byte-offset mapping: `fn maps_utf16_offsets_to_byte_offsets() {`. | High |
| OBS-TST-04 | shell.rs:950 | Surrogate pairs are tested as one column: `let text = "😀x";`. | High |
| OBS-TST-05 | shell.rs:931 | The test module is introduced by `#[cfg(test)]`; no test creates or inspects a window, and the tests exercise two pure functions only. | High |

## 2. UI elements

Five child window controls are created; the frame window and the menu bar are not child controls.

| UI element | Win32 class / message | Position/size source | Behaviour | Citation |
|---|---|---|---|---|
| Top-level frame | class `LovelaceDesktopMainWindow`; proc `main_window_proc`; style `WS_OVERLAPPEDWINDOW \| WS_CLIPCHILDREN`; ex `WS_EX_APPWINDOW`; menu attached at create | `CW_USEDEFAULT` x/y; width/height from `settings.window`; min tracking 900×600; client area feeds `layout()` | Hosts the five children; title from `app.window_title()`; `WM_SETFOCUS` focuses the editor; `WM_CLOSE` runs the dirty-document flow; 250 ms status timer; `WM_DESTROY` saves settings, releases resources and posts quit | shell.rs:480-492, 660-674, 503-518 |
| Menu bar | `HMENU` built by `build_menu()`; attached through the `menu` argument of `CreateWindowExW`; no class | n/a (non-client) | File/Edit/Run/Help; 12 command ids 101-131; three separators; no checked or disabled items set in scope | shell.rs:477, 490, 530-570 |
| Editor | `EDIT`, id 1000, ex `WS_EX_CLIENTEDGE`; `WS_VSCROLL`, `WS_HSCROLL`, `WS_TABSTOP`, `ES_MULTILINE`, `ES_AUTOVSCROLL`, `ES_AUTOHSCROLL`, `ES_WANTRETURN`, `ES_NOHIDESEL` | `(0, 0, editor_w, editor_h)` from `compute_layout` | User text entry; `EN_CHANGE` pushes text via `set_active_text`; 0x7FFFFFFE character limit; Consolas −16; receives focus whenever the frame gains focus | shell.rs:121-144, 206-207, 244, 331-338, 671-674 |
| Variables pane | `EDIT`, id 1001, ex `WS_EX_CLIENTEDGE`; `WS_VSCROLL`, `ES_MULTILINE`, `ES_AUTOVSCROLL`, `ES_READONLY` | `(editor_w + PANE_GAP, 0, right_w, editor_h + PANE_GAP + plot_h)` | Displays `app.variables_text()`; read-only; Segoe UI −14; no horizontal scroll style | shell.rs:146-165, 252-258, 284-287 |
| Logs pane | `EDIT`, id 1002, ex `WS_EX_CLIENTEDGE`; `WS_VSCROLL`, `ES_MULTILINE`, `ES_AUTOVSCROLL`, `ES_READONLY` | `(0, editor_h + PANE_GAP + plot_h + PANE_GAP, width, logs_h)` | Displays `app.logs_text()`; read-only; after any text change the selection is moved to the end to scroll; Segoe UI −14 | shell.rs:166-178, 259-265, 289-293, ffi.rs:756-762 |
| Plot pane | custom `LovelaceDesktopPlotWindow`, id 1003, ex `WS_EX_CLIENTEDGE`; `WS_CHILD \| WS_VISIBLE \| WS_BORDER`; proc `plot_window_proc` | `(0, editor_h + PANE_GAP, editor_w, plot_h)` | Fills white, draws the active document's parsed SVG polylines and text with GDI pens/fonts, or draws grey "no plot yet"; invalidated on layout and refresh | shell.rs:179-191, 245-251, 300-305, 692-747 |
| Status bar | `msctls_statusbar32`, id 1004, no ex style; `SBARS_SIZEGRIP` | Positions itself at the bottom when sent `WM_SIZE`; strip height = outer height, minimum 22 | Writes one text part (part 0) with `app.status_text(line, column)`; refreshed by the 250 ms timer and after every full refresh | shell.rs:192-204, 237-238, 307-327, ffi.rs:730-740 |

## 3. Keyboard shortcuts

Nine accelerator entries are installed (shell.rs:213-224). Menu strings advertise the same
combinations. Accelerators are translated before `TranslateMessage`/`DispatchMessage`, and the
source comment states they arrive as `WM_COMMAND` with code 1 (shell.rs:340-341).

| Keyboard shortcut | Message/accelerator | Action | Citation |
|---|---|---|---|
| Ctrl+N | `ACCEL { fVirt: FCONTROL \| FVIRTKEY, key: 'N', cmd: CMD_NEW }` → `WM_COMMAND` 101 | `Command::NewDocument` applied; effects dispatched | shell.rs:214, 346; ffi.rs:885-897 |
| Ctrl+O | `ACCEL { fcontrol \| fvirtkey, 'O', CMD_OPEN }` → `WM_COMMAND` 102 | Opens the common Open dialog; chosen path applied as `Command::Open(path)` | shell.rs:215, 620-626; ffi.rs:1136-1168 |
| Ctrl+S | `ACCEL { fcontrol \| fvirtkey, 'S', CMD_SAVE }` → `WM_COMMAND` 103 | `Command::Save` applied | shell.rs:216, 348 |
| Ctrl+Shift+S | `ACCEL { fcontrol \| fshift \| fvirtkey, 'S', CMD_SAVE_AS }` → `WM_COMMAND` 104 | Opens the common Save dialog; chosen path applied as `Command::SaveAs(path)` | shell.rs:217, 628-634; ffi.rs:1171-1205 |
| Ctrl+W | `ACCEL { fcontrol \| fvirtkey, 'W', CMD_CLOSE }` → `WM_COMMAND` 105 | `Command::CloseDocument` applied | shell.rs:218, 347 |
| Ctrl+Z | `ACCEL { fcontrol \| fvirtkey, 'Z', CMD_UNDO }` → `WM_COMMAND` 111 | `Command::Undo` applied | shell.rs:219, 351 |
| Ctrl+Y | `ACCEL { fcontrol \| fvirtkey, 'Y', CMD_REDO }` → `WM_COMMAND` 112 | `Command::Redo` applied | shell.rs:220, 352 |
| Ctrl+Enter | `ACCEL { fcontrol \| fvirtkey, VK_RETURN, CMD_RUN }` → `WM_COMMAND` 121 | `Command::Run` applied; resulting `RunRequested` effect is evaluated inline by `eval_full` on the UI thread | shell.rs:221, 349, 381-388 |
| Esc | `ACCEL { fvirtkey, VK_ESCAPE, CMD_CANCEL }` → `WM_COMMAND` 122 | `Command::Cancel` applied | shell.rs:222, 350 |

No other `ACCEL` entries are created, and no `WM_KEYDOWN`/`WM_CHAR` arm exists in either window
procedure; all other key handling is whatever the EDIT controls and `DefWindowProcW` do.

## 4. What could not be determined within SCOPE

1. **Numeric pane geometry.** `PANE_GAP`, `compute_layout`, `apply_layout` and
   `settings.layout` are defined in `crate::app`/`crate::settings`, outside the three scoped files.
   Only the relative expressions in `layout()` are knowable; no pixel values, ratios or defaults.
2. **Initial window size.** `settings.window` is produced by `Settings::load(default_config_path())`
   in code outside scope, so the actual startup width/height (and any saved value) is unknown.
   `CW_USEDEFAULT` applies to position only.
3. **Rendered colours and theming.** `COLOR_WINDOW_BRUSH` is an index (6) into the current Windows
   theme, and no `WM_CTLCOLOR*`, dark-mode, DWM or uxtheme call exists in scope. The actual RGB of
   the frame, edit controls, menu bar and status bar, and whether the app follows light/dark mode,
   cannot be established from these files.
4. **Command and engine semantics.** `AppState::apply`, `set_active_text`, `finish_run`,
   `status_text`, `window_title`, `tab_title`, `engine_mut().eval_full`, `Command`, `Effect`, `Plot`
   and `svg::parse` are all outside scope. The census records only that the shell sends these calls,
   not what they do internally.
5. **Whether Cancel can interrupt a run.** The shell dispatches `Command::Cancel` and evaluates
   `RunRequested` inline on the UI thread; no cancellation token, flag or interrupt call exists in
   scope. Whether `eval_full` is interruptible, and whether a Cancel can be processed while a run is
   evaluating, cannot be determined without reading `app.rs`/the engine and without running the app.
6. **UI Automation / accessibility tree.** No `WM_GETOBJECT` arm, `IAccessible`, UIA provider,
   oleacc/uiautomation link or custom accessibility code appears in the three files; the only
   accessibility exposure available is whatever Windows supplies by default for standard controls
   and for the custom plot window. What that tree actually contains at runtime was not verified
   (no execution, no UIA inspection).
7. **DPI awareness and scaling.** No DPI API call or manifest is in scope; the fonts use negative
   (pixel) heights and layout arithmetic is unitless. Actual physical pixel sizes depend on process
   DPI awareness and display settings, which are outside scope.
8. **Text formats.** Window title, status bar text and tab titles are produced by out-of-scope
   application methods; only the call sites are recorded.
9. **Menu state changes.** No code in scope enables, disables, checks or unchecks menu items, and no
   menu-related notification handling exists. Whether another module or the application updates the
   menu cannot be determined from these files.
10. **Status bar part count.** Only `SB_SETTEXTW` for part 0 is used and `SB_SETPARTS` is never
    imported or called; the control's part layout beyond part 0 is not configured in scope.
11. **Runtime behaviour of system-provided features.** IME composition, clipboard, selection,
    right-click menus, caret movement and default tab navigation are delegated to the EDIT controls
    and Windows; their exact behaviour is not implemented or described in these files.
12. **Test execution.** The unit tests at shell.rs:931-960 were read but not run; `cargo` was not
    invoked because another process holds the build lock.
