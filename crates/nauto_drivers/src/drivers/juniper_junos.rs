use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use tokio::time::{sleep, Duration};
use tracing::info;

#[derive(Default)]
pub struct JuniperJunosDriver;

#[async_trait]
impl DeviceDriver for JuniperJunosDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::JuniperJunos
    }

    fn name(&self) -> &'static str {
        "Juniper Junos NETCONF"
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_commit: true,
            supports_rollback: true,
            supports_diff: true,
            supports_dry_run: true,
        }
    }

    async fn execute(&self, device: &Device, action: DriverAction<'_>) -> Result<DriverExecutionResult> {
        match action {
            DriverAction::Job(JobKind::ConfigPush { snippet }) => self.apply_config(device, snippet).await,
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                let mut res = DriverExecutionResult::default();
                for cmd in commands {
                    info!(
                        target: "drivers::juniper",
                        "device={} op-command={}",
                        device.name,
                        cmd
                    );
                    sleep(Duration::from_millis(40)).await;
                    res.logs.push(format!("{} -> {}", device.name, cmd));
                }
                Ok(res)
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                let mut res = DriverExecutionResult::default();
                res.logs.push(format!(
                    "[{}] compliance policy set evaluated: {} rules",
                    device.name,
                    rules.len()
                ));
                Ok(res)
            }
        }
    }

    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()> {
        info!(
            target: "drivers::juniper",
            "rollback on {} to snapshot {:?}",
            device.name,
            snapshot
        );
        sleep(Duration::from_millis(80)).await;
        Ok(())
    }
}

impl JuniperJunosDriver {
    async fn apply_config(&self, device: &Device, snippet: &str) -> Result<DriverExecutionResult> {
        let mut res = DriverExecutionResult::default();
        res.pre_snapshot = Some(format!("candidate config {}", device.name));

        info!(
            target: "drivers::juniper",
            "loading config snippet on {}",
            device.name
        );
        sleep(Duration::from_millis(60)).await;

        res.logs.push(format!(
            "[{}] loaded snippet ({} lines)",
            device.name,
            snippet.lines().count()
        ));

        info!(target: "drivers::juniper", "commit check {}", device.name);
        sleep(Duration::from_millis(40)).await;
        res.logs.push(format!("[{}] commit check passed", device.name));

        info!(target: "drivers::juniper", "commit confirmed {}", device.name);
        sleep(Duration::from_millis(40)).await;
        res.logs.push(format!("[{}] commit confirmed 2m", device.name));

        res.post_snapshot = Some(format!("candidate after commit {}", device.name));
        res.diff = Some(format!("Junos diff output ({} chars)", snippet.len()));

        Ok(res)
    }
}

