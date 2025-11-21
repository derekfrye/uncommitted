#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};
use uncommitted::{
    DefaultClock, DefaultFsOps, DefaultGitRunner, FsOps, Options, collect_git_rewrite_entries,
    collect_git_rewrite_untracked, collect_report_data,
    output::{TabStyle, format_tab, to_json},
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Tab,
    Json,
}

#[derive(Parser, Debug)]
#[command(version, about = "Report git repo states under roots.")]
struct Args {
    /// Root directories to scan (default: ~/src)
    roots: Vec<PathBuf>,

    /// Directory depth to search (0 = only root itself, 1 = one level of children, etc.)
    #[arg(long, default_value_t = 1)]
    depth: usize,

    /// Ignore untracked files for 'uncommitted'
    #[arg(long)]
    no_untracked: bool,

    /// Print debug info while scanning
    #[arg(long)]
    debug: bool,

    /// Refresh remote tracking refs before computing pushables
    #[arg(long)]
    refresh_remotes: bool,

    /// Output format: tab (default) or json
    #[arg(long, value_enum, default_value_t = OutputFormat::Tab)]
    output: OutputFormat,

    /// Table style to use with --output tab
    #[arg(long, value_enum, default_value_t = TabStyle::Rounded)]
    tab_style: TabStyle,

    /// Path to git rewrite configuration TOML
    #[arg(long, requires = "git_rewrite_path")]
    git_rewrite_toml: Option<PathBuf>,

    /// Path to `git_rewrite` binary
    #[arg(long, requires = "git_rewrite_toml")]
    git_rewrite_path: Option<PathBuf>,

    /// Hide repos whose commits/revs columns are 0
    #[arg(long)]
    omit_repos_up_to_date: bool,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(&args) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), CliError> {
    let fs = DefaultFsOps;
    let git = DefaultGitRunner;
    let clock = DefaultClock;

    let git_rewrite_toml = match args.git_rewrite_toml.as_ref() {
        Some(path) => Some(resolve_path(&fs, path)?),
        None => None,
    };
    let git_rewrite_path = match args.git_rewrite_path.as_ref() {
        Some(path) => Some(resolve_path(&fs, path)?),
        None => None,
    };

    let opts = Options {
        roots: args.roots.clone(),
        depth: args.depth,
        no_untracked: args.no_untracked,
        debug: args.debug,
        refresh_remotes: args.refresh_remotes,
        git_rewrite_toml: git_rewrite_toml.clone(),
        git_rewrite_path: git_rewrite_path.clone(),
    };

    let mut data = collect_report_data(&opts, &fs, &git, &clock);

    if let (Some(config_path), Some(binary_path)) =
        (git_rewrite_toml.as_ref(), git_rewrite_path.as_ref())
    {
        data.untracked_enabled = true;
        let untracked = collect_git_rewrite_untracked(config_path, &data.repos)?;
        data.untracked_repos = untracked;
        let entries = collect_git_rewrite_entries(config_path, binary_path, &clock)?;
        data.git_rewrite = Some(entries);
    }

    match args.output {
        OutputFormat::Tab => {
            let (out, omitted) = format_tab(&data, args.tab_style, args.omit_repos_up_to_date);
            println!("{out}");
            if args.omit_repos_up_to_date {
                println!("{omitted} repos with no changes omitted.");
            }
        }
        OutputFormat::Json => {
            let out = to_json(&data);
            println!("{out}");
        }
    }

    Ok(())
}

fn resolve_path(fs: &DefaultFsOps, path: &Path) -> Result<PathBuf, CliError> {
    let expanded = fs.expand_tilde(path);
    if expanded.is_absolute() {
        return Ok(expanded);
    }
    let cwd = std::env::current_dir()
        .map_err(|e| CliError(format!("failed to resolve current directory: {e}")))?;
    Ok(cwd.join(expanded))
}

#[derive(Debug)]
struct CliError(String);

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CliError {}

impl From<uncommitted::GitRewriteError> for CliError {
    fn from(err: uncommitted::GitRewriteError) -> Self {
        CliError(err.to_string())
    }
}
