use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{Device, TargetSelector};

#[async_trait]
pub trait DeviceInventory: Send + Sync {
    async fn resolve_targets(&self, selector: &TargetSelector) -> Result<Vec<Device>>;
}

pub struct InMemoryInventory {
    devices: Vec<Device>,
}

impl InMemoryInventory {
    pub fn new(devices: Vec<Device>) -> Self {
        Self { devices }
    }
}

#[async_trait]
impl DeviceInventory for InMemoryInventory {
    async fn resolve_targets(&self, selector: &TargetSelector) -> Result<Vec<Device>> {
        let matches = match selector {
            TargetSelector::All => self.devices.clone(),
            TargetSelector::ByIds { ids } => self
                .devices
                .iter()
                .filter(|d| ids.contains(&d.id))
                .cloned()
                .collect(),
            TargetSelector::ByTags { all_of } => self
                .devices
                .iter()
                .filter(|d| all_of.iter().all(|tag| d.tags.contains(tag)))
                .cloned()
                .collect(),
        };
        Ok(matches)
    }
}

