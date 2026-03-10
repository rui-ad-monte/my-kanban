use crate::models::{
    BoardColumn, IssuePlacement, IssueSnapshot, JiraSettings, OutboxAction, PersistedState,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::PathBuf;

const APP_QUALIFIER: &str = "com";
const APP_ORGANIZATION: &str = "joborganizer";
const APP_NAME: &str = "personal-jira-kanban";
const DATABASE_FILE: &str = "kanban.db";

pub struct AppStorage {
    database_path: PathBuf,
}

impl AppStorage {
    pub fn open() -> Result<Self> {
        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
            .ok_or_else(|| anyhow!("unable to resolve local app data directory"))?;
        std::fs::create_dir_all(project_dirs.data_local_dir())
            .context("failed to create data directory")?;

        let storage = Self {
            database_path: project_dirs.data_local_dir().join(DATABASE_FILE),
        };
        storage.bootstrap()?;
        Ok(storage)
    }

    pub fn load_state(&self) -> Result<PersistedState> {
        let connection = self.connection()?;

        let issues = {
            let mut stmt = connection.prepare(
                "
                SELECT issue_key, summary, assignee, jira_status, updated_at
                FROM jira_issue_snapshot
                ORDER BY updated_at DESC
                ",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(IssueSnapshot {
                    issue_key: row.get(0)?,
                    summary: row.get(1)?,
                    assignee: row.get(2)?,
                    jira_status: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let columns = {
            let mut stmt = connection.prepare(
                "
                SELECT id, name, sort_order
                FROM board_columns
                ORDER BY sort_order ASC, id ASC
                ",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(BoardColumn {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                })
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let placements = {
            let mut stmt = connection.prepare(
                "
                SELECT issue_key, column_id, rank, moved_at
                FROM issue_placements
                ",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(IssuePlacement {
                    issue_key: row.get(0)?,
                    column_id: row.get(1)?,
                    rank: row.get(2)?,
                    moved_at: row.get(3)?,
                })
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let outbox_actions = {
            let mut stmt = connection.prepare(
                "
                SELECT
                    id,
                    issue_key,
                    transition_id,
                    COALESCE(transition_name, transition_to),
                    comment,
                    state,
                    last_error,
                    created_at
                FROM outbox_actions
                ORDER BY id DESC
                ",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(OutboxAction {
                    id: row.get(0)?,
                    issue_key: row.get(1)?,
                    transition_id: row.get(2)?,
                    transition_name: row.get(3)?,
                    comment: row.get(4)?,
                    state: row.get(5)?,
                    last_error: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?;

            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        Ok(PersistedState {
            issues,
            columns,
            placements,
            outbox_actions,
        })
    }

    pub fn create_column(&self, name: &str) -> Result<()> {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return Err(anyhow!("column name cannot be empty"));
        }

        let connection = self.connection()?;
        let next_sort_order = connection
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM board_columns",
                [],
                |row| row.get::<_, i64>(0),
            )
            .context("failed to get next column order")?;

        connection
            .execute(
                "
                INSERT INTO board_columns (name, sort_order, created_at)
                VALUES (?1, ?2, ?3)
                ",
                params![clean_name, next_sort_order, now()],
            )
            .context("failed to insert column")?;

        Ok(())
    }

    pub fn move_issue(&self, issue_key: &str, target_column_id: i64, rank: i64) -> Result<()> {
        let connection = self.connection()?;
        connection
            .execute(
                "
                INSERT INTO issue_placements (issue_key, column_id, rank, moved_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(issue_key)
                DO UPDATE SET column_id = excluded.column_id, rank = excluded.rank, moved_at = excluded.moved_at
                ",
                params![issue_key, target_column_id, rank, now()],
            )
            .with_context(|| format!("failed to move issue {issue_key}"))?;

        Ok(())
    }

    pub fn unmap_issue(&self, issue_key: &str) -> Result<()> {
        let connection = self.connection()?;
        connection
            .execute(
                "DELETE FROM issue_placements WHERE issue_key = ?1",
                params![issue_key],
            )
            .with_context(|| format!("failed to unmap issue {issue_key}"))?;

        Ok(())
    }

    pub fn queue_jira_update(
        &self,
        issue_key: &str,
        transition_id: Option<&str>,
        transition_name: Option<&str>,
        comment: Option<&str>,
    ) -> Result<()> {
        if transition_id.is_none() && comment.is_none() {
            return Ok(());
        }

        let connection = self.connection()?;
        connection
            .execute(
                "
                INSERT INTO outbox_actions (
                    issue_key,
                    transition_to,
                    transition_id,
                    transition_name,
                    comment,
                    state,
                    last_error,
                    created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, 'pending', NULL, ?6)
                ",
                params![
                    issue_key,
                    transition_name,
                    transition_id,
                    transition_name,
                    comment,
                    now()
                ],
            )
            .context("failed to queue jira update")?;

        Ok(())
    }

    pub fn load_jira_settings(&self) -> Result<JiraSettings> {
        let connection = self.connection()?;
        let base_url = self.get_setting_value(&connection, "jira_base_url")?;
        let email = self.get_setting_value(&connection, "jira_email")?;
        let jql = self.get_setting_value(&connection, "jira_jql")?;

        Ok(JiraSettings {
            base_url: base_url.unwrap_or_default(),
            email: email.unwrap_or_default(),
            jql: jql.unwrap_or_default(),
        })
    }

    pub fn save_jira_settings(&self, settings: &JiraSettings) -> Result<()> {
        let connection = self.connection()?;
        self.upsert_setting_value(
            &connection,
            "jira_base_url",
            &settings.normalized_base_url(),
        )?;
        self.upsert_setting_value(&connection, "jira_email", &settings.normalized_email())?;
        self.upsert_setting_value(&connection, "jira_jql", &settings.effective_jql())?;
        Ok(())
    }

    pub fn load_jira_api_token(&self) -> Result<Option<String>> {
        let connection = self.connection()?;
        self.get_setting_value(&connection, "jira_api_token")
    }

    pub fn save_jira_api_token(&self, token: &str) -> Result<()> {
        let clean_token = token.trim();
        if clean_token.is_empty() {
            return Ok(());
        }

        let connection = self.connection()?;
        self.upsert_setting_value(&connection, "jira_api_token", clean_token)
    }

    pub fn upsert_issue_snapshots(&self, issues: &[IssueSnapshot]) -> Result<()> {
        let mut connection = self.connection()?;
        let tx = connection
            .transaction()
            .context("failed to start issue upsert transaction")?;

        {
            let mut statement = tx
                .prepare(
                    "
                    INSERT INTO jira_issue_snapshot (issue_key, summary, assignee, jira_status, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    ON CONFLICT(issue_key)
                    DO UPDATE SET
                        summary = excluded.summary,
                        assignee = excluded.assignee,
                        jira_status = excluded.jira_status,
                        updated_at = excluded.updated_at
                    ",
                )
                .context("failed to prepare issue upsert statement")?;

            for issue in issues {
                statement
                    .execute(params![
                        issue.issue_key,
                        issue.summary,
                        issue.assignee,
                        issue.jira_status,
                        issue.updated_at,
                    ])
                    .with_context(|| format!("failed to upsert issue {}", issue.issue_key))?;
            }
        }

        tx.commit()
            .context("failed to commit issue upsert transaction")?;
        Ok(())
    }

    fn get_setting_value(&self, connection: &Connection, key: &str) -> Result<Option<String>> {
        let mut statement = connection
            .prepare("SELECT value FROM settings WHERE key = ?1")
            .with_context(|| format!("failed to prepare settings lookup for '{key}'"))?;

        let value = statement
            .query_row(params![key], |row| row.get::<_, String>(0))
            .optional()
            .with_context(|| format!("failed to read setting '{key}'"))?;

        Ok(value)
    }

    fn upsert_setting_value(&self, connection: &Connection, key: &str, value: &str) -> Result<()> {
        connection
            .execute(
                "
                INSERT INTO settings (key, value)
                VALUES (?1, ?2)
                ON CONFLICT(key)
                DO UPDATE SET value = excluded.value
                ",
                params![key, value],
            )
            .with_context(|| format!("failed to persist setting '{key}'"))?;

        Ok(())
    }

    fn bootstrap(&self) -> Result<()> {
        let connection = self.connection()?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS jira_issue_snapshot (
                issue_key TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                assignee TEXT NOT NULL,
                jira_status TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS board_columns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS issue_placements (
                issue_key TEXT PRIMARY KEY,
                column_id INTEGER,
                rank INTEGER NOT NULL,
                moved_at TEXT NOT NULL,
                FOREIGN KEY(issue_key) REFERENCES jira_issue_snapshot(issue_key) ON DELETE CASCADE,
                FOREIGN KEY(column_id) REFERENCES board_columns(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS outbox_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                issue_key TEXT NOT NULL,
                transition_to TEXT,
                transition_id TEXT,
                transition_name TEXT,
                comment TEXT,
                state TEXT NOT NULL,
                last_error TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(issue_key) REFERENCES jira_issue_snapshot(issue_key) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        self.ensure_outbox_action_columns(&connection)?;

        self.ensure_default_columns(&connection)?;

        Ok(())
    }

    fn ensure_outbox_action_columns(&self, connection: &Connection) -> Result<()> {
        let mut statement = connection
            .prepare("PRAGMA table_info(outbox_actions)")
            .context("failed to inspect outbox_actions schema")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<rusqlite::Result<HashSet<_>>>()?;

        if !columns.contains("transition_to") {
            connection
                .execute(
                    "ALTER TABLE outbox_actions ADD COLUMN transition_to TEXT",
                    [],
                )
                .context("failed to add outbox_actions.transition_to")?;
        }

        if !columns.contains("transition_id") {
            connection
                .execute(
                    "ALTER TABLE outbox_actions ADD COLUMN transition_id TEXT",
                    [],
                )
                .context("failed to add outbox_actions.transition_id")?;
        }

        if !columns.contains("transition_name") {
            connection
                .execute(
                    "ALTER TABLE outbox_actions ADD COLUMN transition_name TEXT",
                    [],
                )
                .context("failed to add outbox_actions.transition_name")?;
        }

        connection
            .execute(
                "
                UPDATE outbox_actions
                SET transition_name = COALESCE(transition_name, transition_to)
                WHERE transition_to IS NOT NULL
                ",
                [],
            )
            .context("failed to migrate legacy outbox transition names")?;

        Ok(())
    }

    fn ensure_default_columns(&self, connection: &Connection) -> Result<()> {
        let count: i64 =
            connection.query_row("SELECT COUNT(1) FROM board_columns", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        for (sort_order, name) in [
            (1_i64, "To Plan"),
            (2, "In Progress"),
            (3, "Waiting"),
            (4, "Done"),
        ] {
            connection.execute(
                "
                INSERT INTO board_columns (name, sort_order, created_at)
                VALUES (?1, ?2, ?3)
                ",
                params![name, sort_order, now()],
            )?;
        }

        Ok(())
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.database_path).context("failed to open sqlite database")
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}
