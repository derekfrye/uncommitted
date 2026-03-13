use crate::{git_rewrite::collect_git_rewrite_untracked, types::UntrackedReason};

use super::scenarios::{scenario_missing_and_ignored, scenario_missing_configured_repo};

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
