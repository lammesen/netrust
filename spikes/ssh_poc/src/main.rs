use anyhow::Result;
use clap::Parser;

#[cfg(feature = "real-ssh")]
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};

#[derive(Parser, Debug)]
struct Args {
    /// Device hostname or IP
    #[arg(short, long)]
    host: String,

    /// SSH username
    #[arg(short, long)]
    username: String,

    /// Password (use only in lab setups)
    #[arg(short, long)]
    password: String,

    /// Command to execute once connected
    #[arg(short, long, default_value = "show version")]
    command: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    run(args).await
}

#[cfg(feature = "real-ssh")]
async fn run(args: Args) -> Result<()> {
    let client = Client::connect(
        (args.host.as_str(), 22),
        &args.username,
        AuthMethod::with_password(&args.password),
        ServerCheckMethod::NoCheck,
    )
    .await?;

    let result = client.execute(&args.command).await?;
    println!("--- device output ---\n{}", result.stdout);
    if !result.stderr.is_empty() {
        eprintln!("stderr:\n{}", result.stderr);
    }
    println!("exit status: {}", result.exit_status);

    Ok(())
}

#[cfg(not(feature = "real-ssh"))]
async fn run(args: Args) -> Result<()> {
    println!(
        "[stub] SSH demo skipped for {} (enable --features real-ssh to talk to devices)",
        args.host
    );
    println!(
        "[stub] would have executed '{}' as {}",
        args.command, args.username
    );
    Ok(())
}
