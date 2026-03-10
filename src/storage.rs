use crate::models::{BoardColumn, IssuePlacement, IssueSnapshot, OutboxAction, PersistedState};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use rusqlite::{params, Connection};
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
                SELECT id, issue_key, transition_to, comment, state, last_error, created_at
                FROM outbox_actions
                ORDER BY id DESC
                ",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(OutboxAction {
                    id: row.get(0)?,
                    issue_key: row.get(1)?,
                    transition_to: row.get(2)?,
                    comment: row.get(3)?,
                    state: row.get(4)?,
                    last_error: row.get(5)?,
                    created_at: row.get(6)?,
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
        transition_to: Option<&str>,
        comment: Option<&str>,
    ) -> Result<()> {
        if transition_to.is_none() && comment.is_none() {
            return Ok(());
        }

        let connection = self.connection()?;
        connection
            .execute(
                "
                INSERT INTO outbox_actions (issue_key, transition_to, comment, state, last_error, created_at)
                VALUES (?1, ?2, ?3, 'pending', NULL, ?4)
                ",
                params![issue_key, transition_to, comment, now()],
            )
            .context("failed to queue jira update")?;

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
                comment TEXT,
                state TEXT NOT NULL,
                last_error TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(issue_key) REFERENCES jira_issue_snapshot(issue_key) ON DELETE CASCADE
            );
            ",
        )?;

        self.ensure_default_columns(&connection)?;
        self.ensure_sample_issues(&connection)?;

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

    fn ensure_sample_issues(&self, connection: &Connection) -> Result<()> {
        let count: i64 =
            connection.query_row("SELECT COUNT(1) FROM jira_issue_snapshot", [], |row| {
                row.get(0)
            })?;
        if count > 0 {
            return Ok(());
        }

        let sample_issues = [
            (
                "APP-101",
                "Design first sync between Jira and local board",
                "me",
                "To Do",
            ),
            (
                "APP-102",
                "Add optional comment and transition modal",
                "me",
                "In Progress",
            ),
            (
                "APP-103",
                "Store custom columns and personal status locally",
                "me",
                "To Do",
            ),
        ];

        for (issue_key, summary, assignee, jira_status) in sample_issues {
            connection.execute(
                "
                INSERT INTO jira_issue_snapshot (issue_key, summary, assignee, jira_status, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ",
                params![issue_key, summary, assignee, jira_status, now()],
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
