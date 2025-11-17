# UX Summary (CLI/TUI/GUI)

## CLI
- Binary: `nauto_cli`
- Commands:
  - `nauto_cli run --job examples/jobs/show_version.yaml --inventory examples/inventory.yaml [--dry-run]`
  - `nauto_cli creds --name lab-default --username admin --password *****`
  - `nauto_cli tui --inventory examples/inventory.yaml`
- Outputs JSON audit lines to `logs/audit.log` for each job.

## TUI
- Implemented with `ratatui` inside the CLI binary (`tui::launch`).
- Features: Device list navigation, detail pane (ID/address/tags/driver), keyboard shortcuts (`q`, `↑`, `↓`).
- Runs in alternate screen with raw mode; cleans up terminal after exit.

## GUI Control Center (Tauri)
- Location: `spikes/tauri_poc`
- Panels: inventory grid, job wizard, scheduling view, compliance snapshot.
- Commands exposed: `list_inventory`, `create_job`, `list_schedules`, `add_schedule`, `compliance_snapshot`.
- Launch via `cd spikes/tauri_poc && cargo tauri dev`.

## Next Steps
- Add job progress streaming to TUI and Tauri using tracing events.
- Back UI screens with real backend APIs once service mode is available.
- Provide packaged binaries for macOS/Windows/Linux once GUI stabilizes.

