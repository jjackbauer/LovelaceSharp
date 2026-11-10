//! Hand-written Win32 bindings, and the safe wrappers the shell uses.
//!
//! The workspace builds with no external crates, so there is no windows-sys here: the module
//! declares exactly the user32, gdi32, comctl32, comdlg32 and kernel32 entry points the shell
//! calls, with the documented ABI (stdcall on 32-bit, the x64 calling convention on 64-bit, which
//! is what "system" selects).
//!
//! Safety policy: this is the only place in the crate with FFI declarations, and every call site
//! below has a SAFETY comment stating why the handles, pointers and buffers are valid for the
//! duration of the call. Callers outside this module never see a raw pointer: they get functions
//! that take and return Rust values.
//!
//! The declarations themselves mirror the Windows headers:
//! - handles are pointer-sized integers (HWND, HDC, HFONT, ...),
//! - WPARAM is usize and LPARAM/LRESULT are isize,
//! - structures use repr(C) with the field order and types from the SDK headers.

#![allow(unsafe_code)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::c_void;
use std::path::PathBuf;

/// A window handle.
pub type HWND = isize;
/// A module instance handle.
pub type HINSTANCE = isize;
/// A menu handle.
pub type HMENU = isize;
/// An icon handle.
pub type HICON = isize;
/// A cursor handle.
pub type HCURSOR = isize;
/// A brush handle.
pub type HBRUSH = isize;
/// A device-context handle.
pub type HDC = isize;
/// A font handle.
pub type HFONT = isize;
/// A pen handle.
pub type HPEN = isize;
/// The first message parameter.
pub type WPARAM = usize;
/// The second message parameter.
pub type LPARAM = isize;
/// A message result.
pub type LRESULT = isize;
/// A window procedure.
pub type WNDPROC = Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>;

// ---------------------------------------------------------------------------------------------
// Constants (winuser.h, wingdi.h, commctrl.h)
// ---------------------------------------------------------------------------------------------

/// Overlapped window with caption, system menu, thick frame, minimise and maximise boxes.
pub const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
/// The window is initially visible.
pub const WS_VISIBLE: u32 = 0x1000_0000;
/// The window is a child window.
pub const WS_CHILD: u32 = 0x4000_0000;
/// Excludes the area occupied by child windows when drawing.
pub const WS_CLIPCHILDREN: u32 = 0x0200_0000;
/// Vertical scroll bar.
pub const WS_VSCROLL: u32 = 0x0020_0000;
/// Horizontal scroll bar.
pub const WS_HSCROLL: u32 = 0x0010_0000;
/// The control participates in tab navigation.
pub const WS_TABSTOP: u32 = 0x0001_0000;
/// A thin border.
pub const WS_BORDER: u32 = 0x0080_0000;
/// A sunken edge.
pub const WS_EX_CLIENTEDGE: u32 = 0x0000_0200;
/// The window appears on the taskbar.
pub const WS_EX_APPWINDOW: u32 = 0x0004_0000;

/// Edit control: text is left-aligned.
pub const ES_LEFT: u32 = 0x0000;
/// Edit control: multi-line.
pub const ES_MULTILINE: u32 = 0x0004;
/// Edit control: scroll vertically when the caret moves past the bottom.
pub const ES_AUTOVSCROLL: u32 = 0x0040;
/// Edit control: scroll horizontally when the caret moves past the right edge.
pub const ES_AUTOHSCROLL: u32 = 0x0080;
/// Edit control: read-only.
pub const ES_READONLY: u32 = 0x0800;
/// Edit control: Enter inserts a carriage return instead of activating the default button.
pub const ES_WANTRETURN: u32 = 0x1000;
/// Edit control: keep the selection visible when the control loses focus.
pub const ES_NOHIDESEL: u32 = 0x0100;

/// Redraw the window when its width changes.
pub const CS_HREDRAW: u32 = 0x0002;
/// Redraw the window when its height changes.
pub const CS_VREDRAW: u32 = 0x0001;

/// Let the system choose the window position.
pub const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
/// Show the window.
pub const SW_SHOW: i32 = 5;

/// WM_CREATE.
pub const WM_CREATE: u32 = 0x0001;
/// WM_DESTROY.
pub const WM_DESTROY: u32 = 0x0002;
/// WM_SIZE.
pub const WM_SIZE: u32 = 0x0005;
/// WM_SETFOCUS.
pub const WM_SETFOCUS: u32 = 0x0007;
/// WM_PAINT.
pub const WM_PAINT: u32 = 0x000F;
/// WM_CLOSE.
pub const WM_CLOSE: u32 = 0x0010;
/// WM_QUIT.
pub const WM_QUIT: u32 = 0x0012;
/// WM_ERASEBKGND.
pub const WM_ERASEBKGND: u32 = 0x0014;
/// WM_GETMINMAXINFO.
pub const WM_GETMINMAXINFO: u32 = 0x0024;
/// WM_SETFONT.
pub const WM_SETFONT: u32 = 0x0030;
/// WM_COMMAND.
pub const WM_COMMAND: u32 = 0x0111;
/// WM_TIMER.
pub const WM_TIMER: u32 = 0x0113;
/// WM_USER, the base of control-defined messages.
pub const WM_USER: u32 = 0x0400;

