mod inventory;
pub mod store;
pub mod queue;

pub use inventory::{DeviceInventory, InMemoryInventory};
use crate::store::{JobStore, NoOpJobStore};

use anyhow::{Context, Result};
use nauto_compliance::{ComplianceEngine, DeviceConfigs};
use nauto_drivers::{DeviceDriver, DriverAction, DriverExecutionResult, DriverRegistry};
use nauto_model::{ComplianceRule, Device, Job, JobResult, TaskStatus, TaskSummary};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
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
    store: Arc<dyn JobStore>,
}

impl<I: DeviceInventory> JobEngine<I> {
    pub fn new(inventory: I, drivers: DriverRegistry) -> Self {
        Self {
            inventory,
            drivers,
            default_parallel: 32,
            store: Arc::new(NoOpJobStore),
        }
    }

    pub fn with_parallel(mut self, parallel: usize) -> Self {
        self.default_parallel = parallel.max(1);
        self
    }

    pub fn with_store<S: JobStore + 'static>(mut self, store: S) -> Self {
        self.store = Arc::new(store);
        self
    }

    #[instrument(skip(self))]
    pub async fn execute(&self, job: Job) -> Result<JobResult> {
        self.store.create_job(&job).await?;

        let devices = self.inventory.resolve_targets(&job.targets).await?;
        if let nauto_model::JobKind::ComplianceCheck { rules } = &job.kind {
            return execute_compliance_job(
                job.id,
                devices,
                rules.clone(),
                &job.parameters,
                self.store.clone(),
            )
            .await;
        }
        let started_at = chrono::Utc::now();
        let semaphore = Arc::new(Semaphore::new(
            job.max_parallel.unwrap_or(self.default_parallel),
        ));
        let mut join_set = tokio::task::JoinSet::new();

        for device in devices {
            let sem = semaphore.clone();
            let driver = self.drivers.find(&device.device_type);
            let job_kind = job.kind.clone();
            let dry_run = job.dry_run;
            let device_id = device.id.clone();

            join_set.spawn(async move {
                let permit = match sem.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => {
                        return TaskSummary {
                            device_id,
                            status: TaskStatus::Failed,
                            started_at: Some(chrono::Utc::now()),
                            finished_at: Some(chrono::Utc::now()),
                            logs: vec!["Semaphore closed".into()],
                            diff: None,
                        }
                    }
                };

                let device_id = device.id.clone();
                match tokio::time::timeout(
                    std::time::Duration::from_secs(300),
                    run_device(device, driver, job_kind, dry_run, permit),
                )
                .await
                {
                    Ok(summary) => summary,
                    Err(_) => TaskSummary {
                        device_id,
                        status: TaskStatus::Failed,
                        started_at: Some(chrono::Utc::now()),
                        finished_at: Some(chrono::Utc::now()),
                        logs: vec!["Job execution timed out".into()],
                        diff: None,
                    },
                }
            });
        }

        let mut device_results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(summary) => {
                    if let Err(e) = self.store.update_task_summary(job.id, &summary).await {
                        error!("failed to persist task summary: {e}");
                    }
                    device_results.push(summary);
                }
                Err(err) => error!("task join error: {err}"),
            }
        }

        let finished_at = chrono::Utc::now();
        let result = JobResult {
            job_id: job.id,
            started_at,
            finished_at,
            device_results,
        };

        self.store.complete_job(job.id, &result).await?;

        Ok(result)
    }
}

