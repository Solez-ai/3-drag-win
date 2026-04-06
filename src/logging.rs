use crate::config::AppPaths;
use anyhow::{Context, Result};
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::fs::OpenOptions;

pub fn init(paths: &AppPaths) -> Result<()> {
    paths.ensure_dirs()?;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.log_path())
        .with_context(|| format!("failed to open {}", paths.log_path().display()))?;

    WriteLogger::init(LevelFilter::Info, Config::default(), file)
        .map_err(|error| anyhow::anyhow!("failed to initialize logging: {error}"))
}
