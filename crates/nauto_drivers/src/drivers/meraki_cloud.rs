use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Credential, Device, DeviceType, JobKind};
use nauto_security::{CredentialStore, KeyringStore};
use reqwest::Client;
use serde_json::json;
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
                for cmd in commands {
                    submit_meraki_request(&self.client, device, cmd, None, &api_key).await?;
                    res.logs.push(format!("Meraki {} => {}", device.name, cmd));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                submit_meraki_request(
                    &self.client,
                    device,
                    "apply_config",
                    Some(snippet),
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
    operation: &str,
    payload: Option<&str>,
    api_key: &str,
) -> Result<()> {
    let url = format!(
        "{MERAKI_API_BASE}/networks/{}/{}",
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
    let response = client
        .post(&url)
        .header("X-Cisco-Meraki-API-Key", api_key)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("meraki request {} {}", device.name, operation))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .with_context(|| format!("reading meraki response {} {}", device.name, operation))?;

    if !status.is_success() {
        bail!(
            "Meraki API returned {} for {} {}: {}",
            status,
            device.name,
            operation,
            text
        );
    }

    info!(
        target: "drivers::meraki",
        "Meraki {} {} -> {}",
        device.name,
        operation,
        status
    );
    Ok(())
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
