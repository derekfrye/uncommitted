use crate::{ReportData, types::UntrackedReason};
use serde_json::json;

#[must_use]
pub fn to_json(data: &ReportData) -> String {
    let uncommitted = data
        .uncommitted
        .iter()
        .map(|e| {
            json!({
                "repo": &e.repo,
                "branch": &e.branch,
                "upstream": &e.upstream,
                "lines": e.lines,
                "files": e.files,
                "untracked": e.untracked,
                "root": &e.root_full,
            })
        })
        .collect::<Vec<_>>();
    let staged = data
        .staged
        .iter()
        .map(|e| {
            json!({
                "repo": &e.repo,
                "branch": &e.branch,
                "lines": e.lines,
                "files": e.files,
                "untracked": e.untracked,
                "root": &e.root_full,
            })
        })
        .collect::<Vec<_>>();
    let pushable = data
        .pushable
        .iter()
        .map(|e| {
            json!({
                "repo": &e.repo,
                "branch": &e.branch,
                "revs": e.revs,
                "earliest_secs": e.earliest_secs,
                "latest_secs": e.latest_secs,
                "root": &e.root_full,
            })
        })
        .collect::<Vec<_>>();
    let untracked_repos = if data.untracked_enabled {
        Some(
            data.untracked_repos
                .iter()
                .map(|entry| {
                    let reason = match entry.reason {
                        UntrackedReason::Ignored => "ignored",
                        UntrackedReason::MissingConfig => "missing_config",
                        UntrackedReason::MissingRepo => "missing",
                    };
                    json!({
                        "repo": &entry.repo,
                        "branch": &entry.branch,
                        "root": &entry.root_full,
                        "root_display": &entry.root_display,
                        "revs": entry.revs,
                        "earliest_secs": entry.earliest_secs,
                        "latest_secs": entry.latest_secs,
                        "reason": reason,
                    })
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let git_rewrite = data.git_rewrite.as_ref().map(|entries| {
        entries
            .iter()
            .map(|entry| {
                json!({
                    "source_repo": &entry.source_repo,
                    "source_branch": &entry.source_branch,
                    "source_path": &entry.source_path,
                    "target_repo": &entry.target_repo,
                    "target_branch": &entry.target_branch,
                    "target_path": &entry.target_path,
                    "commits": entry.commits,
                    "earliest_secs": entry.earliest_secs,
                    "latest_secs": entry.latest_secs,
                })
            })
            .collect::<Vec<_>>()
    });

    json!({
        "uncommitted": uncommitted,
        "staged": staged,
        "pushable": pushable,
        "untracked_repos": untracked_repos,
        "git_rewrite": git_rewrite,
    })
    .to_string()
}
