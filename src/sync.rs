use crate::jira_client::{JiraClient, JiraCredentials};
use crate::models::{JiraSettings, OutboxAction};
use crate::storage::AppStorage;
use anyhow::{Context, Result};

const DEFAULT_MAX_ACTIONS: usize = 50;

#[derive(Clone, Debug, Default)]
pub struct QueueRunSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub remaining_pending: usize,
    pub remaining_failed: usize,
}

pub async fn process_outbox_once(
    settings: JiraSettings,
    api_token: String,
    max_actions: usize,
) -> Result<QueueRunSummary> {
    let storage = AppStorage::open()?;
    let credentials = JiraCredentials::from_settings(&settings, api_token)?;
    let client = JiraClient::new(credentials)?;
    let mut summary = QueueRunSummary::default();

    let batch_size = if max_actions == 0 {
        DEFAULT_MAX_ACTIONS
    } else {
        max_actions
    };

    let actions = storage.load_processable_outbox_actions(batch_size)?;
    for action in actions {
        summary.attempted += 1;

        match process_single_action(&storage, &client, &action).await {
            Ok(()) => {
                summary.succeeded += 1;
            }
            Err(error) => {
                summary.failed += 1;
                let error_details = format!("{error:#}");
                storage.mark_outbox_action_failed(action.id, &error_details)?;
            }
        }
    }

    summary.remaining_pending = storage.count_outbox_actions_by_state("pending")?
        + storage.count_outbox_actions_by_state("processing")?;
    summary.remaining_failed = storage.count_outbox_actions_by_state("failed")?;

    Ok(summary)
}

async fn process_single_action(
    storage: &AppStorage,
    client: &JiraClient,
    action: &OutboxAction,
) -> Result<()> {
    storage.mark_outbox_action_processing(action.id)?;

    if let Some(transition_id) = action.transition_id.as_deref() {
        client
            .transition_issue(&action.issue_key, transition_id)
            .await
            .with_context(|| {
                format!(
                    "failed to transition issue {} with transition {}",
                    action.issue_key, transition_id
                )
            })?;

        storage.clear_outbox_transition(action.id)?;
    }

    if let Some(comment) = action.comment.as_deref() {
        let clean_comment = comment.trim();
        if !clean_comment.is_empty() {
            client
                .add_comment(&action.issue_key, clean_comment)
                .await
                .with_context(|| format!("failed to add comment to issue {}", action.issue_key))?;
        }

        storage.clear_outbox_comment(action.id)?;
    }

    storage.mark_outbox_action_done(action.id)?;
    Ok(())
}
