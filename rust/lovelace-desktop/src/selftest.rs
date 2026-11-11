//! The --selftest acceptance mode: the whole application core, exercised without a window.
//!
//! Every check builds a fresh AppState and drives it through the same Command values the shell
//! uses, then asserts on the pane models. Nothing here needs a message pump, a desktop session
//! or a window station, which is what makes it usable on a build machine. The final check is the
//! structural one: the executable that is running must not import any DLL that can open a socket.

use std::path::PathBuf;

use crate::app::{AppState, Command, Effect, LOG_CAP, LogLevel, RunState};
use crate::engine::RunOutput;
use crate::netcheck;
use crate::settings::{SCHEMA_VERSION, Settings};
use crate::svg;

type Check = fn() -> Result<(), String>;

/// Runs every check, prints one SELFTEST line per check, and returns the process exit code.
pub fn run() -> i32 {
    let engine_name = crate::engine::default_engine_factory()().name();
    println!(
        "SELFTEST lovelace-desktop {} engine={}",
        env!("CARGO_PKG_VERSION"),
        engine_name
    );
    let checks: [(&str, Check); 15] = [
        ("new-document", check_new_document),
        ("insert-text", check_insert_text),
        ("open-file", check_open_file),
        ("run", check_run),
        ("cancel", check_cancel),
        ("precision", check_precision),
        ("save-round-trip", check_save_round_trip),
        ("tab-isolation", check_tab_isolation),
        ("undo-redo", check_undo_redo),
        ("settings-round-trip", check_settings_round_trip),
        ("settings-corrupt", check_settings_corrupt),
        ("log-cap", check_log_cap),
        ("svg-parse", check_svg_parse),
        ("engine-plot", check_engine_plot),
        ("no-network-imports", check_no_network_imports),
    ];

    let mut failed = 0usize;
    for (name, check) in checks {
        match check() {
            Ok(()) => println!("SELFTEST ok {name}"),
            Err(reason) => {
                failed += 1;
                println!("SELFTEST FAIL {name}: {reason}");
            }
        }
    }
    println!("SELFTEST summary checks={} failed={failed}", checks.len());
    if failed == 0 { 0 } else { 1 }
}

fn app() -> AppState {
    AppState::new()
}

fn scratch(tag: &str) -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join(format!(
        "lovelace-desktop-selftest-{}-{tag}",
        std::process::id()
    ));
    if std::fs::create_dir_all(&base).is_ok() {
        return Ok(base);
    }
    let fallback = std::env::current_dir()
        .map_err(|error| format!("no current directory: {error}"))?
        .join("target")
        .join("selftest")
        .join(format!("{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&fallback)
        .map_err(|error| format!("no writable scratch directory: {error}"))?;
    Ok(fallback)
}

fn run_output(app: &AppState) -> String {
    app.logs_text()
}

fn check_new_document() -> Result<(), String> {
    let mut app = app();
    if app.documents.len() != 1 {
        return Err(format!(
            "expected one document, found {}",
            app.documents.len()
        ));
    }
    let effects = app.apply(Command::NewDocument);
    if app.documents.len() != 2 || app.active != 1 {
        return Err(format!(
            "new document did not select a second tab (documents {}, active {})",
            app.documents.len(),
            app.active
        ));
    }
    if !effects.contains(&Effect::Repaint) {
        return Err("NewDocument did not request a repaint".to_string());
    }
    if app.documents[0].precision != 1000 {
        return Err(format!(
            "default precision was {}, expected 1000",
            app.documents[0].precision
        ));
    }
    Ok(())
}

fn check_insert_text() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::InsertText("1 + 2\n".to_string()));
    let document = &app.documents[0];
    if document.buffer != "1 + 2\n" {
        return Err(format!("buffer is {:?}", document.buffer));
    }
    if !document.dirty || document.revision != 1 {
        return Err(format!(
            "dirty {} revision {} after one insert",
            document.dirty, document.revision
        ));
    }
    if document.line_col() != (2, 1) {
        return Err(format!(
            "cursor is at {:?}, expected (2, 1)",
            document.line_col()
        ));
    }
    app.apply(Command::InsertText("3".to_string()));
    if app.documents[0].buffer != "1 + 2\n3" {
        return Err(format!(
            "second insert produced {:?}",
            app.documents[0].buffer
        ));
    }
    Ok(())
}

