use std::path::PathBuf;
use std::process::ExitStatus;

#[derive(Debug)]
pub enum GitRewriteError {
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    InvalidConfig {
        message: String,
    },
    CommandIo {
        source: std::io::Error,
    },
    CommandFailure {
        match_key: String,
        status: ExitStatus,
        stderr: String,
    },
    Json {
        match_key: String,
        source: serde_json::Error,
    },
    UnexpectedJson {
        match_key: String,
        value: serde_json::Value,
    },
    DateParse {
        match_key: String,
        value: String,
        source: chrono::ParseError,
    },
    DateOutOfRange {
        match_key: String,
        value: String,
    },
    ParallelInit {
        source: rayon::ThreadPoolBuildError,
    },
}

impl std::fmt::Display for GitRewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigRead { path, source } => {
                write!(
                    f,
                    "failed to read git rewrite config {}: {source}",
                    path.display()
                )
            }
            Self::ConfigParse { path, source } => {
                write!(
                    f,
                    "failed to parse git rewrite config {}: {source}",
                    path.display()
                )
            }
            Self::InvalidConfig { message } => write!(f, "invalid git rewrite config: {message}"),
            Self::CommandIo { source } => {
                write!(f, "failed to launch git_rewrite binary: {source}")
            }
            Self::CommandFailure {
                match_key,
                status,
                stderr,
            } => {
                if stderr.is_empty() {
                    write!(
                        f,
                        "git_rewrite invocation for match-key {match_key} failed with status {status}"
                    )
                } else {
                    write!(
                        f,
                        "git_rewrite invocation for match-key {match_key} failed with status {status}: {stderr}"
                    )
                }
            }
            Self::Json { match_key, source } => {
                write!(
                    f,
                    "failed to parse git_rewrite output for match-key {match_key}: {source}"
                )
            }
            Self::UnexpectedJson { match_key, value } => {
                write!(
                    f,
                    "unexpected git_rewrite payload for match-key {match_key}: {value}"
                )
            }
            Self::DateParse {
                match_key,
                value,
                source,
            } => {
                write!(
                    f,
                    "failed to parse git_rewrite dt '{value}' for match-key {match_key}: {source}"
                )
            }
            Self::DateOutOfRange { match_key, value } => {
                write!(
                    f,
                    "git_rewrite dt '{value}' for match-key {match_key} did not map to a local timestamp"
                )
            }
            Self::ParallelInit { source } => {
                write!(f, "failed to initialize git rewrite worker pool: {source}")
            }
        }
    }
}

impl std::error::Error for GitRewriteError {}
