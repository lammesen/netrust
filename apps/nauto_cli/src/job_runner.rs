use crate::{audit, plugins};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use nauto_drivers::drivers::{
    AristaEosDriver, CiscoIosDriver, CiscoNxosApiDriver, GenericSshDriver, JuniperJunosDriver,
    MerakiCloudDriver, MockDriver,
};
use nauto_drivers::{DeviceDriver, DriverRegistry};
use nauto_engine::{InMemoryInventory, JobEngine};
use nauto_model::{CapabilitySet, Device, DeviceType, Job, JobKind, JobResult, TargetSelector};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct InventoryFile {
    pub devices: Vec<Device>,
}

#[derive(Debug, Deserialize)]
pub struct JobFile {
    pub name: String,
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub kind: JobKind,
    #[serde(default)]
    pub targets: Option<TargetSelector>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub max_parallel: Option<usize>,
    #[serde(default)]
    pub approval_id: Option<Uuid>,
}

impl From<JobFile> for Job {
    fn from(file: JobFile) -> Job {
        Job {
            id: file.id,
            name: file.name,
            kind: file.kind,
            targets: file.targets.unwrap_or(TargetSelector::All),
            parameters: file.parameters,
            max_parallel: file.max_parallel,
            dry_run: file.dry_run,
            approval_id: file.approval_id,
        }
    }
}

pub async fn run_job(
    job_path: &Path,
    inventory_path: &Path,
    audit_path: &Path,
    dry_run: bool,
) -> Result<(Job, JobResult)> {
    let job = load_job(job_path)?;
    let inventory = load_inventory(inventory_path)?;
    execute_job(job.into(), inventory, audit_path, dry_run).await
}

pub async fn execute_job(
    mut job: Job,
    inventory: InventoryFile,
    audit_path: &Path,
    dry_run: bool,
) -> Result<(Job, JobResult)> {
    if dry_run {
        job.dry_run = true;
    }
    let registry = driver_registry();
    let engine = JobEngine::new(InMemoryInventory::new(inventory.devices.clone()), registry);
    let result = engine.execute(job.clone()).await?;
    audit::record(audit_path.to_path_buf(), &job, &result)?;
    Ok((job, result))
}

pub fn load_inventory(path: &Path) -> Result<InventoryFile> {
    let data = std::fs::read_to_string(path)?;
    let inventory = serde_yaml::from_str(&data)?;
    Ok(inventory)
}

pub fn load_job(path: &Path) -> Result<JobFile> {
    let data = std::fs::read_to_string(path)?;
    let job = serde_yaml::from_str(&data)?;
    Ok(job)
}

#[derive(Debug, Deserialize)]
struct TransactionPlan {
    pub job_name: String,
    pub canary: Vec<String>,
    pub batches: Vec<Vec<String>>,
}

pub async fn run_plan(
    plan_path: &Path,
    base_job: Job,
    inventory: InventoryFile,
    audit_path: &Path,
    dry_run: bool,
) -> Result<Vec<JobResult>> {
    let body = std::fs::read_to_string(plan_path)?;
    let plan: TransactionPlan = serde_yaml::from_str(&body)?;
    if plan.job_name != base_job.name {
        eprintln!(
            "warning: plan targets job '{}' but file is '{}'",
            plan.job_name, base_job.name
        );
    }

    let mut results = Vec::new();
    let mut stages: Vec<Vec<String>> = Vec::new();
    if !plan.canary.is_empty() {
        stages.push(plan.canary);
    }
    stages.extend(plan.batches);

    for (idx, ids) in stages.into_iter().enumerate() {
        if ids.is_empty() {
            continue;
        }
        let filtered = filter_inventory(&inventory, &ids);
        if filtered.devices.is_empty() {
            eprintln!(
                "Stage {} skipped (no matching devices in inventory)",
                idx + 1
            );
            continue;
        }
        let (_job, result) = execute_job(base_job.clone(), filtered, audit_path, dry_run).await?;
        results.push(result);
    }

    Ok(results)
}

fn filter_inventory(base: &InventoryFile, device_ids: &[String]) -> InventoryFile {
    let set: HashSet<_> = device_ids.iter().collect();
    let devices = base
        .devices
        .iter()
        .filter(|device| set.contains(&device.id))
        .cloned()
        .collect();
    InventoryFile { devices }
}

