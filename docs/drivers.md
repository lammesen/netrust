# Vendor Driver Implementations

## Cisco IOS Driver (`nauto_drivers::drivers::cisco_ios`)
- Uses `async-ssh2-tokio` to establish a CLI session over SSH (port 22) with credentials sourced from the OS keyring.
- Every operation is executed as real CLI commands (command batches run verbatim, config pushes run `configure terminal`, stream the snippet, then `write memory`).
- Captures `show running-config` before/after each config push and emits a textual diff using the `similar` crate so audit logs hold concrete configuration state.
- Supports rollback by feeding the captured snapshot back through `configure replace terminal`.

## Juniper Junos Driver (`nauto_drivers::drivers::juniper_junos`)
- Speaks NETCONF over SSH (port 830) using the same keyring-backed credentials.
- Implements the full lock → edit-config → validate → commit → unlock flow, wrapping snippets in `<configuration-text/>` and parsing XML replies for `<rpc-error>`.
- Provides real running-config snapshots/diffs and honors the existing capability flags (`supports_commit`, `supports_dry_run`, `supports_rollback`).
- Operational commands (`JobKind::CommandBatch`) are executed over a standard SSH CLI session so show commands can be run without NETCONF.

## Generic SSH Driver (`nauto_drivers::drivers::generic_ssh`)
- Establishes an SSH session through `async-ssh2-tokio` and executes each command via `exec`.
- Config pushes stream the snippet inside `configure terminal … end` and log the resulting stdout/stderr so even “unknown” vendors get real-time feedback.
- Still advertises no transactional support, but now produces real device output instead of simulated sleeps.

## Arista EOS Driver (`nauto_drivers::drivers::arista_eos`)
- Shares the Cisco SSH transport but issues EOS-specific follow-ups (`copy running-config startup-config`) and captures snapshots/diffs.
- Rollback mirrors the IOS behavior using `configure replace terminal force` when a snapshot is available.

## Cisco NX-OS API Driver (`nauto_drivers::drivers::cisco_nxos_api`)
- Replaced the stubbed logger with authenticated NX-API HTTP calls via `reqwest` (JSON payloads posted to `https://<mgmt_address>/ins`).
- Parses the returned `ins_api.outputs.output[*].code/msg` objects; any non-`200` code bubbles up as an error with the original body for debugging.
- Config pushes now run, verify success, and capture pre/post snapshots by issuing real `show running-config` calls.

## Meraki Cloud Driver (`nauto_drivers::drivers::meraki_cloud`)
- Still performs real REST calls with API tokens sourced from the keyring.
- Advertises `supports_rollback = false` until we have a deterministic rollback API—rollback requests are logged with a warning instead of silently “succeeding.”

### Registry
`DriverRegistry` still bundles all driver implementations so the job engine can resolve a `DeviceType` to its concrete driver. Capability flags now reflect the real transport behaviors (e.g., only Junos advertises rollback/dry-run, Meraki no longer claims rollback).

### Test Coverage
- `driver_capabilities_reported` ensures registry wiring remains intact after capability tweaks.
- `cargo test -p nauto_drivers` compiles the new SSH/NETCONF/NX-API integrations; dedicated transport mocks will be added in a follow-up to exercise failure paths without real hardware.

