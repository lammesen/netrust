use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use clap::{Args, Subcommand};
use nauto_plugin_sdk::CapabilityMask;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct MarketplaceCmd {
    #[command(subcommand)]
    pub action: MarketplaceAction,
    #[arg(long, default_value = "marketplace/index.json")]
    pub index: PathBuf,
    #[arg(long, default_value = "marketplace/plugins")]
    pub install_dir: PathBuf,
}

#[derive(Subcommand)]
pub enum MarketplaceAction {
    List,
    Install { name: String },
    Verify { file: PathBuf },
}

#[derive(Debug, Deserialize)]
struct MarketplaceIndex {
    plugins: Vec<PluginEntry>,
}

#[derive(Debug, Deserialize)]
struct PluginEntry {
    name: String,
    version: String,
    description: String,
    artifact: String,
    signature: Option<String>,
    capabilities: Vec<String>,
}

pub fn run(cmd: MarketplaceCmd) -> Result<()> {
    let index: MarketplaceIndex =
        serde_json::from_slice(&fs::read(&cmd.index).context("load marketplace index")?)?;

    match cmd.action {
        MarketplaceAction::List => {
            for plugin in &index.plugins {
                println!(
                    "{} v{} - {}\n  capabilities: {:?}\n",
                    plugin.name, plugin.version, plugin.description, plugin.capabilities
                );
            }
        }
        MarketplaceAction::Install { name } => {
            let plugin = index
                .plugins
                .iter()
                .find(|p| p.name == name)
                .context("plugin not found")?;
            fs::create_dir_all(&cmd.install_dir)?;
            let src = Path::new("marketplace").join(&plugin.artifact);
            let dst = cmd.install_dir.join(&plugin.artifact);
            fs::copy(&src, &dst).with_context(|| "copy plugin artifact")?;
            println!("Installed {} -> {}", plugin.name, dst.display());
        }
        MarketplaceAction::Verify { file } => {
            let bytes = fs::read(&file)?;
            let hash = Sha256::digest(&bytes);
            println!("SHA-256: {}", hex::encode(hash));
            if let Some(entry) = index
                .plugins
                .iter()
                .find(|p| cmd.install_dir.join(&p.artifact) == file)
            {
                if let Some(sig) = &entry.signature {
                    println!("Recorded signature: {}", sig);
                } else {
                    println!("No signature recorded for {}", entry.name);
                }
            }
        }
    }

    Ok(())
}

