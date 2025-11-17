use crate::{approvals, job_runner};
use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;
use uuid::Uuid;

#[derive(Args)]
pub struct WorkerCmd {
    #[arg(long, default_value = "queue/jobs.jsonl")]
    pub queue: PathBuf,
    #[arg(long, default_value_t = 5)]
    pub limit: usize,
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    #[arg(long, default_value = "approvals/approvals.json")]
    pub approvals: PathBuf,
    #[arg(long, default_value = "queue/results")]
    pub results_dir: PathBuf,
    #[arg(long, default_value = "logs/worker_audit.log")]
    pub audit_log: PathBuf,
}

#[derive(Clone)]
pub struct WorkerOptions {
    pub queue: PathBuf,
    pub limit: usize,
    pub approvals: PathBuf,
    pub results_dir: PathBuf,
    pub audit_log: PathBuf,
}

impl From<&WorkerCmd> for WorkerOptions {
    fn from(cmd: &WorkerCmd) -> Self {
        Self {
            queue: cmd.queue.clone(),
            limit: cmd.limit,
            approvals: cmd.approvals.clone(),
            results_dir: cmd.results_dir.clone(),
            audit_log: cmd.audit_log.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct WorkerStats {
    pub processed: usize,
    pub remaining: usize,
    pub pending_approvals: usize,
}

#[derive(Debug, Deserialize)]
struct QueueItem {
    job: PathBuf,
    inventory: PathBuf,
    #[serde(default)]
    audit_log: Option<PathBuf>,
    #[serde(default)]
    dry_run: bool,
}

pub fn run(cmd: WorkerCmd) -> Result<()> {
    if cmd.dry_run {
        let lines = load_queue_lines(&cmd.queue)?;
        preview(&lines, cmd.limit)?;
        return Ok(());
    }

    let options = WorkerOptions::from(&cmd);
    let stats = process_once(&options)?;
    println!(
        "Processed {} queue item(s); {} remaining in {}",
        stats.processed,
        stats.remaining,
        options.queue.display()
    );
    if stats.pending_approvals > 0 {
        println!(
            "{} item(s) are waiting on approval (see {})",
            stats.pending_approvals,
            options.approvals.display()
        );
    }

    Ok(())
}

pub fn process_once(options: &WorkerOptions) -> Result<WorkerStats> {
    let lines = load_queue_lines(&options.queue)?;
    let runtime = Runtime::new().context("create worker runtime")?;
    let mut remaining = Vec::new();
    let mut processed = Vec::new();
    let mut processed_count = 0usize;
    let mut pending_approvals = 0usize;

    for line in lines {
        if processed_count >= options.limit {
            remaining.push(line.to_string());
            continue;
        }

        let item: QueueItem = match serde_json::from_str(&line) {
            Ok(item) => item,
            Err(err) => {
                println!("Skipping malformed queue entry: {err}");
                continue;
            }
        };

        if let Some(required) = job_requires_approval(&item.job)? {
            if !approvals::is_approved(&options.approvals, &required)? {
                println!(
                    "Pending approval {} for job {:?}; keeping in queue",
                    required, item.job
                );
                remaining.push(line.to_string());
                pending_approvals += 1;
                continue;
            }
        }

        let audit_path = item
            .audit_log
            .clone()
            .unwrap_or_else(|| options.audit_log.clone());

        match runtime.block_on(job_runner::run_job(
            &item.job,
            &item.inventory,
            &audit_path,
            item.dry_run,
        )) {
            Ok((_job, result)) => {
                println!(
                    "Completed job {} -> successes {}",
                    result.job_id,
                    result.success_count()
                );
                persist_result(&options.results_dir, &result)?;
                processed.push(line.to_string());
                processed_count += 1;
            }
            Err(err) => {
                println!("Job {:?} failed: {err:?}; leaving entry in queue", item.job);
                remaining.push(line.to_string());
            }
        }
    }

    persist_queue(&options.queue, &remaining)?;
    append_processed(&options.queue, &processed)?;

    Ok(WorkerStats {
        processed: processed_count,
        remaining: remaining.len(),
        pending_approvals,
    })
}

fn preview(lines: &[String], limit: usize) -> Result<()> {
    for (idx, line) in lines.iter().take(limit).enumerate() {
        let item: QueueItem = serde_json::from_str(line)?;
        println!(
            "[{}] DRY-RUN -> job: {:?}, inventory: {:?}",
            idx + 1,
            item.job,
            item.inventory
        );
    }
    Ok(())
}

fn load_queue_lines(path: &Path) -> Result<Vec<String>> {
    let body = fs::read_to_string(path).unwrap_or_default();
    Ok(body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect())
}

fn job_requires_approval(job_path: &Path) -> Result<Option<Uuid>> {
    let job = job_runner::load_job(job_path)?;
    Ok(job.approval_id)
}

fn persist_result(dir: &Path, result: &nauto_model::JobResult) -> Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("job-{}.json", result.job_id));
    let body = serde_json::to_string_pretty(result)?;
    fs::write(path, body)?;
    Ok(())
}

fn persist_queue(queue: &Path, remaining: &[String]) -> Result<()> {
    let mut body = remaining.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    fs::write(queue, body).with_context(|| "rewrite queue file")
}

fn append_processed(queue: &Path, processed: &[String]) -> Result<()> {
    if processed.is_empty() {
        return Ok(());
    }
    let processed_path = queue.with_file_name(format!(
        "{}.processed",
        queue
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("jobs.jsonl")
    ));
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(processed_path)?;
    for line in processed {
        use std::io::Write;
        writeln!(file, "{line}")?;
    }
    Ok(())
}
