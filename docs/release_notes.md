# Release Notes – v0.1.0 (Milestones 1–3)

## Highlights
- **Core Engine & Drivers**: Async job orchestration with Cisco IOS, Juniper Junos, Arista EOS, Cisco NX-OS API, Meraki Cloud, and generic SSH drivers.
- **Security & UX**: Keyring-backed credentials, audit log, CLI/TUI dashboard, Tauri GUI control center.
- **Compliance & Scheduling**: Compliance engine + CLI exports, cron-based scheduling preview, GitOps integration.
- **Plugins & Marketplace**: WASM plugin SDK, sample host/guest, marketplace index + CLI install/verify commands.
- **Telemetry & Scale**: Telemetry collectors (SNMP/gNMI/HTTP), benchmark tool, scheduling & GitOps docs.
- **Workflows**: Approvals and notification commands, integration points for NetBox imports and ServiceNow change logging.
- **Distributed & Observability**: Worker CLI for queued jobs, Prometheus metrics snapshot command.

## Changelog
- Add new driver modules and capability flags, update sample inventory.
- Build `nauto_compliance`, `nauto_telemetry`, `nauto_plugin_sdk`, `nauto_integrations`, benchmarking and scheduling utilities.
- Expand documentation set (drivers, compliance, scheduling, GitOps, plugins, workflows, performance, distributed, observability).
- Enhance CLI with subcommands for approvals, notify, marketplace, integrations, transactions, worker, telemetry, bench, compliance, gitops, schedule, observability.
- Upgrade Tauri GUI to multi-panel control center (inventory, job wizard, scheduling, compliance snapshot).

## Next Steps
- Tag repo `v0.1.0` after final regression pass.
- Publish binaries (CLI + GUI) and attach to GitHub release.
- Continue plan: regression log, CI setup, service-mode design/prototype, security hardening, quick-start docs, pilot rollout.

