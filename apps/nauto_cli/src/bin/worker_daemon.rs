use anyhow::Result;
use serde::Deserialize;
use std::{fs, thread, time::Duration};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize)]
struct QueueItem {
    job: String,
    inventory: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let queue_path = std::env::var("NAUTO_QUEUE").unwrap_or_else(|_| "queue/jobs.jsonl".into());
    info!("Starting worker daemon (queue={})", queue_path);

    loop {
        process_queue(&queue_path)?;
        thread::sleep(Duration::from_secs(5));
    }
}

fn process_queue(path: &str) -> Result<()> {
    let body = fs::read_to_string(path).unwrap_or_default();
    for (idx, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let item: QueueItem = serde_json::from_str(line)?;
        info!(
            "Queue item #{} -> job={} inventory={}",
            idx + 1,
            item.job,
            item.inventory
        );
        // Future: invoke JobEngine asynchronously.
    }
    Ok(())
}

