use crate::config;
use anyhow::{bail, Context, Result};
use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};
use nauto_model::{Credential, Device};
use nauto_security::{CredentialStore, KeyringStore};
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;
use tokio::fs;

pub const KEYRING_SERVICE: &str = "netrust";
pub const DEFAULT_SSH_PORT: u16 = 22;
pub const DEFAULT_NETCONF_PORT: u16 = 830;

pub fn default_credential_store() -> KeyringStore {
    KeyringStore::new(KEYRING_SERVICE)
}

pub fn command_timeout() -> std::time::Duration {
    config::ssh_command_timeout()
}

pub async fn connect(device: &Device, store: &KeyringStore, port: u16) -> Result<Client> {
    let credential = store
        .resolve(&device.credential)
        .await
        .with_context(|| format!("loading credential {}", device.credential.name))?;
    let (username, auth) = credential_to_auth(&credential).await?;

    let target = SocketAddr::from_str(&device.mgmt_address)
        .map(TargetAddr::Socket)
        .unwrap_or_else(|_| TargetAddr::HostPort(device.mgmt_address.clone(), port));

    let server_check = ServerCheckMethod::DefaultKnownHostsFile;

    match target {
        TargetAddr::Socket(addr) => Client::connect(addr, &username, auth, server_check).await,
        TargetAddr::HostPort(host, port) => {
            Client::connect((host.as_str(), port), &username, auth, server_check).await
        }
    }
    .with_context(|| format!("ssh connect {} ({})", device.name, device.mgmt_address))
}

enum TargetAddr {
    Socket(SocketAddr),
    HostPort(String, u16),
}

async fn credential_to_auth(credential: &Credential) -> Result<(String, AuthMethod)> {
    match credential {
        Credential::UserPassword { username, password } => {
            Ok((username.clone(), AuthMethod::with_password(password)))
        }
        Credential::SshKey {
            username,
            key_path,
            passphrase,
        } => {
            let key_content = fs::read_to_string(Path::new(key_path))
                .await
                .with_context(|| format!("reading ssh key {}", key_path))?;
            Ok((
                username.clone(),
                AuthMethod::with_key(&key_content, passphrase.as_deref()),
            ))
        }
        Credential::Token { .. } => bail!("token-based credential cannot be used for SSH"),
    }
}