fn check_open_file() -> Result<(), String> {
    let dir = scratch("open")?;
    let path = dir.join("opened.ls");
    std::fs::write(&path, "opened = 1\n").map_err(|error| format!("write failed: {error}"))?;
    let mut app = app();
    let effects = app.apply(Command::Open(path.clone()));
    if !effects.contains(&Effect::Repaint) {
        return Err("open did not request a repaint".to_string());
    }
    if app.documents.len() != 2 {
        return Err(format!("open added {} documents", app.documents.len() - 1));
    }
    let document = &app.documents[1];
    if document.buffer != "opened = 1\n" {
        return Err(format!("opened buffer is {:?}", document.buffer));
    }
    if document.path.as_deref() != Some(path.as_path()) {
        return Err("opened document did not record its path".to_string());
    }
    if document.dirty {
        return Err("a freshly opened document must not be dirty".to_string());
    }
    let recent = path.to_string_lossy().into_owned();
    if !app.settings.recent_files.contains(&recent) {
        return Err("recent files did not record the opened path".to_string());
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn check_run() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::InsertText("a = 2 * 3\na + 1".to_string()));
    let effects = app.run_blocking();
    if !effects
        .iter()
        .any(|effect| matches!(effect, Effect::RunRequested(_)))
    {
        return Err("run did not produce a RunRequested effect".to_string());
    }
    let logs = run_output(&app);
    if !logs.contains("= 7") {
        return Err(format!("logs do not contain the result: {logs:?}"));
    }
    if !app.documents[0]
        .vars
        .iter()
        .any(|(name, value)| name == "a" && value.contains('6'))
    {
        return Err(format!(
            "variables do not contain a = 6: {:?}",
            app.documents[0].vars
        ));
    }
    if app.run_state != RunState::Idle {
        return Err("run left the run state at running".to_string());
    }
    if app.documents[0].revision != 2 {
        return Err(format!(
            "revision is {}, expected 2",
            app.documents[0].revision
        ));
    }
    Ok(())
}

fn check_cancel() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::InsertText("1 + 1".to_string()));
    let effects = app.apply(Command::Run);
    if !effects
        .iter()
        .any(|effect| matches!(effect, Effect::RunRequested(_)))
    {
        return Err("run did not produce a RunRequested effect".to_string());
    }
    if app.run_state != RunState::Running || app.pending_run().is_none() {
        return Err("state did not enter running".to_string());
    }
    app.apply(Command::Cancel);
    if app.run_state != RunState::Idle || app.pending_run().is_some() {
        return Err("cancel did not return the state to idle".to_string());
    }
    if !app.logs_text().contains("Run cancelled.") {
        return Err("cancel was not logged".to_string());
    }
    let stale = RunOutput {
        result: Ok("2".to_string()),
        output: vec!["late".to_string()],
        variables: vec![("late".to_string(), "1".to_string())],
        functions: Vec::new(),
        plot: None,
    };
    let revision = app.documents[0].revision;
    if !app.finish_run(0, stale, 3).is_empty() {
        return Err("a cancelled run published a result".to_string());
    }
    if app.documents[0].revision != revision || !app.documents[0].vars.is_empty() {
        return Err("a cancelled run mutated the document".to_string());
    }
    Ok(())
}

fn check_precision() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::NewDocument);
    app.apply(Command::SetPrecision(250));
    if app.documents[1].precision != 250 {
        return Err(format!(
            "document precision is {}",
            app.documents[1].precision
        ));
    }
    if app.documents[1].engine().precision() != 250 {
        return Err(format!(
            "engine precision is {}",
            app.documents[1].engine().precision()
        ));
    }
    if app.documents[0].precision != 1000 {
        return Err("precision leaked into the other document".to_string());
    }
    if app.settings.active_precision != 250 {
        return Err("active precision was not recorded in settings".to_string());
    }
    app.apply(Command::SetPrecision(0));
    if app.documents[1].precision != 1 {
        return Err("precision was not clamped to the minimum".to_string());
    }
    Ok(())
}

