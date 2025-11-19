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
        if device.tags.iter().any(|t| t == "mock:fail") {
            anyhow::bail!("simulated failure for {}", device.name);
        }
        
        if let nauto_model::JobKind::CommandBatch { commands } = action.job_kind() {
            if commands.iter().any(|c| c == "fail") {
                anyhow::bail!("simulated command failure");
            }
            if commands.iter().any(|c| c == "timeout") {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        }

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
