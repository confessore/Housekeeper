# Housekeeper Rust Rules

- Keep command handlers thin; put domain logic in services and SQL in repositories.
- One command, repository, model, or focused service responsibility per file.
- Prefer `pub(crate)` and small interfaces; avoid god modules.
- Do not commit secrets, and never drop tables or databases.
- Run `cargo fmt`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings` before handoff.
