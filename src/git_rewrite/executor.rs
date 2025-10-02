use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Stdio};

use chrono::{DateTime, Local, Utc};

use crate::{system::Clock, types::GitRewriteEntry};

use super::{
    config::RepoPair,
    error::GitRewriteError,
    time::{compute_bounds, parse_local_datetime},
};

pub(crate) fn collect_entries(
    pairs: Vec<RepoPair>,
    binary_path: &Path,
    clock: &dyn Clock,
) -> Result<Vec<GitRewriteEntry>, GitRewriteError> {
    let now_utc: DateTime<Utc> = clock.now().into();
    let now_local: DateTime<Local> = now_utc.with_timezone(&Local);

    let mut entries = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let output = Command::new(binary_path)
            .arg("--source-repository-path")
            .arg(&pair.source.path)
            .arg("--source-repository-branch")
            .arg(&pair.source.branch)
            .arg("--target-repo")
            .arg(&pair.target.path)
            .arg("--target-repo-branch")
            .arg(&pair.target.branch)
            .arg("--commit-from")
            .arg("NEXT")
            .arg("--commit-to")
            .arg("HEAD")
            .arg("--mode")
            .arg("print")
            .arg("--output-format")
            .arg("json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|source| GitRewriteError::CommandIo { source })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(GitRewriteError::CommandFailure {
                match_key: pair.key,
                status: output.status,
                stderr,
            });
        }

        let stdout = output.stdout;
        let values: Vec<serde_json::Value> =
            serde_json::from_slice(&stdout).map_err(|source| GitRewriteError::Json {
                match_key: pair.key.clone(),
                source,
            })?;

        let mut unique_commits: HashSet<String> = HashSet::new();
        let mut timestamps: Vec<DateTime<Local>> = Vec::new();

        for value in &values {
            if let Some(commit) = value
                .get("commit_hash")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                unique_commits.insert(commit.to_string());
            } else {
                unique_commits.insert(value.to_string());
            }

            if let Some(dt_str) = value.get("dt").and_then(|v| v.as_str())
                && !dt_str.trim().is_empty()
            {
                let dt = parse_local_datetime(&pair.key, dt_str)?;
                timestamps.push(dt);
            }
        }

        let commits = u64::try_from(unique_commits.len()).unwrap_or(u64::MAX);
        let (earliest_secs, latest_secs) = compute_bounds(&timestamps, now_local);

        let source_repo = last_component(&pair.source.path);
        let target_repo = last_component(&pair.target.path);

        entries.push(GitRewriteEntry {
            source_repo,
            source_branch: pair.source.branch.clone(),
            source_path: pair.source.path.display().to_string(),
            target_repo,
            target_branch: pair.target.branch.clone(),
            target_path: pair.target.path.display().to_string(),
            commits,
            earliest_secs,
            latest_secs,
        });
    }

    entries.sort_by(|a, b| (&a.source_repo, &a.target_repo).cmp(&(&b.source_repo, &b.target_repo)));
    Ok(entries)
}

fn last_component(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}
