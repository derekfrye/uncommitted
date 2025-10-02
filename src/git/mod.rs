mod metrics;
mod refs;
mod runner;

pub use runner::{DefaultGitRunner, GitRunner};

pub(crate) use metrics::{has_staged, has_uncommitted, staged_metrics, uncommitted_metrics};
pub(crate) use refs::{
    ahead_count_for_ref_pair, commit_age_bounds_for_ref_pair, current_branch, fetch_remote,
    list_local_branches_with_upstream,
};
