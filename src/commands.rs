use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub enum AppCommand {
    EnableDragging,
    DisableDragging,
    EnableAutoStart,
    DisableAutoStart,
    OpenDataDirectory,
    OpenLogDirectory,
    OpenSettings,
    ApplyConfig(AppConfig),
    Exit,
}
