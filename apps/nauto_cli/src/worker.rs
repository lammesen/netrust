use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct WorkerCmd {
    #[arg(long, default_value = "queue/jobs.jsonl")]
    pub queue: PathBuf,
    #[arg(long, default_value_t = 5)]
    pub limit: usize,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct QueueItem {
    job: String,
    inventory: String,
}

pub fn run(cmd: WorkerCmd) -> Result<()> {
    let content = fs::read_to_string(&cmd.queue).context("read queue file")?;
    for (idx, line) in content.lines().filter(|l| !l.is_empty()).take(cmd.limit).enumerate() {
        let item: QueueItem = serde_json::from_str(line)?;
        if cmd.dry_run {
            println!(
                "[{}] DRY-RUN -> job: {}, inventory: {}",
                idx + 1,
                item.job,
                item.inventory
            );
        } else {
            println!(
                "[{}] Dispatching job {} with inventory {}",
                idx + 1,
                item.job,
                item.inventory
            );
            // Future: invoke job engine or enqueue to async executor.
        }
    }
    Ok(())
}

