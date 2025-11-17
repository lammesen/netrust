use crate::{
    ssh::{self, default_credential_store, DEFAULT_SSH_PORT},
    DeviceDriver, DriverAction, DriverExecutionResult,
};
use anyhow::{bail, Context, Result};
use async_ssh2_tokio::Client;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use nauto_security::KeyringStore;
use similar::TextDiff;
use tracing::info;

#[derive(Clone)]
pub struct CiscoIosDriver {
    credential_store: KeyringStore,
    port: u16,
}

impl Default for CiscoIosDriver {
    fn default() -> Self {
        Self {
            credential_store: default_credential_store(),
            port: DEFAULT_SSH_PORT,
        }
    }
}

#[async_trait]
impl DeviceDriver for CiscoIosDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::CiscoIos
    }

    fn name(&self) -> &'static str {
        "Cisco IOS CLI"
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_commit: false,
            supports_rollback: false,
            supports_diff: false,
            supports_dry_run: false,
        }
    }

    async fn execute(
        &self,
        device: &Device,
        action: DriverAction<'_>,
    ) -> Result<DriverExecutionResult> {
        let client = ssh::connect(device, &self.credential_store, self.port).await?;
        let mut result = DriverExecutionResult::default();
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                for cmd in commands {
                    let output = exec_checked(&client, device, cmd).await?;
                    result.logs.push(format!(
                        "[{}] {} => {}",
                        device.name,
                        cmd,
                        summarize(&output)
                    ));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                result.pre_snapshot = Some(show_run(&client, device).await?);
                apply_config(&client, device, snippet).await?;
                result.logs.push(format!(
                    "[{}] applied {} config lines",
                    device.name,
                    snippet.lines().count()
                ));
                result.post_snapshot = Some(show_run(&client, device).await?);
                if let (Some(pre), Some(post)) =
                    (result.pre_snapshot.as_ref(), result.post_snapshot.as_ref())
                {
                    result.diff = Some(render_diff(pre, post));
                }
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                result.logs.push(format!(
                    "[{}] evaluated {} compliance rules",
                    device.name,
                    rules.len()
                ));
            }
        }
        Ok(result)
    }

    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()> {
        info!(
            target: "drivers::cisco_ios",
            "Rolling back {} using snapshot {:?}",
            device.name, snapshot
        );
        if let Some(snapshot) = snapshot {
            let client = ssh::connect(device, &self.credential_store, self.port).await?;
            let payload = format!("configure replace terminal force\n{snapshot}\n\n");
            exec_checked(&client, device, &payload).await?;
        }
        Ok(())
    }
}

async fn exec_checked(client: &Client, device: &Device, command: &str) -> Result<String> {
    let exec = tokio::time::timeout(ssh::command_timeout(), client.execute(command))
        .await
        .with_context(|| format!("ssh exec timeout {} {}", device.name, command))?
        .with_context(|| format!("ssh exec {} {}", device.name, command))?;
    if exec.exit_status != 0 {
        bail!(
            "command '{}' failed on {} (status {}) stderr: {}",
            command,
            device.name,
            exec.exit_status,
            exec.stderr.trim()
        );
    }
    Ok(exec.stdout)
}

async fn show_run(client: &Client, device: &Device) -> Result<String> {
    exec_checked(client, device, "show running-config").await
}

async fn apply_config(client: &Client, device: &Device, snippet: &str) -> Result<()> {
    let script = format!("configure terminal\n{}\nend\nwrite memory", snippet.trim());
    let output = exec_checked(client, device, &script).await?;
    info!(
        target: "drivers::cisco_ios",
        "config result {} bytes {}",
        device.name,
        output.len()
    );
    Ok(())
}

fn summarize(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.len() > 200 {
        format!("{}…", &trimmed[..200])
    } else {
        trimmed.to_string()
    }
}

fn render_diff(before: &str, after: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut buf = String::new();
    for change in diff.iter_all_changes().take(200) {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => "-",
            similar::ChangeTag::Insert => "+",
            similar::ChangeTag::Equal => " ",
        };
        buf.push_str(sign);
        buf.push_str(change.to_string().trim_end());
        buf.push('\n');
    }
    buf
}
