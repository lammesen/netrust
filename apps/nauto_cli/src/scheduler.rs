use anyhow::{Context, Result};
use clap::Args;
use chrono::{DateTime, Utc};
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
}

pub fn run(cmd: ScheduleCmd) -> Result<()> {
    let content = fs::read_to_string(&cmd.file)?;
    let definition: ScheduleFile = serde_yaml::from_str(&content)
        .context("failed to parse schedule definition YAML")?;
    let now = Utc::now();
    for entry in &definition.schedules {
        let schedule = Schedule::from_str(&entry.cron)
            .with_context(|| format!("invalid cron for {}", entry.name))?;
        println!("Schedule: {} (job: {})", entry.name, entry.job);
        print_upcoming(schedule, now, cmd.iterations);
        println!();
    }
    Ok(())
}

fn print_upcoming(schedule: Schedule, start: DateTime<Utc>, iterations: usize) {
    for ts in schedule.after(&start).take(iterations.max(1)) {
        println!("  -> {}", ts);
    }
}

