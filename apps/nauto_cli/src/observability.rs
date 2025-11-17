use anyhow::Result;
use clap::{Args, ValueEnum};
use prometheus::{opts, Encoder, IntCounter, IntGauge, Registry, TextEncoder};
use serde::Serialize;
use serde_json::to_string_pretty;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Args)]
pub struct ObservabilityCmd {
    #[arg(long, default_value_t = MetricsFormat::Text, value_enum)]
    pub format: MetricsFormat,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum MetricsFormat {
    Text,
    Json,
}

pub fn run(cmd: ObservabilityCmd) -> Result<()> {
    let registry = Registry::new();
    let jobs_counter = IntCounter::with_opts(opts!("jobs_total", "Jobs executed")).unwrap();
    let failures_counter =
        IntCounter::with_opts(opts!("jobs_failed_total", "Jobs failed")).unwrap();
    let queue_gauge = IntGauge::with_opts(opts!("queue_depth", "Pending queue depth")).unwrap();

    registry.register(Box::new(jobs_counter.clone()))?;
    registry.register(Box::new(failures_counter.clone()))?;
    registry.register(Box::new(queue_gauge.clone()))?;

    jobs_counter.inc_by(128);
    failures_counter.inc_by(3);
    queue_gauge.set(12);

    let snapshot = ObservabilitySnapshot::new(
        unix_timestamp(),
        jobs_counter.get(),
        failures_counter.get(),
        queue_gauge.get(),
    );

    match cmd.format {
        MetricsFormat::Text => emit_prometheus(&registry, snapshot.scraped_at)?,
        MetricsFormat::Json => emit_json(&snapshot)?,
    }

    Ok(())
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn emit_prometheus(registry: &Registry, scraped_at: u64) -> Result<()> {
    let mut buffer = Vec::new();
    TextEncoder::new().encode(&registry.gather(), &mut buffer)?;
    println!("# scraped_at {}", scraped_at);
    println!("{}", String::from_utf8(buffer)?);
    Ok(())
}

fn emit_json(snapshot: &ObservabilitySnapshot) -> Result<()> {
    println!("{}", to_string_pretty(snapshot)?);
    Ok(())
}

#[derive(Serialize)]
struct ObservabilitySnapshot {
    scraped_at: u64,
    metrics: MetricValues,
}

#[derive(Serialize)]
struct MetricValues {
    jobs_total: i64,
    jobs_failed_total: i64,
    queue_depth: i64,
}

impl ObservabilitySnapshot {
    fn new(scraped_at: u64, jobs_total: i64, jobs_failed_total: i64, queue_depth: i64) -> Self {
        Self {
            scraped_at,
            metrics: MetricValues {
                jobs_total,
                jobs_failed_total,
                queue_depth,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_records_metric_values() {
        let snapshot = ObservabilitySnapshot::new(123, 10, 2, 7);
        assert_eq!(snapshot.scraped_at, 123);
        assert_eq!(snapshot.metrics.jobs_total, 10);
        assert_eq!(snapshot.metrics.jobs_failed_total, 2);
        assert_eq!(snapshot.metrics.queue_depth, 7);
    }
}
