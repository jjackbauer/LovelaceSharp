//! The native Windows shell: one window, four panes, a menu bar, accelerators and a message loop.
//!
//! Design choices, recorded here because they are deliberate:
//!
//! - The editor is a real multi-line EDIT control. IME composition, clipboard, selection,
//!   scrolling, right-click menus and caret movement are the platform's implementation, not ours.
//!   The controller keeps its own buffer and undo stack and synchronises with the control on
//!   EN_CHANGE; programmatic writes set a flag so the echo does not re-enter the controller.
//! - The workspace (variables above functions) and the logs are read-only EDIT controls, so they
//!   are selectable and copyable for free.
//! - The plot pane is a custom child window that paints a white background and then draws the
//!   engine's SVG with GDI: polylines become Polyline calls and text becomes TextOutW. There is
//!   no general SVG renderer here, and there is no web view anywhere in the process.
//! - Runs execute on the UI thread. The controller models a run as "request in flight", so the
//!   Cancel command and the stale-result rule are exercised headlessly by --selftest even though
//!   the shell only ever sends the request and evaluates it inline.
//!
//! Re-entrancy: the shell state lives in a thread_local RefCell and every callback uses
//! try_borrow_mut, so a nested message (for example EN_CHANGE raised by our own SetWindowTextW)
//! can never panic. Dialogs are opened only after the borrow is released, because they pump
//! messages.

#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use crate::app::{AppState, Command, Effect, LogLevel, PANE_GAP, compute_layout};
use crate::settings::{Settings, WindowGeometry, default_config_path};
use crate::svg::{self, Plot, TextAnchor};
use crate::win32::ffi::*;

/// Window class of the main window.
const WINDOW_CLASS: &str = "LovelaceDesktopMainWindow";
/// Window class of the plot pane.
const PLOT_CLASS: &str = "LovelaceDesktopPlotWindow";

// Menu and accelerator command identifiers. They stay below 0x8000 because WM_COMMAND carries
// only the low word.
const CMD_NEW: u16 = 101;
const CMD_OPEN: u16 = 102;
const CMD_SAVE: u16 = 103;
const CMD_SAVE_AS: u16 = 104;
const CMD_CLOSE: u16 = 105;
const CMD_EXIT: u16 = 109;
const CMD_UNDO: u16 = 111;
const CMD_REDO: u16 = 112;
const CMD_CLEAR: u16 = 115;
const CMD_RUN: u16 = 121;
const CMD_CANCEL: u16 = 122;
const CMD_ABOUT: u16 = 131;

// Control identifiers. The editor uses EN_CHANGE, so it needs a stable id too.
const ID_EDITOR: u16 = 1000;
const ID_VARIABLES: u16 = 1001;
const ID_LOGS: u16 = 1002;
const ID_PLOT: u16 = 1003;
const ID_STATUS: u16 = 1004;

/// Timer that refreshes the cursor readout in the status bar.
const TIMER_STATUS: usize = 1;

thread_local! {
    static SHELL: RefCell<Option<Shell>> = const { RefCell::new(None) };
    static SYNCING: Cell<bool> = const { Cell::new(false) };
}

/// Everything the window procedure needs. Owned by the thread that created the window.
struct Shell {
    hwnd: HWND,
    editor: HWND,
    variables: HWND,
    logs: HWND,
    plot: HWND,
    status: HWND,
    font_mono: HFONT,
    font_ui: HFONT,
    font_title: HFONT,
    brush_white: HBRUSH,
    pens: Vec<HPEN>,
    accelerators: isize,
    app: AppState,
    plot_data: Option<Plot>,
    settings_path: Option<PathBuf>,
}

impl Shell {
    fn new(hwnd: HWND, app: AppState, settings_path: Option<PathBuf>) -> Shell {
        Shell {
            hwnd,
            editor: 0,
            variables: 0,
            logs: 0,
            plot: 0,
            status: 0,
            font_mono: 0,
            font_ui: 0,
            font_title: 0,
            brush_white: 0,
            pens: Vec::new(),
            accelerators: 0,
            app,
            plot_data: None,
            settings_path,
        }
    }

