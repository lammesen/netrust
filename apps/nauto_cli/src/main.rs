mod audit;
mod bench;
mod approvals;
mod compliance;
mod gitops;
mod integrations;
mod notifications;
mod worker;
mod transactions;
mod observability;
mod telemetry;
mod scheduler;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use nauto_drivers::drivers::{
    AristaEosDriver, CiscoIosDriver, CiscoNxosApiDriver, GenericSshDriver, JuniperJunosDriver,
    MerakiCloudDriver,
};
use nauto_drivers::DriverRegistry;
use nauto_engine::{InMemoryInventory, JobEngine};
use nauto_model::{Credential, CredentialRef, Device, Job, JobKind, TargetSelector};
use nauto_security::{CredentialStore, KeyringStore};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "nauto", about = "Network automation CLI (MVP)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a job definition against the provided inventory
    Run {
        #[arg(long)]
        job: PathBuf,
        #[arg(long)]
        inventory: PathBuf,
        #[arg(long, default_value = "logs/audit.log")]
        audit_log: PathBuf,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Store credentials securely using the OS keychain
    Creds {
        #[arg(long)]
        name: String,
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    /// Launch the terminal UI dashboard
    Tui {
        #[arg(long)]
        inventory: PathBuf,
    },
    /// Run compliance checks and export reports
    Compliance(compliance::ComplianceCmd),
    /// Preview cron-based schedules
    Schedule(scheduler::ScheduleCmd),
    /// Sync configs to Git repository (GitOps)
    GitOps(gitops::GitOpsCmd),
    /// Manage approval workflow
    Approvals(approvals::ApprovalsCmd),
    /// Send workflow notifications
    Notify(notifications::NotifyCmd),
    /// Integrations (NetBox, ServiceNow, etc.)
    Integrations(integrations::IntegrationsCmd),
    /// Interact with plugin marketplace index
    Marketplace(marketplace::MarketplaceCmd),
    /// Run synthetic benchmark against mock drivers
    Bench(bench::BenchCmd),
    /// Plan staged change transactions
    Transactions(transactions::TransactionsCmd),
    /// Process queued jobs as a worker node
    Worker(worker::WorkerCmd),
    /// Emit Prometheus metrics snapshot
    Observability(observability::ObservabilityCmd),
    /// Run telemetry collectors and print snapshot
    Telemetry(telemetry::TelemetryCmd),
}

#[derive(Debug, Deserialize)]
struct InventoryFile {
    devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
struct JobFile {
    name: String,
    #[serde(default = "Uuid::new_v4")]
    id: Uuid,
    kind: JobKind,
    #[serde(default)]
    targets: Option<TargetSelector>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    max_parallel: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            job,
            inventory,
            audit_log,
            dry_run,
        } => run_job(job, inventory, audit_log, dry_run).await?,
        Commands::Creds {
            name,
            username,
            password,
        } => store_credentials(name, username, password).await?,
        Commands::Tui { inventory } => run_tui(inventory).await?,
        Commands::Compliance(cmd) => compliance::run(cmd)?,
        Commands::Schedule(cmd) => scheduler::run(cmd)?,
        Commands::GitOps(cmd) => gitops::run(cmd)?,
        Commands::Approvals(cmd) => approvals::run(cmd)?,
        Commands::Notify(cmd) => notifications::run(cmd).await?,
        Commands::Integrations(cmd) => integrations::run(cmd)?,
        Commands::Marketplace(cmd) => marketplace::run(cmd)?,
        Commands::Bench(cmd) => bench::run(cmd).await?,
        Commands::Transactions(cmd) => transactions::run(cmd)?,
        Commands::Worker(cmd) => worker::run(cmd)?,
        Commands::Observability(cmd) => observability::run(cmd)?,
        Commands::Telemetry(cmd) => telemetry::run(cmd).await?,
    }

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();
}

async fn run_job(job_path: PathBuf, inventory_path: PathBuf, audit_path: PathBuf, dry_run: bool) -> Result<()> {
    let job_file = load_job(&job_path)?;
    let mut job: Job = job_file.into();
    if dry_run {
        job.dry_run = true;
    }
    let inventory = load_inventory(&inventory_path)?;
    let registry = driver_registry();
    let engine = JobEngine::new(InMemoryInventory::new(inventory.devices.clone()), registry);
    info!("Starting job {} ({})", job.name, job.id);
    let result = engine.execute(job.clone()).await?;
    println!(
        "Job complete: success={} failed={}",
        result.success_count(),
        result.device_results.len() - result.success_count()
    );
    audit::record(audit_path, &job, &result)?;
    Ok(())
}

async fn store_credentials(name: String, username: String, password: String) -> Result<()> {
    let store = KeyringStore::new("netrust");
    let reference = CredentialRef { name };
    let credential = Credential::UserPassword { username, password };
    store.store(&reference, &credential).await?;
    println!("Stored credential {}", reference.name);
    Ok(())
}

async fn run_tui(inventory_path: PathBuf) -> Result<()> {
    let inventory = load_inventory(&inventory_path)?;
    tui::launch(inventory.devices).await
}

fn load_inventory(path: &Path) -> Result<InventoryFile> {
    let data = std::fs::read_to_string(path)?;
    let inventory = serde_yaml::from_str(&data)?;
    Ok(inventory)
}

fn load_job(path: &Path) -> Result<JobFile> {
    let data = std::fs::read_to_string(path)?;
    let job = serde_yaml::from_str(&data)?;
    Ok(job)
}

impl From<JobFile> for Job {
    fn from(file: JobFile) -> Job {
        Job {
            id: file.id,
            name: file.name,
            kind: file.kind,
            targets: file.targets.unwrap_or(TargetSelector::All),
            parameters: Default::default(),
            max_parallel: file.max_parallel,
            dry_run: file.dry_run,
        }
    }
}

fn driver_registry() -> DriverRegistry {
    DriverRegistry::new(vec![
        Arc::new(CiscoIosDriver::default()),
        Arc::new(JuniperJunosDriver::default()),
        Arc::new(GenericSshDriver::default()),
        Arc::new(AristaEosDriver::default()),
        Arc::new(CiscoNxosApiDriver::default()),
        Arc::new(MerakiCloudDriver::default()),
    ])
}