/// Edit notification: the text changed.
pub const EN_CHANGE: u16 = 0x0300;
/// Edit message: get the selection.
pub const EM_GETSEL: u32 = 0x00B0;
/// Edit message: set the selection.
pub const EM_SETSEL: u32 = 0x00B1;
/// Edit message: scroll the caret into view.
pub const EM_SCROLLCARET: u32 = 0x00B7;
/// Edit message: set the text limit.
pub const EM_SETLIMITTEXT: u32 = 0x00C5;

/// Status bar message: set the text of a part (WM_USER + 1).
pub const SB_SETTEXTW: u32 = WM_USER + 1;
/// Status bar style: show the size grip.
pub const SBARS_SIZEGRIP: u32 = 0x0100;
/// The status bar window class.
pub const STATUSCLASSNAME: &str = "msctls_statusbar32";
/// Common control class group: bar controls (status bar, toolbar).
pub const ICC_BAR_CLASSES: u32 = 0x0000_0004;

/// Menu item: a plain string.
pub const MF_STRING: u32 = 0x0000;
/// Menu item: a separator.
pub const MF_SEPARATOR: u32 = 0x0800;
/// Menu item: a popup.
pub const MF_POPUP: u32 = 0x0010;

/// Accelerator: a virtual key.
pub const FVIRTKEY: u8 = 0x01;
/// Accelerator: the Shift key.
pub const FSHIFT: u8 = 0x04;
/// Accelerator: the Control key.
pub const FCONTROL: u8 = 0x08;
/// Virtual key: Return.
pub const VK_RETURN: u16 = 0x0D;
/// Virtual key: Escape.
pub const VK_ESCAPE: u16 = 0x1B;

/// Message box: one OK button.
pub const MB_OK: u32 = 0x0000;
/// Message box: Yes, No, Cancel.
pub const MB_YESNOCANCEL: u32 = 0x0003;
/// Message box: error icon.
pub const MB_ICONERROR: u32 = 0x0010;
/// Message box: warning icon.
pub const MB_ICONWARNING: u32 = 0x0030;
/// Message box: information icon.
pub const MB_ICONINFORMATION: u32 = 0x0040;
/// Message box result: OK.
pub const IDOK: i32 = 1;
/// Message box result: Cancel.
pub const IDCANCEL: i32 = 2;
/// Message box result: Yes.
pub const IDYES: i32 = 6;
/// Message box result: No.
pub const IDNO: i32 = 7;

/// Open dialog: warn before overwriting.
pub const OFN_OVERWRITEPROMPT: u32 = 0x0000_0002;
/// Open dialog: hide the read-only check box.
pub const OFN_HIDEREADONLY: u32 = 0x0000_0004;
/// Open dialog: do not change the current directory.
pub const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
/// Open dialog: the path must exist.
pub const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
/// Open dialog: the file must exist.
pub const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
/// Open dialog: the Explorer-style dialog.
pub const OFN_EXPLORER: u32 = 0x0008_0000;

/// The default GUI font stock object.
pub const DEFAULT_GUI_FONT: i32 = 17;
/// Transparent text background mode.
pub const TRANSPARENT: i32 = 1;
/// Normal font weight.
pub const FW_NORMAL: i32 = 400;
/// Semibold font weight.
pub const FW_SEMIBOLD: i32 = 600;
/// The default character set.
pub const DEFAULT_CHARSET: u32 = 1;

/// The window background colour index plus one, as RegisterClassW expects it.
pub const COLOR_WINDOW_BRUSH: isize = 6;

// ---------------------------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------------------------

/// A point.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct POINT {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

/// A rectangle.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RECT {
    /// Left edge.
    pub left: i32,
    /// Top edge.
    pub top: i32,
    /// Right edge.
    pub right: i32,
    /// Bottom edge.
    pub bottom: i32,
}

/// A width and height.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SIZE {
    /// Width.
    pub cx: i32,
    /// Height.
    pub cy: i32,
}

/// The class registration record.
#[repr(C)]
pub struct WNDCLASSW {
    /// Class style.
    pub style: u32,
    /// Window procedure.
    pub lpfnWndProc: WNDPROC,
    /// Extra class bytes.
    pub cbClsExtra: i32,
    /// Extra window bytes.
    pub cbWndExtra: i32,
    /// Module instance.
    pub hInstance: HINSTANCE,
    /// Icon.
    pub hIcon: HICON,
    /// Cursor.
    pub hCursor: HCURSOR,
    /// Background brush.
    pub hbrBackground: HBRUSH,
    /// Menu resource name.
    pub lpszMenuName: *const u16,
    /// Class name.
    pub lpszClassName: *const u16,
}

