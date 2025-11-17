#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use nauto_cli::job_runner;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};
use uuid::Uuid;

#[derive(Default)]
struct AppState {
    runs: Mutex<Vec<JobSummary>>,
    schedules: Mutex<Vec<ScheduledJob>>,
    compliance: Mutex<Vec<ComplianceResult>>,
    inventory: Vec<InventoryDevice>,
}

impl AppState {
    fn new() -> Self {
        let inventory = load_inventory_snapshot().unwrap_or_else(|err| {
            eprintln!("Failed to load inventory snapshot: {err:?}");
            Vec::new()
        });
        Self {
            runs: Mutex::new(Vec::new()),
            schedules: Mutex::new(vec![]),
            compliance: Mutex::new(vec![
                ComplianceResult {
                    rule: "SSH version 2".into(),
                    passed: true,
                    affected: vec![],
                },
                ComplianceResult {
                    rule: "NTP configured".into(),
                    passed: false,
                    affected: vec!["core-r1".into(), "agg-eos-1".into()],
                },
            ]),
            inventory,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct InventoryDevice {
    id: String,
    name: String,
    device_type: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct JobSummary {
    id: Uuid,
    name: String,
    job_type: String,
    target: String,
    dry_run: bool,
    success: u32,
    failed: u32,
    unchanged: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ScheduledJob {
    id: Uuid,
    name: String,
    cron: String,
    next_run: String,
}

#[derive(Debug, Clone, Serialize)]
struct ComplianceResult {
    rule: String,
    passed: bool,
    affected: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ComplianceSnapshot {
    generated_at: String,
    results: Vec<ComplianceResult>,
}

#[derive(Debug, Deserialize)]
struct JobWizardRequest {
    name: String,
    job_type: String,
    target: String,
    payload: String,
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct ScheduleRequest {
    name: String,
    cron: String,
}

#[tauri::command]
fn list_inventory(state: State<AppState>) -> Vec<InventoryDevice> {
    state.inventory.clone()
}

#[tauri::command]
async fn create_job(
    state: State<'_, AppState>,
    request: JobWizardRequest,
) -> Result<JobSummary, String> {
    match execute_job_request(request).await {
        Ok(summary) => {
            state
                .runs
                .lock()
                .expect("state poisoned")
                .push(summary.clone());
            Ok(summary)
        }
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
fn list_schedules(state: State<AppState>) -> Vec<ScheduledJob> {
    state.schedules.lock().expect("lock poisoned").clone()
}

#[tauri::command]
fn add_schedule(state: State<AppState>, request: ScheduleRequest) -> ScheduledJob {
    let mut schedules = state.schedules.lock().expect("lock poisoned");
    let job = ScheduledJob {
        id: Uuid::new_v4(),
        name: request.name,
        cron: request.cron.clone(),
        next_run: "Next 01:00 UTC".into(),
    };
    schedules.push(job.clone());
    job
}

#[tauri::command]
fn compliance_snapshot(state: State<AppState>) -> ComplianceSnapshot {
    let results = state.compliance.lock().expect("lock poisoned").clone();
    ComplianceSnapshot {
        generated_at: chrono::Utc::now().to_rfc3339(),
        results,
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            list_inventory,
            create_job,
            list_schedules,
            add_schedule,
            compliance_snapshot
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_title("Network Automation GUI")?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn load_inventory_snapshot() -> Result<Vec<InventoryDevice>> {
    let path = repo_root().join("examples/inventory.yaml");
    let inventory = job_runner::load_inventory(&path)?;
    Ok(inventory
        .devices
        .into_iter()
        .map(|device| InventoryDevice {
            id: device.id,
            name: device.name,
            device_type: format!("{:?}", device.device_type),
            tags: device.tags,
        })
        .collect())
}

async fn execute_job_request(request: JobWizardRequest) -> Result<JobSummary> {
    let temp_dir = tempfile::tempdir().context("tempdir for job")?;
    let job_file = temp_dir.path().join("wizard_job.yaml");
    fs::write(&job_file, render_job_yaml(&request)).context("write wizard job")?;
    let audit_log = temp_dir.path().join("audit.log");
    let inventory = repo_root().join("examples/inventory.yaml");
    std::env::set_var("NAUTO_USE_MOCK_DRIVERS", "1");
    let (_job, result) =
        job_runner::run_job(&job_file, &inventory, &audit_log, request.dry_run).await?;
    let failures = result.device_results.len() - result.success_count();
    Ok(JobSummary {
        id: result.job_id,
        name: request.name,
        job_type: request.job_type,
        target: request.target,
        dry_run: request.dry_run,
        success: result.success_count() as u32,
        failed: failures as u32,
        unchanged: 0,
    })
}

fn render_job_yaml(request: &JobWizardRequest) -> String {
    match request.job_type.to_lowercase().as_str() {
        "config_push" => format!(
            "name: {}\nkind:\n  type: config_push\n  snippet: |\n    {}\ntargets:\n  mode: all\ndry_run: {}\n",
            request.name,
            request.payload.replace('\n', "\n    "),
            request.dry_run
        ),
        _ => format!(
            "name: {}\nkind:\n  type: command_batch\n  commands:\n    - {}\ntargets:\n  mode: all\ndry_run: {}\n",
            request.name,
            request.payload,
            request.dry_run
        ),
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}
