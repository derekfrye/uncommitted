use std::time::SystemTime;

use super::super::super::time::{diff_seconds, parse_local_datetime};
use super::super::support::{FixedClock, write_executable_script};
use super::pair_config;
use crate::git_rewrite::collect_git_rewrite_entries;

#[cfg(unix)]
#[test]
fn collect_entries_counts_unique_commits() {
    let fixture = pair_config("", "", 1);
    let script_path = fixture.temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\ncat <<'JSON'\n[\n  { \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/01/24 01:00 PM\" },\n  { \"commit_hash\": \"def\", \"dt\": \"01/01/24 04:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" },\n  { \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" }\n]\nJSON\n",
    );

    let now_dt = parse_local_datetime("test", "01/03/24 01:30 PM").expect("now parse");
    let clock = FixedClock(now_dt.into());
    let entries =
        collect_git_rewrite_entries(&fixture.config_path, &script_path, &clock).expect("entries");

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
    let fixture = pair_config("", "", 1);
    let script_path = fixture.temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\ncat <<'JSON'\n{\n  \"msg\": \"nothing to do\"\n}\nJSON\n",
    );

    let now_dt = parse_local_datetime("test", "01/03/24 01:30 PM").expect("now parse");
    let clock = FixedClock(now_dt.into());
    let entries =
        collect_git_rewrite_entries(&fixture.config_path, &script_path, &clock).expect("entries");

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(entry.commits, 0);
    assert!(entry.earliest_secs.is_none());
    assert!(entry.latest_secs.is_none());
}

#[cfg(unix)]
#[test]
fn collect_entries_skips_pairs_marked_ignore() {
    let fixture = pair_config("ignore = 1\n", "", 42);
    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = fixture.temp.path().join("git_rewrite_stub.sh");
    let entries =
        collect_git_rewrite_entries(&fixture.config_path, &binary_path, &clock).expect("entries");
    assert!(entries.is_empty());
}

#[cfg(unix)]
#[test]
fn collect_entries_reports_missing_target_branch_with_repo_path() {
    let fixture = pair_config("", "", 3);
    let script_path = fixture.temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\nprintf \"Error: Branch 'dev' does not exist in target repository\" >&2\nexit 1\n",
    );

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let err = collect_git_rewrite_entries(&fixture.config_path, &script_path, &clock).unwrap_err();
    let message = err.to_string();

    assert!(message.contains("match-key 3"));
    assert!(message.contains("Branch 'dev' does not exist in target repository"));
    assert!(message.contains(&format!("({})", fixture.target_dir.display())));
}
