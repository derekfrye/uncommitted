use std::path::PathBuf;

use clap::{Args as ClapArgs, ValueEnum};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GitRewriteConfig {
    #[serde(rename = "repo")]
    pub(crate) repos: Vec<RepoSpec>,
}

#[derive(Debug, Deserialize, ClapArgs)]
#[command(about = "Fields under [[repo]] in git_rewrite TOML.")]
pub(crate) struct RepoSpec {
    /// Path to the git repository (TOML: repository-path)
    #[serde(rename = "repository-path")]
    #[arg(long = "repository-path", value_name = "PATH")]
    pub(crate) repository_path: PathBuf,
    /// Branch name for the repository (TOML: repository-branch)
    #[serde(rename = "repository-branch")]
    #[arg(long = "repository-branch", value_name = "BRANCH")]
    pub(crate) repository_branch: String,
    /// Optional commit-ish lower bound (TOML: commit-from; source repo only)
    #[serde(rename = "commit-from")]
    #[arg(long = "commit-from", value_name = "REV")]
    pub(crate) commit_from: Option<String>,
    /// Optional commit-ish upper bound (TOML: commit-to; source repo only)
    #[serde(rename = "commit-to")]
    #[arg(long = "commit-to", value_name = "REV")]
    pub(crate) commit_to: Option<String>,
    /// Optional commit count lookback (TOML: commit-count-lookback; source repo only)
    #[serde(rename = "commit-count-lookback")]
    #[arg(long = "commit-count-lookback", value_name = "COUNT")]
    pub(crate) commit_count_lookback: Option<u64>,
    /// Match key used to pair source/target repos (TOML: match-key; string or integer)
    #[serde(rename = "match-key", deserialize_with = "super::match_key_to_string")]
    #[arg(long = "match-key", value_name = "KEY")]
    pub(crate) match_key: String,
    /// Repo type (TOML: repo-type; source or target)
    #[serde(rename = "repo-type")]
    #[arg(long = "repo-type", value_enum)]
    pub(crate) repo_type: RepoType,
    /// Ignore this repo entry (TOML: ignore; accepts true/false or 1/0)
    #[serde(
        default,
        rename = "ignore",
        deserialize_with = "super::parse::deserialize_ignore_flag"
    )]
    #[arg(long, action = clap::ArgAction::Set, value_name = "BOOL")]
    pub(crate) ignore: bool,
}

#[derive(Debug, Deserialize, ValueEnum, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RepoType {
    Source,
    Target,
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