/// A message.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MSG {
    /// Destination window.
    pub hwnd: HWND,
    /// Message identifier.
    pub message: u32,
    /// First parameter.
    pub wParam: WPARAM,
    /// Second parameter.
    pub lParam: LPARAM,
    /// Posting time.
    pub time: u32,
    /// Cursor position when posted.
    pub pt: POINT,
}

/// The paint state.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PAINTSTRUCT {
    /// Display DC to paint with.
    pub hdc: HDC,
    /// Whether the background should be erased.
    pub fErase: i32,
    /// The invalid rectangle.
    pub rcPaint: RECT,
    /// Reserved.
    pub fRestore: i32,
    /// Reserved.
    pub fIncPaint: i32,
    /// Reserved.
    pub reserved: [u8; 32],
}

impl Default for PAINTSTRUCT {
    fn default() -> PAINTSTRUCT {
        PAINTSTRUCT {
            hdc: 0,
            fErase: 0,
            rcPaint: RECT::default(),
            fRestore: 0,
            fIncPaint: 0,
            reserved: [0; 32],
        }
    }
}

/// Minimum/maximum tracking sizes for WM_GETMINMAXINFO.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MINMAXINFO {
    /// Reserved.
    pub ptReserved: POINT,
    /// Maximum size.
    pub ptMaxSize: POINT,
    /// Maximum position.
    pub ptMaxPosition: POINT,
    /// Minimum tracking size.
    pub ptMinTrackSize: POINT,
    /// Maximum tracking size.
    pub ptMaxTrackSize: POINT,
}

/// One keyboard accelerator.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ACCEL {
    /// Modifiers and the FVIRTKEY flag.
    pub fVirt: u8,
    /// Virtual key code.
    pub key: u16,
    /// Command identifier sent in WM_COMMAND.
    pub cmd: u16,
}

/// The common-controls initialisation record.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct INITCOMMONCONTROLSEX {
    /// Size of this structure.
    pub dwSize: u32,
    /// Which control classes to load.
    pub dwICC: u32,
}

/// The open/save dialog configuration.
#[repr(C)]
pub struct OPENFILENAMEW {
    /// Size of this structure, in bytes.
    pub lStructSize: u32,
    /// Owner window.
    pub hwndOwner: HWND,
    /// Instance handle.
    pub hInstance: HINSTANCE,
    /// Filter pairs, double NUL terminated.
    pub lpstrFilter: *const u16,
    /// Custom filter buffer.
    pub lpstrCustomFilter: *mut u16,
    /// Size of the custom filter buffer.
    pub nMaxCustFilter: u32,
    /// One-based index of the selected filter.
    pub nFilterIndex: u32,
    /// The file name buffer.
    pub lpstrFile: *mut u16,
    /// Size of the file name buffer, in UTF-16 units.
    pub nMaxFile: u32,
    /// Buffer for the file title.
    pub lpstrFileTitle: *mut u16,
    /// Size of the file title buffer.
    pub nMaxFileTitle: u32,
    /// Initial directory.
    pub lpstrInitialDir: *const u16,
    /// Dialog title.
    pub lpstrTitle: *const u16,
    /// Behaviour flags.
    pub Flags: u32,
    /// Offset of the file name within lpstrFile.
    pub nFileOffset: u16,
    /// Offset of the extension within lpstrFile.
    pub nFileExtension: u16,
    /// Default extension.
    pub lpstrDefExt: *const u16,
    /// Application-defined data.
    pub lCustData: LPARAM,
    /// Hook procedure.
    pub lpfnHook: isize,
    /// Custom template name.
    pub lpTemplateName: *const u16,
    /// Reserved.
    pub pvReserved: *mut c_void,
    /// Reserved.
    pub dwReserved: u32,
    /// Extended flags.
    pub FlagsEx: u32,
}

