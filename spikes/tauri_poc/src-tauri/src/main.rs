#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
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
            inventory: vec![
                InventoryDevice {
                    id: "core-r1".into(),
                    name: "Core-R1".into(),
                    device_type: "CiscoIos".into(),
                    tags: vec!["site:oslo".into(), "role:core".into()],
                },
                InventoryDevice {
                    id: "agg-eos-1".into(),
                    name: "Agg-EOS-1".into(),
                    device_type: "AristaEos".into(),
                    tags: vec!["site:oslo".into(), "role:aggregate".into()],
                },
                InventoryDevice {
                    id: "spine-nxapi".into(),
                    name: "Spine-NXAPI".into(),
                    device_type: "CiscoNxosApi".into(),
                    tags: vec!["site:oslo".into(), "role:spine".into()],
                },
                InventoryDevice {
                    id: "meraki-net-1".into(),
                    name: "Meraki-Net-1".into(),
                    device_type: "MerakiCloud".into(),
                    tags: vec!["site:remote".into(), "role:wireless".into()],
                },
            ],
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
fn create_job(state: State<AppState>, request: JobWizardRequest) -> JobSummary {
    let mut runs = state.runs.lock().expect("state poisoned");
    let summary = JobSummary {
        id: Uuid::new_v4(),
        name: request.name,
        job_type: request.job_type,
        target: request.target,
        dry_run: request.dry_run,
        success: 42,
        failed: 3,
        unchanged: 5,
    };
    runs.push(summary.clone());
    summary
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
            if let Some(window) = app.get_window("main") {
                window.set_title("Network Automation GUI")?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

