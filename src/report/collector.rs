use std::env;
use std::path::PathBuf;

use crate::git::GitRunner;
use crate::scan::find_repos;
use crate::system::{Clock, FsOps};
use crate::types::{Options, ReportData};

use super::repository::{RootContext, process_repo};

pub fn collect_report_data(
    opts: &Options,
    fs: &dyn FsOps,
    git: &dyn GitRunner,
    clock: &dyn Clock,
) -> ReportData {
    let rooted = resolve_roots(opts, fs);

    let mut data = ReportData::default();
    data.multi_root = rooted.len() > 1;
    for (root_display, root_full) in &rooted {
        scan_root(root_display, root_full, opts, fs, git, clock, &mut data);
    }

    data
}

fn resolve_roots(opts: &Options, fs: &dyn FsOps) -> Vec<(String, PathBuf)> {
    let default_root = PathBuf::from("~/src");
    let roots = if opts.roots.is_empty() {
        vec![default_root]
    } else {
        opts.roots.clone()
    };

    roots
        .into_iter()
        .map(|root| {
            let display = root.to_string_lossy().to_string();
            let expanded = fs.expand_tilde(&root);
            let full = normalize_root_path(expanded);
            (display, full)
        })
        .collect()
}

fn normalize_root_path(expanded: PathBuf) -> PathBuf {
    if expanded.is_absolute() {
        expanded
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(&expanded))
            .unwrap_or(expanded)
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_root(
    root_display: &str,
    root_full: &PathBuf,
    opts: &Options,
    fs: &dyn FsOps,
    git: &dyn GitRunner,
    clock: &dyn Clock,
    data: &mut ReportData,
) {
    let repos = find_repos(fs, std::slice::from_ref(root_full), opts.depth, opts.debug);
    log_debug(opts, root_display, root_full, repos.len());

    for repo in repos {
        let name = repo
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        process_repo(
            &repo,
            &name,
            RootContext {
                display: root_display,
                full: root_full,
            },
            opts,
            git,
            clock,
            data,
        );
    }
}

fn log_debug(opts: &Options, root_display: &str, root_full: &PathBuf, repo_count: usize) {
    if opts.debug {
        eprintln!(
            "[debug] root_display={} root_full={} depth={} repos_found={}",
            root_display,
            root_full.display(),
            opts.depth,
            repo_count
        );
    }
}
