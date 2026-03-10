# AGENTS.md

Guidance for coding agents working in this repository.

## Project Overview

- Language: Rust (edition 2021).
- App type: Dioxus Desktop application.
- Primary goal: personal Jira workflow layer with local-first board state.
- Persistence: SQLite via `rusqlite`.
- Error model: `anyhow::Result` with contextual errors.

## Current Repository Layout

- `Cargo.toml`: package and dependency definitions.
- `src/main.rs`: Dioxus UI, drag/drop, modal flow, view-level state.
- `src/storage.rs`: SQLite bootstrap and CRUD operations.
- `src/models.rs`: domain structs used by UI and storage.
- `README.md`: run instructions and milestone summary.
- `FOLLOW_UP_PLAN.mds`: roadmap and phased implementation plan.

## Build / Run Commands

- Install dependencies and build checks:
  - `cargo check`
- Run desktop app:
  - `cargo run`
- Build debug binary:
  - `cargo build`
- Build optimized binary:
  - `cargo build --release`

## Lint / Format Commands

- Format all Rust files:
  - `cargo fmt`
- Check formatting without editing:
  - `cargo fmt -- --check`
- Run clippy on all targets:
  - `cargo clippy --all-targets --all-features`
- Strict clippy gate (recommended in CI):
  - `cargo clippy --all-targets --all-features -- -D warnings`

## Test Commands

Note: there are currently no test files in this repo yet, but use these commands as standard.

- Run all tests:
  - `cargo test`
- Run tests in a single crate target (`lib` or `bin` where applicable):
  - `cargo test --lib`
  - `cargo test --bin personal-jira-kanban`
- Run a single test by name substring:
  - `cargo test normalize_optional`
- Run an exact single test name:
  - `cargo test normalize_optional -- --exact`
- Run a namespaced test (module path):
  - `cargo test storage::tests::create_column`
- Run one integration test file:
  - `cargo test --test jira_sync`
- Run one integration test function:
  - `cargo test --test jira_sync pull_assigned_issues -- --exact`
- Show test output (`println!`) while running:
  - `cargo test -- --nocapture`

## Code Style Baseline

- Follow idiomatic Rust and keep code `cargo fmt` clean.
- Prefer small focused functions over large mixed-responsibility blocks.
- Keep modules cohesive: UI in `main.rs`, persistence in `storage.rs`, models in `models.rs`.

## Import Conventions

- Keep imports explicit and minimal.
- Prefer grouped imports per module (`use crate::...`, then external crates, then `std`).
- Use item lists for clarity (example: `use crate::models::{BoardColumn, ...};`).
- Remove unused imports promptly.

## Formatting Conventions

- Always run `cargo fmt` after edits.
- Keep lines readable; rely on rustfmt wrapping behavior.
- Avoid dense one-liners when they hide intent.
- Keep SQL blocks in multiline strings readable and indented consistently.

## Naming Conventions

- Types/structs/enums: `PascalCase`.
- Functions/methods/variables: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.
- Dioxus root component may use `App` with `#[allow(non_snake_case)]` as needed.
- Use descriptive names (`outbox_actions`, `issue_placements`) over abbreviations.

## Type and Modeling Guidelines

- Use typed structs from `models.rs` instead of ad-hoc tuples/maps for app data.
- Keep fields explicit and semantically named (`issue_key`, `jira_status`, etc.).
- Prefer `Option<T>` for truly optional values (e.g., transition/comment).
- Avoid stringly-typed state machines when enums are practical; if strings are used for DB interop, document allowed values.

## Error Handling Guidelines

- Use `anyhow::Result<T>` in app/service boundaries.
- Add context to IO/DB failures using `.context(...)` or `.with_context(...)`.
- Propagate recoverable errors instead of panicking.
- Avoid `unwrap`/`expect` in production paths; reserve for tests/prototypes only.
- Convert user-facing failures into clear UI notices without crashing the app.

## Dioxus UI Guidelines

- Keep UI state in signals and derive view data with pure helper functions.
- Prefer local-first interactions: save local state first, then queue remote sync actions.
- Ensure drag/drop handlers call `prevent_default()` where needed.
- Keep component rendering predictable; avoid hidden side effects in render logic.
- Preserve current visual language unless a deliberate redesign is requested.

## Storage and SQL Guidelines

- Use parameterized queries with `rusqlite::params!` (never string-concatenate SQL inputs).
- Keep schema bootstrap idempotent (`CREATE TABLE IF NOT EXISTS`).
- Enforce foreign keys and preserve referential integrity.
- For multi-step write operations, prefer transactional boundaries when consistency matters.
- Do not silently discard user board mappings unless explicitly intended.

## Documentation and Comments

- Write comments only for non-obvious intent or invariants.
- Keep README/plan docs updated when behavior changes.

## Agent Workflow Expectations

- Before finalizing changes, run at least:
  - `cargo fmt`
  - `cargo check`
- When tests exist, also run:
  - `cargo test`
- If a command cannot run in this environment, state it clearly and provide exact manual command.

## Cursor and Copilot Rule Files

- Checked for `.cursorrules`: not found.
- Checked for `.cursor/rules/`: not found.
- Checked for `.github/copilot-instructions.md`: not found.
- Therefore, this `AGENTS.md` is the primary agent instruction file in-repo.

## Quick Pre-PR Checklist

- Code compiles with `cargo check`.
- Formatting is clean with `cargo fmt -- --check`.
- Clippy is clean (or justified) with `cargo clippy --all-targets --all-features`.
- Tests pass (`cargo test`) when tests exist.
- New behavior documented in `README.md` or follow-up docs when relevant.
