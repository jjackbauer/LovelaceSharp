//! Lovelace Desktop: a native Windows IDE for the Lovelace scripting language.
//!
//! The binary has two modes:
//!
//! - Default: the Win32 shell. One ordinary top-level window, a native multi-line EDIT control
//!   as the editor (so IME, clipboard, selection and scrolling are the platform's, not ours), a
//!   read-only EDIT for variables and functions, a read-only EDIT for the logs, and a custom
//!   child window that draws the engine's plot with GDI. There is no web view, no script engine
//!   and no server: the process is the application.
//! - --selftest: the headless acceptance mode. It drives the same AppState and Command values the
//!   shell uses, prints one SELFTEST line per check, and exits 0 only when every check passes.
//!
//! Safety: unsafe code appears only in src/win32, and only as (a) "unsafe extern system" FFI
//! declarations whose signatures mirror the Windows headers and (b) the callbacks Win32 invokes.
//! Every unsafe block in that module carries a SAFETY comment. The application core, settings,
//! SVG reader, PE-import scanner and selftest contain no unsafe code at all, which the
//! crate-level cfg_attr below enforces on non-Windows builds.

#![cfg_attr(not(windows), forbid(unsafe_code))]

mod app;
mod engine;
mod netcheck;
mod selftest;
mod settings;
mod svg;
#[cfg(windows)]
mod win32;

#[cfg(windows)]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

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

    let unknown: Vec<&String> = args.iter().filter(|arg| arg.starts_with("--")).collect();
    if !unknown.is_empty() {
        eprintln!("unknown option {}\n\n{}", unknown[0], usage());
        std::process::exit(2);
    }
    let initial = args.iter().find(|arg| !arg.starts_with("--")).cloned();
    std::process::exit(win32::shell::run(initial.as_deref()));
}

#[cfg(not(windows))]
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--selftest") {
        std::process::exit(selftest::run());
    }
    eprintln!("this build targets Windows; the native shell is unavailable");
    std::process::exit(2);
}

fn usage() -> String {
    let mut text = String::new();
    text.push_str("lovelace-desktop - a native IDE for the Lovelace scripting language\n\n");
    text.push_str("USAGE:\n");
    text.push_str("  lovelace-desktop [script.ls]   open the window, optionally on a script\n");
    text.push_str("  lovelace-desktop --selftest    run the headless acceptance checks\n");
    text.push_str("  lovelace-desktop --help        this message\n");
    text.push_str("  lovelace-desktop --version     print the version\n\n");
    text.push_str("SHORTCUTS:\n");
    text.push_str("  Ctrl+N new document      Ctrl+O open          Ctrl+S save\n");
    text.push_str("  Ctrl+Shift+S save as     Ctrl+Enter run      Esc cancel run\n");
    text.push_str("  Ctrl+Z undo              Ctrl+Y redo         Ctrl+W close tab\n");
    text
}