pub fn driver_registry() -> DriverRegistry {
    if std::env::var("NAUTO_USE_MOCK_DRIVERS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        return mock_driver_registry();
    }
    let mut drivers: Vec<Arc<dyn DeviceDriver>> = vec![
        Arc::new(CiscoIosDriver::default()),
        Arc::new(JuniperJunosDriver::default()),
        Arc::new(GenericSshDriver::default()),
        Arc::new(AristaEosDriver::default()),
        Arc::new(CiscoNxosApiDriver::default()),
        Arc::new(MerakiCloudDriver::default()),
    ];
    extend_with_plugin_drivers(&mut drivers);
    DriverRegistry::new(drivers)
}

fn mock_driver_registry() -> DriverRegistry {
    let drivers = [
        nauto_model::DeviceType::CiscoIos,
        nauto_model::DeviceType::JuniperJunos,
        nauto_model::DeviceType::GenericSsh,
        nauto_model::DeviceType::AristaEos,
        nauto_model::DeviceType::CiscoNxosApi,
        nauto_model::DeviceType::MerakiCloud,
    ]
    .into_iter()
    .map(|device_type| Arc::new(MockDriver::new(device_type)) as Arc<dyn DeviceDriver>)
    .collect();
    DriverRegistry::new(drivers)
}

fn extend_with_plugin_drivers(drivers: &mut Vec<Arc<dyn DeviceDriver>>) {
    let enable_plugins = std::env::var("NAUTO_ENABLE_PLUGIN_DRIVERS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    for descriptor in plugins::plugin_drivers() {
        match descriptor.device_type.parse::<DeviceType>() {
            Ok(device_type) => {
                if drivers
                    .iter()
                    .any(|driver| driver.device_type() == device_type)
                {
                    continue;
                }
                if !enable_plugins {
                    info!(
                        "Detected plugin driver {} for {:?} (enable via NAUTO_ENABLE_PLUGIN_DRIVERS=1)",
                        descriptor.vendor, device_type
                    );
                    continue;
                }
                let placeholder = PluginDriverPlaceholder::new(&descriptor, device_type);
                drivers.push(Arc::new(placeholder));
            }
            Err(err) => warn!(
                "Plugin {} declared unsupported device type '{}': {}",
                descriptor.vendor, descriptor.device_type, err
            ),
        }
    }
}

struct PluginDriverPlaceholder {
    vendor: String,
    device_type: DeviceType,
    capabilities: CapabilitySet,
}

impl PluginDriverPlaceholder {
    fn new(descriptor: &plugins::PluginDriverDescriptor, device_type: DeviceType) -> Self {
        Self {
            vendor: descriptor.vendor.clone(),
            device_type,
            capabilities: capability_mask_to_set(descriptor.capabilities),
        }
    }
}

#[async_trait]
impl DeviceDriver for PluginDriverPlaceholder {
    fn device_type(&self) -> DeviceType {
        self.device_type.clone()
    }

    fn name(&self) -> &'static str {
        "WASM Plugin Driver"
    }

    fn capabilities(&self) -> CapabilitySet {
        self.capabilities.clone()
    }

    async fn execute(
        &self,
        device: &Device,
        _action: nauto_drivers::DriverAction<'_>,
    ) -> Result<nauto_drivers::DriverExecutionResult> {
        Err(anyhow!(
            "plugin driver from {} is not yet executable for device {}",
            self.vendor,
            device.name
        ))
    }

    async fn rollback(&self, _device: &Device, _snapshot: Option<String>) -> Result<()> {
        Err(anyhow!(
            "plugin driver from {} cannot perform rollback (not implemented)",
            self.vendor
        ))
    }
}

fn capability_mask_to_set(mask: nauto_plugin_sdk::CapabilityMask) -> CapabilitySet {
    CapabilitySet {
        supports_commit: mask.contains(nauto_plugin_sdk::CapabilityMask::COMMIT),
        supports_rollback: mask.contains(nauto_plugin_sdk::CapabilityMask::ROLLBACK),
        supports_diff: mask.contains(nauto_plugin_sdk::CapabilityMask::DIFF),
        supports_dry_run: mask.contains(nauto_plugin_sdk::CapabilityMask::DRY_RUN),
    }
}
