use anyhow::{Context, Result};
use async_trait::async_trait;
use keyring::Entry;
use nauto_model::{Credential, CredentialRef};
use std::collections::HashMap;
use std::path::PathBuf;
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
        let write_res =
            task::spawn_blocking(move || entry.set_password(&json).map_err(anyhow::Error::from))
                .await?;
        match write_res {
            Ok(()) => {
                if let Some(path) = fallback_path() {
                    write_fallback_secret(&path, reference, credential).await?;
                }
                Ok(())
            }
            Err(err) => {
                if let Some(path) = fallback_path() {
                    write_fallback_secret(&path, reference, credential).await?;
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn resolve(&self, reference: &CredentialRef) -> Result<Credential> {
        let entry = self.entry(reference)?;
        match task::spawn_blocking(move || entry.get_password().map_err(anyhow::Error::from))
            .await?
        {
            Ok(secret) => {
                let credential = serde_json::from_str(&secret)?;
                Ok(credential)
            }
            Err(err) => {
                if let Some(path) = fallback_path() {
                    if let Some(credential) = read_fallback_secret(&path, reference).await? {
                        return Ok(credential);
                    }
                }
                Err(err)
            }
        }
    }
}

async fn write_fallback_secret(
    path: &PathBuf,
    reference: &CredentialRef,
    credential: &Credential,
) -> Result<()> {
    let reference = reference.clone();
    let credential = credential.clone();
    let path = path.clone();
    task::spawn_blocking(move || -> Result<()> {
        let mut map = if path.exists() {
            let body = std::fs::read_to_string(&path)?;
            serde_json::from_str::<HashMap<String, Credential>>(&body)
                .unwrap_or_else(|_| HashMap::new())
        } else {
            HashMap::new()
        };
        map.insert(reference.name, credential);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = serde_json::to_string_pretty(&map)?;
        std::fs::write(&path, body)?;
        Ok(())
    })
    .await??;
    Ok(())
}

async fn read_fallback_secret(
    path: &PathBuf,
    reference: &CredentialRef,
) -> Result<Option<Credential>> {
    let path = path.clone();
    let reference = reference.clone();
    let credential = task::spawn_blocking(move || -> Result<Option<Credential>> {
        if !path.exists() {
            return Ok(None);
        }
        let body = std::fs::read_to_string(&path)?;
        let map: HashMap<String, Credential> =
            serde_json::from_str(&body).unwrap_or_else(|_| HashMap::new());
        Ok(map.get(&reference.name).cloned())
    })
    .await??;
    Ok(credential)
}

fn fallback_path() -> Option<PathBuf> {
    std::env::var("NAUTO_KEYRING_FILE").ok().map(PathBuf::from)
}
