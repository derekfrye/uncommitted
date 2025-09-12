use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn write_executable(path: &PathBuf, content: &str) -> std::io::Result<()> {
    fs::write(path, content)?;
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[test]
fn test01_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary workspace with repos a, b, c.
    let tmp = TempDir::new()?;
    let root = tmp.path();
    for name in ["a", "b", "c"] {
        let dir = root.join(name);
        fs::create_dir_all(dir.join(".git"))?; // marker for repo
    }

    // Create a fake `git` to inject expected behavior.
    let fakebin = root.join("fakebin");
    fs::create_dir_all(&fakebin)?;
    let git_path = fakebin.join("git");
    let script = r#"#!/usr/bin/env bash
set -euo pipefail

# Parse -C <repo> if provided, collect remaining args
repo=""
args=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -C)
      repo="$2"; shift 2;;
    *)
      args+=("$1"); shift;;
  esac
done

reponame=""
if [[ -n "$repo" ]]; then
  reponame="$(basename "$repo")"
fi

join_args() { local IFS=" "; echo "$*"; }

cmd="${args[0]:-}"

case "$cmd" in
  diff)
    # Handle staged vs unstaged via presence of --cached
    if printf '%s\n' "${args[@]:1}" | grep -q -- "--cached"; then
      # Staged changes
      if [[ "$reponame" == "b" ]]; then
        if printf '%s\n' "${args[@]:1}" | grep -q -- "--quiet"; then
          exit 1
        fi
        if printf '%s\n' "${args[@]:1}" | grep -q -- "--numstat"; then
          printf '40\t0\tb1\n40\t0\tb2\n'
          exit 0
        fi
      fi
      # default clean
      exit 0
    else
      # Unstaged changes
      if [[ "$reponame" == "a" ]]; then
        if printf '%s\n' "${args[@]:1}" | grep -q -- "--quiet"; then
          exit 1
        fi
        if printf '%s\n' "${args[@]:1}" | grep -q -- "--numstat"; then
          printf '25\t0\ta1\n25\t0\ta2\n25\t0\ta3\n'
          exit 0
        fi
      fi
      # default clean
      exit 0
    fi
    ;;

  ls-files)
    # Only count untracked in repo a
    if [[ "$reponame" == "a" ]]; then
      echo "untracked.txt"
      exit 0
    fi
    exit 0
    ;;

  rev-parse)
    # Upstream query
    if [[ "${args[1]:-}" == "--abbrev-ref" && "${args[2]:-}" == "--symbolic-full-name" ]]; then
      if [[ "$reponame" == "c" ]]; then
        echo "origin/main"
        exit 0
      else
        exit 1
      fi
    fi
    # has_commits check
    if [[ "${args[1]:-}" == "--verify" && "${args[2]:-}" == "HEAD" ]]; then
      if [[ "$reponame" == "a" || "$reponame" == "b" ]]; then
        exit 1
      else
        exit 0
      fi
    fi
    exit 1
    ;;

  rev-list)
    # Ahead count
    if [[ "$reponame" == "c" ]]; then
      echo "7"
      exit 0
    else
      echo "0"
      exit 0
    fi
    ;;

  log)
    # Timestamps for earliest and latest
    if [[ "$reponame" == "c" ]]; then
      now=$(date +%s)
      # 1.4 days and 68 minutes ago
      echo $((now - 120960))
      echo $((now - 4080))
      exit 0
    fi
    exit 0
    ;;
esac

# Default: no output, success
exit 0
"#;
    write_executable(&git_path, script)?;

    // PATH override so our fake git is used
    let mut cmd = Command::cargo_bin("uncommitted")?;
    let new_path = format!(
        "{}:{}",
        fakebin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    cmd.env("PATH", new_path)
        .arg(root)
        .arg("--depth")
        .arg("1");

    let output = cmd.assert().success().get_output().stdout.clone();
    let stdout = String::from_utf8(output)?;

    // Validate uncommitted: a has 75 lines across 3 files and 1 untracked
    assert!(
        stdout.contains("uncommitted: a (75 lines, 3 files, 1 untracked)"),
        "stdout was:\n{}",
        stdout
    );
    // Validate staged: b has 80 lines across 2 files
    assert!(
        stdout.contains("staged: b (80 lines, 2 files, 0 untracked)"),
        "stdout was:\n{}",
        stdout
    );
    // Validate pushable: c ahead by 7, with humanized ages
    assert!(
        stdout.contains("pushable: c (7 revs, earliest: 1.4 days ago, latest: 1.1 hr ago)"),
        "stdout was:\n{}",
        stdout
    );

    Ok(())
}
