use anyhow::{Context, Result};
use async_trait::async_trait;
use keyring::Entry;
use nauto_model::{Credential, CredentialRef};
use tokio::task;

#[async_trait]
pub trait CredentialStore: Send + Sync {
    async fn store(&self, reference: &CredentialRef, credential: &Credential) -> Result<()>;
    async fn resolve(&self, reference: &CredentialRef) -> Result<Credential>;
}

#[derive(Clone)]
pub struct KeyringStore {
    service: String,
}

impl KeyringStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, reference: &CredentialRef) -> Result<Entry> {
        Entry::new(&self.service, &reference.name)
            .with_context(|| format!("keyring entry {}", reference.name))
    }
}

#[async_trait]
impl CredentialStore for KeyringStore {
    async fn store(&self, reference: &CredentialRef, credential: &Credential) -> Result<()> {
        let entry = self.entry(reference)?;
        let json = serde_json::to_string(credential)?;
        task::spawn_blocking(move || entry.set_password(&json).map_err(anyhow::Error::from))
            .await??;
        Ok(())
    }

    async fn resolve(&self, reference: &CredentialRef) -> Result<Credential> {
        let entry = self.entry(reference)?;
        let secret = task::spawn_blocking(move || entry.get_password().map_err(anyhow::Error::from))
            .await??;
        let credential = serde_json::from_str(&secret)?;
        Ok(credential)
    }
}

