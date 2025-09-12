# Repository Guidelines

## Project Structure & Module Organization
- Rust CLI using `clap` and `walkdir` (edition 2024).
- Source lives in `src/` (entrypoint: `src/main.rs`).
- Package metadata in `Cargo.toml`; build artifacts in `target/`.
- Add new modules as `src/<module>.rs` or `src/<mod>/mod.rs`. Place integration tests under `tests/` and fixtures under `tests/fixtures/`.

## Build, Test, and Development Commands
- Build debug: `cargo build` — compiles the crate.
- Run CLI: `cargo run -- <roots> --depth 1` (example: `cargo run -- ~/src --depth 2`).
- Release build: `cargo build --release` — optimized binary in `target/release/uncommitted`.
- Lint: `cargo clippy -- -D warnings` — enforce no warnings.
- Format: `cargo fmt --all` — apply `rustfmt` conventions.
- Tests: `cargo test` — runs unit/integration tests when present.

## Coding Style & Naming Conventions
- Use Rust defaults: 4‑space indent, 100ish column aim, no tabs.
- Naming: `snake_case` for files/functions, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Keep modules focused; prefer small functions. Avoid one-letter names.
- Run `cargo fmt` and `cargo clippy` before pushing.

## Testing Guidelines
- Prefer unit tests near code and integration tests in `tests/` (file pattern: `tests/<feature>_test.rs`).
- Cover CLI behavior with assertions on process exit and output (e.g., using `assert_cmd` and `predicates`).
- Aim for meaningful coverage of repo discovery and git-state checks; include edge cases (no upstream, untracked ignored, depth limits).

## Commit & Pull Request Guidelines
- Use Conventional Commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.
- Commits should be small and scoped; message body explains rationale.
- PRs include: clear description, linked issue (if any), runnable example (`cargo run -- …`) and before/after output where relevant.

## Security & Operational Notes
- The tool shells out to `git`; avoid passing untrusted arguments without validation. Paths are treated as data; symlinks are not followed by default.

## Agent-Specific Instructions
- Only touch files under this directory tree. Follow these conventions when adding modules/tests. Keep changes minimal and focused on the task.
