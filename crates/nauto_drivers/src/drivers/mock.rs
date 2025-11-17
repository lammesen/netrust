use crate::{DeviceDriver, DriverAction, DriverExecutionResult};
use anyhow::Result;
use async_trait::async_trait;
use nauto_model::{CapabilitySet, Device, DeviceType};

#[derive(Clone)]
pub struct MockDriver {
    device_type: DeviceType,
    capabilities: CapabilitySet,
    label: &'static str,
}

impl MockDriver {
    pub fn new(device_type: DeviceType) -> Self {
        Self {
            device_type,
            capabilities: CapabilitySet {
                supports_commit: true,
                supports_rollback: true,
                supports_diff: true,
                supports_dry_run: true,
            },
            label: "Mock Driver",
        }
    }
}

#[async_trait]
impl DeviceDriver for MockDriver {
    fn device_type(&self) -> DeviceType {
        self.device_type.clone()
    }

    fn name(&self) -> &'static str {
        self.label
    }

    fn capabilities(&self) -> CapabilitySet {
        self.capabilities.clone()
    }

    async fn execute(
        &self,
        device: &Device,
        action: DriverAction<'_>,
    ) -> Result<DriverExecutionResult> {
        let mut result = DriverExecutionResult::default();
        result
            .logs
            .push(format!("[mock] device={} action={:?}", device.name, action));
        result.diff = Some("mock diff".into());
        Ok(result)
    }

    async fn rollback(&self, _device: &Device, _snapshot: Option<String>) -> Result<()> {
        Ok(())
    }
}
