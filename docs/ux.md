# UX Summary (CLI/TUI/GUI)

## CLI
- Binary: `nauto_cli`
- Commands:
  - `nauto_cli run --job examples/jobs/show_version.yaml --inventory examples/inventory.yaml [--dry-run]`
  - `nauto_cli creds --name lab-default --username admin --password-prompt`
  - `nauto_cli tui --inventory examples/inventory.yaml`
- Outputs JSON audit lines to `logs/audit.log` plus per-device entries in `logs/audit.devices.jsonl`.
- `run` now prints a live progress spinner (disable via `--no-progress`) and summarizes any failed device IDs after execution.

## TUI
- Implemented with `ratatui` inside the CLI binary (`tui::launch`).
- Features: Device list navigation, detail pane (ID/address/tags/driver), keyboard shortcuts (`q`, `↑`, `↓`, `r` to reload the inventory file on demand).
- Runs in alternate screen with raw mode; cleans up terminal after exit.

## GUI Control Center (Tauri)
- Location: `spikes/tauri_poc`
- Panels: inventory grid, job wizard, scheduling view, compliance snapshot.
- Commands exposed: `list_inventory`, `create_job`, `list_schedules`, `add_schedule`, `compliance_snapshot`.
- Inventory panel now reads the real `examples/inventory.yaml` via the shared `nauto_cli::job_runner` helper so it stays in sync with CLI runs.
- Job wizard invokes the shared job runner asynchronously (with mock drivers) to produce real success/failure summaries while keeping GUI demo-friendly.
- Launch via `cd spikes/tauri_poc && cargo tauri dev`.

## Next Steps
- Add job progress streaming to TUI and Tauri using tracing events.
- Back UI screens with real backend APIs once service mode is available.
- Provide packaged binaries for macOS/Windows/Linux once GUI stabilizes.

## Frontend Plan (Web UI)
The Web UI is being built with React + Vite + TypeScript + Tailwind.
- **Stack**: shadcn/ui, Headless UI, TanStack Query, React Hook Form + Zod.
- **Status**: Scaffolded, themed, layout shell complete.
- **Planned Features**:
    - Inventory Table (TanStack Table).
    - Job Wizard (Form + Validation).
    - Scheduling (Cron helper).
    - Compliance Dashboard.
    - Real-time feedback (Toasts, Optimistic updates).

See [TODO.md](../TODO.md) for the full frontend roadmap.