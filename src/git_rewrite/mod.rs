mod config;
mod error;
mod executor;
mod time;
mod tracking;
mod worker;

pub use error::GitRewriteError;
pub use tracking::collect_git_rewrite_untracked;

use std::path::Path;

use crate::{system::Clock, types::GitRewriteEntry};
use clap::{CommandFactory, Parser};
use config::RepoSpec;

/// Collect rows for the git rewrite table by executing the configured `git_rewrite` binary.
///
/// # Errors
/// Returns an error when the configuration cannot be read, parsed, or is invalid, when the helper
/// binary cannot be launched or fails, or when its JSON output is malformed.
pub fn collect_git_rewrite_entries(
    config_path: &Path,
    binary_path: &Path,
    clock: &dyn Clock,
) -> Result<Vec<GitRewriteEntry>, GitRewriteError> {
    let config = config::load_config(config_path)?;
    let pairs = config::build_pairs(&config)?;
    executor::collect_entries(&pairs, binary_path, clock)
}

#[derive(Parser, Debug)]
#[command(name = "toml", about = "Fields under [[repo]] in git_rewrite TOML.")]
pub struct GitRewriteTomlHelp {
    #[command(flatten)]
    repo: RepoSpec,
}

#[must_use]
pub fn git_rewrite_toml_help() -> String {
    let mut cmd = GitRewriteTomlHelp::command();
    cmd.set_bin_name("uncommitted toml");
    cmd.render_help().to_string()
}

#[cfg(test)]
mod tests;
