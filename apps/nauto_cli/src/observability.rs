use anyhow::Result;
use clap::Args;
use prometheus::{opts, Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Args)]
pub struct ObservabilityCmd {
    #[arg(long, default_value = "json")]
    pub format: String,
}

pub fn run(_cmd: ObservabilityCmd) -> Result<()> {
    let registry = Registry::new();
    let jobs_counter = IntCounter::with_opts(opts!("jobs_total", "Jobs executed")).unwrap();
    let failures_counter = IntCounter::with_opts(opts!("jobs_failed_total", "Jobs failed")).unwrap();
    let queue_gauge = IntGauge::with_opts(opts!("queue_depth", "Pending queue depth")).unwrap();

    registry.register(Box::new(jobs_counter.clone()))?;
    registry.register(Box::new(failures_counter.clone()))?;
    registry.register(Box::new(queue_gauge.clone()))?;

    jobs_counter.inc_by(128);
    failures_counter.inc_by(3);
    queue_gauge.set(12);

    let mut buffer = Vec::new();
    TextEncoder::new().encode(&registry.gather(), &mut buffer)?;
    println!("# scraped_at {}", unix_timestamp());
    println!("{}", String::from_utf8(buffer)?);
    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

