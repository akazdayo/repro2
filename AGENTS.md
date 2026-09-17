# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust 2024 workspace. Executable crates live under `crates/`: `proxy` serves the Axum-based Nix cache gateway, while `builder` and `round` are currently minimal service entry points. Proxy code is split by responsibility in `crates/proxy/src/` (`cache/`, `db/`, and `templates/`). SeaORM migrations live in `migration/src/`; generated database entities belong in `crates/proxy/src/db/entities/`. Design notes and experiments stay in top-level Markdown files. There is no separate asset directory.

## Build, Test, and Development Commands

Enter the reproducible toolchain with `nix develop`. Run repository commands through it when Cargo is not already available:

- `nix develop -c cargo check --workspace` — type-check every workspace crate.
- `nix develop -c cargo test --workspace` — run all unit and async tests.
- `nix develop -c cargo fmt --all -- --check` — verify Rust formatting.
- `nix develop -c cargo clippy --workspace --all-targets -- -D warnings` — reject lint warnings.
- `nix develop -c cargo run -p migration -- up` — apply migrations to `DATABASE_URL` (the dev shell defaults to `sqlite://db.sqlite?mode=rwc`).
- `nix develop -c cargo run -p proxy` — run the HTTP service on port 3000.

Use `.` or `.#attribute` for local flakes; do not use `path:.`, which can copy ignored build outputs into the Nix store.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation). Follow Rust conventions: `snake_case` for modules, functions, and tests; `PascalCase` for types and error enums; `SCREAMING_SNAKE_CASE` for constants. Keep transport, database, and cache-domain types separate, and convert errors at the HTTP boundary with typed errors such as `thiserror` enums. Do not hand-edit generated SeaORM entity files; regenerate them after schema changes.

## Testing Guidelines

Place focused tests beside their implementation in `#[cfg(test)] mod tests`. Name tests by observable behavior, for example `returns_not_found_when_upstream_narinfo_is_missing`. Prefer local Axum listeners and SeaORM mocks over public-network dependencies. No coverage threshold is configured; new behavior and regressions should receive tests. Run formatting, the relevant package tests, and the workspace check before review.

## Commit & Pull Request Guidelines

Recent commits use concise Japanese subjects describing one completed change, such as `上流キャッシュの404をそのまま返すようにした`. Keep commits scoped and avoid narrating every implementation detail. Pull requests should explain the behavior change, affected crates, schema or configuration impact, and exact validation commands. Link related issues when available; screenshots are only useful for changes with visible output.
