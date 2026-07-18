//! Shared user configuration for the TUI and (read-only) the web server.
//!
//! Persisted as YAML at `$XDG_CONFIG_HOME/local-code-review/config.yaml`
//! (falling back to `~/.config/local-code-review/config.yaml`). This is the
//! Rust-side source of truth for cross-cutting preferences — theme, icon glyph
//! set, and diff context width — and is intentionally a superset of the web's
//! localStorage `UserPreferences` (it adds `iconMode` and `diffContextLines`,
//! and deliberately omits `fontSize`, which is meaningless in a terminal).
//!
//! Per-repo review state (comments, reviewed files) stays in the existing
//! `.local-review/` session files — this config holds only global preferences.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Which glyph vocabulary the TUI renders icons with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IconMode {
    /// Nerd Font glyphs (default) — requires a patched terminal font.
    #[serde(rename = "nerdfont")]
    NerdFont,
    /// Widely-supported Unicode symbols.
    Unicode,
    /// Plain ASCII — works in any terminal.
    Ascii,
}

impl Default for IconMode {
    fn default() -> Self {
        IconMode::NerdFont
    }
}

pub const DEFAULT_THEME: &str = "default-dark";
pub const DEFAULT_CONTEXT_LINES: u32 = 3;
pub const MAX_CONTEXT_LINES: u32 = 20;

fn default_theme() -> String {
    DEFAULT_THEME.to_string()
}

fn default_context_lines() -> u32 {
    DEFAULT_CONTEXT_LINES
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Theme id (see `tui::theme`); unknown ids fall back to `default-dark`.
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub icon_mode: IconMode,
    /// Unified diff context lines (git `-U<n>`), clamped to `0..=MAX_CONTEXT_LINES`.
    #[serde(default = "default_context_lines")]
    pub diff_context_lines: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: default_theme(),
            icon_mode: IconMode::default(),
            diff_context_lines: DEFAULT_CONTEXT_LINES,
        }
    }
}

/// `$XDG_CONFIG_HOME/local-code-review/config.yaml` (or `~/.config/...`).
/// `None` when no config directory can be resolved (rare; headless envs).
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("local-code-review").join("config.yaml"))
}

impl Config {
    /// Load preferences, never failing: a missing file or unparseable YAML
    /// yields defaults (with a warning), so a broken config can't brick the app.
    /// Always normalized before return (clamped context; theme left as-is for
    /// the theme registry to resolve, matching the web's `normalizeThemeId`).
    pub fn load() -> Config {
        let Some(path) = config_path() else {
            return Config::default();
        };
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => return Config::default(), // missing file is normal
        };
        match serde_yaml::from_str::<Config>(&raw) {
            Ok(mut cfg) => {
                cfg.normalize();
                cfg
            }
            Err(e) => {
                tracing::warn!("Ignoring malformed config at {}: {}", path.display(), e);
                Config::default()
            }
        }
    }

    /// Write preferences to disk, creating the parent directory as needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path()
            .ok_or_else(|| anyhow::anyhow!("could not resolve a config directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(&path, yaml)?;
        Ok(())
    }

    /// Clamp out-of-range values. Theme validation is deferred to the theme
    /// registry (which owns the id list and its own fallback).
    fn normalize(&mut self) {
        if self.diff_context_lines > MAX_CONTEXT_LINES {
            self.diff_context_lines = MAX_CONTEXT_LINES;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // config_path() reads the process environment; tests that override
    // XDG_CONFIG_HOME must not race each other.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_config_home<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let original = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(tmp.path())));
        match original {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match result {
            Ok(v) => v,
            Err(p) => std::panic::resume_unwind(p),
        }
    }

    #[test]
    fn default_values() {
        let c = Config::default();
        assert_eq!(c.theme, "default-dark");
        assert_eq!(c.icon_mode, IconMode::NerdFont);
        assert_eq!(c.diff_context_lines, 3);
    }

    #[test]
    fn partial_yaml_fills_defaults() {
        // Only theme specified — icon_mode and context should default.
        let cfg: Config = serde_yaml::from_str("theme: catppuccin-mocha\n").unwrap();
        assert_eq!(cfg.theme, "catppuccin-mocha");
        assert_eq!(cfg.icon_mode, IconMode::NerdFont);
        assert_eq!(cfg.diff_context_lines, 3);
    }

    #[test]
    fn empty_yaml_is_all_defaults() {
        let cfg: Config = serde_yaml::from_str("{}\n").unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn icon_mode_serializes_lowercase_nerdfont() {
        let yaml = serde_yaml::to_string(&Config::default()).unwrap();
        assert!(yaml.contains("iconMode: nerdfont"), "got: {}", yaml);
    }

    #[test]
    fn context_is_clamped_on_load() {
        with_config_home(|dir| {
            let path = dir.join("local-code-review").join("config.yaml");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "diffContextLines: 999\n").unwrap();
            assert_eq!(Config::load().diff_context_lines, MAX_CONTEXT_LINES);
        });
    }

    #[test]
    fn load_missing_file_returns_default() {
        with_config_home(|_| {
            assert_eq!(Config::load(), Config::default());
        });
    }

    #[test]
    fn load_malformed_yaml_returns_default() {
        with_config_home(|dir| {
            let path = dir.join("local-code-review").join("config.yaml");
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "this: [is: not: valid").unwrap();
            assert_eq!(Config::load(), Config::default());
        });
    }

    #[test]
    fn save_then_load_round_trips() {
        with_config_home(|_| {
            let cfg = Config {
                theme: "catppuccin-latte".into(),
                icon_mode: IconMode::Ascii,
                diff_context_lines: 7,
            };
            cfg.save().unwrap();
            assert_eq!(Config::load(), cfg);
        });
    }
}
