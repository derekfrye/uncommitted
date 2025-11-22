use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, Utc};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::{ThreadPoolBuilder, prelude::*};
use serde_json::Value;

use crate::{system::Clock, types::GitRewriteEntry};

use super::{
    config::RepoPair,
    error::GitRewriteError,
    time::{compute_bounds, parse_local_datetime},
};

pub(crate) fn collect_entries(
    pairs: Vec<RepoPair>,
    binary_path: &Path,
    clock: &dyn Clock,
) -> Result<Vec<GitRewriteEntry>, GitRewriteError> {
    if pairs.is_empty() {
        return Ok(Vec::new());
    }

    let now_local = current_local_time(clock);
    let (multi, overall, worker_style) = init_progress(pairs.len());
    let thread_pool = build_thread_pool()?;
    let binary_path = binary_path.to_path_buf();
    let multi_for_tasks = Arc::clone(&multi);
    let overall_for_tasks = overall.clone();

    let result = thread_pool.install(|| {
        run_pairs_with_progress(
            pairs,
            &binary_path,
            now_local,
            worker_style,
            multi_for_tasks,
            overall_for_tasks,
        )
    });

    match result {
        Ok(mut entries) => {
            overall.finish_with_message("git rewrite scans complete");
            entries.sort_by(|a, b| {
                (&a.source_repo, &a.target_repo).cmp(&(&b.source_repo, &b.target_repo))
            });
            Ok(entries)
        }
        Err(err) => {
            overall.abandon_with_message("git rewrite scans failed");
            Err(err)
        }
    }
}

fn current_local_time(clock: &dyn Clock) -> DateTime<Local> {
    let now_utc: DateTime<Utc> = clock.now().into();
    now_utc.with_timezone(&Local)
}

fn init_progress(len: usize) -> (Arc<MultiProgress>, ProgressBar, ProgressStyle) {
    let multi = Arc::new(MultiProgress::new());
    let overall = multi.add(ProgressBar::new(len as u64));
    let overall_style =
        ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar());
    overall.set_style(overall_style);
    overall.enable_steady_tick(Duration::from_millis(100));
    overall.set_message("running git rewrite scans");

    let worker_style = ProgressStyle::with_template("{spinner} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());
    (multi, overall, worker_style)
}

fn build_thread_pool() -> Result<rayon::ThreadPool, GitRewriteError> {
    ThreadPoolBuilder::new()
        .num_threads(num_cpus::get())
        .build()
        .map_err(|source| GitRewriteError::ParallelInit { source })
}

fn run_pairs_with_progress(
    pairs: Vec<RepoPair>,
    binary_path: &PathBuf,
    now_local: DateTime<Local>,
    worker_style: ProgressStyle,
    multi: Arc<MultiProgress>,
    overall: ProgressBar,
) -> Result<Vec<GitRewriteEntry>, GitRewriteError> {
    pairs
        .into_par_iter()
        .map(|pair| {
            run_pair_with_progress(
                pair,
                binary_path,
                now_local.clone(),
                worker_style.clone(),
                &multi,
                &overall,
            )
        })
        .collect()
}

fn run_pair_with_progress(
    pair: RepoPair,
    binary_path: &PathBuf,
    now_local: DateTime<Local>,
    worker_style: ProgressStyle,
    multi: &Arc<MultiProgress>,
    overall: &ProgressBar,
) -> Result<GitRewriteEntry, GitRewriteError> {
    let worker_pb = multi.add(ProgressBar::new_spinner());
    worker_pb.set_style(worker_style);
    worker_pb.enable_steady_tick(Duration::from_millis(100));

    let label = format!("{:<24}", last_component(&pair.source.path));
    worker_pb.set_message(format!("{label} {elapsed:>5}s", elapsed = 0));

    let running = Arc::new(AtomicBool::new(true));
    let running_flag = Arc::clone(&running);
    let pb_for_updater = worker_pb.clone();
    let label_for_updater = label.clone();
    let start = Instant::now();

    let updater = thread::spawn(move || {
        while running_flag.load(Ordering::Relaxed) {
            update_worker_message(
                &pb_for_updater,
                &label_for_updater,
                start.elapsed().as_secs(),
            );
            thread::sleep(Duration::from_millis(200));
        }
        update_worker_message(
            &pb_for_updater,
            &label_for_updater,
            start.elapsed().as_secs(),
        );
    });

    let result = run_pair(pair, binary_path, now_local);

    running.store(false, Ordering::Relaxed);
    let _ = updater.join();
    worker_pb.finish_and_clear();
    overall.inc(1);

    result
}

