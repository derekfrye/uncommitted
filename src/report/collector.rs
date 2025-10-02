use std::env;
use std::path::PathBuf;

use crate::scan::find_repos;
use crate::system::{Clock, FsOps};
use crate::types::{Options, ReportData};

use super::repository::{RootContext, process_repo};

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

    let mut rooted: Vec<(String, PathBuf)> = Vec::new();
    for root in &roots {
        let root_display = root.to_string_lossy().to_string();
        let expanded = fs.expand_tilde(root);
        let root_full = if expanded.is_absolute() {
            expanded
        } else {
            match env::current_dir() {
                Ok(cwd) => cwd.join(expanded),
                Err(_) => expanded,
            }
        };
        rooted.push((root_display, root_full));
    }

    let mut data = ReportData {
        multi_root: rooted.len() > 1,
        ..ReportData::default()
    };

    for (root_display, root_full) in &rooted {
        let repos = find_repos(fs, std::slice::from_ref(root_full), opts.depth, opts.debug);

        if opts.debug {
            eprintln!(
                "[debug] root_display={} root_full={} depth={} repos_found={}",
                root_display,
                root_full.display(),
                opts.depth,
                repos.len()
            );
        }

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
                &mut data,
            );
        }
    }

    data
}
