# Observability (Prometheus + Tracing)

## Metrics Snapshot
```bash
nauto_cli observability
```
- Emits Prometheus-formatted counters/gauges (jobs_total, jobs_failed_total, queue_depth).
- Pass `--format json` for a structured snapshot that tooling/QA dashboards can parse without scraping logic.
- Intended to be scraped via cron or piped into a lightweight HTTP exporter.
- Job engine now emits `device_task` tracing spans per device/job execution for correlation.

## Telemetry Collectors

Use the revamped telemetry CLI to execute real SNMP/gNMI/HTTP collectors concurrently:

```bash
nauto_cli telemetry --config examples/telemetry.yaml --format json
```

- The YAML file lists any number of collectors (see `examples/telemetry.yaml`); each entry is executed in parallel via `futures::join_all`.
- SNMP collectors perform blocking gets inside a `spawn_blocking` section so CLI responsiveness is preserved even with slow agents.
- gNMI collectors issue real `GetRequest` RPCs (JSON encoding) and flatten responses into metric names (e.g. `system/state/cpu/utilization`).
- HTTP collectors recurse through JSON payloads, exporting every numeric leaf which makes it easy to ingest ad-hoc REST metrics.
- Output remains selectable between JSON and CSV writers so snapshots can feed dashboards or spreadsheets directly.

## Future Work
- Run the command as a daemon (or integrate into service) exposing metrics endpoint.
- Wire tracing spans to OpenTelemetry exporters, shipping to Jaeger/Tempo.
- Enrich metrics with real job data once service mode is enabled.

