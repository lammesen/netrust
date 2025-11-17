use anyhow::{Context, Result};
use clap::Args;
use nauto_compliance::{ComplianceEngine, DeviceConfigs};
use nauto_model::ComplianceRule;
use serde::Deserialize;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Args)]
pub struct ComplianceCmd {
    #[arg(long)]
    pub rules: PathBuf,
    #[arg(long)]
    pub inputs: PathBuf,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long, default_value = "json", value_parser = ["json", "csv"])]
    pub format: String,
}

#[derive(Debug, Deserialize)]
struct ConfigFile {
    devices: DeviceConfigs,
}

pub fn run(cmd: ComplianceCmd) -> Result<()> {
    let rules: Vec<ComplianceRule> = load_yaml(&cmd.rules).context("failed to load rules file")?;
    let configs: ConfigFile =
        load_yaml(&cmd.inputs).context("failed to load compliance input file")?;

    let outcomes = ComplianceEngine::evaluate(&rules, &configs.devices);
    let output_path = cmd.output;

    match cmd.format.as_str() {
        "csv" => {
            if let Some(path) = output_path {
                let file = fs::File::create(path)?;
                let writer = csv::Writer::from_writer(file);
                ComplianceEngine::export_csv(&outcomes, writer)?;
            } else {
                let writer = csv::Writer::from_writer(io::stdout());
                ComplianceEngine::export_csv(&outcomes, writer)?;
            }
        }
        _ => {
            let json = ComplianceEngine::export_json(&outcomes);
            if let Some(path) = output_path {
                fs::write(path, serde_json::to_string_pretty(&json)?)?;
            } else {
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }
    }

    let summary = ComplianceEngine::summarize(&outcomes);
    eprintln!(
        "Compliance summary -> total: {}, passed: {}, failed: {}",
        summary.total, summary.passed, summary.failed
    );
    Ok(())
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T> {
    let content = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

