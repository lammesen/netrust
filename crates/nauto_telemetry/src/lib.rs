use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[async_trait]
pub trait TelemetryCollector: Send + Sync {
    async fn collect(&self) -> Result<TelemetrySnapshot>;
}

#[derive(Debug, Clone, Serialize)]
pub struct TelemetrySnapshot {
    pub collector: &'static str,
    pub metrics: HashMap<String, f64>,
    pub labels: HashMap<String, String>,
}

pub struct SnmpCollector {
    pub device_id: String,
}

#[async_trait]
impl TelemetryCollector for SnmpCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        sleep(Duration::from_millis(25)).await;
        let mut metrics = HashMap::new();
        metrics.insert("if_up".into(), 48.0);
        metrics.insert("if_down".into(), 2.0);
        let mut labels = HashMap::new();
        labels.insert("device_id".into(), self.device_id.clone());
        Ok(TelemetrySnapshot {
            collector: "snmp",
            metrics,
            labels,
        })
    }
}

pub struct GnmiCollector {
    pub target: String,
}

#[async_trait]
impl TelemetryCollector for GnmiCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        sleep(Duration::from_millis(40)).await;
        let mut metrics = HashMap::new();
        metrics.insert("cpu".into(), 37.2);
        metrics.insert("memory".into(), 68.4);
        let mut labels = HashMap::new();
        labels.insert("target".into(), self.target.clone());
        Ok(TelemetrySnapshot {
            collector: "gnmi",
            metrics,
            labels,
        })
    }
}

pub struct HttpCollector {
    pub endpoint: String,
}

#[async_trait]
impl TelemetryCollector for HttpCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        sleep(Duration::from_millis(30)).await;
        let mut metrics = HashMap::new();
        metrics.insert("clients".into(), 120.0);
        let mut labels = HashMap::new();
        labels.insert("endpoint".into(), self.endpoint.clone());
        Ok(TelemetrySnapshot {
            collector: "http",
            metrics,
            labels,
        })
    }
}

pub async fn collect_all(
    collectors: &[Box<dyn TelemetryCollector>],
) -> Vec<TelemetrySnapshot> {
    let mut snapshots = Vec::new();
    for collector in collectors {
        match collector.collect().await {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(err) => eprintln!("collector failed: {err:?}"),
        }
    }
    snapshots
}

