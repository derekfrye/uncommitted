 

#[derive(Debug, Clone)]
pub struct UncommittedEntry {
    pub repo: String,
    pub lines: u64,
    pub files: u64,
    pub untracked: u64,
}

#[derive(Debug, Clone)]
pub struct StagedEntry {
    pub repo: String,
    pub lines: u64,
    pub files: u64,
    pub untracked: u64,
}

#[derive(Debug, Clone)]
pub struct PushableEntry {
    pub repo: String,
    pub revs: u64,
    pub earliest_secs: Option<u64>,
    pub latest_secs: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct ReportData {
    pub uncommitted: Vec<UncommittedEntry>,
    pub staged: Vec<StagedEntry>,
    pub pushable: Vec<PushableEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub roots: Vec<std::path::PathBuf>,
    pub depth: usize,
    pub no_untracked: bool,
    pub debug: bool,
}
