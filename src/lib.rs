#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

mod git;
pub mod output;
mod report;
mod scan;
mod system;
mod types;

pub use git::{DefaultGitRunner, GitRunner};
pub use report::{collect_report_data, generate_report, humanize_age_public};
pub use system::{Clock, DefaultClock, DefaultFsOps, FsOps};
pub use types::{Options, PushableEntry, ReportData, StagedEntry, UncommittedEntry};
