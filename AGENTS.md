# Repository Guidelines

## Project Structure & Module Organization
This repository is a Cargo workspace with four crates:
- `client/`: Bevy-based game client.
  - Entry and bootstrap: `client/src/main.rs`, `client/src/app/bootstrap.rs`.
  - Composition: `client/src/composition/*` (`client_runtime`, `character_viewer_runtime`, `object_viewer_runtime`).
  - Gameplay layers: `client/src/gameplay/{controllers,scenes,runtime,systems}`.
  - Runtime data/systems: `client/src/scene_runtime/*`.
  - Bins: `client`, `character_viewer`, `object_viewer`.
- `protocol/`: shared packet types, serializers, deserializers, tests, benchmarks (`protocol/src`, `protocol/tests`, `protocol/benches`).
- `server/`: Actix Web connect server with handlers, middleware, monitoring, runtime, and DB layers (`server/src/*`, `server/config/servers.toml`, `server/tests`).
- `common/`: shared world/domain structures used across crates.

Supporting directories:
- `assets/`: game data/static resources (`assets/data`, `assets/shaders`, `assets/wallpapers`) and reports in `assets/reports`.
- `docs/`: architecture and technical notes (`../docs/client-architecture.md`, `../docs/ASSET_CONVERSION.md`).
- `docker/`: local infrastructure for MongoDB and Mongo Express.

## Build, Test, and Development Commands
Run from `rust/` root unless noted.
- `cargo check --workspace`: fast compile checks for all crates.
- `cargo build --workspace`: build all crates.
- `cargo fmt --all`: format all crates.
- `cargo clippy --workspace --all-targets -- -D warnings`: strict linting.
- `cargo test --workspace`: run all unit/integration/doc tests.
- `cargo run -p server`: run connect server.
- `cargo run -p server --bin sim-client -- --help`: run simulation client helper.
- `cd docker && docker-compose up -d`: start MongoDB + Mongo Express.

Client runtime gate:
```bash
cargo check -p client --bin client --bin object_viewer --bin character_viewer
for bin in client object_viewer character_viewer; do
  timeout 20s cargo run -p client --bin "$bin" || test $? -eq 124
done
```

## Coding Style & Naming Conventions
- Use Rust defaults: 4-space indentation and `rustfmt` formatting.
- Follow Rust naming idioms: modules/files in `snake_case`, types/traits in `UpperCamelCase`, constants in `UPPER_SNAKE_CASE`.
- Keep crate boundaries clear:
  - protocol/wire logic in `protocol`
  - HTTP/runtime/session logic in `server`
  - rendering/gameplay/ui in `client`

## Testing Guidelines
- Prefer deterministic tests close to behavior.
- Client structural/contract tests live in `client/tests/`.
- Server tests use `actix_web::test` for endpoints and unit tests for runtime/core pieces.
- Name server test files `*_tests.rs` in `server/tests/`.
- Add/adjust protocol roundtrip tests when packet structures change.

## Commit & Pull Request Guidelines
- Keep commits concise and imperative; prefer scoped subjects (example: `client: move scenes to gameplay layer`).
- PRs should include:
  - What changed and why.
  - Affected crate(s): `client`, `protocol`, `server`, `common`.
  - Commands run (`cargo fmt`, `cargo clippy`, `cargo test`).
  - Config/API updates when relevant.

## Security & Configuration Tips
- Do not commit secrets or `.env` files.
- Keep `MONGODB_URI` aligned with `docker/.env` in local development.
- If path-related issues occur, run commands from `rust/` root.
