use anyhow::Result;
use clap::Args;
use reqwest::Client;
use serde_json::json;

#[derive(Args)]
pub struct NotifyCmd {
    #[arg(long)]
    pub channel: String,
    #[arg(long)]
    pub message: String,
    #[arg(long)]
    pub webhook: Option<String>,
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(cmd: NotifyCmd) -> Result<()> {
    if let Some(url) = &cmd.webhook {
        if cmd.dry_run {
            println!("DRY-RUN -> {}: {}", url, cmd.message);
        } else {
            Client::new()
                .post(url)
                .json(&json!({ "text": format!("[{}] {}", cmd.channel, cmd.message) }))
                .send()
                .await?
                .error_for_status()?;
            println!("Notification delivered to {}", url);
        }
    } else {
        println!("[{}] {}", cmd.channel, cmd.message);
    }
    Ok(())
}

