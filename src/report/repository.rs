use std::collections::HashSet;
use std::path::Path;

use crate::git::{
    current_branch, fetch_remote, has_staged, has_uncommitted, list_local_branches_with_upstream,
    staged_metrics, uncommitted_metrics, upstream_remote_url,
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

struct RepoContext<'a> {
    repo: &'a Path,
    name: &'a str,
    branch: &'a str,
    root_display: &'a str,
    root_full: &'a str,
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
    let ctx = RepoContext {
        repo,
        name,
        branch: &branch,
        root_display: &root_display,
        root_full: &root_full,
    };

    record_uncommitted(&ctx, opts, git, data);
    record_staged(&ctx, git, data);

    let branches = list_local_branches_with_upstream(repo, git);
    refresh_remotes(repo, opts, git, &branches);

    let (head_revs, head_earliest_secs, head_latest_secs) =
        record_pushables(&ctx, branches, git, clock, data);

    add_repo_summary(&ctx, head_revs, head_earliest_secs, head_latest_secs, data);
}

fn record_uncommitted(
    ctx: &RepoContext<'_>,
    opts: &Options,
    git: &dyn crate::git::GitRunner,
    data: &mut ReportData,
) {
    if !has_uncommitted(ctx.repo, !opts.no_untracked, git) {
        return;
    }
    let metrics = uncommitted_metrics(ctx.repo, !opts.no_untracked, git);
    let upstream = upstream_remote_url(ctx.repo, git)
        .map(|url| normalize_upstream_url(&url))
        .filter(|url| !url.is_empty());
    data.uncommitted.push(UncommittedEntry {
        repo: ctx.name.to_string(),
        branch: ctx.branch.to_string(),
        upstream,
        lines: metrics.lines,
        files: metrics.files,
        untracked: metrics.untracked,
        root_display: ctx.root_display.to_string(),
        root_full: ctx.root_full.to_string(),
    });
}

fn record_staged(ctx: &RepoContext<'_>, git: &dyn crate::git::GitRunner, data: &mut ReportData) {
    if !has_staged(ctx.repo, git) {
        return;
    }
    let metrics = staged_metrics(ctx.repo, git);
    data.staged.push(StagedEntry {
        repo: ctx.name.to_string(),
        branch: ctx.branch.to_string(),
        lines: metrics.lines,
        files: metrics.files,
        untracked: metrics.untracked,
        root_display: ctx.root_display.to_string(),
        root_full: ctx.root_full.to_string(),
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

fn record_pushables(
    ctx: &RepoContext<'_>,
    branches: Vec<(String, String)>,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
    data: &mut ReportData,
) -> (Option<u64>, Option<u64>, Option<u64>) {
    let mut head_revs = None;
    let mut head_earliest_secs = None;
    let mut head_latest_secs = None;

    for (branch_name, upstream) in branches {
        let Some(ahead) =
            crate::git::ahead_count_for_ref_pair(ctx.repo, git, &branch_name, &upstream)
        else {
            continue;
        };
        if branch_name == ctx.branch {
            head_revs = Some(ahead);
        }
        if ahead == 0 {
            continue;
        }
        let (earliest, latest) = crate::git::commit_age_bounds_for_ref_pair(
            ctx.repo,
            git,
            clock,
            &branch_name,
            &upstream,
        )
        .unwrap_or((None, None));
        let earliest_secs = earliest.map(|d| d.as_secs());
        let latest_secs = latest.map(|d| d.as_secs());
        if branch_name == ctx.branch {
            head_earliest_secs = earliest_secs;
            head_latest_secs = latest_secs;
        }
        data.pushable.push(PushableEntry {
            repo: ctx.name.to_string(),
            branch: branch_name.clone(),
            revs: ahead,
            earliest_secs,
            latest_secs,
            root_display: ctx.root_display.to_string(),
            root_full: ctx.root_full.to_string(),
        });
    }

    (head_revs, head_earliest_secs, head_latest_secs)
}

fn add_repo_summary(
    ctx: &RepoContext<'_>,
    head_revs: Option<u64>,
    head_earliest_secs: Option<u64>,
    head_latest_secs: Option<u64>,
    data: &mut ReportData,
) {
    data.repos.push(RepoSummary {
        repo: ctx.name.to_string(),
        branch: ctx.branch.to_string(),
        path: ctx.repo.to_path_buf(),
        root_display: ctx.root_display.to_string(),
        root_full: ctx.root_full.to_string(),
        head_revs,
        head_earliest_secs,
        head_latest_secs,
    });
}

fn normalize_upstream_url(url: &str) -> String {
    let trimmed = url.trim();
    if let Some(rest) = trimmed.strip_prefix("git@github.com:") {
        rest.to_string()
    } else {
        trimmed.to_string()
    }
}
