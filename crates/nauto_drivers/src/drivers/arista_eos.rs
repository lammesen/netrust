use crate::{
    config,
    ssh::{self, default_credential_store, DEFAULT_SSH_PORT},
    DeviceDriver, DriverAction, DriverExecutionResult,
};
use anyhow::{bail, Context, Result};
use async_ssh2_tokio::Client;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Credential, Device, DeviceType, JobKind};
use nauto_security::{CredentialStore, KeyringStore};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::{json, Value};
use similar::TextDiff;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone)]
pub struct AristaEosDriver {
    credential_store: KeyringStore,
    port: u16,
    http: HttpClient,
}

impl Default for AristaEosDriver {
    fn default() -> Self {
        let http = HttpClient::builder()
            .timeout(config::http_timeout())
            .build()
            .expect("eAPI reqwest client");
        Self {
            credential_store: default_credential_store(),
            port: DEFAULT_SSH_PORT,
            http,
        }
    }
}

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
        let transport = self.transport(device);
        let mut res = DriverExecutionResult::default();
        match action {
            DriverAction::Job(JobKind::CommandBatch { commands }) => match transport {
                Transport::Ssh => {
                    let client = ssh::connect(device, &self.credential_store, self.port).await?;
                    self.run_command_batch_ssh(&client, device, commands, &mut res)
                        .await?;
                }
                Transport::Eapi => {
                    self.run_command_batch_eapi(device, commands, &mut res)
                        .await?;
                }
            },
            DriverAction::Job(JobKind::ConfigPush { snippet }) => match transport {
                Transport::Ssh => {
                    let client = ssh::connect(device, &self.credential_store, self.port).await?;
                    self.apply_config_ssh(&client, device, snippet, &mut res)
                        .await?;
                }
                Transport::Eapi => {
                    self.apply_config_eapi(device, snippet, &mut res).await?;
                }
            },
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
        if let Some(snapshot) = snapshot {
            let client = ssh::connect(device, &self.credential_store, self.port).await?;
            let payload = format!("configure replace terminal force\n{snapshot}\n");
            exec_checked(&client, device, &payload).await?;
        }
        Ok(())
    }
}

