use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use tokio::time::{sleep, Duration};
use tracing::info;

#[derive(Default)]
pub struct AristaEosDriver;

#[async_trait]
impl DeviceDriver for AristaEosDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::AristaEos
    }

    fn name(&self) -> &'static str {
        "Arista EOS CLI"
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
        let mut res = DriverExecutionResult::default();
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                for cmd in commands {
                    run_cli(device, cmd).await;
                    res.logs.push(format!("[{}] {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                res.pre_snapshot = Some(format!("show running-config for {}", device.name));
                res.logs.push(format!(
                    "[{}] entering config terminal (Arista)",
                    device.name
                ));
                for line in snippet.lines() {
                    run_cli(device, line).await;
                    res.logs.push(format!("{} => {}", device.name, line));
                }
                res.logs.push(format!(
                    "[{}] copy running-config startup-config",
                    device.name
                ));
                res.post_snapshot = Some(format!("post-change config {}", device.name));
                res.diff = Some(format!(
                    "EOS diff placeholder ({} lines)",
                    snippet.lines().count()
                ));
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                res.logs.push(format!(
                    "[{}] evaluated {} compliance rules",
                    device.name,
                    rules.len()
                ));
            }
        }
        Ok(res)
    }

    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()> {
        info!(
            target: "drivers::arista",
            "Rollback requested on {} snapshot {:?}",
            device.name,
            snapshot
        );
        sleep(Duration::from_millis(80)).await;
        Ok(())
    }
}

async fn run_cli(device: &Device, command: &str) {
    info!(
        target: "drivers::arista",
        "device={} command={}",
        device.name,
        command
    );
    sleep(Duration::from_millis(45)).await;
}