impl Default for OPENFILENAMEW {
    fn default() -> OPENFILENAMEW {
        OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: 0,
            hInstance: 0,
            lpstrFilter: std::ptr::null(),
            lpstrCustomFilter: std::ptr::null_mut(),
            nMaxCustFilter: 0,
            nFilterIndex: 0,
            lpstrFile: std::ptr::null_mut(),
            nMaxFile: 0,
            lpstrFileTitle: std::ptr::null_mut(),
            nMaxFileTitle: 0,
            lpstrInitialDir: std::ptr::null(),
            lpstrTitle: std::ptr::null(),
            Flags: 0,
            nFileOffset: 0,
            nFileExtension: 0,
            lpstrDefExt: std::ptr::null(),
            lCustData: 0,
            lpfnHook: 0,
            lpTemplateName: std::ptr::null(),
            pvReserved: std::ptr::null_mut(),
            dwReserved: 0,
            FlagsEx: 0,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Imports
// ---------------------------------------------------------------------------------------------

// SAFETY: these blocks declare the documented Windows entry points. Every argument type mirrors
// the SDK header (handles are pointer-sized, BOOL is i32, UINT is u32), so the calls below use the
// ABI the operating system expects. The declarations add no Rust-side preconditions beyond those
// documented for each call and stated at each call site.
#[link(name = "user32")]
unsafe extern "system" {
    /// Registers a window class.
    pub fn RegisterClassW(class: *const WNDCLASSW) -> u16;
    /// Creates a window.
    pub fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    /// The default window procedure.
    pub fn DefWindowProcW(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    /// Destroys a window.
    pub fn DestroyWindow(hwnd: HWND) -> i32;
    /// Shows a window.
    pub fn ShowWindow(hwnd: HWND, command: i32) -> i32;
    /// Processes pending paints.
    pub fn UpdateWindow(hwnd: HWND) -> i32;
    /// Retrieves a message.
    pub fn GetMessageW(message: *mut MSG, hwnd: HWND, filter_min: u32, filter_max: u32) -> i32;
    /// Translates accelerator keys.
    pub fn TranslateAcceleratorW(hwnd: HWND, table: isize, message: *mut MSG) -> i32;
    /// Translates virtual-key messages into character messages.
    pub fn TranslateMessage(message: *const MSG) -> i32;
    /// Dispatches a message to the window procedure.
    pub fn DispatchMessageW(message: *const MSG) -> LRESULT;
    /// Adds a message to the queue and returns immediately.
    pub fn PostMessageW(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> i32;
    /// Ends the message loop.
    pub fn PostQuitMessage(exit_code: i32);
    /// Sends a message and waits for it to be processed.
    pub fn SendMessageW(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    /// Sets the window text.
    pub fn SetWindowTextW(hwnd: HWND, text: *const u16) -> i32;
    /// Reads the window text.
    pub fn GetWindowTextW(hwnd: HWND, buffer: *mut u16, max_count: i32) -> i32;
    /// Returns the length of the window text, in UTF-16 units.
    pub fn GetWindowTextLengthW(hwnd: HWND) -> i32;
    /// Reads the client rectangle.
    pub fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> i32;
    /// Reads the window rectangle.
    pub fn GetWindowRect(hwnd: HWND, rect: *mut RECT) -> i32;
    /// Moves and resizes a window.
    pub fn MoveWindow(hwnd: HWND, x: i32, y: i32, width: i32, height: i32, repaint: i32) -> i32;
    /// Sets the keyboard focus.
    pub fn SetFocus(hwnd: HWND) -> HWND;
    /// Creates an empty menu.
    pub fn CreateMenu() -> HMENU;
    /// Creates an empty popup menu.
    pub fn CreatePopupMenu() -> HMENU;
    /// Appends a menu item.
    pub fn AppendMenuW(menu: HMENU, flags: u32, id: usize, text: *const u16) -> i32;
    /// Assigns a menu bar to a window.
    pub fn SetMenu(hwnd: HWND, menu: HMENU) -> i32;
    /// Destroys a menu.
    pub fn DestroyMenu(menu: HMENU) -> i32;
    /// Shows a message box.
    pub fn MessageBoxW(owner: HWND, text: *const u16, caption: *const u16, flags: u32) -> i32;
    /// Builds an accelerator table.
    pub fn CreateAcceleratorTableW(entries: *const ACCEL, count: i32) -> isize;
    /// Frees an accelerator table.
    pub fn DestroyAcceleratorTable(table: isize) -> i32;
    /// Invalidates a rectangle.
    pub fn InvalidateRect(hwnd: HWND, rect: *const RECT, erase: i32) -> i32;
    /// Starts painting.
    pub fn BeginPaint(hwnd: HWND, paint: *mut PAINTSTRUCT) -> HDC;
    /// Ends painting.
    pub fn EndPaint(hwnd: HWND, paint: *const PAINTSTRUCT) -> i32;
    /// Fills a rectangle.
    pub fn FillRect(hdc: HDC, rect: *const RECT, brush: HBRUSH) -> i32;
    /// Starts a timer.
    pub fn SetTimer(hwnd: HWND, id: usize, elapsed_ms: u32, callback: usize) -> usize;
    /// Stops a timer.
    pub fn KillTimer(hwnd: HWND, id: usize) -> i32;
    /// Loads a cursor.
    pub fn LoadCursorW(instance: HINSTANCE, name: *const u16) -> HCURSOR;
}

// SAFETY: as above; these are the documented gdi32 entry points used to draw the plot pane.
#[link(name = "gdi32")]
unsafe extern "system" {
    /// Creates a solid brush.
    pub fn CreateSolidBrush(color: u32) -> HBRUSH;
    /// Creates a pen.
    pub fn CreatePen(style: i32, width: i32, color: u32) -> HPEN;
    /// Creates a font.
    pub fn CreateFontW(
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u32,
        underline: u32,
        strike_out: u32,
        char_set: u32,
        out_precision: u32,
        clip_precision: u32,
        quality: u32,
        pitch_and_family: u32,
        face: *const u16,
    ) -> HFONT;
    /// Selects an object into a device context.
    pub fn SelectObject(hdc: HDC, object: isize) -> isize;
    /// Deletes a GDI object.
    pub fn DeleteObject(object: isize) -> i32;
    /// Sets the text colour.
    pub fn SetTextColor(hdc: HDC, color: u32) -> u32;
    /// Sets the background mode.
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    /// Draws text at a position.
    pub fn TextOutW(hdc: HDC, x: i32, y: i32, text: *const u16, count: i32) -> i32;
    /// Draws a connected sequence of line segments.
    pub fn Polyline(hdc: HDC, points: *const POINT, count: i32) -> i32;
    /// Draws a rectangle outline.
    pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32;
    /// Measures a string.
    pub fn GetTextExtentPoint32W(hdc: HDC, text: *const u16, count: i32, size: *mut SIZE) -> i32;
    /// Returns a stock object.
    pub fn GetStockObject(index: i32) -> isize;
}

// SAFETY: as above; the status bar comes from the common controls library.
#[link(name = "comctl32")]
unsafe extern "system" {
    /// Loads common control classes.
    pub fn InitCommonControlsEx(controls: *const INITCOMMONCONTROLSEX) -> i32;
}

// SAFETY: as above; GetOpenFileNameW and GetSaveFileNameW are the common dialog entry points.
#[link(name = "comdlg32")]
unsafe extern "system" {
    /// Shows the Open dialog.
    pub fn GetOpenFileNameW(dialog: *mut OPENFILENAMEW) -> i32;
    /// Shows the Save As dialog.
    pub fn GetSaveFileNameW(dialog: *mut OPENFILENAMEW) -> i32;
}

// SAFETY: as above; kernel32 supplies the module handle used when creating windows.
#[link(name = "kernel32")]
unsafe extern "system" {
    /// Returns the module handle of the running executable.
    pub fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
}

// ---------------------------------------------------------------------------------------------
// Safe wrappers
// ---------------------------------------------------------------------------------------------

/// Encodes text as a NUL-terminated UTF-16 string.
pub fn wide(text: &str) -> Vec<u16> {
    let mut buffer: Vec<u16> = text.encode_utf16().collect();
    buffer.push(0);
    buffer
}

/// Decodes a NUL-terminated UTF-16 buffer.
pub fn from_wide(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|unit| *unit == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

/// Shows a message box and returns the pressed button identifier.
pub fn message_box(owner: HWND, text: &str, caption: &str, flags: u32) -> i32 {
    let text = wide(text);
    let caption = wide(caption);
    // SAFETY: both buffers are NUL-terminated and alive across the call; MessageBoxW only reads
    // them and copies what it needs before returning.
    unsafe { MessageBoxW(owner, text.as_ptr(), caption.as_ptr(), flags) }
}

/// Sets a window's text.
pub fn set_text(hwnd: HWND, text: &str) {
    if hwnd == 0 {
        return;
    }
    let text = wide(text);
    // SAFETY: the buffer is NUL-terminated and outlives the synchronous SetWindowTextW call.
    unsafe {
        SetWindowTextW(hwnd, text.as_ptr());
    }
}

/// Reads a window's text.
pub fn window_text(hwnd: HWND) -> String {
    if hwnd == 0 {
        return String::new();
    }
    // SAFETY: GetWindowTextLengthW takes a valid handle (checked non-zero) and returns a length.
    let length = unsafe { GetWindowTextLengthW(hwnd) }.max(0) as usize;
    let mut buffer = vec![0u16; length + 1];
    // SAFETY: the buffer has room for length + 1 UTF-16 units, and GetWindowTextW never writes
    // past the given capacity. The count is passed as i32, matching the ABI.
    let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    let copied = copied.max(0) as usize;
    buffer.truncate(copied.min(buffer.len()));
    String::from_utf16_lossy(&buffer)
}

/// Sends a message.
pub fn send(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd == 0 {
        return 0;
    }
    // SAFETY: SendMessageW with a valid handle and plain integer parameters; the call is
    // synchronous, so no pointer stored in a parameter can dangle.
    unsafe { SendMessageW(hwnd, message, wparam, lparam) }
}

/// Posts a message.
pub fn post(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) {
    if hwnd == 0 {
        return;
    }
    // SAFETY: PostMessageW copies its arguments into the window's message queue.
    unsafe {
        PostMessageW(hwnd, message, wparam, lparam);
    }
}

/// Returns the client-area size.
pub fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    // SAFETY: rect is a valid writable RECT and the handle is checked by the caller's window.
    let ok = unsafe { GetClientRect(hwnd, &mut rect) };
    if ok == 0 {
        return (0, 0);
    }
    ((rect.right - rect.left).max(0), (rect.bottom - rect.top).max(0))
}

/// Returns the outer window size.
pub fn window_size(hwnd: HWND) -> (i32, i32) {
    let mut rect = RECT::default();
    // SAFETY: rect is a valid writable RECT.
    let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
    if ok == 0 {
        return (0, 0);
    }
    ((rect.right - rect.left).max(0), (rect.bottom - rect.top).max(0))
}

/// Moves and resizes a window.
pub fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    if hwnd == 0 {
        return;
    }
    // SAFETY: MoveWindow takes plain integers and a valid child handle.
    unsafe {
        MoveWindow(hwnd, x, y, width.max(0), height.max(0), 1);
    }
}

/// Sets a control's font.
pub fn set_font(hwnd: HWND, font: HFONT) {
    send(hwnd, WM_SETFONT, font as WPARAM, 1);
}

/// Invalidates a window so that it repaints.
pub fn invalidate(hwnd: HWND) {
    if hwnd == 0 {
        return;
    }
    // SAFETY: InvalidateRect accepts a null rectangle to mean the whole client area.
    unsafe {
        InvalidateRect(hwnd, std::ptr::null(), 0);
    }
}

/// Sets the status bar text.
pub fn set_status_text(hwnd: HWND, text: &str) {
    if hwnd == 0 {
        return;
    }
    let text = wide(text);
    // SAFETY: SB_SETTEXTW copies the NUL-terminated string before SendMessageW returns, and the
    // buffer outlives the call.
    unsafe {
        SendMessageW(hwnd, SB_SETTEXTW, 0, text.as_ptr() as LPARAM);
    }
}

/// Returns the editor's selection start as a UTF-16 code-unit offset.
pub fn editor_selection(hwnd: HWND) -> usize {
    // SAFETY: EM_GETSEL with null out-pointers returns the start position in the low word of the
    // result (the documented behaviour for a 16-bit-safe query).
    let result = unsafe { SendMessageW(hwnd, EM_GETSEL, 0, 0) };
    (result as usize) & 0xFFFF
}

/// Moves the editor's caret to a position.
pub fn editor_set_selection(hwnd: HWND, position: usize) {
    send(hwnd, EM_SETSEL, position, position as isize);
}

/// Scrolls the editor to the caret.
pub fn editor_scroll_to_end(hwnd: HWND) {
    send(hwnd, EM_SCROLLCARET, 0, 0);
    // Moving the selection to the end is what actually scrolls a read-only edit control.
    let length = window_text(hwnd).encode_utf16().count();
    send(hwnd, EM_SETSEL, length, length as isize);
    send(hwnd, EM_SCROLLCARET, 0, 0);
}

/// Starts a timer.
pub fn set_timer(hwnd: HWND, id: usize, milliseconds: u32) {
    // SAFETY: SetTimer with no callback posts WM_TIMER to the window, which is what we want.
    unsafe {
        SetTimer(hwnd, id, milliseconds, 0);
    }
}

/// Stops a timer.
pub fn kill_timer(hwnd: HWND, id: usize) {
    // SAFETY: kills a timer previously created for this window.
    unsafe {
        KillTimer(hwnd, id);
    }
}

/// Returns the executable's module handle.
pub fn module_handle() -> HINSTANCE {
    // SAFETY: a null module name asks for the running executable's handle.
    unsafe { GetModuleHandleW(std::ptr::null()) }
}

/// Loads the standard arrow cursor.
pub fn load_arrow_cursor() -> HCURSOR {
    // SAFETY: IDC_ARROW (32512) is a system resource identifier, cast to the pointer form the API
    // documents for MAKEINTRESOURCE.
    unsafe { LoadCursorW(0, 32512usize as *const u16) }
}

/// Loads the status bar class.
pub fn init_common_controls() -> bool {
    let controls = INITCOMMONCONTROLSEX {
        dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_BAR_CLASSES,
    };
    // SAFETY: the structure is initialised with its own size, as the API requires.
    unsafe { InitCommonControlsEx(&controls) != 0 }
}

/// Registers a window class.
pub fn register_class(name: &str, procedure: WNDPROC, background: HBRUSH) -> bool {
    let name = wide(name);
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: procedure,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: module_handle(),
        hIcon: 0,
        hCursor: load_arrow_cursor(),
        hbrBackground: background,
        lpszMenuName: std::ptr::null(),
        lpszClassName: name.as_ptr(),
    };
    // SAFETY: the WNDCLASSW is fully initialised, the class name is NUL-terminated and alive for
    // the call, and RegisterClassW copies the name before returning.
    unsafe { RegisterClassW(&class) != 0 }
}

/// Creates a window.
pub fn create_window(
    ex_style: u32,
    class: &str,
    title: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HWND,
    menu: HMENU,
    instance: HINSTANCE,
) -> HWND {
    let class = wide(class);
    let title = wide(title);
    // SAFETY: both strings are NUL-terminated and alive across the call; the parent and menu
    // handles are either zero or valid handles owned by this process.
    unsafe {
        CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            menu,
            instance,
            std::ptr::null_mut(),
        )
    }
}

/// Creates a menu bar.
pub fn create_menu() -> HMENU {
    // SAFETY: no arguments; returns a new menu handle or zero.
    unsafe { CreateMenu() }
}

/// Creates a popup menu.
pub fn create_popup_menu() -> HMENU {
    // SAFETY: no arguments; returns a new menu handle or zero.
    unsafe { CreatePopupMenu() }
}

/// Appends a menu item.
pub fn append_menu(menu: HMENU, flags: u32, id: usize, text: &str) {
    let text = wide(text);
    // SAFETY: the menu handle is owned by this process and the text is NUL-terminated; AppendMenuW
    // copies the string.
    unsafe {
        AppendMenuW(menu, flags, id, text.as_ptr());
    }
}

/// Builds an accelerator table from (flags, key, command) triples.
pub fn create_accelerators(entries: &[(u8, u16, u16)]) -> isize {
    let accelerators: Vec<ACCEL> = entries
        .iter()
        .map(|(virt, key, cmd)| ACCEL { fVirt: *virt, key: *key, cmd: *cmd })
        .collect();
    // SAFETY: the array is valid for accelerators.len() entries and CreateAcceleratorTableW copies
    // it before returning.
    unsafe { CreateAcceleratorTableW(accelerators.as_ptr(), accelerators.len() as i32) }
}

/// Frees an accelerator table.
pub fn destroy_accelerators(table: isize) {
    if table == 0 {
        return;
    }
    // SAFETY: the table was created by create_accelerators and is destroyed exactly once.
    unsafe {
        DestroyAcceleratorTable(table);
    }
}

/// Asks the canonical window procedure to handle a message.
pub fn def_window_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: DefWindowProcW is the documented fallback for unhandled messages.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// Returns whether a null-or-zero handle represents failure.
pub fn is_null(handle: isize) -> bool {
    handle == 0
}

/// Creates a font.
pub fn create_font(height: i32, weight: i32, face: &str) -> HFONT {
    let face = wide(face);
    // SAFETY: the face name is NUL-terminated and alive across the call; all other parameters are
    // the documented defaults for a UI font.
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            0,
            0,
            0,
            0,
            face.as_ptr(),
        )
    }
}

