use anyhow::{Context, Result};
use keyring::Entry;

const KEYRING_SERVICE: &str = "personal-jira-kanban";
const KEYRING_ACCOUNT: &str = "jira-api-token";

pub fn load_jira_api_token() -> Result<Option<String>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .context("failed to access keyring entry for jira api token")?;

    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(error).context("failed to read jira api token from keyring"),
    }
}

pub fn save_jira_api_token(token: &str) -> Result<()> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .context("failed to access keyring entry for jira api token")?;
    entry
        .set_password(token)
        .context("failed to store jira api token in keyring")?;
    Ok(())
}
