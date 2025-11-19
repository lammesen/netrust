use anyhow::{Context, Result};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
use nauto_plugin_sdk::CapabilityMask;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::{info, warn};
use wasmtime::{Engine, Linker, Module, Store};

static PLUGIN_HOST: OnceLock<PluginHost> = OnceLock::new();

pub fn load_installed(dir: &Path) -> PluginHost {
    let mut host = PluginHost::new();
    match try_load(dir) {
        Ok(plugins) => {
            if plugins.is_empty() {
                info!("No WASM plugins detected in {}", dir.display());
            } else {
                for plugin in plugins {
                    host.register_driver(plugin.into());
                }
            }
        }
        Err(err) => warn!("Plugin loading failed for {}: {err:?}", dir.display()),
    }
    let _ = PLUGIN_HOST.set(host.clone());
    host
}

#[derive(Clone)]
pub struct PluginHost {
    pub drivers: Vec<PluginDriverDescriptor>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    pub fn register_driver(&mut self, descriptor: PluginDriverDescriptor) {
        info!(
            "Registered plugin driver from {} ({:?})",
            descriptor.vendor, descriptor.capabilities
        );
        self.drivers.push(descriptor);
    }
}

pub fn global_host() -> Option<&'static PluginHost> {
    PLUGIN_HOST.get()
}

pub fn plugin_drivers() -> Vec<PluginDriverDescriptor> {
    global_host()
        .map(|host| host.drivers.clone())
        .unwrap_or_default()
}

#[derive(Clone)]
pub struct PluginDriverDescriptor {
    pub vendor: String,
    pub device_type: String,
    pub capabilities: CapabilityMask,
    pub artifact: PathBuf,
}

struct LoadedPlugin {
    vendor: String,
    device_type: String,
    capabilities: CapabilityMask,
    path: PathBuf,
}

impl From<LoadedPlugin> for PluginDriverDescriptor {
    fn from(plugin: LoadedPlugin) -> PluginDriverDescriptor {
        info!(
            "Loaded plugin {} ({:?}) targeting {} from {}",
            plugin.vendor,
            plugin.capabilities,
            plugin.device_type,
            plugin.path.display()
        );
        PluginDriverDescriptor {
            vendor: plugin.vendor,
            device_type: plugin.device_type,
            capabilities: plugin.capabilities,
            artifact: plugin.path,
        }
    }
}

fn try_load(dir: &Path) -> Result<Vec<LoadedPlugin>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let engine = Engine::default();
    let mut plugins = Vec::new();
    for entry in fs::read_dir(dir).context("reading plugin directory")? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("wasm") {
            continue;
        }
        match load_single(&engine, &path) {
            Ok(plugin) => plugins.push(plugin),
            Err(err) => warn!("Failed to initialize plugin {:?}: {err:?}", path),
        }
    }
    Ok(plugins)
}

fn load_single(engine: &Engine, path: &Path) -> Result<LoadedPlugin> {
    let wasm_bytes = fs::read(path)?;
    verify_signature(path, &wasm_bytes)?;

    let module = Module::new(engine, &wasm_bytes)?;
    // Restrict WASM capabilities (no WASI imports provided, so effectively restricted)
    // To explicitly deny, we just don't link WASI.
    // If the plugin requires WASI, instantiation will fail, which is what we want for now unless we whitelist.
    
    let mut store = Store::new(engine, ());
    let linker = Linker::new(engine);
    // linker.func(...) can be used to provide host functions if needed.
    
    let instance = linker.instantiate(&mut store, &module)?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .context("plugin missing exported memory")?;
    let vendor_ptr = instance
        .get_typed_func::<(), i32>(&mut store, "plugin_vendor_ptr")?
        .call(&mut store, ())?;
    let vendor_len = instance
        .get_typed_func::<(), i32>(&mut store, "plugin_vendor_len")?
        .call(&mut store, ())?;
    let caps_bits = instance
        .get_typed_func::<(), u32>(&mut store, "plugin_capabilities")?
        .call(&mut store, ())?;

    let vendor = read_utf8(
        &mut store,
        &memory,
        vendor_ptr as usize,
        vendor_len as usize,
    )?;
    let device_ptr = instance
        .get_typed_func::<(), i32>(&mut store, "plugin_device_type_ptr")?
        .call(&mut store, ())?;
    let device_len = instance
        .get_typed_func::<(), i32>(&mut store, "plugin_device_type_len")?
        .call(&mut store, ())?;
    let device_type = read_utf8(
        &mut store,
        &memory,
        device_ptr as usize,
        device_len as usize,
    )?;

    Ok(LoadedPlugin {
        vendor,
        device_type,
        capabilities: CapabilityMask::from_bits_truncate(caps_bits),
        path: path.to_path_buf(),
    })
}

fn verify_signature(path: &Path, wasm_bytes: &[u8]) -> Result<()> {
    let pub_key_hex = std::env::var("NAUTO_PLUGIN_PUBLIC_KEY")
        .context("NAUTO_PLUGIN_PUBLIC_KEY not set, cannot verify plugins")?;
    
    let pub_key_bytes = hex::decode(&pub_key_hex)
        .context("invalid public key hex")?;
        
    let verifying_key = VerifyingKey::from_bytes(pub_key_bytes.as_slice().try_into()?)
        .map_err(|_| anyhow::anyhow!("invalid public key length"))?;

    let sig_path = path.with_extension("wasm.sig");
    if !sig_path.exists() {
        anyhow::bail!("missing signature file {:?}", sig_path);
    }
    let sig_bytes = fs::read(&sig_path)?;
    let signature = Signature::from_bytes(sig_bytes.as_slice().try_into().context("invalid signature length")?);

    verifying_key.verify(wasm_bytes, &signature)
        .context("signature verification failed")?;
    
    info!("Verified signature for {:?}", path);
    Ok(())
}

fn read_utf8(
    store: &mut Store<()>,
    memory: &wasmtime::Memory,
    ptr: usize,
    len: usize,
) -> Result<String> {
    let data = memory
        .data(store)
        .get(ptr..ptr + len)
        .context("plugin metadata pointer out of bounds")?;
    Ok(std::str::from_utf8(data)?.to_string())
}
