use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tempfile::{TempDir, tempdir};

use crate::{system::Clock, types::RepoSummary};

pub(super) struct FixedClock(pub(super) SystemTime);

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

pub(super) struct UntrackedScenario {
    pub(super) config_path: PathBuf,
    pub(super) repos: Vec<RepoSummary>,
    missing_repo_path: PathBuf,
    _temp: TempDir,
}

impl UntrackedScenario {
    pub(super) fn expected_missing_repo(&self) -> String {
        self.missing_repo_path.display().to_string()
    }
}

pub(super) fn temp_pair_dirs() -> (TempDir, PathBuf, PathBuf) {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    create_dirs(&[&source_dir, &target_dir]);
    (temp, source_dir, target_dir)
}

pub(super) fn write_file(path: &Path, contents: impl AsRef<[u8]>) {
    fs::write(path, contents).expect("write file");
}

#[cfg(unix)]
pub(super) fn write_executable_script(path: &Path, contents: &str) {
    let mut script = File::create(path).expect("script file");
    script
        .write_all(contents.as_bytes())
        .expect("write script contents");
    script.sync_all().expect("flush script");
    drop(script);
    let mut perms = fs::metadata(path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("perm");
}

pub(super) fn write_basic_pair_config(
    config_path: &Path,
    source_path: &Path,
    source_branch: &str,
    target_path: &Path,
    target_branch: &str,
) {
    let config_contents = format!(
        "\
[[repo]]
repository-path = \"{src}\"
repository-branch = \"{src_branch}\"
match-key = 1
repo-type = \"source\"

[[repo]]
repository-path = \"{dst}\"
repository-branch = \"{dst_branch}\"
match-key = 1
repo-type = \"target\"
",
        src = source_path.display(),
        src_branch = source_branch,
        dst = target_path.display(),
        dst_branch = target_branch
    );
    write_file(config_path, config_contents);
}

pub(super) fn write_missing_and_ignored_config(
    config_path: &Path,
    source_path: &Path,
    target_path: &Path,
    ignored_path: &Path,
) {
    let config_contents = format!(
        "\
[[repo]]
repository-path = \"{src}\"
repository-branch = \"main\"
match-key = 1
repo-type = \"source\"

[[repo]]
repository-path = \"{dst}\"
repository-branch = \"dev\"
match-key = 1
repo-type = \"target\"

[[repo]]
repository-path = \"{ign}\"
repository-branch = \"feature\"
match-key = 2
repo-type = \"source\"
ignore = 1

[[repo]]
repository-path = \"{ign}\"
repository-branch = \"feature\"
match-key = 2
repo-type = \"target\"
ignore = 1
",
        src = source_path.display(),
        dst = target_path.display(),
        ign = ignored_path.display()
    );
    write_file(config_path, config_contents);
}

fn create_dirs(paths: &[&Path]) {
    for path in paths {
        fs::create_dir_all(path).expect("make dir");
    }
}

pub(super) struct RepoArgs<'a> {
    pub(super) name: &'a str,
    pub(super) branch: &'a str,
    pub(super) path: &'a Path,
    pub(super) root_display: &'a str,
    pub(super) root_full: &'a str,
    pub(super) head_revs: Option<u64>,
    pub(super) head_earliest_secs: Option<u64>,
    pub(super) head_latest_secs: Option<u64>,
}

fn repo(args: &RepoArgs<'_>) -> RepoSummary {
    RepoSummary {
        repo: args.name.to_string(),
        branch: args.branch.to_string(),
        path: args.path.to_path_buf(),
        root_display: args.root_display.to_string(),
        root_full: args.root_full.to_string(),
        head_revs: args.head_revs,
        head_earliest_secs: args.head_earliest_secs,
        head_latest_secs: args.head_latest_secs,
    }
}

pub(super) fn create_paths(paths: &[&Path]) {
    create_dirs(paths);
}

pub(super) fn build_repo(args: RepoArgs<'_>) -> RepoSummary {
    repo(&args)
}

pub(super) fn make_scenario(
    config_path: PathBuf,
    repos: Vec<RepoSummary>,
    missing_repo_path: PathBuf,
    temp: TempDir,
) -> UntrackedScenario {
    UntrackedScenario {
        config_path,
        repos,
        missing_repo_path,
        _temp: temp,
    }
}
