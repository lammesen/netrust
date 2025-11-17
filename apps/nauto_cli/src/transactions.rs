use anyhow::{bail, Context, Result};
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
    cmd.ensure_valid()?;
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
        let chunk: Vec<String> = rest.drain(..rest.len().min(cmd.batch_size)).collect();
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

impl TransactionsCmd {
    fn ensure_valid(&self) -> Result<()> {
        if self.canary_size == 0 {
            bail!("canary-size must be greater than zero");
        }
        if self.batch_size == 0 {
            bail!("batch-size must be greater than zero");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_cmd() -> TransactionsCmd {
        TransactionsCmd {
            job: PathBuf::from("job.yaml"),
            inventory: PathBuf::from("inventory.yaml"),
            output: PathBuf::from("output.yaml"),
            canary_size: 5,
            batch_size: 10,
        }
    }

    #[test]
    fn rejects_zero_canary_size() {
        let mut cmd = sample_cmd();
        cmd.canary_size = 0;
        assert!(cmd.ensure_valid().is_err());
    }

    #[test]
    fn rejects_zero_batch_size() {
        let mut cmd = sample_cmd();
        cmd.batch_size = 0;
        assert!(cmd.ensure_valid().is_err());
    }

    #[test]
    fn accepts_positive_values() {
        let cmd = sample_cmd();
        assert!(cmd.ensure_valid().is_ok());
    }
}
