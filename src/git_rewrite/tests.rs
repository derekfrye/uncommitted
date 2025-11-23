use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tempfile::{TempDir, tempdir};

use crate::{
    system::Clock,
    types::{RepoSummary, UntrackedReason},
};

use super::{
    collect_git_rewrite_entries, collect_git_rewrite_untracked,
    config::match_key_to_string,
    time::{diff_seconds, parse_local_datetime},
};

struct FixedClock(SystemTime);

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

#[cfg(unix)]
#[test]
fn collect_entries_counts_unique_commits() {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    fs::create_dir_all(&source_dir).expect("create source");
    fs::create_dir_all(&target_dir).expect("create target");

    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let script_path = temp.path().join("git_rewrite_stub.sh");
    let mut script = File::create(&script_path).expect("script file");
    writeln!(
        script,
        "#!/usr/bin/env bash\ncat <<'JSON'\n[\n  {{ \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/01/24 01:00 PM\" }},\n  {{ \"commit_hash\": \"def\", \"dt\": \"01/01/24 04:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" }},\n  {{ \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" }}\n]\nJSON"
    )
    .expect("write script");
    script.sync_all().expect("flush script");
    drop(script);
    let mut perms = fs::metadata(&script_path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("perm");

    let now_dt = parse_local_datetime("test", "01/03/24 01:30 PM").expect("now parse");
    let clock = FixedClock(now_dt.into());

    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.source_repo, "source_repo");
    assert_eq!(entry.source_branch, "main");
    assert_eq!(entry.target_repo, "target_repo");
    assert_eq!(entry.target_branch, "dev");
    assert_eq!(entry.commits, 2);

    let earliest = diff_seconds(
        now_dt,
        parse_local_datetime("test", "01/01/24 01:00 PM").unwrap(),
    );
    let latest = diff_seconds(
        now_dt,
        parse_local_datetime("test", "01/02/24 01:30 PM").unwrap(),
    );
    assert_eq!(entry.earliest_secs, Some(earliest));
    assert_eq!(entry.latest_secs, Some(latest));
}

#[cfg(unix)]
#[test]
fn collect_entries_handles_nothing_to_do_message() {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    fs::create_dir_all(&source_dir).expect("create source");
    fs::create_dir_all(&target_dir).expect("create target");

    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let script_path = temp.path().join("git_rewrite_stub.sh");
    let mut script = File::create(&script_path).expect("script file");
    let script_body = r#"#!/usr/bin/env bash
cat <<'JSON'
{
  "msg": "nothing to do"
}
JSON
"#;
    script
        .write_all(script_body.as_bytes())
        .expect("write script");
    script.sync_all().expect("flush script");
    drop(script);
    let mut perms = fs::metadata(&script_path).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script_path, perms).expect("perm");

    let now_dt = parse_local_datetime("test", "01/03/24 01:30 PM").expect("now parse");
    let clock = FixedClock(now_dt.into());

    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.commits, 0);
    assert!(entry.earliest_secs.is_none());
    assert!(entry.latest_secs.is_none());
}

#[cfg(unix)]
#[test]
fn collect_entries_skips_pairs_marked_ignore() {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    fs::create_dir_all(&source_dir).expect("create source");
    fs::create_dir_all(&target_dir).expect("create target");

    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 42\nrepo-type = \"source\"\nignore = 1\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 42\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = temp.path().join("git_rewrite_stub.sh");

    let entries = collect_git_rewrite_entries(&config_path, &binary_path, &clock).expect("entries");
    assert!(entries.is_empty());
}

#[test]
fn match_key_accepts_integer() {
    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(deserialize_with = "match_key_to_string")]
        value: String,
    }

    let cfg: Wrapper = toml::from_str("value = 7").expect("parse");
    assert_eq!(cfg.value, "7");
}

