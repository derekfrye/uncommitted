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
        use GitRewriteError::*;
        match self {
            ConfigRead { path, source } => fmt_config_io(f, "read", path, source),
            ConfigParse { path, source } => fmt_config_io(f, "parse", path, source),
            InvalidConfig { message } => write!(f, "invalid git rewrite config: {message}"),
            CommandIo { source } => write!(f, "failed to launch git_rewrite binary: {source}"),
            CommandFailure {
                match_key,
                status,
                stderr,
            } => fmt_command_failure(f, match_key, status, stderr),
            Json { match_key, source } => write!(
                f,
                "failed to parse git_rewrite output for match-key {match_key}: {source}"
            ),
            UnexpectedJson { match_key, value } => write!(
                f,
                "unexpected git_rewrite payload for match-key {match_key}: {value}"
            ),
            DateParse {
                match_key,
                value,
                source,
            } => fmt_date_parse(f, match_key, value, source),
            DateOutOfRange { match_key, value } => write!(
                f,
                "git_rewrite dt '{value}' for match-key {match_key} did not map to a local timestamp"
            ),
            ParallelInit { source } => {
                write!(f, "failed to initialize git rewrite worker pool: {source}")
            }
        }
    }
}

impl std::error::Error for GitRewriteError {}

fn fmt_config_io(
    f: &mut std::fmt::Formatter<'_>,
    action: &str,
    path: &PathBuf,
    source: &impl std::fmt::Display,
) -> std::fmt::Result {
    write!(
        f,
        "failed to {action} git rewrite config {}: {source}",
        path.display()
    )
}

fn fmt_command_failure(
    f: &mut std::fmt::Formatter<'_>,
    match_key: &str,
    status: &ExitStatus,
    stderr: &str,
) -> std::fmt::Result {
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

fn fmt_date_parse(
    f: &mut std::fmt::Formatter<'_>,
    match_key: &str,
    value: &str,
    source: &chrono::ParseError,
) -> std::fmt::Result {
    write!(
        f,
        "failed to parse git_rewrite dt '{value}' for match-key {match_key}: {source}"
    )
}
