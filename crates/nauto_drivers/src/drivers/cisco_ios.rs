use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use tokio::time::{sleep, Duration};
use tracing::info;

#[derive(Default)]
pub struct CiscoIosDriver;

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

    async fn execute(&self, device: &Device, action: DriverAction<'_>) -> Result<DriverExecutionResult> {
        let mut result = DriverExecutionResult::default();
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                for cmd in commands {
                    simulate_cli(device, cmd).await;
                    result.logs.push(format!("[{}] {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                result.pre_snapshot = Some(format!("running-config snapshot for {}", device.name));
                for line in snippet.lines() {
                    simulate_cli(device, line).await;
                    result.logs.push(format!("[{} config] {}", device.name, line));
                }
                result.logs.push(format!("[{}] write memory", device.name));
                result.post_snapshot = Some(format!("running-config after change {}", device.name));
                result.diff = Some(format!("diff -- {} lines changed", snippet.lines().count()));
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
        sleep(Duration::from_millis(100)).await;
        Ok(())
    }
}

async fn simulate_cli(device: &Device, cmd: &str) {
    info!(
        target: "drivers::cisco_ios",
        "device={} cmd={}",
        device.name,
        cmd
    );
    sleep(Duration::from_millis(50)).await;
}

