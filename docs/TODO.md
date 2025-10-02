## Refactor Large Modules

### src/output/tab.rs ✅
- Create directory `src/output/tab/` with `mod.rs` providing `format_tab` and `TabStyle`.
- Extract renderers into dedicated files: `uncommitted.rs`, `staged.rs`, `pushable.rs`, `git_rewrite.rs`.
- Move styling helpers (`apply_style`, `apply_title_line`) into `style.rs` reused by renderers.

### src/git.rs ✅
- Split into `src/git/` directory with `mod.rs` exposing the public surface.
- `runner.rs` for `GitRunner` trait and `DefaultGitRunner`.
- `refs.rs` for branch/upstream helpers and ahead/age computations.
- `metrics.rs` for diff parsing, metrics structs, and staged/uncommitted checks.

### src/git_rewrite.rs
- Reorganize as `src/git_rewrite/` with `mod.rs` containing `collect_git_rewrite_entries`.
- `error.rs` for `GitRewriteError` definition and impl.
- `config.rs` for config structs plus `build_pairs`.
- `executor.rs` for invoking the helper binary and building `GitRewriteEntry` values.
- `time.rs` for datetime parsing and bound computations.
- Keep or relocate tests under the new module tree.

### src/report.rs
- Move to `src/report/` with `mod.rs` re-exporting the public API.
- `collector.rs` for root expansion and repository discovery.
- `repository.rs` for processing a single repo into metrics and pushable entries.
- `format.rs` for human-readable report assembly.
- `humanize.rs` for duration formatting shared by report and tab output.

### Execution Notes
- Refactor one module directory at a time: output/tab → git → git_rewrite → report.
- After each migration run `cargo fmt`, `cargo clippy -- -D warnings -D clippy::pedantic`, and `cargo test`.
- Update imports and visibility as code moves; validate behavior incrementally.
