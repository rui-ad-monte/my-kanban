# Personal Jira Layer (Dioxus Desktop)

Desktop-first starter for a personal Jira workflow layer:

- `Unmapped Issues` section for assigned issues without a personal column.
- `My Kanban` section with custom user columns and drag/drop cards.
- Drop modal with optional Jira transition and optional Jira comment.
- Local persistence in SQLite for issues, columns, placements, and queued Jira updates.

## Run

```bash
cargo run
```

## Current state

- Local-first behavior is implemented.
- Jira API sync is not wired yet; sample issues are seeded on first run.
- Optional transition/comment are queued in `outbox_actions` for future Jira sync worker.

## Next milestone

1. Add Jira Cloud auth setup (`base_url`, email, API token).
2. Pull assigned issues and upsert `jira_issue_snapshot`.
3. Resolve available transitions per issue from Jira.
4. Process `outbox_actions` in a background worker and mark success/failure.
