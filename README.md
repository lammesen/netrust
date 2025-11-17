# netrust

[![CI](https://github.com/your-org/netrust/actions/workflows/ci.yml/badge.svg)](https://github.com/your-org/netrust/actions/workflows/ci.yml)

Cross-platform network automation toolkit in Rust. Milestones 1–3 deliver:
- Async job engine with multi-vendor drivers (Cisco IOS/NX-OS, Juniper Junos, Arista EOS, Meraki Cloud, generic SSH).
- Secure CLI/TUI + Tauri GUI control center.
- Compliance, telemetry, scheduling, GitOps, plugin marketplace, approvals/workflows.

## Quick Links
- [Release Notes](docs/release_notes.md)
- [Quick Start](docs/quick_start.md)
- [Service Architecture](docs/service_architecture.md)
- [Testing Report](docs/testing.md)

## CLI Highlights
```
nauto_cli run --job examples/jobs/show_version.yaml --inventory examples/inventory.yaml
nauto_cli compliance --rules examples/compliance_rules.yaml --inputs examples/compliance_inputs.yaml
nauto_cli bench --devices 1000 --parallel 200
nauto_cli telemetry --format json
nauto_cli transactions --job ... --inventory ... --output plans/plan.yaml
nauto_cli worker --queue queue/jobs.jsonl --dry-run
nauto_cli marketplace list
```

## Development
```bash
cargo fmt --all
cargo clippy --all-targets --all-features
cargo test
```
CI runs fmt/clippy/test/audit plus a Tauri smoke build via GitHub Actions.

## GUI (Web UI)
```
cd apps/web-ui
npm run dev
```
_Note: The previous Tauri spike lives at `spikes/tauri_poc`; we can re-wrap the new web UI with Tauri when needed._

## License
MIT (planned) – update before public release.
