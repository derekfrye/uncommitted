use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::git::{
    ahead_of_upstream, current_branch, has_commits, has_staged, has_uncommitted, push_metrics,
    staged_metrics, uncommitted_metrics,
};
use crate::scan::find_repos;
use crate::system::{Clock, FsOps};
use crate::types::{Options, PushableEntry, ReportData, StagedEntry, UncommittedEntry};

const SEC_PER_MIN: u64 = 60;
const SEC_PER_HOUR: u64 = 60 * 60;
const SEC_PER_DAY: u64 = 60 * 60 * 24;

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
fn humanize_age(dur: Duration) -> String {
    let secs = dur.as_secs();
    if secs < SEC_PER_HOUR {
        let mins = secs as f64 / SEC_PER_MIN as f64;
        format!("{mins:.1} min")
    } else if secs < SEC_PER_DAY {
        let hrs = secs as f64 / SEC_PER_HOUR as f64;
        format!("{hrs:.1} hr")
    } else {
        let days = secs as f64 / SEC_PER_DAY as f64;
        format!("{days:.1} days")
    }
}

#[must_use]
pub fn humanize_age_public(dur: Duration) -> String {
    humanize_age(dur)
}

pub fn collect_report_data(
    opts: &Options,
    fs: &dyn FsOps,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
) -> ReportData {
    let default_root = PathBuf::from("~/src");
    let roots: Vec<PathBuf> = if opts.roots.is_empty() {
        vec![default_root]
    } else {
        opts.roots.clone()
    };

    // Build pairs of (display string from CLI input, expanded/absolute-ish path)
    let mut rooted: Vec<(String, PathBuf)> = Vec::new();
    for r in &roots {
        let root_display = r.to_string_lossy().to_string();
        let expanded = fs.expand_tilde(r);
        // Convert to an absolute-like path when relative (prefix current_dir),
        // but do not resolve symlinks.
        let root_full = if expanded.is_absolute() {
            expanded
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(expanded),
                Err(_) => expanded,
            }
        };
        rooted.push((root_display, root_full));
    }

    let mut data = ReportData::default();
    data.multi_root = rooted.len() > 1;
    let mut no_upstream = Vec::<String>::new();

    // Scan per-root so we can attribute repos to the correct root
    for (root_display, root_full) in &rooted {
        let repos = find_repos(fs, &[root_full.clone()], opts.depth, opts.debug);

        if opts.debug {
            eprintln!(
                "[debug] root_display={} root_full={} depth={} repos_found={}",
                root_display,
                root_full.display(),
                opts.depth,
                repos.len()
            );
        }

        for r in repos {
            let name = r
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            process_repo(
                &r,
                &name,
                root_display,
                root_full,
                opts,
                git,
                clock,
                &mut data,
                &mut no_upstream,
            );
        }
    }
    data
}

fn process_repo(
    r: &std::path::Path,
    name: &str,
    root_display: &str,
    root_full: &Path,
    opts: &Options,
    git: &dyn crate::git::GitRunner,
    clock: &dyn Clock,
    data: &mut ReportData,
    no_upstream: &mut Vec<String>,
) {
    let branch = current_branch(r, git).unwrap_or_else(|| "HEAD".to_string());
    if has_uncommitted(r, !opts.no_untracked, git) {
        let m = uncommitted_metrics(r, !opts.no_untracked, git);
        data.uncommitted.push(UncommittedEntry {
            repo: name.to_string(),
            branch: branch.clone(),
            lines: m.lines,
            files: m.files,
            untracked: m.untracked,
            root_display: root_display.to_string(),
            root_full: root_full.display().to_string(),
        });
    }
    if has_staged(r, git) {
        let m = staged_metrics(r, git);
        data.staged.push(StagedEntry {
            repo: name.to_string(),
            branch: branch.clone(),
            lines: m.lines,
            files: m.files,
            untracked: m.untracked,
            root_display: root_display.to_string(),
            root_full: root_full.display().to_string(),
        });
    }
    let (is_ahead, has_up) = ahead_of_upstream(r, git);
    if is_ahead {
        if let Some(pm) = push_metrics(r, git, clock) {
            data.pushable.push(PushableEntry {
                repo: name.to_string(),
                branch: branch.clone(),
                revs: pm.ahead,
                earliest_secs: pm.earliest_age.map(|d| d.as_secs()),
                latest_secs: pm.latest_age.map(|d| d.as_secs()),
                root_display: root_display.to_string(),
                root_full: root_full.display().to_string(),
            });
        } else {
            data.pushable.push(PushableEntry {
                repo: name.to_string(),
                branch: branch.clone(),
                revs: 0,
                earliest_secs: None,
                latest_secs: None,
                root_display: root_display.to_string(),
                root_full: root_full.display().to_string(),
            });
        }
    } else if !has_up && has_commits(r, git) {
        no_upstream.push(name.to_string());
    }
}

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
    for u in &data.uncommitted {
        uncommitted.push(format!(
            "{} ({} lines, {} files, {} untracked)",
            u.repo, u.lines, u.files, u.untracked
        ));
    }
    for s in &data.staged {
        staged.push(format!(
            "{} ({} lines, {} files, {} untracked)",
            s.repo, s.lines, s.files, s.untracked
        ));
    }
    for p in &data.pushable {
        let earliest = p.earliest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age(Duration::from_secs(secs)),
        );
        let latest = p.latest_secs.map_or_else(
            || "n/a".to_string(),
            |secs| humanize_age(Duration::from_secs(secs)),
        );
        let entry = if p.revs > 0 {
            format!(
                "{} ({} revs, earliest: {earliest} ago, latest: {latest} ago)",
                p.repo, p.revs
            )
        } else {
            p.repo.clone()
        };
        ahead.push(entry);
    }
    let join = |v: Vec<String>| v.join(", ");
    format!(
        "uncommitted: {}\nstaged: {}\npushable: {}",
        join(uncommitted),
        join(staged),
        join(ahead)
    )
}
