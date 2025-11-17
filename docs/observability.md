# Observability (Prometheus + Tracing)

## Metrics Snapshot
```bash
nauto_cli observability
```
- Emits Prometheus-formatted counters/gauges (jobs_total, jobs_failed_total, queue_depth).
- Pass `--format json` for a structured snapshot that tooling/QA dashboards can parse without scraping logic.
- Intended to be scraped via cron or piped into a lightweight HTTP exporter.
- Job engine now emits `device_task` tracing spans per device/job execution for correlation.

## Future Work
- Run the command as a daemon (or integrate into service) exposing metrics endpoint.
- Wire tracing spans to OpenTelemetry exporters, shipping to Jaeger/Tempo.
- Enrich metrics with real job data once service mode is enabled.