fn check_save_round_trip() -> Result<(), String> {
    let dir = scratch("save")?;
    let path = dir.join("round-trip.ls");
    let mut app = app();
    app.apply(Command::InsertText("b = 21\nb * 2\n".to_string()));
    let effects = app.apply(Command::SaveAs(path.clone()));
    if !effects
        .iter()
        .any(|effect| matches!(effect, Effect::Saved(saved) if saved == &path))
    {
        return Err("save did not produce a Saved effect".to_string());
    }
    if app.documents[0].dirty {
        return Err("document is still dirty after saving".to_string());
    }
    let on_disk = std::fs::read_to_string(&path).map_err(|error| format!("read back: {error}"))?;
    if on_disk != "b = 21\nb * 2\n" {
        return Err(format!("file contains {on_disk:?}"));
    }
    app.apply(Command::Open(path.clone()));
    if app.documents[1].buffer != on_disk {
        return Err("reopened buffer differs from the saved bytes".to_string());
    }
    if app.documents[1].dirty {
        return Err("reopened document is dirty".to_string());
    }
    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}

fn check_tab_isolation() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::NewDocument);
    app.apply(Command::InsertText("a = 1".to_string()));
    app.run_blocking();
    if !app.documents[1].vars.iter().any(|(name, _)| name == "a") {
        return Err("the run did not publish a variable in its own document".to_string());
    }
    if !app.documents[0].buffer.is_empty()
        || app.documents[0].revision != 0
        || !app.documents[0].vars.is_empty()
    {
        return Err("running in one document mutated the other".to_string());
    }
    app.apply(Command::SelectTab(0));
    app.apply(Command::InsertText("a".to_string()));
    let effects = app.run_blocking();
    if !effects
        .iter()
        .any(|effect| matches!(effect, Effect::Error(_)))
    {
        return Err("the other document's engine unexpectedly knew the variable".to_string());
    }
    if !app.logs_text().contains("Undefined variable 'a'.") {
        return Err("the undefined-variable error was not logged".to_string());
    }
    if app.documents[1].vars.is_empty() {
        return Err("the first document lost its variables".to_string());
    }
    Ok(())
}

fn check_undo_redo() -> Result<(), String> {
    let mut app = app();
    app.apply(Command::InsertText("ab".to_string()));
    app.apply(Command::InsertText("cd".to_string()));
    app.apply(Command::Undo);
    if app.documents[0].buffer != "ab" {
        return Err(format!("undo left {:?}", app.documents[0].buffer));
    }
    app.apply(Command::Redo);
    if app.documents[0].buffer != "abcd" {
        return Err(format!("redo left {:?}", app.documents[0].buffer));
    }
    app.apply(Command::Undo);
    app.apply(Command::Undo);
    if !app.documents[0].buffer.is_empty() {
        return Err("undo did not reach the empty buffer".to_string());
    }
    if app.documents[0].dirty {
        return Err("undo back to the saved text left the document dirty".to_string());
    }
    Ok(())
}

fn check_settings_round_trip() -> Result<(), String> {
    let mut settings = Settings::new();
    settings.active_precision = 2500;
    settings.window.width = 1440;
    settings.window.height = 900;
    settings.layout.editor_width_permille = 640;
    settings.add_recent("C:/scripts/selftest.ls");
    let text = settings.to_json();
    let again = Settings::from_json(&text);
    if again != settings {
        return Err(format!("round trip changed the settings: {text}"));
    }
    if !again.warnings.is_empty() {
        return Err(format!(
            "round trip produced warnings: {:?}",
            again.warnings
        ));
    }
    if SCHEMA_VERSION != 1 {
        return Err(format!("unexpected schema version {SCHEMA_VERSION}"));
    }
    Ok(())
}

