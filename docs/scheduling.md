# Scheduling (Milestone 1)

## CLI Preview
```bash
nauto_cli schedule --file examples/schedules.yaml --iterations 3
```
- Parses cron expressions via `cron` crate.
- Prints upcoming fire times for each job (useful before wiring to runner service).

## UI Integration
- Tauri GUI exposes "Scheduling" pane where users add schedules (name + cron).
- Entries are persisted in-memory for now; future work will sync with backend service.

## Roadmap
- Connect scheduler output to job engine (spawn background runner).
- Surface status + failure notifications in GUI + CLI.
- Allow enabling/disabling schedules and editing from UI.