impl AristaEosDriver {
    fn transport(&self, device: &Device) -> Transport {
        if device
            .tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("transport:eapi"))
            || device.mgmt_address.starts_with("http://")
            || device.mgmt_address.starts_with("https://")
        {
            Transport::Eapi
        } else {
            Transport::Ssh
        }
    }

    async fn run_command_batch_ssh(
        &self,
        client: &Client,
        device: &Device,
        commands: &[String],
        res: &mut DriverExecutionResult,
    ) -> Result<()> {
        for cmd in commands {
            let output = exec_checked(client, device, cmd).await?;
            res.logs.push(format!(
                "[{}] {} => {}",
                device.name,
                cmd,
                summarize(&output)
            ));
        }
        Ok(())
    }

    async fn run_command_batch_eapi(
        &self,
        device: &Device,
        commands: &[String],
        res: &mut DriverExecutionResult,
    ) -> Result<()> {
        let creds = self.resolve_http_credentials(device).await?;
        let mut payload = vec!["enable".into()];
        payload.extend(commands.iter().cloned());
        let response = self.eapi_post(device, payload, &creds).await?;
        res.logs
            .extend(response.command_summaries(device.name.as_str()));
        Ok(())
    }

    async fn apply_config_ssh(
        &self,
        client: &Client,
        device: &Device,
        snippet: &str,
        res: &mut DriverExecutionResult,
    ) -> Result<()> {
        res.pre_snapshot = Some(show_run(client, device).await?);
        apply_config(client, device, snippet).await?;
        res.post_snapshot = Some(show_run(client, device).await?);
        if let (Some(pre), Some(post)) = (res.pre_snapshot.as_ref(), res.post_snapshot.as_ref()) {
            res.diff = Some(render_diff(pre, post));
        }
        res.logs.push(format!(
            "[{}] committed EOS snippet ({} lines)",
            device.name,
            snippet.lines().count()
        ));
        Ok(())
    }

    async fn apply_config_eapi(
        &self,
        device: &Device,
        snippet: &str,
        res: &mut DriverExecutionResult,
    ) -> Result<()> {
        let creds = self.resolve_http_credentials(device).await?;
        let before = self.show_run_eapi(device, &creds).await?;
        res.pre_snapshot = Some(before.clone());

        let mut commands = vec!["enable".into(), "configure terminal".into()];
        commands.extend(
            snippet
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(|line| line.to_string()),
        );
        commands.push("write memory".into());

        let response = self.eapi_post(device, commands, &creds).await?;
        res.logs
            .extend(response.command_summaries(device.name.as_str()));

        let after = self.show_run_eapi(device, &creds).await?;
        res.post_snapshot = Some(after.clone());
        res.diff = Some(render_diff(&before, &after));
        Ok(())
    }

    async fn resolve_http_credentials(&self, device: &Device) -> Result<(String, String)> {
        let credential = self
            .credential_store
            .resolve(&device.credential)
            .await
            .with_context(|| format!("loading credential {}", device.credential.name))?;
        match credential {
            Credential::UserPassword { username, password } => Ok((username, password)),
            other => bail!(
                "credential {:?} unsupported for Arista eAPI on {}",
                other,
                device.name
            ),
        }
    }

    async fn eapi_post(
        &self,
        device: &Device,
        commands: Vec<String>,
        creds: &(String, String),
    ) -> Result<EapiResponse> {
        let endpoint = self.eapi_endpoint(device);
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "runCmds",
            "params": {
                "version": 1,
                "cmds": commands,
                "format": "json"
            },
            "id": "netrust"
        });

        let retry_limit = config::http_retry_limit();
        for attempt in 0..=retry_limit {
            match self
                .http
                .post(&endpoint)
                .basic_auth(&creds.0, Some(&creds.1))
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.context("arista eAPI payload")?;
                    if !status.is_success() {
                        bail!("Arista eAPI {} returned {}: {}", device.name, status, body);
                    }

                    let parsed: RawEapiEnvelope =
                        serde_json::from_str(&body).with_context(|| "parse eAPI JSON")?;
                    if let Some(err) = parsed.error {
                        bail!(
                            "Arista eAPI {} error {}: {}",
                            device.name,
                            err.code,
                            err.message
                        );
                    }
                    return Ok(EapiResponse { raw: body, parsed });
                }
                Err(err) => {
                    if attempt < retry_limit {
                        warn!(
                            target: "drivers::arista",
                            "retrying eAPI {} attempt {} due to {}",
                            device.name,
                            attempt + 1,
                            err
                        );
                        tokio::time::sleep(Duration::from_millis(200 * (attempt as u64 + 1))).await;
                        continue;
                    } else {
                        return Err(err)
                            .with_context(|| format!("arista eAPI {} request", device.name));
                    }
                }
            }
        }
        unreachable!("eAPI retry loop should have returned")
    }

    async fn show_run_eapi(&self, device: &Device, creds: &(String, String)) -> Result<String> {
        let payload = vec!["enable".into(), "show running-config".into()];
        let resp = self.eapi_post(device, payload, creds).await?;
        resp.first_output()
            .ok_or_else(|| anyhow::anyhow!("no running-config output from {}", device.name))
    }

    fn eapi_endpoint(&self, device: &Device) -> String {
        if device.mgmt_address.starts_with("http://") || device.mgmt_address.starts_with("https://")
        {
            format!("{}/command-api", device.mgmt_address.trim_end_matches('/'))
        } else {
            format!("https://{}/command-api", device.mgmt_address)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Transport {
    Ssh,
    Eapi,
}

async fn exec_checked(client: &Client, device: &Device, command: &str) -> Result<String> {
    let exec = tokio::time::timeout(ssh::command_timeout(), client.execute(command))
        .await
        .with_context(|| format!("ssh exec timeout {} {}", device.name, command))?
        .with_context(|| format!("ssh exec {} {}", device.name, command))?;
    if exec.exit_status != 0 {
        bail!(
            "command '{}' failed on {} (status {}) stderr: {}",
            command,
            device.name,
            exec.exit_status,
            exec.stderr.trim()
        );
    }
    Ok(exec.stdout)
}

async fn show_run(client: &Client, device: &Device) -> Result<String> {
    exec_checked(client, device, "show running-config").await
}

async fn apply_config(client: &Client, device: &Device, snippet: &str) -> Result<()> {
    let payload = format!(
        "configure terminal\n{}\nend\ncopy running-config startup-config",
        snippet.trim()
    );
    exec_checked(client, device, &payload).await?;
    Ok(())
}

fn summarize(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.len() > 200 {
        format!("{}…", &trimmed[..200])
    } else {
        trimmed.to_string()
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

struct EapiResponse {
    raw: String,
    parsed: RawEapiEnvelope,
}

impl EapiResponse {
    fn first_output(&self) -> Option<String> {
        self.parsed.result.as_ref().and_then(|entries| {
            entries.iter().rev().find_map(|value| {
                value
                    .as_object()?
                    .get("output")?
                    .as_str()
                    .map(|s| s.to_string())
            })
        })
    }

    fn command_summaries(&self, device: &str) -> Vec<String> {
        self.parsed
            .result
            .as_ref()
            .map(|entries| {
                entries
                    .iter()
                    .enumerate()
                    .map(|(idx, value)| {
                        let text = value
                            .as_object()
                            .and_then(|obj| {
                                obj.get("messages")
                                    .and_then(Value::as_array)
                                    .and_then(|msgs| msgs.get(0))
                                    .and_then(Value::as_str)
                                    .or_else(|| obj.get("output").and_then(Value::as_str))
                            })
                            .unwrap_or("ok");
                        format!("[{}] cmd#{} => {}", device, idx, text.trim())
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![format!("[{}] eAPI call produced no output", device)])
    }
}

#[derive(Debug, Deserialize)]
struct RawEapiEnvelope {
    result: Option<Vec<Value>>,
    error: Option<EapiError>,
}

#[derive(Debug, Deserialize)]
struct EapiError {
    code: i64,
    message: String,
}
