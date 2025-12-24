use std::fs;
use std::os::unix::process::ExitStatusExt as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use uncommitted::{
    Clock, DefaultFsOps, FsOps, GitRunner, Options, collect_report_data,
    output::{TabStyle, format_tab},
};

const EXPECTED_OUTPUT: &str = concat!(
    "+ Uncommitted Changes -------+-------+-------+-----------+\n",
    "| Repo | Branch | Upstream   | Lines | Files | Untracked |\n",
    "+------+--------+------------+-------+-------+-----------+\n",
    "| a    | main   | acme/a.git |    75 |     3 |         1 |\n",
    "+------+--------+------------+-------+-------+-----------+\n",
    "+ Staged Changes -------+-------+-----------+\n",
    "| Repo | Branch | Lines | Files | Untracked |\n",
    "+------+--------+-------+-------+-----------+\n",
    "| b    | main   |    80 |     2 |         0 |\n",
    "+------+--------+-------+-------+-----------+\n",
    "+ Pushable Commits -------+----------+--------+\n",
    "| Repo | Branch | Commits | Earliest | Latest |\n",
    "+------+--------+---------+----------+--------+\n",
    "| c    | main   |       7 | 1.4 days | 1.1 hr |\n",
    "+------+--------+---------+----------+--------+"
);

struct MockGit;

impl MockGit {
    fn out_ok(stdout: &str) -> std::process::Output {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    fn out_fail() -> std::process::Output {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

impl GitRunner for MockGit {
    fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
        let reponame = repo.file_name().unwrap().to_string_lossy();
        let output = match args.first().copied().unwrap_or("") {
            "for-each-ref" => {
                // Enumerate local branches and their upstreams
                if reponame == "c" {
                    Self::out_ok("main origin/main\n")
                } else {
                    // repos a and b have no upstreams
                    Self::out_ok("main\n")
                }
            }
            "rev-parse" if args.len() == 3 && args[1] == "--abbrev-ref" && args[2] == "HEAD" => {
                // Return a branch name for all repos
                Self::out_ok("main\n")
            }
            "diff" => {
                let rest = &args[1..];
                let has_cached = rest.contains(&"--cached");
                let has_quiet = rest.contains(&"--quiet");
                let has_numstat = rest.contains(&"--numstat");
                if has_cached {
                    if reponame == "b" {
                        if has_quiet {
                            Self::out_fail()
                        } else if has_numstat {
                            Self::out_ok("40\t0\tb1\n40\t0\tb2\n")
                        } else {
                            Self::out_ok("")
                        }
                    } else {
                        Self::out_ok("")
                    }
                } else if reponame == "a" {
                    if has_quiet {
                        Self::out_fail()
                    } else if has_numstat {
                        Self::out_ok("25\t0\ta1\n25\t0\ta2\n25\t0\ta3\n")
                    } else {
                        Self::out_ok("")
                    }
                } else {
                    Self::out_ok("")
                }
            }
            "ls-files" => {
                if reponame == "a" {
                    Self::out_ok("untracked.txt\n")
                } else {
                    Self::out_ok("")
                }
            }
            "rev-parse" => {
                if args.first() == Some(&"rev-parse")
                    && args.get(1) == Some(&"--abbrev-ref")
                    && args.get(2) == Some(&"--symbolic-full-name")
                    && args.get(3) == Some(&"@{u}")
                {
                    if reponame == "c" {
                        Self::out_ok("origin/main\n")
                    } else {
                        Self::out_fail()
                    }
                } else if args.first() == Some(&"rev-parse")
                    && args.get(1) == Some(&"--verify")
                    && args.get(2) == Some(&"HEAD")
                {
                    if reponame == "a" || reponame == "b" {
                        Self::out_fail()
                    } else {
                        Self::out_ok("")
                    }
                } else {
                    Self::out_fail()
                }
            }
            "config" => {
                if args.get(1) == Some(&"--get") {
                    let url = format!("git@github.com:acme/{reponame}.git\n");
                    Self::out_ok(&url)
                } else {
                    Self::out_fail()
                }
            }
            "rev-list" => {
                if reponame == "c" {
                    Self::out_ok("7\n")
                } else {
                    Self::out_ok("0\n")
                }
            }
            "log" => {
                if reponame == "c" {
                    let now = 1_000_000_000u64;
                    let earliest = now - 120_960; // 1.4 days
                    let latest = now - 4_080; // 68 minutes
                    let s = format!("{earliest}\n{latest}\n");
                    Self::out_ok(&s)
                } else {
                    Self::out_ok("")
                }
            }
            _ => Self::out_ok(""),
        };
        Ok(output)
    }
}

struct MockFs;

impl FsOps for MockFs {
    fn is_repo(&self, dir: &Path) -> bool {
        DefaultFsOps.is_repo(dir)
    }

    fn expand_tilde(&self, p: &Path) -> PathBuf {
        p.to_path_buf()
    }
}

struct MockClock;

impl Clock for MockClock {
    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1_000_000_000)
    }
}

fn create_fixture_repos(root: &Path) -> std::io::Result<()> {
    for name in ["a", "b", "c"] {
        let dir = root.join(name);
        fs::create_dir_all(dir.join(".git"))?; // marker for repo
    }
    Ok(())
}

fn build_report(root: &Path) -> String {
    let opts = Options {
        roots: vec![root.to_path_buf()],
        depth: 1,
        no_untracked: false,
        debug: false,
        refresh_remotes: false,
        git_rewrite_toml: None,
        git_rewrite_path: None,
    };
    let data = collect_report_data(&opts, &MockFs, &MockGit, &MockClock);
    format_tab(&data, TabStyle::Ascii, false)
}

#[test]
fn test01_integration() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = TempDir::new()?;
    create_fixture_repos(tmp.path())?;

    let report = build_report(tmp.path());
    assert_eq!(report, EXPECTED_OUTPUT);
    Ok(())
}
