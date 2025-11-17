#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use tauri::{Manager, State};

#[derive(Default)]
struct AppState {
    runs: std::sync::Mutex<Vec<JobSummary>>,
}

#[derive(Debug, Clone, Serialize)]
struct JobSummary {
    name: String,
    success: u32,
    failed: u32,
    unchanged: u32,
}

#[tauri::command]
fn mock_job_summary(state: State<AppState>) -> JobSummary {
    let mut runs = state.runs.lock().expect("state poisoned");
    let summary = JobSummary {
        name: format!("Dry-run {}", runs.len() + 1),
        success: 48,
        failed: 2,
        unchanged: 10,
    };
    runs.push(summary.clone());
    summary
}

fn main() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![mock_job_summary])
        .setup(|app| {
            let window = app.get_window("main").expect("window exists");
            window.set_title("Network Automation GUI Spike")?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

