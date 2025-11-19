# Quick Start Guide

## 1. Prerequisites
- Rust toolchain (stable).
- Node.js + Tauri dependencies (if running GUI).
- Access to NetBox export (optional) and target inventory credentials stored via `nauto_cli creds`.
- **Important**: All commands below assume you are running from the project root directory.

## 2. Store Credentials
```bash
nauto_cli creds \
  --name lab-default \
  --username admin \
  --password-prompt
```
- Use `--password-stdin` in CI pipelines where interactive prompts are not available.

## 3. Import Inventory (NetBox)
```bash
nauto_cli integrations netbox-import \
  --file examples/netbox_devices.json \
  --output inventory_netbox.yaml \
  --credential lab-default
```

## 4. Request Approval
```bash
nauto_cli approvals request \
  --job examples/jobs/show_version.yaml \
  --requested-by alice \
  --note "NTP refresh"
```

## 5. Plan Transaction
```bash
nauto_cli transactions \
  --job examples/jobs/show_version.yaml \
  --inventory inventory_netbox.yaml \
  --output plans/ntp_plan.yaml \
  --canary-size 5 --batch-size 25
```

## 6. Execute (CLI or GUI)
- CLI: `nauto_cli run --job examples/jobs/show_version.yaml --inventory inventory_netbox.yaml`
- GUI: `cd spikes/tauri_poc && cargo tauri dev` then use Job Wizard.

## 7. Telemetry & Observability
```bash
nauto_cli telemetry --format json
nauto_cli observability
```

## 8. Distributed Worker (Preview)
```bash
NAUTO_QUEUE=queue/jobs.jsonl cargo run -p nauto_cli --bin worker_daemon
```

## 9. Marketplace Plugins
```bash
nauto_cli marketplace list
nauto_cli marketplace install --name "VendorX Experimental"
```