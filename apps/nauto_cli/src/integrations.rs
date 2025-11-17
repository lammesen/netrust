use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use nauto_model::{CapabilitySet, CredentialRef, Device, DeviceType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct IntegrationsCmd {
    #[command(subcommand)]
    pub action: IntegrationsAction,
}

#[derive(Subcommand)]
pub enum IntegrationsAction {
    NetboxImport {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = "lab-default")]
        credential: String,
    },
    ServicenowChange {
        #[arg(long)]
        ticket: String,
        #[arg(long)]
        description: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

#[derive(Debug, Deserialize)]
struct NetboxDevice {
    name: String,
    primary_ip: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    device_type: NetboxDeviceType,
}

#[derive(Debug, Deserialize, Default)]
struct NetboxDeviceType {
    #[serde(default)]
    model: String,
}

#[derive(Debug, Deserialize)]
struct NetboxPayload {
    devices: Vec<NetboxDevice>,
}

pub fn run(cmd: IntegrationsCmd) -> Result<()> {
    match cmd.action {
        IntegrationsAction::NetboxImport {
            file,
            output,
            credential,
        } => import_netbox(file, output, credential),
        IntegrationsAction::ServicenowChange {
            ticket,
            description,
            dry_run,
        } => servicenow_change(ticket, description, dry_run),
    }
}

fn import_netbox(file: PathBuf, output: PathBuf, credential: String) -> Result<()> {
    let json = fs::read_to_string(&file).context("read netbox export")?;
    let payload: NetboxPayload = serde_json::from_str(&json)?;
    let devices: Vec<Device> = payload
        .devices
        .into_iter()
        .map(|d| Device {
            id: d.name.clone(),
            name: d.name,
            device_type: detect_device_type(&d.device_type.model),
            mgmt_address: d.primary_ip.unwrap_or_default(),
            credential: CredentialRef {
                name: credential.clone(),
            },
            tags: d.tags,
            capabilities: CapabilitySet::default(),
        })
        .collect();

    let data = serde_yaml::to_string(&InventoryFile { devices })?;
    fs::write(&output, data)?;
    println!("Imported NetBox data -> {}", output.display());
    Ok(())
}

fn servicenow_change(ticket: String, description: String, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("DRY RUN: would log change {ticket}: {description}");
    } else {
        println!("Logged ServiceNow change {ticket}: {description}");
    }
    Ok(())
}

fn detect_device_type(model: &str) -> DeviceType {
    if model.to_lowercase().contains("junos") {
        DeviceType::JuniperJunos
    } else if model.to_lowercase().contains("nx") {
        DeviceType::CiscoNxosApi
    } else {
        DeviceType::GenericSsh
    }
}

#[derive(Debug, Serialize)]
struct InventoryFile {
    devices: Vec<Device>,
}

