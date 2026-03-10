mod app_log;
mod jira_client;
mod models;
mod storage;

use crate::jira_client::{JiraClient, JiraCredentials, JiraTransition};
use crate::models::{BoardColumn, IssuePlacement, IssueSnapshot, JiraSettings};
use crate::storage::AppStorage;
use dioxus::prelude::*;
use std::collections::HashMap;

const APP_STYLE: &str = r#"
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  font-family: "Avenir Next", "Fira Sans", "Segoe UI", sans-serif;
  color: #1f2a37;
  background: radial-gradient(circle at 20% 20%, #fdf6e3 0%, #f5f0e6 40%, #ece8df 100%);
}

.app-shell {
  min-height: 100vh;
  padding: 22px;
}

.topbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  margin-bottom: 18px;
}

.title-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.title-group h1 {
  margin: 0;
  font-size: 30px;
  letter-spacing: 0.2px;
}

.title-group p {
  margin: 0;
  color: #4b5563;
}

.badge {
  padding: 8px 12px;
  border-radius: 999px;
  border: 1px solid #d1d5db;
  background: #ffffff;
  font-size: 12px;
  letter-spacing: 0.4px;
  text-transform: uppercase;
}

.settings-panel {
  margin-bottom: 14px;
  border-radius: 16px;
  border: 1px solid #d1d5db;
  background: rgba(255, 255, 255, 0.9);
  padding: 14px;
  display: grid;
  gap: 12px;
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
}

.settings-header h2 {
  margin: 0;
  font-size: 18px;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 10px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field label {
  color: #4b5563;
  font-size: 13px;
}

.settings-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tabs {
  display: flex;
  gap: 10px;
  margin-bottom: 14px;
}

.tab-btn {
  border: 0;
  border-radius: 12px;
  padding: 10px 16px;
  background: #dde4ea;
  color: #25313f;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.15s ease, background-color 0.15s ease;
}

.tab-btn:hover {
  transform: translateY(-1px);
}

.tab-btn.active {
  background: #115e59;
  color: #f8fafc;
}

.notice {
  margin-bottom: 12px;
  border-radius: 10px;
  padding: 10px 12px;
  background: #ecfeff;
  border: 1px solid #67e8f9;
  color: #0f4d5f;
}

.error {
  margin-bottom: 12px;
  border-radius: 10px;
  padding: 10px 12px;
  background: #fff1f2;
  border: 1px solid #fda4af;
  color: #9f1239;
}

.panel {
  border-radius: 16px;
  border: 1px solid #d3d8df;
  background: rgba(255, 255, 255, 0.88);
  backdrop-filter: blur(2px);
  padding: 16px;
}

.panel h2 {
  margin: 0 0 12px 0;
  font-size: 20px;
}

.list {
  display: grid;
  gap: 12px;
}

.issue-card {
  border-radius: 12px;
  border: 1px solid #d4d7dc;
  background: #ffffff;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  cursor: grab;
}

.issue-card:active {
  cursor: grabbing;
}

.issue-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.issue-key {
  font-family: "Fira Mono", "Consolas", monospace;
  font-size: 12px;
  color: #0f766e;
}

.status-chip {
  border-radius: 999px;
  background: #fef3c7;
  color: #78350f;
  padding: 4px 8px;
  font-size: 12px;
}

.column-creator {
  margin-bottom: 16px;
  display: flex;
  gap: 8px;
}

.text-input {
  flex: 1;
  border-radius: 10px;
  border: 1px solid #d1d5db;
  padding: 10px;
  font-size: 14px;
}

.button {
  border: 0;
  border-radius: 10px;
  padding: 10px 14px;
  background: #115e59;
  color: #f8fafc;
  font-weight: 600;
  cursor: pointer;
}

.button.secondary {
  background: #475569;
}

.kanban-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 14px;
}

.kanban-column {
  border-radius: 14px;
  border: 1px dashed #b7c0c9;
  background: #f8fafc;
  min-height: 220px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.kanban-column h3 {
  margin: 0;
  color: #334155;
}

.drop-hint {
  color: #6b7280;
  font-size: 13px;
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.45);
  display: grid;
  place-items: center;
  padding: 20px;
}

.modal {
  width: min(520px, 100%);
  border-radius: 14px;
  background: #ffffff;
  padding: 18px;
  border: 1px solid #d7dde5;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.modal h3 {
  margin: 0;
}

