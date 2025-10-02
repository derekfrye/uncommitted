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
