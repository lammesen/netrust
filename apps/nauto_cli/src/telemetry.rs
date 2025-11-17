use crate::telemetry_writers::{CsvWriter, JsonWriter, TelemetryWriter};
use anyhow::Result;
use clap::Args;
use nauto_telemetry::{collect_all, GnmiCollector, HttpCollector, SnmpCollector};

#[derive(Args)]
pub struct TelemetryCmd {
    #[arg(long, default_value = "json", value_parser = ["json", "csv"])]
    pub format: String,
}

pub async fn run(cmd: TelemetryCmd) -> Result<()> {
    let collectors: Vec<Box<dyn nauto_telemetry::TelemetryCollector>> = vec![
        Box::new(SnmpCollector {
            device_id: "core-r1".into(),
        }),
        Box::new(GnmiCollector {
            target: "spine-nxapi".into(),
        }),
        Box::new(HttpCollector {
            endpoint: "https://api.meraki.com".into(),
        }),
    ];
    let snapshots = collect_all(&collectors).await;

    match cmd.format.as_str() {
        "csv" => CsvWriter::default().write(&snapshots),
        _ => JsonWriter::default().write(&snapshots),
    }
}

mod telemetry_writers {
    use nauto_telemetry::TelemetrySnapshot;
    use std::io::{self, Write};

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
