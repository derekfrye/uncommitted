use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{RepoSummary, UntrackedReason, UntrackedRepoEntry};

use super::{
    GitRewriteError,
    config::{self, OtherReposSummary},
};

pub fn collect_git_rewrite_untracked(
    config_path: &Path,
    repos: &[RepoSummary],
) -> Result<Vec<UntrackedRepoEntry>, GitRewriteError> {
    let config = config::load_config(config_path)?;
    let build = config::build_other_repos_summary(&config)?;
    Ok(build_untracked_entries(repos, &build))
}

fn build_untracked_entries(
    repos: &[RepoSummary],
    build: &OtherReposSummary,
) -> Vec<UntrackedRepoEntry> {
    let tracked_set: HashSet<PathBuf> = build
        .tracked_endpoints
        .iter()
        .map(|endpoint| normalize_path(&endpoint.path))
        .collect();
    let ignored_set: HashSet<PathBuf> = build
        .ignored_paths
        .iter()
        .map(|p| normalize_path(p))
        .collect();

    let mut entries = Vec::new();
    let mut seen_paths = HashSet::new();
    for repo in repos {
        let repo_path = normalize_path(&repo.path);
        seen_paths.insert(repo_path.clone());
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

    let mut recorded_missing = HashSet::new();
    for endpoint in &build.tracked_endpoints {
        let endpoint_path = normalize_path(&endpoint.path);
        if seen_paths.contains(&endpoint_path) || !recorded_missing.insert(endpoint_path.clone()) {
            continue;
        }
        let display = endpoint.path.display().to_string();
        entries.push(UntrackedRepoEntry {
            repo: display.clone(),
            branch: endpoint.branch.clone(),
            root_display: display.clone(),
            root_full: endpoint_path.display().to_string(),
            revs: None,
            earliest_secs: None,
            latest_secs: None,
            reason: UntrackedReason::MissingRepo,
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
