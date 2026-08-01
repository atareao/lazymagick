use std::collections::HashMap;
use std::path::PathBuf;

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
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create config dir: {e}"))?;
    }
    let content = toml::to_string_pretty(usage)
        .map_err(|e| format!("Cannot serialize usage: {e}"))?;
    std::fs::write(&path, &content)
        .map_err(|e| format!("Cannot write usage: {e}"))?;
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
