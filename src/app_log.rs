use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

const APP_QUALIFIER: &str = "com";
const APP_ORGANIZATION: &str = "joborganizer";
const APP_NAME: &str = "personal-jira-kanban";
const LOG_FILE: &str = "app.log";

pub fn append_error(scope: &str, details: &str) -> Result<PathBuf> {
    let path = log_path()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open log file at {}", path.display()))?;

    writeln!(
        file,
        "{} [ERROR] [{}] {}",
        Utc::now().to_rfc3339(),
        scope,
        details.replace('\n', " | ")
    )
    .context("failed to write error log line")?;

    Ok(path)
}

pub fn log_path() -> Result<PathBuf> {
    let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
        .ok_or_else(|| anyhow!("unable to resolve local app data directory for logs"))?;
    create_dir_all(project_dirs.data_local_dir())
        .context("failed to create local app data path")?;
    Ok(project_dirs.data_local_dir().join(LOG_FILE))
}
