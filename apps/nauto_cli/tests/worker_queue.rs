use nauto_cli::worker::{process_once, WorkerOptions};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn example_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
        .canonicalize()
        .expect("resolve example path")
}

#[test]
fn worker_processes_queue_entries_with_mock_drivers() {
    let temp = TempDir::new().expect("temp dir");
    let queue_path = temp.path().join("jobs.jsonl");
    let audit_log = temp.path().join("audit.log");
    let results_dir = temp.path().join("results");
    let approvals = temp.path().join("approvals.json");
    fs::write(&approvals, "[]").expect("write approvals");

    let job_path = example_path("examples/jobs/show_version.yaml");
    let inventory_path = example_path("examples/inventory.yaml");

    let entry = json!({
        "job": job_path,
        "inventory": inventory_path,
        "dry_run": true
    });
    fs::write(&queue_path, format!("{entry}\n{entry}\n")).expect("seed queue");

    std::env::set_var("NAUTO_USE_MOCK_DRIVERS", "1");

    let options = WorkerOptions {
        queue: queue_path.clone(),
        limit: 2,
        approvals,
        results_dir,
        audit_log,
    };

    let stats = process_once(&options).expect("process queue");
    assert_eq!(stats.processed, 2);
    assert_eq!(stats.remaining, 0);
}
