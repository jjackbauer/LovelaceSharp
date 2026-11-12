//! The persistent settings document.
//!
//! Persistence is deliberately boring: one human-readable JSON object, a schema number, and a
//! loader that treats every problem as "use the defaults and record a warning". A corrupt file,
//! a file written by a newer build, or a field of the wrong type must never be able to stop the
//! application from starting, so from_json has no error return at all.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use lovelace_json::Json;

use crate::engine::DEFAULT_PRECISION;

/// The schema version this build writes and understands.
pub const SCHEMA_VERSION: u32 = 1;

/// Smallest window width the settings loader will accept, in pixels.
pub const MIN_WINDOW_WIDTH: i32 = 900;
/// Smallest window height the settings loader will accept, in pixels.
pub const MIN_WINDOW_HEIGHT: i32 = 600;
/// Largest window dimension the settings loader will accept, in pixels.
pub const MAX_WINDOW_DIMENSION: i32 = 10_000;
/// Largest number of recent files kept.
pub const MAX_RECENT_FILES: usize = 20;
/// Smallest accepted display precision, in decimal places.
pub const MIN_PRECISION: usize = 1;
/// Largest accepted display precision, in decimal places.
pub const MAX_PRECISION: usize = 100_000;

/// Saved window geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowGeometry {
    /// Outer window width, in pixels.
    pub width: i32,
    /// Outer window height, in pixels.
    pub height: i32,
}

impl Default for WindowGeometry {
    fn default() -> WindowGeometry {
        WindowGeometry {
            width: 1280,
            height: 800,
        }
    }
}

/// The layout fractions that the shell turns into pane sizes on every resize.
///
/// They are stored in permille (thousandths) rather than as floats so that the saved document,
/// the layout arithmetic and the tests are all exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutFractions {
    /// Editor column width, as a fraction of the client width.
    pub editor_width_permille: u32,
    /// Logs strip height, as a fraction of the content height.
    pub logs_height_permille: u32,
    /// Plot pane height, as a fraction of the editor column height.
    pub plot_height_permille: u32,
}

impl Default for LayoutFractions {
    fn default() -> LayoutFractions {
        LayoutFractions {
            editor_width_permille: 720,
            logs_height_permille: 220,
            plot_height_permille: 460,
        }
    }
}

/// One tab of the last session: the file it showed, or just its identity for an unsaved buffer.
///
/// DOC-004: a tab with a path is restored by reading that file again; a tab without one can only
/// be restored when DOC-005's recovery journal still holds its buffer, which is why the identity
/// is persisted beside the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionTab {
    /// The file the tab showed, when it had one.
    pub path: Option<String>,
    /// The document identity the tab had, so a recovery record can be matched to it.
    pub document_id: u64,
}

/// The tabs that were open at the last persist, in tab order, and which one was active.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionState {
    /// The open tabs, left to right.
    pub documents: Vec<SessionTab>,
    /// The index of the tab that was selected.
    pub active: usize,
}

impl SessionState {
    /// Builds a session value from `(path, document identity)` pairs and the active index.
    pub fn new(documents: Vec<SessionTab>, active: usize) -> SessionState {
        let active = if documents.is_empty() {
            0
        } else {
            active.min(documents.len() - 1)
        };
        SessionState { documents, active }
    }
}

/// Everything the application persists between runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Schema number of the file this value was loaded from (or wrote).
    pub schema_version: u32,
    /// Saved window geometry.
    pub window: WindowGeometry,
    /// Saved layout fractions.
    pub layout: LayoutFractions,
    /// Most-recently used file paths, newest first.
    pub recent_files: Vec<String>,
    /// Display precision of the document that was active at exit.
    pub active_precision: usize,
    /// DOC-004: the tabs that were open at the last persist, or `None` when nothing was saved.
    pub session: Option<SessionState>,
    /// Problems found while loading. Never serialised; the shell copies them into the log.
    pub warnings: Vec<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            schema_version: SCHEMA_VERSION,
            window: WindowGeometry::default(),
            layout: LayoutFractions::default(),
            recent_files: Vec::new(),
            active_precision: DEFAULT_PRECISION,
            session: None,
            warnings: Vec::new(),
        }
    }
}

