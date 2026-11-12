//! Lovelace Desktop: a native Windows IDE for the Lovelace scripting language.
//!
//! The binary has four modes:
//!
//! - Default: the new native workbench. An eframe/egui 0.37 window with a virtualised editor,
//!   a workspace pane fed by the real engine's snapshots, a logs pane fed by the typed
//!   `ExecutionHost` event stream, an SVG plot pane fed by the engine's `PlotCapture`, tabs and
//!   a status strip. Scripts are evaluated on a worker thread; the window never blocks.
//! - --gui-selftest: the headless acceptance mode for the same workbench. It drives the real
//!   engine through `ExecutionHost` exactly as the window does, prints one line per check and
//!   exits non-zero on failure.
//! - --selftest: the pre-cutover headless acceptance checks for the application core. Kept
//!   working on purpose; it is not the shipped UI.
//! - --legacy-win32: the old Win32 EDIT-control shell, reachable only through this explicit
//!   argument. It is no longer the default UI.
//!
//! Safety: unsafe code appears only in src/win32, and only as (a) "unsafe extern system" FFI
//! declarations whose signatures mirror the Windows headers and (b) the callbacks Win32 invokes.
//! Every unsafe block in that module carries a SAFETY comment. The workbench, execution core,
//! settings, SVG reader, PE-import scanner and both selftests contain no unsafe code, which the
//! crate-level cfg_attr below enforces on non-Windows builds.

#![cfg_attr(not(windows), forbid(unsafe_code))]

mod app;
#[cfg(windows)]
mod diagnostic_map;
mod completion;
mod engine;
mod execution;
mod gui_selftest;
mod keyboard;
mod language;
mod netcheck;
mod run_state;
mod run_trace;
mod selftest;
mod settings;
mod statements;
mod svg;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win32;
mod workbench;

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--gui-selftest") {
        std::process::exit(gui_selftest::run());
    }
    if args.iter().any(|arg| arg == "--selftest") {
        std::process::exit(selftest::run());
    }
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print!("{}", usage());
        return;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("lovelace-desktop {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.iter().any(|arg| arg == "--legacy-win32") {
        let initial = args.iter().find(|arg| !arg.starts_with("--")).cloned();
        std::process::exit(win32::shell::run(initial.as_deref()));
    }

    let unknown: Vec<&String> = args.iter().filter(|arg| arg.starts_with("--")).collect();
    if !unknown.is_empty() {
        eprintln!("unknown option {}\n\n{}", unknown[0], usage());
        std::process::exit(2);
    }
    let initial = args.iter().find(|arg| !arg.starts_with("--")).cloned();
    std::process::exit(ui::run(initial.map(std::path::PathBuf::from)));
}

#[cfg(not(windows))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--gui-selftest") {
        std::process::exit(gui_selftest::run());
    }
    if args.iter().any(|arg| arg == "--selftest") {
        std::process::exit(selftest::run());
    }
    eprintln!("this build targets Windows; the native workbench is unavailable");
    std::process::exit(2);
}

fn usage() -> String {
    let mut text = String::new();
    text.push_str("lovelace-desktop - a native IDE for the Lovelace scripting language\n\n");
    text.push_str("USAGE:\n");
    text.push_str("  lovelace-desktop [script.ls]        open the native workbench\n");
    text.push_str(
        "  lovelace-desktop --gui-selftest     headless acceptance of the workbench run path\n",
    );
    text.push_str(
        "  lovelace-desktop --selftest         headless acceptance of the pre-cutover core\n",
    );
    text.push_str("  lovelace-desktop --legacy-win32 [script.ls]\n");
    text.push_str(
        "                                      open the old Win32 shell (explicit fallback only)\n",
    );
    text.push_str("  lovelace-desktop --help             this message\n");
    text.push_str("  lovelace-desktop --version          print the version\n\n");
    text.push_str("SHORTCUTS:\n");
    text.push_str("  Ctrl+Enter                          run the active document\n");
    text.push_str("  Esc                                 cancel the in-flight run\n");
    text.push_str(
        "  F8                                  jump to the last failed run's diagnostic\n",
    );
    text.push_str(
        "  The workbench toolbar has Run, Cancel (while running) and the precision control.\n",
    );
    text
}