#[test]
fn collect_untracked_reports_missing_and_ignored() {
    let scenario = scenario_missing_and_ignored();
    let entries = collect_git_rewrite_untracked(&scenario.config_path, &scenario.repos)
        .expect("untracked entries");
    assert_eq!(entries.len(), 2);
    let first = &entries[0];
    assert_eq!(first.repo, "missing_repo");
    assert_eq!(first.reason, UntrackedReason::MissingConfig);
    assert_eq!(first.revs, Some(0));

    let second = &entries[1];
    assert_eq!(second.repo, "ignored_repo");
    assert_eq!(second.reason, UntrackedReason::Ignored);
    assert_eq!(second.revs, Some(1));
    assert_eq!(second.earliest_secs, Some(20));
    assert_eq!(second.latest_secs, Some(8));
}

#[test]
fn collect_untracked_reports_missing_configured_repos() {
    let scenario = scenario_missing_configured_repo();
    let entries = collect_git_rewrite_untracked(&scenario.config_path, &scenario.repos)
        .expect("untracked entries");
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.reason, UntrackedReason::MissingRepo);
    assert_eq!(entry.branch, "dev");
    assert_eq!(entry.repo, scenario.expected_missing_repo());
    assert!(entry.revs.is_none());
}

struct UntrackedScenario {
    config_path: PathBuf,
    repos: Vec<RepoSummary>,
    missing_repo_path: PathBuf,
    _temp: TempDir,
}

impl UntrackedScenario {
    fn expected_missing_repo(&self) -> String {
        self.missing_repo_path.display().to_string()
    }
}

fn scenario_missing_and_ignored() -> UntrackedScenario {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    let ignored_dir = temp.path().join("ignored_repo");
    let missing_dir = temp.path().join("missing_repo");
    create_dirs(&[&source_dir, &target_dir, &ignored_dir, &missing_dir]);

    let config_path = temp.path().join("config.toml");
    write_missing_and_ignored_config(&config_path, &source_dir, &target_dir, &ignored_dir);

    let repos = vec![
        repo(
            "source_repo",
            "main",
            &source_dir,
            "~/src",
            "/tmp/src",
            Some(3),
            Some(10),
            Some(5),
        ),
        repo(
            "target_repo",
            "dev",
            &target_dir,
            "~/main",
            "/tmp/main",
            Some(2),
            None,
            None,
        ),
        repo(
            "ignored_repo",
            "feature",
            &ignored_dir,
            "~/src",
            "/tmp/src",
            Some(1),
            Some(20),
            Some(8),
        ),
        repo(
            "missing_repo",
            "main",
            &missing_dir,
            "~/main",
            "/tmp/main",
            Some(0),
            None,
            None,
        ),
    ];

    UntrackedScenario {
        config_path,
        repos,
        missing_repo_path: missing_dir,
        _temp: temp,
    }
}

fn scenario_missing_configured_repo() -> UntrackedScenario {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    create_dirs(&[&source_dir, &target_dir]);

    let config_path = temp.path().join("config.toml");
    write_basic_pair_config(&config_path, &source_dir, "main", &target_dir, "dev");

    let repos = vec![repo(
        "source_repo",
        "main",
        &source_dir,
        "~/src",
        "/tmp/src",
        Some(3),
        Some(10),
        Some(5),
    )];

    UntrackedScenario {
        config_path,
        repos,
        missing_repo_path: target_dir,
        _temp: temp,
    }
}

fn write_missing_and_ignored_config(
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
    fs::write(config_path, config_contents).expect("write config");
}

fn write_basic_pair_config(
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
    fs::write(config_path, config_contents).expect("write config");
}

fn create_dirs(paths: &[&Path]) {
    for path in paths {
        fs::create_dir_all(path).expect("make dir");
    }
}

fn repo(
    name: &str,
    branch: &str,
    path: &Path,
    root_display: &str,
    root_full: &str,
    head_revs: Option<u64>,
    head_earliest_secs: Option<u64>,
    head_latest_secs: Option<u64>,
) -> RepoSummary {
    RepoSummary {
        repo: name.to_string(),
        branch: branch.to_string(),
        path: path.to_path_buf(),
        root_display: root_display.to_string(),
        root_full: root_full.to_string(),
        head_revs,
        head_earliest_secs,
        head_latest_secs,
    }
}