async fn execute_compliance_job(
    job_id: uuid::Uuid,
    devices: Vec<Device>,
    rules: Vec<ComplianceRule>,
    parameters: &HashMap<String, Value>,
    store: Arc<dyn JobStore>,
) -> Result<JobResult> {
    let started_at = chrono::Utc::now();
    let inputs = Arc::new(load_compliance_inputs(parameters)?);
    let rules = Arc::new(rules);

    let mut join_set = tokio::task::JoinSet::new();

    for device in devices {
        let inputs = inputs.clone();
        let rules = rules.clone();

        join_set.spawn(async move {
            let start = chrono::Utc::now();
            if let Some(config) = inputs.get(&device.id) {
                let rules = rules.clone();
                let config = config.clone();
                let device_id = device.id.clone();

                let (passed, logs) = tokio::task::spawn_blocking(move || {
                    evaluate_device_compliance(&device_id, &rules, &config)
                })
                .await
                .unwrap_or((false, vec!["internal error: evaluation panicked".into()]));

                TaskSummary {
                    device_id: device.id,
                    status: if passed {
                        TaskStatus::Success
                    } else {
                        TaskStatus::Failed
                    },
                    started_at: Some(start),
                    finished_at: Some(chrono::Utc::now()),
                    logs,
                    diff: None,
                }
            } else {
                TaskSummary {
                    device_id: device.id,
                    status: TaskStatus::Failed,
                    started_at: Some(start),
                    finished_at: Some(chrono::Utc::now()),
                    logs: vec!["no config provided for compliance evaluation".into()],
                    diff: None,
                }
            }
        });
    }

    let mut device_results = Vec::new();
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(summary) => {
                if let Err(e) = store.update_task_summary(job_id, &summary).await {
                    error!("failed to persist compliance summary: {e}");
                }
                device_results.push(summary);
            }
            Err(err) => error!("compliance task error: {err}"),
        }
    }

    let finished_at = chrono::Utc::now();
    let result = JobResult {
        job_id,
        started_at,
        finished_at,
        device_results,
    };

    store.complete_job(job_id, &result).await?;

    Ok(result)
}

fn load_compliance_inputs(parameters: &HashMap<String, Value>) -> Result<DeviceConfigs> {
    if let Some(path) = parameters
        .get("inputs_path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
    {
        if std::path::Path::new(path)
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            anyhow::bail!("path traversal not allowed in inputs_path");
        }
        let body = fs::read_to_string(path)
            .with_context(|| format!("reading compliance inputs from {}", path))?;
        let parsed: DeviceConfigs =
            serde_yaml::from_str(&body).context("parsing compliance inputs YAML")?;
        return Ok(parsed);
    }

    if let Some(inline) = parameters.get("inputs").and_then(|v| v.as_object()) {
        let mut parsed = DeviceConfigs::new();
        for (device_id, value) in inline {
            if let Some(config) = value.as_str() {
                parsed.insert(device_id.clone(), config.to_string());
            }
        }
        if !parsed.is_empty() {
            return Ok(parsed);
        }
    }

    Err(anyhow::anyhow!(
        "compliance job requires 'inputs_path' (YAML map of device_id->config) or inline 'inputs'"
    ))
}

fn evaluate_device_compliance(
    device_id: &str,
    rules: &[ComplianceRule],
    config: &str,
) -> (bool, Vec<String>) {
    let mut dataset = DeviceConfigs::new();
    dataset.insert(device_id.to_string(), config.to_string());
    let outcomes = ComplianceEngine::evaluate(rules, &dataset);
    let mut logs = Vec::new();
    let mut all_passed = true;
    for outcome in outcomes {
        if outcome.passed {
            logs.push(format!("{}: pass", outcome.rule));
        } else {
            all_passed = false;
            let detail = outcome
                .details
                .unwrap_or_else(|| "missing required pattern".into());
            logs.push(format!("{}: fail ({})", outcome.rule, detail));
        }
    }
    (all_passed, logs)
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
    use nauto_drivers::drivers::MockDriver;
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
        let drivers: Vec<Arc<dyn DeviceDriver>> = [
            DeviceType::CiscoIos,
            DeviceType::JuniperJunos,
            DeviceType::GenericSsh,
        ]
        .into_iter()
        .map(|device_type| Arc::new(MockDriver::new(device_type)) as Arc<dyn DeviceDriver>)
        .collect();
        DriverRegistry::new(drivers)
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
            approval_id: None,
        };

        let result = engine.execute(job).await.expect("job execution");
        assert_eq!(result.device_results.len(), 2);
        assert_eq!(result.success_count(), 2);
    }
}