impl Settings {
    /// A settings value with every field at its default.
    pub fn new() -> Settings {
        Settings::default()
    }

    /// Serialises the settings as pretty-printed JSON. Warnings are not persisted: they
    /// describe this load, not the document.
    pub fn to_json(&self) -> String {
        Json::obj(vec![
            ("schema_version", Json::Num(SCHEMA_VERSION.to_string())),
            (
                "window",
                Json::obj(vec![
                    ("width", Json::Num(self.window.width.to_string())),
                    ("height", Json::Num(self.window.height.to_string())),
                ]),
            ),
            (
                "layout",
                Json::obj(vec![
                    (
                        "editor_width_permille",
                        Json::Num(self.layout.editor_width_permille.to_string()),
                    ),
                    (
                        "logs_height_permille",
                        Json::Num(self.layout.logs_height_permille.to_string()),
                    ),
                    (
                        "plot_height_permille",
                        Json::Num(self.layout.plot_height_permille.to_string()),
                    ),
                ]),
            ),
            (
                "recent_files",
                Json::str_array(self.recent_files.iter().cloned()),
            ),
            (
                "active_precision",
                Json::Num(self.active_precision.to_string()),
            ),
            (
                "session",
                match &self.session {
                    Some(session) => Json::obj(vec![
                        ("active", Json::Num(session.active.to_string())),
                        (
                            "documents",
                            Json::Arr(
                                session
                                    .documents
                                    .iter()
                                    .map(|tab| {
                                        let mut fields = vec![(
                                            "document_id",
                                            Json::Num(tab.document_id.to_string()),
                                        )];
                                        if let Some(path) = &tab.path {
                                            fields.push(("path", Json::Str(path.clone())));
                                        }
                                        Json::obj(fields)
                                    })
                                    .collect(),
                            ),
                        ),
                    ]),
                    None => Json::Null,
                },
            ),
        ])
        .to_pretty()
    }

