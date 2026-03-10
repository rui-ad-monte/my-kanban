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
3. Paste an API token (stored in your system keychain on save).
4. Optional: adjust the JQL used for assigned issue sync.
5. Click `Test Connection`, then `Sync Now`.

## Current state

- Local-first behavior is implemented.
- Manual Jira pull sync is wired (`myself` + `search` for assigned issues).
- Optional transition/comment are still queued in `outbox_actions` for future sync worker.

## Next milestone

1. Resolve available transitions per issue from Jira (for drop modal).
2. Process `outbox_actions` in a background worker and mark success/failure.
3. Add retry/backoff policy and sync status badges on cards.
4. Add column/card reordering polish and conflict indicators.
