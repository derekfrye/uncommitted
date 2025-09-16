 

use std::collections::HashSet;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::system::FsOps;

pub fn find_repos(fs: &dyn FsOps, roots: &[PathBuf], depth: usize, debug: bool) -> Vec<PathBuf> {
    let mut repos = HashSet::<PathBuf>::new();

    for root in roots {
        let root = fs.expand_tilde(root);
        if !root.exists() {
            if debug {
                eprintln!("[debug] root missing: {}", root.display());
            }
            continue;
        }

        if fs.is_repo(&root) {
            if debug {
                eprintln!("[debug] repo: {}", root.display());
            }
            repos.insert(root.clone());
            continue;
        }

        let max_depth = depth.saturating_add(1);
        for entry in WalkDir::new(&root)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let p = entry.path();
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
