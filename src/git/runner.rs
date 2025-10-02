use std::path::Path;
use std::process::{Command, Output, Stdio};

pub trait GitRunner {
    /// Run the `git` command within the given `repo` with `args`.
    ///
    /// # Errors
    /// Returns an error if the `git` process cannot be spawned or fails during execution.
    fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<Output>;
}

pub struct DefaultGitRunner;

impl GitRunner for DefaultGitRunner {
    fn run_git(&self, repo: &Path, args: &[&str]) -> std::io::Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    }
}