.label {
  font-size: 13px;
  color: #475569;
}

.select,
.textarea {
  width: 100%;
  border-radius: 10px;
  border: 1px solid #d1d5db;
  padding: 10px;
  font-size: 14px;
}

.textarea {
  resize: vertical;
  min-height: 88px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Unmapped,
    Kanban,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DropDialogState {
    issue_key: String,
    target_column_id: i64,
    selected_transition_id: String,
    transitions: Vec<JiraTransition>,
    transitions_loading: bool,
    transitions_error: Option<String>,
    comment: String,
}

#[derive(Clone, Debug, Default)]
struct UiState {
    issues: Vec<IssueSnapshot>,
    columns: Vec<BoardColumn>,
    placements: HashMap<String, IssuePlacement>,
    outbox_pending_count: usize,
    jira_settings: JiraSettings,
    jira_token_saved: bool,
    notice: Option<String>,
    load_error: Option<String>,
}

fn main() {
    LaunchBuilder::new()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("Personal Jira Layer")
                    .with_always_on_top(false),
            ),
        )
        .launch(App);
}

#[allow(non_snake_case)]
fn App() -> Element {
    let mut ui_state = use_signal(load_ui_state_safely);
    let mut jira_base_url = use_signal(|| ui_state().jira_settings.base_url.clone());
    let mut jira_email = use_signal(|| ui_state().jira_settings.email.clone());
    let mut jira_jql = use_signal(|| ui_state().jira_settings.effective_jql());
    let mut jira_api_token = use_signal(String::new);
    let mut section = use_signal(|| Section::Unmapped);
    let mut dragging_issue = use_signal(|| Option::<String>::None);
    let mut drop_dialog = use_signal(|| Option::<DropDialogState>::None);
    let mut new_column_name = use_signal(String::new);

    let snapshot = ui_state();
    let unmapped = unmapped_issues(&snapshot);
    let drop_dialog_snapshot = drop_dialog();

    rsx! {
        style { "{APP_STYLE}" }
        div { class: "app-shell",
            header { class: "topbar",
                div { class: "title-group",
                    h1 { "Personal Jira Layer" }
                    p { "Desktop-first Dioxus starter with local board state" }
                }
                span { class: "badge", "Pending Jira updates: {snapshot.outbox_pending_count}" }
            }

            section { class: "settings-panel",
                div { class: "settings-header",
                    h2 { "Jira Connection" }
                    span { class: "badge", if snapshot.jira_token_saved { "Token: saved" } else { "Token: missing" } }
                }
                div { class: "settings-grid",
                    div { class: "field",
                        label { "Base URL" }
                        input {
                            class: "text-input",
                            r#type: "url",
                            value: "{jira_base_url}",
                            placeholder: "https://your-org.atlassian.net",
                            oninput: move |event| jira_base_url.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { "Email" }
                        input {
                            class: "text-input",
                            r#type: "email",
                            value: "{jira_email}",
                            placeholder: "you@company.com",
                            oninput: move |event| jira_email.set(event.value()),
                        }
                    }
                    div { class: "field",
                        label { "API Token" }
                        input {
                            class: "text-input",
                            r#type: "password",
                            value: "{jira_api_token}",
                            placeholder: "Paste new token to set/update",
                            oninput: move |event| jira_api_token.set(event.value()),
                        }
                    }
                }
                div { class: "field",
                    label { "Assigned issues JQL" }
                    textarea {
                        class: "textarea",
                        value: "{jira_jql}",
                        oninput: move |event| jira_jql.set(event.value()),
                    }
                }
                div { class: "settings-actions",
                    button {
                        class: "button",
                        onclick: move |_| {
                            let settings = JiraSettings {
                                base_url: jira_base_url(),
                                email: jira_email(),
                                jql: jira_jql(),
                            };

                            let token_to_store = jira_api_token().trim().to_string();
                            let save_result = AppStorage::open().and_then(|storage| {
                                storage.save_jira_settings(&settings)?;
                                persist_api_token_if_provided(&token_to_store)?;
                                Ok(())
                            });

                            match save_result {
                                Ok(()) => {
                                    jira_base_url.set(settings.normalized_base_url());
                                    jira_email.set(settings.normalized_email());
                                    jira_jql.set(settings.effective_jql());

                                    let mut next = load_ui_state_safely();
                                    next.notice = Some(
                                        if token_to_store.is_empty() {
                                            "Saved Jira settings. API token unchanged.".to_string()
                                        } else {
                                            "Saved Jira settings and updated API token.".to_string()
                                        },
                                    );
                                    ui_state.set(next);
                                }
                                Err(error) => {
                                    let mut next = ui_state();
                                    next.notice = Some(format!(
                                        "Failed to save Jira settings: {}",
                                        format_error_with_log("save_settings", &error)
                                    ));
                                    ui_state.set(next);
                                }
                            }
                        },
                        "Save Settings"
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            let settings = JiraSettings {
                                base_url: jira_base_url(),
                                email: jira_email(),
                                jql: jira_jql(),
                            };

                            let token_input = jira_api_token().trim().to_string();
                            let mut ui_state_signal = ui_state;

                            spawn(async move {
                                let test_result = async {
                                    let api_token = resolve_api_token(&token_input)?;
                                    let credentials = JiraCredentials::from_settings(&settings, api_token)?;
                                    let client = JiraClient::new(credentials)?;
                                    client.test_connection().await
                                }
                                .await;

                                match test_result {
                                    Ok(display_name) => {
                                        let mut next = ui_state_signal();
                                        next.notice = Some(format!(
                                            "Jira connection successful for {display_name}."
                                        ));
                                        ui_state_signal.set(next);
                                    }
                                    Err(error) => {
                                        let mut next = ui_state_signal();
                                        next.notice = Some(format!(
                                            "Jira connection failed: {}",
                                            format_error_with_log("test_connection", &error)
                                        ));
                                        ui_state_signal.set(next);
                                    }
                                }
                            });
                        },
                        "Test Connection"
                    }
                    button {
                        class: "button",
                        onclick: move |_| {
                            let settings = JiraSettings {
                                base_url: jira_base_url(),
                                email: jira_email(),
                                jql: jira_jql(),
                            };
                            let token_to_store = jira_api_token().trim().to_string();

                            let mut ui_state_signal = ui_state;
                            let mut jira_base_url_signal = jira_base_url;
                            let mut jira_email_signal = jira_email;
                            let mut jira_jql_signal = jira_jql;

                            spawn(async move {
                                let sync_result = async {
                                    let api_token = resolve_api_token(&token_to_store)?;
                                    let credentials = JiraCredentials::from_settings(&settings, api_token)?;
                                    let client = JiraClient::new(credentials)?;
                                    let issues = client.fetch_assigned_issues(&settings.effective_jql()).await?;

                                    let storage = AppStorage::open()?;
                                    storage.save_jira_settings(&settings)?;
                                    storage.upsert_issue_snapshots(&issues)?;

                                    Ok::<usize, anyhow::Error>(issues.len())
                                }
                                .await;

                                match sync_result {
                                    Ok(issue_count) => {
                                        jira_base_url_signal.set(settings.normalized_base_url());
                                        jira_email_signal.set(settings.normalized_email());
                                        jira_jql_signal.set(settings.effective_jql());

                                        let mut next = load_ui_state_safely();
                                        next.notice = Some(format!(
                                            "Sync complete: fetched {issue_count} assigned issues from Jira."
                                        ));
                                        ui_state_signal.set(next);
                                    }
                                    Err(error) => {
                                        let mut next = ui_state_signal();
                                        next.notice = Some(format!(
                                            "Sync failed: {}",
                                            format_error_with_log("sync_now", &error)
                                        ));
                                        ui_state_signal.set(next);
                                    }
                                }
                            });
                        },
                        "Sync Now"
                    }
                }
            }

            div { class: "tabs",
                button {
                    class: if section() == Section::Unmapped { "tab-btn active" } else { "tab-btn" },
                    onclick: move |_| section.set(Section::Unmapped),
                    "Unmapped Issues"
                }
                button {
                    class: if section() == Section::Kanban { "tab-btn active" } else { "tab-btn" },
                    onclick: move |_| section.set(Section::Kanban),
                    "My Kanban"
                }
            }

            {
                snapshot
                    .load_error
                    .clone()
                    .map(|error| rsx! { div { class: "error", "{error}" } })
            }
            {
                snapshot
                    .notice
                    .clone()
                    .map(|notice| rsx! { div { class: "notice", "{notice}" } })
            }

            if section() == Section::Unmapped {
                section {
                    class: "panel",
                    ondragover: move |event| event.prevent_default(),
                    ondrop: move |event| {
                        event.prevent_default();
                        if let Some(issue_key) = dragging_issue() {
                            let result = AppStorage::open().and_then(|storage| storage.unmap_issue(&issue_key));
                            match result {
                                Ok(()) => {
                                    let mut next = load_ui_state_safely();
                                    next.notice = Some("Issue moved back to Unmapped.".to_string());
                                    ui_state.set(next);
                                    dragging_issue.set(None);
                                }
                                Err(error) => {
                                    let mut next = ui_state();
                                    next.notice = Some(format!("Could not unmap issue: {error}"));
                                    ui_state.set(next);
                                }
                            }
                        }
                    },
                    h2 { "Unmapped issues assigned to me" }
                    if unmapped.is_empty() {
                        p { class: "drop-hint", "No unmapped issues. Drop a card here to remove personal column mapping." }
                    }
                    div { class: "list",
                        for issue in unmapped {
                            {render_issue_card(issue, dragging_issue)}
                        }
                    }
                }
            }

            if section() == Section::Kanban {
                section { class: "panel",
                    h2 { "My Kanban" }
                    div { class: "column-creator",
                        input {
                            class: "text-input",
                            r#type: "text",
                            value: "{new_column_name}",
                            placeholder: "Add a custom column",
                            oninput: move |event| new_column_name.set(event.value()),
                        }
                        button {
                            class: "button",
                            onclick: move |_| {
                                let name = new_column_name().trim().to_string();
                                if name.is_empty() {
                                    let mut next = ui_state();
                                    next.notice = Some("Type a column name before adding it.".to_string());
                                    ui_state.set(next);
                                    return;
                                }

                                let result = AppStorage::open().and_then(|storage| storage.create_column(&name));
                                match result {
                                    Ok(()) => {
                                        let mut next = load_ui_state_safely();
                                        next.notice = Some(format!("Added column '{name}'."));
                                        ui_state.set(next);
                                        new_column_name.set(String::new());
                                    }
                                    Err(error) => {
                                        let mut next = ui_state();
                                        next.notice = Some(format!("Failed to add column: {error}"));
                                        ui_state.set(next);
                                    }
                                }
                            },
                            "Add Column"
                        }
                    }

                    div { class: "kanban-grid",
                        for column in snapshot.columns.clone() {
                            {
                                let column_id = column.id;
                                let column_issues = issues_in_column(&snapshot, column_id);
                                let issue_count = column_issues.len();
                                rsx! {
                                    section {
                                        key: "{column_id}",
                                        class: "kanban-column",
                                        ondragover: move |event| event.prevent_default(),
                                        ondrop: move |event| {
                                            event.prevent_default();
                                            if let Some(issue_key) = dragging_issue() {
                                                let settings = JiraSettings {
                                                    base_url: jira_base_url(),
                                                    email: jira_email(),
                                                    jql: jira_jql(),
                                                };
                                                let token_input = jira_api_token().trim().to_string();

                                                drop_dialog.set(Some(DropDialogState {
                                                    issue_key: issue_key.clone(),
                                                    target_column_id: column_id,
                                                    selected_transition_id: String::new(),
                                                    transitions: Vec::new(),
                                                    transitions_loading: true,
                                                    transitions_error: None,
                                                    comment: String::new(),
                                                }));
                                                dragging_issue.set(None);

                                                let issue_key_for_fetch = issue_key;
                                                let mut drop_dialog_signal = drop_dialog;

                                                spawn(async move {
                                                    let transitions_result = async {
                                                        let api_token = resolve_api_token(&token_input)?;
                                                        let credentials = JiraCredentials::from_settings(&settings, api_token)?;
                                                        let client = JiraClient::new(credentials)?;
                                                        client.fetch_issue_transitions(&issue_key_for_fetch).await
                                                    }
                                                    .await;

                                                    let Some(mut current_dialog) = drop_dialog_signal() else {
                                                        return;
                                                    };

                                                    if current_dialog.issue_key != issue_key_for_fetch {
                                                        return;
                                                    }

                                                    current_dialog.transitions_loading = false;

                                                    match transitions_result {
                                                        Ok(transitions) => {
                                                            if !transitions.iter().any(|transition| {
                                                                transition.id
                                                                    == current_dialog.selected_transition_id
                                                            }) {
                                                                current_dialog.selected_transition_id =
                                                                    String::new();
                                                            }
                                                            current_dialog.transitions = transitions;
                                                            current_dialog.transitions_error = None;
                                                        }
                                                        Err(error) => {
                                                            current_dialog.transitions = Vec::new();
                                                            let details = format_error_with_log(
                                                                "load_transitions",
                                                                &error,
                                                            );
                                                            current_dialog.transitions_error = Some(format!(
                                                                "Could not load transitions from Jira: {details}"
                                                            ));
                                                        }
                                                    }

                                                    drop_dialog_signal.set(Some(current_dialog));
                                                });
                                            }
                                        },
                                        h3 { "{column.name} ({issue_count})" }
                                        if issue_count == 0 {
                                            p { class: "drop-hint", "Drop issues here" }
                                        }
                                        for issue in column_issues {
                                            {render_issue_card(issue, dragging_issue)}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        {
            drop_dialog_snapshot.map(|dialog| {
                let target_column_name = snapshot
                    .columns
                    .iter()
                    .find(|column| column.id == dialog.target_column_id)
                    .map(|column| column.name.clone())
                    .unwrap_or_else(|| "column".to_string());

                rsx! {
                    div { class: "modal-backdrop",
                        div { class: "modal",
                            h3 { "Finalize move" }
                            p { "Issue {dialog.issue_key} moved to {target_column_name}" }
                            label { class: "label", "Jira transition (optional)" }
                            if dialog.transitions_loading {
                                p { class: "drop-hint", "Loading transitions from Jira..." }
                            }
                            {
                                dialog
                                    .transitions_error
                                    .clone()
                                    .map(|error| rsx! { p { class: "error", "{error}" } })
                            }
                            select {
                                class: "select",
                                value: "{dialog.selected_transition_id}",
                                onchange: move |event| {
                                    if let Some(mut current) = drop_dialog() {
                                        current.selected_transition_id = event.value();
                                        drop_dialog.set(Some(current));
                                    }
                                },
                                option { value: "", "No Jira transition" }
                                for transition in dialog.transitions.clone() {
                                    option {
                                        key: "{transition.id}",
                                        value: "{transition.id}",
                                        "{transition.name}"
                                    }
                                }
                            }
                            if !dialog.transitions_loading && dialog.transitions.is_empty() && dialog.transitions_error.is_none() {
                                p { class: "drop-hint", "No available transitions for this issue." }
                            }
                            label { class: "label", "Comment (optional)" }
                            textarea {
                                class: "textarea",
                                value: "{dialog.comment}",
                                placeholder: "Add comment for Jira (optional)",
                                oninput: move |event| {
                                    if let Some(mut current) = drop_dialog() {
                                        current.comment = event.value();
                                        drop_dialog.set(Some(current));
                                    }
                                },
                            }
                            div { class: "modal-actions",
                                button {
                                    class: "button secondary",
                                    onclick: move |_| drop_dialog.set(None),
                                    "Cancel"
                                }
                                button {
                                    class: "button",
                                    onclick: move |_| {
                                        let Some(current) = drop_dialog() else {
                                            return;
                                        };

                                        let transition_id = normalize_optional(&current.selected_transition_id);
                                        let transition_name = transition_id.as_ref().and_then(|selected_id| {
                                            current
                                                .transitions
                                                .iter()
                                                .find(|transition| transition.id == *selected_id)
                                                .map(|transition| transition.name.clone())
                                        });
                                        let comment = normalize_optional(&current.comment);
                                        let rank = next_rank_for_column(&ui_state(), current.target_column_id);

                                        let save_result = AppStorage::open().and_then(|storage| {
                                            storage.move_issue(&current.issue_key, current.target_column_id, rank)?;
                                            storage.queue_jira_update(
                                                &current.issue_key,
                                                transition_id.as_deref(),
                                                transition_name.as_deref(),
                                                comment.as_deref(),
                                            )?;
                                            Ok(())
                                        });

                                        match save_result {
                                            Ok(()) => {
                                                let mut next = load_ui_state_safely();
                                                next.notice = Some(match (transition_id.is_some(), comment.is_some()) {
                                                    (false, false) => "Move saved locally.".to_string(),
                                                    _ => "Move saved locally and Jira update queued.".to_string(),
                                                });
                                                ui_state.set(next);
                                                drop_dialog.set(None);
                                            }
                                            Err(error) => {
                                                let mut next = ui_state();
                                                next.notice = Some(format!("Could not save move: {error}"));
                                                ui_state.set(next);
                                            }
                                        }
                                    },
                                    "Save Move"
                                }
                            }
                        }
                    }
                }
            })
        }
    }
}

fn render_issue_card(issue: IssueSnapshot, mut dragging_issue: Signal<Option<String>>) -> Element {
    let issue_key = issue.issue_key.clone();
    let issue_key_for_drag = issue_key.clone();
    rsx! {
        article {
            key: "{issue_key}",
            class: "issue-card",
            draggable: true,
            ondragstart: move |_| dragging_issue.set(Some(issue_key_for_drag.clone())),
            ondragend: move |_| dragging_issue.set(None),
            div { class: "issue-header",
                span { class: "issue-key", "{issue.issue_key}" }
                span { class: "status-chip", "{issue.jira_status}" }
            }
            strong { "{issue.summary}" }
        }
    }
}

fn unmapped_issues(state: &UiState) -> Vec<IssueSnapshot> {
    state
        .issues
        .iter()
        .filter(|issue| !state.placements.contains_key(&issue.issue_key))
        .cloned()
        .collect()
}

fn issues_in_column(state: &UiState, column_id: i64) -> Vec<IssueSnapshot> {
    let mut positioned = state
        .issues
        .iter()
        .filter_map(|issue| {
            state
                .placements
                .get(&issue.issue_key)
                .and_then(|placement| {
                    if placement.column_id == Some(column_id) {
                        Some((placement.rank, issue.clone()))
                    } else {
                        None
                    }
                })
        })
        .collect::<Vec<_>>();

    positioned.sort_by_key(|(rank, _)| *rank);
    positioned.into_iter().map(|(_, issue)| issue).collect()
}

fn next_rank_for_column(state: &UiState, column_id: i64) -> i64 {
    state
        .placements
        .values()
        .filter(|placement| placement.column_id == Some(column_id))
        .map(|placement| placement.rank)
        .max()
        .unwrap_or(0)
        + 1
}

fn normalize_optional(value: &str) -> Option<String> {
    let clean = value.trim();
    if clean.is_empty() {
        None
    } else {
        Some(clean.to_string())
    }
}

fn resolve_api_token(token_input: &str) -> anyhow::Result<String> {
    let token = token_input.trim();
    if !token.is_empty() {
        persist_api_token_if_provided(token)?;
        return Ok(token.to_string());
    }

    AppStorage::open()?.load_jira_api_token()?.ok_or_else(|| {
        anyhow::anyhow!(
            "Jira API token is missing. Enter a token and click Save Settings, or paste one to use now."
        )
    })
}

fn persist_api_token_if_provided(token_input: &str) -> anyhow::Result<()> {
    let token = token_input.trim();
    if token.is_empty() {
        return Ok(());
    }

    AppStorage::open()?.save_jira_api_token(token)?;
    Ok(())
}

fn format_error_with_log(scope: &str, error: &anyhow::Error) -> String {
    let details = format!("{error:#}");

    match app_log::append_error(scope, &details) {
        Ok(path) => format!("{details} (log: {})", path.display()),
        Err(log_error) => format!("{details} (also failed to write log: {log_error:#})"),
    }
}

fn load_ui_state_safely() -> UiState {
    match load_ui_state() {
        Ok(state) => state,
        Err(error) => {
            let details = format_error_with_log("load_ui_state", &error);
            UiState {
                load_error: Some(format!("Failed to load local data: {details}")),
                ..UiState::default()
            }
        }
    }
}

fn load_ui_state() -> anyhow::Result<UiState> {
    let storage = AppStorage::open()?;
    let persisted = storage.load_state()?;
    let mut jira_settings = storage.load_jira_settings()?;
    jira_settings.jql = jira_settings.effective_jql();

    let jira_token_saved = match storage.load_jira_api_token() {
        Ok(token) => token.is_some(),
        Err(_) => false,
    };

    let placements = persisted
        .placements
        .into_iter()
        .map(|placement| (placement.issue_key.clone(), placement))
        .collect::<HashMap<_, _>>();

    let outbox_pending_count = persisted
        .outbox_actions
        .iter()
        .filter(|action| action.state == "pending")
        .count();

    Ok(UiState {
        issues: persisted.issues,
        columns: persisted.columns,
        placements,
        outbox_pending_count,
        jira_settings,
        jira_token_saved,
        notice: None,
        load_error: None,
    })
}
