use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;

use crate::system::Clock;
use crate::types::GitRewriteEntry;

#[derive(Debug)]
pub enum GitRewriteError {
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    InvalidConfig {
        message: String,
    },
    CommandIo {
        source: std::io::Error,
    },
    CommandFailure {
        match_key: String,
        status: ExitStatus,
        stderr: String,
    },
    Json {
        match_key: String,
        source: serde_json::Error,
    },
    DateParse {
        match_key: String,
        value: String,
        source: chrono::ParseError,
    },
    DateOutOfRange {
        match_key: String,
        value: String,
    },
}

impl std::fmt::Display for GitRewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigRead { path, source } => {
                write!(
                    f,
                    "failed to read git rewrite config {}: {source}",
                    path.display()
                )
            }
            Self::ConfigParse { path, source } => {
                write!(
                    f,
                    "failed to parse git rewrite config {}: {source}",
                    path.display()
                )
            }
            Self::InvalidConfig { message } => write!(f, "invalid git rewrite config: {message}"),
            Self::CommandIo { source } => {
                write!(f, "failed to launch git_rewrite binary: {source}")
            }
            Self::CommandFailure {
                match_key,
                status,
                stderr,
            } => {
                if stderr.is_empty() {
                    write!(
                        f,
                        "git_rewrite invocation for match-key {match_key} failed with status {status}"
                    )
                } else {
                    write!(
                        f,
                        "git_rewrite invocation for match-key {match_key} failed with status {status}: {stderr}"
                    )
                }
            }
            Self::Json { match_key, source } => {
                write!(
                    f,
                    "failed to parse git_rewrite output for match-key {match_key}: {source}"
                )
            }
            Self::DateParse {
                match_key,
                value,
                source,
            } => {
                write!(
                    f,
                    "failed to parse git_rewrite dt '{value}' for match-key {match_key}: {source}"
                )
            }
            Self::DateOutOfRange { match_key, value } => {
                write!(
                    f,
                    "git_rewrite dt '{value}' for match-key {match_key} did not map to a local timestamp"
                )
            }
        }
    }
}

impl std::error::Error for GitRewriteError {}

#[derive(Debug, Deserialize)]
struct GitRewriteConfig {
    #[serde(rename = "repo")]
    repos: Vec<RepoSpec>,
}

