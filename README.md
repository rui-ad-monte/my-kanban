# Personal Jira Layer (Dioxus Desktop)

Desktop-first starter for a personal Jira workflow layer:

- `Unmapped Issues` section for assigned issues without a personal column.
- `My Kanban` section with custom user columns and drag/drop cards.
- Drop modal with optional Jira transition and optional Jira comment.
- Local persistence in SQLite for issues, columns, placements, and queued Jira updates.
- Jira settings panel with connection test and manual sync of assigned issues.

## Run

```bash
cargo run
```

## Connect Jira (Cloud)

1. Open the `Jira Connection` panel in the app.
2. Fill `Base URL` (example: `https://your-org.atlassian.net`) and `Email`.
3. Paste an API token (stored locally in SQLite settings on save/test/sync).
4. Optional: adjust the JQL used for assigned issue sync.
5. Click `Test Connection`, then `Sync Now`.

## Logs

- The app writes error logs to a local text file: `app.log` in the app data directory.
- On macOS this is typically under `~/Library/Application Support/com.joborganizer.personal-jira-kanban/`.
- Failure notices in the UI include the log file path for quick troubleshooting.

## Token storage

- Jira API token is saved in local SQLite settings.
- This is local machine storage used to avoid repeated token input.

## Current state

- Local-first behavior is implemented.
- Manual Jira pull sync is wired (`myself` + `search/jql` for assigned issues).
- Drop modal now loads live Jira transitions per issue.
- Optional transition/comment are queued in `outbox_actions` (transition id + name) for future sync worker.

## Next milestone

1. Process `outbox_actions` in a background worker and mark success/failure.
2. Add retry/backoff policy and sync status badges on cards.
3. Add column/card reordering polish and conflict indicators.
4. Add stale issue handling and auto-refresh interval.
