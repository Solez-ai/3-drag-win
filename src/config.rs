use anyhow::{Context, Result, anyhow};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    data_dir: PathBuf,
    config_path: PathBuf,
    log_dir: PathBuf,
    log_path: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "solez-ai", "3-win-drag")
            .ok_or_else(|| anyhow!("unable to resolve an application data directory"))?;
        let data_dir = dirs.data_local_dir().to_path_buf();
        let log_dir = data_dir.join("logs");
        let config_path = data_dir.join("config.json");
        let log_path = log_dir.join("3-win-drag.log");

        Ok(Self {
            data_dir,
            config_path,
            log_dir,
            log_path,
        })
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)
            .with_context(|| format!("failed to create {}", self.data_dir.display()))?;
        fs::create_dir_all(&self.log_dir)
            .with_context(|| format!("failed to create {}", self.log_dir.display()))?;
        Ok(())
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub enabled: bool,
    pub launch_at_startup: bool,
    pub touchpad_profile: String,
    pub gesture_action: GestureAction,
    pub gesture_finger_count: u8,
    pub touchpad_sensitivity: f32,
    pub deadzone_pixels: i32,
    pub minimum_update_interval_ms: u64,
    pub smoothing_factor: f32,
    pub ignore_fullscreen_windows: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            launch_at_startup: true,
            touchpad_profile: String::from("drag_drop_precise"),
            gesture_action: GestureAction::MouseDrag,
            gesture_finger_count: 3,
            touchpad_sensitivity: 0.68,
            deadzone_pixels: 8,
            minimum_update_interval_ms: 4,
            smoothing_factor: 0.8,
            ignore_fullscreen_windows: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GestureAction {
    WindowMove,
    MouseDrag,
}

impl Default for GestureAction {
    fn default() -> Self {
        Self::MouseDrag
    }
}

impl GestureAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowMove => "Window move",
            Self::MouseDrag => "Mouse drag",
        }
    }
}

impl AppConfig {
    pub fn load_or_create(paths: &AppPaths) -> Result<Self> {
        paths.ensure_dirs()?;

        if !paths.config_path().exists() {
            let default = Self::default();
            default.save(paths)?;
            return Ok(default);
        }

        let raw = fs::read_to_string(paths.config_path())
            .with_context(|| format!("failed to read {}", paths.config_path().display()))?;

        match serde_json::from_str::<Self>(&raw) {
            Ok(config) => Ok(config),
            Err(error) => {
                let backup = paths.data_dir().join("config.invalid.json");
                let _ = fs::copy(paths.config_path(), &backup);
                let default = Self::default();
                default.save(paths)?;
                Err(anyhow!(
                    "invalid config file detected and reset to defaults: {error}"
                ))
            }
        }
    }

    pub fn save(&self, paths: &AppPaths) -> Result<()> {
        paths.ensure_dirs()?;
        let payload = serde_json::to_string_pretty(self)?;
        fs::write(paths.config_path(), payload)
            .with_context(|| format!("failed to write {}", paths.config_path().display()))
    }
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, GestureAction};

    #[test]
    fn default_config_uses_three_fingers() {
        let config = AppConfig::default();
        assert!(config.enabled);
        assert!(config.launch_at_startup);
        assert_eq!(config.touchpad_profile, "drag_drop_precise");
        assert_eq!(config.gesture_action, GestureAction::MouseDrag);
        assert_eq!(config.gesture_finger_count, 3);
        assert_eq!(config.touchpad_sensitivity, 0.68);
    }
}
