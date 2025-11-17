use anyhow::Result;
use nauto_model::{Job, JobResult, TaskStatus};
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
    failed_devices: Vec<String>,
}

#[derive(Serialize)]
struct DeviceAuditRecord<'a> {
    job_id: &'a str,
    device_id: &'a str,
    status: &'a TaskStatus,
    logs: &'a [String],
    diff_present: bool,
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
        failed_devices: result
            .device_results
            .iter()
            .filter(|device| device.status == TaskStatus::Failed)
            .map(|device| device.device_id.clone())
            .collect(),
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.clone())?;
    writeln!(file, "{}", serde_json::to_string(&record)?)?;

    let device_path = device_log_path(&path);
    if let Some(dir) = device_path.parent() {
        create_dir_all(dir)?;
    }
    let mut device_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(device_path)?;
    for device in &result.device_results {
        let record = DeviceAuditRecord {
            job_id: &record.job_id,
            device_id: &device.device_id,
            status: &device.status,
            logs: &device.logs,
            diff_present: device.diff.is_some(),
        };
        writeln!(device_file, "{}", serde_json::to_string(&record)?)?;
    }
    Ok(())
}

fn device_log_path(base: &PathBuf) -> PathBuf {
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("audit");
    let device_name = format!("{stem}.devices.jsonl");
    base.with_file_name(device_name)
}
