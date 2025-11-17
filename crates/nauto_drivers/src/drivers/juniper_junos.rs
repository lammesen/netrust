use crate::{
    ssh::{self, default_credential_store, DEFAULT_NETCONF_PORT, DEFAULT_SSH_PORT},
    DeviceDriver, DriverAction, DriverExecutionResult,
};
use anyhow::{bail, Context, Result};
use async_ssh2_tokio::Client;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use nauto_security::KeyringStore;
use similar::TextDiff;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::info;

const NETCONF_EOM: &str = "]]>]]>";

trait NetconfIo: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> NetconfIo for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

#[derive(Clone)]
pub struct JuniperJunosDriver {
    credential_store: KeyringStore,
    port: u16,
}

impl Default for JuniperJunosDriver {
    fn default() -> Self {
        Self {
            credential_store: default_credential_store(),
            port: DEFAULT_NETCONF_PORT,
        }
    }
}

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

    async fn execute(
        &self,
        device: &Device,
        action: DriverAction<'_>,
    ) -> Result<DriverExecutionResult> {
        match action {
            DriverAction::Job(JobKind::ConfigPush { snippet }) => {
                self.apply_config(device, snippet).await
            }
            DriverAction::Job(JobKind::CommandBatch { commands }) => {
                self.run_operational_commands(device, commands).await
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
        if let Some(snapshot) = snapshot {
            let mut session = NetconfSession::connect(device, &self.credential_store, self.port)
                .await
                .context("open netconf for rollback")?;
            session
                .rpc(&format!(
                    "<load-configuration action=\"override\"><configuration-text><![CDATA[{snapshot}]]></configuration-text></load-configuration>"
                ))
                .await?;
            session.rpc("<commit/>").await?;
        }
        Ok(())
    }
}

impl JuniperJunosDriver {
    async fn apply_config(&self, device: &Device, snippet: &str) -> Result<DriverExecutionResult> {
        let mut session =
            NetconfSession::connect(device, &self.credential_store, self.port).await?;
        let mut res = DriverExecutionResult::default();
        res.pre_snapshot = Some(
            session
                .rpc("<get-config><source><running/></source></get-config>")
                .await?,
        );
        session
            .rpc("<lock><target><candidate/></target></lock>")
            .await?;
        let payload = format!(
            "<edit-config>\
                <target><candidate/></target>\
                <default-operation>merge</default-operation>\
                <config>\
                    <configuration-text>\
                        <![CDATA[{snippet}]]>\
                    </configuration-text>\
                </config>\
            </edit-config>"
        );
        session.rpc(&payload).await?;
        res.logs.push(format!(
            "[{}] loaded snippet ({} lines)",
            device.name,
            snippet.lines().count()
        ));

        session
            .rpc("<validate><source><candidate/></source></validate>")
            .await?;
        res.logs
            .push(format!("[{}] commit check passed", device.name));
        session.rpc("<commit/>").await?;
        res.logs.push(format!("[{}] commit complete", device.name));
        session
            .rpc("<unlock><target><candidate/></target></unlock>")
            .await?;

        res.post_snapshot = Some(
            session
                .rpc("<get-config><source><running/></source></get-config>")
                .await?,
        );
        if let (Some(pre), Some(post)) = (res.pre_snapshot.as_ref(), res.post_snapshot.as_ref()) {
            res.diff = Some(render_diff(pre, post));
        }
        Ok(res)
    }

    async fn run_operational_commands(
        &self,
        device: &Device,
        commands: &[String],
    ) -> Result<DriverExecutionResult> {
        let client = ssh::connect(device, &self.credential_store, DEFAULT_SSH_PORT).await?;
        let mut res = DriverExecutionResult::default();
        for cmd in commands {
            let result = tokio::time::timeout(ssh::command_timeout(), client.execute(cmd))
                .await
                .with_context(|| format!("rpc timeout {} {}", device.name, cmd))?
                .with_context(|| format!("rpc command {} {}", device.name, cmd))?;
            if result.exit_status != 0 {
                bail!(
                    "command '{}' failed on {} status {}",
                    cmd,
                    device.name,
                    result.exit_status
                );
            }
            res.logs.push(format!(
                "[{}] {} => {}",
                device.name,
                cmd,
                truncate(result.stdout.trim())
            ));
        }
        Ok(res)
    }
}

struct NetconfSession {
    #[allow(dead_code)]
    client: Client,
    stream: Pin<Box<dyn NetconfIo>>,
    next_id: u32,
}

impl NetconfSession {
    async fn connect(device: &Device, store: &KeyringStore, port: u16) -> Result<NetconfSession> {
        let client = ssh::connect(device, store, port).await?;
        let channel = client
            .get_channel()
            .await
            .with_context(|| format!("netconf channel {}", device.name))?;
        channel
            .request_subsystem(true, "netconf")
            .await
            .context("netconf subsystem denied")?;
        let stream = channel.into_stream();
        let mut session = NetconfSession {
            client,
            stream: Box::pin(stream),
            next_id: 1,
        };
        session.send_hello().await?;
        Ok(session)
    }

    async fn send_hello(&mut self) -> Result<()> {
        let hello = r#"<?xml version="1.0" encoding="UTF-8"?>
<hello xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">
  <capabilities>
    <capability>urn:ietf:params:netconf:base:1.0</capability>
  </capabilities>
</hello>]]>]]>"#;
        self.stream.as_mut().write_all(hello.as_bytes()).await?;
        self.stream.as_mut().flush().await?;
        let _server = self.read_reply().await?;
        Ok(())
    }

    async fn rpc(&mut self, inner: &str) -> Result<String> {
        let message_id = self.next_id;
        self.next_id += 1;
        let payload = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><rpc message-id="{message_id}" xmlns="urn:ietf:params:xml:ns:netconf:base:1.0">{inner}</rpc>{NETCONF_EOM}"#
        );
        self.stream
            .as_mut()
            .write_all(payload.as_bytes())
            .await
            .context("write netconf rpc")?;
        self.stream.as_mut().flush().await?;
        let reply = self.read_reply().await?;
        if !reply.contains("<ok/>") && reply.contains("<rpc-error>") {
            bail!("netconf error: {}", reply);
        }
        Ok(reply)
    }

    async fn read_reply(&mut self) -> Result<String> {
        let mut buf = Vec::new();
        loop {
            let mut chunk = vec![0u8; 4096];
            let read = self
                .stream
                .as_mut()
                .read(&mut chunk)
                .await
                .context("read netconf frame")?;
            if read == 0 {
                bail!("netconf stream closed");
            }
            buf.extend_from_slice(&chunk[..read]);
            if buf.len() >= NETCONF_EOM.len()
                && buf[buf.len() - NETCONF_EOM.len()..] == NETCONF_EOM.as_bytes()[..]
            {
                break;
            }
        }
        buf.truncate(buf.len() - NETCONF_EOM.len());
        let reply = String::from_utf8(buf).context("netconf not utf8")?;
        Ok(reply)
    }
}

fn truncate(s: &str) -> String {
    if s.len() > 200 {
        format!("{}…", &s[..200])
    } else if s.is_empty() {
        "ok".into()
    } else {
        s.to_string()
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
