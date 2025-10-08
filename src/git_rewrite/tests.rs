use std::fs::{self, File};
use std::io::Write as _;
use std::time::SystemTime;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use tempfile::tempdir;

use crate::system::Clock;

use super::{
    collect_git_rewrite_entries,
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
