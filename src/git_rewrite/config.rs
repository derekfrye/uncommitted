use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::error::GitRewriteError;

#[derive(Debug, Deserialize)]
pub(crate) struct GitRewriteConfig {
    #[serde(rename = "repo")]
    repos: Vec<RepoSpec>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RepoSpec {
    #[serde(rename = "repository-path")]
    repository_path: PathBuf,
    #[serde(rename = "repository-branch")]
    repository_branch: String,
    #[serde(rename = "commit-from")]
    commit_from: Option<String>,
    #[serde(rename = "commit-to")]
    commit_to: Option<String>,
    #[serde(rename = "commit-count-lookback")]
    commit_count_lookback: Option<u64>,
    #[serde(rename = "match-key", deserialize_with = "match_key_to_string")]
    match_key: String,
    #[serde(rename = "repo-type")]
    repo_type: RepoType,
    #[serde(
        default,
        rename = "ignore",
        deserialize_with = "deserialize_ignore_flag"
    )]
    ignore: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RepoType {
    Source,
    Target,
}

#[derive(Debug, Default)]
pub(crate) struct PairBuilder {
    source: Option<Endpoint>,
    target: Option<Endpoint>,
    ignored: bool,
    paths: Vec<PathBuf>,
    commit_from: Option<String>,
    commit_to: Option<String>,
    commit_count_lookback: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct Endpoint {
    pub(crate) path: PathBuf,
    pub(crate) branch: String,
}

#[derive(Debug)]
pub(crate) struct RepoPair {
    pub(crate) key: String,
    pub(crate) source: Endpoint,
    pub(crate) target: Endpoint,
    pub(crate) commit_from: Option<String>,
    pub(crate) commit_to: Option<String>,
    pub(crate) commit_count_lookback: Option<u64>,
}

#[derive(Debug)]
pub(crate) struct OtherReposSummary {
    pub(crate) pairs: Vec<RepoPair>,
    pub(crate) tracked_endpoints: Vec<Endpoint>,
    pub(crate) ignored_paths: Vec<PathBuf>,
}

pub(crate) fn load_config(path: &Path) -> Result<GitRewriteConfig, GitRewriteError> {
    let config_text =
        std::fs::read_to_string(path).map_err(|source| GitRewriteError::ConfigRead {
            path: path.to_path_buf(),
            source,
        })?;
    toml::from_str(&config_text).map_err(|source| GitRewriteError::ConfigParse {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn build_pairs(config: &GitRewriteConfig) -> Result<Vec<RepoPair>, GitRewriteError> {
    build_other_repos_summary(config).map(|output| output.pairs)
}

pub(crate) fn build_other_repos_summary(
    config: &GitRewriteConfig,
) -> Result<OtherReposSummary, GitRewriteError> {
    let map = populate_pair_builders(&config.repos)?;
    finalize_pairs(map)
}

fn populate_pair_builders(
    repos: &[RepoSpec],
) -> Result<HashMap<String, PairBuilder>, GitRewriteError> {
    let mut map: HashMap<String, PairBuilder> = HashMap::new();
    for repo in repos {
        let entry = map.entry(repo.match_key.clone()).or_default();
        let endpoint = Endpoint {
            path: repo.repository_path.clone(),
            branch: repo.repository_branch.clone(),
        };
        entry.paths.push(repo.repository_path.clone());
        if repo.ignore {
            entry.ignored = true;
        }
        if entry.ignored {
            continue;
        }
        record_override_field(
            &mut entry.commit_from,
            repo.commit_from.as_deref(),
            "commit-from",
            &repo.match_key,
        )?;
        record_override_field(
            &mut entry.commit_to,
            repo.commit_to.as_deref(),
            "commit-to",
            &repo.match_key,
        )?;
        if let Some(value) = repo.commit_count_lookback {
            match repo.repo_type {
                RepoType::Source => record_override_number(
                    &mut entry.commit_count_lookback,
                    value,
                    "commit-count-lookback",
                    &repo.match_key,
                )?,
                RepoType::Target => {
                    return Err(GitRewriteError::InvalidConfig {
                        message: format!(
                            "commit-count-lookback for match-key {} must be defined on source repo",
                            repo.match_key
                        ),
                    });
                }
            }
        }
        match repo.repo_type {
            RepoType::Source => {
                if entry.source.is_some() {
                    return Err(GitRewriteError::InvalidConfig {
                        message: format!(
                            "multiple source repos defined for match-key {}",
                            repo.match_key
                        ),
                    });
                }
                entry.source = Some(endpoint);
            }
            RepoType::Target => {
                if entry.target.is_some() {
                    return Err(GitRewriteError::InvalidConfig {
                        message: format!(
                            "multiple target repos defined for match-key {}",
                            repo.match_key
                        ),
                    });
                }
                entry.target = Some(endpoint);
            }
        }
    }
    Ok(map)
}

fn record_override_field(
    existing: &mut Option<String>,
    incoming: Option<&str>,
    field: &str,
    key: &str,
) -> Result<(), GitRewriteError> {
    if let Some(value) = incoming {
        if let Some(current) = existing {
            if current != value {
                return Err(GitRewriteError::InvalidConfig {
                    message: format!("{field} for match-key {key} must match between repos"),
                });
            }
        } else {
            *existing = Some(value.to_string());
        }
    }
    Ok(())
}

fn record_override_number(
    existing: &mut Option<u64>,
    incoming: u64,
    field: &str,
    key: &str,
) -> Result<(), GitRewriteError> {
    if let Some(current) = existing {
        if *current != incoming {
            return Err(GitRewriteError::InvalidConfig {
                message: format!("{field} for match-key {key} must match between repos"),
            });
        }
    } else {
        *existing = Some(incoming);
    }
    Ok(())
}

fn finalize_pairs(map: HashMap<String, PairBuilder>) -> Result<OtherReposSummary, GitRewriteError> {
    let mut pairs = Vec::new();
    let mut tracked_endpoints = Vec::new();
    let mut ignored_paths = Vec::new();
    for (key, pair) in map {
        if pair.ignored {
            ignored_paths.extend(pair.paths);
            continue;
        }
        let (Some(source), Some(target)) = (pair.source, pair.target) else {
            return Err(GitRewriteError::InvalidConfig {
                message: format!("match-key {key} must define both source and target repos"),
            });
        };
        tracked_endpoints.push(source.clone());
        tracked_endpoints.push(target.clone());
        pairs.push(RepoPair {
            key,
            source,
            target,
            commit_from: pair.commit_from,
            commit_to: pair.commit_to,
            commit_count_lookback: pair.commit_count_lookback,
        });
    }
    pairs.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(OtherReposSummary {
        pairs,
        tracked_endpoints,
        ignored_paths,
    })
}

fn deserialize_ignore_flag<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<toml::Value>::deserialize(deserializer)?;
    match value {
        None => Ok(false),
        Some(toml::Value::Integer(i)) => Ok(i == 1),
        Some(toml::Value::Boolean(b)) => Ok(b),
        Some(other) => Err(serde::de::Error::custom(format!(
            "ignore must be boolean or integer, found {other}"
        ))),
    }
}

pub(crate) fn match_key_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
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
