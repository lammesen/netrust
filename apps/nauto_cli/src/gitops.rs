use anyhow::{Context, Result};
use clap::Args;
use git2::{IndexAddOption, Repository};
use nauto_model::Device;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct GitOpsCmd {
    #[arg(long)]
    pub repo: PathBuf,
    #[arg(long)]
    pub inventory: PathBuf,
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
    #[arg(long)]
    pub commit: bool,
    #[arg(long, default_value = "Update desired configs")]
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct InventoryFile {
    devices: Vec<Device>,
}

pub fn run(cmd: GitOpsCmd) -> Result<()> {
    let repo = Repository::open(&cmd.repo).context("failed to open repo path")?;
    let inventory: InventoryFile = load_yaml(&cmd.inventory)?;

    let target_dir = cmd
        .output_dir
        .clone()
        .unwrap_or_else(|| cmd.repo.join("configs"));
    fs::create_dir_all(&target_dir)?;

    for device in &inventory.devices {
        let config = render_config(device);
        let path = target_dir.join(format!("{}.cfg", device.id));
        fs::write(path, config)?;
    }

    if cmd.commit {
        commit_configs(&repo, &cmd.message)?;
    }

    Ok(())
}

fn commit_configs(repo: &Repository, message: &str) -> Result<()> {
    let mut index = repo.index()?;
    index.add_all(["configs"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let parents: Vec<git2::Commit> = repo
        .head()
        .ok()
        .and_then(|head| head.resolve().ok())
        .and_then(|resolved| resolved.peel_to_commit().ok())
        .into_iter()
        .collect();

    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parent_refs,
    )?;
    Ok(())
}

fn render_config(device: &Device) -> String {
    format!(
        "hostname {}\n! managed by netrust\n! mgmt: {}\n! tags: {}\n",
        device.name,
        device.mgmt_address,
        device.tags.join(", ")
    )
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

