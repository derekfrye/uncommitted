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
}

#[derive(Debug)]
pub(crate) struct PairBuildOutput {
    pub(crate) pairs: Vec<RepoPair>,
    pub(crate) tracked_paths: Vec<PathBuf>,
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
    build_pairs_with_paths(config).map(|output| output.pairs)
}

pub(crate) fn build_pairs_with_paths(
    config: &GitRewriteConfig,
) -> Result<PairBuildOutput, GitRewriteError> {
    let mut map: HashMap<String, PairBuilder> = HashMap::new();
    for repo in &config.repos {
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

    let mut pairs = Vec::new();
    let mut tracked_paths = Vec::new();
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
        tracked_paths.push(source.path.clone());
        tracked_paths.push(target.path.clone());
        pairs.push(RepoPair {
            key,
            source,
            target,
        });
    }
    pairs.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(PairBuildOutput {
        pairs,
        tracked_paths,
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
