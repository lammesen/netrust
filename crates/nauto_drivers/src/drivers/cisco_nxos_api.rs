use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use reqwest::Client;
use serde_json::json;
use tracing::info;

#[derive(Clone)]
pub struct CiscoNxosApiDriver {
    client: Client,
}

impl Default for CiscoNxosApiDriver {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl DeviceDriver for CiscoNxosApiDriver {
    fn device_type(&self) -> DeviceType {
        DeviceType::CiscoNxosApi
    }

    fn name(&self) -> &'static str {
        "Cisco NX-OS API"
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_commit: true,
            supports_rollback: true,
            supports_diff: true,
            supports_dry_run: true,
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
                    let payload = json!({
                        "ins_api": {
                            "version": "1.2",
                            "type": "cli_show",
                            "chunk": "0",
                            "sid": "1",
                            "input": cmd,
                            "output_format": "json"
                        }
                    });
                    simulate_post(&self.client, device, payload.clone()).await;
                    res.logs.push(format!("NX-OS API {} -> {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                res.pre_snapshot = Some(format!("nxapi show running-config {}", device.name));
                let payload = json!({
                    "ins_api": {
                        "version": "1.2",
                        "type": "cli_conf",
                        "chunk": "0",
                        "sid": "1",
                        "input": snippet,
                        "output_format": "json"
                    }
                });
                simulate_post(&self.client, device, payload.clone()).await;
                res.logs.push(format!(
                    "[{}] applied NX-OS config via REST ({} lines)",
                    device.name,
                    snippet.lines().count()
                ));
                res.post_snapshot = Some(format!("nxapi post-change snapshot {}", device.name));
                res.diff = Some(format!("NX-OS diff placeholder ({})", snippet.len()));
            }
            DriverAction::Job(JobKind::ComplianceCheck { rules }) => {
                res.logs.push(format!(
                    "[{}] NX-OS compliance check {} rules",
                    device.name,
                    rules.len()
                ));
            }
        }
        Ok(res)
    }

    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()> {
        info!(
            target: "drivers::nxos",
            "Rollback requested for {} snapshot {:?}",
            device.name,
            snapshot
        );
        Ok(())
    }
}

async fn simulate_post(client: &Client, device: &Device, payload: serde_json::Value) {
    let url = format!("https://{}/ins", device.mgmt_address);
    info!(
        target: "drivers::nxos",
        "POST {} payload {}",
        url,
        payload
    );
    let _ = client; // placeholder for future actual request
}

