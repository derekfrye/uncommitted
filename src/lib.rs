#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

mod types;
mod system;
mod git;
mod scan;
mod report;
pub mod output;

pub use types::{Options, PushableEntry, ReportData, StagedEntry, UncommittedEntry};
pub use system::{Clock, DefaultClock, DefaultFsOps, FsOps};
pub use git::{DefaultGitRunner, GitRunner};
pub use report::{collect_report_data, generate_report, humanize_age_public};
