//! The persistent settings document.
//!
//! Persistence is deliberately boring: one human-readable JSON object, a schema number, and a
//! loader that treats every problem as "use the defaults and record a warning". A corrupt file,
//! a file written by a newer build, or a field of the wrong type must never be able to stop the
//! application from starting, so from_json has no error return at all.

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

    /// Takes the load warnings, leaving the list empty.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
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

/// The per-user settings path: %APPDATA%\Lovelace\desktop.json on Windows, and the usual
/// XDG location elsewhere.
pub fn default_config_path() -> Option<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata).join("Lovelace").join("desktop.json"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("lovelace").join("desktop.json"));
    }
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("lovelace")
            .join("desktop.json"),
    )
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

    #[test]
    fn missing_file_is_not_a_warning() {
        let path = std::env::temp_dir().join("lovelace-does-not-exist-9c1f/desktop.json");
        let loaded = Settings::load(&path);
        assert_eq!(loaded, Settings::new());
        assert!(loaded.warnings.is_empty());
    }
}
