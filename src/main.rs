#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use clap::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about = "Report git repo states under roots.")]
struct Args {
    /// Root directories to scan (default: ~/src)
    roots: Vec<PathBuf>,

    /// Directory depth to search (0 = only root itself, 1 = one level of children, etc.)
    #[arg(long, default_value_t = 1)]
    depth: usize,

    /// Ignore untracked files for 'uncommitted'
    #[arg(long)]
    no_untracked: bool,

    /// Print debug info while scanning
    #[arg(long)]
    debug: bool,
}

fn expand_tilde(p: &Path) -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if p.starts_with("~") && let Ok(rest) = p.strip_prefix("~") {
            return home.join(rest);
        }
    }
    p.to_path_buf()
}

fn run_git(repo: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

fn is_repo(dir: &Path) -> bool {
    dir.join(".git").is_dir()
}

fn find_repos(roots: &[PathBuf], depth: usize, debug: bool) -> Vec<PathBuf> {
    let mut repos = HashSet::<PathBuf>::new();

    for root in roots {
        let root = expand_tilde(root);
        if !root.exists() {
            if debug {
                eprintln!("[debug] root missing: {}", root.display());
            }
            continue;
        }

        // If the root itself is a repo, include it.
        if is_repo(&root) {
            if debug {
                eprintln!("[debug] repo: {}", root.display());
            }
            repos.insert(root.clone());
            continue;
        }

        // Search for ".git" dir up to depth+1 (so we can see children of root)
        let max_depth = depth.saturating_add(1);
        for entry in WalkDir::new(&root)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
            // We only care about directories named ".git"
            if entry.file_type().is_dir()
                && p.file_name().is_some_and(|n| n == ".git")
                && let Some(parent) = p.parent()
            {
                if debug {
                    eprintln!("[debug] repo: {}", parent.display());
                }
                repos.insert(parent.to_path_buf());
            }
        }
    }

    let mut v: Vec<_> = repos.into_iter().collect();
    v.sort_unstable_by(|a, b| a.file_name().cmp(&b.file_name()));
    v
}

#[derive(Debug, Default, Clone, Copy)]
struct ChangeMetrics {
    lines: u64,
    files: u64,
    untracked: u64,
}

fn parse_numstat(s: &str) -> (u64, u64) {
    // Returns (total_lines_changed, files_changed)
    // numstat format: "<added>\t<deleted>\t<path>"; added/deleted may be '-' for binary
    let mut total_lines = 0u64;
    let mut files = 0u64;
    for line in s.lines() {
        let mut parts = line.split('\t');
        let added = parts.next();
        let deleted = parts.next();
        if added.is_some() && deleted.is_some() {
            files = files.saturating_add(1);
            let add_num = added
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let del_num = deleted
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            total_lines = total_lines.saturating_add(add_num.saturating_add(del_num));
        }
    }
    (total_lines, files)
}

fn count_lines(s: &str) -> u64 {
    // Count non-empty lines; git outputs one path per line.
    s.lines().filter(|l| !l.trim().is_empty()).count() as u64
}

fn uncommitted_metrics(repo: &Path, include_untracked: bool) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = run_git(
        repo,
        &["diff", "--numstat", "--ignore-submodules", "--", "."],
    ) {
        let text = String::from_utf8_lossy(&out.stdout);
        let (lines, files) = parse_numstat(&text);
        metrics.lines = lines;
        metrics.files = files;
    }
    if include_untracked {
        if let Ok(out) = run_git(repo, &["ls-files", "--others", "--exclude-standard"]) {
            metrics.untracked = count_lines(&String::from_utf8_lossy(&out.stdout));
        }
    }
    metrics
}

fn staged_metrics(repo: &Path) -> ChangeMetrics {
    let mut metrics = ChangeMetrics::default();
    if let Ok(out) = run_git(
        repo,
        &["diff", "--cached", "--numstat", "--ignore-submodules", "--", "."],
    ) {
        let text = String::from_utf8_lossy(&out.stdout);
        let (lines, files) = parse_numstat(&text);
        metrics.lines = lines;
        metrics.files = files;
    }
    // For consistency with requested "same metrics", include untracked count here too.
    if let Ok(out) = run_git(repo, &["ls-files", "--others", "--exclude-standard"]) {
        metrics.untracked = count_lines(&String::from_utf8_lossy(&out.stdout));
    }
    metrics
}

#[derive(Debug, Default, Clone)]
struct PushMetrics {
    ahead: u64,
    earliest_age: Option<Duration>,
    latest_age: Option<Duration>,
}

