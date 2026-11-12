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
//! - --journey <ID>: the five mandatory journeys (J-02, J-03, J-09, J-10, J-15), executed
//!   headlessly through the same workbench/`ExecutionHost` path and the same plot pane. It prints
//!   the observed values and exits non-zero when the expected observable in
//!   `docs/goal-desktop-v4/journeys.md` does not match.
//! - --selftest: the pre-cutover headless acceptance checks for the application core. Kept
//!   working on purpose; it is not the shipped UI.
//! - --raster-svg: the round-29 fidelity harness entry point. It rasterises one SVG file through
//!   the pane's exact resvg path (system fonts, PLT-002 resolver) to a PNG, so a reference
//!   rasteriser can be compared against the product rather than against a look-alike.
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
mod completion;
#[cfg(windows)]
mod diagnostic_map;
mod document_io;
mod engine;
mod execution;
mod gui_selftest;
mod history;
mod inspector;
#[cfg(windows)]
mod journey;
mod keyboard;
mod language;
mod log_kind;
mod netcheck;
mod quick_eval;
mod recovery;
mod results_tree;
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
mod workspace_table;

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|arg| arg == "--gui-selftest") {
        std::process::exit(gui_selftest::run());
    }
    if let Some(index) = args.iter().position(|arg| arg == "--journey") {
        std::process::exit(journey::run(&args[index + 1..]));
    }
    if let Some(index) = args.iter().position(|arg| arg == "--raster-svg") {
        std::process::exit(ui::plot::raster_cli(&args[index + 1..]));
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
    if args.iter().any(|arg| arg == "--journey") {
        eprintln!("--journey requires the Windows build");
        std::process::exit(2);
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
    text.push_str("  lovelace-desktop --journey <ID> [--journey <ID> ...] [--out <dir>]\n");
    text.push_str(
        "                                      run J-02/J-03/J-09/J-10/J-15 through the same\n",
    );
    text.push_str(
        "                                      workbench path; non-zero when the journeys.md\n",
    );
    text.push_str("                                      expected observable does not match\n");
    text.push_str(
        "  lovelace-desktop --selftest         headless acceptance of the pre-cutover core\n",
    );
    text.push_str("  lovelace-desktop --raster-svg <in.svg> --out <out.png> [--scale N]\n");
    text.push_str(
        "                                      rasterise one SVG exactly as the plot pane does\n",
    );
    text.push_str("  lovelace-desktop --legacy-win32 [script.ls]\n");
    text.push_str(
        "                                      open the old Win32 shell (explicit fallback only)\n",
    );
    text.push_str("  lovelace-desktop --help             this message\n");
    text.push_str("  lovelace-desktop --version          print the version\n\n");
    text.push_str("SHORTCUTS:\n");
    text.push_str("  Ctrl+N                              new document tab\n");
    text.push_str("  Ctrl+O                              open a Lovelace script from disk\n");
    text.push_str("  Ctrl+S                              save the active document\n");
    text.push_str("  Ctrl+Shift+S                        save the active document as a new file\n");
    text.push_str(
        "  Ctrl+W                              close the active tab (prompts when dirty)\n",
    );
    text.push_str("  Ctrl+Enter                          run the active document\n");
    text.push_str("  Esc                                 cancel the in-flight run\n");
    text.push_str("  Ctrl+Z                              undo the active document's last edit\n");
    text.push_str("  Ctrl+Y, Ctrl+Shift+Z                redo the last undone edit\n");
    text.push_str(
        "  F8                                  jump to the last failed run's diagnostic\n",
    );
    text.push_str(
        "  The File bar has New, Open..., Save, Save As..., Recent, Close and Quit; closing a\n",
    );
    text.push_str(
        "  document with unsaved changes asks Save / Discard / Cancel. The toolbar has Run,\n",
    );
    text.push_str("  Cancel (while running) and the precision control.\n");
    text.push_str(
        "  At launch the last session's tabs come back by reading their files again, and after a\n",
    );
    text.push_str(
        "  crash a recovery prompt offers the journaled buffers. Nothing is executed by either.\n",
    );
    text
}