    fn create_children(&mut self) {
        let instance = module_handle();
        self.font_mono = create_font(-16, FW_NORMAL, "Consolas");
        self.font_ui = create_font(-14, FW_NORMAL, "Segoe UI");
        self.font_title = create_font(-18, FW_SEMIBOLD, "Segoe UI");
        self.brush_white = create_brush(0x00FF_FFFF);
        self.pens = vec![
            create_pen(2, 0x001F_4E79),
            create_pen(2, 0x00C0_5034),
            create_pen(2, 0x0038_7638),
            create_pen(2, 0x0080_60A0),
        ];

        let editor_style = WS_CHILD
            | WS_VISIBLE
            | WS_VSCROLL
            | WS_HSCROLL
            | WS_TABSTOP
            | ES_LEFT
            | ES_MULTILINE
            | ES_AUTOVSCROLL
            | ES_AUTOHSCROLL
            | ES_WANTRETURN
            | ES_NOHIDESEL;
        self.editor = create_window(
            WS_EX_CLIENTEDGE,
            "EDIT",
            "",
            editor_style,
            0,
            0,
            10,
            10,
            self.hwnd,
            ID_EDITOR as isize,
            instance,
        );

        let pane_style = WS_CHILD
            | WS_VISIBLE
            | WS_VSCROLL
            | ES_LEFT
            | ES_MULTILINE
            | ES_AUTOVSCROLL
            | ES_READONLY;
        self.variables = create_window(
            WS_EX_CLIENTEDGE,
            "EDIT",
            "",
            pane_style,
            0,
            0,
            10,
            10,
            self.hwnd,
            ID_VARIABLES as isize,
            instance,
        );
        self.logs = create_window(
            WS_EX_CLIENTEDGE,
            "EDIT",
            "",
            pane_style,
            0,
            0,
            10,
            10,
            self.hwnd,
            ID_LOGS as isize,
            instance,
        );
        self.plot = create_window(
            WS_EX_CLIENTEDGE,
            PLOT_CLASS,
            "",
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            0,
            0,
            10,
            10,
            self.hwnd,
            ID_PLOT as isize,
            instance,
        );
        self.status = create_window(
            0,
            STATUSCLASSNAME,
            "",
            WS_CHILD | WS_VISIBLE | SBARS_SIZEGRIP,
            0,
            0,
            10,
            10,
            self.hwnd,
            ID_STATUS as isize,
            instance,
        );

        send(self.editor, EM_SETLIMITTEXT, 0x7FFF_FFFE, 0);
        set_font(self.editor, self.font_mono);
        set_font(self.variables, self.font_ui);
        set_font(self.logs, self.font_ui);
    }

    fn install_accelerators(&mut self) {
        let entries: [(u8, u16, u16); 9] = [
            (FCONTROL | FVIRTKEY, b'N' as u16, CMD_NEW),
            (FCONTROL | FVIRTKEY, b'O' as u16, CMD_OPEN),
            (FCONTROL | FVIRTKEY, b'S' as u16, CMD_SAVE),
            (FCONTROL | FSHIFT | FVIRTKEY, b'S' as u16, CMD_SAVE_AS),
            (FCONTROL | FVIRTKEY, b'W' as u16, CMD_CLOSE),
            (FCONTROL | FVIRTKEY, b'Z' as u16, CMD_UNDO),
            (FCONTROL | FVIRTKEY, b'Y' as u16, CMD_REDO),
            (FCONTROL | FVIRTKEY, VK_RETURN, CMD_RUN),
            (FVIRTKEY, VK_ESCAPE, CMD_CANCEL),
        ];
        self.accelerators = create_accelerators(&entries);
    }

    fn layout(&mut self) {
        if self.status == 0 {
            return;
        }
        let (width, height) = client_size(self.hwnd);
        if width <= 0 || height <= 0 {
            return;
        }
        // The status bar positions itself at the bottom of the client area when it receives
        // WM_SIZE, so ask it to do that first and then measure the strip.
        send(self.status, WM_SIZE, 0, 0);
        let status_height = window_size(self.status).1.max(22);
        let content_height = (height - status_height).max(0);
        self.app.apply_layout(width, content_height);
        let sizes = compute_layout(width, content_height, &self.app.settings.layout);
        self.app.layout = sizes;

        move_window(self.editor, 0, 0, sizes.editor_w, sizes.editor_h);
        move_window(
            self.plot,
            0,
            sizes.editor_h + PANE_GAP,
            sizes.editor_w,
            sizes.plot_h,
        );
        move_window(
            self.variables,
            sizes.editor_w + PANE_GAP,
            0,
            sizes.right_w,
            sizes.editor_h + PANE_GAP + sizes.plot_h,
        );
        move_window(
            self.logs,
            0,
            sizes.editor_h + PANE_GAP + sizes.plot_h + PANE_GAP,
            width,
            sizes.logs_h,
        );
        invalidate(self.plot);
    }

