use crate::{
    ssh::{self, default_credential_store, DEFAULT_SSH_PORT},
    DeviceDriver, DriverAction, DriverExecutionResult,
};
use anyhow::{bail, Context, Result};
use async_ssh2_tokio::Client;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use nauto_security::KeyringStore;

const MAX_LOG_BYTES: usize = 512;

#[derive(Clone)]
pub struct GenericSshDriver {
    credential_store: KeyringStore,
    port: u16,
}

impl Default for GenericSshDriver {
    fn default() -> Self {
        Self {
            credential_store: default_credential_store(),
            port: DEFAULT_SSH_PORT,
        }
    }
}

#[async_trait]
impl DeviceDriver for GenericSshDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::GenericSsh
    }

    fn name(&self) -> &'static str {
        "Generic SSH"
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

        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                self.run_command_batch(&client, device, commands).await
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                self.push_snippet(&client, device, snippet).await
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                let mut res = DriverExecutionResult::default();
                res.logs.push(format!(
                    "[{}] compliance placeholder executed {} rules",
                    device.name,
                    rules.len()
                ));
                Ok(res)
            }
        }
    }

    async fn rollback(&self, _device: &Device, _snapshot: Option<String>) -> Result<()> {
        Ok(())
    }
}

impl GenericSshDriver {
    async fn run_command_batch(
        &self,
        client: &Client,
        device: &Device,
        commands: &[String],
    ) -> Result<DriverExecutionResult> {
        let mut res = DriverExecutionResult::default();
        for cmd in commands {
            let stdout = exec_and_check(client, device, cmd).await?;
            res.logs.push(format!(
                "[{}] {} => {}",
                device.name,
                cmd,
                summarize(&stdout)
            ));
        }
        Ok(res)
    }

    async fn push_snippet(
        &self,
        client: &Client,
        device: &Device,
        snippet: &str,
    ) -> Result<DriverExecutionResult> {
        let mut res = DriverExecutionResult::default();
        res.logs.push(format!(
            "[{}] streaming {} config lines over SSH",
            device.name,
            snippet.lines().count()
        ));

        let script = format!("configure terminal\n{}\nend\nwrite memory", snippet);
        let output = exec_and_check(client, device, &script).await?;
        res.logs.push(format!(
            "[{}] config committed => {}",
            device.name,
            summarize(&output)
        ));
        Ok(res)
    }
}

async fn exec_and_check(client: &Client, device: &Device, command: &str) -> Result<String> {
    let result = tokio::time::timeout(ssh::command_timeout(), client.execute(command))
        .await
        .with_context(|| format!("ssh exec timeout {} {}", device.name, command))?
        .with_context(|| format!("ssh exec {} {}", device.name, command))?;
    if result.exit_status != 0 {
        bail!(
            "command '{}' failed on {} (status {}) stderr: {}",
            command,
            device.name,
            result.exit_status,
            result.stderr.trim()
        );
    }
    Ok(result.stdout)
}

fn summarize(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return "ok".into();
    }
    if trimmed.len() > MAX_LOG_BYTES {
        format!("{}…", &trimmed[..MAX_LOG_BYTES])
    } else {
        trimmed.to_string()
    }
}