    /// Parses a settings document. Any parse failure, wrong schema, or unusable field falls
    /// back to a default and appends a warning; this function never panics and never fails.
    pub fn from_json(text: &str) -> Settings {
        let mut settings = Settings::new();
        let root = match Json::parse(text) {
            Ok(Json::Obj(map)) => Json::Obj(map),
            Ok(_) => {
                settings
                    .warnings
                    .push("settings root is not a JSON object; defaults used.".to_string());
                return settings;
            }
            Err(error) => {
                settings.warnings.push(format!(
                    "settings are not valid JSON ({error}); defaults used."
                ));
                return settings;
            }
        };

        let version = match root.get("schema_version") {
            Some(Json::Num(n)) => n.parse::<u32>().ok(),
            _ => None,
        };
        let version = match version {
            Some(v) => v,
            None => {
                settings.warnings.push(
                    "settings schema_version is missing or not a number; defaults used."
                        .to_string(),
                );
                return settings;
            }
        };
        if version > SCHEMA_VERSION {
            settings.warnings.push(format!(
                "settings schema_version {version} is newer than supported {SCHEMA_VERSION}; defaults used."
            ));
            return settings;
        }
        if version < SCHEMA_VERSION {
            settings.warnings.push(format!(
                "settings schema_version {version} is older than supported {SCHEMA_VERSION}; missing fields use defaults."
            ));
        }

        let mut warnings: Vec<String> = Vec::new();
        if let Some(window) = root.get("window") {
            if let Json::Obj(_) = window {
                settings.window.width = read_i64(
                    window,
                    "width",
                    settings.window.width as i64,
                    MIN_WINDOW_WIDTH as i64,
                    MAX_WINDOW_DIMENSION as i64,
                    &mut warnings,
                ) as i32;
                settings.window.height = read_i64(
                    window,
                    "height",
                    settings.window.height as i64,
                    MIN_WINDOW_HEIGHT as i64,
                    MAX_WINDOW_DIMENSION as i64,
                    &mut warnings,
                ) as i32;
            } else {
                warnings.push("window is not an object; default geometry used.".to_string());
            }
        }
        if let Some(layout) = root.get("layout") {
            if let Json::Obj(_) = layout {
                settings.layout.editor_width_permille = read_i64(
                    layout,
                    "editor_width_permille",
                    settings.layout.editor_width_permille as i64,
                    300,
                    900,
                    &mut warnings,
                ) as u32;
                settings.layout.logs_height_permille = read_i64(
                    layout,
                    "logs_height_permille",
                    settings.layout.logs_height_permille as i64,
                    100,
                    600,
                    &mut warnings,
                ) as u32;
                settings.layout.plot_height_permille = read_i64(
                    layout,
                    "plot_height_permille",
                    settings.layout.plot_height_permille as i64,
                    150,
                    850,
                    &mut warnings,
                ) as u32;
            } else {
                warnings.push("layout is not an object; default fractions used.".to_string());
            }
        }
        settings.active_precision = read_i64(
            &root,
            "active_precision",
            settings.active_precision as i64,
            MIN_PRECISION as i64,
            MAX_PRECISION as i64,
            &mut warnings,
        ) as usize;
        if let Some(recent) = root.get("recent_files") {
            match recent.as_arr() {
                Some(items) => {
                    // Preserve document order (newest first); add_recent would reverse it.
                    let mut loaded: Vec<String> = Vec::new();
                    for item in items {
                        if let Some(path) = item.as_str() {
                            let path = path.trim();
                            if !path.is_empty() && !loaded.iter().any(|existing| existing == path) {
                                loaded.push(path.to_string());
                            }
                        }
                    }
                    loaded.truncate(MAX_RECENT_FILES);
                    settings.recent_files = loaded;
                }
                None => warnings.push("recent_files is not an array; ignored.".to_string()),
            }
        }
        if let Some(session) = root.get("session") {
            // `null` is how a settings file with no session is written; it is not a problem.
            // A malformed session never fails the load either: read_session records the warning.
            if !matches!(session, Json::Null) {
                if let Some(parsed) = read_session(session, &mut warnings) {
                    settings.session = Some(parsed);
                }
            }
        }
        settings.warnings.extend(warnings);
        settings
    }

    /// Reads a settings file, returning defaults when it is missing or unreadable.
    pub fn load(path: &Path) -> Settings {
        match std::fs::read_to_string(path) {
            Ok(text) => Settings::from_json(&text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::new(),
            Err(error) => {
                let mut settings = Settings::new();
                settings.warnings.push(format!(
                    "cannot read settings at {}: {error}; defaults used.",
                    path.display()
                ));
                settings
            }
        }
    }

    /// Writes the settings file, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut text = self.to_json();
        text.push('\n');
        std::fs::write(path, text)
    }

    /// Records a recently used path, newest first, without duplicates.
    pub fn add_recent(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            return;
        }
        self.recent_files.retain(|existing| existing != path);
        self.recent_files.insert(0, path.to_string());
        self.recent_files.truncate(MAX_RECENT_FILES);
    }

    /// DOC-004: drops every recent path that no longer exists on disk, returning how many went.
    ///
    /// This runs when the settings are loaded into the running application, so the File bar's
    /// Recent menu never offers a file that was deleted between two launches. A path that exists
    /// but cannot be read is kept: the Open path reports that failure, which is more useful than
    /// silently forgetting the file.
    pub fn prune_missing_recent(&mut self) -> usize {
        let before = self.recent_files.len();
        self.recent_files
            .retain(|path| Path::new(path).exists());
        before - self.recent_files.len()
    }

    /// Replaces the persisted session with `session`.
    pub fn set_session(&mut self, session: SessionState) {
        self.session = Some(session);
    }

    /// Takes the load warnings, leaving the list empty.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