fn check_settings_corrupt() -> Result<(), String> {
    let broken = Settings::from_json("{ this is not json");
    if broken.window.width != 1280 || broken.active_precision != 1000 {
        return Err("corrupt settings did not fall back to the defaults".to_string());
    }
    if broken.warnings.is_empty() {
        return Err("corrupt settings recorded no warning".to_string());
    }
    let newer = Settings::from_json(&format!(
        "{{\"schema_version\": {}, \"active_precision\": 7}}",
        SCHEMA_VERSION + 1
    ));
    if newer.active_precision != 1000 {
        return Err("newer-schema settings were partially accepted".to_string());
    }
    if !newer
        .warnings
        .iter()
        .any(|warning| warning.contains("newer than supported"))
    {
        return Err(format!(
            "newer-schema warning missing: {:?}",
            newer.warnings
        ));
    }
    Ok(())
}

fn check_log_cap() -> Result<(), String> {
    let mut app = app();
    for index in 0..(LOG_CAP + 10) {
        app.push_log(LogLevel::Info, format!("line {index}"));
    }
    if app.logs.len() != LOG_CAP {
        return Err(format!(
            "log holds {} entries, cap is {LOG_CAP}",
            app.logs.len()
        ));
    }
    if app.logs[0].text != "line 10" {
        return Err(format!(
            "oldest entry is {:?}, expected line 10",
            app.logs[0].text
        ));
    }
    Ok(())
}

fn check_svg_parse() -> Result<(), String> {
    let generated = svg::make_plot_svg("selftest plot", &[1, 4, 2, 8]);
    let plot = svg::parse(&generated);
    if plot.title.as_deref() != Some("selftest plot") {
        return Err(format!("title is {:?}", plot.title));
    }
    if plot.polylines.len() != 1 || plot.polylines[0].points.len() != 4 {
        return Err(format!(
            "polylines {}, points {}",
            plot.polylines.len(),
            plot.polylines
                .first()
                .map(|line| line.points.len())
                .unwrap_or(0)
        ));
    }
    if plot.texts.is_empty() {
        return Err("no text labels were parsed".to_string());
    }
    if plot.width != 640 * svg::MILLI {
        return Err(format!("width is {}", plot.width));
    }
    Ok(())
}

fn check_engine_plot() -> Result<(), String> {
    let mut app = app();
    let engine_name = app.documents[0].engine().name();
    // The two engines spell the same plot differently; the check adapts to whichever is linked
    // in, which is the point: the SVG under test is the engine's own output.
    let source = if engine_name == "fake" {
        "plot(\"selftest\", 1, 4, 2)"
    } else {
        "plot([1, 4, 2], [4, 5, 6], \"selftest\")"
    };
    app.apply(Command::InsertText(source.to_string()));
    app.run_blocking();
    let plot = match app.documents[0].last_plot.as_ref() {
        Some(plot) => plot,
        None => {
            return Err(format!(
                "the {engine_name} engine produced no plot; logs: {}",
                app.logs_text()
            ));
        }
    };
    let parsed = svg::parse(&plot.svg);
    if parsed.polylines.len() != 1 {
        return Err(format!(
            "parsed {} polylines from the engine SVG",
            parsed.polylines.len()
        ));
    }
    if parsed.polylines[0].points.len() != 3 {
        return Err(format!(
            "parsed {} points, expected 3",
            parsed.polylines[0].points.len()
        ));
    }
    if parsed.texts.is_empty() && parsed.title.is_none() {
        return Err("the engine SVG carried no label".to_string());
    }
    Ok(())
}

fn check_no_network_imports() -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate the running executable: {error}"))?;
    let imports = netcheck::file_imports(&executable)
        .map_err(|error| format!("cannot read {}: {error}", executable.display()))?;
    if imports.is_empty() {
        return Err("the import table was empty, so the check would be vacuous".to_string());
    }
    let has_kernel = imports
        .iter()
        .any(|name| name.eq_ignore_ascii_case("kernel32.dll"));
    if !has_kernel {
        return Err(format!("no kernel32 import found in {imports:?}"));
    }
    let networking = netcheck::network_imports(&imports);
    if !networking.is_empty() {
        return Err(format!("networking DLLs imported: {networking:?}"));
    }
    Ok(())
}
