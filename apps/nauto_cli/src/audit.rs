use anyhow::Result;
use nauto_model::{Job, JobResult};
use serde::Serialize;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize)]
struct AuditRecord<'a> {
    job_id: String,
    job_name: &'a str,
    success: usize,
    failure: usize,
    started_at: String,
    finished_at: String,
}

pub fn record(path: PathBuf, job: &Job, result: &JobResult) -> Result<()> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir)?;
    }

    let record = AuditRecord {
        job_id: job.id.to_string(),
        job_name: &job.name,
        success: result.success_count(),
        failure: result
            .device_results
            .len()
            .saturating_sub(result.success_count()),
        started_at: result.started_at.to_rfc3339(),
        finished_at: result.finished_at.to_rfc3339(),
    };

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(&record)?)?;
    Ok(())
}
