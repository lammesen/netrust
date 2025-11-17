use anyhow::Result;
use nauto_cli::worker::{process_once, WorkerOptions};
use std::path::PathBuf;
use std::{thread, time::Duration};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let queue_path = env_path("NAUTO_QUEUE", "queue/jobs.jsonl");
    let limit = env_usize("NAUTO_WORKER_LIMIT", 5);
    let approvals = env_path("NAUTO_APPROVALS_PATH", "approvals/approvals.json");
    let results = env_path("NAUTO_RESULTS_DIR", "queue/results");
    let audit = env_path("NAUTO_WORKER_AUDIT_LOG", "logs/worker_audit.log");

    let options = WorkerOptions {
        queue: queue_path.clone(),
        limit,
        approvals,
        results_dir: results,
        audit_log: audit,
    };

    info!(
        "Starting worker daemon (queue={}, limit={})",
        queue_path.display(),
        limit
    );

    loop {
        match process_once(&options) {
            Ok(stats) => {
                if stats.processed > 0 {
                    info!(
                        "Processed {} queue item(s); {} remaining ({} pending approval)",
                        stats.processed, stats.remaining, stats.pending_approvals
                    );
                }
            }
            Err(err) => error!("Worker iteration failed: {err:?}"),
        }
        thread::sleep(Duration::from_secs(5));
    }
}

fn env_path(var: &str, default: &str) -> PathBuf {
    std::env::var(var)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(default))
}

fn env_usize(var: &str, default: usize) -> usize {
    std::env::var(var)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
