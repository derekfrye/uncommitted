use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct UncommittedEntry {
    pub repo: String,
    pub branch: String,
    pub lines: u64,
    pub files: u64,
    pub untracked: u64,
    // Root as passed on CLI (e.g., "~/src")
    pub root_display: String,
    // Expanded root path for JSON (e.g., "/home/user/src")
    pub root_full: String,
}

#[derive(Debug, Clone)]
pub struct StagedEntry {
    pub repo: String,
    pub branch: String,
    pub lines: u64,
    pub files: u64,
    pub untracked: u64,
    pub root_display: String,
    pub root_full: String,
}

#[derive(Debug, Clone)]
pub struct PushableEntry {
    pub repo: String,
    pub branch: String,
    pub revs: u64,
    pub earliest_secs: Option<u64>,
    pub latest_secs: Option<u64>,
    pub root_display: String,
    pub root_full: String,
}

#[derive(Debug, Clone)]
pub struct GitRewriteEntry {
    pub source_repo: String,
    pub source_branch: String,
    pub source_path: String,
    pub target_repo: String,
    pub target_branch: String,
    pub target_path: String,
    pub commits: u64,
    pub earliest_secs: Option<u64>,
    pub latest_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RepoSummary {
    pub repo: String,
    pub branch: String,
    pub path: PathBuf,
    pub root_display: String,
    pub root_full: String,
    pub head_revs: Option<u64>,
    pub head_earliest_secs: Option<u64>,
    pub head_latest_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UntrackedReason {
    Ignored,
    MissingConfig,
    MissingRepo,
}

#[derive(Debug, Clone)]
pub struct UntrackedRepoEntry {
    pub repo: String,
    pub branch: String,
    pub root_display: String,
    pub root_full: String,
    pub revs: Option<u64>,
    pub earliest_secs: Option<u64>,
    pub latest_secs: Option<u64>,
    pub reason: UntrackedReason,
}

#[derive(Debug, Default, Clone)]
pub struct ReportData {
    pub uncommitted: Vec<UncommittedEntry>,
    pub staged: Vec<StagedEntry>,
    pub pushable: Vec<PushableEntry>,
    pub git_rewrite: Option<Vec<GitRewriteEntry>>,
    pub multi_root: bool,
    pub repos: Vec<RepoSummary>,
    pub untracked_repos: Vec<UntrackedRepoEntry>,
    pub untracked_enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub roots: Vec<std::path::PathBuf>,
    pub depth: usize,
    pub no_untracked: bool,
    pub debug: bool,
    pub refresh_remotes: bool,
    pub git_rewrite_toml: Option<std::path::PathBuf>,
    pub git_rewrite_path: Option<std::path::PathBuf>,
}