/// Creates a solid brush.
pub fn create_brush(color: u32) -> HBRUSH {
    // SAFETY: no pointers involved; returns a new GDI object or zero.
    unsafe { CreateSolidBrush(color) }
}

/// Creates a pen.
pub fn create_pen(width: i32, color: u32) -> HPEN {
    // SAFETY: PS_SOLID is 0; no pointers involved.
    unsafe { CreatePen(0, width, color) }
}

/// Deletes a GDI object.
pub fn delete_object(object: isize) {
    if object == 0 {
        return;
    }
    // SAFETY: the object was created by this module and is deleted exactly once, after it is no
    // longer selected into any device context.
    unsafe {
        DeleteObject(object);
    }
}

/// Selects a GDI object into a device context.
pub fn select_object(hdc: HDC, object: isize) -> isize {
    // SAFETY: hdc is the DC currently being painted and object is a GDI handle created above.
    unsafe { SelectObject(hdc, object) }
}

/// Sets the text colour.
pub fn set_text_color(hdc: HDC, color: u32) {
    // SAFETY: hdc is the DC currently being painted.
    unsafe {
        SetTextColor(hdc, color);
    }
}

/// Sets the background mode.
pub fn set_bk_mode(hdc: HDC, mode: i32) {
    // SAFETY: hdc is the DC currently being painted.
    unsafe {
        SetBkMode(hdc, mode);
    }
}

