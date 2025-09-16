#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use uncommitted::{
    collect_report_data, generate_report, output::{format_tab, to_json, TabStyle}, DefaultClock,
    DefaultFsOps, DefaultGitRunner, Options,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Print,
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

    /// Output format: print (default), tab, or json
    #[arg(long, value_enum, default_value_t = OutputFormat::Print)]
    output: OutputFormat,

    /// Table style to use with --output tab
    #[arg(long, value_enum, default_value_t = TabStyle::Rounded)]
    tab_style: TabStyle,
}

fn main() {
    let args = Args::parse();
    let opts = Options {
        roots: args.roots.clone(),
        depth: args.depth,
        no_untracked: args.no_untracked,
        debug: args.debug,
    };
    let fs = DefaultFsOps;
    let git = DefaultGitRunner;
    let clock = DefaultClock;
    match args.output {
        OutputFormat::Print => {
            let report = generate_report(&opts, &fs, &git, &clock);
            println!("{report}");
        }
        OutputFormat::Tab => {
            let data = collect_report_data(&opts, &fs, &git, &clock);
            let out = format_tab(&data, args.tab_style);
            println!("{out}");
        }
        OutputFormat::Json => {
            let data = collect_report_data(&opts, &fs, &git, &clock);
            let out = to_json(&data);
            println!("{out}");
        }
    }
}
