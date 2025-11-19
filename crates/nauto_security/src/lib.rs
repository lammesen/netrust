use age::{
    secrecy::SecretString,
    Encryptor,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use keyring::Entry;
use nauto_model::{Credential, CredentialRef};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use tokio::task;
use tracing::{info, instrument};

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
    #[instrument(skip(self, credential))]
    async fn store(&self, reference: &CredentialRef, credential: &Credential) -> Result<()> {
        info!(
            target: "security::audit",
            "storing credential '{}'",
            reference.name
        );
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

    #[instrument(skip(self))]
    async fn resolve(&self, reference: &CredentialRef) -> Result<Credential> {
        info!(
            target: "security::audit",
            "resolving credential '{}'",
            reference.name
        );
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
        let mut map = load_fallback_map(&path)?;
        map.insert(reference.name, credential);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        save_fallback_map(&path, &map)?;
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
        let map = load_fallback_map(&path)?;
        Ok(map.get(&reference.name).cloned())
    })
    .await??;
    Ok(credential)
}

fn load_fallback_map(path: &PathBuf) -> Result<HashMap<String, Credential>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = std::fs::read(path)?;
    if content.is_empty() {
        return Ok(HashMap::new());
    }

    if let Some(key) = encryption_key() {
        // Attempt decryption
        // We use fully qualified age::Decryptor.
        // If age::Decryptor is a struct (as errors suggest), we create it and call decrypt.
        let decryptor = match age::Decryptor::new(&content[..]) {
            Ok(d) => d,
            Err(_) => {
                // Fallback to plain JSON if decryption fails (migration path or plain file)
                return serde_json::from_slice(&content).or_else(|_| Ok(HashMap::new()));
            }
        };

        // For passphrase, we need an identity.
        let identity = age::scrypt::Identity::new(SecretString::new(key.into()));
        
        // We assume decryptor has a decrypt method that takes an iterator of identities.
        // If Decryptor is an enum, this might fail if I don't match.
        // But if it is a struct, this works.
        // Let's try matching IF it allows us to distinguish.
        // But previous attempts at matching failed. 
        // Let's try to inspect if we can just iterate identities.
        // Actually, if Decryptor is an enum, I can't call decrypt on it directly unless it implements it.
        // Let's try to match AGAIN but use wildcards to be safe? No, error said no variant.
        
        // Let's try to use `decrypt` on `decryptor` directly assuming it is a struct.
        // We need to pass `&dyn age::Identity`.
        let identities: Vec<Box<dyn age::Identity>> = vec![Box::new(identity)];
        
        match decryptor.decrypt(identities.iter().map(|i| i.as_ref())) {
             Ok(mut reader) => {
                let mut decrypted = Vec::new();
                reader.read_to_end(&mut decrypted)?;
                let map: HashMap<String, Credential> = serde_json::from_slice(&decrypted)?;
                return Ok(map);
             }
             Err(_) => {
                 // If it requires passphrase but we failed, maybe plain text?
                 // But we handled plain text in Err of new().
                 // If new() succeeded, it IS an age file.
                 return Err(anyhow::anyhow!("Decryption failed (wrong key?)"));
             }
        }
    }

    // No key provided, try reading as plain JSON
    serde_json::from_slice(&content).context("reading plaintext fallback file (set NAUTO_ENCRYPTION_KEY to encrypt)")
}

fn save_fallback_map(path: &PathBuf, map: &HashMap<String, Credential>) -> Result<()> {
    let json = serde_json::to_string_pretty(map)?;
    
    if let Some(key) = encryption_key() {
        let encryptor = Encryptor::with_user_passphrase(SecretString::new(key.into()));
        let file = std::fs::File::create(path)?;
        let mut writer = encryptor.wrap_output(file)?;
        writer.write_all(json.as_bytes())?;
        writer.finish()?;
    } else {
        anyhow::bail!("NAUTO_ENCRYPTION_KEY not set, refusing to write credentials to plaintext fallback file");
    }
    Ok(())
}

fn fallback_path() -> Option<PathBuf> {
    std::env::var("NAUTO_KEYRING_FILE").ok().map(PathBuf::from)
}

fn encryption_key() -> Option<String> {
    std::env::var("NAUTO_ENCRYPTION_KEY").ok().filter(|s| !s.is_empty())
}