/// Draws text at a position.
pub fn text_out(hdc: HDC, x: i32, y: i32, text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return;
    }
    // SAFETY: the UTF-16 buffer is valid for units.len() elements, which is passed as the count,
    // so no terminator is read.
    unsafe {
        TextOutW(hdc, x, y, units.as_ptr(), units.len() as i32);
    }
}

/// Draws a polyline.
pub fn polyline(hdc: HDC, points: &[POINT]) {
    if points.len() < 2 {
        return;
    }
    // SAFETY: the POINT array is valid for points.len() elements, passed as the count.
    unsafe {
        Polyline(hdc, points.as_ptr(), points.len() as i32);
    }
}

/// Draws a rectangle outline.
pub fn rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) {
    // SAFETY: hdc is the DC currently being painted.
    unsafe {
        Rectangle(hdc, left, top, right, bottom);
    }
}

/// Fills a rectangle with a brush.
pub fn fill_rect(hdc: HDC, rect: &RECT, brush: HBRUSH) {
    if brush == 0 {
        return;
    }
    // SAFETY: rect points to a live RECT and brush is a GDI brush owned by the shell.
    unsafe {
        FillRect(hdc, rect, brush);
    }
}

/// Measures the width of a string in the current font.
pub fn text_width(hdc: HDC, text: &str) -> i32 {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return 0;
    }
    let mut size = SIZE::default();
    // SAFETY: the UTF-16 buffer is valid for units.len() elements and size is a writable SIZE.
    let ok = unsafe { GetTextExtentPoint32W(hdc, units.as_ptr(), units.len() as i32, &mut size) };
    if ok == 0 { 0 } else { size.cx }
}