    fn refresh_all(&mut self) {
        if self.editor == 0 {
            return;
        }
        let buffer = self
            .app
            .active_doc()
            .map(|document| document.buffer.clone())
            .unwrap_or_default();
        if window_text(self.editor) != buffer {
            SYNCING.with(|flag| flag.set(true));
            set_text(self.editor, &buffer);
            SYNCING.with(|flag| flag.set(false));
        }

        let variables = self.app.variables_text();
        if window_text(self.variables) != variables {
            set_text(self.variables, &variables);
        }

        let logs = self.app.logs_text();
        if window_text(self.logs) != logs {
            set_text(self.logs, &logs);
            editor_scroll_to_end(self.logs);
        }

        let title = self.app.window_title();
        if window_text(self.hwnd) != title {
            set_text(self.hwnd, &title);
        }

        self.plot_data = self
            .app
            .active_doc()
            .and_then(|document| document.last_plot.as_ref())
            .map(|capture| svg::parse(&capture.svg));
        invalidate(self.plot);

        self.refresh_status();
    }

    fn refresh_status(&mut self) {
        if self.status == 0 {
            return;
        }
        let position = editor_selection(self.editor);
        let (line, column, byte_offset) = {
            let buffer = self
                .app
                .active_doc()
                .map(|document| document.buffer.as_str())
                .unwrap_or("");
            let (line, column) = line_col_utf16(buffer, position);
            (line, column, byte_offset_utf16(buffer, position))
        };
        if let Some(document) = self.app.active_doc_mut() {
            document.set_cursor(byte_offset);
        }
        set_status_text(self.status, &self.app.status_text(line, column));
    }

    fn on_command(&mut self, id: u16, code: u16) {
        if id == ID_EDITOR && code == EN_CHANGE {
            if SYNCING.with(Cell::get) {
                return;
            }
            let text = window_text(self.editor);
            let effects = self.app.set_active_text(&text);
            self.dispatch(effects);
            return;
        }
        // Menu items deliver code 0 and accelerators deliver code 1; everything else is a
        // control notification this shell does not handle.
        if code != 0 && code != 1 {
            return;
        }
        let command = match id {
            CMD_NEW => Command::NewDocument,
            CMD_CLOSE => Command::CloseDocument,
            CMD_SAVE => Command::Save,
            CMD_RUN => Command::Run,
            CMD_CANCEL => Command::Cancel,
            CMD_UNDO => Command::Undo,
            CMD_REDO => Command::Redo,
            CMD_CLEAR => Command::ClearVariables,
            CMD_ABOUT => {
                message_box(
                    self.hwnd,
                    "Lovelace Desktop\n\nA native IDE for the Lovelace scripting language.\n\
                     No web view, no local server: one process, one window.\n\n\
                     Ctrl+Enter runs the document, Ctrl+S saves it, Esc cancels.",
                    "About Lovelace Desktop",
                    MB_OK | MB_ICONINFORMATION,
                );
                return;
            }
            CMD_EXIT => {
                post(self.hwnd, WM_CLOSE, 0, 0);
                return;
            }
            _ => return,
        };
        let effects = self.app.apply(command);
        self.dispatch(effects);
    }

    /// Performs the effects of a command. A RunRequested effect is evaluated here, on the UI
    /// thread, and its outcome is published through finish_run.
    fn dispatch(&mut self, effects: Vec<Effect>) {
        let mut repaint = false;
        for effect in effects {
            match effect {
                Effect::RunRequested(request) => {
                    let started = std::time::Instant::now();
                    let outcome = match self.app.documents.get_mut(request.document) {
                        Some(document) => document.engine_mut().eval_full(&request.source),
                        None => continue,
                    };
                    let elapsed = started.elapsed().as_millis() as u64;
                    let follow_up = self.app.finish_run(request.document, outcome, elapsed);
                    // finish_run only ever returns Repaint (plus Error, already logged).
                    repaint |= !follow_up.is_empty();
                }
                Effect::Saved(_) | Effect::Error(_) => repaint = true,
                Effect::Repaint => repaint = true,
            }
        }
        if repaint {
            self.refresh_all();
        }
    }

