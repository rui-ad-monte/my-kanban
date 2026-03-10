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
    pub transition_to: Option<String>,
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
