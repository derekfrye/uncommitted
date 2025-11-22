use std::collections::HashSet;
use std::path::Path;

use crate::git::{
    current_branch, fetch_remote, has_staged, has_uncommitted, list_local_branches_with_upstream,
    staged_metrics, uncommitted_metrics,
};
use crate::system::Clock;
use crate::types::{
    Options, PushableEntry, RepoSummary, ReportData, StagedEntry, UncommittedEntry,
};

#[derive(Copy, Clone)]
pub(crate) struct RootContext<'a> {
    pub(crate) display: &'a str,
    pub(crate) full: &'a Path,
}

pub(crate) fn process_repo(
    repo: &Path,
    name: &str,
    root: RootContext<'_>,
    opts: &Options,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
    data: &mut ReportData,
) {
    let branch = current_branch(repo, git).unwrap_or_else(|| "HEAD".to_string());
    let root_display = root.display.to_string();
    let root_full = root.full.display().to_string();

    record_uncommitted(
        repo,
        name,
        &branch,
        opts,
        git,
        &root_display,
        &root_full,
        data,
    );
    record_staged(repo, name, &branch, git, &root_display, &root_full, data);

    let branches = list_local_branches_with_upstream(repo, git);
    refresh_remotes(repo, opts, git, &branches);

    let (head_revs, head_earliest_secs, head_latest_secs) = record_pushables(
        repo,
        name,
        &branch,
        branches,
        git,
        clock,
        &root_display,
        &root_full,
        data,
    );

    add_repo_summary(
        repo,
        name,
        branch,
        head_revs,
        head_earliest_secs,
        head_latest_secs,
        &root_display,
        &root_full,
        data,
    );
}

fn record_uncommitted(
    repo: &Path,
    name: &str,
    branch: &str,
    opts: &Options,
    git: &dyn crate::git::GitRunner,
    root_display: &str,
    root_full: &str,
    data: &mut ReportData,
) {
    if !has_uncommitted(repo, !opts.no_untracked, git) {
        return;
    }
    let metrics = uncommitted_metrics(repo, !opts.no_untracked, git);
    data.uncommitted.push(UncommittedEntry {
        repo: name.to_string(),
        branch: branch.to_string(),
        lines: metrics.lines,
        files: metrics.files,
        untracked: metrics.untracked,
        root_display: root_display.to_string(),
        root_full: root_full.to_string(),
    });
}

fn record_staged(
    repo: &Path,
    name: &str,
    branch: &str,
    git: &dyn crate::git::GitRunner,
    root_display: &str,
    root_full: &str,
    data: &mut ReportData,
) {
    if !has_staged(repo, git) {
        return;
    }
    let metrics = staged_metrics(repo, git);
    data.staged.push(StagedEntry {
        repo: name.to_string(),
        branch: branch.to_string(),
        lines: metrics.lines,
        files: metrics.files,
        untracked: metrics.untracked,
        root_display: root_display.to_string(),
        root_full: root_full.to_string(),
    });
}

fn refresh_remotes(
    repo: &Path,
    opts: &Options,
    git: &dyn crate::git::GitRunner,
    branches: &[(String, String)],
) {
    if !opts.refresh_remotes || branches.is_empty() {
        return;
    }
    let mut remotes = HashSet::new();
    for (_, upstream) in branches {
        if let Some((remote, _rest)) = upstream.split_once('/') {
            remotes.insert(remote.to_string());
        }
    }
    for remote in remotes {
        let _ = fetch_remote(repo, git, &remote);
    }
}

#[allow(clippy::too_many_arguments)]
fn record_pushables(
    repo: &Path,
    name: &str,
    head_branch: &str,
    branches: Vec<(String, String)>,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
    root_display: &str,
    root_full: &str,
    data: &mut ReportData,
) -> (Option<u64>, Option<u64>, Option<u64>) {
    let mut head_revs = None;
    let mut head_earliest_secs = None;
    let mut head_latest_secs = None;

    for (branch_name, upstream) in branches {
        let Some(ahead) = crate::git::ahead_count_for_ref_pair(repo, git, &branch_name, &upstream)
        else {
            continue;
        };
        if branch_name == head_branch {
            head_revs = Some(ahead);
        }
        if ahead == 0 {
            continue;
        }
        let (earliest, latest) =
            crate::git::commit_age_bounds_for_ref_pair(repo, git, clock, &branch_name, &upstream)
                .unwrap_or((None, None));
        let earliest_secs = earliest.map(|d| d.as_secs());
        let latest_secs = latest.map(|d| d.as_secs());
        if branch_name == head_branch {
            head_earliest_secs = earliest_secs;
            head_latest_secs = latest_secs;
        }
        data.pushable.push(PushableEntry {
            repo: name.to_string(),
            branch: branch_name.clone(),
            revs: ahead,
            earliest_secs,
            latest_secs,
            root_display: root_display.to_string(),
            root_full: root_full.to_string(),
        });
    }

    (head_revs, head_earliest_secs, head_latest_secs)
}

fn add_repo_summary(
    repo: &Path,
    name: &str,
    branch: String,
    head_revs: Option<u64>,
    head_earliest_secs: Option<u64>,
    head_latest_secs: Option<u64>,
    root_display: &str,
    root_full: &str,
    data: &mut ReportData,
) {
    data.repos.push(RepoSummary {
        repo: name.to_string(),
        branch,
        path: repo.to_path_buf(),
        root_display: root_display.to_string(),
        root_full: root_full.to_string(),
        head_revs,
        head_earliest_secs,
        head_latest_secs,
    });
}
