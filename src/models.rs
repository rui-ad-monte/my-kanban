#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueSnapshot {
    pub issue_key: String,
    pub summary: String,
    pub assignee: String,
    pub jira_status: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardColumn {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssuePlacement {
    pub issue_key: String,
    pub column_id: Option<i64>,
    pub rank: i64,
    pub moved_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxAction {
    pub id: i64,
    pub issue_key: String,
    pub transition_id: Option<String>,
    pub transition_name: Option<String>,
    pub comment: Option<String>,
    pub state: String,
    pub last_error: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Default)]
pub struct PersistedState {
    pub issues: Vec<IssueSnapshot>,
    pub columns: Vec<BoardColumn>,
    pub placements: Vec<IssuePlacement>,
    pub outbox_actions: Vec<OutboxAction>,
}

pub const DEFAULT_ASSIGNED_ISSUES_JQL: &str =
    "assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JiraSettings {
    pub base_url: String,
    pub email: String,
    pub jql: String,
}

impl Default for JiraSettings {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            email: String::new(),
            jql: DEFAULT_ASSIGNED_ISSUES_JQL.to_string(),
        }
    }
}

impl JiraSettings {
    pub fn normalized_base_url(&self) -> String {
        self.base_url.trim().trim_end_matches('/').to_string()
    }

    pub fn normalized_email(&self) -> String {
        self.email.trim().to_string()
    }

    pub fn effective_jql(&self) -> String {
        let clean = self.jql.trim();
        if clean.is_empty() {
            DEFAULT_ASSIGNED_ISSUES_JQL.to_string()
        } else {
            clean.to_string()
        }
    }
}
