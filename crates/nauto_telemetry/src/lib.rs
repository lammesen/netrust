use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::future::join_all;
use gnmi_proto::g_nmi_client::GNmiClient;
use gnmi_proto::{GetRequest, Path, PathElem, TypedValue};
use reqwest::Client;
use serde::Serialize;
use snmp::{SyncSession, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio::task;
use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::Request;

pub mod gnmi_proto {
    tonic::include_proto!("gnmi");
}

pub use gnmi_proto::DataType as GnmiDataType;
pub use gnmi_proto::Encoding as GnmiEncoding;

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
    pub target: String,
    pub community: String,
    pub oids: Vec<String>,
    pub timeout: Duration,
}

#[async_trait]
impl TelemetryCollector for SnmpCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        let target = self.target.clone();
        let community = self.community.clone();
        let oids = self.oids.clone();
        let timeout = self.timeout;
        let metrics = task::spawn_blocking(move || -> Result<HashMap<String, f64>> {
            let mut session =
                SyncSession::new(target.as_str(), &community.into_bytes(), Some(timeout), 0)
                    .map_err(|err| anyhow::anyhow!("snmp session error: {err:?}"))?;
            let mut values = HashMap::new();
            for oid in oids {
                let parsed_oid = parse_oid(&oid)?;
                let pdu = session
                    .get(&parsed_oid)
                    .map_err(|err| anyhow::anyhow!("snmp get {} {}: {err:?}", target, oid))?;
                if let Some((_name, value)) = pdu.varbinds.into_iter().next() {
                    if let Some(number) = snmp_value_to_f64(&value) {
                        values.insert(oid, number);
                    }
                }
            }
            Ok(values)
        })
        .await??;
        let mut labels = HashMap::new();
        labels.insert("device_id".into(), self.device_id.clone());
        labels.insert("target".into(), self.target.clone());
        Ok(TelemetrySnapshot {
            collector: "snmp",
            metrics: metrics.into_iter().collect(),
            labels,
        })
    }
}

pub struct GnmiCollector {
    pub address: String,
    pub path: Vec<String>,
    pub data_type: GnmiDataType,
    pub encoding: GnmiEncoding,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[async_trait]
impl TelemetryCollector for GnmiCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        let endpoint =
            if self.address.starts_with("http://") || self.address.starts_with("https://") {
                Endpoint::from_shared(self.address.clone())?
            } else {
                Endpoint::from_shared(format!("http://{}", self.address))?
            };

        let channel = endpoint.connect().await?;
        let mut client = GNmiClient::new(channel);

        let request = GetRequest {
            prefix: Some(Path {
                origin: "".into(),
                elem: vec![],
                target: "".into(),
            }),
            path: vec![Path {
                origin: "".into(),
                target: "".into(),
                elem: self
                    .path
                    .iter()
                    .map(|segment| PathElem {
                        name: segment.clone(),
                        key: Default::default(),
                    })
                    .collect(),
            }],
            r#type: self.data_type as i32,
            encoding: self.encoding as i32,
        };

        let mut request = Request::new(request);
        if let Some(user) = &self.username {
            request
                .metadata_mut()
                .insert("username", MetadataValue::try_from(user.as_str())?);
        }
        if let Some(pass) = &self.password {
            request
                .metadata_mut()
                .insert("password", MetadataValue::try_from(pass.as_str())?);
        }

        let response = client.get(request).await?.into_inner();

        let mut metrics = HashMap::new();
        for notification in response.notification {
            for update in notification.update {
                if let Some(val) = update.val.as_ref().and_then(typed_value_to_f64) {
                    let name = path_to_string(update.path.as_ref());
                    metrics.insert(name, val);
                }
            }
        }

        let mut labels = HashMap::new();
        labels.insert("target".into(), self.address.clone());
        Ok(TelemetrySnapshot {
            collector: "gnmi",
            metrics,
            labels,
        })
    }
}

pub struct HttpCollector {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    client: Client,
}

#[async_trait]
impl TelemetryCollector for HttpCollector {
    async fn collect(&self) -> Result<TelemetrySnapshot> {
        let mut request = self.client.get(&self.endpoint);
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("http collector {}", self.endpoint))?
            .error_for_status()
            .with_context(|| format!("http collector status {}", self.endpoint))?;
        let payload: serde_json::Value = response.json().await?;
        let mut metrics = HashMap::new();
        extract_numeric_fields("", &payload, &mut metrics);
        let mut labels = HashMap::new();
        labels.insert("endpoint".into(), self.endpoint.clone());
        Ok(TelemetrySnapshot {
            collector: "http",
            metrics,
            labels,
        })
    }
}

