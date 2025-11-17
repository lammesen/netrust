use anyhow::{Context, Result};
use clap::Args;
use nauto_model::{Device, Job};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct TransactionsCmd {
    #[arg(long)]
    pub job: PathBuf,
    #[arg(long)]
    pub inventory: PathBuf,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long, default_value_t = 5)]
    pub canary_size: usize,
    #[arg(long, default_value_t = 50)]
    pub batch_size: usize,
}

#[derive(Debug, Serialize)]
struct TransactionPlan {
    job_name: String,
    canary: Vec<String>,
    batches: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct InventoryFile {
    devices: Vec<Device>,
}

pub fn run(cmd: TransactionsCmd) -> Result<()> {
    let job: JobDefinition = load_yaml(&cmd.job)?;
    let inventory: InventoryFile = load_yaml(&cmd.inventory)?;
    let mut device_ids: Vec<String> = inventory.devices.iter().map(|d| d.id.clone()).collect();
    device_ids.sort();

    let canary = device_ids
        .iter()
        .take(cmd.canary_size.min(device_ids.len()))
        .cloned()
        .collect::<Vec<_>>();
    let mut rest = device_ids
        .into_iter()
        .skip(canary.len())
        .collect::<Vec<_>>();

    let mut batches = Vec::new();
    while !rest.is_empty() {
        let chunk: Vec<String> = rest
            .drain(..rest.len().min(cmd.batch_size))
            .collect();
        batches.push(chunk);
    }

    let plan = TransactionPlan {
        job_name: job.name,
        canary,
        batches,
    };
    let yaml = serde_yaml::to_string(&plan)?;
    fs::write(&cmd.output, yaml)?;
    println!("Transaction plan written to {}", cmd.output.display());
    Ok(())
}

#[derive(Debug, Deserialize)]
struct JobDefinition {
    name: String,
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let content = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

