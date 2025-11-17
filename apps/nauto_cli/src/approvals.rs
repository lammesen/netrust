use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Args)]
pub struct ApprovalsCmd {
    #[command(subcommand)]
    pub action: ApprovalsAction,
    #[arg(long, default_value = "approvals/approvals.json")]
    pub store: PathBuf,
}

#[derive(Subcommand)]
pub enum ApprovalsAction {
    Request {
        #[arg(long)]
        job: PathBuf,
        #[arg(long)]
        requested_by: String,
        #[arg(long)]
        note: Option<String>,
    },
    Approve {
        #[arg(long)]
        id: String,
        #[arg(long)]
        approver: String,
    },
    List,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApprovalRecord {
    id: Uuid,
    job_path: String,
    requested_by: String,
    note: Option<String>,
    status: ApprovalStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum ApprovalStatus {
    Pending,
    Approved,
}

pub fn run(cmd: ApprovalsCmd) -> Result<()> {
    let mut store = ApprovalStore::load(&cmd.store)?;
    match cmd.action {
        ApprovalsAction::Request {
            job,
            requested_by,
            note,
        } => {
            let record = store.add_request(job, requested_by, note)?;
            store.save(&cmd.store)?;
            println!("Approval requested: {}", record.id);
        }
        ApprovalsAction::Approve { id, approver } => {
            store.approve(&id, approver)?;
            store.save(&cmd.store)?;
            println!("Approved {}", id);
        }
        ApprovalsAction::List => {
            for record in &store.records {
                println!(
                    "{} | {} | {:?} | {}",
                    record.id, record.job_path, record.status, record.requested_by
                );
            }
        }
    }
    Ok(())
}

struct ApprovalStore {
    records: Vec<ApprovalRecord>,
}

impl ApprovalStore {
    fn load(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self { records: Vec::new() });
        }
        let content = fs::read_to_string(path)?;
        let records = serde_json::from_str(&content)?;
        Ok(Self { records })
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(&self.records)?;
        fs::write(path, body)?;
        Ok(())
    }

    fn add_request(
        &mut self,
        job_path: PathBuf,
        requested_by: String,
        note: Option<String>,
    ) -> Result<ApprovalRecord> {
        if !job_path.exists() {
            anyhow::bail!("job file {:?} not found", job_path);
        }
        let record = ApprovalRecord {
            id: Uuid::new_v4(),
            job_path: job_path.to_string_lossy().to_string(),
            requested_by,
            note,
            status: ApprovalStatus::Pending,
        };
        self.records.push(record.clone());
        Ok(record)
    }

    fn approve(&mut self, id: &str, approver: String) -> Result<()> {
        let uuid = Uuid::parse_str(id).context("invalid approval id")?;
        for record in &mut self.records {
            if record.id == uuid {
                record.status = ApprovalStatus::Approved;
                record.note
                    .get_or_insert_with(|| format!("Approved by {}", approver));
                return Ok(());
            }
        }
        anyhow::bail!("approval ID {} not found", id);
    }
}

