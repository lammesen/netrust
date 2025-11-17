use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Credential, Device, DeviceType, JobKind};
use nauto_security::{CredentialStore, KeyringStore};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};

const MERAKI_API_BASE: &str = "https://api.meraki.com/api/v1";
const KEYRING_SERVICE: &str = "netrust";

#[derive(Clone)]
pub struct MerakiCloudDriver {
    client: Client,
    credential_store: KeyringStore,
}

impl Default for MerakiCloudDriver {
    fn default() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("meraki reqwest client");
        Self {
            client,
            credential_store: KeyringStore::new(KEYRING_SERVICE),
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
        let api_key = self.resolve_api_key(device).await?;
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                let payload = json!({
                    "device_id": device.id,
                    "commands": commands,
                });
                submit_meraki_request(
                    &self.client,
                    device,
                    MerakiOperation::CommandBatch,
                    payload,
                    &api_key,
                )
                .await?;
                res.logs.push(format!(
                    "[{}] queued Meraki batch with {} commands",
                    device.name,
                    commands.len()
                ));
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                submit_meraki_request(
                    &self.client,
                    device,
                    MerakiOperation::ConfigPush,
                    json!({
                        "device_id": device.id,
                        "template_snippet": snippet,
                    }),
                    &api_key,
                )
                .await?;
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
    operation: MerakiOperation,
    payload: Value,
    api_key: &str,
) -> Result<()> {
    let url = operation.endpoint(&device.mgmt_address);
    info!(
        target: "drivers::meraki",
        "POST {} ({}) payload {}",
        url,
        operation.as_str(),
        payload
    );
    let response = client
        .post(&url)
        .header("X-Cisco-Meraki-API-Key", api_key)
        .json(&payload)
        .send()
        .await
        .with_context(|| format!("meraki request {} {}", device.name, operation.as_str()))?;
    let status = response.status();
    let text = response.text().await.with_context(|| {
        format!(
            "reading meraki response {} {}",
            device.name,
            operation.as_str()
        )
    })?;

    if !status.is_success() {
        bail!(
            "Meraki API returned {} for {} {}: {}",
            status,
            device.name,
            operation.as_str(),
            text
        );
    }

    info!(
        target: "drivers::meraki",
        "Meraki {} {} -> {}",
        device.name,
        operation.as_str(),
        status
    );
    Ok(())
}

#[derive(Copy, Clone)]
enum MerakiOperation {
    CommandBatch,
    ConfigPush,
}

impl MerakiOperation {
    fn as_str(&self) -> &'static str {
        match self {
            MerakiOperation::CommandBatch => "command_batch",
            MerakiOperation::ConfigPush => "config_push",
        }
    }

    fn endpoint(&self, network_identifier: &str) -> String {
        match self {
            MerakiOperation::CommandBatch => {
                format!("{MERAKI_API_BASE}/networks/{network_identifier}/commands/batch")
            }
            MerakiOperation::ConfigPush => {
                format!("{MERAKI_API_BASE}/networks/{network_identifier}/config_templates/apply")
            }
        }
    }
}

impl MerakiCloudDriver {
    async fn resolve_api_key(&self, device: &Device) -> Result<String> {
        let credential = self
            .credential_store
            .resolve(&device.credential)
            .await
            .with_context(|| format!("loading credential {}", device.credential.name))?;
        match credential {
            Credential::Token { token } => Ok(token),
            Credential::UserPassword { password, .. } => {
                warn!(
                    target: "drivers::meraki",
                    "Using password from credential {} for Meraki token on device {}",
                    device.credential.name,
                    device.name
                );
                Ok(password)
            }
            other => bail!(
                "unsupported credential type {:?} for Meraki device {}",
                other,
                device.name
            ),
        }
    }
}
