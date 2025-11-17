# Regression Report (v0.1.0 Release Prep)

Date: 2025-11-17

## Toolchain
- `cargo check`
- `cargo test` (now executes on Linux, macOS, and Windows via matrix CI)

Both completed successfully with no warnings or failures.

### Notable Automated Tests Added
- `apps/nauto_cli/tests/worker_queue.rs` exercises the worker queue processor end-to-end using the mock driver registry (verifies queue rewriting, results persistence, and approval deferrals).
- `crates/nauto_telemetry/src/lib.rs::collect_all_filters_failures` ensures concurrent collector execution filters failing collectors without aborting the remaining snapshots.

## CLI Smoke Tests
| Command | Args | Result |
|---------|------|--------|
| `nauto_cli bench` | `--devices 100 --parallel 50` | Completed; reported throughput for synthetic drivers. |
| `nauto_cli telemetry` | `--format json` | Produced telemetry snapshot (SNMP/GNMI/HTTP collectors). |
| `nauto_cli transactions` | `--job examples/jobs/show_version.yaml --inventory examples/inventory.yaml --output plans/test_plan.yaml --canary-size 2 --batch-size 2` | Generated transaction plan YAML. |
| `nauto_cli worker` | `--queue queue/jobs.jsonl --limit 1 --dry-run` | Simulated dispatch for first queue item. |
| `nauto_cli integrations netbox-import` | `--file examples/netbox_devices.json --output inventory_netbox.yaml --credential lab-default` | Converted NetBox JSON to inventory YAML. |

No errors were observed; outputs inspected locally where applicable.

## Artifacts
- `docs/release_notes.md` – summary of milestones and release highlights.
- `plans/test_plan.yaml` – example transaction plan.
- `inventory_netbox.yaml` – NetBox-derived inventory sample.

## Follow-ups
- Proceed with CI workflow setup (fmt/clippy/test + Tauri build job) – tracking matrix job health over the next week.
- Continue plan steps per `net.plan.md`.