impl HttpCollector {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            headers: HashMap::new(),
            client: Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("http telemetry client"),
        }
    }
}

pub async fn collect_all(collectors: &[Box<dyn TelemetryCollector>]) -> Vec<TelemetrySnapshot> {
    let futures = collectors.iter().map(|collector| collector.collect());
    let results = join_all(futures).await;
    results
        .into_iter()
        .filter_map(|result| match result {
            Ok(snapshot) => Some(snapshot),
            Err(err) => {
                eprintln!("collector failed: {err:?}");
                None
            }
        })
        .collect()
}

fn parse_oid(oid: &str) -> Result<Vec<u32>> {
    oid.split('.')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            segment.parse::<u32>().map_err(|err| {
                anyhow::anyhow!("invalid OID segment '{}' in {} ({err})", segment, oid)
            })
        })
        .collect()
}

fn snmp_value_to_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(v) => Some(*v as f64),
        Value::Counter32(v) => Some(*v as f64),
        Value::Unsigned32(v) => Some(*v as f64),
        Value::Timeticks(v) => Some(*v as f64),
        Value::Counter64(v) => Some(*v as f64),
        _ => None,
    }
}

fn typed_value_to_f64(value: &TypedValue) -> Option<f64> {
    use gnmi_proto::typed_value::Value as GnmiValue;
    match value.value.as_ref()? {
        GnmiValue::FloatVal(v) => Some(*v),
        GnmiValue::IntVal(v) => Some(*v as f64),
        GnmiValue::UintVal(v) => Some(*v as f64),
        GnmiValue::BoolVal(v) => Some(if *v { 1.0 } else { 0.0 }),
        GnmiValue::StringVal(v) => v.parse().ok(),
        GnmiValue::JsonVal(bytes) => serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .and_then(|val| match val {
                serde_json::Value::Number(num) => num.as_f64(),
                _ => None,
            }),
    }
}

fn path_to_string(path: Option<&Path>) -> String {
    if let Some(path) = path {
        let mut parts = Vec::new();
        for elem in &path.elem {
            if elem.key.is_empty() {
                parts.push(elem.name.clone());
            } else {
                let keys = elem
                    .key
                    .clone()
                    .into_iter()
                    .map(|(k, v)| format!("[{}={}]", k, v))
                    .collect::<String>();
                parts.push(format!("{}{}", elem.name, keys));
            }
        }
        parts.join("/")
    } else {
        "gnmi/value".into()
    }
}

fn extract_numeric_fields(
    prefix: &str,
    value: &serde_json::Value,
    metrics: &mut HashMap<String, f64>,
) {
    match value {
        serde_json::Value::Number(number) => {
            if let Some(f) = number.as_f64() {
                metrics.insert(prefix.trim_matches('.').into(), f);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                extract_numeric_fields(&new_prefix, nested, metrics);
            }
        }
        serde_json::Value::Array(array) => {
            for (idx, nested) in array.iter().enumerate() {
                let new_prefix = format!("{}[{}]", prefix, idx);
                extract_numeric_fields(&new_prefix, nested, metrics);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    struct TestCollector {
        label: &'static str,
        metric: &'static str,
        counter: Arc<AtomicUsize>,
        fail: bool,
    }

    #[async_trait]
    impl TelemetryCollector for TestCollector {
        async fn collect(&self) -> Result<TelemetrySnapshot> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            self.counter.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                anyhow::bail!("intentional test failure");
            }
            let mut metrics = HashMap::new();
            metrics.insert(self.metric.into(), 1.0);
            Ok(TelemetrySnapshot {
                collector: self.label,
                metrics,
                labels: HashMap::new(),
            })
        }
    }

    #[tokio::test]
    async fn collect_all_filters_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let collectors: Vec<Box<dyn TelemetryCollector>> = vec![
            Box::new(TestCollector {
                label: "ok",
                metric: "m1",
                counter: counter.clone(),
                fail: false,
            }),
            Box::new(TestCollector {
                label: "fail",
                metric: "m2",
                counter: counter.clone(),
                fail: true,
            }),
        ];

        let snapshots = collect_all(&collectors).await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].collector, "ok");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