    fn save_settings(&mut self) {
        let Some(path) = self.settings_path.clone() else {
            return;
        };
        let (width, height) = window_size(self.hwnd);
        if width > 0 && height > 0 {
            self.app.settings.window = WindowGeometry { width, height };
        }
        let precision = self
            .app
            .active_doc()
            .map(|document| document.precision)
            .unwrap_or(self.app.settings.active_precision);
        self.app.settings.active_precision = precision;
        if let Err(error) = self.app.settings.save(&path) {
            self.app.push_log(
                LogLevel::Warn,
                format!("Cannot save settings to {}: {error}", path.display()),
            );
        }
    }

    /// Frees every GDI object and the accelerator table. Idempotent: handles are zeroed.
    fn release_resources(&mut self) {
        for pen in self.pens.drain(..) {
            delete_object(pen);
        }
        delete_object(self.brush_white);
        self.brush_white = 0;
        delete_object(self.font_mono);
        delete_object(self.font_ui);
        delete_object(self.font_title);
        self.font_mono = 0;
        self.font_ui = 0;
        self.font_title = 0;
        destroy_accelerators(self.accelerators);
        self.accelerators = 0;
    }
}

/// Runs the shell. Returns the process exit code.
pub fn run(initial: Option<&str>) -> i32 {
    let settings_path = default_config_path();
    let settings = match &settings_path {
        Some(path) => Settings::load(path),
        None => Settings::new(),
    };
    let mut app = AppState::with_settings(settings);
    app.adopt_settings_warnings();
    if let Some(script) = initial {
        if !script.trim().is_empty() {
            app.apply(Command::Open(PathBuf::from(script)));
        }
    }

    if !init_common_controls() {
        message_box(
            0,
            "Could not initialise the Windows common controls.",
            "Lovelace Desktop",
            MB_OK | MB_ICONERROR,
        );
        return 1;
    }
    if !register_class(WINDOW_CLASS, Some(main_window_proc), COLOR_WINDOW_BRUSH)
        || !register_class(PLOT_CLASS, Some(plot_window_proc), COLOR_WINDOW_BRUSH)
    {
        message_box(
            0,
            "Could not register the window classes.",
            "Lovelace Desktop",
            MB_OK | MB_ICONERROR,
        );
        return 1;
    }

    let menu = build_menu();
    let geometry = app.settings.window;
    let title = app.window_title();
    let hwnd = create_window(
        WS_EX_APPWINDOW,
        WINDOW_CLASS,
        &title,
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        geometry.width,
        geometry.height,
        0,
        menu,
        module_handle(),
    );
    if hwnd == 0 {
        message_box(
            0,
            "Could not create the main window.",
            "Lovelace Desktop",
            MB_OK | MB_ICONERROR,
        );
        return 1;
    }

    SHELL.with(|cell| *cell.borrow_mut() = Some(Shell::new(hwnd, app, settings_path)));
    let ready = with_shell(|shell| {
        shell.create_children();
        shell.install_accelerators();
        shell.layout();
        shell.refresh_all();
    })
    .is_some();
    if !ready {
        destroy_window(hwnd);
        return 1;
    }

    show_window(hwnd);
    update_window(hwnd);
    set_timer(hwnd, TIMER_STATUS, 250);

    let exit_code = message_loop(hwnd);
    with_shell(|shell| {
        shell.save_settings();
        shell.release_resources();
    });
    SHELL.with(|cell| *cell.borrow_mut() = None);
    exit_code
}

