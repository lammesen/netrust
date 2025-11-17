use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Args;
use cron::Schedule;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Args)]
pub struct ScheduleCmd {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub iterations: usize,
    #[arg(
        long,
        help = "Optional queue file; when set, occurrences are appended as worker queue items"
    )]
    pub queue: Option<PathBuf>,
    #[arg(
        long,
        default_value = "examples/inventory.yaml",
        help = "Inventory to use when a schedule entry omits an explicit inventory path"
    )]
    pub default_inventory: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ScheduleFile {
    schedules: Vec<ScheduleEntry>,
}

#[derive(Debug, Deserialize)]
struct ScheduleEntry {
    name: String,
    cron: String,
    job: String,
    #[serde(default)]
    inventory: Option<String>,
    #[serde(default)]
    dry_run: bool,
}

pub fn run(cmd: ScheduleCmd) -> Result<()> {
    let content = fs::read_to_string(&cmd.file)?;
    let definition: ScheduleFile =
        serde_yaml::from_str(&content).context("failed to parse schedule definition YAML")?;
    let now = Utc::now();
    let default_inventory_str = cmd.default_inventory.to_string_lossy().to_string();
    for entry in &definition.schedules {
        let schedule = Schedule::from_str(&entry.cron)
            .with_context(|| format!("invalid cron for {}", entry.name))?;
        let inventory_label = entry.inventory.as_deref().unwrap_or(&default_inventory_str);
        println!(
            "Schedule: {} (job: {}, inventory: {})",
            entry.name, entry.job, inventory_label
        );
        print_upcoming(schedule.clone(), now, cmd.iterations);
        if let Some(queue) = &cmd.queue {
            enqueue(
                queue,
                entry,
                schedule,
                cmd.iterations,
                &cmd.default_inventory,
            )?;
        }
        println!();
    }
    Ok(())
}

fn print_upcoming(schedule: Schedule, start: DateTime<Utc>, iterations: usize) {
    for ts in schedule.after(&start).take(iterations.max(1)) {
        println!("  -> {}", ts);
    }
}

fn enqueue(
    queue_path: &PathBuf,
    entry: &ScheduleEntry,
    schedule: Schedule,
    iterations: usize,
    default_inventory: &PathBuf,
) -> Result<()> {
    let inventory = entry
        .inventory
        .clone()
        .unwrap_or_else(|| default_inventory.to_string_lossy().to_string());
    let mut writer = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(queue_path)?;
    for ts in schedule.after(&Utc::now()).take(iterations.max(1)) {
        let payload = serde_json::json!({
            "job": &entry.job,
            "inventory": &inventory,
            "dry_run": entry.dry_run,
            "scheduled_for": ts.to_rfc3339(),
        });
        use std::io::Write;
        writeln!(writer, "{}", payload.to_string())?;
    }
    println!(
        "Enqueued {} occurrence(s) of {} into {}",
        iterations.max(1),
        entry.name,
        queue_path.display()
    );
    Ok(())
}
