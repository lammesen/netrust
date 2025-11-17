use anyhow::Result;
use clap::Args;
use nauto_drivers::drivers::GenericSshDriver;
use nauto_drivers::DriverRegistry;
use nauto_engine::{InMemoryInventory, JobEngine};
use nauto_model::{
    CapabilitySet, CredentialRef, Device, DeviceType, Job, JobKind, TargetSelector,
};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[derive(Args)]
pub struct BenchCmd {
    #[arg(long, default_value_t = 1000)]
    pub devices: usize,
    #[arg(long, default_value_t = 100)]
    pub parallel: usize,
}

pub async fn run(cmd: BenchCmd) -> Result<()> {
    let devices = build_devices(cmd.devices);
    let inventory = InMemoryInventory::new(devices);
    let registry = DriverRegistry::new(vec![Arc::new(GenericSshDriver::default())]);
    let engine = JobEngine::new(inventory, registry).with_parallel(cmd.parallel);

    let job = Job {
        id: Uuid::new_v4(),
        name: format!("bench-{}-{}", cmd.devices, cmd.parallel),
        kind: JobKind::CommandBatch {
            commands: vec!["show version".into()],
        },
        targets: TargetSelector::All,
        parameters: Default::default(),
        max_parallel: None,
        dry_run: false,
    };

    let start = Instant::now();
    let result = engine.execute(job).await?;
    let elapsed = start.elapsed().as_secs_f64();
    let total = result.device_results.len() as f64;
    let throughput = if elapsed > 0.0 {
        total / elapsed
    } else {
        total
    };

    println!("Devices processed: {}", total as usize);
    println!("Elapsed: {:.2}s", elapsed);
    println!("Throughput: {:.2} devices/sec", throughput);
    Ok(())
}

fn build_devices(count: usize) -> Vec<Device> {
    (0..count)
        .map(|i| Device {
            id: format!("bench-{i}"),
            name: format!("bench-{i}"),
            device_type: DeviceType::GenericSsh,
            mgmt_address: format!("10.0.0.{i}"),
            credential: CredentialRef {
                name: "bench".into(),
            },
            tags: vec!["bench".into()],
            capabilities: CapabilitySet::default(),
        })
        .collect()
}

