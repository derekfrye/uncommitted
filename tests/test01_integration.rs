use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use uncommitted::{generate_report, Clock, DefaultFsOps, FsOps, GitRunner, Options};
use std::os::unix::process::ExitStatusExt as _;

#[test]
fn test01_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary workspace with repos a, b, c.
    let tmp = TempDir::new()?;
    let root = tmp.path();
    for name in ["a", "b", "c"] {
        let dir = root.join(name);
        fs::create_dir_all(dir.join(".git"))?; // marker for repo
    }

    // Mock Git implementation in Rust to simulate repo states.
    struct MockGit;
    impl MockGit {
        fn out_ok(stdout: &str) -> std::io::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            })
        }
        fn out_fail() -> std::io::Result<std::process::Output> {
            Ok(std::process::Output {
                status: std::process::ExitStatus::from_raw(1),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }
    impl GitRunner for MockGit {
        fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
            let reponame = repo.file_name().unwrap().to_string_lossy();
            match args.first().copied().unwrap_or("") {
                "diff" => {
                    let rest = &args[1..];
                    let has_cached = rest.iter().any(|a| *a == "--cached");
                    let has_quiet = rest.iter().any(|a| *a == "--quiet");
                    let has_numstat = rest.iter().any(|a| *a == "--numstat");
                    if has_cached {
                        if reponame == "b" {
                            if has_quiet {
                                return Self::out_fail();
                            }
                            if has_numstat {
                                return Self::out_ok("40\t0\tb1\n40\t0\tb2\n");
                            }
                        }
                        return Self::out_ok("");
                    }
                    if reponame == "a" {
                        if has_quiet {
                            return Self::out_fail();
                        }
                        if has_numstat {
                            return Self::out_ok("25\t0\ta1\n25\t0\ta2\n25\t0\ta3\n");
                        }
                    }
                    Self::out_ok("")
                }
                "ls-files" => {
                    if reponame == "a" {
                        return Self::out_ok("untracked.txt\n");
                    }
                    Self::out_ok("")
                }
                "rev-parse" => {
                    if args.get(0) == Some(&"rev-parse")
                        && args.get(1) == Some(&"--abbrev-ref")
                        && args.get(2) == Some(&"--symbolic-full-name")
                        && args.get(3) == Some(&"@{u}")
                    {
                        if reponame == "c" {
                            return Self::out_ok("origin/main\n");
                        }
                        return Self::out_fail();
                    }
                    if args.get(0) == Some(&"rev-parse")
                        && args.get(1) == Some(&"--verify")
                        && args.get(2) == Some(&"HEAD")
                    {
                        if reponame == "a" || reponame == "b" {
                            return Self::out_fail();
                        }
                        return Self::out_ok("");
                    }
                    Self::out_fail()
                }
                "rev-list" => {
                    if reponame == "c" {
                        return Self::out_ok("7\n");
                    }
                    Self::out_ok("0\n")
                }
                "log" => {
                    if reponame == "c" {
                        let now = 1_000_000_000u64;
                        let earliest = now - 120_960; // 1.4 days
                        let latest = now - 4_080; // 68 minutes
                        let s = format!("{}\n{}\n", earliest, latest);
                        return Self::out_ok(&s);
                    }
                    Self::out_ok("")
                }
                _ => Self::out_ok("")
            }
        }
    }

    // Mock FsOps to allow dependency injection of expansion and repo detection
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

    let opts = Options {
        roots: vec![root.to_path_buf()],
        depth: 1,
        no_untracked: false,
        debug: false,
    };
    let report = generate_report(&opts, &MockFs, &MockGit, &MockClock);

    let expected = concat!(
        "uncommitted: a (75 lines, 3 files, 1 untracked)\n",
        "staged: b (80 lines, 2 files, 0 untracked)\n",
        "pushable: c (7 revs, earliest: 1.4 days ago, latest: 1.1 hr ago)"
    );
    assert_eq!(report, expected);
    Ok(())
}

