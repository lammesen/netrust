use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use nauto_cli::{
    approvals, bench, compliance, gitops, integrations, job_runner, marketplace, notifications,
    observability, plugins, scheduler, telemetry, transactions, tui, worker,
};
use nauto_model::{Credential, CredentialRef, TaskStatus};
use nauto_security::{CredentialStore, KeyringStore};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing_subscriber::EnvFilter;

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
        #[arg(long, default_value_t = false, help = "Disable CLI progress indicator")]
        no_progress: bool,
    },
    /// Store credentials securely using the OS keychain
    Creds {
        #[arg(long)]
        name: String,
        #[arg(long)]
        username: String,
        #[arg(
            long,
            help = "Provide the password directly (not recommended; use only in CI)",
            conflicts_with_all = ["password_stdin", "password_prompt"]
        )]
        password: Option<String>,
        #[arg(
            long = "password-stdin",
            default_value_t = false,
            help = "Read the password from STDIN (trailing newlines are trimmed)",
            conflicts_with = "password_prompt"
        )]
        password_stdin: bool,
        #[arg(
            long = "password-prompt",
            default_value_t = false,
            help = "Force an interactive password prompt even if STDIN is piped"
        )]
        password_prompt: bool,
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

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let _plugin_host = plugins::load_installed(Path::new("marketplace/plugins"));
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            job,
            inventory,
            audit_log,
            dry_run,
            no_progress,
        } => {
            let mut progress = if no_progress {
                None
            } else {
                Some(ProgressBar::start("Executing job"))
            };
            let (_job, result) = job_runner::run_job(&job, &inventory, &audit_log, dry_run).await?;
            if let Some(mut spinner) = progress.take() {
                spinner.stop();
                println!();
            }
            println!(
                "Job complete: success={} failed={}",
                result.success_count(),
                result.device_results.len() - result.success_count()
            );
            let failed: Vec<_> = result
                .device_results
                .iter()
                .filter(|task| task.status == TaskStatus::Failed)
                .map(|task| task.device_id.clone())
                .collect();
            if !failed.is_empty() {
                println!("Failed devices: {}", failed.join(", "));
            }
            println!("Audit log: {}", audit_log.display());
        }
        Commands::Creds {
            name,
            username,
            password,
            password_stdin,
            password_prompt,
        } => {
            let password_value = resolve_password(password, password_stdin, password_prompt)
                .context("password input")?;
            store_credentials(name, username, password_value).await?
        }
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

async fn store_credentials(name: String, username: String, password: String) -> Result<()> {
    let store = KeyringStore::new("netrust");
    let reference = CredentialRef { name };
    let credential = Credential::UserPassword { username, password };
    store.store(&reference, &credential).await?;
    println!("Stored credential {}", reference.name);
    Ok(())
}

fn resolve_password(
    password_flag: Option<String>,
    password_stdin: bool,
    password_prompt: bool,
) -> Result<String> {
    if let Some(value) = password_flag {
        eprintln!("warning: --password exposes secrets via argv; prefer --password-prompt or --password-stdin");
        return Ok(value);
    }

    if password_stdin {
        return read_password_from_stdin();
    }

    if password_prompt {
        return prompt_for_password();
    }

    if std::io::stdin().is_terminal() {
        return prompt_for_password();
    }

    bail!(
        "stdin is not a TTY; provide --password-stdin for automation or --password-prompt to force interactive entry"
    );
}

fn prompt_for_password() -> Result<String> {
    let password = rpassword::prompt_password("Credential password: ")
        .context("reading password interactively")?;
    if password.is_empty() {
        bail!("password cannot be empty");
    }
    Ok(password)
}

fn read_password_from_stdin() -> Result<String> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("reading password from stdin")?;
    let password = buffer
        .trim_end_matches(|c| c == '\n' || c == '\r')
        .to_string();
    if password.is_empty() {
        bail!("password from stdin cannot be empty");
    }
    Ok(password)
}

async fn run_tui(inventory_path: PathBuf) -> Result<()> {
    tui::launch(inventory_path).await
}

struct ProgressBar {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ProgressBar {
    fn start(message: &str) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let label = message.to_string();
        let thread_stop = stop.clone();
        let handle = thread::spawn(move || {
            let frames = ["|", "/", "-", "\\"];
            let mut idx = 0usize;
            while !thread_stop.load(Ordering::SeqCst) {
                print!("\r{} {}", label, frames[idx % frames.len()]);
                let _ = std::io::stdout().flush();
                idx = (idx + 1) % frames.len();
                thread::sleep(Duration::from_millis(200));
            }
            print!("\r{:width$}\r", "", width = label.len() + 2);
            let _ = std::io::stdout().flush();
        });

        Self {
            stop,
            handle: Some(handle),
        }
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        self.stop();
    }
}
