use crate::{config, DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Credential, Device, DeviceType, JobKind};
use nauto_security::{CredentialStore, KeyringStore};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use similar::TextDiff;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct CiscoNxosApiDriver {
    client: Client,
    credential_store: KeyringStore,
}

impl Default for CiscoNxosApiDriver {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(config::http_timeout())
                .build()
                .expect("nxapi client"),
            credential_store: KeyringStore::new("netrust"),
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
        let credentials = self.resolve_credentials(device).await?;
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
                    let reply = self.post(device, payload, &credentials).await?;
                    res.logs
                        .push(format!("NX-OS API {} -> {}", device.name, reply.summary()));
                    res.logs
                        .extend(reply.command_summaries(device.name.as_str()));
                }
            }
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                let before = self
                    .run_show(device, "show running-config", &credentials)
                    .await?;
                res.pre_snapshot = Some(before.clone());
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
                let reply = self.post(device, payload, &credentials).await?;
                res.logs.push(reply.summary());
                res.logs
                    .extend(reply.command_summaries(device.name.as_str()));
                res.logs.push(format!(
                    "[{}] applied NX-OS config via REST ({} lines)",
                    device.name,
                    snippet.lines().count()
                ));
                let after = self
                    .run_show(device, "show running-config", &credentials)
                    .await?;
                res.post_snapshot = Some(after.clone());
                res.diff = Some(render_diff(&before, &after));
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
        match snapshot {
            Some(snapshot) => {
                let credentials = self.resolve_credentials(device).await?;
                info!(
                    target: "drivers::nxos",
                    "Rollback {} using snapshot ({} bytes)",
                    device.name,
                    snapshot.len()
                );
                let payload = json!({
                    "ins_api": {
                        "version": "1.2",
                        "type": "cli_conf",
                        "chunk": "0",
                        "sid": "rollback",
                        "input": snapshot,
                        "output_format": "json"
                    }
                });
                let reply = self.post(device, payload, &credentials).await?;
                info!(
                    target: "drivers::nxos",
                    "Rollback result {} -> {}",
                    device.name,
                    reply.summary()
                );
            }
            None => {
                info!(
                    target: "drivers::nxos",
                    "Rollback requested for {} but no snapshot was provided",
                    device.name
                );
            }
        }
        Ok(())
    }
}

impl CiscoNxosApiDriver {
    async fn resolve_credentials(&self, device: &Device) -> Result<(String, String)> {
        let credential = self
            .credential_store
            .resolve(&device.credential)
            .await
            .with_context(|| format!("loading credential {}", device.credential.name))?;
        match credential {
            Credential::UserPassword { username, password } => Ok((username, password)),
            other => bail!(
                "credential {:?} unsupported for NX-OS device {}",
                other,
                device.name
            ),
        }
    }

    async fn post(
        &self,
        device: &Device,
        payload: Value,
        creds: &(String, String),
    ) -> Result<NxapiResponse> {
        let url = format!("https://{}/ins", device.mgmt_address);
        let retry_limit = config::http_retry_limit();
        for attempt in 0..=retry_limit {
            match self
                .client
                .post(&url)
                .basic_auth(&creds.0, Some(&creds.1))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp
                        .text()
                        .await
                        .with_context(|| format!("nxapi response {}", device.name))?;
                    if !status.is_success() {
                        bail!("NX-OS responded {}: {}", status, body);
                    }
                    let parsed: NxapiEnvelope =
                        serde_json::from_str(&body).with_context(|| "parse nxapi json")?;
                    if !parsed.is_success() {
                        bail!("NX-OS error: {}", body);
                    }
                    return Ok(NxapiResponse { raw: body, parsed });
                }
                Err(err) => {
                    if attempt < retry_limit {
                        warn!(
                            target: "drivers::nxos",
                            "retrying nxapi request {} attempt {} due to {}",
                            device.name,
                            attempt + 1,
                            err
                        );
                        tokio::time::sleep(Duration::from_millis(200 * (attempt as u64 + 1))).await;
                        continue;
                    } else {
                        return Err(err).with_context(|| format!("nxapi request {}", device.name));
                    }
                }
            }
        }
        Err(anyhow::anyhow!(
            "nxapi retries exhausted for {}",
            device.name
        ))
    }

    async fn run_show(
        &self,
        device: &Device,
        command: &str,
        creds: &(String, String),
    ) -> Result<String> {
        let payload = json!({
            "ins_api": {
                "version": "1.2",
                "type": "cli_show",
                "chunk": "0",
                "sid": "1",
                "input": command,
                "output_format": "json"
            }
        });
        let reply = self.post(device, payload, creds).await?;
        Ok(reply.raw)
    }
}

struct NxapiResponse {
    raw: String,
    parsed: NxapiEnvelope,
}

impl NxapiResponse {
    fn summary(&self) -> String {
        self.parsed
            .ins_api
            .outputs
            .summary()
            .unwrap_or_else(|| "success".into())
    }

    fn command_summaries(&self, device: &str) -> Vec<String> {
        self.parsed.ins_api.outputs.command_messages(device)
    }
}

#[derive(Debug, Deserialize)]
struct NxapiEnvelope {
    #[serde(rename = "ins_api")]
    ins_api: NxapiInner,
}

impl NxapiEnvelope {
    fn is_success(&self) -> bool {
        self.ins_api.outputs.is_success()
    }
}

#[derive(Debug, Deserialize)]
struct NxapiInner {
    outputs: NxapiOutputs,
}

#[derive(Debug, Deserialize)]
struct NxapiOutputs {
    #[serde(default)]
    output: Vec<NxapiOutput>,
}

impl NxapiOutputs {
    fn is_success(&self) -> bool {
        self.output.iter().all(|o| o.is_success())
    }

    fn summary(&self) -> Option<String> {
        self.output
            .iter()
            .map(|o| o.msg.clone().unwrap_or_else(|| "ok".into()))
            .reduce(|a, b| format!("{a}; {b}"))
    }

    fn command_messages(&self, device: &str) -> Vec<String> {
        self.output
            .iter()
            .enumerate()
            .map(|(idx, o)| {
                let msg = o.msg.clone().unwrap_or_else(|| "ok".into());
                let code = o.code.clone().unwrap_or_else(|| "200".into());
                format!("[{}] cmd#{} => code={} msg={}", device, idx, code, msg)
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct NxapiOutput {
    code: Option<String>,
    msg: Option<String>,
}

impl NxapiOutput {
    fn is_success(&self) -> bool {
        self.code
            .as_deref()
            .map(|code| code == "200")
            .unwrap_or(true)
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
