use std::collections::HashMap;
use std::path::PathBuf;

use super::types::{Endpoint, GitRewriteConfig, OtherReposSummary, RepoPair, RepoSpec, RepoType};
use crate::git_rewrite::error::GitRewriteError;

#[derive(Debug, Default)]
struct PairBuilder {
    source: Option<Endpoint>,
    target: Option<Endpoint>,
    ignored: bool,
    paths: Vec<PathBuf>,
    commit_from: Option<String>,
    commit_to: Option<String>,
    commit_count_lookback: Option<u64>,
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
            record_commit_count(entry, repo, value)?;
        }
        assign_endpoint(entry, endpoint, repo)?;
    }
    Ok(map)
}

fn record_commit_count(
    entry: &mut PairBuilder,
    repo: &RepoSpec,
    value: u64,
) -> Result<(), GitRewriteError> {
    match repo.repo_type {
        RepoType::Source => record_override_number(
            &mut entry.commit_count_lookback,
            value,
            "commit-count-lookback",
            &repo.match_key,
        ),
        RepoType::Target => Err(GitRewriteError::InvalidConfig {
            message: format!(
                "commit-count-lookback for match-key {} must be defined on source repo",
                repo.match_key
            ),
        }),
    }
}

fn assign_endpoint(
    entry: &mut PairBuilder,
    endpoint: Endpoint,
    repo: &RepoSpec,
) -> Result<(), GitRewriteError> {
    match repo.repo_type {
        RepoType::Source => store_endpoint(&mut entry.source, endpoint, "source", &repo.match_key),
        RepoType::Target => store_endpoint(&mut entry.target, endpoint, "target", &repo.match_key),
    }
}

fn store_endpoint(
    slot: &mut Option<Endpoint>,
    endpoint: Endpoint,
    label: &str,
    key: &str,
) -> Result<(), GitRewriteError> {
    if slot.is_some() {
        return Err(GitRewriteError::InvalidConfig {
            message: format!("multiple {label} repos defined for match-key {key}"),
        });
    }
    *slot = Some(endpoint);
    Ok(())
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