/// Builds the menu bar: File, Edit, Run, Help.
fn build_menu() -> HMENU {
    let file = create_popup_menu();
    append_menu(file, MF_STRING, CMD_NEW as usize, "&New\tCtrl+N");
    append_menu(file, MF_STRING, CMD_OPEN as usize, "&Open...\tCtrl+O");
    append_menu(file, MF_STRING, CMD_SAVE as usize, "&Save\tCtrl+S");
    append_menu(
        file,
        MF_STRING,
        CMD_SAVE_AS as usize,
        "Save &As...\tCtrl+Shift+S",
    );
    append_menu(file, MF_SEPARATOR, 0, "");
    append_menu(file, MF_STRING, CMD_CLOSE as usize, "&Close Tab\tCtrl+W");
    append_menu(file, MF_SEPARATOR, 0, "");
    append_menu(file, MF_STRING, CMD_EXIT as usize, "E&xit");

    let edit = create_popup_menu();
    append_menu(edit, MF_STRING, CMD_UNDO as usize, "&Undo\tCtrl+Z");
    append_menu(edit, MF_STRING, CMD_REDO as usize, "&Redo\tCtrl+Y");
    append_menu(edit, MF_SEPARATOR, 0, "");
    append_menu(edit, MF_STRING, CMD_CLEAR as usize, "Clear &Variables");

    let run = create_popup_menu();
    append_menu(run, MF_STRING, CMD_RUN as usize, "&Run\tCtrl+Enter");
    append_menu(run, MF_STRING, CMD_CANCEL as usize, "&Cancel\tEsc");

    let help = create_popup_menu();
    append_menu(
        help,
        MF_STRING,
        CMD_ABOUT as usize,
        "&About Lovelace Desktop",
    );

    let menu = create_menu();
    append_menu(menu, MF_POPUP, file as usize, "&File");
    append_menu(menu, MF_POPUP, edit as usize, "&Edit");
    append_menu(menu, MF_POPUP, run as usize, "&Run");
    append_menu(menu, MF_POPUP, help as usize, "&Help");
    menu
}

fn message_loop(hwnd: HWND) -> i32 {
    let mut message = MSG::default();
    loop {
        let result = get_message(&mut message);
        if result == 0 {
            break message.wParam as i32;
        }
        if result < 0 {
            break 2;
        }
        let table = with_shell(|shell| shell.accelerators).unwrap_or(0);
        if !translate_accelerator(hwnd, table, &mut message) {
            translate_message(&message);
            dispatch_message(&message);
        }
    }
}

/// Runs the action with the shell, or does nothing when the shell is absent or already borrowed.
/// The try_borrow is what makes nested messages harmless.
fn with_shell<R>(action: impl FnOnce(&mut Shell) -> R) -> Option<R> {
    SHELL.with(|cell| {
        cell.try_borrow_mut()
            .ok()
            .and_then(|mut slot| slot.as_mut().map(action))
    })
}

/// The main window procedure.
///
/// SAFETY: every call made here either goes through a safe wrapper in ffi.rs or accesses the
/// lparam MINMAXINFO pointer, which WM_GETMINMAXINFO guarantees is non-null and writable for the
/// duration of the call.
unsafe extern "system" fn main_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SIZE => {
            with_shell(|shell| shell.layout());
            return 0;
        }
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as u16;
            let code = ((wparam >> 16) & 0xFFFF) as u16;
            match id {
                CMD_OPEN if code == 0 || code == 1 => {
                    if let Some(path) = open_file_dialog(hwnd) {
                        with_shell(|shell| {
                            let effects = shell.app.apply(Command::Open(path));
                            shell.dispatch(effects);
                        });
                    }
                }
                CMD_SAVE_AS if code == 0 || code == 1 => {
                    if let Some(path) = save_file_dialog(hwnd, &suggested_file_name()) {
                        with_shell(|shell| {
                            let effects = shell.app.apply(Command::SaveAs(path));
                            shell.dispatch(effects);
                        });
                    }
                }
                _ => {
                    with_shell(|shell| shell.on_command(id, code));
                }
            }
            return 0;
        }
        WM_TIMER => {
            with_shell(|shell| shell.refresh_status());
            return 0;
        }
        WM_CLOSE => {
            if close_flow(hwnd) {
                destroy_window(hwnd);
            }
            return 0;
        }
        WM_DESTROY => {
            with_shell(|shell| {
                shell.save_settings();
                shell.release_resources();
            });
            post_quit(0);
            return 0;
        }
        WM_GETMINMAXINFO => {
            let info = lparam as *mut MINMAXINFO;
            if !info.is_null() {
                // SAFETY: WM_GETMINMAXINFO passes a writable MINMAXINFO in lparam; the null check
                // and the message contract make this dereference valid.
                unsafe {
                    (*info).ptMinTrackSize = POINT { x: 900, y: 600 };
                }
            }
            return 0;
        }
        WM_SETFOCUS => {
            let editor = with_shell(|shell| shell.editor).unwrap_or(0);
            set_focus(editor);
            return 0;
        }
        _ => {}
    }
    def_window_proc(hwnd, message, wparam, lparam)
}

