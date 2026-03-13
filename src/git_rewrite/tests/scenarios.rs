use tempfile::tempdir;

use super::support::{
    RepoArgs, UntrackedScenario, build_repo, create_paths, make_scenario, write_basic_pair_config,
    write_missing_and_ignored_config,
};

pub(super) fn scenario_missing_and_ignored() -> UntrackedScenario {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    let ignored_dir = temp.path().join("ignored_repo");
    let missing_dir = temp.path().join("missing_repo");
    create_paths(&[&source_dir, &target_dir, &ignored_dir, &missing_dir]);

    let config_path = temp.path().join("config.toml");
    write_missing_and_ignored_config(&config_path, &source_dir, &target_dir, &ignored_dir);

    let repos = vec![
        build_repo(RepoArgs {
            name: "source_repo",
            branch: "main",
            path: &source_dir,
            root_display: "~/src",
            root_full: "/tmp/src",
            head_revs: Some(3),
            head_earliest_secs: Some(10),
            head_latest_secs: Some(5),
        }),
        build_repo(RepoArgs {
            name: "target_repo",
            branch: "dev",
            path: &target_dir,
            root_display: "~/main",
            root_full: "/tmp/main",
            head_revs: Some(2),
            head_earliest_secs: None,
            head_latest_secs: None,
        }),
        build_repo(RepoArgs {
            name: "ignored_repo",
            branch: "feature",
            path: &ignored_dir,
            root_display: "~/src",
            root_full: "/tmp/src",
            head_revs: Some(1),
            head_earliest_secs: Some(20),
            head_latest_secs: Some(8),
        }),
        build_repo(RepoArgs {
            name: "missing_repo",
            branch: "main",
            path: &missing_dir,
            root_display: "~/main",
            root_full: "/tmp/main",
            head_revs: Some(0),
            head_earliest_secs: None,
            head_latest_secs: None,
        }),
    ];

    make_scenario(config_path, repos, missing_dir, temp)
}

pub(super) fn scenario_missing_configured_repo() -> UntrackedScenario {
    let temp = tempdir().expect("tempdir");
    let source_dir = temp.path().join("source_repo");
    let target_dir = temp.path().join("target_repo");
    create_paths(&[&source_dir, &target_dir]);

    let config_path = temp.path().join("config.toml");
    write_basic_pair_config(&config_path, &source_dir, "main", &target_dir, "dev");

    let repos = vec![build_repo(RepoArgs {
        name: "source_repo",
        branch: "main",
        path: &source_dir,
        root_display: "~/src",
        root_full: "/tmp/src",
        head_revs: Some(3),
        head_earliest_secs: Some(10),
        head_latest_secs: Some(5),
    })];

    make_scenario(config_path, repos, target_dir, temp)
}
