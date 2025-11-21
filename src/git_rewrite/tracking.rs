use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{RepoSummary, UntrackedReason, UntrackedRepoEntry};

use super::{
    GitRewriteError,
    config::{self, PairBuildOutput},
};

pub fn collect_git_rewrite_untracked(
    config_path: &Path,
    repos: &[RepoSummary],
) -> Result<Vec<UntrackedRepoEntry>, GitRewriteError> {
    let config = config::load_config(config_path)?;
    let build = config::build_pairs_with_paths(&config)?;
    Ok(build_untracked_entries(repos, &build))
}

fn build_untracked_entries(
    repos: &[RepoSummary],
    build: &PairBuildOutput,
) -> Vec<UntrackedRepoEntry> {
    let tracked_set: HashSet<PathBuf> = build
        .tracked_paths
        .iter()
        .map(|p| normalize_path(p))
        .collect();
    let ignored_set: HashSet<PathBuf> = build
        .ignored_paths
        .iter()
        .map(|p| normalize_path(p))
        .collect();

    let mut entries = Vec::new();
    for repo in repos {
        let repo_path = normalize_path(&repo.path);
        if tracked_set.contains(&repo_path) {
            continue;
        }
        let reason = if ignored_set.contains(&repo_path) {
            UntrackedReason::Ignored
        } else {
            UntrackedReason::MissingConfig
        };

        entries.push(UntrackedRepoEntry {
            repo: repo.repo.clone(),
            branch: repo.branch.clone(),
            root_display: repo.root_display.clone(),
            root_full: repo.root_full.clone(),
            revs: repo.head_revs,
            earliest_secs: repo.head_earliest_secs,
            latest_secs: repo.head_latest_secs,
            reason,
        });
    }

    entries.sort_by(|a, b| {
        (&a.root_display, &a.repo, &a.branch).cmp(&(&b.root_display, &b.repo, &b.branch))
    });
    entries
}

fn normalize_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
