use std::path::Path;

use super::GitRunner;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ChangeMetrics {
    pub(crate) lines: u64,
    pub(crate) files: u64,
    pub(crate) untracked: u64,
}

pub(crate) fn uncommitted_metrics(
    repo: &Path,
    include_untracked: bool,
    git: &dyn GitRunner,
) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = git.run_git(
        repo,
        &["diff", "--numstat", "--ignore-submodules", "--", "."],
    ) {
        let text = String::from_utf8_lossy(&out.stdout);
        let (lines, files) = parse_numstat(&text);
        metrics.lines = lines;
        metrics.files = files;
    }
    if include_untracked
        && let Ok(out) = git.run_git(repo, &["ls-files", "--others", "--exclude-standard"])
    {
        metrics.untracked = count_lines(&String::from_utf8_lossy(&out.stdout));
    }
    metrics
}

pub(crate) fn staged_metrics(repo: &Path, git: &dyn GitRunner) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = git.run_git(
        repo,
        &[
            "diff",
            "--cached",
            "--numstat",
            "--ignore-submodules",
            "--",
            ".",
        ],
    ) {
        let text = String::from_utf8_lossy(&out.stdout);
        let (lines, files) = parse_numstat(&text);
        metrics.lines = lines;
        metrics.files = files;
    }
    if let Ok(out) = git.run_git(repo, &["ls-files", "--others", "--exclude-standard"]) {
        metrics.untracked = count_lines(&String::from_utf8_lossy(&out.stdout));
    }
    metrics
}

pub(crate) fn has_uncommitted(repo: &Path, include_untracked: bool, git: &dyn GitRunner) -> bool {
    if let Ok(out) = git.run_git(repo, &["diff", "--quiet", "--ignore-submodules", "--", "."]) {
        if !out.status.success() {
            return true;
        }
    } else {
        return false;
    }

    if include_untracked
        && let Ok(out) = git.run_git(repo, &["ls-files", "--others", "--exclude-standard"])
        && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
    {
        return true;
    }
    false
}

pub(crate) fn has_staged(repo: &Path, git: &dyn GitRunner) -> bool {
    if let Ok(out) = git.run_git(
        repo,
        &[
            "diff",
            "--cached",
            "--quiet",
            "--ignore-submodules",
            "--",
            ".",
        ],
    ) {
        return !out.status.success();
    }
    false
}

fn parse_numstat(s: &str) -> (u64, u64) {
    let mut total_lines = 0u64;
    let mut files = 0u64;
    for line in s.lines() {
        let mut parts = line.split('\t');
        let added = parts.next();
        let deleted = parts.next();
        if added.is_some() && deleted.is_some() {
            files = files.saturating_add(1);
            let add_num = added.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            let del_num = deleted.and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            total_lines = total_lines.saturating_add(add_num.saturating_add(del_num));
        }
    }
    (total_lines, files)
}

fn count_lines(s: &str) -> u64 {
    s.lines().filter(|l| !l.trim().is_empty()).count() as u64
}