/// Begins painting.
pub fn paint_begin(hwnd: HWND) -> (HDC, PAINTSTRUCT) {
    let mut paint = PAINTSTRUCT::default();
    // SAFETY: paint is a writable PAINTSTRUCT; BeginPaint validates the handle.
    let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
    (hdc, paint)
}

/// Ends painting.
pub fn paint_end(hwnd: HWND, paint: &PAINTSTRUCT) {
    // SAFETY: paint is the structure that BeginPaint filled in for this same window.
    unsafe {
        EndPaint(hwnd, paint);
    }
}

/// Posts the quit message.
pub fn post_quit(code: i32) {
    // SAFETY: PostQuitMessage only sets the thread's quit flag.
    unsafe {
        PostQuitMessage(code);
    }
}

/// Destroys a window.
pub fn destroy_window(hwnd: HWND) -> bool {
    // SAFETY: the handle belongs to this thread and this call is the single destruction path.
    unsafe { DestroyWindow(hwnd) != 0 }
}

/// Shows a window.
pub fn show_window(hwnd: HWND) {
    // SAFETY: the handle is a live top-level window.
    unsafe {
        ShowWindow(hwnd, SW_SHOW);
    }
}

/// Processes pending paint messages.
pub fn update_window(hwnd: HWND) {
    // SAFETY: the handle is a live window.
    unsafe {
        UpdateWindow(hwnd);
    }
}

