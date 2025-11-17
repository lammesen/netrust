use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose, Engine};
use clap::{Args, Subcommand};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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
    #[arg(long, default_value_t = false)]
    pub allow_unsigned: bool,
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
            let bytes = fs::read(&src).with_context(|| format!("read {}", src.display()))?;
            ensure_signature(
                &bytes,
                plugin.signature.as_deref(),
                cmd.allow_unsigned,
                &plugin.name,
            )?;
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
                ensure_signature(
                    &bytes,
                    entry.signature.as_deref(),
                    cmd.allow_unsigned,
                    &entry.name,
                )?;
                println!("Signature verified for {}", entry.name);
            } else {
                println!("File not present in marketplace index");
            }
        }
    }

    Ok(())
}

const MARKETPLACE_PUBKEY_B64: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

fn ensure_signature(
    bytes: &[u8],
    signature: Option<&str>,
    allow_unsigned: bool,
    label: &str,
) -> Result<()> {
    match signature {
        Some(sig) => {
            let key = verifying_key()?;
            let sig_bytes = general_purpose::STANDARD
                .decode(sig.trim())
                .context("decode signature")?;
            let sig_array: [u8; 64] = sig_bytes
                .try_into()
                .map_err(|_| anyhow!("signature must be 64 bytes"))?;
            let signature = Signature::from_bytes(&sig_array);
            key.verify(bytes, &signature)
                .map_err(|err| anyhow!("signature verification failed for {label}: {err}"))
        }
        None if allow_unsigned => Ok(()),
        None => Err(anyhow!(
            "plugin {} is unsigned; rerun with --allow-unsigned to override",
            label
        )),
    }
}

fn verifying_key() -> Result<VerifyingKey> {
    let bytes = general_purpose::STANDARD
        .decode(MARKETPLACE_PUBKEY_B64)
        .context("decode embedded marketplace public key")?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow!("marketplace public key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&array).map_err(|err| anyhow!("invalid public key: {err}"))
}
