# External Integrations (Milestone 3)

## NetBox Import
```bash
nauto_cli integrations netbox-import \
  --file examples/netbox_devices.json \
  --output inventory_from_netbox.yaml \
  --credential lab-default
```
- Parses NetBox JSON export (`devices[*]` list) and produces an inventory YAML compatible with the engine.
- Device type inferred from NetBox `device_type.model` (fallback to `GenericSsh`).

## ServiceNow Change Log
```bash
nauto_cli integrations servicenow-change \
  --ticket CHG0012345 \
  --description "Bulk NTP rollout" \
  --dry-run
```
- Simulates logging a change request; `--dry-run` prints without sending.
- Future work: call ServiceNow REST API when credentials are configured.

## Workflow
- Use NetBox import to seed the local inventory before running jobs.
- Record change ticket ID with ServiceNow command prior to job execution, then reference ticket in job labels.

