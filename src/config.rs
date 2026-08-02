use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::style::Color;

/// Parse a color name string into a [`ratatui::style::Color`].
///
/// Supports named colors (case-insensitive): Black, Red, Green, Yellow, Blue,
/// Magenta, Cyan, White, DarkGray, LightRed, LightGreen, LightYellow, LightBlue,
/// LightMagenta, LightCyan, LightWhite, Gray, Grey, and Reset.
/// Also supports hex colors like "#FF0000" and "#abc".
pub fn parse_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "darkgray" | "dark_gray" | "grey" | "gray" => Color::DarkGray,
        "lightred" | "light_red" => Color::LightRed,
        "lightgreen" | "light_green" => Color::LightGreen,
        "lightyellow" | "light_yellow" => Color::LightYellow,
        "lightblue" | "light_blue" => Color::LightBlue,
        "lightmagenta" | "light_magenta" => Color::LightMagenta,
        "lightcyan" | "light_cyan" => Color::LightCyan,
        "lightwhite" | "light_white" => Color::White,
        "reset" => Color::Reset,
        hex if hex.starts_with('#') => {
            let hex = &hex[1..];
            match hex.len() {
                3 => {
                    let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                    let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                    let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                    Color::Rgb(r, g, b)
                }
                6 => {
                    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                    Color::Rgb(r, g, b)
                }
                _ => Color::DarkGray,
            }
        }
        _ => Color::DarkGray,
    }
}

/// Theme definition — all UI colors as named strings for easy customization.
///
/// Each field is a color name (case-insensitive) or hex code like `"#FF8800"`.
/// Default values match the original hardcoded color scheme.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Theme {
    /// Border color for the focused panel.
    #[serde(default = "default_border_focused")]
    pub border_focused: String,
    /// Border color for unfocused panels.
    #[serde(default = "default_border_unfocused")]
    pub border_unfocused: String,
    /// Cursor foreground (highlighted line).
    #[serde(default = "default_cursor_fg")]
    pub cursor_fg: String,
    /// Cursor background (highlighted line).
    #[serde(default = "default_cursor_bg")]
    pub cursor_bg: String,
    /// Selected item foreground (e.g. active recipe).
    #[serde(default = "default_selected_fg")]
    pub selected_fg: String,
    /// Normal text foreground.
    #[serde(default = "default_text_fg")]
    pub text_fg: String,
    /// Dim / secondary text foreground.
    #[serde(default = "default_dim_text_fg")]
    pub dim_text_fg: String,
    /// Accent color for UI highlights (popup borders, active fields).
    #[serde(default = "default_accent_fg")]
    pub accent_fg: String,
    /// Warning / notice foreground.
    #[serde(default = "default_warning_fg")]
    pub warning_fg: String,
    /// Error foreground.
    #[serde(default = "default_error_fg")]
    pub error_fg: String,
    /// Info log foreground.
    #[serde(default = "default_info_fg")]
    pub info_fg: String,
    /// Success log foreground.
    #[serde(default = "default_success_fg")]
    pub success_fg: String,
    /// Progress bar foreground.
    #[serde(default = "default_progress_fg")]
    pub progress_fg: String,
    /// Directory name foreground.
    #[serde(default = "default_directory_fg")]
    pub directory_fg: String,
    /// Background color for popups.
    #[serde(default = "default_background")]
    pub background: String,
    /// Title / path display foreground.
    #[serde(default = "default_title_fg")]
    pub title_fg: String,
}

fn default_border_focused() -> String {
    "Green".into()
}
fn default_border_unfocused() -> String {
    "DarkGray".into()
}
fn default_cursor_fg() -> String {
    "Cyan".into()
}
fn default_cursor_bg() -> String {
    "DarkGray".into()
}
fn default_selected_fg() -> String {
    "Green".into()
}
fn default_text_fg() -> String {
    "White".into()
}
fn default_dim_text_fg() -> String {
    "DarkGray".into()
}
fn default_accent_fg() -> String {
    "Cyan".into()
}
fn default_warning_fg() -> String {
    "Yellow".into()
}
fn default_error_fg() -> String {
    "Red".into()
}
fn default_info_fg() -> String {
    "Blue".into()
}
fn default_success_fg() -> String {
    "Green".into()
}
fn default_progress_fg() -> String {
    "Cyan".into()
}
fn default_directory_fg() -> String {
    "Blue".into()
}
fn default_background() -> String {
    "Black".into()
}
fn default_title_fg() -> String {
    "Yellow".into()
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            border_focused: default_border_focused(),
            border_unfocused: default_border_unfocused(),
            cursor_fg: default_cursor_fg(),
            cursor_bg: default_cursor_bg(),
            selected_fg: default_selected_fg(),
            text_fg: default_text_fg(),
            dim_text_fg: default_dim_text_fg(),
            accent_fg: default_accent_fg(),
            warning_fg: default_warning_fg(),
            error_fg: default_error_fg(),
            info_fg: default_info_fg(),
            success_fg: default_success_fg(),
            progress_fg: default_progress_fg(),
            directory_fg: default_directory_fg(),
            background: default_background(),
            title_fg: default_title_fg(),
        }
    }
}

/// Parsed theme colors — computed once from [`Theme`] strings.
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub cursor_fg: Color,
    pub cursor_bg: Color,
    pub selected_fg: Color,
    pub text_fg: Color,
    pub dim_text_fg: Color,
    pub accent_fg: Color,
    pub warning_fg: Color,
    pub error_fg: Color,
    pub info_fg: Color,
    pub success_fg: Color,
    pub progress_fg: Color,
    pub directory_fg: Color,
    pub background: Color,
    pub title_fg: Color,
}

