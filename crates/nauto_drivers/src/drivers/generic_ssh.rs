use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use tokio::time::{sleep, Duration};
use tracing::info;

#[derive(Default)]
pub struct GenericSshDriver;

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
        let mut res = DriverExecutionResult::default();
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                for cmd in commands {
                    info!(target: "drivers::generic", "{} -> {}", device.name, cmd);
                    sleep(Duration::from_millis(30)).await;
                    res.logs.push(format!("{} => {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                res.logs.push(format!(
                    "[{}] streaming {} config lines",
                    device.name,
                    snippet.lines().count()
                ));
                sleep(Duration::from_millis(35)).await;
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                res.logs.push(format!(
                    "[{}] compliance rules executed {}",
                    device.name,
                    rules.len()
                ));
            }
        }
        Ok(res)
    }

    async fn rollback(&self, _device: &Device, _snapshot: Option<String>) -> Result<()> {
        Ok(())
    }
}
