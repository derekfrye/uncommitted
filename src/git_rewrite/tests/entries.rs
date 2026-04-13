use std::fs;
use std::time::SystemTime;

use super::support::{FixedClock, temp_pair_dirs, write_executable_script, write_file};
use crate::git_rewrite::{GitRewriteError, collect_git_rewrite_entries};

use super::super::time::{diff_seconds, parse_local_datetime};

#[cfg(unix)]
#[test]
fn collect_entries_counts_unique_commits() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let script_path = temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\ncat <<'JSON'\n[\n  { \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/01/24 01:00 PM\" },\n  { \"commit_hash\": \"def\", \"dt\": \"01/01/24 04:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" },\n  { \"commit_hash\": \"abc\", \"dt\": \"01/02/24 08:00 AM\", \"original_commit_dt\": \"01/02/24 01:30 PM\" }\n]\nJSON\n",
    );

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
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let script_path = temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\ncat <<'JSON'\n{\n  \"msg\": \"nothing to do\"\n}\nJSON\n",
    );

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
fn collect_entries_respects_commit_overrides() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\ncommit-from = \"v1\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\ncommit-to = \"v2\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let log_path = temp.path().join("args.log");
    let script_path = temp.path().join("git_rewrite_stub.sh");
    let script_body = format!(
        "#!/usr/bin/env bash\nprintf \"%s\\n\" \"$*\" > \"{}\"\ncat <<'JSON'\n[]\nJSON\n",
        log_path.display()
    );
    write_executable_script(&script_path, &script_body);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");
    assert_eq!(entries.len(), 1);

    let args = fs::read_to_string(&log_path).expect("log args");
    assert!(args.contains("--commit-from v1"));
    assert!(args.contains("--commit-to v2"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_commit_count_lookback() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\ncommit-count-lookback = 5\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let log_path = temp.path().join("args.log");
    let script_path = temp.path().join("git_rewrite_stub.sh");
    let script_body = format!(
        "#!/usr/bin/env bash\nprintf \"%s\\n\" \"$*\" > \"{}\"\ncat <<'JSON'\n[]\nJSON\n",
        log_path.display()
    );
    write_executable_script(&script_path, &script_body);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");
    assert_eq!(entries.len(), 1);

    let args = fs::read_to_string(&log_path).expect("log args");
    assert!(args.contains("--commit-msg-to-match-on-for-next-logic 5"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_no_metrics_when_present_on_source() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nno-metrics = true\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let log_path = temp.path().join("args.log");
    let script_path = temp.path().join("git_rewrite_stub.sh");
    let script_body = format!(
        "#!/usr/bin/env bash\nprintf \"%s\\n\" \"$*\" > \"{}\"\ncat <<'JSON'\n[]\nJSON\n",
        log_path.display()
    );
    write_executable_script(&script_path, &script_body);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");
    assert_eq!(entries.len(), 1);

    let args = fs::read_to_string(&log_path).expect("log args");
    assert!(args.contains("--no-metrics"));
}

#[cfg(unix)]
#[test]
fn collect_entries_passes_no_metrics_when_present_on_target() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nno-metrics = true\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let log_path = temp.path().join("args.log");
    let script_path = temp.path().join("git_rewrite_stub.sh");
    let script_body = format!(
        "#!/usr/bin/env bash\nprintf \"%s\\n\" \"$*\" > \"{}\"\ncat <<'JSON'\n[]\nJSON\n",
        log_path.display()
    );
    write_executable_script(&script_path, &script_body);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let entries = collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");
    assert_eq!(entries.len(), 1);

    let args = fs::read_to_string(&log_path).expect("log args");
    assert!(args.contains("--no-metrics"));
}

#[cfg(unix)]
#[test]
fn collect_entries_skips_pairs_marked_ignore() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 42\nrepo-type = \"source\"\nignore = 1\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 42\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = temp.path().join("git_rewrite_stub.sh");
    let entries = collect_git_rewrite_entries(&config_path, &binary_path, &clock).expect("entries");
    assert!(entries.is_empty());
}

#[cfg(unix)]
#[test]
fn collect_entries_reports_missing_target_branch_with_repo_path() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 3\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\nmatch-key = 3\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let script_path = temp.path().join("git_rewrite_stub.sh");
    write_executable_script(
        &script_path,
        "#!/usr/bin/env bash\nprintf \"Error: Branch 'dev' does not exist in target repository\" >&2\nexit 1\n",
    );

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let err = collect_git_rewrite_entries(&config_path, &script_path, &clock).unwrap_err();
    let message = err.to_string();

    assert!(message.contains("match-key 3"));
    assert!(message.contains("Branch 'dev' does not exist in target repository"));
    assert!(message.contains(&format!("({})", target_dir.display())));
}

#[test]
fn commit_overrides_must_match_between_repos() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\ncommit-from = \"v1\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\ncommit-from = \"v2\"\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = temp.path().join("git_rewrite_stub.sh");
    let err = collect_git_rewrite_entries(&config_path, &binary_path, &clock).unwrap_err();

    match err {
        GitRewriteError::InvalidConfig { message } => {
            assert!(message.contains("commit-from"));
            assert!(message.contains("match-key 1"));
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn commit_count_lookback_requires_source_repo() {
    let (temp, source_dir, target_dir) = temp_pair_dirs();
    let config_path = temp.path().join("config.toml");
    let config_contents = format!(
        "[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"main\"\nmatch-key = 1\nrepo-type = \"source\"\n\n[[repo]]\nrepository-path = \"{}\"\nrepository-branch = \"dev\"\ncommit-count-lookback = 3\nmatch-key = 1\nrepo-type = \"target\"\n",
        source_dir.display(),
        target_dir.display()
    );
    write_file(&config_path, config_contents);

    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = temp.path().join("git_rewrite_stub.sh");
    let err = collect_git_rewrite_entries(&config_path, &binary_path, &clock).unwrap_err();

    match err {
        GitRewriteError::InvalidConfig { message } => {
            assert!(message.contains("commit-count-lookback"));
            assert!(message.contains("match-key 1"));
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}
