use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use crate::system::Clock;

use super::GitRunner;

#[must_use]
pub(crate) fn current_branch(repo: &Path, git: &dyn GitRunner) -> Option<String> {
    let out = git
        .run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

#[must_use]
pub(crate) fn list_local_branches_with_upstream(
    repo: &Path,
    git: &dyn GitRunner,
) -> Vec<(String, String)> {
    let out = git
        .run_git(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname:short) %(upstream:short)",
                "refs/heads",
            ],
        )
        .ok();
    let mut branches = Vec::new();
    if let Some(out) = out
        && out.status.success()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let mut parts = line.split_whitespace();
            if let Some(branch) = parts.next()
                && let Some(upstream) = parts.next()
                && !upstream.trim().is_empty()
            {
                branches.push((branch.to_string(), upstream.to_string()));
            }
        }
    }
    branches
}

pub(crate) fn fetch_remote(repo: &Path, git: &dyn GitRunner, remote: &str) -> bool {
    git.run_git(repo, &["fetch", "--prune", "--no-tags", remote])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn ahead_count_for_ref_pair(
    repo: &Path,
    git: &dyn GitRunner,
    branch: &str,
    upstream: &str,
) -> Option<u64> {
    let count = git
        .run_git(
            repo,
            &["rev-list", "--count", &format!("{upstream}..{branch}")],
        )
        .ok()?;
    if !count.status.success() {
        return None;
    }
    String::from_utf8_lossy(&count.stdout)
        .trim()
        .parse::<u64>()
        .ok()
}

pub(crate) fn commit_age_bounds_for_ref_pair(
    repo: &Path,
    git: &dyn GitRunner,
    clock: &dyn Clock,
    branch: &str,
    upstream: &str,
) -> Option<(Option<Duration>, Option<Duration>)> {
    let log = git
        .run_git(
            repo,
            &["log", "--format=%ct", &format!("{upstream}..{branch}")],
        )
        .ok()?;
    if !log.status.success() {
        return None;
    }
    let now_secs = clock
        .now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    let mut min_age: Option<Duration> = None;
    let mut max_age: Option<Duration> = None;
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if let Ok(ts) = line.trim().parse::<u64>() {
            let age = Duration::from_secs(now_secs.saturating_sub(ts));
            min_age = Some(match min_age {
                Some(cur) if cur < age => cur,
                _ => age,
            });
            max_age = Some(match max_age {
                Some(cur) if cur > age => cur,
                _ => age,
            });
        }
    }
    Some((max_age, min_age))
}
