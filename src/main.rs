#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
mod linux;

mod app;
mod autostart;
mod commands;
mod config;
mod ffi;
mod logging;
mod settings_ui;
mod single_instance;
mod touchpad;
mod tray;

fn main() {
    if let Err(error) = app::run() {
        if error.to_string().contains("already running") {
            return;
        }

        #[cfg(windows)]
        ffi::show_error_dialog("3-win-drag failed to start", &error.to_string());

        #[cfg(not(windows))]
        eprintln!("{error:#}");
    }
}
