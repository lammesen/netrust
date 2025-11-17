mod inventory;

pub use inventory::{DeviceInventory, InMemoryInventory};

use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use nauto_drivers::{DeviceDriver, DriverAction, DriverExecutionResult, DriverRegistry};
use nauto_model::{Job, JobResult, TaskStatus, TaskSummary};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Semaphore;
use tracing::{error, info, info_span, instrument};

#[derive(Error, Debug)]
pub enum JobEngineError {
    #[error("no driver registered for device type")]
    MissingDriver,
}

pub struct JobEngine<I: DeviceInventory> {
    inventory: I,
    drivers: DriverRegistry,
    default_parallel: usize,
}

impl<I: DeviceInventory> JobEngine<I> {
    pub fn new(inventory: I, drivers: DriverRegistry) -> Self {
        Self {
            inventory,
            drivers,
            default_parallel: 32,
        }
    }

    pub fn with_parallel(mut self, parallel: usize) -> Self {
        self.default_parallel = parallel.max(1);
        self
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, job: Job) -> Result<JobResult> {
        let devices = self.inventory.resolve_targets(&job.targets).await?;
        let started_at = chrono::Utc::now();
        let semaphore = Arc::new(Semaphore::new(
            job.max_parallel.unwrap_or(self.default_parallel),
        ));
        let mut tasks = FuturesUnordered::new();

        for device in devices {
            let sem = semaphore.clone();
            let driver = self.drivers.find(&device.device_type);
            let job_kind = job.kind.clone();
            let dry_run = job.dry_run;

            tasks.push(tokio::spawn(async move {
                let permit = sem.acquire_owned().await.expect("semaphore closed");
                run_device(device, driver, job_kind, dry_run, permit).await
            }));
        }

        let mut device_results = Vec::new();
        while let Some(res) = tasks.next().await {
            match res {
                Ok(summary) => device_results.push(summary),
                Err(err) => error!("task join error: {err}"),
            }
        }

        let finished_at = chrono::Utc::now();
        Ok(JobResult {
            job_id: job.id,
            started_at,
            finished_at,
            device_results,
        })
    }
}

async fn run_device(
    device: nauto_model::Device,
    driver: Option<Arc<dyn DeviceDriver>>,
    job_kind: nauto_model::JobKind,
    dry_run: bool,
    permit: tokio::sync::OwnedSemaphorePermit,
) -> TaskSummary {
    let span = info_span!(
        "device_task",
        device = %device.name,
        job_kind = job_kind_label(&job_kind)
    );
    let _enter = span.enter();
    let start = chrono::Utc::now();

    let summary = match driver {
        Some(driver) => match execute_with_driver(&device, driver, job_kind, dry_run).await {
            Ok(result) => TaskSummary {
                device_id: device.id.clone(),
                status: TaskStatus::Success,
                started_at: Some(start),
                finished_at: Some(chrono::Utc::now()),
                logs: result.logs,
                diff: result.diff,
            },
            Err(err) => {
                error!(
                    target: "engine::device",
                    "device={} failed: {err:?}",
                    device.name
                );
                TaskSummary {
                    device_id: device.id.clone(),
                    status: TaskStatus::Failed,
                    started_at: Some(start),
                    finished_at: Some(chrono::Utc::now()),
                    logs: vec![format!("error: {err}")],
                    diff: None,
                }
            }
        },
        None => TaskSummary {
            device_id: device.id.clone(),
            status: TaskStatus::Skipped,
            started_at: Some(start),
            finished_at: Some(chrono::Utc::now()),
            logs: vec!["No driver available".into()],
            diff: None,
        },
    };

    drop(permit);
    summary
}

async fn execute_with_driver(
    device: &nauto_model::Device,
    driver: Arc<dyn DeviceDriver>,
    job_kind: nauto_model::JobKind,
    dry_run: bool,
) -> Result<DriverExecutionResult> {
    if dry_run && !driver.capabilities().supports_dry_run {
        info!(
            target: "engine::device",
            "device={} dry-run requested but unsupported, skipping apply",
            device.name
        );
        return Ok(DriverExecutionResult {
            logs: vec!["Dry run skipped (not supported)".into()],
            ..Default::default()
        });
    }

    driver.execute(device, DriverAction::Job(&job_kind)).await
}

fn job_kind_label(kind: &nauto_model::JobKind) -> &'static str {
    match kind {
        nauto_model::JobKind::CommandBatch { .. } => "command_batch",
        nauto_model::JobKind::ConfigPush { .. } => "config_push",
        nauto_model::JobKind::ComplianceCheck { .. } => "compliance_check",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nauto_drivers::drivers::{CiscoIosDriver, GenericSshDriver, JuniperJunosDriver};
    use nauto_model::{CapabilitySet, CredentialRef, Device, DeviceType, Job, TargetSelector};
    use std::sync::Arc;
    use uuid::Uuid;

    fn mock_devices() -> Vec<Device> {
        vec![
            Device {
                id: "r1".into(),
                name: "core-r1".into(),
                device_type: DeviceType::CiscoIos,
                mgmt_address: "10.0.0.1".into(),
                credential: CredentialRef {
                    name: "default".into(),
                },
                tags: vec!["site:oslo".into(), "role:core".into()],
                capabilities: CapabilitySet::default(),
            },
            Device {
                id: "j1".into(),
                name: "edge-j1".into(),
                device_type: DeviceType::JuniperJunos,
                mgmt_address: "10.0.0.2".into(),
                credential: CredentialRef {
                    name: "default".into(),
                },
                tags: vec!["site:oslo".into(), "role:edge".into()],
                capabilities: CapabilitySet::default(),
            },
        ]
    }

    fn registry() -> DriverRegistry {
        DriverRegistry::new(vec![
            Arc::new(CiscoIosDriver::default()),
            Arc::new(JuniperJunosDriver::default()),
            Arc::new(GenericSshDriver::default()),
        ])
    }

    #[tokio::test]
    async fn runs_job_across_devices() {
        let inventory = InMemoryInventory::new(mock_devices());
        let engine = JobEngine::new(inventory, registry()).with_parallel(4);

        let job = Job {
            id: Uuid::new_v4(),
            name: "Bulk show version".into(),
            kind: nauto_model::JobKind::CommandBatch {
                commands: vec!["show version".into()],
            },
            targets: TargetSelector::All,
            parameters: Default::default(),
            max_parallel: None,
            dry_run: false,
        };

        let result = engine.execute(job).await.expect("job execution");
        assert_eq!(result.device_results.len(), 2);
        assert_eq!(result.success_count(), 2);
    }
}
