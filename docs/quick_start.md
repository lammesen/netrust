# Quick Start Guide

## 1. Prerequisites
- Rust toolchain (stable).
- Node.js + Tauri dependencies (if running GUI).
- Access to NetBox export (optional) and target inventory credentials stored via `nauto_cli creds`.

## 2. Import Inventory (NetBox)
```bash
nauto_cli integrations netbox-import \
  --file examples/netbox_devices.json \
  --output inventory_netbox.yaml \
  --credential lab-default
```

## 3. Request Approval
```bash
nauto_cli approvals request \
  --job examples/jobs/show_version.yaml \
  --requested-by alice \
  --note "NTP refresh"
```

## 4. Plan Transaction
```bash
nauto_cli transactions \
  --job examples/jobs/show_version.yaml \
  --inventory inventory_netbox.yaml \
  --output plans/ntp_plan.yaml \
  --canary-size 5 --batch-size 25
```

## 5. Execute (CLI or GUI)
- CLI: `nauto_cli run --job examples/jobs/show_version.yaml --inventory inventory_netbox.yaml`
- GUI: `cd spikes/tauri_poc && cargo tauri dev` then use Job Wizard.

## 6. Telemetry & Observability
```bash
nauto_cli telemetry --format json
nauto_cli observability
```

## 7. Distributed Worker (Preview)
```bash
NAUTO_QUEUE=queue/jobs.jsonl cargo run -p nauto_cli --bin worker_daemon
```

## 8. Marketplace Plugins
```bash
nauto_cli marketplace list
nauto_cli marketplace install --name "VendorX Experimental"
```

