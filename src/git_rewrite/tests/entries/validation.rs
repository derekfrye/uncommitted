use std::time::SystemTime;

use super::super::support::FixedClock;
use super::pair_config;
use crate::git_rewrite::{GitRewriteError, collect_git_rewrite_entries};

#[test]
fn commit_overrides_must_match_between_repos() {
    let fixture = pair_config("commit-from = \"v1\"\n", "commit-from = \"v2\"\n", 1);
    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = fixture.temp.path().join("git_rewrite_stub.sh");
    let err = collect_git_rewrite_entries(&fixture.config_path, &binary_path, &clock).unwrap_err();

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
    let fixture = pair_config("", "commit-count-lookback = 3\n", 1);
    let clock = FixedClock(SystemTime::UNIX_EPOCH);
    let binary_path = fixture.temp.path().join("git_rewrite_stub.sh");
    let err = collect_git_rewrite_entries(&fixture.config_path, &binary_path, &clock).unwrap_err();

    match err {
        GitRewriteError::InvalidConfig { message } => {
            assert!(message.contains("commit-count-lookback"));
            assert!(message.contains("match-key 1"));
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}
