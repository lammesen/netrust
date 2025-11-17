# Advanced Change Management

## Transaction Plan CLI
```bash
nauto_cli transactions \
  --job examples/jobs/show_version.yaml \
  --inventory examples/inventory.yaml \
  --output plans/ntp_plan.yaml \
  --canary-size 5 \
  --batch-size 50
```
- Both `--canary-size` and `--batch-size` must be greater than zero; the CLI now validates inputs up front to avoid runtime hangs.

Generates a YAML plan listing:
- `canary`: first N devices to test change.
- `batches`: subsequent chunks (size configurable).

## Usage Flow
1. Generate plan.
2. Execute canary subset manually (or via job targeting `canary` IDs).
3. If successful, run remaining batches sequentially, pausing between for validation.

## Future Enhancements
- Integrate directly with job engine to auto-execute plan.
- Link approvals/notifications per batch.

