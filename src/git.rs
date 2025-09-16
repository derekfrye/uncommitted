#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, UNIX_EPOCH};

use crate::system::Clock;

pub trait GitRunner {
    fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<Output>;
}

pub struct DefaultGitRunner;
impl GitRunner for DefaultGitRunner {
    fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ChangeMetrics {
    pub(crate) lines: u64,
    pub(crate) files: u64,
    pub(crate) untracked: u64,
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

pub(crate) fn uncommitted_metrics(repo: &Path, include_untracked: bool, git: &dyn GitRunner) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = git.run_git(repo, &["diff", "--numstat", "--ignore-submodules", "--", "."]) {
        let text = String::from_utf8_lossy(&out.stdout);
        let (lines, files) = parse_numstat(&text);
        metrics.lines = lines;
        metrics.files = files;
    }
    if include_untracked {
        if let Ok(out) = git.run_git(repo, &["ls-files", "--others", "--exclude-standard"]) {
            metrics.untracked = count_lines(&String::from_utf8_lossy(&out.stdout));
        }
    }
    metrics
}

pub(crate) fn staged_metrics(repo: &Path, git: &dyn GitRunner) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = git.run_git(repo, &["diff", "--cached", "--numstat", "--ignore-submodules", "--", "."]) {
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
    if let Ok(out) = git.run_git(repo, &["diff", "--cached", "--quiet", "--ignore-submodules", "--", "."]) {
        return !out.status.success();
    }
    false
}

pub(crate) fn ahead_of_upstream(repo: &Path, git: &dyn GitRunner) -> (bool, bool) {
    if let Ok(u) = git.run_git(repo, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"]) {
        if !u.status.success() {
            return (false, false);
        }
        let upstream = String::from_utf8_lossy(&u.stdout);
        let upstream = upstream.trim();
        if upstream.is_empty() {
            return (false, false);
        }
        if let Ok(cnt) = git.run_git(repo, &["rev-list", "--count", &format!("{upstream}..HEAD")]) {
            let n = String::from_utf8_lossy(&cnt.stdout).trim().parse::<u64>().unwrap_or(0);
            return (n > 0, true);
        }
    }
    (false, false)
}

pub(crate) fn has_commits(repo: &Path, git: &dyn GitRunner) -> bool {
    git.run_git(repo, &["rev-parse", "--verify", "HEAD"]) 
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Default, Clone)]
pub(crate) struct PushMetrics {
    pub(crate) ahead: u64,
    pub(crate) earliest_age: Option<Duration>,
    pub(crate) latest_age: Option<Duration>,
}

pub(crate) fn push_metrics(repo: &Path, git: &dyn GitRunner, clock: &dyn Clock) -> Option<PushMetrics> {
    let uref = upstream_ref(repo, git)?;
    let ahead = count_ahead(repo, git, &uref)?;
    if ahead == 0 {
        return None;
    }
    let (earliest, latest) = commit_age_bounds(repo, git, clock, &uref)?;
    Some(PushMetrics { ahead, earliest_age: earliest, latest_age: latest })
}

fn upstream_ref(repo: &Path, git: &dyn GitRunner) -> Option<String> {
    let upstream = git
        .run_git(repo, &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .ok()?;
    if !upstream.status.success() {
        return None;
    }
    let uref = String::from_utf8_lossy(&upstream.stdout).trim().to_string();
    if uref.is_empty() { None } else { Some(uref) }
}

fn count_ahead(repo: &Path, git: &dyn GitRunner, uref: &str) -> Option<u64> {
    let count = git.run_git(repo, &["rev-list", "--count", &format!("{uref}..HEAD")]).ok()?;
    if !count.status.success() { return None; }
    Some(String::from_utf8_lossy(&count.stdout).trim().parse::<u64>().unwrap_or(0))
}

fn commit_age_bounds(
    repo: &Path,
    git: &dyn GitRunner,
    clock: &dyn Clock,
    uref: &str,
) -> Option<(Option<Duration>, Option<Duration>)> {
    let log = git.run_git(repo, &["log", "--format=%ct", &format!("{uref}..HEAD")]).ok()?;
    if !log.status.success() { return None; }
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
            min_age = Some(match min_age { Some(cur) if cur < age => cur, _ => age });
            max_age = Some(match max_age { Some(cur) if cur > age => cur, _ => age });
        }
    }
    Some((max_age, min_age))
}
