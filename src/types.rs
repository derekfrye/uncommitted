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

#[derive(Debug, Default, Clone)]
pub struct ReportData {
    pub uncommitted: Vec<UncommittedEntry>,
    pub staged: Vec<StagedEntry>,
    pub pushable: Vec<PushableEntry>,
    pub multi_root: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub roots: Vec<std::path::PathBuf>,
    pub depth: usize,
    pub no_untracked: bool,
    pub debug: bool,
    pub refresh_remotes: bool,
}
