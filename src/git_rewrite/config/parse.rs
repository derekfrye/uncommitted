use std::path::Path;

use serde::Deserialize;

use super::types::GitRewriteConfig;
use crate::git_rewrite::error::GitRewriteError;

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

pub(crate) fn deserialize_ignore_flag<'de, D>(deserializer: D) -> Result<bool, D::Error>
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