/// Reads a persisted session (DOC-004).
///
/// A malformed session never fails the load: it is ignored with a warning and the next launch
/// simply opens the starter documents. Entries without a usable document identity are skipped,
/// because an unsaved buffer could not be matched to its recovery record without one.
fn read_session(session: &Json, warnings: &mut Vec<String>) -> Option<SessionState> {
    let mut documents = Vec::new();
    match session.get("documents").and_then(Json::as_arr) {
        Some(items) => {
            for (index, item) in items.iter().enumerate() {
                let Some(document_id) = item
                    .get("document_id")
                    .and_then(|value| match value {
                        Json::Num(text) => text.parse::<u64>().ok(),
                        _ => None,
                    })
                else {
                    warnings.push(format!(
                        "session document {index} has no document_id; it was skipped."
                    ));
                    continue;
                };
                let path = match item.get("path") {
                    None | Some(Json::Null) => None,
                    Some(Json::Str(path)) => {
                        let path = path.trim();
                        if path.is_empty() {
                            warnings.push(format!(
                                "session document {index} has an empty path; it was skipped."
                            ));
                            continue;
                        }
                        Some(path.to_string())
                    }
                    Some(_) => {
                        warnings.push(format!(
                            "session document {index} has a path that is not a string; it was skipped."
                        ));
                        continue;
                    }
                };
                documents.push(SessionTab { path, document_id });
            }
        }
        None => {
            warnings.push("session.documents is not an array; the session was ignored.".to_string());
            return None;
        }
    }
    if documents.is_empty() {
        return None;
    }
    let active = match session.get("active") {
        Some(Json::Num(text)) => text.parse::<usize>().unwrap_or(0),
        None => 0,
        Some(_) => {
            warnings.push("session.active is not a number; using tab 1.".to_string());
            0
        }
    };
    Some(SessionState::new(documents, active))
}

/// Reads an integer field, clamping it into range and recording a warning when it is missing,
/// not a number, or out of range.
fn read_i64(
    container: &Json,
    name: &str,
    default: i64,
    lo: i64,
    hi: i64,
    warnings: &mut Vec<String>,
) -> i64 {
    match container.get(name) {
        Some(Json::Num(text)) => match text.parse::<i64>() {
            Ok(value) if (lo..=hi).contains(&value) => value,
            Ok(value) => {
                let clamped = value.clamp(lo, hi);
                warnings.push(format!(
                    "{name}={value} is outside {lo}..={hi}; using {clamped}."
                ));
                clamped
            }
            Err(_) => {
                warnings.push(format!("{name} is not an integer; using {default}."));
                default
            }
        },
        Some(_) => {
            warnings.push(format!("{name} is not a number; using {default}."));
            default
        }
        None => {
            warnings.push(format!("{name} is missing; using {default}."));
            default
        }
    }
}

/// The per-user application directory: %APPDATA%\Lovelace on Windows, and the usual XDG
/// location elsewhere. Every file the application writes outside a document lives below it,
/// so the settings file and the plot directory are always siblings resolved the same way.
fn app_dir_from(
    appdata: Option<&OsStr>,
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(appdata) = appdata {
        return Some(PathBuf::from(appdata).join("Lovelace"));
    }
    if let Some(xdg) = xdg_config_home {
        return Some(PathBuf::from(xdg).join("lovelace"));
    }
    let home = home?;
    Some(PathBuf::from(home).join(".config").join("lovelace"))
}

