# AGENTS.md

## Workflow
- Do not auto-commit changes to git. Always ask the user before committing or pushing.

## Build & Test
- `cargo build` — compile the project
- `cargo test` — run all tests (1 ignored test is expected)
- `cargo run` — launch the GTK application

## Architecture
- GTK 3 / Glib single-threaded UI with background tasks via `glib::spawn_future`
- SQLite database via `rusqlite` for account storage
- Secrets stored in OS keyring via `secret-service` crate
- `Arc<Mutex<Connection>>` wraps SQLite connection for thread-safe DB access

## Code Conventions
- Use `expect()` instead of `unwrap()` for user-facing errors
- Use `#[strong]` for GTK widget references in closures
- Database operations use `RepositoryError` type for consistent error handling

## UI
- Main window defined in `data/authenticator-rs.ui` (Glade XML)
- Search bar: Ctrl+f to toggle visibility and focus, Esc to clear
- TOTP codes auto-refresh via progress bar

## Gotchas
- GTK requires all UI operations on the main thread
- `RefCell` is `Send` but not `Sync` — safe for single-threaded GTK but don’t share across threads
- Keyring operations may fail if the OS keyring is locked
