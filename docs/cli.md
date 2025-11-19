# CLI & TUI UX

Binary: `nauto_cli`

## Commands
- `nauto_cli creds --name lab-default --username admin --password-prompt`
  - Stores credentials securely using the OS keychain via the `KeyringStore`. Use `--password-stdin` for automation or `--password` only when you accept the argv exposure risk.
- `nauto_cli run --job examples/jobs/show_version.yaml --inventory examples/inventory.yaml`
  - Loads YAML definitions, executes the async job engine, and writes a JSON audit line to `logs/audit.log`.
- `nauto_cli tui --inventory examples/inventory.yaml`
  - Opens the ratatui-based dashboard. Use ↑/↓ to navigate devices, `q` to exit.

## Audit Logs
Located at `logs/audit.log`. Each line is JSON containing job metadata to feed SIEM/Splunk.

## Dry-Run Flag
`--dry-run` overrides the job definition, enabling plan-only executions where supported. The engine auto-skips drivers without dry-run capability.