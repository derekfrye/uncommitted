use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::{DateTime, Local};
use serde_json::{Map, Value};

use crate::types::GitRewriteEntry;

use super::{
    config::RepoPair,
    error::GitRewriteError,
    time::{compute_bounds, parse_local_datetime},
};

pub(crate) fn run_pair(
    pair: RepoPair,
    binary_path: &PathBuf,
    now_local: DateTime<Local>,
) -> Result<GitRewriteEntry, GitRewriteError> {
    let output = invoke_git_rewrite(&pair, binary_path)?;
    let values = parse_payload(&pair, &output.stdout)?;
    let (commits, timestamps) = summarize_entries(&values, &pair)?;
    let (earliest_secs, latest_secs) = compute_bounds(&timestamps, now_local);

    let source_repo = repo_display_name(&pair.source.path);
    let target_repo = repo_display_name(&pair.target.path);

    Ok(GitRewriteEntry {
        source_repo,
        source_branch: pair.source.branch.clone(),
        source_path: pair.source.path.display().to_string(),
        target_repo,
        target_branch: pair.target.branch.clone(),
        target_path: pair.target.path.display().to_string(),
        commits,
        earliest_secs,
        latest_secs,
    })
}

fn invoke_git_rewrite(
    pair: &RepoPair,
    binary_path: &PathBuf,
) -> Result<std::process::Output, GitRewriteError> {
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
            match_key: pair.key.clone(),
            status: output.status,
            stderr,
        });
    }

    Ok(output)
}

fn parse_payload(pair: &RepoPair, stdout: &[u8]) -> Result<Vec<Value>, GitRewriteError> {
    let parsed: Value = serde_json::from_slice(stdout).map_err(|source| GitRewriteError::Json {
        match_key: pair.key.clone(),
        source,
    })?;
    match parsed {
        Value::Array(entries) => Ok(entries),
        Value::Object(map) => handle_object_payload(pair, map),
        other => Err(GitRewriteError::UnexpectedJson {
            match_key: pair.key.clone(),
            value: other,
        }),
    }
}

fn handle_object_payload(
    pair: &RepoPair,
    map: Map<String, Value>,
) -> Result<Vec<Value>, GitRewriteError> {
    if map
        .get("msg")
        .and_then(Value::as_str)
        .is_some_and(|msg| msg.eq_ignore_ascii_case("nothing to do"))
    {
        Ok(Vec::new())
    } else {
        Err(GitRewriteError::UnexpectedJson {
            match_key: pair.key.clone(),
            value: Value::Object(map),
        })
    }
}

fn summarize_entries(
    values: &[Value],
    pair: &RepoPair,
) -> Result<(u64, Vec<DateTime<Local>>), GitRewriteError> {
    let mut unique_commits: HashSet<String> = HashSet::new();
    let mut timestamps: Vec<DateTime<Local>> = Vec::new();

    for value in values {
        if let Some(commit) = value
            .get("commit_hash")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            unique_commits.insert(commit.to_string());
        } else {
            unique_commits.insert(value.to_string());
        }

        let timestamp_field = value.get("original_commit_dt").or_else(|| value.get("dt"));

        if let Some(dt_str) = timestamp_field.and_then(|v| v.as_str())
            && !dt_str.trim().is_empty()
        {
            let dt = parse_local_datetime(&pair.key, dt_str)?;
            timestamps.push(dt);
        }
    }

    let commits = u64::try_from(unique_commits.len()).unwrap_or(u64::MAX);
    Ok((commits, timestamps))
}

pub(crate) fn repo_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}