/// Sets the keyboard focus.
pub fn set_focus(hwnd: HWND) {
    if hwnd == 0 {
        return;
    }
    // SAFETY: the handle is a live control belonging to this thread.
    unsafe {
        SetFocus(hwnd);
    }
}

/// Reads one message from the queue. Returns a negative value on error, zero on WM_QUIT.
pub fn get_message(message: &mut MSG) -> i32 {
    // SAFETY: message is a live, writable MSG; the filters are zero, so every message is returned.
    unsafe { GetMessageW(message, 0, 0, 0) }
}

/// Offers a message to the accelerator table. Returns true when it was consumed.
pub fn translate_accelerator(hwnd: HWND, table: isize, message: &mut MSG) -> bool {
    if table == 0 {
        return false;
    }
    // SAFETY: message is the live MSG just returned by get_message and table is a live handle.
    unsafe { TranslateAcceleratorW(hwnd, table, message) != 0 }
}

/// Translates character messages.
pub fn translate_message(message: &MSG) {
    // SAFETY: message is a live MSG owned by the caller.
    unsafe {
        TranslateMessage(message);
    }
}

/// Dispatches a message.
pub fn dispatch_message(message: &MSG) {
    // SAFETY: message is a live MSG owned by the caller.
    unsafe {
        DispatchMessageW(message);
    }
}

/// Shows the Open dialog and returns the chosen path.
pub fn open_file_dialog(owner: HWND) -> Option<PathBuf> {
    let filter: Vec<u16> = "Lovelace scripts (*.ls)\0*.ls;*.lvl\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title = wide("Open Lovelace script");
    let mut file = vec![0u16; 2048];
    let mut dialog = OPENFILENAMEW {
        hwndOwner: owner,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: file.as_mut_ptr(),
        nMaxFile: file.len() as u32,
        lpstrTitle: title.as_ptr(),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_EXPLORER | OFN_NOCHANGEDIR
            | OFN_HIDEREADONLY,
        ..OPENFILENAMEW::default()
    };
    // SAFETY: every pointer in the structure refers to a local buffer that is alive for the whole
    // call, and lStructSize was set by Default. The dialog reads and writes only those buffers.
    let ok = unsafe { GetOpenFileNameW(&mut dialog) };
    if ok == 0 {
        return None;
    }
    let path = from_wide(&file);
    if path.is_empty() { None } else { Some(PathBuf::from(path)) }
}

/// Shows the Save As dialog and returns the chosen path.
pub fn save_file_dialog(owner: HWND, suggested: &str) -> Option<PathBuf> {
    let filter: Vec<u16> = "Lovelace scripts (*.ls)\0*.ls\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title = wide("Save Lovelace script");
    let extension = wide("ls");
    let mut file = vec![0u16; 2048];
    for (index, unit) in suggested.encode_utf16().enumerate() {
        if index + 1 < file.len() {
            file[index] = unit;
        }
    }
    let mut dialog = OPENFILENAMEW {
        hwndOwner: owner,
        lpstrFilter: filter.as_ptr(),
        nFilterIndex: 1,
        lpstrFile: file.as_mut_ptr(),
        nMaxFile: file.len() as u32,
        lpstrTitle: title.as_ptr(),
        lpstrDefExt: extension.as_ptr(),
        Flags: OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_EXPLORER | OFN_NOCHANGEDIR,
        ..OPENFILENAMEW::default()
    };
    // SAFETY: as for open_file_dialog; all pointers are to locals alive across the call.
    let ok = unsafe { GetSaveFileNameW(&mut dialog) };
    if ok == 0 {
        return None;
    }
    let path = from_wide(&file);
    if path.is_empty() { None } else { Some(PathBuf::from(path)) }
}

/// A stock object index for the default GUI font, for callers that need it.
pub const fn default_gui_font() -> isize {
    DEFAULT_GUI_FONT as isize
}
