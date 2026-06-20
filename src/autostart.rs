use anyhow::{Context, Result};
use auto_launch::{AutoLaunch, AutoLaunchBuilder};

#[cfg(windows)]
use auto_launch::WindowsEnableMode;

pub fn synchronize(enabled: bool) -> Result<()> {
    let launcher = build_launcher()?;
    let current = launcher.is_enabled().unwrap_or(false);

    match (enabled, current) {
        (true, false) => launcher.enable().context("failed to enable auto-start")?,
        (false, true) => launcher.disable().context("failed to disable auto-start")?,
        _ => {}
    }

    Ok(())
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let launcher = build_launcher()?;
    if enabled {
        launcher.enable().context("failed to enable auto-start")?;
    } else {
        launcher.disable().context("failed to disable auto-start")?;
    }

    Ok(())
}

fn build_launcher() -> Result<AutoLaunch> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let exe = exe.to_string_lossy().into_owned();

    let mut builder = AutoLaunchBuilder::new();
    builder
        .set_app_name("3-win-drag")
        .set_app_path(&exe);

    #[cfg(windows)]
    builder.set_windows_enable_mode(WindowsEnableMode::CurrentUser);

    builder
        .build()
        .context("failed to create auto-launch entry")
}
