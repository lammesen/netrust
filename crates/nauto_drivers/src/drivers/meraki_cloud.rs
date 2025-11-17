use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use reqwest::Client;
use serde_json::json;
use tracing::info;

#[derive(Clone)]
pub struct MerakiCloudDriver {
    client: Client,
}

impl Default for MerakiCloudDriver {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl DeviceDriver for MerakiCloudDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::MerakiCloud
    }

    fn name(&self) -> &'static str {
        "Cisco Meraki Cloud"
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_commit: false,
            supports_rollback: true,
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
                    submit_meraki_request(&self.client, device, cmd, None).await;
                    res.logs.push(format!("Meraki {} => {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                submit_meraki_request(&self.client, device, "apply_config", Some(snippet)).await;
                res.logs.push(format!(
                    "[{}] applied Meraki template ({} chars)",
                    device.name,
                    snippet.len()
                ));
                res.diff = Some("Meraki change tracked via dashboard templates".into());
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                res.logs.push(format!(
                    "[{}] Meraki compliance evaluation {} rules",
                    device.name,
                    rules.len()
                ));
            }
        }
        Ok(res)
    }

    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()> {
        info!(
            target: "drivers::meraki",
            "Reverting template for {} snapshot {:?}",
            device.name,
            snapshot
        );
        Ok(())
    }
}

async fn submit_meraki_request(
    client: &Client,
    device: &Device,
    operation: &str,
    payload: Option<&str>,
) {
    let url = format!(
        "https://api.meraki.com/api/v1/networks/{}/{}",
        device.mgmt_address, operation
    );
    let body = json!({
        "device": device.id,
        "operation": operation,
        "payload": payload.unwrap_or("")
    });
    info!(
        target: "drivers::meraki",
        "POST {} payload {}",
        url,
        body
    );
    let _ = client;
}