/// The plot pane's window procedure.
///
/// SAFETY: the only foreign pointer handled here is the device context delivered by WM_PAINT,
/// which paint_begin and paint_end bracket; all other work goes through safe wrappers.
unsafe extern "system" fn plot_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_ERASEBKGND => {
            let (width, height) = client_size(hwnd);
            let brush = with_shell(|shell| shell.brush_white).unwrap_or(0);
            fill_rect(
                wparam as HDC,
                &RECT {
                    left: 0,
                    top: 0,
                    right: width,
                    bottom: height,
                },
                brush,
            );
            return 1;
        }
        WM_PAINT => {
            paint_plot(hwnd);
            return 0;
        }
        _ => {}
    }
    def_window_proc(hwnd, message, wparam, lparam)
}

fn paint_plot(hwnd: HWND) {
    let (hdc, paint) = paint_begin(hwnd);
    let (width, height) = client_size(hwnd);
    let state = with_shell(|shell| {
        (
            shell.plot_data.clone(),
            shell.brush_white,
            shell.font_ui,
            shell.font_title,
            shell.pens.clone(),
        )
    });
    let (plot, brush, font_ui, font_title, pens) = state.unwrap_or((None, 0, 0, 0, Vec::new()));
    let area = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    fill_rect(hdc, &area, brush);
    match plot {
        Some(plot) if !plot.is_empty() => {
            draw_plot(hdc, &plot, width, height, font_ui, font_title, &pens);
        }
        _ => {
            set_bk_mode(hdc, TRANSPARENT);
            set_text_color(hdc, 0x0080_8080);
            text_out(hdc, 16, 16, "no plot yet");
        }
    }
    paint_end(hwnd, &paint);
}

/// Draws the parsed SVG subset: polylines first, then text labels and a centring title.
fn draw_plot(
    hdc: HDC,
    plot: &Plot,
    width: i32,
    height: i32,
    font_ui: HFONT,
    font_title: HFONT,
    pens: &[HPEN],
) {
    if width < 4 || height < 4 || plot.width <= 0 || plot.height <= 0 {
        return;
    }
    let to_x = |x: i64| -> i32 { ((x * width as i64) / plot.width) as i32 };
    let to_y = |y: i64| -> i32 { ((y * height as i64) / plot.height) as i32 };

    for (index, line) in plot.polylines.iter().enumerate() {
        if line.points.len() < 2 {
            continue;
        }
        let points: Vec<POINT> = line
            .points
            .iter()
            .map(|point| POINT {
                x: to_x(point.x),
                y: to_y(point.y),
            })
            .collect();
        if pens.is_empty() {
            polyline(hdc, &points);
        } else {
            let previous = select_object(hdc, pens[index % pens.len()]);
            polyline(hdc, &points);
            select_object(hdc, previous);
        }
    }

    set_bk_mode(hdc, TRANSPARENT);
    set_text_color(hdc, 0x0020_2020);
    for item in &plot.texts {
        let size = ((item.size * height as i64) / plot.height).clamp(9, 28) as i32;
        let font = if item.bold { font_title } else { font_ui };
        if font == 0 {
            continue;
        }
        let previous = select_object(hdc, font);
        let mut x = to_x(item.x);
        match item.anchor {
            TextAnchor::Middle => x -= text_width(hdc, &item.text) / 2,
            TextAnchor::End => x -= text_width(hdc, &item.text),
            TextAnchor::Start => {}
        }
        let top = to_y(item.y) - (size * 4) / 5;
        text_out(hdc, x.max(0), top.max(0), &item.text);
        select_object(hdc, previous);
    }

    if let Some(title) = &plot.title {
        let already_drawn = plot.texts.iter().any(|item| &item.text == title);
        if !already_drawn && font_title != 0 {
            let previous = select_object(hdc, font_title);
            set_text_color(hdc, 0x0010_1010);
            let text = text_width(hdc, title);
            text_out(hdc, ((width - text) / 2).max(0), 4, title);
            select_object(hdc, previous);
        }
    }
}

