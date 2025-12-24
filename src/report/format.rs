use std::time::Duration;

use crate::system::{Clock, FsOps};
use crate::types::{
    GitRewriteEntry, Options, PushableEntry, StagedEntry, UncommittedEntry, UntrackedReason,
    UntrackedRepoEntry,
};

use super::collector::collect_report_data;
use super::humanize::humanize_age;

pub fn generate_report(
    opts: &Options,
    fs: &dyn FsOps,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
) -> String {
    let data = collect_report_data(opts, fs, git, clock);
    let uncommitted = change_rows(&data.uncommitted);
    let staged = change_rows(&data.staged);
    let pushable = pushable_rows(&data.pushable);
    let mut sections = vec![
        format_section("uncommitted", &uncommitted),
        format_section("staged", &staged),
        format_section("pushable", &pushable),
    ];

    if data.untracked_enabled {
        let untracked = untracked_rows(&data.untracked_repos);
        sections.push(format_section("untracked", &untracked));
    }

    if let Some(entries) = &data.git_rewrite {
        let git_rewrite = git_rewrite_rows(entries);
        sections.push(format_section("git_rewrite", &git_rewrite));
    }

    sections.join("\n")
}

fn change_rows<T>(entries: &[T]) -> Vec<String>
where
    T: ChangeEntry,
{
    entries
        .iter()
        .map(|entry| {
            format!(
                "{} ({} lines, {} files, {} untracked)",
                entry.repo(),
                entry.lines(),
                entry.files(),
                entry.untracked()
            )
        })
        .collect()
}

fn pushable_rows(entries: &[PushableEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| {
            if entry.revs > 0 {
                format!(
                    "{} ({} revs, earliest: {} ago, latest: {} ago)",
                    entry.repo,
                    entry.revs,
                    format_age(entry.earliest_secs),
                    format_age(entry.latest_secs)
                )
            } else {
                entry.repo.clone()
            }
        })
        .collect()
}

fn untracked_rows(entries: &[UntrackedRepoEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| {
            let reason = match entry.reason {
                UntrackedReason::Ignored => "ignored",
                UntrackedReason::MissingConfig => "missing-config",
                UntrackedReason::MissingRepo => "missing",
            };
            format!(
                "{}:{} ({reason}, branch: {}, revs: {}, earliest: {} ago, latest: {} ago)",
                entry.root_display,
                entry.repo,
                entry.branch,
                entry
                    .revs
                    .map_or_else(|| "n/a".to_string(), |v| v.to_string()),
                format_age(entry.earliest_secs),
                format_age(entry.latest_secs)
            )
        })
        .collect()
}

fn git_rewrite_rows(entries: &[GitRewriteEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| {
            format!(
                "{}->{} (commits: {}, earliest: {} ago, latest: {} ago)",
                entry.source_repo,
                entry.target_repo,
                entry.commits,
                format_age(entry.earliest_secs),
                format_age(entry.latest_secs)
            )
        })
        .collect()
}

fn format_section(label: &str, rows: &[String]) -> String {
    format!("{label}: {}", rows.join(", "))
}

fn format_age(value: Option<u64>) -> String {
    value.map_or_else(
        || "n/a".to_string(),
        |secs| humanize_age(Duration::from_secs(secs)),
    )
}

trait ChangeEntry {
    fn repo(&self) -> &str;
    fn lines(&self) -> u64;
    fn files(&self) -> u64;
    fn untracked(&self) -> u64;
}

impl ChangeEntry for UncommittedEntry {
    fn repo(&self) -> &str {
        &self.repo
    }
    fn lines(&self) -> u64 {
        self.lines
    }
    fn files(&self) -> u64 {
        self.files
    }
    fn untracked(&self) -> u64 {
        self.untracked
    }
}

impl ChangeEntry for StagedEntry {
    fn repo(&self) -> &str {
        &self.repo
    }
    fn lines(&self) -> u64 {
        self.lines
    }
    fn files(&self) -> u64 {
        self.files
    }
    fn untracked(&self) -> u64 {
        self.untracked
    }
}