/// The per-user application directory for this process.
fn app_dir() -> Option<PathBuf> {
    app_dir_from(
        std::env::var_os("APPDATA").as_deref(),
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// The per-user settings path: %APPDATA%\Lovelace\desktop.json on Windows, and the usual
/// XDG location elsewhere.
pub fn default_config_path() -> Option<PathBuf> {
    app_dir().map(|directory| directory.join("desktop.json"))
}

/// The per-user plot directory: %APPDATA%\Lovelace\plots on Windows, and the usual XDG
/// location elsewhere. It is derived from the same application directory as
/// [`default_config_path`], so the product never writes beside the executable or in the
/// process working directory (PLF-006).
pub fn default_plot_dir() -> Option<PathBuf> {
    app_dir().map(|directory| directory.join("plots"))
}

/// DOC-005: the per-user recovery directory, `%APPDATA%\Lovelace\recovery` on Windows.
///
/// It is the sibling of the settings file, so every file the application writes outside a
/// document lives under the one application-scoped directory (PLF-006). One record file per
/// journaled document lives here and nowhere else.
pub fn default_recovery_dir() -> Option<PathBuf> {
    app_dir().map(|directory| directory.join("recovery"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_json() {
        let mut settings = Settings::new();
        settings.window = WindowGeometry {
            width: 1440,
            height: 900,
        };
        settings.layout.editor_width_permille = 640;
        settings.layout.logs_height_permille = 250;
        settings.layout.plot_height_permille = 400;
        settings.active_precision = 2500;
        settings.add_recent("C:/scripts/a.ls");
        settings.add_recent("C:/scripts/b.ls");

        let text = settings.to_json();
        let again = Settings::from_json(&text);
        assert_eq!(again, settings);
        assert!(
            again.warnings.is_empty(),
            "round trip must be warning-free: {:?}",
            again.warnings
        );
    }

    #[test]
    fn corrupt_json_falls_back_to_defaults_with_a_warning() {
        let settings = Settings::from_json("{ this is not json");
        let defaults = Settings::new();
        assert_eq!(settings.window, defaults.window);
        assert_eq!(settings.layout, defaults.layout);
        assert_eq!(settings.active_precision, defaults.active_precision);
        assert!(settings.recent_files.is_empty());
        assert_eq!(settings.warnings.len(), 1);
        assert!(settings.warnings[0].contains("not valid JSON"));
    }

    #[test]
    fn non_object_root_falls_back_to_defaults() {
        let settings = Settings::from_json("[1, 2, 3]");
        assert_eq!(settings.window, WindowGeometry::default());
        assert!(settings.warnings[0].contains("not a JSON object"));
    }

    #[test]
    fn newer_schema_refuses_to_guess_and_uses_defaults() {
        let text = r#"{"schema_version": 99, "window": {"width": 100, "height": 100}}"#;
        let settings = Settings::from_json(text);
        assert_eq!(settings.window, WindowGeometry::default());
        assert!(
            settings.warnings[0].contains("newer than supported"),
            "unexpected warning: {:?}",
            settings.warnings
        );
    }

    #[test]
    fn out_of_range_and_wrong_typed_fields_are_clamped_with_warnings() {
        let text = r#"{
            "schema_version": 1,
            "window": {"width": 10, "height": "tall"},
            "layout": {"editor_width_permille": 9999, "logs_height_permille": 0, "plot_height_permille": 460},
            "recent_files": ["a.ls", "b.ls", "a.ls"],
            "active_precision": -5
        }"#;
        let settings = Settings::from_json(text);
        assert_eq!(settings.window.width, MIN_WINDOW_WIDTH);
        assert_eq!(settings.window.height, WindowGeometry::default().height);
        assert_eq!(settings.layout.editor_width_permille, 900);
        assert_eq!(settings.layout.logs_height_permille, 100);
        assert_eq!(settings.active_precision, MIN_PRECISION);
        assert_eq!(
            settings.recent_files,
            vec!["a.ls".to_string(), "b.ls".to_string()]
        );
        assert!(
            settings.warnings.len() >= 4,
            "warnings: {:?}",
            settings.warnings
        );
    }

    #[test]
    fn missing_schema_version_is_corruption() {
        let settings = Settings::from_json(r#"{"window": {"width": 1000, "height": 700}}"#);
        assert_eq!(settings.window, WindowGeometry::default());
        assert!(settings.warnings[0].contains("schema_version"));
    }

    #[test]
    fn recent_files_deduplicate_and_cap() {
        let mut settings = Settings::new();
        for index in 0..(MAX_RECENT_FILES + 5) {
            settings.add_recent(&format!("file-{index}.ls"));
        }
        assert_eq!(settings.recent_files.len(), MAX_RECENT_FILES);
        assert_eq!(
            settings.recent_files[0],
            format!("file-{}.ls", MAX_RECENT_FILES + 4)
        );
        settings.add_recent("file-29.ls");
        assert_eq!(settings.recent_files.len(), MAX_RECENT_FILES);
        assert_eq!(
            settings
                .recent_files
                .iter()
                .filter(|path| path.as_str() == "file-29.ls")
                .count(),
            1
        );
    }

    #[test]
    fn app_dir_prefers_appdata_then_xdg_then_home() {
        let appdata = OsStr::new(r"C:\Users\example\AppData\Roaming");
        let xdg = OsStr::new("/home/example/.config");
        let home = OsStr::new("/home/example");

        let from_appdata =
            app_dir_from(Some(appdata), Some(xdg), Some(home)).expect("appdata wins");
        assert_eq!(
            from_appdata,
            PathBuf::from(appdata).join("Lovelace"),
            "the Windows application directory is %APPDATA%\\Lovelace"
        );
        assert_eq!(
            from_appdata.join("desktop.json"),
            PathBuf::from(appdata).join("Lovelace").join("desktop.json")
        );

        let from_xdg = app_dir_from(None, Some(xdg), Some(home)).expect("xdg second");
        assert_eq!(from_xdg, PathBuf::from(xdg).join("lovelace"));

        let from_home = app_dir_from(None, None, Some(home)).expect("home last");
        assert_eq!(
            from_home,
            PathBuf::from(home).join(".config").join("lovelace")
        );

        assert!(app_dir_from(None, None, None).is_none());
    }

    #[test]
    fn plot_dir_is_derived_from_the_settings_application_directory() {
        let appdata = OsStr::new(r"C:\Users\example\AppData\Roaming");
        let directory = app_dir_from(Some(appdata), None, None).expect("appdata");
        assert_eq!(
            directory.join("plots"),
            directory
                .join("desktop.json")
                .parent()
                .unwrap()
                .join("plots"),
            "the plot directory is the sibling of desktop.json, not the working directory"
        );
        assert!(directory.join("plots").ends_with("plots"));
        assert!(directory.join("plots").starts_with(PathBuf::from(appdata)));
    }

    #[test]
    fn default_plot_dir_matches_default_config_path_parent() {
        let Some(config) = default_config_path() else {
            // No APPDATA/XDG/HOME in this environment: the derivation itself is covered above.
            return;
        };
        let expected = config.parent().expect("config has a parent").join("plots");
        assert_eq!(default_plot_dir(), Some(expected));
    }

    #[test]
    fn save_and_load_round_trip_on_disk() {
        let dir = std::env::temp_dir().join(format!("lovelace-settings-{}", std::process::id()));
        let path = dir.join("desktop.json");
        let mut settings = Settings::new();
        settings.active_precision = 50;
        settings.add_recent("C:/x/y.ls");
        settings.save(&path).expect("save settings");
        let loaded = Settings::load(&path);
        assert_eq!(loaded, settings);
        assert!(loaded.warnings.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// DOC-004: a session saves and loads with its tab order and active tab intact.
    #[test]
    fn session_tab_order_and_active_tab_round_trip_through_disk() {
        let directory =
            std::env::temp_dir().join(format!("lovelace-session-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("scratch directory");
        let settings_path = directory.join("desktop.json");
        let first = directory.join("first.lv");
        let second = directory.join("second.lv");
        let third = directory.join("third.lv");
        for path in [&first, &second, &third] {
            std::fs::write(path, "a = 1\n").expect("fixture");
        }

        let mut settings = Settings::new();
        settings.add_recent(&second.to_string_lossy());
        settings.add_recent(&first.to_string_lossy());
        settings.set_session(SessionState::new(
            vec![
                SessionTab {
                    path: Some(first.to_string_lossy().into_owned()),
                    document_id: 7,
                },
                SessionTab {
                    path: None,
                    document_id: 8,
                },
                SessionTab {
                    path: Some(third.to_string_lossy().into_owned()),
                    document_id: 9,
                },
            ],
            1,
        ));
        settings.save(&settings_path).expect("save settings");

        let loaded = Settings::load(&settings_path);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
        let session = loaded.session.as_ref().expect("session survives a save/load");
        assert_eq!(session.active, 1, "the active tab is persisted");
        assert_eq!(session.documents.len(), 3, "tab order is persisted");
        assert_eq!(session.documents[0].document_id, 7);
        assert_eq!(session.documents[2].document_id, 9);
        assert_eq!(
            session.documents[2].path.as_deref(),
            Some(third.to_string_lossy().as_ref())
        );
        assert_eq!(session.documents[1].path, None, "a pathless tab stays pathless");
        assert_eq!(session.documents[1].document_id, 8);
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// DOC-004: reopening a file moves it to the top of the recent list and never duplicates it.
    #[test]
    fn recent_files_move_a_reopened_file_to_the_top_without_duplicating_it() {
        let mut settings = Settings::new();
        settings.add_recent("C:/scripts/a.lv");
        settings.add_recent("C:/scripts/b.lv");
        settings.add_recent("C:/scripts/c.lv");
        assert_eq!(
            settings.recent_files,
            vec![
                "C:/scripts/c.lv".to_string(),
                "C:/scripts/b.lv".to_string(),
                "C:/scripts/a.lv".to_string()
            ]
        );
        settings.add_recent("C:/scripts/a.lv");
        assert_eq!(settings.recent_files[0], "C:/scripts/a.lv");
        assert_eq!(
            settings
                .recent_files
                .iter()
                .filter(|path| path.as_str() == "C:/scripts/a.lv")
                .count(),
            1,
            "a re-opened file must appear exactly once"
        );
        assert_eq!(settings.recent_files.len(), 3);
    }

    /// DOC-004: a recent path that no longer exists is pruned when the settings are loaded.
    #[test]
    fn recent_paths_that_no_longer_exist_are_pruned_on_load() {
        let directory =
            std::env::temp_dir().join(format!("lovelace-recent-prune-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("scratch directory");
        let kept = directory.join("kept.lv");
        let gone = directory.join("gone.lv");
        std::fs::write(&kept, "a = 1\n").expect("fixture");
        let _ = std::fs::remove_file(&gone);

        let mut settings = Settings::new();
        settings.recent_files = vec![
            gone.to_string_lossy().into_owned(),
            kept.to_string_lossy().into_owned(),
        ];
        let pruned = settings.prune_missing_recent();
        assert_eq!(pruned, 1);
        assert_eq!(
            settings.recent_files,
            vec![kept.to_string_lossy().into_owned()],
            "the existing file is kept in its position"
        );
        // A second prune finds nothing to do.
        assert_eq!(settings.prune_missing_recent(), 0);
        let _ = std::fs::remove_dir_all(&directory);
    }

    /// DOC-004: a malformed session falls back to no session with a warning, never a crash.
    #[test]
    fn a_malformed_session_is_ignored_with_a_warning() {
        let text = r#"{
            "schema_version": 1,
            "session": { "documents": "not an array" }
        }"#;
        let settings = Settings::from_json(text);
        assert!(settings.session.is_none());
        assert!(
            settings.warnings.iter().any(|warning| warning.contains("session.documents")),
            "{:?}",
            settings.warnings
        );

        let text = r#"{
            "schema_version": 1,
            "session": { "active": 9, "documents": [
                { "document_id": 4 },
                { "path": "C:/x.lv" }
            ] }
        }"#;
        let settings = Settings::from_json(text);
        let session = settings.session.expect("one valid entry remains");
        assert_eq!(session.documents.len(), 1, "the entry without an id is skipped");
        assert_eq!(session.active, 0, "the active index is clamped to the tabs");
    }

    /// DOC-005: the recovery directory is the sibling of the settings file.
    #[test]
    fn recovery_dir_is_the_sibling_of_the_settings_file() {
        assert_eq!(
            default_recovery_dir(),
            default_config_path().map(|path| path.parent().unwrap().join("recovery"))
        );
    }

    #[test]
    fn missing_file_is_not_a_warning() {
        let path = std::env::temp_dir().join("lovelace-does-not-exist-9c1f/desktop.json");
        let loaded = Settings::load(&path);
        assert_eq!(loaded, Settings::new());
        assert!(loaded.warnings.is_empty());
    }
}
