use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, Utc};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::{ThreadPoolBuilder, prelude::*};

use crate::{system::Clock, types::GitRewriteEntry};

use super::{config::RepoPair, error::GitRewriteError, worker};

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

    let label = format!("{:<24}", worker::repo_display_name(&pair.source.path));
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

    let result = worker::run_pair(pair, binary_path, now_local);

    running.store(false, Ordering::Relaxed);
    let _ = updater.join();
    worker_pb.finish_and_clear();
    overall.inc(1);

    result
}

fn update_worker_message(pb: &ProgressBar, label: &str, elapsed: u64) {
    pb.set_message(format!("{label} {elapsed:>5}s"));
}
