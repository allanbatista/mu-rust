# Repository Guidelines

## Project Structure & Module Organization
This repository is a Cargo workspace with three crates:
- `client/`: Bevy-based game client (`client/src/scenes`, `client/src/world`).
- `protocol/`: shared packet types, serializers, deserializers, tests, and benchmarks (`protocol/src`, `protocol/tests`, `protocol/benches`).
- `server/`: Actix Web connect server with configuration, handlers, middleware, monitoring, and DB layers (`server/src/*`, `server/config/servers.toml`, `server/tests`).

Large game resources live in `assets/`. Local Docker infra for MongoDB lives in `docker/`.

## Build, Test, and Development Commands
Run from workspace root unless noted.
- `cargo check --workspace`: fast compile checks for all crates.
- `cargo build --workspace`: build all crates.
- `cargo test --workspace`: run all unit/integration tests.
- `cargo run --manifest-path server/Cargo.toml`: run the connect server with workspace-relative config paths.
- `cargo test --manifest-path server/Cargo.toml`: run server-focused tests.
- `cargo test --manifest-path protocol/Cargo.toml`: run protocol roundtrip tests.
- `cargo bench --manifest-path protocol/Cargo.toml`: run Criterion serialization benchmarks.
- `cd docker && docker-compose up -d`: start MongoDB + Mongo Express for local server development.

## Coding Style & Naming Conventions
- Use Rust defaults: 4-space indentation and `rustfmt` formatting.
- Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` before opening a PR.
- Follow Rust naming idioms: modules/files in `snake_case`, types/traits in `UpperCamelCase`, constants in `UPPER_SNAKE_CASE`.
- Keep crate boundaries clear: protocol logic in `protocol`, HTTP/domain logic in `server`, rendering/gameplay in `client`.

## Testing Guidelines
- Prefer small, deterministic tests close to behavior.
- Server tests use `actix_web::test` for endpoint coverage and `#[test]` for core logic.
- Name test files `*_tests.rs` in `server/tests/`; use descriptive test function names (e.g., `test_rate_limit_blocks_over_limit`).
- Add/adjust protocol roundtrip tests when packet structures change.

## Commit & Pull Request Guidelines
Current history is minimal (`start project`), so keep commits concise and imperative. Prefer scoped subjects, for example: `server: add heartbeat validation`.

For PRs, include:
- What changed and why.
- Affected crate(s): `client`, `protocol`, `server`.
- Commands run (`cargo fmt`, `cargo clippy`, `cargo test`).
- Config/API updates (example request/response when endpoints change).

## Security & Configuration Tips
- Do not commit secrets or `.env` files.
- Keep `MONGODB_URI` aligned with `docker/.env` when using local containers.
- If server config paths fail, run from workspace root with `--manifest-path server/Cargo.toml`.