fn push_metrics(repo: &Path) -> Option<PushMetrics> {
    // Determine upstream
    let upstream = run_git(
        repo,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()?;
    if !upstream.status.success() {
        return None;
    }
    let uref = String::from_utf8_lossy(&upstream.stdout).trim().to_string();
    if uref.is_empty() {
        return None;
    }

    // Count ahead
    let count = run_git(repo, &["rev-list", "--count", &format!("{uref}..HEAD")]).ok()?;
    if !count.status.success() {
        return None;
    }
    let ahead = String::from_utf8_lossy(&count.stdout)
        .trim()
        .parse::<u64>()
        .unwrap_or(0);
    if ahead == 0 {
        return None;
    }

    // Collect commit timestamps ahead of upstream
    let log = run_git(repo, &["log", "--format=%ct", &format!("{uref}..HEAD")]).ok()?;
    if !log.status.success() {
        return None;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs();
    let mut min_age: Option<Duration> = None;
    let mut max_age: Option<Duration> = None;
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if let Ok(ts) = line.trim().parse::<u64>() {
            let age_secs = now.saturating_sub(ts);
            let age = Duration::from_secs(age_secs);
            min_age = Some(match min_age {
                Some(cur) if cur < age => cur,
                Some(_) => age,
                None => age,
            });
            max_age = Some(match max_age {
                Some(cur) if cur > age => cur,
                Some(_) => age,
                None => age,
            });
        }
    }

    Some(PushMetrics {
        ahead,
        earliest_age: min_age,
        latest_age: max_age,
    })
}

 
fn has_uncommitted(repo: &Path, include_untracked: bool) -> bool {
    // unstaged changes
    if let Ok(out) = run_git(repo, &["diff", "--quiet", "--ignore-submodules", "--", "."]) {
        if !out.status.success() {
            return true;
        }
    } else {
        return false; // can't run git; treat as clean
    }

    if include_untracked
        && let Ok(out) = run_git(repo, &["ls-files", "--others", "--exclude-standard"])
        && !String::from_utf8_lossy(&out.stdout).trim().is_empty()
    {
        return true;
    }
    false
}

fn has_staged(repo: &Path) -> bool {
    if let Ok(out) = run_git(repo, &["diff", "--cached", "--quiet", "--ignore-submodules", "--", "."]) {
        return !out.status.success();
    }
    false
}

fn ahead_of_upstream(repo: &Path) -> (bool, bool) {
    // returns (ahead, has_upstream)
    if let Ok(u) = run_git(
        repo,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    ) {
        if !u.status.success() {
            return (false, false);
        }
        let upstream = String::from_utf8_lossy(&u.stdout);
        let upstream = upstream.trim();
        if upstream.is_empty() {
            return (false, false);
        }
        if let Ok(cnt) = run_git(repo, &["rev-list", "--count", &format!("{upstream}..HEAD")]) {
            let n = String::from_utf8_lossy(&cnt.stdout).trim().parse::<u64>().unwrap_or(0);
            return (n > 0, true);
        }
    }
    (false, false)
}

fn has_commits(repo: &Path) -> bool {
    run_git(repo, &["rev-parse", "--verify", "HEAD"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn humanize_age(dur: Duration) -> String {
    // Round to one decimal place per requirements.
    let secs = dur.as_secs();
    const SEC_PER_MIN: u64 = 60;
    const SEC_PER_HOUR: u64 = 60 * 60;
    const SEC_PER_DAY: u64 = 60 * 60 * 24;
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

fn main() {
    let args = Args::parse();

    let default_root = PathBuf::from("~/src");
    let roots = if args.roots.is_empty() {
        vec![default_root]
    } else {
        args.roots.clone()
    };

    let repos = find_repos(&roots, args.depth, args.debug);

    if args.debug {
        eprintln!(
            "[debug] roots={:?} depth={} repos_found={}",
            roots, args.depth, repos.len()
        );
    }

    let mut uncommitted = Vec::<String>::new();
    let mut staged = Vec::<String>::new();
    let mut ahead = Vec::<String>::new();
    let mut no_upstream = Vec::<String>::new();

    for r in repos {
        let name = r.file_name().unwrap_or_default().to_string_lossy().to_string();

        if has_uncommitted(&r, !args.no_untracked) {
            let m = uncommitted_metrics(&r, !args.no_untracked);
            uncommitted.push(format!(
                "{name} ({} lines, {} files, {} untracked)",
                m.lines, m.files, m.untracked
            ));
        }
        if has_staged(&r) {
            let m = staged_metrics(&r);
            staged.push(format!(
                "{name} ({} lines, {} files, {} untracked)",
                m.lines, m.files, m.untracked
            ));
        }
        let (is_ahead, has_up) = ahead_of_upstream(&r);
        if is_ahead {
            if let Some(pm) = push_metrics(&r) {
                let earliest = pm
                    .earliest_age
                    .map(humanize_age)
                    .unwrap_or_else(|| "n/a".to_string());
                let latest = pm
                    .latest_age
                    .map(humanize_age)
                    .unwrap_or_else(|| "n/a".to_string());
                ahead.push(format!(
                    "{name} ({} revs, earliest: {earliest} ago, latest: {latest} ago)",
                    pm.ahead
                ));
            } else {
                // Fallback if metrics couldn't be computed for some reason
                ahead.push(format!("{name}"));
            }
        } else if !has_up && has_commits(&r) {
            no_upstream.push(name.clone());
        }
    }

    let join = |v: Vec<String>| v.join(", ");

    println!("uncommitted: {}", join(uncommitted));
    println!("staged: {}", join(staged));
    println!("pushable: {}", join(ahead));
    // Uncomment if you want to see repos with commits but no upstream set:
    // println!("no upstream: {}", join(no_upstream));
}
