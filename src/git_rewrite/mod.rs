mod config;
mod error;
mod executor;
mod time;

pub use error::GitRewriteError;

use std::path::Path;

use crate::{system::Clock, types::GitRewriteEntry};

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
    executor::collect_entries(pairs, binary_path, clock)
}

#[cfg(test)]
mod tests;