impl From<&Theme> for ThemeColors {
    fn from(t: &Theme) -> Self {
        Self {
            border_focused: parse_color(&t.border_focused),
            border_unfocused: parse_color(&t.border_unfocused),
            cursor_fg: parse_color(&t.cursor_fg),
            cursor_bg: parse_color(&t.cursor_bg),
            selected_fg: parse_color(&t.selected_fg),
            text_fg: parse_color(&t.text_fg),
            dim_text_fg: parse_color(&t.dim_text_fg),
            accent_fg: parse_color(&t.accent_fg),
            warning_fg: parse_color(&t.warning_fg),
            error_fg: parse_color(&t.error_fg),
            info_fg: parse_color(&t.info_fg),
            success_fg: parse_color(&t.success_fg),
            progress_fg: parse_color(&t.progress_fg),
            directory_fg: parse_color(&t.directory_fg),
            background: parse_color(&t.background),
            title_fg: parse_color(&t.title_fg),
        }
    }
}

/// Returns the user recipes directory: `~/.config/lazymagick/recipes/`.
///
/// Uses XDG\_CONFIG\_HOME if set (via `dirs::config_dir`), falls back to
/// `~/.config/lazymagick/recipes/`.
pub fn user_recipes_dir() -> PathBuf {
    config_dir().join("recipes")
}

/// Returns the config directory: `~/.config/lazymagick/`.
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
        .join("lazymagick")
}

/// Returns the path to `usage.toml` inside the config directory.
pub fn usage_path() -> PathBuf {
    config_dir().join("usage.toml")
}

/// Load usage counts from `usage.toml`, or return an empty map.
pub fn load_usage() -> HashMap<String, u64> {
    let path = usage_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Save usage counts to `usage.toml`.
pub fn save_usage(usage: &HashMap<String, u64>) -> Result<(), String> {
    let path = usage_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create config dir: {e}"))?;
    }
    let content =
        toml::to_string_pretty(usage).map_err(|e| format!("Cannot serialize usage: {e}"))?;
    std::fs::write(&path, &content).map_err(|e| format!("Cannot write usage: {e}"))?;
    Ok(())
}

/// Returns the path to `settings.toml` inside the config directory.
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}

/// Application settings persisted to `settings.toml`.
///
/// All fields have sensible defaults so the file is optional — if it does not
/// exist or fails to parse, [`Settings::load`] returns `Default`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Settings {
    /// Suffix appended to output filenames when the output format matches the
    /// input format (e.g. `photo_lazymagick.png`).
    #[serde(default = "default_suffix")]
    pub auto_suffix: String,

    /// If `true`, skip the "run this command?" confirmation dialog.
    #[serde(default)]
    pub skip_run_confirm: bool,

    /// If `true`, skip the "overwrite existing file?" confirmation dialog.
    #[serde(default)]
    pub skip_overwrite_confirm: bool,

    /// Directory to start in on launch.
    #[serde(default)]
    pub default_directory: Option<String>,

    /// Color theme configuration.
    #[serde(default)]
    pub theme: Theme,
}

fn default_suffix() -> String {
    "lazymagick".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_suffix: default_suffix(),
            skip_run_confirm: false,
            skip_overwrite_confirm: false,
            default_directory: None,
            theme: Theme::default(),
        }
    }
}

impl Settings {
    /// Load settings from `settings.toml`, or return defaults if the file
    /// doesn't exist or is malformed.
    pub fn load() -> Self {
        let path = settings_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    /// Save settings to `settings.toml`.
    ///
    /// Creates the config directory if it doesn't exist.
    pub fn save(&self) -> Result<(), String> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create config dir: {e}"))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Cannot serialize settings: {e}"))?;
        std::fs::write(&path, &content).map_err(|e| format!("Cannot write settings: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert_eq!(s.auto_suffix, "lazymagick");
        assert!(!s.skip_run_confirm);
        assert!(!s.skip_overwrite_confirm);
    }

    #[test]
    fn load_returns_defaults_when_no_file() {
        // Ensure we don't accidentally pick up a real config file.
        let saved = settings_path();
        let exists = saved.exists();
        let s = Settings::load();
        assert_eq!(s.auto_suffix, "lazymagick");
        // Clean up only if we created it (we shouldn't have, but be safe).
        if !exists && saved.exists() {
            let _ = std::fs::remove_file(&saved);
        }
    }

    #[test]
    fn round_trip_through_temp_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let settings_path = tmp.path().join("settings.toml");

        let s = Settings {
            auto_suffix: "test".into(),
            skip_run_confirm: true,
            skip_overwrite_confirm: false,
            default_directory: Some("/tmp".into()),
            theme: Theme::default(),
        };

        let toml_str = toml::to_string_pretty(&s).unwrap();
        std::fs::write(&settings_path, &toml_str).unwrap();

        let loaded: Settings =
            toml::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(loaded.auto_suffix, "test");
        assert!(loaded.skip_run_confirm);
        assert!(!loaded.skip_overwrite_confirm);
        assert_eq!(loaded.default_directory, Some("/tmp".into()));
    }

    #[test]
    fn usage_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let usage_path = tmp.path().join("usage.toml");

        let mut usage: HashMap<String, u64> = HashMap::new();
        usage.insert("webp 85".into(), 5u64);
        usage.insert("strip".into(), 3u64);

        let content = toml::to_string_pretty(&usage).unwrap();
        std::fs::write(&usage_path, &content).unwrap();

        let loaded: HashMap<String, u64> =
            toml::from_str(&std::fs::read_to_string(&usage_path).unwrap()).unwrap();
        assert_eq!(loaded.get("webp 85"), Some(&5));
        assert_eq!(loaded.get("strip"), Some(&3));
        assert_eq!(loaded.len(), 2);
    }
}