#[derive(Debug, Deserialize)]
struct RepoSpec {
    #[serde(rename = "repository-path")]
    repository_path: PathBuf,
    #[serde(rename = "repository-branch")]
    repository_branch: String,
    #[serde(rename = "match-key", deserialize_with = "match_key_to_string")]
    match_key: String,
    #[serde(rename = "repo-type")]
    repo_type: RepoType,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RepoType {
    Source,
    Target,
}

#[derive(Debug, Default)]
struct PairBuilder {
    source: Option<Endpoint>,
    target: Option<Endpoint>,
}

#[derive(Debug, Clone)]
struct Endpoint {
    path: PathBuf,
    branch: String,
}

#[derive(Debug)]
struct RepoPair {
    key: String,
    source: Endpoint,
    target: Endpoint,
}

/// Collect rows for the git rewrite table by executing the configured `git_rewrite` binary.
///
/// # Errors
/// Returns an error when the configuration cannot be read, parsed, or is invalid, when the
/// helper binary cannot be launched or fails, or when its JSON output is malformed.
pub fn collect_git_rewrite_entries(
    config_path: &Path,
    binary_path: &Path,
    clock: &dyn Clock,
) -> Result<Vec<GitRewriteEntry>, GitRewriteError> {
    let config_text =
        std::fs::read_to_string(config_path).map_err(|source| GitRewriteError::ConfigRead {
            path: config_path.to_path_buf(),
            source,
        })?;
    let config: GitRewriteConfig =
        toml::from_str(&config_text).map_err(|source| GitRewriteError::ConfigParse {
            path: config_path.to_path_buf(),
            source,
        })?;
    let pairs = build_pairs(&config)?;
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

fn build_pairs(config: &GitRewriteConfig) -> Result<Vec<RepoPair>, GitRewriteError> {
    let mut map: HashMap<String, PairBuilder> = HashMap::new();
    for repo in &config.repos {
        let entry = map.entry(repo.match_key.clone()).or_default();
        let endpoint = Endpoint {
            path: repo.repository_path.clone(),
            branch: repo.repository_branch.clone(),
        };
        match repo.repo_type {
            RepoType::Source => {
                if entry.source.replace(endpoint).is_some() {
                    return Err(GitRewriteError::InvalidConfig {
                        message: format!(
                            "multiple source repos defined for match-key {}",
                            repo.match_key
                        ),
                    });
                }
            }
            RepoType::Target => {
                if entry.target.replace(endpoint).is_some() {
                    return Err(GitRewriteError::InvalidConfig {
                        message: format!(
                            "multiple target repos defined for match-key {}",
                            repo.match_key
                        ),
                    });
                }
            }
        }
    }

    let mut result = Vec::new();
    for (key, pair) in map {
        let (Some(source), Some(target)) = (pair.source, pair.target) else {
            return Err(GitRewriteError::InvalidConfig {
                message: format!("match-key {key} must define both source and target repos"),
            });
        };
        result.push(RepoPair {
            key,
            source,
            target,
        });
    }
    result.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(result)
}

fn compute_bounds(
    timestamps: &[DateTime<Local>],
    now: DateTime<Local>,
) -> (Option<u64>, Option<u64>) {
    if timestamps.is_empty() {
        return (None, None);
    }
    let earliest = timestamps.iter().min().copied();
    let latest = timestamps.iter().max().copied();
    let earliest_secs = earliest.map(|dt| diff_seconds(now, dt));
    let latest_secs = latest.map(|dt| diff_seconds(now, dt));
    (earliest_secs, latest_secs)
}

fn diff_seconds(now: DateTime<Local>, other: DateTime<Local>) -> u64 {
    let diff = now.signed_duration_since(other);
    diff.num_seconds().try_into().unwrap_or_default()
}

fn last_component(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn parse_local_datetime(match_key: &str, value: &str) -> Result<DateTime<Local>, GitRewriteError> {
    let naive = NaiveDateTime::parse_from_str(value, "%m/%d/%y %I:%M %p").map_err(|source| {
        GitRewriteError::DateParse {
            match_key: match_key.to_string(),
            value: value.to_string(),
            source,
        }
    })?;
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => Ok(dt),
        LocalResult::None => Err(GitRewriteError::DateOutOfRange {
            match_key: match_key.to_string(),
            value: value.to_string(),
        }),
    }
}

fn match_key_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value {
        toml::Value::String(s) => Ok(s),
        toml::Value::Integer(i) => Ok(i.to_string()),
        other => Err(serde::de::Error::custom(format!(
            "match-key must be string or integer, found {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write as _;
    use std::time::SystemTime;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use tempfile::tempdir;

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
            "#!/usr/bin/env bash\ncat <<'JSON'\n[\n  {{ \"commit_hash\": \"abc\", \"dt\": \"01/01/24 01:00 PM\" }},\n  {{ \"commit_hash\": \"def\", \"dt\": \"01/02/24 01:30 PM\" }},\n  {{ \"commit_hash\": \"abc\", \"dt\": \"01/02/24 01:30 PM\" }}\n]\nJSON"
        )
        .expect("write script");
        script.sync_all().expect("flush script");
        drop(script);
        let mut perms = fs::metadata(&script_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("perm");

        let now_dt = parse_local_datetime("test", "01/03/24 01:30 PM").expect("now parse");
        let clock = FixedClock(now_dt.into());

        let entries =
            collect_git_rewrite_entries(&config_path, &script_path, &clock).expect("entries");

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
}
