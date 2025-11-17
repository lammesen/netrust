pub mod drivers;

use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType, JobKind};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum DriverAction<'a> {
    Job(&'a JobKind),
}

#[derive(Debug, Clone, Default)]
pub struct DriverExecutionResult {
    pub logs: Vec<String>,
    pub pre_snapshot: Option<String>,
    pub post_snapshot: Option<String>,
    pub diff: Option<String>,
}

#[async_trait]
pub trait DeviceDriver: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> CapabilitySet;
    async fn execute(&self, device: &Device, action: DriverAction<'_>) -> Result<DriverExecutionResult>;
    async fn rollback(
        &self,
        device: &Device,
        snapshot: Option<String>,
    ) -> Result<()>;
}

pub type DynDeviceDriver = Arc<dyn DeviceDriver>;

pub struct DriverRegistry {
    drivers: Vec<DynDeviceDriver>,
}

impl DriverRegistry {
    pub fn new(drivers: Vec<DynDeviceDriver>) -> Self {
        Self { drivers }
    }

    pub fn find(&self, device_type: &DeviceType) -> Option<DynDeviceDriver> {
        self.drivers
            .iter()
            .find(|driver| &driver.device_type() == device_type)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::drivers::{
        AristaEosDriver, CiscoIosDriver, CiscoNxosApiDriver, GenericSshDriver, JuniperJunosDriver,
        MerakiCloudDriver,
    };
    use super::*;

    #[tokio::test]
    async fn driver_capabilities_reported() {
        let registry = DriverRegistry::new(vec![
            Arc::new(CiscoIosDriver::default()),
            Arc::new(JuniperJunosDriver::default()),
            Arc::new(GenericSshDriver::default()),
            Arc::new(AristaEosDriver::default()),
            Arc::new(CiscoNxosApiDriver::default()),
            Arc::new(MerakiCloudDriver::default()),
        ]);

        let ios = registry.find(&nauto_model::DeviceType::CiscoIos).unwrap();
        assert_eq!(ios.name(), "Cisco IOS CLI");

        let junos = registry.find(&nauto_model::DeviceType::JuniperJunos).unwrap();
        assert!(junos.capabilities().supports_commit);

        let generic = registry.find(&nauto_model::DeviceType::GenericSsh).unwrap();
        assert_eq!(generic.capabilities().supports_commit, false);

        let arista = registry.find(&nauto_model::DeviceType::AristaEos).unwrap();
        assert_eq!(arista.name(), "Arista EOS CLI");

        let nxos = registry.find(&nauto_model::DeviceType::CiscoNxosApi).unwrap();
        assert!(nxos.capabilities().supports_diff);

        let meraki = registry.find(&nauto_model::DeviceType::MerakiCloud).unwrap();
        assert!(meraki.capabilities().supports_rollback);
    }
}

