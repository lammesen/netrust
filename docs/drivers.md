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

### Registry
`DriverRegistry` bundles these implementations so the job engine can discover the correct driver per device type at runtime.

