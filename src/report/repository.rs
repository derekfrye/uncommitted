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
    if has_uncommitted(repo, !opts.no_untracked, git) {
        let metrics = uncommitted_metrics(repo, !opts.no_untracked, git);
        data.uncommitted.push(UncommittedEntry {
            repo: name.to_string(),
            branch: branch.clone(),
            lines: metrics.lines,
            files: metrics.files,
            untracked: metrics.untracked,
            root_display: root.display.to_string(),
            root_full: root.full.display().to_string(),
        });
    }
    if has_staged(repo, git) {
        let metrics = staged_metrics(repo, git);
        data.staged.push(StagedEntry {
            repo: name.to_string(),
            branch: branch.clone(),
            lines: metrics.lines,
            files: metrics.files,
            untracked: metrics.untracked,
            root_display: root.display.to_string(),
            root_full: root.full.display().to_string(),
        });
    }

    let branches = list_local_branches_with_upstream(repo, git);

    if opts.refresh_remotes && !branches.is_empty() {
        let mut remotes = HashSet::<String>::new();
        for (_, upstream) in &branches {
            if let Some((remote, _rest)) = upstream.split_once('/') {
                remotes.insert(remote.to_string());
            }
        }
        for remote in remotes {
            let _ = fetch_remote(repo, git, &remote);
        }
    }

    let mut head_revs: Option<u64> = None;
    let mut head_earliest_secs: Option<u64> = None;
    let mut head_latest_secs: Option<u64> = None;

    for (branch_name, upstream) in branches {
        let is_head = branch_name == branch;
        if let Some(ahead) =
            crate::git::ahead_count_for_ref_pair(repo, git, &branch_name, &upstream)
        {
            if is_head {
                head_revs = Some(ahead);
            }
            if ahead > 0 {
                let (earliest, latest) = crate::git::commit_age_bounds_for_ref_pair(
                    repo,
                    git,
                    clock,
                    &branch_name,
                    &upstream,
                )
                .unwrap_or((None, None));
                let earliest_secs = earliest.map(|d| d.as_secs());
                let latest_secs = latest.map(|d| d.as_secs());
                if is_head {
                    head_earliest_secs = earliest_secs;
                    head_latest_secs = latest_secs;
                }
                data.pushable.push(PushableEntry {
                    repo: name.to_string(),
                    branch: branch_name.clone(),
                    revs: ahead,
                    earliest_secs,
                    latest_secs,
                    root_display: root.display.to_string(),
                    root_full: root.full.display().to_string(),
                });
            }
        }
    }

    data.repos.push(RepoSummary {
        repo: name.to_string(),
        branch,
        path: repo.to_path_buf(),
        root_display: root.display.to_string(),
        root_full: root.full.display().to_string(),
        head_revs,
        head_earliest_secs,
        head_latest_secs,
    });
}
