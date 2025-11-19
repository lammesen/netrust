# Performance & Scale

## Benchmark Command

Use the CLI bench tool to simulate large jobs with mock drivers:

```bash
nauto_cli bench --devices 5000 --parallel 200
```

Outputs devices processed, elapsed time, and throughput (devices/sec). Adjust device count and parallelism to match target scale tests.

## Engine Tuning
- Use job-level `--max-parallel` (CLI) or `Job.max_parallel` fields to control concurrency per run.
- Default semaphore limit is 32; bench command lets you validate safe increases before applying to real jobs.

## Telemetry Integration
- `nauto_cli telemetry --format json|csv` runs SNMP/gNMI/HTTP collectors (simulated) to validate dashboard ingestion and measure latency contributions.
- Dashboard panels (Tauri GUI) will consume the same collectors when connected to backend services.

## Stress Testing Plan
- Run `nauto_cli bench` nightly with at least 10k synthetic devices; track throughput and failure counts over time.
- Profile memory/CPU while bench command executes to detect regressions.
- Expand collectors to hit mock SNMP/gNMI servers to ensure telemetry pipeline scales with bench load.