use anyhow::Result;
use clap::Args;
use nauto_telemetry::{
    collect_all, GnmiCollector, GnmiDataType, GnmiEncoding, HttpCollector, SnmpCollector,
    TelemetryCollector,
};
use telemetry_writers::{CsvWriter, JsonWriter, TelemetryWriter};

#[derive(Args)]
pub struct TelemetryCmd {
    #[arg(long, default_value = "json", value_parser = ["json", "csv"])]
    pub format: String,
    #[arg(long, default_value = "127.0.0.1:161")]
    pub snmp_target: String,
    #[arg(long, default_value = "public")]
    pub snmp_community: String,
    #[arg(long, value_delimiter = ',', default_value = "1.3.6.1.2.1.1.3.0")]
    pub snmp_oid: Vec<String>,
    #[arg(long, default_value = "127.0.0.1:9339")]
    pub gnmi_addr: String,
    #[arg(
        long,
        value_delimiter = '/',
        default_value = "/system/state/cpu/utilization"
    )]
    pub gnmi_path: Vec<String>,
    #[arg(long)]
    pub gnmi_username: Option<String>,
    #[arg(long)]
    pub gnmi_password: Option<String>,
    #[arg(long, default_value = "http://localhost:8080/metrics")]
    pub http_endpoint: String,
    #[arg(long, value_parser = parse_header)]
    pub http_header: Vec<HeaderArg>,
    #[arg(long, help = "Optional YAML configuration file describing collectors")]
    pub config: Option<std::path::PathBuf>,
}

pub async fn run(cmd: TelemetryCmd) -> Result<()> {
    let collectors: Vec<Box<dyn TelemetryCollector>> = if let Some(config_path) = &cmd.config {
        telemetry_config::load_collectors(config_path)?
    } else {
        let snmp = SnmpCollector {
            device_id: cmd.snmp_target.clone(),
            target: cmd.snmp_target.clone(),
            community: cmd.snmp_community.clone(),
            oids: cmd.snmp_oid.clone(),
            timeout: std::time::Duration::from_secs(2),
        };

        let gnmi = GnmiCollector {
            address: cmd.gnmi_addr.clone(),
            path: cmd
                .gnmi_path
                .iter()
                .filter(|segment| !segment.is_empty())
                .map(|segment| segment.trim_start_matches('/').to_string())
                .collect(),
            data_type: GnmiDataType::State,
            encoding: GnmiEncoding::Json,
            username: cmd.gnmi_username.clone(),
            password: cmd.gnmi_password.clone(),
        };

        let mut http = HttpCollector::new(cmd.http_endpoint.clone());
        for header in &cmd.http_header {
            http.headers
                .insert(header.key.clone(), header.value.clone());
        }

        vec![Box::new(snmp), Box::new(gnmi), Box::new(http)]
    };
    let snapshots = collect_all(&collectors).await;

    match cmd.format.as_str() {
        "csv" => CsvWriter::default().write(&snapshots),
        _ => JsonWriter::default().write(&snapshots),
    }
}

#[derive(Clone)]
pub struct HeaderArg {
    pub key: String,
    pub value: String,
}

fn parse_header(input: &str) -> Result<HeaderArg, String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "headers must be formatted as key=value".to_string())?;
    if key.is_empty() {
        return Err("header key cannot be empty".into());
    }
    Ok(HeaderArg {
        key: key.to_string(),
        value: value.to_string(),
    })
}

mod telemetry_writers {
    use nauto_telemetry::TelemetrySnapshot;
    use std::io;

    pub trait TelemetryWriter {
        fn write(&self, snapshots: &[TelemetrySnapshot]) -> Result<(), anyhow::Error>;
    }

    #[derive(Default)]
    pub struct JsonWriter;
    impl TelemetryWriter for JsonWriter {
        fn write(&self, snapshots: &[TelemetrySnapshot]) -> Result<(), anyhow::Error> {
            println!("{}", serde_json::to_string_pretty(snapshots)?);
            Ok(())
        }
    }

    #[derive(Default)]
    pub struct CsvWriter;
    impl TelemetryWriter for CsvWriter {
        fn write(&self, snapshots: &[TelemetrySnapshot]) -> Result<(), anyhow::Error> {
            let mut wtr = csv::Writer::from_writer(io::stdout());
            wtr.write_record(["collector", "metric", "value", "labels"])?;
            for snapshot in snapshots {
                for (name, value) in &snapshot.metrics {
                    wtr.write_record([
                        snapshot.collector,
                        name,
                        &value.to_string(),
                        &serde_json::to_string(&snapshot.labels)?,
                    ])?;
                }
            }
            wtr.flush()?;
            Ok(())
        }
    }
}

mod telemetry_config {
    use super::*;
    use anyhow::{Context, Result};
    use serde::Deserialize;
    use std::fs;

    #[derive(Debug, Deserialize)]
    struct TelemetryConfigFile {
        collectors: Vec<CollectorConfig>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    enum CollectorConfig {
        Snmp {
            device_id: String,
            target: String,
            community: String,
            oids: Vec<String>,
            #[serde(default = "default_timeout_secs")]
            timeout_secs: u64,
        },
        Gnmi {
            address: String,
            path: Vec<String>,
            #[serde(default)]
            username: Option<String>,
            #[serde(default)]
            password: Option<String>,
        },
        Http {
            endpoint: String,
            #[serde(default)]
            headers: std::collections::HashMap<String, String>,
        },
    }

    fn default_timeout_secs() -> u64 {
        2
    }

    impl CollectorConfig {
        fn into_collector(self) -> Result<Box<dyn TelemetryCollector>> {
            match self {
                CollectorConfig::Snmp {
                    device_id,
                    target,
                    community,
                    oids,
                    timeout_secs,
                } => Ok(Box::new(SnmpCollector {
                    device_id,
                    target,
                    community,
                    oids,
                    timeout: std::time::Duration::from_secs(timeout_secs),
                })),
                CollectorConfig::Gnmi {
                    address,
                    path,
                    username,
                    password,
                } => Ok(Box::new(GnmiCollector {
                    address,
                    path,
                    data_type: GnmiDataType::State,
                    encoding: GnmiEncoding::Json,
                    username,
                    password,
                })),
                CollectorConfig::Http { endpoint, headers } => {
                    let mut collector = HttpCollector::new(endpoint);
                    collector.headers.extend(headers);
                    Ok(Box::new(collector))
                }
            }
        }
    }

    pub fn load_collectors(path: &std::path::Path) -> Result<Vec<Box<dyn TelemetryCollector>>> {
        let body = fs::read_to_string(path)
            .with_context(|| format!("reading telemetry config {path:?}"))?;
        let config: TelemetryConfigFile =
            serde_yaml::from_str(&body).context("parsing telemetry config YAML")?;
        config
            .collectors
            .into_iter()
            .map(|cfg| cfg.into_collector())
            .collect()
    }
}
