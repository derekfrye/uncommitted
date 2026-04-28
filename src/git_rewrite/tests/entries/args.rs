use std::fs;
use std::time::SystemTime;

use super::super::support::{FixedClock, write_executable_script};
use super::pair_config;
use crate::git_rewrite::collect_git_rewrite_entries;

#[cfg(unix)]
#[test]
fn collect_entries_respects_commit_overrides() {
    let fixture = pair_config("commit-from = \"v1\"\n", "commit-to = \"v2\"\n", 1);
    let args = collect_logged_args(&fixture);

    assert!(args.contains("--commit-from v1"));
    assert!(args.contains("--commit-to v2"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_commit_count_lookback() {
    let fixture = pair_config("commit-count-lookback = 5\n", "", 1);
    let args = collect_logged_args(&fixture);

    assert!(args.contains("--commit-msg-to-match-on-for-next-logic 5"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_no_metrics_when_present_on_source() {
    let fixture = pair_config("no-metrics = true\n", "", 1);
    let args = collect_logged_args(&fixture);

    assert!(args.contains("--no-metrics"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_no_metrics_when_present_on_target() {
    let fixture = pair_config("", "no-metrics = true\n", 1);
    let args = collect_logged_args(&fixture);

    assert!(args.contains("--no-metrics"));
}

#[cfg(unix)]
fn collect_logged_args(fixture: &super::EntryFixture) -> String {
    let log_path = fixture.temp.path().join("args.log");
    let script_path = fixture.temp.path().join("git_rewrite_stub.sh");
    let script_body = format!(
        "#!/usr/bin/env bash\nprintf \"%s\\n\" \"$*\" > \"{}\"\ncat <<'JSON'\n[]\nJSON\n",
        log_path.display()
    );
    write_executable_script(&script_path, &script_body);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let entries =
        collect_git_rewrite_entries(&fixture.config_path, &script_path, &clock).expect("entries");
    assert_eq!(entries.len(), 1);

    fs::read_to_string(&log_path).expect("log args")
}
