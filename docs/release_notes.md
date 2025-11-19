# Release Notes – v0.1.0 (Milestones 1–3)

## Highlights
- **Core Engine & Drivers**: Async job orchestration with Cisco IOS, Juniper Junos, Arista EOS, Cisco NX-OS API, Meraki Cloud, and generic SSH drivers.
- **Security & UX**: Keyring-backed credentials, audit log, CLI/TUI dashboard, Tauri GUI control center.
- **Compliance & Scheduling**: Compliance engine + CLI exports, cron-based scheduling preview, GitOps integration.
- **Plugins & Marketplace**: WASM plugin SDK, sample host/guest, marketplace index + CLI install/verify commands.
- **Telemetry & Scale**: Telemetry collectors (SNMP/gNMI/HTTP), benchmark tool, scheduling & GitOps docs.
- **Workflows**: Approvals and notification commands, integration points for NetBox imports and ServiceNow change logging.
- **Distributed & Observability**: Worker CLI for queued jobs, Prometheus metrics snapshot command.
- **Reliability & UX Additions**: Configurable driver timeouts/retries, NX-OS rollback via NX-API, CA-bundled builds, per-device audit logs, CLI progress spinner, TUI refresh key, GUI job wizard backed by the real job runner.

## Changelog
- Implement configurable SSH/HTTP timeouts (`NAUTO_SSH_TIMEOUT_SECS`, `NAUTO_HTTP_TIMEOUT_SECS`) and retry limits (`NAUTO_HTTP_RETRIES`) shared across all drivers; ship repo-local Mozilla CA bundle and refresh script.
- Extend NX-OS API driver with real rollback support and ensure audit logs capture per-device JSON lines.
- Add `NAUTO_KEYRING_FILE` fallback for headless deployments; mirror secrets into encrypted JSON when OS keyrings are unavailable.
- Enhance CLI run UX (spinner + failed device summary), TUI (`r` reload key), and GUI (inventory sourced from examples + job wizard executing mock-driver runs via `nauto_cli::job_runner`).
- Harden worker mode: reusable queue processor, worker daemon invoking real jobs, and an integration test that covers queue rewrites/results persistence.
- Load WASM plugin metadata (vendor/device type/capabilities) at startup, enforce signature verification during install, and expose descriptors for future driver registration.
- Add new driver modules and capability flags, update sample inventory.
- Build `nauto_compliance`, `nauto_telemetry`, `nauto_plugin_sdk`, `nauto_integrations`, benchmarking and scheduling utilities.
- Expand documentation set (drivers, compliance, scheduling, GitOps, plugins, workflows, performance, distributed, observability).
- Enhance CLI with subcommands for approvals, notify, marketplace, integrations, transactions, worker, telemetry, bench, compliance, gitops, schedule, observability.
- Upgrade Tauri GUI to multi-panel control center (inventory, job wizard, scheduling, compliance snapshot).

## Next Steps
- Tag repo `v0.1.0` after final regression pass.
- Publish binaries (CLI + GUI) and attach to GitHub release.
- Continue plan: regression log, CI setup, service-mode design/prototype, security hardening, quick-start docs, pilot rollout.