/// Returns the active document's display name, dirty flag and path without holding the borrow.
fn active_document_state() -> Option<(String, bool, Option<PathBuf>)> {
    with_shell(|shell| {
        let document = shell.app.documents.get(shell.app.active)?;
        Some((
            shell.app.tab_title(shell.app.active),
            document.dirty,
            document.path.clone(),
        ))
    })
    .flatten()
}

/// Saves the active document to a path. Returns true only when it is clean afterwards.
fn save_active_to(path: PathBuf) -> bool {
    with_shell(|shell| {
        let effects = shell.app.apply(Command::SaveAs(path));
        shell.dispatch(effects);
        !shell
            .app
            .documents
            .get(shell.app.active)
            .map(|document| document.dirty)
            .unwrap_or(true)
    })
    .unwrap_or(false)
}

/// The WM_CLOSE flow: prompt when the active document has unsaved changes.
///
/// The message box and the file dialog are opened outside the shell borrow because they pump
/// messages, which would otherwise re-enter a borrowed RefCell (safely ignored, but the dialog's
/// own WM_SIZE and WM_PAINT handling should still see the shell).
fn close_flow(hwnd: HWND) -> bool {
    let Some((name, dirty, path)) = active_document_state() else {
        return true;
    };
    if !dirty {
        return true;
    }
    let answer = message_box(
        hwnd,
        &format!("Save changes to {name} before closing?"),
        "Lovelace Desktop",
        MB_YESNOCANCEL | MB_ICONWARNING,
    );
    if answer == IDCANCEL {
        return false;
    }
    if answer == IDNO {
        return true;
    }
    let target = match path {
        Some(path) => path,
        None => match save_file_dialog(hwnd, &format!("{name}.ls")) {
            Some(path) => path,
            None => return false,
        },
    };
    save_active_to(target)
}

fn suggested_file_name() -> String {
    with_shell(|shell| {
        let name = shell
            .app
            .documents
            .get(shell.app.active)
            .and_then(|document| document.path.as_ref())
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned());
        name.unwrap_or_else(|| format!("{}.ls", shell.app.tab_title(shell.app.active)))
    })
    .unwrap_or_else(|| "script.ls".to_string())
}

/// Converts a UTF-16 code-unit offset into a 1-based line and column, the way the status bar
/// reports the caret. Surrogate pairs count as one character; CRLF counts as one break.
fn line_col_utf16(text: &str, units: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    let mut consumed = 0usize;
    for ch in text.chars() {
        let width = ch.len_utf16();
        if consumed + width > units {
            break;
        }
        consumed += width;
        match ch {
            '\n' => {
                line += 1;
                column = 1;
            }
            '\r' => {}
            _ => column += 1,
        }
    }
    (line, column)
}

/// Converts a UTF-16 code-unit offset (what an EDIT control reports) into a byte offset into
/// the same text, always landing on a character boundary.
fn byte_offset_utf16(text: &str, units: usize) -> usize {
    let mut consumed = 0usize;
    for (byte, ch) in text.char_indices() {
        if consumed >= units {
            return byte;
        }
        consumed += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::{byte_offset_utf16, line_col_utf16};

    #[test]
    fn maps_utf16_offsets_to_line_and_column() {
        assert_eq!(line_col_utf16("ab\ncd", 0), (1, 1));
        assert_eq!(line_col_utf16("ab\ncd", 2), (1, 3));
        assert_eq!(line_col_utf16("ab\ncd", 3), (2, 1));
        assert_eq!(line_col_utf16("ab\ncd", 5), (2, 3));
        assert_eq!(line_col_utf16("ab\r\ncd", 4), (2, 1));
        assert_eq!(line_col_utf16("", 0), (1, 1));
    }

    #[test]
    fn maps_utf16_offsets_to_byte_offsets() {
        assert_eq!(byte_offset_utf16("abc", 0), 0);
        assert_eq!(byte_offset_utf16("abc", 2), 2);
        assert_eq!(byte_offset_utf16("abc", 99), 3);
        let text = "😀x";
        assert_eq!(byte_offset_utf16(text, 2), 4);
        assert_eq!(byte_offset_utf16(text, 3), 5);
    }

    #[test]
    fn surrogate_pairs_count_as_one_column() {
        assert_eq!(line_col_utf16("\u{1f600}x", 0), (1, 1));
        assert_eq!(line_col_utf16("\u{1f600}x", 2), (1, 2));
        assert_eq!(line_col_utf16("\u{1f600}x", 3), (1, 3));
    }
}
