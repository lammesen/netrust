# Vendor Driver Implementations

The initial driver set covers the three MVP platforms referenced in `plan.md` Section 6.

## Cisco IOS Driver
- Module: `nauto_drivers::drivers::cisco_ios`
- Simulates CLI workflows (enter config mode, push lines, write memory).
- Advertises no transactional features to ensure the engine uses diff-based safeguards.

## Juniper Junos Driver
- Module: `nauto_drivers::drivers::juniper_junos`
- Models commit-check and commit-confirm flows and exposes rollback support.
- Engine can leverage the `supports_dry_run` flag for commit-check only runs.

## Generic SSH Driver
- Module: `nauto_drivers::drivers::generic_ssh`
- Provides best-effort command/config streaming for unsupported vendors.
- Useful for quick coverage while bespoke drivers are developed.

## Arista EOS Driver
- Module: `nauto_drivers::drivers::arista_eos`
- Mirrors Cisco CLI behavior (config mode, copy run start) with EOS-specific logging.

## Cisco NX-OS API Driver
- Module: `nauto_drivers::drivers::cisco_nxos_api`
- Builds NX-API JSON payloads for command/config execution (simulated via logs for now).
- Marked as transactional (commit/dry-run capable) for future integration with real endpoints.

## Meraki Cloud Driver
- Module: `nauto_drivers::drivers::meraki_cloud`
- Represents REST-based operations against Meraki Dashboard APIs (org/network derived from `mgmt_address`).
- Rollback modeled as template reversion call.

### Registry
`DriverRegistry` bundles these implementations so the job engine can discover the correct driver per device type at runtime.

### Test Coverage
- `driver_capabilities_reported` (crate tests) validates registry lookups + capability masks.
- Future work: mock transport layer to simulate error handling, rollback paths, and diffs.

