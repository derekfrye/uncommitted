#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use clap::Parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
            uncommitted.push(name.clone());
        }
        if has_staged(&r) {
            staged.push(name.clone());
        }
        let (is_ahead, has_up) = ahead_of_upstream(&r);
        if is_ahead {
            ahead.push(name.clone());
        } else if !has_up && has_commits(&r) {
            no_upstream.push(name.clone());
        }
    }

    let join = |v: Vec<String>| v.join(", ");

    println!("uncommitted: {}", join(uncommitted));
    println!("staged: {}", join(staged));
    println!("could push: {}", join(ahead));
    // Uncomment if you want to see repos with commits but no upstream set:
    // println!("no upstream: {}", join(no_upstream));
}