fn update_worker_message(pb: &ProgressBar, label: &str, elapsed: u64) {
    pb.set_message(format!("{label} {elapsed:>5}s"));
}

fn run_pair(
    pair: RepoPair,
    binary_path: &PathBuf,
    now_local: DateTime<Local>,
) -> Result<GitRewriteEntry, GitRewriteError> {
    let output = invoke_git_rewrite(&pair, binary_path)?;
    let values = parse_payload(&pair, &output.stdout)?;
    let (commits, timestamps) = summarize_entries(&values, &pair)?;
    let (earliest_secs, latest_secs) = compute_bounds(&timestamps, now_local);

    let source_repo = last_component(&pair.source.path);
    let target_repo = last_component(&pair.target.path);

    Ok(GitRewriteEntry {
        source_repo,
        source_branch: pair.source.branch.clone(),
        source_path: pair.source.path.display().to_string(),
        target_repo,
        target_branch: pair.target.branch.clone(),
        target_path: pair.target.path.display().to_string(),
        commits,
        earliest_secs,
        latest_secs,
    })
}

fn invoke_git_rewrite(
    pair: &RepoPair,
    binary_path: &PathBuf,
) -> Result<std::process::Output, GitRewriteError> {
    let output = Command::new(binary_path)
        .arg("--source-repository-path")
        .arg(&pair.source.path)
        .arg("--source-repository-branch")
        .arg(&pair.source.branch)
        .arg("--target-repo")
        .arg(&pair.target.path)
        .arg("--target-repo-branch")
        .arg(&pair.target.branch)
        .arg("--commit-from")
        .arg("NEXT")
        .arg("--commit-to")
        .arg("HEAD")
        .arg("--mode")
        .arg("print")
        .arg("--output-format")
        .arg("json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| GitRewriteError::CommandIo { source })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitRewriteError::CommandFailure {
            match_key: pair.key.clone(),
            status: output.status,
            stderr,
        });
    }

    Ok(output)
}

fn parse_payload(pair: &RepoPair, stdout: &[u8]) -> Result<Vec<Value>, GitRewriteError> {
    let parsed: Value = serde_json::from_slice(stdout).map_err(|source| GitRewriteError::Json {
        match_key: pair.key.clone(),
        source,
    })?;
    match parsed {
        Value::Array(entries) => Ok(entries),
        Value::Object(map) => handle_object_payload(pair, map),
        other => Err(GitRewriteError::UnexpectedJson {
            match_key: pair.key.clone(),
            value: other,
        }),
    }
}

fn handle_object_payload(
    pair: &RepoPair,
    map: serde_json::Map<String, Value>,
) -> Result<Vec<Value>, GitRewriteError> {
    if map
        .get("msg")
        .and_then(Value::as_str)
        .is_some_and(|msg| msg.eq_ignore_ascii_case("nothing to do"))
    {
        Ok(Vec::new())
    } else {
        Err(GitRewriteError::UnexpectedJson {
            match_key: pair.key.clone(),
            value: Value::Object(map),
        })
    }
}

fn summarize_entries(
    values: &[Value],
    pair: &RepoPair,
) -> Result<(u64, Vec<DateTime<Local>>), GitRewriteError> {
    let mut unique_commits: HashSet<String> = HashSet::new();
    let mut timestamps: Vec<DateTime<Local>> = Vec::new();

    for value in values {
        if let Some(commit) = value
            .get("commit_hash")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            unique_commits.insert(commit.to_string());
        } else {
            unique_commits.insert(value.to_string());
        }

        let timestamp_field = value.get("original_commit_dt").or_else(|| value.get("dt"));

        if let Some(dt_str) = timestamp_field.and_then(|v| v.as_str())
            && !dt_str.trim().is_empty()
        {
            let dt = parse_local_datetime(&pair.key, dt_str)?;
            timestamps.push(dt);
        }
    }

    let commits = u64::try_from(unique_commits.len()).unwrap_or(u64::MAX);
    Ok((commits, timestamps))
}

fn last_component(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(std::string::ToString::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}
