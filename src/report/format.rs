use std::time::Duration;

use crate::system::{Clock, FsOps};
use crate::types::Options;

use super::collector::collect_report_data;
use super::humanize::humanize_age;

pub fn generate_report(
    opts: &Options,
    fs: &dyn FsOps,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
) -> String {
    let data = collect_report_data(opts, fs, git, clock);
    let mut uncommitted = Vec::<String>::new();
    let mut staged = Vec::<String>::new();
    let mut ahead = Vec::<String>::new();

    for entry in &data.uncommitted {
        uncommitted.push(format!(
            "{} ({} lines, {} files, {} untracked)",
            entry.repo, entry.lines, entry.files, entry.untracked
        ));
    }

    for entry in &data.staged {
        staged.push(format!(
            "{} ({} lines, {} files, {} untracked)",
            entry.repo, entry.lines, entry.files, entry.untracked
        ));
    }

    for entry in &data.pushable {
        let earliest = entry.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age(Duration::from_secs(secs)),
        );
        let latest = entry.latest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age(Duration::from_secs(secs)),
        );
        let description = if entry.revs > 0 {
            format!(
                "{} ({} revs, earliest: {earliest} ago, latest: {latest} ago)",
                entry.repo, entry.revs
            )
        } else {
            entry.repo.clone()
        };
        ahead.push(description);
    }

    let join = |values: Vec<String>| values.join(", ");
    let mut sections = vec![
        format!("uncommitted: {}", join(uncommitted)),
        format!("staged: {}", join(staged)),
        format!("pushable: {}", join(ahead)),
    ];

    if let Some(entries) = &data.git_rewrite {
        let mut rewrite = Vec::new();
        for entry in entries {
            let earliest = entry.earliest_secs.map_or_else(
                || "n/a".to_string(),
                |secs| humanize_age(Duration::from_secs(secs)),
            );
            let latest = entry.latest_secs.map_or_else(
                || "n/a".to_string(),
                |secs| humanize_age(Duration::from_secs(secs)),
            );
            rewrite.push(format!(
                "{}->{} (commits: {}, earliest: {earliest} ago, latest: {latest} ago)",
                entry.source_repo, entry.target_repo, entry.commits
            ));
        }
        sections.push(format!("git_rewrite: {}", join(rewrite)));
    }

    sections.join("\n")
}
